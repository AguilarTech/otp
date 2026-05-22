mod crypto;
pub mod error;
pub mod frame;
mod pad;
mod state;

#[cfg(test)]
mod tests;

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

pub use error::{Result, VaultError};
pub use frame::{FRAME_VERSION, MAC_LEN, POLY_KEY_LEN};

use crypto::{compute_mac, verify_mac, xor_in_place};
use frame::{parse_frame, Header};
use state::{atomic_write, Pairing, State};

#[derive(Copy, Clone, Debug)]
enum Direction {
    Out,
    In,
}

#[derive(Debug, Clone, Serialize)]
pub struct PairingInfo {
    pub id: Uuid,
    pub name: String,
    pub created_at_ms: u64,
    pub out_total: u64,
    pub out_remaining: u64,
    pub in_total: u64,
    pub in_remaining: u64,
    pub seq_out: u64,
    pub last_seq_in: u64,
    pub drive_folder_id: String,
}

impl From<&Pairing> for PairingInfo {
    fn from(p: &Pairing) -> Self {
        Self {
            id: p.id,
            name: p.name.clone(),
            created_at_ms: p.created_at_ms,
            out_total: p.out_total,
            out_remaining: p.out_total.saturating_sub(p.out_cursor),
            in_total: p.in_total,
            in_remaining: p.in_total.saturating_sub(p.in_cursor),
            seq_out: p.seq_out,
            last_seq_in: p.last_seq_in,
            drive_folder_id: p.drive_folder_id.clone(),
        }
    }
}

pub struct DecryptedMessage {
    pub pairing_id: Uuid,
    pub seq: u64,
    pub timestamp_ms: u64,
    pub plaintext: Zeroizing<Vec<u8>>,
}

pub struct Vault {
    base: PathBuf,
    state: Mutex<State>,
}

impl Vault {
    pub fn open(base: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&base)?;
        std::fs::create_dir_all(base.join("pads"))?;
        let st = State::load_or_init(&base.join("pairings.toml"))?;
        Ok(Self {
            base,
            state: Mutex::new(st),
        })
    }

    fn state_path(&self) -> PathBuf {
        self.base.join("pairings.toml")
    }

    fn pad_path(&self, id: &Uuid, direction: Direction) -> PathBuf {
        let name = match direction {
            Direction::Out => "out.pad",
            Direction::In => "in.pad",
        };
        self.base.join("pads").join(id.to_string()).join(name)
    }

    fn persist(&self, st: &State) -> Result<()> {
        let raw = toml::to_string(st)?;
        atomic_write(&self.state_path(), raw.as_bytes())
    }

    pub fn list_pairings(&self) -> Result<Vec<PairingInfo>> {
        let st = self.state.lock().unwrap();
        Ok(st.pairings.iter().map(PairingInfo::from).collect())
    }

    /// Remove a pairing from state, then best-effort wipe and delete its
    /// pad files. Committing the state change first means a crash mid-wipe
    /// still leaves the pairing forgotten from the app's perspective —
    /// consistent with the rest of the vault's "advance cursor, then
    /// best-effort zero" ordering.
    pub fn delete_pairing(&self, pairing_id: &Uuid) -> Result<()> {
        let mut st = self.state.lock().unwrap();
        let idx = st
            .pairings
            .iter()
            .position(|p| &p.id == pairing_id)
            .ok_or(VaultError::PairingNotFound(*pairing_id))?;
        st.pairings.remove(idx);
        self.persist(&st)?;
        drop(st);

        let out_path = self.pad_path(pairing_id, Direction::Out);
        let in_path = self.pad_path(pairing_id, Direction::In);
        let _ = wipe_and_remove(&out_path);
        let _ = wipe_and_remove(&in_path);
        let _ = std::fs::remove_dir(self.base.join("pads").join(pairing_id.to_string()));
        Ok(())
    }

    pub fn set_drive_folder_id(&self, pairing_id: &Uuid, folder_id: String) -> Result<()> {
        let mut st = self.state.lock().unwrap();
        {
            let p = st.find_mut(pairing_id)?;
            p.drive_folder_id = folder_id;
        }
        self.persist(&st)
    }

    pub fn clear_drive_folder_id(&self, pairing_id: &Uuid) -> Result<()> {
        let mut st = self.state.lock().unwrap();
        {
            let p = st.find_mut(pairing_id)?;
            p.drive_folder_id = String::new();
        }
        self.persist(&st)
    }

    /// Generate two fresh random pads of the given size and register the
    /// pairing locally without writing to a USB volume. The Tauri UI uses
    /// `create_and_export_pairing` for the real flow; this stays exposed
    /// for tests and programmatic callers.
    #[allow(dead_code)]
    pub fn create_pairing(&self, name: String, pad_size: u64) -> Result<PairingInfo> {
        let id = Uuid::new_v4();
        let dir = self.base.join("pads").join(id.to_string());
        std::fs::create_dir_all(&dir)?;
        generate_pad(&self.pad_path(&id, Direction::Out), pad_size)?;
        generate_pad(&self.pad_path(&id, Direction::In), pad_size)?;
        let now = now_ms();
        let p = Pairing {
            id,
            name,
            created_at_ms: now,
            out_total: pad_size,
            out_cursor: 0,
            in_total: pad_size,
            in_cursor: 0,
            seq_out: 0,
            last_seq_in: 0,
            drive_folder_id: String::new(),
        };
        let info = PairingInfo::from(&p);
        let mut st = self.state.lock().unwrap();
        st.pairings.push(p);
        self.persist(&st)?;
        Ok(info)
    }

    /// Register a pairing whose pad material is already in memory.
    /// Used by tests; the Tauri UI calls `import_pairing_from_usb`.
    #[allow(dead_code)]
    pub fn import_pairing(
        &self,
        id: Uuid,
        name: String,
        out_pad: &[u8],
        in_pad: &[u8],
    ) -> Result<PairingInfo> {
        {
            let st = self.state.lock().unwrap();
            if st.pairings.iter().any(|p| p.id == id) {
                return Err(VaultError::PairingExists(id));
            }
        }
        let dir = self.base.join("pads").join(id.to_string());
        std::fs::create_dir_all(&dir)?;
        write_pad(&self.pad_path(&id, Direction::Out), out_pad)?;
        write_pad(&self.pad_path(&id, Direction::In), in_pad)?;
        let p = Pairing {
            id,
            name,
            created_at_ms: now_ms(),
            out_total: out_pad.len() as u64,
            out_cursor: 0,
            in_total: in_pad.len() as u64,
            in_cursor: 0,
            seq_out: 0,
            last_seq_in: 0,
            drive_folder_id: String::new(),
        };
        let info = PairingInfo::from(&p);
        let mut st = self.state.lock().unwrap();
        st.pairings.push(p);
        self.persist(&st)?;
        Ok(info)
    }

    /// Create a pairing on the local vault and export the matching pad
    /// material + sidecar to a directory on USB so a peer can import it.
    /// Convention: `A.pad` is the creator's outbound stream (importer's
    /// inbound), `B.pad` is the creator's inbound (importer's outbound).
    pub fn create_and_export_pairing(
        &self,
        name: String,
        originator_hint: String,
        pad_size: u64,
        usb_dir: &Path,
    ) -> Result<PairingInfo> {
        if pad_size <= POLY_KEY_LEN {
            return Err(VaultError::InvalidState(
                "pad size must exceed Poly1305 key length (32 bytes)",
            ));
        }
        let id = Uuid::new_v4();
        let local_dir = self.base.join("pads").join(id.to_string());
        std::fs::create_dir_all(&local_dir)?;

        let usb_pairing_dir = usb_dir.join(pairing_export_dir_name(&name, &id));
        std::fs::create_dir_all(&usb_pairing_dir)?;

        let local_out = self.pad_path(&id, Direction::Out);
        let local_in = self.pad_path(&id, Direction::In);
        let usb_a = usb_pairing_dir.join("A.pad");
        let usb_b = usb_pairing_dir.join("B.pad");

        generate_pad_multi(&[&local_out, &usb_a], pad_size)?;
        generate_pad_multi(&[&local_in, &usb_b], pad_size)?;

        let now = now_ms();
        let sidecar = SidecarV1 {
            schema_version: SIDECAR_SCHEMA_VERSION,
            pairing_id: id,
            created_at_ms: now,
            originator_hint,
        };
        std::fs::write(
            usb_pairing_dir.join("pairing.toml"),
            toml::to_string(&sidecar)?,
        )?;

        let p = Pairing {
            id,
            name,
            created_at_ms: now,
            out_total: pad_size,
            out_cursor: 0,
            in_total: pad_size,
            in_cursor: 0,
            seq_out: 0,
            last_seq_in: 0,
            drive_folder_id: String::new(),
        };
        let info = PairingInfo::from(&p);
        let mut st = self.state.lock().unwrap();
        st.pairings.push(p);
        self.persist(&st)?;
        Ok(info)
    }

    /// Import a pairing from a USB folder previously produced by
    /// `create_and_export_pairing` on a peer's machine. Roles are swapped:
    /// the importer's outbound is the creator's inbound (`B.pad`) and vice
    /// versa.
    pub fn import_pairing_from_usb(
        &self,
        name: String,
        usb_pairing_dir: &Path,
    ) -> Result<PairingInfo> {
        let raw = std::fs::read_to_string(usb_pairing_dir.join("pairing.toml"))?;
        let sidecar: SidecarV1 = toml::from_str(&raw)?;
        if sidecar.schema_version != SIDECAR_SCHEMA_VERSION {
            return Err(VaultError::InvalidState("unsupported sidecar schema"));
        }
        let id = sidecar.pairing_id;
        {
            let st = self.state.lock().unwrap();
            if st.pairings.iter().any(|p| p.id == id) {
                return Err(VaultError::PairingExists(id));
            }
        }
        let usb_a = usb_pairing_dir.join("A.pad");
        let usb_b = usb_pairing_dir.join("B.pad");
        let local_dir = self.base.join("pads").join(id.to_string());
        std::fs::create_dir_all(&local_dir)?;

        std::fs::copy(&usb_b, self.pad_path(&id, Direction::Out))?;
        std::fs::copy(&usb_a, self.pad_path(&id, Direction::In))?;

        let out_total = std::fs::metadata(self.pad_path(&id, Direction::Out))?.len();
        let in_total = std::fs::metadata(self.pad_path(&id, Direction::In))?.len();

        let p = Pairing {
            id,
            name,
            created_at_ms: now_ms(),
            out_total,
            out_cursor: 0,
            in_total,
            in_cursor: 0,
            seq_out: 0,
            last_seq_in: 0,
            drive_folder_id: String::new(),
        };
        let info = PairingInfo::from(&p);
        let mut st = self.state.lock().unwrap();
        st.pairings.push(p);
        self.persist(&st)?;
        Ok(info)
    }

    pub fn encrypt(&self, pairing_id: &Uuid, plaintext: &[u8]) -> Result<Vec<u8>> {
        let length: u32 = plaintext
            .len()
            .try_into()
            .map_err(|_| VaultError::InvalidPlaintextLen(plaintext.len()))?;

        let mut st = self.state.lock().unwrap();
        let snapshot = st.find(pairing_id)?.clone();
        let needed = POLY_KEY_LEN
            .checked_add(length as u64)
            .ok_or(VaultError::InvalidState("length overflow"))?;
        let available = snapshot.out_total.saturating_sub(snapshot.out_cursor);
        if needed > available {
            return Err(VaultError::PadExhausted { needed, available });
        }
        let offset = snapshot.out_cursor;

        let pad_path = self.pad_path(pairing_id, Direction::Out);
        let raw = pad::read_at(&pad_path, offset, needed as usize)?;
        let buf: Zeroizing<Vec<u8>> = Zeroizing::new(raw);

        let mut mac_key = [0u8; 32];
        mac_key.copy_from_slice(&buf[..32]);
        let keystream = &buf[32..];

        let mut ciphertext = plaintext.to_vec();
        xor_in_place(&mut ciphertext, keystream);

        let header = Header {
            version: FRAME_VERSION,
            pairing_id: *pairing_id.as_bytes(),
            seq: snapshot.seq_out + 1,
            offset,
            length,
            timestamp_ms: now_ms(),
        };
        let header_bytes = header.encode();

        let mut mac_input = Vec::with_capacity(header_bytes.len() + ciphertext.len());
        mac_input.extend_from_slice(&header_bytes);
        mac_input.extend_from_slice(&ciphertext);
        let tag = compute_mac(&mac_key, &mac_input);
        mac_key.zeroize();
        mac_input.zeroize();

        // Advance cursor first — this is the consumption commit. Subsequent
        // on-disk zeroize is best-effort cleanup; a crash between the two
        // leaves bytes wasted (cursor moved past them) but not reusable.
        {
            let p = st.find_mut(pairing_id)?;
            p.out_cursor = offset + needed;
            p.seq_out += 1;
        }
        self.persist(&st)?;
        if let Err(e) = pad::zeroize_range(&pad_path, offset, needed) {
            eprintln!("warning: on-disk zeroize of out.pad failed: {}", e);
        }

        let mut frame = Vec::with_capacity(header_bytes.len() + ciphertext.len() + MAC_LEN);
        frame.extend_from_slice(&header_bytes);
        frame.extend_from_slice(&ciphertext);
        frame.extend_from_slice(&tag);
        Ok(frame)
    }

    pub fn decrypt(&self, frame: &[u8]) -> Result<DecryptedMessage> {
        let (header, ciphertext, mac_tag) = parse_frame(frame)?;
        let pairing_id = Uuid::from_bytes(header.pairing_id);

        let mut st = self.state.lock().unwrap();
        let snapshot = st.find(&pairing_id)?.clone();

        if header.seq <= snapshot.last_seq_in {
            return Err(VaultError::Replay {
                got: header.seq,
                last: snapshot.last_seq_in,
            });
        }
        let needed = POLY_KEY_LEN
            .checked_add(header.length as u64)
            .ok_or(VaultError::InvalidState("length overflow"))?;
        let end = header
            .offset
            .checked_add(needed)
            .ok_or(VaultError::InvalidState("offset overflow"))?;
        if end > snapshot.in_total {
            return Err(VaultError::OffsetOutOfBounds {
                offset: header.offset,
                length: needed,
                pad_size: snapshot.in_total,
            });
        }
        if header.offset < snapshot.in_cursor {
            // A later message has already been verified and consumed past
            // this offset — those pad bytes are gone. Refuse rather than
            // produce garbage.
            return Err(VaultError::OffsetOutOfBounds {
                offset: header.offset,
                length: needed,
                pad_size: snapshot.in_cursor,
            });
        }

        let pad_path = self.pad_path(&pairing_id, Direction::In);
        let raw = pad::read_at(&pad_path, header.offset, needed as usize)?;
        let buf: Zeroizing<Vec<u8>> = Zeroizing::new(raw);

        let mut mac_key = [0u8; 32];
        mac_key.copy_from_slice(&buf[..32]);
        let keystream = &buf[32..];

        let header_bytes = header.encode();
        let mut mac_input = Vec::with_capacity(header_bytes.len() + ciphertext.len());
        mac_input.extend_from_slice(&header_bytes);
        mac_input.extend_from_slice(ciphertext);
        let verify_res = verify_mac(&mac_key, &mac_input, mac_tag);
        mac_key.zeroize();
        mac_input.zeroize();
        verify_res?;

        let mut plaintext = Zeroizing::new(ciphertext.to_vec());
        xor_in_place(&mut plaintext, keystream);

        let new_cursor = end;
        let zero_from = snapshot.in_cursor;
        {
            let p = st.find_mut(&pairing_id)?;
            p.in_cursor = new_cursor;
            p.last_seq_in = header.seq;
        }
        self.persist(&st)?;
        let zero_len = new_cursor.saturating_sub(zero_from);
        if let Err(e) = pad::zeroize_range(&pad_path, zero_from, zero_len) {
            eprintln!("warning: on-disk zeroize of in.pad failed: {}", e);
        }

        Ok(DecryptedMessage {
            pairing_id,
            seq: header.seq,
            timestamp_ms: header.timestamp_ms,
            plaintext,
        })
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[allow(dead_code)]
fn generate_pad(path: &Path, size: u64) -> Result<()> {
    generate_pad_multi(&[path], size)
}

fn generate_pad_multi(paths: &[&Path], size: u64) -> Result<()> {
    let mut files: Vec<File> = paths
        .iter()
        .map(File::create)
        .collect::<std::io::Result<Vec<_>>>()?;
    const CHUNK: usize = 1024 * 1024;
    let mut buf = vec![0u8; CHUNK];
    let mut written: u64 = 0;
    while written < size {
        let n = std::cmp::min(size - written, CHUNK as u64) as usize;
        OsRng.fill_bytes(&mut buf[..n]);
        for f in &mut files {
            f.write_all(&buf[..n])?;
        }
        written += n as u64;
    }
    for f in &mut files {
        f.sync_all()?;
    }
    buf.zeroize();
    Ok(())
}

#[allow(dead_code)]
fn write_pad(path: &Path, data: &[u8]) -> Result<()> {
    let mut f = File::create(path)?;
    f.write_all(data)?;
    f.sync_all()?;
    Ok(())
}

fn wipe_and_remove(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let len = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if len > 0 {
        let _ = pad::zeroize_range(path, 0, len);
    }
    std::fs::remove_file(path)?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct SidecarV1 {
    schema_version: u32,
    pairing_id: Uuid,
    created_at_ms: u64,
    #[serde(default)]
    originator_hint: String,
}

pub const SIDECAR_SCHEMA_VERSION: u32 = 1;

fn sanitize_for_path(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .take(32)
        .collect()
}

fn short_id(id: &Uuid) -> String {
    id.to_string().chars().take(8).collect()
}

fn pairing_export_dir_name(name: &str, id: &Uuid) -> String {
    let n = sanitize_for_path(name);
    if n.is_empty() {
        format!("otp-pairing-{}", short_id(id))
    } else {
        format!("otp-pairing-{}-{}", n, short_id(id))
    }
}
