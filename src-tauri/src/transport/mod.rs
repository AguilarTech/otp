pub mod drive;
pub mod error;
pub mod oauth;
pub mod picker;
pub mod poller;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use keyring::Entry;
use serde::{Deserialize, Serialize};

pub use drive::DriveClient;
pub use error::{Result, TransportError};

const KEYRING_SERVICE: &str = "com.aguilartech.otp";
const KEYRING_USER: &str = "google_drive_refresh_token";

/// Build-time embedding of the Google Cloud credentials. CLOUD_SETUP.md
/// covers the console steps. If the env vars are unset at build time,
/// transport commands surface `ClientNotConfigured` (OAuth) or
/// `PickerNotConfigured` (Picker) and the manual paste flow remains
/// the only usable transport.
const CLIENT_ID: Option<&str> = option_env!("OTP_GOOGLE_CLIENT_ID");
const CLIENT_SECRET: Option<&str> = option_env!("OTP_GOOGLE_CLIENT_SECRET");
const API_KEY: Option<&str> = option_env!("OTP_GOOGLE_API_KEY");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub client_configured: bool,
    pub picker_configured: bool,
    pub connected: bool,
}

pub fn api_key() -> Option<&'static str> {
    API_KEY
}

/// Picker's `appId` is the numeric Cloud project number. For Desktop
/// OAuth clients the client_id starts with `<project_number>-` so we
/// extract it instead of demanding a separate env var.
pub fn app_id() -> Option<String> {
    CLIENT_ID
        .and_then(|id| id.split('-').next())
        .filter(|s| s.chars().all(|c| c.is_ascii_digit()))
        .map(|s| s.to_string())
}

#[derive(Clone)]
struct CachedAccessToken {
    token: String,
    expires_at: Instant,
}

struct TransportState {
    refresh_token: Option<String>,
    access_token: Option<CachedAccessToken>,
}

pub struct Transport {
    state: Mutex<TransportState>,
    http: reqwest::Client,
}

impl Transport {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .build()
            .expect("reqwest client construction");
        let refresh_token = load_refresh_token_from_keyring().ok();
        Self {
            state: Mutex::new(TransportState {
                refresh_token,
                access_token: None,
            }),
            http,
        }
    }

    pub fn status(&self) -> ConnectionStatus {
        ConnectionStatus {
            client_configured: CLIENT_ID.is_some(),
            picker_configured: API_KEY.is_some(),
            connected: self
                .state
                .lock()
                .unwrap()
                .refresh_token
                .as_ref()
                .map(|t| !t.is_empty())
                .unwrap_or(false),
        }
    }

    pub async fn connect(&self) -> Result<()> {
        let (client_id, client_secret) = ensure_client_configured()?;
        let tokens = oauth::run_loopback_flow(client_id, client_secret).await?;
        store_refresh_token_in_keyring(&tokens.refresh_token)?;
        let mut st = self.state.lock().unwrap();
        st.refresh_token = Some(tokens.refresh_token);
        st.access_token = Some(CachedAccessToken {
            token: tokens.access_token,
            expires_at: Instant::now()
                + Duration::from_secs(tokens.expires_in_secs.saturating_sub(60).max(60)),
        });
        Ok(())
    }

    pub fn disconnect(&self) -> Result<()> {
        let _ = clear_refresh_token_from_keyring();
        let mut st = self.state.lock().unwrap();
        st.refresh_token = None;
        st.access_token = None;
        Ok(())
    }

    pub async fn access_token(&self) -> Result<String> {
        // Fast path under std mutex (no await).
        {
            let st = self.state.lock().unwrap();
            if let Some(at) = &st.access_token {
                if at.expires_at > Instant::now() + Duration::from_secs(30) {
                    return Ok(at.token.clone());
                }
            }
        }
        let refresh_token = {
            let st = self.state.lock().unwrap();
            st.refresh_token
                .clone()
                .ok_or(TransportError::NotConnected)?
        };
        let (client_id, client_secret) = ensure_client_configured()?;
        let new_token = self
            .refresh_access_token(client_id, client_secret, &refresh_token)
            .await?;
        {
            let mut st = self.state.lock().unwrap();
            st.access_token = Some(new_token.clone());
        }
        Ok(new_token.token)
    }

    async fn refresh_access_token(
        &self,
        client_id: &str,
        client_secret: Option<&str>,
        refresh_token: &str,
    ) -> Result<CachedAccessToken> {
        #[derive(Deserialize)]
        struct RefreshResponse {
            access_token: String,
            expires_in: u64,
        }
        let mut params: Vec<(&str, &str)> = vec![
            ("client_id", client_id),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];
        if let Some(secret) = client_secret {
            params.push(("client_secret", secret));
        }
        let resp = self
            .http
            .post(oauth::TOKEN_URL)
            .form(&params)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            // 400 invalid_grant on revoked refresh token → clear state.
            if status == 400 {
                let mut st = self.state.lock().unwrap();
                st.refresh_token = None;
                st.access_token = None;
                let _ = clear_refresh_token_from_keyring();
            }
            return Err(TransportError::DriveApi { status, body });
        }
        let parsed: RefreshResponse = resp.json().await?;
        Ok(CachedAccessToken {
            token: parsed.access_token,
            expires_at: Instant::now()
                + Duration::from_secs(parsed.expires_in.saturating_sub(60).max(60)),
        })
    }
}

fn ensure_client_configured() -> Result<(&'static str, Option<&'static str>)> {
    let id = CLIENT_ID.ok_or(TransportError::ClientNotConfigured)?;
    Ok((id, CLIENT_SECRET))
}

fn load_refresh_token_from_keyring() -> Result<String> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    Ok(entry.get_password()?)
}

fn store_refresh_token_in_keyring(token: &str) -> Result<()> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    entry.set_password(token)?;
    Ok(())
}

fn clear_refresh_token_from_keyring() -> Result<()> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    match entry.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(TransportError::from(e)),
    }
}
