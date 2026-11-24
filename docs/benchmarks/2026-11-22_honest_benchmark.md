# triM. Dürüst ve Tekrarlanabilir Medya Benchmark Raporu

**Tarih:** 2026-11-24 23:26:57 +0300
**Komut Satırı:** `cargo run --release -p trim-media --example perf_bench -- /home/mozcelik/İndirilenler/MVI_8603.MOV`

> [!IMPORTANT]
> **Önceki Sürüm Düzeltme Notu:** Bu dosyanın önceki sürümündeki sayılar hatalıydı ve silinmiştir:
> - `1612470.7×` hızlanma oranı hatalıydı: 45 ms'lik sabit halka önbellek toleransı, 23.976 fps'te (kare süresi 41.7 ms) ardışık her scrub adımını yanlışlıkla `cache-hit` sayarak `0.000 ms` ölçüyordu.
> - `7133.8 FPS` unpaced decode sayısı hatalıydı: `get_frame_for_time` son kareyi (`current_frame.clone()`) döndürdüğü için tüketici döngüsü yeni kare beklemeden aynı kareyi sayıyordu.
> - `671/671 (%100)` teslim ve `0.000 ms PTS drift` hatalıydı: dönen karenin gerçek PTS'i kontrol edilmiyor ve `target_t - n/fps` (her zaman 0) hesaplanıyordu.
> - `FFmpeg CPU %0.0` hatalıydı: `/proc/<pid>/stat` çocuk süreç kapandıktan sonra okunduğu için `0` dönüyordu.

## 1. Sistem ve Ortam Bilgisi

- **İşlemci (CPU):** 12th Gen Intel(R) Core(TM) i5-12450H (12 mantıksal çekirdek)
- **Bellek (RAM):** 23731.6 MB toplam, 16421.5 MB kullanılabilir
- **İşletim Sistemi / Çekirdek:** Linux 7.0.0-43-generic x86_64 GNU/Linux
- **FFmpeg Sürümü:** ffmpeg version 6.1.1-3ubuntu5 Copyright (c) 2000-2023 the FFmpeg developers

## 2. Test Edilen Medya

- **Etiket:** `GERÇEK`
- **Dosya:** `/home/mozcelik/İndirilenler/MVI_8603.MOV`
- **Kodek / Çözünürlük / FPS:** h264 · 1920×1080 · `24000/1001` (23.976 fps)
- **Bit Hızı / Piksel Formatı / Süre:** 44647.0 kbps · `yuvj420p` · 31.45 s

## 3. Süreç Başlatma (Spawn) Maliyeti (Ayrı Ölçüm, 5 Tekrar)

| Aşama | Medyan (ms) | p95 (ms) | Min (ms) | Max (ms) |
|---|---:|---:|---:|---:|
| **FFmpeg Process Spawn** | 1.12 | 1.13 | 1.11 | 1.13 |
| **İlk Kareye Ulaşma (Spawn + Decode)** | 106.98 | 117.37 | 105.35 | 117.37 |

## 4. Oynatma: Paced (Kaynak FPS'e Kilitli) vs Unpaced (Yeni Kare Çekme Hızı) — 5 Tekrar

*PTS toleransı: ±20.85 ms (`0.5 / fps`). Yinelenen/bayat kareler düşen kare sayılır.*

| Metrik | Medyan | p95 | Min | Max |
|---|---:|---:|---:|---:|
| **Paced Teslim Edilen (%)** | 100.0% | 100.0% | 100.0% | 100.0% |
| **Paced Düşen / Bayat (%)** | 0.0% | 0.0% | 0.0% | 0.0% |
| **Paced Kare Gecikmesi p95 (ms)** | 0.48 | 0.52 | 0.42 | 0.52 |
| **Paced PTS Sapma Maks (ms)** | 0.00 | 0.00 | 0.00 | 0.00 |
| **triM CPU (%)** | 3.4% | 3.7% | 3.0% | 3.7% |
| **FFmpeg Alt Süreç CPU (%) (`/proc/<pid>/stat`)** | 91.5% | 101.0% | 83.7% | 101.0% |
| **FFmpeg Tepe RSS (MB)** | 180.8 | 181.9 | 179.8 | 181.9 |
| **Unpaced Benzersiz Decode FPS** | 578.5 FPS | 614.0 FPS | 547.9 FPS | 614.0 FPS |

## 5. Seek Gecikmesi: Cache-Miss vs Cache-Hit (Ayrıştırılmış)

| Seek Tipi | Durum | Medyan (ms) | p95 (ms) | Min (ms) | Max (ms) |
|---|---|---:|---:|---:|---:|
| **Cold Seek (`decode_frame_raw_sized`)** | Cache-Miss (yeni süreç) | 201.59 | 244.91 | 141.32 | 247.61 |
| **Hot Seek (`PlaybackStreamer` restart)** | Cache-Miss (konum atlama) | 130.95 | 138.89 | 100.62 | 162.53 |
| **Scrub İleri Adım (`PersistentSeekDecoder`)** | Cache-Miss (yeni kare) | 1.08 | 3.95 | 0.00 | 4.19 |
| **Scrub Tekrar Sorgu (`recent_frames`)** | Cache-Hit (Arc paylaşımı) | 0.0014 | 0.0035 | 0.0001 | 0.0077 |

### P0-2 Önce / Sonra Karşılaştırması: Sabit 45 ms Tolerans vs FPS'e Bağlı Tolerans

| Senaryo (100 ileri kare adımı, önbellekte yalnızca önceki kare var) | Eski Sabit `45 ms` | Yeni `0.5 / fps` |
|---|---:|---:|
| **23.976 fps yanlış komşu kare eşleşmesi** | 100 / 100 | 0 / 100 |
| **30.0 fps yanlış komşu kare eşleşmesi** | 100 / 100 | 0 / 100 |
| **60.0 fps yanlış komşu kare eşleşmesi** | 100 / 100 | 0 / 100 |

## 6. Zaman Çizelgesi Yazılım Kompozisyonu (Cache-Miss, 5 Tekrar)

| Track Sayısı | Medyan (ms/kare) | p95 (ms/kare) | Min (ms/kare) | Max (ms/kare) | RSS (MB) |
|---:|---:|---:|---:|---:|---:|
| 1 | 2.61 | 3.43 | 2.59 | 3.43 | 18.8 |
| 3 | 4.18 | 4.50 | 3.19 | 4.50 | 62.8 |
| 6 | 5.85 | 6.90 | 4.46 | 6.90 | 69.7 |
