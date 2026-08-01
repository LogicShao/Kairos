//! 每日任务提醒：为每个设置了提醒时间的每日任务，在对应时刻发系统通知。
//!
//! 仿 `ai/scheduler.rs` 的单 cancel-token 单线程循环：每轮加载所有启用提醒的
//! 每日任务，算出下一个最近的提醒时刻并 sleep 到点，再对同一时刻到期的任务
//! 逐个发通知；配置变更经 `reschedule` 重建线程。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use chrono::{Duration as ChronoDuration, FixedOffset, NaiveDateTime, NaiveTime, Utc};
use rusqlite::Connection;
use tauri::AppHandle;

use super::ids;
use super::system::show_system_notification;

type CancelToken = Arc<Mutex<Option<Arc<AtomicBool>>>>;

/// 单例 cancel token（仿 ai/scheduler）。
fn cancel_token() -> &'static CancelToken {
    static TOKEN: OnceLock<CancelToken> = OnceLock::new();
    TOKEN.get_or_init(|| Arc::new(Mutex::new(None)))
}

/// 取消当前提醒线程（置旧 token 为 true）。
fn cancel() {
    if let Ok(mut guard) = cancel_token().lock() {
        if let Some(token) = guard.take() {
            token.store(true, Ordering::SeqCst);
        }
    }
}

/// 每日任务提醒记录。
struct DailyReminder {
    id: i64,
    title: String,
    reminder_time: NaiveTime,
}

/// 读取所有启用提醒的每日任务（未删除、is_daily=1、reminder_time 非空）。
fn load_reminders(conn: &Connection) -> Vec<DailyReminder> {
    let Ok(mut stmt) = conn.prepare(
        "SELECT id, title, reminder_time FROM tasks
         WHERE is_daily = 1 AND reminder_time IS NOT NULL AND reminder_time != '' AND deleted_at IS NULL",
    ) else {
        return Vec::new();
    };
    let rows = match stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    }) {
        Ok(rows) => rows,
        Err(_) => return Vec::new(),
    };

    rows.filter_map(|r| r.ok())
        .filter_map(|(id, title, time)| {
            NaiveTime::parse_from_str(&time, "%H:%M").ok().map(|reminder_time| {
                DailyReminder {
                    id,
                    title,
                    reminder_time,
                }
            })
        })
        .collect()
}

/// 某提醒时刻的下一次到达时间：今天未过则今天该时刻，已过则明天该时刻。
fn next_occurrence_dt(now: NaiveDateTime, target: NaiveTime) -> NaiveDateTime {
    let today_target = now.date().and_time(target);
    if today_target >= now {
        today_target
    } else {
        today_target + ChronoDuration::days(1)
    }
}

/// 计算下一批到期提醒。返回 (等待秒数, 应通知的提醒)。
/// 无任何提醒时等待 1 小时后重试，避免空转忙轮询。
fn plan_next(reminders: &[DailyReminder], now: NaiveDateTime) -> (u64, Vec<DailyReminder>) {
    if reminders.is_empty() {
        return (3600, Vec::new());
    }

    let next_dt = reminders
        .iter()
        .map(|r| next_occurrence_dt(now, r.reminder_time))
        .min()
        .expect("reminders non-empty");

    let wait = (next_dt - now).num_seconds().max(0) as u64;
    let due = if wait == 0 {
        reminders
            .iter()
            .filter(|r| next_occurrence_dt(now, r.reminder_time) == next_dt)
            .map(|r| DailyReminder {
                id: r.id,
                title: r.title.clone(),
                reminder_time: r.reminder_time,
            })
            .collect()
    } else {
        Vec::new()
    };
    (wait, due)
}

/// 确保提醒线程存在；配置变更后调用以重建（cancel + restart）。
pub fn ensure_scheduled(db: Arc<Mutex<Connection>>, app_handle: AppHandle) {
    cancel();
    let token = Arc::new(AtomicBool::new(false));
    if let Ok(mut guard) = cancel_token().lock() {
        *guard = Some(token.clone());
    } else {
        return;
    }

    thread::spawn(move || loop {
        if token.load(Ordering::SeqCst) {
            break;
        }

        let (wait, due) = {
            let conn = match db.lock() {
                Ok(conn) => conn,
                Err(_) => {
                    thread::sleep(Duration::from_secs(60));
                    continue;
                }
            };
            let reminders = load_reminders(&conn);
            let now = china_now();
            plan_next(&reminders, now)
        };

        thread::sleep(Duration::from_secs(wait));
        if token.load(Ordering::SeqCst) {
            break;
        }

        for reminder in due {
            if token.load(Ordering::SeqCst) {
                break;
            }
            let body = format!("「{}」—— 别忘了今天完成", reminder.title);
            show_system_notification(
                &app_handle,
                ids::stable_id(&format!("daily-task:{}", reminder.id)),
                "每日任务提醒",
                &body,
            );
        }
    });
}

/// 任务/提醒配置变更后重建提醒线程。
pub fn reschedule(db: Arc<Mutex<Connection>>, app_handle: &AppHandle) {
    ensure_scheduled(db, app_handle.clone());
}

/// +08:00 当前 naive datetime。
fn china_now() -> NaiveDateTime {
    let offset = FixedOffset::east_opt(8 * 3600).expect("china utc offset");
    Utc::now().with_timezone(&offset).naive_local()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn reminder(id: i64, title: &str, time: &str) -> DailyReminder {
        DailyReminder {
            id,
            title: title.to_string(),
            reminder_time: NaiveTime::parse_from_str(time, "%H:%M").unwrap(),
        }
    }

    #[test]
    fn test_next_occurrence_dt_same_day_and_next_day() {
        let now = NaiveDate::from_ymd_opt(2026, 8, 1)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap();
        // 10:00 → 今天 10:00
        let t = NaiveTime::parse_from_str("10:00", "%H:%M").unwrap();
        assert_eq!(
            next_occurrence_dt(now, t),
            now.date().and_hms_opt(10, 0, 0).unwrap()
        );
        // 08:00 → 明天 08:00
        let t2 = NaiveTime::parse_from_str("08:00", "%H:%M").unwrap();
        assert_eq!(
            next_occurrence_dt(now, t2),
            now.date().and_hms_opt(8, 0, 0).unwrap() + ChronoDuration::days(1)
        );
    }

    #[test]
    fn test_plan_next_single() {
        let now = NaiveDate::from_ymd_opt(2026, 8, 1)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap();
        let reminders = vec![reminder(1, "背单词", "20:00")];
        let (wait, due) = plan_next(&reminders, now);
        assert_eq!(wait, 11 * 3600);
        assert!(due.is_empty());
    }

    #[test]
    fn test_plan_next_due_at_now() {
        let now = NaiveDate::from_ymd_opt(2026, 8, 1)
            .unwrap()
            .and_hms_opt(20, 0, 0)
            .unwrap();
        let reminders = vec![reminder(1, "背单词", "20:00")];
        let (wait, due) = plan_next(&reminders, now);
        assert_eq!(wait, 0);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, 1);
    }

    #[test]
    fn test_plan_next_same_time_multiple() {
        let reminders = vec![
            reminder(1, "A", "08:00"),
            reminder(2, "B", "08:00"),
            reminder(3, "C", "09:00"),
        ];
        // 7:00 时最近是 8:00，尚未到期 → 等待 1 小时、due 为空。
        let now = NaiveDate::from_ymd_opt(2026, 8, 1)
            .unwrap()
            .and_hms_opt(7, 0, 0)
            .unwrap();
        let (wait, due) = plan_next(&reminders, now);
        assert_eq!(wait, 3600);
        assert!(due.is_empty());

        // 到点 8:00：同一时刻多个任务同时到期。
        let at_due = NaiveDate::from_ymd_opt(2026, 8, 1)
            .unwrap()
            .and_hms_opt(8, 0, 0)
            .unwrap();
        let (wait2, due2) = plan_next(&reminders, at_due);
        assert_eq!(wait2, 0);
        assert_eq!(due2.len(), 2);
    }

    #[test]
    fn test_plan_next_empty() {
        let now = NaiveDate::from_ymd_opt(2026, 8, 1)
            .unwrap()
            .and_hms_opt(7, 0, 0)
            .unwrap();
        let (wait, due) = plan_next(&[], now);
        assert_eq!(wait, 3600);
        assert!(due.is_empty());
    }
}
