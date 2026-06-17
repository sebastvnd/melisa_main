// src/mcore/errors/econfig.rs
#[derive(Debug)]
pub enum ConfigError {
    InvalidValue { field: String, reason: String },
    MissingField(String),
    FileNotFound(String),
    ParseError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidValue { field, reason } => {
                write!(f, "Config '{}' tidak valid: {}", field, reason)
            }
            ConfigError::MissingField(field) => write!(f, "Field config '{}' wajib diisi", field),
            ConfigError::FileNotFound(path) => write!(f, "File config tidak ditemukan: {}", path),
            ConfigError::ParseError(msg) => write!(f, "Format config tidak valid: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}
