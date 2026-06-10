use reqwest;

use crate::mcore::melisad::services::node::NodeStatus;

// Check satu node secara khusus
pub async fn check_node_network(url: String) -> NodeStatus {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap_or_default();

    match client.get(&url).send().await {
        Ok(response) if response.status().is_success() => NodeStatus::Active,
        _ => NodeStatus::Stopped,
    }
}
