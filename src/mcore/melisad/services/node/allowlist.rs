// mcore/melisad/services/node/allowlist.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

use crate::mcore::config::load_config::{AllowedNodeConfig, CONFIG, CONFIG_PATH};
use crate::mcore::errors::enode::NodeError;

pub static NODE_ALLOWLIST: Lazy<NodeAllowlist> =
    Lazy::new(|| NodeAllowlist::new(&CONFIG.nodes.allowed_nodes, CONFIG_PATH));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowedNode {
    pub name: String,
    pub url: Option<String>,
    pub domain: String,
    pub route_path: String,
    pub created_at: u64,
}

pub struct NodeAllowlist {
    entries: Mutex<Vec<AllowedNode>>,
    #[cfg_attr(test, allow(dead_code))]
    config_path: String,
}

impl NodeAllowlist {
    pub fn new(config_entries: &[AllowedNodeConfig], config_path: &str) -> Self {
        let entries = config_entries
            .iter()
            .filter_map(|entry| AllowedNode::from_config(entry).map_err(|_| ()).ok())
            .collect();

        let allowlist = NodeAllowlist {
            entries: Mutex::new(entries),
            config_path: config_path.to_string(),
        };
        allowlist
    }

    pub fn add(
        &self,
        name: &str,
        url: Option<&str>,
        domain: &str,
        route_path: &str,
    ) -> Result<AllowedNode, NodeError> {
        let entry = AllowedNode {
            name: normalize_name(name)?,
            url: match url {
                Some(url) if !url.trim().is_empty() => Some(normalize_url(url)?),
                _ => None,
            },
            domain: normalize_domain(domain)?,
            route_path: normalize_route_path(route_path)?,
            created_at: now_sec(),
        };

        let mut entries = self.entries.lock();
        if entries
            .iter()
            .any(|existing| same_allowed_node(existing, &entry))
        {
            return Err(NodeError::AlreadyExists);
        }

        entries.push(entry.clone());
        self.append_to_config(&entry)?;
        Ok(entry)
    }

    pub fn list(&self) -> Vec<AllowedNode> {
        self.entries.lock().clone()
    }

    pub fn ensure_allowed(
        &self,
        name: &str,
        url: &str,
        domain: &str,
        route_path: &str,
    ) -> Result<(), NodeError> {
        let name = normalize_name(name)?;
        let url = normalize_url(url)?;
        let domain = normalize_domain(domain)?;
        let route_path = normalize_route_path(route_path)?;

        let entries = self.entries.lock();
        let allowed = entries.iter().any(|entry| {
            entry.name == name
                && entry.domain == domain
                && entry.route_path == route_path
                && entry
                    .url
                    .as_ref()
                    .map_or(true, |allowed_url| allowed_url == &url)
        });

        if allowed {
            Ok(())
        } else {
            Err(NodeError::NotAllowed(format!(
                "'{}' is not present in node allowlist",
                name
            )))
        }
    }

    #[cfg(test)]
    pub fn reset_for_test(&self) {
        self.entries.lock().clear();
    }

    #[cfg(not(test))]
    fn append_to_config(&self, entry: &AllowedNode) -> Result<(), NodeError> {
        use std::fs::OpenOptions;
        use std::io::Write;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config_path)?;

        let url_line = entry
            .url
            .as_ref()
            .map(|url| format!("url = \"{}\"\n", escape_toml_string(url)))
            .unwrap_or_default();

        write!(
            file,
            "\n[[nodes.allowed_nodes]]\nname = \"{}\"\n{}domain = \"{}\"\nroute_path = \"{}\"\n",
            escape_toml_string(&entry.name),
            url_line,
            escape_toml_string(&entry.domain),
            escape_toml_string(&entry.route_path),
        )?;
        Ok(())
    }

    #[cfg(test)]
    fn append_to_config(&self, _entry: &AllowedNode) -> Result<(), NodeError> {
        Ok(())
    }
}

impl AllowedNode {
    fn from_config(config: &AllowedNodeConfig) -> Result<Self, NodeError> {
        Ok(AllowedNode {
            name: normalize_name(&config.name)?,
            url: match config.url.as_deref() {
                Some(url) if !url.trim().is_empty() => Some(normalize_url(url)?),
                _ => None,
            },
            domain: normalize_domain(&config.domain)?,
            route_path: normalize_route_path(&config.route_path)?,
            created_at: now_sec(),
        })
    }
}

fn same_allowed_node(left: &AllowedNode, right: &AllowedNode) -> bool {
    left.name == right.name || left.url.is_some() && left.url == right.url
}

fn normalize_name(name: &str) -> Result<String, NodeError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(NodeError::InvalidInput("name cannot be empty".to_string()));
    }
    Ok(name.to_string())
}

fn normalize_url(url: &str) -> Result<String, NodeError> {
    let url = url.trim().trim_end_matches('/').to_string();
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

fn now_sec() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg_attr(test, allow(dead_code))]
fn escape_toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
