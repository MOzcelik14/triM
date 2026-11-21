//! triM. Dark Theme and Precision Amber styling.

use egui::{Color32, Rounding, Stroke, Visuals};

pub const COLOR_BG: Color32 = Color32::from_rgb(0x15, 0x15, 0x18);
pub const COLOR_PANEL: Color32 = Color32::from_rgb(0x1c, 0x1c, 0x20);
pub const COLOR_WIDGET: Color32 = Color32::from_rgb(0x24, 0x24, 0x2b);
pub const COLOR_WIDGET_HOVER: Color32 = Color32::from_rgb(0x2f, 0x2f, 0x38);
pub const COLOR_BORDER: Color32 = Color32::from_rgb(0x28, 0x28, 0x2e);
pub const COLOR_TEXT: Color32 = Color32::from_rgb(0xe4, 0xe4, 0xea);
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(0x8e, 0x8e, 0xa0);
pub const COLOR_AMBER: Color32 = Color32::from_rgb(0xe0, 0x7a, 0x38);
pub const COLOR_AMBER_HOVER: Color32 = Color32::from_rgb(0xe8, 0x8a, 0x4c);
pub const COLOR_AMBER_ACTIVE: Color32 = Color32::from_rgb(0xc9, 0x67, 0x28);

// Timeline clip colors
pub const COLOR_CLIP_VIDEO: Color32 = Color32::from_rgb(0x2a, 0x55, 0x80);
pub const COLOR_CLIP_VIDEO_SELECTED: Color32 = Color32::from_rgb(0x32, 0x69, 0xa0);
pub const COLOR_CLIP_AUDIO: Color32 = Color32::from_rgb(0x24, 0x64, 0x46);
pub const COLOR_CLIP_AUDIO_SELECTED: Color32 = Color32::from_rgb(0x2d, 0x7d, 0x58);

/// Available studio theme presets: (id, label_tr, label_en)
pub const STUDIO_THEMES: &[(&str, &str, &str)] = &[
    (
        "indigo",
        "Varsayılan İndigo & Amber",
        "Default Indigo & Amber",
    ),
    (
        "noir_lime",
        "Sinema Lime (Noir Studio)",
        "Cinema Lime (Noir Studio)",
    ),
    (
        "monochrome",
        "Saf Monokrom (Karanlık Oda)",
        "Pure Monochrome (Darkroom)",
    ),
    (
        "midnight_blue",
        "Gece Mavisi (Deep Broadcast)",
        "Midnight Blue (Deep Broadcast)",
    ),
    (
        "warm_tungsten",
        "Sıcak Tungsten (Analog Lab)",
        "Warm Tungsten (Analog Lab)",
    ),
];

pub fn apply_studio_theme(ctx: &egui::Context, theme_id: &str) {
    let mut visuals = Visuals::dark();

    let (panel_bg, win_bg, extreme_bg, widget_bg, widget_hov, border_col, active_bg, sel_bg) =
        match theme_id {
            "noir_lime" => (
                Color32::from_rgb(0x14, 0x16, 0x14),
                Color32::from_rgb(0x0e, 0x10, 0x0e),
                Color32::from_rgb(0x0a, 0x0c, 0x0a),
                Color32::from_rgb(0x1e, 0x22, 0x1e),
                Color32::from_rgb(0x28, 0x30, 0x28),
                Color32::from_rgb(0x2c, 0x36, 0x2c),
                Color32::from_rgb(0x52, 0x9e, 0x28),
                Color32::from_rgb(0x42, 0x82, 0x20),
            ),
            "monochrome" => (
                Color32::from_rgb(0x18, 0x18, 0x18),
                Color32::from_rgb(0x11, 0x11, 0x11),
                Color32::from_rgb(0x0b, 0x0b, 0x0b),
                Color32::from_rgb(0x22, 0x22, 0x22),
                Color32::from_rgb(0x2e, 0x2e, 0x2e),
                Color32::from_rgb(0x2b, 0x2b, 0x2b),
                Color32::from_rgb(0x60, 0x60, 0x68),
                Color32::from_rgb(0x4a, 0x4a, 0x52),
            ),
            "midnight_blue" => (
                Color32::from_rgb(0x13, 0x18, 0x24),
                Color32::from_rgb(0x0d, 0x11, 0x1b),
                Color32::from_rgb(0x09, 0x0c, 0x14),
                Color32::from_rgb(0x1c, 0x23, 0x33),
                Color32::from_rgb(0x26, 0x30, 0x45),
                Color32::from_rgb(0x24, 0x2e, 0x42),
                Color32::from_rgb(0x25, 0x63, 0xeb),
                Color32::from_rgb(0x1d, 0x4e, 0xd8),
            ),
            "warm_tungsten" => (
                Color32::from_rgb(0x1f, 0x1b, 0x18),
                Color32::from_rgb(0x17, 0x14, 0x12),
                Color32::from_rgb(0x11, 0x0e, 0x0c),
                Color32::from_rgb(0x2b, 0x25, 0x20),
                Color32::from_rgb(0x38, 0x30, 0x29),
                Color32::from_rgb(0x33, 0x2b, 0x25),
                Color32::from_rgb(0xd9, 0x77, 0x24),
                Color32::from_rgb(0xb4, 0x5d, 0x18),
            ),
            _ => (
                COLOR_PANEL,
                COLOR_BG,
                Color32::from_rgb(0x12, 0x12, 0x15),
                COLOR_WIDGET,
                COLOR_WIDGET_HOVER,
                COLOR_BORDER,
                Color32::from_rgb(0x32, 0x6e, 0xbf),
                Color32::from_rgb(0x28, 0x5a, 0x9e),
            ),
        };

    visuals.panel_fill = panel_bg;
    visuals.window_fill = win_bg;
    visuals.extreme_bg_color = extreme_bg;
    visuals.faint_bg_color = win_bg;

    // Normal widgets
    visuals.widgets.noninteractive.bg_fill = panel_bg;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, border_col);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT);
    visuals.widgets.noninteractive.rounding = Rounding::same(3.0);

    visuals.widgets.inactive.bg_fill = widget_bg;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, border_col);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT);
    visuals.widgets.inactive.rounding = Rounding::same(3.0);

    // Hover
    visuals.widgets.hovered.bg_fill = widget_hov;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0x42, 0x42, 0x4e));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.hovered.rounding = Rounding::same(3.0);

    // Active
    visuals.widgets.active.bg_fill = active_bg;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(3.0);

    // Selection
    visuals.selection.bg_fill = sel_bg;
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);

    ctx.set_visuals(visuals);
}

pub fn setup_theme(ctx: &egui::Context) {
    apply_studio_theme(ctx, "indigo");

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (egui::TextStyle::Heading, egui::FontId::proportional(19.0)),
        (
            egui::TextStyle::Name("Subheading".into()),
            egui::FontId::proportional(16.0),
        ),
        (egui::TextStyle::Body, egui::FontId::proportional(13.5)),
        (egui::TextStyle::Button, egui::FontId::proportional(13.5)),
        (egui::TextStyle::Monospace, egui::FontId::monospace(12.5)),
        (egui::TextStyle::Small, egui::FontId::proportional(11.0)),
    ]
    .into();
    style.spacing.button_padding = egui::vec2(7.0, 4.0);
    style.spacing.item_spacing = egui::vec2(7.0, 5.0);
    style.spacing.interact_size = egui::vec2(40.0, 22.0);
    ctx.set_style(style);

    // Load system symbol/unicode fonts if available so that glyphs never render as empty squares
    let mut fonts = egui::FontDefinitions::default();
    let sys_fonts = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansSymbols-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf",
    ];
    for (i, path) in sys_fonts.iter().enumerate() {
        if let Ok(data) = std::fs::read(path) {
            let font_name = format!("sys_font_{}", i);
            fonts
                .font_data
                .insert(font_name.clone(), egui::FontData::from_owned(data));
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push(font_name.clone());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push(font_name);
        }
    }

    register_popular_system_fonts(&mut fonts);
    if let Ok(mut guard) = global_font_defs().lock() {
        *guard = fonts.clone();
    }
    ctx.set_fonts(fonts);
}

static LOADED_NAMED_FONTS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashSet<String>>,
> = std::sync::OnceLock::new();

static GLOBAL_FONT_DEFS: std::sync::OnceLock<std::sync::Mutex<egui::FontDefinitions>> =
    std::sync::OnceLock::new();

fn loaded_named_fonts() -> &'static std::sync::Mutex<std::collections::HashSet<String>> {
    LOADED_NAMED_FONTS.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()))
}

fn global_font_defs() -> &'static std::sync::Mutex<egui::FontDefinitions> {
    GLOBAL_FONT_DEFS.get_or_init(|| std::sync::Mutex::new(egui::FontDefinitions::default()))
}

/// Dynamically registers a custom `.ttf` or `.otf` font file into egui so live preview matches FFmpeg export.
pub fn register_custom_font_file(ctx: &egui::Context, path: &std::path::Path) {
    let key = path.to_string_lossy().to_string();
    if let Ok(set) = loaded_named_fonts().lock() {
        if set.contains(&key) {
            return;
        }
    }
    let Ok(data) = std::fs::read(path) else {
        return;
    };
    if let Ok(mut fonts) = global_font_defs().lock() {
        let fallback_prop = fonts
            .families
            .get(&egui::FontFamily::Proportional)
            .cloned()
            .unwrap_or_default();
        let data_key = format!("custom_font_{key}");
        fonts
            .font_data
            .insert(data_key.clone(), egui::FontData::from_owned(data));
        let mut chain = vec![data_key];
        chain.extend(fallback_prop);
        fonts
            .families
            .insert(egui::FontFamily::Name(key.clone().into()), chain);
        ctx.set_fonts(fonts.clone());
    }
    if let Ok(mut set) = loaded_named_fonts().lock() {
        set.insert(key);
    }
}

/// Resolves a `TextOverlay::font_family` name to a registered `egui::FontFamily`,
/// falling back safely to `Monospace` or `Proportional` if the font is not installed locally.
pub fn resolve_egui_font_family(font_family: &str) -> egui::FontFamily {
    if font_family == "Monospace" {
        return egui::FontFamily::Monospace;
    }
    if let Ok(set) = loaded_named_fonts().lock() {
        if set.contains(font_family) {
            return egui::FontFamily::Name(font_family.to_string().into());
        }
    }
    egui::FontFamily::Proportional
}

fn collect_font_files(
    dir: &std::path::Path,
    depth: usize,
    index: &mut std::collections::HashMap<String, std::path::PathBuf>,
) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_font_files(&path, depth + 1, index);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_low = ext.to_ascii_lowercase();
            if ext_low == "ttf" || ext_low == "otf" {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let norm: String = stem
                        .to_ascii_lowercase()
                        .chars()
                        .filter(|c| c.is_ascii_alphanumeric())
                        .collect();
                    index.entry(norm).or_insert(path);
                }
            }
        }
    }
}

fn font_candidate_keys(family: &str) -> Vec<String> {
    let base: String = family
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let mut keys = vec![
        format!("{base}regular"),
        base.clone(),
        format!("{base}r"),
        format!("{base}book"),
        format!("{base}medium"),
    ];
    match family {
        "Times New Roman" => keys.extend(["times".into(), "timesnewromanpsmt".into()]),
        "Courier New" => keys.extend(["cour".into(), "couriernewpsmt".into()]),
        "Trebuchet MS" => keys.extend(["trebuc".into()]),
        "Serif" => keys.extend(["dejavuserif".into(), "liberationserifregular".into()]),
        _ => {}
    }
    keys
}

fn register_popular_system_fonts(fonts: &mut egui::FontDefinitions) {
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    #[cfg(target_os = "windows")]
    {
        dirs.push(std::path::PathBuf::from(r"C:\Windows\Fonts"));
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            dirs.push(
                std::path::PathBuf::from(local)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Fonts"),
            );
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        dirs.push(std::path::PathBuf::from("/usr/share/fonts"));
        dirs.push(std::path::PathBuf::from("/usr/local/share/fonts"));
        if let Ok(home) = std::env::var("HOME") {
            let h = std::path::PathBuf::from(home);
            dirs.push(h.join(".local/share/fonts"));
            dirs.push(h.join(".fonts"));
        }
    }

    let mut index = std::collections::HashMap::new();
    for dir in dirs {
        if dir.is_dir() {
            collect_font_files(&dir, 0, &mut index);
        }
    }
    if index.is_empty() {
        return;
    }

    let fallback_prop = fonts
        .families
        .get(&egui::FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    for &family in trim_core::clip::POPULAR_FONT_FAMILIES {
        if family == "Sans-Serif" || family == "Monospace" {
            continue;
        }
        let found = font_candidate_keys(family)
            .into_iter()
            .find_map(|k| index.get(&k).cloned());
        if let Some(path) = found {
            if let Ok(data) = std::fs::read(&path) {
                let data_key = format!("named_{family}");
                fonts
                    .font_data
                    .insert(data_key.clone(), egui::FontData::from_owned(data));
                let mut chain = vec![data_key];
                chain.extend(fallback_prop.iter().cloned());
                fonts
                    .families
                    .insert(egui::FontFamily::Name(family.into()), chain);
                if let Ok(mut set) = loaded_named_fonts().lock() {
                    set.insert(family.to_string());
                }
            }
        }
    }
}
