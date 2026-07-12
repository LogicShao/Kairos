//! LZU 认证管理。
//!
//! 封装登录、登出、token 管理逻辑。token 仅存于运行期内存，不持久化到磁盘。

use std::sync::Arc;

use rusqlite::Connection;

use crate::lzu::appservice::AppServiceClient;
use crate::lzu::easytong::{EasyTongClient, EasyTongSession};
use crate::lzu::error::LzuError;
use crate::lzu::models::{LzuProfileSummary, LzuSession};

/// LZU 认证管理器。
///
/// 维护运行期 session 状态，不持久化到磁盘。
/// 前端只能通过 `AuthStatus` 结构获取登录状态摘要。
pub struct LzuAuth {
    /// AppService HTTP client（Arc 包装以便廉价 clone）
    pub client: Arc<AppServiceClient>,
    /// EasyTong HTTP client（Arc 包装以便廉价 clone）
    pub easytong_client: Arc<EasyTongClient>,
    /// 当前登录会话（None 表示未登录）
    pub session: Option<LzuSession>,
}

impl LzuAuth {
    /// 创建新的认证管理器。
    pub fn new(client: AppServiceClient, easytong_client: EasyTongClient) -> Self {
        LzuAuth {
            client: Arc::new(client),
            easytong_client: Arc::new(easytong_client),
            session: None,
        }
    }

    /// 设置登录 session（由 command 层在登录成功后调用）。
    pub fn set_session(
        &mut self,
        username: String,
        login_token: String,
        gateway_token: String,
        profile: Option<LzuProfileSummary>,
    ) {
        self.session = Some(LzuSession {
            username,
            login_token,
            gateway_token,
            st: None,
            easytong: None,
            profile,
        });
    }

    /// 更新后端内部使用的服务票据。
    pub fn set_st(&mut self, st: String) -> Result<(), LzuError> {
        match &mut self.session {
            Some(session) => {
                session.st = Some(st);
                Ok(())
            }
            None => Err(LzuError::NotLoggedIn),
        }
    }

    /// 更新当前 session 的低敏身份摘要。
    pub fn set_profile(&mut self, profile: Option<LzuProfileSummary>) -> Result<(), LzuError> {
        match &mut self.session {
            Some(session) => {
                session.profile = profile;
                Ok(())
            }
            None => Err(LzuError::NotLoggedIn),
        }
    }

    /// 更新后端内部使用的 EasyTong 会话。
    pub fn set_easytong_session(&mut self, easytong: EasyTongSession) -> Result<(), LzuError> {
        match &mut self.session {
            Some(session) => {
                session.easytong = Some(easytong);
                Ok(())
            }
            None => Err(LzuError::NotLoggedIn),
        }
    }

    /// 持久化当前会话到 SQLite。
    pub fn persist(&self, conn: &Connection) -> Result<(), LzuError> {
        match &self.session {
            Some(s) => {
                crate::db::lzu_session::upsert_session(
                    conn,
                    &s.username,
                    &s.login_token,
                    &s.gateway_token,
                    s.profile.as_ref(),
                )
                .map_err(|e| LzuError::Internal(format!("保存登录状态失败: {e}")))?;
                Ok(())
            }
            None => {
                crate::db::lzu_session::delete_session(conn)
                    .map_err(|e| LzuError::Internal(format!("清除登录状态失败: {e}")))?;
                Ok(())
            }
        }
    }

    /// 从 SQLite 恢复会话（仅内存，不写回）。
    pub fn restore_from_db(&mut self, conn: &Connection) {
        if let Ok(Some(row)) = crate::db::lzu_session::get_session(conn) {
            let profile: Option<LzuProfileSummary> =
                serde_json::from_str(&row.profile_json).ok().flatten();
            self.session = Some(LzuSession {
                username: row.username,
                login_token: row.login_token,
                gateway_token: row.gateway_token,
                st: None,
                easytong: None,
                profile,
            });
        }
    }
}

/// 线程安全的 LZU 认证管理器，可注册为 Tauri 管理状态。
pub type SharedLzuAuth = std::sync::Mutex<LzuAuth>;

/// 创建共享的 LZU 认证管理器实例。
pub fn create_shared_auth() -> Result<SharedLzuAuth, LzuError> {
    let client = AppServiceClient::new()?;
    let easytong_client = EasyTongClient::new()?;
    let auth = LzuAuth::new(client, easytong_client);
    Ok(std::sync::Mutex::new(auth))
}
