use std::io::Stdout;

use crate::mcore::melisad::services::node::{NodeError, NodeManager, NodeProcess};

// api membuat node baru
// data flow 2
pub fn create_node(name: &str, pid: u32, url: &str) -> Result<NodeProcess, NodeError> {
    let node = NodeManager::get_instance();
    node.create(name, pid, url)
}

pub fn delete_node(hash: &str) -> Result<(), NodeError> {
    let node = NodeManager::get_instance();
    node.delete(hash)
}
