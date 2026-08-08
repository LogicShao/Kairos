use rusqlite::{params, Connection, Result, Row};

use chrono::FixedOffset;

use super::models::{
    CreatePomodoroSessionRequest, PomodoroConfig, PomodoroRuntimeState, PomodoroSession,
    UpdatePomodoroConfigRequest,
};

/// 获取番茄钟配置。若 config 表尚无记录（首次启动），插入默认值并返回。
/// 默认: 25 分钟工作 / 5 分钟短休息 / 15 分钟长休息 / 每 4 次 work 触发一次 long_break /
/// 不自动开始下一阶段（需用户按开始）。
pub fn get_config(conn: &Connection) -> Result<PomodoroConfig> {
    let result = conn.query_row(
        "SELECT id, work_seconds, short_break_seconds, long_break_seconds, sessions_before_long_break, auto_start_next_phase
         FROM pomodoro_config WHERE id = 1",
        [],
        |row| {
            Ok(PomodoroConfig {
                id: row.get(0)?,
                work_seconds: row.get(1)?,
                short_break_seconds: row.get(2)?,
                long_break_seconds: row.get(3)?,
                sessions_before_long_break: row.get(4)?,
                auto_start_next_phase: row.get::<_, i64>(5)? != 0,
            })
        },
    );

    match result {
        Ok(config) => Ok(config),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let default = PomodoroConfig {
                id: 1,
                work_seconds: 1500,
                short_break_seconds: 300,
                long_break_seconds: 900,
                sessions_before_long_break: 4,
                auto_start_next_phase: false,
            };
            conn.execute(
                "INSERT INTO pomodoro_config (id, work_seconds, short_break_seconds, long_break_seconds, sessions_before_long_break, auto_start_next_phase)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    default.id,
                    default.work_seconds,
                    default.short_break_seconds,
                    default.long_break_seconds,
                    default.sessions_before_long_break,
                    default.auto_start_next_phase as i64,
                ],
            )?;
            Ok(default)
        }
        Err(e) => Err(e),
    }
}

pub fn update_config(conn: &Connection, req: &UpdatePomodoroConfigRequest) -> Result<()> {
    conn.execute(
        "UPDATE pomodoro_config
         SET work_seconds = ?1, short_break_seconds = ?2, long_break_seconds = ?3, sessions_before_long_break = ?4, auto_start_next_phase = ?5
         WHERE id = 1",
        params![
            req.work_seconds,
            req.short_break_seconds,
            req.long_break_seconds,
            req.sessions_before_long_break,
            req.auto_start_next_phase as i64,
        ],
    )?;
    Ok(())
}

pub fn create_session(conn: &Connection, req: &CreatePomodoroSessionRequest) -> Result<i64> {
    conn.execute(
        "INSERT INTO pomodoro_sessions (sync_id, started_at, session_type, task_id) VALUES (?1, ?2, ?3, ?4)",
        params![
            crate::sync::ids::new_sync_id(),
            req.started_at,
            req.session_type,
            req.task_id,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_session_end(conn: &Connection, id: i64, ended_at: &str) -> Result<()> {
    conn.execute(
        "UPDATE pomodoro_sessions SET ended_at = ?1 WHERE id = ?2",
        params![ended_at, id],
    )?;
    Ok(())
}

fn row_to_session(row: &Row<'_>) -> Result<PomodoroSession> {
    Ok(PomodoroSession {
        id: row.get(0)?,
        sync_id: row.get(1)?,
        started_at: row.get(2)?,
        ended_at: row.get(3)?,
        session_type: row.get(4)?,
        task_id: row.get(5)?,
        deleted_at: row.get(6)?,
    })
}

pub fn get_sessions(conn: &Connection, limit: i64, offset: i64) -> Result<Vec<PomodoroSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, sync_id, started_at, ended_at, session_type, task_id, deleted_at
         FROM pomodoro_sessions
         WHERE deleted_at IS NULL
         ORDER BY started_at DESC
         LIMIT ?1 OFFSET ?2",
    )?;

    let rows = stmt.query_map(params![limit, offset], row_to_session)?;

    rows.collect()
}

pub fn get_sessions_by_task(conn: &Connection, task_id: i64) -> Result<Vec<PomodoroSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, sync_id, started_at, ended_at, session_type, task_id, deleted_at
         FROM pomodoro_sessions
         WHERE task_id = ?1 AND deleted_at IS NULL
         ORDER BY started_at DESC",
    )?;

    let rows = stmt.query_map(params![task_id], row_to_session)?;

    rows.collect()
}

/// 返回今天（UTC+8）的本地日期字符串 YYYY-MM-DD。
pub fn today_local_date() -> String {
    let china_offset = FixedOffset::east_opt(8 * 3600).expect("china utc offset");
    chrono::Utc::now()
        .with_timezone(&china_offset)
        .format("%Y-%m-%d")
        .to_string()
}

/// 返回今天（UTC+8）对应 UTC 时间范围 [start, end)。
fn today_utc_range() -> (String, String) {
    let china_offset = FixedOffset::east_opt(8 * 3600).expect("china utc offset");
    let now_local = chrono::Utc::now().with_timezone(&china_offset);
    let today_local = now_local.date_naive();
    let start_local = today_local
        .and_hms_opt(0, 0, 0)
        .expect("invalid start time");
    let end_local = (today_local + chrono::Duration::days(1))
        .and_hms_opt(0, 0, 0)
        .expect("invalid end time");
    let start_utc = start_local
        .and_local_timezone(china_offset)
        .unwrap()
        .to_utc();
    let end_utc = end_local.and_local_timezone(china_offset).unwrap().to_utc();
    (
        start_utc.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        end_utc.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    )
}

/// 获取或创建 pomodoro_runtime_state 记录（仅一条，id = 1）。
pub fn get_or_create_runtime_state(conn: &Connection) -> Result<PomodoroRuntimeState> {
    let result = conn.query_row(
        "SELECT id, phase, remaining_seconds, total_seconds, is_running,
                active_session_id, date_key, last_seen_at, interrupted,
                created_at, updated_at
         FROM pomodoro_runtime_state WHERE id = 1",
        [],
        |row| {
            Ok(PomodoroRuntimeState {
                id: row.get(0)?,
                phase: row.get(1)?,
                remaining_seconds: row.get(2)?,
                total_seconds: row.get(3)?,
                is_running: row.get::<_, i64>(4)? != 0,
                active_session_id: row.get(5)?,
                date_key: row.get(6)?,
                last_seen_at: row.get(7)?,
                interrupted: row.get::<_, i64>(8)? != 0,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        },
    );

    match result {
        Ok(state) => Ok(state),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let now = crate::db::chrono_now();
            let date_key = today_local_date();
            conn.execute(
                "INSERT INTO pomodoro_runtime_state
                 (id, phase, remaining_seconds, total_seconds, is_running,
                  active_session_id, date_key, last_seen_at, interrupted,
                  created_at, updated_at)
                 VALUES (1, 'work', 1500, 1500, 0, NULL, ?1, ?2, 0, ?3, ?3)",
                params![date_key, now, now],
            )?;
            Ok(PomodoroRuntimeState {
                id: 1,
                phase: "work".to_string(),
                remaining_seconds: 1500,
                total_seconds: 1500,
                is_running: false,
                active_session_id: None,
                date_key,
                last_seen_at: now.clone(),
                interrupted: false,
                created_at: now.clone(),
                updated_at: now,
            })
        }
        Err(e) => Err(e),
    }
}

/// 更新 pomodoro_runtime_state 记录（id = 1）。
pub fn update_runtime_state(
    conn: &Connection,
    phase: &str,
    remaining_seconds: i64,
    total_seconds: i64,
    is_running: bool,
    active_session_id: Option<i64>,
    interrupted: bool,
) -> Result<()> {
    let now = crate::db::chrono_now();
    let date_key = today_local_date();
    conn.execute(
        "UPDATE pomodoro_runtime_state
         SET phase = ?1, remaining_seconds = ?2, total_seconds = ?3,
             is_running = ?4, active_session_id = ?5, date_key = ?6,
             last_seen_at = ?7, interrupted = ?8, updated_at = ?9
         WHERE id = 1",
        params![
            phase,
            remaining_seconds,
            total_seconds,
            is_running as i64,
            active_session_id,
            date_key,
            now,
            interrupted as i64,
            now,
        ],
    )?;
    Ok(())
}

/// 清除中断标记（interrupted = 0）。
pub fn clear_runtime_interruption(conn: &Connection) -> Result<()> {
    let now = crate::db::chrono_now();
    conn.execute(
        "UPDATE pomodoro_runtime_state
         SET interrupted = 0, updated_at = ?1
         WHERE id = 1",
        params![now],
    )?;
    Ok(())
}

/// 统计今天（UTC+8）已正常结束的 work sessions 数量。
/// 条件：session_type = 'work'，ended_at IS NOT NULL，deleted_at IS NULL，
/// 且 started_at 落在今天 UTC 范围内。
pub fn count_completed_work_sessions_for_date(conn: &Connection) -> Result<i64> {
    let (start_utc, end_utc) = today_utc_range();
    conn.query_row(
        "SELECT COUNT(*) FROM pomodoro_sessions
         WHERE session_type = 'work'
           AND ended_at IS NOT NULL
           AND deleted_at IS NULL
           AND started_at >= ?1
           AND started_at < ?2",
        params![start_utc, end_utc],
        |row| row.get(0),
    )
}

/// 查找最近一条未结束（ended_at IS NULL）且未删除的 work session。
/// 用于启动时检测是否存在孤儿 session。
pub fn find_latest_open_work_session(conn: &Connection) -> Result<Option<i64>> {
    let result = conn.query_row(
        "SELECT id FROM pomodoro_sessions
         WHERE session_type = 'work'
           AND ended_at IS NULL
           AND deleted_at IS NULL
         ORDER BY started_at DESC
         LIMIT 1",
        [],
        |row| row.get::<_, i64>(0),
    );
    match result {
        Ok(id) => Ok(Some(id)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// 软删除指定 session（设置 deleted_at = now）。
pub fn soft_delete_session(conn: &Connection, id: i64) -> Result<()> {
    let now = crate::db::chrono_now();
    conn.execute(
        "UPDATE pomodoro_sessions SET deleted_at = ?1 WHERE id = ?2",
        params![now, id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::setup_db;

    fn sample_session(
        started_at: &str,
        session_type: &str,
        task_id: Option<i64>,
    ) -> CreatePomodoroSessionRequest {
        CreatePomodoroSessionRequest {
            started_at: started_at.to_string(),
            session_type: session_type.to_string(),
            task_id,
        }
    }

    #[test]
    fn test_get_config_creates_default() {
        let conn = setup_db();

        let config = get_config(&conn).expect("Failed to get config");
        assert_eq!(config.id, 1);
        assert_eq!(config.work_seconds, 1500);
        assert_eq!(config.short_break_seconds, 300);
        assert_eq!(config.long_break_seconds, 900);
        assert_eq!(config.sessions_before_long_break, 4);

        let config2 = get_config(&conn).expect("Failed to get config again");
        assert_eq!(config.id, config2.id);
    }

    #[test]
    fn test_update_config() {
        let conn = setup_db();

        let _ = get_config(&conn).expect("Failed to get initial config");

        let update = UpdatePomodoroConfigRequest {
            work_seconds: 1800,
            short_break_seconds: 600,
            long_break_seconds: 1200,
            sessions_before_long_break: 3,
            auto_start_next_phase: true,
        };
        update_config(&conn, &update).expect("Failed to update config");

        let config = get_config(&conn).expect("Failed to get updated config");
        assert_eq!(config.work_seconds, 1800);
        assert_eq!(config.short_break_seconds, 600);
        assert_eq!(config.long_break_seconds, 1200);
        assert_eq!(config.sessions_before_long_break, 3);
        assert!(config.auto_start_next_phase);
    }

    #[test]
    fn test_create_and_end_session() {
        let conn = setup_db();

        let req = sample_session("2024-06-01T10:00:00Z", "work", None);
        let id = create_session(&conn, &req).expect("Failed to create session");
        assert!(id > 0);

        update_session_end(&conn, id, "2024-06-01T10:25:00Z").expect("Failed to end session");

        let sessions = get_sessions(&conn, 10, 0).expect("Failed to list sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, id);
        assert_eq!(sessions[0].session_type, "work");
        assert_eq!(
            sessions[0].ended_at.as_deref(),
            Some("2024-06-01T10:25:00Z")
        );
    }

    #[test]
    fn test_get_sessions_pagination() {
        let conn = setup_db();

        for i in 0..5 {
            let req = sample_session(&format!("2024-06-01T10:0{}:00Z", i), "work", None);
            create_session(&conn, &req).expect("Failed to create session");
        }

        let page1 = get_sessions(&conn, 3, 0).expect("Failed to get page 1");
        assert_eq!(page1.len(), 3);

        let page2 = get_sessions(&conn, 3, 3).expect("Failed to get page 2");
        assert_eq!(page2.len(), 2);
    }

    #[test]
    fn test_get_sessions_by_task() {
        let conn = setup_db();

        let task_req = crate::db::models::CreateTaskRequest {
            title: String::from("Related Task"),
            description: String::new(),
            status: String::from("todo"),
            priority: String::from("medium"),
            due_date: None,
            tags: String::from("[]"),
            is_daily: false,
            reminder_time: None,
            remind_at: None,
        };
        let task_id =
            crate::db::tasks::create_task(&conn, &task_req).expect("Failed to create task");

        let req1 = sample_session("2024-06-01T10:00:00Z", "work", Some(task_id));
        create_session(&conn, &req1).expect("Failed to create session");

        let req2 = sample_session("2024-06-01T10:30:00Z", "short_break", None);
        create_session(&conn, &req2).expect("Failed to create session");

        let sessions = get_sessions_by_task(&conn, task_id).expect("Failed to query by task");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].task_id, Some(task_id));
    }

    #[test]
    fn test_empty_sessions() {
        let conn = setup_db();

        let sessions = get_sessions(&conn, 10, 0).expect("Failed to list sessions");
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_get_or_create_runtime_state() {
        let conn = setup_db();

        let state = get_or_create_runtime_state(&conn).expect("Failed to get runtime state");
        assert_eq!(state.id, 1);
        assert_eq!(state.phase, "work");
        assert_eq!(state.remaining_seconds, 1500);
        assert_eq!(state.total_seconds, 1500);
        assert!(!state.is_running);
        assert!(!state.interrupted);

        // Calling again returns same record
        let state2 = get_or_create_runtime_state(&conn).expect("Failed to get runtime state");
        assert_eq!(state2.id, 1);
    }

    #[test]
    fn test_update_runtime_state() {
        let conn = setup_db();

        let _ = get_or_create_runtime_state(&conn).expect("Failed to create initial");

        update_runtime_state(&conn, "short_break", 120, 300, true, None, false)
            .expect("Failed to update runtime state");

        let state = get_or_create_runtime_state(&conn).expect("Failed to get updated state");
        assert_eq!(state.phase, "short_break");
        assert_eq!(state.remaining_seconds, 120);
        assert_eq!(state.total_seconds, 300);
        assert!(state.is_running);
    }

    #[test]
    fn test_update_runtime_state_persists_active_session_and_interruption() {
        let conn = setup_db();

        let req = sample_session(
            &chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "work",
            None,
        );
        let session_id = create_session(&conn, &req).expect("Failed to create session");
        let _ = get_or_create_runtime_state(&conn).expect("Failed to create initial");

        update_runtime_state(&conn, "work", 900, 1500, true, Some(session_id), true)
            .expect("Failed to update runtime state");

        let state = get_or_create_runtime_state(&conn).expect("Failed to get updated state");
        assert_eq!(state.phase, "work");
        assert_eq!(state.remaining_seconds, 900);
        assert!(state.is_running);
        assert_eq!(state.active_session_id, Some(session_id));
        assert!(state.interrupted);
    }

    #[test]
    fn test_clear_runtime_interruption() {
        let conn = setup_db();

        let _ = get_or_create_runtime_state(&conn).expect("Failed to create initial");
        update_runtime_state(&conn, "work", 1500, 1500, false, None, true)
            .expect("Failed to set interrupted");

        clear_runtime_interruption(&conn).expect("Failed to clear interruption");

        let state = get_or_create_runtime_state(&conn).expect("Failed to get state");
        assert!(!state.interrupted);
    }

    #[test]
    fn test_count_completed_work_sessions() {
        let conn = setup_db();

        // No sessions initially
        let count = count_completed_work_sessions_for_date(&conn).expect("Failed to count");
        assert_eq!(count, 0);

        // Create an un-ended session — should not count
        let req = sample_session(
            &chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "work",
            None,
        );
        let id1 = create_session(&conn, &req).expect("Failed to create session");

        let count = count_completed_work_sessions_for_date(&conn).expect("Failed to count");
        assert_eq!(count, 0, "unended session should not count");

        // End the session — now it should count
        update_session_end(
            &conn,
            id1,
            &chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        )
        .expect("Failed to end session");
        let count = count_completed_work_sessions_for_date(&conn).expect("Failed to count");
        assert_eq!(count, 1);

        // Soft-delete the session — should no longer count
        soft_delete_session(&conn, id1).expect("Failed to delete");
        let count = count_completed_work_sessions_for_date(&conn).expect("Failed to count");
        assert_eq!(count, 0, "soft-deleted session should not count");
    }

    #[test]
    fn test_find_latest_open_work_session() {
        let conn = setup_db();

        let result = find_latest_open_work_session(&conn).expect("Failed to find");
        assert!(result.is_none(), "no sessions means no open session");

        // Create an ended session
        let req = sample_session("2024-06-01T10:00:00Z", "work", None);
        let id1 = create_session(&conn, &req).expect("Failed to create session");
        update_session_end(&conn, id1, "2024-06-01T10:25:00Z").expect("Failed to end session");

        let result = find_latest_open_work_session(&conn).expect("Failed to find");
        assert!(result.is_none(), "ended session should not be returned");

        // Create an open session
        let req2 = sample_session("2024-06-01T11:00:00Z", "work", None);
        let id2 = create_session(&conn, &req2).expect("Failed to create session");
        let result = find_latest_open_work_session(&conn).expect("Failed to find");
        assert_eq!(result, Some(id2));
    }

    #[test]
    fn test_soft_delete_session() {
        let conn = setup_db();

        let req = sample_session("2024-06-01T10:00:00Z", "work", None);
        let id = create_session(&conn, &req).expect("Failed to create session");

        soft_delete_session(&conn, id).expect("Failed to soft delete");

        // Session should not appear in normal listing
        let sessions = get_sessions(&conn, 10, 0).expect("Failed to list sessions");
        assert!(sessions.is_empty());
    }
}
