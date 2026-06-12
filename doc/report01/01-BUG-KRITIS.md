# Bug Kritis — Melisa Core

> Bug-bug berikut menyebabkan **perilaku yang salah secara nyata** dan harus segera diperbaiki sebelum deployment apapun.

---

## BUG-01 · Logger: `log_info` dan `log_warn` Menulis ke File yang Salah

**Tingkat Risiko:** 🔴 Kritis  
**File:** `src/mcore/mlog/logger.rs`  
**Baris:** 183, 200

### Deskripsi

Fungsi `log_info` dan `log_warn` sama-sama mendorong log line ke `buffer.error_logs`, bukan ke buffer yang sesuai. Akibatnya:

- Semua pesan `[INFO]` akan muncul di `error.log`, **bukan** di tempat yang seharusnya.
- Semua pesan `[WARN]` juga masuk ke `error.log` (yang bisa dibenarkan secara konvensi, tapi tidak terdokumentasi).
- Tidak ada buffer `info_logs` di struct `LogBuffer`, sehingga pesan INFO tercampur dengan ERROR.

### Kode Bermasalah

```rust
// logger.rs — log_info (baris 183)
pub fn log_info(&self, msg: &str) -> std::io::Result<()> {
    // ...
    if let Ok(mut buffer) = self.buffer.lock() {
        buffer.error_logs.push(log_line); // ❌ SALAH: harusnya info_logs atau dedicated buffer
    }
    // ...
}

// logger.rs — log_warn (baris 200)
pub fn log_warn(&self, msg: &str) -> std::io::Result<()> {
    // ...
    if let Ok(mut buffer) = self.buffer.lock() {
        buffer.error_logs.push(log_line); // ❌ Tercampur dengan error messages
    }
    // ...
}
```

### Dampak

- Operator tidak bisa memisahkan INFO dari ERROR di log file.
- Monitoring yang membaca `error.log` akan kebanjiran pesan INFO (startup, health check, dsb.).
- ELK Stack / Grafana integration akan menginterpretasi info sebagai error.

### Rekomendasi Perbaikan

**Opsi A — Tambah `info_logs` buffer terpisah:**

```rust
struct LogBuffer {
    access_logs: Vec<String>,
    error_logs: Vec<String>,
    debug_logs: Vec<String>,
    info_logs: Vec<String>,  // ✅ Tambahkan field ini
    last_flush: SystemTime,
}
```

Kemudian pada `log_info`:
```rust
buffer.info_logs.push(log_line); // ✅
```

Dan pada `flush()`, tulis `info_logs` ke `error.log` atau file terpisah `info.log`.

**Opsi B — Tulis INFO dan WARN langsung ke `error.log` (gaya Nginx):**

Ini adalah perilaku Nginx yang umum (error.log berisi INFO, WARN, ERROR sesuai level). Jika ini yang diinginkan, ubah komentar dan dokumentasi agar jelas, dan pertahankan logika level filtering yang sudah ada.

---

## BUG-02 · Logger: `log_error` Memiliki Level-Check yang Tidak Pernah Terpenuhi (Dead Logic)

**Tingkat Risiko:** 🟠 Sedang  
**File:** `src/mcore/mlog/logger.rs`  
**Baris:** 139–141

### Deskripsi

Fungsi `log_error` memiliki pengecekan level yang tidak pernah bisa `true`:

```rust
pub fn log_error(&self, msg: &str) -> std::io::Result<()> {
    if !self.config.error_log_enabled {
        return Ok(());
    }

    if self.level > LogLevel::Error {  // ❌ Tidak pernah true! Error adalah level tertinggi.
        return Ok(());
    }
    // ...
}
```

Enum `LogLevel` didefinisikan sebagai `Debug < Info < Warn < Error`. Tidak ada nilai yang lebih besar dari `Error`, sehingga kondisi `self.level > LogLevel::Error` adalah selalu `false`.

### Dampak

Kode ini tidak menyebabkan bug nyata (error tetap terlog), tapi menciptakan kebingungan bagi developer yang membaca kode. Mereka mungkin berasumsi ada mekanisme "silent error mode" yang ternyata tidak ada.

### Rekomendasi Perbaikan

Hapus pengecekan yang tidak berguna tersebut:

```rust
pub fn log_error(&self, msg: &str) -> std::io::Result<()> {
    if !self.config.error_log_enabled {
        return Ok(());
    }
    // Level check dihapus — error selalu dilog jika error_log_enabled = true
    // ...
}
```

---

## BUG-03 · Load Balancer: `LeastConnections` Diimplementasikan Sebagai Sort-by-PID

**Tingkat Risiko:** 🔴 Kritis  
**File:** `src/mcore/melisad/proxy/loadbalancer.rs`  
**Baris:** 53–57

### Deskripsi

Strategi `LeastConnections` dijanjikan dalam konfigurasi (`load_balancer_strategy = "least_connections"`) namun diimplementasikan dengan cara yang sepenuhnya salah:

```rust
LoadBalancingStrategy::LeastConnections => {
    // Simplified: sort by PID
    matching_nodes.sort_by_key(|n| n.pid);  // ❌ Sort by PID bukan Least Connections!
    Some(matching_nodes[0].clone())
}
```

`LeastConnections` seharusnya memilih node dengan jumlah koneksi aktif paling sedikit. Menggunakan PID untuk sorting tidak ada hubungannya sama sekali dengan jumlah koneksi.

### Dampak

- Pengguna yang mengkonfigurasi `least_connections` berpikir sistem menggunakan algoritma tersebut, padahal tidak.
- Load balancing yang tidak seimbang — node dengan PID terkecil akan selalu mendapat seluruh traffic.
- Bug tersembunyi: tidak ada error/warning yang memberi tahu pengguna bahwa strategi ini tidak berfungsi sebagaimana mestinya.

### Rekomendasi Perbaikan

**Opsi A — Implementasi sesungguhnya** (membutuhkan counter per node):

Tambahkan `AtomicUsize` connection counter ke `NodeProcess` atau ke `LoadBalancer`:

```rust
// Tambahkan ke NodeManager atau LoadBalancer
connection_counts: Arc<DashMap<String, AtomicUsize>>,

// Pada LeastConnections:
LoadBalancingStrategy::LeastConnections => {
    matching_nodes.sort_by_key(|n| {
        self.connection_counts
            .get(&n.hash)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    });
    Some(matching_nodes[0].clone())
}
```

**Opsi B — Tandai sebagai belum diimplementasikan** (minimal tidak menyesatkan):

```rust
LoadBalancingStrategy::LeastConnections => {
    // TODO: Implementasi sesungguhnya membutuhkan connection tracking
    // Sementara fallback ke RoundRobin
    let idx = self.round_robin_index.fetch_add(1, Ordering::Relaxed) % matching_nodes.len();
    Some(matching_nodes[idx].clone())
}
```

---

## BUG-04 · Load Balancer: Strategi `Random` Tidak Acak Secara Nyata

**Tingkat Risiko:** 🟠 Sedang  
**File:** `src/mcore/melisad/proxy/loadbalancer.rs`  
**Baris:** 58–65

### Deskripsi

Strategi `Random` menggunakan nanosecond timestamp sebagai sumber "acak":

```rust
LoadBalancingStrategy::Random => {
    let idx = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as usize
        % matching_nodes.len();
    Some(matching_nodes[idx].clone())
}
```

Ini bukan random yang baik karena:
1. Jika `matching_nodes.len()` kecil (misalnya 2), distribusi akan sangat bias — nanosecond selalu genap atau ganjil secara bergantian di beberapa CPU.
2. Di lingkungan high-throughput, banyak request dalam interval nanosecond yang sama akan mendapat index yang sama.
3. Crate `rand` sudah ada di `Cargo.toml` tapi tidak digunakan di sini.

### Rekomendasi Perbaikan

Gunakan crate `rand` yang sudah tersedia:

```rust
use rand::Rng;

LoadBalancingStrategy::Random => {
    let idx = rand::rng().random_range(0..matching_nodes.len());
    Some(matching_nodes[idx].clone())
}
```

---

## BUG-05 · `log_proxy` Bypass Buffer — Perilaku Inkonsisten

**Tingkat Risiko:** 🟠 Sedang  
**File:** `src/mcore/mlog/logger.rs`  
**Baris:** 2109–2115

### Deskripsi

Semua fungsi log (`log_access`, `log_error`, `log_debug`, `log_info`, `log_warn`) menggunakan sistem buffer dan flush berkala. Namun `log_proxy` **menulis langsung ke disk** setiap dipanggil:

```rust
pub fn log_proxy(&self, msg: &str) -> std::io::Result<()> {
    let timestamp = Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string();
    let log_line = format!("[{}] {}", timestamp, msg);
    self.write_to_file(&self.config.proxy_log_path(), &log_line)?; // ❌ Direct write!
    self.proxy_rotator.check_and_rotate()?;
    Ok(())
}
```

### Dampak

- `log_proxy` dipanggil oleh `metrics.rs` setiap `metrics_report_interval_secs` (default 60s). Meskipun jarang, inkonsistensi ini berarti `proxy.log` tidak ter-buffer sementara log lain ter-buffer.
- Di future jika `log_proxy` dipanggil lebih sering, akan menjadi I/O bottleneck.
- Jika `flush()` tidak dipanggil, `proxy.log` tetap ter-write tapi log lain tidak.

### Rekomendasi Perbaikan

Tambahkan `proxy_logs` ke `LogBuffer` dan gunakan buffer yang konsisten:

```rust
struct LogBuffer {
    access_logs: Vec<String>,
    error_logs: Vec<String>,
    debug_logs: Vec<String>,
    proxy_logs: Vec<String>,  // ✅ Tambahkan ini
    last_flush: SystemTime,
}

pub fn log_proxy(&self, msg: &str) -> std::io::Result<()> {
    let timestamp = Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string();
    let log_line = format!("[{}] {}", timestamp, msg);

    if let Ok(mut buffer) = self.buffer.lock() {
        buffer.proxy_logs.push(log_line);  // ✅ Gunakan buffer
    }

    self.check_and_flush()?;
    Ok(())
}
```

---

*Lihat `02-IMPLEMENTASI-BELUM-SELESAI.md` untuk fungsi/fitur yang belum diimplementasikan.*
