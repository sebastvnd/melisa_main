mod mcore;
use std::time::Duration;
use mcore::melisad::services::node::{NodeManager, NodeError};

// Di mulai untuk umat manusia
// Juni 2026
// Kita ke ijen kan?
// Kamu masih ingetkan ...f
#[tokio::main]
async fn main() {
    let name = "Example Pfsfdsfrocsffff";
    let pid = 124346665;
    let url = "http://akses";
    
    // Ambil instance manager sekali saja agar kode lebih bersih
    let manager = NodeManager::get_instance();

    // Tangani proses pembuatan secara elegan
    match manager.create(name, pid, url) {
        Ok(_) => println!("Node '{}' berhasil dibuat.", name),
        Err(NodeError::AlreadyExists) => println!("Info: Node '{}' sudah aktif sebelumnya.", name),
        Err(e) => println!("Error tidak terduga saat membuat node: {:?}", e),
    }

    // Ambil daftar hash (Pastikan nama method di NodeManager kamu adalah `list`)
    let hashes = match manager.list() {
        Some(v) => v,
        None => {
            println!("Tidak ada proses node yang ditemukan.");
            return;
        }
    };

    println!("\n--- Daftar Hash Node Aktif ---");
    for (index, hash) in hashes.iter().enumerate() {
        println!("{}. {}", index + 1, hash);
    }

    println!("--- [MELISAD DAEMON STARTUP] ---");

    // 1. Ambil instance tunggal dari NodeManager
    // Saat ini dipanggil, ia otomatis me-load data dari NODE_FILE ke memory (RwLock)
    let node_manager = NodeManager::get_instance();

    // 2. Jalankan pengecekan status URL untuk semua node secara async
    println!("Memulai sinkronisasi dan verifikasi node...");
    match node_manager.startup_node_check().await {
        Ok(_) => {
            println!("✓ Semua node berhasil divalidasi dan status terbaru disimpan ke disk.");
        }
        Err(err) => {
            // Menggunakan bunderan log error jika ada masalah I/O atau JSON parsing
            eprintln!("⚠️ Gagal memvalidasi node saat startup: {:?}", err);
        }
    }

    // 3. Jalankan core logic melisad setelah ini (misal: binding Unix Domain Socket, listen event, dll)
    println!("Daemon melisad siap menerima instruksi.");
    
    // Looping utama daemon agar tidak langsung exit (contoh jika menggunakan loop)
    loop { tokio::time::sleep(Duration::from_secs(3600)).await; }
}
