pub mod courses;
pub mod exams;
pub mod migrations;
pub mod models;
pub mod notifications;
pub mod pomodoro;
pub mod sync;
pub mod tasks;

use rusqlite::{Connection, Result};

/// Open a SQLite connection, enable WAL mode and foreign keys, and run migrations.
pub fn get_connection(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrations::run_migrations(&conn)?;
    Ok(conn)
}

/// Return the current UTC time as an ISO 8601 string.
pub(crate) fn chrono_now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
