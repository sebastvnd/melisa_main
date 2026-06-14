/// Melisa Logging System
///
/// Robust logging system inspired by Nginx
/// - Access logs (HTTP requests)
/// - Error logs
/// - Debug logs
/// - Automatic log rotation
/// - Configurable log levels
/// - Buffered writing for performance
pub mod log_config;
pub mod logger;
pub mod rotation;

pub use logger::LOGGER;
