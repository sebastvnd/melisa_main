use std::io::Stdout;

use crate::mcore::errors::e_node::NodeError;
use crate::mcore::melisad::services::mconf::{HASH_LENGTH, PID_END, PID_START};
use crate::mcore::melisad::services::node::{NodeManager, NodeProcess};

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
