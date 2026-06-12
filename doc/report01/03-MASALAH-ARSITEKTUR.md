# Masalah Arsitektur — Melisa Core

> Masalah di bagian ini tidak langsung menyebabkan crash atau hasil salah, tetapi mempengaruhi **maintainability, scalability, dan keandalan** sistem jangka panjang.

---

## ARCH-01 · `CONFIG` dan `LOGGER` Panic Saat Inisialisasi Gagal

**Tingkat Risiko:** 🔴 Tinggi  
**File:** `src/mcore/config/load_config.rs` (baris 8), `src/mcore/mlog/logger.rs` (baris 2174–2183)

### Deskripsi

Kedua static global ini menggunakan `.unwrap()` atau `panic!()` saat inisialisasi gagal:

```rust
// load_config.rs
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    Config::from_file(CONFIG_PATH).unwrap()  // ❌ Panic jika melisa.conf tidak ada
});

// logger.rs
pub static LOGGER: Lazy<Logger> = Lazy::new(|| {
    match Logger::new(CONFIG.logging.clone()) {
        Ok(logger) => logger,
        Err(e) => {
            eprintln!("Failed to initialize logger: {}", e);
            panic!("Cannot initialize logger");  // ❌ Panic tanpa pesan yang membantu
        }
    }
});
```

### Dampak

- Jika `melisa.conf` tidak ditemukan atau formatnya salah (misalnya typo TOML), proses langsung mati dengan pesan panic yang tidak ramah.
- Tidak ada fallback ke nilai default, tidak ada petunjuk lokasi file yang dicari.
- Operator yang baru deploy tidak tahu harus berbuat apa.

### Rekomendasi

Tangani error di `main()` dengan pesan yang informatif:

```rust
// main.rs
async fn main() {
    // Tangani CONFIG error secara eksplisit sebelum masuk ke async runtime
    let config = match Config::from_file(CONFIG_PATH) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Gagal memuat konfigurasi dari '{}'", CONFIG_PATH);
            eprintln!("   Penyebab: {}", e);
            eprintln!("   Pastikan file '{}' ada dan formatnya valid TOML.", CONFIG_PATH);
            eprintln!("   Contoh minimal: lihat melisa.conf.example");
            std::process::exit(1);
        }
    };
}
```

Atau ubah `CONFIG` dari `Lazy<Config>` menjadi diinisialisasi secara eksplisit di `main`, bukan sebagai static global yang panic tersembunyi.

---

## ARCH-02 · Tidak Ada Graceful Shutdown

**Tingkat Risiko:** 🟠 Sedang  
**File:** `src/mcore/melisad/proxy/server.rs`, `src/main.rs`

### Deskripsi

Server proxy berjalan dalam loop tak berujung tanpa menangani sinyal OS:

```rust
// server.rs
loop {
    let (stream, peer_addr) = listener.accept().await?;
    // ...
    tokio::spawn(async move {
        // Handle connection...
    });
}
// ← Tidak ada SIGTERM/SIGINT handling
// ← Tidak ada cara untuk menunggu koneksi aktif selesai
// ← Tidak ada cleanup saat mati
```

### Dampak

- `kill <pid>` atau `Ctrl+C` langsung membunuh proses, memotong semua koneksi aktif di tengah request.
- Buffer log yang belum di-flush akan hilang (kehilangan data log).
- Node state di memory yang belum disimpan ke disk akan hilang.
- Dalam container/Kubernetes, rolling deploy akan menyebabkan request error.

### Rekomendasi

Tambahkan signal handling dengan `tokio::signal`:

```rust
use tokio::signal;

pub async fn run_proxy_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = format!("{}:{}", CONFIG.host, CONFIG.port);
    let listener = TcpListener::bind(&addr).await?;

    let shutdown = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        let _ = LOGGER.log_info("Melisa shutdown signal received");
        let _ = LOGGER.flush();
        let _ = NODE_MANAGER.flush();
    };

    tokio::select! {
        result = accept_loop(listener, /* ... */) => result,
        _ = shutdown => {
            println!("Melisa shutting down gracefully...");
            Ok(())
        }
    }
}
```

---

## ARCH-03 · Konstanta `NODE_FILE` di `mconf.rs` Bertentangan dengan Config

**Tingkat Risiko:** 🟠 Sedang  
**File:** `src/mcore/melisad/services/mconf.rs` (baris 3)

### Deskripsi

```rust
// mconf.rs
pub const NODE_FILE: &str = "nodes.json"; // berisi daftar node
```

Konstanta ini mendefinisikan path hardcoded `"nodes.json"`, namun sistem sebenarnya menggunakan `CONFIG.nodes.storage_file` dari file konfigurasi. Dua sumber kebenaran ini bisa tidak sinkron:

```toml
# melisa.conf — bisa diubah operator
[nodes]
storage_file = "nodes.json"  # operator bisa ubah ini ke "/var/melisa/nodes.db"
```

`NODE_FILE` hanya digunakan di test (`adapter/json.rs`), artinya test selalu berjalan dengan `"nodes.json"` meski config production mungkin berbeda.

### Dampak

- Test yang menggunakan `NODE_FILE` tidak mencerminkan kondisi production yang sebenarnya.
- Jika `storage_file` diubah di config, test masih berjalan di path lama dan bisa memberi hasil positif palsu.
- Membingungkan developer: "mana yang benar, `NODE_FILE` atau `CONFIG.nodes.storage_file`?"

### Rekomendasi

**Hapus `NODE_FILE`** dari `mconf.rs`. Pada test, gunakan `TempDir` secara eksplisit dan inisialisasi `NodeManager` langsung (sudah dilakukan di `manager.rs` tests dengan benar):

```rust
// ✅ Cara yang benar — sudah ada di manager.rs tests
let temp_dir = TempDir::new().unwrap();
let node_file = temp_dir.path().join("test-nodes.json");
let manager = NodeManager::new(node_file.to_str().unwrap());
```

---

## ARCH-04 · `startup_node_check` Dipakai untuk Dua Tujuan Berbeda

**Tingkat Risiko:** 🟡 Rendah–Sedang  
**File:** `src/main.rs` (baris 46, 63), `src/mcore/melisad/probes/starup_node.rs`

### Deskripsi

Fungsi `startup_node_check` digunakan untuk dua hal berbeda:

```rust
// main.rs — saat startup (sekali)
NODE_MANAGER.startup_node_check().await?;

// main.rs — health monitor berkala (terus-menerus)
fn spawn_node_health_monitor(interval: Duration) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            if let Err(err) = NODE_MANAGER.startup_node_check().await { // ← Nama tidak sesuai
```

Nama `startup_node_check` secara semantik berarti "check yang dilakukan saat startup". Menggunakannya untuk periodic health check melanggar prinsip naming yang jelas.

### Dampak

- Developer yang membaca kode akan bingung mengapa "startup check" dipanggil terus-menerus.
- Jika logika startup check perlu dibedakan dari periodic check (misalnya startup perlu validasi lebih ketat), tidak ada pemisahan yang jelas.

### Rekomendasi

Pisahkan menjadi dua fungsi:

```rust
impl NodeManager {
    /// Dijalankan sekali saat daemon startup
    /// Melakukan validasi ketat: cek semua node, update status, simpan ke disk
    pub async fn startup_validation(&self) -> Result<(), NodeError> {
        // Logika yang sudah ada
    }

    /// Dijalankan secara berkala untuk memantau kesehatan node
    /// Bisa lebih ringan dari startup_validation
    pub async fn periodic_health_check(&self) -> Result<(), NodeError> {
        // Bisa menggunakan logika yang sama atau lebih ringan
        self.startup_validation().await
    }
}
```

---

## ARCH-05 · Tidak Ada Rate Limiting atau Request Validation

**Tingkat Risiko:** 🟠 Sedang  
**File:** `src/mcore/melisad/proxy/handler.rs`, `src/mcore/melisad/proxy/server.rs`

### Deskripsi

Proxy menerima semua koneksi masuk tanpa batasan apapun:
- Tidak ada rate limiting per IP
- Tidak ada batas ukuran request body
- Tidak ada validasi Host header (bisa diisi header apapun)
- Tidak ada timeout untuk koneksi yang sangat lambat (slow loris attack)
- Tidak ada batas jumlah koneksi concurrent

### Dampak

- Rentan terhadap DDoS sederhana — satu client bisa membuka ribuan koneksi.
- Memory exhaustion jika request body sangat besar.
- Slow loris attack: klien mengirim header sangat lambat, menahan koneksi.

### Rekomendasi Bertahap

**Tahap 1 — Minimal viable protection** (segera):

```rust
// server.rs — tambahkan batas koneksi concurrent
use std::sync::atomic::{AtomicUsize, Ordering};

static ACTIVE_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);
const MAX_CONNECTIONS: usize = 10_000;

// Pada accept loop:
if ACTIVE_CONNECTIONS.load(Ordering::Relaxed) >= MAX_CONNECTIONS {
    // Tolak koneksi baru
    drop(stream);
    continue;
}
```

**Tahap 2 — Tambahkan ke config** (`melisa.conf`):

```toml
[proxy]
max_concurrent_connections = 10000
max_request_body_bytes = 10485760  # 10 MB
connection_timeout_secs = 30
```

**Tahap 3 — Rate limiting per IP** (jangka menengah):
Gunakan crate `governor` untuk token bucket rate limiting.

---

*Lihat `04-KUALITAS-KODE.md` untuk code smell dan inkonsistensi.*
