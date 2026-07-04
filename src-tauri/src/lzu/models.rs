//! LZU API 请求/响应数据模型。

use serde::{Deserialize, Serialize};

// ─── 登录 ───

/// 登录请求体（明文原型，将被 AES 加密后发送）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub app_os: i32,
    pub name: String,
    pub pwd: String,
}

/// 登录成功后的 token 数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginData {
    pub login_token: Option<String>,
    pub gateway_token: Option<String>,
    #[serde(default)]
    pub reset_pwd_token: Option<String>,
    #[serde(default)]
    pub need_conf_security: Option<String>,
}

/// 登录响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub code: i64,
    pub message: String,
    pub data: Option<LoginData>,
}

/// 通用字符串响应（用于登出、getSt 等）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringDataResponse {
    pub code: i64,
    pub message: String,
    pub data: Option<String>,
}

// ─── 课表 ───

/// 学期信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XlxxData {
    pub dqrqszzc: Option<String>,
    pub zzx: Option<String>,
    pub ksrq: Option<String>,
    pub xqm: Option<String>,
    pub xn: Option<String>,
    pub xq: Option<String>,
}

/// 学期信息响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XlxxResponse {
    pub code: i64,
    pub message: String,
    pub data: Option<XlxxData>,
}

/// LZU 课表课程记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseInfo {
    pub kch: Option<String>,
    pub kcmc: Option<String>,
    pub jsxm: Option<String>,
    pub jc: Option<String>,
    pub skjsl: Option<String>,
    pub skxql: Option<String>,
    pub week: Option<String>,
    pub bs: Option<String>,
    pub xykh: Option<String>,
    pub xn: Option<String>,
    pub xqm: Option<i64>,
    pub status: Option<i64>,
    pub color: Option<String>,
    pub sksj: Option<String>,
    pub xf: Option<String>,
    pub week_fb: Option<String>,
    pub kcrq: Option<String>,
    pub create_time: Option<String>,
    pub create_user_id: Option<String>,
}

/// 课表响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleResponse {
    pub code: i64,
    pub message: String,
    pub data: Option<Vec<CourseInfo>>,
}

// ─── 认证状态 ───

/// LZU 登录会话状态（后端运行期内存状态）。
///
/// 不持久化到磁盘，不暴露给前端。
#[derive(Debug, Clone)]
pub struct LzuSession {
    /// 登录用户名
    pub username: String,
    /// 登录 token，用于 getSt 等需参数传递的接口
    pub login_token: String,
    /// 网关 token，用于大部分接口的 Authorization header
    pub gateway_token: String,
    /// 最近一次刷新的服务票据，仅供后端内部链路使用
    pub st: Option<String>,
}

/// 前端可见的登录状态摘要（不包含 token 原文）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    pub is_logged_in: bool,
    pub username: Option<String>,
}

impl From<&Option<LzuSession>> for AuthStatus {
    fn from(session: &Option<LzuSession>) -> Self {
        match session {
            Some(s) => AuthStatus {
                is_logged_in: true,
                username: Some(s.username.clone()),
            },
            None => AuthStatus {
                is_logged_in: false,
                username: None,
            },
        }
    }
}
