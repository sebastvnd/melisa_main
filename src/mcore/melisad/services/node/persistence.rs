// mcore/melisad/services/node/persistence.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

use crate::mcore::config::load_config::SHARD_COUNT;
use crate::mcore::errors::enode::NodeError;
use crate::mcore::melisad::services::node::manager::NodeManager;
use crate::mcore::melisad::services::node::models::NodeIdentity;
use crate::mcore::melisad::services::node::radix;
use crate::mcore::melisad::services::node::shard::NodeShard;
use crate::mcore::melisad::services::node::types::{join_sparse_idx, shard_size};
use crate::mcore::mlog::LOGGER;
use std::fs;

impl NodeManager {
    #[cfg(not(test))]
    pub fn flush(&self) -> Result<(), NodeError> {
        let mut all_identities = Vec::new();

        for shard_id in 0..SHARD_COUNT {
            let shard = self.shards[shard_id].lock();
            for local_idx in shard.active_local_indices() {
                if let Some(identity) = shard.get_identity(local_idx) {
                    all_identities.push(identity);
                }
            }
        }

        let json = serde_json::to_string_pretty(&all_identities)?;
        fs::write(&self.storage_path, json)?;
        Ok(())
    }

    #[cfg(test)]
    pub fn flush(&self) -> Result<(), NodeError> {
        Ok(())
    }

    pub(crate) fn load_from_disk(&self) {
        let content = match fs::read_to_string(&self.storage_path) {
            Ok(content) if !content.trim().is_empty() => content,
            _ => return,
        };

        if content.trim() == "{}" {
            return;
        }

        let identities: Vec<NodeIdentity> = match serde_json::from_str(&content) {
            Ok(identities) => identities,
            Err(err) => {
                let _ = LOGGER.log_error(&format!(
                    "persistence: failed to parse node file '{}': {}",
                    self.storage_path, err
                ));
                return;
            }
        };

        let mut loaded = 0usize;
        for identity in identities {
            let preferred = self.pick_shard_for_insert();
            let domain = identity.domain.clone();
            let route_path = identity.route_path.clone();
            let mut inserted = false;

            for offset in 0..SHARD_COUNT {
                let shard_id = (preferred + offset) % SHARD_COUNT;
                let mut shard = self.shards[shard_id].lock();
                if let Ok((local_idx, _health)) = shard.insert(identity.clone()) {
                    drop(shard);
                    let sparse_idx = join_sparse_idx(shard_id, local_idx);
                    radix::insert_path(&self.radix_table, &domain, &route_path, sparse_idx);
                    inserted = true;
                    loaded += 1;
                    break;
                }
            }

            if !inserted {
                let _ = LOGGER.log_error(&format!(
                    "persistence: registry full while loading '{}' from disk",
                    identity.name
                ));
            }
        }

        if loaded > 0 {
            let _ = LOGGER.log_info(&format!(
                "persistence: loaded {} node(s) from '{}'",
                loaded, self.storage_path
            ));
        }
    }

    #[cfg(test)]
    pub fn reset_for_test(&self) {
        for shard_id in 0..SHARD_COUNT {
            let mut shard = self.shards[shard_id].lock();
            *shard = NodeShard::new(shard_size(shard_id));
        }
        self.radix_table.clear();
    }
}
