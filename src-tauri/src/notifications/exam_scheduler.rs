use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use super::ids;
use crate::db::models::Exam;

type CancelToken = Arc<AtomicBool>;
type CancelTokenMap = Arc<Mutex<HashMap<i32, CancelToken>>>;

// ── Global cancellation token store ────────────────────────────────────────────
//
// Key: notification ID (from ids::stable_id)
// Value: Arc<AtomicBool> — set to true when the notification is cancelled.
// The background thread checks this flag before showing the notification.

fn cancel_tokens() -> &'static CancelTokenMap {
    static TOKENS: OnceLock<CancelTokenMap> = OnceLock::new();
    TOKENS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

// ── Public helpers ────────────────────────────────────────────────────────────

/// Build a deterministic notification ID from kind, sync_id, and offset_minutes.
pub fn build_notification_id(kind: &str, sync_id: &str, offset_minutes: i64) -> i32 {
    ids::stable_id(&format!("{kind}:{sync_id}:{offset_minutes}"))
}

/// Human-readable offset description for notification body.
fn offset_description(minutes: i64) -> String {
    if minutes >= 1440 && minutes % 1440 == 0 {
        format!("{}天", minutes / 1440)
    } else if minutes >= 60 && minutes % 60 == 0 {
        format!("{}小时", minutes / 60)
    } else {
        format!("{}分钟", minutes)
    }
}

// ── Core scheduling functions ─────────────────────────────────────────────────

/// Full rebuild: cancel all existing exam notifications, then schedule for all
/// future exams according to the current notification config.
pub fn schedule_exam_notifications(
    conn: &Connection,
    app_handle: &AppHandle,
) -> Result<(), String> {
    if !super::is_available() {
        return Ok(());
    }
    // Cancel all existing timers
    cancel_all_exam_notifications()?;

    // Read config
    let config =
        crate::db::notifications::get_notification_config(conn).map_err(|e| e.to_string())?;

    if !config.enabled {
        log::info!("exam notifications disabled, skipping schedule");
        return Ok(());
    }

    let offsets: Vec<i64> = serde_json::from_str(&config.exam_offsets_json)
        .map_err(|e| format!("invalid exam_offsets_json: {e}"))?;

    // Get all non-deleted exams
    let exams = crate::db::exams::get_all_exams(conn).map_err(|e| e.to_string())?;

    let now = Utc::now();

    for exam in &exams {
        let exam_dt = match parse_exam_datetime(&exam.exam_datetime) {
            Ok(dt) => dt,
            Err(e) => {
                log::warn!(
                    "skip exam {} ({}) — bad datetime: {e}",
                    exam.sync_id,
                    exam.course_name,
                );
                continue;
            }
        };

        if exam_dt <= now {
            continue; // exam already started/passed
        }

        schedule_one_exam_inner(app_handle, exam, &offsets, exam_dt, now);
    }

    Ok(())
}

/// Schedule notifications for a single exam (called after create/update).
pub fn schedule_exam_for_one(
    conn: &Connection,
    app_handle: &AppHandle,
    exam: &Exam,
) -> Result<(), String> {
    if !super::is_available() {
        return Ok(());
    }
    let config =
        crate::db::notifications::get_notification_config(conn).map_err(|e| e.to_string())?;

    if !config.enabled {
        return Ok(());
    }

    let offsets: Vec<i64> = serde_json::from_str(&config.exam_offsets_json)
        .map_err(|e| format!("invalid exam_offsets_json: {e}"))?;

    let exam_dt = parse_exam_datetime(&exam.exam_datetime)?;
    let now = Utc::now();

    if exam_dt <= now {
        return Ok(()); // already past
    }

    schedule_one_exam_inner(app_handle, exam, &offsets, exam_dt, now);
    Ok(())
}

/// Cancel all notifications for a specific exam (called before update/delete).
///
/// We recalculate the notification IDs from the exam's sync_id plus the currently
/// configured offsets, then cancel those entries.
pub fn cancel_exam_notifications(
    conn: &Connection,
    _app_handle: &AppHandle,
    exam: &Exam,
) -> Result<(), String> {
    if !super::is_available() {
        return Ok(());
    }
    let config =
        crate::db::notifications::get_notification_config(conn).map_err(|e| e.to_string())?;

    let offsets: Vec<i64> = serde_json::from_str(&config.exam_offsets_json)
        .map_err(|e| format!("invalid exam_offsets_json: {e}"))?;

    let mut tokens = cancel_tokens().lock().unwrap();
    for &offset in &offsets {
        let id = build_notification_id("exam", &exam.sync_id, offset);
        if let Some(token) = tokens.get(&id) {
            token.store(true, Ordering::SeqCst);
        }
        tokens.remove(&id);
    }

    Ok(())
}

/// Cancel all exam notifications (used before a full rebuild).
pub fn cancel_all_exam_notifications() -> Result<(), String> {
    // 直接取消并清空全部考试 token。
    // 线程在 show 前会读取取消标记，因此这里不需要再扫描 exams/offsets 二次 remove。
    // 之前在持有 tokens 锁时再次调用 cancel_tokens().lock()，会在启动重建路径中自锁卡死。

    let mut tokens = cancel_tokens().lock().unwrap();
    for token in tokens.values() {
        token.store(true, Ordering::SeqCst);
    }
    tokens.clear();

    Ok(())
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn parse_exam_datetime(s: &str) -> Result<DateTime<Utc>, String> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| format!("invalid exam_datetime '{s}': {e}"))
}

fn schedule_one_exam_inner(
    app_handle: &AppHandle,
    exam: &Exam,
    offsets: &[i64],
    exam_dt: DateTime<Utc>,
    now: DateTime<Utc>,
) {
    for &offset in offsets {
        let scheduled_time = exam_dt - chrono::Duration::minutes(offset);
        if scheduled_time <= now {
            continue; // already in the past
        }

        let id = build_notification_id("exam", &exam.sync_id, offset);
        let app_handle = app_handle.clone();
        let exam_name = exam.course_name.clone();
        let desc = offset_description(offset);

        // Register a cancellation token
        let cancel_token = Arc::new(AtomicBool::new(false));
        cancel_tokens()
            .lock()
            .unwrap()
            .insert(id, cancel_token.clone());

        let wait_ms = (scheduled_time - Utc::now()).num_milliseconds();
        if wait_ms <= 0 {
            continue;
        }

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(wait_ms as u64));

            // Check if cancelled before showing
            if cancel_token.load(Ordering::SeqCst) {
                log::debug!("notification {id} cancelled before show");
                return;
            }

            let body = format!("考试「{exam_name}」将在 {desc} 后开始");
            if let Err(e) = app_handle
                .notification()
                .builder()
                .id(id)
                .title(&exam_name)
                .body(body)
                .show()
            {
                log::error!("failed to show exam notification {id}: {e}");
            }

            // Clean up token after showing
            cancel_tokens().lock().unwrap().remove(&id);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_notification_id_deterministic() {
        let id1 = build_notification_id("exam", "abc-123", 1440);
        let id2 = build_notification_id("exam", "abc-123", 1440);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_build_notification_id_different_offsets() {
        let id1 = build_notification_id("exam", "abc-123", 1440);
        let id2 = build_notification_id("exam", "abc-123", 60);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_offset_description() {
        assert_eq!(offset_description(1440), "1天");
        assert_eq!(offset_description(2880), "2天");
        assert_eq!(offset_description(60), "1小时");
        assert_eq!(offset_description(120), "2小时");
        assert_eq!(offset_description(30), "30分钟");
        assert_eq!(offset_description(5), "5分钟");
    }

    #[test]
    fn test_cancel_all_exam_notifications_clears_tokens_without_relocking() {
        let token_a = Arc::new(AtomicBool::new(false));
        let token_b = Arc::new(AtomicBool::new(false));
        {
            let mut tokens = cancel_tokens().lock().unwrap();
            tokens.insert(1, token_a.clone());
            tokens.insert(2, token_b.clone());
        }

        cancel_all_exam_notifications().expect("cancel all should succeed");

        assert!(token_a.load(Ordering::SeqCst));
        assert!(token_b.load(Ordering::SeqCst));
        assert!(cancel_tokens().lock().unwrap().is_empty());
    }
}
