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

// ─── 用户资料 ───

/// LZU 用户资料响应中的白名单字段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfoData {
    #[serde(default)]
    xm: Option<String>,
    #[serde(default)]
    rybh: Option<String>,
    #[serde(default)]
    dwmc: Option<String>,
    #[serde(default)]
    rylb: Option<String>,
    #[serde(default)]
    xykh: Option<String>,
}

/// LZU 用户资料响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfoResponse {
    pub code: i64,
    pub message: String,
    pub data: Option<UserInfoData>,
}

/// 前端可见的 LZU 身份摘要，只包含低敏白名单字段。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LzuProfileSummary {
    /// 姓名，用于登录身份确认。
    pub display_name: Option<String>,
    /// 人员编号；资料缺失时回退到登录用户名。
    pub person_no: Option<String>,
    /// 学院、部门或单位名称。
    pub department: Option<String>,
    /// 人员类别。
    pub role: Option<String>,
    /// 校园卡号后 4 位；不返回完整卡号。
    pub campus_card_tail: Option<String>,
}

impl LzuProfileSummary {
    pub fn from_user_info(data: &UserInfoData, fallback_username: &str) -> Self {
        LzuProfileSummary {
            display_name: normalized_optional(&data.xm),
            person_no: normalized_optional(&data.rybh)
                .or_else(|| normalized_str(fallback_username)),
            department: normalized_optional(&data.dwmc),
            role: normalized_optional(&data.rylb),
            campus_card_tail: campus_card_tail(&data.xykh),
        }
    }
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
    /// 当前登录账号的低敏身份摘要。
    pub profile: Option<LzuProfileSummary>,
}

/// 前端可见的登录状态摘要（不包含 token 原文）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    pub is_logged_in: bool,
    pub username: Option<String>,
    pub profile: Option<LzuProfileSummary>,
}

impl From<&Option<LzuSession>> for AuthStatus {
    fn from(session: &Option<LzuSession>) -> Self {
        match session {
            Some(s) => AuthStatus {
                is_logged_in: true,
                username: Some(s.username.clone()),
                profile: s.profile.clone(),
            },
            None => AuthStatus {
                is_logged_in: false,
                username: None,
                profile: None,
            },
        }
    }
}

fn normalized_optional(value: &Option<String>) -> Option<String> {
    value.as_deref().and_then(normalized_str)
}

fn normalized_str(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn campus_card_tail(value: &Option<String>) -> Option<String> {
    let card = value.as_deref()?.trim();
    let chars: Vec<char> = card.chars().collect();
    if chars.len() < 4 {
        return None;
    }
    Some(chars[chars.len() - 4..].iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_profile_summary_keeps_only_whitelisted_fields() {
        let data: UserInfoData = serde_json::from_value(json!({
            "xm": " 张三 ",
            "rybh": " 20260001 ",
            "dwmc": " 计算机学院 ",
            "rylb": " 本科生 ",
            "xykh": "320230941441",
            "sfzjh": "620000199901011234",
            "yddh": "13800000000",
            "dzxx": "student@example.com"
        }))
        .expect("valid userInfo fixture");

        let summary = LzuProfileSummary::from_user_info(&data, "fallback");

        assert_eq!(
            summary,
            LzuProfileSummary {
                display_name: Some("张三".to_string()),
                person_no: Some("20260001".to_string()),
                department: Some("计算机学院".to_string()),
                role: Some("本科生".to_string()),
                campus_card_tail: Some("1441".to_string()),
            }
        );

        let serialized = serde_json::to_string(&summary).expect("summary serializes");
        assert!(!serialized.contains("sfzjh"));
        assert!(!serialized.contains("yddh"));
        assert!(!serialized.contains("dzxx"));
        assert!(!serialized.contains("320230941441"));
    }

    #[test]
    fn test_profile_summary_uses_username_fallback_and_hides_short_card() {
        let data: UserInfoData = serde_json::from_value(json!({
            "xm": "",
            "rybh": " ",
            "xykh": "123"
        }))
        .expect("valid userInfo fixture");

        let summary = LzuProfileSummary::from_user_info(&data, "  fallback_user  ");

        assert_eq!(summary.display_name, None);
        assert_eq!(summary.person_no, Some("fallback_user".to_string()));
        assert_eq!(summary.campus_card_tail, None);
    }
}
