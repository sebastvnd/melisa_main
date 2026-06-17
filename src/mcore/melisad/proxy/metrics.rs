use crate::mcore::mlog::LOGGER;
/// Proxy metrics tracking dan reporting
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct ProxyMetrics {
    pub total_requests: AtomicUsize,
    pub total_errors: AtomicUsize,
    pub total_bytes_forwarded: AtomicUsize,
    pub active_connections: AtomicUsize,
}

impl ProxyMetrics {
    pub fn new() -> Self {
        ProxyMetrics {
            total_requests: AtomicUsize::new(0),
            total_errors: AtomicUsize::new(0),
            total_bytes_forwarded: AtomicUsize::new(0),
            active_connections: AtomicUsize::new(0),
        }
    }

    pub fn record_request(&self, bytes_forwarded: usize, is_error: bool) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_forwarded
            .fetch_add(bytes_forwarded, Ordering::Relaxed);
        if is_error {
            self.total_errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn log_metrics(&self) {
        let total = self.total_requests.load(Ordering::Relaxed);
        let errors = self.total_errors.load(Ordering::Relaxed);
        let bytes = self.total_bytes_forwarded.load(Ordering::Relaxed);

        let msg = format!(
            "Metrics - Total Requests: {}, Errors: {}, Bytes Forwarded: {} MB, Active: {}",
            total,
            errors,
            bytes / (1024 * 1024),
            self.active_connections.load(Ordering::Relaxed)
        );

        let _ = LOGGER.log_proxy(&msg);
    }

    pub fn increment_active(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement_active(&self) {
        // Gunakan fetch_update dengan saturating_sub untuk cegah underflow
        self.active_connections
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                Some(v.saturating_sub(1))
            })
            .ok(); // Abaikan error (tidak akan terjadi karena closure selalu Some)
    }
}
