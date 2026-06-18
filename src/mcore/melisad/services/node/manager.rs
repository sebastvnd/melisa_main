use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, RwLock, atomic::AtomicUsize};

use crate::mcore::config::load_config::CONFIG;
use crate::mcore::melisad::services::node::models::{NodeProcess, NodeStatus};
use crate::mcore::errors::enode::NodeError;
use crate::mcore::mlog::LOGGER;

/// NodeManager manages all registered backend nodes
/// Uses Arc<RwLock> + Copy-on-Write semantics untuk thread-safe updates
pub struct NodeManager {
    /// HashMap of nodes, wrapped in Arc untuk zero-copy sharing
    pub processes: RwLock<Arc<HashMap<String, NodeProcess>>>,

    /// Track cumulative bytes untuk trigger flush
    pub accumulated_bytes: AtomicUsize,

    /// Path ke storage file (dari config)
    pub storage_path: String,
}

/// Global singleton instance
pub static NODE_MANAGER: Lazy<NodeManager> = Lazy::new(|| {
    let storage_path = CONFIG.nodes.storage_file.clone();
    NodeManager::new(&storage_path)
});

impl NodeManager {
    /// Inisialisasi NodeManager dengan membaca dari file
    pub fn new(path: &str) -> Self {
        let processes: HashMap<String, NodeProcess> = match fs::read_to_string(path) {
            Ok(content) if !content.trim().is_empty() => {
                let mut loaded: HashMap<String, NodeProcess> =
                    serde_json::from_str(&content).unwrap_or_default();

                for (hash, node) in loaded.iter_mut() {
                    node.hash = hash.clone();
                }

                loaded
            }
            _ => HashMap::new(),
        };

        NodeManager {
            processes: RwLock::new(Arc::new(processes)),
            accumulated_bytes: AtomicUsize::new(0),
            storage_path: path.to_string(),
        }
    }

    /// Find node by URL (untuk deduplication check)
    pub fn find_by_url(&self, url: &str) -> Option<NodeProcess> {
        let processes_lock = self.processes.read().unwrap();
        processes_lock
            .values()
            .find(|node| node.url == url)
            .cloned()
    }
   
    /// Get nodes dengan status tertentu (untuk monitoring)
    pub fn get_nodes_by_status(&self, status: NodeStatus) -> Vec<NodeProcess> {
        let processes_lock = self.processes.read().unwrap();
        processes_lock
            .values() // Menggunakan .values() lebih idiomatic daripada .iter() jika tidak butuh key
            .filter(|node| node.status == status)
            .cloned()
            .collect()
    }
    
    /// Get nodes yang "suspicious" untuk dalam bentuk Tuple (Nama, Total Gagal)
    pub fn get_suspicious_nodes(&self) -> Vec<(String, u32)> {
        let processes_lock = self.processes.read().unwrap();
        processes_lock
            .values()
            .filter(|node| node.consecutive_failures > 10)
            .map(|node| (node.name.clone(), node.consecutive_failures))
            .collect()
    }
    
    /// Cleanup nodes yang sudah mati > N seconds
    /// Note: Diubah menjadi fungsi sinkronus biasa karena tidak menggunakan call .await di dalamnya
    pub fn cleanup_dead_nodes(&self, timeout_secs: u64) -> std::result::Result<usize, NodeError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let mut count = 0;
        {
            let mut processes_lock = self.processes.write().unwrap();
            let mut new_map = (**processes_lock).clone();
            
            new_map.retain(|hash, node| {
                let time_since_last_seen = now - node.last_heartbeat;
                if time_since_last_seen > timeout_secs {
                    count += 1;
                    // Ditambahkan 'let _ =' untuk membungkam warning unused Result
                    let _ = LOGGER.log_info(&format!(
                        "Cleanup: Removed node {} (offline for {} secs)",
                        hash, time_since_last_seen
                    ));
                    false
                } else {
                    true
                }
            });
            
            *processes_lock = Arc::new(new_map);
        }
        
        // Panggil self.flush() bawaan persistence.rs secara langsung tanpa repot import
        self.flush()?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_node_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let node_file = temp_dir.path().join("test-nodes.json");

        fs::write(&node_file, "{}").unwrap();

        let manager = NodeManager::new(node_file.to_str().unwrap());
        assert_eq!(manager.storage_path, node_file.to_str().unwrap());
    }

    #[test]
    fn test_node_manager_load_existing() {
        let temp_dir = TempDir::new().unwrap();
        let node_file = temp_dir.path().join("test-nodes.json");

        // TAMBAHKAN field "hash" ke dalam objek JSON bawah ini
        let test_data = r#"{
            "abc123": {
                "name": "test-node",
                "hash": "abc123",
                "url": "http://localhost:3000",
                "domain": "test.local",
                "route_path": "/api",
                "status": "Active",
                "created_at": 1000,
                "last_heartbeat": 1000,
                "last_health_check": 1000,
                "consecutive_failures": 0,
                "registered_from_ip": "127.0.0.1",
                "version": "1.0.0"
            }
        }"#;
        fs::write(&node_file, test_data).unwrap();

        let manager = NodeManager::new(node_file.to_str().unwrap());
        let nodes_lock = manager.processes.read().unwrap();
        assert_eq!(nodes_lock.len(), 1);
    }
}