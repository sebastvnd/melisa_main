// src/mcore/melisad/probes/startup_node.rs
// ✅ UPDATED: Dengan auto-cleanup, suspicious detection, dan enhanced logging

use crate::mcore::errors::enode::NodeError;
use crate::mcore::melisad::probes::liveness_node::check_node_network_with_client;
use crate::mcore::melisad::services::node::{NodeManager, NodeStatus};
use crate::mcore::mlog::LOGGER;
use std::sync::Arc;
use std::time::Duration;

impl NodeManager {
    /// Main health check function dengan cleanup dan monitoring
    /// 
    /// Alur:
    /// 1. Buat HTTP client untuk concurrent checks
    /// 2. Lock read untuk get semua nodes
    /// 3. Lakukan health check concurrent ke semua nodes
    /// 4. Lock write untuk update status
    /// 5. Apply cleanup policies
    /// 6. Save state to disk
    pub async fn startup_node_check(&self) -> std::result::Result<(), NodeError> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_default();

        // Get current timestamp untuk comparison
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // ============================================================
        // Step 1: Lock read untuk persiapan concurrent checks
        // ============================================================
        let checks = {
            let processes_lock = self.processes.read().unwrap();
            let mut tasks = Vec::new();

            for (hash, node) in processes_lock.iter() {
                let hash_clone = hash.clone();
                let url_clone = node.url.clone();
                let client_clone = http_client.clone();

                let check_future = async move {
                    let status = check_node_network_with_client(&client_clone, &url_clone).await;
                    (hash_clone, status)
                };
                tasks.push(check_future);
            }
            tasks
        }; // Lock otomatis lepas di sini

        // ============================================================
        // Step 2: Execute concurrent health checks
        // ============================================================
        let results = futures::future::join_all(checks).await;

        // ============================================================
        // Step 3: Lock write untuk update status dan apply policies
        // ============================================================
        {
            let mut processes_lock = self.processes.write().unwrap();

            // Clone map untuk mutasi
            let mut new_map = (**processes_lock).clone();

            // Track nodes yang akan di-remove
            let mut nodes_to_remove = Vec::new();
            let mut status_changes = Vec::new();

            // ============================================================
            // Process health check results
            // ============================================================
            for (hash, new_status) in results {
                if let Some(node) = new_map.get_mut(&hash) {
                    let old_status = node.status.clone();

                    // Update health status (ini akan update consecutive_failures juga)
                    node.update_health_status(new_status.clone());

                    // Track perubahan status untuk logging
                    if old_status != node.status {
                        status_changes.push((
                            hash.clone(),
                            node.name.clone(),
                            old_status.clone(),
                            node.status.clone(),
                        ));
                    }

                    // ============================================================
                    // Apply cleanup policies
                    // ============================================================

                    // Policy 1: Auto-remove jika node offline > 1 hour (3600 seconds)
                    if node.status == NodeStatus::Stopped {
                        let time_since_last_heartbeat = now - node.last_heartbeat;
                        let timeout_secs = 3600; // 1 hour

                        if time_since_last_heartbeat > timeout_secs {
                            nodes_to_remove.push((
                                hash.clone(),
                                node.name.clone(),
                                time_since_last_heartbeat,
                            ));
                            continue;
                        }
                    }

                    // Policy 2: Mark as Suspicious jika consecutive failures > threshold
                    if node.consecutive_failures > 20 && node.status != NodeStatus::Suspicious {
                        node.status = NodeStatus::Suspicious;
                        status_changes.push((
                            hash.clone(),
                            node.name.clone(),
                            NodeStatus::Stopped,
                            NodeStatus::Suspicious,
                        ));
                    }

                    // Policy 3: Remove Suspicious nodes yang sudah fail > 50 times
                    if node.status == NodeStatus::Suspicious && node.consecutive_failures > 50 {
                        nodes_to_remove.push((
                            hash.clone(),
                            node.name.clone(),
                            node.consecutive_failures as u64,
                        ));
                        continue;
                    }
                }
            }

            // ============================================================
            // Execute removals
            // ============================================================
            for (hash, name, reason) in nodes_to_remove {
                new_map.remove(&hash);

                let log_msg = if reason > 50 {
                    // reason is consecutive_failures
                    format!(
                        "Auto-cleanup: Removed node '{}' [{}] - too many failures: {}",
                        name, hash, reason
                    )
                } else {
                    // reason is time_since_last_heartbeat in seconds
                    let hours = reason / 3600;
                    let minutes = (reason % 3600) / 60;
                    format!(
                        "Auto-cleanup: Removed node '{}' [{}] - offline for {}h {}m",
                        name, hash, hours, minutes
                    )
                };

                let _ = LOGGER.log_info(&log_msg);
            }

            // ============================================================
            // Log status changes untuk monitoring
            // ============================================================
            for (hash, name, old_status, new_status) in status_changes {
                let _ = LOGGER.log_debug(&format!(
                    "Node status change: '{}' [{}] {} → {}",
                    name, hash, old_status, new_status
                ));
            }

            // ============================================================
            // Update processes dengan new map
            // ============================================================
            *processes_lock = Arc::new(new_map);
        } // Write lock otomatis lepas di sini

        // ============================================================
        // Step 4: Save state to disk
        // ============================================================
        self.save_state_to_disk()?;

        Ok(())
    }

    /// Fungsi utilitas untuk menulis state ke JSON file
    fn save_state_to_disk(&self) -> std::result::Result<(), NodeError> {
        self.flush()?;
        Ok(())
    }

    /// ============================================================
    /// NEW: Utility functions untuk operational monitoring
    /// ============================================================

    /// Manual cleanup untuk nodes yang sudah dead > timeout_seconds
    /// 
    /// Berguna untuk:
    /// - Manual intervention jika auto-cleanup tidak berjalan
    /// - Batch cleanup dengan custom timeout
    /// - Administrative tasks
    /// 
    /// # Arguments
    /// * `timeout_seconds` - Node offline lebih dari ini akan di-remove
    /// 
    /// # Returns
    /// Jumlah nodes yang di-remove
    pub async fn cleanup_node(&self, timeout_seconds: u64) -> std::result::Result<usize, NodeError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut count = 0;
        {
            let mut processes_lock = self.processes.write().unwrap();
            let mut new_map = (**processes_lock).clone();

            let nodes_to_remove: Vec<_> = new_map
                .iter()
                .filter_map(|(hash, node)| {
                    let time_since_last_heartbeat = now - node.last_heartbeat;
                    if time_since_last_heartbeat > timeout_seconds {
                        Some((hash.clone(), node.name.clone(), time_since_last_heartbeat))
                    } else {
                        None
                    }
                })
                .collect();

            for (hash, name, time_offline) in nodes_to_remove {
                new_map.remove(&hash);
                count += 1;

                let _ = LOGGER.log_info(&format!(
                    "Manual cleanup: Removed node '{}' [{}] - offline for {} seconds",
                    name, hash, time_offline
                ));
            }

            *processes_lock = Arc::new(new_map);
        }

        self.save_state_to_disk()?;
        Ok(count)
    }

    /// Get diagnostic info tentang semua nodes
    /// Berguna untuk monitoring dashboard dan troubleshooting
    pub fn get_health_diagnostic(&self) -> HealthDiagnostic {
        let processes_lock = self.processes.read().unwrap();

        let mut total_nodes = 0;
        let mut active_count = 0;
        let mut stopped_count = 0;
        let mut suspicious_count = 0;
        let mut total_failures = 0;
        let mut nodes_with_high_failures = Vec::new();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for (_hash, node) in processes_lock.iter() {
            total_nodes += 1;

            match node.status {
                NodeStatus::Active => active_count += 1,
                NodeStatus::Stopped => stopped_count += 1,
                NodeStatus::Suspicious => suspicious_count += 1,
            }

            total_failures += node.consecutive_failures as u64;

            if node.consecutive_failures > 10 {
                nodes_with_high_failures.push((
                    node.name.clone(),
                    node.consecutive_failures,
                    now - node.last_heartbeat,
                ));
            }
        }

        HealthDiagnostic {
            total_nodes,
            active: active_count,
            stopped: stopped_count,
            suspicious: suspicious_count,
            total_accumulated_failures: total_failures,
            high_failure_nodes: nodes_with_high_failures,
        }
    }
}

// ============================================================
// Data structures untuk monitoring
// ============================================================

#[derive(Debug, serde::Serialize)]
pub struct HealthDiagnostic {
    pub total_nodes: usize,
    pub active: usize,
    pub stopped: usize,
    pub suspicious: usize,
    pub total_accumulated_failures: u64,
    pub high_failure_nodes: Vec<(String, u32, u64)>, // (name, failure_count, offline_secs)
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[tokio::test]
//     async fn test_cleanup_dead_nodes() {
//         // Test case untuk ensure cleanup logic works correctly
//         // Membutuhkan proper test setup dengan NODE_MANAGER mock
//     }
// }
