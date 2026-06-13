// Node module - explicitly load from node/ subdirectory
// by using inline module declaration instead of `pub mod node;`
pub mod node {
    pub mod manager;
    pub mod models;
    pub mod operations;
    pub mod persistence;

    // Re-export public API
    pub use manager::{NODE_MANAGER, NodeManager};
    pub use models::{NodeProcess, NodeStatus};
}
