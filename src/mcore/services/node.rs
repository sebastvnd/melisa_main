use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::format;
use std::fs;
use std::fs::TryLockError::Error;
use std::io::Result;
use std::os::unix::process;
use std::sync::{OnceLock, RwLock};

use crate::mcore::services::config::NODE_FILE;
use crate::mcore::services::hashing::generate_hash;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeProcess {
    #[serde(skip)]
    pub hash: String,
    pub name: String,
    pub pid: u32,
    pub status: NodeStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeStatus {
    Active,
    Stopped,
}

#[derive(Debug)]
pub enum NodeError {
    AlreadyExists,                // Error jika node dengan nama tersebut sudah ada
    IoError((std::io::Error)),    // Menyimpan error dari file system
    JsonError(serde_json::Error), // Menyimpan error dari serde_json
}

pub struct NodeManager {
    processes: RwLock<HashMap<String, NodeProcess>>,
}

// TODO: ubah fn create_node dan delete_node untuk menggunakan NodeManager sebagai state management
impl NodeManager {
    pub fn get_instance() -> &'static Self {
        static INSTANCE: OnceLock<NodeManager> = OnceLock::new();

        INSTANCE.get_or_init(|| {
            let path = NODE_FILE;

            let processes = match fs::read_to_string(path) {
                Ok(content) if !content.trim().is_empty() => {
                    serde_json::from_str(&content).unwrap_or_default()
                }
                _ => HashMap::new(),
            };

            NodeManager {
                processes: RwLock::new(processes),
            }
        })
    }

    // TODO: ganti result bool menjadi Enum agar lebih tangguh
    pub fn create(&self, name: &str, pid: u32) -> std::result::Result<NodeProcess, NodeError> {
        let mut processes_lock = self.processes.write().unwrap();
        let hash = generate_hash(name);

        if processes_lock.contains_key(&hash) {
            return Err(NodeError::AlreadyExists);
        }

        let node = NodeProcess {
            hash: hash.clone(),
            name: name.to_string(),
            pid,
            status: NodeStatus::Active,
        };

        processes_lock.insert(hash, node.clone());

        let json_data =
            serde_json::to_string_pretty(&*processes_lock).map_err(NodeError::JsonError)?;

        fs::write(NODE_FILE, json_data).map_err(NodeError::IoError)?;

        Ok(node)
    }

    pub fn delete(&self, hash: &str) -> Result<()> {
        let mut processes_lock = self.processes.write().unwrap();

        if processes_lock.remove(hash).is_some() {
            fs::write(NODE_FILE, serde_json::to_string_pretty(&*processes_lock)?)?;
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("error '{}' not found", hash),
            ))
        }
    }
    #[cfg(test)]
    pub fn reset_for_test(&self) {
        let mut processes_lock = self.processes.write().unwrap();
        processes_lock.clear();
    }
}
