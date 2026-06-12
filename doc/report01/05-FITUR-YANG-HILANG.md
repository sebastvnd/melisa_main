# Fitur yang Hilang — Melisa Core

> Fitur-fitur ini tidak ada dalam codebase saat ini namun **dibutuhkan** untuk menjadikan Melisa proxy yang layak production. Beberapa di antaranya dikonfigurasi namun tidak diimplementasikan.

---

## MISSING-01 · Tidak Ada REST API untuk Manajemen Node

**Tingkat Risiko:** 🔴 Tinggi

### Deskripsi

Semua infrastruktur untuk REST API sudah ada — `ApiRequest`, `ApiResponse`, `api_create_node`, `api_delete_node` — namun **tidak ada HTTP endpoint yang mengekspos fungsi-fungsi ini**. Saat ini satu-satunya cara menambah/menghapus node adalah:

1. Mengedit `nodes.json` secara manual.
2. Memanggil fungsi Rust secara langsung via kode (tidak mungkin dari luar proses).

Tidak ada `POST /nodes`, `DELETE /nodes/:hash`, atau `GET /nodes` yang bisa dipanggil dari luar.

### Apa yang Dibutuhkan

Tambahkan port terpisah (admin API port) atau path khusus untuk management:

```
GET    /admin/nodes              → List semua node
POST   /admin/nodes              → Tambah node baru (body: ApiRequest<CreateNodeData>)
DELETE /admin/nodes/:hash        → Hapus node
GET    /admin/nodes/:hash        → Detail node
PATCH  /admin/nodes/:hash/status → Update status node (enable/disable)
GET    /admin/health             → Health check daemon
GET    /admin/metrics            → Expose metrics
```

### Implementasi Minimal

Tambahkan listener terpisah di `main.rs`:

```rust
// main.rs
async fn run_melisa() -> Result<(), Box<dyn Error + Send + Sync>> {
    // ... existing setup ...
    
    // Admin API di port terpisah
    let admin_port = config.admin_port.unwrap_or(8081);
    tokio::spawn(run_admin_api(admin_port));
    
    // Proxy server tetap di port utama
    run_proxy_server().await?;
}
```

```toml
# melisa.conf
admin_port = 8081
admin_enabled = true
# admin_token = "secret-token"  # Opsional: token untuk autentikasi
```

---

## MISSING-02 · Tidak Ada Mekanisme Update/Edit Node

**Tingkat Risiko:** 🟠 Sedang

### Deskripsi

`NodeManager` hanya memiliki `create`, `delete`, dan `list`. Tidak ada operasi `update`:
- Tidak bisa mengubah URL node tanpa delete + recreate.
- Tidak bisa men-disable node tanpa menghapusnya.
- Tidak bisa mengubah domain atau route_path node aktif.

Hal ini berarti setiap perubahan konfigurasi node akan menyebabkan downtime sementara (node dihapus lalu dibuat ulang).

### Apa yang Dibutuhkan

```rust
impl NodeManager {
    /// Update URL dan/atau status sebuah node
    pub fn update(
        &self,
        hash: &str,
        new_url: Option<&str>,
        new_status: Option<NodeStatus>,
    ) -> Result<NodeProcess, NodeError> {
        let mut processes_lock = self.processes.write().unwrap();
        let mut new_map = (**processes_lock).clone();
        
        let node = new_map.get_mut(hash).ok_or(NodeError::NotFound)?;
        
        if let Some(url) = new_url {
            node.url = normalize_url(url)?;
        }
        if let Some(status) = new_status {
            node.status = status;
        }
        
        let updated = node.clone();
        *processes_lock = Arc::new(new_map);
        drop(processes_lock);
        
        self.flush()?;
        Ok(updated)
    }
    
    /// Enable/disable sebuah node tanpa menghapusnya
    pub fn set_status(&self, hash: &str, status: NodeStatus) -> Result<(), NodeError> {
        self.update(hash, None, Some(status))?;
        Ok(())
    }
}
```

---

## MISSING-03 · Tidak Ada Mekanisme Autentikasi/Otorisasi

**Tingkat Risiko:** 🟠 Sedang

### Deskripsi

Proxy tidak memiliki autentikasi apapun:
- Siapa saja yang bisa mencapai port proxy bisa mengirim request.
- Jika admin API diimplementasikan (lihat MISSING-01), tanpa autentikasi siapa saja bisa menambah/menghapus node.
- Tidak ada API key, token, mTLS, atau mekanisme lain.

### Apa yang Dibutuhkan (Minimal untuk Admin API)

```toml
# melisa.conf
[admin]
enabled = true
port = 8081
auth_token = "your-secret-token-here"  # Set via environment variable lebih baik
```

```rust
// Middleware autentikasi sederhana
fn check_admin_auth(req: &Request<Incoming>, config: &AdminConfig) -> bool {
    if !config.auth_enabled {
        return true;
    }
    
    req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == format!("Bearer {}", config.auth_token))
        .unwrap_or(false)
}
```

Untuk jangka panjang, pertimbangkan mTLS antara `mnode` dan `melisad`.

---

## MISSING-04 · Metrics Tidak Diekspos ke Luar (No Observability Endpoint)

**Tingkat Risiko:** 🟡 Sedang

### Deskripsi

`ProxyMetrics` mengumpulkan data (`total_requests`, `total_errors`, `bytes_forwarded`, `active_connections`) namun hanya menulis ke log file setiap interval tertentu. Data ini:

- Tidak bisa di-scrape oleh Prometheus
- Tidak bisa divisualisasikan real-time di Grafana
- Hanya tersedia lewat grep pada log file

`LOGGING.md` menyebut integrasi Grafana namun tidak ada implementasinya.

### Apa yang Dibutuhkan

**Opsi A — Prometheus-compatible `/metrics` endpoint:**

```rust
// metrics.rs — tambahkan metode expose
impl ProxyMetrics {
    pub fn to_prometheus_text(&self) -> String {
        format!(
            "# HELP melisa_requests_total Total HTTP requests proxied\n\
             # TYPE melisa_requests_total counter\n\
             melisa_requests_total {}\n\
             # HELP melisa_errors_total Total proxy errors\n\
             # TYPE melisa_errors_total counter\n\
             melisa_errors_total {}\n\
             # HELP melisa_active_connections Current active connections\n\
             # TYPE melisa_active_connections gauge\n\
             melisa_active_connections {}\n\
             # HELP melisa_bytes_forwarded_total Total bytes forwarded\n\
             # TYPE melisa_bytes_forwarded_total counter\n\
             melisa_bytes_forwarded_total {}\n",
            self.total_requests.load(Ordering::Relaxed),
            self.total_errors.load(Ordering::Relaxed),
            self.active_connections.load(Ordering::Relaxed),
            self.total_bytes_forwarded.load(Ordering::Relaxed),
        )
    }
}
```

Expose di `/metrics` path pada admin API atau port tersendiri.

**Opsi B — JSON metrics endpoint:**

```json
GET /admin/metrics
{
  "total_requests": 15420,
  "total_errors": 23,
  "active_connections": 47,
  "bytes_forwarded_mb": 1024,
  "error_rate_percent": 0.15,
  "uptime_secs": 86400
}
```

---

## MISSING-05 · Tidak Ada Health Endpoint yang Bisa Di-probe dari Luar

**Tingkat Risiko:** 🟠 Sedang

### Deskripsi

Melisa punya `liveness_node.rs` untuk mengecek kesehatan node backend, tapi tidak ada endpoint untuk mengecek kesehatan Melisa daemon itu sendiri. Kubernetes, load balancer, dan monitoring system tidak bisa mengetahui apakah Melisa masih hidup dan sehat.

### Apa yang Dibutuhkan

Endpoint `GET /health` pada proxy port (atau admin port):

```json
// Response 200 OK jika sehat
{
  "status": "healthy",
  "version": "0.1.0-beta",
  "uptime_secs": 3600,
  "nodes": {
    "total": 5,
    "active": 4,
    "stopped": 1
  },
  "proxy": {
    "listening": true,
    "address": "127.0.0.1:8080"
  }
}
```

```json
// Response 503 jika ada masalah kritis
{
  "status": "degraded",
  "reason": "No active nodes available"
}
```

### Implementasi di `handler.rs`

```rust
// Intercept path /health sebelum load balancing
if path == "/health" || path == "/_melisa/health" {
    return Ok(build_health_response());
}
```

---

*Lihat `06-REKOMENDASI-AKSI.md` untuk daftar aksi prioritas yang bisa langsung dikerjakan tim.*
