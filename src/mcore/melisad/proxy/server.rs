// mcore/melisad/proxy/server.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::service::service_fn;
use hyper::{Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

use crate::mcore::config::load_config::{CONFIG, MAX_CONCURRENT_CONNECTIONS};
use crate::mcore::melisad::proxy::handler::handle_proxy_request;
use crate::mcore::melisad::proxy::loadbalancer::{LoadBalancer, LoadBalancingStrategy};
use crate::mcore::melisad::proxy::metrics::ProxyMetrics;
use crate::mcore::mlog::LOGGER;

// TODO Tambah endpoint baru untuk monitoring

// GET /api/nodes/status → List semua nodes dengan statusnya
// GET /api/nodes/suspicious → List nodes yang dicurigai
// GET /api/nodes/dead → List nodes yang sudah offline > timeout
// POST /api/nodes/cleanup → Manual cleanup expired nodes

pub async fn run_proxy_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize
    let addr = format!("{}:{}", CONFIG.host, CONFIG.port);
    let listener = TcpListener::bind(&addr).await?;
    let _ = LOGGER.log_info(&format!("Melisa proxy listening on {}", addr));

    println!("Melisa proxy listening on http://{}", addr);
    println!(
        "Load balancer: {}, node file: {}",
        CONFIG.proxy.load_balancer_strategy, CONFIG.nodes.storage_file
    );

    // Setup HTTP client dengan connection pooling
    let client = Arc::new(
        reqwest::Client::builder()
            .pool_max_idle_per_host(CONFIG.proxy.max_idle_per_host)
            .timeout(Duration::from_secs(CONFIG.proxy.request_timeout_secs))
            .redirect(reqwest::redirect::Policy::none())
            .no_gzip()
            .no_brotli()
            .no_zstd()
            .no_deflate()
            .build()?,
    );

    // Setup load balancer
    let load_balancer = Arc::new(match CONFIG.proxy.load_balancer_strategy.as_str() {
        // "least_connections" => LoadBalancer::new(LoadBalancingStrategy::LeastConnections),
        "random" => LoadBalancer::new(LoadBalancingStrategy::Random),
        _ => LoadBalancer::new(LoadBalancingStrategy::RoundRobin),
    });

    // Metrics
    let metrics = Arc::new(ProxyMetrics::new());
    let metrics_clone = metrics.clone();

    // Spawn metrics reporter
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(
                CONFIG.proxy.metrics_report_interval_secs.max(1),
            ))
            .await;
            metrics_clone.log_metrics();
        }
    });

    let global_limit = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));

    // Main accept loop
    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let peer_addr = peer_addr.to_string();

        metrics.increment_active();

        let metrics_clone = metrics.clone();
        let client_clone = client.clone();
        let lb_clone = load_balancer.clone();
        let global_limit_clone = global_limit.clone();

        tokio::spawn(async move {
            let _permit = match global_limit_clone.try_acquire_owned() {
                Ok(permit) => permit,
                Err(_) => {
                    // SERVER OVERLOAD! Lakukan Load Shedding secara elegan via HTTP 503
                    let _ = LOGGER.log_error(&format!(
                        "Load Shedding: Server overloaded. Serving HTTP 503 to Klien {}",
                        peer_addr
                    ));

                    // Buat service darurat untuk mengirimkan sinyal 503 Service Unavailable
                    let emergency_svc = service_fn(|_req| async {
                        let html_503 = "<html><head><title>503 Overloaded</title></head>\
                                            <body style='font-family:sans-serif; text-align:center; padding-top:100px;'>\
                                            <h1>503 Service Unavailable</h1>\
                                            <p>Mohon maaf, server sedang menerima beban trafik yang sangat tinggi. Silakan coba sesaat lagi.</p>\
                                            </body></html>";

                        let mut res = Response::new(Full::new(Bytes::from(html_503)));
                        *res.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
                        res.headers_mut().insert(
                            hyper::header::CONTENT_TYPE,
                            hyper::header::HeaderValue::from_static("text/html; charset=utf-8"),
                        );
                        Ok::<_, hyper::Error>(res)
                    });

                    let io = TokioIo::new(stream);
                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(io, emergency_svc)
                        .await;

                    metrics_clone.decrement_active();
                    return;
                }
            };

            let svc = service_fn(|req| {
                handle_proxy_request(
                    req,
                    client_clone.clone(),
                    lb_clone.clone(),
                    metrics_clone.clone(),
                    peer_addr.clone(),
                )
            });

            let io = TokioIo::new(stream);
            if let Err(err) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, svc)
                .await
            {
                let _ = LOGGER.log_debug(&format!("Connection error: {:?}", err));
            }

            metrics_clone.decrement_active();
        });
    }
}
