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
    ctx.set_fonts(fonts);
}
