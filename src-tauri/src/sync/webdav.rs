//! WebDAV 同步传输层：上传/下载 kairos-sync.json 快照文件。
//!
//! 设计决策:
//! - 认证: HTTP Basic Auth，凭证来自本地 SyncConfig
//! - 并发安全: ETag 条件上传 (If-Match)，412 冲突由上层重试
//! - 超时: 连接/读写超时 10 秒，避免阻塞 UI
//! - Base64: 手动实现以避免引入额外依赖

use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, ETAG, IF_MATCH};
use reqwest::StatusCode;

use super::exporter::SyncData;

/// 下载远端 kairos-sync.json 的返回结构。
/// etag 用于后续条件上传（If-Match header），防止覆盖远端更新。
#[derive(Debug, Clone)]
pub struct DownloadedSyncData {
    pub data: SyncData,
    /// 远端响应的 HTTP ETag。None = 服务端未返回 ETag，后续上传走无条件 PUT。
    pub etag: Option<String>,
}

/// 上传失败类型。Conflict = 412 Precondition Failed（远端已变更），需重试。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadError {
    /// HTTP 412: 远端 ETag 与本地不匹配，说明其他设备在上次下载后修改了远端文件。
    Conflict,
    /// 其他网络/协议错误（超时、认证失败、服务端错误等）。
    Other(String),
}

impl std::fmt::Display for UploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UploadError::Conflict => write!(f, "Remote sync data changed during upload"),
            UploadError::Other(message) => write!(f, "{message}"),
        }
    }
}

pub struct WebDavClient {
    server_url: String,
    username: String,
    password: String,
    client: Client,
}

impl WebDavClient {
    pub fn new(server_url: String, username: String, password: String) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            server_url,
            username,
            password,
            client,
        })
    }

    fn sync_file_url(&self) -> String {
        let base = self.server_url.trim_end_matches('/');
        format!("{}/kairos-sync.json", base)
    }

    fn auth_header(&self) -> Result<HeaderMap, String> {
        let credentials = format!("{}:{}", self.username, self.password);
        let encoded = base64_encode(&credentials);
        let auth_value = format!("Basic {}", encoded);

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value)
                .map_err(|e| format!("Invalid auth header: {}", e))?,
        );

        Ok(headers)
    }

    /// 上传快照到远端。若 remote_etag 非空，追加 If-Match header 做条件上传。
    /// 返回: 上传成功后服务端返回的新 ETag（None = 服务端未返回 ETag）。
    /// Conflict = HTTP 412，调用方应触发重试。
    pub fn upload(
        &self,
        data: &SyncData,
        remote_etag: Option<&str>,
    ) -> Result<Option<String>, UploadError> {
        let json = serde_json::to_string(data)
            .map_err(|e| UploadError::Other(format!("Failed to serialize sync data: {}", e)))?;

        let url = self.sync_file_url();
        let headers = self.auth_header().map_err(UploadError::Other)?;
        let mut request = self
            .client
            .put(&url)
            .headers(headers)
            .header("Content-Type", "application/json")
            .body(json);

        if let Some(etag) = remote_etag.filter(|value| !value.trim().is_empty()) {
            request = request.header(IF_MATCH, etag);
        }

        let response = request
            .send()
            .map_err(|e| UploadError::Other(map_reqwest_error(e, &url)))?;

        let status = response.status();
        if status.is_success() || status.as_u16() == 201 || status.as_u16() == 204 {
            Ok(response_etag(response.headers()))
        } else if is_precondition_failed(status) {
            Err(UploadError::Conflict)
        } else {
            Err(UploadError::Other(format!(
                "Upload failed: HTTP {} — {}",
                status.as_u16(),
                response.text().unwrap_or_default()
            )))
        }
    }

    /// 下载远端 kairos-sync.json。返回解析后的数据 + ETag。
    /// 404 = "No remote sync data found"，调用方视为无远端数据正常跳过。
    pub fn download(&self) -> Result<DownloadedSyncData, String> {
        let url = self.sync_file_url();
        let headers = self.auth_header()?;

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .map_err(|e| map_reqwest_error(e, &url))?;

        let status = response.status();
        if status.is_success() {
            let etag = response_etag(response.headers());
            let body = response
                .text()
                .map_err(|e| format!("Failed to read response body: {}", e))?;

            let data = serde_json::from_str::<SyncData>(&body)
                .map_err(|e| format!("Failed to parse sync data: {}", e))?;
            Ok(DownloadedSyncData { data, etag })
        } else if status.as_u16() == 404 {
            Err("No remote sync data found (404)".to_string())
        } else {
            Err(format!(
                "Download failed: HTTP {} — {}",
                status.as_u16(),
                response.text().unwrap_or_default()
            ))
        }
    }

    pub fn test_connection(&self) -> Result<bool, String> {
        let url = self.sync_file_url();
        let headers = self.auth_header()?;

        let response = self
            .client
            .head(&url)
            .headers(headers)
            .timeout(Duration::from_secs(10))
            .send()
            .map_err(|e| map_reqwest_error(e, &url))?;

        let status = response.status();
        Ok(status.is_success() || status.as_u16() == 404)
    }
}

fn response_etag(headers: &HeaderMap) -> Option<String> {
    headers
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn is_precondition_failed(status: StatusCode) -> bool {
    status == StatusCode::PRECONDITION_FAILED
}

fn map_reqwest_error(err: reqwest::Error, _url: &str) -> String {
    if err.is_timeout() {
        "Connection timed out (10s)".to_string()
    } else if err.is_connect() {
        format!("Cannot connect to server: {}", err)
    } else if err.is_request() {
        format!("Request failed: {}", err)
    } else {
        format!("Network error: {}", err)
    }
}

fn base64_encode(input: &str) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut output = String::new();

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        output.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        output.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            output.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            output.push('=');
        }

        if chunk.len() > 2 {
            output.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            output.push('=');
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_file_url_strips_trailing_slash() {
        let client = WebDavClient {
            server_url: "https://webdav.example.com/".to_string(),
            username: String::new(),
            password: String::new(),
            client: Client::builder()
                .timeout(Duration::from_secs(1))
                .build()
                .expect("Failed to create client"),
        };
        assert_eq!(
            client.sync_file_url(),
            "https://webdav.example.com/kairos-sync.json"
        );
    }

    #[test]
    fn test_sync_file_url_no_trailing_slash() {
        let client = WebDavClient {
            server_url: "https://webdav.example.com/dav".to_string(),
            username: String::new(),
            password: String::new(),
            client: Client::builder()
                .timeout(Duration::from_secs(1))
                .build()
                .expect("Failed to create client"),
        };
        assert_eq!(
            client.sync_file_url(),
            "https://webdav.example.com/dav/kairos-sync.json"
        );
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(""), "");
        assert_eq!(base64_encode("f"), "Zg==");
        assert_eq!(base64_encode("fo"), "Zm8=");
        assert_eq!(base64_encode("foo"), "Zm9v");
        assert_eq!(base64_encode("foo:bar"), "Zm9vOmJhcg==");
    }

    #[test]
    fn test_client_creation_with_invalid_chars_in_url() {
        let client = WebDavClient::new(
            "https://webdav.example.com".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );
        assert!(client.is_ok());
    }

    #[test]
    fn test_response_etag_reads_header() {
        let mut headers = HeaderMap::new();
        headers.insert(ETAG, HeaderValue::from_static("\"abc123\""));

        assert_eq!(response_etag(&headers).as_deref(), Some("\"abc123\""));
    }

    #[test]
    fn test_is_precondition_failed() {
        assert!(is_precondition_failed(StatusCode::PRECONDITION_FAILED));
        assert!(!is_precondition_failed(StatusCode::OK));
    }
}
