#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod vault;

use std::path::{Path, PathBuf};

use base64::Engine;
use serde::Serialize;
use tauri::{Manager, State};
use uuid::Uuid;

use vault::{PairingInfo, Vault};

struct AppState {
    vault: Vault,
}

#[derive(Serialize)]
struct DecryptResultDto {
    pairing_id: String,
    seq: u64,
    timestamp_ms: u64,
    plaintext: String,
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
fn encrypt_message(
    state: State<'_, AppState>,
    pairing_id: String,
    plaintext: String,
) -> Result<String, String> {
    let id = Uuid::parse_str(&pairing_id).map_err(|e| e.to_string())?;
    let frame = state
        .vault
        .encrypt(&id, plaintext.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&frame))
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

#[cfg(unix)]
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
            let vault =
                Vault::open(base).map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            app.manage(AppState { vault });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_pairings,
            create_pairing,
            import_pairing,
            encrypt_message,
            decrypt_message,
            usb_free_space,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
