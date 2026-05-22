use super::error::{Result, VaultError};

pub const FRAME_VERSION: u8 = 1;
pub const HEADER_LEN: usize = 1 + 16 + 8 + 8 + 4 + 8;
pub const MAC_LEN: usize = 16;
pub const POLY_KEY_LEN: u64 = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub version: u8,
    pub pairing_id: [u8; 16],
    pub seq: u64,
    pub offset: u64,
    pub length: u32,
    pub timestamp_ms: u64,
}

impl Header {
    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut buf = [0u8; HEADER_LEN];
        buf[0] = self.version;
        buf[1..17].copy_from_slice(&self.pairing_id);
        buf[17..25].copy_from_slice(&self.seq.to_be_bytes());
        buf[25..33].copy_from_slice(&self.offset.to_be_bytes());
        buf[33..37].copy_from_slice(&self.length.to_be_bytes());
        buf[37..45].copy_from_slice(&self.timestamp_ms.to_be_bytes());
        buf
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_LEN {
            return Err(VaultError::FrameTooShort(bytes.len()));
        }
        let version = bytes[0];
        if version != FRAME_VERSION {
            return Err(VaultError::UnsupportedVersion(version));
        }
        let mut pairing_id = [0u8; 16];
        pairing_id.copy_from_slice(&bytes[1..17]);
        let seq = u64::from_be_bytes(bytes[17..25].try_into().unwrap());
        let offset = u64::from_be_bytes(bytes[25..33].try_into().unwrap());
        let length = u32::from_be_bytes(bytes[33..37].try_into().unwrap());
        let timestamp_ms = u64::from_be_bytes(bytes[37..45].try_into().unwrap());
        Ok(Self {
            version,
            pairing_id,
            seq,
            offset,
            length,
            timestamp_ms,
        })
    }
}

pub fn parse_frame(frame: &[u8]) -> Result<(Header, &[u8], &[u8; MAC_LEN])> {
    let header = Header::decode(frame)?;
    let ct_start = HEADER_LEN;
    let ct_end = ct_start
        .checked_add(header.length as usize)
        .ok_or(VaultError::FrameTooShort(frame.len()))?;
    let frame_end = ct_end
        .checked_add(MAC_LEN)
        .ok_or(VaultError::FrameTooShort(frame.len()))?;
    if frame.len() < frame_end {
        return Err(VaultError::FrameTooShort(frame.len()));
    }
    let ciphertext = &frame[ct_start..ct_end];
    let mac_slice = &frame[ct_end..ct_end + MAC_LEN];
    let mac: &[u8; MAC_LEN] = mac_slice.try_into().unwrap();
    Ok((header, ciphertext, mac))
}
