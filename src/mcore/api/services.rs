// src/mcore/api/services.rs

use crate::mcore::errors::enode::NodeError;
use crate::mcore::melisad::services::mconf::{PID_END, PID_START}; // Pastikan import ini ada
use crate::mcore::melisad::services::node::{NODE_MANAGER, NodeProcess};

// Tambah argumen domain dan route_path di signature fungsi
pub fn create_node(
    name: &str,
    pid: u32,
    url: &str,
    domain: &str,
    route_path: &str,
) -> Result<NodeProcess, NodeError> {
    if name.trim().is_empty() || !(PID_START..=PID_END).contains(&pid) {
        Err(NodeError::InvalidInput("invalid input format".to_string()))
    } else {
        // SEKARANG SUDAH DINAMIS: Menggunakan parameter kiriman API
        NODE_MANAGER.create(name, pid, url, domain, route_path)
    }
}

pub fn delete_node(hash: &str) -> Result<(), NodeError> {
    if hash.trim().len() != 64 {
        Err(NodeError::InvalidInput("invalid hash format".to_string()))
    } else {
        // Gunakan instance global yang sama
        NODE_MANAGER.delete(hash.trim())
    }
}
