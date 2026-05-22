use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransportError {
    #[error("not connected to Google Drive")]
    NotConnected,
    #[error("OAuth client not configured at build time — see CLOUD_SETUP.md")]
    ClientNotConfigured,
    #[error("OAuth flow error: {0}")]
    OAuth(String),
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("Drive API {status}: {body}")]
    DriveApi { status: u16, body: String },
    #[error("keyring: {0}")]
    Keyring(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, TransportError>;

impl From<reqwest::Error> for TransportError {
    fn from(e: reqwest::Error) -> Self {
        Self::Http(e.to_string())
    }
}

impl From<keyring::Error> for TransportError {
    fn from(e: keyring::Error) -> Self {
        Self::Keyring(e.to_string())
    }
}

impl From<url::ParseError> for TransportError {
    fn from(e: url::ParseError) -> Self {
        Self::OAuth(format!("invalid url: {}", e))
    }
}
