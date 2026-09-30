/**
 * triM. — Product Website Interaction & Bilingual Engine
 * Vanilla JavaScript, zero runtime dependencies.
 */

(function () {
  'use strict';

  const translations = {
    tr: {
      site_title: "triM. — Linux İçin Hafif ve Doğrusal Olmayan Video Düzenleyici",
      meta_description: "Linux için modern, hızlı ve yerel video düzenleyici. Python 3.12, Qt6/PySide6, FFmpeg ve PyAV ile geliştirildi. Electron içermez.",
      nav_features: "Özellikler",
      nav_architecture: "Mimari",
      nav_install: "Kurulum",
      nav_roadmap: "Yol Haritası",
      nav_github: "GitHub",
      hero_badge: "v0.2.0 Yayınlandı — Ubuntu / Debian & Linux",
      hero_subtitle: "Linux için hafif, modern ve yerel bir doğrusal olmayan video düzenleyici.",
      hero_desc: "Kurgu, kırpma, zaman çizgisi ve ses senkronizasyonuna odaklanan yalın masaüstü deneyimi. Python, Qt6, FFmpeg ve PyAV üzerine inşa edildi; gereksiz bellek yükü veya karmaşık bulut bağımlılıkları barındırmaz.",
      cta_download: "Ubuntu / Debian Paketini İndir (.deb)",
      cta_source: "Kaynak Kod",
      quick_install_copy: "Kopyala",
      quick_install_copied: "Kopyalandı!",
      preview_title: "triM. — Yerel Linux Arayüzü",
      preview_tab_timeline: "Zaman Çizgisi & Kurgu",
      preview_tab_empty: "Açılış Durumu (Minimal)",
      
      feat_section_label: "İşlevsellik",
      feat_section_title: "Hızlı, Hassas ve Sağlam Kurgu Araçları",
      feat_section_lead: "Şişkinlikten uzak, sadece videonuzu kesmeye, hizalamaya ve dışa aktarmaya odaklanan çekirdek NLE yetenekleri.",
      
      feat_1_title: "Çok Kanallı Zaman Çizgisi",
      feat_1_desc: "Bağımsız video ve ses kanalları, manyetik kare yakalama (snapping), razor jilet kesimi (S) ve boşluksuz silme (Shift+Delete).",
      feat_2_title: "Donanım Destekli Oynatma & Ses Senkronu",
      feat_2_desc: "PyAV ve ffmpeg libav ile tam kare hassasiyetinde demuxing, sıfır çıtırtılı ses tamponlama ve gerçek zamanlı oynatma hızı.",
      feat_3_title: "Dalga Formu & Canlı VU Metre",
      feat_3_desc: "Çok iş parçacıklı ses dalga formu analizi, önbellekleme ve çift kanallı dinamik RMS dBFS tepe göstergesi.",
      feat_4_title: "İnteraktif Fade & Geçiş Kolları",
      feat_4_desc: "Doğrudan klip üzerinde sürüklenebilir video opaklık ve ses kazanç eğrileri, siyaha/beyaza geçiş efektleri.",
      feat_5_title: "Tipografi & Alt Bant (Title) Üretici",
      feat_5_desc: "Sistem yazı tipleriyle altyazı, alt bant (lower third) ve metin kartı oluşturma, alfa kanallı şeffaf yerleşim.",
      feat_6_title: "Renk & Görüntü Ayarları",
      feat_6_desc: "Parlaklık, kontrast ve doygunluk denetimleri, tek tıkla siyah-beyaz dönüştürme ve görsel ölçekleme/konumlandırma.",
      feat_7_title: "Arka Planda FFmpeg Dışa Aktarımı",
      feat_7_desc: "Preset tabanlı çözünürlük/FPS seçenekleri, filtre grafiği derlemesi ve arayüzü dondurmayan arka plan render işlemi.",
      feat_8_title: "Otomatik Kayıt & Çökme Kurtarma",
      feat_8_desc: "Periyodik arka plan durum yedekleri sayesinde beklenmeyen sistem kapanmalarında projeyi tek tıkla geri getirme.",
      feat_9_title: "Flatpak Paket Dağıtımı",
      feat_9_desc: "Flathub üzerinden tek tıkla kurulum ve yalıtılmış çalışma ortamı için AppStream / Flatpak entegrasyonu.",
      feat_10_title: "4K Proxy Medya İş Akışı",
      feat_10_desc: "Düşük donanımlı sistemlerde bile akıcı 60fps kurgu için otomatik düşük çözünürlüklü önizleme proxy üretimi.",

      badge_ready: "Hazır",
      badge_planned: "Planlanan",
      badge_done: "Tamamlandı",
      badge_future: "Gelecek",

      spec_section_label: "Teknoloji",
      spec_section_title: "Gerçek Yerel Masaüstü Performansı",
      spec_section_lead: "triM., web sarmalayıcıları (Electron) kullanmaz. Linux grafik altyapısına doğrudan Qt6 ile bağlanır.",
      spec_ui: "Grafik Arayüz (GUI)",
      spec_ui_val: "PySide6 / Qt6 (Wayland & X11)",
      spec_media: "Video & Ses Çözücü",
      spec_media_val: "PyAV (FFmpeg C API bağlayıcıları)",
      spec_render: "Render & Dışa Aktarma",
      spec_render_val: "FFmpeg CLI filtre motoru",
      spec_data: "Sayısal İşleme & Waveform",
      spec_data_val: "NumPy 1.26+",
      spec_app_id: "Uygulama Kimliği",
      spec_app_id_val: "io.github.mozcelik14.triM",
      spec_integration_title: "Linux Yerel Masaüstü Entegrasyonu",
      spec_integration_desc: "Ölçeklenebilir hicolor simge hiyerarşisi, .trim ve eski .cutline proje dosyaları için MIME eşleme, Wayland kesirli ölçekleme ve AppStream meta veri standartlarıyla tam uyumludur.",

      install_section_label: "Dağıtım",
      install_section_title: "Hemen Kurun ve Başlayın",
      install_section_lead: "Ubuntu, Debian, Linux Mint, Pop!_OS ve türevleri için hazır .deb paketini doğrudan kurabilir veya tek komutla masaüstünüze entegre edebilirsiniz.",
      install_tab_deb: "Ubuntu / Debian / Mint (.deb)",
      install_tab_script: "Tek Komutla Yükleyici",
      install_tab_source: "Kaynak Koddan",

      deb_comment_1: "# 1. v0.2.0 sürümünden trim_0.2.0_all.deb paketini indirin",
      deb_comment_2: "# 2. apt ile kurun (bağımlılıkları otomatik çözer)",
      deb_comment_3: "# 3. Uygulama menüsünden veya terminalden başlatın",
      script_comment_1: "# Depoyu klonlayıp otomatik masaüstü yükleyicisini çalıştırın",
      script_comment_2: "# İkonları, kısayolları ve ~/.local/bin/trim bağlantısını kurar",
      source_comment_1: "# Geliştirici sanal ortam kurulumu",

      roadmap_section_label: "Gelişim",
      roadmap_section_title: "Geliştirme Yol Haritası",
      roadmap_section_lead: "Projenin mevcut kararlı yetenekleri ve yakın gelecekteki hedefleri.",
      roadmap_current: "v0.2.0 — Mevcut Sürüm",
      roadmap_next: "v0.3.0 — Yakında",
      roadmap_later: "Gelecek Sürümler",

      road_1: "Çok kanallı video ve ses zaman çizgisi",
      road_2: "Kare hassasiyetinde ses senkronu ve PyAV motoru",
      road_3: "İnteraktif fade kolları ve dalga formu görselleştirme",
      road_4: "Çift kanallı stereo VU metre ve J-K-L shuttle kontrolleri",
      road_5: "Renk ayarları, geçişler ve başlık/metin üretici",
      road_6: "Ubuntu / Debian (.deb) paketleme ve otomatik masaüstü yükleyici",
      road_7: "Çoklu klip grup seçimi ve toplu taşıma",
      road_8: "Flathub / Flatpak manifest dağıtımı",
      road_9: "Kanal mikseri (Mute/Solo) ve ses kazanç sürgüleri",
      road_10: "Konum, ölçek ve opaklık için Keyframe animasyonu",
      road_11: "Düşük donanımlı sistemler için 4K proxy akışı",
      road_12: "LADSPA / LV2 ses filtre eklenti desteği",
      road_13: "Sesi Ayır (Detach Audio) bağımsız kanal iş akışı",
      road_14: "Hızlı ripple kırpma kısayolları (Q/W)",
      road_15: "Hız rampalama ve optik akış zaman eşleme",
      road_16: "İç içe zaman çizgileri ve çoklu kamera kurgusu",

      oss_section_label: "Topluluk",
      oss_section_title: "Özgür ve Açık Kaynak",
      oss_section_lead: "triM. MIT lisansı altında tamamen açık kaynak kodludur. Bağımsız geliştirilir, katkılara açıktır.",
      oss_btn_repo: "GitHub Deposu",
      oss_btn_issues: "Sorun Bildir (Issues)",
      oss_btn_releases: "Tüm Sürümler",

      footer_copy: "triM. — Bağımsız, modern Linux video kurgu yazılımı.",
      footer_license: "MIT Lisansı",
      footer_author: "M. Özçelik"
    },
    en: {
      site_title: "triM. — A Lightweight Non-Linear Video Editor for Linux",
      meta_description: "A fast, modern, native non-linear video editor for Linux. Built with Python 3.12, Qt6/PySide6, FFmpeg, and PyAV. No Electron bloat.",
      nav_features: "Features",
      nav_architecture: "Architecture",
      nav_install: "Installation",
      nav_roadmap: "Roadmap",
      nav_github: "GitHub",
      hero_badge: "v0.2.0 Released — Ubuntu / Debian & Linux",
      hero_subtitle: "A lightweight, native non-linear video editor for Linux.",
      hero_desc: "A streamlined creative desktop experience focused on cutting, trimming, timeline sequencing, and audio synchronization. Built with Python, Qt6, FFmpeg, and PyAV — without cloud lock-in or Electron memory bloat.",
      cta_download: "Download Ubuntu / Debian Package (.deb)",
      cta_source: "Source Code",
      quick_install_copy: "Copy",
      quick_install_copied: "Copied!",
      preview_title: "triM. — Native Linux Interface",
      preview_tab_timeline: "Timeline & Editing",
      preview_tab_empty: "Clean Empty State",
      
      feat_section_label: "Capabilities",
      feat_section_title: "Fast, Precise, and Solid Editing Tools",
      feat_section_lead: "Essential non-linear editing capabilities focused purely on cutting, arranging, and exporting video without visual clutter.",
      
      feat_1_title: "Multi-Track Timeline",
      feat_1_desc: "Independent video and audio tracks with magnetic frame snapping, razor cuts (S), and ripple deletion (Shift+Delete).",
      feat_2_title: "Hardware-Assisted Playback & Sync",
      feat_2_desc: "Accurate sequential demuxing with PyAV and ffmpeg libav, zero-crackling audio buffer queue, and exact real-time playback.",
      feat_3_title: "Audio Waveforms & Live VU Meter",
      feat_3_desc: "Multi-threaded peak extraction, persistent caching, and dual-channel dynamic RMS dBFS meters with peak hold.",
      feat_4_title: "Interactive Fade & Transition Handles",
      feat_4_desc: "Direct clip-level draggable opacity curves, audio gain ramps, and smooth Dip to Black / Dip to White transitions.",
      feat_5_title: "Typography & Title Generator",
      feat_5_desc: "Native title, subtitle, and lower-third generation with system font selection, alignment flags, and alpha transparency.",
      feat_6_title: "Color Adjustments & Transforms",
      feat_6_desc: "Live brightness, contrast, and saturation controls, instant black-and-white conversion, and position/scale transforms.",
      feat_7_title: "Background FFmpeg Rendering",
      feat_7_desc: "Preset-driven resolution and framerate exports, multi-threaded filtergraph execution, and non-blocking background queue.",
      feat_8_title: "Autosave & Crash Recovery",
      feat_8_desc: "Periodic background state snapshots ensuring immediate one-click project recovery in the event of power or system failure.",
      feat_9_title: "Flatpak Distribution",
      feat_9_desc: "AppStream metadata and Flatpak sandbox packaging for one-click installation via Flathub.",
      feat_10_title: "4K Proxy Media Workflow",
      feat_10_desc: "Automatic background downscaling to edit high-bitrate 4K footage smoothly at 60fps on modest hardware.",

      badge_ready: "Available",
      badge_planned: "Planned",
      badge_done: "Done",
      badge_future: "Future",

      spec_section_label: "Engineering",
      spec_section_title: "True Native Desktop Performance",
      spec_section_lead: "triM. rejects heavy web wrappers. It connects directly to Linux display servers via Qt6 and Wayland.",
      spec_ui: "User Interface (GUI)",
      spec_ui_val: "PySide6 / Qt6 (Wayland & X11)",
      spec_media: "Media Demuxing & Audio",
      spec_media_val: "PyAV (FFmpeg C API bindings)",
      spec_render: "Render & Filter Engine",
      spec_render_val: "FFmpeg CLI filtergraph",
      spec_data: "Numerical & Waveforms",
      spec_data_val: "NumPy 1.26+",
      spec_app_id: "Application ID",
      spec_app_id_val: "io.github.mozcelik14.triM",
      spec_integration_title: "Linux Native Integration",
      spec_integration_desc: "Complies with FreeDesktop standards including scalable hicolor icon hierarchy, MIME association for .trim and legacy .cutline files, Wayland fractional scaling, and AppStream cataloging.",

      install_section_label: "Distribution",
      install_section_title: "Install in Seconds",
      install_section_lead: "Download the prebuilt .deb package directly for Ubuntu, Debian, Linux Mint, Pop!_OS and derivatives, or install seamlessly using the automated desktop script.",
      install_tab_deb: "Ubuntu / Debian / Mint (.deb)",
      install_tab_script: "One-Line Installer",
      install_tab_source: "From Source",

      deb_comment_1: "# 1. Download trim_0.2.0_all.deb from GitHub Releases",
      deb_comment_2: "# 2. Install with apt (automatically handles dependencies)",
      deb_comment_3: "# 3. Launch from application menu or terminal",
      script_comment_1: "# Clone and run automated desktop installer",
      script_comment_2: "# Installs icons, desktop shortcuts, ~/.local/bin/trim, and environment",
      source_comment_1: "# Developer virtual environment setup",

      roadmap_section_label: "Roadmap",
      roadmap_section_title: "Development Milestones",
      roadmap_section_lead: "Transparent roadmap showing current verified capabilities and planned architecture milestones.",
      roadmap_current: "v0.2.0 — Current Release",
      roadmap_next: "v0.3.0 — Up Next",
      roadmap_later: "Future Milestones",

      road_1: "Multi-track video and audio timeline",
      road_2: "Frame-accurate audio sync and PyAV playback engine",
      road_3: "Interactive fade handles and audio waveform caching",
      road_4: "Dual-channel stereo VU meter and J-K-L shuttle playback",
      road_5: "Color adjustments, transitions, and title generator",
      road_6: "Ubuntu / Debian (.deb) packaging and automated desktop installer",
      road_7: "Multi-clip box selection and batch drag operations",
      road_8: "Flathub / Flatpak manifest packaging",
      road_9: "Track mixer with mute/solo & audio gain sliders",
      road_10: "Keyframe animation for position, scale, and opacity",
      road_11: "Background 4K proxy generation workflow",
      road_12: "LADSPA / LV2 audio effect plugin architecture",
      road_13: "Audio detach ('Detach Audio') workflow",
      road_14: "Ripple trim shortcuts (Q/W)",
      road_15: "Speed ramping & optical flow time remapping",
      road_16: "Nested timelines & multicam synchronization",

      oss_section_label: "Community",
      oss_section_title: "Free and Open Source",
      oss_section_lead: "triM. is built openly under the permissive MIT license. Independent, hackable, and welcoming to contributors.",
      oss_btn_repo: "GitHub Repository",
      oss_btn_issues: "Issue Tracker",
      oss_btn_releases: "All Releases",

      footer_copy: "triM. — Modern, lightweight open-source video editing for Linux.",
      footer_license: "MIT License",
      footer_author: "M. Özçelik"
    }
  };

  let currentLang = 'en';

  function detectLanguage() {
    const saved = localStorage.getItem('trim_lang');
    if (saved === 'tr' || saved === 'en') {
      return saved;
    }
    const nav = navigator.language || navigator.userLanguage || '';
    return nav.toLowerCase().startsWith('tr') ? 'tr' : 'en';
  }

  function setLanguage(lang) {
    if (!translations[lang]) return;
    currentLang = lang;
    localStorage.setItem('trim_lang', lang);
    document.documentElement.lang = lang;

    // Update document title and meta
    document.title = translations[lang].site_title;
    const metaDesc = document.querySelector('meta[name="description"]');
    if (metaDesc) metaDesc.setAttribute('content', translations[lang].meta_description);

    // Update all i18n text nodes
    document.querySelectorAll('[data-i18n]').forEach((el) => {
      const key = el.getAttribute('data-i18n');
      if (translations[lang][key]) {
        el.textContent = translations[lang][key];
      }
    });

    // Update switcher active states
    document.querySelectorAll('.lang-btn').forEach((btn) => {
      btn.classList.toggle('active', btn.getAttribute('data-lang') === lang);
    });
  }

  // Setup Language Switcher
  function initLanguage() {
    const initial = detectLanguage();
    setLanguage(initial);

    document.querySelectorAll('.lang-btn').forEach((btn) => {
      btn.addEventListener('click', () => {
        const selected = btn.getAttribute('data-lang');
        setLanguage(selected);
      });
    });
  }

  // Setup Image Preview Tabs
  function initPreviewTabs() {
    const tabTimeline = document.getElementById('tab-preview-timeline');
    const tabEmpty = document.getElementById('tab-preview-empty');
    const previewImg = document.getElementById('preview-image');

    if (!tabTimeline || !tabEmpty || !previewImg) return;

    tabTimeline.addEventListener('click', () => {
      tabTimeline.classList.add('active');
      tabEmpty.classList.remove('active');
      previewImg.src = './assets/screenshots/trim_timeline.png';
      previewImg.alt = 'triM. Timeline and Video Preview Interface';
    });

    tabEmpty.addEventListener('click', () => {
      tabEmpty.classList.add('active');
      tabTimeline.classList.remove('active');
      previewImg.src = './assets/screenshots/trim_empty_state.png';
      previewImg.alt = 'triM. Clean Empty State Interface';
    });
  }

  // Setup Installation Tabs
  function initInstallTabs() {
    const tabBtns = document.querySelectorAll('.install-tab-btn');
    const panels = document.querySelectorAll('.install-panel');

    tabBtns.forEach((btn) => {
      btn.addEventListener('click', () => {
        const targetId = btn.getAttribute('data-target');
        tabBtns.forEach((b) => b.classList.remove('active'));
        panels.forEach((p) => p.classList.remove('active'));

        btn.classList.add('active');
        const targetPanel = document.getElementById(targetId);
        if (targetPanel) targetPanel.classList.add('active');
      });
    });
  }

  // Setup Quick Install Copy
  function initCopyButtons() {
    document.querySelectorAll('.copy-btn').forEach((btn) => {
      btn.addEventListener('click', () => {
        const textToCopy = btn.getAttribute('data-clipboard-text');
        if (!textToCopy) return;

        navigator.clipboard.writeText(textToCopy).then(() => {
          const originalText = btn.textContent;
          btn.textContent = translations[currentLang].quick_install_copied;
          btn.style.color = '#38C172';
          setTimeout(() => {
            btn.textContent = originalText;
            btn.style.color = '';
          }, 2000);
        }).catch((err) => {
          console.warn('Clipboard write failed:', err);
        });
      });
    });
  }

  // DOM Content Loaded
  document.addEventListener('DOMContentLoaded', () => {
    initLanguage();
    initPreviewTabs();
    initInstallTabs();
    initCopyButtons();
  });
})();
