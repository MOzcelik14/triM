// triM — Lightweight Desktop Video Editor Documentation
// Theme toggle, language switch, and screenshot modal.

const I18N = {
  tr: {
    hero_tag: "Rust ile Geliştirilen Masaüstü Video Düzenleyici",
    hero_title: "Hafif, yerel ve doğrudan.",
    hero_desc: "triM, arayüzü egui ile çizilen, medya çözümleme ve kodlama için sistemdeki FFmpeg araçlarını kullanan, 13 MB boyutunda bir masaüstü video kurgu yazılımıdır. Tarayıcı motoru veya harici bulut bağımlılığı içermez.",
    btn_download: "Sürümleri İndir",
    btn_github: "GitHub Deposu",
    nav_features: "Özellikler",
    nav_gallery: "Ekranlar",
    nav_install: "Kurulum",
    nav_arch: "Mimari",
    nav_limits: "Sınırlamalar",
    sec_features: "Temel Özellikler",
    sec_features_sub: "İhtiyaç duyulan kurgu araçları, doğrudan yerel kodla.",
    feat_edit_title: "Kurgu ve Zaman Çizgisi",
    feat_color_title: "Renk ve Görsel Efektler",
    feat_audio_title: "Çok Kanallı Ses",
    feat_export_title: "Dışa Aktarma ve Standartlar",
    sec_gallery: "Canlı Uygulama Ekranları",
    sec_gallery_sub: "Uygulamanın v1.1.1 sürümünden doğrudan alınan ekran görüntüleri. Büyütmek için tıklayın.",
    sec_install: "Kurulum ve Derleme",
    sec_arch: "Mimari ve Tasarım Kararları",
    sec_limits: "Bilinen Sınırlamalar",
    theme_light: "Açık",
    theme_dark: "Koyu",
    caption_welcome: "Karşılama Ekranı (Yeni proje ve şablonlar)",
    caption_workspace: "Ana Kurgu Masası (Medya havuzu, monitör, zaman çizgisi)",
    caption_timeline: "Zaman Çizgisi Detayı (Çok kanallı ses/video, dalga formu)",
    caption_color_grading: "Renk Düzenleme (Lift/Gamma/Gain çarkları ve 3D LUT)",
    caption_audio_mixer: "Ses Mikseri (Kanal fader'ları ve VU metre)",
    caption_export_studio: "Dışa Aktarma Stüdyosu (GPU hızlandırma ve EBU R128)",
    caption_shortcuts: "Klavye Kısayolları Penceresi (F1)",
    caption_vertical_9_16: "Dikey Tuval (9:16 Shorts/Reels kılavuzları)"
  },
  en: {
    hero_tag: "Native Desktop Video Editor Built in Rust",
    hero_title: "Lightweight, native, and direct.",
    hero_desc: "triM is a 13 MB desktop non-linear video editor written in Rust with egui, using system FFmpeg for media decoding and encoding. No web runtime, no cloud account dependencies.",
    btn_download: "Download Releases",
    btn_github: "GitHub Repository",
    nav_features: "Features",
    nav_gallery: "Screenshots",
    nav_install: "Installation",
    nav_arch: "Architecture",
    nav_limits: "Limitations",
    sec_features: "Core Features",
    sec_features_sub: "Essential editing tools powered by native code.",
    feat_edit_title: "Editing & Timeline",
    feat_color_title: "Color & Visual Effects",
    feat_audio_title: "Multi-Track Audio",
    feat_export_title: "Export & Standards",
    sec_gallery: "In-App Screenshots",
    sec_gallery_sub: "Real screenshots taken directly from triM v1.1.1. Click to enlarge.",
    sec_install: "Installation & Build",
    sec_arch: "Architecture & Design Choices",
    sec_limits: "Known Limitations",
    theme_light: "Light",
    theme_dark: "Dark",
    caption_welcome: "Welcome Dialog (New project templates)",
    caption_workspace: "Main Workspace (Media pool, monitors, timeline)",
    caption_timeline: "Timeline Detail (Multi-track audio/video, waveforms)",
    caption_color_grading: "Color Grading (Lift/Gamma/Gain wheels & 3D LUT)",
    caption_audio_mixer: "Audio Mixer (Channel faders and VU meters)",
    caption_export_studio: "Export Studio (GPU acceleration & EBU R128)",
    caption_shortcuts: "Keyboard Shortcuts Window (F1)",
    caption_vertical_9_16: "Vertical Canvas (9:16 Shorts/Reels safe zones)"
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
  }
}

function applyLanguage(lang) {
  currentLang = lang;
  document.documentElement.lang = lang;
  localStorage.setItem("trim_lang", lang);
  
  const dict = I18N[lang] || I18N.tr;
  document.querySelectorAll("[data-i18n]").forEach(el => {
    const key = el.getAttribute("data-i18n");
    if (dict[key]) {
      el.textContent = dict[key];
    }
  });

  const langBtn = document.getElementById("lang-toggle");
  if (langBtn) {
    langBtn.textContent = lang.toUpperCase();
  }
}

document.addEventListener("DOMContentLoaded", () => {
  applyTheme(currentTheme);
  applyLanguage(currentLang);

  // Theme Toggle
  const themeBtn = document.getElementById("theme-toggle");
  if (themeBtn) {
    themeBtn.addEventListener("click", () => {
      applyTheme(currentTheme === "dark" ? "light" : "dark");
    });
  }

  // Language Toggle
  const langBtn = document.getElementById("lang-toggle");
  if (langBtn) {
    langBtn.addEventListener("click", () => {
      applyLanguage(currentLang === "tr" ? "en" : "tr");
    });
  }

  // Lightbox Modal
  const modal = document.getElementById("image-modal");
  const modalImg = document.getElementById("modal-img");
  const modalCaption = document.getElementById("modal-caption");
  const modalClose = document.getElementById("modal-close");

  document.querySelectorAll(".gallery-card").forEach(card => {
    card.addEventListener("click", () => {
      const img = card.querySelector("img");
      const caption = card.querySelector(".gallery-card-caption");
      if (modal && modalImg && img) {
        modalImg.src = img.src;
        modalImg.alt = img.alt || "";
        if (modalCaption && caption) {
          modalCaption.textContent = caption.textContent;
        }
        modal.classList.add("is-open");
      }
    });
  });

  if (modalClose) {
    modalClose.addEventListener("click", () => {
      modal.classList.remove("is-open");
    });
  }

  if (modal) {
    modal.addEventListener("click", (e) => {
      if (e.target === modal) {
        modal.classList.remove("is-open");
      }
    });
  }

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && modal && modal.classList.contains("is-open")) {
      modal.classList.remove("is-open");
    }
  });
});
