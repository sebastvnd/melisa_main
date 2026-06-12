#[derive(Debug)]
pub enum NodeError {
    AlreadyExists,           // Error jika node dengan nama tersebut sudah ada
    IoError(std::io::Error), // Menyimpan error dari file system
    JsonError(serde_json::Error),
    NotFound,
    InvalidInput(String), // Untuk input yang tidak valid, seperti nama kosong
    FailedValidation(String), // gagal memvalidasi node
}

impl std::fmt::Display for NodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeError::AlreadyExists => write!(f, "node already exists"),
            NodeError::IoError(err) => write!(f, "node storage I/O error: {}", err),
            NodeError::JsonError(err) => write!(f, "node storage JSON error: {}", err),
            NodeError::NotFound => write!(f, "node not found"),
            NodeError::InvalidInput(msg) => write!(f, "invalid node input: {}", msg),
            NodeError::FailedValidation(msg) => write!(f, "node validation failed: {}", msg),
        }
    }
}

impl std::error::Error for NodeError {}

impl From<std::io::Error> for NodeError {
    fn from(err: std::io::Error) -> Self {
        NodeError::IoError(err)
    }
}

impl From<serde_json::Error> for NodeError {
    fn from(err: serde_json::Error) -> Self {
        NodeError::JsonError(err)
    }
}
