use rusqlite::{params, Connection, Result, Row};

use super::models::{CreatePomodoroProfileRequest, PomodoroProfile, UpdatePomodoroProfileRequest};

pub fn get_all_pomodoro_profiles(conn: &Connection) -> Result<Vec<PomodoroProfile>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, work_seconds, short_break_seconds, long_break_seconds,
                sessions_before_long_break, is_builtin, created_at, updated_at
         FROM pomodoro_profiles
         ORDER BY is_builtin DESC, name ASC",
    )?;
    let rows = stmt.query_map([], row_to_profile)?;
    rows.collect()
}

pub fn get_pomodoro_profile(conn: &Connection, id: i64) -> Result<PomodoroProfile> {
    conn.query_row(
        "SELECT id, name, work_seconds, short_break_seconds, long_break_seconds,
                sessions_before_long_break, is_builtin, created_at, updated_at
         FROM pomodoro_profiles
         WHERE id = ?1",
        params![id],
        row_to_profile,
    )
}

pub fn create_pomodoro_profile(
    conn: &Connection,
    req: &CreatePomodoroProfileRequest,
) -> Result<i64> {
    let now = super::chrono_now();
    conn.execute(
        "INSERT INTO pomodoro_profiles
            (name, work_seconds, short_break_seconds, long_break_seconds,
             sessions_before_long_break, is_builtin, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?6)",
        params![
            req.name.trim(),
            req.work_seconds,
            req.short_break_seconds,
            req.long_break_seconds,
            req.sessions_before_long_break,
            now,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_pomodoro_profile(
    conn: &Connection,
    id: i64,
    req: &UpdatePomodoroProfileRequest,
) -> Result<()> {
    let now = super::chrono_now();
    conn.execute(
        "UPDATE pomodoro_profiles
         SET name = ?1, work_seconds = ?2, short_break_seconds = ?3,
             long_break_seconds = ?4, sessions_before_long_break = ?5, updated_at = ?6
         WHERE id = ?7 AND is_builtin = 0",
        params![
            req.name.trim(),
            req.work_seconds,
            req.short_break_seconds,
            req.long_break_seconds,
            req.sessions_before_long_break,
            now,
            id,
        ],
    )?;
    Ok(())
}

pub fn delete_pomodoro_profile(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM pomodoro_profiles WHERE id = ?1 AND is_builtin = 0",
        params![id],
    )?;
    Ok(())
}

fn row_to_profile(row: &Row<'_>) -> Result<PomodoroProfile> {
    Ok(PomodoroProfile {
        id: row.get(0)?,
        name: row.get(1)?,
        work_seconds: row.get(2)?,
        short_break_seconds: row.get(3)?,
        long_break_seconds: row.get(4)?,
        sessions_before_long_break: row.get(5)?,
        is_builtin: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::setup_db;

    fn sample_request(name: &str) -> CreatePomodoroProfileRequest {
        CreatePomodoroProfileRequest {
            name: name.to_string(),
            work_seconds: 1800,
            short_break_seconds: 300,
            long_break_seconds: 1200,
            sessions_before_long_break: 4,
        }
    }

    #[test]
    fn test_builtin_profiles_exist() {
        let conn = setup_db();
        let profiles = get_all_pomodoro_profiles(&conn).expect("get profiles");
        let names: Vec<_> = profiles
            .iter()
            .map(|profile| profile.name.as_str())
            .collect();
        assert!(names.contains(&"default"));
        assert!(names.contains(&"intense"));
        assert!(names.contains(&"relaxed"));
    }

    #[test]
    fn test_create_update_delete_custom_profile() {
        let conn = setup_db();
        let id = create_pomodoro_profile(&conn, &sample_request("custom")).expect("create profile");

        let update = UpdatePomodoroProfileRequest {
            name: "custom 2".to_string(),
            work_seconds: 2400,
            short_break_seconds: 600,
            long_break_seconds: 1800,
            sessions_before_long_break: 3,
        };
        update_pomodoro_profile(&conn, id, &update).expect("update profile");

        let profile = get_pomodoro_profile(&conn, id).expect("get profile");
        assert_eq!(profile.name, "custom 2");
        assert_eq!(profile.work_seconds, 2400);

        delete_pomodoro_profile(&conn, id).expect("delete profile");
        assert!(get_pomodoro_profile(&conn, id).is_err());
    }

    #[test]
    fn test_builtin_profile_delete_is_ignored() {
        let conn = setup_db();
        let default = get_all_pomodoro_profiles(&conn)
            .expect("get profiles")
            .into_iter()
            .find(|profile| profile.name == "default")
            .expect("default profile");

        delete_pomodoro_profile(&conn, default.id).expect("delete builtin ignored");

        let profile = get_pomodoro_profile(&conn, default.id).expect("default still exists");
        assert!(profile.is_builtin);
    }
}
