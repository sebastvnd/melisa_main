use sha2::{Digest, Sha256};

use crate::mcore::melisad::services::mconf::NODE_FILE;

// menghasilkan hash unik input
pub fn generate_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();

    result.iter().map(|b| format!("{:02x}", b)).collect()
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
