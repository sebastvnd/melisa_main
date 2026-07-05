// mcore/melisad/services/node/types.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

use std::fmt;
use std::sync::atomic::{AtomicU8, Ordering};

use crate::mcore::config::load_config::{BASE_SHARD_SIZE, REMAINDER};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStatus {
    Active,
    Stopped,
    Suspicious,
}

impl NodeStatus {
    const ACTIVE: u8 = 0;
    const STOPPED: u8 = 1;
    const SUSPICIOUS: u8 = 2;

    #[inline]
    pub const fn to_code(self) -> u8 {
        match self {
            NodeStatus::Active => Self::ACTIVE,
            NodeStatus::Stopped => Self::STOPPED,
            NodeStatus::Suspicious => Self::SUSPICIOUS,
        }
    }

    #[inline]
    pub const fn from_code(code: u8) -> Self {
        match code {
            Self::ACTIVE => NodeStatus::Active,
            Self::STOPPED => NodeStatus::Stopped,
            Self::SUSPICIOUS => NodeStatus::Suspicious,
            _ => panic!("Invalid node status code"),
        }
    }
}

impl fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeStatus::Active => write!(f, "Active"),
            NodeStatus::Stopped => write!(f, "Stopped"),
            NodeStatus::Suspicious => write!(f, "Suspicious"),
        }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct AtomicNodeStatus(AtomicU8);

impl AtomicNodeStatus {
    #[inline]
    pub fn new(status: NodeStatus) -> Self {
        Self(AtomicU8::new(status.to_code()))
    }

    #[inline]
    pub fn load(&self) -> NodeStatus {
        NodeStatus::from_code(self.0.load(Ordering::Relaxed))
    }

    #[inline]
    pub fn store(&self, status: NodeStatus) {
        self.0.store(status.to_code(), Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IndexSlot {
    pub data_index: u32,
    pub version: u32,
}

impl IndexSlot {
    pub const EMPTY: IndexSlot = IndexSlot {
        data_index: u32::MAX,
        version: 0,
    };
}

#[inline]
pub const fn shard_size(shard_id: usize) -> u32 {
    if shard_id == 0 {
        BASE_SHARD_SIZE + REMAINDER
    } else {
        BASE_SHARD_SIZE
    }
}

#[inline]
pub const fn shard_start_offset(shard_id: usize) -> u32 {
    if shard_id == 0 {
        0
    } else {
        BASE_SHARD_SIZE + REMAINDER + (shard_id as u32 - 1) * BASE_SHARD_SIZE
    }
}

#[inline]
pub const fn split_sparse_idx(sparse_idx: u32) -> (usize, u32) {
    if sparse_idx < BASE_SHARD_SIZE + REMAINDER {
        (0, sparse_idx)
    } else {
        let rest = sparse_idx - (BASE_SHARD_SIZE + REMAINDER);
        let shard_id = 1 + (rest / BASE_SHARD_SIZE) as usize;
        let local_idx = rest % BASE_SHARD_SIZE;
        (shard_id, local_idx)
    }
}

#[inline]
pub const fn join_sparse_idx(shard_id: usize, local_idx: u32) -> u32 {
    shard_start_offset(shard_id) + local_idx
}

// TODO: TULIS CODE TESTNYA -
