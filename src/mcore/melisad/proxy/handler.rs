/// HTTP request handling - routing dan forwarding
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use std::sync::Arc;
use std::time::Instant;

use crate::mcore::config::load_config::CONFIG;
use crate::mcore::melisad::proxy::forwarder::forward_request_with_retry;
use crate::mcore::melisad::proxy::loadbalancer::LoadBalancer;
use crate::mcore::melisad::proxy::metrics::ProxyMetrics;
use crate::mcore::melisad::services::node::NODE_MANAGER;
use crate::mcore::mlog::LOGGER;

pub async fn handle_proxy_request(
    req: Request<Incoming>,
    client: Arc<reqwest::Client>,
    load_balancer: Arc<LoadBalancer>,
    metrics: Arc<ProxyMetrics>,
    peer_addr: String,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let start = Instant::now();
    let request_id = format!("REQ-{}", uuid::Uuid::new_v4().simple());
    let (parts, body) = req.into_parts();

    // Extract metadata
    let method = parts.method.clone();
    let uri = parts.uri.clone();
    let headers = parts.headers;
    let host = headers
        .get(hyper::header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    let path = uri.path().to_string();
    let path_and_query = uri
        .path_and_query()
        .map(|path_and_query| path_and_query.as_str().to_string())
        .unwrap_or_else(|| "/".to_string());
    let body_bytes = body.collect().await?.to_bytes();

    // Try to select node via load balancer
    if let Some(target_node) = load_balancer.select_node(&host, &path, &NODE_MANAGER) {
        let upstream_node_name = format!("{} ({})", target_node.name, target_node.url);
        let _ = LOGGER.log_debug(&format!(
            "[{}] Route matched -> {}",
            request_id, upstream_node_name
        ));

        // Construct upstream URL
        let upstream_url = format!(
            "{}{}",
            target_node.url.trim_end_matches('/'),
            path_and_query
        );

        // Forward request dengan retry
        let response = forward_request_with_retry(
            &client,
            &method,
            &upstream_url,
            &headers,
            body_bytes,
            &request_id,
            CONFIG.proxy.max_retries,
            CONFIG.proxy.retry_backoff_ms,
        )
        .await;

        let duration_ms = start.elapsed().as_millis();

        match response {
            Ok(forwarded) => {
                let bytes_len = forwarded.body.len();
                metrics.record_request(bytes_len, false);

                let _ = LOGGER.log_access(
                    &peer_addr,
                    method.as_str(),
                    &path_and_query,
                    forwarded.status.as_u16(),
                    bytes_len,
                    duration_ms,
                    Some(&target_node.name),
                );

                let mut proxy_response = Response::new(Full::new(forwarded.body));
                *proxy_response.status_mut() = forwarded.status;
                proxy_response.headers_mut().extend(forwarded.headers);
                Ok(proxy_response)
            }
            Err(err) => {
                metrics.record_request(0, true);
                let _ = LOGGER.log_error(&format!(
                    "[{}] Failed to reach upstream ({}): {:?}",
                    request_id, upstream_url, err
                ));

                let _ = LOGGER.log_access(
                    &peer_addr,
                    method.as_str(),
                    &path_and_query,
                    502,
                    0,
                    duration_ms,
                    Some("error"),
                );

                // --- 1. MODIFIKASI: HTML TAMPILAN 502 (UPSTREAM TIMEOUT/DOWN) ---
                let error_html = format!(
                    "<!DOCTYPE html>\n\
                    <html lang=\"id\">\n\
                    <head>\n\
                        <meta charset=\"UTF-8\">\n\
                        <title>502 Bad Gateway - Melisa Gateway</title>\n\
                        <style>\n\
                            body {{ font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; text-align: center; padding: 100px 20px; background-color: #f8f9fa; color: #343a40; }}\n\
                            h1 {{ font-size: 48px; color: #dc3545; margin-bottom: 10px; }}\n\
                            p {{ font-size: 18px; color: #6c757d; }}\n\
                            hr {{ max-width: 600px; margin: 30px auto; border: 0; border-top: 1px solid #dee2e6; }}\n\
                            .footer {{ font-size: 12px; color: #adb5bd; }}\n\
                        </style>\n\
                    </head>\n\
                    <body>\n\
                        <h1>502 Bad Gateway</h1>\n\
                        <p>Ups! Terjadi kesalahan internal. Gateway Melisa gagal menghubungi upstream server.</p>\n\
                        <hr>\n\
                        <p class=\"footer\">Melisa Proxy Gateway/0.1.0-beta &bull; Request ID: {}</p>\n\
                    </body>\n\
                    </html>",
                    request_id
                );

                let mut error_response = Response::new(Full::new(Bytes::from(error_html)));
                *error_response.status_mut() = StatusCode::BAD_GATEWAY;

                // Set Header Content-Type ke text/html
                error_response.headers_mut().insert(
                    hyper::header::CONTENT_TYPE,
                    hyper::header::HeaderValue::from_static("text/html; charset=utf-8"),
                );
                Ok(error_response)
            }
        }
    } else {
        metrics.record_request(0, true);

        // --- 2. MODIFIKASI: CEK APAKAH ADA NODE AKTIF DI DALAM REGISTRY ---
        let total_active_nodes = NODE_MANAGER
            .processes
            .read()
            .unwrap()
            .values()
            .filter(|node| node.status == crate::mcore::melisad::services::node::NodeStatus::Active)
            .count();

        let (status_code, html_body) = if total_active_nodes == 0 {
            let _ = LOGGER.log_error(&format!(
                "[{}] No active nodes available in the framework for {}{}",
                request_id, host, path
            ));

            // Jika tidak ada node sama sekali yang aktif di sistem (Kasus All Nodes Down)
            (
                StatusCode::BAD_GATEWAY, // Nginx biasanya melempar 502 atau 503 saat upstream kosong
                format!(
                    "<!DOCTYPE html>\n\
                    <html lang=\"id\">\n\
                    <head>\n\
                        <meta charset=\"UTF-8\">\n\
                        <title>502 Bad Gateway - No Active Upstream</title>\n\
                        <style>\n\
                            body {{ font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; text-align: center; padding: 100px 20px; background-color: #f8f9fa; color: #343a40; }}\n\
                            h1 {{ font-size: 48px; color: #fd7e14; margin-bottom: 10px; }}\n\
                            p {{ font-size: 18px; color: #6c757d; }}\n\
                            hr {{ max-width: 600px; margin: 30px auto; border: 0; border-top: 1px solid #dee2e6; }}\n\
                            .footer {{ font-size: 12px; color: #adb5bd; }}\n\
                        </style>\n\
                    </head>\n\
                    <body>\n\
                        <h1>502 Bad Gateway</h1>\n\
                        <p>Melisa Proxy: Tidak ada backend pekerja (node) yang aktif saat ini untuk melayani permintaan Anda.</p>\n\
                        <hr>\n\
                        <p class=\"footer\">Melisa Proxy Gateway/0.1.0-beta &bull; Request ID: {}</p>\n\
                    </body>\n\
                    </html>",
                    request_id
                ),
            )
        } else {
            let _ = LOGGER.log_error(&format!(
                "[{}] No route found for {}{}",
                request_id, host, path
            ));

            // Jika ada node aktif, tapi tidak ada yang cocok dengan domain/path request tersebut (Kasus Murni 404)
            (
                StatusCode::NOT_FOUND,
                format!(
                    "<!DOCTYPE html>\n\
                    <html lang=\"id\">\n\
                    <head>\n\
                        <meta charset=\"UTF-8\">\n\
                        <title>404 Not Found</title>\n\
                        <style>\n\
                            body {{ font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; text-align: center; padding: 100px 20px; background-color: #f8f9fa; color: #343a40; }}\n\
                            h1 {{ font-size: 48px; color: #6c757d; margin-bottom: 10px; }}\n\
                            p {{ font-size: 18px; color: #6c757d; }}\n\
                            hr {{ max-width: 600px; margin: 30px auto; border: 0; border-top: 1px solid #dee2e6; }}\n\
                            .footer {{ font-size: 12px; color: #adb5bd; }}\n\
                        </style>\n\
                    </head>\n\
                    <body>\n\
                        <h1>404 Not Found</h1>\n\
                        <p>Halaman atau konfigurasi route yang Anda tuju tidak ditemukan pada sistem Melisa.</p>\n\
                        <hr>\n\
                        <p class=\"footer\">Melisa Proxy Gateway/0.1.0-beta &bull; Path: {} &bull; Request ID: {}</p>\n\
                    </body>\n\
                    </html>",
                    path, request_id
                ),
            )
        };

        let duration_ms = start.elapsed().as_millis();
        let _ = LOGGER.log_access(
            &peer_addr,
            method.as_str(),
            &path_and_query,
            status_code.as_u16(),
            0,
            duration_ms,
            None,
        );

        let mut response = Response::new(Full::new(Bytes::from(html_body)));
        *response.status_mut() = status_code;

        // Set Header Content-Type ke text/html
        response.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            hyper::header::HeaderValue::from_static("text/html; charset=utf-8"),
        );
        Ok(response)
    }
}
