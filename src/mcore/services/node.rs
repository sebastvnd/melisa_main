use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Result;

use crate::mcore::services::hashing::generate_hash;
use crate::mcore::services::config::NODE_FILE;

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

impl NodeProcess {
    // Buat proses baru
    pub fn new(name: &str, pid: u32) -> Result<Self> {
        let path = NODE_FILE;

        let mut processes: HashMap<String, NodeProcess> = match fs::read_to_string(path) {
            Ok(content) if !content.trim().is_empty() => {
                serde_json::from_str(&content).unwrap_or_default()
            }
            _ => HashMap::new(),
        };

        let hash = generate_hash(name);

        let new_process = NodeProcess {
            hash: hash.clone(),
            name: name.to_string(),
            pid,
            status: NodeStatus::Active,
        };

        processes.insert(hash, new_process.clone());

        fs::write(path, serde_json::to_string_pretty(&processes)?)?;

        Ok(new_process)
    }
    pub fn delete(hash: &str) -> Result<()> {
        let path = NODE_FILE;

        let mut processes: HashMap<String, NodeProcess> = match fs::read_to_string(path) {
            Ok(content) if !content.trim().is_empty() => {
                let mut map: HashMap<String, NodeProcess> =
                    serde_json::from_str(&content).unwrap_or_default();

                for (key, proc) in map.iter_mut() {
                    proc.hash = key.clone();
                }
                map
            }
            _ => HashMap::new(),
        };

        if processes.remove(hash).is_some() {
            fs::write(path, serde_json::to_string_pretty(&processes)?)?;
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("error '{}' not found", hash),
            ))
        }
    }
}
