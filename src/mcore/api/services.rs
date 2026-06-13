// src/mcore/api/services.rs
// API Layer - Business logic untuk node management
// Alur: HTTP request → adapter (format) → api/services (logic) → melisad (operations)

use crate::mcore::errors::enode::NodeError;
use crate::mcore::melisad::mconf::{PID_END, PID_START};
use crate::mcore::melisad::services::node::{NODE_MANAGER, NodeProcess};
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

/// API Service: Create node dengan validation
/// Alur data: incoming request → validation → NODE_MANAGER.create() → return NodeProcess
pub fn create_node(
    name: &str,
    pid: Option<u32>,  // Optional - generate jika tidak ada
    url: &str,
    domain: &str,
    route_path: &str,
) -> Result<NodeProcess, NodeError> {
    // Validation - nama tidak boleh kosong
    if name.trim().is_empty() {
        return Err(NodeError::InvalidInput("name cannot be empty".to_string()));
    }

    // Gunakan PID yang diberikan, atau generate virtual PID
    let final_pid = match pid {
        Some(p) if (PID_START..=PID_END).contains(&p) => p,
        Some(_) => {
            // PID diberikan secara eksplisit tapi di luar range → tolak
            return Err(NodeError::InvalidInput(
                "pid out of allowed range".to_string(),
            ));
        }
        None => {
            // Tidak ada PID → generate virtual PID secara otomatis
            generate_virtual_pid(&format!("{}-{}", name, url))
        }
    };

    // Delegasi ke melisad layer
    NODE_MANAGER.create(name, final_pid, url, domain, route_path)
}

pub fn delete_node(hash: &str) -> Result<(), NodeError> {
    if hash.trim().len() != 64 {
        Err(NodeError::InvalidInput("invalid hash format".to_string()))
    } else {
        // Gunakan instance global yang sama
        NODE_MANAGER.delete(hash.trim())
    }
}
