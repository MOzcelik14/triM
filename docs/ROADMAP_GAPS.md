# triM. Yol Haritası Boşluk Raporu (Tarihsel Faz 0 Envanteri & Kalan Açıklar)

> [!NOTE]
> Özelliklerin güncel durumu için **tek yetkili kaynak** [FEATURE_MATRIX.md](FEATURE_MATRIX.md) belgesidir. Aşağıdaki Bölüm 2 maddelerinin bir kısmı sonraki fazlarda tamamlanmıştır ve güncel durumları parantez içinde belirtilmiştir.

---

## 1. Büyük Dosya Envanteri (> 800 Satır)

Toplam `crates/` altında 68.557 satır Rust kodu içinde 800 satırı aşan **27 dosya** tespit edildi:

| Satır | Dosya | Neden Büyük / Faz Planı |
|---:|---|---|
| 3368 | `crates/trim-media/src/video.rs` | PlaybackStreamer, SeekDecoder, VideoEngine ve yazılım efektleri tek dosyada |
| 3202 | `crates/trim-commands/src/commands.rs` | 25+ timeline komutu tek dosyada |
| 3126 | `crates/trim-export/src/filtergraph.rs` | Video/ses/efekt/maske/altyazı FFmpeg filtergraph üretimi tek dosyada |
| 2686 | `crates/trim-ui/src/canvas/render.rs` | Track/klip çizimi, dalga formu, keyframe, sürükleme ve sağ tık menüsü tek fonksiyonda (Faz 5 hedefi) |
| 1961 | `crates/trim-ui/src/tests.rs` | Tüm UI entegrasyon testleri tek dosyada |
| 1826 | `crates/trim-ui/src/inspector/color_card/mod.rs` | Primary, tekerlekler, eğriler, HSL, LUT, vinyet/keskinleştirme tek kartta (Faz 5 hedefi) |
| 1826 | `crates/trim-core/src/color.rs` | Renk preset'leri, LUT ayrıştırıcı, piksel matematiği ve testler |
| 1800 | `crates/trim-core/src/clip.rs` | Klip modeli, efekt yapılandırmaları ve testler |
| 1788 | `crates/trim-media/src/audio.rs` | PCM decode, mikser, reverse/speed DSP ve testler |
| 1423 | `crates/trim-ui/src/media_bin/mod.rs` | Grid/List görünümü, bin ağacı, sürükle-bırak ve efektler sekmesi (Faz 5 hedefi) |
| 1340 | `crates/trim-ui/src/monitor/mod.rs` | Source/Program monitör, transport, multicam ve metin/maske etkileşimi (Faz 5 hedefi) |
| 1303 | `crates/trim-commands/src/lib.rs` | Undo stack ve birim testleri |
| 1281 | `crates/trim-ui/src/app.rs` | `TrimApp` durumu, kısayol döngüsü ve ana `eframe::App` güncellemesi (Faz 5 hedefi) |
| 1267 | `crates/trim-render/src/lib.rs` | OpenGL/glow GPU compositor ve shader yönetimi |
| 1257 | `crates/trim-ui/src/app/timeline_panel.rs` | Zaman çizelgesi paneli, araç çubuğu ve canvas aksiyon dağıtıcısı (Faz 5 hedefi) |
| 1232 | `crates/trim-ui/src/app/dialogs_dispatch.rs` | Tüm modal diyalogların gönderim ve sonuç yönetimi |
| 1150 | `crates/trim-media/examples/perf_bench.rs` | Kapsamlı medya benchmark koşucusu |
| 1113 | `crates/trim-project/src/lib.rs` | Proje kaydet/yükle, bundle, autosave ve testler |
| 1109 | `crates/trim-ui/src/dialogs/export_dialog.rs` | Export ayarları, önayarlar ve render kuyruğu gönderimi |
| 1023 | `crates/trim-ui/src/scopes_panel.rs` | Parade, Waveform, Vectorscope ve Histogram çizimi |
| 1008 | `crates/trim-ui/src/monitor/overlays.rs` | Monitör üzeri metin/maske/güvenli alan çizimleri |
| 998 | `crates/trim-ui/src/dialogs/preferences_dialog.rs` | Çok sekmeli tercihler diyaloğu |
| 983 | `crates/trim-export/src/fcpxml.rs` | FCPXML içe/dışa aktarım |
| 944 | `crates/trim-ui/src/inspector.rs` | Inspector ana yönlendiricisi ve keyframe eğri popup'ı |
| 934 | `crates/trim-ui/src/app/media_actions.rs` | Medya içe aktarım, proxy, ses/sahne/transkripsiyon işleri |
| 917 | `crates/trim-media/src/analyzer.rs` | FFprobe medya çözümleyicisi |
| 893 | `crates/trim-core/src/timeline.rs` | Sekans modeli, snap ve çoklu track işlemleri |
| 860 | `crates/trim-ui/src/app/side_panels.rs` | Sol/sağ yan panel düzeni |

---

## 2. Kategori Bazlı Boşluklar (Öncelik Sırasıyla)

### A. Düzenleme İşlevselliği (Faz 1)
1. **Roll Trim ve Slide Trim (`Tamamlandı`):** `RollTrimCommand`, `SlideTrimCommand` ve export eşlik testleri eklendi (bkz. [FEATURE_MATRIX.md](FEATURE_MATRIX.md)).
2. **Medya Havuzu Subclip Desteği (`Tamamlandı`):** `Subclip` modeli ve serileştirme testleri eklendi (bkz. [FEATURE_MATRIX.md](FEATURE_MATRIX.md)).
3. **Timeline Keyframe Varsayılan Eğrisi & Çoklu-Keyframe Grafik (`Kısmi`):** `canvas/render.rs:1459` yeni eklenen keyframe'leri `Linear` olarak ekliyor; Inspector popup'ında eğri editörü var ancak zaman çizelgesi üzerinde veya genişletilmiş eğri panelinde çoklu keyframe eğrisi düzenlenemiyor.
4. **Multicam Timecode Senkronu (`Tamamlandı`):** `align_selected_clips_by_timecode` ve undo testleri eklendi (bkz. [FEATURE_MATRIX.md](FEATURE_MATRIX.md)).
5. **Canlı Önizlemede Stabilizasyon (`Kısmi`):** Export'ta `deshake` var, ancak önizlemede stabilizasyon uyarısı/önizleme yolu yok.

### B. Performans (Faz 2)
1. **Çok Katmanlı Kompozisyon:** 6 video track yazılım kompozisyonu şu an 640×360 cache-miss'te `15.80 ms/kare` (1080p için daha yüksek); profil (`perf`/flamegraph) alınıp 1080p hedefi ölçülmeli ve sıcak döngüler optimize edilmeli.
2. **Donanım Decode Algılama & Fallback (`Tamamlandı`):** `crates/trim-media/src/ffmpeg.rs` içinde `probe_working_hwaccel()` ve `streamer.rs` içinde otomatik CPU fallback bağlandı.
3. **Proxy / Önbellek Bellek Tavanı (`Kısmi`):** `rss_leak_test` ve byte-sınırlı LRU önbellek eklendi; yüksek çözünürlüklü kliplerde otomatik proxy kullanım eşiği hâlâ açık.
4. **Benchmark Regresyon Koruması (`Tamamlandı`):** `scripts/bench-compare` eklendi.

### C. UX ve Erişilebilirlik (Faz 3)
1. **Uzun İşlem İptal Tutarlılığı (`Tamamlandı`):** Export, Proxy ve arka plan `bg_task_cancel_flag` iptal desteği eklendi.
2. **Çift Dilli (TR/EN) Arayüz & Kısayol Özelleştirme (`Kısmi`):** Durum çubuğu (`status_message`) atamaları `i18n` katmanına taşındı ve 12 temel `ShortcutAction` için `KeymapConfig`/`ShortcutsDialog` bağlantısı kuruldu; ancak `inspector` (~197) ve `canvas` (~26) metinleri hâlâ sabit Türkçe ve J/K/L, M, ok tuşları vb. sabit kısayol olarak kalıyor.
3. **Erişilebilirlik (`accesskit` / HiDPI / Klavye Navigasyonu):** `egui` accesskit desteği değerlendirilip etkinleştirilmeli ve IME/ekran okuyucu sınırları belgelenmeli.
4. **Onboarding / Boş Proje Durumu:** İlk açılışta örnek proje veya hızlı başlangıç rehberi akışı cilalanmalı.

### D. Linux Entegrasyonu (Faz 4)
1. **Konteyner Kurulum Doğrulaması:** `.deb` (Ubuntu) ve `.rpm` (Fedora) paketlerinin temiz konteynerlerde kurulum + ikili çalıştırma doğrulaması ve `appstreamcli validate` CI adımı.
2. **Wayland / X11 / Portal / PipeWire Matrisi:** Sürükle-bırak, pano, HiDPI ve PipeWire ses yollarının test kapsamı ve dokümantasyonu.

### E. Mimari ve Bakım (Faz 5)
1. **800 Satır Üstü UI Dosyalarının Bölünmesi:** `canvas/render.rs` (2686), `color_card/mod.rs` (1826), `media_bin/mod.rs` (1423), `monitor/mod.rs` (1340), `app.rs` (1281), `timeline_panel.rs` (1257) modüllerinin ~800 satır altına bölünmesi.
2. **Mimari Sınır Denetimi:** `scripts/check-boundaries` ile `trim-ui`'ın çekirdek katmanları baypas etmediğinin CI'da doğrulanması.
3. **ADR, `CONTRIBUTING.md` ve Dependabot/Renovate:** `docs/adr/` karar kayıtları eklenmesi.

### F. Güvenilirlik ve Test (Faz 6)
1. **Fuzz / Property Testing:** Proje dosyası (`Project::from_json`) ve FFprobe JSON ayrıştırma için `proptest` / fuzz testleri.
2. **Uzun Oturum Bellek Sızıntısı Testi:** Uzun scrub/playback döngüsünde RSS artış sınırı ölçümü.
3. **Kapsam Eşiği & Süreç Hijyeni Linti:** `cargo llvm-cov` kurulumu + CI eşiği ve `ChildGuard`/`TrackedChildPool` dışı spawn denetimi.

### G. Doküman (Faz 7)
1. Kullanıcı kılavuzu, `cargo doc --no-deps` uyarısız rustdoc, benchmark metodoloji rehberi ve `README.md` özellik matrisi güncellemesi.
