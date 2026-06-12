# Rekomendasi Aksi — Melisa Core Team

> Daftar aksi prioritas yang sudah terurut berdasarkan dampak dan kemudahan implementasi.  
> Gunakan dokumen ini sebagai **backlog teknis** untuk sprint/iteration berikutnya.

---

## Matriks Prioritas

| ID | Masalah | Dampak | Usaha | Prioritas |
|----|---------|:------:|:-----:|:---------:|
| BUG-01 | Logger INFO/WARN salah buffer | 🔴 Tinggi | 🟢 Rendah | **P0** |
| BUG-03 | LeastConnections salah implementasi | 🔴 Tinggi | 🟡 Sedang | **P0** |
| BUG-05 | `log_proxy` bypass buffer | 🟠 Sedang | 🟢 Rendah | **P1** |
| BUG-04 | Random tidak acak | 🟠 Sedang | 🟢 Rendah | **P1** |
| ARCH-01 | CONFIG/LOGGER panic | 🔴 Tinggi | 🟡 Sedang | **P1** |
| INCOMPLETE-01 | `mnode` kosong | 🔴 Tinggi | 🔴 Tinggi | **P1** |
| INCOMPLETE-02 | Test palsu forwarder | 🟠 Sedang | 🟡 Sedang | **P1** |
| QA-02 | Typo `starup_node.rs` | 🟡 Rendah | 🟢 Rendah | **P2** |
| QA-05 | Error response tanpa Content-Type | 🟡 Rendah | 🟢 Rendah | **P2** |
| QA-06 | access_log_format tidak digunakan | 🟡 Rendah | 🟡 Sedang | **P2** |
| ARCH-02 | Tidak ada graceful shutdown | 🟠 Sedang | 🟡 Sedang | **P2** |
| ARCH-03 | NODE_FILE vs CONFIG conflict | 🟡 Rendah | 🟢 Rendah | **P2** |
| BUG-02 | Dead logic di log_error | 🟡 Rendah | 🟢 Rendah | **P3** |
| QA-01 | Double clone | 🟡 Rendah | 🟢 Rendah | **P3** |
| QA-03 | Rename generate_hash | 🟡 Rendah | 🟢 Rendah | **P3** |
| MISSING-01 | REST API untuk manajemen node | 🔴 Tinggi | 🔴 Tinggi | **P2** |
| MISSING-04 | Metrics endpoint | 🟠 Sedang | 🟡 Sedang | **P3** |
| MISSING-05 | Health endpoint | 🟠 Sedang | 🟢 Rendah | **P2** |
| ARCH-05 | Rate limiting | 🔴 Tinggi | 🔴 Tinggi | **P3** |

---

## Sprint 1 — Perbaikan Bug Kritis (Estimasi: 3–5 hari)

Fokus pada bug yang menyebabkan perilaku salah secara nyata.

### Ticket S1-01: Fix Logger Buffer Routing
**Referensi:** BUG-01, BUG-05  
**Assignee:** Backend Logger

```
Tugas:
1. Tambahkan field `info_logs: Vec<String>` ke struct `LogBuffer`
2. Ubah `log_info` untuk push ke `info_logs`
3. Ubah `log_warn` untuk push ke `error_logs` (atau buat `warn_logs` terpisah) — 
   pastikan keputusan ini terdokumentasi
4. Tambahkan `proxy_logs: Vec<String>` ke `LogBuffer`
5. Ubah `log_proxy` untuk menggunakan buffer, bukan direct write
6. Update `flush()` untuk drain semua buffer termasuk yang baru
7. Tulis test untuk memverifikasi routing yang benar:
   - log_info → access/info file, BUKAN error file
   - log_proxy → proxy.log via buffer
```

**Definisi Done:** Test baru lulus. `grep -c "INFO" error.log` = 0 setelah startup normal.

---

### Ticket S1-02: Fix Load Balancer LeastConnections
**Referensi:** BUG-03, BUG-04  
**Assignee:** Proxy Team

```
Tugas (Opsi A — Full implementation):
1. Tambahkan `connection_counts: Arc<DashMap<String, AtomicUsize>>` ke `LoadBalancer`
2. Increment counter saat connection masuk (di handler.rs)
3. Decrement saat connection selesai
4. Implementasikan LeastConnections berdasarkan counter sebenarnya

Tugas (Opsi B — Honest placeholder, lebih cepat):
1. Ubah LeastConnections untuk fallback ke RoundRobin
2. Tambahkan log WARNING: "LeastConnections not fully implemented, using RoundRobin"
3. Buat issue untuk full implementation di sprint berikutnya

Tugas (Random fix — wajib, 15 menit):
1. Import `rand::Rng`
2. Ganti `SystemTime::now()...as_nanos() % len` dengan `rand::rng().random_range(0..len)`
```

---

### Ticket S1-03: Fix Error Handling CONFIG dan LOGGER
**Referensi:** ARCH-01  
**Assignee:** Platform Team

```
Tugas:
1. Ubah CONFIG untuk tidak menggunakan .unwrap() di static
2. Pindahkan inisialisasi CONFIG ke awal main() dengan error handling yang baik
3. Tampilkan pesan error yang membantu: path yang dicoba, contoh format yang benar
4. Lakukan hal yang sama untuk LOGGER
5. Tambahkan melisa.conf.example ke repo sebagai referensi

Contoh output error yang baik:
❌ Melisa gagal start: konfigurasi tidak ditemukan
   File yang dicari: melisa.conf
   Pastikan file tersebut ada di direktori yang sama dengan binary.
   Contoh konfigurasi minimal: lihat melisa.conf.example
```

---

## Sprint 2 — Penguatan Test & Typo Fix (Estimasi: 3–4 hari)

### Ticket S2-01: Ganti Test Palsu di `forwarder.rs`
**Referensi:** INCOMPLETE-02

```
Tugas:
1. Tambahkan `wiremock = "0.6"` ke dev-dependencies Cargo.toml
2. Hapus test `assert_eq!(1, 1)`
3. Tulis test berikut:
   - test_forward_success: mock server returns 200, assert response 200
   - test_forward_retry_on_500: mock returns 500 dua kali lalu 200, assert retry works
   - test_forward_max_retries_exceeded: selalu 500, assert error dikembalikan setelah N retry
   - test_forward_network_error: server tidak ada, assert error
   - test_backoff_duration: pastikan backoff exponential/linear sesuai config
   - test_hop_by_hop_headers_filtered: pastikan Connection/Transfer-Encoding tidak diteruskan
```

---

### Ticket S2-02: Fix Typo dan Code Smell Kecil
**Referensi:** QA-02, QA-05, BUG-02, ARCH-03

```
Tugas (semua bisa di-batch dalam satu PR):
1. git mv starup_node.rs → startup_node.rs, update mod.rs
2. Tambahkan Content-Type: application/json ke semua error response di handler.rs
3. Hapus dead logic `if self.level > LogLevel::Error` di log_error
4. Hapus/deprecate NODE_FILE dari mconf.rs, update test yang menggunakannya
5. Pindahkan/hapus komentar personal di main.rs
6. Tambahkan HASH_LENGTH ke validasi di delete_node di api/services.rs
```

---

### Ticket S2-03: Perbaiki Double Clone dan Rename
**Referensi:** QA-01, QA-03, QA-07, ARCH-04

```
Tugas:
1. Refactor (**processes_lock).clone() dengan komentar penjelasan yang jelas
2. Rename generate_hash → generate_node_id dengan doc comment yang akurat
3. Rename startup_node_check → pisahkan menjadi:
   - startup_validation() → hanya dipanggil saat startup
   - periodic_health_check() → dipanggil oleh health monitor
4. Rename save_state_to_disk → hapus, ganti langsung dengan self.flush()
```

---

## Sprint 3 — Graceful Shutdown & Health Endpoint (Estimasi: 2–3 hari)

### Ticket S3-01: Graceful Shutdown
**Referensi:** ARCH-02

```
Tugas:
1. Tambahkan tokio::signal ke Cargo.toml jika belum ada (sudah included di tokio::full)
2. Implementasikan SIGTERM/SIGINT handler di run_proxy_server()
3. Pada shutdown:
   a. Stop menerima koneksi baru
   b. Tunggu koneksi aktif selesai (dengan timeout, misalnya 30s)
   c. LOGGER.flush() — pastikan semua log ter-flush
   d. NODE_MANAGER.flush() — pastikan state ter-simpan
4. Test shutdown dengan `kill -SIGTERM $(pgrep melisa)`
   Verifikasi: tidak ada log entry yang hilang, semua koneksi aktif selesai
```

---

### Ticket S3-02: Health Endpoint
**Referensi:** MISSING-05

```
Tugas:
1. Intercept GET /_melisa/health di handler.rs sebelum load balancing
2. Response 200 dengan JSON: status, version, node counts, uptime
3. Response 503 jika tidak ada node aktif sama sekali
4. Tambahkan ke LOGGING.md dan README dokumentasi endpoint ini
```

---

## Sprint 4 — REST API Node Management (Estimasi: 5–7 hari)

### Ticket S4-01: Admin HTTP API
**Referensi:** MISSING-01, MISSING-02, MISSING-03

```
Tugas:
1. Tambahkan config [admin] ke melisa.conf:
   - admin_port = 8081
   - admin_enabled = true
   - admin_token = "" (kosong = no auth)
2. Implementasikan router sederhana untuk admin API:
   POST   /nodes       → create_node
   GET    /nodes       → list_nodes
   DELETE /nodes/:hash → delete_node
   PATCH  /nodes/:hash → update_node (URL, status)
3. Implementasikan NodeManager::update() dan set_status()
4. Tambahkan Bearer token auth jika admin_token di-set
5. Semua response menggunakan ApiResponse<T> yang diperbaiki
6. Tulis integration test untuk setiap endpoint
```

---

## Sprint 5 — mnode Worker (Estimasi: 7–10 hari)

### Ticket S5-01: Implementasi mnode
**Referensi:** INCOMPLETE-01

```
Ini adalah fitur besar. Breakdown:

Minggu 1:
1. Definisikan protokol registrasi: mnode → melisad via HTTP POST /nodes
2. mnode parse argumen CLI: --name, --port, --domain, --route
3. mnode auto-register ke melisa daemon saat startup
4. mnode auto-unregister saat shutdown (SIGTERM handler)

Minggu 2:
5. mnode expose /health endpoint untuk liveness probe
6. mnode bisa serve application yang di-spawn sebagai child process
7. Atau mnode bisa sebagai sidecar yang forward ke process lain

Dependencies yang perlu ditambahkan ke mnode/Cargo.toml:
- reqwest = { version = "0.13", features = ["json"] }
- tokio = { version = "1", features = ["full"] }
- clap = "4" (CLI argument parsing)
- serde = { version = "1", features = ["derive"] }
```

---

## Sprint 6 — Observability (Estimasi: 3–5 hari)

### Ticket S6-01: Metrics Endpoint + access_log_format
**Referensi:** MISSING-04, QA-06

```
Tugas:
1. Implementasikan to_prometheus_text() di ProxyMetrics
2. Expose di GET /admin/metrics (atau /_melisa/metrics di proxy port)
3. Implementasikan format parser untuk access_log_format config
4. Test bahwa mengubah access_log_format di config benar-benar mengubah format log
```

---

## Quick Wins — Bisa Dikerjakan Kapan Saja (< 1 jam per item)

Berikut adalah perubahan kecil yang bisa dikerjakan siapa saja di tim tanpa perlu sprint planning:

```
□ Hapus assert_eq!(1,1) di forwarder.rs — ganti dengan TODO comment
□ Rename starup_node.rs → startup_node.rs
□ Tambahkan Content-Type header ke error responses
□ Ganti rand di loadbalancer.rs (sudah ada di Cargo.toml, tinggal dipakai)
□ Hapus komentar personal di main.rs
□ Tambahkan #[allow(dead_code)] atau hapus ConfigError yang tidak terpakai
□ Dokumentasikan behavior log_warn yang menulis ke error_logs (jika memang by design)
```

---

## Checklist Pre-Release (Sebelum v0.1.0 Stabil)

Sebelum Melisa dianggap siap untuk production use, pastikan semua item ini terpenuhi:

```
Bugs Kritis:
[ ] BUG-01: Logger INFO/WARN routing diperbaiki
[ ] BUG-03: LeastConnections diimplementasikan dengan benar (atau jelas as fallback)
[ ] BUG-04: Random menggunakan rand::rng()
[ ] BUG-05: log_proxy menggunakan buffer

Ketangguhan:
[ ] CONFIG panic diganti dengan error handling yang informatif
[ ] Graceful shutdown diimplementasikan
[ ] Health endpoint tersedia di /_melisa/health

Test Coverage:
[ ] forwarder.rs memiliki test nyata (bukan assert_eq!(1,1))
[ ] handler.rs memiliki test
[ ] api/services.rs memiliki test
[ ] Integration test untuk skenario end-to-end

Operability:
[ ] Metrics bisa di-scrape (minimal JSON endpoint)
[ ] REST API untuk node management tersedia
[ ] melisa.conf.example tersedia di repo
[ ] README menjelaskan cara run, configure, dan manage nodes

Code Quality:
[ ] Tidak ada typo di nama file (starup_node.rs)
[ ] Tidak ada komentar personal di production code
[ ] access_log_format benar-benar digunakan
```

---

## Catatan Akhir untuk Tim

Arsitektur Melisa menunjukkan **pemahaman yang solid tentang Rust concurrent programming** — pola `Arc<RwLock>` dengan Copy-on-Write semantics, penggunaan `Lazy` untuk singleton, async HTTP dengan Tokio, dan struktur modul yang logis. Ini fondasi yang kuat.

Namun ada beberapa anti-pattern yang perlu diperhatikan ke depan:

1. **Jangan commit test palsu.** Lebih baik tidak ada test daripada `assert_eq!(1, 1)` yang memberi false confidence.

2. **Static global yang panic tersembunyi** (`CONFIG.unwrap()`, `LOGGER panic!`) adalah time bomb di production. Selalu tangani error secara eksplisit di `main()`.

3. **Stub yang tidak diberi komentar** (seperti LeastConnections yang tidak LeastConnections) lebih berbahaya dari tidak ada fitur sama sekali. Selalu tandai dengan `// TODO:` atau `unimplemented!()` jika fitur belum siap.

4. **Format config yang tidak digunakan** (`access_log_format`) menciptakan bug tersembunyi — operator percaya mereka mengubah perilaku padahal tidak. Selalu pastikan config yang terdokumentasi benar-benar diimplementasikan.

Tim ini jelas kompeten — tinggal sedikit disiplin dalam menyelesaikan setiap fitur sampai tuntas sebelum pindah ke yang berikutnya. 🦀

---

*Dokumen ini dibuat berdasarkan analisis kode dari `repomix-output-ernoba-melisa_beta_git-8.md` pada Juni 2026.*
