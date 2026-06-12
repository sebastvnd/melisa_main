use crate::mcore::errors::enode::NodeError;
use crate::mcore::melisad::probes::liveness_node::check_node_network_with_client;
use crate::mcore::melisad::services::node::NodeManager;
use std::sync::Arc;

impl NodeManager {
    pub async fn startup_node_check(&self) -> std::result::Result<(), NodeError> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .unwrap_or_default();

        let checks = {
            let processes_lock = self.processes.read().unwrap();
            let mut tasks = Vec::new();

            for (hash, node) in processes_lock.iter() {
                let hash_clone = hash.clone();
                let url_clone = node.url.clone();
                let client_clone = http_client.clone();

                let check_future = async move {
                    let status = check_node_network_with_client(&client_clone, &url_clone).await;
                    (hash_clone, status)
                };
                tasks.push(check_future);
            }
            tasks
        }; //lepas Read lock 

        let results = futures::future::join_all(checks).await;
        {
            // 1. Buka write lock seperti biasa
            let mut processes_lock = self.processes.write().unwrap();

            // 2. READ & COPY: Lakukan double dereference (**) untuk menembus Guard dan Arc,
            // lalu kloning seluruh HashMap internalnya ke variabel baru yang mutabel.
            let mut new_map = (**processes_lock).clone();

            // 3. UPDATE: Sekarang kamu bisa pakai .get_mut() dengan aman di map kloningan baru
            for (hash, new_status) in results {
                if let Some(node) = new_map.get_mut(&hash) {
                    node.status = new_status;
                }
            }

            // 4. SWAP: Bungkus kembali map baru dengan Arc dan timpa pointer lama
            *processes_lock = Arc::new(new_map);
        } // Write lock otomatis lepas di sini

        // Simpan ke disk
        self.save_state_to_disk()?;

        Ok(())
    }

    /// Fungsi utilitas untuk menulis state ke JSON file
    fn save_state_to_disk(&self) -> std::result::Result<(), NodeError> {
        self.flush()?;
        Ok(())
    }
}
