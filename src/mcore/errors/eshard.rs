#[derive(Debug, PartialEq, Eq)]
pub enum ShardError {
    Full,
    NotFound,
}

impl std::fmt::Display for ShardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShardError::Full => write!(f, "shard full"),
            ShardError::NotFound => write!(f, "not found"),
        }
    }
}
