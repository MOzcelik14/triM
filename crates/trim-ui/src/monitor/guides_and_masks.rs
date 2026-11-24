use super::{ActiveMaskOverlay, MonitorAction};
use egui::{Color32, Pos2, Rect, Response, Stroke, Ui, Vec2};

pub(super) fn fit_aspect(max_w: f32, max_h: f32, aspect: f32) -> (f32, f32) {
    if max_w <= 0.0 || max_h <= 0.0 {
        return (100.0, 100.0);
    }
    let h_from_w = max_w / aspect;
    if h_from_w <= max_h {
        (max_w, h_from_w)
    } else {
        (max_h * aspect, max_h)
    }
}

pub(super) fn draw_corner_guides(painter: &egui::Painter, rect: Rect) {
    let guide_len = 10.0;
    let guide_stroke = Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 45));

    // Top-Left
    painter.line_segment(
        [
            rect.left_top(),
            Pos2::new(rect.left() + guide_len, rect.top()),
        ],
        guide_stroke,
    );
    painter.line_segment(
        [
            rect.left_top(),
            Pos2::new(rect.left(), rect.top() + guide_len),
        ],
        guide_stroke,
    );

    // Top-Right
    painter.line_segment(
        [
            rect.right_top(),
            Pos2::new(rect.right() - guide_len, rect.top()),
        ],
        guide_stroke,
    );
    painter.line_segment(
        [
            rect.right_top(),
            Pos2::new(rect.right(), rect.top() + guide_len),
        ],
        guide_stroke,
    );

    // Bottom-Left
    painter.line_segment(
        [
            rect.left_bottom(),
            Pos2::new(rect.left() + guide_len, rect.bottom()),
        ],
        guide_stroke,
    );
    painter.line_segment(
        [
            rect.left_bottom(),
            Pos2::new(rect.left(), rect.bottom() - guide_len),
        ],
        guide_stroke,
    );

    // Bottom-Right
    painter.line_segment(
        [
            rect.right_bottom(),
            Pos2::new(rect.right() - guide_len, rect.bottom()),
        ],
        guide_stroke,
    );
    painter.line_segment(
        [
            rect.right_bottom(),
            Pos2::new(rect.right(), rect.bottom() - guide_len),
        ],
        guide_stroke,
    );
}

pub(super) fn draw_button(painter: &egui::Painter, rect: Rect, text: &str, is_primary: bool) {
    let bg = if is_primary {
        Color32::from_rgb(0xe0, 0x7a, 0x38)
    } else {
        Color32::from_rgb(0x24, 0x24, 0x2b)
    };
    let text_col = Color32::WHITE;

    painter.rect_filled(rect, 4.0, bg);
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(
            1.0_f32,
            if is_primary {
                Color32::from_rgb(0xcb, 0x69, 0x29)
            } else {
                Color32::from_rgb(0x38, 0x38, 0x44)
            },
        ),
    );

    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(12.0),
        text_col,
    );
}

pub(super) fn draw_safe_zones(painter: &egui::Painter, rect: Rect, is_vertical: bool) {
    let action_safe = rect.shrink2(Vec2::new(rect.width() * 0.05, rect.height() * 0.05));
    let title_safe = rect.shrink2(Vec2::new(rect.width() * 0.10, rect.height() * 0.10));

    let stroke_action = Stroke::new(
        1.0_f32,
        Color32::from_rgba_premultiplied(0x38, 0xa0, 0xe8, 60),
    );
    let stroke_title = Stroke::new(
        1.0_f32,
        Color32::from_rgba_premultiplied(0xe0, 0xaa, 0x50, 70),
    );

    painter.rect_stroke(action_safe, 0.0, stroke_action);
    painter.rect_stroke(title_safe, 0.0, stroke_title);

    // Center crosshair
    let c = rect.center();
    let cross_len = 8.0;
    let cross_stroke = Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 60));
    painter.line_segment(
        [
            Pos2::new(c.x - cross_len, c.y),
            Pos2::new(c.x + cross_len, c.y),
        ],
        cross_stroke,
    );
    painter.line_segment(
        [
            Pos2::new(c.x, c.y - cross_len),
            Pos2::new(c.x, c.y + cross_len),
        ],
        cross_stroke,
    );

    if is_vertical {
        // Vertical Shorts / Reels / TikTok UI zones overlay
        // Right side buttons zone (likes, comments, share)
        let right_ui = Rect::from_min_max(
            Pos2::new(
                rect.right() - rect.width() * 0.18,
                rect.top() + rect.height() * 0.40,
            ),
            Pos2::new(rect.right(), rect.top() + rect.height() * 0.85),
        );
        painter.rect_filled(
            right_ui,
            2.0,
            Color32::from_rgba_premultiplied(240, 60, 60, 25),
        );
        painter.rect_stroke(
            right_ui,
            2.0,
            Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(240, 60, 60, 60)),
        );

        // Bottom captions zone
        let bottom_ui = Rect::from_min_max(
            Pos2::new(rect.left(), rect.bottom() - rect.height() * 0.20),
            Pos2::new(rect.right(), rect.bottom()),
        );
        painter.rect_filled(
            bottom_ui,
            2.0,
            Color32::from_rgba_premultiplied(240, 60, 60, 20),
        );
        painter.rect_stroke(
            bottom_ui,
            2.0,
            Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(240, 60, 60, 50)),
        );

        painter.text(
            Pos2::new(rect.left() + 6.0, rect.bottom() - 14.0),
            egui::Align2::LEFT_BOTTOM,
            "Reels/TikTok UI Alanı",
            egui::FontId::proportional(9.5),
            Color32::from_rgba_premultiplied(240, 180, 180, 140),
        );
    } else {
        // 2.39:1 Anamorphic Cinemascope letterbox safe-frame guides on landscape canvas
        let scope_h = (rect.width() / 2.39).min(rect.height());
        if scope_h + 4.0 < rect.height() {
            let bar_h = (rect.height() - scope_h) * 0.5;
            let top_y = rect.top() + bar_h;
            let bot_y = rect.bottom() - bar_h;
            let scope_stroke = Stroke::new(
                1.0_f32,
                Color32::from_rgba_premultiplied(0xb7, 0xe3, 0x6a, 110),
            );
            painter.line_segment(
                [
                    Pos2::new(rect.left(), top_y),
                    Pos2::new(rect.right(), top_y),
                ],
                scope_stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(rect.left(), bot_y),
                    Pos2::new(rect.right(), bot_y),
                ],
                scope_stroke,
            );
            painter.text(
                Pos2::new(rect.right() - 8.0, top_y - 3.0),
                egui::Align2::RIGHT_BOTTOM,
                "2.39:1 Sinemaskop",
                egui::FontId::monospace(8.5),
                Color32::from_rgba_premultiplied(0xb7, 0xe3, 0x6a, 160),
            );
        }
    }
}

#[derive(Clone, Debug)]
enum MaskHandleKind {
    Center,
    Corner,
    Rotation,
    BezierVertex(usize),
    BezierInTangent(usize),
    BezierOutTangent(usize),
}

#[derive(Clone, Debug)]
struct MaskDragState {
    mask_idx: usize,
    kind: MaskHandleKind,
    old_masks: Vec<trim_core::Mask>,
}

pub(super) fn handle_mask_overlay_interaction(
    ui: &mut Ui,
    painter: &egui::Painter,
    frame_rect: Rect,
    response: &Response,
    ov: &mut ActiveMaskOverlay,
    action: &mut Option<MonitorAction>,
) {
    if ov.masks.is_empty() || frame_rect.width() <= 1.0 || frame_rect.height() <= 1.0 {
        return;
    }

    let uv_to_screen = |u: f32, v: f32| -> Pos2 {
        Pos2::new(
            frame_rect.left() + u * frame_rect.width(),
            frame_rect.top() + v * frame_rect.height(),
        )
    };
    let screen_to_uv = |p: Pos2| -> [f32; 2] {
        [
            ((p.x - frame_rect.left()) / frame_rect.width()).clamp(0.0, 1.0),
            ((p.y - frame_rect.top()) / frame_rect.height()).clamp(0.0, 1.0),
        ]
    };

    let drag_state_id = ui.make_persistent_id(format!("monitor_mask_drag_{}", ov.clip_id));
    let mut drag_state: Option<MaskDragState> = ui.data_mut(|d| d.get_temp(drag_state_id));

    let mut hit_candidates: Vec<(usize, MaskHandleKind, Pos2)> = Vec::new();
    let aspect = frame_rect.width() / frame_rect.height().max(1.0);

    for (m_idx, mask) in ov.masks.iter().enumerate() {
        if !mask.enabled {
            continue;
        }
        let p = mask.eval_at(ov.clip_time);
        let col = if mask.target == trim_core::MaskTarget::ClipAlpha {
            Color32::from_rgb(0x2d, 0xd4, 0xbf)
        } else {
            Color32::from_rgb(0xf5, 0x9e, 0x0b)
        };
        let stroke = Stroke::new(1.5_f32, col);
        let rot = p.rotation_deg.to_radians();
        let (sin_r, cos_r) = rot.sin_cos();

        let local_to_uv = |lx: f32, ly: f32| -> [f32; 2] {
            let rx = lx * cos_r - ly * sin_r;
            let ry = lx * sin_r + ly * cos_r;
            [p.pos_x + rx / aspect, p.pos_y + ry]
        };

        match mask.shape {
            trim_core::MaskShape::Rectangle => {
                let hx = (p.scale_x * 0.5) * aspect;
                let hy = p.scale_y * 0.5;
                let corners_local = [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)];
                let pts: Vec<Pos2> = corners_local
                    .iter()
                    .map(|&(lx, ly)| {
                        let [u, v] = local_to_uv(lx, ly);
                        uv_to_screen(u, v)
                    })
                    .collect();
                painter.add(egui::Shape::closed_line(pts.clone(), stroke));
                for pt in &pts {
                    painter.rect_filled(Rect::from_center_size(*pt, Vec2::splat(7.0)), 1.0, col);
                    hit_candidates.push((m_idx, MaskHandleKind::Corner, *pt));
                }
                let [ru, rv] = local_to_uv(0.0, -hy - 0.06);
                let rot_pos = uv_to_screen(ru, rv);
                let top_mid = uv_to_screen(local_to_uv(0.0, -hy)[0], local_to_uv(0.0, -hy)[1]);
                painter.line_segment([top_mid, rot_pos], Stroke::new(1.0_f32, col));
                painter.circle_filled(rot_pos, 4.5, Color32::WHITE);
                painter.circle_stroke(rot_pos, 4.5, stroke);
                hit_candidates.push((m_idx, MaskHandleKind::Rotation, rot_pos));
            }
            trim_core::MaskShape::Ellipse => {
                let hx = (p.scale_x * 0.5) * aspect;
                let hy = p.scale_y * 0.5;
                let steps = 36;
                let mut pts = Vec::with_capacity(steps);
                for s in 0..steps {
                    let ang = (s as f32 / steps as f32) * std::f32::consts::TAU;
                    let [u, v] = local_to_uv(hx * ang.cos(), hy * ang.sin());
                    pts.push(uv_to_screen(u, v));
                }
                painter.add(egui::Shape::closed_line(pts, stroke));
                for &(lx, ly) in &[(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)] {
                    let [u, v] = local_to_uv(lx, ly);
                    let pt = uv_to_screen(u, v);
                    painter.rect_filled(Rect::from_center_size(pt, Vec2::splat(6.5)), 1.0, col);
                    hit_candidates.push((m_idx, MaskHandleKind::Corner, pt));
                }
                let [ru, rv] = local_to_uv(0.0, -hy - 0.06);
                let rot_pos = uv_to_screen(ru, rv);
                let [tu, tv] = local_to_uv(0.0, -hy);
                painter.line_segment([uv_to_screen(tu, tv), rot_pos], Stroke::new(1.0_f32, col));
                painter.circle_filled(rot_pos, 4.5, Color32::WHITE);
                painter.circle_stroke(rot_pos, 4.5, stroke);
                hit_candidates.push((m_idx, MaskHandleKind::Rotation, rot_pos));
            }
            trim_core::MaskShape::Bezier => {
                let poly = trim_core::mask::flatten_bezier_polygon(&mask.vertices, 12);
                if poly.len() >= 2 {
                    let pts: Vec<Pos2> = poly.iter().map(|uv| uv_to_screen(uv[0], uv[1])).collect();
                    painter.add(egui::Shape::closed_line(pts, stroke));
                }
                for (vi, v) in mask.vertices.iter().enumerate() {
                    let vp = uv_to_screen(v.pos[0], v.pos[1]);
                    let ip = uv_to_screen(v.pos[0] + v.in_tangent[0], v.pos[1] + v.in_tangent[1]);
                    let op = uv_to_screen(v.pos[0] + v.out_tangent[0], v.pos[1] + v.out_tangent[1]);
                    painter.line_segment([vp, ip], Stroke::new(1.0_f32, Color32::from_gray(170)));
                    painter.line_segment([vp, op], Stroke::new(1.0_f32, Color32::from_gray(170)));
                    painter.circle_filled(ip, 3.0, Color32::LIGHT_GRAY);
                    painter.circle_filled(op, 3.0, Color32::LIGHT_GRAY);
                    painter.circle_filled(vp, 4.5, col);
                    hit_candidates.push((m_idx, MaskHandleKind::BezierVertex(vi), vp));
                    hit_candidates.push((m_idx, MaskHandleKind::BezierInTangent(vi), ip));
                    hit_candidates.push((m_idx, MaskHandleKind::BezierOutTangent(vi), op));
                }
            }
        }

        let center_pos = uv_to_screen(p.pos_x, p.pos_y);
        painter.circle_filled(center_pos, 5.0, col);
        painter.circle_stroke(center_pos, 5.0, Stroke::new(1.0_f32, Color32::WHITE));
        hit_candidates.push((m_idx, MaskHandleKind::Center, center_pos));
    }

    if response.drag_started() {
        if let Some(ptr) = response.interact_pointer_pos() {
            let mut best: Option<(usize, MaskHandleKind, f32)> = None;
            for (m_idx, kind, hp) in &hit_candidates {
                let d = ptr.distance(*hp);
                if d <= 12.0 && best.as_ref().is_none_or(|b| d < b.2) {
                    best = Some((*m_idx, kind.clone(), d));
                }
            }
            if let Some((m_idx, kind, _)) = best {
                drag_state = Some(MaskDragState {
                    mask_idx: m_idx,
                    kind,
                    old_masks: ov.masks.clone(),
                });
                ui.data_mut(|d| d.insert_temp(drag_state_id, drag_state.clone()));
            }
        }
    }

    if response.dragged() {
        if let (Some(ref ds), Some(ptr)) = (&drag_state, response.interact_pointer_pos()) {
            let uv = screen_to_uv(ptr);
            let mut new_masks = ov.masks.clone();
            if let Some(m) = new_masks.get_mut(ds.mask_idx) {
                match ds.kind {
                    MaskHandleKind::Center => {
                        let dx = uv[0] as f64 - m.pos_x;
                        let dy = uv[1] as f64 - m.pos_y;
                        m.pos_x = uv[0] as f64;
                        m.pos_y = uv[1] as f64;
                        if m.shape == trim_core::MaskShape::Bezier {
                            for v in &mut m.vertices {
                                v.pos[0] += dx as f32;
                                v.pos[1] += dy as f32;
                            }
                        }
                    }
                    MaskHandleKind::Corner => {
                        let sx = ((uv[0] as f64 - m.pos_x).abs() * 2.0).clamp(0.02, 2.0);
                        let sy = ((uv[1] as f64 - m.pos_y).abs() * 2.0).clamp(0.02, 2.0);
                        m.scale_x = sx;
                        m.scale_y = sy;
                    }
                    MaskHandleKind::Rotation => {
                        let cp = uv_to_screen(m.pos_x as f32, m.pos_y as f32);
                        let ang = (ptr.y - cp.y).atan2(ptr.x - cp.x).to_degrees() + 90.0;
                        m.rotation = ang as f64;
                    }
                    MaskHandleKind::BezierVertex(vi) => {
                        if let Some(v) = m.vertices.get_mut(vi) {
                            v.pos = uv;
                        }
                    }
                    MaskHandleKind::BezierInTangent(vi) => {
                        if let Some(v) = m.vertices.get_mut(vi) {
                            v.in_tangent = [uv[0] - v.pos[0], uv[1] - v.pos[1]];
                        }
                    }
                    MaskHandleKind::BezierOutTangent(vi) => {
                        if let Some(v) = m.vertices.get_mut(vi) {
                            v.out_tangent = [uv[0] - v.pos[0], uv[1] - v.pos[1]];
                        }
                    }
                }
                *action = Some(MonitorAction::MasksEdited {
                    clip_id: ov.clip_id.clone(),
                    old_masks: ds.old_masks.clone(),
                    new_masks,
                    commit_undo: false,
                });
            }
        }
    }

    if response.drag_stopped() {
        if let Some(ds) = drag_state.take() {
            ui.data_mut(|d| d.remove::<Option<MaskDragState>>(drag_state_id));
            *action = Some(MonitorAction::MasksEdited {
                clip_id: ov.clip_id.clone(),
                old_masks: ds.old_masks,
                new_masks: ov.masks.clone(),
                commit_undo: true,
            });
        }
    }
}
