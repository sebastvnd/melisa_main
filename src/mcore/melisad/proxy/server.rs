/// Proxy server main loop - accepts connections dan dispatches requests
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;

use crate::mcore::config::load_config::CONFIG;
use crate::mcore::melisad::proxy::handler::handle_proxy_request;
use crate::mcore::melisad::proxy::loadbalancer::{LoadBalancer, LoadBalancingStrategy};
use crate::mcore::melisad::proxy::metrics::ProxyMetrics;
use crate::mcore::mlog::LOGGER;

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

    // TODO PINDAHIN INI KE CONFIG
    const MAX_CONCURRENT_CONNECTIONS: usize = 10000; // Tentukan batas aman

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

    // Main accept loop
    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let peer_addr = peer_addr.to_string();

        // --- MITIGASI DOS: PEMBATASAN KONEKSI KONTEMPORER ---
        let current_active = metrics
            .active_connections
            .load(std::sync::atomic::Ordering::Relaxed);
        if current_active >= MAX_CONCURRENT_CONNECTIONS {
            let _ = LOGGER.log_error(&format!(
                "DoS Protection: Dropping connection from {} due to high load",
                peer_addr
            ));
            continue; // Tolak koneksi baru secara instan jika server penuh
        }
        // ---------------------------------------------------

        metrics.increment_active();
        let metrics_clone = metrics.clone();

        let client = client.clone();
        let lb = load_balancer.clone();
        let metrics = metrics.clone();

        // Spawn handler per connection
        tokio::spawn(async move {
            let svc = service_fn(|req| {
                let client = client.clone();
                let lb = lb.clone();
                let metrics = metrics.clone();
                let peer_addr = peer_addr.clone();

                handle_proxy_request(req, client, lb, metrics, peer_addr)
            });

            // Wrap tokio socket with hyper-util's TokioIo adapter
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
