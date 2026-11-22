# triM. Dürüst ve Gerçekçi Medya & Önizleme Benchmark Raporu

**Tarih:** 2026-11-22 13:14:47 +0300
**Ölçüm Aracı:** `cargo run --release -p trim-media --example perf_bench`

## 1. Sistem ve Ortam Bilgisi

- **İşlemci (CPU):** 12th Gen Intel(R) Core(TM) i5-12450H (12 mantıksal çekirdek)
- **Bellek (RAM):** Toplam 23731.6 MB (~23.2 GB), Kullanılabilir 16534.4 MB
- **İşletim Sistemi / Çekirdek:** Linux 7.0.0-43-generic x86_64 GNU/Linux
- **Sanallaştırma (VM):** Bare Metal (none)
- **FFmpeg Sürümü:** ffmpeg version 6.1.1-3ubuntu5 Copyright (c) 2000-2023 the FFmpeg developers

## 2. Test Edilen Medya Özellikleri

- **Etiket:** `GERÇEK`
- **Dosya Yolu:** `/home/mozcelik/İndirilenler/MVI_8603.MOV`
- **Kodek & Profil:** h264 (Constrained Baseline)
- **Çözünürlük:** 1920×1080
- **Rasyonel FPS:** `24000/1001` (23.976 fps)
- **Bit Hızı:** 44647.0 kbps (~44.6 Mbps)
- **Piksel Formatı:** `yuvj420p`
- **Süre:** 31.448 saniye
- **GOP Yapısı:** Toplam 63 I-kare, 691 P-kare, 0 B-kare

## 3. İleri Oynatma: Paced (Zaman Kısıtlı) vs Serbest Ham Decode (Side-by-Side)

| Metrik | Paced (Zaman Kısıtlı) Gerçek Zamanlı | Serbest Ham Decode (Unpaced) |
|---|---:|---:|
| **Yöntem** | Deadline = $t_0 + n/fps$ (uyku ile senkron) | Boru hattından kesintisiz maksimum çekim |
| **Efektif FPS** | **23.976 FPS** (ekran hızına kilitli) | **7133.8 FPS** (işlemci decode sınırı) |
| **Ölçülen Süre** | 28.0 s (ilk 2.0 s ısınma hariç) | 0.10 s |
| **Teslim Edilen Kare** | 671 / 671 (100.0%) | 720 kare |
| **Kaçırılan / Düşen Kare** | **0 (0.0%)** | 0 (deadline yok) |
| **Kare Gecikmesi p50 / p95 / p99** | 0.39 ms / 0.52 ms / 0.57 ms | N/A |
| **İlk Kareye Ulaşma Süresi** | 125.6 ms | N/A |
| **PTS Sürüklenme (Drift) Maks** | 0.000 ms | N/A |
| **triM İşlemci (CPU) / Bellek** | %3.1 CPU / 59.1 MB RSS | N/A |
| **FFmpeg İşlemci (CPU) / Bellek** | %0.0 CPU / 194.0 MB RSS | N/A |

### 634 FPS vs 130 FPS / Gerçek Zaman Çelişkisinin Açıklaması
Eski benchmark raporundaki 634 FPS (ileri akış) ile 130 FPS (CPU decode) arasındaki çelişki tamamen ölçüm yöntemi farkından kaynaklanmaktadır:
1. 634 FPS sayısı, ffmpeg'den okunan ham boru hattının zamanlama kısıtı (pacing deadline) olmadan serbest akışta (unpaced free-run) ne kadar hızlı boşaltıldığını göstermektedir. Bu bir 'oynatma' performansı değil, boru hattı verimidir.
2. 130 FPS sayısı ise tek bir komutla 2 saniyelik klibin tamamının decode edilip diske/null'a yazılmasıdır.
3. Paced gerçek zamanlı oynatma yapıldığında ise kareler oynatma saatine (23.976 fps) göre deadline ile sorgulanır. Gerçek oynatmada FPS daima video kaynak hızına (23.976) kilitlidir; asıl başarı kriteri düşen kare oranı ve gecikme dağılımıdır.

## 4. Rastgele Seek Gecikmesi: Soğuk (Cold) vs Sıcak (Hot)

*Sabit tohumlu rastgele zaman noktalarında 100 iterasyon:*

| Seek Tipi | p50 Gecikme | p95 Gecikme | p99 Gecikme | Süreç Maliyeti |
|---|---:|---:|---:|---|
| **Soğuk Seek (`decode_frame_raw`)** | **170.9 ms** | 192.7 ms | 197.0 ms | Her seek için yeni `ffmpeg` process spawn + probe + seek |
| **Sıcak Seek (Açık `PlaybackStreamer`)** | **111.3 ms** | 119.4 ms | 125.2 ms | Çalışan streamer pipe'ına yeni zaman konumu verme |
| **Kalıcı Scrub Seek (`PersistentSeekDecoder`)** | **0.000 ms** | 0.000 ms | 0.001 ms | Kesintisiz akış borusu ve halka önbellekten anlık okuma |
| **Hızlanma Oranı (Soğuk / Scrub)** | **1612470.7×** | - | - | Kalıcı decoder sayesinde scrub gecikmesi ortadan kalktı |

## 5. Zaman Çizelgesi Kompozisyonu (Önbellek Hilesiz, 300 Benzersiz Sorgu)

> [!NOTE]
> Bu ölçüm GL context'siz ortamda yazılım CPU compositing yolunu (`draw_clip_onto_canvas`) ölçmektedir. GPU compositor bu harness kapsamı dışındadır.

| Senaryo | Kare Başına Süre (ms) | Efektif FPS | Cache Vuruş Oranı | RSS (Bellek) |
|---|---:|---:|---:|---:|
| **1 Video Track** | **0.37 ms/frame** | 2720.6 FPS | 0.0% | 52.1 MB |
| **3 Video Track** | **6.51 ms/frame** | 153.5 FPS | 0.0% | 188.7 MB |
| **6 Video Track** | **11.71 ms/frame** | 85.4 FPS | 0.0% | 317.9 MB |

## 6. Renk Aralığı Doğruluğu (Kaynak `yuvj420p` Full Range vs Export)

- **Kaynak Piksel Formatı:** `yuvj420p (full range pc, smpte170m/bt709)`
- **Ortalama |ΔRGB| Sapması:** **1.10 / 255** (ΔR: 1.07, ΔG: 0.93, ΔB: 1.31)
- **Önizleme Siyah/Beyaz Seviyesi:** Min RGB = [0, 0, 0], Max RGB = [182, 177, 183]
- **Dışa Aktarma Siyah/Beyaz Seviyesi:** Min RGB = [0, 0, 0], Max RGB = [181, 178, 184]
- **Değerlendirme:** YÜKSEK UYUM (mean |ΔRGB| <= 5.0): Preview ve export renk aralığı kabul edilebilir sınırlar içinde.

## 7. Donanım Hızlandırma (HW Decode) Denemeleri

| Motor | Durum | 60 Kare Süresi | Decode FPS | Açıklama |
|---|---|---:|---:|---|
| **CPU Reference (swscale)** | ✅ Başarılı | 0.201 s | 299.2 FPS | Temel yazılım decode ve RGBA ölçekleme hattı |
| **VAAPI (Intel iHD)** | ❌ Başarısız | - | - | Bu ortamda çalışmadı: [AVHWDeviceContext @ 0x5fd8cc2a0e80] Failed to initialise VAAPI connection: -1 (unknown libva error). Device creation failed: -5. Failed to set value '/dev/dri/renderD128' for option 'vaapi_device': Input/output error Error parsing global options: Input/output error |
| **CUDA (NVIDIA RTX 3050)** | ✅ Başarılı | 0.523 s | 114.8 FPS | Başarılı ancak context init + hwdownload nedeniyle 60 kare için CPU'dan daha uzun sürdü (0.523s vs CPU 0.201s). |

## 8. Türetilmiş 4K HEVC Medya Sonuçları (`TÜRETİLMİŞ`)

> [!WARNING]
> Bu medya orijinal 1080p klibin libx265 ile 3840x2160 çözünürlüğe yukarı ölçeklenmesiyle oluşturulmuştur; gerçek 4K kamera çekimi değildir.

- **Paced Oynatma:** Teslim Edilen = %100.0, Düşen = %0.0, Gecikme p50 = 0.38 ms, p95 = 0.54 ms
- **Serbest Ham Decode:** 1232.6 FPS
- **Seek Gecikmesi:** Soğuk p50 = 747.2 ms, Sıcak p50 = 522.3 ms (Fark: 1.4×)

