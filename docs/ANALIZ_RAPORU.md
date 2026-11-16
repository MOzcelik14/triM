# triM. Kodbase Analiz Raporu (v2.0.0)

Bu rapor, **triM.** çalışma alanındaki tüm crate'lerin (`trim-core`, `trim-commands`, `trim-timeline`, `trim-project`, `trim-media`, `trim-export`, `trim-ui`, `trim-app`) mimari incelemesini, tespit edilen hata/performans/eşzamanlılık/FFmpeg/platform/kalite sorunlarını, gerçekte eksik olan profesyonel NLE özellik önerilerini ve önceliklendirilmiş uygulama yol haritasını içerir.

> **Doğrulama Komutları (16 Kasım 2026):**
> - `cargo check --workspace` — Başarılı (0 hata)
> - `cargo clippy --workspace --all-targets` — Başarılı (0 hata, 0 uyarı)
> - `cargo test --workspace` — Başarılı (tüm testler geçti: `trim-core` 68 + 1 entegrasyon, `trim-export` 28, `trim-media` 45, `trim-project` 6, `trim-timeline` 2, `trim-ui` 49)

---

## Mimari Özeti

### Crate Sorumlulukları ve Bağımlılıklar

| Crate | Bağımlılıklar | Temel Sorumluluklar |
|---|---|---|
| `trim-core` | `serde`, `serde_json`, `uuid`, `thiserror`, `log` | Temel alan modelleri (`Project`, `TimelineModel`, `Track`, `Clip`, `MediaItem`, `Marker`, `Sequence`), zaman/timecode matematiği (`time`), keyframe/Bezier/speed-ramp integrasyonu (`keyframe`), maske geometrisi (`mask`), renk/LUT/curve/qualifier matematiği (`color`), scope hesapları (`scopes`), birleştirme (`compositing`), SRT/VTT altyazı (`subtitles`), YouTube/MP4 chapter ve EDL/FCPXML veri üretimi (`interchange`), `LruCache`, yol/tercih yardımcıları (`paths`, `preferences`). |
| `trim-commands` | `trim-core`, `uuid`, `thiserror`, `log` | `Command` trait'i, `UndoStack`, `CommandError` (`src/lib.rs`) ve 30+ geri alınabilir kurgu/inspector komutu (`src/commands.rs`: kesme, kırpma, ripple, taşıma, renk, hız, keyframe, otomatik sessizlik kesimi, makro komut). |
| `trim-timeline` | `trim-core`, `trim-commands` *(kullanılmayan: `uuid`, `thiserror`, `log`)* | Oynatma kafası (playhead) tabanlı düzenleme noktası bulma (`find_prev_edit_point`, `find_next_edit_point`) ve `split_at_playhead`, `ripple_trim_head`, `ripple_trim_tail` yüksek seviye akışları. |
| `trim-project` | `trim-core`, `serde`, `serde_json`, `thiserror`, `log` | Atomik `.trim` proje kaydetme/yükleme (`save_project`, `load_project`), taşınabilir paketleme (`bundle_project`), otomatik kayıt ve çökme kurtarma (`AutosaveManager`). |
| `trim-media` | `trim-core`, `serde`, `serde_json`, `image`, `thiserror`, `log`, `uuid` | FFmpeg/FFprobe/FFplay süreç yönetimi ve donanım kodlayıcı tespiti (`ffmpeg.rs`), medya/atom analizi (`analyzer.rs`), `VideoEngine` / `PlaybackStreamer` / `AsyncFrameDecoder` / CPU kompozitör / hareket takibi (`video.rs`), `AudioMixer` + 6-bant EQ + Compressor/De-Esser + `LookaheadLimiter` + `AudioStreamPlayer` (`audio.rs`, `dsp/limiter.rs`), dalga formu/kare önbelleği (`waveform.rs`, `thumbnail.rs`), proxy, sahne/sessizlik algılama, Whisper deşifre, ekran ve ses kaydı. |
| `trim-export` | `trim-core`, `trim-media`, `serde`, `serde_json`, `uuid` *(kullanılmayan: `serde_json`, `thiserror`)* | `TimelineExporter` ile FFmpeg `-filter_complex` üretim hattı (`filtergraph.rs`), arka plan dışa aktarma yürütücüsü (`runner.rs`), sıralı render kuyruğu (`queue.rs`), hazır dışa aktarma profilleri (`presets.rs`), CMX 3600 EDL & FCPXML içe/dışa aktarımı (`edl.rs`, `fcpxml.rs`). |
| `trim-ui` | Tüm alt crate'ler + `egui`, `eframe`, `rfd`, `image`, `uuid`, `log` | `eframe::App` masaüstü arayüzü (`TrimApp`), OpenGL FBO tabanlı `GpuCompositor` (`gpu_compositor.rs`), Timeline Canvas, Source/Program Monitor, Inspector, Media Bin, Scopes Panel, VU Meter, kısayol profilleri, TR/EN yerelleştirme ve 14 modal diyalog. |
| `trim-app` | Tüm workspace crate'leri + `egui`, `eframe`, `env_logger`, `log`, `image`, Windows'ta `winres` | Masaüstü ikili giriş noktası (`trim`), CLI bayrakları (`--no-splash`, `--open-export`, vb.), pencere ikonu ve `eframe::run_native` başlatıcısı. |

### Veri Akışı (`Proje Modeli → Timeline → Decode → Render → Export`)

```text
[trim-project] ──yükle/kurtar──▶ [trim-core: Project / TimelineModel / Track / Clip]
                                           ▲                        │
                                    Command::execute/undo           │ get_flattened_timeline()
                                           │                        ▼
[trim-ui: TrimApp / Panels] ──▶ [trim-commands: UndoStack]   [trim-media]
         │                                                      ├── VideoEngine (PlaybackStreamer / AsyncFrameDecoder / LRU 300)
         │                                                      └── AudioMixer (48kHz PCM LRU 512MB + Biquad EQ + Limiter)
         ▼                                                                  │
[trim-ui: playback.rs] ◀────────── CompositorLayer / RgbaImage ─────────────┘
         │
         ├──▶ GpuCompositor (OpenGL FBO) ──▶ monitor_gpu_texture_id (+ readback -> ScopesPanel)
         └──▶ (Fallback) CPU RgbaImage   ──▶ egui::TextureHandle
         
[trim-ui: ExportDialog] ──▶ [trim-export: RenderQueue / ExportRunner]
                                  └── TimelineExporter::build_export_pipeline_range ──▶ ffmpeg (-filter_complex / -progress pipe:1)
```

---

## Sorunlar

### A. Hatalar ve Mantık Sorunları

1. **Boş timeline'da Play sonsuz zaman sayacı**
   - **Konum:** `crates/trim-ui/src/app/playback.rs:140-145`
   - **Açıklama:** Oynatma durdurma koşulu `if dur > 0.0 && (cur >= dur || cur < 0.0)` şeklinde yazıldığı için `dur == 0.0` (boş timeline) durumunda hiçbir zaman tetiklenmiyor. Space ile oynatma başlatıldığında `current_time` sonsuza kadar ilerliyor.
   - **Etkisi:** Boş projede oynatma kafası timeline dışına kaçıyor, süre sayacı anlamsız şekilde büyüyor.
   - **Önerilen Çözüm:** `dur <= 0.0` ise oynatmayı başlatmamak veya anında `is_playing = false; current_time = 0.0;` yapmak.
   - **Önem / Efor:** Yüksek / S

2. **Undo/Redo sonrası video/ses önbelleği ve monitör karesi temizlenmiyor**
   - **Konum:** `crates/trim-ui/src/app.rs:1324-1349, 1533-1563` ve `crates/trim-ui/src/app/playback.rs:244-247`
   - **Açıklama:** Klavye ve menü üzerinden yapılan `undo`/`redo` sonrasında `video_engine.clear_cache()`, `audio_mixer.clear_cache()` ve `last_rendered_monitor_frame = None` çağrılmıyor (`paste_clipboard_clips` içinde ise çağrılıyor).
   - **Etkisi:** Kullanıcı renk, kırpma veya klip değişikliğini geri aldığında önizleme monitörü eski kareyi/sesi göstermeye devam edebiliyor.
   - **Önerilen Çözüm:** `undo` ve `redo` başarıyla tamamlandığında merkezi bir `invalidate_playback_caches()` yardımcısı çağırmak.
   - **Önem / Efor:** Yüksek / S

3. **`Clip::trim_in` ve `Clip::trim_out` keyframe, hız rampası ve ters oynatmada hatalı**
   - **Konum:** `crates/trim-core/src/clip.rs:1169-1180`
   - **Açıklama:** Doğrusal `dt * self.speed` kullanılıyor (`map_timeline_to_source` yerine); klip başından kırpıldığında klip-göreli `keyframes` ve `masks[*].keyframes` kaydırılmıyor; `reverse == true` iken `trim_in` yanlışlıkla `source_in`'i ileri sarıyor, `trim_out` ise `source_out`'u bozuyor.
   - **Etkisi:** Ters oynatılan, hız rampalı veya animasyonlu kliplerde kırpma sonrası kareler ve animasyon zamanlaması kayar.
   - **Önerilen Çözüm:** `trim_in`/`trim_out` içinde `map_timeline_to_source` kullanmak, `reverse` için `source_out` / `source_in` yönünü ayırmak ve keyframe zamanlarını `dt` kadar kaydırmak.
   - **Önem / Efor:** Yüksek / M

4. **`SplitClipCommand` ve `BatchSplitClipCommand` maske keyframe'lerini ve N-yollu bağlı klipleri atlıyor**
   - **Konum:** `crates/trim-commands/src/commands.rs:337-395, 434-470, 584-662`
   - **Açıklama:** `split_single_clip_core` klip `keyframes`'ini bölüyor ancak `masks[*].keyframes`'i bölmüyor/kaydırmıyor. `SplitClipCommand::execute` ilk bağlı klibi bulunca `break` yapıyor (3+ bağlı klip varsa diğerleri eski `linked_group_id` ile kopuyor). `BatchSplitClipCommand` ise hiçbir keyframe'i veya bağlı klibi bölmüyor. Ayrıca `SplitClipCommand` kendi içinde `track.locked` kontrolü yapmıyor.
   - **Etkisi:** Çoklu ses/video bağlı gruplarda ve maskeli kliplerde bölme sonrası senkron/animasyon bozulması.
   - **Önerilen Çözüm:** `split_single_clip_core` içinde maske keyframe'lerini de `split_offset` noktasından bölmek, `linked_group_id` eşleşen tüm klipleri toplamak ve kilitli kanalları reddetmek.
   - **Önem / Efor:** Yüksek / M

5. **`AutoSilenceCutCommand` fade/geçişleri kopyalıyor ve `undo` yanlış klipleri kaydırabiliyor**
   - **Konum:** `crates/trim-commands/src/commands.rs:1533-1608`
   - **Açıklama:** Oluşturulan her alt klibe orijinal `fade_in`, `fade_out`, `transition_*` ve kaydırılmamış `keyframes` kopyalanıyor. Diğer kanallar kaydırılmıyor ve `undo` kaydedilmiş klip ID'leri yerine `timeline_in >= shifted_boundary - 1e-4` koşuluyla eşleşen her klibi kaydırıyor; ayrıca redo'da yeni UUID üretiliyor.
   - **Etkisi:** Otomatik sessizlik kesiminden sonra istenmeyen geçiş tekrarları, keyframe kayması ve geri almada timeline bozulması.
   - **Önerilen Çözüm:** Fade/transition'ları yalnızca ilk/son parçaya koymak, `execute` sırasında kaydırılan `(track_id, clip_id)` ve üretilen UUID'leri saklayıp `undo`/`redo`'da deterministik kullanmak.
   - **Önem / Efor:** Yüksek / M

6. **`RippleDeleteCommand` yalnızca 1 bağlı klibi siliyor ve diğer kanallarda çakışma yaratabiliyor**
   - **Konum:** `crates/trim-commands/src/commands.rs:734-762`
   - **Açıklama:** İlk bağlı klibi bulduğunda `break;` yaptığı için 3+ bağlı klipten gerisi kalıyor. Tüm kilitli olmayan kanallarda `timeline_in >= clip.timeline_out - 1e-4` olan klipleri `dur` kadar sola çekmek, diğer kanallardaki daha erken kliplerin üstüne binmeye (overlap) yol açabiliyor.
   - **Etkisi:** Çoklu bağlı gruplarda artık klip kalması ve üst/alt kanallarda klip çakışması.
   - **Önerilen Çözüm:** Tüm `linked_group_id` eşleşmelerini silmek ve kaydırma öncesi diğer kanallardaki minimum boşluk (`headroom`) kontrolünü uygulamak.
   - **Önem / Efor:** Yüksek / M

7. **Klip başı DSP durumu ve reverb/echo tamponu her ses parçasında sıfırlanıyor**
   - **Konum:** `crates/trim-media/src/audio.rs:292-377`
   - **Açıklama:** `env_level`, `hp_prev_in/out`, `eq_state`, `comp_env`, `deess_prev` ve `delay_buf` (reverb/echo) her `mix_raw_window` çağrısında yerel değişken olarak sıfırdan oluşturuluyor.
   - **Etkisi:** Her 20-50ms ses bloğu sınırında çıtırtı/tık sesi oluşuyor, compressor zarfı sürekli resetleniyor ve reverb/echo kuyruğu kesiliyor.
   - **Önerilen Çözüm:** `AudioMixer` içinde `(clip_id)` anahtarlı kalıcı bir `ClipDspState` haritası tutup seek/pause dışında durumu korumak.
   - **Önem / Efor:** Yüksek / M

8. **`PlaybackStreamer` `clip.speed` ve hız rampalarını yok sayıyor (A/V kayması ve sürekli yeniden başlatma)**
   - **Konum:** `crates/trim-media/src/video.rs:242, 324, 159`
   - **Açıklama:** `PlaybackStreamer` kare PTS değerini `1.0x` hız varsayarak (`source_start_time + frame_idx / fps`) hesaplıyor; `render_timeline_frame` ise `map_timeline_to_source` ile hızlandırılmış/rampalı zaman istiyor. Fark `0.6s`'yi aştığında akış sürekli baştan başlatılıyor.
   - **Etkisi:** Hızlandırılmış veya speed-ramp uygulanmış kliplerde oynatma sırasında takılma ve ses/görüntü senkron kayması.
   - **Önerilen Çözüm:** `PlaybackStreamer` başlatılırken hız bilgisini veya FFmpeg `setpts` filtresini/PTS ölçeklemesini akışa dahil etmek.
   - **Önem / Efor:** Yüksek / M

9. **Çoklu video kanalında aynı anda 2+ klip oynatılırken üst kanala alt kanal karesi çiziliyor**
   - **Konum:** `crates/trim-media/src/video.rs:942-967, 1005, 1286-1314, 1345`
   - **Açıklama:** `VideoEngine` yalnızca tek bir `PlaybackStreamer` ve tek bir `last_raw_source_frame` tutuyor. İkinci ve sonraki eşzamanlı video klipleri önbellekte yoksa `last_raw_source_frame` geri dönülüyor.
   - **Etkisi:** Picture-in-Picture veya çok kanallı kurguda oynatma sırasında üstteki klip yerine alttaki klibin karesi görünebiliyor.
   - **Önerilen Çözüm:** `VideoEngine` içinde aktif klip başına `HashMap<String, PlaybackStreamer>` (en fazla aktif katman sayısı kadar) yönetmek.
   - **Önem / Efor:** Yüksek / M

10. **Keyframe-duyarsız `-ss` ve kesirli FPS timecode kayması**
    - **Konum:** `crates/trim-media/src/video.rs:48-51, 204-207`, `crates/trim-core/src/time.rs:4-53`
    - **Açıklama:** `-i` öncesi `-ss` uzun-GOP videolarda en yakın önceki keyframe'e atlar ama ilk kare `source_start_time` olarak etiketlenir. `seconds_to_timecode` ise 29.97fps gibi kesirli hızlarda `total_frames / fps.round()` yaptığı için gerçek saatle kayma üretir; ayrıca `mm/ss/ff` sınır doğrulaması yoktur.
    - **Önem / Efor:** Orta / M

11. **Undo/Redo kapsamı dışında kalan mutasyonlar & eksik command alanları**
    - **Konum:** `crates/trim-ui/src/app/timeline_actions.rs:616-646`, `crates/trim-ui/src/app/project_io.rs:203-214`, `crates/trim-ui/src/app.rs:4549-4554`, `crates/trim-commands/src/commands.rs:2570-2627`, `crates/trim-commands/src/lib.rs:30-69`
    - **Açıklama:** `paste_clipboard_clips` yeni eklenen kanalları Undo dışında bırakıyor; `import_subtitles_from_path` tamamen `undo_stack` dışı çalışıyor; Whisper deşifre eklemeleri tek `MacroCommand` değil; `SetClipColorGradingCommand` `halation`, `bloom`, `chromatic_aberration`, `custom_curves`, `hsl_curves`, `hsl_qualifier` alanlarını kapsamıyor; `UndoStack` hata durumunda komutu düşürüyor ve üst sınırı yok.
    - **Önem / Efor:** Orta / S-M

12. **`AutosaveManager` kararsız `DefaultHasher` kullanıyor ve `bundle_project` proxy/LUT dosyalarını atlıyor**
    - **Konum:** `crates/trim-project/src/lib.rs:123-174, 204-209, 224`
    - **Açıklama:** `DefaultHasher` Rust sürümüne göre değişebilir (güncelleme sonrası eski kurtarma dosyası eşleşmeyebilir). `bundle_project` ise `proxy_path` ve `custom_lut_path` dosyalarını pakete kopyalamıyor ve dosya adında `project.settings.name` temizlenmeden (`/` veya `..`) kullanılıyor.
    - **Önem / Efor:** Yüksek / M

13. **`mask_cache` keyframe değer değişikliklerini hashlemiyor ve `ScreenRecordDialog` `X` ile kapatılınca kayıt yetim kalıyor**
    - **Konum:** `crates/trim-ui/src/gpu_compositor.rs:1434`, `crates/trim-ui/src/dialogs/screen_record_dialog.rs:199-200`
    - **Açıklama:** `mask_cache` anahtarı yalnızca `keyframes.len()` hashlediği için duraklatılmış kafada keyframe değeri değişince eski maske kalır. Kayıt penceresi `X` ile kapatılınca `self.close()` çağrılmadığı için kayıt süreci arka planda devam eder.
    - **Önem / Efor:** Orta / S

14. **Diğer mantık hataları (`RippleTrim*`, `CloseGapCommand`, `AudioMixer` clamping, `scene_detect`, `transcribe`, `flattened_sequences`)**
    - **Konum:** `crates/trim-commands/src/commands.rs:1057, 1231, 1780-1795`, `crates/trim-media/src/audio.rs:267-290`, `crates/trim-media/src/scene_detect.rs:45-61`, `crates/trim-media/src/silence_detect.rs:55-69`, `crates/trim-media/src/transcribe.rs:92-96`, `crates/trim-core/src/sequence.rs:43-45`
    - **Açıklama:** `RippleTrimHead/Tail` sınır toleransı `trim-timeline` ile uyumsuz; `CloseGapCommand::undo` tüm sonraki klipleri kaydırabiliyor; `AudioMixer` PCM sonunu aşan kliplerde sesi zamana yayarak uzatıyor; sahne/sessizlik algılama FFmpeg çıkış kodunu kontrol etmiyor; Whisper çıktısı paylaşımlı sabit temp klasörüne yazılıyor; döngüsel nested sequence çözülmemiş klip bırakabiliyor.
    - **Önem / Efor:** Orta / S-M

---

### B. Çökme Riskleri (Panic / Unwrap / Sınır Aşımı)

1. **6'dan fazla parametrik EQ bandı etkinse dizi sınırı aşımı paniği**
   - **Konum:** `crates/trim-media/src/audio.rs:297-318`
   - **Açıklama:** `eq_coeffs` ve `eq_state` sabit `[..; 6]` boyutunda, ancak `for band in &peq.bands` döngüsü `active_eq_count < 6` kontrolü yapmıyor. Release profilinde `panic = "abort"` (`Cargo.toml:38`) olduğundan uygulama anında kapanır.
   - **Önem / Efor:** Yüksek / S

2. **Non-UTF-8 dosya yollarında `unwrap()` paniği**
   - **Konum:** `crates/trim-ui/src/app/timeline_actions.rs:66`, `crates/trim-ui/src/app/media_actions.rs:46`
   - **Açıklama:** `out_png.to_str().unwrap()` ve `out.file_name().unwrap()` Linux'ta non-UTF-8 yollarda uygulamayı çökertir. (`audio.rs:740`, `video.rs:41` ise `unwrap_or("")` ile sessizce başarısız olur.)
   - **Önem / Efor:** Orta / S

3. **EDL dışa aktarmada UTF-8 byte dilimleme paniği**
   - **Konum:** `crates/trim-export/src/edl.rs:108`
   - **Açıklama:** `&clip.id[..clip.id.len().min(8)]` çok baytlı UTF-8 karakter içeren elle düzenlenmiş klip ID'lerinde karakter sınırı ortasından kesip panik oluşturabilir.
   - **Önem / Efor:** Orta / S

4. **Fullscreen monitörde `< 56px` yükseklikte sıfıra bölme / negatif boyut**
   - **Konum:** `crates/trim-ui/src/app/playback.rs:453-462`
   - **Açıklama:** Pencere aşırı küçültüldüğünde `monitor_bounds.height() <= 0.0` olursa `w / h` bölmesi `inf` veya negatif boyut üretir.
   - **Önem / Efor:** Orta / S

5. **`rasterize_masks_rgba` taşma riski ve `ChildGuard` `deref` paniği**
   - **Konum:** `crates/trim-core/src/mask.rs:447`, `crates/trim-media/src/ffmpeg.rs:240-255`
   - **Açıklama:** `w * h * 4` taşma kontrolsüz (`checked_mul` yok); `ChildGuard::deref` tüketilmiş süreçte `.expect(...)` ile panikler.
   - **Önem / Efor:** Düşük / S

---

### C. Performans

1. **Timeline Canvas'ta her karede senkron `Path::exists()` sistem çağrısı ve viewport culling eksikliği**
   - **Konum:** `crates/trim-ui/src/canvas.rs:274, 386-550`
   - **Açıklama:** Yatay görünür alan (`clip_rect`) dışında kalan klipler atlanmıyor. Her karede her klip için `Path::new(&m.file_path).exists()` çağrısı (senkron `stat()` syscall) yapılıyor; ayrıca `HashSet` ve `format!("strip:...")` tahsisleri tekrarlanıyor.
   - **Etkisi:** Büyük projelerde (200+ klip) veya ağ diskinde UI kare hızı ciddi düşer ve takılmalar yaşanır.
   - **Önerilen Çözüm:** `clip_rect` ile yatay görünür klip filtrelemesi eklemek ve `offline_media_ids` durumunu yalnızca proje yükleme/relink/dosya kontrolü sırasında önbelleklemek.
   - **Önem / Efor:** Kritik / M

2. **`.cube` 3D LUT dosyası CPU önizlemede her karede diskten okunup parse ediliyor**
   - **Konum:** `crates/trim-media/src/video.rs:1505-1509`
   - **Açıklama:** Özel `.cube` LUT takılı kliplerde `parse_cube_lut_file` her karede disk I/O + 64³ float parse yapıyor.
   - **Önerilen Çözüm:** `VideoEngine` içinde `(path, mtime)` anahtarlı bir `Arc<CubeLut3D>` LRU önbelleği tutmak.
   - **Önem / Efor:** Yüksek / S

3. **GPU Compositor içinde CPU piksel döngüsü, önbelleksiz uniform sorguları ve gereksiz maske yüklemeleri**
   - **Konum:** `crates/trim-ui/src/gpu_compositor.rs:1220-1244, 1449-1480, 1609-1737`
   - **Açıklama:** `custom_curves`, `hsl_curves` veya `hsl_qualifier` açıkken frame UI thread'de klonlanıp CPU'da piksel piksel işleniyor; her katmanda 45+ GL uniform konumu string ile yeniden aranıyor; `needs_raster == false` olsa bile maske GPU'ya tekrar yükleniyor.
   - **Önem / Efor:** Yüksek / S-M

4. **Scopes açıkken senkron `gl.read_pixels` + çift RGBA klonlama + UI thread scope üretimi**
   - **Konum:** `crates/trim-ui/src/gpu_compositor.rs:1778-1793`, `crates/trim-ui/src/app/playback.rs:204-211, 242`, `crates/trim-ui/src/scopes_panel.rs:109-147`
   - **Açıklama:** Her zaman değişiminde GPU pipeline `read_pixels` ile durduruluyor, RGBA verisi hem `scopes_panel` hem `app` içine klonlanıyor ve 4-6 scope dokusu UI thread'de üretiliyor.
   - **Önem / Efor:** Yüksek / M

5. **Scrubbing sırasında UI thread'de senkron `decode_frame_raw` ve sınırsız önbellekler**
   - **Konum:** `crates/trim-media/src/video.rs:39-77, 675, 802-813, 1364`, `crates/trim-ui/src/app.rs:3419`, `crates/trim-ui/src/gpu_compositor.rs:846, 851`
   - **Açıklama:** `last_raw_source_frame` boşken UI thread senkron `ffmpeg` başlatıyor; `VideoEngine::image_cache`, `VideoFrameDecoder::image_cache`, `GpuCompositor::lut_cache`/`mask_cache` ve `media_bin_textures` (`strip:*` silinmiyor) sınırsız büyüyor.
   - **Önem / Efor:** Yüksek / M-L

6. **Tüm ses dosyasını sıkıştırılmamış PCM olarak RAM'e açma (`decode_audio_to_pcm`) ve `interpolate_keyframes` tahsisleri**
   - **Konum:** `crates/trim-media/src/audio.rs:739-803`, `crates/trim-core/src/keyframe.rs:186-191, 236-242`
   - **Açıklama:** 1 saatlik video ~691 MB RAM kaplayıp 512 MB `MAX_PCM_CACHE_BYTES` sınırını aşarak sürekli yeniden decode tetikliyor. `integrate_keyframes` ise tek çağrıda `interpolate_keyframes`'i 601 kez çağırıp her seferinde `Vec<Keyframe>` klonlayıp sıralıyor.
   - **Önem / Efor:** Yüksek / M-L

---

### D. Eşzamanlılık

1. **`AudioMixer::preload_audio_async` sınırsız thread/process açıyor ve `get_audio_pcm` oynatma/UI thread'ini `Condvar::wait` ile blokluyor**
   - **Konum:** `crates/trim-media/src/audio.rs:85-163`
   - **Açıklama:** 50 medya içe aktarılınca 50 eşzamanlı `ffmpeg` decode süreci başlatılıyor. Oynatma sırasında önbellekte olmayan ses için `cvar.wait` ile bekleniyor, hata dönerse caller thread'de senkron tekrar decode ediliyor.
   - **Önem / Efor:** Kritik / M

2. **Whisper deşifre, Akıllı Sahne Kesimi, Sessizlik Algılama ve Audio Sync UI thread'de senkron ve iptal edilemez**
   - **Konum:** `crates/trim-ui/src/app.rs:4510-4581`, `crates/trim-ui/src/app/media_actions.rs:427, 530`, `crates/trim-ui/src/app/timeline_actions.rs:777`
   - **Açıklama:** Uzun videolarda bu işlemler UI thread'ini 30-120+ saniye dondurur ("Yanıt Vermiyor") ve iptal butonu yoktur.
   - **Önem / Efor:** Kritik / M

3. **Medya içe aktarma, proje açma (`ffprobe`) ve Source Monitor resim açma UI thread'de senkron**
   - **Konum:** `crates/trim-ui/src/app/media_actions.rs:135, 194-196`, `crates/trim-ui/src/app/project_io.rs:43-52`, `crates/trim-ui/src/app/playback.rs:396-398`
   - **Açıklama:** Toplu dosya/klasör içe aktarmada her dosya için UI thread'de `ffprobe` çalıştırılıyor; 4K/8K resimler UI thread'de açılıyor.
   - **Önem / Efor:** Yüksek / M

4. **Eşzamanlı Proxy oluşturmada alıcı kanalın üzerine yazılması**
   - **Konum:** `crates/trim-ui/src/app/media_actions.rs:51-54`, `crates/trim-ui/src/app.rs:970-985`
   - **Açıklama:** İkinci proxy başlatıldığında `self.proxy_rx = Some(rx)` önceki kanalı ezer; ilk medya sonsuza kadar "Proxy oluşturuluyor" durumunda takılı kalır.
   - **Önem / Efor:** Orta / S

5. **`PlaybackStreamer` ham `Child` yönetimi ve thread yarışları**
   - **Konum:** `crates/trim-media/src/video.rs:218-233, 298-313, 423-443`
   - **Açıklama:** `ChildGuard` yerine ham `Child` kullanılıyor; `stdout.take()` `None` ise `wait()` yapılmadan zombi bırakılabiliyor ve `stop()` worker thread'i join etmiyor.
   - **Önem / Efor:** Orta / S

---

### E. FFmpeg Entegrasyonu ve Export Hattı

1. **VAAPI ve ProRes 422 export profilleri bozuk**
   - **Konum:** `crates/trim-export/src/filtergraph.rs:1278, 1322-1323, 1346-1353, 1416-1428`
   - **Açıklama:** `HardwareAccel::Vaapi` `-vaapi_device` ve `format=nv12,hwupload` eklemeden `yuv420p` veriyor. `Apple ProRes 422` (`prores_ks`) ise desteklemediği `-pix_fmt yuv420p` bayrağını zorluyor (`yuv422p10le` gerekli).
   - **Önem / Efor:** Yüksek / S-M

2. **Export iptal yarışı ve durdurulamayan/yetim `ffmpeg` süreci**
   - **Konum:** `crates/trim-export/src/runner.rs:50-51, 104-139`
   - **Açıklama:** İptal bayrağı yalnızca `reader.lines()` içinde kontrol edildiği için FFmpeg çıktı vermeden takılırsa `child.kill()` çağrılmaz; uygulama kapatılırsa süreç yetim kalır; aynı runner'da hızlı iptal + yeniden başlatma `is_cancelled` bayrağını yarışa sokar.
   - **Önem / Efor:** Yüksek / M

3. **Preview ↔ Export Özellik Paritesi Eksikleri**
   - **Konum:** `crates/trim-export/src/filtergraph.rs:345-363, 799-801, 1073, 1211` vs `crates/trim-core/src/clip.rs:673, 719-743`
   - **Açıklama:** Önizlemede çalışan şu özellikler FFmpeg dışa aktarımında eksik veya eksik animasyonlu:
     - `rotation` (ve rotation keyframe'leri) export edilmiyor.
     - `track_matte_above` export edilmiyor.
     - `hsl_curves` ve `hsl_qualifier` export edilmiyor.
     - `speed` keyframe'leri video `setpts`'de uygulanıyor ama ses `atempo` yalnızca sabit `clip.speed`'i kullanıyor (A/V senkron kayması).
     - `scale` keyframe'leri ve maske keyframe'leri (`t=0.0` dondurulmuş) export'ta animasyonsuz kalıyor.
     - Range export (`build_export_pipeline_range:963-965`) `reverse == true` kliplerde yanlış kaynak aralığını dilimliyor.
     - Video kanalındaki bölünmemiş ses (`filtergraph.rs:1160`) sessizce atlanıyor.
   - **Önem / Efor:** Yüksek / M-L

4. **Windows 32KB komut satırı limiti ve geçici `.pgm` maske dosyası sızıntısı**
   - **Konum:** `crates/trim-export/src/filtergraph.rs:494-515, 1075-1105, 1309-1310`
   - **Açıklama:** Karmaşık timeline'larda veya harf-harf `Typewriter` yazılarında inline `-filter_complex` Windows `CreateProcess` 32KB sınırını aşabilir (`-filter_complex_script` kullanılmalı). Üretilen `trim_mask_*.pgm` dosyaları export sonrası silinmiyor ve `clip.id` ile çakışabiliyor.
   - **Önem / Efor:** Yüksek / S

---

### F. Platform Farkları (Linux vs Windows)

1. **Eksik FFmpeg uyarısı Linux'ta Windows komutları gösteriyor; Türkçe `Videolar` klasörü atlanıyor**
   - **Konum:** `crates/trim-ui/src/app.rs:4998-5027`, `crates/trim-ui/src/dialogs/export_dialog.rs:49-66`
   - **Açıklama:** FFmpeg eksik penceresi her işletim sisteminde `winget` / `ffmpeg.exe` / `install.ps1` talimatları veriyor. Varsayılan dışa aktarma yolu ise `Videos`, `Masaüstü`, `Desktop` arıyor ancak Türkçe Linux'taki `Videolar` XDG klasörünü kontrol etmiyor.
   - **Önem / Efor:** Orta / S

2. **Ekran/Ses kaydı aygıt adları Wayland ve yerelleştirilmiş Windows'ta başarısız olabilir** *(kısmen doğrulanmadı — canlı donanım/Wayland oturumunda test edilmedi)*
   - **Konum:** `crates/trim-media/src/screen_record.rs:57-98`, `crates/trim-media/src/voiceover.rs:26-100`
   - **Açıklama:** Linux'ta `x11grab` ve `/dev/video0` sabitlenmiş (saf Wayland'de çalışmaz); Windows'ta `dshow` için `"audio=default"` / `"Microphone"` sabitlenmiş (Türkçe Windows'ta `"Mikrofon"` veya aygıt listeleme gerekir).
   - **Önem / Efor:** Orta / M

3. **FCPXML `file://` URI kodlaması Windows yollarında ve özel karakterlerde hatalı**
   - **Konum:** `crates/trim-core/src/interchange.rs:198`, `crates/trim-export/src/fcpxml.rs:20-27, 285-311`
   - **Açıklama:** `file://C:\...` üretiliyor ve boşluk/`#`/`?` percent-encode edilmiyor; `decode_file_url` `/C:/...` başında `/` bırakıyor.
   - **Önem / Efor:** Orta / S

---

### G. Kullanılabilirlik, Kısayollar ve i18n

1. **i18n (TR/EN) kapsama boşluğu**
   - **Konum:** `crates/trim-ui/src/i18n.rs:49-254`, `inspector.rs`, `canvas.rs`, `monitor.rs`, `scopes_panel.rs`, `media_bin.rs`, `dialogs/*`, `trim-core/*`
   - **Açıklama:** `i18n.rs` içindeki `inspector_*`, `tab_*`, `tool_*`, `dialog_*` anahtarları neredeyse hiç kullanılmıyor; UI panellerinin ve diyalogların büyük çoğunluğu ile `trim-core` varsayılan isimleri/YouTube chapter başlıkları (`"Giriş"`, `"Bölüm {}"`) sabit Türkçe.
   - **Önem / Efor:** Yüksek / M

2. **Kısayol yardım penceresi ile gerçek kısayollar çelişiyor**
   - **Konum:** `crates/trim-ui/src/dialogs/shortcuts_dialog.rs:87, 99, 109, 118` vs `crates/trim-ui/src/app.rs:1124-1162, 1324-1349`
   - **Açıklama:** `Ctrl+R` tüm profillerde hız diyaloğu olarak yazılmış ama `Key::R` bağlaması yok; DaVinci profilinde `T` "Trim Mode" yazıyor ama `EditTool::Type` açılıyor; DaVinci profilinde `A` `Select` seçiyor ama durum çubuğunda `"İz Seçme Aracı (A)"` yazıyor.
   - **Önem / Efor:** Orta / S

3. **Arka plan hata geri bildirimi & erişilebilirlik**
   - **Konum:** `crates/trim-ui/src/app.rs:147`
   - **Açıklama:** Proxy/export/save hataları yalnızca tek satırlık `status_message` alanında görünüp bir sonraki işlemde eziliyor (kalıcı bildirim/toast geçmişi ve `accesskit` etiketleri yok).
   - **Önem / Efor:** Düşük / M

---

### H. Kod Kalitesi, Bağımlılıklar ve Test Boşlukları

- **Kullanılmayan Bağımlılıklar:** `trim-timeline/Cargo.toml:11-13` (`uuid`, `thiserror`, `log`), `trim-export/Cargo.toml:12-13` (`serde_json`, `thiserror`). — **Düşük / S**
- **Bakımı Durmuş Build Bağımlılığı:** `trim-app/Cargo.toml:27` (`winres = "0.1"` yerine `winresource` önerilir). Workspace genelindeki tüm crate lisansları (`MIT` / `Apache-2.0`) `GPL-3.0-or-later` ile uyumludur. — **Düşük / S**
- **Aşırı Büyük Dosyalar & Tekrarlar:** `commands.rs` (2.830 satır), `inspector.rs` (3.200+ satır), `app.rs` (5.068 satır); `trim-timeline/src/lib.rs` içinde klip arama bloğu 3 kez tekrarlanmış; `mask.rs` içinde SDF mantığı iki yerde yazılmış. — **Düşük / M**
- **Test Eksikleri:** `runner.rs` (0 test), `presets.rs` (0 test), `trim-timeline` (`ripple_trim_head/tail` 0 test), `trim-commands` (`TrimClipCommand`, `RippleTrim*`, `SetClipColorGradingCommand` testleri yok), `trim-project/src/lib.rs:479` testi `/tmp/` yoluna yazıyor (`tempdir` yerine). — **Orta / M**

---

## Özellik Önerileri

> **Not:** triM. v2.0.0 hâlihazırda keyframe animasyonu, geçişler, renk tekerlekleri/eğriler/LUT/qualifier, Fairlight EQ/kompresör, proxy, nested/compound sequence, motion titles, speed ramp, vektör maskeler, autosave/kurtarma, export preset'leri ve OpenGL GPU kompozitöre **sahiptir**. Aşağıdaki öneriler yalnızca gerçekten eksik olan boşlukları hedefler.

### Hızlı Kazanımlar (Quick Wins)

1. **Merkezi Asenkron İş Kuyruğu & İlerleme/İptal Bildirim Sistemi (Background Job Manager)**
   - **Ne Olduğu:** Whisper deşifre, sahne kesimi, sessizlik algılama, medya `ffprobe` taraması, proxy ve audio sync işlerini tek bir iptal edilebilir arka plan iş yöneticisine taşıyıp alt çubukta/toast alanında canlı ilerleme ve iptal butonu sunmak.
   - **Faydası:** Uzun işlemlerde arayüz donmasını ("Not Responding") tamamen ortadan kaldırır.
   - **Etkilenecek Crate'ler:** `trim-media`, `trim-ui` (`app.rs`, `media_actions.rs`, `timeline_actions.rs`).
   - **Teknik Yaklaşım:** `std::sync::mpsc` + `Arc<AtomicBool>` iptal token'ı + `ChildGuard` ile worker havuzu; `TrimApp::update` içinde her kare poll.
   - **Riskler:** Eşzamanlı proje mutasyonlarında stale `clip_id` kontrolü gerekir.
   - **Efor:** M

2. **Roll, Slide ve Rate Stretch Kurgu Araçları**
   - **Ne Olduğu:** İki bitişik klibin kesim noktasını birlikte kaydıran **Roll**, klibi komşuları arasında kaydıran **Slide** ve klibi kenarından çekerek hızını (`clip.speed`) ayarlayan **Rate Stretch** araçları.
   - **Faydası:** Profesyonel kurguda temel kesim akışını tamamlar.
   - **Etkilenecek Crate'ler:** `trim-commands`, `trim-ui` (`toolbar.rs`, `canvas.rs`, `app.rs`).
   - **Teknik Yaklaşım:** Mevcut `TrimClipCommand` ve `MacroCommand` üzerine `RollEditCommand` / `SlideEditCommand` eklemek; `EditTool` enum'una 3 araç eklemek.
   - **Riskler:** `linked_group_id` ve kilitli kanallarla etkileşim dikkatle test edilmeli.
   - **Efor:** M

3. **Sesli Scrubbing ve JKL Shuttle Ses Önizlemesi**
   - **Ne Olduğu:** Oynatma kafasını sürüklerken (scrub) kısa ~40ms ses parçacığı çalma ve `2x/4x` veya `-1x` JKL shuttle sırasında yeniden örneklenmiş ses oynatma.
   - **Faydası:** Kesim noktasını (örneğin nefes/kelime başlangıcını) kulakla bulmayı kolaylaştırır.
   - **Etkilenecek Crate'ler:** `trim-media` (`audio.rs`), `trim-ui` (`app/playback.rs`).
   - **Teknik Yaklaşım:** `AudioMixer::mix_timeline_chunk`'a hız/yön parametresi ve scrub modunda debounced kısa pencere okuma eklemek.
   - **Riskler:** Hızlı sürüklemede audio buffer taşmasını önlemek için kuyruk limiti gerekir.
   - **Efor:** M

### Orta Vadeli

4. **GPU Shader İçinde Curves, HSL Curves & 3D HSL Qualifier**
   - **Ne Olduğu:** Şu anda CPU'da piksel piksel çalışan özel eğriler, 6-vektör HSL ve 3D HSL Qualifier işlemlerini `GpuCompositor` GLSL fragment shader'ına (1D/3D LUT texture olarak) taşımak.
   - **Faydası:** Renk düzenleme sırasında 4K/60fps gerçek zamanlı önizleme akıcılığı sağlar.
   - **Etkilenecek Crate'ler:** `trim-ui` (`gpu_compositor.rs`).
   - **Teknik Yaklaşım:** Eğri/HSL tablolarını `256x1` RGBA texture olarak yükleyip shader içinde örneklemek; qualifier maskesini doğrudan GPU'da hesaplamak.
   - **Riskler:** OpenGL 3.3 / GLES uyumluluğu ve shader branch maliyeti.
   - **Efor:** L

5. **Zaman Çizelgesinde Genişletilebilir Keyframe / Graph Editor Kanalı**
   - **Ne Olduğu:** Video kliplerin altında Position, Scale, Rotation, Opacity ve Color keyframe'lerini elmas noktalar ve Cubic Bezier eğrileriyle doğrudan timeline üzerinde sürükleyip düzenleyebilme.
   - **Faydası:** Inspector'a bağımlı kalmadan zaman çizelgesi üzerindeki ses/video ritmine göre hassas animasyon ayarı yapılmasını sağlar.
   - **Etkilenecek Crate'ler:** `trim-ui` (`canvas.rs`, `app.rs`), `trim-commands`.
   - **Teknik Yaklaşım:** Track yüksekliği genişletildiğinde klip alt yarısında seçili özelliğin keyframe/eğri katmanını çizmek ve sürüklemeyi `MacroCommand` ile bağlamak.
   - **Riskler:** Mevcut audio volume rubberband etkileşimiyle çakışmayı önlemek için mod geçişi gerekir.
   - **Efor:** L

6. **Ayarlanabilir Klavye Kısayol Editörü & Kaydedilebilir Çalışma Alanı (Workspace) Düzenleri**
   - **Ne Olduğu:** Kullanıcının kendi kısayol haritasını özelleştirebilmesi ve Editing / Color / Audio gibi panel yerleşimlerini kaydedip tek tıkla değiştirebilmesi.
   - **Faydası:** Premiere/Resolve alışkanlıklarına tam uyum ve farklı görevlere özel ekran alanı kullanımı.
   - **Etkilenecek Crate'ler:** `trim-core` (`preferences.rs`), `trim-ui` (`shortcuts.rs`, `preferences_dialog.rs`, `app.rs`).
   - **Teknik Yaklaşım:** `UserPreferences` içine `custom_shortcuts: HashMap<String, String>` ve `workspace_layout` serileştirmesi eklemek.
   - **Riskler:** Çakışan tuş atamaları için doğrulama UI'ı gerekir.
   - **Efor:** M

### Büyük Hedefler

7. **Akışlı (Streaming) Ses & Sürekli Scrub Decoder Altyapısı (Libav / Kalıcı FFmpeg Worker)**
   - **Ne Olduğu:** Tüm ses dosyasını RAM'de dev `Vec<[i16; 2]>` olarak tutmak yerine pencereli/bloklu (paged) PCM okuma ve scrub için her harekette yeni `ffmpeg` süreci başlatmak yerine kalıcı seekable decoder worker kullanma.
   - **Faydası:** Saatlerce süren 4K/uzun projelerde RAM patlamasını önler ve scrub gecikmesini 100ms+ seviyesinden ~10-15ms seviyesine indirir.
   - **Etkilenecek Crate'ler:** `trim-media` (`audio.rs`, `video.rs`).
   - **Teknik Yaklaşım:** Ses için 10 saniyelik PCM blok LRU önbelleği; video scrub için dosya başına açık kalan kalıcı FFmpeg/demuxer veya C-ABI (`ffmpeg-next`) adaptörü.
   - **Riskler:** C-ABI tercih edilirse cross-platform derleme/paketleme karmaşıklığı artar; bloklu PCM daha düşük riskli ilk adımdır.
   - **Efor:** L

8. **EDL / FCPXML Timeline İçe Aktarma Arayüzü & Proje Medya Eşleme**
   - **Ne Olduğu:** `trim-export/src/edl.rs` ve `fcpxml.rs` içinde hâlihazırda bulunan `parse_edl` / `parse_fcpxml` / `import_interchange` motorunu UI üzerinden dosya açma/içe aktarma akışına bağlamak.
   - **Faydası:** Diğer editörlerden gelen kurguları triM. içinde açıp devam etme imkânı verir.
   - **Etkilenecek Crate'ler:** `trim-export`, `trim-project`, `trim-ui` (`app.rs`, `relink_dialog.rs`).
   - **Teknik Yaklaşım:** Menüden "EDL / FCPXML İçe Aktar" aksiyonu ile `Project` üretip eksik medyaları mevcut `RelinkDialog`'a yönlendirmek.
   - **Riskler:** Harici editör efekt/geçiş farkları için uyarı raporu gerekir.
   - **Efor:** M

---

## Yol Haritası

Her aşama bağımsız olarak ayrı bir görev şeklinde uygulanabilecek parçalara bölünmüştür:

### Faz 1: Kritik Hatalar, Panik Koruması ve Kararlılık
1. **Görev 1.1 — Oynatma & Undo/Redo Tutarlılığı:** Boş timeline'da sonsuz oynatma düzeltmesi (`playback.rs`), Undo/Redo sonrası merkezi önbellek temizliği (`app.rs`), `SetClipColorGradingCommand` yeni alanlarının eklenmesi (`commands.rs`), `UndoStack` üst sınır ve hata koruması.
2. **Görev 1.2 — Güvenli Trim/Split/Ripple & Bağlı Klipler:** `Clip::trim_in/out` reverse/keyframe/speed-ramp düzeltmesi (`clip.rs`), `SplitClipCommand` / `BatchSplitClipCommand` maske keyframe & N-yollu `linked_group_id` desteği, `AutoSilenceCutCommand` & `CloseGapCommand` deterministik undo.
3. **Görev 1.3 — Çökme (Panic) Koruması:** `audio.rs` >6 EQ bandı sınır kontrolü, `timeline_actions.rs` / `media_actions.rs` / `edl.rs` non-UTF-8 ve slicing güvenli hale getirme, `playback.rs` fullscreen sıfıra bölme koruması, `ScreenRecordDialog` kapanış temizliği.
4. **Görev 1.4 — Ses DSP Kalıcılığı & Çoklu Video Kanalı:** `AudioMixer` içinde klip başına kalıcı `ClipDspState` + reverb tamponu, `VideoEngine` içinde aktif klip başına `PlaybackStreamer`, `PlaybackStreamer` hız/ChildGuard düzeltmesi.

### Faz 2: Performans ve Eşzamanlılık
5. **Görev 2.1 — Timeline Canvas & GPU Compositor Hızlandırması:** `canvas.rs` yatay viewport culling + önbellekli offline medya durumu; `gpu_compositor.rs` uniform location önbellekleme, maske keyframe hash düzeltmesi ve gereksiz maske upload/LUT disk okuma (`VideoEngine` LUT LRU) kaldırma.
6. **Görev 2.2 — Arka Plan İş Yöneticisi (UI Donmalarını Kaldırma):** Whisper, Sahne Kesimi, Sessizlik Algılama, Toplu `ffprobe` ve Source resim açmayı iptal edilebilir worker havuzuna taşıma; `AudioMixer::preload_audio_async` eşzamanlılık sınırı ve `proxy_rx` çoklu iş kuyruğu.
7. **Görev 2.3 — Keyframe & Scopes Optimizasyonu:** `integrate_keyframes` için tek seferlik sıralı dilim değerlendirme (601 sort yerine), `GpuCompositor` içinde curves/HSL shader geçişi ve Scopes arka plan iş parçacığı.

### Faz 3: FFmpeg Export Paritesi ve Platform Sağlamlaştırma
8. **Görev 3.1 — FFmpeg Export Düzeltmeleri:** VAAPI (`hwupload`) & ProRes 422 (`yuv422p10le`) düzeltmesi, `ExportRunner` iptal yarışı ve yetim süreç koruması, `-filter_complex_script` ve geçici `.pgm` temizliği.
9. **Görev 3.2 — Preview ↔ Export Tam Parite:** `rotation`, `track_matte_above`, `hsl_curves`/`hsl_qualifier`, `speed` keyframe ses senkronu (`atempo` zinciri) ve `reverse` + `range` dilimleme düzeltmeleri.
10. **Görev 3.3 — Platform & Proje Taşınabilirliği:** `AutosaveManager` kararlı FNV-1a/SHA256 hash geçişi, `bundle_project` proxy/LUT kopyalama ve dosya adı temizliği, Linux/Windows FFmpeg uyarı mesajı ayrımı, FCPXML `file://` URI kodlaması ve i18n TR/EN tamamlama.

### Faz 4: Yeni Özellikler ve Test Güçlendirme
11. **Görev 4.1 — Kurguda Hızlı Kazanımlar:** Roll / Slide / Rate Stretch araçları, Sesli Scrubbing & JKL ses önizlemesi, kısayol dokümantasyonu eşitleme.
12. **Görev 4.2 — Gelişmiş İş Akışları & Test Kapsamı:** Timeline Keyframe/Graph Editor, EDL/FCPXML İçe Aktarma UI'ı, Özelleştirilebilir Kısayol/Workspace düzenleri; `runner.rs`, `presets.rs`, `trim-timeline` ve `trim-commands` eksik birim testlerinin eklenmesi.
