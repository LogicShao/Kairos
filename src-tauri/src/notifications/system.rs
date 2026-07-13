#[cfg(windows)]
use std::path::PathBuf;
#[cfg(windows)]
use std::sync::OnceLock;

#[cfg(windows)]
use tauri::path::BaseDirectory;
use tauri::AppHandle;
#[cfg(windows)]
use tauri::Manager;
use tauri_plugin_notification::{NotificationExt, PermissionState};

#[cfg(windows)]
/// Windows toast sender identity. Must match `tauri.conf.json` identifier.
const WINDOWS_APP_ID: &str = "com.kairos.app";
#[cfg(windows)]
const WINDOWS_APP_NAME: &str = "Kairos";
#[cfg(windows)]
const WINDOWS_NOTIFICATION_ICON: &str = "Square44x44Logo.png";

pub fn show_system_notification(app_handle: &AppHandle, id: i32, title: &str, body: &str) {
    #[cfg(windows)]
    {
        if let Err(e) = show_windows_notification(app_handle, title, body) {
            log::warn!("failed to show Windows notification with Kairos app id: {e}");
            show_tauri_notification(app_handle, id, title, body);
        }
    }

    #[cfg(not(windows))]
    {
        show_tauri_notification(app_handle, id, title, body);
    }
}

pub fn request_system_notification_permission(
    app_handle: &AppHandle,
) -> Result<PermissionState, String> {
    if !super::is_available() {
        return Err("通知插件不可用".to_string());
    }

    app_handle
        .notification()
        .request_permission()
        .map_err(|e| e.to_string())
}

pub fn permission_state_key(state: PermissionState) -> &'static str {
    match state {
        PermissionState::Granted => "granted",
        PermissionState::Denied => "denied",
        PermissionState::Prompt => "prompt",
        PermissionState::PromptWithRationale => "prompt-with-rationale",
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
fn show_windows_notification(
    app_handle: &AppHandle,
    title: &str,
    body: &str,
) -> Result<(), String> {
    let icon_path = windows_notification_icon_path(app_handle)?;
    ensure_windows_app_id_registered(&icon_path)?;

    tauri_winrt_notification::Toast::new(WINDOWS_APP_ID)
        .title(title)
        .text1(body)
        .icon(
            &icon_path,
            tauri_winrt_notification::IconCrop::Square,
            WINDOWS_APP_NAME,
        )
        .show()
        .map_err(|e| e.to_string())
}

#[cfg(windows)]
fn ensure_windows_app_id_registered(icon_path: &std::path::Path) -> Result<(), String> {
    static REGISTERED: OnceLock<Result<(), String>> = OnceLock::new();

    REGISTERED
        .get_or_init(|| register_windows_app_id(icon_path))
        .clone()
}

#[cfg(windows)]
fn register_windows_app_id(icon_path: &std::path::Path) -> Result<(), String> {
    use windows_registry::CURRENT_USER;

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

#[cfg(windows)]
fn windows_notification_icon_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let mut candidates = Vec::new();

    if let Ok(path) = app_handle
        .path()
        .resolve(WINDOWS_NOTIFICATION_ICON, BaseDirectory::Resource)
    {
        candidates.push(path);
    }

    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        candidates.push(resource_dir.join("icons").join(WINDOWS_NOTIFICATION_ICON));
    }

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join("icons").join(WINDOWS_NOTIFICATION_ICON));
        candidates.push(
            current_dir
                .join("src-tauri")
                .join("icons")
                .join(WINDOWS_NOTIFICATION_ICON),
        );
    }

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            format!("failed to find Windows notification icon {WINDOWS_NOTIFICATION_ICON}")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_state_key_matches_plugin_values() {
        assert_eq!(permission_state_key(PermissionState::Granted), "granted");
        assert_eq!(permission_state_key(PermissionState::Denied), "denied");
        assert_eq!(permission_state_key(PermissionState::Prompt), "prompt");
        assert_eq!(
            permission_state_key(PermissionState::PromptWithRationale),
            "prompt-with-rationale"
        );
    }
}
