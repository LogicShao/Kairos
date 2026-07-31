//! DeepSeek Provider：OpenAI 兼容 `POST /v1/chat/completions`，非流式（本轮不做 SSE）。

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::ai::prompt;
use crate::ai::{AIService, AiBriefResult, AiError, BriefSource};
use crate::commands::briefing::TodayBriefingResponse;
use crate::db::models::AiConfig;

/// 请求超时（深色网络下单次生成上限）。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

pub struct DeepseekProvider {
    client: reqwest::blocking::Client,
    base_url: String,
    model: String,
    api_key: String,
}

impl DeepseekProvider {
    /// 从配置构建 Provider。`key` 为 `ai/crypto` 的 AES 文件密钥，用于解密密文。
    pub fn new(config: &AiConfig, key: &[u8; 16]) -> Result<Self, AiError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| AiError::Network(format!("创建 HTTP client 失败: {e}")))?;
        let api_key = crate::ai::crypto::decrypt_api_key(&config.api_key_encrypted, key)
            .map_err(AiError::Network)?;
        Ok(Self {
            client,
            base_url: config.base_url.clone(),
            model: config.model.clone(),
            api_key,
        })
    }
}

impl AIService for DeepseekProvider {
    fn generate_daily_brief(
        &self,
        briefing: &TodayBriefingResponse,
    ) -> Result<AiBriefResult, AiError> {
        let user_message = prompt::build_user_message(briefing).map_err(AiError::Parse)?;

        let body = ChatCompletionRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: prompt::SYSTEM_PROMPT.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_message,
                },
            ],
            temperature: prompt::TEMPERATURE,
            max_tokens: prompt::MAX_TOKENS,
            stream: false,
        };

        // base_url 不含 /v1，此处统一追加（避免 /v1/v1）。
        let url = format!(
            "{}/v1/chat/completions",
            self.base_url.trim_end_matches('/')
        );
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .map_err(map_reqwest_error)?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(AiError::Http {
                status: status.as_u16(),
                body,
            });
        }

        let parsed: ChatCompletionResponse = response
            .json()
            .map_err(|e| AiError::Parse(format!("解析 AI 响应失败: {e}")))?;
        let content = parsed
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AiError::Parse("AI 响应 choices 为空".to_string()))?
            .message
            .content;

        Ok(AiBriefResult {
            summary: content,
            source: BriefSource::Ai,
            generated_at: crate::db::chrono_now(),
            model: Some(self.model.clone()),
        })
    }
}

// ─── OpenAI 兼容 DTO ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

/// 按 error-handling.md 约定把 reqwest 错误映射为 `AiError`。
fn map_reqwest_error(err: reqwest::Error) -> AiError {
    if err.is_timeout() {
        AiError::Timeout
    } else if err.is_connect() {
        AiError::Network(format!("无法连接 AI 服务: {err}"))
    } else if err.is_request() {
        AiError::Network(format!("请求失败: {err}"))
    } else {
        AiError::Network(format!("网络错误: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_completion_request_serialization() {
        let body = ChatCompletionRequest {
            model: "deepseek-chat".to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: prompt::SYSTEM_PROMPT.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: "data".to_string(),
                },
            ],
            temperature: prompt::TEMPERATURE,
            max_tokens: prompt::MAX_TOKENS,
            stream: false,
        };

        let json = serde_json::to_value(&body).expect("serialize request");
        assert_eq!(json["model"], "deepseek-chat");
        assert!((json["temperature"].as_f64().unwrap() - 0.7).abs() < 1e-6);
        assert_eq!(json["max_tokens"], 400);
        assert_eq!(json["stream"], false);
        assert_eq!(json["messages"].as_array().unwrap().len(), 2);
        assert_eq!(json["messages"][0]["role"], "system");
    }

    #[test]
    fn test_url_appends_v1_once() {
        // 验证 base_url 不含 /v1 时拼接正确，且 trim 尾部斜杠不会重复。
        let url = |base: &str| format!("{}/v1/chat/completions", base.trim_end_matches('/'));
        assert_eq!(
            url("https://api.deepseek.com"),
            "https://api.deepseek.com/v1/chat/completions"
        );
        assert_eq!(
            url("https://api.deepseek.com/"),
            "https://api.deepseek.com/v1/chat/completions"
        );
    }
}
