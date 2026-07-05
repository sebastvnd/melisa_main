// mcore/adapter/json.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

use serde::{Deserialize, Serialize};

use crate::mcore::api::services::{create_node, delete_node};
use crate::mcore::errors::enode::NodeError;

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
    pub url: String,
    pub domain: String,
    pub route_path: String,
    pub ip: String,
    pub version: String,
    #[serde(default)]
    pub invite_code: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Action {
    CreateNode,
    DeleteNode,
}

/// Adapter layer: Convert HTTP request ke API call
/// Alur: HTTP body → CreateNodeData → create_node() → NODE_MANAGER
pub async fn api_create_node(request: &ApiRequest<CreateNodeData>) -> Result<u32, NodeError> {
    create_node(
        &request.data.name,
        &request.data.url,
        &request.data.domain,
        &request.data.route_path,
        &request.data.ip,
        &request.data.version,
        request.data.invite_code.as_deref(),
    )
    .await
}

pub fn api_delete_node(pid: u32) -> Result<(), NodeError> {
    delete_node(pid)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::mcore::api::services::allow_node;
    use crate::mcore::melisad::services::node::NODE_MANAGER;
    use crate::mcore::melisad::services::node::allowlist::NODE_ALLOWLIST;
    use once_cell::sync::Lazy;
    use serial_test::{self, serial};
    use std::sync::Mutex;

    static TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[tokio::test]
    #[serial]
    async fn test_new_node() {
        let _guard = TEST_LOCK.lock().unwrap();
        NODE_MANAGER.reset_for_test();
        NODE_ALLOWLIST.reset_for_test();
        allow_node(
            "melisa-api",
            Some("http://localhost:3000"),
            "melisa.local",
            "/beta",
        )
        .unwrap();

        let node = ApiRequest {
            version: "1.0".to_string(),
            action: Action::CreateNode,
            request_id: "id001".to_string(),
            timestamp: 17828661,
            data: CreateNodeData {
                name: "melisa-api".to_string(),
                url: "http://localhost:3000".to_string(),
                domain: "melisa.local".to_string(),
                route_path: "/beta".to_string(),
                ip: "192.0.0.1".to_string(),
                version: "0.1.0".to_string(),
                invite_code: None,
            },
        };

        let first = api_create_node(&node).await;
        assert!(
            first.is_ok(),
            "Harusnya sukses membuat node baru, tapi dapet: {:?}",
            first
        );

        // Verify bahwa node berhasil dibuat
        let pid = first.unwrap();
        let first_node = NODE_MANAGER.get(pid).expect("node must exist");
        assert_eq!(first_node.identity.name, "melisa-api");
        assert_eq!(
            first_node.health.status,
            crate::mcore::melisad::services::node::NodeStatusSerde::Active
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_node() {
        let _guard = TEST_LOCK.lock().unwrap();
        NODE_MANAGER.reset_for_test();
        NODE_ALLOWLIST.reset_for_test();
        allow_node(
            "melisa-delete-test",
            Some("http://localhost:3001"),
            "delete.local",
            "/test",
        )
        .unwrap();

        let node = ApiRequest {
            version: "1.0".to_string(),
            action: Action::CreateNode,
            request_id: "id002".to_string(),
            timestamp: 17828662,
            data: CreateNodeData {
                name: "melisa-delete-test".to_string(),
                url: "http://localhost:3001".to_string(),
                domain: "delete.local".to_string(),
                route_path: "/test".to_string(),
                ip: "192.0.0.1".to_string(),
                version: "0.1.0".to_string(),
                invite_code: None,
            },
        };

        let create_result = api_create_node(&node).await;
        assert!(
            create_result.is_ok(),
            "Node harus berhasil dibuat terlebih dahulu"
        );

        let pid = create_result.unwrap();

        let delete_result = delete_node(pid);
        assert!(
            delete_result.is_ok(),
            "Harusnya sukses menghapus node yang ada"
        );

        // Verify node sudah terhapus (tidak bisa delete ulang)
        let delete_again = delete_node(pid);
        assert!(
            matches!(delete_again, Err(NodeError::NotFound)),
            "Harusnya gagal menghapus node yang sudah terhapus"
        );
    }
}
