// triM. — Interactive Landing Page Script (No external framework dependencies)
document.addEventListener("DOMContentLoaded", () => {
  const tr = {
    title: "triM. — Masaüstü Video Düzenleyici",
    description: "triM. — Hızlı, hafif ve bağımsız masaüstü video düzenleyici. 4K akıllı proxy, çok kanallı zaman çizgisi, SRT/VTT altyazı motoru, canlı ses kaydı ve GPU hızlandırmalı render desteği.",
    skip: "İçeriğe geç",
    nav_features: "Özellikler",
    nav_proxy: "Akıllı Proxy",
    nav_audio: "Ses & Renk",
    nav_shortcuts: "Kısayollar",
    nav_download: "İndir",

    hero_kicker: "MASAÜSTÜ VİDEO DÜZENLEYİCİ · v0.6.0",
    hero_line1: "Kurgunun ritmini",
    hero_line2: "hissedin.",
    hero_description: "Büyük kurulum dosyaları, ağır tarayıcı motorları ve karmaşık abonelikler olmadan video kurgulayın. triM., 4K akıllı proxy iş akışı, zaman çizgisi ses kaydı, SRT altyazı motoru ve GPU hızlandırmalı render desteğiyle ihtiyacınız olan her şeyi yaklaşık 19 MB boyutta sunar.",
    hero_download: "Hemen İndir",
    hero_explore: "Özellikleri Keşfet",
    hero_note: "Linux ve Windows için bağımsız yerel masaüstü uygulaması · ~19 MB",

    preview_heading: "UYGULAMADAN CANLI GÖRÜNTÜLER",
    tab_workspace: "Kurgu Masası",
    tab_subtitles: "Altyazı & Smart Cut",
    tab_vertical: "Dikey Reels (9:16)",
    tab_audio: "Ses & EQ",
    tab_shortcuts: "Kısayol Profilleri",
    tab_export: "GPU Dışa Aktarım",
    preview_caption: "Gerçek uygulama görüntüsü / triM. v0.6.0",

    features_index: "KURGU ARAÇLARI",
    features_title: "İhtiyacınız olan her şey <em>tek ekranda.</em>",
    features_intro: "Gereksiz katmanlar ve dikkat dağıtan menüler olmadan; videonuzu ekleyin, düzenleyin ve tamamlayın.",

    feat_proxy_title: "4K Çekimlerde Akıcı Önizleme",
    feat_proxy_desc: "Büyük 4K ve UHD ham videolarla çalışırken önizlemenin takılmaması için tek tıkla arka planda 720p H.264 proxy üretin. Zaman çizgisindeki [⚡ Proxy] düğmesiyle hafif önizlemeye geçin; kurgunuz 60 FPS akarken, dışa aktarım sırasında triM. doğrudan orijinal tam çözünürlüklü dosyanızı render eder.",

    feat_timeline_title: "Hassas Kesim & Akıllı Sahne Algılama",
    feat_timeline_desc: "Çok kanallı video ve ses izleri üzerinde serbestçe çalışın. Manyetik yapışma, çizgiden kesme (S), dalgalanma kırpması (Q/W) ve boşluk kapatma (Shift+Delete) hız kazandırır. Akıllı Sahne Algılama (Smart Cut) özelliğiyle uzun çekimlerdeki kamera açı değişimleri otomatik algılanır ve tek tıkla jiletle ayrılır.",

    feat_audio_title: "Canlı Seslendirme & EBU R128",
    feat_audio_desc: "Harici bir kayıt programı açmanıza gerek kalmadan, ses izinin başındaki [🎤] butonuna basarak doğrudan mikrofonunuzdan konuşma kaydedin. Çok kanallı mikser konsolu, konuşma netliği EQ hazır ayarları, dip gürültü filtresi (Noise Gate) ve YouTube, Spotify standartlarında (-14 LUFS / -23 LUFS) EBU R128 ses normalizasyonu elinizin altında.",

    feat_text_color_title: "Altyazı Süiti, Canlı Titler & 3D LUT",
    feat_text_color_desc: ".SRT ve .VTT altyazı dosyalarını doğrudan projenize aktarın ya da dışa verin. Canlı Titler ile alt yazı kutularını, boyutları ve renkleri Inspector üzerinden anında düzenleyin. Dahili Teal & Orange, Vintage Film ve Monochrome gibi sinematik profilleri kullanın veya harici .cube 3D LUT dosyalarınızı projeye dahil edin.",

    feature_note: "Yukarıdaki tüm ekran görüntüleri doğrudan triM. masaüstü uygulamasından alınmıştır.",

    workflow_index: "YAKLAŞIM",
    workflow_title: "Gereksiz yük yok.<br><em>Doğrudan masaüstünüzde.</em>",
    workflow_text: "triM., bir web tarayıcısı penceresi içine hapsedilmiş ağır uygulamalardan biri değildir. Bilgisayarınızın kaynaklarını boşa harcamayan, açılışı birkaç saniye süren ve internet bağlantısına ihtiyaç duymayan yerel bir masaüstü yazılımıdır. Tüm medyalarınız, proje dosyalarınız ve yaratıcı süreciniz tamamen sizin kontrolünüzdedir.",
    point_1: "~19 MB Bağımsız Masaüstü",
    point_2: "GPU Donanım Render Desteği",
    point_3: "Taşınabilir Kayıpsız Proje Paketleme",

    shortcuts_index: "KISAYOLLAR",
    shortcuts_title: "Alıştığınız kısayolları <em>bırakmayın.</em>",
    shortcuts_description: "Kurgu yaparken parmak hafızası her şeydir. triM., kendi seri tuş düzeninin yanı sıra tek tıkla Adobe Premiere Pro veya DaVinci Resolve kısayol profillerine geçmenizi sağlar.",
    shortcuts_link: "Tüm kısayolları gör",
    key_split: "Klip Bölme (Split / Razor)",
    key_trim: "Başı / Sonu Kırp (Ripple Trim)",
    key_select: "Seçim Aracı (Selection)",
    key_ripple: "Dalgalanma Düzenleme (Ripple)",
    key_play: "Oynat / Duraklat",
    key_shuttle: "Geri / Dur / İleri (Shuttle)",
    key_undo: "Geri Al",

    install_index: "BAŞLANGIÇ",
    install_title: "Saniyeler içinde <em>başlayın.</em>",
    install_description: "triM. tamamen ücretsiz ve açık kaynaklıdır. Tek satırlık komutla kurabilir, hazır paketleri indirebilir veya kaynak kodundan derleyebilirsiniz.",
    install_release: "GitHub Sürümlerine Git",
    terminal_title: "Hızlı Kurulum ve İndirme",

    step_linux_curl: "Terminalinizi açın ve tek satırda kurun:",
    hint_linux_curl: "Uygulama simgesi ve masaüstü kısayolu otomatik oluşturulur. Kaldırmak için: install.sh --uninstall",
    step_win_ps: "PowerShell'i açın ve tek komutla kurun:",
    hint_win_ps: "Windows Başlat menüsü kısayolu oluşturulur. FFmpeg otomatik kontrol edilir.",
    step_direct_download: "İşletim sisteminize uygun paketi doğrudan indirin:",
    step_build: "Depoyu klonlayıp kaynak kodundan derleyin:",
    hint_source_build: "Sisteminizde derleme araçlarının ve FFmpeg'in kurulu olması yeterlidir.",

    copy: "Kopyala",
    copied: "Kopyalandı!",
    copy_failed: "Kopyalanamadı",

    footer_tagline: "Masaüstü video düzenleyici. Hafif, hızlı ve bağımsız.",
    footer_releases: "Sürümler ↗",
    footer_license: "GPL-3.0 Lisansı ↗",
    footer_credit: "Linux & Windows için özenle geliştirildi.",
    back_top: "Yukarı ↑",
    menu_open: "Menüyü aç",
    menu_close: "Menüyü kapat"
  };

  const en = {
    title: "triM. — Desktop Video Editor",
    description: "triM. — Fast, lightweight, and native desktop video editor with 4K smart proxies, multi-track timeline, SRT/VTT subtitles, live voiceover, and GPU hardware export.",
    skip: "Skip to content",
    nav_features: "Features",
    nav_proxy: "Smart Proxy",
    nav_audio: "Audio & Color",
    nav_shortcuts: "Shortcuts",
    nav_download: "Download",

    hero_kicker: "DESKTOP VIDEO EDITOR · v0.6.0",
    hero_line1: "Edit video with",
    hero_line2: "clarity and speed.",
    hero_description: "Edit video without massive downloads, sluggish web wrappers, or complicated cloud subscriptions. triM. delivers 4K smart proxy workflows, timeline voiceover recording, an SRT subtitle suite, and GPU-accelerated export in a compact ~19 MB native application.",
    hero_download: "Get triM.",
    hero_explore: "Explore Features",
    hero_note: "Standalone native desktop app for Linux & Windows · ~19 MB",

    preview_heading: "REAL APPLICATION SCREENSHOTS",
    tab_workspace: "Workspace",
    tab_subtitles: "Subtitles & Smart Cut",
    tab_vertical: "Vertical 9:16",
    tab_audio: "Audio & EQ",
    tab_shortcuts: "Shortcut Profiles",
    tab_export: "GPU Render",
    preview_caption: "Actual app screenshots / triM. v0.6.0",

    features_index: "THE TOOLS",
    features_title: "Everything you need <em>on one screen.</em>",
    features_intro: "No bloat, no distracting menus. Add your media, edit your timeline, and export your master cut.",

    feat_proxy_title: "Silky-Smooth 4K & UHD Playback",
    feat_proxy_desc: "Keep playback smooth when working with heavy 4K and UHD raw footage. Generate lightweight 720p H.264 proxies in the background with one click. Switch to proxy mode on the timeline for instant 60 FPS scrubbing, while triM. preserves your original full-resolution master files during export.",

    feat_timeline_title: "Precision Cutting & Smart Scene Detection",
    feat_timeline_desc: "Work across layered video and audio tracks with complete precision. Magnetic snapping, razor cutting (S), ripple trims (Q/W), and ripple delete (Shift+Delete) keep you in flow. Smart Scene Detection automatically detects camera cut points in long source takes and slices them instantly.",

    feat_audio_title: "Live Voiceover & EBU R128 Loudness",
    feat_audio_desc: "Record high-quality voiceovers straight into your timeline by clicking the microphone button [🎤] on any audio track. Polish your audio with a multi-channel mixer, Voice Clarity EQ, Noise Gate, and master EBU R128 loudness normalization for YouTube and Spotify (-14 LUFS / -23 LUFS).",

    feat_text_color_title: "Subtitle Suite, Live Titler & 3D LUTs",
    feat_text_color_desc: "Import and export .SRT and .VTT subtitle files seamlessly with automatic lower-third track generation. Use the Live Titler to format text, boxes, and drop shadows in real time. Apply cinematic 3D LUTs (Teal & Orange, Vintage Film, Monochrome Noir) or load your own .cube files.",

    feature_note: "All screenshots displayed above were captured directly from the running triM. desktop application.",

    workflow_index: "THE APPROACH",
    workflow_title: "Zero unnecessary bloat.<br><em>Direct on your desktop.</em>",
    workflow_text: "triM. is not a heavy web application trapped inside a browser window. It is a native desktop editor that respects your hardware, launches in seconds, and never requires an internet connection. Your media, project files, and creative choices remain entirely in your control.",
    point_1: "~19 MB Standalone App",
    point_2: "Hardware GPU Export",
    point_3: "Portable Bundled Projects",

    shortcuts_index: "SHORTCUTS",
    shortcuts_title: "Keep your existing <em>muscle memory.</em>",
    shortcuts_description: "Muscle memory matters when editing. In addition to triM.'s fast default hotkeys, switch on the fly to Adobe Premiere Pro or DaVinci Resolve shortcut profiles.",
    shortcuts_link: "See all keyboard shortcuts",
    key_split: "Split / Razor Tool",
    key_trim: "Ripple Trim (Head / Tail)",
    key_select: "Selection Tool",
    key_ripple: "Ripple Edit Tool",
    key_play: "Play / Pause",
    key_shuttle: "Shuttle (Reverse / Stop / Forward)",
    key_undo: "Undo",

    install_index: "GET STARTED",
    install_title: "Get started in <em>seconds.</em>",
    install_description: "triM. is completely free and open source. Install it with a single terminal command, download standalone packages, or build directly from source.",
    install_release: "Visit GitHub Releases",
    terminal_title: "Quick Install & Download",

    step_linux_curl: "Open your terminal and install with a single command:",
    hint_linux_curl: "Creates desktop shortcut and app icon automatically. To uninstall: install.sh --uninstall",
    step_win_ps: "Open PowerShell and run the one-line installer:",
    hint_win_ps: "Creates Start Menu shortcuts and sets up the portable folder. FFmpeg is automatically checked.",
    step_direct_download: "Download ready-to-run packages directly for your system:",
    step_build: "Clone repository and build with Cargo:",
    hint_source_build: "Requires standard build tools and system FFmpeg.",

    copy: "Copy",
    copied: "Copied!",
    copy_failed: "Failed to copy",

    footer_tagline: "Desktop video editor. Lightweight, fast, and native.",
    footer_releases: "Releases ↗",
    footer_license: "GPL-3.0 License ↗",
    footer_credit: "Built with care for Linux & Windows.",
    back_top: "Back to top ↑",
    menu_open: "Open menu",
    menu_close: "Close menu"
  };

  const translations = { tr, en };
  const richKeys = new Set(["hero_line2", "features_title", "workflow_title", "shortcuts_title", "install_title"]);
  let current = "tr";
  try {
    const saved = localStorage.getItem("trim_language");
    if (saved === "en" || saved === "tr") current = saved;
  } catch (_) {}

  // 1. Language Handler
  function setLanguage(lang) {
    if (!translations[lang]) return;
    current = lang;
    const dict = translations[lang];
    document.documentElement.lang = lang;
    document.title = dict.title;
    document.querySelector('meta[name="description"]')?.setAttribute("content", dict.description);

    document.querySelectorAll("[data-i18n]").forEach(element => {
      const key = element.dataset.i18n;
      if (!(key in dict)) return;
      if (richKeys.has(key)) {
        element.innerHTML = dict[key];
      } else {
        element.textContent = dict[key];
      }
    });

    document.querySelectorAll("[data-lang]").forEach(button => {
      const active = button.dataset.lang === lang;
      button.classList.toggle("is-active", active);
      button.setAttribute("aria-pressed", String(active));
    });

    const menu = document.getElementById("menu-toggle");
    if (menu) {
      menu.setAttribute("aria-label", dict[menu.getAttribute("aria-expanded") === "true" ? "menu_close" : "menu_open"]);
    }
    try { localStorage.setItem("trim_language", lang); } catch (_) {}
  }

  document.querySelectorAll("[data-lang]").forEach(button => {
    button.addEventListener("click", () => setLanguage(button.dataset.lang));
  });
  setLanguage(current);

  // 2. Interactive Showcase (Screenshot Stage)
  document.querySelectorAll("[data-preview]").forEach(button => {
    button.addEventListener("click", () => {
      const name = button.dataset.preview;
      document.querySelectorAll("[data-preview]").forEach(tab => {
        const active = tab === button;
        tab.classList.toggle("is-active", active);
        tab.setAttribute("aria-pressed", String(active));
      });
      document.querySelectorAll("[data-image]").forEach(img => {
        img.classList.toggle("is-active", img.dataset.image === name);
      });
    });
  });

  // 3. Shortcuts Profile Switcher (triM. / Premiere / Resolve)
  const shortcutProfiles = {
    trim: {
      split: "<kbd>S</kbd> <kbd>C</kbd>",
      trim: "<kbd>Q</kbd> <kbd>W</kbd>",
      select: "<kbd>V</kbd>",
      ripple: "<kbd>B</kbd>",
      play: "<kbd>Space</kbd>",
      shuttle: "<kbd>J</kbd> <kbd>K</kbd> <kbd>L</kbd>",
      undo: "<kbd>Ctrl</kbd> <kbd>Z</kbd>"
    },
    premiere: {
      split: "<kbd>Ctrl</kbd> <kbd>K</kbd> / <kbd>C</kbd>",
      trim: "<kbd>Q</kbd> <kbd>W</kbd>",
      select: "<kbd>V</kbd>",
      ripple: "<kbd>B</kbd>",
      play: "<kbd>Space</kbd>",
      shuttle: "<kbd>J</kbd> <kbd>K</kbd> <kbd>L</kbd>",
      undo: "<kbd>Ctrl</kbd> <kbd>Z</kbd>"
    },
    resolve: {
      split: "<kbd>Ctrl</kbd> <kbd>\\</kbd> / <kbd>B</kbd>",
      trim: "<kbd>Shift</kbd> <kbd>[</kbd> <kbd>]</kbd>",
      select: "<kbd>A</kbd>",
      ripple: "<kbd>T</kbd>",
      play: "<kbd>Space</kbd>",
      shuttle: "<kbd>J</kbd> <kbd>K</kbd> <kbd>L</kbd>",
      undo: "<kbd>Ctrl</kbd> <kbd>Z</kbd>"
    }
  };

  document.querySelectorAll("[data-profile]").forEach(button => {
    button.addEventListener("click", () => {
      const prof = button.dataset.profile;
      if (!shortcutProfiles[prof]) return;
      document.querySelectorAll("[data-profile]").forEach(b => b.classList.toggle("is-active", b === button));

      const mapping = shortcutProfiles[prof];
      document.getElementById("sc-split").innerHTML = mapping.split;
      document.getElementById("sc-trim").innerHTML = mapping.trim;
      document.getElementById("sc-select").innerHTML = mapping.select;
      document.getElementById("sc-ripple").innerHTML = mapping.ripple;
      document.getElementById("sc-play").innerHTML = mapping.play;
      document.getElementById("sc-shuttle").innerHTML = mapping.shuttle;
      document.getElementById("sc-undo").innerHTML = mapping.undo;
    });
  });

  // 4. Install Panes Tabs (Linux, Windows, Packages, Source)
  document.querySelectorAll("[data-install]").forEach(button => {
    button.addEventListener("click", () => {
      const target = button.dataset.install;
      document.querySelectorAll("[data-install]").forEach(b => b.classList.toggle("is-active", b === button));
      document.querySelectorAll(".install-pane").forEach(p => {
        p.classList.toggle("is-active", p.id === `pane-${target}`);
      });
    });
  });

  // 5. Copy to Clipboard Button
  document.querySelectorAll("[data-copy]").forEach(button => {
    button.addEventListener("click", async () => {
      const target = document.getElementById(button.dataset.copy);
      if (!target) return;
      try {
        await navigator.clipboard.writeText(target.textContent.trim());
        button.textContent = translations[current].copied;
      } catch (_) {
        button.textContent = translations[current].copy_failed;
      }
      button.disabled = true;
      setTimeout(() => {
        button.disabled = false;
        button.textContent = translations[current].copy;
      }, 1600);
    });
  });

  // 6. Mobile Navigation Menu Toggle
  const menu = document.getElementById("menu-toggle");
  const nav = document.getElementById("primary-nav");
  if (menu && nav) {
    const setMenu = open => {
      menu.setAttribute("aria-expanded", String(open));
      menu.setAttribute("aria-label", translations[current][open ? "menu_close" : "menu_open"]);
      nav.classList.toggle("is-open", open);
    };
    menu.addEventListener("click", () => setMenu(menu.getAttribute("aria-expanded") !== "true"));
    nav.querySelectorAll("a").forEach(link => link.addEventListener("click", () => setMenu(false)));
    document.addEventListener("keydown", e => { if (e.key === "Escape") setMenu(false); });
    window.addEventListener("resize", () => { if (window.innerWidth > 900) setMenu(false); });
  }

  // 7. Scrolled Header Border
  const header = document.getElementById("site-header");
  const updateHeader = () => header?.classList.toggle("is-scrolled", window.scrollY > 12);
  window.addEventListener("scroll", updateHeader, { passive: true });
  updateHeader();
});
