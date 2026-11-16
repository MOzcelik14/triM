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

pub fn setup_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    visuals.panel_fill = COLOR_PANEL;
    visuals.window_fill = COLOR_BG;
    visuals.extreme_bg_color = Color32::from_rgb(0x12, 0x12, 0x15);
    visuals.faint_bg_color = COLOR_BG;

    // Normal widgets
    visuals.widgets.noninteractive.bg_fill = COLOR_PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT);
    visuals.widgets.noninteractive.rounding = Rounding::same(3.0);

    visuals.widgets.inactive.bg_fill = COLOR_WIDGET;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT);
    visuals.widgets.inactive.rounding = Rounding::same(3.0);

    // Hover
    visuals.widgets.hovered.bg_fill = COLOR_WIDGET_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0x42, 0x42, 0x4e));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.hovered.rounding = Rounding::same(3.0);

    // Active
    visuals.widgets.active.bg_fill = Color32::from_rgb(0x32, 0x6e, 0xbf);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0x4a, 0x8a, 0xe8));
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(3.0);

    // Selection - high contrast, pure white text over slate blue
    visuals.selection.bg_fill = Color32::from_rgb(0x28, 0x5a, 0x9e);
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);

    ctx.set_visuals(visuals);

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
    ctx.set_fonts(fonts);
}

static LOADED_NAMED_FONTS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashSet<String>>,
> = std::sync::OnceLock::new();

fn loaded_named_fonts() -> &'static std::sync::Mutex<std::collections::HashSet<String>> {
    LOADED_NAMED_FONTS.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()))
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
