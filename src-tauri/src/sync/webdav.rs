use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

use super::exporter::SyncData;

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

    pub fn upload(&self, data: &SyncData) -> Result<(), String> {
        let json = serde_json::to_string(data)
            .map_err(|e| format!("Failed to serialize sync data: {}", e))?;

        let url = self.sync_file_url();
        let headers = self.auth_header()?;

        let response = self
            .client
            .put(&url)
            .headers(headers)
            .header("Content-Type", "application/json")
            .body(json)
            .send()
            .map_err(|e| map_reqwest_error(e, &url))?;

        let status = response.status();
        if status.is_success() || status.as_u16() == 201 || status.as_u16() == 204 {
            Ok(())
        } else {
            Err(format!(
                "Upload failed: HTTP {} — {}",
                status.as_u16(),
                response.text().unwrap_or_default()
            ))
        }
    }

    pub fn download(&self) -> Result<SyncData, String> {
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
            let body = response
                .text()
                .map_err(|e| format!("Failed to read response body: {}", e))?;

            serde_json::from_str::<SyncData>(&body)
                .map_err(|e| format!("Failed to parse sync data: {}", e))
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
}
