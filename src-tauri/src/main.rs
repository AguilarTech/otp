#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod transport;
mod vault;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use serde::Serialize;
use tauri::{Manager, State};
use uuid::Uuid;

use transport::{ConnectionStatus, DriveClient, Transport};
use vault::{PairingInfo, Vault};

const POLL_INTERVAL: Duration = Duration::from_secs(30);
const DRIVE_FOLDER_NAME_PREFIX: &str = "otp-msgr";
const MAX_ATTACHMENT_BYTES: u64 = 50 * 1024 * 1024;

// Plaintext-payload type tags. The first byte of every encrypted plaintext
// identifies what comes after; 0x00 and bytes outside this set are treated
// as legacy raw UTF-8 text (pre-tagging) so old test messages still decode.
const TAG_TEXT: u8 = 0x01;
const TAG_FILE: u8 = 0x02;

struct AppState {
    vault: Arc<Vault>,
    transport: Arc<Transport>,
}

#[derive(Serialize)]
struct DecryptResultDto {
    pairing_id: String,
    seq: u64,
    timestamp_ms: u64,
    kind: &'static str,
    text: Option<String>,
    file_name: Option<String>,
    file_size: Option<u64>,
    file_bytes_b64: Option<String>,
}

#[derive(Serialize)]
struct SendResultDto {
    frame: String,
    uploaded_file_id: Option<String>,
    pad_consumed: u64,
}

#[tauri::command]
fn list_pairings(state: State<'_, AppState>) -> Result<Vec<PairingInfo>, String> {
    state.vault.list_pairings().map_err(|e| e.to_string())
}

#[tauri::command]
fn create_pairing(
    state: State<'_, AppState>,
    name: String,
    originator_hint: String,
    pad_size_bytes: u64,
    usb_dir: String,
) -> Result<PairingInfo, String> {
    state
        .vault
        .create_and_export_pairing(name, originator_hint, pad_size_bytes, &PathBuf::from(usb_dir))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn import_pairing(
    state: State<'_, AppState>,
    name: String,
    usb_pairing_dir: String,
) -> Result<PairingInfo, String> {
    state
        .vault
        .import_pairing_from_usb(name, &PathBuf::from(usb_pairing_dir))
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn send_text_message(
    state: State<'_, AppState>,
    pairing_id: String,
    plaintext: String,
) -> Result<SendResultDto, String> {
    let id = Uuid::parse_str(&pairing_id).map_err(|e| e.to_string())?;
    let mut payload = Vec::with_capacity(1 + plaintext.len());
    payload.push(TAG_TEXT);
    payload.extend_from_slice(plaintext.as_bytes());
    encrypt_and_optionally_upload(&state, &id, &payload).await
}

#[tauri::command]
async fn send_file_attachment(
    state: State<'_, AppState>,
    pairing_id: String,
    file_path: String,
) -> Result<SendResultDto, String> {
    let id = Uuid::parse_str(&pairing_id).map_err(|e| e.to_string())?;
    let path = PathBuf::from(file_path);
    let metadata = std::fs::metadata(&path).map_err(|e| format!("cannot read file: {}", e))?;
    if metadata.len() > MAX_ATTACHMENT_BYTES {
        return Err(format!(
            "attachment is too large ({:.1} MB). The current limit is {} MB so a single message can't drain your whole pad.",
            metadata.len() as f64 / (1024.0 * 1024.0),
            MAX_ATTACHMENT_BYTES / (1024 * 1024),
        ));
    }
    let file_bytes = std::fs::read(&path).map_err(|e| format!("cannot read file: {}", e))?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("attachment")
        .to_string();
    if name.len() > u16::MAX as usize {
        return Err("filename is unreasonably long".into());
    }

    let mut payload = Vec::with_capacity(3 + name.len() + file_bytes.len());
    payload.push(TAG_FILE);
    payload.extend_from_slice(&(name.len() as u16).to_be_bytes());
    payload.extend_from_slice(name.as_bytes());
    payload.extend_from_slice(&file_bytes);
    encrypt_and_optionally_upload(&state, &id, &payload).await
}

async fn encrypt_and_optionally_upload(
    state: &State<'_, AppState>,
    pairing_id: &Uuid,
    plaintext_payload: &[u8],
) -> Result<SendResultDto, String> {
    let frame_bytes = state
        .vault
        .encrypt(pairing_id, plaintext_payload)
        .map_err(|e| e.to_string())?;
    let frame_b64 = base64::engine::general_purpose::STANDARD.encode(&frame_bytes);

    let folder_id = state
        .vault
        .list_pairings()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|p| &p.id == pairing_id)
        .map(|p| p.drive_folder_id)
        .unwrap_or_default();

    let mut uploaded_file_id = None;
    if !folder_id.is_empty() && state.transport.status().connected {
        let drive = DriveClient::new(state.transport.clone());
        let blob_name = format!("{}.bin", Uuid::new_v4());
        match drive.upload(&folder_id, &blob_name, &frame_bytes).await {
            Ok(file_id) => uploaded_file_id = Some(file_id),
            Err(e) => {
                return Err(format!(
                    "encrypted and pad consumed, but Drive upload failed: {}. \
                     Save or copy the encrypted message as a fallback.",
                    e
                ));
            }
        }
    }

    Ok(SendResultDto {
        frame: frame_b64,
        uploaded_file_id,
        pad_consumed: 32 + plaintext_payload.len() as u64,
    })
}

#[tauri::command]
fn decrypt_message(
    state: State<'_, AppState>,
    frame_b64: String,
) -> Result<DecryptResultDto, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(frame_b64.trim())
        .map_err(|e| format!("invalid base64: {}", e))?;
    let msg = state.vault.decrypt(&bytes).map_err(|e| e.to_string())?;
    build_decrypt_dto(&msg.pairing_id, msg.seq, msg.timestamp_ms, &msg.plaintext)
}

fn build_decrypt_dto(
    pairing_id: &Uuid,
    seq: u64,
    timestamp_ms: u64,
    plaintext: &[u8],
) -> Result<DecryptResultDto, String> {
    let pairing_id_s = pairing_id.to_string();
    match plaintext.first() {
        Some(&TAG_TEXT) => {
            let text = std::str::from_utf8(&plaintext[1..])
                .map_err(|_| "decrypted text is not valid UTF-8".to_string())?
                .to_string();
            Ok(DecryptResultDto {
                pairing_id: pairing_id_s,
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
                return Err("decrypted file header is truncated".into());
            }
            let name_len =
                u16::from_be_bytes([plaintext[1], plaintext[2]]) as usize;
            let header_end = 3 + name_len;
            if plaintext.len() < header_end {
                return Err("decrypted filename is truncated".into());
            }
            let file_name = std::str::from_utf8(&plaintext[3..header_end])
                .map_err(|_| "filename is not valid UTF-8".to_string())?
                .to_string();
            let content = &plaintext[header_end..];
            Ok(DecryptResultDto {
                pairing_id: pairing_id_s,
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
            // Legacy: untagged plaintext from earlier builds. Treat as text.
            let text = std::str::from_utf8(plaintext)
                .map_err(|_| "decrypted message is not valid UTF-8".to_string())?
                .to_string();
            Ok(DecryptResultDto {
                pairing_id: pairing_id_s,
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

#[tauri::command]
fn write_file_text(path: String, contents: String) -> Result<(), String> {
    std::fs::write(&path, contents.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_file_bytes(path: String, contents_b64: String) -> Result<(), String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(contents_b64)
        .map_err(|e| format!("invalid base64: {}", e))?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())
}

#[tauri::command]
fn usb_free_space(path: String) -> Result<u64, String> {
    available_space(&PathBuf::from(path)).map_err(|e| e.to_string())
}

#[tauri::command]
fn oauth_status(state: State<'_, AppState>) -> ConnectionStatus {
    state.transport.status()
}

#[tauri::command]
async fn oauth_connect(state: State<'_, AppState>) -> Result<(), String> {
    state.transport.connect().await.map_err(|e| e.to_string())
}

#[tauri::command]
fn oauth_disconnect(state: State<'_, AppState>) -> Result<(), String> {
    state.transport.disconnect().map_err(|e| e.to_string())
}

#[tauri::command]
async fn drive_create_folder(
    state: State<'_, AppState>,
    pairing_id: String,
    peer_email: String,
) -> Result<String, String> {
    let id = Uuid::parse_str(&pairing_id).map_err(|e| e.to_string())?;
    let drive = DriveClient::new(state.transport.clone());
    let folder_name = format!("{}-{}", DRIVE_FOLDER_NAME_PREFIX, Uuid::new_v4());
    let folder_id = drive
        .create_folder(&folder_name)
        .await
        .map_err(|e| e.to_string())?;
    if !peer_email.trim().is_empty() {
        if let Err(e) = drive
            .share_folder(&folder_id, peer_email.trim(), "writer")
            .await
        {
            let _ = drive.delete(&folder_id).await;
            return Err(format!("created folder but share failed: {}", e));
        }
    }
    state
        .vault
        .set_drive_folder_id(&id, folder_id.clone())
        .map_err(|e| e.to_string())?;
    Ok(folder_id)
}

#[tauri::command]
async fn drive_bind_folder(
    state: State<'_, AppState>,
    pairing_id: String,
    folder_id: String,
) -> Result<(), String> {
    let id = Uuid::parse_str(&pairing_id).map_err(|e| e.to_string())?;
    let trimmed = folder_id.trim().to_string();
    if trimmed.is_empty() {
        return Err("folder id is empty".into());
    }
    let drive = DriveClient::new(state.transport.clone());
    drive
        .get_folder(&trimmed)
        .await
        .map_err(|e| format!("could not access folder via drive.file scope: {}", e))?;
    state
        .vault
        .set_drive_folder_id(&id, trimmed)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn drive_pick_folder(
    state: State<'_, AppState>,
    pairing_id: String,
) -> Result<String, String> {
    let id = Uuid::parse_str(&pairing_id).map_err(|e| e.to_string())?;
    let api_key = transport::api_key()
        .ok_or_else(|| transport::TransportError::PickerNotConfigured.to_string())?;
    let app_id = transport::app_id().unwrap_or_default();
    let access_token = state
        .transport
        .access_token()
        .await
        .map_err(|e| e.to_string())?;

    let folder_id = transport::picker::run_picker_flow(transport::picker::PickerInputs {
        access_token: &access_token,
        api_key,
        app_id: &app_id,
    })
    .await
    .map_err(|e| e.to_string())?;

    let drive = DriveClient::new(state.transport.clone());
    drive
        .get_folder(&folder_id)
        .await
        .map_err(|e| format!("picker returned a folder but drive.file cannot see it: {}", e))?;

    state
        .vault
        .set_drive_folder_id(&id, folder_id.clone())
        .map_err(|e| e.to_string())?;
    Ok(folder_id)
}

#[tauri::command]
fn drive_unbind_folder(
    state: State<'_, AppState>,
    pairing_id: String,
) -> Result<(), String> {
    let id = Uuid::parse_str(&pairing_id).map_err(|e| e.to_string())?;
    state
        .vault
        .clear_drive_folder_id(&id)
        .map_err(|e| e.to_string())
}

#[cfg(unix)]
#[allow(clippy::unnecessary_cast)] // f_bavail/f_frsize widths differ across libc targets
fn available_space(path: &Path) -> std::io::Result<u64> {
    use std::os::unix::ffi::OsStrExt;
    let cstr = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let mut buf: libc::statvfs = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::statvfs(cstr.as_ptr(), &mut buf) };
    if ret != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok((buf.f_bavail as u64).saturating_mul(buf.f_frsize as u64))
}

#[cfg(not(unix))]
fn available_space(_path: &Path) -> std::io::Result<u64> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "available-space detection is not implemented on this platform; enter pad size manually",
    ))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let base = app
                .path()
                .app_data_dir()
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            let vault = Arc::new(
                Vault::open(base).map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?,
            );
            let transport = Arc::new(Transport::new());

            let v = Arc::clone(&vault);
            let t = Arc::clone(&transport);
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                transport::poller::run(v, t, handle, POLL_INTERVAL).await;
            });

            app.manage(AppState { vault, transport });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_pairings,
            create_pairing,
            import_pairing,
            send_text_message,
            send_file_attachment,
            decrypt_message,
            write_file_text,
            write_file_bytes,
            usb_free_space,
            oauth_status,
            oauth_connect,
            oauth_disconnect,
            drive_create_folder,
            drive_bind_folder,
            drive_pick_folder,
            drive_unbind_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
