/// Node CRUD operations
use crate::mcore::config::load_config::CONFIG;
use crate::mcore::errors::enode::NodeError;
use crate::mcore::melisad::services::hashing::generate_hash;
use crate::mcore::melisad::services::mconf::{PID_END, PID_START};
use crate::mcore::melisad::services::node::manager::NodeManager;
use crate::mcore::melisad::services::node::models::{NodeProcess, NodeStatus};
use std::sync::atomic::Ordering;

impl NodeManager {
    /// Buat node baru dengan nama, pid, url, domain, dan route path
    pub fn create(
        &self,
        name: &str,
        pid: u32,
        url: &str,
        domain: &str,
        route_path: &str,
    ) -> std::result::Result<NodeProcess, NodeError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(NodeError::InvalidInput("name cannot be empty".to_string()));
        }

        if !(PID_START..=PID_END).contains(&pid) {
            return Err(NodeError::InvalidInput(
                "pid out of allowed range".to_string(),
            ));
        }

        let url = normalize_url(url)?;
        let domain = normalize_domain(domain)?;
        let route_path = normalize_route_path(route_path)?;
        let hash = generate_hash(name);

        // Acquire write lock untuk update state
        let mut processes_lock = self.processes.write().unwrap();
        let mut new_map = (*processes_lock.clone()).clone();

        if new_map.contains_key(&hash) {
            return Err(NodeError::AlreadyExists);
        }

        let node = NodeProcess {
            hash: hash.clone(),
            name: name.to_string(),
            pid,
            url,
            domain,
            route_path,
            status: NodeStatus::Active,
        };

        // Insert ke map baru
        new_map.insert(hash, node.clone());

        // Estimate bytes untuk tracking
        let estimated_bytes = serde_json::to_string(&node).unwrap_or_default().len();

        // Swap Arc dengan yang baru (Copy-on-Write semantics)
        *processes_lock = std::sync::Arc::new(new_map);
        drop(processes_lock);

        // Track accumulated bytes dan trigger flush jika perlu
        let current_accumulated = self
            .accumulated_bytes
            .fetch_add(estimated_bytes, Ordering::SeqCst)
            + estimated_bytes;

        if current_accumulated >= CONFIG.nodes.flush_threshold_bytes as usize {
            self.flush()?;
        }

        Ok(node)
    }

    /// Hapus node berdasarkan hash
    pub fn delete(&self, hash: &str) -> std::result::Result<(), NodeError> {
        let mut processes_lock = self.processes.write().unwrap();
        let mut new_map = (*processes_lock.clone()).clone();

        if new_map.remove(hash).is_some() {
            *processes_lock = std::sync::Arc::new(new_map);
            drop(processes_lock);

            // Track deletion (estimate 1 KB)
            let current_accumulated =
                self.accumulated_bytes.fetch_add(1024, Ordering::SeqCst) + 1024;
            if current_accumulated >= CONFIG.nodes.flush_threshold_bytes as usize {
                self.flush()?;
            }

            Ok(())
        } else {
            Err(NodeError::NotFound)
        }
    }

    /// List semua node hashes yang registered
    pub fn list(&self) -> Option<Vec<String>> {
        // Clone Arc pointer dengan cepat, lepas read lock immediately
        let snapshot = {
            let processes_lock = self.processes.read().unwrap();
            processes_lock.clone()
        };

        let mut list: Vec<String> = snapshot.keys().cloned().collect();
        list.sort();

        if list.is_empty() { None } else { Some(list) }
    }

    /// Get node berdasarkan hash
    pub fn get(&self, hash: &str) -> Option<NodeProcess> {
        let processes_lock = self.processes.read().unwrap();
        processes_lock.get(hash).cloned()
    }
}

fn normalize_url(url: &str) -> Result<String, NodeError> {
    let url = url.trim().trim_end_matches('/').to_string();

    if url.is_empty() {
        return Err(NodeError::InvalidInput("url cannot be empty".to_string()));
    }

    let parsed = reqwest::Url::parse(&url)
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
