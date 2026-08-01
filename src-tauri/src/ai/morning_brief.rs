//! 今日摘要生成编排：按日缓存 → in-flight 护栏 → 窄锁快照 → AI/规则降级 → double-check upsert。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use chrono::FixedOffset;
use rusqlite::Connection;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager};

use crate::ai::deepseek::StreamChunk;
use crate::ai::prompt;
use crate::ai::{resolve_service, AIService, AiBriefResult, BriefSource};
use crate::db::models::AiMorningBrief;
use crate::timer::PomodoroEngine;

/// 模块级 in-flight 护栏：手动触发与 7:00 自动触发并发时只发一次请求，防双计费。
fn inflight() -> &'static Arc<AtomicBool> {
    static FLAG: OnceLock<Arc<AtomicBool>> = OnceLock::new();
    FLAG.get_or_init(|| Arc::new(AtomicBool::new(false)))
}

/// 生成进行中自动释放护栏（正常返回与 `?` 提前返回都会触发 Drop）。
struct InFlightGuard(&'static Arc<AtomicBool>);
impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// +08:00 今日日期 YYYY-MM-DD。
pub fn today_china() -> String {
    let offset = FixedOffset::east_opt(8 * 3600).expect("china utc offset");
    chrono::Utc::now()
        .with_timezone(&offset)
        .format("%Y-%m-%d")
        .to_string()
}

/// 生成并持久化今日摘要（AI 或规则降级），成功后广播 `ai-brief-generated` 事件。
///
/// `force=false` 时今日已有缓存直接返回（零成本）；`force=true` 强制重新调用。
/// 供手动生成命令与 7:00 定时线程共用同一入口。
pub fn generate_today_brief(
    db: &Arc<Mutex<Connection>>,
    engine: &Arc<Mutex<PomodoroEngine>>,
    app_handle: &AppHandle,
    force: bool,
) -> Result<AiMorningBrief, String> {
    let date = today_china();

    // 1. 非强制时先查今日缓存。
    if !force {
        if let Some(brief) = {
            let conn = db.lock().map_err(|e| e.to_string())?;
            crate::db::ai::get_morning_brief(&conn, &date).map_err(|e| e.to_string())?
        } {
            return Ok(brief);
        }
    }

    // 2. in-flight 护栏：抢不到则重查缓存，仍无则提示生成中。
    let flag = inflight();
    if flag.swap(true, Ordering::AcqRel) {
        let cached = {
            let conn = db.lock().map_err(|e| e.to_string())?;
            crate::db::ai::get_morning_brief(&conn, &date).map_err(|e| e.to_string())?
        };
        return match cached {
            Some(brief) => Ok(brief),
            None => Err("正在生成今日摘要，请稍候".to_string()),
        };
    }
    let _guard = InFlightGuard(flag);

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("解析应用数据目录失败: {e}"))?;

    // 3. 窄锁快照：聚合数据 + 读配置 + 读 key 文件。释放锁后再发网络请求。
    let (briefing, config, key) = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let eng = engine.lock().map_err(|e| e.to_string())?;
        let briefing = crate::commands::briefing::collect_today_briefing(&conn, &eng)?;
        let config = crate::db::ai::get_ai_config(&conn).map_err(|e| e.to_string())?;
        let key = crate::ai::crypto::load_key(&app_data_dir)?;
        (briefing, config, key)
    };

    // 4. 分发：AI 成功且通过结构校验 → 用 AI 结果；否则降级本地规则。
    let rule = crate::ai::rule::RuleBasedService;
    let result: AiBriefResult = match resolve_service(&config, &key) {
        Some(service) => match service.generate_daily_brief(&briefing) {
            Ok(ai_result) if prompt::validate_ai_output(&ai_result.summary) => ai_result,
            Ok(summary) => {
                log::warn!(
                    "AI 摘要结构校验失败（len={}，开头120字符: {}），降级本地规则",
                    summary.summary.chars().count(),
                    summary.summary.chars().take(120).collect::<String>()
                );
                rule.generate_daily_brief(&briefing)
                    .map_err(|e| e.to_string())?
            }
            Err(e) => {
                log::warn!("AI 摘要生成失败，降级本地规则: {e:?}");
                rule.generate_daily_brief(&briefing)
                    .map_err(|e| e.to_string())?
            }
        },
        None => rule
            .generate_daily_brief(&briefing)
            .map_err(|e| e.to_string())?,
    };

    let source = match result.source {
        BriefSource::Ai => "ai",
        BriefSource::Rule => "rule",
    }
    .to_string();
    let brief = AiMorningBrief {
        id: 0,
        date: date.clone(),
        markdown: result.summary,
        source,
        model: result.model.unwrap_or_default(),
        generated_at: result.generated_at,
        created_at: String::new(),
        updated_at: String::new(),
    };

    // 5. double-check upsert（仍在 in-flight 护栏保护下，杜绝并发重复写入）。
    let saved = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        crate::db::ai::upsert_morning_brief(&conn, &brief).map_err(|e| e.to_string())?
    };

    // 6. 广播事件：7:00 自动生成时若 Today 页打开则自动刷新。
    let _ = app_handle.emit("ai-brief-generated", &saved);

    Ok(saved)
}

/// 流式生成并持久化今日摘要（手动触发专用）。
///
/// 与 `generate_today_brief` 共用今日缓存、in-flight 护栏与降级编排，但 AI 调用
/// 走 SSE 流式：每段 delta 经 `channel` 实时推送到前端。结构校验失败或网络错误
/// 仍降级规则引擎，此时返回值为规则结果（前端以返回值覆盖流式预览）。
pub async fn generate_today_brief_streaming(
    db: &Arc<Mutex<Connection>>,
    engine: &Arc<Mutex<PomodoroEngine>>,
    app_handle: &AppHandle,
    force: bool,
    channel: &Channel<StreamChunk>,
) -> Result<AiMorningBrief, String> {
    let date = today_china();

    // 1. 非强制时先查今日缓存。
    if !force {
        if let Some(brief) = {
            let conn = db.lock().map_err(|e| e.to_string())?;
            crate::db::ai::get_morning_brief(&conn, &date).map_err(|e| e.to_string())?
        } {
            return Ok(brief);
        }
    }

    // 2. in-flight 护栏：抢不到则重查缓存，仍无则提示生成中。
    let flag = inflight();
    if flag.swap(true, Ordering::AcqRel) {
        let cached = {
            let conn = db.lock().map_err(|e| e.to_string())?;
            crate::db::ai::get_morning_brief(&conn, &date).map_err(|e| e.to_string())?
        };
        return match cached {
            Some(brief) => Ok(brief),
            None => Err("正在生成今日摘要，请稍候".to_string()),
        };
    }
    let _guard = InFlightGuard(flag);

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("解析应用数据目录失败: {e}"))?;

    // 3. 窄锁快照：聚合数据 + 读配置 + 读 key 文件。释放锁后再发网络请求（流式路径用 spawn_blocking 卸载）。
    let db_clone = db.clone();
    let engine_clone = engine.clone();
    let (briefing, config, key) = tauri::async_runtime::spawn_blocking(move || {
        let conn = db_clone.lock().map_err(|e| e.to_string())?;
        let eng = engine_clone.lock().map_err(|e| e.to_string())?;
        let briefing = crate::commands::briefing::collect_today_briefing(&conn, &eng)?;
        let config = crate::db::ai::get_ai_config(&conn).map_err(|e| e.to_string())?;
        let key = crate::ai::crypto::load_key(&app_data_dir)?;
        Ok::<_, String>((briefing, config, key))
    })
    .await
    .map_err(|e| format!("spawn_blocking 连接池已关闭: {e}"))??;

    // 4. 分发：AI 流式成功且通过结构校验 → 用 AI 结果；否则降级本地规则（流式路径）。
    let rule = crate::ai::rule::RuleBasedService;
    let result: AiBriefResult = match crate::ai::resolve_streaming_provider(&config, &key) {
        Some(provider) => {
            match provider
                .generate_daily_brief_streaming(&briefing, channel)
                .await
            {
                Ok(summary) if prompt::validate_ai_output(&summary) => AiBriefResult {
                    summary,
                    source: BriefSource::Ai,
                    generated_at: crate::db::chrono_now(),
                    model: Some(config.model.clone()),
                },
                Ok(summary) => {
                    log::warn!(
                        "AI 流式摘要结构校验失败（len={}，开头120字符: {}），降级本地规则",
                        summary.chars().count(),
                        summary.chars().take(120).collect::<String>()
                    );
                    rule.generate_daily_brief(&briefing).map_err(|e| e.to_string())?
                }
                Err(e) => {
                    log::warn!("AI 流式生成失败，降级本地规则: {e:?}");
                    rule.generate_daily_brief(&briefing).map_err(|e| e.to_string())?
                }
            }
        }
        None => rule
            .generate_daily_brief(&briefing)
            .map_err(|e| e.to_string())?,
    };

    let source = match result.source {
        BriefSource::Ai => "ai",
        BriefSource::Rule => "rule",
    }
    .to_string();
    let brief = AiMorningBrief {
        id: 0,
        date: date.clone(),
        markdown: result.summary,
        source,
        model: result.model.unwrap_or_default(),
        generated_at: result.generated_at,
        created_at: String::new(),
        updated_at: String::new(),
    };

    // 5. double-check upsert（仍在 in-flight 护栏保护下，杜绝并发重复写入）。
    let saved = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        crate::db::ai::upsert_morning_brief(&conn, &brief).map_err(|e| e.to_string())?
    };

    // 6. 广播事件：页面打开时自动刷新为落库结果。
    let _ = app_handle.emit("ai-brief-generated", &saved);

    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_today_china_format() {
        let date = today_china();
        // YYYY-MM-DD
        assert_eq!(date.len(), 10);
        assert_eq!(date.chars().filter(|c| *c == '-').count(), 2);
        let parsed = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d");
        assert!(parsed.is_ok(), "today_china 应输出合法日期");
    }
}
