// mcore/melisad/services/mod.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

// Node module - explicitly load from node/ subdirectory
// by using inline module declaration instead of `pub mod node;`
pub mod node {
    pub mod allowlist;
    pub mod manager;
    pub mod models;
    pub mod persistence;
    pub mod radix;
    pub mod shard;
    pub mod types;

    pub use manager::NODE_MANAGER;
    pub use models::NodeStatusSerde;
    pub use types::NodeStatus;
}
