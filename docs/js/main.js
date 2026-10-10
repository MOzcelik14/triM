/**
 * triM. — Product Website Interactive Scripts
 * Features:
 * - Dynamic i18n Translation (TR/EN) with localStorage persistence
 * - Interactive Bento Card Cursor Spotlight Effect
 * - Showcase Preview Tab Switcher
 * - Distro Dependencies Tab Switcher
 * - Keyboard Shortcuts Category Filter
 * - Clipboard Copy Buttons with Animated Toast Feedback
 * - Sticky Header Scroll Detection
 */

document.addEventListener("DOMContentLoaded", () => {
  // ------------------------------------------------------------------------
  // 1. i18n Translations Dictionary
  // ------------------------------------------------------------------------
  const translations = {
    tr: {
      site_title: "triM. — Linux İçin Modern ve Hafif Video Düzenleyici (Rust NLE)",
      nav_features: "Özellikler",
      nav_architecture: "Mimari",
      nav_shortcuts: "Kısayollar",
      nav_install: "Kurulum",
      nav_github: "GitHub",

      hero_pill: "%100 Saf Rust • Sıfır Electron • 19 MB Yerel Linux İkili Dosyası",
      hero_title_1: "Linux İçin",
      hero_title_2: "Yüksek Performanslı",
      hero_title_3: "Yerel Video Düzenleyici.",
      hero_desc: "Rust, egui ve FFmpeg üzerine sıfırdan inşa edilmiş modern NLE. 235+ FPS asenkron oynatma akışı, çok kanallı stereo ses mikseri ve manyetik kurgu zaman çizgisi — Electron hantallığı ve web yükü olmadan.",
      cta_download: "Son Sürümü İndir",
      cta_architecture: "Mimarisi ve Modülleri İncele",
      quick_install_copy: "Kopyala",
      quick_install_copied: "Kopyalandı!",

      tab_timeline: "Aktif Kurgu & Monitör",
      tab_empty: "Minimal Açılış Ekranı",

      stat_1_label: "Bağımsız Native ELF",
      stat_1_desc: "Sıfır Electron / 0 web bellek yükü",
      stat_2_label: "FPS Oynatma Akışı",
      stat_2_desc: "PlaybackStreamer rawvideo borusu",
      stat_3_label: "UI Kilitlenme Süresi",
      stat_3_desc: "AsyncFrameDecoder ile akıcı sarma",
      stat_4_label: "Birim & Entegrasyon Testi",
      stat_4_desc: "%100 başarılı test kapsamı",

      feat_label: "Yetenekler & Motor",
      feat_title: "Hızlı, Kararlı ve Güçlü Masaüstü Kurgusu",
      feat_subtitle: "Gereksiz katmanlar ve şişkinlik olmadan, yalnızca videonuzu en yüksek hızda kesip işlemeye odaklanan modern NLE mimarisi.",

      card_1_title: "Yüksek Hızlı Asenkron Video Akış Motoru",
      card_1_desc: "Oynatma sırasında UI thread'i dondurmadan arka planda çalışan PlaybackStreamer; 640x360 rawvideo borusu ile 235+ FPS hızında kare üretir. Zaman çizgisi sarılırken eski istekler otomatik olarak atılarak sıfır takılmayla en güncel kare gösterilir.",
      card_2_title: "48kHz Stereo Ses Mikseri",
      card_2_desc: "Çok kanallı 16-bit PCM miksajı, canlı RMS / Peak çift kanallı VU göstergesi, kanal bazlı ses/mute/solo denetimleri ve tek tıkla sesi ayırma (Detach Audio).",
      card_3_title: "Manyetik Çok Kanallı Zaman Çizgisi",
      card_3_desc: "Katmanlı video (V1..Vn) ve ses (A1..An) kanalları. Oynatma çizgisinden jilet kesimi (S), başı kırpıp kaydırma (Q), sonu kırpıp kaydırma (W) ve boşluğu kapatarak silme (Shift+Delete).",
      card_4_title: "Anahtar Kareler & Renk Ayarları",
      card_4_desc: "Klip opaklığı, konum X/Y, ölçek, döndürme, parlaklık, kontrast ve doygunluk üzerinde Smoothstep Ease-In-Out eğrileri ile hassas anahtar kare animasyonu.",
      card_5_title: "FFmpeg Filtergraph Dışa Aktarımı",
      card_5_desc: "Çok iş parçacıklı karmaşık filtre grafikleri ([v0][v1]overlay, scale, pad, eq, fade). 1080p, 720p, 4K, YouTube 60fps ve WebM önayarları ile canlı ilerleme takibi.",

      arch_label: "Yazılım Mimarisi",
      arch_title: "8 Modüllü Cargo Çalışma Alanı",
      arch_subtitle: "Uygulama, sorumlulukların net olarak ayrıldığı ve birbirinden bağımsız test edilebilen 8 katmanlı Cargo workspace yapısına sahiptir.",

      crate_core: "Zaman kodu (SMPTE), medya modelleri, zaman çizgisi hiyerarşisi, işaretçiler ve anahtar kare enterpolasyonu.",
      crate_commands: "14 adet atomik, geri alınabilir komut ve kurgu geçmişi yığını (Command Pattern & UndoStack).",
      crate_timeline: "Çizgiden bölme, baş/kuyruk ripple trimleme ve edit noktaları arasında atlama algoritmaları.",
      crate_project: "Atomik kaydetme (.tmp -> rename), .trim ve .cutline JSON geriye dönük uyumu, Autosave yöneticisi.",
      crate_media: "PlaybackStreamer (235+ FPS pipe), AsyncFrameDecoder, 48kHz stereo ses mikseri ve cpal ses cihazı.",
      crate_export: "FFmpeg filtergraph oluşturucu, dışa aktarma işçisi ve canlı yüzde/kare ilerleme bildirimleri.",
      crate_ui: "Donanım hızlandırmalı modern egui arayüzü: kanvas, sinema monitörü, denetçi, VU metre, diyaloglar.",
      crate_app: "Masaüstü ikili dosya giriş noktası, pencere oluşturma ve komut satırı dosya açma desteği (CLI).",

      sc_label: "Üretkenlik & Hız",
      sc_title: "Profesyonel Klavye Kısayolları",
      sc_subtitle: "Farenizi bırakmadan, endüstri standardı NLE kısayollarıyla yıldırım hızında kurgu yapın.",
      filter_all: "Tümü",
      filter_editing: "Kurgu & Kesme",
      filter_playback: "Oynatma & Gezinme",
      filter_project: "Proje & Dosya",

      sc_play: "Oynat / Duraklat",
      sc_split: "Çizgiden Kes / Razor Böl",
      sc_q: "Başı Çizgiye Kırp ve Kaydır",
      sc_w: "Sonu Çizgiye Kırp ve Kaydır",
      sc_delete: "Seçili Klibi Sil",
      sc_ripple_del: "Boşluğu Kapatıp Sil (Ripple)",
      sc_shuttle_rev: "Geri Shuttle (-1x..-8x)",
      sc_shuttle_pause: "Shuttle Duraklat",
      sc_shuttle_fwd: "İleri Shuttle (1x..8x)",
      sc_step: "1 Kare Geri / İleri",
      sc_jump_edit: "Önceki / Sonraki Kesim Noktası",
      sc_home_end: "Başa / Sona Git",
      sc_marker: "İşaretçi (Marker) Ekle",
      sc_undo: "Geri Al / Yinele",
      sc_save: "Projeyi Kaydet",

      inst_label: "Hemen Başlayın",
      inst_title: "Kurulum ve Kaynak Kodu Derleme",
      inst_subtitle: "Tek bir terminal komutuyla derleyip saniyeler içinde çalıştırmaya başlayın.",
      copy_deps: "Kopyala",
      copy_build: "Kopyala",
      build_step: "Ardından depoyu klonlayıp release profilinde derleyin:",
      footer_releases: "Sürümler"
    },

    en: {
      site_title: "triM. — Modern, Native Video Editor for Linux in Rust",
      nav_features: "Features",
      nav_architecture: "Architecture",
      nav_shortcuts: "Shortcuts",
      nav_install: "Installation",
      nav_github: "GitHub",

      hero_pill: "100% Pure Rust • Zero Electron • 19 MB Native Linux Binary",
      hero_title_1: "High-Performance",
      hero_title_2: "Native Video Editing",
      hero_title_3: "Built for Linux.",
      hero_desc: "A modern NLE written from scratch in Rust, egui, and FFmpeg. Multi-threaded 235+ FPS playback streaming, multi-track stereo audio mixing, and precision magnetic timeline — with zero Electron bloat.",
      cta_download: "Download Latest Release",
      cta_architecture: "Explore Architecture & Crates",
      quick_install_copy: "Copy",
      quick_install_copied: "Copied!",

      tab_timeline: "Active Timeline & Monitor",
      tab_empty: "Clean Empty State",

      stat_1_label: "Standalone Native ELF",
      stat_1_desc: "Zero Electron / 0 web memory overhead",
      stat_2_label: "Playback Stream FPS",
      stat_2_desc: "PlaybackStreamer rawvideo pipeline",
      stat_3_label: "UI Lockup / Latency",
      stat_3_desc: "AsyncFrameDecoder silky scrubbing",
      stat_4_label: "Unit & Integration Tests",
      stat_4_desc: "100% passing test coverage",

      feat_label: "Capabilities & Engine",
      feat_title: "Fast, Resilient Desktop Video Editing",
      feat_subtitle: "A modern NLE architecture focused entirely on cutting, sequencing, and exporting video at maximum speed without visual clutter.",

      card_1_title: "High-Speed Asynchronous Video Pipeline",
      card_1_desc: "PlaybackStreamer operates in the background without locking the UI thread, feeding 640x360 rawvideo at 235+ FPS. During scrubbing, stale requests are automatically discarded (request coalescing) to display the freshest frame with zero lag.",
      card_2_title: "48kHz Stereo Audio Engine",
      card_2_desc: "Multi-track 16-bit PCM mixing, live RMS / Peak stereo VU meters, track-level volume/mute/solo controls, and instant audio separation (Detach Audio).",
      card_3_title: "Magnetic Multi-Track Timeline",
      card_3_desc: "Layered video (V1..Vn) and audio (A1..An) tracks. Razor split cuts (S), ripple trim head (Q), ripple trim tail (W), and gapless ripple delete (Shift+Delete).",
      card_4_title: "Keyframes & Color Grading",
      card_4_desc: "Smoothstep Ease-In-Out animation curves across clip position, scale, opacity, rotation, brightness, contrast, and saturation.",
      card_5_title: "FFmpeg Filtergraph Exporter",
      card_5_desc: "Multi-threaded complex filtergraphs ([v0][v1]overlay, scale, pad, eq, fade). Production presets for 1080p, 720p, 4K, YouTube 60fps, and WebM with real-time progress.",

      arch_label: "Software Architecture",
      arch_title: "8-Crate Modular Cargo Workspace",
      arch_subtitle: "The application is structured into 8 cleanly decoupled layers, each isolated and independently testable.",

      crate_core: "SMPTE timecode, media models, timeline hierarchy, markers, and keyframe interpolators.",
      crate_commands: "14 atomic undoable commands and editing history stack (Command Pattern & UndoStack).",
      crate_timeline: "Razor splitting, ripple trimming (head/tail), and edit point jumping algorithms.",
      crate_project: "Atomic disk I/O (.tmp -> rename), .trim and .cutline JSON compatibility, Autosave manager.",
      crate_media: "PlaybackStreamer (235+ FPS pipe), AsyncFrameDecoder, 48kHz stereo mixer, and cpal audio device.",
      crate_export: "FFmpeg filtergraph builder, export runner thread, and live progress percentage/frame notifications.",
      crate_ui: "Hardware-accelerated egui interface: timeline canvas, cinema monitor, inspector, VU meter, dialogs.",
      crate_app: "Desktop binary entry point, window bootstrap, and command-line file loading (CLI).",

      sc_label: "Productivity & Speed",
      sc_title: "Industry-Standard Keyboard Shortcuts",
      sc_subtitle: "Edit at lightning speed without reaching for the mouse using standard NLE bindings.",
      filter_all: "All",
      filter_editing: "Editing & Cuts",
      filter_playback: "Playback & Scrub",
      filter_project: "Project & File",

      sc_play: "Play / Pause",
      sc_split: "Razor Cut / Split at Playhead",
      sc_q: "Ripple Trim Head to Playhead",
      sc_w: "Ripple Trim Tail to Playhead",
      sc_delete: "Delete Selected Clip",
      sc_ripple_del: "Ripple Delete (Close Gap)",
      sc_shuttle_rev: "Shuttle Reverse (-1x..-8x)",
      sc_shuttle_pause: "Shuttle Pause",
      sc_shuttle_fwd: "Shuttle Forward (1x..8x)",
      sc_step: "Step 1 Frame Backward / Forward",
      sc_jump_edit: "Jump to Prev / Next Edit Point",
      sc_home_end: "Jump to Timeline Start / End",
      sc_marker: "Add Marker at Playhead",
      sc_undo: "Undo / Redo",
      sc_save: "Save Project",

      inst_label: "Get Started",
      inst_title: "Installation & Source Compilation",
      inst_subtitle: "Build with a single terminal command and launch in seconds.",
      copy_deps: "Copy",
      copy_build: "Copy",
      build_step: "Then clone the repository and build in release profile:",
      footer_releases: "Releases"
    }
  };

  // ------------------------------------------------------------------------
  // 2. Language Switcher Engine
  // ------------------------------------------------------------------------
  let currentLang = localStorage.getItem("trim_lang") || "tr";

  function applyLanguage(lang) {
    currentLang = lang;
    localStorage.setItem("trim_lang", lang);
    document.documentElement.lang = lang;

    // Update active button state
    document.querySelectorAll(".lang-btn").forEach(btn => {
      btn.classList.toggle("active", btn.dataset.lang === lang);
    });

    const dict = translations[lang] || translations.tr;
    document.title = dict.site_title;

    document.querySelectorAll("[data-i18n]").forEach(el => {
      const key = el.dataset.i18n;
      if (dict[key]) {
        el.textContent = dict[key];
      }
    });
  }

  document.querySelectorAll(".lang-btn").forEach(btn => {
    btn.addEventListener("click", () => {
      applyLanguage(btn.dataset.lang);
    });
  });

  applyLanguage(currentLang);

  // ------------------------------------------------------------------------
  // 3. Bento Card Mouse Spotlight Hover Effect
  // ------------------------------------------------------------------------
  const bentoCards = document.querySelectorAll(".bento-card");
  bentoCards.forEach(card => {
    card.addEventListener("mousemove", e => {
      const rect = card.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;
      card.style.setProperty("--mouse-x", `${x}px`);
      card.style.setProperty("--mouse-y", `${y}px`);
    });
  });

  // ------------------------------------------------------------------------
  // 4. App Showcase Tabs
  // ------------------------------------------------------------------------
  const tabBtns = document.querySelectorAll(".tab-btn");
  const imgTimeline = document.getElementById("img-timeline");
  const imgEmpty = document.getElementById("img-empty");

  tabBtns.forEach(btn => {
    btn.addEventListener("click", () => {
      tabBtns.forEach(b => b.classList.remove("active"));
      btn.classList.add("active");

      const tab = btn.dataset.tab;
      if (tab === "timeline") {
        imgTimeline.classList.add("active");
        imgEmpty.classList.remove("active");
      } else {
        imgEmpty.classList.add("active");
        imgTimeline.classList.remove("active");
      }
    });
  });

  // ------------------------------------------------------------------------
  // 5. Distro Dependencies Tab Switcher
  // ------------------------------------------------------------------------
  const distroTabs = document.querySelectorAll(".distro-tab");
  const depsCode = document.getElementById("deps-code");

  const distroCommands = {
    debian: "sudo apt update && sudo apt install build-essential pkg-config libasound2-dev libx11-dev libgl1-mesa-dev ffmpeg",
    fedora: "sudo dnf install gcc pkgconf-pkg-config alsa-lib-devel libX11-devel mesa-libGL-devel ffmpeg",
    arch: "sudo pacman -S base-devel alsa-lib libx11 mesa ffmpeg"
  };

  distroTabs.forEach(tab => {
    tab.addEventListener("click", () => {
      distroTabs.forEach(t => t.classList.remove("active"));
      tab.classList.add("active");

      const distro = tab.dataset.distro;
      if (distroCommands[distro]) {
        depsCode.textContent = distroCommands[distro];
      }
    });
  });

  // ------------------------------------------------------------------------
  // 6. Keyboard Shortcuts Category Filter
  // ------------------------------------------------------------------------
  const filterBtns = document.querySelectorAll(".filter-btn");
  const shortcutItems = document.querySelectorAll(".shortcut-item");

  filterBtns.forEach(btn => {
    btn.addEventListener("click", () => {
      filterBtns.forEach(b => b.classList.remove("active"));
      btn.classList.add("active");

      const cat = btn.dataset.category;
      shortcutItems.forEach(item => {
        if (cat === "all" || item.dataset.cat === cat) {
          item.style.display = "flex";
        } else {
          item.style.display = "none";
        }
      });
    });
  });

  // ------------------------------------------------------------------------
  // 7. Generic Copy-to-Clipboard Buttons
  // ------------------------------------------------------------------------
  function setupCopyButton(btnId, getTextFn) {
    const btn = document.getElementById(btnId);
    if (!btn) return;

    btn.addEventListener("click", async () => {
      const text = getTextFn();
      try {
        await navigator.clipboard.writeText(text);
        const originalText = btn.textContent;
        const copiedLabel = currentLang === "en" ? "Copied!" : "Kopyalandı!";
        btn.textContent = copiedLabel;
        btn.classList.add("copied");

        setTimeout(() => {
          btn.textContent = originalText;
          btn.classList.remove("copied");
        }, 2000);
      } catch (err) {
        console.error("Clipboard copy failed:", err);
      }
    });
  }

  setupCopyButton("copy-quick-install", () => {
    return document.getElementById("copy-quick-install").dataset.text;
  });

  setupCopyButton("copy-deps", () => {
    return depsCode.textContent.trim();
  });

  setupCopyButton("copy-build", () => {
    return `git clone https://github.com/MOzcelik14/triM.git\ncd triM\ncargo build --release\n./target/release/trim`;
  });

  // ------------------------------------------------------------------------
  // 8. Sticky Header Scroll Effect
  // ------------------------------------------------------------------------
  const header = document.getElementById("site-header");
  window.addEventListener("scroll", () => {
    if (window.scrollY > 24) {
      header.classList.add("scrolled");
    } else {
      header.classList.remove("scrolled");
    }
  }, { passive: true });
});
