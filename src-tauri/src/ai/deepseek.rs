//! DeepSeek Provider：OpenAI 兼容 `POST /v1/chat/completions`。
//! 手动触发生成走 SSE 流式（`stream:true`），7:00 定时走非流式（`stream:false`）。

use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;

use crate::ai::prompt;
use crate::ai::{AIService, AiBriefResult, AiError, BriefSource};
use crate::commands::briefing::TodayBriefingResponse;
use crate::db::models::AiConfig;

/// 请求超时（深色网络下单次生成上限）。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// 用于流式/异步路径的 Provider（不包含 blocking client，避免 async ctx 使用）。
pub struct AsyncDeepseekProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
    api_key: String,
}

impl AsyncDeepseekProvider {
    /// 仅构造异步 client，安全在 async 上下文中调用。
    pub fn new(config: &AiConfig, key: &[u8; 16]) -> Result<Self, AiError> {
        let api_key = crate::ai::crypto::decrypt_api_key(&config.api_key_encrypted, key)
            .map_err(AiError::Network)?;
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| AiError::Network(format!("创建 HTTP client 失败: {e}")))?;
        Ok(Self {
            client,
            base_url: config.base_url.clone(),
            model: config.model.clone(),
            api_key,
        })
    }

    /// 流式生成：SSE 逐 chunk 经 channel 推送到前端，返回累积完整文本。
    pub async fn generate_daily_brief_streaming(
        &self,
        briefing: &TodayBriefingResponse,
        channel: &Channel<StreamChunk>,
    ) -> Result<String, AiError> {
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
            stream: true,
        };

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
            .await
            .map_err(map_reqwest_error)?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AiError::Http {
                status: status.as_u16(),
                body,
            });
        }

        let text = collect_sse_stream(response, channel).await?;

        // 流式取到内容 → 直接返回。部分第三方中转 SSE 实现不稳定（空内容），
        // 回退非流式请求。
        if !text.is_empty() {
            return Ok(text);
        }

        log::warn!("SSE 流式返回空内容，回退非流式请求");
        self.generate_daily_brief_non_streaming(briefing)
            .await
    }

    /// 非流式回退：stream=false 发送请求，返回完整响应文本。
    /// 第三方中转 API 在非流式模式下通常更可靠。
    async fn generate_daily_brief_non_streaming(
        &self,
        briefing: &TodayBriefingResponse,
    ) -> Result<String, AiError> {
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
            .await
            .map_err(map_reqwest_error)?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AiError::Http {
                status: status.as_u16(),
                body,
            });
        }

        let parsed: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| AiError::Parse(format!("解析 AI 非流式响应失败: {e}")))?;
        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| AiError::Parse("AI 响应 choices 为空".to_string()))
    }
}

pub struct DeepseekProvider {
    client: reqwest::blocking::Client,
    base_url: String,
    model: String,
    api_key: String,
}

impl DeepseekProvider {
    /// 仅构建 blocking client，在同步上下文（7:00 定时）中调用。
    /// 运行时探测 tokio 上下文：若在 async 运行时内调用，返回错误而非硬 panic。
    pub fn new(config: &AiConfig, key: &[u8; 16]) -> Result<Self, AiError> {
        // tokio 为 reqwest 传递依赖，此处运行时探测作为安全护栏：
        // 在活跃 tokio 运行时内构建 blocking client 的 blocking pool 会 panic
        // （"Cannot block the current thread from within a runtime"），
        // 故极早期拦截，返回有意义的错误而非崩溃。
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(AiError::Network(
                "不能在异步上下文中调用 blocking AI 生成器".to_string(),
            ));
        }
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

// ─── SSE 流式解析 ────────────────────────────────────────────────────────────────

/// 流式输出 chunk（经 Tauri Channel 推送到前端，camelCase 序列化）。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamChunk {
    pub delta: String,
}

/// OpenAI SSE chunk 中的增量片段（仅取 delta.content）。
#[derive(Deserialize)]
struct ChatCompletionChunk {
    choices: Vec<ChunkChoice>,
}

#[derive(Deserialize)]
struct ChunkChoice {
    delta: ChunkDelta,
}

#[derive(Deserialize)]
struct ChunkDelta {
    content: Option<String>,
}

/// 从 SSE 事件流累积 delta.content 并逐块推送 channel，返回累积完整文本。
///
/// 按行解析（不依赖空行分隔，兼容 LF/CRLF）：SSE 每个事件是一行 `data: {json}`。
/// 若服务端忽略 `stream:true` 返回完整 JSON，则按非流式 DTO 兜底解析。
async fn collect_sse_stream(
    response: reqwest::Response,
    channel: &Channel<StreamChunk>,
) -> Result<String, AiError> {
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let content_encoding = response
        .headers()
        .get(reqwest::header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let mut stream = response.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();
    let mut full = String::new();
    let mut data_lines = 0usize;
    let mut parsed_deltas = 0usize;
    let mut first_data_line: Option<String> = None;
    // 失败诊断：记录前 N 行原始内容（无论是否 data: 开头）。
    let mut raw_lines: Vec<String> = Vec::new();
    const MAX_RAW_LINES: usize = 10;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(map_reqwest_error)?;
        buf.extend_from_slice(&chunk);

        // 切出完整行（以 \n 结尾）逐行解析；残留部分等下个 chunk。
        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=pos).collect();
            let line_text = String::from_utf8_lossy(&line);
            let trimmed = line_text.trim_end_matches(['\r', '\n']);
            if !trimmed.is_empty() && raw_lines.len() < MAX_RAW_LINES {
                raw_lines.push(trimmed.chars().take(160).collect());
            }
            if let Some(data) = trimmed.trim_start().strip_prefix("data:") {
                data_lines += 1;
                if first_data_line.is_none() {
                    first_data_line = Some(data.trim().chars().take(200).collect());
                }
            }
            if let Some(delta) = handle_sse_line(trimmed, &mut full) {
                parsed_deltas += 1;
                let _ = channel.send(StreamChunk { delta });
            }
        }
    }

    // 流结束后的残留行（最后一行可能无 \n 结尾）。
    if !buf.is_empty() {
        let line_text = String::from_utf8_lossy(&buf);
        let trimmed = line_text.trim_end_matches(['\r', '\n']);
        if !trimmed.is_empty() && raw_lines.len() < MAX_RAW_LINES {
            raw_lines.push(trimmed.chars().take(160).collect());
        }
        if let Some(data) = trimmed.trim_start().strip_prefix("data:") {
            data_lines += 1;
            if first_data_line.is_none() {
                first_data_line = Some(data.trim().chars().take(200).collect());
            }
        }
        if let Some(delta) = handle_sse_line(trimmed, &mut full) {
            parsed_deltas += 1;
            let _ = channel.send(StreamChunk { delta });
        }
    }

    // 兜底：未解析出任何 delta 且响应体是 JSON → 按非流式 DTO 解析一次。
    if full.is_empty() && !buf.is_empty() {
        let text = String::from_utf8_lossy(&buf);
        if text.trim_start().starts_with('{') {
            if let Ok(parsed) = serde_json::from_str::<ChatCompletionResponse>(&text) {
                if let Some(content) = parsed.choices.into_iter().next() {
                    full = content.message.content;
                    let _ = channel.send(StreamChunk { delta: full.clone() });
                }
            }
        }
    }

    if full.is_empty() {
        if data_lines > 0 && parsed_deltas == 0 {
            log::warn!(
                "SSE 收到 {data_lines} 个 data 行但全部解析失败（content-type={content_type}, content-encoding={content_encoding}，首个 data: {first:?}）",
                first = first_data_line
            );
        } else if !raw_lines.is_empty() {
            log::warn!(
                "SSE 流式响应前 {n} 行（{content_type}, {content_encoding}）:\n{lines}",
                n = raw_lines.len(),
                lines = raw_lines.join("\n")
            );
        } else {
            log::warn!(
                "SSE 流式响应未解析出内容（content-type={content_type}, content-encoding={content_encoding}，残留{}字节，收到 {} 个 chunk）",
                buf.len(),
                data_lines
            );
        }
    }

    Ok(full)
}

/// 处理一行 SSE 文本：若是 `data:` 行则解析 delta 并追加到 `full`。
/// 返回推送的 delta（供 channel 转发）；非 data 行 / 结束标记 / 无内容返回 `None`。
fn handle_sse_line(line: &str, full: &mut String) -> Option<String> {
    let data = line.trim_start().strip_prefix("data:")?.trim();
    if data == "[DONE]" {
        return None;
    }
    let delta = parse_sse_delta(data)?;
    full.push_str(&delta);
    Some(delta)
}

/// 解析单个 SSE `data:` 行 JSON，提取 `choices[0].delta.content`。
/// 非法 JSON 或该 chunk 无文本增量返回 `None`。
fn parse_sse_delta(data: &str) -> Option<String> {
    serde_json::from_str::<ChatCompletionChunk>(data)
        .ok()?
        .choices
        .first()?
        .delta
        .content
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocking_provider_rejects_async_context() {
        // 回归：在 tokio 异步上下文中构造 blocking client 会硬 panic
        // （"Cannot drop a runtime in a context where blocking is not allowed"）。
        // 运行时探测应在构建 client 前拦截，返回错误而非崩溃。
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("build current_thread runtime");
        let config = crate::db::models::AiConfig {
            id: 1,
            enabled: true,
            base_url: "https://api.deepseek.com".to_string(),
            model: "deepseek-v4-flash".to_string(),
            api_key_encrypted: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
        };
        let result = rt.block_on(async { DeepseekProvider::new(&config, &[0u8; 16]) });
        assert!(result.is_err(), "async 上下文应被探测拦截而非 panic");
    }

    #[test]
    fn test_chat_completion_request_serialization() {
        let body = ChatCompletionRequest {
            model: "deepseek-v4-flash".to_string(),
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
        assert_eq!(json["model"], "deepseek-v4-flash");
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

    #[test]
    fn test_parse_sse_delta_extracts_content() {
        assert_eq!(
            parse_sse_delta(r#"{"choices":[{"delta":{"content":"你好"}}]}"#),
            Some("你好".to_string())
        );
    }

    #[test]
    fn test_parse_sse_delta_none_for_no_content() {
        // 无 content 增量（如 role 标记）或非法 JSON → None。
        assert_eq!(
            parse_sse_delta(r#"{"choices":[{"delta":{"role":"assistant"}}]}"#),
            None
        );
        assert_eq!(parse_sse_delta("not json"), None);
    }

    #[test]
    fn test_handle_sse_line_extracts_delta() {
        let mut full = String::new();
        // data 行 → 提取 delta 并追加。
        assert_eq!(
            handle_sse_line(r#"data: {"choices":[{"delta":{"content":"你好"}}]}"#, &mut full),
            Some("你好".to_string())
        );
        assert_eq!(full, "你好");
        // 空行 / 注释行 / [DONE] / 无 content 的 data 行 → 不追加。
        assert_eq!(handle_sse_line("", &mut full), None);
        assert_eq!(handle_sse_line(":keep-alive", &mut full), None);
        assert_eq!(handle_sse_line("data: [DONE]", &mut full), None);
        assert_eq!(handle_sse_line(r#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#, &mut full), None);
        assert_eq!(full, "你好");
    }

    #[test]
    fn test_handle_sse_line_multiple_deltas_accumulate() {
        let mut full = String::new();
        handle_sse_line(r#"data: {"choices":[{"delta":{"content":"今天"}}]}"#, &mut full);
        handle_sse_line(r#"data: {"choices":[{"delta":{"content":"很棒"}}]}"#, &mut full);
        assert_eq!(full, "今天很棒");
    }
}
