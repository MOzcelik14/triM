//! Internationalization (i18n) and multi-language dictionary for triM.

/// Supported user interface languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    #[default]
    Turkish,
    English,
}

impl Language {
    pub const ALL: [Language; 2] = [Language::Turkish, Language::English];

    /// Parse language from ISO code or string ("tr", "en").
    pub fn from_code(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "en" | "en_us" | "en_gb" | "english" => Self::English,
            _ => Self::Turkish,
        }
    }

    /// Return two-letter ISO language code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Turkish => "tr",
            Self::English => "en",
        }
    }

    /// Display label for language dropdowns.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Turkish => "Türkçe (TR)",
            Self::English => "English (EN)",
        }
    }

    /// Translate a localized UI key into this language.
    pub fn t<'a>(&self, key: &'a str) -> &'a str {
        translate(*self, key)
    }
}

/// Translate key into given language, with graceful fallback.
pub fn tr(lang: Language, key: &str) -> &str {
    translate(lang, key)
}

fn translate(lang: Language, key: &str) -> &str {
    match (lang, key) {
        // --- Top Menu Bar ---
        (Language::Turkish, "menu_file") => "Dosya",
        (Language::English, "menu_file") => "File",
        (Language::Turkish, "menu_edit") => "Düzen",
        (Language::English, "menu_edit") => "Edit",
        (Language::Turkish, "menu_sequence") => "Sekans",
        (Language::English, "menu_sequence") => "Sequence",
        (Language::Turkish, "menu_tracks") => "Kanallar",
        (Language::English, "menu_tracks") => "Tracks",
        (Language::Turkish, "menu_window") => "Pencere",
        (Language::English, "menu_window") => "Window",
        (Language::Turkish, "menu_help") => "Yardım",
        (Language::English, "menu_help") => "Help",

        // File Menu
        (Language::Turkish, "menu_welcome") => "Başlangıç Ekranı...",
        (Language::English, "menu_welcome") => "Project Hub / Welcome...",
        (Language::Turkish, "menu_new_project") => "Yeni Boş Proje (Ctrl+N)",
        (Language::English, "menu_new_project") => "New Empty Project (Ctrl+N)",
        (Language::Turkish, "menu_open_project") => "Proje Aç... (Ctrl+O)",
        (Language::English, "menu_open_project") => "Open Project... (Ctrl+O)",
        (Language::Turkish, "menu_save") => "Kaydet (Ctrl+S)",
        (Language::English, "menu_save") => "Save (Ctrl+S)",
        (Language::Turkish, "menu_save_as") => "Farklı Kaydet...",
        (Language::English, "menu_save_as") => "Save As...",
        (Language::Turkish, "menu_bundle") => "Projeyi Paketle / Arşivle... (Taşınabilir)",
        (Language::English, "menu_bundle") => "Bundle / Archive Project... (Portable)",
        (Language::Turkish, "menu_import_media") => "Medya İçe Aktar... (Ctrl+I)",
        (Language::English, "menu_import_media") => "Import Media... (Ctrl+I)",
        (Language::Turkish, "menu_relink") => "Çevrimdışı Medyaları Yeniden Bağla (Relink)...",
        (Language::English, "menu_relink") => "Relink Offline Media...",
        (Language::Turkish, "menu_import_interchange") => "Projeyi İçe Aktar (FCPXML / EDL)...",
        (Language::English, "menu_import_interchange") => "Import Project (FCPXML / EDL)...",
        (Language::Turkish, "menu_import_subtitles") => "Altyazı Dosyası İçe Aktar (.srt, .vtt)...",
        (Language::English, "menu_import_subtitles") => "Import Subtitle File (.srt, .vtt)...",
        (Language::Turkish, "menu_export_subtitles") => "Altyazıları Dışa Aktar (.srt)...",
        (Language::English, "menu_export_subtitles") => "Export Subtitles (.srt)...",
        (Language::Turkish, "menu_export_edl") => "Projeyi EDL Olarak Dışa Aktar (.edl)...",
        (Language::English, "menu_export_edl") => "Export Project as EDL (.edl)...",
        (Language::Turkish, "menu_export_fcpxml") => "Projeyi FCPXML Olarak Dışa Aktar (Premiere/DaVinci)...",
        (Language::English, "menu_export_fcpxml") => "Export Project as FCPXML (Premiere/DaVinci)...",
        (Language::Turkish, "menu_export_studio") => "Dışa Aktarma Stüdyosu... (Ctrl+M)",
        (Language::English, "menu_export_studio") => "Export Studio... (Ctrl+M)",
        (Language::Turkish, "menu_render_queue") => "📋 Render Kuyruğu... (Ctrl+Shift+E)",
        (Language::English, "menu_render_queue") => "📋 Render Queue... (Ctrl+Shift+E)",
        (Language::Turkish, "menu_exit") => "Çıkış (Ctrl+Q)",
        (Language::English, "menu_exit") => "Exit (Ctrl+Q)",

        // Edit Menu
        (Language::Turkish, "menu_undo") => "Geri Al (Ctrl+Z)",
        (Language::English, "menu_undo") => "Undo (Ctrl+Z)",
        (Language::Turkish, "menu_redo") => "Yinele (Ctrl+Y)",
        (Language::English, "menu_redo") => "Redo (Ctrl+Y)",
        (Language::Turkish, "menu_cut") => "Kes (Ctrl+X)",
        (Language::English, "menu_cut") => "Cut (Ctrl+X)",
        (Language::Turkish, "menu_copy") => "Kopyala (Ctrl+C)",
        (Language::English, "menu_copy") => "Copy (Ctrl+C)",
        (Language::Turkish, "menu_paste") => "Yapıştır (Ctrl+V)",
        (Language::English, "menu_paste") => "Paste (Ctrl+V)",
        (Language::Turkish, "menu_delete") => "Seçilenleri Sil (Delete)",
        (Language::English, "menu_delete") => "Delete Selected (Delete)",
        (Language::Turkish, "menu_ripple_delete") => "Dalgalı Sil (Shift+Delete)",
        (Language::English, "menu_ripple_delete") => "Ripple Delete (Shift+Delete)",
        (Language::Turkish, "menu_select_all") => "Tüm Klipleri Seç (Ctrl+A)",
        (Language::English, "menu_select_all") => "Select All Clips (Ctrl+A)",
        (Language::Turkish, "menu_deselect") => "Seçimi Kaldır (Ctrl+D)",
        (Language::English, "menu_deselect") => "Deselect All (Ctrl+D)",
        (Language::Turkish, "menu_link_clips") => "Klipleri Bağla / Ayır (Ctrl+L)",
        (Language::English, "menu_link_clips") => "Link / Unlink Clips (Ctrl+L)",
        (Language::Turkish, "menu_silence_cut") => "Sessizlikleri Otomatik Kes (Jump-Cut)...",
        (Language::English, "menu_silence_cut") => "Auto Cut Silence (Jump-Cut)...",
        (Language::Turkish, "menu_transcribe") => "🎙️ Lokal Whisper ile Otomatik Deşifre Et...",
        (Language::English, "menu_transcribe") => "🎙️ Auto Transcribe with Local Whisper...",
        (Language::Turkish, "menu_screen_record") => "📹 Ekran ve Web Kamerası Kaydet...",
        (Language::English, "menu_screen_record") => "📹 Record Screen & Webcam...",
        (Language::Turkish, "menu_shortcut_profile") => "Klavye Kısayol Profili",
        (Language::English, "menu_shortcut_profile") => "Keyboard Shortcut Profile",
        (Language::Turkish, "menu_preferences") => "⚙️ Tercihler / Ayarlar... (Ctrl+,)",
        (Language::English, "menu_preferences") => "⚙️ Preferences / Settings... (Ctrl+,)",

        // Sequence Menu
        (Language::Turkish, "menu_split_playhead") => "Oynatma Çizgisinde Kes (S / Ctrl+K)",
        (Language::English, "menu_split_playhead") => "Split at Playhead (S / Ctrl+K)",
        (Language::Turkish, "menu_smart_cut") => "🎬 Seçili Klipte Sahneleri Algıla ve Böl (Smart Cut)",
        (Language::English, "menu_smart_cut") => "🎬 Smart Scene Cut Selected Clip",
        (Language::Turkish, "menu_marker") => "İşaretçi (Marker) Ekle / Kaldır (M)",
        (Language::English, "menu_marker") => "Add / Remove Marker (M)",
        (Language::Turkish, "menu_aspect_ratio") => "En-Boy Oranı / Format",
        (Language::English, "menu_aspect_ratio") => "Aspect Ratio / Format",
        (Language::Turkish, "menu_safe_zones") => "Güvenli Alan Rehber Çizgileri",
        (Language::English, "menu_safe_zones") => "Safe Zone Guides",
        (Language::Turkish, "menu_proxies") => "⚡ Akıllı Proxy Önizleme (720p H.264)",
        (Language::English, "menu_proxies") => "⚡ Smart Proxy Preview (720p H.264)",
        (Language::Turkish, "menu_go_home") => "Başa Git (Home)",
        (Language::English, "menu_go_home") => "Go to Start (Home)",
        (Language::Turkish, "menu_go_end") => "Sona Git (End)",
        (Language::English, "menu_go_end") => "Go to End (End)",

        // Tracks Menu
        (Language::Turkish, "menu_add_video_track") => "Video Kanalı Ekle (+ Video)",
        (Language::English, "menu_add_video_track") => "Add Video Track (+ Video)",
        (Language::Turkish, "menu_add_audio_track") => "Ses Kanalı Ekle (+ Ses)",
        (Language::English, "menu_add_audio_track") => "Add Audio Track (+ Audio)",
        (Language::Turkish, "menu_reset_track_heights") => "Tüm Kanal Yüksekliklerini Sıfırla (56px)",
        (Language::English, "menu_reset_track_heights") => "Reset All Track Heights (56px)",

        // Window Menu
        (Language::Turkish, "menu_source_monitor") => "Kaynak Monitörü (İkili Görünüm)",
        (Language::English, "menu_source_monitor") => "Source Monitor (Dual View)",
        (Language::Turkish, "menu_audio_mixer") => "Çok Kanallı Ses Mikseri",
        (Language::English, "menu_audio_mixer") => "Multi-Channel Audio Mixer",
        (Language::Turkish, "menu_title_editor") => "Metin / Başlık Editörü",
        (Language::English, "menu_title_editor") => "Text / Title Editor",

        // Help Menu
        (Language::Turkish, "menu_shortcuts_f1") => "Klavye Kısayolları (F1)",
        (Language::English, "menu_shortcuts_f1") => "Keyboard Shortcuts (F1)",
        (Language::Turkish, "menu_about") => "triM. Hakkında",
        (Language::English, "menu_about") => "About triM.",

        // Header Action Buttons
        (Language::Turkish, "btn_export") => "Dışa Aktar",
        (Language::English, "btn_export") => "Export",
        (Language::Turkish, "btn_mixer") => "Mikser",
        (Language::English, "btn_mixer") => "Mixer",
        (Language::Turkish, "btn_title") => "Başlık",
        (Language::English, "btn_title") => "Title",
        (Language::Turkish, "btn_settings") => "Ayarlar",
        (Language::English, "btn_settings") => "Settings",

        // Toolbar
        (Language::Turkish, "tool_selection") => "Seçim Aracı (V)",
        (Language::English, "tool_selection") => "Selection Tool (V)",
        (Language::Turkish, "tool_track_select") => "İleri Kanal Seçim Aracı (A)",
        (Language::English, "tool_track_select") => "Track Select Forward (A)",
        (Language::Turkish, "tool_ripple") => "Dalgalanma Düzenleme Aracı (B)",
        (Language::English, "tool_ripple") => "Ripple Edit Tool (B)",
        (Language::Turkish, "tool_razor") => "Jilet Aracı (C)",
        (Language::English, "tool_razor") => "Razor Tool (C)",
        (Language::Turkish, "tool_slip") => "Kaydırma Aracı (Y)",
        (Language::English, "tool_slip") => "Slip Tool (Y)",
        (Language::Turkish, "tool_type") => "Metin Yazım Aracı (T)",
        (Language::English, "tool_type") => "Type Tool (T)",
        (Language::Turkish, "tool_hand") => "El / Kaydırma Aracı (H)",
        (Language::English, "tool_hand") => "Hand Tool (H)",
        (Language::Turkish, "tool_zoom") => "Yakınlaştırma Aracı (Z)",
        (Language::English, "tool_zoom") => "Zoom Tool (Z)",

        // Media Bin Tabs & Actions
        (Language::Turkish, "tab_project") => "Proje",
        (Language::English, "tab_project") => "Project",
        (Language::Turkish, "tab_effects") => "Efektler",
        (Language::English, "tab_effects") => "Effects",
        (Language::Turkish, "tab_titles") => "Başlıklar",
        (Language::English, "tab_titles") => "Titles",
        (Language::Turkish, "tab_queue") => "Kuyruk",
        (Language::English, "tab_queue") => "Queue",
        (Language::Turkish, "empty_bin_drop") => "Medyalarınızı Buraya Sürükleyin",
        (Language::English, "empty_bin_drop") => "Drop Your Media Files Here",
        (Language::Turkish, "empty_bin_desc") => "Video, ses veya görsel dosyalarınızı klasörden sürükleyin ya da alttaki butondan seçin.",
        (Language::English, "empty_bin_desc") => "Drag video, audio, or image files from your file manager, or browse with the button below.",
        (Language::Turkish, "btn_import_media") => "Medya İçe Aktar...",
        (Language::English, "btn_import_media") => "Import Media...",
        (Language::Turkish, "filter_all") => "Tümü",
        (Language::English, "filter_all") => "All",
        (Language::Turkish, "filter_videos") => "Videolar",
        (Language::English, "filter_videos") => "Videos",
        (Language::Turkish, "filter_audio") => "Sesler",
        (Language::English, "filter_audio") => "Audio",
        (Language::Turkish, "filter_images") => "Görseller",
        (Language::English, "filter_images") => "Images",

        // Inspector
        (Language::Turkish, "inspector_title") => "Klip Denetçisi",
        (Language::English, "inspector_title") => "Clip Inspector",
        (Language::Turkish, "inspector_no_selection") => "Seçili Klip Yok",
        (Language::English, "inspector_no_selection") => "No Clip Selected",
        (Language::Turkish, "inspector_transform") => "Dönüşüm (Transform)",
        (Language::English, "inspector_transform") => "Transform",
        (Language::Turkish, "inspector_opacity") => "Opaklık & Karışım",
        (Language::English, "inspector_opacity") => "Opacity & Blending",
        (Language::Turkish, "inspector_color") => "Renk Düzenleme (Color Wheels)",
        (Language::English, "inspector_color") => "Color Wheels & Grading",
        (Language::Turkish, "inspector_audio") => "Ses Kazancı (Gain)",
        (Language::English, "inspector_audio") => "Audio Gain",
        (Language::Turkish, "inspector_speed") => "Oynatma Hızı",
        (Language::English, "inspector_speed") => "Playback Speed",

        // Status & General
        (Language::Turkish, "status_ready") => "Hazır",
        (Language::English, "status_ready") => "Ready",
        (Language::Turkish, "dialog_save") => "Kaydet",
        (Language::English, "dialog_save") => "Save",
        (Language::Turkish, "dialog_cancel") => "İptal",
        (Language::English, "dialog_cancel") => "Cancel",
        (Language::Turkish, "dialog_apply") => "Uygula",
        (Language::English, "dialog_apply") => "Apply",
        (Language::Turkish, "dialog_close") => "Kapat",
        (Language::English, "dialog_close") => "Close",

        // Fallback: return key verbatim if not found
        (_, other) => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i18n_translation_keys() {
        assert_eq!(Language::Turkish.t("menu_file"), "Dosya");
        assert_eq!(Language::English.t("menu_file"), "File");
        assert_eq!(Language::Turkish.t("menu_edit"), "Düzen");
        assert_eq!(Language::English.t("menu_edit"), "Edit");
        assert_eq!(Language::Turkish.t("btn_settings"), "Ayarlar");
        assert_eq!(Language::English.t("btn_settings"), "Settings");
    }

    #[test]
    fn test_language_from_code() {
        assert_eq!(Language::from_code("en"), Language::English);
        assert_eq!(Language::from_code("EN_US"), Language::English);
        assert_eq!(Language::from_code("tr"), Language::Turkish);
        assert_eq!(Language::from_code("unknown"), Language::Turkish);
    }
}
