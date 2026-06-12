//! # MNode - Melisa Network Node
//!
//! A node worker yang auto-register ke Melisa network
//! Berfungsi untuk:
//! - Serve HTML/Static files dari public/html directory
//! - Backend operations dengan API endpoints
//! - Auto-register ke melisa management API

use std::sync::Arc;
use tokio::net::TcpListener;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;

mod config;
mod handler;
mod registration;

use config::NodeConfig;
use handler::handle_request;
use registration::register_with_melisa;

const CONFIG_FILE: &str = "mnode.conf";

#[tokio::main]
async fn main() {
    println!("╔════════════════════════════════════════════╗");
    println!("║      MNode Worker - Melisa Network         ║");
    println!("╚════════════════════════════════════════════╝");
    println!("");

    // Load config dari file atau environment
    let config = match NodeConfig::from_config_file(CONFIG_FILE) {
        Ok(cfg) => {
            println!("✓ Config loaded from {}", CONFIG_FILE);
            cfg
        }
        Err(e) => {
            println!("Warning: Could not load {}: {}", CONFIG_FILE, e);
            println!("Using environment variables");
            NodeConfig::from_env()
        }
    };

    println!("Node URL: {}", config.node_url());
    println!("Domain: {}", config.domain);
    println!("Route Path: {}", config.route_path);
    println!("Static Files: {} ({})", 
        config.static_files_dir,
        if config.static_files_enabled { "enabled" } else { "disabled" }
    );
    println!("Melisa Management: {}:{}", config.melisa_host, config.melisa_port);
    println!("");

    // Register dengan melisa
    println!("Connecting to Melisa Management API...");
    if let Err(e) = register_with_melisa(&config).await {
        eprintln!("⚠ Warning: Failed to register with Melisa: {}", e);
        eprintln!("Continuing anyway - node will be unavailable until registered");
    } else {
        println!("✓ Successfully registered with Melisa");
    }
    println!("");

    // Start HTTP server
    if let Err(e) = run_node_server(&config).await {
        eprintln!("Node stopped: {}", e);
        std::process::exit(1);
    }
}

async fn run_node_server(config: &NodeConfig) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;
    let config = Arc::new(config.clone());

    println!("╔════════════════════════════════════════════╗");
    println!("║     MNode Server Ready - http://{}     ║", addr);
    println!("╚════════════════════════════════════════════╝");
    println!("");
    println!("Endpoints:");
    println!("  GET  /             - Serve index.html atau default page");
    println!("  GET  /api/info     - Node information (JSON)");
    println!("  GET  /api/health   - Health check (JSON)");
    println!("  GET  /*            - Serve static files dari public/html");
    println!("");
    println!("Static Files Location: {}", config.static_files_dir);
    println!("To add custom files:");
    println!("  1. Create/edit HTML files in: {}", config.static_files_dir);
    println!("  2. MNode will auto-serve them from / path");
    println!("");

    loop {
        let (stream, _peer_addr) = listener.accept().await?;
        let config = config.clone();

        tokio::spawn(async move {
            let svc = service_fn(|req| {
                let config = config.clone();
                handle_request(req, config)
            });

            let io = TokioIo::new(stream);

            if let Err(err) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, svc)
                .await
            {
                eprintln!("Connection error: {:?}", err);
            }
        });
    }
}

