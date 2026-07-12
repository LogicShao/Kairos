//! LZU 登录会话持久化。
//!
//! 将 login_token / gateway_token / profile 存入本地 SQLite，
//! 应用重启时自动恢复，避免每次启动都重新登录。
//! 不纳入 WebDAV 同步。

use rusqlite::{params, Connection, Result};

use crate::lzu::models::LzuProfileSummary;

/// 持久化的会话行。
#[derive(Debug, Clone)]
pub struct LzuSessionRow {
    pub username: String,
    pub login_token: String,
    pub gateway_token: String,
    pub profile_json: String,
}

pub fn upsert_session(
    conn: &Connection,
    username: &str,
    login_token: &str,
    gateway_token: &str,
    profile: Option<&LzuProfileSummary>,
) -> Result<()> {
    let now = super::chrono_now();
    let profile_json = serde_json::to_string(&profile).unwrap_or_else(|_| "{}".to_string());
    conn.execute(
        "INSERT INTO lzu_session (id, username, login_token, gateway_token, profile_json, created_at, updated_at)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?5)
         ON CONFLICT(id) DO UPDATE SET
            username = excluded.username,
            login_token = excluded.login_token,
            gateway_token = excluded.gateway_token,
            profile_json = excluded.profile_json,
            updated_at = excluded.updated_at",
        params![username, login_token, gateway_token, profile_json, now],
    )?;
    Ok(())
}

pub fn get_session(conn: &Connection) -> Result<Option<LzuSessionRow>> {
    match conn.query_row(
        "SELECT username, login_token, gateway_token, profile_json FROM lzu_session WHERE id = 1",
        [],
        |row| {
            Ok(LzuSessionRow {
                username: row.get(0)?,
                login_token: row.get(1)?,
                gateway_token: row.get(2)?,
                profile_json: row.get(3)?,
            })
        },
    ) {
        Ok(row) => Ok(Some(row)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn delete_session(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM lzu_session WHERE id = 1", [])?;
    Ok(())
}
