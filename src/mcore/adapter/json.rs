use serde::{Deserialize, Serialize};

use crate::mcore::api::services::*;
use crate::mcore::errors::enode::NodeError;
use crate::mcore::melisad::services::node::NodeProcess;

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
    pub name: String,
    #[serde(default)]  // Make pid optional - will be generated if not provided
    pub pid: Option<u32>,
    pub url: String,
    pub domain: String,
    pub route_path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Action {
    CreateNode,
    DeleteNode,
}

/// Adapter layer: Convert HTTP request ke API call
/// Alur: HTTP body → CreateNodeData → create_node() → NODE_MANAGER
pub fn api_create_node(request: &ApiRequest<CreateNodeData>) -> Result<NodeProcess, NodeError> {
    create_node(
        &request.data.name,
        request.data.pid,  // Optional - API service will generate if needed
        &request.data.url,
        &request.data.domain,
        &request.data.route_path,
    )
}

pub fn api_delete_node(hash: &str) -> Result<(), NodeError> {
    delete_node(hash)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::mcore::melisad::services::mconf::NODE_FILE;
    use crate::mcore::melisad::services::node::NODE_MANAGER;
    use once_cell::sync::Lazy;
    use std::fs;
    use std::sync::Mutex;

    static TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn test_new_node() {
        let _guard = TEST_LOCK.lock().unwrap();
        let _ = fs::write(NODE_FILE, "{}");

        // Reset singleton node manager untuk test
        NODE_MANAGER.reset_for_test();

        let node = ApiRequest {
            version: "1.0".to_string(),
            action: Action::CreateNode,
            request_id: "id001".to_string(),
            timestamp: 17828661,
            data: CreateNodeData {
                name: "melisa-api".to_string(),
                pid: Some(100000),
                url: "http://localhost:3000".to_string(),
                domain: "melisa.local".to_string(),
                route_path: "/beta".to_string(),
            },
        };

        let first = api_create_node(&node);
        assert!(
            first.is_ok(),
            "Harusnya sukses membuat node baru, tapi dapet: {:?}",
            first
        );

        // Verify bahwa node berhasil dibuat
        let first_node = first.unwrap();
        assert_eq!(first_node.name, "melisa-api");
        assert_eq!(
            first_node.status,
            crate::mcore::melisad::services::node::NodeStatus::Active
        );

        let second = api_create_node(&node);
        assert!(
            matches!(second, Err(NodeError::AlreadyExists)),
            "Harusnya gagal karena node dengan nama yang sama sudah ada"
        );
    }

    #[test]
    fn test_delete_node() {
        let _guard = TEST_LOCK.lock().unwrap();
        let _ = fs::write(NODE_FILE, "{}");

        // Reset dan setup test
        NODE_MANAGER.reset_for_test();

        let node = ApiRequest {
            version: "1.0".to_string(),
            action: Action::CreateNode,
            request_id: "id002".to_string(),
            timestamp: 17828662,
            data: CreateNodeData {
                name: "melisa-delete-test".to_string(),
                pid: Some(100001),
                url: "http://localhost:3001".to_string(),
                domain: "delete.local".to_string(),
                route_path: "/test".to_string(),
            },
        };

        let create_result = api_create_node(&node);
        assert!(
            create_result.is_ok(),
            "Node harus berhasil dibuat terlebih dahulu"
        );

        let hash_target = create_result.unwrap().hash;

        // Test delete
        let delete_result = delete_node(&hash_target);
        assert!(
            delete_result.is_ok(),
            "Harusnya sukses menghapus node yang ada"
        );

        // Verify node sudah terhapus (tidak bisa delete ulang)
        let delete_again = delete_node(&hash_target);
        assert!(
            matches!(delete_again, Err(NodeError::NotFound)),
            "Harusnya gagal menghapus node yang sudah terhapus"
        );
    }

    #[test]
    fn test_invalid_pid() {
        let _guard = TEST_LOCK.lock().unwrap();
        let _ = fs::write(NODE_FILE, "{}");
        NODE_MANAGER.reset_for_test();

        let invalid_node = ApiRequest {
            version: "1.0".to_string(),
            action: Action::CreateNode,
            request_id: "id003".to_string(),
            timestamp: 17828663,
            data: CreateNodeData {
                name: "invalid-pid-node".to_string(),
                pid: Some(5000), // PID di bawah PID_START (100000)
                url: "http://localhost:3002".to_string(),
                domain: "invalid.local".to_string(),
                route_path: "/invalid".to_string(),
            },
        };

        let result = api_create_node(&invalid_node);
        assert!(
            matches!(result, Err(NodeError::InvalidInput(_))),
            "Harusnya gagal karena PID tidak valid"
        );
    }
}
