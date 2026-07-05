//! LZU HTTP client 共用的低层响应工具。

use reqwest::Response;

use crate::lzu::error::LzuError;

pub async fn read_response_body(
    response: Response,
    service: &str,
    operation: &str,
) -> Result<String, LzuError> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|_| LzuError::Network(format!("读取 LZU {service} {operation} 响应失败")))?;

    log::info!("LZU {service} {operation} response status: {status}");
    Ok(body)
}
