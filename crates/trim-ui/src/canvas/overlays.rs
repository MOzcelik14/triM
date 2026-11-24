use crate::theme::*;
use crate::toolbar::EditTool;
use egui::{Color32, Pos2, Rect, Response, Stroke, Ui, Vec2};
use trim_core::media::MediaType;
use trim_core::project::Project;

use super::geometry::*;
use super::state::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn render_overlays_and_cursor(
    ui: &mut Ui,
    painter: &egui::Painter,
    response: &Response,
    rect: Rect,
    project: &Project,
    state: &mut TimelineCanvasState,
    track_offsets: &[(f32, f32)],
    current_time: f64,
    pointer: &egui::PointerState,
    snap_thresh: f64,
    action: &mut Option<CanvasAction>,
) {
    // 2. Vertical marker guidelines across canvas
    let tracks_bottom = track_offsets
        .last()
        .map(|(ty, h)| ty + h)
        .unwrap_or(rect.top() + TRACK_TOP_PADDING);

    // 2. Vertical marker guidelines across canvas
    for marker in &project.timeline.markers {
        let mx = rect.left() + (marker.time as f32 * state.pixels_per_second);
        if mx >= rect.left() && mx <= rect.right() {
            painter.line_segment(
                [Pos2::new(mx, rect.top()), Pos2::new(mx, tracks_bottom)],
                Stroke::new(
                    1.0_f32,
                    Color32::from_rgba_premultiplied(0xe0, 0x7a, 0x38, 0x80),
                ),
            );
        }
    }

    // 3. Playhead line
    let ph_x = rect.left() + (current_time as f32 * state.pixels_per_second);
    painter.line_segment(
        [Pos2::new(ph_x, rect.top()), Pos2::new(ph_x, tracks_bottom)],
        Stroke::new(2.0_f32, COLOR_AMBER),
    );

    // 4. Snapping guide line
    if let Some(sx) = state.snap_guide_x {
        painter.line_segment(
            [Pos2::new(sx, rect.top()), Pos2::new(sx, tracks_bottom)],
            Stroke::new(1.5_f32, Color32::from_rgb(0xff, 0xff, 0xff)),
        );
    }

    // 5. Rubberband selection rectangle
    if let Some(rb) = state.rubberband_rect {
        painter.rect_filled(
            rb,
            0.0,
            Color32::from_rgba_premultiplied(0xe0, 0x7a, 0x38, 40),
        );
        painter.rect_stroke(rb, 0.0, Stroke::new(1.0_f32, COLOR_AMBER));
    }

    // 5b. Dragging clip ghost on target track (vertical cross-track preview)
    if state.drag_mode == DragMode::Move {
        if let (Some(tr_id), Some(cl_id), Some(target_tr_id)) = (
            &state.drag_track_id,
            &state.drag_clip_id,
            &state.drag_target_track_id,
        ) {
            if target_tr_id != tr_id {
                if let Some(target_idx) = project
                    .timeline
                    .tracks
                    .iter()
                    .position(|t| &t.id == target_tr_id)
                {
                    if let Some(orig_tr) = project.timeline.get_track_by_id(tr_id) {
                        if let Some(cl) = orig_tr.get_clip_by_id(cl_id) {
                            let (ghost_ty, ghost_h) = track_offsets[target_idx];
                            let ghost_cx =
                                rect.left() + (cl.timeline_in as f32 * state.pixels_per_second);
                            let ghost_cw =
                                (cl.duration() as f32 * state.pixels_per_second).max(4.0);
                            let ghost_rect = Rect::from_min_size(
                                Pos2::new(ghost_cx, ghost_ty),
                                Vec2::new(ghost_cw, ghost_h),
                            );
                            painter.rect_filled(
                                ghost_rect,
                                4.0,
                                Color32::from_rgba_premultiplied(0x38, 0xa0, 0xf0, 80),
                            );
                            painter.rect_stroke(
                                ghost_rect,
                                4.0,
                                Stroke::new(2.0_f32, Color32::from_rgb(0x5b, 0x9e, 0xe8)),
                            );
                        }
                    }
                }
            }
        }
    }

    // 6. Drag-and-drop payload from Media Bin onto Canvas
    if let Some(payload) = response.dnd_release_payload::<String>() {
        let media_id = (*payload).clone();
        if let Some(pos) = pointer.interact_pos() {
            let drop_time = ((pos.x - rect.left()) / state.pixels_per_second).max(0.0) as f64;
            let (snapped_time, _) = project.timeline.snap_time_with_playhead(
                drop_time,
                current_time,
                Some(snap_thresh),
                None,
            );

            // Find target track under mouse Y
            let mut target_track_id = None;
            for (t_idx, tr) in project.timeline.tracks.iter().enumerate() {
                let (ty, track_h) = track_offsets[t_idx];
                if pos.y >= ty && pos.y <= ty + track_h + state.track_spacing {
                    target_track_id = Some(tr.id.clone());
                    break;
                }
            }

            // Fallback track by media type
            let final_track_id = target_track_id.unwrap_or_else(|| {
                if let Some(media) = project.get_media(&media_id) {
                    if media.media_type == MediaType::Audio {
                        project
                            .timeline
                            .get_audio_tracks()
                            .first()
                            .map(|t| t.id.clone())
                    } else {
                        project
                            .timeline
                            .get_video_tracks()
                            .first()
                            .map(|t| t.id.clone())
                    }
                } else {
                    None
                }
                .unwrap_or_else(|| {
                    project
                        .timeline
                        .tracks
                        .first()
                        .map(|t| t.id.clone())
                        .unwrap_or_default()
                })
            });

            if !final_track_id.is_empty() {
                *action = Some(CanvasAction::MediaDropped {
                    media_id,
                    track_id: final_track_id,
                    time: snapped_time,
                });
            }
        }
    }

    if response.dnd_hover_payload::<String>().is_some() {
        if let Some(pos) = pointer.hover_pos() {
            let guide_time = ((pos.x - rect.left()) / state.pixels_per_second).max(0.0);
            let guide_x = rect.left() + guide_time * state.pixels_per_second;
            painter.line_segment(
                [
                    Pos2::new(guide_x, rect.top()),
                    Pos2::new(guide_x, rect.bottom()),
                ],
                Stroke::new(2.0_f32, Color32::from_rgb(0xe0, 0x7a, 0x38)),
            );
        }
    }

    // Set appropriate cursor based on active tool and drag mode
    match state.active_tool {
        EditTool::Razor => {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        EditTool::TrackSelect => {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeEast);
        }
        EditTool::Hand => {
            ui.ctx()
                .set_cursor_icon(if state.drag_mode == DragMode::HandPan {
                    egui::CursorIcon::Grabbing
                } else {
                    egui::CursorIcon::Grab
                });
        }
        EditTool::Type => {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        }
        EditTool::Slip | EditTool::Roll | EditTool::Slide => {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        EditTool::Ripple => {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
        }
        _ => {
            if matches!(
                state.drag_mode,
                DragMode::FadeIn
                    | DragMode::FadeOut
                    | DragMode::TrimIn
                    | DragMode::TrimOut
                    | DragMode::Slip
                    | DragMode::Roll
                    | DragMode::Slide
                    | DragMode::RippleIn
                    | DragMode::RippleOut
            ) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
        }
    }

    // Razor Hover Guide line (with magnetic snapping to playhead, neighbor clips, and markers)
    if state.active_tool == EditTool::Razor {
        if let Some(hover_pos) = pointer.hover_pos() {
            if rect.contains(hover_pos) {
                if let Some((tr_id, cl_id, _)) = find_clip_at_pos(project, state, rect, hover_pos) {
                    if let Some(tr) = project.timeline.get_track_by_id(&tr_id) {
                        if !tr.locked {
                            if let Some(cl) = tr.get_clip_by_id(&cl_id) {
                                let raw_cut_t =
                                    ((hover_pos.x - rect.left()) / state.pixels_per_second) as f64;
                                let (cut_t, snap_pt) = project.timeline.snap_time_with_playhead(
                                    raw_cut_t,
                                    current_time,
                                    Some(snap_thresh),
                                    Some(&cl_id),
                                );
                                if cut_t > cl.timeline_in + 0.04 && cut_t < cl.timeline_out - 0.04 {
                                    let t_idx = project
                                        .timeline
                                        .tracks
                                        .iter()
                                        .position(|t| t.id == tr_id)
                                        .unwrap_or(0);
                                    let (ty, track_h) = track_offsets[t_idx];
                                    let cx = rect.left() + (cut_t as f32 * state.pixels_per_second);
                                    if snap_pt.is_some() {
                                        painter.line_segment(
                                            [
                                                Pos2::new(cx, rect.top()),
                                                Pos2::new(cx, rect.bottom()),
                                            ],
                                            Stroke::new(
                                                1.5_f32,
                                                Color32::from_rgb(0xff, 0xff, 0xff),
                                            ),
                                        );
                                    }
                                    painter.line_segment(
                                        [Pos2::new(cx, ty), Pos2::new(cx, ty + track_h)],
                                        Stroke::new(2.0_f32, Color32::from_rgb(0xff, 0x48, 0x48)),
                                    );

                                    // Draw companion cut guides across linked tracks
                                    let is_alt = ui.input(|i| i.modifiers.alt);
                                    if !is_alt {
                                        if let Some(ref gid) = cl.linked_group_id {
                                            for (other_idx, other_tr) in
                                                project.timeline.tracks.iter().enumerate()
                                            {
                                                if other_tr.id != tr_id && !other_tr.locked {
                                                    if let Some(comp_c) =
                                                        other_tr.clips.iter().find(|c| {
                                                            c.linked_group_id.as_ref() == Some(gid)
                                                        })
                                                    {
                                                        if cut_t > comp_c.timeline_in + 1e-4
                                                            && cut_t < comp_c.timeline_out - 1e-4
                                                        {
                                                            let (comp_ty, comp_th) =
                                                                track_offsets[other_idx];
                                                            painter.line_segment(
                                                                [
                                                                    Pos2::new(cx, comp_ty),
                                                                    Pos2::new(
                                                                        cx,
                                                                        comp_ty + comp_th,
                                                                    ),
                                                                ],
                                                                Stroke::new(
                                                                    2.0_f32,
                                                                    Color32::from_rgb(
                                                                        0xff, 0x60, 0x60,
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    let badge_rect = Rect::from_center_size(
                                        Pos2::new(cx, ty + 10.0),
                                        Vec2::new(36.0, 16.0),
                                    );
                                    painter.rect_filled(
                                        badge_rect,
                                        3.0,
                                        Color32::from_rgb(0xd0, 0x30, 0x30),
                                    );
                                    painter.text(
                                        badge_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "KES",
                                        egui::FontId::proportional(10.0),
                                        Color32::WHITE,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Set dynamic cursor icons on hover
    if response.hovered() {
        if let Some(pos) = pointer.hover_pos() {
            if state.drag_mode == DragMode::None {
                match state.active_tool {
                    EditTool::Razor => {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                    }
                    EditTool::Hand => {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                    }
                    EditTool::Type => {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                    }
                    _ => {
                        if let Some((_, _, mode)) = find_clip_at_pos(project, state, rect, pos) {
                            match mode {
                                DragMode::TrimIn
                                | DragMode::TrimOut
                                | DragMode::Slip
                                | DragMode::RippleIn
                                | DragMode::RippleOut => {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                }
                                DragMode::VolumeRubberband => {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                                }
                                DragMode::FadeIn | DragMode::FadeOut => {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            } else if state.drag_mode == DragMode::VolumeRubberband {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }
        }
    }
}
