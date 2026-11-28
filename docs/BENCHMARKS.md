# Benchmark Metodolojisi (`perf_bench` ve `bench-compare`)

Bu doküman, `triM.` performans ölçümlerinin nasıl çalıştırıldığını, nasıl yorumlandığını ve ölçüm sınırlarını açıklar.

---

## 1. Nasıl Koşturulur?

### Temel Benchmark (`perf_bench`)
```bash
cargo run --release -p trim-media --example perf_bench --locked
```
Opsiyonel gerçek medya klibi varsa (`MVI_8603.MOV`), `perf_bench` otomatik olarak hem sentetik hem de gerçek H.264 1080p 23.976 fps akışını ölçer; gerçek klip yoksa yalnızca sentetik/mevcut senaryoları raporlar.

### Karşılaştırmalı Regresyon Kontrolü (`scripts/bench-compare`)
```bash
# 1. Mevcut çıktıyı temel çizgi olarak kaydet:
cargo run --release -p trim-media --example perf_bench --locked > /tmp/baseline.txt

# 2. Değişiklik sonrası karşılaştır (varsayılan tolerans %15):
bash scripts/bench-compare /tmp/baseline.txt
```
Paylaşımlı CI makinelerindeki CPU gürültüsü nedeniyle `bench-compare` CI'da zorunlu kapı olarak değil, yerel geliştirme ve faz doğrulama aracı olarak çalıştırılır.

---

## 2. Ölçülen Senaryolar

`perf_bench` şu temel metrikleri raporlar:
- **Seek / Scrub Gecikmesi**: Ardışık ileri/geri kare arama süreleri (`min`, `mean`, `p50`, `p95`, `max`, ms).
- **Ardışık Çözümleme (Sequential Decode)**: Paced ve tam hızlı FFmpeg pipe okuma verimi (fps).
- **Çok Katmanlı Kompozisyon (CPU Compositing)**: 6-track 1080p RGBA katman birleştirme süresi (kare başına ms).
- **4K ve Efektli Kompozisyon**: 3840×2160 çok katmanlı + harmanlama/opaklık kompozisyon süresi.
- **Bellek ve Önbellek**: `FrameBufferPool` yeniden kullanım oranı ve uzun oturum RSS kararlılığı (`crates/trim-media/tests/rss_leak_test.rs`).

---

## 3. Sonuçların Yorumlanması

- **p50 (Medyan)** tipik kullanıcı deneyimini, **p95 / max** ise önbellek ıskalaması veya GC/alloc sıçramalarını gösterir.
- Önce/sonra karşılaştırmaları mutlaka **aynı makinede, `--release --locked` profilinde ve aynı medya/çözünürlükte** yapılmalıdır.
- Güncel temel çizgi ve faz ölçüm sonuçları [docs/benchmarks/baseline.md](benchmarks/baseline.md) dosyasında tutulur.

---

## 4. Sınırlar

- **Donanım Hızlandırma (`-hwaccel`)**: Çalışma zamanında `probe_working_hwaccel()` (VAAPI/CUDA/Vulkan) ile test edilir; GPU olmayan başsız ortamlarda yalnızca yazılım (CPU) yolu ölçülebilir.
- **GPU OpenGL Sunumu**: `perf_bench` CPU kompozisyon ve FFmpeg pipe katmanını ölçer; canlı X11/Wayland VSync kare hızı masaüstü oturumunda ayrıca doğrulanmalıdır.
- **Termal ve Arka Plan Gürültüsü**: Dizüstü/paylaşımlı ortamlarda %5–10 arası dalgalanma normaldir; küçük farklar anlamlı iyileşme sayılmaz.
