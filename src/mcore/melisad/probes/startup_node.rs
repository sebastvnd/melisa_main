// mcore/melisad/probes/startup_node.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

use crate::mcore::config::load_config::{PID_START, SHARD_COUNT};
use crate::mcore::errors::enode::NodeError;
use crate::mcore::melisad::probes::liveness_node::check_node_network_with_client;
use crate::mcore::melisad::services::node::manager::NodeManager;
use crate::mcore::melisad::services::node::models::NodeHealth;
use crate::mcore::melisad::services::node::types::{NodeStatus, join_sparse_idx};
use crate::mcore::mlog::LOGGER;
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
pub struct HealthDiagnostic {
    pub total_nodes: usize,
    pub active: usize,
    pub stopped: usize,
    pub suspicious: usize,
    pub total_accumulated_failures: u64,
    pub high_failure_nodes: Vec<(String, u32, u64)>,
}

impl NodeManager {
    pub async fn startup_node_check(&self) -> Result<(), NodeError> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_default();

        let mut check_targets: Vec<(u32, String, Arc<NodeHealth>)> = Vec::new();
        for shard_id in 0..SHARD_COUNT {
            let shard = self.shards[shard_id].lock();
            for local_idx in shard.active_local_indices() {
                if let (Some(identity), Some(health)) = (
                    shard.get_identity(local_idx),
                    shard.get_health_handle(local_idx),
                ) {
                    let sparse_idx = join_sparse_idx(shard_id, local_idx);
                    check_targets.push((PID_START + sparse_idx, identity.url, health));
                }
            }
        }

        let checks = check_targets.into_iter().map(|(pid, url, health)| {
            let client = http_client.clone();
            async move {
                let status = check_node_network_with_client(&client, &url).await;
                health.record_health_check(status);
                (pid, health)
            }
        });

        let results = futures::future::join_all(checks).await;

        for (pid, health) in results {
            if health.status() == NodeStatus::Stopped && health.is_dead(3600) {
                if self.delete(pid).is_ok() {
                    let _ = LOGGER.log_info(&format!(
                        "startup_check: removed dead node [PID {}] after 1 hour offline",
                        pid
                    ));
                }
            }
        }

        self.flush()?;
        Ok(())
    }

    pub fn get_health_diagnostic(&self) -> HealthDiagnostic {
        let now = now_sec();
        let mut total_nodes = 0;
        let mut active = 0;
        let mut stopped = 0;
        let mut suspicious = 0;
        let mut total_failures = 0u64;
        let mut high_failure_nodes = Vec::new();

        for shard_id in 0..SHARD_COUNT {
            let shard = self.shards[shard_id].lock();
            for local_idx in shard.active_local_indices() {
                if let (Some(identity), Some(health)) = (
                    shard.get_identity(local_idx),
                    shard.get_health_handle(local_idx),
                ) {
                    total_nodes += 1;
                    let failures = health.consecutive_failures();
                    total_failures += failures as u64;

                    match health.status() {
                        NodeStatus::Active => active += 1,
                        NodeStatus::Stopped => stopped += 1,
                        NodeStatus::Suspicious => suspicious += 1,
                    }

                    if failures > 10 {
                        high_failure_nodes.push((
                            identity.name,
                            failures,
                            now.saturating_sub(health.last_heartbeat()),
                        ));
                    }
                }
            }
        }

        HealthDiagnostic {
            total_nodes,
            active,
            stopped,
            suspicious,
            total_accumulated_failures: total_failures,
            high_failure_nodes,
        }
    }

    pub async fn cleanup_node(&self, timeout_second: u64) -> Result<usize, NodeError> {
        let now = now_sec();
        let mut pids_to_remove = Vec::new();

        for shard_id in 0..SHARD_COUNT {
            let shard = self.shards[shard_id].lock();
            for local_idx in shard.active_local_indices() {
                if let (Some(identity), Some(health)) = (
                    shard.get_identity(local_idx),
                    shard.get_health_handle(local_idx),
                ) {
                    let time_offline = now.saturating_sub(health.last_heartbeat());
                    if time_offline > timeout_second {
                        let sparse_idx = join_sparse_idx(shard_id, local_idx);
                        pids_to_remove.push((PID_START + sparse_idx, identity.name, time_offline));
                    }
                }
            }
        }

        let mut count = 0;
        for (pid, name, time_offline) in pids_to_remove {
            if self.delete(pid).is_ok() {
                count += 1;
                let _ = LOGGER.log_info(&format!(
                    "cleanup: removed node '{}' [PID {}] after {}s offline",
                    name, pid, time_offline
                ));
            }
        }

        Ok(count)
    }
}

fn now_sec() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
