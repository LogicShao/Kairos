use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use crate::db::models::SyncConfig;
use crate::sync::exporter::SyncResult;
use crate::sync::{self, AutoSyncState, SyncGuard};

#[derive(Debug, Clone, Serialize)]
pub struct SyncConfigView {
    pub id: i64,
    pub server_url: String,
    pub username: String,
    pub auto_sync: bool,
    pub last_sync_at: Option<String>,
    pub remote_etag: Option<String>,
    pub device_id: Option<String>,
    pub dataset_id: Option<String>,
    pub password_configured: bool,
}

impl From<SyncConfig> for SyncConfigView {
    fn from(config: SyncConfig) -> Self {
        Self {
            id: config.id,
            server_url: config.server_url,
            username: config.username,
            auto_sync: config.auto_sync,
            last_sync_at: config.last_sync_at,
            remote_etag: config.remote_etag,
            device_id: config.device_id,
            dataset_id: config.dataset_id,
            password_configured: !config.password.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSyncConfigCmd {
    pub server_url: String,
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
    pub auto_sync: bool,
}

fn app_db_path(app_handle: &tauri::AppHandle) -> Result<String, String> {
    let db_path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("kairos.db");
    db_path
        .to_str()
        .map(str::to_string)
        .ok_or_else(|| "数据库路径无效".to_string())
}

fn open_app_connection(app_handle: &tauri::AppHandle) -> Result<Connection, String> {
    let db_path = app_db_path(app_handle)?;
    crate::db::get_connection(&db_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_sync_config(db: State<'_, Arc<Mutex<Connection>>>) -> Result<SyncConfigView, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::sync::get_sync_config(&conn)
        .map(SyncConfigView::from)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_sync_config(
    db: State<'_, Arc<Mutex<Connection>>>,
    sync_state: State<'_, Arc<Mutex<AutoSyncState>>>,
    app_handle: tauri::AppHandle,
    config: UpdateSyncConfigCmd,
) -> Result<(), String> {
    // 先写数据库（持有 DB 锁，不持有 sync_state 锁）
    let (prev_auto_sync, now_should_run) = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let prev = crate::db::sync::get_sync_config(&conn).map_err(|e| e.to_string())?;
        let mut next = prev.clone();
        next.server_url = config.server_url;
        next.username = config.username;
        if let Some(password) = config.password {
            next.password = password;
        }
        next.auto_sync = config.auto_sync;

        crate::db::sync::update_sync_config(&conn, &next).map_err(|e| e.to_string())?;
        (
            prev.auto_sync && !prev.server_url.is_empty(),
            next.auto_sync && !next.server_url.is_empty(),
        )
    };

    // 短暂锁 sync_state，只做 AtomicBool 读写 + 线程启动
    {
        let state = sync_state.lock().map_err(|e| e.to_string())?;

        if !prev_auto_sync && now_should_run {
            sync::spawn_auto_sync_worker(app_db_path(&app_handle)?, &state, app_handle.clone());
        } else if prev_auto_sync && !now_should_run {
            // 从可运行 -> 不可运行：关闭自动同步
            state.stop_worker();
        }
    }

    Ok(())
}

#[tauri::command]
pub fn test_sync_connection(db: State<'_, Arc<Mutex<Connection>>>) -> Result<bool, String> {
    let config = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        crate::db::sync::get_sync_config(&conn).map_err(|e| e.to_string())?
    };

    if config.server_url.is_empty() {
        return Err("未配置服务器地址".to_string());
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
        SyncGuard::acquire(&running).ok_or_else(|| "同步正在进行中".to_string())?;

    let result = {
        let mut conn = open_app_connection(&app_handle)?;
        sync::execute_sync(&mut conn)
    };
    // _guard drop → running.store(false, Release)

    match &result {
        Ok(_sync_result) => {
            // 读取持久化后的 last_sync_at 并通过事件通知前端
            if let Ok(conn) = open_app_connection(&app_handle) {
                if let Ok(cfg) = crate::db::sync::get_sync_config(&conn) {
                    if let Some(ref ts) = cfg.last_sync_at {
                        sync::emit_sync_finished(&app_handle, ts);
                    }
                }
            }
        }
        Err(e) => {
            log::warn!("Manual sync failed: {}", e);
        }
    }

    result
}
