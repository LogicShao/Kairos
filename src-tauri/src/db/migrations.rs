use rusqlite::{Connection, Result};

pub fn run_migrations(conn: &Connection) -> Result<()> {
    // Create the migrations tracking table if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version INTEGER PRIMARY KEY,
            name    TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Define all migrations in order
    let migrations: Vec<(i32, &str, &str)> = vec![
        (
            1,
            "initial_schema",
            "
            CREATE TABLE IF NOT EXISTS pomodoro_config (
                id INTEGER PRIMARY KEY DEFAULT 1,
                work_seconds INTEGER NOT NULL DEFAULT 1500,
                short_break_seconds INTEGER NOT NULL DEFAULT 300,
                long_break_seconds INTEGER NOT NULL DEFAULT 900,
                sessions_before_long_break INTEGER NOT NULL DEFAULT 4
            );

            CREATE TABLE IF NOT EXISTS pomodoro_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                session_type TEXT NOT NULL CHECK(session_type IN ('work', 'short_break', 'long_break')),
                task_id INTEGER,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'todo' CHECK(status IN ('todo', 'in_progress', 'done')),
                priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('high', 'medium', 'low')),
                due_date TEXT,
                tags TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS courses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                day_of_week INTEGER NOT NULL CHECK(day_of_week BETWEEN 1 AND 7),
                start_time TEXT NOT NULL,
                end_time TEXT NOT NULL,
                location TEXT NOT NULL DEFAULT '',
                teacher TEXT NOT NULL DEFAULT '',
                color TEXT NOT NULL DEFAULT '#3B82F6',
                semester TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS exams (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                course_name TEXT NOT NULL,
                exam_datetime TEXT NOT NULL,
                location TEXT NOT NULL DEFAULT '',
                notes TEXT NOT NULL DEFAULT '',
                course_id INTEGER,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (course_id) REFERENCES courses(id) ON DELETE SET NULL
            );
            ",
        ),
    ];

    let current_version: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM _migrations",
            [],
            |row| row.get(0),
        )?;

    for (version, name, sql) in migrations {
        if version > current_version {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO _migrations (version, name) VALUES (?1, ?2)",
                rusqlite::params![version, name],
            )?;
            tx.commit()?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_creates_tables() {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
        // Enable foreign keys
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("Failed to enable foreign keys");

        run_migrations(&conn).expect("Migrations failed");

        // Verify _migrations table exists and has version 1
        let version: i32 = conn
            .query_row("SELECT version FROM _migrations WHERE version = 1", [], |row| {
                row.get(0)
            })
            .expect("Migration record not found");
        assert_eq!(version, 1);

        // Verify all tables exist
        let table_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_migrations'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count tables");
        assert_eq!(table_count, 5);

        // Verify pomodoro_config has default row
        let has_default: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pomodoro_config",
                [],
                |row| row.get(0),
            )
            .expect("Failed to query pomodoro_config");
        assert!(!has_default, "pomodoro_config should be empty by default");

        // Verify CHECK constraints exist by inserting valid/invalid data
        conn.execute(
            "INSERT INTO tasks (title, description, status, priority, tags, created_at, updated_at)
             VALUES ('test', '', 'todo', 'medium', '[]', '2024-01-01', '2024-01-01')",
            [],
        )
        .expect("Valid task insert failed");

        // Invalid status should fail
        let result = conn.execute(
            "INSERT INTO tasks (title, description, status, priority, tags, created_at, updated_at)
             VALUES ('test2', '', 'invalid_status', 'medium', '[]', '2024-01-01', '2024-01-01')",
            [],
        );
        assert!(result.is_err(), "Invalid status should be rejected");
    }

    #[test]
    fn test_migration_idempotent() {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("Failed to enable foreign keys");

        // Run migrations twice - second should be a no-op
        run_migrations(&conn).expect("First migration failed");
        run_migrations(&conn).expect("Second migration should be idempotent");

        // Should still only have one migration record
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
            .expect("Failed to count migrations");
        assert_eq!(count, 1);
    }
}
