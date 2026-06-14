/// Konfigurasi logging untuk Melisa
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Direktori penyimpanan log (default: ./logs)
    #[serde(default = "default_log_dir")]
    pub log_dir: String,

    /// Enable access log (HTTP request logging)
    #[serde(default = "default_true")]
    pub access_log_enabled: bool,

    /// Format access log (nginx-style)
    #[serde(default = "default_access_log_format")]
    pub access_log_format: String,

    /// Enable error log
    #[serde(default = "default_true")]
    pub error_log_enabled: bool,

    /// Enable debug log
    #[serde(default)]
    pub debug_log_enabled: bool,

    // Enable proxy log
    #[serde(default = "default_true")]
    pub proxy_log_enabled: bool,

    /// Max ukuran file log sebelum rotation (dalam MB)
    #[serde(default = "default_max_file_size")]
    pub max_file_size_mb: u64,

    /// Jumlah file log yang di-retained sebelum dihapus
    #[serde(default = "default_max_backups")]
    pub max_backups: usize,

    /// Flush log ke disk interval (dalam milliseconds)
    #[serde(default = "default_flush_interval")]
    pub flush_interval_ms: u64,

    /// Log level: "debug", "info", "warn", "error"
    #[serde(default = "default_level")]
    pub level: String,
}

fn default_log_dir() -> String {
    "./logs".to_string()
}

fn default_true() -> bool {
    true
}

fn default_access_log_format() -> String {
    "$remote_addr - - [$time_local] \"$request\" $status $bytes_sent \"$http_referer\" \"$http_user_agent\" $request_time".to_string()
}

fn default_max_file_size() -> u64 {
    100
}

fn default_max_backups() -> usize {
    10
}

fn default_flush_interval() -> u64 {
    1000
}

fn default_level() -> String {
    "info".to_string()
}

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig {
            log_dir: default_log_dir(),
            access_log_enabled: default_true(),
            access_log_format: default_access_log_format(),
            error_log_enabled: default_true(),
            debug_log_enabled: false,
            proxy_log_enabled: default_true(),
            max_file_size_mb: default_max_file_size(),
            max_backups: default_max_backups(),
            flush_interval_ms: default_flush_interval(),
            level: default_level(),
        }
    }
}

impl LogConfig {
    /// Validate dan create log directories
    pub fn setup(&self) -> std::io::Result<()> {
        let log_path = Path::new(&self.log_dir);
        std::fs::create_dir_all(log_path)?;
        Ok(())
    }

    /// Get access log file path
    pub fn access_log_path(&self) -> PathBuf {
        Path::new(&self.log_dir).join("access.log")
    }

    /// Get error log file path
    pub fn error_log_path(&self) -> PathBuf {
        Path::new(&self.log_dir).join("error.log")
    }

    /// Get debug log file path
    pub fn debug_log_path(&self) -> PathBuf {
        Path::new(&self.log_dir).join("debug.log")
    }

    /// Get proxy-specific log file path
    pub fn proxy_log_path(&self) -> PathBuf {
        Path::new(&self.log_dir).join("proxy.log")
    }
}