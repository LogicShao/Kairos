#[cfg(windows)]
use std::sync::OnceLock;

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

#[cfg(windows)]
/// Windows toast sender identity. Must match `tauri.conf.json` identifier.
const WINDOWS_APP_ID: &str = "com.kairos.app";
#[cfg(windows)]
const WINDOWS_APP_NAME: &str = "Kairos";

pub fn show_system_notification(app_handle: &AppHandle, id: i32, title: &str, body: &str) {
    #[cfg(windows)]
    {
        if let Err(e) = show_windows_notification(title, body) {
            log::warn!("failed to show Windows notification with Kairos app id: {e}");
            show_tauri_notification(app_handle, id, title, body);
        }
    }

    #[cfg(not(windows))]
    {
        show_tauri_notification(app_handle, id, title, body);
    }
}

fn show_tauri_notification(app_handle: &AppHandle, id: i32, title: &str, body: &str) {
    if let Err(e) = app_handle
        .notification()
        .builder()
        .id(id)
        .title(title)
        .body(body)
        .show()
    {
        log::error!("failed to show system notification: {e}");
    }
}

#[cfg(windows)]
fn show_windows_notification(title: &str, body: &str) -> Result<(), String> {
    ensure_windows_app_id_registered()?;

    tauri_winrt_notification::Toast::new(WINDOWS_APP_ID)
        .title(title)
        .text1(body)
        .show()
        .map_err(|e| e.to_string())
}

#[cfg(windows)]
fn ensure_windows_app_id_registered() -> Result<(), String> {
    static REGISTERED: OnceLock<Result<(), String>> = OnceLock::new();

    REGISTERED.get_or_init(register_windows_app_id).clone()
}

#[cfg(windows)]
fn register_windows_app_id() -> Result<(), String> {
    use windows_registry::CURRENT_USER;

    let icon_path = std::env::current_exe()
        .map_err(|e| format!("failed to resolve current exe for notification icon: {e}"))?;
    let icon_uri = windows_registry::HSTRING::from(icon_path.to_string_lossy().as_ref());

    let key = CURRENT_USER
        .create(format!(r"SOFTWARE\Classes\AppUserModelId\{WINDOWS_APP_ID}"))
        .map_err(|e| format!("failed to create AppUserModelId registry key: {e}"))?;

    key.set_string("DisplayName", WINDOWS_APP_NAME)
        .map_err(|e| format!("failed to set notification display name: {e}"))?;
    key.set_string("IconBackgroundColor", "0")
        .map_err(|e| format!("failed to set notification icon background: {e}"))?;
    key.set_hstring("IconUri", &icon_uri)
        .map_err(|e| format!("failed to set notification icon uri: {e}"))?;

    Ok(())
}
