# triM. Preview & Media Benchmark Baseline (v2.3.0)

Ölçüm aracı: `cargo run --release -p trim-media --example perf_bench`  
Tarih: 2026-11-21  
Tekrar: Her senaryo 5 koşu (medyan ve min..max yayılımı).

## 1. Tek Akış İleri Oynatma & Rastgele Seek (CPU Decode -> 640×360 RGBA)

| Medya | İleri Akış FPS (medyan) | Yayılım | Düşen Kare | Seek p50 (medyan) | Seek p95 (medyan) |
|---|---:|---|---:|---:|---:|
| **1080p H.264** | 1619.6 FPS | 1615.6..1644.8 | 0 | 177.1 ms | 226.7 ms |
| **4K H.264** | 634.5 FPS | 550.3..660.0 | 0 | 434.8 ms | 586.8 ms |
| **4K HEVC** | 619.8 FPS | 578.5..631.3 | 0 | 499.5 ms | 729.0 ms |

## 2. Zaman Çizelgesi Kompozitör (1080p H.264, `render_timeline_frame`, 60 kare)

| Senaryo | Kare Süresi (medyan) | Yayılım | RSS |
|---|---:|---|---:|
| **1 Video Track** | 0.00 ms/frame (önbellekli hızlı yol) | 0.00..0.00 | 6.1 MB |
| **3 Video Track (opacity=0.5)** | 11.23 ms/frame | 10.87..11.57 | 117.7 MB |

## 3. Donanım Decode Denemesi (Görev 2.1, 2 sn 4K H.264 -> 640×360 RGBA)
- **CPU decode (`scale=640:360,format=rgba`):** `0.460 s` (~130 FPS)
- **VAAPI decode (`-hwaccel vaapi -vf scale_vaapi=w=640:h=360,hwdownload,format=nv12,format=rgba`):** Başarısız (`Failed to initialise VAAPI connection: -1 (unknown libva error)`).
- **CUDA decode (`-hwaccel cuda -vf scale_cuda=w=640:h=360,hwdownload,format=nv12,format=rgba`):** `4.630 s` (**CPU'dan ~10× daha yavaş**, CUDA bağlam başlatma + `hwdownload` maliyeti nedeniyle).
- **Karar:** Ölçülmüş kazanç bulunmadığı için preview decode yolu CPU'da bırakıldı.

## 4. Ses / Video Senkron Ofseti & UI Thread İzolasyonu (Görev 2.2)
- Başlangıç (duvar saati + UI tick içinde `render_audio_interval`): **ölçülmedi** (donanım ses saati yoktu).
- Sonrası: `AudioPlaybackWorker` ayrı iş parçacığında 20 ms chunk render + `AudioStreamPlayer` beslemesi yapıyor; `update_playback_tick` içinden `render_audio_interval` kaldırıldı. Oynatma saati `compute_audio_clock_time` (`frames_written / 48000 - 0.025s` sink gecikmesi) ile ses çıkışından türetiliyor; fiziksel mikrofon/kamera clap-sync ölçümü bu ortamda **ölçülmedi**.
