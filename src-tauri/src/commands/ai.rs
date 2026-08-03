use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use crate::ai::deepseek::StreamChunk;
use crate::db::models::{AiConfigView, AiMorningBrief, UpdateAiConfigRequest};
use crate::timer::PomodoroEngine;

/// 读取 AI 配置视图。key 明文/密文均不过桥，仅返回是否已配置。
#[tauri::command]
pub fn get_ai_config(db: State<'_, Arc<Mutex<Connection>>>) -> Result<AiConfigView, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let config = crate::db::ai::get_ai_config(&conn).map_err(|e| e.to_string())?;
    Ok(AiConfigView {
        id: config.id,
        enabled: config.enabled,
        base_url: config.base_url,
        model: config.model,
        api_key_configured: !config.api_key_encrypted.is_empty(),
        sync_enabled: config.sync_enabled,
        created_at: config.created_at,
        updated_at: config.updated_at,
    })
}

/// 合并更新 AI 配置。`req.api_key` 语义：Some(非空)=加密写入；Some("")=清空；None=保留。
/// 保存成功后重建 7:00 调度线程。
#[tauri::command]
pub fn update_ai_config(
    db: State<'_, Arc<Mutex<Connection>>>,
    engine: State<'_, Arc<Mutex<PomodoroEngine>>>,
    app_handle: AppHandle,
    req: UpdateAiConfigRequest,
) -> Result<AiConfigView, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;

    let new_key_encrypted: Option<String> = match &req.api_key {
        Some(key) if !key.trim().is_empty() => {
            let key_bytes = crate::ai::crypto::ensure_key_file(&app_data_dir)?;
            Some(crate::ai::crypto::encrypt_api_key(key, &key_bytes)?)
        }
        Some(_) => Some(String::new()),
        None => None,
    };

    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        crate::db::ai::update_ai_config(&conn, &req, new_key_encrypted)
            .map_err(|e| e.to_string())?;
    }

    // 配置变更（enabled/base_url/model/key）后重建调度。
    crate::ai::scheduler::reschedule(db.inner().clone(), engine.inner().clone(), app_handle)?;

    get_ai_config(db)
}

/// 读取恢复密钥（DEK 的 hex）。未生成（从未同步过 AI 设置）时返回 `None`。
#[tauri::command]
pub fn get_ai_sync_recovery_key(
    app_handle: AppHandle,
) -> Result<Option<String>, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    match crate::sync::ai_settings::load_dek(&app_data_dir)? {
        Some(dek) => Ok(Some(crate::sync::ai_settings::recovery_key_hex(&dek))),
        None => Ok(None),
    }
}

/// 写入恢复密钥（DEK hex 转回 32 字节落盘）。用于重装设备后恢复 AI 设置同步。
#[tauri::command]
pub fn set_ai_sync_recovery_key(
    app_handle: AppHandle,
    hex_key: String,
) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let dek = crate::sync::ai_settings::recovery_key_from_hex(&hex_key)?;
    crate::sync::ai_settings::save_dek(&app_data_dir, &dek)
}

/// 读取今日摘要缓存；未生成为 `None`（前端据此显示生成按钮）。
#[tauri::command]
pub fn get_ai_morning_brief(
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<Option<AiMorningBrief>, String> {
    let date = crate::ai::morning_brief::today_china();
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::ai::get_morning_brief(&conn, &date).map_err(|e| e.to_string())
}

/// 生成今日摘要（AI 或规则降级）并持久化。`force=true` 强制重新调用。
#[tauri::command]
pub fn generate_ai_morning_brief(
    db: State<'_, Arc<Mutex<Connection>>>,
    engine: State<'_, Arc<Mutex<PomodoroEngine>>>,
    app_handle: AppHandle,
    force: bool,
) -> Result<AiMorningBrief, String> {
    crate::ai::morning_brief::generate_today_brief(db.inner(), engine.inner(), &app_handle, force)
}

/// 流式生成今日摘要（手动触发）。chunk 经 Channel 实时推送，完成后返回最终落库结果。
#[tauri::command]
pub async fn generate_ai_morning_brief_streaming(
    db: State<'_, Arc<Mutex<Connection>>>,
    engine: State<'_, Arc<Mutex<PomodoroEngine>>>,
    app_handle: AppHandle,
    channel: Channel<StreamChunk>,
    force: bool,
) -> Result<AiMorningBrief, String> {
    crate::ai::morning_brief::generate_today_brief_streaming(
        db.inner(),
        engine.inner(),
        &app_handle,
        force,
        &channel,
    )
    .await
}
