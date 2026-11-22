# triM. (v2.5.0) Kapsamlı Teknik Denetim Raporu

**Denetim Tarihi:** 2026-11-22  
**İncelenen Sürüm:** `v2.5.0` (9 crate'lik Cargo workspace: `trim-core`, `trim-commands`, `trim-timeline`, `trim-project`, `trim-media`, `trim-render`, `trim-export`, `trim-ui`, `trim-app`)  
**Kapsam:** Salt-okunur statik kod incelemesi, mimari/güvenlik/performans analizi ve derleme/test/bağımlılık araç doğrulaması. Kaynak kod, bağımlılık veya yapılandırma dosyalarında hiçbir değişiklik yapılmamıştır.

---

## 1. Doğrulama Araçları Çıktı Özeti

| Araç / Komut | Durum | Gözlenen Çıktı |
| :--- | :--- | :--- |
| `cargo check --workspace` | Başarılı (0 hata) | Tüm 9 crate temiz derlendi. |
| `cargo clippy --workspace --all-targets` | Başarılı (0 uyarı) | Hiçbir clippy uyarısı üretilmedi (`-D clippy::unwrap_used` vb. CI kapısıyla uyumlu). |
| `cargo test --workspace` | Başarılı (251/251) | `trim-commands` (21), `trim-core` (78 birim + 1 entegrasyon), `trim-export` (36 birim + 2 entegrasyon), `trim-media` (53), `trim-project` (9), `trim-render` (2), `trim-timeline` (3), `trim-ui` (46), `trim-app` (0); 0 başarısız, 0 atlanan. |
| `cargo deny check` | Başarılı (`ok`) | `advisories ok, bans ok, licenses ok, sources ok`. Tek uyarı: `deny.toml:10` içindeki `RUSTSEC-2026-0486` istisnası şu an `cargo-deny` kriteriyle eşleşmedi (`advisory-not-detected`). |
| `cargo audit` | Başarılı (0 kritik, 6 uyarı) | `Cargo.lock` içinde 415 bağımlılık tarandı; 0 zafiyet, 6 izin verilen uyarı: `paste 1.0.15` (`RUSTSEC-2024-0436` unmaintained), `ttf-parser 0.25.1` (`RUSTSEC-2026-0192` unmaintained), `zbus_names 4.3.4` (`RUSTSEC-2026-0371` unmaintained), `zvariant 5.15.0` (`RUSTSEC-2026-0370` unmaintained), `zvariant_derive 5.15.0` (`RUSTSEC-2026-0372` unmaintained), `rustix 1.1.5` (`RUSTSEC-2026-0486` unsound — xattr zero-length buffer). |
| `unsafe` ve `unwrap`/`expect`/`panic!` Taraması | Temiz | Üretim (non-test) kodunda **0 `unwrap()`/`expect()`/`panic!()`/`unreachable!()`** bulundu. `unsafe` yalnızca `crates/trim-render/src/lib.rs` içindeki OpenGL `glow` FFI sarmalayıcılarında yer almaktadır; diğer 8 crate'te `0 unsafe` vardır. |

---

## 2. Yönetici Özeti (En Kritik 5 Bulgu)

Denetimde doğrudan bellek güvenliği ihlali veya üretim kodunda `panic!` doğuran **Kritik** seviyede bir hata tespit edilmemiştir; buna karşın kullanıcı deneyimini, veri güvenliğini, export doğruluğunu ve yanıt verebilirliği doğrudan etkileyen **13 Yüksek** bulgu saptanmıştır. En kritik 5 bulgu şunlardır:

1. **UI Thread Üzerinde Senkron FFmpeg / `ffprobe` / `image::open` Blokajları (`BUL-01`, `BUL-02`, `BUL-03`):** Medya içe aktarma/klasör tarama (`crates/trim-ui/src/app/media_actions.rs:137-215`), proje yükleme (`crates/trim-ui/src/app/project_io.rs:43-56`), kaynak monitörü görsel yükleme (`crates/trim-ui/src/app/playback.rs:424-457` — hata durumunda her karede sonsuz tekrar deneme) ve zaman çizelgesi boşluk geçişlerinde (`crates/trim-media/src/video.rs:1406-1432`) senkron alt süreç/dosya I/O UI thread'i dondurmaktadır.
2. **Bozuk/Kötü Niyetli Medyada Sınırsız `.output()` OOM/DoS ve Eksik Timeout (`BUL-04`, `BUL-05`):** `analyzer.rs:31-45,606-624`, `scene_detect.rs:45-47`, `silence_detect.rs:55-57` ve `transcribe.rs:136-138` sınırsız `Command::output()` kullanır; `decode_audio_to_pcm` (`audio.rs:827-904`) ise ~1 GB'lık çift tampon tahsis eder ve ses içermeyen dosyalarda 14'e kadar ardışık FFmpeg süreci başlatır.
3. **Video Kanallarındaki Kliplerin Seslerinin Export Edilmemesi & İptal Edilen Export'un Asılı Kalması (`BUL-06`, `BUL-07`):** `crates/trim-export/src/filtergraph.rs:1362` yalnızca `TrackType::Audio` izlerini karıştırdığı için Video izlerindeki kliplerin sesi çıktıya aktarılmaz; `video_tracks.is_empty()` (`1174-1177`) ses-odaklı projeleri reddeder. Ayrıca `runner.rs:158-181` iptal bayrağını yalnızca `stdout` satırı geldikçe kontrol ettiğinden takılan FFmpeg süreci `cancel()` sonrası öldürülmez.
4. **Symlink / Boyut Sınırı Eksiklikleri ve `clip.id` Path Traversal Riski (`BUL-08`, `BUL-09`, `BUL-10`, `BUL-11`):** `AutosaveManager::list_snapshots` (`trim-project/src/lib.rs:510-547`) ve `LookPresetFile::load_from_file` (`trim-core/src/looks.rs:82-103`) symlink ve dosya boyutu kontrolü yapmaz; `resolve_safe_media_path` (`trim-project/src/lib.rs:64-70`) symlink hedefinde güvenli boş string yerine göreli yola düşer; `filtergraph.rs:1268,1285,1423` maske `.pgm` dosya adında `clip.id`'yi sanitize etmez.
5. **Ayar Katmanlarında GPU Önizleme Eksikliği & `trim_core::keymap` / `looks` UI Bağlantısının Eksikliği (`BUL-12`, `BUL-13`):** `trim-render/src/lib.rs:890-892` nedeniyle `custom_curves`, `hsl_curves` ve `hsl_qualifier` ayar katmanlarında GPU önizlemede hiç çalışmaz. Ek olarak `trim-core` içindeki `KeymapConfig` (`shortcuts.rs:1` re-export edilse de `app.rs:701-1095` sabit kodlu), `FinalCutPro` profili, `.trimlook`, `TitleTemplate` ve `list_snapshots()` henüz UI'a bağlanmamıştır.

---

## 3. İnceleme Alanlarına Göre Teknik Değerlendirme

1. **Mimari:** 9 crate'lik workspace temiz ve döngüsüz bir DAG oluşturmaktadır (`trim-core` -> `trim-commands` -> `trim-timeline`; `trim-core` -> `trim-project` / `trim-media`; `trim-media` -> `trim-render` / `trim-export`; tümü -> `trim-ui` -> `trim-app`). `v2.5.0` ile `trim-render` ayrımı yapılmıştır. Zaman modeli tüm veri yapılarında `f64` saniye tabanlıdır (`clip.rs:691-696`); `export_fcpxml` (`interchange.rs:237-242`) kesirli kare hızlarını tam sayıya yuvarlar (`29.97 -> 30`). Aynı video kanalında çakışan kliplerde `Track::find_clip_at` (`track.rs:111-113`) yalnızca ilk klibi döndürür.
2. **Güvenlik / Zafiyet:** FFmpeg FFI yerine `std::process::Command` ile çalıştırılmaktadır. `ChildGuard` (`ffmpeg.rs:196-301`) ve `Cargo.toml:39` (`panic = "unwind"`) zombi süreçleri büyük ölçüde engeller. Ancak `scene_detect`, `silence_detect`, `transcribe` ve `thumbnail` modüllerinde `-protocol_whitelist file` eksiktir (`BUL-22`); Windows `dshow` aygıt adları (`voiceover.rs:31`) ve bazı çıktı yolları sanitizasyondan yoksundur (`BUL-32`). `unsafe` yalnızca `trim-render/src/lib.rs` içindeki GL FFI çağrılarındadır.
3. **Doğruluk ve Dayanıklılık:** Üretim kodunda `0 unwrap/expect/panic!`. Ancak `NaN`/`Inf` doğrulaması `Clip::new`, `Clip::trim_out`, `ChangeClipSpeedCommand`, `RippleTrimHead/TailCommand` ve `Project::validate_and_sanitize` alanlarının tamamını kapsamaz (`BUL-15`, `BUL-17`). `track.locked` kontrolü komutlar arasında tutarsızdır (`BUL-16`). `AudioPlaybackWorker::drop` (`audio.rs:1483-1495`) bloklu `stdin.write_all` veya uzun decode sırasında ana thread'i kilitleyebilir (`BUL-14`).
4. **Medya Motoru:** `FrameBufferPool` (`frame_pool.rs`) `PlaybackStreamer` içine eklenmiştir; ancak `VideoEngine` gelen `Arc<RgbaImage>` karesini 3 alanda kopyaladığı için `Arc::try_unwrap` başarısız olmakta ve geri dönüşüm verimsiz kalmaktadır (`BUL-23`). Hız değişiminde `audio.rs:1427-1461` `frames_written = 0` yaptığı için ses saati kısa süreli `None` döner ve ~25ms kayma birikir (`BUL-25`). `frame_cache` anahtarı `(eff_path, ts_key)` çözünürlük içermez (`BUL-24`).
5. **Performans:** `canvas.rs:298-309` her karede tüm timeline'ı tarayıp `linked_group_clips` ve `String` tahsisleri yapar (`BUL-18`), ses dalga formları için her karede binlerce `Shape::line_segment` üretir (`BUL-19`). `update_monitor_frame` (`playback.rs:207-285`) duraklatılmışken dahi her karede GPU render ve Scope readback çalıştırır (`BUL-20`).
6. **Proje Dosyası ve Veri Bütünlüğü:** `CURRENT_PROJECT_VERSION` (`v0` -> `v1`) geçişi ve `NamedTempFile` atomik kayıt mevcuttur; ancak `AutosaveManager::list_snapshots` ve `LookPresetFile::load_from_file` boyut/symlink denetimi eksiktir (`BUL-08`, `BUL-09`), `AutosaveManager::save_versioned_snapshot` çift serileştirme yapar (`BUL-31`), `AutoSilenceCutCommand::undo` sezgisel kaydırma kullanır (`BUL-18/cmd` -> `BUL-28`).
7. **Kod Kalitesi:** `cargo clippy --workspace --all-targets` 0 uyarı, `251/251` test geçiyor. Ancak `trim-app` (0 test) ve `trim-timeline` (3 test) kapsamı dardır; `mask.rs:349-397` ile `503-559` SDF kodu tekrarlıdır; `trim-project/src/lib.rs:769-917` testi `/tmp` yoluna sabit yazmaktadır.
8. **Platform ve Dağıtım:** Workspace `GPL-3.0-or-later` uyumludur (`deny.toml:15-31`). Ancak `.github/workflows/release-linux.yml:195-207` Windows Strategy 1 indirmesinde SHA-256 doğrulaması ve `FFMPEG-LICENSE.txt` kopyalaması eksiktir (`BUL-13/pkg` -> `BUL-13b` tablodaki `BUL-13` ile). `Cargo.lock`'ta çift `glow` (`0.13.1` + `0.14.2`) bulunur ve CI'da SBOM üretilmez (`BUL-30`).
9. **Kullanılabilirlik ve Eksik Özellikler:** `KeymapConfig`, `FinalCutPro` profili, `.trimlook` kütüphanesi, `TitleTemplate` ve sürüm snapshot tarayıcısı çekirdek/i18n katmanında hazır olup UI'a bağlanmamıştır (`BUL-13`, `BUL-29`). Dil `en` seçilse bile durum mesajları, Canvas, Inspector ve Scopes Türkçe sabit kalmaktadır (`BUL-21`).

---

## 4. Bulgu Tablosu

| ID | Önem | Kategori | Dosya:satır | Açıklama | Önerilen çözüm | Tahmini Efor |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **BUL-01** | Yüksek | UI / Performans | `crates/trim-ui/src/app/media_actions.rs:137-215`, `project_io.rs:43-56` | Medya içe aktarma, klasör tarama ve proje yüklemede senkron `FFprobeAnalyzer::analyze` UI thread'i bloklar. | `ffprobe` analizini `bg_task_tx`/`rx` arka plan havuzuna taşıyıp yer tutucu gösterin. | M |
| **BUL-02** | Yüksek | UI / Dayanıklılık | `crates/trim-ui/src/app/playback.rs:424-457` | Kaynak monitöründe senkron `image::open` çalışır; hata durumunda `last_decoded_id` güncellenmediği için her karede yeniden denenir. | Görsel decode'u arka plana alın ve hata önbelleği/işareti ile sonsuz denemeyi durdurun. | M |
| **BUL-03** | Yüksek | Medya / UI | `crates/trim-media/src/video.rs:934-948,1096,1406-1432` | `VideoEngine` boşluk çıkışı, eksik katman karesi ve Auto-WB analizinde UI thread üzerinde senkron FFmpeg decode yapar. | `async_decoder.request_seek_sized` kullanıp kare gelene kadar son kare/boş dönün. | M |
| **BUL-04** | Yüksek | Güvenlik / DoS | `crates/trim-media/src/analyzer.rs:31-45,606-624`, `scene_detect.rs:45-47`, `silence_detect.rs:55-57`, `transcribe.rs:136-138` | `Command::output()` sınırsız stdout/stderr okur ve timeout içermez; bozuk medya OOM/donma yaratabilir. | `ChildGuard` + `Read::take(MAX_BYTES)` + timeout içeren `run_command_bounded` kullanın. | M |
| **BUL-05** | Yüksek | Bellek / Medya | `crates/trim-media/src/audio.rs:827-904`, `waveform.rs:230-295` | `decode_audio_to_pcm` ~1 GB çift tampon tahsis eder; ses olmayan dosyada 14'e kadar ardışık FFmpeg süreci başlatır. | Pipe'tan chunk okuyarak doğrudan `[i16; 2]` doldurun; ses akışı yoksa brute-force'u atlayın. | S |
| **BUL-06** | Yüksek | Export / Doğruluk | `crates/trim-export/src/filtergraph.rs:1174-1177,1362-1366` | Video izlerindeki kliplerin sesi export edilmez (`TrackType::Audio` şartı); `video_tracks.is_empty()` ses-only projeyi reddeder. | Video izlerindeki sesli klipleri de audio mix'e katın; yalnızca `has_any_clip` kontrolü yapın. | M |
| **BUL-07** | Yüksek | Export / Süreç | `crates/trim-export/src/runner.rs:87-99,158-181` | İptal edilen FFmpeg süreci `stdout` satırı gelene kadar öldürülmez; `total_duration <= 0` bölmesi `NaN` üretir; `_temp_dir == None` iken büyük filtergraph script fallback çalışmaz. | `Arc<Mutex<Option<Child>>>` ile anında `kill()`, `total_duration.max(0.001)` ve otomatik `TempDir` ekleyin. | M |
| **BUL-08** | Yüksek | Güvenlik / Proje | `crates/trim-project/src/lib.rs:510-547` | `AutosaveManager::list_snapshots` symlink kontrolü ve `MAX_PROJECT_FILE_BYTES` sınırı olmadan `fs::read_to_string` çağırır. | `symlink_metadata` + `File::take(MAX_PROJECT_FILE_BYTES + 1)` ekleyin. | S |
| **BUL-09** | Yüksek | Güvenlik / Core | `crates/trim-core/src/looks.rs:54-67,73-103` | `LookPresetFile::load_from_file` boyut/symlink denetimi yapmaz; `apply_to_clip` `NaN`/`Inf`/aşırı float değerlerini sınırlamaz. | 1 MiB + symlink kontrolü, `is_finite()`/clamp ve atomik kayıt ekleyin. | S |
| **BUL-10** | Yüksek | Güvenlik / Proje | `crates/trim-project/src/lib.rs:52-71` | `resolve_safe_media_path` symlink hedefte boş string yerine göreli yola düşer; mutlak yollarda `..`/symlink kontrolü yapmaz. | Symlink tespitinde `String::new()` dönün; mutlak yolda da `ParentDir` engelleyin. | S |
| **BUL-11** | Yüksek | Güvenlik / Export | `crates/trim-export/src/filtergraph.rs:1268,1285,1423` | Maske `.pgm` dosya adlarında `clip.id` sanitize edilmeden kullanılır (path traversal riski). | `.cube` yazımındaki `safe_id` sanitizasyonunu ortaklaştırıp maskelerde de kullanın. | S |
| **BUL-12** | Yüksek | Render / Parite | `crates/trim-render/src/lib.rs:463-500,890-892` | `custom_curves`, `hsl_curves` ve `hsl_qualifier` ayar katmanlarında (`is_adjustment_layer`) GPU önizlemede hiç uygulanmaz. | Bu 3 aşamayı shader/1D-3D LUT üzerinden `u_backdrop_tex` yoluna da taşıyın. | M |
| **BUL-13** | Yüksek | Dağıtım / Güvenlik | `.github/workflows/release-linux.yml:194-209,281-284`, `scripts/package-linux.sh:219-227` | Windows Strategy 1 FFmpeg indirmesinde SHA-256 doğrulaması ve `FFMPEG-LICENSE.txt` kopyalaması eksik; Linux `linuxdeploy` hash'i CI'da sabitlenmemiş. | Sabit sürümlü + SHA-256 doğrulamalı indirme ve her dalda `FFMPEG-LICENSE.txt` ekleyin. | S |
| **BUL-14** | Yüksek | Medya / Kilit | `crates/trim-media/src/audio.rs:1130-1131,1483-1495` | `AudioPlaybackWorker::drop` bloklu `stdin.write_all` veya uzun decode sırasında `h.join()` ile ana thread'i kilitleyebilir. | `drop` öncesi aktif alt süreci öldürecek atomic/handle iptali ekleyin. | S |
| **BUL-15** | Yüksek | UI / Mimari | `crates/trim-ui/src/shortcuts.rs:1`, `app.rs:358-362,701-1095`, `dialogs/shortcuts_dialog.rs:20-98` | `trim_core::keymap` (`KeymapConfig`, `ShortcutAction`, `FinalCutPro`) UI'a bağlanmamış; kısayollar `app.rs`'de sabit kodlu. | `app.rs` kısayollarını `KeymapConfig` üzerinden yönlendirin ve `FinalCutPro` eşlemesini ekleyin. | M |
| **BUL-16** | Orta | UI / Performans | `crates/trim-ui/src/canvas.rs:298-309,350-351,523` | `render_timeline_canvas` her karede tüm timeline'ı tarayıp `linked_group_clips` `HashMap`'i ve `String` tahsisleri yapar. | `linked_group_clips`'i timeline değişiminde önbelleğe alın; `String` klonlarını kaldırın. | M |
| **BUL-17** | Orta | UI / Performans | `crates/trim-ui/src/canvas.rs:624-644` | Görünür her ses klibi için her karede binlerce `Shape::line_segment` ve `Vec` tahsis edilir. | Dalga formunu zoom/genişlik bazlı önbellekli `Mesh` veya `PathShape` olarak çizin. | M |
| **BUL-18** | Orta | UI / Performans | `crates/trim-ui/src/app/playback.rs:207-285`, `scopes_panel.rs:117` | Duraklatılmış monitörde her karede GPU render ve `glReadPixels` scope readback çalışır; CPU modda throttle yoktur ve RGBA tamponu çift saklanır. | Değişiklik yoksa render'ı atlayın; CPU scope'a `83ms` throttle ve tek `Arc<[u8]>` ekleyin. | M |
| **BUL-19** | Orta | Render / Kaynak | `crates/trim-render/src/lib.rs:20,330-338,427-461,810-812,1167-1194` | `GpuCompositor::drop` GL kaynaklarını silmez; `scope_fbo` ve resize sonrası `backdrop_fbo` tamlık kontrolü ile `FRAMEBUFFER_SRGB` restorasyonu eksiktir. | `Drop` içinde `destroy()` çağırın, tüm FBO'ları doğrulayın ve `FRAMEBUFFER_SRGB`'yi restore edin. | S |
| **BUL-20** | Orta | Export / Parite | `crates/trim-export/src/filtergraph.rs:481,511-545,828,858,1254-1488` | `zoom_in/out` geçişi, normal klipte `ColorAndEffects` maskesi, keyframed `scale`/maske ve `crop`/`corner_pin` sırası GPU preview ile uyumsuz. | Normal klipte `has_fx_mask` `maskedmerge` ekleyin, `crop`'u `rotate`/`perspective` önüne alın. | M |
| **BUL-21** | Orta | UI / i18n | `crates/trim-ui/src/app.rs:469-1135`, `canvas.rs`, `media_bin.rs`, `scopes_panel.rs`, `inspector.rs` | `en` seçildiğinde dahi durum mesajları, Canvas, MediaBin, Scopes, Dialoglar ve Inspector Türkçe sabit kalır. | `Language` parametresini tüm panellere geçirin ve `i18n::tr` sözlüğünü tamamlayın. | M |
| **BUL-22** | Orta | Güvenlik / Medya | `crates/trim-media/src/scene_detect.rs:18-37`, `silence_detect.rs:23-47`, `transcribe.rs:59-73`, `thumbnail.rs:202-280` | Bu 4 modülde yerel FFmpeg çağrılarında `-protocol_whitelist file` bayrağı eksik. | Tüm yerel FFmpeg komutlarına `-protocol_whitelist file` ekleyin. | S |
| **BUL-23** | Orta | Medya / Bellek | `crates/trim-media/src/frame_pool.rs:55,72-76`, `video.rs:445-465,908-910,1351-1354` | `VideoEngine` `Arc<RgbaImage>`'i 3 alanda tuttuğu için `FrameBufferPool::recycle_arc` (`Arc::try_unwrap`) pratikte geri dönüşüm yapamaz. | Oynatma sırasında eski `Arc` referanslarını bırakın veya pool checkout'a geçin. | S |
| **BUL-24** | Orta | Medya / Cache | `crates/trim-media/src/video.rs:838-839,1041-1059,1521-1550` | `frame_cache` anahtarı çözünürlük içermez; `image_cache` LRU değil ve `VideoFrameDecoder::get_image` boyut/limit kontrolsüz `image::open` çağırır. | `frame_cache` anahtarına `(w, h)` ekleyin; `image::Limits` ve `LruCache` kullanın. | M |
| **BUL-25** | Orta | Medya / Senkron | `crates/trim-media/src/audio.rs:1219-1231,1427-1471`, `video.rs:248,297,313-420` | Hız değişiminde `audio_clock_time` `None` döner ve ~25ms kayma birikir; VFR'de sentetik PTS kullanılır; yüksek hızda video FFmpeg'e hız iletilmez. | Hız değişiminde `cursor_time`'ı güncelleyin; ses resampling için lineer/sinc ve video hız filtresi ekleyin. | M |
| **BUL-26** | Orta | Medya / Yarış | `crates/trim-media/src/video.rs:785,925-932,1137-1148` | Tekil `is_decoding_seek` bayrağı çok katmanlı scrubbing sırasında diğer katmanların seek isteğini bloklar. | Katman/klip bazlı veya `(path, ts_key)` tabanlı seek takibine geçin. | S |
| **BUL-27** | Orta | Core / Doğruluk | `crates/trim-core/src/clip.rs:1162,1407,1420`, `project.rs:475-545`, `commands.rs:1157,1332,1553` | `Clip::new`, `trim_out`, `change_speed`, `RippleTrimHead/Tail` ve `validate_and_sanitize` `NaN`/`Inf` girişlerini tüm alanlarda süzmez. | Tüm zaman, hız, transform ve keyframe alanlarına `is_finite()` doğrulaması ekleyin. | S |
| **BUL-28** | Orta | Commands / Undo | `crates/trim-commands/src/commands.rs` (çeşitli komutlar + `1635-1710`) | `track.locked` kontrolü 10+ komutta eksik; `AutoSilenceCutCommand::undo` sezgisel konum hesabıyla çakışan klipleri bozabilir. | Tüm iz mutasyonlarına `track.locked` kontrolü ve `AutoSilenceCutCommand`'a `shifted_clips` kaydı ekleyin. | M |
| **BUL-29** | Orta | UI / Özellikler | `crates/trim-ui/src/i18n.rs:251-265`, `color_card.rs`, `title_dialog.rs`, `recovery_dialog.rs` | `.trimlook`, `TitleTemplate` ve `AutosaveManager::list_snapshots()` i18n anahtarları hazır olduğu halde UI'a bağlanmamış. | Renk kartı, başlık penceresi ve kurtarma penceresine bu 3 özelliği bağlayın. | M |
| **BUL-30** | Orta | Dağıtım / SBOM | `Cargo.lock:1194-1215`, `deny.toml:8-11`, `.github/workflows/ci.yml`, `release-linux.yml` | `Cargo.lock`'ta çift `glow` (`0.13.1`, `0.14.2`) var; `rustix` `RUSTSEC-2026-0486` istisnası mevcut; release'de SBOM üretilmiyor. | Release iş akışına CycloneDX/SPDX SBOM adımı ekleyin ve `rustix` yamasını takip edin. | S |
| **BUL-31** | Düşük | Proje / Bütünlük | `crates/trim-project/src/lib.rs:413-559`, `preferences.rs:104-124`, `keymap.rs:219-227` | `save_versioned_snapshot` çift serileştirme ve 1ms çakışması yaşayabilir; `check_recovery` `is_file()` kullanır; `keymap`/`preferences` kaydı `NamedTempFile` kullanmaz. | Serileştirilmiş JSON'u paylaşın, `symlink_metadata` ve `NamedTempFile` kullanın. | S |
| **BUL-32** | Düşük | Güvenlik / Medya | `crates/trim-media/src/voiceover.rs:31`, `screen_record.rs:67-81`, `thumbnail.rs`, `proxy.rs` | Windows `dshow` aygıt adları sanitize edilmez; çıktılarda `-` öneki ve Linux'ta `.to_string_lossy()` non-UTF-8 yolları bozabilir. | `Command::arg(&Path)` kullanın ve `dshow` / çıktı yollarını sanitize edin. | S |
| **BUL-33** | Düşük | UI / Hata / Erişilebilirlik | `crates/trim-ui/src/app.rs:560+`, `playback.rs:666-705`, `canvas.rs:259+` | `undo_stack.push` hataları `let _ =` ile yutulur; oynatma sırasında undo `audio_worker` projesini yenilemez; özel çizimlerde `accesskit` etiketi yoktur. | Push hatalarını raporlayın, undo'da `audio_worker`'ı yenileyin, `widget_info` ekleyin. | M |
| **BUL-34** | Düşük | Export / Araçlar | `crates/trim-export/src/edl.rs:90`, `fcpxml.rs:420-429`, `filtergraph.rs:47-59,773` | `edl.rs` video izlerini sayarak `A2` yazar; `fcpxml.rs` kapanmayan CDATA'da bozulur; `ChapterCard` alt başlıkta `{font_part}` eksik. | Ses iz sayacını ayırın, CDATA sonu yoksa hata dönün, satır `773`'e `{font_part}` ekleyin. | S |

---

## 5. Kritik ve Yüksek Bulgular İçin Ayrıntılı Açıklama ve Düzeltme Önerileri

### BUL-01 & BUL-02 & BUL-03: UI Thread Üzerinde Senkron Medya / FFprobe / Görsel Decode Blokajı
- **Konumlar:** `crates/trim-ui/src/app/media_actions.rs:137-215`, `crates/trim-ui/src/app/project_io.rs:43-56`, `crates/trim-ui/src/app/playback.rs:424-457`, `crates/trim-media/src/video.rs:934-948,1406-1432`
- **Etki:** Büyük bir klasör içe aktarılırken her dosya için ardışık `ffprobe` süreci başlatılması UI'ı saniyelerce dondurur. Kaynak monitöründe bozuk bir görsel seçiliyse `image::open` başarısız oldukça `last_decoded_id` güncellenmediği için **her karede** diskten yeniden okuma yapılır. Zaman çizelgesinde bir boşluktan çıkıldığında `VideoEngine` ilk kareyi senkron `decode_frame_raw_sized` ile bekler.
- **Somut Düzeltme:**
  - `import_media_path` ve `load_project_from_path` içindeki `FFprobeAnalyzer::analyze` çağrılarını arka plan iş parçacığına (`bg_task_tx`) taşıyın.
  - `playback.rs:424-457` içinde hata durumunda `last_decoded_id = Some(media_id.clone())` (veya `failed_media_ids: HashSet<String>`) işaretleyin:
```rust
match image::open(&media_path) {
    Ok(img) => { /* texture yükle */ }
    Err(e) => {
        log::warn!("Source monitor image decode failed: {e}");
        self.source_monitor.last_decoded_id = Some(media_id);
    }
}
```
  - `VideoEngine::render_timeline_frame` (`video.rs:1406-1432`) içinde senkron `decode_frame_raw_sized` yerine `self.async_decoder.request_seek_sized(...)` tetikleyin.

### BUL-04 & BUL-05: Bozuk Medyada Sınırsız `.output()` OOM/DoS ve Çift PCM Tamponu
- **Konumlar:** `crates/trim-media/src/analyzer.rs:31-45,606-624`, `scene_detect.rs:45-47`, `silence_detect.rs:55-57`, `transcribe.rs:136-138`, `audio.rs:827-904`, `waveform.rs:230-295`
- **Etki:** Bozuk veya kötü amaçlı bir medya dosyası `ffmpeg`/`ffprobe` çıktısını kilitleyebilir veya GB'larca stdout/stderr üreterek OOM'a yol açabilir. `decode_audio_to_pcm` iki adet 512 MB vektörü aynı anda tutarak ~1 GB pik bellek kullanır ve sessiz/hatalı medyada 14 alt süreç dener.
- **Somut Düzeltme:**
  - Ortak bir `run_command_bounded(cmd, max_bytes, timeout)` yardımcısı yazın (`ChildGuard` + `stdout.take(max_bytes)` + süre kontrolü).
  - `decode_audio_to_pcm` içinde 64 KB'lık geçici okuma tamponuyla doğrudan `Vec<[i16; 2]>` doldurun; önceden `ffprobe` ile ses akışı olmadığı biliniyorsa 6-codec fallback döngüsüne girmeyin.

### BUL-06 & BUL-07: Export Ses Eksikliği, Süreç İptal Kilitlenmesi ve `total_duration == 0`
- **Konumlar:** `crates/trim-export/src/filtergraph.rs:1174-1177,1362-1366`, `crates/trim-export/src/runner.rs:87-99,158-181`
- **Etki:** Video izindeki kliplerin (ör. V1 üzerindeki kamera klibinin) dahili sesi export'a yansımaz; yalnızca ses izlerinden oluşan projeler reddedilir. Export iptal edildiğinde FFmpeg yeni progress satırı basmazsa `reader.lines()` bloklu kalır.
- **Somut Düzeltme:**
  - `filtergraph.rs:1362`: `track.track_type == TrackType::Audio` yerine `!track.muted && track.audible` olan video/ses izlerindeki `has_audio()` kliplerini karıştırın; `1174-1177` kontrolünü `has_any_clip` ile değiştirin.
  - `runner.rs`: `Child` tutamacını `Arc<Mutex<Option<Child>>>` içinde saklayıp `cancel()` çağrısında doğrudan `kill()` tetikleyin; `170` satırında `total_duration.max(0.001)` kullanın.

### BUL-08, BUL-09, BUL-10 & BUL-11: Symlink, Boyut Sınırı, Float Sanitizasyonu ve Path Traversal
- **Konumlar:** `crates/trim-project/src/lib.rs:52-71,510-547`, `crates/trim-core/src/looks.rs:54-103`, `crates/trim-export/src/filtergraph.rs:1268,1285,1423`
- **Somut Düzeltme:**
  - `list_snapshots` ve `LookPresetFile::load_from_file` içine `fs::symlink_metadata` (`!meta.file_type().is_symlink() && meta.is_file()`) ve `File::take(MAX_BYTES + 1)` sınırı ekleyin.
  - `resolve_safe_media_path` (`trim-project/src/lib.rs:64-70`) içinde symlink görüldüğünde `String::new()` döndürün.
  - `filtergraph.rs:1268,1285,1423` maske dosya adlarında `sanitize_clip_id(&clip.id)` kullanın.

### BUL-12 & BUL-15: Ayar Katmanı GPU Paritesi ve `KeymapConfig` UI Bağlantısı
- **Konumlar:** `crates/trim-render/src/lib.rs:463-500,890-892`, `crates/trim-ui/src/app.rs:358-362,701-1095`
- **Somut Düzeltme:**
  - `trim-render` içinde `custom_curves`, `hsl_curves` ve `hsl_qualifier` için 3D/1D LUT dokusu üreterek `u_is_adjustment_layer == 1` yolunda da uygulayın.
  - `TrimApp` içinde `KeymapConfig` yükleyip `app.rs:701-1095` kısayollarını `keymap.action_for_input(...)` ile çalıştırın; `"Final Cut Pro"` profilini `ShortcutProfile::FinalCutPro` ile eşleyin.

### BUL-13 & BUL-14: Release Hash/Lisans Bütünlüğü ve `AudioPlaybackWorker::drop`
- **Konumlar:** `.github/workflows/release-linux.yml:194-209`, `crates/trim-media/src/audio.rs:1483-1495`
- **Somut Düzeltme:**
  - Windows release iş akışında Strategy 1 için sabit sürümlü + SHA-256 doğrulamalı FFmpeg paketi ve `FFMPEG-LICENSE.txt` kopyalaması ekleyin.
  - `AudioPlaybackWorker::drop` öncesinde aktif `AudioStreamPlayer` child handle'ını sonlandıracak atomic iptal mekanizması ekleyin.

---

## 6. Geliştirme Yol Haritası

- **Kısa Vadeli (Hızlı Kazanımlar — 1–2 gün):**
  - `BUL-08`, `BUL-09`, `BUL-10`, `BUL-11` (symlink, boyut sınırı, `clip.id` sanitizasyonu, float clamp).
  - `BUL-05` çift tamponun kaldırılması, `BUL-22` `-protocol_whitelist file` tamamlanması, `BUL-13` Windows FFmpeg SHA-256 + lisans kopyası.
  - `BUL-27` `NaN`/`Inf` korumaları ve `BUL-34` EDL/FCPXML/ChapterCard küçük düzeltmeleri.
- **Orta Vadeli (1–2 hafta):**
  - `BUL-01`, `BUL-02`, `BUL-03`, `BUL-04`, `BUL-14` (UI thread'den tüm FFprobe/FFmpeg/`image::open` blokajlarının kaldırılması, `run_command_bounded`, güvenli `AudioPlaybackWorker` kapatma).
  - `BUL-06`, `BUL-07`, `BUL-20` (Video izlerinin ses export'u, ses-only timeline, export anında kill, `ColorAndEffects` maske & geçiş paritesi).
  - `BUL-16`, `BUL-17`, `BUL-18`, `BUL-23`, `BUL-24`, `BUL-25` (Canvas/waveform önbellekleme, pause render atlama, `FrameBufferPool` geri dönüşümünün etkinleştirilmesi, ses saati hız senkronu).
- **Uzun Vadeli (2–4+ hafta):**
  - `BUL-12` (Ayar katmanı curves/HSL GPU shader/LUT birleştirme), `BUL-15` & `BUL-29` (`KeymapConfig` yeniden atama, `.trimlook`, `TitleTemplate`, `list_snapshots` tam UI entegrasyonu), `BUL-21` (tam TR/EN i18n), `BUL-30` (SBOM ve `glow` tekilleştirme), `BUL-33` (erişilebilirlik `accesskit`).

---

## 7. Öncelik Sırasına Göre Uygulanabilir Düzeltme Planı ve Sonraki Prompt İş Bölümü

Bir sonraki düzeltme oturumunda aşama aşama ilerlemek için önerilen iş bölümü:

1. **Aşama A — Güvenlik, Symlink/Path & Veri Bütünlüğü (`S` Eforlu Hızlı Kazanımlar):**
   - Kapsam: `BUL-08`, `BUL-09`, `BUL-10`, `BUL-11`, `BUL-22`, `BUL-27`, `BUL-13`, `BUL-31`, `BUL-32`.
   - Çıktı: Güvenlik açığı/path traversal/symlink risklerinin sıfırlanması ve CI/release SHA-256 + lisans uyumu.
2. **Aşama B — Donma Önleme, Medya Motoru & Export Doğruluğu (`Yüksek` Öğeler):**
   - Kapsam: `BUL-01`, `BUL-02`, `BUL-03`, `BUL-04`, `BUL-05`, `BUL-06`, `BUL-07`, `BUL-14`.
   - Çıktı: UI thread'in FFmpeg/FFprobe'dan tamamen arındırılması, bounded `run_command_bounded`, video izlerinin ses export'u ve anında iptal edilebilir export runner.
3. **Aşama C — Performans, Render Paritesi & Komut/Cache Dayanıklılığı (`Orta` Öğeler):**
   - Kapsam: `BUL-12`, `BUL-16`, `BUL-17`, `BUL-18`, `BUL-19`, `BUL-20`, `BUL-23`, `BUL-24`, `BUL-25`, `BUL-26`, `BUL-28`, `BUL-34`.
   - Çıktı: Canvas/waveform/scope bellek ve CPU yükünün düşürülmesi, `FrameBufferPool` ve `AudioClock` iyileştirmeleri, `track.locked` ve `AutoSilenceCutCommand::undo` tutarlılığı.
4. **Aşama D — UI Bağlantıları, i18n, SBOM & Erişilebilirlik:**
   - Kapsam: `BUL-15`, `BUL-21`, `BUL-29`, `BUL-30`, `BUL-33`.
   - Çıktı: `KeymapConfig` + `FinalCutPro` + `.trimlook` + `TitleTemplate` + snapshot tarayıcısı UI bağlantısı, tam TR/EN yerelleştirme ve SBOM üretimi.
