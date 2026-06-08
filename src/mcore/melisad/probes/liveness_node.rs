use reqwest;

use crate::mcore::melisad::services::node::{NodeProcess, NodeStatus};

impl NodeProcess {
    pub async fn health_check(&self) -> NodeStatus {
        let node = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .unwrap_or_default();

        match node.get(&self.url).send().await {
            Ok(response) if response.status().is_success() => NodeStatus::Active,
            _ => NodeStatus::Stopped,
        }
    }
}