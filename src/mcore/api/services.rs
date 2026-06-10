use std::io::Stdout;

use crate::mcore::melisad::services::node::{NodeError, NodeManager, NodeProcess};
use crate::mcore::melisad::services::config::{PID_START, PID_END, HASH_LENGTH};

// api membuat node baru
// data flow 2
pub fn create_node(name: &str, pid: u32, url: &str) -> Result<NodeProcess, NodeError> {
    let node = NodeManager::get_instance();

    if name.trim().is_empty() || !(PID_START..=PID_END).contains(&pid) {
        Err(NodeError::InvalidInput("invalid input format".to_string()))
    } else {
        node.create(name, pid, url)
    }
}

pub fn delete_node(hash: &str) -> Result<(), NodeError> {
    let node = NodeManager::get_instance();

    if hash.trim().len() != HASH_LENGTH {
        Err(NodeError::InvalidInput("invalid hash format".to_string()))
    } else {
        node.delete(hash.trim())
    }
}
