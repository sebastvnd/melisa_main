//! # Melisa Project
//!
//! An open server architecture framework inspired by Pingora, fully written in Rust.
//!
//! - **Version:** 0.1.0-beta
//! - **License:** MIT

mod mcore;
use crate::mcore::errors::e_node::NodeError;
use crate::mcore::melisad::services::node::NodeManager;
use std::time::Duration;

// Di mulai untuk umat manusia
// Juni 2026
// Kita ke ijen kan?
// Kamu masih ingetkan ...f

#[tokio::main]
async fn main() {
    println!("--- [MELISAD DAEMON STARTUP] ---");
    let node_manager = NodeManager::get_instance();

    // 2. Jalankan pengecekan status URL untuk semua node secara async
    println!("Memulai sinkronisasi dan verifikasi node...");
    match node_manager.startup_node_check().await {
        Ok(_) => {
            println!("✓ Semua node berhasil divalidasi dan status terbaru disimpan ke disk.");
        }
        Err(err) => {
            // Menggunakan bunderan log error jika ada masalah I/O atau JSON parsing
            eprintln!("{:?}", NodeError::FailedValidation(format!("{:?}", err)));
        }
    }

    // Looping utama daemon agar tidak langsung exit (contoh jika menggunakan loop)
    loop {
        tokio::time::sleep(Duration::from_secs(3600)).await;
    }
}
