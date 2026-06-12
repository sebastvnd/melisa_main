use crate::mcore::errors::enode::NodeError;
use crate::mcore::melisad::services::node::manager::NodeManager;
/// Node persistence (file I/O) dan state management
use std::fs;
use std::sync::atomic::Ordering;

impl NodeManager {
    /// Flush node state ke JSON file
    pub fn flush(&self) -> std::result::Result<(), NodeError> {
        let snapshot = {
            let processes_lock = self.processes.read().unwrap();
            processes_lock.clone()
        };

        let json_data =
            serde_json::to_string_pretty(snapshot.as_ref()).map_err(NodeError::JsonError)?;

        // Write ke storage path yang dikonfigurasi
        fs::write(&self.storage_path, json_data).map_err(NodeError::IoError)?;

        // Reset accumulated bytes counter
        self.accumulated_bytes.store(0, Ordering::SeqCst);
        Ok(())
    }

    /// Reset state untuk testing (dangerous - jangan gunakan di production!)
    #[cfg(test)]
    pub fn reset_for_test(&self) {
        let mut processes_lock = self.processes.write().unwrap();
        *processes_lock = std::sync::Arc::new(std::collections::HashMap::new());
        self.accumulated_bytes.store(0, Ordering::SeqCst);
    }
}
