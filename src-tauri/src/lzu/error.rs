//! LZU 模块错误类型。

use std::fmt;

/// LZU API 操作中可能出现的错误分类。
#[derive(Debug)]
pub enum LzuError {
    /// 网络请求失败（DNS、连接被拒、TLS 等）
    Network(String),
    /// 请求超时
    Timeout(String),
    /// AES 加解密或 hex 编解码失败
    Crypto(String),
    /// JSON 序列化/反序列化失败
    Json(String),
    /// API 返回的业务错误（code != 1）
    Api { code: i64, message: String },
    /// 本地配置缺失或格式不正确
    Config(String),
    /// 未登录或 token 不可用
    NotLoggedIn,
    /// 登录失败（用户名或密码错误等）
    LoginFailed(String),
    /// 其他内部错误
    Internal(String),
}

impl fmt::Display for LzuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LzuError::Network(msg) => write!(f, "网络错误: {msg}"),
            LzuError::Timeout(msg) => write!(f, "请求超时: {msg}"),
            LzuError::Crypto(msg) => write!(f, "加密/解密错误: {msg}"),
            LzuError::Json(msg) => write!(f, "数据解析错误: {msg}"),
            LzuError::Api { code, message } => {
                write!(f, "服务返回异常 (code={code}): {message}")
            }
            LzuError::Config(msg) => write!(f, "LZU 配置错误: {msg}"),
            LzuError::NotLoggedIn => write!(f, "尚未登录"),
            LzuError::LoginFailed(msg) => write!(f, "登录失败: {msg}"),
            LzuError::Internal(msg) => write!(f, "内部错误: {msg}"),
        }
    }
}

impl std::error::Error for LzuError {}

impl From<serde_json::Error> for LzuError {
    fn from(e: serde_json::Error) -> Self {
        LzuError::Json(e.to_string())
    }
}

impl From<reqwest::Error> for LzuError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            LzuError::Timeout(e.to_string())
        } else if e.is_connect() {
            LzuError::Network(e.to_string())
        } else {
            LzuError::Network(e.to_string())
        }
    }
}
