use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use super::drive::DriveClient;
use super::Transport;
use crate::vault::{Vault, VaultError};

#[derive(Debug, Clone, Serialize)]
pub struct InboxMessageEvent {
    pub pairing_id: String,
    pub seq: u64,
    pub timestamp_ms: u64,
    pub plaintext: String,
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
                let plaintext = match std::str::from_utf8(&msg.plaintext) {
                    Ok(s) => s.to_string(),
                    Err(_) => {
                        eprintln!("warning: non-UTF-8 plaintext in {}", file.id);
                        continue;
                    }
                };
                let _ = app.emit(
                    "inbox-message",
                    InboxMessageEvent {
                        pairing_id: msg.pairing_id.to_string(),
                        seq: msg.seq,
                        timestamp_ms: msg.timestamp_ms,
                        plaintext,
                    },
                );
                let _ = drive.delete(&file.id).await;
            }
            Err(VaultError::Replay { .. }) => {
                // Already accepted; safe to delete.
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
