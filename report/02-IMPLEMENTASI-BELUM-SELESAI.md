# Implementasi yang Belum Selesai — Melisa Core

> Bagian ini mendokumentasikan fungsi, modul, dan fitur yang ada di codebase namun **belum diimplementasikan secara nyata** — berupa stub, placeholder, atau wrapper kosong yang dapat menyesatkan.

---

## INCOMPLETE-01 · `mnode` Worker — Tidak Ada Implementasi Sama Sekali

**Tingkat Risiko:** 🔴 Kritis  
**File:** `mnode/src/main.rs`

### Deskripsi

`mnode` adalah workspace terpisah yang dimaksudkan sebagai "node worker" namun seluruh isinya hanya satu baris `println!`:

```rust
// mnode/src/main.rs
fn main() {
    println!("mnode is node worker");
}
```

Cargo.toml-nya juga kosong tanpa dependencies:

```toml
[package]
name = "mnode"
version = "0.1.0"
edition = "2024"

[dependencies]
# ← Kosong
```

### Apa yang Seharusnya Ada

Berdasarkan arsitektur Melisa sebagai reverse proxy, `mnode` seharusnya bertugas sebagai:
- Process worker untuk menangani upstream request secara terisolasi
- Registrar diri ke `NODE_MANAGER` via API
- Memiliki health endpoint untuk liveness probe
- Melaporkan status ke daemon melisa

### Dampak

- Seluruh konsep "node worker" tidak berfungsi.
- Tim yang bergantung pada `mnode` akan mendapatkan hasil yang tidak terduga.
- Arsitektur multi-process yang direncanakan tidak bisa berjalan.

### Rekomendasi

Minimal, `mnode` harus:

```rust
use std::net::TcpListener;

fn main() {
    // 1. Parse argumen: port, name, pid, domain, route_path
    // 2. Daftarkan diri ke melisa daemon via HTTP API
    // 3. Buka HTTP listener sederhana
    // 4. Expose /health endpoint
    // 5. Handle sinyal SIGTERM untuk unregister diri
}
```

Buatkan issue tracker khusus untuk ini karena merupakan komponen inti.

---

## INCOMPLETE-02 · Test `forwarder.rs` Adalah Test Palsu

**Tingkat Risiko:** 🟠 Sedang  
**File:** `src/mcore/melisad/proxy/forwarder.rs`  
**Baris:** 941–944

### Deskripsi

Satu-satunya test di file `forwarder.rs` yang mengurus forwarding HTTP — salah satu komponen paling penting dari proxy — adalah:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_forward_request_logic() {
        // Test placeholder - actual integration tests should use mock server
        assert_eq!(1, 1);  // ❌ Test ini tidak menguji apapun
    }
}
```

`assert_eq!(1, 1)` adalah ekspresi yang selalu `true`. Test ini tidak pernah bisa gagal, tidak menguji apapun, dan memberikan **false confidence** bahwa forwarder sudah ditest.

### Dampak

- `cargo test` akan lulus meski ada bug di `forward_request_with_retry`.
- Retry logic, header filtering, backoff duration — tidak ada satupun yang ditest.
- CI pipeline akan hijau meski fungsi inti rusak.

### Rekomendasi Perbaikan

Gunakan `mockito` atau `wiremock` untuk membuat mock HTTP server:

```toml
# Cargo.toml — dev-dependencies
[dev-dependencies]
wiremock = "0.6"
tokio-test = "0.4"
```

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::method;

    #[tokio::test]
    async fn test_forward_request_success() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let result = forward_request_with_retry(
            &client,
            &hyper::Method::GET,
            &mock_server.uri(),
            &HeaderMap::new(),
            Bytes::new(),
            "test-req-001",
            0,  // no retry
            0,
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_retry_on_server_error() {
        let mock_server = MockServer::start().await;
        // Simulasi 2x 500, kemudian 200
        // Test bahwa retry bekerja sesuai `max_retries`
        // ...
    }
}
```

---

## INCOMPLETE-03 · `save_state_to_disk` Adalah Wrapper Tidak Berguna

**Tingkat Risiko:** 🟡 Rendah  
**File:** `src/mcore/melisad/probes/starup_node.rs`  
**Baris:** 57–61

### Deskripsi

Fungsi `save_state_to_disk` hanya memanggil `self.flush()` tanpa tambahan logika apapun:

```rust
/// Fungsi utilitas untuk menulis state ke JSON file
fn save_state_to_disk(&self) -> std::result::Result<(), NodeError> {
    self.flush()?;  // ← Hanya ini yang dilakukan
    Ok(())
}
```

Ini adalah layer abstraksi yang tidak menambahkan nilai. Pemanggil `save_state_to_disk()` bisa langsung memanggil `self.flush()` dengan efek yang identik. Komentar doc-nya pun tidak akurat ("menulis state ke JSON file" — padahal `flush()` yang melakukan itu).

### Dampak

- Menambah cognitive overhead saat membaca kode.
- Developer baru mungkin berpikir ada logika lebih di `save_state_to_disk` yang berbeda dari `flush`.

### Rekomendasi

Hapus fungsi ini dan ganti pemanggilan di `startup_node_check`:

```rust
// Sebelum:
self.save_state_to_disk()?;

// Sesudah:
self.flush()?;
```

Atau, jika memang ingin ada abstraksi, isi dengan logika nyata (misalnya atomic write, backup sebelum overwrite, dsb.).

---

## INCOMPLETE-04 · `ApiResponse<T>` Terdefinisi Tapi Tidak Pernah Digunakan

**Tingkat Risiko:** 🟡 Rendah  
**File:** `src/mcore/adapter/json.rs`  
**Baris:** 16–22

### Deskripsi

Struct `ApiResponse<T>` didefinisikan tapi:
1. Tidak di-derive `Serialize`/`Deserialize` sehingga tidak bisa digunakan untuk JSON response.
2. Tidak digunakan di manapun dalam codebase.
3. Tidak ada endpoint HTTP yang mengembalikan tipe ini.

```rust
pub struct ApiResponse<T> {
    pub request_id: String,
    pub success: bool,
    pub code: u16,
    pub message: String,
    pub data: Option<T>,
    // ↑ Tidak ada #[derive(Serialize, Deserialize, Debug)]
    // ↑ Tidak ada impl apapun
    // ↑ Tidak dipakai di manapun
}
```

### Dampak

- Jika ada developer yang mencoba menggunakan `ApiResponse` untuk mengembalikan JSON, akan mendapat compile error karena tidak ada `Serialize`.
- Menandakan bahwa REST API layer belum diimplementasikan sama sekali — semua node management saat ini hanya bisa dilakukan via kode langsung, bukan via HTTP API.

### Rekomendasi

**Opsi A — Lengkapi implementasinya:**

```rust
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
        ApiResponse {
            request_id,
            success: true,
            code: 200,
            message: "OK".to_string(),
            data: Some(data),
        }
    }

    pub fn error(request_id: String, code: u16, message: String) -> Self {
        ApiResponse {
            request_id,
            success: false,
            code,
            message,
            data: None,
        }
    }
}
```

**Opsi B — Hapus jika tidak ada rencana REST API:**

Jika node management akan selalu dilakukan via konfigurasi file, hapus struct ini untuk mengurangi noise.

---

## INCOMPLETE-05 · `ConfigError` Tidak Pernah Digunakan

**Tingkat Risiko:** 🟡 Rendah  
**File:** `src/mcore/errors/e_config.rs`

### Deskripsi

```rust
#[derive(Debug)]
pub enum ConfigError {
    InvalidValue(String),
}
```

Enum ini terdefinisi tapi tidak pernah digunakan di manapun. `Config::from_file` mengembalikan `Box<dyn std::error::Error>`, bukan `ConfigError`. Tidak ada fungsi lain yang menggunakan tipe ini.

### Rekomendasi

Jika `ConfigError` dimaksudkan untuk validasi config (misalnya port di luar range, path tidak valid, dsb.), implementasikan validasi tersebut:

```rust
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
                write!(f, "Config field '{}' invalid: {}", field, reason),
            ConfigError::MissingField(field) =>
                write!(f, "Required config field '{}' is missing", field),
            ConfigError::FileNotFound(path) =>
                write!(f, "Config file not found: {}", path),
            ConfigError::ParseError(msg) =>
                write!(f, "Config parse error: {}", msg),
        }
    }
}
```

Atau hapus file ini hingga ada kebutuhan nyata.

---

*Lihat `03-MASALAH-ARSITEKTUR.md` untuk masalah desain dan struktur.*
