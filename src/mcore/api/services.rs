// mcore/api/services.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

// src/mcore/api/services.rs

use crate::mcore::errors::enode::NodeError;
use crate::mcore::melisad::services::node::allowlist::{AllowedNode, NODE_ALLOWLIST};
use crate::mcore::melisad::services::node::{NODE_MANAGER, NodeStatus};
use crate::mcore::mlog::LOGGER;

/// create node api
pub async fn create_node(
    name: &str,
    url: &str,
    domain: &str,
    route_path: &str,
    client_ip: &str,
    client_version: &str,
    invite_code: Option<&str>,
) -> Result<u32, NodeError> {
    if name.trim().is_empty() {
        return Err(NodeError::InvalidInput("name cannot be empty".to_string()));
    }
    if url.trim().is_empty() {
        return Err(NodeError::InvalidInput("url cannot be empty".to_string()));
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(NodeError::InvalidInput(
            "url must start with http:// or https://".to_string(),
        ));
    }

    let invited_by = match invite_code.map(str::trim).filter(|code| !code.is_empty()) {
        Some(code) => {
            let (parent_pid, parent_identity) = NODE_MANAGER
                .find_by_invite_code(code)
                .ok_or(NodeError::InvalidInvite)?;
            if parent_identity.domain != domain.trim().trim_end_matches('.').to_ascii_lowercase()
                || parent_identity.route_path != normalize_route_path_for_check(route_path)?
            {
                return Err(NodeError::InvalidInvite);
            }
            Some(parent_pid)
        }
        None => {
            NODE_ALLOWLIST.ensure_allowed(name, url, domain, route_path)?;
            None
        }
    };

    if NODE_MANAGER
        .list()
        .into_iter()
        .filter_map(|pid| NODE_MANAGER.get_identity(pid))
        .any(|identity| identity.name == name.trim())
    {
        return Err(NodeError::AlreadyExists);
    }

    if let Some((existing_pid, existing_identity)) = NODE_MANAGER.find_by_url(url) {
        let _ = LOGGER.log_warn(&format!(
            "Registration rejected: node '{}' [PID {}] already uses URL {}",
            existing_identity.name, existing_pid, url
        ));
        return Err(NodeError::AlreadyExists);
    }

    match NODE_MANAGER.create_with_inviter(
        name,
        url,
        domain,
        route_path,
        client_ip,
        client_version,
        invited_by,
    ) {
        Ok(pid) => {
            let _ = LOGGER.log_info(&format!(
                "Node created: '{}' [PID {}] from {} ({})",
                name, pid, client_ip, client_version
            ));
            Ok(pid)
        }
        Err(err) => {
            let _ = LOGGER.log_error(&format!("Failed to create node '{}': {}", name, err));
            Err(err)
        }
    }
}

pub fn delete_node(pid: u32) -> Result<(), NodeError> {
    NODE_MANAGER.delete(pid)
}

pub fn allow_node(
    name: &str,
    url: Option<&str>,
    domain: &str,
    route_path: &str,
) -> Result<AllowedNode, NodeError> {
    NODE_ALLOWLIST.add(name, url, domain, route_path)
}

pub fn list_allowed_nodes() -> Vec<AllowedNode> {
    NODE_ALLOWLIST.list()
}

// MONITORING HELPERS

#[derive(Debug, serde::Serialize)]
pub struct NodesSummary {
    pub total: usize,
    pub active: usize,
    pub stopped: usize,
    pub suspicious: usize,
}

fn normalize_route_path_for_check(route_path: &str) -> Result<String, NodeError> {
    let route_path = route_path.trim();
    if route_path.is_empty() {
        return Ok("/".to_string());
    }
    if !route_path.starts_with('/') {
        return Err(NodeError::InvalidInput(
            "route_path must start with '/'".to_string(),
        ));
    }
    let normalized = route_path.trim_end_matches('/');
    if normalized.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(normalized.to_string())
    }
}

/// Mengumpulkan ringkasan status seluruh cluster node untuk dashboard management
pub fn get_nodes_summary() -> NodesSummary {
    let total = NODE_MANAGER.total_active();
    let active = NODE_MANAGER
        .get_pids_by_status(|status| *status == NodeStatus::Active)
        .len();
    let stopped = NODE_MANAGER
        .get_pids_by_status(|status| *status == NodeStatus::Stopped)
        .len();
    let suspicious = NODE_MANAGER
        .get_pids_by_status(|status| *status == NodeStatus::Suspicious)
        .len();

    NodesSummary {
        total,
        active,
        stopped,
        suspicious,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_node_creation_lifecycle() {
        // Reset state singleton agar test berjalan bersih isolasi
        NODE_MANAGER.reset_for_test();
        NODE_ALLOWLIST.reset_for_test();
        allow_node(
            "test-node",
            Some("http://localhost:3000"),
            "test.local",
            "/api",
        )
        .unwrap();

        let result = create_node(
            "test-node",
            "http://localhost:3000",
            "test.local",
            "/api",
            "127.0.0.1",
            "0.1.0",
            None,
        )
        .await;

        assert!(
            result.is_ok(),
            "Harusnya sukses mendaftar karena PID dialokasikan otomatis dari rentang terkecil"
        );

        let pid = result.unwrap();
        let node = NODE_MANAGER.get(pid).expect("node must exist after create");
        assert!(node.identity.created_at > 0);
        assert_eq!(
            node.health.status,
            crate::mcore::melisad::services::node::NodeStatusSerde::Active
        );
    }

    #[tokio::test]
    #[serial]
    async fn invited_child_can_share_parent_route_for_load_balancing() {
        NODE_MANAGER.reset_for_test();
        NODE_ALLOWLIST.reset_for_test();
        allow_node("node1", Some("http://localhost:3100"), "app.local", "/api").unwrap();

        let parent_pid = create_node(
            "node1",
            "http://localhost:3100",
            "app.local",
            "/api",
            "127.0.0.1",
            "0.1.0",
            None,
        )
        .await
        .expect("parent node should register from allowlist");
        let parent = NODE_MANAGER.get(parent_pid).expect("parent should exist");

        let child_pid = create_node(
            "node1c1",
            "http://localhost:3101",
            "app.local",
            "/api",
            "127.0.0.1",
            "0.1.0",
            Some(&parent.identity.invite_code),
        )
        .await
        .expect("child node should register with parent invite");

        let child = NODE_MANAGER.get(child_pid).expect("child should exist");
        assert_eq!(child.identity.invited_by, Some(parent_pid));

        let matching = NODE_MANAGER.find_matching_nodes_by_route("app.local", "/api/users");
        assert_eq!(matching.len(), 2);
    }

    #[tokio::test]
    #[serial]
    async fn duplicate_node_url_is_rejected() {
        NODE_MANAGER.reset_for_test();
        NODE_ALLOWLIST.reset_for_test();
        allow_node("node1", Some("http://localhost:3200"), "app.local", "/api").unwrap();

        let first = create_node(
            "node1",
            "http://localhost:3200",
            "app.local",
            "/api",
            "127.0.0.1",
            "0.1.0",
            None,
        )
        .await;
        assert!(first.is_ok());

        let duplicate = create_node(
            "node1-copy",
            "http://localhost:3200",
            "app.local",
            "/api",
            "127.0.0.1",
            "0.1.0",
            Some(
                &NODE_MANAGER
                    .get(first.unwrap())
                    .expect("node should exist")
                    .identity
                    .invite_code,
            ),
        )
        .await;
        assert!(matches!(duplicate, Err(NodeError::AlreadyExists)));
    }
}
