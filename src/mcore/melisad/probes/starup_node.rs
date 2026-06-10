use futures;
use std::fs;

use crate::mcore::errors::e_node::NodeError;
use crate::mcore::melisad::services::mconf::NODE_FILE;
use crate::mcore::melisad::services::node::{NodeManager, NodeProcess, NodeStatus};

impl NodeManager {
    pub async fn startup_node_check(&self) -> std::result::Result<(), NodeError> {
        // Buat SATU client
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .unwrap_or_default();

        let checks = {
            let processes_lock = self.processes.read().unwrap();
            let mut tasks = Vec::new();

            for (hash, node) in processes_lock.iter() {
                let hash_clone = hash.clone();
                let url_clone = node.url.clone();
                let client_clone = http_client.clone();

                let check_future = async move {
                    let status = match client_clone.get(&url_clone).send().await {
                        Ok(response) if response.status().is_success() => NodeStatus::Active,
                        _ => NodeStatus::Stopped,
                    };
                    (hash_clone, status)
                };
                tasks.push(check_future);
            }
            tasks
        }; //lepas Read lock 

        let results = futures::future::join_all(checks).await;
        {
            let mut processes_lock = self.processes.write().unwrap();
            for (hash, new_status) in results {
                if let Some(node) = processes_lock.get_mut(&hash) {
                    node.status = new_status;
                }
            }
        } // lepas Write lock 

        // Simpan ke disk
        self.save_state_to_disk()?;

        Ok(())
    }

    /// Fungsi utilitas untuk menulis state ke JSON file
    fn save_state_to_disk(&self) -> std::result::Result<(), NodeError> {
        let processes_lock = self.processes.read().unwrap();

        let json_data =
            serde_json::to_string_pretty(&*processes_lock).map_err(NodeError::JsonError)?; // Pastikan enum NodeError::JsonError ada

        fs::write(NODE_FILE, json_data).map_err(NodeError::IoError)?; // Pastikan enum NodeError::IoError ada

        Ok(())
    }
}
