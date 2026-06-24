use rusqlite::{params, Connection, Result};

use super::models::{
    CreatePomodoroSessionRequest, PomodoroConfig, PomodoroSession, UpdatePomodoroConfigRequest,
};

pub fn get_config(conn: &Connection) -> Result<PomodoroConfig> {
    let result = conn.query_row(
        "SELECT id, work_seconds, short_break_seconds, long_break_seconds, sessions_before_long_break
         FROM pomodoro_config WHERE id = 1",
        [],
        |row| {
            Ok(PomodoroConfig {
                id: row.get(0)?,
                work_seconds: row.get(1)?,
                short_break_seconds: row.get(2)?,
                long_break_seconds: row.get(3)?,
                sessions_before_long_break: row.get(4)?,
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
            };
            conn.execute(
                "INSERT INTO pomodoro_config (id, work_seconds, short_break_seconds, long_break_seconds, sessions_before_long_break)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    default.id,
                    default.work_seconds,
                    default.short_break_seconds,
                    default.long_break_seconds,
                    default.sessions_before_long_break,
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
         SET work_seconds = ?1, short_break_seconds = ?2, long_break_seconds = ?3, sessions_before_long_break = ?4
         WHERE id = 1",
        params![req.work_seconds, req.short_break_seconds, req.long_break_seconds, req.sessions_before_long_break],
    )?;
    Ok(())
}

pub fn create_session(conn: &Connection, req: &CreatePomodoroSessionRequest) -> Result<i64> {
    conn.execute(
        "INSERT INTO pomodoro_sessions (started_at, session_type, task_id) VALUES (?1, ?2, ?3)",
        params![req.started_at, req.session_type, req.task_id],
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

pub fn get_sessions(conn: &Connection, limit: i64, offset: i64) -> Result<Vec<PomodoroSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, started_at, ended_at, session_type, task_id
         FROM pomodoro_sessions
         ORDER BY started_at DESC
         LIMIT ?1 OFFSET ?2",
    )?;

    let rows = stmt.query_map(params![limit, offset], |row| {
        Ok(PomodoroSession {
            id: row.get(0)?,
            started_at: row.get(1)?,
            ended_at: row.get(2)?,
            session_type: row.get(3)?,
            task_id: row.get(4)?,
        })
    })?;

    rows.collect()
}

pub fn get_sessions_by_task(conn: &Connection, task_id: i64) -> Result<Vec<PomodoroSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, started_at, ended_at, session_type, task_id
         FROM pomodoro_sessions
         WHERE task_id = ?1
         ORDER BY started_at DESC",
    )?;

    let rows = stmt.query_map(params![task_id], |row| {
        Ok(PomodoroSession {
            id: row.get(0)?,
            started_at: row.get(1)?,
            ended_at: row.get(2)?,
            session_type: row.get(3)?,
            task_id: row.get(4)?,
        })
    })?;

    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("Failed to enable foreign keys");
        migrations::run_migrations(&conn).expect("Migrations failed");
        conn
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
        };
        update_config(&conn, &update).expect("Failed to update config");

        let config = get_config(&conn).expect("Failed to get updated config");
        assert_eq!(config.work_seconds, 1800);
        assert_eq!(config.short_break_seconds, 600);
        assert_eq!(config.long_break_seconds, 1200);
        assert_eq!(config.sessions_before_long_break, 3);
    }

    #[test]
    fn test_create_and_end_session() {
        let conn = setup_db();

        let req = CreatePomodoroSessionRequest {
            started_at: "2024-06-01T10:00:00Z".to_string(),
            session_type: "work".to_string(),
            task_id: None,
        };
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
            let req = CreatePomodoroSessionRequest {
                started_at: format!("2024-06-01T10:0{}:00Z", i),
                session_type: "work".to_string(),
                task_id: None,
            };
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
        };
        let task_id =
            crate::db::tasks::create_task(&conn, &task_req).expect("Failed to create task");

        let req1 = CreatePomodoroSessionRequest {
            started_at: "2024-06-01T10:00:00Z".to_string(),
            session_type: "work".to_string(),
            task_id: Some(task_id),
        };
        create_session(&conn, &req1).expect("Failed to create session");

        let req2 = CreatePomodoroSessionRequest {
            started_at: "2024-06-01T10:30:00Z".to_string(),
            session_type: "short_break".to_string(),
            task_id: None,
        };
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
}
