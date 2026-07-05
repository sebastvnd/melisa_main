// mcore/melisad/proxy/loadbalancer.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

use rand::prelude::IndexedRandom;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::mcore::melisad::services::node::manager::NodeManager;
use crate::mcore::melisad::services::node::models::NodeProcessView;

#[derive(Debug, Clone, Copy)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    Random,
}

#[derive(Clone)]
pub struct LoadBalancer {
    strategy: LoadBalancingStrategy,
    round_robin_index: Arc<AtomicUsize>,
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        LoadBalancer {
            strategy,
            round_robin_index: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn select_node(
        &self,
        domain: &str,
        path: &str,
        node_manager: &NodeManager,
    ) -> Option<NodeProcessView> {
        let matching_nodes = node_manager.find_matching_nodes_by_route(domain, path);
        if matching_nodes.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalancingStrategy::RoundRobin => {
                let idx =
                    self.round_robin_index.fetch_add(1, Ordering::Relaxed) % matching_nodes.len();
                Some(matching_nodes[idx].clone())
            }
            LoadBalancingStrategy::Random => {
                let mut rng = rand::rng();
                matching_nodes.choose(&mut rng).cloned()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_balancer_creation() {
        let lb = LoadBalancer::new(LoadBalancingStrategy::RoundRobin);
        assert_eq!(lb.round_robin_index.load(Ordering::Relaxed), 0);
    }
}
