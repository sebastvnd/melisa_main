// src/mcore/api/services.rs
// ✅ UPDATED: Dengan deduplication check dan enhanced validation

use crate::mcore::config::load_config::{PID_END, PID_START};
use crate::mcore::errors::enode::NodeError;
use crate::mcore::melisad::services::node::{NODE_MANAGER, NodeProcess, NodeStatus};
use crate::mcore::melisad::probes::liveness_node::check_node_network;
use crate::mcore::mlog::LOGGER;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Generate virtual PID untuk worker nodes
/// Range: 100,000 - 999,999 (reserved untuk managed nodes)
pub fn generate_virtual_pid(node_identifier: &str) -> u32 {
    let mut hasher = DefaultHasher::new();
    node_identifier.hash(&mut hasher);
    let hash_value = hasher.finish();

    // Map ke range PID_START..=PID_END (100k - 999k)
    let range_size = (PID_END - PID_START + 1) as u64;
    let virtual_pid = PID_START as u64 + (hash_value % range_size);
    virtual_pid as u32
}

/// API Service: Create node dengan validation + deduplication
/// 
/// Alur:
/// 1. Validasi input (name, url format)
/// 2. ✅ Check deduplication: apakah URL sudah ada?
/// 3. ✅ Jika ada: lakukan liveness check ke node lama
/// 4. ✅ Jika node lama masih aktif: REJECT (duplicate)
/// 5. ✅ Jika node lama sudah mati: REPLACE
/// 6. Create node baru
pub async fn create_node_with_deduplication(
    name: &str,
    pid: Option<u32>,
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

    let final_pid = match pid {
        Some(p) if (PID_START..=PID_END).contains(&p) => p,
        Some(_) => {
            return Err(NodeError::InvalidInput(format!(
                "pid out of allowed range [{}, {}]",
                PID_START, PID_END
            )));
        }
        None => {
            // Generate virtual PID dari node identifier
            generate_virtual_pid(&format!("{}-{}", name, url))
        }
    };

    // ============================================================
    // Step 4: Create node via NODE_MANAGER (dengan info client)
    // ============================================================

    match NODE_MANAGER.create_with_metadata(
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

/// Old synchronous version (backward compatibility)
/// ⚠️ DEPRECATED - Gunakan create_node_with_deduplication() yang async
pub fn create_node(
    name: &str,
    url: &str,
    domain: &str,
    route_path: &str,
) -> Result<NodeProcess, NodeError> {
    // Validation - nama tidak boleh kosong
    if name.trim().is_empty() {
        return Err(NodeError::InvalidInput("name cannot be empty".to_string()));
    }

    // Gunakan PID yang diberikan, atau generate virtual PID
    let final_pid = generate_virtual_pid(&format!("{}-{}", name, url));


    // Delegasi ke melisad layer
    NODE_MANAGER.create(name, final_pid, url, domain, route_path)
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
    let all_nodes = NODE_MANAGER.list().ok_or_else(|| {
        NodeError::InvalidInput("Failed to get nodes list".to_string())
    })?;

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
    use crate::mcore::config::load_config::NODE_FILE;
    use once_cell::sync::Lazy;
    use std::fs;
    use std::sync::Mutex;

    static TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn test_validate_name() {
        let result = create_node(
            "",
            "http://localhost:3000",
            "test.local",
            "/api",
        );
        assert!(matches!(result, Err(NodeError::InvalidInput(_))));
    }

    #[test]
    fn test_pid_validation() {
        let result = create_node(
            "test-node",
            "http://localhost:3000",
            "test.local",
            "/api",
        );
        // Diubah menjadi assert Ok karena PID sekarang digenerate otomatis dengan valid
        assert!(result.is_ok(), "Harusnya sukses karena virtual PID digenerate otomatis");
        
        let node = result.unwrap();
        assert!(node.last_health_check > 0);
    }

    #[test]
    fn test_virtual_pid_generation() {
        let pid1 = generate_virtual_pid("node1-http://localhost:3000");
        let pid2 = generate_virtual_pid("node2-http://localhost:3001");

        assert!(pid1 >= PID_START && pid1 <= PID_END);
        assert!(pid2 >= PID_START && pid2 <= PID_END);
        assert_ne!(pid1, pid2); // Different identifiers should generate different PIDs
    }
}
