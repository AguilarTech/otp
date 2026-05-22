use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::{Result, VaultError};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub schema_version: u32,
    #[serde(default, rename = "pairing")]
    pub pairings: Vec<Pairing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pairing {
    pub id: Uuid,
    pub name: String,
    pub created_at_ms: u64,
    pub out_total: u64,
    pub out_cursor: u64,
    pub in_total: u64,
    pub in_cursor: u64,
    pub seq_out: u64,
    pub last_seq_in: u64,
    #[serde(default)]
    pub drive_folder_id: String,
}

impl State {
    pub fn empty() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            pairings: Vec::new(),
        }
    }

    pub fn load_or_init(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::empty());
        }
        let raw = std::fs::read_to_string(path)?;
        let st: State = toml::from_str(&raw)?;
        if st.schema_version != SCHEMA_VERSION {
            return Err(VaultError::InvalidState("schema_version mismatch"));
        }
        Ok(st)
    }

    pub fn find(&self, id: &Uuid) -> Result<&Pairing> {
        self.pairings
            .iter()
            .find(|p| &p.id == id)
            .ok_or(VaultError::PairingNotFound(*id))
    }

    pub fn find_mut(&mut self, id: &Uuid) -> Result<&mut Pairing> {
        self.pairings
            .iter_mut()
            .find(|p| &p.id == id)
            .ok_or(VaultError::PairingNotFound(*id))
    }
}

pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or(VaultError::InvalidState("state path has no parent"))?;
    let file_name = path
        .file_name()
        .ok_or(VaultError::InvalidState("state path has no file name"))?
        .to_string_lossy();
    let tmp = parent.join(format!(".{}.tmp", file_name));
    {
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    #[cfg(unix)]
    {
        let d = File::open(parent)?;
        let _ = d.sync_all();
    }
    Ok(())
}
