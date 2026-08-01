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
        (
            2,
            "sync_config",
            "
            CREATE TABLE IF NOT EXISTS sync_config (
                id INTEGER PRIMARY KEY DEFAULT 1,
                server_url TEXT NOT NULL DEFAULT '',
                username TEXT NOT NULL DEFAULT '',
                password TEXT NOT NULL DEFAULT '',
                auto_sync INTEGER NOT NULL DEFAULT 0,
                last_sync_at TEXT
            );
            ",
        ),
        (
            3,
            "course_week_and_exam_range",
            "
            ALTER TABLE courses ADD COLUMN week_pattern TEXT NOT NULL DEFAULT '';
            ALTER TABLE courses ADD COLUMN semester_start_date TEXT NOT NULL DEFAULT '';
            ALTER TABLE exams ADD COLUMN exam_end_datetime TEXT NOT NULL DEFAULT '';
            ALTER TABLE exams ADD COLUMN semester TEXT NOT NULL DEFAULT '';
            ",
        ),
        (
            4,
            "sync_identity_and_tombstones",
            "",  // SQL 由 apply_sync_identity_and_tombstones_migration 处理
        ),
        (
            5,
            "notification_config",
            "
            CREATE TABLE IF NOT EXISTS notification_config (
                id INTEGER PRIMARY KEY DEFAULT 1,
                enabled INTEGER NOT NULL DEFAULT 1,
                exam_offsets_json TEXT NOT NULL DEFAULT '[1440,60]',
                android_channel_created INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            INSERT OR IGNORE INTO notification_config (id, enabled, exam_offsets_json, android_channel_created, created_at, updated_at)
            VALUES (1, 1, '[1440,60]', 0, datetime('now'), datetime('now'));
            ",
        ),
        (
            6,
            "pomodoro_runtime_state",
            "
            CREATE TABLE IF NOT EXISTS pomodoro_runtime_state (
                id INTEGER PRIMARY KEY DEFAULT 1,
                phase TEXT NOT NULL DEFAULT 'work' CHECK(phase IN ('work', 'short_break', 'long_break')),
                remaining_seconds INTEGER NOT NULL DEFAULT 1500,
                total_seconds INTEGER NOT NULL DEFAULT 1500,
                is_running INTEGER NOT NULL DEFAULT 0,
                active_session_id INTEGER,
                date_key TEXT NOT NULL,
                last_seen_at TEXT NOT NULL,
                interrupted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            ",
        ),
        (
            7,
            "widget_config",
            "
            CREATE TABLE IF NOT EXISTS widget_config (
                id INTEGER PRIMARY KEY DEFAULT 1,
                enabled INTEGER NOT NULL DEFAULT 0,
                mode TEXT NOT NULL DEFAULT 'medium' CHECK(mode IN ('small', 'medium', 'large')),
                opacity REAL NOT NULL DEFAULT 0.92 CHECK(opacity >= 0.6 AND opacity <= 1.0),
                always_on_top INTEGER NOT NULL DEFAULT 1,
                locked INTEGER NOT NULL DEFAULT 0,
                x INTEGER,
                y INTEGER,
                width INTEGER NOT NULL DEFAULT 320 CHECK(width BETWEEN 220 AND 520),
                height INTEGER NOT NULL DEFAULT 220 CHECK(height BETWEEN 140 AND 420),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            INSERT OR IGNORE INTO widget_config
                (id, enabled, mode, opacity, always_on_top, locked, width, height, created_at, updated_at)
            VALUES
                (1, 0, 'medium', 0.92, 1, 0, 320, 220, datetime('now'), datetime('now'));
            ",
        ),
        (
            8,
            "semester_context",
            "
            CREATE TABLE IF NOT EXISTS semester_context (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                academic_year TEXT,
                term TEXT,
                term_label TEXT NOT NULL,
                start_date TEXT NOT NULL,
                current_week INTEGER CHECK(current_week IS NULL OR current_week >= 1),
                total_weeks INTEGER CHECK(total_weeks IS NULL OR total_weeks >= 1),
                refreshed_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(source, term_label)
            );
            CREATE INDEX IF NOT EXISTS idx_semester_context_source_refreshed_at
                ON semester_context(source, refreshed_at);
            ",
        ),
        (
            9,
            "lzu_session",
            "
            CREATE TABLE IF NOT EXISTS lzu_session (
                id INTEGER PRIMARY KEY DEFAULT 1,
                username TEXT NOT NULL,
                login_token TEXT NOT NULL,
                gateway_token TEXT NOT NULL,
                profile_json TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            ",
        ),
        (
            10,
            "term_phases_and_pomodoro_profiles",
            "
            CREATE TABLE IF NOT EXISTS term_phases (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sync_id TEXT NOT NULL,
                term_label TEXT NOT NULL,
                phase_type TEXT NOT NULL CHECK(phase_type IN ('teaching', 'exam', 'break')),
                start_week INTEGER NOT NULL CHECK(start_week >= 1),
                end_week INTEGER NOT NULL CHECK(end_week >= start_week),
                affects_courses INTEGER NOT NULL DEFAULT 1,
                affects_exam_notifications INTEGER NOT NULL DEFAULT 1,
                pomodoro_profile TEXT NOT NULL DEFAULT 'default',
                notification_rules_json TEXT NOT NULL DEFAULT '{}',
                sort_order INTEGER NOT NULL DEFAULT 0,
                deleted_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_term_phases_term_label
                ON term_phases(term_label, sort_order);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_term_phases_sync_id
                ON term_phases(sync_id)
                WHERE sync_id IS NOT NULL AND sync_id != '';

            CREATE TABLE IF NOT EXISTS pomodoro_profiles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                work_seconds INTEGER NOT NULL DEFAULT 1500 CHECK(work_seconds >= 60),
                short_break_seconds INTEGER NOT NULL DEFAULT 300 CHECK(short_break_seconds >= 60),
                long_break_seconds INTEGER NOT NULL DEFAULT 900 CHECK(long_break_seconds >= 60),
                sessions_before_long_break INTEGER NOT NULL DEFAULT 4 CHECK(sessions_before_long_break >= 1),
                is_builtin INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            INSERT OR IGNORE INTO pomodoro_profiles
                (name, work_seconds, short_break_seconds, long_break_seconds, sessions_before_long_break, is_builtin, created_at, updated_at)
            VALUES
                ('default', 1500, 300, 900, 4, 1, datetime('now'), datetime('now')),
                ('intense', 3000, 600, 1800, 3, 1, datetime('now'), datetime('now')),
                ('relaxed', 1500, 600, 1200, 4, 1, datetime('now'), datetime('now'));
            ",
        ),
        (
            11,
            "ai_config_and_morning_brief",
            "
            CREATE TABLE IF NOT EXISTS ai_config (
                id INTEGER PRIMARY KEY DEFAULT 1,
                enabled INTEGER NOT NULL DEFAULT 0,
                base_url TEXT NOT NULL DEFAULT 'https://api.deepseek.com',
                model TEXT NOT NULL DEFAULT 'deepseek-v4-flash',
                api_key_encrypted TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            INSERT OR IGNORE INTO ai_config
                (id, enabled, base_url, model, api_key_encrypted, created_at, updated_at)
            VALUES
                (1, 0, 'https://api.deepseek.com', 'deepseek-v4-flash', '', datetime('now'), datetime('now'));

            CREATE TABLE IF NOT EXISTS ai_morning_brief (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL UNIQUE,
                markdown TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'ai' CHECK(source IN ('ai', 'rule')),
                model TEXT NOT NULL DEFAULT '',
                generated_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_ai_morning_brief_date
                ON ai_morning_brief(date);
            ",
        ),
        (
            12,
            "normalize_compact_dates",
            "
            -- 归一化 LZU 导入遗留的 YYYYMMDD 格式学期开始日期为 YYYY-MM-DD。
            -- 幂等：已标准化的日期 length=10 不满足 WHERE，二次执行无副作用。
            UPDATE courses
            SET semester_start_date =
                substr(semester_start_date, 1, 4) || '-' ||
                substr(semester_start_date, 5, 2) || '-' ||
                substr(semester_start_date, 7, 2)
            WHERE length(semester_start_date) = 8
              AND semester_start_date NOT LIKE '%-%'
              AND semester_start_date GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]';

            UPDATE semester_context
            SET start_date =
                substr(start_date, 1, 4) || '-' ||
                substr(start_date, 5, 2) || '-' ||
                substr(start_date, 7, 2)
            WHERE length(start_date) = 8
              AND start_date NOT LIKE '%-%'
              AND start_date GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]';
            ",
        ),
        (
            13,
            "retire_deepseek_chat_model",
            "
            -- DeepSeek 官方已废弃 deepseek-chat，将遗留默认配置迁移到新默认模型 deepseek-v4-flash。
            -- 幂等：migration 11 起默认即新模型，二次执行无匹配行。
            UPDATE ai_config
            SET model = 'deepseek-v4-flash', updated_at = datetime('now')
            WHERE model = 'deepseek-chat';
            ",
        ),
        (
            14,
            "daily_task_fields",
            "", // SQL 由 apply_daily_task_fields_migration 处理（add_column_if_missing 幂等加列）
        ),
    ];

    let current_version: i32 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM _migrations",
        [],
        |row| row.get(0),
    )?;

    for (version, name, sql) in migrations {
        if version > current_version {
            let tx = conn.unchecked_transaction()?;
            match version {
                3 => apply_course_week_and_exam_range_migration(&tx)?,
                4 => apply_sync_identity_and_tombstones_migration(&tx)?,
                14 => apply_daily_task_fields_migration(&tx)?,
                _ => tx.execute_batch(sql)?,
            }
            tx.execute(
                "INSERT INTO _migrations (version, name) VALUES (?1, ?2)",
                rusqlite::params![version, name],
            )?;
            tx.commit()?;
        }
    }

    Ok(())
}

/// 每日任务字段（幂等加列，避免旧库重跑 v4+ 迁移时 duplicate column）。
fn apply_daily_task_fields_migration(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "tasks", "is_daily", "is_daily INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(
        conn,
        "tasks",
        "last_completed_date",
        "last_completed_date TEXT",
    )?;
    add_column_if_missing(conn, "tasks", "reminder_time", "reminder_time TEXT")?;
    Ok(())
}

fn apply_sync_identity_and_tombstones_migration(conn: &Connection) -> Result<()> {
    for table in ["tasks", "courses", "exams", "pomodoro_sessions"] {
        add_column_if_missing(conn, table, "sync_id", "sync_id TEXT")?;
        add_column_if_missing(conn, table, "deleted_at", "deleted_at TEXT")?;
        backfill_sync_ids(conn, table)?;
        conn.execute(
            &format!(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_{table}_sync_id
                 ON {table}(sync_id)
                 WHERE sync_id IS NOT NULL AND sync_id != ''"
            ),
            [],
        )?;
    }

    add_column_if_missing(conn, "sync_config", "remote_etag", "remote_etag TEXT")?;
    add_column_if_missing(conn, "sync_config", "device_id", "device_id TEXT")?;
    add_column_if_missing(conn, "sync_config", "dataset_id", "dataset_id TEXT")?;

    backfill_sync_config_ids(conn)?;

    Ok(())
}

fn backfill_sync_ids(conn: &Connection, table: &str) -> Result<()> {
    let mut stmt = conn.prepare(&format!(
        "SELECT id FROM {table} WHERE sync_id IS NULL OR sync_id = ''"
    ))?;
    let ids = stmt
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>>>()?;

    for id in ids {
        conn.execute(
            &format!("UPDATE {table} SET sync_id = ?1 WHERE id = ?2"),
            rusqlite::params![crate::sync::ids::new_sync_id(), id],
        )?;
    }

    Ok(())
}

fn backfill_sync_config_ids(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE sync_config
         SET device_id = CASE WHEN device_id IS NULL OR device_id = '' THEN ?1 ELSE device_id END,
             dataset_id = CASE WHEN dataset_id IS NULL OR dataset_id = '' THEN ?2 ELSE dataset_id END
         WHERE id = 1",
        rusqlite::params![
            crate::sync::ids::new_sync_id(),
            crate::sync::ids::new_sync_id(),
        ],
    )?;

    Ok(())
}

fn apply_course_week_and_exam_range_migration(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "courses",
        "week_pattern",
        "week_pattern TEXT NOT NULL DEFAULT ''",
    )?;
    add_column_if_missing(
        conn,
        "courses",
        "semester_start_date",
        "semester_start_date TEXT NOT NULL DEFAULT ''",
    )?;
    add_column_if_missing(
        conn,
        "exams",
        "exam_end_datetime",
        "exam_end_datetime TEXT NOT NULL DEFAULT ''",
    )?;
    add_column_if_missing(
        conn,
        "exams",
        "semester",
        "semester TEXT NOT NULL DEFAULT ''",
    )?;
    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for existing in columns {
        if existing? == column {
            return Ok(());
        }
    }

    conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {definition}"), [])?;
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
            .query_row(
                "SELECT version FROM _migrations WHERE version = 1",
                [],
                |row| row.get(0),
            )
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
        assert_eq!(table_count, 15);

        // Verify pomodoro_config has default row
        let has_default: bool = conn
            .query_row("SELECT COUNT(*) > 0 FROM pomodoro_config", [], |row| {
                row.get(0)
            })
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

        // Should have exactly fourteen migration records applied once each.
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
            .expect("Failed to count migrations");
        assert_eq!(count, 14);
    }

    #[test]
    fn test_migration_v3_tolerates_preexisting_columns() {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("Failed to enable foreign keys");

        conn.execute_batch(
            "
            CREATE TABLE _migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            INSERT INTO _migrations (version, name) VALUES (1, 'initial_schema');
            INSERT INTO _migrations (version, name) VALUES (2, 'sync_config');

            CREATE TABLE pomodoro_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                session_type TEXT NOT NULL CHECK(session_type IN ('work', 'short_break', 'long_break')),
                task_id INTEGER,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL
            );

            CREATE TABLE tasks (
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

            CREATE TABLE courses (
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
                updated_at TEXT NOT NULL,
                week_pattern TEXT NOT NULL DEFAULT '',
                semester_start_date TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE exams (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                course_name TEXT NOT NULL,
                exam_datetime TEXT NOT NULL,
                location TEXT NOT NULL DEFAULT '',
                notes TEXT NOT NULL DEFAULT '',
                course_id INTEGER,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                exam_end_datetime TEXT NOT NULL DEFAULT '',
                semester TEXT NOT NULL DEFAULT '',
                FOREIGN KEY (course_id) REFERENCES courses(id) ON DELETE SET NULL
            );

            CREATE TABLE sync_config (
                id INTEGER PRIMARY KEY DEFAULT 1,
                server_url TEXT NOT NULL DEFAULT '',
                username TEXT NOT NULL DEFAULT '',
                password TEXT NOT NULL DEFAULT '',
                auto_sync INTEGER NOT NULL DEFAULT 0,
                last_sync_at TEXT
            );
            ",
        )
        .expect("Failed to seed preexisting schema");

        run_migrations(&conn).expect("Migration should tolerate existing v3 columns");

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
            .expect("Failed to count migrations");
        assert_eq!(count, 14);
    }

    #[test]
    fn test_migration_v4_backfills_sync_metadata() {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("Failed to enable foreign keys");

        run_migrations(&conn).expect("Migrations failed");

        conn.execute(
            "INSERT INTO sync_config (id, server_url, username, password, auto_sync)
             VALUES (1, '', '', '', 0)",
            [],
        )
        .expect("Failed to seed sync config");
        conn.execute(
            "INSERT INTO tasks (title, description, status, priority, tags, created_at, updated_at)
             VALUES ('task', '', 'todo', 'medium', '[]', '2024-01-01', '2024-01-01')",
            [],
        )
        .expect("Failed to seed task");

        // Simulate an older database that got v4 columns before values were backfilled.
        conn.execute("UPDATE tasks SET sync_id = ''", [])
            .expect("Failed to clear sync_id");
        conn.execute("UPDATE sync_config SET device_id = '', dataset_id = ''", [])
            .expect("Failed to clear config ids");
        conn.execute("DELETE FROM _migrations WHERE version >= 4", [])
            .expect("Failed to clear v4+ migration markers");

        run_migrations(&conn).expect("v4 rerun should backfill missing metadata");

        let task_sync_id: String = conn
            .query_row("SELECT sync_id FROM tasks WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("Failed to query task sync_id");
        assert!(!task_sync_id.is_empty());

        let (device_id, dataset_id): (String, String) = conn
            .query_row(
                "SELECT device_id, dataset_id FROM sync_config WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("Failed to query sync config ids");
        assert!(!device_id.is_empty());
        assert!(!dataset_id.is_empty());
    }

    #[test]
    fn test_migration_v10_creates_default_pomodoro_profiles() {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("Failed to enable foreign keys");

        run_migrations(&conn).expect("Migrations failed");

        let profiles: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT name FROM pomodoro_profiles WHERE is_builtin = 1 ORDER BY name")
                .expect("prepare profiles query");
            stmt.query_map([], |row| row.get::<_, String>(0))
                .expect("query profiles")
                .collect::<Result<Vec<_>>>()
                .expect("collect profiles")
        };

        assert_eq!(profiles, vec!["default", "intense", "relaxed"]);
    }

    #[test]
    fn test_migration_v10_term_phase_constraints() {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("Failed to enable foreign keys");

        run_migrations(&conn).expect("Migrations failed");

        let invalid_type = conn.execute(
            "INSERT INTO term_phases
                (sync_id, term_label, phase_type, start_week, end_week, created_at, updated_at)
             VALUES ('phase-1', '2026S1', 'holiday', 1, 2, '2026-01-01', '2026-01-01')",
            [],
        );
        assert!(invalid_type.is_err());

        let invalid_range = conn.execute(
            "INSERT INTO term_phases
                (sync_id, term_label, phase_type, start_week, end_week, created_at, updated_at)
             VALUES ('phase-2', '2026S1', 'break', 5, 4, '2026-01-01', '2026-01-01')",
            [],
        );
        assert!(invalid_range.is_err());
    }

    #[test]
    fn test_migration_v12_normalizes_compact_dates() {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("Failed to enable foreign keys");

        // 先建旧表 + 写入 YYYYMMDD 数据，再跑迁移验证归一化。
        conn.execute(
            "CREATE TABLE IF NOT EXISTS courses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sync_id TEXT NOT NULL DEFAULT '',
                name TEXT NOT NULL,
                day_of_week INTEGER NOT NULL,
                start_time TEXT NOT NULL,
                end_time TEXT NOT NULL,
                week_pattern TEXT NOT NULL DEFAULT '',
                semester_start_date TEXT NOT NULL DEFAULT '',
                location TEXT NOT NULL DEFAULT '',
                teacher TEXT NOT NULL DEFAULT '',
                color TEXT NOT NULL DEFAULT '',
                semester TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                deleted_at TEXT
            )",
            [],
        )
        .expect("seed preexisting courses table");

        conn.execute(
            "INSERT INTO courses (name, day_of_week, start_time, end_time, week_pattern, semester_start_date, semester, created_at, updated_at)
             VALUES ('测试课', 1, '08:00', '09:00', '', '20260309', '2026S2', '2026-01-01', '2026-01-01')",
            [],
        )
        .expect("seed compact date course");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS semester_context (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                academic_year TEXT,
                term TEXT,
                term_label TEXT NOT NULL,
                start_date TEXT NOT NULL,
                current_week INTEGER,
                total_weeks INTEGER,
                refreshed_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(source, term_label)
            )",
            [],
        )
        .expect("seed preexisting semester_context table");

        conn.execute(
            "INSERT INTO semester_context (source, term_label, start_date, refreshed_at, created_at, updated_at)
             VALUES ('lzu', '2026S2', '20260309', '2026-01-01', '2026-01-01', '2026-01-01')",
            [],
        )
        .expect("seed compact date context");

        run_migrations(&conn).expect("Migrations should normalize compact dates");

        let course_date: String = conn
            .query_row(
                "SELECT semester_start_date FROM courses WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .expect("read course date");
        assert_eq!(course_date, "2026-03-09");

        let context_date: String = conn
            .query_row(
                "SELECT start_date FROM semester_context WHERE term_label = '2026S2'",
                [],
                |row| row.get(0),
            )
            .expect("read context date");
        assert_eq!(context_date, "2026-03-09");
    }
}
