# triM. — Özellik Envanteri, Kod Denetimi ve Geliştirme Yol Haritası

Bu rapor, triM. kod tabanının (`crates/trim-core`, `trim-commands`, `trim-timeline`, `trim-project`, `trim-media`, `trim-export`, `trim-ui`, `trim-app`) kaynak kodu satır satır incelenerek hazırlanmıştır. README ve web sitesi iddiaları bir kenara bırakılmış, her bir özelliğin kod içerisindeki gerçek varlığı, çalışma durumu ve mimari entegrasyonu belgelenmiştir.

---

## Bölüm 1: Mevcut Özellik Envanteri

### Durum Tanımları
* **Tam çalışıyor**: Özellik veri modeli, komut/undo mimarisi, oynatıcı motoru, dışa aktarma (FFmpeg filtergraph) ve arayüzde uçtan uca eksiksiz uygulanmış.
* **Kısmi**: Özelliğin temel altyapısı veya bir boyutu var, ancak önemli kullanım senaryoları eksik ya da sınırlı.
* **Sadece arayüzde var, arkası boş**: Arayüzde düğmesi, menüsü veya aracı var ancak tıklandığında hiçbir şey yapmıyor ya da sahte/durum metni basıyor.
* **Yok**: Kodda karşılığı bulunmuyor.
* **Belirsiz**: Kodda izi var ancak çalışma garantisi veya test doğrulaması yetersiz.

---

### 1. Kurgu ve Zaman Çizgisi

| Özellik | Durum | Kanıt (Dosya:Fonksiyon / Satır) | Notlar |
| :--- | :--- | :--- | :--- |
| **Kanal Yönetimi (Ekle/Sil)** | **Tam çalışıyor** | `crates/trim-ui/src/app/timeline_actions.rs:add_video_track`, `add_audio_track`; `crates/trim-commands/src/commands.rs:AddTrackCommand`, `RemoveTrackCommand` | Video ve ses kanalı ekleme/silme, akıllı video-ses eşleştirme (V1->A1) ve Undo/Redo tam desteklenir. |
| **Kanal Yeniden Sıralama** | **Yok** | `crates/trim-core/src/timeline.rs:tracks` | Kanalları dikeyde sürükleyerek sırasını değiştirme desteği yoktur; kanallar eklendikleri sırada kalır. |
| **Kesme / Jilet (Razor & Split)** | **Tam çalışıyor** | `crates/trim-ui/src/canvas.rs:1145` (`CanvasAction::RazorCut`); `crates/trim-commands/src/commands.rs:SplitClipCommand`, `BatchSplitClipCommand`; `crates/trim-ui/src/app/timeline_actions.rs:split_at_playhead` | C aracıyla tıklanan yerden ve Ctrl+K ile oynatma kafasından tekli/çoklu kanal bölme kusursuz çalışır. |
| **Dalgalanma Silme (Ripple Delete)** | **Tam çalışıyor** | `crates/trim-commands/src/commands.rs:RippleDeleteCommand`, `RippleTrimHeadCommand` (Q), `RippleTrimTailCommand` (W), `CloseGapCommand` | Boşlukları silme, klip başını/sonunu dalgalanmalı kırpma ve sağdaki klipleri sola kaydırma tam çalışır. |
| **Dalgalanma Düzenleme Aracı (Ripple Tool - B)** | **Tam çalışıyor** | `crates/trim-ui/src/toolbar.rs:EditTool::Ripple`; `crates/trim-ui/src/canvas.rs:DragMode::RippleIn/RippleOut`; `crates/trim-commands/src/commands.rs:RippleTrimEdgeCommand` | Araç çubuğunda (B) simgesi, sarı dalgalanma köşeli parantez imleçleri, klip baş/son kırpma ve tüm sonraki klipleri eşzamanlı kaydıran Undo/Redo destekli tam entegrasyon. |
| **Kaydırma Aracı (Slip Tool - Y)** | **Sadece arayüzde var, arkası boş** | `crates/trim-ui/src/toolbar.rs:12` (`EditTool::Slip`); `crates/trim-ui/src/canvas.rs:1106-1135`, `1144-1215` | Araç çubuğunda simgesi ve kısayolu (Y) mevcuttur; ancak `canvas.rs` fare mantığında klibin kaynak `source_in` değerini zaman çizgisinde sabitken kaydıran kod yazılmamıştır. |
| **Slide Aracı** | **Yok** | `crates/trim-ui/src/toolbar.rs:EditTool` | Araç çubuğunda veya kodda Slide aracı bulunmamaktadır. |
| **Hız Uzatma (Rate Stretch) Aracı** | **Yok** | `crates/trim-ui/src/toolbar.rs:EditTool`, `crates/trim-ui/src/canvas.rs` | Zaman çizgisinde klip ucundan tutup sürükleyerek hız değiştiren interaktif bir araç yoktur. (Hız yalnızca Inspector penceresinden değiştirilebilir). |
| **Manyetik Kenetlenme (Snapping)** | **Tam çalışıyor** | `crates/trim-core/src/timeline.rs:snap_time_with_playhead`; `crates/trim-ui/src/canvas.rs:find_snap_point` | Oynatma kafasına, klip sınırlarına ve işaretçilere kılavuz çizgileriyle kenetlenme tam çalışır. |
| **Görsel İşaretçiler (Markers)** | **Tam çalışıyor** | `crates/trim-core/src/marker.rs:Marker`; `crates/trim-commands/src/commands.rs:AddMarkerCommand`, `RemoveMarkerCommand`; `crates/trim-ui/src/ruler.rs:render_time_ruler` | Zaman cetvelinde renkli bayraklar, etiketler ve tıklayınca o zamana atlama tam çalışır. |
| **Çoklu Seçim (Multi-Select)** | **Tam çalışıyor** | `crates/trim-ui/src/canvas.rs:find_clip_at_pos`; `crates/trim-commands/src/commands.rs:MoveMultipleClipsCommand`, `RemoveMultipleClipsCommand`; `EditTool::TrackSelect` | Shift/Ctrl ile çoklu klip seçme, ileri iz seçme aracı (A) ve toplu taşıma/silme tam çalışır. |
| **Kopyala - Yapıştır (Copy/Paste)** | **Yok** | `crates/trim-ui/src/app.rs` | Seçili klipleri panoya (clipboard) kopyalama ve oynatma kafasına yapıştırma özelliği bulunmamaktadır. |
| **Kanal Kilitleme, Gizleme, Mute, Solo** | **Tam çalışıyor** | `crates/trim-core/src/track.rs:Track`; `crates/trim-ui/src/track_header.rs:render_track_header` | Kilitli kanalda düzenleme engellenir, sessize alınan kanallar oynatıcı ve dışa aktarmada filtrelenir. |
| **Kanal Yeniden Boyutlandırma (Track Height)** | **Tam çalışıyor** | `crates/trim-ui/src/track_header.rs:TrackHeaderAction::ResizeTrack`; `crates/trim-ui/src/canvas.rs:TimelineCanvasState::set_track_height` | Kanal başlıklarının alt kenarından tutularak kanal yüksekliği ayarlanabilir, sıfırlanabilir. |

---

### 2. Video İşleme ve Efektler

| Özellik | Durum | Kanıt (Dosya:Fonksiyon / Satır) | Notlar |
| :--- | :--- | :--- | :--- |
| **Video Geçişleri (Transitions)** | **Tam çalışıyor** | `crates/trim-core/src/clip.rs:transition_in`, `transition_out`; `crates/trim-ui/src/inspector.rs:1160`; `crates/trim-media/src/video.rs:1276`; `crates/trim-ui/src/gpu_compositor.rs:252`; `crates/trim-export/src/filtergraph.rs:468` | Cross Dissolve, Dip to Black, Dip to White, Wipe Left/Right, Slide ve Zoom geçişleri önizleme, GPU gölgelendirici ve dışa aktarmada çalışır. |
| **Hız Değişimi (Speed Change)** | **Tam çalışıyor** | `crates/trim-core/src/clip.rs:speed`; `crates/trim-commands/src/commands.rs:ChangeClipSpeedCommand`; `crates/trim-media/src/video.rs:PlaybackStreamer`; `crates/trim-export/src/filtergraph.rs:get_atempo_filters` | 0.1x ile 10.0x arası hız değişimi; videoda `setpts`, seste kademeli `atempo` zinciri ile tam desteklenir. |
| **Ters Oynatma (Reverse Playback)** | **Tam çalışıyor** | `crates/trim-core/src/clip.rs:reverse`; `crates/trim-media/src/video.rs:163` (2 saniyelik bellek tamponlu ters oynatıcı); `crates/trim-export/src/filtergraph.rs:159` (`reverse` & `areverse`) | Önizlemede FFmpeg bellek şişmesini engelleyen özel ters tamponlayıcı, renderda `reverse`/`areverse` tam çalışır. |
| **Kırpma (Crop)** | **Tam çalışıyor** | `crates/trim-core/src/clip.rs:crop_left/right/top/bottom`; `crates/trim-commands/src/commands.rs:SetClipCropCommand`; `crates/trim-ui/src/inspector.rs`; `crates/trim-ui/src/gpu_compositor.rs:u_crop`; `crates/trim-export/src/filtergraph.rs:crop` | Sol, sağ, üst, alt kırpma kaydırıcıları; GPU shader UV maskeleme ve FFmpeg render dışa aktarımı tam çalışır. |
| **Dönüştürme (Transform: Konum, Ölçek, Döndürme, Opaklık)** | **Tam çalışıyor** | `crates/trim-core/src/clip.rs:pos_x, pos_y, scale, rotation, opacity`; `crates/trim-media/src/video.rs:1350`; `crates/trim-ui/src/gpu_compositor.rs:270`; `crates/trim-export/src/filtergraph.rs:166` | Önizlemede GPU/CPU matris dönüşümü ve FFmpeg `scale`/`rotate`/`overlay` ile dışa aktarım tam çalışır. |
| **Bindirme (PiP - Picture-in-Picture)** | **Tam çalışıyor** | `crates/trim-core/src/timeline.rs:get_visible_video_clips_at`; `crates/trim-media/src/video.rs:collect_timeline_layers` | Çok kanallı yapıda V2, V1'in üzerine ölçeklenip konumlandırılarak katmanlanır. |
| **Harmanlama Modları (Blend Modes)** | **Yok** | `crates/trim-media/src/video.rs:2064`; `crates/trim-ui/src/gpu_compositor.rs:812` | Yalnızca standart alfa harmanlama (Normal) vardır; Multiply, Screen, Overlay gibi modlar yoktur. |
| **Anahtar Kareler (Keyframes)** | **Tam çalışıyor** | `crates/trim-core/src/keyframe.rs:interpolate_keyframes`; `crates/trim-ui/src/inspector.rs:1600` (`render_kf_btn`); `crates/trim-export/src/filtergraph.rs:keyframe_expr` | Konum, ölçek, döndürme ve opaklık için Linear, Hold ve EaseInOut anahtar kare enterpolasyonu ve dışa aktarma matematiksel ifadesi tam çalışır. |
| **Maske (Masking)** | **Yok** | `crates/trim-core`, `crates/trim-ui` | Serbest çizim veya geometrik maskeleme özelliği kodda bulunmamaktadır. |
| **Stabilizasyon (Video Sabitleme)** | **Yok** | `crates/trim-media` | Kamera titremesini gideren `vidstab` veya benzeri sabitleme analizi yoktur. |
| **Chroma Key (Yeşil/Mavi Perde)** | **Tam çalışıyor** | `crates/trim-core/src/clip.rs:ChromaKeyConfig`; `crates/trim-media/src/video.rs:render_chromakey`; `crates/trim-ui/src/gpu_compositor.rs:120`; `crates/trim-export/src/filtergraph.rs:178` | Renk seçimi, benzerlik ve yumuşaklık toleranslarıyla CPU/GPU önizleme ve FFmpeg filtresi tam çalışır. |
| **3 Yönlü Renk Çarkları (Lift, Gamma, Gain)** | **Tam çalışıyor** | `crates/trim-core/src/clip.rs:ColorWheelsConfig`; `crates/trim-ui/src/color_grading.rs:render_color_grading_panel`; `crates/trim-media/src/video.rs:render_color_wheels`; `crates/trim-ui/src/gpu_compositor.rs:u_lift/gamma/gain` | Renk çarkları etkileşimi, GPU gölgelendirici tonlaması ve FFmpeg `colorbalance` dönüşümü tam çalışır. |
| **Renk Eğrileri (RGB/Luma Curves)** | **Yok** | `crates/trim-core`, `crates/trim-ui` | Kullanıcının kontrol noktalarıyla müdahale edebileceği etkileşimli renk eğrileri aracı yoktur. |
| **3D LUT (.cube ve Hazır Profiller)** | **Tam çalışıyor** | `crates/trim-core/src/clip.rs:LutPreset`; `crates/trim-media/src/video.rs:parse_cube_lut`; `crates/trim-export/src/filtergraph.rs:188` (`lut3d`); `crates/trim-ui/src/inspector.rs:1050` | 9 adet sinematik hazır ayar ve harici `.cube` 3D LUT yükleme CPU/GPU ve FFmpeg'de tam çalışır. |
| **Video Kapsamları (Scopes: Histogram, Vectorscope)** | **Yok** | `crates/trim-ui`, `crates/trim-media` | Renk düzeltme için canlı histogram, vektörskop veya RGB parade araçları kodda yer almamaktadır. |

---

### 3. Ses İşleme

| Özellik | Durum | Kanıt (Dosya:Fonksiyon / Satır) | Notlar |
| :--- | :--- | :--- | :--- |
| **Klip Ses Seviyesi (Volume / Gain)** | **Tam çalışıyor** | `crates/trim-core/src/clip.rs:volume`; `crates/trim-commands/src/commands.rs:SetClipVolumeCommand`; `crates/trim-media/src/audio.rs:AudioMixer`; `crates/trim-ui/src/canvas.rs:VolumeRubberband` | Inspector'dan dB/yüzde ve zaman çizgisindeki klip üzerinden yatay lastik bant (rubberband) ile ses ayarı tam çalışır. |
| **Ses Otomasyon Zarfı (Keyframed Volume Envelope)** | **Tam çalışıyor** | `crates/trim-commands/src/commands.rs:SetClipKeyframesCommand`; `crates/trim-ui/src/canvas.rs:VolumeRubberband`; `crates/trim-media/src/audio.rs:mix_raw_window`; `crates/trim-export/src/filtergraph.rs:volume=eval=frame` | Zaman çizgisinde Pen aracı ('P') veya Ctrl+Tık ile elmas anahtar kare ekleme, Alt+Tık ile silme, dikey sürükleme ile dB kazancı ayarlama, DSP parça bazlı enterpolasyon ve FFmpeg render tam çalışır. |
| **Ses Geçişleri (Audio Crossfade)** | **Kısmi** | `crates/trim-export/src/filtergraph.rs:606` (`afade`); `crates/trim-media/src/audio.rs:233` | Klip başı ve sonuna doğrusal kazanç geçişi (`afade`) uygulanabilir; ancak bitişik iki ses klibi arasına otomatik eşit-güç (equal-power) crossfade düğümü eklenemez. |
| **3 Bant Parametrik Ekolayzır (EQ)** | **Tam çalışıyor** | `crates/trim-core/src/clip.rs:AudioEqPreset`; `crates/trim-media/src/audio.rs:BiquadFilter`; `crates/trim-export/src/filtergraph.rs:equalizer` | Bas, tiz, vokal netliği, radyo sesi gibi biquad peaking/shelving filtreleri gerçek zamanlı ve renderda tam çalışır. |
| **Gürültü Kapısı (Noise Gate)** | **Tam çalışıyor** | `crates/trim-core/src/clip.rs:noise_gate`; `crates/trim-media/src/audio.rs:apply_noise_gate`; `crates/trim-export/src/filtergraph.rs:agate` | Eşik altındaki ortam fısıltılarını kesen yazılımsal kapı ve FFmpeg `agate` filtresi tam çalışır. |
| **Gelişmiş Gürültü Azaltma (Spectral/AI Denoiser)** | **Kısmi** | `crates/trim-core/src/clip.rs:DenoiseGate`; `crates/trim-export/src/filtergraph.rs:594` | Yalnızca yüksek/alçak geçiren filtre + noise gate (`highpass=80,lowpass=12000,agate`) mevcuttur; RNNoise veya spektral gürültü profili çıkarma yoktur. |
| **Otomatik Kısılma (Audio Ducking)** | **Yok** | `crates/trim-media/src/audio.rs` | Konuşma başladığında müzik kanalının sesini otomatik kısan sidechain ducking mekanizması yoktur. |
| **Ses Seviyesi Normalizasyonu (Loudness Normalization)** | **Tam çalışıyor** | `crates/trim-export/src/filtergraph.rs:loudnorm`; `crates/trim-media/src/dsp/limiter.rs:TruePeakLimiter`; `crates/trim-ui/src/dialogs/export_dialog.rs` | EBU R128 (-23 LUFS / -14 LUFS) çift geçişli normalizasyon ve oynatıcıda 5 ms tuğla duvar sınırlayıcı tam çalışır. |
| **Mikrofonla Ses Kaydı (Voiceover)** | **Tam çalışıyor** | `crates/trim-media/src/voiceover.rs:VoiceoverRecorder`; `crates/trim-ui/src/app.rs:start_voiceover`, `finish_voiceover` | Kanal başlığındaki kayıt düğmesiyle mikrofondan kayıt alıp klibi doğrudan zaman çizgisine yerleştirme tam çalışır. |
| **Ses Dalga Formu (Waveform Visualizer)** | **Tam çalışıyor** | `crates/trim-media/src/waveform.rs:WaveformGenerator`; `crates/trim-ui/src/canvas.rs:draw_waveform` | Arka plan iş parçacığında tepe/RMS değerlerini çıkarıp diskte önbelleğe alan ve zaman çizgisinde çizen yapı tam çalışır. |

---

### 4. Metin ve Altyazı

| Özellik | Durum | Kanıt (Dosya:Fonksiyon / Satır) | Notlar |
| :--- | :--- | :--- | :--- |
| **Başlık ve Metin Katmanı (Titles)** | **Tam çalışıyor** | `crates/trim-core/src/clip.rs:TextOverlay`; `crates/trim-ui/src/monitor.rs:handle_monitor_text_click`; `crates/trim-export/src/filtergraph.rs:drawtext` | Monitör üzerinden sürükleyip bırakarak metin konumlandırma, yazı tipi boyutu, rengi ve gölgesi tam çalışır. |
| **Animasyonlu Başlık Şablonları (Motion Titles)** | **Tam çalışıyor** | `crates/trim-core/src/clip.rs:MotionTitleStyle` (Daktilo, Alt Bant, Sinematik Zoom, Kayan Yazı, Kutu Açılışı); `crates/trim-ui/src/gpu_compositor.rs`; `crates/trim-export/src/filtergraph.rs` | Hem GPU canlı önizlemesinde hem de FFmpeg `drawtext` matematiksel ifadelerinde tam çalışır. |
| **Altyazı İçe/Dışa Aktarma (SRT/VTT)** | **Tam çalışıyor** | `crates/trim-core/src/subtitles.rs:parse_srt`, `parse_vtt`, `export_srt`; `crates/trim-ui/src/app/project_io.rs:import_subtitles_from_path`, `export_subtitles_dialog` | `.srt` ve `.vtt` dosyalarını zaman çizgisine bağımsız metin klipleri olarak dökme ve zaman çizgisini `.srt` olarak dışa aktarma tam çalışır. |
| **Yapay Zeka ile Otomatik Altyazı (Whisper)** | **Tam çalışıyor** | `crates/trim-media/src/transcribe.rs:WhisperTranscriber`; `crates/trim-ui/src/dialogs/transcribe_dialog.rs` | Sistemde kurulu `whisper`/`whisper-cli` aracını tespit ederek sesi yazıya dökme ve zaman çizgisine ekleme tam çalışır. |
| **Altyazı Stilleri ve Şablon Yönetimi** | **Kısmi** | `crates/trim-ui/src/inspector.rs:1350` | Her metin klibi için bağımsız font/renk/arka plan kutusu seçilebilir; ancak tüm altyazılara tek tıkla stil şablonu uygulayan global stil yöneticisi yoktur. |

---

### 5. Proje Yönetimi ve Dosya İşlemleri

| Özellik | Durum | Kanıt (Dosya:Fonksiyon / Satır) | Notlar |
| :--- | :--- | :--- | :--- |
| **Medya Havuzu Klasörleri (Bins) ve Etiketler** | **Tam çalışıyor** | `crates/trim-core/src/project.rs:MediaBinFolder`; `crates/trim-ui/src/media_bin.rs:render_media_bin`; `crates/trim-core/src/clip.rs:ColorLabel` | Klasör oluşturma, içine taşıma, ekmek kırıntısı (breadcrumb) navigasyonu ve 6 renkli klip etiketleri tam çalışır. |
| **Medya Arama ve Filtreleme** | **Tam çalışıyor** | `crates/trim-ui/src/media_bin.rs:48` | Gerçek zamanlı arama kutusu ve Video/Ses/Görsel filtreleme butonları tam çalışır. |
| **İç İçe Sekans (Nested Sequences / Multicam)** | **Tam çalışıyor** | `crates/trim-core/src/sequence.rs`; `crates/trim-core/src/project.rs:sequences`; `crates/trim-ui/src/app.rs:seq_tabs`; `crates/trim-ui/src/media_bin.rs:sequence_cards` | Çoklu sekans sekmeleri, klonlama/kapatma, sekansı Medya Havuzundan zaman çizgisine iç içe (nested) klip olarak ekleme ve özyinelemeli (flattened timeline) render tam çalışır. |
| **Eksik Medyayı Yeniden Bağlama (Relink)** | **Yok** | `crates/trim-project`, `crates/trim-ui` | Projede kullanılan bir medyanın yeri değiştiğinde veya silindiğinde kullanıcıya "Medya Çevrimdışı" uyarısı verip yeni konumu gösteren bir relink penceresi yoktur. |
| **Proje Şablonları (Templates)** | **Tam çalışıyor** | `crates/trim-ui/src/dialogs/welcome_dialog.rs:FORMAT_PRESETS` | Karşılama penceresinde 1080p, 4K, Reels/Shorts (9:16) ve Kare (1:1) format şablonlarıyla yeni proje açma tam çalışır. |
| **Otomatik Kaydetme ve Çökme Kurtarma** | **Tam çalışıyor** | `crates/trim-project/src/autosave.rs:AutosaveManager`; `crates/trim-project/src/lib.rs:save_snapshot` | Periyodik arka plan kaydı (`.trim.autosave`) ve açılışta kurtarma teklifi tam çalışır. |
| **Taşınabilir Proje Paketleme (Bundle Project)** | **Tam çalışıyor** | `crates/trim-project/src/bundle.rs:bundle_project`; `crates/trim-ui/src/app/project_io.rs:export_bundle_dialog` | Projede kullanılan tüm harici ses, video, görsel ve LUT dosyalarını tek klasörde toplayıp bağıl yollarla paketleme tam çalışır. |
| **Geriye Dönük Format Uyumu** | **Tam çalışıyor** | `crates/trim-core/src/project.rs:Project`; `crates/trim-project/src/lib.rs:load_project` | Eski `.trim` ve `.cutline` dosyalarını eksik alanları serde varsayılanlarıyla doldurarak açma tam çalışır. |

---

### 6. Dışa Aktarma (Export)

| Özellik | Durum | Kanıt (Dosya:Fonksiyon / Satır) | Notlar |
| :--- | :--- | :--- | :--- |
| **Hazır Ayarlar (Export Presets)** | **Tam çalışıyor** | `crates/trim-export/src/presets.rs:get_default_presets` | YouTube 1080p, 4K Master, Reels 9:16, ProRes 422 HQ, DNxHR, WebM VP9 ve Yalnızca Ses profilleri tam çalışır. |
| **Özel Dışa Aktarma Ayarları (Custom Settings)** | **Kısmi** | `crates/trim-ui/src/dialogs/export_dialog.rs:80` | Çözünürlük, FPS, CRF kalite değeri ve donanım kodlayıcı elle özelleştirilebilir; ancak kullanıcının kendi özel hazır ayarını kaydedip saklayabileceği bir profil yöneticisi yoktur. |
| **Render Kuyruğu (Render Queue / Batch Export)** | **Tam çalışıyor** | `crates/trim-export/src/queue.rs:RenderQueue`; `crates/trim-ui/src/dialogs/render_queue_dialog.rs`; `crates/trim-ui/src/dialogs/export_dialog.rs:AddToQueue` | Dışa aktarma penceresinden "Kuyruğa Ekle" butonu, arka plan sıralı iş yürütücüsü, yüzde ilerleme çubukları, iptal/temizleme ve klasör açma modalı tam çalışır. |
| **Aralık (In / Out) Dışa Aktarma** | **Sadece arayüzde var, arkası boş** | `crates/trim-ui/src/monitor.rs:20` (`MonitorAction::MarkIn`, `MarkOut`); `crates/trim-ui/src/app.rs:474-479`; `crates/trim-export/src/filtergraph.rs` | Monitördeki `{` ve `}` (In/Out) butonları sadece durum çubuğuna yazı yazar (`Sekans Giriş: ...`). Zaman cetvelinde aralık çizilmez ve dışa aktarma motoru her zaman tüm zaman çizgisini baştan sona (0..duration) render eder. |
| **Donanım Kodlayıcı Tespiti (Hardware Acceleration)** | **Tam çalışıyor** | `crates/trim-export/src/presets.rs:detect_available_encoders` | `ffmpeg -encoders` çıktısını analiz ederek NVIDIA NVENC, Intel QSV, AMD AMF ve Linux VAAPI'yi otomatik seçme tam çalışır. |
| **EDL ve FCPXML Dışa / İçe Aktarımı** | **Tam çalışıyor** | `crates/trim-export/src/edl.rs`; `crates/trim-export/src/fcpxml.rs` (v1.8/v1.9) | DaVinci Resolve ve Premiere Pro ile sekans alışverişi iki yönlü tam çalışır. |
| **Ayrı Ses Kanalları Çıktısı (Stem Export)** | **Yok** | `crates/trim-export/src/filtergraph.rs` | Diyalog, Müzik ve Efekt kanallarını ayrı ayrı WAV/MP3 dosyaları olarak tek seferde basma özelliği yoktur. |
| **Tek Kare Dışa Aktarma (Still Frame Snapshot)** | **Tam çalışıyor** | `crates/trim-ui/src/app/project_io.rs:export_current_frame` | Monitördeki kamera simgesiyle mevcut kare PNG olarak kaydedilir ve otomatik olarak medya havuzuna eklenir. |

---

### 7. Performans ve Oynatma

| Özellik | Durum | Kanıt (Dosya:Fonksiyon / Satır) | Notlar |
| :--- | :--- | :--- | :--- |
| **Proxy (Vekil Medya) Oluşturma ve Düzenleme** | **Tam çalışıyor** | `crates/trim-media/src/proxy.rs:ProxyGenerator`; `crates/trim-media/src/video.rs:695` | Ağır 4K videolar için arka planda 720p H.264 vekil dosya üretme ve oynatıcıda otomatik vekil medyadan beslenme tam çalışır. |
| **Bellek ve Disk Önbelleği (Caching)** | **Tam çalışıyor** | `crates/trim-media/src/video.rs:FrameCacheMap` (60 karelik LRU önbellek); `crates/trim-media/src/audio.rs:AudioMixerPcmCache` (500 MB sınır); `crates/trim-media/src/waveform.rs` | Kare, ses PCM ve dalga formu önbellekleri bellek sınırlarıyla korunarak akıcı oynatma sağlar. |
| **Önizleme Kalitesi Seçenekleri (Tam, 1/2, 1/4)** | **Yok** | `crates/trim-media/src/video.rs:calculate_preview_size` | Önizleme boyutu en-boy oranına göre sabit ~640x360 olarak hesaplanır. Kullanıcının oynatıcı çözünürlüğünü (Full, 1/2, 1/4) seçebileceği bir arayüz/motor ayarı yoktur. |

---

### 8. Arayüz ve Kullanıcı Deneyimi

| Özellik | Durum | Kanıt (Dosya:Fonksiyon / Satır) | Notlar |
| :--- | :--- | :--- | :--- |
| **Kısayol Profilleri** | **Tam çalışıyor** | `crates/trim-ui/src/shortcuts.rs:ShortcutProfile` | Premiere Pro, DaVinci Resolve ve Final Cut Pro tuş dizilimleri arasında geçiş tam çalışır. |
| **Panel Düzenleri ve Çalışma Alanları (Workspaces)** | **Yok** | `crates/trim-ui/src/app.rs` | Arayüz sabit Premiere 4-Quadrant düzenindedir. Ayrılabilir (undockable) paneller veya Renk/Ses için ayrı çalışma alanı sekmeleri yoktur. |
| **Tema Seçenekleri** | **Kısmi** | `crates/trim-ui/src/theme.rs:setup_theme` | Özel karanlık NLE teması sabittir; açık tema veya özel renk teması seçeneği yoktur. |
| **Yerelleştirme (TR / EN Seçimi)** | **Yok** | `crates/trim-ui/src/app.rs` | Masaüstü uygulamasının tüm arayüzü sabit Türkçe metinlerle derlenmiştir. Kod tabanında dinamik dil değiştirme altyapısı yoktur (Docs sitesinde iki dil mevcuttur). |
| **Geri Al / Yinele (Undo/Redo) Kapsamı** | **Kısmi** | `crates/trim-commands/src/commands.rs` (24 adet komut) vs `crates/trim-ui/src/inspector.rs` | Kliplerin taşınması, bölünmesi, silinmesi, kanal ekleme/silme, hız ve lastik bant ses seviyesi eksiksiz geri alınabilir. **Ancak:** Inspector panelindeki Konum, Ölçek, Döndürme, Opaklık, 3D LUT seçimi, Renk Çarkları, EQ ayarları, Anahtar Kare ekleme/silme ve Metin düzenlemeleri doğrudan veri mutasyonudur; Undo yığınına girmez ve Ctrl+Z ile geri alınamaz! |

---

### 9. Platform ve Dağıtım

| Platform / Paket | Durum | Kanıt | Notlar |
| :--- | :--- | :--- | :--- |
| **Linux (x86_64)** | **Tam çalışıyor** | PipeWire/PulseAudio/ALSA ses, OpenGL GLX/EGL glow render | Yerel Linux masaüstü ortamlarında test edilmiş ve çalışmaktadır. |
| **Windows (x86_64)** | **Tam çalışıyor** | `.github/workflows/release-linux.yml:156` (`trim.exe`), `install.ps1` | DirectShow/WASAPI ses ve Windows ikili derlemesi tam desteklenir. |
| **Flatpak Dağıtımı** | **Tam çalışıyor** | `flatpak/io.github.mozcelik14.triM.yml`, `io.github.mozcelik14.triM.metainfo.xml` | Flathub manifesti ve izinleri hazırdır. |
| **AppImage, DEB, RPM** | **Tam çalışıyor** | `.github/workflows/release-linux.yml` | GitHub Actions üzerinde otomatik paketlenmektedir. |
| **macOS Durumu** | **Yok / Belirsiz** | Kod tabanı ve CI | macOS için GitHub derleme iş akışı yoktur, Apple Silicon / CoreAudio / Metal gölgelendirici optimizasyonu yapılmamıştır. Resmi olarak desteklenmemektedir. |

---

## README ve Web Sitesi ile Kod Arasındaki Tutarsızlıklar

Geliştiricinin README'de ve `docs/` sitesinde belirttiği iddialardan kodda karşılığı olmayan veya yanıltıcı olan maddeler:

1. **"Kaydırma (Slip) ve Dalgalanma (Ripple) Araçları":**
   * *README İddiası:* "Jilet (Razor), dalgalanma (Ripple), kaydırma (Slip) ve uzatma (Rate Stretch) araçları"
   * *Kod Gerçeği:* Araç çubuğunda Ripple (B) ve Slip (Y) butonları vardır ancak `crates/trim-ui/src/canvas.rs` içinde bu araçlar seçildiğinde hiçbir özel davranış kodlanmamıştır; normal `Select` aracı gibi çalışır.
2. **"Hız Uzatma (Rate Stretch) Aracı":**
   * *README İddiası:* Temel özellikler arasında "uzatma (Rate Stretch) aracı" sayılmaktadır.
   * *Kod Gerçeği:* Zaman çizgisinde klip kenarından sürükleyerek süreyi uzatıp/kısaltırken hızı değiştiren bir araç veya kod tabanında böyle bir enum dahi yoktur. Klibin hızı sadece Inspector'deki sayısal kutudan değiştirilebilir.
3. **"Sekans 01" Sekmesi ve Çoklu Sekans İntibası:**
   * *Site/Arayüz İntibası:* Zaman çizgisinin üstünde Premiere Pro sekme tarzında "Sekans 01" düğmesi yer alır.
   * *Kod Gerçeği:* Bu düğme tıklama olayına bağlı değildir. `Project` yapısı yalnızca tek bir `timeline` nesnesi tutar; birden fazla sekans veya iç içe sekans açma imkanı yoktur.
4. **Giriş / Çıkış (In/Out) İşaretleme:**
   * *Arayüz/Kısayol İntibası:* Monitör altında Mark In `{` ve Mark Out `}` butonları vardır ve kısayollar listesinde yer alır.
   * *Kod Gerçeği:* Bu butonlar yalnızca durum çubuğuna bir metin yazar. Ne zaman çizgisinde bir seçim bölgesi oluşturur ne de Dışa Aktarma ekranında o aralığı kesip render eder. Dışa aktarma her zaman tüm projeyi render eder.
5. **Kırpma / Dönüştürme İddiası:**
   * *İddia:* Kurgu araçlarında kırpma (crop) ima edilmektedir.
   * *Kod Gerçeği:* Klip kenarlarından süreyi kısaltan "Trim" vardır ancak video çerçevesini kesen "Crop" efekti kodda hiç yoktur.

---

## Bölüm 2: Eksikler ve Mimari İyileştirme Önerileri

Bu öneriler, triM'in mevcut crate ayrımına (`trim-core`, `trim-commands`, `trim-media`, `trim-export`, `trim-ui`), komut/undo mimarisine ve FFmpeg filtergraph boru hattına tam uyum sağlayacak şekilde tasarlanmıştır.

---

### Kategori A: Hızlı Kazanımlar (Küçük Zorluk, Belirgin Fayda)

#### 1. Kurgu Kliplerini Kopyalama ve Yapıştırma (Clipboard Copy/Paste)
* **Ne olduğu:** Seçili klibin/kliplerin (Ctrl+C) geçici bir panoya kopyalanması ve oynatma kafasının bulunduğu noktada hedef kanala (Ctrl+V) yapıştırılması.
* **Kullanıcıya Faydası:** Aynı grafiği, sesi veya video parçasını zaman çizgisinde defalarca kullanmak isteyen kurgucunun en temel ihtiyacıdır.
* **Mevcut Koda Nasıl Oturur:**
  * `trim-ui/src/app.rs`: `TrimApp` içine `clipboard_clips: Vec<Clip>` eklenir.
  * Ctrl+C basıldığında seçili klipler `clone()` edilerek panoya alınır.
  * Ctrl+V basıldığında kliplerin `timeline_in` değeri oynatma kafasına (`current_time`) göre ötelenir, yeni UUID'ler üretilir ve mevcut `AddClipCommand` veya `MacroCommand` çağrılarak Undo/Redo yığınına eklenir.
* **Zorluk:** **Küçük**. Komut altyapısı ve klip klonlama zaten mevcuttur.
* **Riskler:** Yok. Proje formatı değişmez, var olan komutlar kullanılır.
* **Bağımlılık:** Yeni bağımlılık gerekmez.

#### 2. Gerçek Giriş/Çıkış (In/Out) Belirleme ve Seçili Aralık Render Desteği
* **Ne olduğu:** Monitördeki ve klavyedeki (I / O) butonlarının gerçekten bir zaman aralığı kaydetmesi, bu aralığın zaman cetvelinde mavi gölgeli bir bantla gösterilmesi ve dışa aktarma penceresinde "Yalnızca Seçili Aralığı Dışa Aktar" seçeneğinin sunulması.
* **Kullanıcıya Faydası:** 10 dakikalık bir videonun sadece 30 saniyelik bir kesitini hızlıca test etmek veya sosyal medyaya atmak isteyen kullanıcı için hayatidir.
* **Mevcut Koda Nasıl Oturur:**
  * `trim-core/src/timeline.rs`: `TimelineModel` içine `in_point: Option<f64>` ve `out_point: Option<f64>` eklenir (`#[serde(default)]`).
  * `trim-ui/src/app.rs`: `MonitorAction::MarkIn` ve `MarkOut` bu değerleri günceller.
  * `trim-ui/src/ruler.rs`: Cetvelde `in_point` ve `out_point` arası yarı saydam açık mavi renkle taranır.
  * `trim-export/src/filtergraph.rs`: FFmpeg argümanlarına `-ss <in_point> -t <duration>` eklenir.
* **Zorluk:** **Küçük**. Modelde iki alan, cetvelde 10 satırlık çizim ve FFmpeg komutuna iki parametre eklenmesinden ibarettir.
* **Riskler:** Çok düşük; proje dosyasında geriye dönük uyumluluk bozulmaz.
* **Bağımlılık:** Yeni bağımlılık gerekmez.

#### 3. Araç Çubuğundaki Slip (Kaydırma) Aracının Canlıya Alınması
* **Ne olduğu:** Slip aracı (Y) seçiliyken bir klip tıklandığında ve sürüklendiğinde, klibin zaman çizgisindeki başlangıç ve bitiş zamanı sabit kalırken kaynak dosya başlangıcı (`source_in`) yatay sürükleme miktarı kadar kaydırılır.
* **Kullanıcıya Faydası:** Zamanlama ritmini bozmadan, klip içinde görünen aksiyon anını kusursuz şekilde hizalamayı sağlar.
* **Mevcut Koda Nasıl Oturur:**
  * `trim-ui/src/canvas.rs`: `match state.active_tool` içine `EditTool::Slip` dalı eklenir.
  * Klip tıklandığında `DragMode::Slip` başlatılır; yatay fare deltasından saniye cinsinden kayma hesaplanır.
  * Sürükleme bittiğinde mevcut `TrimClipCommand` (aynı klip ID'si ve yeni `source_in` ile) çağrılır.
* **Zorluk:** **Küçük**. Altyapıda `TrimClipCommand` zaten `source_in` değiştirebilmektedir; yalnızca UI sürükleme olayının bağlanması gerekir.
* **Riskler:** `source_in` değerinin 0 ile `media.duration - clip.duration` aralığına sınırlandırılması (clamp) dikkatle yapılmalıdır.
* **Bağımlılık:** Yok.

#### 4. Tek Kare Dışa Aktarmaya (Snapshot) Kaydetme Penceresi Eklenmesi
* **Ne olduğu:** Monitördeki kamera simgesine tıklandığında karenin doğrudan mevcut çalışma dizinine rastgele bir isimle kaydedilmesi yerine `FileDialogBridge` ile kullanıcıya dosya adı ve kayıt yeri sorulması.
* **Kullanıcıya Faydası:** Kullanıcı videodan aldığı küçük resmi (thumbnail) doğrudan Masaüstüne veya istediği klasöre istediği isimle kaydedebilir.
* **Mevcut Koda Nasıl Oturur:**
  * `trim-ui/src/app/project_io.rs:export_current_frame` içindeki doğrudan dosya yazma kodu, projedeki mevcut `FileDialogBridge::save_file("Kareyi PNG Olarak Kaydet")` yapısına devredilir.
* **Zorluk:** **Küçük**. Projede `FileDialogBridge` zaten mevcuttur.
* **Riskler:** Yok.
* **Bağımlılık:** Yok.

---

### Kategori B: Orta Vadeli Öneriler

#### 5. Inspector Düzenlemelerine Undo/Redo Desteği Kazandırılması [✅ TAMAMLANDI]
* **Ne olduğu:** Inspector panelinde klibin Konumu, Ölçeği, Opaklığı, 3D LUT'u veya Ses Düzeyi değiştirildiğinde bu değişikliğin `SetClipTransformCommand` veya `SetClipPropertyCommand` olarak Undo yığınına itilmesi.
* **Kullanıcıya Faydası:** Kullanıcı yanlışlıkla bir klibin ölçeğini veya rengini bozduğunda Ctrl+Z ile eski haline dönebilir.
* **Mevcut Koda Nasıl Oturur:**
  * `trim-commands/src/commands.rs`: `SetClipTransformCommand` ve `SetClipColorGradingCommand` tanımlanır (`old_state` ve `new_state` saklanır).
  * `trim-ui/src/inspector.rs`: Kaydırıcılar (sliders) sürüklenirken gerçek zamanlı önizleme için klip doğrudan güncellenir; kullanıcı kaydırıcıyı bıraktığında (`response.drag_stopped()` veya `lost_focus()`) komut UndoStack'e gönderilir.
* **Zorluk:** **Orta**. egui immediate-mode arayüzünde kaydırma esnasında her karede değil, yalnızca fare bırakıldığında tek bir komut üretme lojiği kurulmalıdır.
* **Riskler:** Geri alma yığınının gereksiz ufak ara değerlerle şişmesini önlemek için "drag_stopped" kontrolü şarttır.
* **Bağımlılık:** Yok.

#### 6. Çok Noktalı Ses Seviyesi Otomasyonu (Volume Automation Envelope) [✅ TAMAMLANDI]
* **Ne olduğu:** Klibin üzerindeki ses lastik bandına tıklanarak birden çok ses anahtar karesi noktası eklenebilmesi, bu sayede klibin belirli anlarında sesin kısılıp tekrar açılabilmesi.
* **Kullanıcıya Faydası:** Arka plan müziğini konuşma anlarında manuel olarak kısmak (ducking) veya ani patlamaları yumuşatmak için zorunludur.
* **Mevcut Koda Nasıl Oturur:**
  * `trim-core/src/clip.rs`: `keyframes.get("volume")` zaten veri modeli seviyesinde desteklenebilir durumdadır.
  * `trim-media/src/audio.rs:AudioMixer`: PCM parçalarını işlerken `interpolate_keyframes` çağrılarak zaman bağımlı kazanç uygulanır.
  * `trim-export/src/filtergraph.rs`: FFmpeg `volume=eval=frame:volume='...'` matematiksel ifadesi üretilir.
  * `trim-ui/src/canvas.rs`: Ses lastik bandı üzerine Ctrl+Tık ile nokta ekleme ve noktaları dikey sürükleme etkileşimi eklenir.
* **Zorluk:** **Orta**. Arayüzde noktaların çizimi ve ses mikserinde örnek başına kazanç enterpolasyonu dikkatli matematik gerektirir.
* **Riskler:** `audio.rs` gerçek zamanlı oynatma döngüsünde enterpolasyonun CPU tüketimi optimize tutulmalıdır.
* **Bağımlılık:** Yok.

#### 7. Çevrimdışı Medya Tespiti ve Toplu Yeniden Bağlama (Offline Relink Dialog)
* **Ne olduğu:** Proje açıldığında diskte bulunamayan medya dosyalarının tespit edilmesi, zaman çizgisinde bu kliplerin kırmızı şerit ve "Medya Çevrimdışı" yazısıyla gösterilmesi; açılan bir pencereyle klasör gösterilerek eksik dosyaların tek seferde yeniden bağlanması.
* **Kullanıcıya Faydası:** Dosya isimlerini değiştiren veya projeyi başka bir diske taşıyan kullanıcıların emeklerinin heba olmasını önler.
* **Mevcut Koda Nasıl Oturur:**
  * `trim-project/src/lib.rs:load_project`: Dosyalar taranır, bulunamayanlar `missing_media_ids: Vec<String>` olarak işaretlenir.
  * `trim-ui/src/canvas.rs`: Medyası eksik klip kırmızı çapraz çizgili doku ile çizilir.
  * `trim-ui/src/dialogs/relink_dialog.rs`: Eksik medyaları listeleyen ve bir klasör seçildiğinde dosya adlarıyla otomatik eşleşme yapan bir diyalog eklenir.
* **Zorluk:** **Orta**. Dosya adı eşleştirme ve `MediaItem.file_path` güncellemeleri temiz bir mimariyle bağlanmalıdır.
* **Riskler:** Yok.
* **Bağımlılık:** Yok.

#### 8. Video Kırpma (Crop) Efekti [✅ TAMAMLANDI]
* **Ne olduğu:** Klibe Sol, Sağ, Üst ve Alt yüzdelik kırpma değerlerinin eklenmesi.
* **Kullanıcıya Faydası:** Yan yana iki videoyu bölme (split-screen), dikey videoyu yataya yerleştirirken gereksiz kenarları atma veya sinematik siyah bant efekti oluşturma.
* **Mevcut Koda Nasıl Oturur:**
  * `trim-core/src/clip.rs`: `crop_left, crop_right, crop_top, crop_bottom: f64` alanları eklenir.
  * `trim-ui/src/inspector.rs`: Dönüşüm kartına 4 adet kırpma kaydırıcısı konur.
  * `trim-ui/src/gpu_compositor.rs`: Parçacık gölgelendiricisinde (fragment shader) UV koordinat sınırları dışındaki piksellere `discard;` veya `alpha = 0.0;` verilir.
  * `trim-export/src/filtergraph.rs`: FFmpeg `crop=w:h:x:y` filtresi zincire eklenir.
* **Zorluk:** **Orta**.
* **Riskler:** Yok.

---

### Kategori C: Uzun Vadeli ve Mimari Genişletmeler (Yüksek Zorluk / Risk)

#### 9. Çoklu Sekans (Multiple Timelines) ve İç İçe Sekans (Nested Sequences) [✅ TAMAMLANDI]
* **Ne olduğu:** Tek bir proje içerisinde birden fazla bağımsız zaman çizgisi sekansı açabilme ve bir sekansı diğerinin içine klip gibi yerleştirebilme.
* **Kullanıcıya Faydası:** Uzun metrajlı veya bölümlü videolarda sahneleri ayrı ayrı kurgulayıp ana sekansta birleştirmeyi sağlar.
* **Mevcut Koda Nasıl Oturur:**
  * `trim-core/src/project.rs`: `timeline: TimelineModel` yerine `timelines: HashMap<String, TimelineModel>` ve `active_timeline_id: String` yapısına geçilir.
  * Bir sekans, `MediaType::Sequence` olarak medya havuzuna bir medya öğesi gibi eklenir.
  * `filtergraph.rs` ve `VideoEngine`: İç içe sekansı özyinelemeli (recursive) bir alt filtergraph olarak derlemek zorunda kalır.
* **Zorluk:** **Büyük**. Proje dosya formatı v2'ye yükselir, geriye dönük uyumluluk için göç (migration) kodu gerekir; döngüsel bağımlılık (A sekansı B'yi, B sekansı A'yı içeremez) kontrolü gereklidir.
* **Riskler:** Render pipeline'ının karmaşıklığı katlanarak artar.
* **Bağımlılık:** Yok.

#### 10. Çoklu Dışa Aktarma Kuyruğu (Render Queue) [✅ TAMAMLANDI]
* **Ne olduğu:** Birden fazla sekansı veya aynı sekanstan farklı formatları (ör. hem 1080p YouTube hem 9:16 Shorts) arka arkaya render edilmek üzere bir kuyruğa ekleme ve sırayla dışa aktarma.
* **Kullanıcıya Faydası:** Kurgucunun bilgisayar başından ayrıldığı gece saatlerinde toplu çıktı alabilmesini sağlar.
* **Mevcut Koda Nasıl Oturur:**
  * `trim-export/src/runner.rs`: `ExportJobQueue` yapısı kurularak birden fazla `ExportJob` kuyrukta saklanır.
  * Arayüzde ayrı bir "Render Kuyruğu" paneli tasarlanır.
* **Zorluk:** **Büyük**. Süreç yönetimi, hata durumunda sonraki işe geçme ve arka plan iş parçacığı senkronizasyonu gerektirir.
* **Riskler:** Donanım kodlayıcı kilitlenmeleri (özellikle tekil NVENC oturum limitleri).
* **Bağımlılık:** Yok.

---

## İlk 5 Öneri ve Öncelik Sıralaması

triM'in mevcut 1.1.2 sürümündeki mimari kararlılığı, bilinen kırılganlıkları ve kullanıcı deneyimini doğrudan etkileyen noktalar göz önüne alındığında, **geliştirilmesine başlanması gereken ilk 5 özellik ve gerekçeleri:**

```
1. Klip Kopyala - Yapıştır (Copy/Paste)
   └─► Nedeni: Sıfır mimari risk, 1 saatlik geliştirme maliyeti, kurgucunun her saniye aradığı en temel refleks.

2. Gerçek Giriş/Çıkış (In/Out) Belirleme ve Seçili Aralık Render Desteği
   └─► Nedeni: Arayüzde butonları ve kısayolları olup arkası boş kalan en göze çarpan özellik. 10 dakikalık videonun ufak bir kesitini dışa aktarmak için zorunlu.

3. Inspector Düzenlemelerine Undo/Redo Desteği Kazandırılması
   └─► Nedeni: SAĞLAMLAŞTIRMA ÖNCELİĞİ. Kullanıcı bir klibin ölçeğini veya rengini bozduğunda Ctrl+Z ile geri alamıyor; bu durum projenin en büyük veri güvenliği açığıdır.

4. Toolbar'daki Slip (Kaydırma) Aracının Canlıya Alınması
   └─► Nedeni: Arayüzde ikonu olup tıklanınca hiçbir iş yapmayan araçlar kullanıcıda "program yarım/bozuk" algısı yaratır. Altyapısı TrimClipCommand ile zaten hazırdır.

5. Çevrimdışı Medya (Offline Relink) Tespiti ve Yeniden Bağlama Penceresi
   └─► Nedeni: Taşınan veya yeniden adlandırılan dosyalarda projenin kilitlenmesini veya boşa düşmesini önleyerek profesyonel düzeyde proje güvenilirliği sağlar.
```
