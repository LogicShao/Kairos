//! LZU AppService HTTP client 封装。
//!
//! 提供对 `https://appservice.lzu.edu.cn` 的基础 HTTP 请求能力。
//! 每个请求显式控制加密行为，不修改全局 client 状态。

use crate::lzu::crypto;
use crate::lzu::error::LzuError;
use crate::lzu::models::{LoginResponse, ScheduleResponse, UserInfoResponse, XlxxResponse};
use crate::lzu::services::ServiceDirectoryApiResponse;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::Client;
use serde::de::DeserializeOwned;

/// User-Agent 与 FasterLZU 保持一致
const USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 12; SM-S7110 Build/V417IR; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/101.0.4951.61 Mobile Safari/537.36 lzdx_ua JHZF_LZDXAPP";

/// AppService base URL
const BASE_URL: &str = "https://appservice.lzu.edu.cn";

/// 登录接口路径
const LOGIN_PATH: &str = "/api/eusp-unify-terminal/app-user/login";

/// 登出接口路径
const LOGOUT_PATH: &str = "/api/eusp-unify-terminal/app-user/logout";

/// 获取学期信息接口路径
const XLXX_PATH: &str = "/api/lzu-teaching-research/kcb/getXlxx";

/// 获取课表接口路径
const SCHEDULE_PATH: &str = "/api/lzu-teaching-research/kcb/getZdyCourse";

/// 获取 ST 接口路径
const GET_ST_PATH: &str = "/api/eusp-unify-terminal/app-user/getSt";

/// 获取用户资料接口路径
const USER_INFO_PATH: &str = "/api/eusp-unify-terminal/app-user/userInfo";

/// 获取服务目录接口路径
const SERVICE_DIRECTORY_PATH: &str =
    "/api/eusp-terminal-management/api/v2/getServiceInfoDetailByTerminalRole";

/// AppService HTTP client。
///
/// 每个实例持有独立的 `reqwest::Client`，不共享全局可变状态。
pub struct AppServiceClient {
    client: Client,
}

impl AppServiceClient {
    /// 创建新的 AppService client。
    pub fn new() -> Result<Self, LzuError> {
        let mut default_headers = HeaderMap::new();
        default_headers.insert("User-Agent", HeaderValue::from_static(USER_AGENT));
        default_headers.insert("Host", HeaderValue::from_static("appservice.lzu.edu.cn"));
        default_headers.insert("Connection", HeaderValue::from_static("Keep-Alive"));
        default_headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json;charset=UTF-8"),
        );
        default_headers.insert("Accept-Encoding", HeaderValue::from_static("gzip"));

        let client = Client::builder()
            .default_headers(default_headers)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| LzuError::Network(format!("创建 HTTP client 失败: {e}")))?;

        Ok(AppServiceClient { client })
    }

    /// 登录 LZU AppService。
    ///
    /// 加密 body 并发送 POST 请求。响应先尝试直接 JSON 解析，失败则 AES 解密后再解析。
    pub async fn login(
        &self,
        request: &crate::lzu::models::LoginRequest,
    ) -> Result<LoginResponse, LzuError> {
        let plaintext = serde_json::to_string(request)?;
        let body = self
            .post_encrypted_body(LOGIN_PATH, &plaintext, "", None, "login")
            .await?;
        parse_appservice_response(&body)
    }

    /// 获取 LZU 学期信息。
    pub async fn get_xlxx(&self, gateway_token: &str) -> Result<XlxxResponse, LzuError> {
        let response = self
            .client
            .post(format!("{BASE_URL}{XLXX_PATH}"))
            .header("Authorization", gateway_token)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await?;

        let body = crate::lzu::http::read_response_body(response, "AppService", "get_xlxx").await?;
        parse_appservice_response(&body)
    }

    /// 获取 LZU 当前登录用户资料。
    pub async fn user_info(
        &self,
        login_token: &str,
        gateway_token: &str,
    ) -> Result<UserInfoResponse, LzuError> {
        let response = self
            .client
            .get(format!("{BASE_URL}{USER_INFO_PATH}"))
            .header("Authorization", gateway_token)
            .header("Content-Type", "text/plain")
            .query(&[("loginToken", login_token)])
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LzuError::Timeout("userInfo 请求超时".to_string())
                } else {
                    LzuError::Network("userInfo 请求失败".to_string())
                }
            })?;

        let body =
            crate::lzu::http::read_response_body(response, "AppService", "user_info").await?;
        parse_appservice_response(&body)
    }

    /// 获取当前登录账号可见的服务目录原始响应。
    pub async fn get_service_directory(
        &self,
        login_token: &str,
    ) -> Result<ServiceDirectoryApiResponse, LzuError> {
        let plaintext = format!("terminalId=1&loginToken={login_token}");
        let body = self
            .post_encrypted_body(
                SERVICE_DIRECTORY_PATH,
                &plaintext,
                "",
                Some("application/x-www-form-urlencoded"),
                "get_service_directory",
            )
            .await?;
        parse_appservice_response(&body)
    }

    /// 获取指定教学周课表。
    pub async fn get_schedule(
        &self,
        gateway_token: &str,
        week_index: i64,
    ) -> Result<ScheduleResponse, LzuError> {
        let response = self
            .client
            .get(format!("{BASE_URL}{SCHEDULE_PATH}"))
            .header("Authorization", gateway_token)
            .header("Content-Type", "text/plain")
            .query(&[("zc", week_index), ("qsbz", 0)])
            .send()
            .await?;

        let body =
            crate::lzu::http::read_response_body(response, "AppService", "get_schedule").await?;
        parse_appservice_response(&body)
    }

    /// 登出 LZU AppService。
    ///
    /// 发送加密的 `loginToken=<token>` body。
    pub async fn logout(
        &self,
        login_token: &str,
        gateway_token: &str,
    ) -> Result<crate::lzu::models::StringDataResponse, LzuError> {
        let plaintext = format!("loginToken={login_token}");
        let body = self
            .post_encrypted_body(
                LOGOUT_PATH,
                &plaintext,
                gateway_token,
                Some("application/x-www-form-urlencoded"),
                "logout",
            )
            .await?;
        parse_appservice_response(&body)
    }

    /// 刷新 Service Ticket。返回值只供后端保存和后续服务链路使用。
    pub async fn get_st(
        &self,
        login_token: &str,
        service_id: &str,
    ) -> Result<crate::lzu::models::StringDataResponse, LzuError> {
        // getSt 把 loginToken 放在 query 中，传输错误不能直接透出 reqwest 原文。
        let response = self
            .client
            .get(format!("{BASE_URL}{GET_ST_PATH}"))
            .query(&[
                ("loginToken", login_token),
                ("serviceId", service_id),
                ("service", ""),
            ])
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LzuError::Timeout("getSt 请求超时".to_string())
                } else {
                    LzuError::Network("getSt 请求失败".to_string())
                }
            })?;
        let body = crate::lzu::http::read_response_body(response, "AppService", "get_st").await?;
        parse_appservice_response(&body)
    }

    async fn post_encrypted_body(
        &self,
        path: &str,
        plaintext: &str,
        authorization: &str,
        content_type: Option<&'static str>,
        operation: &str,
    ) -> Result<String, LzuError> {
        let encrypted = crypto::encrypt(plaintext)?;
        let mut request = self
            .client
            .post(format!("{BASE_URL}{path}"))
            .header("Authorization", authorization)
            .header("Transfer-Encrypt", "true")
            .body(encrypted);

        if let Some(content_type) = content_type {
            request = request.header("Content-Type", content_type);
        }

        crate::lzu::http::read_response_body(request.send().await?, "AppService", operation).await
    }
}

fn parse_appservice_response<T>(body: &str) -> Result<T, LzuError>
where
    T: DeserializeOwned,
{
    if let Ok(parsed) = serde_json::from_str::<T>(body) {
        return Ok(parsed);
    }

    let decrypted = crypto::decrypt(body)?;
    Ok(serde_json::from_str::<T>(&decrypted)?)
}
