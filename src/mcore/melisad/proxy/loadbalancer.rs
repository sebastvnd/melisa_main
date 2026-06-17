/// Load balancing strategies untuk node selection\
use rand::seq::{IndexedMutRandom, SliceRandom};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::mcore::melisad::services::node::{NodeManager, NodeProcess};

#[derive(Debug, Clone, Copy)]
pub enum LoadBalancingStrategy {
    /// Round-robin distribution
    RoundRobin,

    /// Least connections
    // LeastConnections,

    /// Random selection
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

    /// Select node berdasarkan domain, path, dan strategy
    pub fn select_node(
        &self,
        domain: &str,
        path: &str,
        node_manager: &NodeManager,
    ) -> Option<NodeProcess> {
        let mut matching_nodes = node_manager.find_matching_nodes_by_route(domain, path);

        if matching_nodes.is_empty() {
            return None;
        }

        // Select based on strategy
        match self.strategy {
            LoadBalancingStrategy::RoundRobin => {
                if matching_nodes.is_empty() {
                    return None;
                }

                // Menggunakan fetch_add yang sudah benar,
                // tapi pastikan tidak terjadi panic jika matching_nodes kosong
                let idx =
                    self.round_robin_index.fetch_add(1, Ordering::Relaxed) % matching_nodes.len();
                Some(matching_nodes[idx].clone())
            }

            // LoadBalancingStrategy::LeastConnections => {
            //     if matching_nodes.is_empty() {
            //         return None;
            //     }

            //     // Alih-alih melakukan sorting pada seluruh array (O(N log N)),
            //     // gunakan `min_by_key` untuk mencari nilai terkecil dalam satu putaran (O(N)).
            //     // CATATAN: Asumsikan struct node Anda memiliki field atau method `active_connections()`.
            //     let selected_node = matching_nodes
            //         .iter()
            //         .min_by_key(|n| n.active_connections) // Ganti dengan .active_connections() jika berupa method
            //         .cloned();

            //     selected_node
            // }
            LoadBalancingStrategy::Random => {
                if matching_nodes.is_empty() {
                    return None;
                }

                let mut rng = rand::rng();
                matching_nodes.choose_mut(&mut rng).cloned()
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
