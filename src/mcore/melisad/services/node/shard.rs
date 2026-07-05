// mcore/melisad/services/node/shard.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::mcore::errors::eshard::ShardError;
use crate::mcore::melisad::services::node::models::{NodeHealth, NodeIdentity};
use crate::mcore::melisad::services::node::types::IndexSlot;

pub struct NodeShard {
    index_lookup: Vec<IndexSlot>,
    identities: Vec<NodeIdentity>,
    health: Vec<Arc<NodeHealth>>,
    reverse_map: Vec<u32>,
    dead_ranges: BTreeMap<u32, u32>,
    current_max_idx: u32,
    shard_size: u32,
    load_estimate: AtomicU32,
}

impl NodeShard {
    pub fn new(shard_size: u32) -> Self {
        NodeShard {
            index_lookup: vec![IndexSlot::EMPTY; shard_size as usize],
            identities: Vec::new(),
            health: Vec::new(),
            reverse_map: Vec::new(),
            dead_ranges: BTreeMap::new(),
            current_max_idx: 0,
            shard_size,
            load_estimate: AtomicU32::new(0),
        }
    }

    #[inline]
    pub fn load_estimate(&self) -> u32 {
        self.load_estimate.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn capacity(&self) -> u32 {
        self.shard_size
    }

    pub fn insert(&mut self, identity: NodeIdentity) -> Result<(u32, Arc<NodeHealth>), ShardError> {
        let local_idx = if let Some((start, end)) = self.dead_ranges.pop_first() {
            if start < end {
                self.dead_ranges.insert(start + 1, end);
            }
            start
        } else {
            if self.current_max_idx >= self.shard_size {
                self.load_estimate.store(self.shard_size, Ordering::Relaxed);
                return Err(ShardError::Full);
            }
            let idx = self.current_max_idx;
            self.current_max_idx += 1;
            idx
        };

        let dense_idx = self.identities.len() as u32;
        let health = Arc::new(NodeHealth::new());

        self.identities.push(identity);
        self.health.push(health.clone());
        self.reverse_map.push(local_idx);

        let prev_version = self.index_lookup[local_idx as usize].version;
        self.index_lookup[local_idx as usize] = IndexSlot {
            data_index: dense_idx,
            version: prev_version,
        };

        self.load_estimate
            .store(self.identities.len() as u32, Ordering::Relaxed);

        Ok((local_idx, health))
    }

    pub fn remove(&mut self, local_idx: u32) -> Result<NodeIdentity, ShardError> {
        if local_idx as usize >= self.index_lookup.len() {
            return Err(ShardError::NotFound);
        }

        let slot = self.index_lookup[local_idx as usize];
        if slot.data_index == u32::MAX {
            return Err(ShardError::NotFound);
        }

        let identity = self.swap_remove_dense(slot.data_index as usize);

        self.index_lookup[local_idx as usize].data_index = u32::MAX;
        self.index_lookup[local_idx as usize].version = self.index_lookup[local_idx as usize]
            .version
            .wrapping_add(1);

        Self::add_dead_range(&mut self.dead_ranges, local_idx);

        self.load_estimate
            .store(self.identities.len() as u32, Ordering::Relaxed);

        Ok(identity)
    }

    fn swap_remove_dense(&mut self, dense_idx: usize) -> NodeIdentity {
        let last_dense_idx = self.identities.len() - 1;

        if dense_idx != last_dense_idx {
            self.identities.swap(dense_idx, last_dense_idx);
            self.health.swap(dense_idx, last_dense_idx);
            self.reverse_map.swap(dense_idx, last_dense_idx);

            let shifted_local_idx = self.reverse_map[dense_idx];
            self.index_lookup[shifted_local_idx as usize].data_index = dense_idx as u32;
        }

        self.reverse_map.pop();
        self.health.pop();
        self.identities
            .pop()
            .expect("dense_idx valid means identities is not empty")
    }

    fn add_dead_range(dead_range: &mut BTreeMap<u32, u32>, dead_idx: u32) {
        let mut start = dead_idx;
        let mut end = dead_idx;

        if let Some((&prev_start, &prev_end)) = dead_range.range(..dead_idx).next_back() {
            if prev_end == dead_idx.wrapping_sub(1) {
                start = prev_start;
                dead_range.remove(&prev_start);
            }
        }

        if let Some(&next_end) = dead_range.get(&(dead_idx + 1)) {
            end = next_end;
            dead_range.remove(&(dead_idx + 1));
        }

        dead_range.insert(start, end);
    }

    pub fn get_identity(&self, local_idx: u32) -> Option<NodeIdentity> {
        let slot = self.index_lookup.get(local_idx as usize)?;
        if slot.data_index == u32::MAX {
            return None;
        }
        self.identities.get(slot.data_index as usize).cloned()
    }

    pub fn get_health_handle(&self, local_idx: u32) -> Option<Arc<NodeHealth>> {
        let slot = self.index_lookup.get(local_idx as usize)?;
        if slot.data_index == u32::MAX {
            return None;
        }
        self.health.get(slot.data_index as usize).cloned()
    }

    pub fn active_local_indices(&self) -> impl Iterator<Item = u32> + '_ {
        self.index_lookup
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.data_index != u32::MAX)
            .map(|(local_idx, _)| local_idx as u32)
    }

    pub fn active_count(&self) -> usize {
        self.identities.len()
    }
}

// TODO TULIS CODE TEST NYA
