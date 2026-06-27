use rusqlite::{params, Connection, Result};

use super::models::{NotificationConfig, UpdateNotificationConfig};

/// 获取通知配置（单例，id=1）。
/// 若表中尚无记录（首次迁移未执行），插入默认值并返回。
pub fn get_notification_config(conn: &Connection) -> Result<NotificationConfig> {
    let result = conn.query_row(
        "SELECT id, enabled, exam_offsets_json, android_channel_created, created_at, updated_at
         FROM notification_config WHERE id = 1",
        [],
        |row| {
            Ok(NotificationConfig {
                id: row.get(0)?,
                enabled: row.get::<_, i64>(1)? != 0,
                exam_offsets_json: row.get(2)?,
                android_channel_created: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    );

    match result {
        Ok(config) => Ok(config),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let now = super::chrono_now();
            let default = NotificationConfig {
                id: 1,
                enabled: true,
                exam_offsets_json: String::from("[1440,60]"),
                android_channel_created: false,
                created_at: now.clone(),
                updated_at: now,
            };
            conn.execute(
                "INSERT INTO notification_config (id, enabled, exam_offsets_json, android_channel_created, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    default.id,
                    default.enabled as i64,
                    default.exam_offsets_json,
                    default.android_channel_created as i64,
                    default.created_at,
                    default.updated_at,
                ],
            )?;
            Ok(default)
        }
        Err(e) => Err(e),
    }
}

/// 更新通知配置（单例，id=1）。
/// 仅更新传入了 Some 的字段，自动设置 updated_at 为当前 UTC 时间。
pub fn update_notification_config(conn: &Connection, req: &UpdateNotificationConfig) -> Result<()> {
    // 先获取当前配置，用于合并可选字段
    let current = get_notification_config(conn)?;

    let enabled = req.enabled.unwrap_or(current.enabled) as i64;
    let exam_offsets_json = req
        .exam_offsets_json
        .as_deref()
        .unwrap_or(&current.exam_offsets_json);
    let android_channel_created = req
        .android_channel_created
        .unwrap_or(current.android_channel_created) as i64;
    let updated_at = super::chrono_now();

    conn.execute(
        "UPDATE notification_config
         SET enabled = ?1, exam_offsets_json = ?2, android_channel_created = ?3, updated_at = ?4
         WHERE id = 1",
        params![
            enabled,
            exam_offsets_json,
            android_channel_created,
            updated_at
        ],
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
    fn test_get_notification_config_returns_migration_default() {
        let conn = setup_db();

        let config = get_notification_config(&conn).expect("Failed to get config");
        assert_eq!(config.id, 1);
        assert!(config.enabled);
        assert_eq!(config.exam_offsets_json, "[1440,60]");
        assert!(!config.android_channel_created);
    }

    #[test]
    fn test_get_notification_config_idempotent() {
        let conn = setup_db();

        let config1 = get_notification_config(&conn).expect("Failed to get config");
        let config2 = get_notification_config(&conn).expect("Failed to get config again");
        assert_eq!(config1.enabled, config2.enabled);
        assert_eq!(config1.exam_offsets_json, config2.exam_offsets_json);
    }

    #[test]
    fn test_update_notification_config_full() {
        let conn = setup_db();

        let req = UpdateNotificationConfig {
            enabled: Some(false),
            exam_offsets_json: Some(String::from("[30,10]")),
            android_channel_created: Some(true),
        };
        update_notification_config(&conn, &req).expect("Failed to update config");

        let config = get_notification_config(&conn).expect("Failed to get updated config");
        assert!(!config.enabled);
        assert_eq!(config.exam_offsets_json, "[30,10]");
        assert!(config.android_channel_created);
    }

    #[test]
    fn test_update_notification_config_partial() {
        let conn = setup_db();

        // Only update enabled, keep other fields as-is
        let req = UpdateNotificationConfig {
            enabled: Some(false),
            exam_offsets_json: None,
            android_channel_created: None,
        };
        update_notification_config(&conn, &req).expect("Failed to update config");

        let config = get_notification_config(&conn).expect("Failed to get updated config");
        assert!(!config.enabled);
        // Default values should be preserved
        assert_eq!(config.exam_offsets_json, "[1440,60]");
        assert!(!config.android_channel_created);
    }

    #[test]
    fn test_update_notification_config_updates_updated_at() {
        let conn = setup_db();

        let old = get_notification_config(&conn).expect("Failed to get config");

        std::thread::sleep(std::time::Duration::from_secs(1));

        let req = UpdateNotificationConfig {
            enabled: Some(false),
            exam_offsets_json: None,
            android_channel_created: None,
        };
        update_notification_config(&conn, &req).expect("Failed to update config");

        let new = get_notification_config(&conn).expect("Failed to get updated config");
        assert_ne!(
            old.updated_at, new.updated_at,
            "updated_at should change after update"
        );
    }
}
