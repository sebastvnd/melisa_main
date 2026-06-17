/// Node Configuration
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

pub const SECRET_MANAGEMENT_TOKEN: &str = "DEFAULT_SECRET_NODE_TOKEN";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeConfig {
    pub name: String,           // ← Node name for registration
    pub host: String,
    pub port: u16,
    pub melisa_host: String,
    pub melisa_port: u16,
    pub domain: String,
    pub route_path: String,
    pub static_files_dir: String,
    pub static_files_enabled: bool,
    pub api_enabled: bool,
}

impl NodeConfig {
    /// Load config dari file mnode.conf atau environment
    pub fn from_config_file(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Check if file exists
        if Path::new(config_path).exists() {
            let content = fs::read_to_string(config_path)?;
            let config: TomlConfig = toml::from_str(&content)?;
            Ok(config.to_node_config())
        } else {
            // Fallback ke environment variables
            Ok(Self::from_env())
        }
    }

    pub fn from_env() -> Self {
        let node_name = env::var("MNODE_NAME")
            .unwrap_or_else(|_| {
                let hostname = hostname::get()
                    .ok()
                    .and_then(|h| h.into_string().ok())
                    .unwrap_or_else(|| "mnode".to_string());
                format!("mnode-{}", hostname)
            });

        let port: u16 = env::var("MNODE_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        let melisa_host = env::var("MELISA_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let melisa_port: u16 = env::var("MELISA_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8888);

        let domain = env::var("MNODE_DOMAIN").unwrap_or_else(|_| "mnode.local".to_string());
        let route_path = env::var("MNODE_ROUTE_PATH").unwrap_or_else(|_| "/mnode".to_string());
        let static_files_dir = env::var("STATIC_FILES_DIR").unwrap_or_else(|_| "./public/html".to_string());

        NodeConfig {
            name: node_name,
            host: "127.0.0.1".to_string(),
            port,
            melisa_host,
            melisa_port,
            domain,
            route_path,
            static_files_dir,
            static_files_enabled: true,
            api_enabled: true,
        }
    }

    pub fn melisa_url(&self) -> String {
        format!("http://{}:{}", self.melisa_host, self.melisa_port)
    }

    pub fn node_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

/// Temporary struct untuk deserialize TOML config
#[derive(Debug, Deserialize)]
struct TomlConfig {
    host: String,
    port: u16,
    
    #[serde(default)]
    registration: RegistrationConfig,
    
    #[serde(default)]
    static_files: StaticFilesConfig,
    
    #[serde(default)]
    api: ApiConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct RegistrationConfig {
    melisa_host: String,
    melisa_port: u16,
    node_name: String,
    node_domain: String,
    node_route_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct StaticFilesConfig {
    directory: String,
    enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct ApiConfig {
    enabled: bool,
    base_path: String,
}

impl Default for RegistrationConfig {
    fn default() -> Self {
        RegistrationConfig {
            melisa_host: "127.0.0.1".to_string(),
            melisa_port: 8888,
            node_name: "mnode-service".to_string(),
            node_domain: "mnode.local".to_string(),
            node_route_path: "/mnode".to_string(),
        }
    }
}

impl Default for StaticFilesConfig {
    fn default() -> Self {
        StaticFilesConfig {
            directory: "./public/html".to_string(),
            enabled: true,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        ApiConfig {
            enabled: true,
            base_path: "/api".to_string(),
        }
    }
}

impl TomlConfig {
    fn to_node_config(self) -> NodeConfig {
        NodeConfig {
            name: self.registration.node_name,
            host: self.host,
            port: self.port,
            melisa_host: self.registration.melisa_host,
            melisa_port: self.registration.melisa_port,
            domain: self.registration.node_domain,
            route_path: self.registration.node_route_path,
            static_files_dir: self.static_files.directory,
            static_files_enabled: self.static_files.enabled,
            api_enabled: self.api.enabled,
        }
    }
}

