// mcore/melisad/probes/liveness_node.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

use reqwest;

use crate::mcore::melisad::services::node::types::NodeStatus;

// Check satu node secara khusus
pub async fn check_node_network(url: String) -> NodeStatus {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap_or_default();

    check_node_network_with_client(&client, &url).await
}

pub async fn check_node_network_with_client(client: &reqwest::Client, url: &str) -> NodeStatus {
    match client.get(url).send().await {
        Ok(response) if response.status().is_success() => NodeStatus::Active,
        _ => NodeStatus::Stopped,
    }
}
