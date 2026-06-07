use serde::{Deserialize, Serialize};

use crate::mcore::adapter::json::Action::CreateNode;
use crate::mcore::api::services::*;
use crate::mcore::services::node::{NodeError, NodeManager, NodeProcess};

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiRequest<T> {
    pub version: String,
    pub action: Action,
    pub request_id: String,
    pub timestamp: u64,
    pub data: T,
}

pub struct ApiResponse<T> {
    pub request_id: String,
    pub success: bool,
    pub code: u16,
    pub message: String,
    pub data: Option<T>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateNodeData {
    name: String,
    pid: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Action {
    CreateNode,
    DeleteNode,
}

pub fn api_create_node(request: &ApiRequest<CreateNodeData>) -> Result<NodeProcess, NodeError> {
    create_node(&request.data.name, request.data.pid)
}

pub fn api_delete_node(hash: &str) -> Result<(), NodeError> {
    delete_node(hash)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::mcore::services::config::NODE_FILE;
    use std::fs;

    #[test]
    fn test_new_node() {
        let _ = fs::write(NODE_FILE, "{}");

        // Kosongkan cache di dalam Singleton NodeManager
        NodeManager::get_instance().reset_for_test();

        let node = ApiRequest {
            version: "1.0".to_string(),
            action: Action::CreateNode,
            request_id: "id001".to_string(),
            timestamp: 17828661,
            data: CreateNodeData {
                name: "melisa beta".to_string(),
                pid: 808,
            },
        };

        let first = api_create_node(&node);
        assert!(
            first.is_ok(),
            "Harusnya sukses karena state sudah di-reset, tapi dapet: {:?}",
            first
        );

        let second = api_create_node(&node);

        assert!(
            matches!(second, Err(NodeError::AlreadyExists)),
            "Harusnya gagal karena duplikat"
        );
    }

    #[test]
    fn test_delete_node() {
        let _ = fs::write(NODE_FILE, "{}");
        let manager = NodeManager::get_instance();
        manager.reset_for_test();

        let node = ApiRequest {
            version: "1.0".to_string(),
            action: Action::CreateNode,
            request_id: "id001".to_string(),
            timestamp: 17828661,
            data: CreateNodeData {
                name: "melisa".to_string(),
                pid: 808,
            },
        };


        let create = match api_create_node(&node) {
            Ok(proses ) => proses.hash.to_string(),
            Err(err) => format!("Error terjadi: {:?}", err),   
        };


        let hash_target = create;
        let delete = delete_node(&hash_target);

        // Pastikan sekarang harusnya sukses (Ok)
        assert!(
            delete.is_ok(),
            "Harusnya sukses menghapus node, tetapi dapet error: {:?}",
            delete
        );
    }
}
