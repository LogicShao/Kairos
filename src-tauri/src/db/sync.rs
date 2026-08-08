use rusqlite::{params, Connection, Result};

use super::models::SyncConfig;

/// 获取同步配置。若 sync_config 表尚无记录（首次启动），插入默认值并返回。
/// device_id 和 dataset_id 初始化为新的 UUID，用于 trace 和数据集分组。
pub fn get_sync_config(conn: &Connection) -> Result<SyncConfig> {
    let result = conn.query_row(
        "SELECT id, server_url, username, password, auto_sync, last_sync_at, remote_etag, device_id, dataset_id, ai_settings_remote_etag
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
                remote_etag: row.get(6)?,
                device_id: row.get(7)?,
                dataset_id: row.get(8)?,
                ai_settings_remote_etag: row.get(9)?,
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
                remote_etag: None,
                device_id: Some(crate::sync::ids::new_sync_id()),
                dataset_id: Some(crate::sync::ids::new_sync_id()),
                ai_settings_remote_etag: None,
            };
            conn.execute(
                "INSERT INTO sync_config (id, server_url, username, password, auto_sync, last_sync_at, remote_etag, device_id, dataset_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    default.id,
                    default.server_url,
                    default.username,
                    default.password,
                    default.auto_sync as i64,
                    default.last_sync_at,
                    default.remote_etag,
                    default.device_id,
                    default.dataset_id,
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
         SET server_url = ?1, username = ?2, password = ?3, auto_sync = ?4
         WHERE id = ?5",
        params![
            config.server_url,
            config.username,
            config.password,
            config.auto_sync as i64,
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

pub fn update_remote_etag(conn: &Connection, etag: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE sync_config SET remote_etag = ?1 WHERE id = 1",
        params![etag],
    )?;
    Ok(())
}

/// 记录 AI 设置加密包文件的上次上传 ETag（独立于主快照）。
pub fn update_ai_settings_remote_etag(conn: &Connection, etag: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE sync_config SET ai_settings_remote_etag = ?1 WHERE id = 1",
        params![etag],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::setup_db;

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
    fn test_update_sync_config_preserves_protocol_metadata() {
        let conn = setup_db();

        let initial = get_sync_config(&conn).expect("Failed to get initial config");
        update_last_sync_at(&conn, "2024-07-01T12:00:00Z").expect("Failed to set last_sync_at");
        update_remote_etag(&conn, Some("\"etag-existing\"")).expect("Failed to set remote_etag");

        let updated = SyncConfig {
            id: 1,
            server_url: "https://webdav.example.com".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            auto_sync: true,
            last_sync_at: Some("stale-frontend-value".to_string()),
            remote_etag: Some("\"stale-etag\"".to_string()),
            device_id: Some("stale-device".to_string()),
            dataset_id: Some("stale-dataset".to_string()),
            ai_settings_remote_etag: None,
        };
        update_sync_config(&conn, &updated).expect("Failed to update config");

        let config = get_sync_config(&conn).expect("Failed to get updated config");
        assert_eq!(config.server_url, "https://webdav.example.com");
        assert_eq!(config.username, "user");
        assert_eq!(config.password, "pass");
        assert!(config.auto_sync);
        assert_eq!(config.last_sync_at.as_deref(), Some("2024-07-01T12:00:00Z"));
        assert_eq!(config.remote_etag.as_deref(), Some("\"etag-existing\""));
        assert_eq!(config.device_id, initial.device_id);
        assert_eq!(config.dataset_id, initial.dataset_id);
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
