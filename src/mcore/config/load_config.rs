use serde::Deserialize;
use std::fs;

use crate::mcore::errors::econfig::ConfigError;
use crate::mcore::mlog::{LOGGER, log_config::LogConfig};
use once_cell::sync::Lazy;

// const variabels
pub const NODE_FILE: &str = "nodes.json"; // berisi daftar node

// batasan pid untuk node yang valid
pub const PID_START: u32 = 100_000;
pub const PID_END: u32 = 999_999;

pub const VERSION: &str = "0.1.0"; // versi melisa

pub const HASH_LENGTH: usize = 64; // panjang hash

// Salin melisa.conf.example ke melisa.conf jika tidak ada
pub const CONFIG_PATH: &str = "melisa.conf"; // file konfigurasi

pub static CONFIG: Lazy<Config> = Lazy::new(|| match Config::from_file(CONFIG_PATH) {
    Ok(cfg) => cfg,
    Err(e) => {
        eprintln!("╔══════════════════════════════════════════════╗");
        eprintln!("║         MELISA CONFIGURATION ERROR           ║");
        eprintln!("╚══════════════════════════════════════════════╝");
        eprintln!();
        eprintln!("  Error: Cannot read config file '{}'", CONFIG_PATH);
        eprintln!("  Melisa version {}", VERSION);
        eprintln!("  > {}", e);
        eprintln!();
        eprintln!("  README:");
        eprintln!("    cp melisa.conf.example melisa.conf");
        eprintln!("    # and edit melisa.conf up to you");
        eprintln!();

        let _ = LOGGER.log_error(&format!(
            "{}",
            ConfigError::FileNotFound("melisa.conf".to_string())
        ));
        std::process::exit(1);
    }
});

#[derive(Debug, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,

    #[serde(default)]
    pub logging: LogConfig,

    #[serde(default)]
    pub nodes: NodesConfig,

    #[serde(default)]
    pub proxy: ProxyConfig,

    #[serde(default)]
    pub management: ManagementConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NodesConfig {
    #[serde(default = "default_storage_file")]
    pub storage_file: String,

    #[serde(default = "default_flush_threshold")]
    pub flush_threshold_bytes: u64,

    #[serde(default = "default_health_check_interval")]
    pub health_check_interval_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProxyConfig {
    #[serde(default = "default_strategy")]
    pub load_balancer_strategy: String,

    #[serde(default = "default_timeout")]
    pub request_timeout_secs: u64,

    #[serde(default = "default_idle_per_host")]
    pub max_idle_per_host: usize,

    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    #[serde(default = "default_retry_backoff")]
    pub retry_backoff_ms: u64,

    #[serde(default = "default_metrics_interval")]
    pub metrics_report_interval_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ManagementConfig {
    #[serde(default = "default_management_port")]
    pub port: u16,

    #[serde(default = "default_management_enabled")]
    pub enabled: bool,
}

impl Default for NodesConfig {
    fn default() -> Self {
        NodesConfig {
            storage_file: default_storage_file(),
            flush_threshold_bytes: default_flush_threshold(),
            health_check_interval_secs: default_health_check_interval(),
        }
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        ProxyConfig {
            load_balancer_strategy: default_strategy(),
            request_timeout_secs: default_timeout(),
            max_idle_per_host: default_idle_per_host(),
            max_retries: default_max_retries(),
            retry_backoff_ms: default_retry_backoff(),
            metrics_report_interval_secs: default_metrics_interval(),
        }
    }
}

impl Default for ManagementConfig {
    fn default() -> Self {
        ManagementConfig {
            port: default_management_port(),
            enabled: default_management_enabled(),
        }
    }
}

fn default_storage_file() -> String {
    NODE_FILE.to_string()
}

fn default_flush_threshold() -> u64 {
    51200 // 50 KB
}

fn default_health_check_interval() -> u64 {
    30
}

fn default_strategy() -> String {
    "round_robin".to_string()
}

fn default_timeout() -> u64 {
    30 // seconds
}

fn default_idle_per_host() -> usize {
    32
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_backoff() -> u64 {
    100 // ms
}

fn default_metrics_interval() -> u64 {
    60 // seconds
}

fn default_management_port() -> u16 {
    8888
}

fn default_management_enabled() -> bool {
    true
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_config_default() {
        let mut file = NamedTempFile::new().unwrap();
        write!(
            file,
            r#"
host = "127.0.0.1"
port = 8080

[logging]
log_dir = "./test-logs"

[nodes]
storage_file = "test-nodes.json"
"#
        )
        .unwrap();

        let config = Config::from_file(file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.logging.log_dir, "./test-logs");
        assert_eq!(config.nodes.storage_file, "test-nodes.json");
    }

    #[test]
    fn test_load_config_with_proxy() {
        let mut file = NamedTempFile::new().unwrap();
        write!(
            file,
            r#"
host = "0.0.0.0"
port = 3000

[proxy]
load_balancer_strategy = "least_connections"
request_timeout_secs = 60
max_retries = 5
"#
        )
        .unwrap();

        let config = Config::from_file(file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.proxy.load_balancer_strategy, "least_connections");
        assert_eq!(config.proxy.request_timeout_secs, 60);
        assert_eq!(config.proxy.max_retries, 5);
    }
}
