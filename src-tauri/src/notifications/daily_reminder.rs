//! 任务提醒：每日任务（reminder_time）与普通任务一次性提醒（remind_at）。
//!
//! 仿 `ai/scheduler.rs` 的单 cancel-token 单线程循环：每轮加载所有启用提醒的
//! 任务，算出下一个最近的提醒时刻并 sleep 到点，再对同一时刻到期的任务
//! 逐个发通知；配置变更经 `reschedule` 重建线程。
//!
//! 两类提醒：
//! - 每日任务（is_daily=1, reminder_time 非空）：每天到点重复提醒。
//! - 普通任务（is_daily=0, remind_at 非空）：到点提醒一次，发完清空 remind_at。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use chrono::{Duration as ChronoDuration, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};
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

/// 提醒类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReminderKind {
    /// 每日任务：reminder_time（HH:MM）每天重复。
    Daily,
    /// 普通任务：remind_at（YYYY-MM-DD HH:MM）一次性。
    OneShot,
}

/// 任务提醒记录。
#[derive(Debug, Clone)]
struct Reminder {
    id: i64,
    title: String,
    kind: ReminderKind,
    /// 每日任务：reminder_time 解析出的当日时刻（占位日期，仅取 time）；
    /// 普通任务：remind_at 解析出的具体时刻（+08:00）。
    when: NaiveDateTime,
}

impl Reminder {
    /// 下一次触发时刻。每日任务按今天/明天滚动；一次性任务直接取 remind_at。
    fn next_occurrence(&self, now: NaiveDateTime) -> NaiveDateTime {
        match self.kind {
            ReminderKind::Daily => next_occurrence_dt(now, self.when.time()),
            ReminderKind::OneShot => self.when,
        }
    }

    /// 一次性任务是否已过期（提醒时刻错过不补发）。
    /// 未触发时 remind_at 已过去超过 1 分钟视为过期，不再排程，等待启动清理。
    fn is_expired(&self, now: NaiveDateTime) -> bool {
        match self.kind {
            ReminderKind::Daily => false,
            ReminderKind::OneShot => now >= self.when + ChronoDuration::seconds(60),
        }
    }
}

/// 读取所有启用提醒的任务（未删除）：
/// - 每日任务（is_daily=1、reminder_time 非空）
/// - 普通任务（is_daily=0、remind_at 非空）
fn load_reminders(conn: &Connection) -> Vec<Reminder> {
    let mut reminders = Vec::new();

    // 每日任务：reminder_time（HH:MM）每天重复。
    if let Ok(mut stmt) = conn.prepare(
        "SELECT id, title, reminder_time FROM tasks
         WHERE is_daily = 1 AND reminder_time IS NOT NULL AND reminder_time != '' AND deleted_at IS NULL",
    ) {
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        }) {
            for row in rows.flatten() {
                if let Ok(t) = NaiveTime::parse_from_str(&row.2, "%H:%M") {
                    reminders.push(Reminder {
                        id: row.0,
                        title: row.1,
                        kind: ReminderKind::Daily,
                        when: NaiveDate::from_ymd_opt(2000, 1, 1)
                            .expect("valid placeholder date")
                            .and_time(t),
                    });
                }
            }
        }
    }

    // 普通任务：remind_at（YYYY-MM-DD HH:MM，+08:00）一次性。
    if let Ok(mut stmt) = conn.prepare(
        "SELECT id, title, remind_at FROM tasks
         WHERE is_daily = 0 AND remind_at IS NOT NULL AND remind_at != '' AND deleted_at IS NULL",
    ) {
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        }) {
            for row in rows.flatten() {
                if let Ok(dt) = NaiveDateTime::parse_from_str(&row.2, "%Y-%m-%d %H:%M") {
                    reminders.push(Reminder {
                        id: row.0,
                        title: row.1,
                        kind: ReminderKind::OneShot,
                        when: dt,
                    });
                }
            }
        }
    }

    reminders
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
fn plan_next(reminders: &[Reminder], now: NaiveDateTime) -> (u64, Vec<Reminder>) {
    // 一次性任务已过期（错过不补发）不参与排程。
    let active: Vec<Reminder> = reminders
        .iter()
        .filter(|r| !r.is_expired(now))
        .cloned()
        .collect();

    if active.is_empty() {
        return (3600, Vec::new());
    }

    let next_dt = active
        .iter()
        .map(|r| r.next_occurrence(now))
        .min()
        .expect("active reminders non-empty");

    let wait = (next_dt - now).num_seconds().max(0) as u64;
    let due = if wait == 0 {
        active
            .iter()
            .filter(|r| r.next_occurrence(now) == next_dt)
            .cloned()
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
            match reminder.kind {
                ReminderKind::Daily => {
                    let body = format!("「{}」—— 别忘了今天完成", reminder.title);
                    show_system_notification(
                        &app_handle,
                        ids::stable_id(&format!("daily-task:{}", reminder.id)),
                        "每日任务提醒",
                        &body,
                    );
                }
                ReminderKind::OneShot => {
                    let body = format!("「{}」—— 到时间了，别忘了完成", reminder.title);
                    show_system_notification(
                        &app_handle,
                        ids::stable_id(&format!("task-remind:{}", reminder.id)),
                        "任务提醒",
                        &body,
                    );
                    // 一次性提醒发完即清空，防止下次轮询重复触发。
                    if let Ok(conn) = db.lock() {
                        let _ = conn.execute(
                            "UPDATE tasks SET remind_at = NULL, updated_at = ?1 WHERE id = ?2",
                            rusqlite::params![crate::db::chrono_now(), reminder.id],
                        );
                    }
                }
            }
        }
    });
}

/// 任务/提醒配置变更后重建提醒线程。
pub fn reschedule(db: Arc<Mutex<Connection>>, app_handle: &AppHandle) {
    ensure_scheduled(db, app_handle.clone());
}

/// 清理已过期的一次性 remind_at（应用错过提醒不补发）。
/// 幂等：无过期任务时无副作用。启动时在 ensure_scheduled 前调用。
pub fn clear_expired_remind_at(conn: &Connection) {
    let now_str = china_now().format("%Y-%m-%d %H:%M").to_string();
    let _ = conn.execute(
        "UPDATE tasks SET remind_at = NULL, updated_at = ?1
         WHERE is_daily = 0 AND remind_at IS NOT NULL AND remind_at != '' AND remind_at < ?2",
        rusqlite::params![crate::db::chrono_now(), now_str],
    );
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

    fn daily(id: i64, title: &str, time: &str) -> Reminder {
        Reminder {
            id,
            title: title.to_string(),
            kind: ReminderKind::Daily,
            when: NaiveDate::from_ymd_opt(2000, 1, 1)
                .unwrap()
                .and_time(NaiveTime::parse_from_str(time, "%H:%M").unwrap()),
        }
    }

    fn one_shot(id: i64, title: &str, when: &str) -> Reminder {
        Reminder {
            id,
            title: title.to_string(),
            kind: ReminderKind::OneShot,
            when: NaiveDateTime::parse_from_str(when, "%Y-%m-%d %H:%M").unwrap(),
        }
    }

    fn dt(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, mo, d)
            .unwrap()
            .and_hms_opt(h, mi, s)
            .unwrap()
    }

    #[test]
    fn test_next_occurrence_dt_same_day_and_next_day() {
        let now = dt(2026, 8, 1, 9, 0, 0);
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
    fn test_plan_next_daily_single() {
        let now = dt(2026, 8, 1, 9, 0, 0);
        let reminders = vec![daily(1, "背单词", "20:00")];
        let (wait, due) = plan_next(&reminders, now);
        assert_eq!(wait, 11 * 3600);
        assert!(due.is_empty());
    }

    #[test]
    fn test_plan_next_daily_due_at_now() {
        let now = dt(2026, 8, 1, 20, 0, 0);
        let reminders = vec![daily(1, "背单词", "20:00")];
        let (wait, due) = plan_next(&reminders, now);
        assert_eq!(wait, 0);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, 1);
    }

    #[test]
    fn test_plan_next_daily_same_time_multiple() {
        let reminders = vec![
            daily(1, "A", "08:00"),
            daily(2, "B", "08:00"),
            daily(3, "C", "09:00"),
        ];
        // 7:00 时最近是 8:00，尚未到期 → 等待 1 小时、due 为空。
        let now = dt(2026, 8, 1, 7, 0, 0);
        let (wait, due) = plan_next(&reminders, now);
        assert_eq!(wait, 3600);
        assert!(due.is_empty());

        // 到点 8:00：同一时刻多个任务同时到期。
        let at_due = dt(2026, 8, 1, 8, 0, 0);
        let (wait2, due2) = plan_next(&reminders, at_due);
        assert_eq!(wait2, 0);
        assert_eq!(due2.len(), 2);
    }

    #[test]
    fn test_plan_next_empty() {
        let now = dt(2026, 8, 1, 7, 0, 0);
        let (wait, due) = plan_next(&[], now);
        assert_eq!(wait, 3600);
        assert!(due.is_empty());
    }

    #[test]
    fn test_plan_next_one_shot_future() {
        // 一次性任务在未来时刻：等待至 remind_at，不立即触发。
        let now = dt(2026, 8, 3, 9, 0, 0);
        let reminders = vec![one_shot(1, "交报告", "2026-08-05 14:00")];
        let (wait, due) = plan_next(&reminders, now);
        assert_eq!(wait, 190800);
        assert!(due.is_empty());
    }

    #[test]
    fn test_plan_next_one_shot_due() {
        // 一次性到点（同一分钟内）：wait=0，触发。
        let now = dt(2026, 8, 3, 14, 0, 30);
        let reminders = vec![one_shot(1, "交报告", "2026-08-03 14:00")];
        let (wait, due) = plan_next(&reminders, now);
        assert_eq!(wait, 0);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, 1);
        assert_eq!(due[0].kind, ReminderKind::OneShot);
    }

    #[test]
    fn test_plan_next_one_shot_expired_skipped() {
        // 一次性已过期（超过 1 分钟）不排程，错过不补发。
        let now = dt(2026, 8, 3, 14, 5, 0);
        let reminders = vec![one_shot(1, "交报告", "2026-08-03 14:00")];
        let (wait, due) = plan_next(&reminders, now);
        assert_eq!(wait, 3600);
        assert!(due.is_empty());
    }

    #[test]
    fn test_plan_next_mixed_daily_and_one_shot() {
        // 每日任务与一次性任务混合：取最近触发时刻。
        let now = dt(2026, 8, 3, 9, 0, 0);
        let reminders = vec![
            daily(1, "背单词", "20:00"),
            one_shot(2, "交报告", "2026-08-03 10:00"),
        ];
        let (wait, due) = plan_next(&reminders, now);
        assert_eq!(wait, 3600);
        assert!(due.is_empty());

        // 10:00 一次性到点，同时每日 20:00 未到。
        let at_due = dt(2026, 8, 3, 10, 0, 0);
        let (wait2, due2) = plan_next(&reminders, at_due);
        assert_eq!(wait2, 0);
        assert_eq!(due2.len(), 1);
        assert_eq!(due2[0].id, 2);
    }

    #[test]
    fn test_clear_expired_remind_at() {
        // 启动清理：过期 remind_at 清空，未来 remind_at 保留。
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO tasks (sync_id, title, description, status, priority, tags, is_daily, remind_at, created_at, updated_at)
             VALUES ('s1', '过期', '', 'todo', 'medium', '[]', 0, '2020-01-01 08:00', '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tasks (sync_id, title, description, status, priority, tags, is_daily, remind_at, created_at, updated_at)
             VALUES ('s2', '未来', '', 'todo', 'medium', '[]', 0, '2099-01-01 08:00', '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z')",
            [],
        )
        .unwrap();

        clear_expired_remind_at(&conn);

        let expired: Option<String> = conn
            .query_row(
                "SELECT remind_at FROM tasks WHERE sync_id = 's1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let future: Option<String> = conn
            .query_row(
                "SELECT remind_at FROM tasks WHERE sync_id = 's2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(expired.is_none(), "过期 remind_at 应被清理");
        assert_eq!(future.as_deref(), Some("2099-01-01 08:00"));
    }
}
