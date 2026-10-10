//! Timecode Ruler with adaptive ticks, marker pins and seek scrub.

use egui::{Color32, Pos2, Response, Sense, Stroke, Ui, Vec2};
use trim_core::marker::Marker;
use trim_core::time::seconds_to_timecode;

pub enum RulerAction {
    Seek(f64),
    MarkerClicked(String), // marker_id
}

pub fn render_time_ruler(
    ui: &mut Ui,
    pixels_per_second: f32,
    scroll_x: f32,
    total_duration: f64,
    fps: f64,
    current_time: f64,
    markers: &[Marker],
) -> (Response, Option<RulerAction>) {
    let avail_width = ui.available_width().max(600.0);
    let total_width = (total_duration as f32 + 30.0) * pixels_per_second;
    let ruler_width = avail_width.max(total_width);
    let ruler_height = 26.0;

    let (rect, response) = ui.allocate_exact_size(Vec2::new(ruler_width, ruler_height), Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    // Ruler background
    painter.rect_filled(rect, 0.0, Color32::from_rgb(0x18, 0x18, 0x1c));
    painter.line_segment(
        [Pos2::new(rect.left(), rect.bottom()), Pos2::new(rect.right(), rect.bottom())],
        Stroke::new(1.0_f32, Color32::from_rgb(0x28, 0x28, 0x30)),
    );

    // Adaptive tick interval
    let tick_interval = if pixels_per_second >= 200.0 {
        0.5
    } else if pixels_per_second >= 80.0 {
        1.0
    } else if pixels_per_second >= 30.0 {
        2.0
    } else if pixels_per_second >= 15.0 {
        5.0
    } else {
        10.0
    };

    let start_sec = (scroll_x / pixels_per_second).floor() as u64;
    let end_sec = ((scroll_x + avail_width) / pixels_per_second).ceil() as u64 + 10;

    let mut t = (start_sec as f64 / tick_interval).floor() * tick_interval;
    while t <= end_sec as f64 {
        let x = rect.left() + (t as f32 * pixels_per_second);
        if x >= rect.left() && x <= rect.right() {
            let is_major = (t.round() - t).abs() < 1e-4 && (t.round() as u64 % (tick_interval as u64 * 2).max(1) == 0);
            let tick_h = if is_major { 10.0 } else { 5.0 };

            painter.line_segment(
                [Pos2::new(x, rect.bottom() - tick_h), Pos2::new(x, rect.bottom())],
                Stroke::new(1.0_f32, Color32::from_rgb(0x40, 0x40, 0x4c)),
            );

            if is_major {
                let tc_str = seconds_to_timecode(t, fps);
                painter.text(
                    Pos2::new(x + 3.0, rect.top() + 4.0),
                    egui::Align2::LEFT_TOP,
                    tc_str,
                    egui::FontId::monospace(9.0),
                    Color32::from_rgb(0x80, 0x80, 0x90),
                );
            }
        }
        t += tick_interval;
    }

    let mut action = None;

    // Draw marker diamond pins on the ruler
    for marker in markers {
        let mx = rect.left() + (marker.time as f32 * pixels_per_second);
        if mx >= rect.left() && mx <= rect.right() {
            let my = rect.top() + 13.0;
            let m_color = parse_color(&marker.color).unwrap_or(Color32::from_rgb(0xe0, 0x7a, 0x38));

            // Diamond path: 4 vertices
            let sz = 5.0;
            let diamond = vec![
                Pos2::new(mx, my - sz),
                Pos2::new(mx + sz, my),
                Pos2::new(mx, my + sz),
                Pos2::new(mx - sz, my),
            ];
            painter.add(egui::Shape::convex_polygon(diamond, m_color, Stroke::new(1.0_f32, Color32::from_rgb(0x10, 0x10, 0x14))));

            if let Some(mouse_pos) = response.hover_pos() {
                if (mouse_pos.x - mx).abs() <= 6.0 && (mouse_pos.y - my).abs() <= 8.0 {
                    let tc = seconds_to_timecode(marker.time, fps);
                    let name = if marker.name.is_empty() { "İşaretçi" } else { &marker.name };
                    egui::show_tooltip_text(ui.ctx(), ui.layer_id(), response.id, format!("{} ({})", name, tc));

                    if ui.input(|i| i.pointer.primary_clicked()) {
                        action = Some(RulerAction::MarkerClicked(marker.id.clone()));
                    }
                }
            }
        }
    }

    // Playhead handle badge on ruler
    let ph_x = rect.left() + (current_time as f32 * pixels_per_second);
    let ph_badge = vec![
        Pos2::new(ph_x - 5.0, rect.top()),
        Pos2::new(ph_x + 5.0, rect.top()),
        Pos2::new(ph_x + 5.0, rect.top() + 16.0),
        Pos2::new(ph_x, rect.bottom()),
        Pos2::new(ph_x - 5.0, rect.top() + 16.0),
    ];
    painter.add(egui::Shape::convex_polygon(
        ph_badge,
        Color32::from_rgb(0xe0, 0x7a, 0x38),
        Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x20, 0x26)),
    ));

    // Handle click / drag scrub
    if response.dragged() || response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let t = ((pos.x - rect.left()) / pixels_per_second).max(0.0) as f64;
            action = Some(RulerAction::Seek(t));
        }
    }

    (response, action)
}

fn parse_color(hex: &str) -> Option<Color32> {
    let h = hex.trim().strip_prefix('#').unwrap_or(hex);
    if h.len() == 6 {
        let r = u8::from_str_radix(&h[0..2], 16).ok()?;
        let g = u8::from_str_radix(&h[2..4], 16).ok()?;
        let b = u8::from_str_radix(&h[4..6], 16).ok()?;
        Some(Color32::from_rgb(r, g, b))
    } else {
        None
    }
}
