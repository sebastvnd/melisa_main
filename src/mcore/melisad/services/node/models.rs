// mcore/melisad/services/node/models.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

// src/mcore/melisad/services/node/models.rs

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::mcore::melisad::services::node::types::{AtomicNodeStatus, NodeStatus};

#[inline]
fn now_sec() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeIdentity {
    pub name: String,
    pub url: String,
    pub domain: String,
    pub route_path: String,
    pub registered_from_ip: String,
    pub version: String,
    pub created_at: u64,
    #[serde(default = "generate_invite_code")]
    pub invite_code: String,
    #[serde(default)]
    pub invited_by: Option<u32>,
}

impl NodeIdentity {
    pub fn new(
        name: String,
        url: String,
        domain: String,
        route_path: String,
        registered_from_ip: String,
        version: String,
    ) -> Self {
        Self::new_with_inviter(
            name,
            url,
            domain,
            route_path,
            registered_from_ip,
            version,
            None,
        )
    }

    pub fn new_with_inviter(
        name: String,
        url: String,
        domain: String,
        route_path: String,
        registered_from_ip: String,
        version: String,
        invited_by: Option<u32>,
    ) -> Self {
        NodeIdentity {
            name,
            url,
            domain,
            route_path,
            registered_from_ip,
            version,
            created_at: now_sec(),
            invite_code: generate_invite_code(),
            invited_by,
        }
    }
}

fn generate_invite_code() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

#[cfg_attr(feature = "health-cache-padding", repr(align(64)))]
#[derive(Debug)]
pub struct NodeHealth {
    status: AtomicNodeStatus,
    last_heartbeat: AtomicU64,
    last_health_check: AtomicU64,
    consecutive_failures: AtomicU32,
}

impl NodeHealth {
    pub fn new() -> Self {
        let now = now_sec();
        NodeHealth {
            status: AtomicNodeStatus::new(NodeStatus::Active),
            last_heartbeat: AtomicU64::new(now),
            last_health_check: AtomicU64::new(now),
            consecutive_failures: AtomicU32::new(0),
        }
    }

    #[inline]
    pub fn status(&self) -> NodeStatus {
        self.status.load()
    }

    #[inline]
    pub fn last_heartbeat(&self) -> u64 {
        self.last_heartbeat.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn last_health_check(&self) -> u64 {
        self.last_health_check.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures.load(Ordering::Relaxed)
    }

    pub fn record_health_check(&self, result: NodeStatus) {
        let now = now_sec();
        self.last_health_check.store(now, Ordering::Relaxed);

        match result {
            NodeStatus::Active => {
                self.status.store(NodeStatus::Active);
                self.last_heartbeat.store(now, Ordering::Relaxed);
                self.consecutive_failures.store(0, Ordering::Relaxed);
            }
            NodeStatus::Stopped => {
                self.status.store(NodeStatus::Stopped);
                self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
            }
            NodeStatus::Suspicious => {
                self.status.store(NodeStatus::Suspicious);
            }
        }
    }

    pub fn record_heartbeat(&self) {
        let now = now_sec();
        self.last_heartbeat.store(now, Ordering::Relaxed);
        self.status.store(NodeStatus::Active);
        self.consecutive_failures.store(0, Ordering::Relaxed);
    }

    #[inline]
    pub fn is_dead(&self, timeout_seconds: u64) -> bool {
        let now = now_sec();
        let last = self.last_heartbeat();
        now.saturating_sub(last) > timeout_seconds
    }

    // this fn for report admin (api respons)
    // dont run in hot path because full cloning node data
    pub fn snapshot(&self) -> NodeHealthSnapshot {
        NodeHealthSnapshot {
            status: self.status().into(),
            last_heartbeat: self.last_heartbeat(),
            last_health_check: self.last_health_check(),
            consecutive_failures: self.consecutive_failures(),
        }
    }
}

impl Default for NodeHealth {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealthSnapshot {
    pub status: NodeStatusSerde,
    pub last_heartbeat: u64,
    pub last_health_check: u64,
    pub consecutive_failures: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatusSerde {
    Active,
    Stopped,
    Suspicious,
}

impl From<NodeStatus> for NodeStatusSerde {
    fn from(s: NodeStatus) -> Self {
        match s {
            NodeStatus::Active => NodeStatusSerde::Active,
            NodeStatus::Stopped => NodeStatusSerde::Stopped,
            NodeStatus::Suspicious => NodeStatusSerde::Suspicious,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeProcessView {
    pub pid: u32,
    pub identity: NodeIdentity,
    pub health: NodeHealthSnapshot,
}

// TODO: ADD TEST CODE
