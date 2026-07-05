// mcore/melisad/services/node/radix.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

pub struct RadixNode {
    pub children: DashMap<String, Arc<RadixNode>>,
    pub sparse_indices: parking_lot::RwLock<Vec<u32>>,
    live_count: AtomicU32,
}

impl RadixNode {
    pub fn new() -> Self {
        RadixNode {
            children: DashMap::new(),
            sparse_indices: parking_lot::RwLock::new(Vec::new()),
            live_count: AtomicU32::new(0),
        }
    }

    #[inline]
    fn inc_live(&self) {
        self.live_count.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    fn dec_live(&self) -> u32 {
        let prev = self
            .live_count
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                Some(v.saturating_sub(1))
            });
        prev.unwrap_or(0).saturating_sub(1)
    }

    #[inline]
    fn live(&self) -> u32 {
        self.live_count.load(Ordering::Relaxed)
    }
}

impl Default for RadixNode {
    fn default() -> Self {
        Self::new()
    }
}

pub type RadixRoutingTable = DashMap<String, Arc<RadixNode>>;

#[inline]
pub fn path_segments(route_path: &str) -> Vec<&str> {
    route_path.split('/').filter(|s| !s.is_empty()).collect()
}

pub fn insert_path(table: &RadixRoutingTable, domain: &str, route_path: &str, sparse_idx: u32) {
    let root = table
        .entry(domain.to_string())
        .or_insert_with(|| Arc::new(RadixNode::new()))
        .clone();
    root.inc_live();

    let mut current = root;
    for seg in path_segments(route_path) {
        let next = current
            .children
            .entry(seg.to_string())
            .or_insert_with(|| Arc::new(RadixNode::new()))
            .clone();
        next.inc_live();
        current = next;
    }

    current.sparse_indices.write().push(sparse_idx);
}

pub fn remove_path(table: &RadixRoutingTable, domain: &str, route_path: &str, sparse_idx: u32) {
    let Some(root_entry) = table.get(domain) else {
        return;
    };
    let root = Arc::clone(&*root_entry);
    drop(root_entry);

    let segments = path_segments(route_path);

    let mut path_nodes: Vec<(Option<String>, Arc<RadixNode>)> =
        Vec::with_capacity(segments.len() + 1);
    path_nodes.push((None, root));

    {
        let mut current = Arc::clone(&path_nodes[0].1);
        for seg in &segments {
            let next = {
                let Some(next_ref) = current.children.get(*seg) else {
                    return;
                };
                Arc::clone(&*next_ref)
            };

            path_nodes.push((Some(seg.to_string()), Arc::clone(&next)));
            current = next;
        }
    }

    let (_, leaf) = path_nodes.last().expect("path_nodes root");
    leaf.sparse_indices.write().retain(|&idx| idx != sparse_idx);

    let mut remaining_per_level = Vec::with_capacity(path_nodes.len());
    for (_, node) in path_nodes.iter().rev() {
        remaining_per_level.push(node.dec_live());
    }

    remaining_per_level.reverse();

    for i in (0..path_nodes.len()).rev() {
        let (seg_name, node) = &path_nodes[i];
        let remaining = remaining_per_level[i];

        if remaining != 0 {
            break;
        }

        if !node.children.is_empty() {
            break;
        }

        match seg_name {
            Some(seg) => {
                if let Some(parent_entry) = path_nodes.get(i.wrapping_sub(1)) {
                    parent_entry.1.children.remove(seg);
                }
            }
            None => {
                table.remove_if(domain, |_, v| Arc::ptr_eq(v, node));
            }
        }
    }
}

pub fn find_duplicate(
    table: &RadixRoutingTable,
    domain: &str,
    route_path: &str,
    url_matches: impl Fn(u32) -> bool,
) -> bool {
    let Some(root) = table.get(domain) else {
        return false;
    };
    let current = root.clone();
    drop(root);
    let mut current = current;

    for seg in path_segments(route_path) {
        let next = {
            let Some(next_ref) = current.children.get(seg) else {
                return false;
            };
            Arc::clone(&*next_ref)
        };
        current = next;
    }

    let indices = current.sparse_indices.read();
    indices.iter().any(|&idx| url_matches(idx))
}

pub fn total_node_count(table: &RadixRoutingTable) -> usize {
    fn count_subtree(node: &RadixNode) -> usize {
        let mut total = 1;
        for entry in node.children.iter() {
            total += count_subtree(entry.value());
        }
        total
    }

    table.iter().map(|entry| count_subtree(entry.value())).sum()
}

// TODO: TULIS TEST
