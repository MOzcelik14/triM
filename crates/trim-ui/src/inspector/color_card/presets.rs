use egui::{Color32, Rounding, Stroke, Ui};
use trim_core::color::{BrightnessContrastPreset, LutPreset};

pub(super) fn draw_look_preset_thumbnail(ui: &mut Ui, rect: egui::Rect, preset: LutPreset) {
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        Rounding::same(4.0),
        Color32::from_rgb(0x0b, 0x0d, 0x12),
    );

    // Render 4 horizontal bands representing sky/cool, skin/warm, foliage/mid, and deep shadow
    // passed directly through `preset.apply_rgb` for an authentic live look preview.
    let sample_colors: [[f32; 3]; 4] = [
        [0.24, 0.56, 0.86], // Sky / Cyan-Blue highlight
        [0.88, 0.62, 0.44], // Skin / Warm mid-high
        [0.36, 0.64, 0.38], // Foliage / Green midtone
        [0.18, 0.20, 0.28], // Shadow
    ];
    let band_w = rect.width() / 4.0;
    for (idx, base) in sample_colors.iter().enumerate() {
        let [r, g, b] = preset.apply_rgb(base[0], base[1], base[2]);
        let col = Color32::from_rgb(
            (r * 255.0).round() as u8,
            (g * 255.0).round() as u8,
            (b * 255.0).round() as u8,
        );
        let x0 = rect.left() + idx as f32 * band_w;
        let x1 = if idx == 3 { rect.right() } else { x0 + band_w };
        let sub =
            egui::Rect::from_min_max(egui::pos2(x0, rect.top()), egui::pos2(x1, rect.bottom()));
        let rounding = match idx {
            0 => Rounding {
                nw: 4.0,
                sw: 4.0,
                ne: 0.0,
                se: 0.0,
            },
            3 => Rounding {
                nw: 0.0,
                sw: 0.0,
                ne: 4.0,
                se: 4.0,
            },
            _ => Rounding::ZERO,
        };
        painter.rect_filled(sub, rounding, col);
    }
    painter.rect_stroke(
        rect,
        Rounding::same(4.0),
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 28)),
    );
}

pub(super) fn draw_bc_preset_thumbnail(
    ui: &mut Ui,
    rect: egui::Rect,
    preset: BrightnessContrastPreset,
) {
    let painter = ui.painter();
    let (br, ct, gm) = preset.params();
    let steps = 5usize;
    let step_w = rect.width() / steps as f32;
    let inv_g = (1.0 / gm.max(0.1)) as f32;

    for i in 0..steps {
        let t = (i as f32 + 0.5) / steps as f32;
        let mut v = (ct as f32) * (t - 0.5) + 0.5 + (br as f32);
        v = v.clamp(0.0, 1.0).powf(inv_g).clamp(0.0, 1.0);
        let u = (v * 255.0).round() as u8;
        let col = Color32::from_rgb(u, u, (u as u16 * 105 / 100).min(255) as u8);
        let x0 = rect.left() + i as f32 * step_w;
        let x1 = if i + 1 == steps {
            rect.right()
        } else {
            x0 + step_w
        };
        let sub =
            egui::Rect::from_min_max(egui::pos2(x0, rect.top()), egui::pos2(x1, rect.bottom()));
        painter.rect_filled(sub, Rounding::ZERO, col);
    }
    painter.rect_stroke(
        rect,
        Rounding::same(3.0),
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 24)),
    );
}
