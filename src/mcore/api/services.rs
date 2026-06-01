use crate::mcore::services::node::NodeProcess;

// api membuat node baru
pub fn create_node(name: &str, pid: u32) -> String{
    let respond = match NodeProcess::new(name, pid) {
        Ok(node) => format!("Node created: {:?}", node),
        Err(e) => format!("Error creating node: {}", e),
    };
    respond
}