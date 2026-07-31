//! AI 服务抽象与分发（AI Morning Brief）。
//!
//! # 设计决策
//!
//! - 输出同构：AI 与规则引擎都返回 `AiBriefResult`，`source` 区分来源（`ai`/`rule`）。
//! - 降级点：命令层 `resolve_service()` 决定走 AI 还是规则；AI 调用任何失败一律回退规则引擎。
//! - 成本护栏：`max_tokens=400` 硬编码（见 `prompt`），数据裁剪减 token，按日缓存 + in-flight 防重复计费。
//!
//! 详细设计见 `.trellis/tasks/07-31-ai-morning-brief/design.md`。

pub mod crypto;
pub mod deepseek;
pub mod morning_brief;
pub mod prompt;
pub mod rule;
pub mod scheduler;

use serde::Serialize;

use crate::commands::briefing::TodayBriefingResponse;
use crate::db::models::AiConfig;

/// 摘要生成来源。DB CHECK、serde 序列化、前端字面量三处取值一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BriefSource {
    /// deepseek AI 生成。
    Ai,
    /// 本地规则引擎生成（未启用 / 离线 / 失败降级）。
    Rule,
}

/// 摘要生成结果（AI 版与规则版同构）。
#[derive(Debug, Clone, Serialize)]
pub struct AiBriefResult {
    /// 摘要 markdown 正文（固定 5 分节 + 来源 footer）。
    pub summary: String,
    /// 来源："ai" 或 "rule"。
    pub source: BriefSource,
    /// 生成时间，RFC3339。
    pub generated_at: String,
    /// 仅 source=Ai 时有模型名，如 "deepseek-chat"。
    pub model: Option<String>,
}

/// AI 调用失败类型。命令层负责转换为用户可见的 String（见 error-handling.md）。
#[derive(Debug)]
pub enum AiError {
    /// 客户端侧请求超时（15s）。
    Timeout,
    /// 网络连接/请求失败。
    Network(String),
    /// HTTP 非 2xx，含状态码与服务端响应体。
    Http { status: u16, body: String },
    /// 响应 JSON 解析失败或 choices 为空。
    Parse(String),
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::Timeout => write!(f, "AI 请求超时，请稍后重试"),
            AiError::Network(msg) => write!(f, "网络错误: {msg}"),
            AiError::Http { status, body } => match *status {
                401 => write!(f, "API key 无效，请在 AI 设置中重新配置"),
                429 => write!(f, "请求过于频繁，请稍后重试"),
                _ => write!(f, "AI 服务返回错误 {status}: {body}"),
            },
            AiError::Parse(msg) => write!(f, "AI 响应解析失败: {msg}"),
        }
    }
}

/// AI 服务抽象。入参为已聚合的 `TodayBriefingResponse`，实现方不接触 DB/锁。
pub trait AIService: Send + Sync {
    fn generate_daily_brief(
        &self,
        briefing: &TodayBriefingResponse,
    ) -> Result<AiBriefResult, AiError>;
}

/// 分发器：未启用或无 key → `None`（命令层走规则引擎）。
///
/// `key` 为 `ai/crypto` 的 16 字节 AES 文件密钥，用于解密 DB 中的 key 密文。
pub fn resolve_service(config: &AiConfig, key: &[u8; 16]) -> Option<Box<dyn AIService>> {
    if config.enabled && !config.api_key_encrypted.is_empty() {
        deepseek::DeepseekProvider::new(config, key)
            .ok()
            .map(|p| Box::new(p) as Box<dyn AIService>)
    } else {
        None
    }
}
