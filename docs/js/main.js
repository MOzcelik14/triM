// triM — Desktop Video Editor Documentation
// Theme toggle, full bilingual i18n, screenshot gallery filters, copy tool, and shortcuts live search.

const I18N = {
  tr: {
    // Navigation
    skip_to_content: "İçeriğe geç",
    nav_home: "Ana Sayfa",
    nav_features: "Özellikler",
    nav_gallery: "Ekranlar",
    nav_install: "Kurulum",
    nav_arch: "Mimari",
    nav_limits: "Sınırlamalar",
    nav_shortcuts: "Kısayollar",
    nav_releasing: "Yayın Rehberi",

    // Hero
    hero_tag: "Rust + egui + FFmpeg ile Geliştirilen Masaüstü Video Düzenleyici",
    hero_title: "Hafif, yerel ve doğrudan.",
    hero_desc: "triM, arayüzü egui ile doğrudan çizilen, medya çözümleme ve kodlama için sistemdeki FFmpeg araçlarını kullanan, 13 MB boyutunda yerel bir masaüstü video düzenleyicisidir. Tarayıcı motoru, Electron veya bulut bağımlılığı içermez.",
    btn_download: "Sürümleri İndir ↘",
    btn_github: "GitHub Deposu ↗",
    window_title: "triM. v1.2.2 — Sinematik Kurgu Masası",

    // Features Section
    sec_features: "Temel Özellikler",
    sec_features_heading: "İhtiyaç duyulan kurgu araçları, doğrudan yerel kodla.",
    sec_features_sub: "Ağır kurulum paketleri ve karmaşık menüler yerine doğrudan kurguya odaklanan modüler araçlar.",
    badge_native: "Yerel",
    badge_color: "3D LUT",
    badge_audio: "48 kHz",
    badge_export: "GPU Hızlandırma",

    feat_edit_title: "Kurgu ve Zaman Çizgisi",
    feat_edit_1: "Çok kanallı video ve ses zaman çizgisi (V1..Vn, A1..An)",
    feat_edit_2: "Jilet (Razor), Dalgalanma (Ripple), Kaydırma (Slip) ve Hız Uzatma araçları",
    feat_edit_3: "Oynatma kafasına ve klip kenarlarına manyetik kenetlenme (Snapping)",
    feat_edit_4: "Akıllı video-ses eşleştirmesi (V1 → A1, V2 → A2) ve otomatik bağlama",
    feat_edit_5: "Zaman çizgisi üzerinde renkli görsel işaretçiler (Markers)",

    feat_color_title: "Renk ve Görsel Efektler",
    feat_color_1: "3 yönlü renk çarkları (Lift, Gamma, Gain) ile tonlama",
    feat_color_2: "<code>.cube</code> formatında 3D LUT uygulama ve hazır renk profilleri",
    feat_color_3: "Chroma Key (yeşil ve mavi perde renk anahtarlama)",
    feat_color_4: "Opaklık geçişleri (fade-in, fade-out) ve katman harmanlama",
    feat_color_5: "Konum, ölçek, döndürme ve opaklık için anahtar kareler (Keyframes)",

    feat_audio_title: "Çok Kanallı Ses Mikseri",
    feat_audio_1: "48 kHz stereo gerçek zamanlı yazılımsal PCM ses mikseri",
    feat_audio_2: "Kanal bazlı kazanç (dB), sessize alma (Mute) ve solo dinleme",
    feat_audio_3: "3 bant parametrik ekolayzır ve ses hazır ayarları (vokal netliği, bas)",
    feat_audio_4: "Doğrudan zaman çizgisine mikrofonla seslendirme kaydı (Voiceover)",
    feat_audio_5: "Çıkışta kırpılmayı önleyen tuğla duvar sınırlayıcı (Lookahead Limiter)",

    feat_export_title: "Dışa Aktarma ve Standartlar",
    feat_export_1: "H.264, HEVC, VP9, AV1, ProRes ve DNxHR formatlarında render",
    feat_export_2: "Donanım hızlandırma tespiti (NVENC, Intel QSV, AMD AMF, Linux VAAPI)",
    feat_export_3: "EBU R128 standardında entegre ses normalizasyonu (<code>loudnorm</code>)",
    feat_export_4: "Final Cut Pro (<code>.fcpxml</code> v1.8/v1.9) ve CMX 3600 (<code>.edl</code>) aktarımı",
    feat_export_5: "9:16 Dikey kurgu modu ve güvenli alan çizgileri (Reels / Shorts)",

    // Gallery Section
    sec_gallery: "Canlı Uygulama Ekranları",
    sec_gallery_heading: "Gerçek Çalışma Alanı Görünümleri",
    sec_gallery_sub: "Uygulamanın v1.2.2 sürümünden doğrudan alınan ekran görüntüleri. Büyütmek için tıklayın.",
    filter_all: "Tümü (8)",
    filter_edit: "Kurgu Masası",
    filter_color_audio: "Renk & Ses",
    filter_export: "Dışa Aktarma & Dikey",

    caption_welcome: "Karşılama Ekranı (Yeni proje şablonları)",
    caption_workspace: "Ana Kurgu Masası (Medya havuzu, monitör, zaman çizgisi)",
    caption_timeline: "Zaman Çizgisi Detayı (Çok kanallı ses/video, dalga formu)",
    caption_color_grading: "Renk Düzenleme (Lift/Gamma/Gain çarkları ve efektler)",
    caption_audio_mixer: "Ses Mikseri (Kanal fader'ları ve VU metre)",
    caption_export_studio: "Dışa Aktarma Stüdyosu (GPU hızlandırma ve EBU R128)",
    caption_shortcuts: "Klavye Kısayolları Penceresi (F1)",
    caption_vertical_9_16: "Dikey Tuval (9:16 Shorts/Reels kılavuzları)",

    // Installation Section
    sec_install: "Kurulum ve Derleme",
    sec_install_heading: "Kurulum Seçenekleri",
    sec_install_sub: "Hazır paketleri indirin veya kaynak kodundan tek komutla derleyin.",
    install_req: "Gereksinimler:",
    install_req_desc: "64-bit Linux veya Windows, OpenGL 3.3+ grafik desteği, sistemde kurulu <code>ffmpeg</code> ve <code>ffprobe</code>.",
    install_oneliner_title: "Linux Tek Satır Kurulum:",
    install_packages_title: "Paket İndirmeleri:",
    install_packages_btn: "GitHub Sürümler Sayfası (.deb, .rpm, .AppImage, .zip) ↗",
    install_source_title: "Kaynaktan Derleme:",
    copy_btn: "Kopyala",
    copied_btn: "Kopyalandı!",

    // Architecture Section
    sec_arch: "Mimari ve Tasarım Kararları",
    sec_arch_heading: "Modüler Rust Mimarisi",
    sec_arch_sub: "triM, sorumlulukların ayrıldığı 8 bağımsız Cargo crate'inden oluşur.",
    th_crate: "Crate",
    th_responsibility: "Sorumluluk",
    arch_core: "Zaman hesabı, klip, kanal, proje modelleri ve bellek sınırları (LruCache)",
    arch_commands: "Geri alınabilir (Undo/Redo) komut nesneleri ve komut yığını",
    arch_timeline: "Zaman çizgisi kurgu algoritmaları, kesim noktası tespiti",
    arch_project: "Atomik dosya kaydetme (.trim), otomatik yedekleme ve kurtarma",
    arch_media: "FFmpeg alt süreç yöneticisi, asenkron kare çözümleme, PCM ses mikseri",
    arch_export: "FFmpeg filtergraph derleyicisi, EDL ve FCPXML aktarımı",
    arch_ui: "egui arayüzü, zaman çizgisi çizimi, monitörler ve denetçi paneli",
    arch_app: "Masaüstü çalıştırılabilir ikili giriş noktası",

    // Limitations Section
    sec_limits: "Bilinen Sınırlamalar",
    sec_limits_heading: "Açık ve Dürüst Teknik Detaylar",
    limits_1_title: "Önizleme Çözünürlüğü:",
    limits_1_desc: "Zaman çizgisi önizlemesi CPU üzerinde ölçeklenerek 640x360 çözünürlükte işlenir ve OpenGL dokusu olarak yüklenir. Çok sayıda eşzamanlı katman içeren sahnelerde CPU kullanımı artabilir.",
    limits_2_title: "Alt Süreç Tabanlı Medya Hattı:",
    limits_2_desc: "C kütüphanelerini doğrudan bağlamak yerine sistemdeki FFmpeg izole bir alt süreç olarak çalıştırılır. Bu tercih arayüz çökmesini önler; ancak uzun GOP aralığına sahip dosyalarda ilk arama sırasında 15-40 ms civarında gecikme olabilir. Akıcı kurgu için proxy modu önerilir.",
    limits_3_title: "Platform Kapsamı:",
    limits_3_desc: "Mikrofonla ses kaydı (Voiceover) ve ekran kaydı şu anda Linux'a özgüdür. Windows derlemesinde temel ses oynatımı ffplay üzerinden çalışır; doğrudan mikrofon kaydı henüz Windows'ta desteklenmemektedir.",
    limits_4_title: "Renk Derinliği:",
    limits_4_desc: "Önizleme 8-bit sRGB alanında çalışır. 10-bit HDR kaynak medyalar önizleme sırasında 8-bite indirgenir; nihai render FFmpeg motoru üzerinden tam bit derinliğinde dışa aktarılır.",

    // Footer
    footer_license: "triM — GPL-3.0-or-later lisansı altında açık kaynaklıdır. Geliştirici: Murat Özçelik.",

    // Shortcuts Page Specific
    sc_label: "Referans Kılavuzu",
    sc_title: "Klavye Kısayolları",
    sc_intro: "triM, endüstri standardı NLE (Non-Linear Editor) iş akışlarını destekler. Kısayol profilinizi dilediğiniz zaman uygulama içinden değiştirebilir veya F1 tuşuna basarak kısayol rehberini açabilirsiniz.",
    sc_search_placeholder: "Kısayol veya işlev ara... (örn: jilet, böl, space, ripple)",
    sc_transport_title: "1. Oynatma ve Gezinme (Transport)",
    sc_edit_title: "2. Araçlar ve Düzenleme (NLE Profil Karşılaştırması)",
    sc_edit_desc: "Alışık olduğunuz Premiere Pro veya DaVinci Resolve tuş düzenini tek tıkla kullanabilirsiniz.",
    sc_project_title: "3. Proje ve Genel Kısayollar",
    th_shortcut: "Kısayol",
    th_function: "İşlev",
    th_trim_default: "triM Varsayılan",
    th_premiere: "Premiere Pro",
    th_resolve: "DaVinci Resolve",
    sc_fn_space: "Oynat / Duraklat (Play / Pause)",
    sc_fn_j: "Geriye doğru oynat (her basışta 2x, 4x, 8x hızlanır)",
    sc_fn_k: "Oynatmayı anında durdur",
    sc_fn_l: "İleriye doğru oynat (her basışta 2x, 4x, 8x hızlanır)",
    sc_fn_left: "1 kare geriye git",
    sc_fn_right: "1 kare ileriye git",
    sc_fn_shift_left: "1 saniye geriye atla",
    sc_fn_shift_right: "1 saniye ileriye atla",
    sc_fn_home: "Zaman çizgisinin en başına git (00:00:00:00)",
    sc_fn_end: "Zaman çizgisindeki en son klibin bitişine git",
    sc_fn_i: "Kaynak monitöründe giriş noktasını belirle (Mark In)",
    sc_fn_o: "Kaynak monitöründe çıkış noktasını belirle (Mark Out)",
    sc_fn_select: "Seçim Aracı (Selection Tool)",
    sc_fn_razor: "Jilet / Kesme Aracı (Razor / Blade)",
    sc_fn_split: "Oynatma kafası konumunda klibi böl (Split Clip)",
    sc_fn_ripple: "Dalgalanma Düzenleme (Ripple Edit)",
    sc_fn_rate: "Hız Uzatma / Sıkıştırma (Rate Stretch)",
    sc_fn_slip: "Kaydırma Aracı (Slip Edit)",
    sc_fn_transition: "Geçiş Aracı (Transition)",
    sc_fn_hand: "El / Kaydırma Aracı (Hand Pan)",
    sc_fn_zoom: "Yakınlaştırma Aracı (Zoom)",
    sc_fn_type: "Metin ve Yazı Aracı (Type Tool)",
    sc_fn_ripple_left: "Baştan oynatma kafasına kadar dalgalanarak kırp (Ripple Trim Head)",
    sc_fn_ripple_right: "Oynatma kafasından sona kadar dalgalanarak kırp (Ripple Trim Tail)",
    sc_fn_delete: "Seçili klibi sil",
    sc_fn_ripple_del: "Klibi ve boşluğu kapatıp dalgalanarak sil (Ripple Delete)",
    sc_fn_new: "Yeni proje sihirbazını aç",
    sc_fn_open: "Var olan .trim projesini aç",
    sc_fn_save: "Projeyi kaydet",
    sc_fn_save_as: "Projeyi farklı kaydet",
    sc_fn_import: "Medya havuzuna video/ses/görsel içe aktar",
    sc_fn_export: "Dışa Aktarma Stüdyosu'nu aç (Render / Export)",
    sc_fn_undo: "Son işlemi geri al (Undo)",
    sc_fn_redo: "Geri alınan işlemi yinele (Redo)",
    sc_fn_select_all: "Tüm klipleri seç",
    sc_fn_f1: "Klavye kısayolları penceresini aç / kapat",
    sc_fn_m: "Oynatma kafası konumuna renkli işaretçi (Marker) ekle",
    sc_fn_n: "Manyetik kenetlenmeyi aç / kapat (Magnetic Snapping)",

    // Releasing Page Specific
    rel_label: "Geliştirici Dokümantasyonu",
    rel_title: "Sürüm Yayınlama Rehberi",
    rel_intro: "Bu kılavuz, triM projesinde yeni bir sürümün hazırlanması, derlenmesi, test edilmesi, Git etiketinin oluşturulması ve GitHub Actions CI hattı üzerinden çoklu platform paketlerinin yayınlanması adımlarını belgeler.",
    rel_step1_title: "1. Sürüm Numarasını Güncelleme",
    rel_step1_desc: "triM, Semantic Versioning (SemVer: MAJOR.MINOR.PATCH) standardını takip eder. Yeni sürüm için ilgili dosyalarda sürüm numarasını artırın:",
    rel_step1_lock: "Ardından Cargo.lock dosyasını güncellemek için cargo check komutunu çalıştırın.",
    rel_step1_meta: "Meta veri dosyalarını güncelleyin:",
    rel_step2_title: "2. Doğrulama ve Testler",
    rel_step2_desc: "Etiket oluşturmadan önce çalışma alanındaki tüm 131+ testin, biçimlendirme kurallarının ve clippy denetimlerinin hatasız tamamlandığından emin olun:",
    rel_step3_title: "3. Git Etiketi (Tag) ve Push",
    rel_step3_desc: "Değişiklikleri commit edin ve imzalı/açıklamalı bir Git etiketi oluşturarak depoya gönderin:",
    rel_step4_title: "4. Otomatik CI / CD Dağıtım Paketleri",
    rel_step4_desc: "v* formatında bir etiket pushlandığında .github/workflows/release-linux.yml iş akışı otomatik devreye girer ve GitHub Releases üzerinde aşağıdaki paketleri derleyip yayınlar:",
    th_package: "Paket Formatı",
    th_target: "Hedef Platform / Dağıtım"
  },
  en: {
    // Navigation
    skip_to_content: "Skip to content",
    nav_home: "Home",
    nav_features: "Features",
    nav_gallery: "Screenshots",
    nav_install: "Installation",
    nav_arch: "Architecture",
    nav_limits: "Limitations",
    nav_shortcuts: "Shortcuts",
    nav_releasing: "Releasing",

    // Hero
    hero_tag: "Native Desktop Video Editor Built in Rust + egui + FFmpeg",
    hero_title: "Lightweight, native, and direct.",
    hero_desc: "triM is a 13 MB native desktop non-linear video editor with its UI rendered immediately via egui, using system FFmpeg for media decoding and encoding. Zero browser engine, zero Electron, zero cloud account lock-in.",
    btn_download: "Download Releases ↘",
    btn_github: "GitHub Repository ↗",
    window_title: "triM. v1.2.2 — Cinematic Editing Suite",

    // Features Section
    sec_features: "Core Features",
    sec_features_heading: "Essential editing tools powered by native code.",
    sec_features_sub: "Modular tools focused directly on cutting and editing without heavy bloatware or convoluted nested menus.",
    badge_native: "Native",
    badge_color: "3D LUT",
    badge_audio: "48 kHz",
    badge_export: "GPU Accelerated",

    feat_edit_title: "Editing & Timeline",
    feat_edit_1: "Multi-track video and audio timeline (V1..Vn, A1..An)",
    feat_edit_2: "Razor, Ripple, Slip, and Rate Stretch editing tools",
    feat_edit_3: "Magnetic snapping to playhead and clip boundaries",
    feat_edit_4: "Smart video-to-audio track pairing (V1 → A1, V2 → A2) and clip grouping",
    feat_edit_5: "Color-coded visual timeline markers",

    feat_color_title: "Color & Visual Effects",
    feat_color_1: "3-Way Color Wheels (Lift, Gamma, Gain) grading",
    feat_color_2: "3D LUT support in <code>.cube</code> format with cinematic presets",
    feat_color_3: "Chroma Key (green and blue screen keying)",
    feat_color_4: "Opacity fades (fade-in, fade-out) and layer compositing",
    feat_color_5: "Keyframe animation for Position, Scale, Rotation, and Opacity",

    feat_audio_title: "Multi-Track Audio Mixer",
    feat_audio_1: "48 kHz stereo real-time software PCM audio mixer",
    feat_audio_2: "Per-channel gain (dB), Mute, and Solo monitoring",
    feat_audio_3: "3-band parametric EQ and audio presets (voice clarity, bass boost)",
    feat_audio_4: "Direct timeline voiceover recording via system microphone",
    feat_audio_5: "Brickwall lookahead peak limiter to eliminate digital clipping",

    feat_export_title: "Export & Standards",
    feat_export_1: "Master export in H.264, HEVC, VP9, AV1, ProRes, and DNxHR",
    feat_export_2: "Hardware acceleration detection (NVENC, Intel QSV, AMD AMF, Linux VAAPI)",
    feat_export_3: "Integrated audio loudness normalization to EBU R128 (<code>loudnorm</code>)",
    feat_export_4: "NLE interchange with Final Cut Pro (<code>.fcpxml</code> v1.8/v1.9) and CMX 3600 (<code>.edl</code>)",
    feat_export_5: "9:16 Vertical canvas mode with social safe zone guides (Reels / Shorts)",

    // Gallery Section
    sec_gallery: "Live In-App Screenshots",
    sec_gallery_heading: "Real In-App Views",
    sec_gallery_sub: "Real screenshots captured directly from triM v1.2.2. Click to enlarge.",
    filter_all: "All (8)",
    filter_edit: "Workspace & Timeline",
    filter_color_audio: "Color & Audio",
    filter_export: "Export & Vertical",

    caption_welcome: "Welcome Dialog (New project templates)",
    caption_workspace: "Main Workspace (Media pool, monitors, timeline)",
    caption_timeline: "Timeline Detail (Multi-track audio/video, waveforms)",
    caption_color_grading: "Color Grading (Lift/Gamma/Gain wheels & effects)",
    caption_audio_mixer: "Audio Mixer (Channel faders and VU meters)",
    caption_export_studio: "Export Studio (GPU acceleration & EBU R128)",
    caption_shortcuts: "Keyboard Shortcuts Window (F1)",
    caption_vertical_9_16: "Vertical Canvas (9:16 Shorts/Reels safe zones)",

    // Installation Section
    sec_install: "Installation & Build",
    sec_install_heading: "Installation Options",
    sec_install_sub: "Download pre-built binaries or build from source with a single command.",
    install_req: "Requirements:",
    install_req_desc: "64-bit Linux or Windows, OpenGL 3.3+ graphics driver, system <code>ffmpeg</code> and <code>ffprobe</code> installed.",
    install_oneliner_title: "Linux One-Line Install:",
    install_packages_title: "Binary Packages:",
    install_packages_btn: "GitHub Releases Page (.deb, .rpm, .AppImage, .zip) ↗",
    install_source_title: "Build From Source:",
    copy_btn: "Copy",
    copied_btn: "Copied!",

    // Architecture Section
    sec_arch: "Architecture & Design Choices",
    sec_arch_heading: "Modular Rust Architecture",
    sec_arch_sub: "triM is divided into 8 independent Cargo crates with clean boundaries.",
    th_crate: "Crate",
    th_responsibility: "Responsibility",
    arch_core: "Timecode arithmetic, clip, track, project models and LRU memory cache",
    arch_commands: "Reversible command objects and undo/redo history stack",
    arch_timeline: "Timeline editing algorithms, ripple delete, cut-point discovery",
    arch_project: "Atomic file persistence (.trim), autosave and crash recovery",
    arch_media: "FFmpeg subprocess supervisor, async frame streaming, PCM mixer",
    arch_export: "FFmpeg filtergraph compiler, EDL and FCPXML interchange",
    arch_ui: "egui frontend, timeline rendering, monitors, and inspector",
    arch_app: "Desktop binary entry point and CLI supervisor",

    // Limitations Section
    sec_limits: "Known Limitations",
    sec_limits_heading: "Honest Technical Details",
    limits_1_title: "Preview Resolution:",
    limits_1_desc: "Timeline playback is rendered on the CPU at 640x360 scaled resolution and uploaded as an OpenGL texture. Projects with many simultaneous composite layers will experience higher CPU usage.",
    limits_2_title: "Subprocess-Based Media Pipeline:",
    limits_2_desc: "System FFmpeg is invoked as an isolated subprocess rather than linking C shared libraries. This prevents UI segfaults from codec crashes, but results in 15-40 ms seek latency on long-GOP files. Proxy mode is recommended for snappy editing.",
    limits_3_title: "Platform Scope:",
    limits_3_desc: "Voiceover microphone recording and screen capture are currently Linux-only. On Windows, basic playback works through ffplay; direct microphone capture is not yet implemented.",
    limits_4_title: "Color Depth:",
    limits_4_desc: "UI preview operates in 8-bit sRGB. 10-bit HDR source media is tone-mapped to 8-bit during preview; final rendering occurs at full bit depth via FFmpeg.",

    // Footer
    footer_license: "triM — Open source under GPL-3.0-or-later. Created by Murat Özçelik.",

    // Shortcuts Page Specific
    sc_label: "Reference Guide",
    sc_title: "Keyboard Shortcuts",
    sc_intro: "triM supports industry-standard NLE workflows. Switch your active shortcut profile at any time in-app or press F1 to open the quick cheat sheet.",
    sc_search_placeholder: "Search shortcuts or functions... (e.g. blade, cut, space, ripple)",
    sc_transport_title: "1. Transport & Playback",
    sc_edit_title: "2. Tools & Editing (NLE Profile Comparison)",
    sc_edit_desc: "Seamlessly switch between triM default, Premiere Pro, or DaVinci Resolve keyboard profiles.",
    sc_project_title: "3. Project & General Shortcuts",
    th_shortcut: "Shortcut",
    th_function: "Function",
    th_trim_default: "triM Default",
    th_premiere: "Premiere Pro",
    th_resolve: "DaVinci Resolve",
    sc_fn_space: "Play / Pause",
    sc_fn_j: "Shuttle reverse (press repeatedly for 2x, 4x, 8x speed)",
    sc_fn_k: "Stop playback immediately",
    sc_fn_l: "Shuttle forward (press repeatedly for 2x, 4x, 8x speed)",
    sc_fn_left: "Step 1 frame backward",
    sc_fn_right: "Step 1 frame forward",
    sc_fn_shift_left: "Jump 1 second backward",
    sc_fn_shift_right: "Jump 1 second forward",
    sc_fn_home: "Jump to timeline start (00:00:00:00)",
    sc_fn_end: "Jump to the end of the last clip",
    sc_fn_i: "Mark In point on source monitor",
    sc_fn_o: "Mark Out point on source monitor",
    sc_fn_select: "Selection Tool",
    sc_fn_razor: "Razor / Blade Cut Tool",
    sc_fn_split: "Split clip at current playhead position",
    sc_fn_ripple: "Ripple Edit Tool",
    sc_fn_rate: "Rate Stretch Tool",
    sc_fn_slip: "Slip Edit Tool",
    sc_fn_transition: "Transition Tool",
    sc_fn_hand: "Hand Pan Tool",
    sc_fn_zoom: "Zoom Tool",
    sc_fn_type: "Text & Title Type Tool",
    sc_fn_ripple_left: "Ripple trim from head to playhead position",
    sc_fn_ripple_right: "Ripple trim from playhead position to tail",
    sc_fn_delete: "Delete selected clip",
    sc_fn_ripple_del: "Ripple delete selected clip and close gap",
    sc_fn_new: "Open new project wizard",
    sc_fn_open: "Open existing .trim project",
    sc_fn_save: "Save project",
    sc_fn_save_as: "Save project as...",
    sc_fn_import: "Import video/audio/image media",
    sc_fn_export: "Open Export Studio dialog (Render / Export)",
    sc_fn_undo: "Undo last action",
    sc_fn_redo: "Redo last undone action",
    sc_fn_select_all: "Select all clips on timeline",
    sc_fn_f1: "Toggle keyboard shortcuts cheat sheet",
    sc_fn_m: "Add color marker at playhead position",
    sc_fn_n: "Toggle magnetic snapping",

    // Releasing Page Specific
    rel_label: "Developer Documentation",
    rel_title: "Release & Packaging Guide",
    rel_intro: "This guide documents the process of preparing, testing, tagging, and distributing multi-platform packages for triM through GitHub Actions CI.",
    rel_step1_title: "1. Version Number Bumping",
    rel_step1_desc: "triM strictly follows Semantic Versioning (SemVer: MAJOR.MINOR.PATCH). Bump the version in the following files:",
    rel_step1_lock: "Then run cargo check to update the Cargo.lock dependency tree.",
    rel_step1_meta: "Update packaging metadata files:",
    rel_step2_title: "2. Verification and Tests",
    rel_step2_desc: "Ensure all 131+ workspace unit tests, code formatting checks, and strict clippy linters pass before publishing:",
    rel_step3_title: "3. Git Tagging and Push",
    rel_step3_desc: "Commit your changes and create an annotated release tag:",
    rel_step4_title: "4. Automated CI / CD Distribution Packages",
    rel_step4_desc: "Pushing a v* tag automatically triggers .github/workflows/release-linux.yml which compiles and publishes the following assets to GitHub Releases:",
    th_package: "Package Format",
    th_target: "Target Platform / Distro"
  }
};

let currentLang = localStorage.getItem("trim_lang") || "tr";
let currentTheme = localStorage.getItem("trim_theme") || "dark";

function applyTheme(theme) {
  currentTheme = theme;
  document.documentElement.setAttribute("data-theme", theme);
  localStorage.setItem("trim_theme", theme);
  const btn = document.getElementById("theme-toggle");
  if (btn) {
    btn.textContent = theme === "dark" ? "☼ Açık" : "☾ Koyu";
    if (currentLang === "en") {
      btn.textContent = theme === "dark" ? "☼ Light" : "☾ Dark";
    }
  }
}

function applyLanguage(lang) {
  currentLang = lang;
  document.documentElement.lang = lang;
  localStorage.setItem("trim_lang", lang);
  
  const dict = I18N[lang] || I18N.tr;

  // Text content replacements
  document.querySelectorAll("[data-i18n]").forEach(el => {
    const key = el.getAttribute("data-i18n");
    if (dict[key]) {
      el.textContent = dict[key];
    }
  });

  // HTML content replacements (for formatting with <code> etc.)
  document.querySelectorAll("[data-i18n-html]").forEach(el => {
    const key = el.getAttribute("data-i18n-html");
    if (dict[key]) {
      el.innerHTML = dict[key];
    }
  });

  // Placeholder replacements
  document.querySelectorAll("[data-i18n-placeholder]").forEach(el => {
    const key = el.getAttribute("data-i18n-placeholder");
    if (dict[key]) {
      el.placeholder = dict[key];
    }
  });

  const langBtn = document.getElementById("lang-toggle");
  if (langBtn) {
    langBtn.textContent = lang === "tr" ? "TR" : "EN";
  }

  const themeBtn = document.getElementById("theme-toggle");
  if (themeBtn) {
    if (lang === "en") {
      themeBtn.textContent = currentTheme === "dark" ? "☼ Light" : "☾ Dark";
    } else {
      themeBtn.textContent = currentTheme === "dark" ? "☼ Açık" : "☾ Koyu";
    }
  }
}

document.addEventListener("DOMContentLoaded", () => {
  // Apply saved theme and language
  applyTheme(currentTheme);
  applyLanguage(currentLang);

  // Theme Toggle Listener
  const themeToggle = document.getElementById("theme-toggle");
  if (themeToggle) {
    themeToggle.addEventListener("click", () => {
      applyTheme(currentTheme === "dark" ? "light" : "dark");
    });
  }

  // Language Toggle Listener
  const langToggle = document.getElementById("lang-toggle");
  if (langToggle) {
    langToggle.addEventListener("click", () => {
      applyLanguage(currentLang === "tr" ? "en" : "tr");
    });
  }

  // Gallery Filters
  const filterBtns = document.querySelectorAll(".gallery-filter-btn");
  const galleryCards = document.querySelectorAll(".gallery-card");

  filterBtns.forEach(btn => {
    btn.addEventListener("click", () => {
      filterBtns.forEach(b => b.classList.remove("active"));
      btn.classList.add("active");
      const category = btn.getAttribute("data-filter");

      galleryCards.forEach(card => {
        if (category === "all" || card.getAttribute("data-category") === category) {
          card.style.display = "block";
        } else {
          card.style.display = "none";
        }
      });
    });
  });

  // Lightbox Modal for Screenshots
  const modal = document.getElementById("image-modal");
  const modalImg = document.getElementById("modal-img");
  const modalCaption = document.getElementById("modal-caption");
  const modalClose = document.getElementById("modal-close");

  if (modal && modalImg && modalClose) {
    document.querySelectorAll(".gallery-card").forEach(card => {
      card.addEventListener("click", () => {
        const img = card.querySelector("img");
        const caption = card.querySelector(".gallery-card-caption");
        if (img) {
          modalImg.src = img.src;
          modalImg.alt = img.alt || "Ekran Görüntüsü";
          modalCaption.textContent = caption ? caption.textContent : "";
          modal.classList.add("is-open");
          document.body.style.overflow = "hidden";
        }
      });
    });

    const closeModal = () => {
      modal.classList.remove("is-open");
      document.body.style.overflow = "";
    };

    modalClose.addEventListener("click", closeModal);
    modal.addEventListener("click", (e) => {
      if (e.target === modal) {
        closeModal();
      }
    });

    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && modal.classList.contains("is-open")) {
        closeModal();
      }
    });
  }

  // Copy-to-Clipboard tool
  document.querySelectorAll(".copy-btn").forEach(btn => {
    btn.addEventListener("click", () => {
      const targetId = btn.getAttribute("data-target");
      const codeEl = document.getElementById(targetId);
      if (codeEl) {
        const text = codeEl.textContent.trim();
        navigator.clipboard.writeText(text).then(() => {
          const original = btn.textContent;
          const dict = I18N[currentLang] || I18N.tr;
          btn.textContent = dict.copied_btn || "Kopyalandı!";
          btn.style.borderColor = "var(--brand-orange)";
          setTimeout(() => {
            btn.textContent = original;
            btn.style.borderColor = "";
          }, 2000);
        });
      }
    });
  });

  // Live search for shortcuts.html
  const shortcutSearch = document.getElementById("shortcut-search");
  if (shortcutSearch) {
    shortcutSearch.addEventListener("input", (e) => {
      const query = e.target.value.toLowerCase().trim();
      const rows = document.querySelectorAll(".shortcut-table tbody tr");
      rows.forEach(row => {
        const text = row.textContent.toLowerCase();
        if (!query || text.includes(query)) {
          row.style.display = "";
        } else {
          row.style.display = "none";
        }
      });
    });
  }
});
