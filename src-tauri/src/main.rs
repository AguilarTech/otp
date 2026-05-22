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

struct AppState {
    vault: Arc<Vault>,
    transport: Arc<Transport>,
}

#[derive(Serialize)]
struct DecryptResultDto {
    pairing_id: String,
    seq: u64,
    timestamp_ms: u64,
    plaintext: String,
}

#[derive(Serialize)]
struct SendResultDto {
    /// Always present — opaque base64 frame.
    frame: String,
    /// When Drive is connected and the pairing has a folder bound, the
    /// frame is uploaded automatically and this is the Drive file id.
    uploaded_file_id: Option<String>,
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
async fn send_message(
    state: State<'_, AppState>,
    pairing_id: String,
    plaintext: String,
) -> Result<SendResultDto, String> {
    let id = Uuid::parse_str(&pairing_id).map_err(|e| e.to_string())?;
    let frame_bytes = state
        .vault
        .encrypt(&id, plaintext.as_bytes())
        .map_err(|e| e.to_string())?;
    let frame_b64 = base64::engine::general_purpose::STANDARD.encode(&frame_bytes);

    // Look up folder binding (snapshot).
    let folder_id = state
        .vault
        .list_pairings()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|p| p.id == id)
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
                    "frame encrypted and pad consumed, but Drive upload failed: {}. \
                     Copy the frame manually as a fallback.",
                    e
                ));
            }
        }
    }

    Ok(SendResultDto {
        frame: frame_b64,
        uploaded_file_id,
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
    let plaintext = std::str::from_utf8(&msg.plaintext)
        .map_err(|_| "decrypted message is not valid UTF-8".to_string())?
        .to_string();
    Ok(DecryptResultDto {
        pairing_id: msg.pairing_id.to_string(),
        seq: msg.seq,
        timestamp_ms: msg.timestamp_ms,
        plaintext,
    })
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

    // Verify drive.file scope now sees the folder (Picker should have
    // granted it). If get fails, something's off — surface to the user
    // before persisting the binding.
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
            send_message,
            decrypt_message,
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
