# Melisa Core — Laporan Audit Kode Iterasi Kedua

> **Versi Kode:** `melisa_beta` v0.1.0 · Edisi Rust 2024
> **Tanggal Audit:** Juni 2026
> **Penyusun:** Analisis Langsung dari Source Code (AI-Assisted, bukan dari laporan sebelumnya)
> **Referensi Sebelumnya:** `report/00-OVERVIEW.md` s/d `report/07-QUICK-REFERENCE.md`

---

## Catatan Metodologi

Laporan ini **berbeda** dari laporan sebelumnya di folder `report/`. Laporan sebelumnya sebagian besar didasarkan pada analisis statis yang mengidentifikasi potensi bug. Laporan ini dihasilkan dari **pembacaan langsung seluruh 4.813 baris kode aktual** (`repomix-output-ernoba-melisa_beta_git-9.md`) dan membandingkan kondisi kode sekarang dengan temuan laporan lama.

Hasilnya:
- Beberapa bug lama **sudah diperbaiki** (tapi tidak tuntas)
- Ditemukan **4 bug baru** yang tidak ada di laporan sebelumnya
- Beberapa bug lama **statusnya berubah** dari yang dilaporkan

---

## Ringkasan Eksekutif

| Kategori | Jumlah | Tingkat Risiko |
|----------|:------:|:--------------:|
| Bug Baru (tidak ada di laporan lama) | 4 | 🔴 Kritis – 🟠 Sedang |
| Bug Lama: Diperbaiki Sebagian | 3 | 🟠 Masih Berisiko |
| Bug Lama: Sudah Sepenuhnya Diperbaiki | 2 | ✅ |
| Masalah Struktur Folder | 6 | 🟠 Sedang |
| Logika Cacat (Dead Logic / Redundant) | 4 | 🟡 Rendah–Sedang |
| Kode Tidak Terpakai / Inkonsisten | 5 | 🟡 Rendah |
| **Total Temuan** | **24** | — |

---

## Bagian 1 — Bug Baru yang Belum Pernah Dilaporkan

### BUG-NEW-01 · `flush()` Tidak Mendrain Buffer `info_logs`, `warn_logs`, `proxy_logs`

**Tingkat Risiko:** 🔴 Kritis
**File:** `src/mcore/mlog/logger.rs`
**Dampak:** Semua log INFO, WARN, dan Proxy **tidak pernah ditulis ke disk**

#### Deskripsi

Laporan lama (BUG-01) melaporkan bahwa `log_info` dan `log_warn` mendorong log ke buffer yang salah (`error_logs`). Perbaikannya dilakukan: tiga buffer baru ditambahkan ke struct `LogBuffer`:

```rust
struct LogBuffer {
    access_logs: Vec<String>,
    error_logs: Vec<String>,
    debug_logs: Vec<String>,
    info_logs: Vec<String>,   // ← ditambahkan
    warn_logs: Vec<String>,   // ← ditambahkan
    proxy_logs: Vec<String>,  // ← ditambahkan
    last_flush: SystemTime,
}
```

`log_info`, `log_warn`, dan `log_proxy` kini mendorong ke buffer yang benar. **Namun fungsi `flush()` tidak diperbarui** untuk mendrain ketiga buffer baru tersebut:

```rust
// flush() SAAT INI — TIDAK LENGKAP:
pub fn flush(&self) -> std::io::Result<()> {
    if let Ok(mut buffer) = self.buffer.lock() {
        for line in buffer.access_logs.drain(..) { /* ✅ ok */ }
        for line in buffer.error_logs.drain(..) { /* ✅ ok */ }
        for line in buffer.debug_logs.drain(..) { /* ✅ ok */ }
        // ← info_logs TIDAK PERNAH di-drain
        // ← warn_logs TIDAK PERNAH di-drain
        // ← proxy_logs TIDAK PERNAH di-drain
        buffer.last_flush = SystemTime::now();
    }
    Ok(())
}
```

#### Dampak

- Semua pesan `[INFO]` (startup, health check, validasi node) **tidak pernah sampai ke file apapun**.
- Semua pesan `[WARN]` sama.
- Semua log proxy metrics (`log_proxy`) tidak pernah ditulis ke `proxy.log`.
- Buffer terus terisi memori tanpa pernah dikosongkan — potensi memory leak pada proses yang berjalan lama.
- Saat daemon di-restart, semua log yang tersimpan di buffer **hilang permanen**.

#### Rekomendasi Perbaikan

Tambahkan tiga blok drain ke `flush()`. Untuk `info_logs` dan `warn_logs`, ikuti konvensi Nginx (tulis ke `error.log`), dan dokumentasikan keputusan ini:

```rust
pub fn flush(&self) -> std::io::Result<()> {
    if let Ok(mut buffer) = self.buffer.lock() {
        for line in buffer.access_logs.drain(..) {
            self.write_to_file(&self.config.access_log_path(), &line)?;
            self.access_rotator.check_and_rotate()?;
        }
        for line in buffer.error_logs.drain(..) {
            self.write_to_file(&self.config.error_log_path(), &line)?;
            self.error_rotator.check_and_rotate()?;
        }
        for line in buffer.debug_logs.drain(..) {
            self.write_to_file(&self.config.debug_log_path(), &line)?;
            self.debug_rotator.check_and_rotate()?;
        }
        // ✅ TAMBAHKAN: INFO dan WARN ke error.log (konvensi Nginx)
        for line in buffer.info_logs.drain(..) {
            self.write_to_file(&self.config.error_log_path(), &line)?;
            self.error_rotator.check_and_rotate()?;
        }
        for line in buffer.warn_logs.drain(..) {
            self.write_to_file(&self.config.error_log_path(), &line)?;
            self.error_rotator.check_and_rotate()?;
        }
        // ✅ TAMBAHKAN: proxy logs ke proxy.log
        for line in buffer.proxy_logs.drain(..) {
            self.write_to_file(&self.config.proxy_log_path(), &line)?;
            self.proxy_rotator.check_and_rotate()?;
        }
        buffer.last_flush = SystemTime::now();
    }
    Ok(())
}
```

**Definisi Done:** Jalankan daemon, panggil beberapa request, hentikan proses. Verifikasi `error.log` mengandung baris `[INFO]` startup dan `proxy.log` mengandung metrics.

---

### BUG-NEW-02 · `errors/mod.rs` Tidak Mengekspos Module `econfig`

**Tingkat Risiko:** 🔴 Kritis (compile-time, tersembunyi)
**File:** `src/mcore/errors/mod.rs`, `src/mcore/errors/econfig.rs`

#### Deskripsi

File `econfig.rs` ada di disk dan berisi definisi `ConfigError`:

```rust
// econfig.rs
#[derive(Debug)]
pub enum ConfigError {
    InvalidValue(String),
}
```

Namun `mod.rs` di folder yang sama **hanya mengekspos `enode`**:

```rust
// errors/mod.rs — SAAT INI:
pub mod enode;
// ← pub mod econfig; TIDAK ADA
```

Akibatnya, `ConfigError` tidak bisa diakses dari modul manapun di luar folder `errors/`. Kode yang mencoba `use crate::mcore::errors::econfig::ConfigError` akan gagal kompilasi.

#### Dampak

- Seluruh `econfig.rs` adalah dead code yang tidak bisa digunakan.
- Developer yang mencoba menggunakan `ConfigError` untuk validasi konfigurasi akan mendapat compile error yang membingungkan.
- Memperkuat temuan INCOMPLETE-05 dari laporan lama (ConfigError tidak terpakai) — sekarang jelas mengapa: selain tidak dipakai, juga tidak bisa dipakai.

#### Rekomendasi Perbaikan

```rust
// src/mcore/errors/mod.rs
pub mod enode;
pub mod econfig;  // ✅ Tambahkan baris ini
```

Kemudian kembangkan `ConfigError` menjadi tipe yang berguna:

```rust
// src/mcore/errors/econfig.rs
#[derive(Debug)]
pub enum ConfigError {
    InvalidValue { field: String, reason: String },
    MissingField(String),
    FileNotFound(String),
    ParseError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidValue { field, reason } =>
                write!(f, "Config '{}' tidak valid: {}", field, reason),
            ConfigError::MissingField(field) =>
                write!(f, "Field config '{}' wajib diisi", field),
            ConfigError::FileNotFound(path) =>
                write!(f, "File config tidak ditemukan: {}", path),
            ConfigError::ParseError(msg) =>
                write!(f, "Format config tidak valid: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}
```

---

### BUG-NEW-03 · `melisa.conf` Mendefinisikan Field `proxy_log_enabled` yang Tidak Ada di Struct

**Tingkat Risiko:** 🟠 Sedang
**File:** `melisa.conf` (baris 26), `src/mcore/mlog/config.rs`

#### Deskripsi

File konfigurasi mendefinisikan field berikut di seksi `[logging]`:

```toml
# melisa.conf
[logging]
access_log_enabled = true
error_log_enabled = true
proxy_log_enabled = true    # ← field ini
debug_log_enabled = false
```

Namun struct `LogConfig` di `config.rs` tidak memiliki field `proxy_log_enabled`:

```rust
pub struct LogConfig {
    pub access_log_enabled: bool,
    pub error_log_enabled: bool,
    pub debug_log_enabled: bool,
    // ← proxy_log_enabled TIDAK ADA
}
```

`serde` dengan konfigurasi default akan **mengabaikan field yang tidak dikenal secara diam-diam**. Operator yang mengatur `proxy_log_enabled = false` untuk menonaktifkan proxy log tidak akan mendapat efek apapun — proxy log tetap aktif tanpa peringatan.

#### Dampak

- Konfigurasi yang didokumentasikan tidak berfungsi.
- Tidak ada cara untuk menonaktifkan proxy log via konfigurasi.
- Menambah kebingungan: "kenapa proxy log masih muncul padahal saya set false?"

#### Rekomendasi Perbaikan

```rust
// src/mcore/mlog/config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_true")]
    pub access_log_enabled: bool,

    #[serde(default = "default_true")]
    pub error_log_enabled: bool,

    #[serde(default)]
    pub debug_log_enabled: bool,

    // ✅ TAMBAHKAN field ini:
    #[serde(default = "default_true")]
    pub proxy_log_enabled: bool,

    // ...
}

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig {
            // ...
            proxy_log_enabled: true,  // ✅
        }
    }
}
```

Kemudian gunakan field ini di `log_proxy`:

```rust
pub fn log_proxy(&self, msg: &str) -> std::io::Result<()> {
    if !self.config.proxy_log_enabled {  // ✅ Hormati config
        return Ok(());
    }
    // ...
}
```

---

### BUG-NEW-04 · Dependency `portpicker` Tidak Pernah Digunakan

**Tingkat Risiko:** 🟡 Rendah
**File:** `Cargo.toml` (baris 9)

#### Deskripsi

```toml
[dependencies]
portpicker = "0.1.1"  # ← tidak ada satu pun `use portpicker` di seluruh codebase
```

Pencarian di seluruh source menunjukkan tidak ada file yang mengimport atau menggunakan crate ini. Kemungkinan sisa dari eksperimen awal yang lupa dihapus.

#### Dampak

- Menambah compile time.
- Menambah binary size.
- Membingungkan developer baru yang mencari konteks penggunaan `portpicker`.

#### Rekomendasi

```toml
# Hapus dari Cargo.toml:
# portpicker = "0.1.1"
```

---

## Bagian 2 — Status Update Bug dari Laporan Lama

### BUG-01 (Logger Buffer) — DIPERBAIKI SEBAGIAN ⚠️

**Status:** Setengah selesai. Routing buffer sudah benar, tapi `flush()` belum diperbarui.

Lihat **BUG-NEW-01** di atas untuk detail dan perbaikan lengkap.

---

### BUG-03 (LeastConnections = Sort by PID) — DIKOMENTARI, BELUM SELESAI ⚠️

**Status:** Strategy `LeastConnections` kini di-comment out sepenuhnya, bukan lagi sort-by-PID. Ini lebih baik (tidak menyesatkan), namun ada masalah baru:

```rust
// Komentar di loadbalancer.rs yang akan GAGAL KOMPILASI jika di-uncomment:
// let selected_node = matching_nodes
//     .iter()
//     .min_by_key(|n| n.active_connections)  // ← field ini TIDAK ADA di NodeProcess!
//     .cloned();
```

Struct `NodeProcess` di `models.rs` tidak memiliki field `active_connections`. Jika tim mencoba uncomment implementasi yang sudah ada, akan langsung compile error.

```rust
// models.rs — NodeProcess struct SAAT INI:
pub struct NodeProcess {
    pub hash: String,
    pub name: String,
    pub pid: u32,
    pub url: String,
    pub domain: String,
    pub route_path: String,
    pub status: NodeStatus,
    // ← active_connections TIDAK ADA
}
```

**Rekomendasi:** Sebelum uncomment LeastConnections, tambahkan dulu field dan tracking-nya:

```rust
// Opsi A: tambahkan ke NodeProcess (perlu update serialization)
pub struct NodeProcess {
    // ... field lain ...
    #[serde(default)]
    pub active_connections: u64,
}

// Opsi B: tracking terpisah di LoadBalancer (tidak mengubah model)
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct LoadBalancer {
    strategy: LoadBalancingStrategy,
    round_robin_index: Arc<AtomicUsize>,
    connection_counts: Arc<Mutex<HashMap<String, u64>>>,  // ✅ hash → count
}
```

---

### BUG-04 (Random = Nanosecond) — SUDAH DIPERBAIKI ✅

`Random` strategy kini menggunakan `rand::rng()` dengan `choose_mut()`:

```rust
LoadBalancingStrategy::Random => {
    let mut rng = rand::rng();
    matching_nodes.choose_mut(&mut rng).cloned()  // ✅
}
```

---

### BUG-05 (log_proxy Bypass Buffer) — DIPERBAIKI SEBAGIAN ⚠️

`log_proxy` sudah menggunakan buffer. Tapi buffer `proxy_logs` tidak pernah di-drain di `flush()`. Lihat **BUG-NEW-01**.

---

### ARCH-01 (CONFIG Panic) — BELUM DIPERBAIKI ❌

Masih menggunakan `.unwrap()` pada static:

```rust
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    Config::from_file(CONFIG_PATH).unwrap()  // ❌ masih ada
});
```

**Rekomendasi** (dari laporan lama, belum dikerjakan):

```rust
// src/main.rs — inisialisasi eksplisit dengan error yang informatif:
async fn main() {
    let config = Config::from_file(CONFIG_PATH).unwrap_or_else(|e| {
        eprintln!("❌ Melisa gagal start: konfigurasi tidak dapat dimuat.");
        eprintln!("   File yang dicari : {}", CONFIG_PATH);
        eprintln!("   Penyebab         : {}", e);
        eprintln!("   Solusi           : pastikan melisa.conf ada dan formatnya valid TOML.");
        eprintln!("   Referensi        : lihat melisa.conf.example");
        std::process::exit(1);
    });
    // Gunakan OnceLock bukan Lazy untuk CONFIG
}
```

---

### ARCH-02 (Graceful Shutdown) — BELUM DIPERBAIKI ❌

Server masih menggunakan loop tak berujung tanpa signal handling. Restart atau `kill` akan memotong koneksi aktif dan kehilangan log yang belum di-flush.

**Rekomendasi:**

```rust
// src/mcore/melisad/proxy/server.rs
use tokio::signal;

pub async fn run_proxy_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = format!("{}:{}", CONFIG.host, CONFIG.port);
    let listener = TcpListener::bind(&addr).await?;

    let shutdown = async {
        signal::ctrl_c().await.expect("Ctrl+C handler gagal dipasang");
        let _ = LOGGER.log_info("Shutdown signal diterima, memulai graceful shutdown...");
        let _ = LOGGER.flush();
        let _ = NODE_MANAGER.flush();
    };

    tokio::select! {
        result = accept_loop(listener) => result,
        _ = shutdown => {
            println!("Melisa shutdown dengan bersih.");
            Ok(())
        }
    }
}
```

---

## Bagian 3 — Masalah Struktur Folder

### STRUCT-01 · Folder `report/` Ada di Dalam Codebase Production

**Tingkat Risiko:** 🟠 Sedang

```
melisa_beta/
├── report/         ← ❌ laporan analisis AI masuk ke repo
│   ├── 00-OVERVIEW.md
│   ├── 01-BUG-KRITIS.md
│   └── ...
└── src/
```

Laporan analisis AI (termasuk laporan lama dan laporan ini) bukan bagian dari source code yang di-deploy. Keberadaannya di repository utama:
- Menambah ukuran repository.
- Berpotensi membocorkan konteks internal ke publik jika repo di-open source.
- Membingungkan contributor baru tentang batas antara kode dan dokumentasi.

**Rekomendasi:**

```bash
# Pindahkan ke branch docs terpisah atau folder docs/ yang di-.gitignore:
mkdir -p docs/audit
git mv report/ docs/audit/
echo "docs/audit/" >> .gitignore

# Atau buat repository terpisah untuk laporan audit
```

---

### STRUCT-02 · Typo `starup_node.rs` (Harus `startup_node.rs`)

**Tingkat Risiko:** 🟡 Rendah
**Masih belum diperbaiki** sejak laporan lama.

```bash
git mv src/mcore/melisad/probes/starup_node.rs \
       src/mcore/melisad/probes/startup_node.rs
```

```rust
// probes/mod.rs — update:
pub mod startup_node;  // ✅
```

---

### STRUCT-03 · `mnode/` Adalah Crate Yatim — Bukan Workspace Member

**Tingkat Risiko:** 🔴 Kritis (arsitektur)

`Cargo.toml` di root adalah package biasa, **bukan workspace**:

```toml
# Cargo.toml root — SAAT INI (bukan workspace):
[package]
name = "melisa_beta"
version = "0.1.0"
edition = "2024"

[dependencies]
# ...
# ← TIDAK ADA [workspace] members
```

Artinya `mnode/` adalah crate yang berdiri sendiri tanpa koneksi ke build system melisa. `cargo build` dari root tidak akan membangun `mnode`. `cargo test --workspace` tidak akan menguji `mnode`.

**Rekomendasi:**

```toml
# Cargo.toml root — UBAH menjadi workspace:
[workspace]
members = [".", "mnode"]
resolver = "2"

[package]
name = "melisa_beta"
version = "0.1.0"
edition = "2024"
```

```toml
# mnode/Cargo.toml — tambahkan dependencies minimal:
[package]
name = "mnode"
version = "0.1.0"
edition = "2024"

[dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.13", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = "4"
```

---

### STRUCT-04 · `errors/mod.rs` Tidak Mengekspos `econfig`

Sudah dibahas di **BUG-NEW-02**. Masalah ini juga merupakan masalah struktural — sebuah file ada di disk tapi tidak terdaftar di module system Rust.

---

### STRUCT-05 · Tidak Ada `melisa.conf.example` di Repository

Operator baru tidak punya referensi konfigurasi minimal. Jika `melisa.conf` tidak ada, proses langsung panic (lihat ARCH-01).

**Buat file `melisa.conf.example`:**

```toml
# melisa.conf.example — konfigurasi minimal Melisa Proxy
# Salin file ini ke melisa.conf dan sesuaikan nilainya.

host = "127.0.0.1"
port = 8080

[logging]
log_dir = "./logs"
access_log_enabled = true
error_log_enabled = true
debug_log_enabled = false
proxy_log_enabled = true
level = "info"
max_file_size_mb = 100
max_backups = 10
flush_interval_ms = 1000

[nodes]
storage_file = "nodes.json"
flush_threshold_bytes = 51200
health_check_interval_secs = 30

[proxy]
load_balancer_strategy = "round_robin"
request_timeout_secs = 30
max_idle_per_host = 32
max_retries = 3
retry_backoff_ms = 100
metrics_report_interval_secs = 60
```

---

### STRUCT-06 · Tidak Ada Folder `docs/` untuk Dokumentasi Operasional

Repository tidak memiliki dokumentasi untuk operator:
- Bagaimana cara menjalankan Melisa pertama kali?
- Bagaimana cara mendaftarkan node (saat ini hanya bisa via edit `nodes.json` manual)?
- Format `nodes.json`?
- Arti setiap field konfigurasi?

---

## Bagian 4 — Logika Cacat dan Dead Code

### LOGIC-01 · Pengecekan `is_empty()` Redundan Dua Kali di `loadbalancer.rs`

```rust
// select_node():
let mut matching_nodes = node_manager.find_matching_nodes_by_route(domain, path);

if matching_nodes.is_empty() {
    return None;  // ← Guard di sini sudah cukup
}

match self.strategy {
    LoadBalancingStrategy::RoundRobin => {
        if matching_nodes.is_empty() {  // ❌ TIDAK MUNGKIN true di sini
            return None;
        }
        // ...
    }
    LoadBalancingStrategy::Random => {
        if matching_nodes.is_empty() {  // ❌ TIDAK MUNGKIN true di sini
            return None;
        }
        // ...
    }
}
```

Guard di baris 44 sudah memastikan `matching_nodes` tidak kosong saat memasuki blok `match`. Pengecekan di dalam setiap arm adalah dead code.

**Perbaikan:**

```rust
pub fn select_node(&self, domain: &str, path: &str, nm: &NodeManager) -> Option<NodeProcess> {
    let mut nodes = nm.find_matching_nodes_by_route(domain, path);
    if nodes.is_empty() { return None; }  // satu-satunya guard yang diperlukan

    match self.strategy {
        LoadBalancingStrategy::RoundRobin => {
            // langsung gunakan nodes tanpa cek ulang
            let idx = self.round_robin_index.fetch_add(1, Ordering::Relaxed) % nodes.len();
            Some(nodes[idx].clone())
        }
        LoadBalancingStrategy::Random => {
            nodes.choose_mut(&mut rand::rng()).cloned()
        }
    }
}
```

---

### LOGIC-02 · `startup_node_check` Digunakan untuk Dua Tujuan Semantis Berbeda

Belum diperbaiki dari laporan lama (ARCH-04).

```rust
// main.rs:
NODE_MANAGER.startup_node_check().await?;  // ← semantik: "cek saat startup"

// health monitor:
if let Err(err) = NODE_MANAGER.startup_node_check().await {  // ← semantik: "cek berkala"
```

`startup_node_check` dipanggil untuk dua tujuan yang berbeda. Nama yang semantis tidak sesuai membuat kode sulit dipahami.

**Rekomendasi:**

```rust
impl NodeManager {
    /// Dijalankan sekali saat daemon startup.
    /// Melakukan validasi menyeluruh dan flush ke disk.
    pub async fn startup_validation(&self) -> Result<(), NodeError> {
        self.run_health_checks().await?;
        self.flush()?;
        Ok(())
    }

    /// Dijalankan secara berkala oleh health monitor.
    pub async fn periodic_health_check(&self) -> Result<(), NodeError> {
        self.run_health_checks().await?;
        Ok(())
    }

    /// Implementasi bersama — cek semua node dan update status
    async fn run_health_checks(&self) -> Result<(), NodeError> {
        // logika yang sekarang ada di startup_node_check
    }
}
```

---

### LOGIC-03 · Double Clone yang Membingungkan di `operations.rs`

Belum diperbaiki dari laporan lama (QA-01).

```rust
// operations.rs baris 38 dan 80:
let mut new_map = (*processes_lock.clone()).clone();  // ❌ Membingungkan
```

Cara baca yang lebih jelas:

```rust
// Clone pertama: Arc::clone() — murah, hanya increment ref count
// Deref *: akses HashMap di dalam Arc
// Clone kedua: HashMap::clone() — deep copy, O(n)
let mut new_map = (**processes_lock).clone();  // ✅ Lebih ekspresif
```

---

### LOGIC-04 · `ApiResponse<T>` Tanpa `#[derive(Serialize)]` — Tidak Bisa Dijadikan JSON

Belum diperbaiki dari laporan lama (INCOMPLETE-04).

```rust
// adapter/json.rs — SAAT INI:
pub struct ApiResponse<T> {   // ❌ Tidak ada derive Serialize/Deserialize
    pub request_id: String,
    pub success: bool,
    pub code: u16,
    pub message: String,
    pub data: Option<T>,
}
```

Struct ini tidak bisa digunakan untuk mengembalikan JSON response. Karena REST API (MISSING-01) belum diimplementasikan, ini belum crash — tapi ketika REST API dibuat, ini akan jadi compile error pertama yang dihadapi.

```rust
// Perbaikan:
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiResponse<T: Serialize> {
    pub request_id: String,
    pub success: bool,
    pub code: u16,
    pub message: String,
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(request_id: String, data: T) -> Self {
        Self { request_id, success: true, code: 200, message: "OK".into(), data: Some(data) }
    }

    pub fn error(request_id: String, code: u16, message: String) -> Self {
        Self { request_id, success: false, code, message, data: None }
    }
}
```

---

## Bagian 5 — Kode Tidak Terpakai dan Inkonsistensi

### DEAD-01 · `portpicker` — Dependency Tidak Digunakan

Dibahas di BUG-NEW-04. Hapus dari `Cargo.toml`.

### DEAD-02 · `NODE_FILE` Konstanta Bertentangan dengan Config

Belum diperbaiki dari laporan lama (ARCH-03). `mconf.rs` mendefinisikan:

```rust
pub const NODE_FILE: &str = "nodes.json";
```

Sementara production code menggunakan `CONFIG.nodes.storage_file`. Test di `adapter/json.rs` menggunakan `NODE_FILE` — artinya test tidak mencerminkan kondisi production jika `storage_file` diubah.

**Rekomendasi:** Hapus `NODE_FILE`. Test harus menggunakan `TempDir` langsung.

### DEAD-03 · `access_log_format` Config Tidak Pernah Digunakan

Belum diperbaiki dari laporan lama (QA-06). Field `access_log_format` dikonfigurasi di `melisa.conf` dan di struct `LogConfig`, tapi `log_access()` menggunakan format hardcoded.

### DEAD-04 · `save_state_to_disk` Wrapper Tanpa Nilai Tambah

Belum diperbaiki dari laporan lama (INCOMPLETE-03).

```rust
fn save_state_to_disk(&self) -> Result<(), NodeError> {
    self.flush()?;  // hanya ini — tidak ada logika tambahan
    Ok(())
}
```

Hapus fungsi ini dan ganti pemanggilan di `startup_node_check` dengan `self.flush()` langsung.

### DEAD-05 · Komentar Personal di `src/main.rs`

Belum dihapus dari laporan lama (QA-08).

```rust
// Di mulai untuk umat manusia
// Juni 2026
// Kita ke ijen kan?
// Kamu masih ingetkan ...f
```

Pindahkan ke `CHANGELOG.md` atau hapus dari production code.

---

## Bagian 6 — Matriks Prioritas Perbaikan

| ID | File | Masalah | Usaha | Prioritas |
|----|------|---------|:-----:|:---------:|
| BUG-NEW-01 | `logger.rs` | flush() tidak drain info/warn/proxy | 🟢 Rendah | **P0** |
| BUG-NEW-02 | `errors/mod.rs` | econfig tidak di-export | 🟢 Rendah | **P0** |
| ARCH-01 | `load_config.rs` | CONFIG.unwrap() → panic | 🟡 Sedang | **P0** |
| BUG-NEW-03 | `config.rs` | proxy_log_enabled tidak ada di struct | 🟢 Rendah | **P1** |
| ARCH-02 | `server.rs` | Tidak ada graceful shutdown | 🟡 Sedang | **P1** |
| STRUCT-03 | `Cargo.toml` | mnode bukan workspace member | 🟢 Rendah | **P1** |
| BUG-01 (lama) | `logger.rs` | LeastConnections butuh active_connections | 🟠 Tinggi | **P1** |
| STRUCT-05 | — | Tidak ada melisa.conf.example | 🟢 Rendah | **P1** |
| STRUCT-01 | `report/` | Laporan AI masuk repo production | 🟢 Rendah | **P2** |
| STRUCT-02 | `starup_node.rs` | Typo nama file | 🟢 Rendah | **P2** |
| LOGIC-01 | `loadbalancer.rs` | Redundant is_empty() check | 🟢 Rendah | **P2** |
| LOGIC-02 | `main.rs` | startup_node_check nama semantis salah | 🟢 Rendah | **P2** |
| DEAD-03 | `logger.rs` | access_log_format tidak diimplementasikan | 🟡 Sedang | **P2** |
| LOGIC-04 | `adapter/json.rs` | ApiResponse tanpa Serialize | 🟢 Rendah | **P2** |
| LOGIC-03 | `operations.rs` | Double clone membingungkan | 🟢 Rendah | **P3** |
| DEAD-01 | `Cargo.toml` | portpicker tidak dipakai | 🟢 Rendah | **P3** |
| DEAD-02 | `mconf.rs` | NODE_FILE vs storage_file conflict | 🟢 Rendah | **P3** |
| DEAD-04 | `starup_node.rs` | save_state_to_disk wrapper kosong | 🟢 Rendah | **P3** |
| DEAD-05 | `main.rs` | Komentar personal | 🟢 Rendah | **P3** |

---

## Bagian 7 — Checklist Pre-Release

Sebelum Melisa `v0.1.0` dianggap stabil, pastikan semua item berikut terpenuhi:

### Bug Kritis
- [ ] **BUG-NEW-01:** `flush()` drain semua buffer (info/warn/proxy logs tidak hilang)
- [ ] **BUG-NEW-02:** `errors/mod.rs` mengekspos `econfig`
- [ ] **BUG-NEW-03:** `proxy_log_enabled` ada di `LogConfig` struct
- [ ] **ARCH-01:** `CONFIG` tidak panik — error message yang informatif saat startup
- [ ] **ARCH-02:** Graceful shutdown dengan `tokio::signal`

### Struktur dan Konfigurasi
- [ ] `melisa.conf.example` ada di repository
- [ ] `mnode/` terdaftar sebagai workspace member
- [ ] Typo `starup_node.rs` diperbaiki
- [ ] `portpicker` dihapus dari `Cargo.toml`
- [ ] `report/` dipindahkan ke `docs/` atau repository terpisah

### Test Coverage
- [ ] `proxy/forwarder.rs` punya test nyata (bukan `assert_eq!(1, 1)`)
- [ ] `proxy/handler.rs` punya minimal satu test
- [ ] `api/services.rs` punya test
- [ ] Test untuk routing buffer logger (INFO tidak masuk ke access.log, dll.)

### Operabilitas
- [ ] `GET /_melisa/health` tersedia dan mengembalikan status daemon
- [ ] REST API untuk manajemen node (`POST/GET/DELETE /nodes`)
- [ ] `access_log_format` benar-benar digunakan di `log_access()`
- [ ] `mnode` diimplementasikan minimal (registrasi + health endpoint)

### Code Quality
- [ ] Tidak ada komentar personal di production code (`main.rs` baris 16-19)
- [ ] `startup_node_check` dipisah menjadi `startup_validation` + `periodic_health_check`
- [ ] `save_state_to_disk` wrapper dihapus
- [ ] `NODE_FILE` konstanta dihapus — gunakan `TempDir` di test

---

## Catatan Akhir

Audit ini menemukan bahwa perbaikan BUG-01 dari laporan sebelumnya **dikerjakan setengah jalan**: routing buffer sudah benar, tapi `flush()` tidak diperbarui sehingga tiga buffer baru tidak pernah ditulis ke disk. Ini pola yang perlu diperhatikan — ada kebiasaan memulai perbaikan tanpa menyelesaikannya sampai tuntas (juga terlihat dari `LeastConnections` yang di-comment tapi placeholder implementasinya mengacu field yang tidak ada).

Fondasi Melisa tetap solid: pola `Arc<RwLock>` dengan CoW semantics, async/await Tokio, log rotation, dan struktur modul yang logis menunjukkan pemahaman Rust yang matang. Bug-bug yang ditemukan bukan karena ketidaktahuan arsitektur, melainkan karena perubahan dilakukan terburu-buru tanpa memverifikasi seluruh code path yang terpengaruh.

Satu aturan sederhana yang disarankan untuk sprint ke depan: **setiap perubahan pada struct atau fungsi, selalu verifikasi semua caller-nya juga diperbarui** — terutama untuk fungsi seperti `flush()` yang merupakan critical path dari sistem logging.

---

*Laporan ini dihasilkan dari analisis langsung `repomix-output-ernoba-melisa_beta_git-9.md` pada Juni 2026.*
*Untuk laporan audit sebelumnya, lihat `report/00-OVERVIEW.md` s/d `report/07-QUICK-REFERENCE.md`.*
