# triM. Dürüst ve Tekrarlanabilir Medya Benchmark Raporu

**Tarih:** 2026-11-24 22:20:38 +0300
**Komut Satırı:** `cargo run --release -p trim-media --example perf_bench -- /home/mozcelik/İndirilenler/MVI_8603.MOV`

> [!IMPORTANT]
> **Önceki Sürüm Düzeltme Notu:** Bu dosyanın önceki sürümündeki sayılar hatalıydı ve silinmiştir:
> - `1612470.7×` hızlanma oranı hatalıydı: 45 ms'lik sabit halka önbellek toleransı, 23.976 fps'te (kare süresi 41.7 ms) ardışık her scrub adımını yanlışlıkla `cache-hit` sayarak `0.000 ms` ölçüyordu.
> - `7133.8 FPS` unpaced decode sayısı hatalıydı: `get_frame_for_time` son kareyi (`current_frame.clone()`) döndürdüğü için tüketici döngüsü yeni kare beklemeden aynı kareyi sayıyordu.
> - `671/671 (%100)` teslim ve `0.000 ms PTS drift` hatalıydı: dönen karenin gerçek PTS'i kontrol edilmiyor ve `target_t - n/fps` (her zaman 0) hesaplanıyordu.
> - `FFmpeg CPU %0.0` hatalıydı: `/proc/<pid>/stat` çocuk süreç kapandıktan sonra okunduğu için `0` dönüyordu.

## 1. Sistem ve Ortam Bilgisi

- **İşlemci (CPU):** 12th Gen Intel(R) Core(TM) i5-12450H (12 mantıksal çekirdek)
- **Bellek (RAM):** 23731.6 MB toplam, 18135.8 MB kullanılabilir
- **İşletim Sistemi / Çekirdek:** Linux 7.0.0-43-generic x86_64 GNU/Linux
- **FFmpeg Sürümü:** ffmpeg version 6.1.1-3ubuntu5 Copyright (c) 2000-2023 the FFmpeg developers

## 2. Test Edilen Medya

- **Etiket:** `KULLANICI KLİBİ`
- **Dosya:** `/home/mozcelik/İndirilenler/MVI_8603.MOV`
- **Kodek / Çözünürlük / FPS:** h264 · 1920×1080 · `24000/1001` (23.976 fps)
- **Bit Hızı / Piksel Formatı / Süre:** 44647.0 kbps · `yuvj420p` · 31.45 s

## 3. Süreç Başlatma (Spawn) Maliyeti (Ayrı Ölçüm, 5 Tekrar)

| Aşama | Medyan (ms) | p95 (ms) | Min (ms) | Max (ms) |
|---|---:|---:|---:|---:|
| **FFmpeg Process Spawn** | 1.11 | 1.12 | 1.10 | 1.12 |
| **İlk Kareye Ulaşma (Spawn + Decode)** | 98.30 | 112.94 | 94.69 | 112.94 |

## 4. Oynatma: Paced (Kaynak FPS'e Kilitli) vs Unpaced (Yeni Kare Çekme Hızı) — 5 Tekrar

*PTS toleransı: ±20.85 ms (`0.5 / fps`). Yinelenen/bayat kareler düşen kare sayılır.*

| Metrik | Medyan | p95 | Min | Max |
|---|---:|---:|---:|---:|
| **Paced Teslim Edilen (%)** | 100.0% | 100.0% | 100.0% | 100.0% |
| **Paced Düşen / Bayat (%)** | 0.0% | 0.0% | 0.0% | 0.0% |
| **Paced Kare Gecikmesi p95 (ms)** | 0.57 | 0.64 | 0.55 | 0.64 |
| **Paced PTS Sapma Maks (ms)** | 0.00 | 0.00 | 0.00 | 0.00 |
| **triM CPU (%)** | 4.1% | 4.4% | 3.4% | 4.4% |
| **FFmpeg Alt Süreç CPU (%) (`/proc/<pid>/stat`)** | 101.3% | 103.7% | 97.9% | 103.7% |
| **FFmpeg Tepe RSS (MB)** | 179.6 | 180.7 | 179.5 | 180.7 |
| **Unpaced Benzersiz Decode FPS** | 717.3 FPS | 735.0 FPS | 655.1 FPS | 735.0 FPS |

## 5. Seek Gecikmesi: Cache-Miss vs Cache-Hit (Ayrıştırılmış)

| Seek Tipi | Durum | Medyan (ms) | p95 (ms) | Min (ms) | Max (ms) |
|---|---|---:|---:|---:|---:|
| **Cold Seek (`decode_frame_raw_sized`)** | Cache-Miss (yeni süreç) | 154.06 | 165.88 | 116.96 | 192.36 |
| **Hot Seek (`PlaybackStreamer` restart)** | Cache-Miss (konum atlama) | 106.41 | 113.95 | 96.35 | 114.27 |
| **Scrub İleri Adım (`PersistentSeekDecoder`)** | Cache-Miss (yeni kare) | 1.07 | 2.36 | 0.00 | 5.87 |
| **Scrub Tekrar Sorgu (`recent_frames`)** | Cache-Hit (Arc paylaşımı) | 0.0005 | 0.0007 | 0.0001 | 0.0010 |

### P0-2 Önce / Sonra Karşılaştırması: Sabit 45 ms Tolerans vs FPS'e Bağlı Tolerans

| Senaryo (100 ileri kare adımı, önbellekte yalnızca önceki kare var) | Eski Sabit `45 ms` | Yeni `0.5 / fps` |
|---|---:|---:|
| **23.976 fps yanlış komşu kare eşleşmesi** | 100 / 100 | 0 / 100 |
| **30.0 fps yanlış komşu kare eşleşmesi** | 100 / 100 | 0 / 100 |
| **60.0 fps yanlış komşu kare eşleşmesi** | 100 / 100 | 0 / 100 |

## 6. Zaman Çizelgesi Yazılım Kompozisyonu (Cache-Miss, 5 Tekrar)

| Track Sayısı | Medyan (ms/kare) | p95 (ms/kare) | Min (ms/kare) | Max (ms/kare) | RSS (MB) |
|---:|---:|---:|---:|---:|---:|
| 1 | 2.06 | 2.53 | 1.98 | 2.53 | 67.1 |
| 3 | 9.04 | 9.54 | 8.48 | 9.54 | 75.0 |
| 6 | 15.80 | 16.85 | 15.31 | 16.85 | 138.3 |
