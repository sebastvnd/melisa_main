# Quick Reference — Temuan Melisa Core Audit

> Ringkasan satu halaman dari seluruh temuan audit. Cetak atau pin di channel tim.

---

## 🔴 HARUS DIPERBAIKI SEKARANG

| File | Masalah | Fix |
|------|---------|-----|
| `mlog/logger.rs:183` | `log_info` push ke `error_logs` (SALAH!) | Push ke `info_logs` atau buffer yang benar |
| `mlog/logger.rs:200` | `log_warn` push ke `error_logs` (tercampur) | Dokumentasikan atau pisahkan |
| `mlog/logger.rs:2109` | `log_proxy` bypass buffer, direct write | Gunakan buffer seperti fungsi lain |
| `proxy/loadbalancer.rs:55` | `LeastConnections` = sort by PID (SALAH!) | Implementasikan dengan connection counter atau fallback RoundRobin |
| `proxy/loadbalancer.rs:60` | `Random` pakai nanoseconds (bias) | Pakai `rand::rng().random_range()` |
| `mnode/src/main.rs` | Hanya `println!`, tidak ada implementasi | Implementasikan worker atau buat issue |

---

## 🟠 PERBAIKI SEGERA

| File | Masalah | Fix |
|------|---------|-----|
| `config/load_config.rs:8` | `CONFIG.unwrap()` → panic jika file tidak ada | Tangani error di `main()` |
| `mlog/logger.rs:2180` | `panic!("Cannot initialize logger")` | Tangani di `main()` |
| `proxy/forwarder.rs:943` | `assert_eq!(1,1)` bukan test nyata | Tulis test dengan wiremock |
| `proxy/server.rs` | Tidak ada graceful shutdown | Tambahkan tokio::signal handling |
| Seluruh proxy | Tidak ada health endpoint | Tambahkan `/_melisa/health` |

---

## 🟡 BERSIHKAN DALAM SPRINT INI

| File | Masalah | Fix |
|------|---------|-----|
| `probes/starup_node.rs` | Typo nama file (`starup`) | `git mv → startup_node.rs` |
| `proxy/handler.rs` | Error response tanpa `Content-Type` | Tambahkan `application/json` header |
| `mlog/logger.rs:139` | `if self.level > Error` tidak pernah true | Hapus dead check |
| `mlog/config.rs` | `access_log_format` field tidak digunakan | Implementasikan format parser |
| `errors/e_config.rs` | `ConfigError` tidak pernah dipakai | Hapus atau implementasikan |
| `adapter/json.rs` | `ApiResponse` tanpa `Serialize` derive | Tambahkan derive atau hapus |
| `probes/starup_node.rs:59` | `save_state_to_disk` = wrapper kosong | Hapus, panggil `flush()` langsung |
| `services/mconf.rs:3` | `NODE_FILE` bentrok dengan `CONFIG.nodes.storage_file` | Hapus konstanta duplikat |
| `main.rs:17-19` | Komentar personal di production code | Pindahkan ke CHANGELOG |

---

## 📋 BACKLOG FITUR (Belum Ada)

| Fitur | Prioritas | Referensi |
|-------|:---------:|-----------|
| REST API: `POST/GET/DELETE /nodes` | Tinggi | MISSING-01 |
| `NodeManager::update()` dan `set_status()` | Sedang | MISSING-02 |
| Auth token untuk admin API | Sedang | MISSING-03 |
| Metrics endpoint (Prometheus/JSON) | Sedang | MISSING-04 |
| Rate limiting & max connections | Sedang | ARCH-05 |

---

## 🧪 Test Coverage Status

```
✅ adapter/json.rs      — Ada, cukup baik
✅ config/load_config.rs — Ada, baik
✅ probes/find_node.rs  — Ada, baik
✅ services/hashing.rs  — Ada, cukup
✅ mlog/logger.rs       — Ada, baik
⚠️ proxy/forwarder.rs   — Ada tapi PALSU (assert_eq!(1,1))
⚠️ proxy/loadbalancer.rs — Minimal (hanya test creation)
❌ api/services.rs      — Tidak ada
❌ proxy/handler.rs     — Tidak ada
❌ mnode               — Tidak ada
```

---

## 🏗️ Arsitektur: Apa yang Sudah Bagus

Jangan hanya fokus pada masalah — ini yang sudah dilakukan dengan benar:

- ✅ `Arc<RwLock>` + Copy-on-Write semantics di NodeManager
- ✅ Pemisahan modul yang jelas (probes / proxy / services / mlog)
- ✅ Async/await dengan Tokio yang benar
- ✅ Log buffer untuk performa (non-blocking I/O)
- ✅ `NodeError` enum yang komprehensif dengan `Display` impl
- ✅ Config serde deserialization dengan defaults
- ✅ Penggunaan `once_cell::Lazy` untuk singleton yang thread-safe
- ✅ Retry logic dengan backoff di forwarder
- ✅ Hop-by-hop header filtering di forwarder
- ✅ Route specificity ranking di find_node

---

*Full report: lihat `00-OVERVIEW.md` s/d `06-REKOMENDASI-AKSI.md`*
