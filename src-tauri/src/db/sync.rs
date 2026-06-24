use rusqlite::{params, Connection, Result};

use super::models::SyncConfig;

pub fn get_sync_config(conn: &Connection) -> Result<SyncConfig> {
    let result = conn.query_row(
        "SELECT id, server_url, username, password, auto_sync, last_sync_at
         FROM sync_config WHERE id = 1",
        [],
        |row| {
            Ok(SyncConfig {
                id: row.get(0)?,
                server_url: row.get(1)?,
                username: row.get(2)?,
                password: row.get(3)?,
                auto_sync: row.get::<_, i64>(4)? != 0,
                last_sync_at: row.get(5)?,
            })
        },
    );

    match result {
        Ok(config) => Ok(config),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let default = SyncConfig {
                id: 1,
                server_url: String::new(),
                username: String::new(),
                password: String::new(),
                auto_sync: false,
                last_sync_at: None,
            };
            conn.execute(
                "INSERT INTO sync_config (id, server_url, username, password, auto_sync, last_sync_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    default.id,
                    default.server_url,
                    default.username,
                    default.password,
                    default.auto_sync as i64,
                    default.last_sync_at,
                ],
            )?;
            Ok(default)
        }
        Err(e) => Err(e),
    }
}

pub fn update_sync_config(conn: &Connection, config: &SyncConfig) -> Result<()> {
    conn.execute(
        "UPDATE sync_config
         SET server_url = ?1, username = ?2, password = ?3, auto_sync = ?4, last_sync_at = ?5
         WHERE id = ?6",
        params![
            config.server_url,
            config.username,
            config.password,
            config.auto_sync as i64,
            config.last_sync_at,
            config.id,
        ],
    )?;
    Ok(())
}

pub fn update_last_sync_at(conn: &Connection, timestamp: &str) -> Result<()> {
    conn.execute(
        "UPDATE sync_config SET last_sync_at = ?1 WHERE id = 1",
        params![timestamp],
    )?;
    Ok(())
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

        let config = get_sync_config(&conn).expect("Failed to get config");
        assert_eq!(config.id, 1);
        assert_eq!(config.server_url, "");
        assert_eq!(config.username, "");
        assert_eq!(config.password, "");
        assert!(!config.auto_sync);
        assert!(config.last_sync_at.is_none());

        let config2 = get_sync_config(&conn).expect("Failed to get config again");
        assert_eq!(config.id, config2.id);
    }

    #[test]
    fn test_update_sync_config() {
        let conn = setup_db();

        let _ = get_sync_config(&conn).expect("Failed to get initial config");

        let updated = SyncConfig {
            id: 1,
            server_url: "https://webdav.example.com".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            auto_sync: true,
            last_sync_at: Some("2024-06-01T10:00:00Z".to_string()),
        };
        update_sync_config(&conn, &updated).expect("Failed to update config");

        let config = get_sync_config(&conn).expect("Failed to get updated config");
        assert_eq!(config.server_url, "https://webdav.example.com");
        assert_eq!(config.username, "user");
        assert_eq!(config.password, "pass");
        assert!(config.auto_sync);
        assert_eq!(config.last_sync_at.as_deref(), Some("2024-06-01T10:00:00Z"));
    }

    #[test]
    fn test_update_last_sync_at() {
        let conn = setup_db();

        let _ = get_sync_config(&conn).expect("Failed to get initial config");

        update_last_sync_at(&conn, "2024-07-01T12:00:00Z").expect("Failed to update last_sync_at");

        let config = get_sync_config(&conn).expect("Failed to get config");
        assert_eq!(config.last_sync_at.as_deref(), Some("2024-07-01T12:00:00Z"));
    }
}
