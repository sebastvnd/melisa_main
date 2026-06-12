//! # Melisa Project
//!
//! An open server architecture inspired by Pingora/Nginx, fully written in Rust.
//!
//! - **Version:** 0.1.0-beta
//! - **License:** MIT

mod mcore;
use crate::mcore::config::load_config::CONFIG;
use crate::mcore::melisad::proxy::run_proxy_server;
use crate::mcore::melisad::services::node::NODE_MANAGER;
use crate::mcore::mlog::LOGGER;
use std::error::Error;
use std::time::Duration;

// Di mulai untuk umat manusia
// Juni 2026
// Kita ke ijen kan?
// Kamu masih ingetkan ...f

#[tokio::main]
async fn main() {
    if let Err(err) = run_melisa().await {
        eprintln!("Melisa stopped: {}", err);
        let _ = LOGGER.log_error(&format!("Melisa stopped: {}", err));
        let _ = LOGGER.flush();
        std::process::exit(1);
    }
}

async fn run_melisa() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = &*CONFIG;
    let node_count = NODE_MANAGER.list().map_or(0, |nodes| nodes.len());

    println!("--- [MELISAD DAEMON STARTUP] ---");
    println!("Config: melisa.conf");
    println!("Listen: http://{}:{}", config.host, config.port);
    println!(
        "Node registry: {} ({} node)",
        config.nodes.storage_file, node_count
    );

    let _ = LOGGER.log_info("Melisa daemon startup");

    println!("Memulai sinkronisasi dan verifikasi node...");
    NODE_MANAGER.startup_node_check().await?;
    println!("Node registry berhasil divalidasi dan disimpan.");
    let _ = LOGGER.log_info("Node startup validation completed");

    spawn_node_health_monitor(Duration::from_secs(
        config.nodes.health_check_interval_secs.max(1),
    ));

    run_proxy_server().await?;
    Ok(())
}

fn spawn_node_health_monitor(interval: Duration) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;

            if let Err(err) = NODE_MANAGER.startup_node_check().await {
                let _ = LOGGER.log_error(&format!("Periodic node health check failed: {}", err));
            } else {
                let _ = LOGGER.log_debug("Periodic node health check completed");
            }
        }
    });
}
