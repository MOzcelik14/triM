# triM. Dürüst ve Tekrarlanabilir Medya Benchmark Raporu

**Tarih:** 2026-11-28 14:08:31 +0300
**Komut Satırı:** `cargo run --release -p trim-media --example perf_bench -- /home/mozcelik/İndirilenler/MVI_8603.MOV`

> [!IMPORTANT]
> **Önceki Sürüm Düzeltme Notu:** Bu dosyanın önceki sürümündeki sayılar hatalıydı ve silinmiştir:
> - `1612470.7×` hızlanma oranı hatalıydı: 45 ms'lik sabit halka önbellek toleransı, 23.976 fps'te (kare süresi 41.7 ms) ardışık her scrub adımını yanlışlıkla `cache-hit` sayarak `0.000 ms` ölçüyordu.
> - `7133.8 FPS` unpaced decode sayısı hatalıydı: `get_frame_for_time` son kareyi (`current_frame.clone()`) döndürdüğü için tüketici döngüsü yeni kare beklemeden aynı kareyi sayıyordu.
> - `671/671 (%100)` teslim ve `0.000 ms PTS drift` hatalıydı: dönen karenin gerçek PTS'i kontrol edilmiyor ve `target_t - n/fps` (her zaman 0) hesaplanıyordu.
> - `FFmpeg CPU %0.0` hatalıydı: `/proc/<pid>/stat` çocuk süreç kapandıktan sonra okunduğu için `0` dönüyordu.

## 1. Sistem ve Ortam Bilgisi

- **İşlemci (CPU):** 12th Gen Intel(R) Core(TM) i5-12450H (12 mantıksal çekirdek)
- **Bellek (RAM):** 23731.5 MB toplam, 16275.7 MB kullanılabilir
- **İşletim Sistemi / Çekirdek:** Linux 7.0.0-44-generic x86_64 GNU/Linux
- **FFmpeg Sürümü:** ffmpeg version 6.1.1-3ubuntu5 Copyright (c) 2000-2023 the FFmpeg developers

## 2. Test Edilen Medya

- **Etiket:** `GERÇEK`
- **Dosya:** `/home/mozcelik/İndirilenler/MVI_8603.MOV`
- **Kodek / Çözünürlük / FPS:** h264 · 1920×1080 · `24000/1001` (23.976 fps)
- **Bit Hızı / Piksel Formatı / Süre:** 44647.0 kbps · `yuvj420p` · 31.45 s

## 3. Süreç Başlatma (Spawn) Maliyeti (Ayrı Ölçüm, 5 Tekrar)

| Aşama | Medyan (ms) | p95 (ms) | Min (ms) | Max (ms) |
|---|---:|---:|---:|---:|
| **FFmpeg Process Spawn** | 1.11 | 1.13 | 1.06 | 1.13 |
| **İlk Kareye Ulaşma (Spawn + Decode)** | 212.97 | 217.15 | 210.10 | 217.15 |

## 4. Oynatma: Paced (Kaynak FPS'e Kilitli) vs Unpaced (Yeni Kare Çekme Hızı) — 5 Tekrar

*PTS toleransı: ±20.85 ms (`0.5 / fps`). Yinelenen/bayat kareler düşen kare sayılır.*

| Metrik | Medyan | p95 | Min | Max |
|---|---:|---:|---:|---:|
| **Paced Teslim Edilen (%)** | 100.0% | 100.0% | 100.0% | 100.0% |
| **Paced Düşen / Bayat (%)** | 0.0% | 0.0% | 0.0% | 0.0% |
| **Paced Kare Gecikmesi p95 (ms)** | 0.65 | 0.89 | 0.62 | 0.89 |
| **Paced PTS Sapma Maks (ms)** | 0.00 | 0.00 | 0.00 | 0.00 |
| **triM CPU (%)** | 4.7% | 5.1% | 4.4% | 5.1% |
| **FFmpeg Alt Süreç CPU (%) (`/proc/<pid>/stat`)** | 55.0% | 56.7% | 55.0% | 56.7% |
| **FFmpeg Tepe RSS (MB)** | 363.4 | 363.5 | 363.3 | 363.5 |
| **Unpaced Benzersiz Decode FPS** | 478.3 FPS | 483.8 FPS | 471.7 FPS | 483.8 FPS |

## 5. Seek Gecikmesi: Cache-Miss vs Cache-Hit (Ayrıştırılmış)

| Seek Tipi | Durum | Medyan (ms) | p95 (ms) | Min (ms) | Max (ms) |
|---|---|---:|---:|---:|---:|
| **Cold Seek (`decode_frame_raw_sized`)** | Cache-Miss (yeni süreç) | 310.21 | 357.34 | 297.41 | 382.23 |
| **Hot Seek (`PlaybackStreamer` restart)** | Cache-Miss (konum atlama) | 295.45 | 316.88 | 221.21 | 326.31 |
| **Scrub İleri Adım (`PersistentSeekDecoder`)** | Cache-Miss (yeni kare) | 1.25 | 2.18 | 1.03 | 2.30 |
| **Scrub Tekrar Sorgu (`recent_frames`)** | Cache-Hit (Arc paylaşımı) | 0.0004 | 0.0008 | 0.0001 | 0.0022 |

### P0-2 Önce / Sonra Karşılaştırması: Sabit 45 ms Tolerans vs FPS'e Bağlı Tolerans

| Senaryo (100 ileri kare adımı, önbellekte yalnızca önceki kare var) | Eski Sabit `45 ms` | Yeni `0.5 / fps` |
|---|---:|---:|
| **23.976 fps yanlış komşu kare eşleşmesi** | 100 / 100 | 0 / 100 |
| **30.0 fps yanlış komşu kare eşleşmesi** | 100 / 100 | 0 / 100 |
| **60.0 fps yanlış komşu kare eşleşmesi** | 100 / 100 | 0 / 100 |

## 6. Zaman Çizelgesi Yazılım Kompozisyonu (Cache-Miss, 5 Tekrar)

| Track Sayısı | Medyan (ms/kare) | p95 (ms/kare) | Min (ms/kare) | Max (ms/kare) | RSS (MB) |
|---:|---:|---:|---:|---:|---:|
| 1 | 7.23 | 7.33 | 6.16 | 7.33 | 61.0 |
| 3 | 7.31 | 8.40 | 6.36 | 8.40 | 66.3 |
| 6 | 8.36 | 8.78 | 7.30 | 8.78 | 67.2 |
