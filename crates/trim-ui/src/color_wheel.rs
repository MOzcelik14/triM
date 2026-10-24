//! DaVinci Resolve style 3-Point Color Wheels (Lift, Gamma, Gain) egui widget.

use egui::{Color32, Sense, Stroke, Ui, Vec2};
use trim_core::clip::ColorWheelsConfig;

/// Draws an interactive circular color wheel for color grading (Lift, Gamma, or Gain).
///
/// `rgb`: `[R, G, B]` shifts.
/// `is_offset`: true for Lift (centered at 0.0, range -0.5..0.5), false for Gamma/Gain (centered at 1.0, range 0.2..2.0).
pub fn render_color_wheel(
    ui: &mut Ui,
    title: &str,
    rgb: &mut [f32; 3],
    lum: &mut f32,
    is_offset: bool,
) -> bool {
    let mut changed = false;

    ui.vertical(|ui| {
        ui.label(egui::RichText::new(title).size(11.0).strong());

        let radius = 34.0f32;
        let size = Vec2::splat(radius * 2.0 + 8.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::drag());

        let center = rect.center();
        let painter = ui.painter_at(rect);

        // Draw wheel background disc
        painter.circle_filled(center, radius, Color32::from_rgb(26, 28, 36));
        painter.circle_stroke(
            center,
            radius,
            Stroke::new(1.5_f32, Color32::from_rgb(55, 60, 75)),
        );

        // Subtle color hue accents at cardinal points
        let rim_r = radius - 3.0;
        painter.circle_filled(
            center + Vec2::new(0.0, -rim_r),
            3.0,
            Color32::from_rgb(240, 80, 80),
        ); // Red top
        painter.circle_filled(
            center + Vec2::new(rim_r * 0.866, rim_r * 0.5),
            3.0,
            Color32::from_rgb(80, 240, 80),
        ); // Green bottom right
        painter.circle_filled(
            center + Vec2::new(-rim_r * 0.866, rim_r * 0.5),
            3.0,
            Color32::from_rgb(80, 120, 255),
        ); // Blue bottom left

        // Crosshairs
        painter.line_segment(
            [
                center - Vec2::new(radius * 0.4, 0.0),
                center + Vec2::new(radius * 0.4, 0.0),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(45, 50, 65)),
        );
        painter.line_segment(
            [
                center - Vec2::new(0.0, radius * 0.4),
                center + Vec2::new(0.0, radius * 0.4),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(45, 50, 65)),
        );

        // Calculate normalized puck offset from RGB
        // Red is up (-Y), Green is bottom-right (+X, +Y), Blue is bottom-left (-X, +Y)
        let (base_val, max_delta) = if is_offset {
            (0.0f32, 0.5f32)
        } else {
            (1.0f32, 0.8f32)
        };

        let dr = (rgb[0] - base_val).clamp(-max_delta, max_delta) / max_delta;
        let dg = (rgb[1] - base_val).clamp(-max_delta, max_delta) / max_delta;
        let db = (rgb[2] - base_val).clamp(-max_delta, max_delta) / max_delta;

        // Project RGB deltas to 2D wheel coords
        let mut puck_offset = Vec2::new(
            (dg - db) * 0.866 * (radius - 5.0),
            (-dr + (dg + db) * 0.5) * (radius - 5.0),
        );

        if puck_offset.length() > (radius - 5.0) {
            puck_offset = puck_offset.normalized() * (radius - 5.0);
        }

        // Handle mouse dragging
        if response.dragged() {
            if let Some(mouse_pos) = response.interact_pointer_pos() {
                let mut delta = mouse_pos - center;
                if delta.length() > (radius - 5.0) {
                    delta = delta.normalized() * (radius - 5.0);
                }
                let norm_len = (delta.length() / (radius - 5.0)).clamp(0.0, 1.0);
                let angle = delta.y.atan2(delta.x); // -PI to PI

                // Convert polar wheel back to RGB deltas
                let nr = -angle.sin() * norm_len * max_delta;
                let ng = (angle + std::f32::consts::FRAC_PI_6).sin() * norm_len * max_delta;
                let nb = (-angle + std::f32::consts::FRAC_PI_6).sin() * norm_len * max_delta;

                rgb[0] = (base_val + nr + *lum).clamp(
                    if is_offset { -1.0 } else { 0.0 },
                    if is_offset { 1.0 } else { 3.0 },
                );
                rgb[1] = (base_val + ng + *lum).clamp(
                    if is_offset { -1.0 } else { 0.0 },
                    if is_offset { 1.0 } else { 3.0 },
                );
                rgb[2] = (base_val + nb + *lum).clamp(
                    if is_offset { -1.0 } else { 0.0 },
                    if is_offset { 1.0 } else { 3.0 },
                );
                changed = true;
            }
        }

        // Double click to reset
        if response.double_clicked() {
            rgb[0] = base_val;
            rgb[1] = base_val;
            rgb[2] = base_val;
            *lum = 0.0;
            changed = true;
        }

        // Draw puck
        let puck_pos = center + puck_offset;
        painter.circle_filled(puck_pos, 4.5, Color32::from_rgb(255, 255, 255));
        painter.circle_stroke(
            puck_pos,
            4.5,
            Stroke::new(1.0_f32, Color32::from_rgb(20, 20, 20)),
        );

        // Luminance slider below wheel
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Y:")
                    .size(10.0)
                    .color(Color32::from_rgb(140, 145, 160)),
            );
            if ui
                .add(egui::DragValue::new(lum).speed(0.01).range(if is_offset {
                    -0.5..=0.5
                } else {
                    -0.8..=0.8
                }))
                .changed()
            {
                changed = true;
            }
        });
    });

    changed
}

/// Renders the complete 3-Point Color Wheels panel for a clip.
pub fn render_color_wheels_panel(ui: &mut Ui, wheels: &mut ColorWheelsConfig) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;

        if render_color_wheel(
            ui,
            "Lift (Gölgeler)",
            &mut wheels.lift,
            &mut wheels.luminance[0],
            true,
        ) {
            changed = true;
        }
        if render_color_wheel(
            ui,
            "Gamma (Orta)",
            &mut wheels.gamma,
            &mut wheels.luminance[1],
            false,
        ) {
            changed = true;
        }
        if render_color_wheel(
            ui,
            "Gain (Açık)",
            &mut wheels.gain,
            &mut wheels.luminance[2],
            false,
        ) {
            changed = true;
        }
    });

    ui.add_space(4.0);
    if ui
        .button(egui::RichText::new("Çarkları Sıfırla").size(10.5))
        .clicked()
    {
        *wheels = ColorWheelsConfig::default();
        changed = true;
    }

    changed
}
