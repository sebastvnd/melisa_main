/// Node CRUD operations
use crate::mcore::config::load_config::CONFIG;
use crate::mcore::config::load_config::{PID_END, PID_START};
use crate::mcore::errors::enode::NodeError;
use crate::mcore::melisad::services::node::manager::NodeManager;
use crate::mcore::melisad::services::node::models::{NodeProcess, NodeStatus};
use crate::mcore::melisad::utils::hashing::generate_hash;
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
        let mut new_map = (**processes_lock).clone();

        if new_map.contains_key(&hash) {
            return Err(NodeError::AlreadyExists);
        }

        // TODO APAKAH INI DAPAT MEMBATASI NODE YANG MEMILIKI RUTE
        // YANG SAMA APAKAH 1 NODE HANYA DAPAT MENGKLAIM 1 RUTE ATAU BISA MULTIPLE
        // NODE DENGAN RUTE YANG TUMPAK TINDIHAN

        // --- TAMBAHKAN VALIDASI ANTI-HIJACKING ---
        // Cek apakah kombinasi domain + rute ini sudah diklaim oleh node lain
        let is_route_conflict = new_map.values().any(|existing_node| {
            existing_node.domain == domain
                && (existing_node.route_path == route_path
                    || route_path.starts_with(&existing_node.route_path))
        });

        if is_route_conflict {
            return Err(NodeError::InvalidInput(
                "Route hijacking detected: Domain and route path already claimed by another node"
                    .to_string(),
            ));
        }
        // -----------------------------------------

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
        let mut new_map = (**processes_lock).clone();

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

#[cfg(test)]
mod operations_tests {
    use crate::mcore::config::load_config::{PID_END, PID_START};
    use crate::mcore::errors::enode::NodeError;
    use crate::mcore::melisad::services::node::{NodeManager, NodeStatus};
    use tempfile::TempDir;

    // -----------------------------------------------------------------
    // Helper: buat NodeManager dengan tempfile agar terisolasi
    // -----------------------------------------------------------------
    fn make_manager() -> (NodeManager, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nodes.json");
        std::fs::write(&path, "{}").unwrap();
        let mgr = NodeManager::new(path.to_str().unwrap());
        (mgr, dir) // TempDir harus di-return agar tidak di-drop lebih awal
    }

    // -----------------------------------------------------------------
    // CREATE – sukses
    // -----------------------------------------------------------------

    #[test]
    fn test_create_node_success() {
        let (mgr, _dir) = make_manager();
        let result = mgr.create(
            "web-1",
            100_000,
            "http://localhost:3000",
            "example.com",
            "/api",
        );
        assert!(result.is_ok(), "create harus sukses: {:?}", result);
        let node = result.unwrap();
        assert_eq!(node.name, "web-1");
        assert_eq!(node.pid, 100_000);
        assert_eq!(node.url, "http://localhost:3000");
        assert_eq!(node.domain, "example.com");
        assert_eq!(node.route_path, "/api");
        assert_eq!(node.status, NodeStatus::Active);
    }

    #[test]
    fn test_create_node_hash_is_64_chars() {
        let (mgr, _dir) = make_manager();
        let node = mgr
            .create(
                "hash-test",
                100_001,
                "http://localhost:3001",
                "example.com",
                "/",
            )
            .unwrap();
        assert_eq!(node.hash.len(), 64, "Hash node harus 64 karakter");
    }

    /// Node dengan PID boundary minimum harus sukses
    #[test]
    fn test_create_node_pid_boundary_min() {
        let (mgr, _dir) = make_manager();
        let r = mgr.create("pid-min", PID_START, "http://localhost:3010", "x.com", "/");
        assert!(r.is_ok(), "PID_START harus valid: {:?}", r);
    }

    /// Node dengan PID boundary maximum harus sukses
    #[test]
    fn test_create_node_pid_boundary_max() {
        let (mgr, _dir) = make_manager();
        let r = mgr.create("pid-max", PID_END, "http://localhost:3011", "x.com", "/max");
        assert!(r.is_ok(), "PID_END harus valid: {:?}", r);
    }

    /// Route path kosong harus dinormalisasi ke "/"
    #[test]
    fn test_create_node_empty_route_path_normalized_to_slash() {
        let (mgr, _dir) = make_manager();
        let node = mgr
            .create(
                "route-empty",
                100_002,
                "http://localhost:3002",
                "example.com",
                "",
            )
            .unwrap();
        assert_eq!(node.route_path, "/");
    }

    /// Route path dengan trailing slash harus dihilangkan
    #[test]
    fn test_create_node_trailing_slash_normalized() {
        let (mgr, _dir) = make_manager();
        let node = mgr
            .create(
                "trail",
                100_003,
                "http://localhost:3003",
                "example.com",
                "/api/",
            )
            .unwrap();
        assert_eq!(node.route_path, "/api");
    }

    /// Domain harus dinormalisasi ke lowercase
    #[test]
    fn test_create_node_domain_normalized_to_lowercase() {
        let (mgr, _dir) = make_manager();
        let node = mgr
            .create(
                "case-test",
                100_004,
                "http://localhost:3004",
                "EXAMPLE.COM",
                "/",
            )
            .unwrap();
        assert_eq!(node.domain, "example.com");
    }

    /// URL dengan trailing slash harus dihilangkan
    #[test]
    fn test_create_node_url_trailing_slash_removed() {
        let (mgr, _dir) = make_manager();
        let node = mgr
            .create(
                "url-slash",
                100_005,
                "http://localhost:3005/",
                "example.com",
                "/",
            )
            .unwrap();
        assert_eq!(node.url, "http://localhost:3005");
    }

    // -----------------------------------------------------------------
    // CREATE – error cases
    // -----------------------------------------------------------------

    #[test]
    fn test_create_node_empty_name_rejected() {
        let (mgr, _dir) = make_manager();
        let r = mgr.create("", 100_006, "http://localhost:3006", "x.com", "/");
        assert!(
            matches!(r, Err(NodeError::InvalidInput(_))),
            "Nama kosong harus InvalidInput"
        );
    }

    #[test]
    fn test_create_node_whitespace_only_name_rejected() {
        let (mgr, _dir) = make_manager();
        let r = mgr.create("   ", 100_007, "http://localhost:3007", "x.com", "/");
        assert!(matches!(r, Err(NodeError::InvalidInput(_))));
    }

    #[test]
    fn test_create_node_pid_below_range_rejected() {
        let (mgr, _dir) = make_manager();
        let r = mgr.create(
            "low-pid",
            PID_START - 1,
            "http://localhost:3008",
            "x.com",
            "/",
        );
        assert!(
            matches!(r, Err(NodeError::InvalidInput(_))),
            "PID di bawah range harus ditolak"
        );
    }

    #[test]
    fn test_create_node_pid_above_range_rejected() {
        let (mgr, _dir) = make_manager();
        let r = mgr.create(
            "high-pid",
            PID_END + 1,
            "http://localhost:3009",
            "x.com",
            "/",
        );
        assert!(
            matches!(r, Err(NodeError::InvalidInput(_))),
            "PID di atas range harus ditolak"
        );
    }

    #[test]
    fn test_create_node_pid_zero_rejected() {
        let (mgr, _dir) = make_manager();
        let r = mgr.create("zero-pid", 0, "http://localhost:3010", "x.com", "/");
        assert!(matches!(r, Err(NodeError::InvalidInput(_))));
    }

    #[test]
    fn test_create_node_invalid_url_rejected() {
        let (mgr, _dir) = make_manager();
        let r = mgr.create("bad-url", 100_010, "not-a-url", "x.com", "/");
        assert!(
            matches!(r, Err(NodeError::InvalidInput(_))),
            "URL tidak valid harus ditolak"
        );
    }

    #[test]
    fn test_create_node_ftp_url_rejected() {
        let (mgr, _dir) = make_manager();
        let r = mgr.create("ftp-node", 100_011, "ftp://files.example.com", "x.com", "/");
        assert!(
            matches!(r, Err(NodeError::InvalidInput(_))),
            "Skema ftp:// harus ditolak"
        );
    }

    #[test]
    fn test_create_node_empty_url_rejected() {
        let (mgr, _dir) = make_manager();
        let r = mgr.create("no-url", 100_012, "", "x.com", "/");
        assert!(matches!(r, Err(NodeError::InvalidInput(_))));
    }

    #[test]
    fn test_create_node_empty_domain_rejected() {
        let (mgr, _dir) = make_manager();
        let r = mgr.create("no-domain", 100_013, "http://localhost:3013", "", "/");
        assert!(matches!(r, Err(NodeError::InvalidInput(_))));
    }

    #[test]
    fn test_create_node_domain_with_path_rejected() {
        let (mgr, _dir) = make_manager();
        let r = mgr.create(
            "bad-domain",
            100_014,
            "http://localhost:3014",
            "example.com/path",
            "/",
        );
        assert!(
            matches!(r, Err(NodeError::InvalidInput(_))),
            "Domain dengan path harus ditolak"
        );
    }

    #[test]
    fn test_create_node_route_without_leading_slash_rejected() {
        let (mgr, _dir) = make_manager();
        let r = mgr.create("no-slash", 100_015, "http://localhost:3015", "x.com", "api");
        assert!(
            matches!(r, Err(NodeError::InvalidInput(_))),
            "Route tanpa '/' harus ditolak"
        );
    }

    // -----------------------------------------------------------------
    // CREATE – duplikat
    // -----------------------------------------------------------------

    #[test]
    fn test_create_duplicate_name_rejected() {
        let (mgr, _dir) = make_manager();
        mgr.create("dup-node", 100_020, "http://localhost:3020", "x.com", "/")
            .unwrap();
        let r = mgr.create("dup-node", 100_021, "http://localhost:3021", "y.com", "/v2");
        assert!(
            matches!(r, Err(NodeError::AlreadyExists)),
            "Node dengan nama yang sama harus AlreadyExists"
        );
    }

    // -----------------------------------------------------------------
    // DELETE
    // -----------------------------------------------------------------

    #[test]
    fn test_delete_existing_node_success() {
        let (mgr, _dir) = make_manager();
        let node = mgr
            .create(
                "to-delete",
                100_030,
                "http://localhost:3030",
                "x.com",
                "/del",
            )
            .unwrap();
        let r = mgr.delete(&node.hash);
        assert!(r.is_ok(), "Delete node yang ada harus sukses: {:?}", r);
    }

    #[test]
    fn test_delete_removes_node_from_list() {
        let (mgr, _dir) = make_manager();
        let node = mgr
            .create("del-check", 100_031, "http://localhost:3031", "x.com", "/")
            .unwrap();
        mgr.delete(&node.hash).unwrap();
        assert!(
            mgr.get(&node.hash).is_none(),
            "Node harus tidak ada setelah dihapus"
        );
    }

    #[test]
    fn test_delete_nonexistent_node_returns_not_found() {
        let (mgr, _dir) = make_manager();
        let fake_hash = "a".repeat(64);
        let r = mgr.delete(&fake_hash);
        assert!(
            matches!(r, Err(NodeError::NotFound)),
            "Delete hash tidak ada harus NotFound"
        );
    }

    #[test]
    fn test_delete_twice_returns_not_found() {
        let (mgr, _dir) = make_manager();
        let node = mgr
            .create("double-del", 100_032, "http://localhost:3032", "x.com", "/")
            .unwrap();
        mgr.delete(&node.hash).unwrap();
        let r = mgr.delete(&node.hash);
        assert!(matches!(r, Err(NodeError::NotFound)));
    }

    // -----------------------------------------------------------------
    // LIST
    // -----------------------------------------------------------------

    #[test]
    fn test_list_empty_returns_none() {
        let (mgr, _dir) = make_manager();
        assert!(mgr.list().is_none(), "List kosong harus None");
    }

    #[test]
    fn test_list_returns_all_hashes() {
        let (mgr, _dir) = make_manager();
        let n1 = mgr
            .create("list-a", 100_040, "http://localhost:3040", "x.com", "/a")
            .unwrap();
        let n2 = mgr
            .create("list-b", 100_041, "http://localhost:3041", "x.com", "/b")
            .unwrap();
        let list = mgr.list().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&n1.hash));
        assert!(list.contains(&n2.hash));
    }

    #[test]
    fn test_list_is_sorted() {
        let (mgr, _dir) = make_manager();
        mgr.create("list-z", 100_042, "http://localhost:3042", "x.com", "/")
            .unwrap();
        mgr.create("list-a2", 100_043, "http://localhost:3043", "x.com", "/a2")
            .unwrap();
        mgr.create("list-m", 100_044, "http://localhost:3044", "x.com", "/m")
            .unwrap();
        let list = mgr.list().unwrap();
        let mut sorted = list.clone();
        sorted.sort();
        assert_eq!(list, sorted, "List harus terurut secara leksikografis");
    }

    #[test]
    fn test_list_after_delete_shrinks() {
        let (mgr, _dir) = make_manager();
        let n1 = mgr
            .create("shrink-a", 100_045, "http://localhost:3045", "x.com", "/")
            .unwrap();
        mgr.create("shrink-b", 100_046, "http://localhost:3046", "y.com", "/")
            .unwrap();
        mgr.delete(&n1.hash).unwrap();
        let list = mgr.list().unwrap();
        assert_eq!(list.len(), 1);
        assert!(!list.contains(&n1.hash));
    }

    // -----------------------------------------------------------------
    // GET
    // -----------------------------------------------------------------

    #[test]
    fn test_get_existing_node() {
        let (mgr, _dir) = make_manager();
        let node = mgr
            .create(
                "get-me",
                100_050,
                "http://localhost:3050",
                "get.com",
                "/get",
            )
            .unwrap();
        let found = mgr.get(&node.hash);
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.name, "get-me");
        assert_eq!(found.pid, 100_050);
    }

    #[test]
    fn test_get_nonexistent_returns_none() {
        let (mgr, _dir) = make_manager();
        assert!(mgr.get(&"b".repeat(64)).is_none());
    }

    #[test]
    fn test_get_after_delete_returns_none() {
        let (mgr, _dir) = make_manager();
        let node = mgr
            .create("del-get", 100_051, "http://localhost:3051", "x.com", "/")
            .unwrap();
        mgr.delete(&node.hash).unwrap();
        assert!(mgr.get(&node.hash).is_none());
    }

    // -----------------------------------------------------------------
    // HTTPS URL
    // -----------------------------------------------------------------

    #[test]
    fn test_create_node_https_url_accepted() {
        let (mgr, _dir) = make_manager();
        let r = mgr.create(
            "https-node",
            100_060,
            "https://secure.example.com",
            "secure.com",
            "/",
        );
        assert!(r.is_ok(), "URL https:// harus diterima: {:?}", r);
    }
}
