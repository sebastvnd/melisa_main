use rand::RngExt;
use sha2::{Digest, Sha256};

use crate::mcore::services::config::NODE_FILE;

// menghasilkan hash unik input + random number
pub fn generate_hash(input: &str) -> String {
    let mut rng = rand::rng();
    let random_number: u64 = rng.random();
    let mut hasher = Sha256::new();
    hasher.update(input.to_owned() + &random_number.to_string());
    let result = hasher.finalize();

    result.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn get_hash() -> Option<Vec<String>> {
    let path = NODE_FILE;
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let json_str = if content.trim().is_empty() {
        "{}"
    } else {
        &content
    };
    let map: std::collections::HashMap<String, serde_json::Value> =
        serde_json::from_str(json_str).unwrap_or_default();
    let mut hash_list: Vec<String> = map.keys().cloned().collect();
    hash_list.sort();

    if hash_list.is_empty() {
        None
    } else {
        Some(hash_list)
    }
}
