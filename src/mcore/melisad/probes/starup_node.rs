use futures;
use std::fs;

use crate::mcore::melisad::services::node::{NodeError, NodeManager, NodeProcess, NodeStatus};
use crate::mcore::melisad::services::config::NODE_FILE;

impl NodeManager {
    pub async fn startup_node_check(&self) -> std::result::Result<(), NodeError> {
        let mut processes_lock = self.processes.write().unwrap();

        let mut tasks = Vec::new();
        let mut hashes = Vec::new();

        for (hash, node) in processes_lock.iter() {
            hashes.push(hash.clone());
            let task = NodeProcess::health_check(&node);
            tasks.push(task);
        }

        // Jalankan semua tasks secara concurrent
        let results = futures::future::join_all(tasks).await;

        // Update status berdasarkan hasil
        for (hash, new_status) in hashes.into_iter().zip(results.into_iter()) {
            if let Some(node) = processes_lock.get_mut(&hash) {
                node.status = new_status;
            }
        }

        // Tulis ke file
        let json_data = serde_json::to_string_pretty(&*processes_lock).map_err(NodeError::JsonError)?;
        fs::write(NODE_FILE, json_data).map_err(NodeError::IoError)?;

        Ok(())
    }
}