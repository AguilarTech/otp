use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use super::drive::DriveClient;
use super::Transport;
use crate::vault::{Vault, VaultError};

const TAG_TEXT: u8 = 0x01;
const TAG_FILE: u8 = 0x02;

#[derive(Debug, Clone, Serialize)]
pub struct InboxMessageEvent {
    pub pairing_id: String,
    pub seq: u64,
    pub timestamp_ms: u64,
    pub kind: &'static str,
    pub text: Option<String>,
    pub file_name: Option<String>,
    pub file_size: Option<u64>,
    pub file_bytes_b64: Option<String>,
}

pub async fn run(
    vault: Arc<Vault>,
    transport: Arc<Transport>,
    app: AppHandle,
    interval: Duration,
) {
    let drive = DriveClient::new(transport.clone());
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;
        if !transport.status().connected {
            continue;
        }
        let pairings = match vault.list_pairings() {
            Ok(ps) => ps,
            Err(_) => continue,
        };
        for p in pairings {
            if p.drive_folder_id.is_empty() {
                continue;
            }
            if let Err(e) = poll_pairing(&drive, &vault, &app, &p.id, &p.drive_folder_id).await {
                eprintln!(
                    "warning: poll failed for pairing {} ({}): {}",
                    p.name, p.id, e
                );
            }
        }
    }
}

async fn poll_pairing(
    drive: &DriveClient,
    vault: &Vault,
    app: &AppHandle,
    pairing_id: &Uuid,
    folder_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let files = drive.list_folder(folder_id).await?;
    for file in files {
        let data = match drive.download(&file.id).await {
            Ok(d) => d,
            Err(e) => {
                eprintln!("download failed for {}: {}", file.id, e);
                continue;
            }
        };
        match vault.decrypt(&data) {
            Ok(msg) => {
                if &msg.pairing_id != pairing_id {
                    eprintln!(
                        "warning: file {} in folder for pairing {} decrypted to pairing {}",
                        file.id, pairing_id, msg.pairing_id
                    );
                    continue;
                }
                match build_event(&msg.pairing_id, msg.seq, msg.timestamp_ms, &msg.plaintext) {
                    Some(event) => {
                        let _ = app.emit("inbox-message", event);
                        let _ = drive.delete(&file.id).await;
                    }
                    None => {
                        eprintln!(
                            "warning: could not parse decrypted plaintext in {}; leaving file in place",
                            file.id
                        );
                    }
                }
            }
            Err(VaultError::Replay { .. }) => {
                let _ = drive.delete(&file.id).await;
            }
            Err(e) => {
                eprintln!(
                    "warning: decrypt failed for {} ({}): {}; leaving file in place",
                    file.id, file.name, e
                );
            }
        }
    }
    Ok(())
}

fn build_event(
    pairing_id: &Uuid,
    seq: u64,
    timestamp_ms: u64,
    plaintext: &[u8],
) -> Option<InboxMessageEvent> {
    match plaintext.first() {
        Some(&TAG_TEXT) => {
            let text = std::str::from_utf8(&plaintext[1..]).ok()?.to_string();
            Some(InboxMessageEvent {
                pairing_id: pairing_id.to_string(),
                seq,
                timestamp_ms,
                kind: "text",
                text: Some(text),
                file_name: None,
                file_size: None,
                file_bytes_b64: None,
            })
        }
        Some(&TAG_FILE) => {
            if plaintext.len() < 3 {
                return None;
            }
            let name_len = u16::from_be_bytes([plaintext[1], plaintext[2]]) as usize;
            let header_end = 3 + name_len;
            if plaintext.len() < header_end {
                return None;
            }
            let file_name = std::str::from_utf8(&plaintext[3..header_end])
                .ok()?
                .to_string();
            let content = &plaintext[header_end..];
            Some(InboxMessageEvent {
                pairing_id: pairing_id.to_string(),
                seq,
                timestamp_ms,
                kind: "file",
                text: None,
                file_name: Some(file_name),
                file_size: Some(content.len() as u64),
                file_bytes_b64: Some(
                    base64::engine::general_purpose::STANDARD.encode(content),
                ),
            })
        }
        _ => {
            // Legacy untagged text from earlier builds.
            let text = std::str::from_utf8(plaintext).ok()?.to_string();
            Some(InboxMessageEvent {
                pairing_id: pairing_id.to_string(),
                seq,
                timestamp_ms,
                kind: "text",
                text: Some(text),
                file_name: None,
                file_size: None,
                file_bytes_b64: None,
            })
        }
    }
}
