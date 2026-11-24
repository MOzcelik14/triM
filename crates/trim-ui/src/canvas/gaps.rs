use super::state::TimelineCanvasState;
use crate::theme::COLOR_AMBER;
use egui::{Color32, Pos2, Rect, Stroke, Ui, Vec2};
use trim_core::track::Track;

use super::state::DragMode;

#[allow(clippy::too_many_arguments)]
pub(super) fn render_track_gaps(
    ui: &Ui,
    painter: &egui::Painter,
    rect: Rect,
    view_clip_rect: Rect,
    track: &Track,
    ty: f32,
    track_h: f32,
    state: &TimelineCanvasState,
) {
    let mut prev_t = 0.0;
    for clip in &track.clips {
        if clip.timeline_in - prev_t > 0.04 {
            let g_in = prev_t;
            let g_out = clip.timeline_in;
            let gx = rect.left() + (g_in as f32 * state.pixels_per_second);
            let gw = ((g_out - g_in) as f32 * state.pixels_per_second).max(2.0);
            let gap_rect = Rect::from_min_size(Pos2::new(gx, ty), Vec2::new(gw, track_h));
            if gap_rect.right() < view_clip_rect.left() - 32.0
                || gap_rect.left() > view_clip_rect.right() + 32.0
            {
                prev_t = prev_t.max(clip.timeline_out);
                continue;
            }

            let is_gap_sel = state
                .selected_gap
                .as_ref()
                .is_some_and(|(sid, s_in, s_out)| {
                    sid == &track.id && (*s_in - g_in).abs() < 1e-9 && (*s_out - g_out).abs() < 1e-9
                });
            let is_hovered = ui
                .input(|i| i.pointer.hover_pos())
                .map(|p| gap_rect.contains(p))
                .unwrap_or(false);

            if is_gap_sel {
                painter.rect_filled(
                    gap_rect,
                    2.0,
                    Color32::from_rgba_premultiplied(0xf5, 0x9e, 0x0b, 50),
                );
                painter.rect_stroke(gap_rect, 2.0, Stroke::new(1.5_f32, COLOR_AMBER));

                // Diagonal hatch lines inside selected gap
                let step = 14.0_f32;
                let mut lx = gap_rect.left();
                while lx < gap_rect.right() + gap_rect.height() {
                    let p1 = Pos2::new(
                        lx.min(gap_rect.right()),
                        gap_rect.top() + (lx - gap_rect.right()).max(0.0),
                    );
                    let p2 = Pos2::new(
                        (lx - gap_rect.height()).max(gap_rect.left()),
                        gap_rect.top()
                            + (lx - (lx - gap_rect.height()).max(gap_rect.left()))
                                .min(gap_rect.height()),
                    );
                    painter.line_segment(
                        [p1, p2],
                        Stroke::new(
                            1.0_f32,
                            Color32::from_rgba_premultiplied(0xf5, 0x9e, 0x0b, 70),
                        ),
                    );
                    lx += step;
                }

                if gap_rect.width() > 64.0 {
                    let dur = g_out - g_in;
                    let badge_text = format!("⏩ Boşluk: {:.2}s", dur);
                    let badge_rect =
                        Rect::from_center_size(gap_rect.center(), Vec2::new(104.0, 18.0));
                    painter.rect_filled(
                        badge_rect,
                        4.0,
                        Color32::from_rgba_premultiplied(19, 21, 28, 225),
                    );
                    painter.rect_stroke(badge_rect, 4.0, Stroke::new(1.0_f32, COLOR_AMBER));
                    painter.text(
                        badge_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        badge_text,
                        egui::FontId::proportional(10.0),
                        COLOR_AMBER,
                    );
                }
            } else if is_hovered && state.drag_mode == DragMode::None {
                painter.rect_filled(
                    gap_rect,
                    2.0,
                    Color32::from_rgba_premultiplied(255, 255, 255, 12),
                );
                painter.rect_stroke(
                    gap_rect,
                    2.0,
                    Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 40)),
                );
            }
        }
        prev_t = prev_t.max(clip.timeline_out);
    }
}
