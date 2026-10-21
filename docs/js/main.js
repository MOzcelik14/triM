// triM. — static GitHub Pages interactions; no framework required.
document.addEventListener("DOMContentLoaded", () => {
  const tr = {
    title: "triM. — Linux için video kurgusu, kendi ritminde.",
    description: "triM. — Linux için Rust ile geliştirilmiş, hafif ve yerel video düzenleyici. Çok kanallı kurgu, ses miksajı ve FFmpeg ile dışa aktarım.",
    skip: "İçeriğe geç", nav_features: "Özellikler", nav_workflow: "Yaklaşım", nav_shortcuts: "Kısayollar", nav_download: "İndir",
    hero_kicker: "LINUX İÇİN, LINUX'TA.", hero_line1: "İyi bir kurgu", hero_line2: "akışını", hero_line3: "bozmaz.",
    hero_description: "triM. Linux için geliştirilen açık kaynaklı, yerel bir video editörü. Görüntüyü, sesi ve ritmi aynı çizgide buluştur. Arada hiçbir fazlalık olmadan.",
    hero_download: "triM.'yi indir", hero_explore: "Arayüzü keşfet", hero_note: "Açık kaynak · Rust · Electron yok.",
    preview_heading: "YAKINDAN BİR BAKIŞ", preview_edit: "Kurgu", preview_start: "Başlangıç", preview_caption: "Gerçek uygulama görüntüsü / triM. v0.4.0",
    stack_caption: "ALTINDA NE VAR?", features_index: "ARAÇLAR", features_title: "Her karede <em>kontrol sende.</em>",
    features_intro: "Kurgu masasında ihtiyacın olan şeyler. Birbirini tamamlayan, dikkat dağıtmayan araçlar.",
    feature_1_title: "Kes. Taşı. Devam et.", feature_1_desc: "Çok kanallı zaman çizgisi, manyetik hizalama, jilet kesimi ve ripple düzenleme. Fikrinin hızına yetişen bir kurgu alanı.",
    feature_2_title: "Ses, hikâyenin yarısı.", feature_2_desc: "Çok kanallı stereo miksaj, seviye takibi, mute/solo ve videodan sesi ayırma.",
    feature_3_title: "Hareketi sen belirle.", feature_3_desc: "Anahtar karelerle konum, ölçek, opaklık ve renk ayarlarına hayat ver.",
    feature_4_title: "Kurgunu dışarı taşı.", feature_4_desc: "FFmpeg tabanlı dışa aktarımla projenin son halini oluştur. 1080p, 4K, WebM ve daha fazlası.",
    feature_note: "Görsel şemalar temsili; yukarıdaki uygulama ekranları gerçektir.",
    workflow_index: "YAKLAŞIM", workflow_title: "Daha az katman.<br><em>Daha çok hikâye.</em>",
    workflow_text: "triM. bir web uygulamasının masaüstüne taşınmış hali değil. Linux için Rust ile yazılmış yerel bir editör. Medyan, proje dosyaların ve yaratıcı sürecin senin elinde.",
    point_1: "Yerel masaüstü", point_2: "Açık kaynak kod", point_3: "FFmpeg altyapısı",
    shortcuts_index: "KISAYOLLAR", shortcuts_title: "Fikrin hızlıysa,<br><em>ellerin de olsun.</em>",
    shortcuts_description: "Fareyi daha az kullan, ritmi daha az böl. triM.’de en sık kullanacağın birkaç kısayol.", shortcuts_link: "Tüm kısayolları gör",
    key_play: "Oynat / duraklat", key_cut: "Oynatma kafasından kes", key_trim: "Başı / sonu kırp", key_shuttle: "Geri / dur / ileri", key_undo: "Geri al",
    install_index: "BAŞLANGIÇ", install_title: "Hazırsan,<br><em>kurgu başlasın.</em>",
    install_description: "triM. ücretsiz ve açık kaynaklı. Yayınlanan sürümü indir veya kaynak kodundan derle. Seçim senin.",
    install_release: "GitHub sürümlerine git", terminal_title: "Kaynak koddan kurulum",
    step_deps: "Gerekli paketleri yükle", step_build: "Depoyu klonla ve derle", copy: "Kopyala", copied: "Kopyalandı!", copy_failed: "Kopyalanamadı",
    terminal_hint: "Rust / Cargo kurulu olmalıdır. Dağıtımınıza göre ek paketler gerekebilir.",
    footer_tagline: "Kurgunun kendi ritmi var.", footer_releases: "Sürümler ↗", footer_credit: "Linux için özenle geliştirildi.", back_top: "Yukarı ↑",
    menu_open: "Menüyü aç", menu_close: "Menüyü kapat"
  };
  const en = {
    title: "triM. — Native video editing for Linux.",
    description: "triM. — A lightweight, native video editor for Linux built with Rust. Multi-track editing, stereo audio mixing and FFmpeg export.",
    skip: "Skip to content", nav_features: "Features", nav_workflow: "Philosophy", nav_shortcuts: "Shortcuts", nav_download: "Download",
    hero_kicker: "FOR LINUX. ON LINUX.", hero_line1: "Make the cut.", hero_line2: "Stay in", hero_line3: "the flow.",
    hero_description: "triM. is a native, open-source video editor made for Linux. Bring picture, sound and rhythm together on one timeline. No unnecessary layers in between.",
    hero_download: "Get triM.", hero_explore: "Explore the editor", hero_note: "Open source · Rust · No Electron.",
    preview_heading: "INSIDE THE EDITOR", preview_edit: "Editing", preview_start: "Start", preview_caption: "Actual app screenshots / triM. v0.4.0",
    stack_caption: "UNDER THE HOOD", features_index: "THE TOOLS", features_title: "Every frame, <em>your call.</em>",
    features_intro: "The essentials for making your cut. Thoughtful tools that work together without getting in your way.",
    feature_1_title: "Cut. Move. Carry on.", feature_1_desc: "A layered timeline with magnetic snapping, razor splits, and ripple editing. Built to keep up with your ideas.",
    feature_2_title: "Sound is half the story.", feature_2_desc: "Multi-track stereo mixing, level monitoring, mute/solo controls and detachable audio.",
    feature_3_title: "Make motion your own.", feature_3_desc: "Animate position, scale, opacity and color adjustments with keyframes.",
    feature_4_title: "Take your edit further.", feature_4_desc: "Export through FFmpeg and finish your project in 1080p, 4K, WebM and more.",
    feature_note: "Feature diagrams are illustrative; the editor screenshots above are real.",
    workflow_index: "THE APPROACH", workflow_title: "Less in between.<br><em>More story to tell.</em>",
    workflow_text: "triM. isn't a web app wrapped in a desktop window. It's a native Linux editor written in Rust. Your media, project files and creative process stay in your hands.",
    point_1: "Native desktop", point_2: "Open-source code", point_3: "FFmpeg foundation",
    shortcuts_index: "SHORTCUTS", shortcuts_title: "Keep the ideas coming.<br><em>Keep your hands moving.</em>",
    shortcuts_description: "Less reaching for the mouse, more time in the flow. A handful of shortcuts you'll use every day.", shortcuts_link: "See all shortcuts",
    key_play: "Play / pause", key_cut: "Split at playhead", key_trim: "Trim head / tail", key_shuttle: "Reverse / pause / forward", key_undo: "Undo",
    install_index: "GET STARTED", install_title: "Ready to<br><em>make the cut?</em>",
    install_description: "triM. is free and open source. Download a release or build it from source. It's your call.",
    install_release: "Go to GitHub releases", terminal_title: "Build from source",
    step_deps: "Install dependencies", step_build: "Clone and build", copy: "Copy", copied: "Copied!", copy_failed: "Could not copy",
    terminal_hint: "Rust / Cargo must be installed. Additional packages may be needed for your distribution.",
    footer_tagline: "Every cut has its rhythm.", footer_releases: "Releases ↗", footer_credit: "Built with care for Linux.", back_top: "Back to top ↑",
    menu_open: "Open menu", menu_close: "Close menu"
  };
  const translations = { tr, en };
  const richKeys = new Set(["features_title", "workflow_title", "shortcuts_title", "install_title"]);
  let current = "tr";
  try { const saved = localStorage.getItem("trim_language"); if (saved === "en" || saved === "tr") current = saved; } catch (_) {}

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
      if (richKeys.has(key)) element.innerHTML = dict[key]; // Static trusted copy only.
      else element.textContent = dict[key];
    });
    document.querySelectorAll("[data-lang]").forEach(button => {
      const active = button.dataset.lang === lang;
      button.classList.toggle("is-active", active);
      button.setAttribute("aria-pressed", String(active));
    });
    document.querySelectorAll(".preview-image").forEach(image => {
      if (image.dataset.image === "edit") image.alt = lang === "en" ? "Actual triM. editor, timeline and preview" : "triM. gerçek kurgu ekranı, zaman çizgisi ve önizleme";
      else image.alt = lang === "en" ? "triM. empty project startup screen" : "triM. boş proje açılış ekranı";
    });
    const menu = document.getElementById("menu-toggle");
    if (menu) menu.setAttribute("aria-label", dict[menu.getAttribute("aria-expanded") === "true" ? "menu_close" : "menu_open"]);
    try { localStorage.setItem("trim_language", lang); } catch (_) {}
  }

  document.querySelectorAll("[data-lang]").forEach(button => button.addEventListener("click", () => setLanguage(button.dataset.lang)));
  setLanguage(current);

  document.querySelectorAll("[data-preview]").forEach(button => {
    button.addEventListener("click", () => {
      const name = button.dataset.preview;
      document.querySelectorAll("[data-preview]").forEach(tab => {
        const active = tab === button;
        tab.classList.toggle("is-active", active);
        tab.setAttribute("aria-pressed", String(active));
      });
      document.querySelectorAll("[data-image]").forEach(img => img.classList.toggle("is-active", img.dataset.image === name));
    });
  });

  const distroCommands = {
    debian: "sudo apt update\nsudo apt install build-essential pkg-config libasound2-dev libx11-dev libgl1-mesa-dev ffmpeg",
    fedora: "sudo dnf install gcc pkgconf-pkg-config alsa-lib-devel libX11-devel mesa-libGL-devel ffmpeg",
    arch: "sudo pacman -S base-devel alsa-lib libx11 mesa ffmpeg"
  };
  const deps = document.getElementById("dependencies-command");
  document.querySelectorAll("[data-distro]").forEach(button => button.addEventListener("click", () => {
    if (!distroCommands[button.dataset.distro] || !deps) return;
    deps.textContent = distroCommands[button.dataset.distro];
    document.querySelectorAll("[data-distro]").forEach(tab => {
      const active = tab === button;
      tab.classList.toggle("is-active", active);
      tab.setAttribute("aria-pressed", String(active));
    });
  }));

  document.querySelectorAll("[data-copy]").forEach(button => button.addEventListener("click", async () => {
    const target = document.getElementById(button.dataset.copy);
    if (!target) return;
    try { await navigator.clipboard.writeText(target.textContent.trim()); button.textContent = translations[current].copied; }
    catch (_) { button.textContent = translations[current].copy_failed; }
    button.disabled = true;
    setTimeout(() => { button.disabled = false; button.textContent = translations[current].copy; }, 1600);
  }));

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
  const header = document.getElementById("site-header");
  const updateHeader = () => header?.classList.toggle("is-scrolled", window.scrollY > 12);
  window.addEventListener("scroll", updateHeader, { passive: true });
  updateHeader();
});
