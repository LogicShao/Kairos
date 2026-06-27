use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tauri::State;

use crate::db::models::{NotificationConfig, UpdateNotificationConfig};

#[tauri::command]
pub fn get_notification_config(
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<NotificationConfig, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::notifications::get_notification_config(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_notification_config(
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: tauri::AppHandle,
    req: UpdateNotificationConfig,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::notifications::update_notification_config(&conn, &req).map_err(|e| e.to_string())?;

    // If exam offsets or enabled flag may have changed, rebuild all exam notifications
    if req.exam_offsets_json.is_some() || req.enabled.is_some() {
        if let Err(e) =
            crate::notifications::exam_scheduler::schedule_exam_notifications(&conn, &app_handle)
        {
            log::error!("failed to rebuild exam notifications after config update: {e}");
        }
    }

    Ok(())
}

/// 请求系统通知权限。
/// 当前为桩实现，权限请求将在后续 Phase C/D 中接入插件。
#[tauri::command]
pub fn request_notification_permission() -> Result<(), String> {
    log::info!("Notification permission requested (stub)");
    Ok(())
}
