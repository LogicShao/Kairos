use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tauri::{Manager, State};

use crate::db::models::SyncConfig;
use crate::sync::exporter::SyncResult;
use crate::sync::{self, AutoSyncState, SyncGuard};

#[tauri::command]
pub fn get_sync_config(db: State<'_, Arc<Mutex<Connection>>>) -> Result<SyncConfig, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::sync::get_sync_config(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_sync_config(
    db: State<'_, Arc<Mutex<Connection>>>,
    sync_state: State<'_, Arc<Mutex<AutoSyncState>>>,
    app_handle: tauri::AppHandle,
    config: SyncConfig,
) -> Result<(), String> {
    // 先写数据库（持有 DB 锁，不持有 sync_state 锁）
    let prev_auto_sync = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let prev = crate::db::sync::get_sync_config(&conn).map_err(|e| e.to_string())?;
        crate::db::sync::update_sync_config(&conn, &config).map_err(|e| e.to_string())?;
        prev.auto_sync && !prev.server_url.is_empty()
    };

    // 短暂锁 sync_state，只做 AtomicBool 读写 + 线程启动
    let now_should_run = config.auto_sync && !config.server_url.is_empty();
    {
        let state = sync_state.lock().map_err(|e| e.to_string())?;

        if !prev_auto_sync && now_should_run {
            let db_path = app_handle
                .path()
                .app_data_dir()
                .map_err(|e| e.to_string())?
                .join("kairos.db");
            let db_path_str = db_path.to_str().ok_or("invalid db path")?.to_string();
            sync::spawn_auto_sync_worker(db_path_str, &state, app_handle.clone());
        } else if prev_auto_sync && !now_should_run {
            // 从可运行 -> 不可运行：关闭自动同步
            state.stop_worker();
        }
    }

    Ok(())
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

/// 手动同步命令。
///
/// 复用统一同步入口 `sync::execute_sync`，通过 `SyncGuard` 与自动同步共享 `running` 护栏。
/// 若已有同步进行中则返回可恢复错误。
/// `sync_state` 锁只在开始时短暂持有以 clone `running` Arc，不在网络 I/O 期间持有。
#[tauri::command]
pub fn sync_now(
    db: State<'_, Arc<Mutex<Connection>>>,
    sync_state: State<'_, Arc<Mutex<AutoSyncState>>>,
    app_handle: tauri::AppHandle,
) -> Result<SyncResult, String> {
    // 短暂锁 sync_state，只用于 clone running Arc
    let running = {
        let state = sync_state.lock().map_err(|e| e.to_string())?;
        state.running.clone()
    };
    // 锁已释放，后续网络 I/O 不会阻塞 update_sync_config

    // RAII 护栏：guard drop 时自动释放 running
    let _guard =
        SyncGuard::acquire(&running).ok_or_else(|| "Sync already in progress".to_string())?;

    let result = {
        let mut conn = db.lock().map_err(|e| e.to_string())?;
        sync::execute_sync(&mut conn)
    };
    // _guard drop → running.store(false, Release)

    match &result {
        Ok(_sync_result) => {
            // 读取持久化后的 last_sync_at 并通过事件通知前端
            let conn = db.lock().map_err(|e| e.to_string())?;
            if let Ok(cfg) = crate::db::sync::get_sync_config(&conn) {
                if let Some(ref ts) = cfg.last_sync_at {
                    sync::emit_sync_finished(&app_handle, ts);
                }
            }
        }
        Err(e) => {
            log::warn!("Manual sync failed: {}", e);
        }
    }

    result
}
