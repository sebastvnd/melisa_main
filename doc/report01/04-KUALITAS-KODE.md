# Kualitas Kode — Melisa Core

> Temuan di bagian ini adalah masalah **code quality** — tidak langsung menyebabkan bug, namun menurunkan keterbacaan, konsistensi, dan kemampuan maintain kode jangka panjang.

---

## QA-01 · Double Clone yang Tidak Perlu di `operations.rs`

**File:** `src/mcore/melisad/services/node/operations.rs`  
**Baris:** 42, 80

### Deskripsi

Pola `(*processes_lock.clone()).clone()` digunakan dua kali — pada `create` dan `delete`:

```rust
// operations.rs — create (baris 42)
let mut new_map = (*processes_lock.clone()).clone();

// operations.rs — delete (baris 80)
let mut new_map = (*processes_lock.clone()).clone();
```

Ini adalah ekspresi yang membingungkan. Penjelasannya:
- `processes_lock` bertipe `RwLockWriteGuard<Arc<HashMap<...>>>`
- `processes_lock.clone()` — clone `Arc` (murah, hanya increment ref count)
- `*` — deref Arc untuk mendapat `&HashMap`
- `.clone()` — clone seluruh `HashMap` (mahal, O(n))

Ekspresi ini benar secara fungsional, tapi bisa ditulis lebih jelas.

### Rekomendasi

```rust
// Lebih jelas dan sama efisiennya:
let mut new_map = (**processes_lock).clone();
// Atau:
let current_arc = processes_lock.read();  // ← tapi ini sudah write lock...
// Cara terbaik:
let mut new_map: HashMap<String, NodeProcess> = processes_lock.as_ref().clone();
```

Tambahkan komentar jika pola ini dipertahankan:

```rust
// Clone HashMap dari dalam Arc untuk CoW (Copy-on-Write) semantics
// Arc::clone() hanya increment ref count (murah), HashMap::clone() adalah deep copy
let mut new_map = (**processes_lock).clone();
```

---

## QA-02 · Typo pada Nama File: `starup_node.rs`

**File:** `src/mcore/melisad/probes/starup_node.rs`  
**File:** `src/mcore/melisad/probes/mod.rs` (baris 3)

### Deskripsi

Nama file mengandung typo — `starup` seharusnya `startup`:

```
probes/
├── find_node.rs
├── liveness_node.rs
├── mod.rs
└── starup_node.rs    ← seharusnya startup_node.rs
```

```rust
// mod.rs
pub mod starup_node;  // ← typo ikut masuk
```

### Dampak

- Typo akan masuk ke version control dan sulit dihapus setelah banyak referensi.
- IDE autocomplete akan menyarankan nama yang salah.
- Developer baru akan bingung apakah ini disengaja.

### Rekomendasi

Rename file dan update semua referensi:

```bash
git mv src/mcore/melisad/probes/starup_node.rs \
       src/mcore/melisad/probes/startup_node.rs
```

Update `mod.rs`:
```rust
pub mod startup_node;  // ✅
```

---

## QA-03 · `generate_hash` Hanya Hash Nama — Tidak Unik Secara Umum

**File:** `src/mcore/melisad/services/hashing.rs`

### Deskripsi

Hash node dibuat dari nama saja:

```rust
pub fn generate_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    // ...
}
```

Dipanggil dengan:
```rust
let hash = generate_hash(name);  // hanya dari name
```

Ini artinya:
- Dua node dengan nama "melisa-api" **selalu** mendapat hash yang identik di semua environment.
- Hash bersifat deterministik dari nama — bukan "identifier unik" dalam arti sebenarnya.
- Fungsi `generate_hash` bersifat general (menerima `&str`) tapi selalu dipanggil dengan nama node.

### Dampak

- Tidak ada masalah fungsional saat ini karena sistem memang menggunakan hash sebagai deduplikasi nama.
- Namun jika logika berubah (misalnya ingin dua node dengan nama sama tapi domain berbeda), seluruh sistem ID perlu diubah.
- Komentar di `models.rs` menyebut `hash` sebagai "Unique hash identifier" yang secara teknis tidak sepenuhnya akurat.

### Rekomendasi

Jika deduplikasi per nama memang yang diinginkan, dokumentasikan dengan jelas:

```rust
/// Menghasilkan identifier deterministik dari nama node.
/// Hash yang sama untuk nama yang sama adalah perilaku yang DIINGINKAN
/// karena digunakan untuk mencegah duplikasi nama node.
/// Jangan tambahkan salt/timestamp — ini bukan untuk keamanan, tapi untuk deduplication.
pub fn generate_node_id(name: &str) -> String {
    // ...
}
```

Rename juga fungsinya dari `generate_hash` ke `generate_node_id` agar lebih deskriptif.

---

## QA-04 · `check_node_network` Membuat HTTP Client Baru Setiap Panggilan

**File:** `src/mcore/melisad/probes/liveness_node.rs`  
**Baris:** 6–13

### Deskripsi

```rust
pub async fn check_node_network(url: String) -> NodeStatus {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap_or_default();  // ← Client baru setiap panggilan!

    check_node_network_with_client(&client, &url).await
}
```

`reqwest::Client` yang dibuat dengan `Client::builder()` membawa connection pool, TLS config, dan resources lainnya. Membuat client baru setiap panggilan berarti:
- Tidak ada connection reuse
- Setiap health check membuka koneksi TCP baru dari awal
- Resources yang tidak perlu di-alokasikan dan di-dealokasikan

`check_node_network_with_client` — versi yang lebih baik yang menerima client dari luar — sudah ada dan digunakan dengan benar di `starup_node.rs`.

### Dampak

- Performa health check yang lebih lambat dari seharusnya.
- Overhead yang tidak perlu, terutama jika health check dipanggil sering.

### Rekomendasi

Fungsi `check_node_network` (tanpa client parameter) sebaiknya **tidak diexport** atau dihapus, karena selalu inferior dibanding versi `_with_client`. Atau jika tetap dipertahankan untuk convenience, dokumentasikan batasannya:

```rust
/// ⚠️ Membuat HTTP client baru setiap panggilan — hanya untuk penggunaan satu kali.
/// Untuk health check berulang, gunakan `check_node_network_with_client` dengan client
/// yang di-reuse.
pub async fn check_node_network(url: String) -> NodeStatus {
    // ...
}
```

---

## QA-05 · Error Response Tidak Memiliki Header `Content-Type`

**File:** `src/mcore/melisad/proxy/handler.rs`  
**Baris:** 1059–1065, 1088–1093

### Deskripsi

Response error (502 Bad Gateway, 404 Not Found) mengembalikan body JSON tanpa set header `Content-Type: application/json`:

```rust
// handler.rs — bad gateway response
let error_body = format!(
    "{{\"error\": \"Bad Gateway\", \"request_id\": \"{}\"}}",
    request_id
);
let mut error_response = Response::new(Full::new(Bytes::from(error_body)));
*error_response.status_mut() = StatusCode::BAD_GATEWAY;
// ← Tidak ada: error_response.headers_mut().insert(CONTENT_TYPE, ...)
Ok(error_response)
```

### Dampak

- Client yang melakukan `response.json()` mungkin gagal karena tidak ada `Content-Type: application/json`.
- Browser developer tools akan menampilkan response sebagai teks biasa, bukan JSON.
- Tidak sesuai dengan konvensi HTTP standar.

### Rekomendasi

Buat helper function untuk error response:

```rust
use hyper::header::{CONTENT_TYPE, HeaderValue};

fn json_error_response(status: StatusCode, body: String) -> Response<Full<Bytes>> {
    let mut response = Response::new(Full::new(Bytes::from(body)));
    *response.status_mut() = status;
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    response
}

// Penggunaan:
let error_response = json_error_response(
    StatusCode::BAD_GATEWAY,
    format!(r#"{{"error":"Bad Gateway","request_id":"{}"}}"#, request_id),
);
```

---

## QA-06 · `access_log_format` di Config Tidak Pernah Digunakan

**File:** `src/mcore/mlog/config.rs` (field `access_log_format`), `src/mcore/mlog/logger.rs`

### Deskripsi

`LogConfig` memiliki field `access_log_format` yang sepenuhnya dikonfigurasi:

```rust
// config.rs
pub struct LogConfig {
    #[serde(default = "default_access_log_format")]
    pub access_log_format: String,
    // ...
}
```

Namun `log_access` di `logger.rs` menggunakan format hardcoded yang tidak membaca field ini:

```rust
// logger.rs — log_access (baris 111-121)
let log_line = format!(
    "{} - - [{}] \"{} {} HTTP/1.1\" {} {} \"{:.0}ms\" \"{}\"",
    remote_addr, timestamp, request_method, request_uri,
    status_code, bytes_sent, response_time_ms, upstream
    // ← Format ini HARDCODED, tidak menggunakan self.config.access_log_format
);
```

`LOGGING.md` bahkan mendokumentasikan bahwa format bisa dikustomisasi via config, tapi implementasinya tidak melakukannya.

### Dampak

- Dokumentasi berbohong kepada pengguna.
- Operator yang mencoba mengubah `access_log_format` tidak akan melihat efek apapun.
- Field config ini sia-sia memakan memory.

### Rekomendasi

Implementasikan parser format Nginx-style:

```rust
fn format_access_log(
    format: &str,
    remote_addr: &str,
    request_method: &str,
    request_uri: &str,
    status_code: u16,
    bytes_sent: usize,
    response_time_ms: u128,
    upstream: &str,
    timestamp: &str,
) -> String {
    format
        .replace("$remote_addr", remote_addr)
        .replace("$time_local", timestamp)
        .replace("$request", &format!("{} {} HTTP/1.1", request_method, request_uri))
        .replace("$status", &status_code.to_string())
        .replace("$bytes_sent", &bytes_sent.to_string())
        .replace("$request_time", &format!("{:.0}ms", response_time_ms))
        .replace("$upstream_node", upstream)
}
```

---

## QA-07 · Validasi Hash di `delete_node` Redundan

**File:** `src/mcore/api/services.rs`  
**Baris:** 24

### Deskripsi

```rust
pub fn delete_node(hash: &str) -> Result<(), NodeError> {
    if hash.trim().len() != 64 {
        Err(NodeError::InvalidInput("invalid hash format".to_string()))
    } else {
        NODE_MANAGER.delete(hash.trim())
    }
}
```

Validasi panjang 64 karakter di layer API ini bersifat redundan karena:
1. Semua hash di sistem dihasilkan oleh `generate_hash` yang selalu menghasilkan SHA256 hex = 64 karakter.
2. Jika hash valid (dari sistem), selalu 64 karakter.
3. Jika hash tidak valid (misalnya typo dari user), `NODE_MANAGER.delete()` akan mengembalikan `NodeError::NotFound` yang lebih informatif.

Konstanta `HASH_LENGTH = 64` sudah ada di `mconf.rs` tapi tidak digunakan di sini.

### Rekomendasi

**Opsi A — Gunakan konstanta yang sudah ada:**

```rust
use crate::mcore::melisad::services::mconf::HASH_LENGTH;

if hash.trim().len() != HASH_LENGTH {
    return Err(NodeError::InvalidInput(
        format!("hash must be {} characters", HASH_LENGTH)
    ));
}
```

**Opsi B — Hapus validasi, andalkan `NotFound` dari NodeManager:**

```rust
pub fn delete_node(hash: &str) -> Result<(), NodeError> {
    NODE_MANAGER.delete(hash.trim())
    // NotFound akan dikembalikan jika hash tidak ada
}
```

---

## QA-08 · Komentar Personal di `src/main.rs`

**File:** `src/main.rs`  
**Baris:** 16–19

### Deskripsi

```rust
// Di mulai untuk umat manusia
// Juni 2026
// Kita ke ijen kan?
// Kamu masih ingetkan ...f
```

Komentar personal/informal ada di file entry point production code.

### Dampak

Tidak ada dampak fungsional, namun:
- Tidak profesional untuk kode yang akan di-open source atau diaudit.
- Bisa membingungkan developer baru yang tidak tahu konteksnya.
- Jika Melisa akan dipublikasikan, komentar ini akan terekspos publik.

### Rekomendasi

Pindahkan ke `CHANGELOG.md` atau `HISTORY.md` sebagai catatan rilis, atau hapus dari kode production.

---

*Lihat `05-FITUR-YANG-HILANG.md` untuk fitur penting yang belum ada.*
