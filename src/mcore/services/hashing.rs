use sha2::{Digest, Sha256};

use crate::mcore::services::config::NODE_FILE;

// menghasilkan hash unik input + random number
pub fn generate_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();

    result.iter().map(|b| format!("{:02x}", b)).collect()
}

// fn ini di gunakan untuk mencari hash yang ada di nodes.json
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

#[cfg(test)]
mod test {
    use super::*;
    #[test]

    // test fungsi generate_hash : harus menghasilkan char acak 64
    fn test_generate_hash() {
        let input = "test";

        let hash = generate_hash(input);
        assert_eq!(hash.len(), 64, "Hash > 64 Char")
    }
}
