use rusqlite::{params, Connection, Result};

use super::models::{AiConfig, AiMorningBrief, UpdateAiConfigRequest};

/// 获取 AI 配置（单例，id=1）。
/// 若表中尚无记录（首次迁移未执行），插入迁移默认值并返回。
pub fn get_ai_config(conn: &Connection) -> Result<AiConfig> {
    let result = conn.query_row(
        "SELECT id, enabled, base_url, model, api_key_encrypted, sync_enabled, created_at, updated_at
         FROM ai_config WHERE id = 1",
        [],
        |row| {
            Ok(AiConfig {
                id: row.get(0)?,
                enabled: row.get::<_, i64>(1)? != 0,
                base_url: row.get(2)?,
                model: row.get(3)?,
                api_key_encrypted: row.get(4)?,
                sync_enabled: row.get::<_, i64>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    );

    match result {
        Ok(config) => Ok(config),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let now = super::chrono_now();
            let default = AiConfig {
                id: 1,
                enabled: false,
                base_url: String::from("https://api.deepseek.com"),
                model: String::from("deepseek-v4-flash"),
                api_key_encrypted: String::new(),
                sync_enabled: false,
                created_at: now.clone(),
                updated_at: now,
            };
            conn.execute(
                "INSERT INTO ai_config (id, enabled, base_url, model, api_key_encrypted, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    default.id,
                    default.enabled as i64,
                    default.base_url,
                    default.model,
                    default.api_key_encrypted,
                    default.created_at,
                    default.updated_at,
                ],
            )?;
            Ok(default)
        }
        Err(e) => Err(e),
    }
}

/// 更新 AI 配置（单例，id=1）。
/// 仅合并传入了 Some 的字段；`new_key_encrypted` 语义：
/// - `Some(cipher)` 替换密文（命令层已用随机 IV 加密新 key）
/// - `Some("")` 清空 key
/// - `None` 保留当前密文
pub fn update_ai_config(
    conn: &Connection,
    req: &UpdateAiConfigRequest,
    new_key_encrypted: Option<String>,
) -> Result<()> {
    let current = get_ai_config(conn)?;

    let enabled = req.enabled.unwrap_or(current.enabled) as i64;
    let base_url = req.base_url.as_deref().unwrap_or(&current.base_url);
    let model = req.model.as_deref().unwrap_or(&current.model);
    let api_key_encrypted = match new_key_encrypted {
        Some(cipher) => cipher,
        None => current.api_key_encrypted,
    };
    let sync_enabled = req.sync_enabled.unwrap_or(current.sync_enabled) as i64;
    let updated_at = super::chrono_now();

    conn.execute(
        "UPDATE ai_config
         SET enabled = ?1, base_url = ?2, model = ?3, api_key_encrypted = ?4, sync_enabled = ?5, updated_at = ?6
         WHERE id = 1",
        params![enabled, base_url, model, api_key_encrypted, sync_enabled, updated_at],
    )?;
    Ok(())
}

/// 同步路径专用：整体覆盖 AI 配置并**保留传入的 updated_at**。
///
/// 与 `update_ai_config` 的差异：后者总是打上 `chrono_now()`，而 LWW 同步必须保留
/// 胜者一方的 updated_at（否则每次同步都会把时间戳拨到"现在"，破坏跨设备胜负判定）。
/// 不修改 sync_enabled —— 该开关属于各设备本地偏好，不进同步包。
pub fn apply_synced_config(
    conn: &Connection,
    enabled: bool,
    base_url: &str,
    model: &str,
    api_key_encrypted: &str,
    updated_at: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE ai_config
         SET enabled = ?1, base_url = ?2, model = ?3, api_key_encrypted = ?4, updated_at = ?5
         WHERE id = 1",
        params![
            enabled as i64,
            base_url,
            model,
            api_key_encrypted,
            updated_at,
        ],
    )?;
    Ok(())
}

/// 读取指定日期的今日摘要；无记录返回 `None`。
pub fn get_morning_brief(conn: &Connection, date: &str) -> Result<Option<AiMorningBrief>> {
    let result = conn.query_row(
        "SELECT id, date, markdown, source, model, generated_at, created_at, updated_at
         FROM ai_morning_brief WHERE date = ?1",
        params![date],
        |row| {
            Ok(AiMorningBrief {
                id: row.get(0)?,
                date: row.get(1)?,
                markdown: row.get(2)?,
                source: row.get(3)?,
                model: row.get(4)?,
                generated_at: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    );

    match result {
        Ok(brief) => Ok(Some(brief)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// 写入今日摘要（date 冲突则更新内容，保留 id）。返回落库后的完整记录。
pub fn upsert_morning_brief(conn: &Connection, brief: &AiMorningBrief) -> Result<AiMorningBrief> {
    let now = super::chrono_now();
    conn.execute(
        "INSERT INTO ai_morning_brief (date, markdown, source, model, generated_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(date) DO UPDATE SET
            markdown = excluded.markdown,
            source = excluded.source,
            model = excluded.model,
            generated_at = excluded.generated_at,
            updated_at = excluded.updated_at",
        params![
            brief.date,
            brief.markdown,
            brief.source,
            brief.model,
            brief.generated_at,
            now,
            now,
        ],
    )?;

    match get_morning_brief(conn, &brief.date)? {
        Some(saved) => Ok(saved),
        // 刚 upsert 成功却查不到，属逻辑不变量被破坏。
        None => Err(rusqlite::Error::QueryReturnedNoRows),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::setup_db;

    #[test]
    fn test_get_ai_config_returns_migration_default() {
        let conn = setup_db();

        let config = get_ai_config(&conn).expect("Failed to get config");
        assert_eq!(config.id, 1);
        assert!(!config.enabled);
        assert_eq!(config.base_url, "https://api.deepseek.com");
        assert_eq!(config.model, "deepseek-v4-flash");
        assert!(config.api_key_encrypted.is_empty());
    }

    #[test]
    fn test_update_ai_config_partial_keeps_others() {
        let conn = setup_db();

        let req = UpdateAiConfigRequest {
            enabled: Some(true),
            model: Some(String::from("deepseek-v4-flash")),
            ..Default::default()
        };
        update_ai_config(&conn, &req, None).expect("Failed to update config");

        let config = get_ai_config(&conn).expect("Failed to get updated config");
        assert!(config.enabled);
        assert_eq!(config.model, "deepseek-v4-flash");
        // base_url 与 key 未动
        assert_eq!(config.base_url, "https://api.deepseek.com");
        assert!(config.api_key_encrypted.is_empty());
    }

    #[test]
    fn test_update_ai_config_key_set_clear_keep() {
        let conn = setup_db();

        // 写入新 key
        update_ai_config(
            &conn,
            &UpdateAiConfigRequest::default(),
            Some(String::from("cipher-v1")),
        )
        .expect("Failed to set key");
        assert_eq!(get_ai_config(&conn).unwrap().api_key_encrypted, "cipher-v1");

        // None 保留
        update_ai_config(&conn, &UpdateAiConfigRequest::default(), None)
            .expect("Failed to keep key");
        assert_eq!(get_ai_config(&conn).unwrap().api_key_encrypted, "cipher-v1");

        // Some("") 清空
        update_ai_config(
            &conn,
            &UpdateAiConfigRequest::default(),
            Some(String::new()),
        )
        .expect("Failed to clear key");
        assert!(get_ai_config(&conn).unwrap().api_key_encrypted.is_empty());
    }

    #[test]
    fn test_morning_brief_roundtrip_and_upsert() {
        let conn = setup_db();

        assert!(get_morning_brief(&conn, "2026-07-31")
            .expect("Failed to query")
            .is_none());

        let brief = AiMorningBrief {
            id: 0,
            date: "2026-07-31".to_string(),
            markdown: "# 2026-07-31 晨间摘要\n## 今日重点\n...".to_string(),
            source: "ai".to_string(),
            model: "deepseek-v4-flash".to_string(),
            generated_at: "2026-07-31T07:00:00Z".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
        };
        let saved = upsert_morning_brief(&conn, &brief).expect("Failed to upsert");
        assert!(saved.id > 0);
        assert_eq!(saved.source, "ai");

        // date 冲突时更新内容，id 保留
        let updated = AiMorningBrief {
            markdown: "updated".to_string(),
            source: "rule".to_string(),
            ..brief
        };
        let resaved = upsert_morning_brief(&conn, &updated).expect("Failed to re-upsert");
        assert_eq!(resaved.id, saved.id, "upsert 应保留 id");
        assert_eq!(resaved.markdown, "updated");
        assert_eq!(resaved.source, "rule");

        let fetched = get_morning_brief(&conn, "2026-07-31")
            .expect("Failed to query")
            .expect("Should exist");
        assert_eq!(fetched.markdown, "updated");
    }

    #[test]
    fn test_get_morning_brief_other_date_none() {
        let conn = setup_db();

        let brief = AiMorningBrief {
            id: 0,
            date: "2026-07-31".to_string(),
            markdown: "m".to_string(),
            source: "ai".to_string(),
            model: "deepseek-v4-flash".to_string(),
            generated_at: "2026-07-31T07:00:00Z".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
        };
        upsert_morning_brief(&conn, &brief).expect("Failed to upsert");

        assert!(get_morning_brief(&conn, "2026-08-01")
            .expect("Failed to query")
            .is_none());
    }
}
