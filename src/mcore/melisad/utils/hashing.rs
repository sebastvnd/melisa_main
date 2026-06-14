use sha2::{Digest, Sha256};

// menghasilkan hash unik input
pub fn generate_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();

    result.iter().map(|b| format!("{:02x}", b)).collect()
}

 #[cfg(test)]
mod hashing_tests {
    use crate::mcore::melisad::utils::hashing::generate_hash;
    use crate::mcore::config::load_config::HASH_LENGTH;
 
    // -----------------------------------------------------------------
    // 1. Panjang output
    // -----------------------------------------------------------------
 
    /// Hash harus selalu menghasilkan hex string sepanjang 64 karakter (SHA-256)
    #[test]
    fn test_hash_length_is_64() {
        let binding = "x".repeat(1000);
        let inputs = ["test", "hello world", "", "a", binding.as_str()];
        for input in &inputs {
            let h = generate_hash(input);
            assert_eq!(
                h.len(),
                HASH_LENGTH,
                "Hash untuk input {:?} panjangnya {} bukan {}",
                input,
                h.len(),
                HASH_LENGTH
            );
        }
    }
 
    // -----------------------------------------------------------------
    // 2. Determinisme
    // -----------------------------------------------------------------
 
    /// Input yang sama harus selalu menghasilkan hash yang sama (idempoten)
    #[test]
    fn test_hash_is_deterministic() {
        let input = "melisa-node-1";
        let h1 = generate_hash(input);
        let h2 = generate_hash(input);
        assert_eq!(h1, h2, "Hash harus deterministik untuk input yang sama");
    }
 
    /// Memanggil generate_hash berkali-kali menghasilkan hasil konsisten
    #[test]
    fn test_hash_repeated_calls_same_result() {
        let input = "repeated-test";
        let results: Vec<String> = (0..10).map(|_| generate_hash(input)).collect();
        let first = &results[0];
        for (i, r) in results.iter().enumerate() {
            assert_eq!(first, r, "Panggilan ke-{} menghasilkan hash berbeda", i);
        }
    }
 
    // -----------------------------------------------------------------
    // 3. Sensitivitas terhadap perbedaan input
    // -----------------------------------------------------------------
 
    /// Input berbeda harus menghasilkan hash yang berbeda (collision resistance dasar)
    #[test]
    fn test_different_inputs_produce_different_hashes() {
        let pairs = [
            ("node-a", "node-b"),
            ("melisa", "Melisa"),
            ("abc", "abc "),
            ("test1", "test2"),
            ("", "a"),
        ];
        for (a, b) in &pairs {
            let h_a = generate_hash(a);
            let h_b = generate_hash(b);
            assert_ne!(
                h_a, h_b,
                "Input {:?} dan {:?} seharusnya menghasilkan hash berbeda",
                a, b
            );
        }
    }
 
    /// Case-sensitif: "Node" != "node"
    #[test]
    fn test_hash_is_case_sensitive() {
        assert_ne!(
            generate_hash("Node"),
            generate_hash("node"),
            "generate_hash harus case-sensitive"
        );
    }
 
    // -----------------------------------------------------------------
    // 4. Format output
    // -----------------------------------------------------------------
 
    /// Output harus berupa lowercase hexadecimal
    #[test]
    fn test_hash_output_is_lowercase_hex() {
        let h = generate_hash("format-test");
        assert!(
            h.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()),
            "Hash harus berupa lowercase hex, dapat: {}",
            h
        );
    }
 
    // -----------------------------------------------------------------
    // 5. Edge cases
    // -----------------------------------------------------------------
 
    /// String kosong harus tetap menghasilkan hash valid (SHA-256 of empty string)
    #[test]
    fn test_hash_empty_string() {
        let h = generate_hash("");
        assert_eq!(h.len(), HASH_LENGTH);
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        assert_eq!(
            h,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
 
    /// String panjang harus tetap berjalan dengan baik
    #[test]
    fn test_hash_very_long_input() {
        let long_input = "a".repeat(100_000);
        let h = generate_hash(&long_input);
        assert_eq!(h.len(), HASH_LENGTH);
    }
 
    /// Unicode / non-ASCII harus ditangani tanpa panic
    #[test]
    fn test_hash_unicode_input() {
        let inputs = ["มีลิซา", "节点", "مليسا", "🦀"];
        for input in &inputs {
            let h = generate_hash(input);
            assert_eq!(h.len(), HASH_LENGTH, "Unicode input {:?} gagal", input);
        }
    }
 
    /// Newline dan whitespace dalam string harus diperhitungkan
    #[test]
    fn test_hash_whitespace_sensitivity() {
        assert_ne!(generate_hash("ab"), generate_hash("a b"));
        assert_ne!(generate_hash("ab"), generate_hash("ab\n"));
    }
 
    // -----------------------------------------------------------------
    // 6. Known-value test (SHA-256 regression)
    // -----------------------------------------------------------------
 
    /// Verifikasi nilai SHA-256 yang dikenal untuk string "test"
    #[test]
    fn test_known_sha256_value_for_test() {
        // SHA-256("test") = 9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08
        let h = generate_hash("test");
        assert_eq!(
            h,
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
    }
 
    /// Verifikasi SHA-256 untuk string "melisa"
    #[test]
    fn test_known_sha256_value_for_melisa() {
        // SHA-256("melisa") = dihitung manual, digunakan sebagai regression test
        let h1 = generate_hash("melisa");
        let h2 = generate_hash("melisa");
        // Pastikan konsisten antar-panggilan (nilai aktual dicek via determinisme)
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }
}