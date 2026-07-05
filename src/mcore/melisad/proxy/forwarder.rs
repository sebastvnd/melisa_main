// mcore/melisad/proxy/forwarder.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

use crate::mcore::mlog::LOGGER;
use hyper::body::Bytes;
use hyper::{HeaderMap, Method, StatusCode};
use std::time::Duration;

pub struct ForwardedResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

/// Forward HTTP request ke upstream dengan automatic retry
pub async fn forward_request_with_retry(
    client: &reqwest::Client,
    method: &Method,
    upstream_url: &str,
    headers: &HeaderMap,
    body: Bytes,
    request_id: &str,
    max_retries: u32,
    retry_backoff_ms: u64,
) -> Result<ForwardedResponse, Box<dyn std::error::Error + Send + Sync>> {
    let reqwest_method = reqwest::Method::from_bytes(method.as_str().as_bytes())?;
    let max_attempts = max_retries.saturating_add(1).max(1);
    let mut attempt = 1;

    loop {
        let mut request = client
            .request(reqwest_method.clone(), upstream_url)
            .body(body.clone());

        for (name, value) in headers.iter() {
            if should_skip_request_header(name.as_str()) {
                continue;
            }

            if let Ok(value) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
                request = request.header(name.as_str(), value);
            }
        }

        match request.send().await {
            Ok(res) => {
                if res.status().is_server_error() && attempt < max_attempts {
                    let _ = LOGGER.log_debug(&format!(
                        "[{}] Retry {}/{} for {} (upstream status: {})",
                        request_id,
                        attempt,
                        max_retries,
                        upstream_url,
                        res.status()
                    ));

                    tokio::time::sleep(backoff_duration(retry_backoff_ms, attempt)).await;
                    attempt += 1;
                    continue;
                }

                let status = StatusCode::from_u16(res.status().as_u16())?;
                let response_headers = copy_response_headers(res.headers());
                let body_bytes = res.bytes().await?;

                return Ok(ForwardedResponse {
                    status,
                    headers: response_headers,
                    body: body_bytes,
                });
            }
            Err(err) if attempt < max_attempts => {
                let _ = LOGGER.log_debug(&format!(
                    "[{}] Retry {}/{} for {} (reason: {})",
                    request_id, attempt, max_retries, upstream_url, err
                ));

                tokio::time::sleep(backoff_duration(retry_backoff_ms, attempt)).await;
                attempt += 1;
                continue;
            }
            Err(err) => {
                return Err(Box::new(err));
            }
        }
    }
}

fn backoff_duration(retry_backoff_ms: u64, attempt: u32) -> Duration {
    Duration::from_millis(retry_backoff_ms.saturating_mul(attempt as u64))
}

fn copy_response_headers(headers: &reqwest::header::HeaderMap) -> HeaderMap {
    let mut copied = HeaderMap::new();

    for (name, value) in headers.iter() {
        if should_skip_response_header(name.as_str()) {
            continue;
        }

        let header_name = hyper::header::HeaderName::from_bytes(name.as_str().as_bytes());
        let header_value = hyper::header::HeaderValue::from_bytes(value.as_bytes());

        if let (Ok(header_name), Ok(header_value)) = (header_name, header_value) {
            copied.append(header_name, header_value);
        }
    }

    copied
}

fn should_skip_request_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "content-length"
            | "host"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

fn should_skip_response_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "content-length"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_forward_request_logic() {
        // Test placeholder - actual integration tests should use mock server
        assert_eq!(1, 1);
    }
}
