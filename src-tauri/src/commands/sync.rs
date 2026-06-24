use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tauri::State;

use crate::db::models::SyncConfig;
use crate::sync::exporter::{SyncResult, SyncStats};

#[tauri::command]
pub fn get_sync_config(db: State<'_, Arc<Mutex<Connection>>>) -> Result<SyncConfig, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::sync::get_sync_config(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_sync_config(
    db: State<'_, Arc<Mutex<Connection>>>,
    config: SyncConfig,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::sync::update_sync_config(&conn, &config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn test_sync_connection(db: State<'_, Arc<Mutex<Connection>>>) -> Result<bool, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let config = crate::db::sync::get_sync_config(&conn).map_err(|e| e.to_string())?;

    if config.server_url.is_empty() {
        return Err("Server URL not configured".to_string());
    }

    let client = crate::sync::webdav::WebDavClient::new(
        config.server_url,
        config.username,
        config.password,
    )?;

    client.test_connection()
}

#[tauri::command]
pub fn sync_now(db: State<'_, Arc<Mutex<Connection>>>) -> Result<SyncResult, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let config = crate::db::sync::get_sync_config(&conn).map_err(|e| e.to_string())?;

    if config.server_url.is_empty() {
        return Err("Server URL not configured".to_string());
    }

    let client = crate::sync::webdav::WebDavClient::new(
        config.server_url.clone(),
        config.username.clone(),
        config.password.clone(),
    )?;

    let mut downloaded = false;

    let remote_data = match client.download() {
        Ok(data) => {
            downloaded = true;
            Some(data)
        }
        Err(e) => {
            if e.contains("404") {
                log::info!("No remote data found, will upload local only");
                None
            } else {
                return Err(format!("Download failed: {}", e));
            }
        }
    };

    let local_data = crate::sync::exporter::export_all(&conn).map_err(|e| e.to_string())?;

    let stats = if let Some(remote) = remote_data {
        let merged_stats =
            crate::sync::exporter::import_all(&conn, &remote).map_err(|e| e.to_string())?;

        let merged_local = crate::sync::exporter::export_all(&conn).map_err(|e| e.to_string())?;

        client
            .upload(&merged_local)
            .map_err(|e| format!("Upload failed: {}", e))?;

        merged_stats
    } else {
        client
            .upload(&local_data)
            .map_err(|e| format!("Upload failed: {}", e))?;

        SyncStats {
            tasks_merged: 0,
            courses_merged: 0,
            exams_merged: 0,
            sessions_merged: 0,
            conflicts: 0,
        }
    };

    let uploaded = true;

    let now = crate::db::chrono_now();
    crate::db::sync::update_last_sync_at(&conn, &now).map_err(|e| e.to_string())?;

    Ok(SyncResult {
        uploaded,
        downloaded,
        stats,
    })
}
