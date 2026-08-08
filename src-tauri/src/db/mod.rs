pub mod ai;
pub mod courses;
pub mod exams;
pub mod lzu_session;
pub mod migrations;
pub mod models;
pub mod notifications;
pub mod pomodoro;
pub mod pomodoro_profiles;
pub mod semester;
pub mod sync;
pub mod tasks;
pub mod term_phases;

use rusqlite::{params, Connection, Result};

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

/// Open an in-memory connection with foreign keys enabled and migrations applied.
/// Shared by every `#[cfg(test)]` module's `setup_db()` to avoid 15 identical copies.
#[cfg(test)]
pub(crate) fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("Failed to enable foreign keys");
    migrations::run_migrations(&conn).expect("Migrations failed");
    conn
}

/// Generic soft-delete: sets `deleted_at = updated_at = now` for the given row.
pub(crate) fn soft_delete(conn: &Connection, table: &str, id: i64) -> Result<()> {
    let now = chrono_now();
    conn.execute(
        &format!("UPDATE {table} SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2"),
        params![now, id],
    )?;
    Ok(())
}
