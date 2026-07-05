//! LZU 认证管理。
//!
//! 封装登录、登出、token 管理逻辑。token 仅存于运行期内存，不持久化到磁盘。

use std::sync::Arc;

use crate::lzu::appservice::AppServiceClient;
use crate::lzu::error::LzuError;
use crate::lzu::models::{LzuProfileSummary, LzuSession};

/// LZU 认证管理器。
///
/// 维护运行期 session 状态，不持久化到磁盘。
/// 前端只能通过 `AuthStatus` 结构获取登录状态摘要。
pub struct LzuAuth {
    /// AppService HTTP client（Arc 包装以便廉价 clone）
    pub client: Arc<AppServiceClient>,
    /// 当前登录会话（None 表示未登录）
    pub session: Option<LzuSession>,
}

impl LzuAuth {
    /// 创建新的认证管理器。
    pub fn new(client: AppServiceClient) -> Self {
        LzuAuth {
            client: Arc::new(client),
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
}

/// 线程安全的 LZU 认证管理器，可注册为 Tauri 管理状态。
pub type SharedLzuAuth = std::sync::Mutex<LzuAuth>;

/// 创建共享的 LZU 认证管理器实例。
pub fn create_shared_auth() -> Result<SharedLzuAuth, LzuError> {
    let client = AppServiceClient::new()?;
    let auth = LzuAuth::new(client);
    Ok(std::sync::Mutex::new(auth))
}
