// src/mcore/api/services.rs
// ✅ UPDATED: Dengan deduplication check dan enhanced validation

use crate::mcore::errors::enode::NodeError;
use crate::mcore::melisad::probes::liveness_node::check_node_network;
use crate::mcore::melisad::services::node::{NODE_MANAGER, NodeProcess, NodeStatus};
use crate::mcore::melisad::utils::pid::generate_pid;
use crate::mcore::mlog::LOGGER;

// TODO pembuatan node baru sekarang seharusnya memakai yang ini fungis create_node()
//      yang lama sudah tidak berfungsi.
//      create node + validation deduplication

pub async fn create_node(
    name: &str,
    url: &str,
    domain: &str,
    route_path: &str,
    client_ip: &str,
    client_version: &str,
) -> Result<NodeProcess, NodeError> {
    // ============================================================
    // Step 1: Basic Validation
    // ============================================================

    if name.trim().is_empty() {
        return Err(NodeError::InvalidInput("name cannot be empty".to_string()));
    }

    if url.trim().is_empty() {
        return Err(NodeError::InvalidInput("url cannot be empty".to_string()));
    }

    // Validate URL format (basic check)
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(NodeError::InvalidInput(
            "url must start with http:// or https://".to_string(),
        ));
    }

    // ============================================================
    // Step 2: ✅ DEDUPLICATION CHECK - Cek apakah URL sudah ada
    // ============================================================

    if let Some(existing_node) = NODE_MANAGER.find_by_url(url) {
        let _ = LOGGER.log_info(&format!(
            "Deduplication: Found existing node '{}' with same URL. Performing liveness check...",
            existing_node.name
        ));

        // ✅ Step 2a: Check apakah node lama masih aktif
        let is_old_node_alive = check_node_network(existing_node.url.clone()).await;

        match is_old_node_alive {
            NodeStatus::Active => {
                // Node lama MASIH AKTIF → REJECT registration baru
                let error_msg = format!(
                    "Deduplication rejected: Node '{}' with URL {} is still active. \
                    Delete the old node first or use a different URL.",
                    existing_node.name, url
                );

                let _ = LOGGER.log_warn(&error_msg);

                return Err(NodeError::AlreadyExists);
            }

            NodeStatus::Stopped | NodeStatus::Suspicious => {
                // Node lama SUDAH MATI → REPLACE dengan node baru
                let old_hash = existing_node.hash.clone();
                let old_name = existing_node.name.clone();

                match NODE_MANAGER.delete(&old_hash) {
                    Ok(_) => {
                        let _ = LOGGER.log_info(&format!(
                            "Deduplication: Replaced dead node '{}' [{}] with new registration \
                            from {} (version {}). Old node was in state: {:?}",
                            old_name, old_hash, client_ip, client_version, existing_node.status
                        ));
                    }
                    Err(e) => {
                        let _ = LOGGER.log_error(&format!(
                            "Deduplication: Failed to remove old node '{}': {}",
                            old_hash, e
                        ));
                        // Continue anyway - akan overwrite
                    }
                }
            }
        }
    }

    // ============================================================
    // Step 3: Validate & Generate PID
    // ============================================================

    let final_pid = generate_pid(&format!("{}-{}", name, url));

    // ============================================================
    // Step 4: Create node via NODE_MANAGER (dengan info client)
    // ============================================================

    match NODE_MANAGER.create(
        name,
        final_pid,
        url,
        domain,
        route_path,
        client_ip,
        client_version,
    ) {
        Ok(node_process) => {
            let _ = LOGGER.log_info(&format!(
                "Node created successfully: {} [{}] from {} (v{})",
                node_process.name, node_process.hash, client_ip, client_version
            ));
            Ok(node_process)
        }
        Err(e) => {
            let _ = LOGGER.log_error(&format!("Failed to create node: {}", e));
            Err(e)
        }
    }
}

/// Delete node dengan validation
pub fn delete_node(hash: &str) -> Result<(), NodeError> {
    if hash.trim().len() != 64 {
        return Err(NodeError::InvalidInput("invalid hash format".to_string()));
    }

    match NODE_MANAGER.delete(hash.trim()) {
        Ok(_) => {
            let _ = LOGGER.log_info(&format!("Node deleted: {}", hash));
            Ok(())
        }
        Err(e) => {
            let _ = LOGGER.log_warn(&format!("Failed to delete node {}: {}", hash, e));
            Err(e)
        }
    }
}

// ============================================================
// MONITORING HELPERS
// ============================================================

/// Get summary dari semua node untuk monitoring
pub fn get_nodes_summary() -> Result<NodesSummary, NodeError> {
    let all_nodes = NODE_MANAGER
        .list()
        .ok_or_else(|| NodeError::InvalidInput("Failed to get nodes list".to_string()))?;

    let mut active = 0;
    let mut stopped = 0;
    let mut suspicious = 0;

    for hash in &all_nodes {
        // Ambil data NodeProcess berdasarkan hash terlebih dahulu
        if let Some(node) = NODE_MANAGER.get(hash) {
            match node.status {
                NodeStatus::Active => active += 1,
                NodeStatus::Stopped => stopped += 1,
                NodeStatus::Suspicious => suspicious += 1,
            }
        }
    }

    Ok(NodesSummary {
        total: all_nodes.len(),
        active,
        stopped,
        suspicious,
    })
}

/// Summary struct untuk monitoring response
#[derive(Debug, serde::Serialize)]
pub struct NodesSummary {
    pub total: usize,
    pub active: usize,
    pub stopped: usize,
    pub suspicious: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    static TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[tokio::test]
    async fn test_pid_validation() {
        let result = create_node(
            "test-node",
            "http://localhost:3000",
            "test.local",
            "/api",
            "192.0.0.1",
            "0.1.0",
        )
        .await;
        // Diubah menjadi assert Ok karena PID sekarang digenerate otomatis dengan valid
        assert!(
            result.is_ok(),
            "Harusnya sukses karena virtual PID digenerate otomatis"
        );

        let node = result.unwrap();
        assert!(node.last_health_check > 0);
    }
}
