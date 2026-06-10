// const variabels

// node.json menyimpan daftar proses yang dibuat node_services
pub const NODE_FILE: &str = "nodes.json";

// batasan pid untuk node yang valid
pub const PID_START: u32 = 100_000;
pub const PID_END: u32 = 999_999;

pub const HASH_LENGTH: usize = 64; // panjang hash
