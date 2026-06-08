use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::{OnceLock, RwLock};

use crate::mcore::melisad::services::config::NODE_FILE;
use crate::mcore::melisad::services::hashing::generate_hash;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeProcess {
    #[serde(skip)]
    pub hash: String,
    pub name: String,
    pub pid: u32,
    pub url: String,
    pub status: NodeStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeStatus {
    Active,
    Stopped,
    Unknown,
}

#[derive(Debug)]
pub enum NodeError {
    AlreadyExists,             // Error jika node dengan nama tersebut sudah ada
    IoError((std::io::Error)), // Menyimpan error dari file system
    JsonError(serde_json::Error),
    NotFound,
}

pub struct NodeManager {
    pub processes: RwLock<HashMap<String, NodeProcess>>,
    pub node : NodeProcess,
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
                node: NodeProcess {
                    hash: String::new(),
                    name: String::new(),
                    pid: 0,
                    url: String::new(),
                    status: NodeStatus::Unknown,
                }
            }
        })
    }

    pub fn create(&self, name: &str, pid: u32, url: &str) -> std::result::Result<NodeProcess, NodeError> {
        let mut processes_lock = self.processes.write().unwrap();
        let hash = generate_hash(name);

        if processes_lock.contains_key(&hash) {
            return Err(NodeError::AlreadyExists);
        }

        let node = NodeProcess {
            hash: hash.clone(),
            name: name.to_string(),
            pid,
            url: url.to_string(),
            status: NodeStatus::Active,
        };

        processes_lock.insert(hash, node.clone());

        let json_data =
            serde_json::to_string_pretty(&*processes_lock).map_err(NodeError::JsonError)?;

        fs::write(NODE_FILE, json_data).map_err(NodeError::IoError)?;

        Ok(node)
    }

    pub fn delete(&self, hash: &str) -> std::result::Result<(), NodeError> {
        let mut processes_lock = self.processes.write().unwrap();

        if processes_lock.remove(hash).is_some() {
            let json_data =
                serde_json::to_string_pretty(&*processes_lock).map_err(NodeError::JsonError)?;

            fs::write(NODE_FILE, json_data).map_err(NodeError::IoError)?;

            Ok(())
        } else {
            Err(NodeError::NotFound)
        }
    }

    pub fn list(&self) -> Option<Vec<String>> {
        let processes_lock = self.processes.read().unwrap();

        let mut list: Vec<String> = processes_lock.keys().cloned().collect();
        list.sort();

        if list.is_empty() {
            None
        } else {
            Some(list)
        }
    }
    
    #[cfg(test)]
    pub fn reset_for_test(&self) {
        let mut processes_lock = self.processes.write().unwrap();
        processes_lock.clear();
    }
}
