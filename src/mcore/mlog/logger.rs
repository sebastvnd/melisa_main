/// Logger utama dengan buffering dan rotation
use crate::mcore::config::load_config::CONFIG;
use crate::mcore::mlog::log_config::LogConfig;
use crate::mcore::mlog::rotation::LogRotator;
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "debug" => LogLevel::Debug,
            "warn" => LogLevel::Warn,
            "error" => LogLevel::Error,
            _ => LogLevel::Info,
        }
    }
}

pub struct Logger {
    config: LogConfig,
    access_rotator: LogRotator,
    error_rotator: LogRotator,
    debug_rotator: LogRotator,
    proxy_rotator: LogRotator,
    buffer: Arc<Mutex<LogBuffer>>,
    level: LogLevel,
}

struct LogBuffer {
    access_logs: Vec<String>,
    error_logs: Vec<String>,
    debug_logs: Vec<String>,
    info_logs: Vec<String>,
    warn_logs: Vec<String>,
    proxy_logs: Vec<String>,
    last_flush: SystemTime,
}

impl Logger {
    pub fn new(config: LogConfig) -> std::io::Result<Self> {
        config.setup()?;

        let access_path = config.access_log_path();
        let error_path = config.error_log_path();
        let debug_path = config.debug_log_path();
        let proxy_path = config.proxy_log_path();

        let access_rotator =
            LogRotator::new(access_path, config.max_file_size_mb, config.max_backups);
        let error_rotator =
            LogRotator::new(error_path, config.max_file_size_mb, config.max_backups);
        let debug_rotator =
            LogRotator::new(debug_path, config.max_file_size_mb, config.max_backups);
        let proxy_rotator =
            LogRotator::new(proxy_path, config.max_file_size_mb, config.max_backups);

        let level = LogLevel::from_str(&config.level);

        Ok(Logger {
            config,
            access_rotator,
            error_rotator,
            debug_rotator,
            proxy_rotator,
            buffer: Arc::new(Mutex::new(LogBuffer {
                access_logs: Vec::new(),
                error_logs: Vec::new(),
                debug_logs: Vec::new(),
                info_logs: Vec::new(),
                warn_logs: Vec::new(),
                proxy_logs: Vec::new(),
                last_flush: SystemTime::now(),
            })),
            level,
        })
    }

    /// Log HTTP request (access log format Nginx-style)
    pub fn log_access(
        &self,
        remote_addr: &str,
        request_method: &str,
        request_uri: &str,
        status_code: u16,
        bytes_sent: usize,
        response_time_ms: u128,
        upstream_node: Option<&str>,
    ) -> std::io::Result<()> {
        if !self.config.access_log_enabled {
            return Ok(());
        }

        let timestamp = Local::now().format("%d/%b/%Y:%H:%M:%S %z").to_string();
        let upstream = upstream_node.unwrap_or("-");

        // Format mirip Nginx
        let log_line = format!(
            "{} - - [{}] \"{} {} HTTP/1.1\" {} {} \"{:.0}ms\" \"{}\"",
            remote_addr,
            timestamp,
            request_method,
            request_uri,
            status_code,
            bytes_sent,
            response_time_ms,
            upstream
        );

        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.access_logs.push(log_line);
        }

        self.check_and_flush()?;
        Ok(())
    }

    /// Log error
    pub fn log_error(&self, msg: &str) -> std::io::Result<()> {
        if !self.config.error_log_enabled {
            return Ok(());
        }

        let timestamp = Local::now().format("%Y/%m/%d %H:%M:%S").to_string();
        let log_line = format!("[{}] [ERROR] {}", timestamp, msg);

        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.error_logs.push(log_line);
        }

        self.check_and_flush()?;
        Ok(())
    }

    /// Log debug message
    pub fn log_debug(&self, msg: &str) -> std::io::Result<()> {
        if !self.config.debug_log_enabled {
            return Ok(());
        }

        if self.level > LogLevel::Debug {
            return Ok(());
        }

        let timestamp = Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string();
        let log_line = format!("[{}] [DEBUG] {}", timestamp, msg);

        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.debug_logs.push(log_line);
        }

        self.check_and_flush()?;
        Ok(())
    }

    /// Log info message
    pub fn log_info(&self, msg: &str) -> std::io::Result<()> {
        if self.level > LogLevel::Info {
            return Ok(());
        }

        let timestamp = Local::now().format("%Y/%m/%d %H:%M:%S").to_string();
        let log_line = format!("[{}] [INFO] {}", timestamp, msg);

        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.info_logs.push(log_line); // Simpan di info log untuk info level
        }

        self.check_and_flush()?;
        Ok(())
    }

    /// Log warning message
    pub fn log_warn(&self, msg: &str) -> std::io::Result<()> {
        if self.level > LogLevel::Warn {
            return Ok(());
        }

        let timestamp = Local::now().format("%Y/%m/%d %H:%M:%S").to_string();
        let log_line = format!("[{}] [WARN] {}", timestamp, msg);

        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.warn_logs.push(log_line);
        }

        self.check_and_flush()?;
        Ok(())
    }

    /// Log proxy-specific events
    pub fn log_proxy(&self, msg: &str) -> std::io::Result<()> {
        if !self.config.proxy_log_enabled {
            // ✅ Hormati config
            return Ok(());
        }
        let timestamp = Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string();
        let log_line = format!("[{}] {}", timestamp, msg);

        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.proxy_logs.push(log_line);
        }

        self.check_and_flush()?;
        Ok(())
    }

    /// Manual flush buffers ke disk
    pub fn flush(&self) -> std::io::Result<()> {
        if let Ok(mut buffer) = self.buffer.lock() {
            for line in buffer.access_logs.drain(..) {
                self.write_to_file(&self.config.access_log_path(), &line)?;
                self.access_rotator.check_and_rotate()?;
            }
            for line in buffer.error_logs.drain(..) {
                self.write_to_file(&self.config.error_log_path(), &line)?;
                self.error_rotator.check_and_rotate()?;
            }
            for line in buffer.debug_logs.drain(..) {
                self.write_to_file(&self.config.debug_log_path(), &line)?;
                self.debug_rotator.check_and_rotate()?;
            }

            for line in buffer.info_logs.drain(..) {
                self.write_to_file(&self.config.error_log_path(), &line)?;
                self.error_rotator.check_and_rotate()?;
            }
            for line in buffer.warn_logs.drain(..) {
                self.write_to_file(&self.config.error_log_path(), &line)?;
                self.error_rotator.check_and_rotate()?;
            }

            for line in buffer.proxy_logs.drain(..) {
                self.write_to_file(&self.config.proxy_log_path(), &line)?;
                self.proxy_rotator.check_and_rotate()?;
            }
            buffer.last_flush = SystemTime::now();
        }
        Ok(())
    }

    /// Check apakah perlu flush berdasarkan waktu
    fn check_and_flush(&self) -> std::io::Result<()> {
        if let Ok(buffer) = self.buffer.lock() {
            let elapsed = buffer
                .last_flush
                .elapsed()
                .unwrap_or(std::time::Duration::from_secs(0));

            if elapsed.as_millis() as u64 >= self.config.flush_interval_ms {
                drop(buffer); // Release lock sebelum flush
                self.flush()?;
            }
        }

        Ok(())
    }

    /// Write line ke file (append mode)
    fn write_to_file(&self, path: &std::path::Path, line: &str) -> std::io::Result<()> {
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;

        writeln!(file, "{}", line)?;
        Ok(())
    }
}

// Global logger instance
use once_cell::sync::Lazy;

pub static LOGGER: Lazy<Logger> = Lazy::new(|| {
    match Logger::new(CONFIG.logging.clone()) {
        Ok(logger) => logger,
        Err(e) => {
            eprintln!("Failed to initialize logger: {}", e);
            // Fallback ke default config jika setup gagal
            panic!("Cannot initialize logger");
        }
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_logger_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = LogConfig::default();
        config.log_dir = temp_dir.path().to_str().unwrap().to_string();

        let logger = Logger::new(config).unwrap();
        assert_eq!(logger.level, LogLevel::Info);
    }

    #[test]
    fn test_access_log() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = LogConfig::default();
        config.log_dir = temp_dir.path().to_str().unwrap().to_string();
        config.access_log_enabled = true;

        let logger = Logger::new(config).unwrap();
        logger
            .log_access(
                "127.0.0.1",
                "GET",
                "/api/test",
                200,
                1024,
                45,
                Some("node-1"),
            )
            .unwrap();
        logger.flush().unwrap();

        let access_log_path = temp_dir.path().join("access.log");
        assert!(access_log_path.exists());

        let content = std::fs::read_to_string(access_log_path).unwrap();
        assert!(content.contains("127.0.0.1"));
        assert!(content.contains("GET"));
        assert!(content.contains("200"));
    }

    #[test]
    fn test_error_log() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = LogConfig::default();
        config.log_dir = temp_dir.path().to_str().unwrap().to_string();
        config.error_log_enabled = true;

        let logger = Logger::new(config).unwrap();
        logger.log_error("Test error message").unwrap();
        logger.flush().unwrap();

        let error_log_path = temp_dir.path().join("error.log");
        assert!(error_log_path.exists());

        let content = std::fs::read_to_string(error_log_path).unwrap();
        assert!(content.contains("ERROR"));
        assert!(content.contains("Test error message"));
    }

    #[test]
    fn test_log_level_filtering() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = LogConfig::default();
        config.log_dir = temp_dir.path().to_str().unwrap().to_string();
        config.level = "warn".to_string();
        config.debug_log_enabled = true;

        let logger = Logger::new(config).unwrap();
        assert_eq!(logger.level, LogLevel::Warn);

        logger.log_debug("This should not appear").unwrap();
        logger.flush().unwrap();

        let debug_log_path = temp_dir.path().join("debug.log");
        if debug_log_path.exists() {
            let content = std::fs::read_to_string(debug_log_path).unwrap();
            assert!(!content.contains("This should not appear"));
        }
    }
}
