use std::io;

use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("pairing not found: {0}")]
    PairingNotFound(Uuid),
    #[error("pairing already exists: {0}")]
    PairingExists(Uuid),
    #[error("pad exhausted: needed {needed} bytes, {available} available")]
    PadExhausted { needed: u64, available: u64 },
    #[error("frame too short: {0} bytes")]
    FrameTooShort(usize),
    #[error("unsupported frame version: {0}")]
    UnsupportedVersion(u8),
    #[error("mac verification failed")]
    MacFailed,
    #[error("replay: seq {got} <= last accepted {last}")]
    Replay { got: u64, last: u64 },
    #[error("offset out of bounds: {offset} + {length} > {pad_size}")]
    OffsetOutOfBounds {
        offset: u64,
        length: u64,
        pad_size: u64,
    },
    #[error("invalid plaintext length: {0}")]
    InvalidPlaintextLen(usize),
    #[error("toml: {0}")]
    Toml(String),
    #[error("invalid state: {0}")]
    InvalidState(&'static str),
}

pub type Result<T> = std::result::Result<T, VaultError>;

impl From<toml::de::Error> for VaultError {
    fn from(e: toml::de::Error) -> Self {
        Self::Toml(e.to_string())
    }
}

impl From<toml::ser::Error> for VaultError {
    fn from(e: toml::ser::Error) -> Self {
        Self::Toml(e.to_string())
    }
}
