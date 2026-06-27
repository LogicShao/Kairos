//! Sync protocol v2: 单文件 JSON 快照同步。
//!
//! 设计决策:
//! - 合并键: sync_id (跨设备 UUID)，v1 兼容回退到 SQLite id
//! - 胜负判定: LWW (Last-Writer-Wins)，比较 effective_timestamp
//! - 墓碑: deleted_at 非空即已删除，正常查询过滤 WHERE deleted_at IS NULL
//! - ETag: 条件上传 (If-Match) 防止覆盖更新，412 冲突时重试一次
//! - 重试: 仅重试一次，再次失败返回错误让用户稍后操作
//! - 串行护栏: 手动/自动同步共享 running 标志，禁止并发执行
//! - session-scoped: 自动同步仅在应用进程存活期间调度，关闭后不触发

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rusqlite::Connection;
use tauri::Emitter;

use crate::db;
use crate::sync::exporter::{SyncResult, SyncStats};
use crate::sync::webdav::{UploadError, WebDavClient};

pub mod exporter;
pub mod ids;
pub mod webdav;

/// 启动后延迟 5 秒再自动同步，避开应用初始化和数据库预热阶段。
const AUTO_SYNC_STARTUP_DELAY_SECS: u64 = 5;
/// 自动同步固定间隔 15 分钟。当前不做指数退避或用户自定义间隔。
const AUTO_SYNC_INTERVAL_SECS: u64 = 15 * 60;

/// 自动同步调度状态，由 Tauri manage 持有。
///
/// - `enabled`: 用户是否开启自动同步。false 时后台线程退出循环。
/// - `running`: 当前是否已有同步实例在执行，手动/自动共享的全局串行护栏。
/// - `worker_epoch`: 自动同步工作代次。每次关闭/重新开启都会 bump，旧线程据此自行退出。
///
/// 两个字段都是 `Arc<AtomicBool>`，外部可通过 clone Arc 在不持有 Mutex 锁的情况下访问。
/// `Mutex` 仅用于满足 Tauri State 的 `Send + Sync` 要求。
pub struct AutoSyncState {
    pub enabled: Arc<AtomicBool>,
    pub running: Arc<AtomicBool>,
    worker_epoch: Arc<AtomicU64>,
}

impl Default for AutoSyncState {
    fn default() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(false)),
            worker_epoch: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl AutoSyncState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 启动新的自动同步工作代次，并返回传给工作线程的代次号。
    pub fn start_worker(&self) -> u64 {
        self.enabled.store(true, Ordering::Relaxed);
        self.worker_epoch.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// 停止当前自动同步工作代次。即使用户快速重新开启，旧线程也会因代次失效而退出。
    pub fn stop_worker(&self) {
        self.enabled.store(false, Ordering::Relaxed);
        self.worker_epoch.fetch_add(1, Ordering::Relaxed);
    }

    pub fn worker_epoch_handle(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.worker_epoch)
    }

    #[cfg(test)]
    fn is_worker_current(&self, epoch: u64) -> bool {
        self.enabled.load(Ordering::Relaxed) && self.worker_epoch.load(Ordering::Relaxed) == epoch
    }
}

/// RAII 护栏守卫：构造时已持有 `running`，drop 时自动释放。
///
/// 使用方式:
/// ```ignore
/// let guard = SyncGuard::acquire(&state.running)?;
/// // ... 执行同步 ...
/// // guard drop 时自动调用 finish_sync
/// ```
pub struct SyncGuard {
    running: Arc<AtomicBool>,
}

impl SyncGuard {
    /// 尝试获取同步执行权。成功返回 Some(guard)，失败返回 None。
    pub fn acquire(running: &Arc<AtomicBool>) -> Option<Self> {
        if running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
        {
            Some(Self {
                running: Arc::clone(running),
            })
        } else {
            None
        }
    }
}

impl Drop for SyncGuard {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
    }
}

/// 统一同步入口：下载远端 → LWW 合并 → 条件上传 → ETag 重试 → 写库。
///
/// 手动 `sync_now` 和自动同步线程都调用此函数。
/// 调用方负责: 打开数据库连接、管理 `running` 护栏（通过 `SyncGuard` 或手动）。
/// 此函数本身不操作 AutoSyncState。
pub fn execute_sync(conn: &mut Connection) -> Result<SyncResult, String> {
    let config = db::sync::get_sync_config(conn).map_err(|e| e.to_string())?;

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
            stats = merge_remote(conn, &remote)?;
            true
        }
        None => false,
    };

    let (uploaded_exported_at, uploaded_etag, retry_stats) =
        upload_snapshot(conn, &client, remote_etag_for_upload.as_deref())
            .map(|(exported_at, etag)| (exported_at, etag, empty_sync_stats()))
            .or_else(|error| match error {
                UploadError::Conflict => retry_after_remote_conflict(conn, &client),
                UploadError::Other(message) => Err(format!("Upload failed: {}", message)),
            })?;
    add_sync_stats(&mut stats, retry_stats);

    db::sync::update_last_sync_at(conn, &uploaded_exported_at).map_err(|e| e.to_string())?;
    db::sync::update_remote_etag(conn, uploaded_etag.as_deref()).map_err(|e| e.to_string())?;

    Ok(SyncResult {
        uploaded: true,
        downloaded,
        stats,
    })
}

/// 向所有监听窗口发送同步完成事件，携带最新的 `last_sync_at`。
pub fn emit_sync_finished(app_handle: &tauri::AppHandle, last_sync_at: &str) {
    let payload = serde_json::json!({ "last_sync_at": last_sync_at });
    if let Err(e) = app_handle.emit("sync-finished", payload) {
        log::warn!("Failed to emit sync-finished event: {}", e);
    }
}

/// 启动自动同步工作线程，并绑定当前工作代次。
pub fn spawn_auto_sync_worker(
    db_path: String,
    state: &AutoSyncState,
    app_handle: tauri::AppHandle,
) {
    let enabled = Arc::clone(&state.enabled);
    let running = Arc::clone(&state.running);
    let worker_epoch = state.worker_epoch_handle();
    let current_epoch = state.start_worker();

    std::thread::spawn(move || {
        auto_sync_loop(
            db_path,
            enabled,
            running,
            worker_epoch,
            current_epoch,
            app_handle,
        );
    });
}

/// 自动同步后台循环（session-scoped: 进程存活期间有效）。
///
/// 职责: 定时调度，调用统一同步入口。不持有全局锁，不操作前端 state。
/// 通过 `enabled` flag 接收停止信号；通过 `running` flag (`SyncGuard`) 与手动同步互斥。
pub fn auto_sync_loop(
    db_path: String,
    enabled: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    worker_epoch: Arc<AtomicU64>,
    current_epoch: u64,
    app_handle: tauri::AppHandle,
) {
    // 启动延迟，避免与 app 初始化资源竞争；秒级检查确保停用/重启能尽快生效。
    if !wait_for_next_cycle(
        AUTO_SYNC_STARTUP_DELAY_SECS,
        &enabled,
        &worker_epoch,
        current_epoch,
    ) {
        return;
    }

    loop {
        if !worker_should_continue(&enabled, &worker_epoch, current_epoch) {
            break;
        }

        // 通过 SyncGuard 获取串行护栏（RAII: drop 时自动释放）
        if let Some(_guard) = SyncGuard::acquire(&running) {
            match db::get_connection(&db_path) {
                Ok(mut conn) => {
                    match execute_sync(&mut conn) {
                        Ok(_result) => {
                            // 读取持久化后的 last_sync_at 通知前端
                            if let Ok(cfg) = db::sync::get_sync_config(&conn) {
                                if let Some(ref ts) = cfg.last_sync_at {
                                    emit_sync_finished(&app_handle, ts);
                                }
                            }
                            log::info!("Auto-sync completed successfully");
                        }
                        Err(e) => {
                            log::warn!("Auto-sync failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Auto-sync failed to open database: {}", e);
                }
            }
            // _guard drop → running.store(false)
        } else {
            log::info!("Auto-sync skipped: sync already in progress");
        }

        // 从完成后开始等待 15 分钟，期间周期性检查停用/重启信号。
        if !wait_for_next_cycle(
            AUTO_SYNC_INTERVAL_SECS,
            &enabled,
            &worker_epoch,
            current_epoch,
        ) {
            break;
        }
    }
}

// ─── 同步辅助函数（模块内可见）─────────────────────────────────────

/// 只有“开关仍为 true 且工作代次未失效”时，当前 worker 才允许继续运行。
fn worker_should_continue(
    enabled: &Arc<AtomicBool>,
    worker_epoch: &Arc<AtomicU64>,
    current_epoch: u64,
) -> bool {
    enabled.load(Ordering::Relaxed) && worker_epoch.load(Ordering::Relaxed) == current_epoch
}

/// 秒级 sleep + 代次检查，让停用/重启自动同步时无需等满整个周期。
fn wait_for_next_cycle(
    seconds: u64,
    enabled: &Arc<AtomicBool>,
    worker_epoch: &Arc<AtomicU64>,
    current_epoch: u64,
) -> bool {
    for _ in 0..seconds {
        if !worker_should_continue(enabled, worker_epoch, current_epoch) {
            return false;
        }
        std::thread::sleep(Duration::from_secs(1));
    }

    worker_should_continue(enabled, worker_epoch, current_epoch)
}

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

fn download_remote(client: &WebDavClient) -> Result<Option<webdav::DownloadedSyncData>, String> {
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

fn merge_remote(
    conn: &mut Connection,
    remote: &webdav::DownloadedSyncData,
) -> Result<SyncStats, String> {
    exporter::import_all(conn, &remote.data).map_err(|e| e.to_string())
}

fn upload_snapshot(
    conn: &mut Connection,
    client: &WebDavClient,
    remote_etag: Option<&str>,
) -> Result<(String, Option<String>), UploadError> {
    let data = exporter::export_all(conn).map_err(|e| UploadError::Other(e.to_string()))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_sync_state_invalidates_old_worker_on_restart() {
        let state = AutoSyncState::new();

        let first_epoch = state.start_worker();
        assert!(state.is_worker_current(first_epoch));

        state.stop_worker();
        assert!(!state.is_worker_current(first_epoch));

        let second_epoch = state.start_worker();
        assert!(second_epoch > first_epoch);
        assert!(state.is_worker_current(second_epoch));
        assert!(!state.is_worker_current(first_epoch));
    }

    #[test]
    fn test_sync_guard_prevents_parallel_syncs() {
        let state = AutoSyncState::new();

        let guard = SyncGuard::acquire(&state.running).expect("first guard should acquire");
        assert!(SyncGuard::acquire(&state.running).is_none());

        drop(guard);
        assert!(SyncGuard::acquire(&state.running).is_some());
    }
}
