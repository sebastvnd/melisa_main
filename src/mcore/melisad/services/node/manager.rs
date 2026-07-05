// mcore/melisad/services/node/manager.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use url::Url;

use crate::mcore::config::load_config::{CONFIG, PID_END, PID_START, SHARD_COUNT};
use crate::mcore::errors::enode::NodeError;
use crate::mcore::errors::eshard::ShardError;
use crate::mcore::melisad::services::node::models::{NodeHealth, NodeIdentity, NodeProcessView};
use crate::mcore::melisad::services::node::radix::{self, RadixRoutingTable};
use crate::mcore::melisad::services::node::shard::NodeShard;
use crate::mcore::melisad::services::node::types::{join_sparse_idx, shard_size, split_sparse_idx};

// Global singleton - NODE_MANAGER
pub static NODE_MANAGER: Lazy<NodeManager> = Lazy::new(|| {
    let storage_path = CONFIG.nodes.storage_file.clone();
    NodeManager::new(&storage_path)
});

pub struct NodeManager {
    pub(crate) shards: Box<[Mutex<NodeShard>; SHARD_COUNT]>,
    pub(crate) radix_table: RadixRoutingTable,
    next_shard: AtomicUsize,
    pub storage_path: String,
}

impl NodeManager {
    pub fn new(storage_path: &str) -> Self {
        let shards: Vec<Mutex<NodeShard>> = (0..SHARD_COUNT)
            .map(|id| Mutex::new(NodeShard::new(shard_size(id))))
            .collect();

        let mgr = NodeManager {
            shards: shards
                .into_boxed_slice()
                .try_into()
                .unwrap_or_else(|_| unreachable!("SHARD_COUNT fixed")),
            radix_table: RadixRoutingTable::new(),
            next_shard: AtomicUsize::new(0),
            storage_path: storage_path.to_string(),
        };

        mgr.load_from_disk();
        mgr
    }

    #[inline]
    fn pick_shard(&self) -> usize {
        self.next_shard.fetch_add(1, Ordering::Relaxed) % SHARD_COUNT
    }

    pub(crate) fn pick_shard_for_insert(&self) -> usize {
        const SAMPLE_SIZE_SPOT: usize = 4;

        let start = self.pick_shard();
        let mut best_shard = start;
        let mut best_load = u32::MAX;

        for offset in 0..SAMPLE_SIZE_SPOT {
            let shard_id = (start + offset) % SHARD_COUNT;
            let load = self.shards[shard_id].lock().load_estimate();
            if load < best_load {
                best_load = load;
                best_shard = shard_id;
            }
        }
        best_shard
    }

    pub fn create(
        &self,
        name: &str,
        url: &str,
        domain: &str,
        route_path: &str,
        registered_from_ip: &str,
        version: &str,
    ) -> Result<u32, NodeError> {
        self.create_with_inviter(
            name,
            url,
            domain,
            route_path,
            registered_from_ip,
            version,
            None,
        )
    }

    pub fn create_with_inviter(
        &self,
        name: &str,
        url: &str,
        domain: &str,
        route_path: &str,
        registered_from_ip: &str,
        version: &str,
        invited_by: Option<u32>,
    ) -> Result<u32, NodeError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(NodeError::InvalidInput("name cannot be empty".to_string()));
        }

        let url = normalize_url(url)?;
        let domain = normalize_domain(domain)?;
        let route_path = normalize_route_path(route_path)?;

        // check duplication
        let is_duplicate =
            radix::find_duplicate(&self.radix_table, &domain, &route_path, |sparse_idx| {
                let (shard_id, local_idx) = split_sparse_idx(sparse_idx);
                let shard = self.shards[shard_id].lock();
                shard
                    .get_identity(local_idx)
                    .map(|id| id.url == url)
                    .unwrap_or(false)
            });

        if is_duplicate {
            return Err(NodeError::AlreadyExists);
        }

        let identity = NodeIdentity::new_with_inviter(
            name.to_string(),
            url.clone(),
            domain.clone(),
            route_path.clone(),
            registered_from_ip.to_string(),
            version.to_string(),
            invited_by,
        );

        let preferred_shard = self.pick_shard_for_insert();
        {
            let mut shard = self.shards[preferred_shard].lock();
            if let Ok((local_idx, _health)) = shard.insert(identity.clone()) {
                drop(shard);
                let sparse_idx = join_sparse_idx(preferred_shard, local_idx);
                radix::insert_path(&self.radix_table, &domain, &route_path, sparse_idx);
                let _ = self.flush();
                return Ok(PID_START + sparse_idx);
            }
        }

        for offset in 1..SHARD_COUNT {
            let shard_id = (preferred_shard + offset) % SHARD_COUNT;
            let mut shard = self.shards[shard_id].lock();
            match shard.insert(identity.clone()) {
                Ok((local_idx, _health)) => {
                    drop(shard);
                    let sparse_idx = join_sparse_idx(shard_id, local_idx);
                    radix::insert_path(&self.radix_table, &domain, &route_path, sparse_idx);
                    let _ = self.flush();
                    return Ok(PID_START + sparse_idx);
                }
                Err(ShardError::Full) => continue,
                Err(ShardError::NotFound) => unreachable!(),
            }
        }

        Err(NodeError::RegistryFull)
    }

    pub fn delete(&self, pid: u32) -> Result<(), NodeError> {
        // TODO: pindahkan pengecekan ke layer services
        if pid < PID_START || pid > PID_END {
            return Err(NodeError::InvalidInput(
                "Out of PID Range bounds".to_string(),
            ));
        }

        let sparse_idx = pid - PID_START;
        let (shard_id, local_idx) = split_sparse_idx(sparse_idx);

        let identity = {
            let shard = self.shards[shard_id].lock();
            shard.get_identity(local_idx).ok_or(NodeError::NotFound)?
        };

        // Delete from radix frist
        radix::remove_path(
            &self.radix_table,
            &identity.domain,
            &identity.route_path,
            sparse_idx,
        );

        let mut shard = self.shards[shard_id].lock();
        match shard.remove(local_idx) {
            Ok(_) => {
                drop(shard);
                let _ = self.flush();
                Ok(())
            }
            Err(ShardError::NotFound) => Err(NodeError::NotFound),
            Err(ShardError::Full) => unreachable!(),
        }
    }

    pub fn get_identity(&self, pid: u32) -> Option<NodeIdentity> {
        let sparse_idx = pid.checked_sub(PID_START)?;
        if sparse_idx > PID_END - PID_START {
            return None;
        }
        let (shard_id, local_idx) = split_sparse_idx(sparse_idx);
        self.shards[shard_id].lock().get_identity(local_idx)
    }

    /// Ambil Arc<NodeHealth> berdasarkan PID.
    /// Lock shard hanya sesaat untuk clone Arc - health-checker tidak perlu
    /// mengunci shard lagi setelah mendapat handle ini.
    pub fn get_health_handle(&self, pid: u32) -> Option<Arc<NodeHealth>> {
        let sparse_idx = pid.checked_sub(PID_START)?;
        if sparse_idx > PID_END - PID_START {
            return None;
        }
        let (shard_id, local_idx) = split_sparse_idx(sparse_idx);
        self.shards[shard_id].lock().get_health_handle(local_idx)
    }

    /// Get view lengkap (identity + health snapshot) untuk satu PID.
    /// Dipakai oleh API response / CLI - bukan hot path internal.
    pub fn get(&self, pid: u32) -> Option<NodeProcessView> {
        let identity = self.get_identity(pid)?;
        let health = self.get_health_handle(pid)?;
        Some(NodeProcessView {
            pid,
            identity,
            health: health.snapshot(),
        })
    }

    /// List semua PID aktif (terurut naik karena iterasi sparse_idx linear).
    pub fn list(&self) -> Vec<u32> {
        let mut result = Vec::new();
        for shard_id in 0..SHARD_COUNT {
            let shard = self.shards[shard_id].lock();
            for local_idx in shard.active_local_indices() {
                let sparse_idx = join_sparse_idx(shard_id, local_idx);
                result.push(PID_START + sparse_idx);
            }
        }
        result
    }

    pub fn total_active(&self) -> usize {
        self.shards.iter().map(|s| s.lock().active_count()).sum()
    }

    /// Cari node berdasarkan URL (untuk deduplication check di api/services.rs).
    /// Mengiterasi shard satu per satu - bukan hot path, hanya saat registrasi.
    pub fn find_by_url(&self, url: &str) -> Option<(u32, NodeIdentity)> {
        for shard_id in 0..SHARD_COUNT {
            let shard = self.shards[shard_id].lock();
            for local_idx in shard.active_local_indices() {
                if let Some(identity) = shard.get_identity(local_idx) {
                    if identity.url == url {
                        let sparse_idx = join_sparse_idx(shard_id, local_idx);
                        let pid = PID_START + sparse_idx;
                        return Some((pid, identity));
                    }
                }
            }
        }
        None
    }

    pub fn find_by_invite_code(&self, invite_code: &str) -> Option<(u32, NodeIdentity)> {
        let invite_code = invite_code.trim();
        if invite_code.is_empty() {
            return None;
        }

        for shard_id in 0..SHARD_COUNT {
            let shard = self.shards[shard_id].lock();
            for local_idx in shard.active_local_indices() {
                if let Some(identity) = shard.get_identity(local_idx) {
                    if identity.invite_code == invite_code {
                        let sparse_idx = join_sparse_idx(shard_id, local_idx);
                        return Some((PID_START + sparse_idx, identity));
                    }
                }
            }
        }
        None
    }

    /// Cari node berdasarkan status kesehatan - dipakai oleh monitoring.
    pub fn get_pids_by_status(
        &self,
        filter: impl Fn(&crate::mcore::melisad::services::node::types::NodeStatus) -> bool,
    ) -> Vec<u32> {
        let mut result = Vec::new();
        for shard_id in 0..SHARD_COUNT {
            let shard = self.shards[shard_id].lock();
            for local_idx in shard.active_local_indices() {
                if let Some(health) = shard.get_health_handle(local_idx) {
                    if filter(&health.status()) {
                        let sparse_idx = join_sparse_idx(shard_id, local_idx);
                        result.push(PID_START + sparse_idx);
                    }
                }
            }
        }
        result
    }

    /// Jumlah RadixNode dalam routing table (untuk monitoring/metrik).
    pub fn radix_node_count(&self) -> usize {
        radix::total_node_count(&self.radix_table)
    }
}

impl Default for NodeManager {
    fn default() -> Self {
        Self::new("nodes.json")
    }
}

fn normalize_url(url: &str) -> Result<String, NodeError> {
    let url = url.trim().trim_end_matches('/').to_string();
    if url.is_empty() {
        return Err(NodeError::InvalidInput("url cannot be empty".to_string()));
    }
    let parsed = Url::parse(&url)
        .map_err(|_| NodeError::InvalidInput("url must be a valid http/https URL".to_string()))?;
    match parsed.scheme() {
        "http" | "https" => Ok(url),
        _ => Err(NodeError::InvalidInput(
            "url scheme must be http or https".to_string(),
        )),
    }
}

fn normalize_domain(domain: &str) -> Result<String, NodeError> {
    let domain = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    if domain.is_empty() {
        return Err(NodeError::InvalidInput(
            "domain cannot be empty".to_string(),
        ));
    }
    if domain.contains('/') {
        return Err(NodeError::InvalidInput(
            "domain must not contain a path".to_string(),
        ));
    }
    Ok(domain)
}

fn normalize_route_path(route_path: &str) -> Result<String, NodeError> {
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
