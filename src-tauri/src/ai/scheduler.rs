//! 7:00 定时触发（+08:00）。仿 `pomodoro_scheduler` 的单 cancel-token 模式。
//!
//! 仅在 AI 已启用且有 key 时调度；配置变更经 `reschedule` 重建。
//! Android 跟随进程存活（与现有考试通知同级），不承诺系统级准点。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use chrono::{Duration as ChronoDuration, FixedOffset, NaiveTime, Utc};
use rusqlite::Connection;
use tauri::AppHandle;

use crate::timer::PomodoroEngine;

type CancelToken = Arc<Mutex<Option<Arc<AtomicBool>>>>;

/// 单例 cancel token（仿 pomodoro_scheduler）。
fn cancel_token() -> &'static CancelToken {
    static TOKEN: OnceLock<CancelToken> = OnceLock::new();
    TOKEN.get_or_init(|| Arc::new(Mutex::new(None)))
}

/// 取消当前调度线程（置旧 token 为 true）。
fn cancel() {
    if let Ok(mut guard) = cancel_token().lock() {
        if let Some(token) = guard.take() {
            token.store(true, Ordering::SeqCst);
        }
    }
}

/// 距 +08:00 下一 07:00 的时长。
fn wait_until_next_seven_am() -> Duration {
    let offset = FixedOffset::east_opt(8 * 3600).expect("china utc offset");
    let china_now = Utc::now().with_timezone(&offset);
    let seven = NaiveTime::from_hms_opt(7, 0, 0).expect("07:00 is valid");

    let target_naive = if china_now.time() < seven {
        china_now
            .date_naive()
            .and_hms_opt(7, 0, 0)
            .expect("valid hms")
    } else {
        (china_now.date_naive() + ChronoDuration::days(1))
            .and_hms_opt(7, 0, 0)
            .expect("valid hms")
    };
    // China 本地 = UTC + 8h，故 UTC 目标 = China 本地 naive - 8h。
    let target_utc = target_naive.and_utc() - ChronoDuration::hours(8);
    let wait_secs = (target_utc - Utc::now()).num_seconds().max(0);
    Duration::from_secs(wait_secs as u64)
}

/// 确保调度线程存在：仅当 AI 已启用且有 key 时启动；否则取消既有线程。
pub fn ensure_scheduled(
    db: Arc<Mutex<Connection>>,
    engine: Arc<Mutex<PomodoroEngine>>,
    app_handle: AppHandle,
) {
    let enabled_and_configured = match db.lock() {
        Ok(conn) => crate::db::ai::get_ai_config(&conn)
            .map(|c| c.enabled && !c.api_key_encrypted.is_empty())
            .unwrap_or(false),
        Err(_) => false,
    };
    if !enabled_and_configured {
        cancel();
        return;
    }

    cancel();
    let token = Arc::new(AtomicBool::new(false));
    if let Ok(mut guard) = cancel_token().lock() {
        *guard = Some(token.clone());
    } else {
        return;
    }

    thread::spawn(move || loop {
        let wait = wait_until_next_seven_am();
        thread::sleep(wait);
        if token.load(Ordering::SeqCst) {
            break;
        }

        match crate::ai::morning_brief::generate_today_brief(&db, &engine, &app_handle, false) {
            Ok(brief) => {
                let body: String = brief.markdown.chars().take(60).collect();
                crate::notifications::system::show_system_notification(
                    &app_handle,
                    crate::notifications::ids::stable_id("ai:morning-brief"),
                    "今日 AI 摘要已生成",
                    &body,
                );
            }
            Err(e) => {
                log::warn!("7:00 AI 摘要生成失败: {e}");
            }
        }
    });
}

/// 配置变更后重建调度线程：取消旧线程 + 按需重启。
pub fn reschedule(
    db: Arc<Mutex<Connection>>,
    engine: Arc<Mutex<PomodoroEngine>>,
    app_handle: AppHandle,
) -> Result<(), String> {
    ensure_scheduled(db, engine, app_handle);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    #[test]
    fn test_wait_until_next_seven_am_positive() {
        let wait = wait_until_next_seven_am();
        assert!(wait.as_secs() > 0);
        // 距离下一 07:00 最多 24 小时 + 余量。
        assert!(wait.as_secs() < 25 * 3600);
    }

    #[test]
    fn test_next_seven_am_boundary() {
        // 07:00:00 整应算今天（time() < 07:00 为 false → 明天），06:59:59 算今天。
        let offset = FixedOffset::east_opt(8 * 3600).unwrap();
        let before = DateTime::parse_from_rfc3339("2026-07-31T06:59:59+08:00")
            .unwrap()
            .with_timezone(&Utc);
        let after = DateTime::parse_from_rfc3339("2026-07-31T07:00:00+08:00")
            .unwrap()
            .with_timezone(&Utc);

        let china = |utc: &DateTime<Utc>| utc.with_timezone(&offset);

        let before_naive = china(&before).date_naive().and_hms_opt(7, 0, 0).unwrap();
        let before_wait = (before_naive.and_utc() - ChronoDuration::hours(8) - before)
            .num_seconds()
            .max(0);
        // 06:59:59 → 今天 07:00，差 1 秒。
        assert_eq!(before_wait, 1);

        let after_naive = (china(&after).date_naive() + ChronoDuration::days(1))
            .and_hms_opt(7, 0, 0)
            .unwrap();
        let after_wait = (after_naive.and_utc() - ChronoDuration::hours(8) - after)
            .num_seconds()
            .max(0);
        // 07:00:00 → 明天 07:00，差 24 小时。
        assert_eq!(after_wait, 24 * 3600);
    }
}
