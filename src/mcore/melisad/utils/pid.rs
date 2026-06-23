use std::hash::{DefaultHasher, Hash, Hasher};

use crate::mcore::config::load_config::{PID_END, PID_START};

pub fn generate_pid(node_identifier: &str) -> u32 {
    let mut hasher = DefaultHasher::new();
    node_identifier.hash(&mut hasher);
    let hash_value = hasher.finish();

    // Map range PID_START ..=PID_END
    let range_size = (PID_END - PID_START) as u64;
    let pid = PID_START as u64 + (hash_value % range_size);

    pid as u32
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_generate_pid_is_u32() {
        let (name, url) = (String::from("melisa"), String::from("melisa.com"));
        let pid = generate_pid(&format!("{}-{}", name, url));

        let type_data = std::any::type_name_of_val(&pid);

        assert_eq!(type_data, "u32", "harusnya bernilai u32");
    }

    #[test]
    fn test_diverent_pid() {
        let pid1 = generate_pid("node1-http://localhost:3000");
        let pid2 = generate_pid("node2-http://localhost:3001");

        assert!(pid1 >= PID_START && pid1 <= PID_END);
        assert!(pid2 >= PID_START && pid2 <= PID_END);
        assert_ne!(pid1, pid2);
    }
}
