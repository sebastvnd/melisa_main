/// Management Server - dedicated port untuk node management operations
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use crate::mcore::config::load_config::CONFIG;
use crate::mcore::handler::handler::handle_management_request;
use crate::mcore::mlog::LOGGER;

pub async fn run_management_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !CONFIG.management.enabled {
        let _ = LOGGER.log_info("Management API is disabled");
        return Ok(());
    }

    let addr = format!("{}:{}", CONFIG.host, CONFIG.management.port);
    let listener = TcpListener::bind(&addr).await?;
    let _ = LOGGER.log_info(&format!("Management API listening on {}", addr));

    println!("Management API listening on http://{}", addr);
    println!("  POST /register   - Register a new node");
    println!("  POST /unregister - Unregister a node");
    println!("  GET  /nodes      - List all nodes");

    loop {
        let (stream, _peer_addr) = listener.accept().await?;

        let svc = service_fn(|req| handle_management_request(req));

        let io = TokioIo::new(stream);

        // FIXED: spawn task per koneksi agar concurrent
        tokio::spawn(async move {
            if let Err(err) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, svc)
                .await
            {
                let _ = LOGGER.log_debug(&format!("Management connection error: {:?}", err));
            }
        });
    }
}
