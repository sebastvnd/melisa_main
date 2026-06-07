use std::io::Stdout;

use crate::mcore::services::node::{NodeError, NodeManager, NodeProcess};

// api membuat node baru
// data flow 2
pub fn create_node(name: &str, pid: u32) -> Result<NodeProcess, NodeError> {
    let node = NodeManager::get_instance();
    node.create(name, pid)
}
