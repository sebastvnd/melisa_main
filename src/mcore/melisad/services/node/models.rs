// src/mcore/melisad/services/node/models.rs
// ✅ UPDATED: Dengan timestamp tracking dan extended metadata

use serde::{Deserialize, Serialize};
use std::fmt;

/// Node status enum - extended dengan status Suspicious
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Node sedang aktif dan responsif
    Active,
    /// Node tidak responsif terhadap health check
    Stopped,
    /// Node masih ada tapi banyak gagal (suspicious)
    Suspicious,
}

impl fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeStatus::Active => write!(f, "Active"),
            NodeStatus::Stopped => write!(f, "Stopped"),
            NodeStatus::Suspicious => write!(f, "Suspicious"),
        }
    }
}

/// Struktur data untuk tracking satu node
///
/// Alur data:
/// 1. MNode register → NodeProcess dibuat
/// 2. Setiap 30 detik, health check → update status & timestamp
/// 3. Jika Stopped > timeout → auto cleanup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeProcess {
    /// Nama node (dari registration)
    pub name: String,

    /// Hash SHA256(name+url) - unique identifier
    pub hash: String,

    /// Base URL dari node (http://10.0.0.1:3000)
    pub url: String,

    /// Domain yang di-route ke node ini
    pub domain: String,

    /// Route path (e.g., /api, /app)
    pub route_path: String,

    /// Status node saat ini
    pub status: NodeStatus,

    /// ✅ BARU: Unix timestamp saat node pertama kali di-register
    pub created_at: u64,

    /// ✅ BARU: Unix timestamp last successful health check
    /// Digunakan untuk: mengetahui kapan node terakhir hidup
    pub last_heartbeat: u64,

    /// ✅ BARU: Unix timestamp terakhir health check dilakukan
    /// Digunakan untuk: membedakan "belum di-check" vs "sudah di-check tapi failed"
    pub last_health_check: u64,

    /// ✅ BARU: Jumlah consecutive failures health check
    /// Digunakan untuk: mendeteksi node yang problematic (Suspicious status)
    pub consecutive_failures: u32,

    /// ✅ BARU: IP address dari client yang melakukan registration
    /// Digunakan untuk: debugging dan audit trail
    pub registered_from_ip: String,

    /// ✅ BARU: Versi MNode yang melakukan registration
    /// Digunakan untuk: compatibility checking
    pub version: String,
}

impl NodeProcess {
    /// Create new NodeProcess dengan timestamp initialization
    ///
    /// # Arguments
    /// * `name` - Nama node
    /// * `hash` - SHA256 hash
    /// * `url` - Base URL
    /// * `domain` - Domain routing
    /// * `route_path` - Route path
    /// * `registered_from_ip` - IP dari registrant
    /// * `version` - Versi MNode
    ///
    /// # Returns
    /// New NodeProcess dengan status=Active dan timestamps initialized ke current time
    pub fn new(
        name: String,
        hash: String,
        url: String,
        domain: String,
        route_path: String,
        registered_from_ip: String,
        version: String,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        NodeProcess {
            name,
            hash,
            url,
            domain,
            route_path,
            status: NodeStatus::Active,
            created_at: now,
            last_heartbeat: now,
            last_health_check: now,
            consecutive_failures: 0,
            registered_from_ip,
            version,
        }
    }

    /// Update node status dan timestamps setelah health check
    ///
    /// # Arguments
    /// * `new_status` - Hasil health check (Active/Stopped)
    pub fn update_health_status(&mut self, new_status: NodeStatus) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.last_health_check = now;

        match new_status {
            NodeStatus::Active => {
                // Health check success
                self.status = NodeStatus::Active;
                self.last_heartbeat = now;
                self.consecutive_failures = 0;
            }
            NodeStatus::Stopped => {
                // Health check failed
                self.status = NodeStatus::Stopped;
                self.consecutive_failures += 1;
            }
            NodeStatus::Suspicious => {
                self.status = NodeStatus::Suspicious;
            }
        }
    }

    /// Check apakah node sudah dead (tidak responsif untuk waktu tertentu)
    ///
    /// # Arguments
    /// * `timeout_seconds` - Berapa detik node boleh offline sebelum dianggap "dead"
    ///
    /// # Returns
    /// true jika last_heartbeat > timeout_seconds yang lalu
    pub fn is_dead(&self, timeout_seconds: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        (now - self.last_heartbeat) > timeout_seconds
    }

    /// Get uptime information untuk monitoring
    ///
    /// # Returns
    /// (time_online_secs, time_since_last_heartbeat_secs)
    pub fn get_uptime_info(&self) -> (u64, u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let time_online = now - self.created_at;
        let time_since_heartbeat = now - self.last_heartbeat;

        (time_online, time_since_heartbeat)
    }

    /// Human-readable summary untuk logging
    pub fn summary(&self) -> String {
        let (uptime, since_heartbeat) = self.get_uptime_info();
        format!(
            "{} [{}] url={} uptime={}s last_heartbeat={}s failures={}",
            self.name, self.status, self.url, uptime, since_heartbeat, self.consecutive_failures
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = NodeProcess::new(
            "test-node".to_string(),
            "abc123".to_string(),
            "http://localhost:3000".to_string(),
            "test.local".to_string(),
            "/api".to_string(),
            "127.0.0.1".to_string(),
            "1.0.0".to_string(),
        );

        assert_eq!(node.name, "test-node");
        assert_eq!(node.status, NodeStatus::Active);
        assert_eq!(node.consecutive_failures, 0);
    }

    #[test]
    fn test_is_dead() {
        let mut node = NodeProcess::new(
            "test-node".to_string(),
            "abc123".to_string(),
            "http://localhost:3000".to_string(),
            "test.local".to_string(),
            "/api".to_string(),
            "127.0.0.1".to_string(),
            "1.0.0".to_string(),
        );

        // Fresh node should not be dead
        assert!(!node.is_dead(3600)); // 1 hour timeout

        // Simulate node being dead
        node.last_heartbeat = 0;
        assert!(node.is_dead(1)); // Definitely dead if timeout is 1 sec
    }
}
