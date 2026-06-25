use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tauri::State;

use crate::db::models::SyncConfig;
use crate::sync::exporter::{SyncResult, SyncStats};
use crate::sync::webdav::{DownloadedSyncData, UploadError, WebDavClient};

fn empty_sync_stats() -> SyncStats {
    SyncStats {
        tasks_merged: 0,
        courses_merged: 0,
        exams_merged: 0,
        sessions_merged: 0,
        conflicts: 0,
    }
}

fn add_sync_stats(target: &mut SyncStats, incoming: SyncStats) {
    target.tasks_merged += incoming.tasks_merged;
    target.courses_merged += incoming.courses_merged;
    target.exams_merged += incoming.exams_merged;
    target.sessions_merged += incoming.sessions_merged;
    target.conflicts += incoming.conflicts;
}

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

/// 核心同步流程:
/// 1. 下载远端快照 + ETag → 合并到本地（LWW）
/// 2. 导出本地合并后的快照 → 条件上传（If-Match: etag）
/// 3. 若上传返回 412 → 重新下载 + 合并 + 上传（仅重试一次）
/// 4. 成功后记录 last_sync_at 和新 ETag
#[tauri::command]
pub fn sync_now(db: State<'_, Arc<Mutex<Connection>>>) -> Result<SyncResult, String> {
    let mut conn = db.lock().map_err(|e| e.to_string())?;
    let config = crate::db::sync::get_sync_config(&conn).map_err(|e| e.to_string())?;

    if config.server_url.is_empty() {
        return Err("Server URL not configured".to_string());
    }

    let client = WebDavClient::new(
        config.server_url.clone(),
        config.username.clone(),
        config.password.clone(),
    )?;

    let mut stats = empty_sync_stats();
    let mut remote_etag_for_upload: Option<String> = None;

    let downloaded = match download_remote(&client)? {
        Some(remote) => {
            remote_etag_for_upload = remote.etag.clone();
            stats = merge_remote(&mut conn, &remote)?;
            true
        }
        None => false,
    };

    let (uploaded_exported_at, uploaded_etag, retry_stats) =
        upload_snapshot(&mut conn, &client, remote_etag_for_upload.as_deref())
            .map(|(exported_at, etag)| (exported_at, etag, empty_sync_stats()))
            .or_else(|error| match error {
                UploadError::Conflict => retry_after_remote_conflict(&mut conn, &client),
                UploadError::Other(message) => Err(format!("Upload failed: {}", message)),
            })?;
    add_sync_stats(&mut stats, retry_stats);

    crate::db::sync::update_last_sync_at(&conn, &uploaded_exported_at)
        .map_err(|e| e.to_string())?;
    crate::db::sync::update_remote_etag(&conn, uploaded_etag.as_deref())
        .map_err(|e| e.to_string())?;

    Ok(SyncResult {
        uploaded: true,
        downloaded,
        stats,
    })
}

fn download_remote(client: &WebDavClient) -> Result<Option<DownloadedSyncData>, String> {
    match client.download() {
        Ok(remote) => Ok(Some(remote)),
        Err(e) => {
            if e.contains("404") {
                log::info!("No remote data found, will upload local only");
                Ok(None)
            } else {
                Err(format!("Download failed: {}", e))
            }
        }
    }
}

fn merge_remote(conn: &mut Connection, remote: &DownloadedSyncData) -> Result<SyncStats, String> {
    crate::sync::exporter::import_all(conn, &remote.data).map_err(|e| e.to_string())
}

fn upload_snapshot(
    conn: &mut Connection,
    client: &WebDavClient,
    remote_etag: Option<&str>,
) -> Result<(String, Option<String>), UploadError> {
    let data =
        crate::sync::exporter::export_all(conn).map_err(|e| UploadError::Other(e.to_string()))?;
    let exported_at = data.exported_at.clone();
    let etag = client.upload(&data, remote_etag)?;
    Ok((exported_at, etag))
}

/// ETag 冲突后的重试流程（仅执行一次）:
/// 重新下载远端（获取新 ETag）→ 合并到本地（LWW）→ 条件上传。
/// 再次 412 → 放弃并返回错误（避免无限重试）。
fn retry_after_remote_conflict(
    conn: &mut Connection,
    client: &WebDavClient,
) -> Result<(String, Option<String>, SyncStats), String> {
    log::warn!("Remote sync data changed during upload; retrying once");
    let remote = client
        .download()
        .map_err(|e| format!("Download failed after upload conflict: {}", e))?;
    let retry_stats = merge_remote(conn, &remote)?;
    let (exported_at, etag) =
        upload_snapshot(conn, client, remote.etag.as_deref()).map_err(|error| match error {
            UploadError::Conflict => "Upload failed: remote changed again during retry".to_string(),
            UploadError::Other(message) => format!("Upload failed: {}", message),
        })?;
    Ok((exported_at, etag, retry_stats))
}
