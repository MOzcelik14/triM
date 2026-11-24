# triM. Performans Temel Çizgisi (Baseline — v2.5.5)

**Tarih:** 2026-11-24 22:20 +0300  
**Komut:** `cargo run --release -p trim-media --example perf_bench -- "/home/mozcelik/İndirilenler/MVI_8603.MOV"`  
**Test Medyası (`GERÇEK`):** Canon EOS 2000D (`MVI_8603.MOV`), H.264 Constrained Baseline, 1920×1080, `24000/1001` (23.976 fps), `yuvj420p`, 44647.0 kbps, 31.45 s.  
**Donanım / Ortam:** 12th Gen Intel(R) Core(TM) i5-12450H (12 mantıksal çekirdek), 23.2 GB RAM, Linux 7.0.0-43-generic x86_64, FFmpeg 6.1.1-3ubuntu5.

## Özet Ölçümler (5 Tekrarlı Medyan / p95)

| Kategori | Ölçüm | Medyan | p95 | Min | Max |
|---|---|---:|---:|---:|---:|
| **Spawn** | FFmpeg Process Spawn | 1.11 ms | — | — | — |
| **Spawn** | İlk Kareye Ulaşma (Spawn + Decode) | 98.30 ms | — | — | — |
| **Paced Oynatma (23.976 fps, ±20.85 ms PTS tol.)** | Teslim Edilen / Düşen Kare | 100.0% / 0.0% | — | — | — |
| **Paced Oynatma** | FFmpeg Alt Süreç CPU (`/proc/<pid>/stat`) | 101.3% | — | — | — |
| **Unpaced Decode** | Benzersiz Kare Çekme Hızı | 717.3 FPS | — | 655.1 FPS | 735.0 FPS |
| **Seek (Cache-Miss)** | Cold Seek (`decode_frame_raw_sized`) | 154.06 ms | — | — | — |
| **Seek (Cache-Miss)** | Hot Seek (`PlaybackStreamer` restart) | 106.41 ms | — | — | — |
| **Seek (Cache-Miss)** | Scrub İleri Adım (`PersistentSeekDecoder`) | 1.07 ms | — | — | — |
| **Seek (Cache-Hit)** | Scrub Tekrar Sorgu (`recent_frames` Arc) | 0.0005 ms | — | — | — |
| **Yazılım Kompozisyonu (640×360, Cache-Miss)** | 1 Video Track | 2.06 ms/kare | 2.53 ms | — | — |
| **Yazılım Kompozisyonu (640×360, Cache-Miss)** | 3 Video Track | 9.04 ms/kare | 9.54 ms | — | — |
| **Yazılım Kompozisyonu (640×360, Cache-Miss)** | 6 Video Track | 15.80 ms/kare | 16.85 ms | — | — |

Ayrıntılı çıktı ve P0-2 önce/sonra tolerans karşılaştırması için bkz. [`2026-11-22_honest_benchmark.md`](./2026-11-22_honest_benchmark.md) ve [`2026-11-22_honest_benchmark.json`](./2026-11-22_honest_benchmark.json).

## Kapsam Temel Çizgisi (`cargo llvm-cov`)

- **Durum:** `cargo-llvm-cov` bu ortamda kurulu değil (`error: no such command: llvm-cov`).
- **Ölçüm Yöntemi (Faz 6 için):** `rustup component add llvm-tools-preview && cargo install cargo-llvm-cov` sonrasında `cargo llvm-cov --workspace --locked --summary-only` ile satır/bölge kapsamı ölçülecektir.
- **Mevcut Test Sayısı:** Workspace genelinde `fn test_` ile tanımlı **278** birim/entegrasyon testi mevcuttur.
