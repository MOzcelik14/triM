use crate::theme::*;
use egui::{Color32, Pos2, Rect, Stroke, Ui, Vec2};
use trim_core::project::Project;

use super::state::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_active_drag(
    ui: &mut Ui,
    painter: &egui::Painter,
    rect: Rect,
    project: &mut Project,
    state: &mut TimelineCanvasState,
    track_offsets: &[(f32, f32)],
    current_time: f64,
    pointer: &egui::PointerState,
    snap_thresh: f64,
    action: &mut Option<CanvasAction>,
) {
    // Active drag handling
    if state.drag_mode != DragMode::None {
        if let Some(pos) = pointer.interact_pos() {
            let dx = pos.x - state.drag_start_x;
            let dt = (dx / state.pixels_per_second) as f64;

            match state.drag_mode {
                DragMode::Rubberband => {
                    if let Some(start) = state.rubberband_start {
                        let rb = Rect::from_two_pos(start, pos);
                        state.rubberband_rect = Some(rb);
                        // Multi select intersected clips
                        let mut intersected = Vec::new();
                        for (t_idx, tr) in project.timeline.tracks.iter().enumerate() {
                            let (ty, track_h) = track_offsets[t_idx];
                            for cl in &tr.clips {
                                let cx =
                                    rect.left() + (cl.timeline_in as f32 * state.pixels_per_second);
                                let cw = (cl.duration() as f32 * state.pixels_per_second).max(4.0);
                                let cl_rect =
                                    Rect::from_min_size(Pos2::new(cx, ty), Vec2::new(cw, track_h));
                                if rb.intersects(cl_rect) {
                                    intersected.push((tr.id.clone(), cl.id.clone()));
                                }
                            }
                        }
                        state.selected_clip_pairs = intersected;
                    }
                }
                DragMode::Scrub => {
                    let t = ((pos.x - rect.left()) / state.pixels_per_second).max(0.0) as f64;
                    let snapped = project.timeline.snap_time(t, Some(snap_thresh), None);
                    *action = Some(CanvasAction::Seek(snapped));
                }
                DragMode::Move => {
                    if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id)
                    {
                        let target_in = (state.drag_orig_in + dt).max(0.0);
                        let clip_dur = (state.drag_orig_out - state.drag_orig_in).max(0.0);
                        let (snapped_in, snap_pt) = project.timeline.snap_clip_bounds(
                            target_in,
                            clip_dur,
                            current_time,
                            Some(snap_thresh),
                            Some(cl_id),
                        );
                        let actual_dt = snapped_in - state.drag_orig_in;

                        state.snap_guide_x =
                            snap_pt.map(|pt| rect.left() + (pt as f32 * state.pixels_per_second));

                        // Determine target track under cursor Y (matching track type, unlocked)
                        if let Some(orig_tr) = project.timeline.get_track_by_id(tr_id) {
                            let orig_type = orig_tr.track_type;
                            let mut target_tr_id = tr_id.clone();
                            for (t_idx, tr) in project.timeline.tracks.iter().enumerate() {
                                let (ty, track_h) = track_offsets[t_idx];
                                if pos.y >= ty && pos.y <= ty + track_h + state.track_spacing {
                                    if tr.track_type == orig_type && !tr.locked {
                                        target_tr_id = tr.id.clone();
                                    }
                                    break;
                                }
                            }
                            state.drag_target_track_id = Some(target_tr_id);
                        }

                        if state.selected_clip_pairs.len() > 1 {
                            let min_orig = state
                                .batch_orig_ins
                                .iter()
                                .map(|(_, _, o)| *o)
                                .fold(f64::INFINITY, f64::min);
                            let safe_dt = if min_orig.is_finite() {
                                actual_dt.max(-min_orig)
                            } else {
                                actual_dt
                            };
                            for (t_id, c_id, orig_in) in &state.batch_orig_ins {
                                if let Some(tr) = project.timeline.get_track_by_id_mut(t_id) {
                                    if let Some(cl) = tr.get_clip_by_id_mut(c_id) {
                                        cl.move_to((orig_in + safe_dt).max(0.0));
                                    }
                                }
                            }
                        } else if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                cl.move_to(snapped_in);
                            }
                        }
                    }
                }
                DragMode::TrimIn => {
                    if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id)
                    {
                        let target_in = (state.drag_orig_in + dt).max(0.0);
                        let (snapped_in, snap_pt) = project.timeline.snap_time_with_playhead(
                            target_in,
                            current_time,
                            Some(snap_thresh),
                            Some(cl_id),
                        );
                        state.snap_guide_x =
                            snap_pt.map(|pt| rect.left() + (pt as f32 * state.pixels_per_second));
                        let actual_dt = snapped_in - state.drag_orig_in;
                        if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                cl.trim_in(snapped_in);
                            }
                        }
                        for (comp_tr_id, comp_cl_id, orig_in, _, _, _) in &state.drag_companions {
                            if let Some(tr) = project.timeline.get_track_by_id_mut(comp_tr_id) {
                                if let Some(cl) = tr.get_clip_by_id_mut(comp_cl_id) {
                                    let comp_target_in = (orig_in + actual_dt).max(0.0);
                                    cl.trim_in(comp_target_in);
                                }
                            }
                        }
                    }
                }
                DragMode::TrimOut => {
                    if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id)
                    {
                        let target_out = (state.drag_orig_out + dt).max(state.drag_orig_in + 0.04);
                        let (snapped_out, snap_pt) = project.timeline.snap_time_with_playhead(
                            target_out,
                            current_time,
                            Some(snap_thresh),
                            Some(cl_id),
                        );
                        state.snap_guide_x =
                            snap_pt.map(|pt| rect.left() + (pt as f32 * state.pixels_per_second));
                        let actual_dt = snapped_out - state.drag_orig_out;
                        if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                cl.trim_out(snapped_out);
                            }
                        }
                        for (comp_tr_id, comp_cl_id, _, orig_out, _, _) in &state.drag_companions {
                            if let Some(tr) = project.timeline.get_track_by_id_mut(comp_tr_id) {
                                if let Some(cl) = tr.get_clip_by_id_mut(comp_cl_id) {
                                    let comp_target_out =
                                        (orig_out + actual_dt).max(cl.timeline_in + 0.04);
                                    cl.trim_out(comp_target_out);
                                }
                            }
                        }
                    }
                }
                DragMode::RippleOut => {
                    if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id)
                    {
                        let target_out = (state.drag_orig_out + dt).max(state.drag_orig_in + 0.04);
                        let (snapped_out, snap_pt) = project.timeline.snap_time_with_playhead(
                            target_out,
                            current_time,
                            Some(snap_thresh),
                            Some(cl_id),
                        );
                        state.snap_guide_x =
                            snap_pt.map(|pt| rect.left() + (pt as f32 * state.pixels_per_second));
                        let shift_dt = snapped_out - state.drag_orig_out;
                        if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                cl.trim_out(snapped_out);
                            }
                        }
                        for (comp_tr_id, comp_cl_id, _, orig_out, _, _) in &state.drag_companions {
                            if let Some(tr) = project.timeline.get_track_by_id_mut(comp_tr_id) {
                                if let Some(cl) = tr.get_clip_by_id_mut(comp_cl_id) {
                                    let comp_target_out =
                                        (orig_out + shift_dt).max(cl.timeline_in + 0.04);
                                    cl.trim_out(comp_target_out);
                                }
                            }
                        }
                        for (t_id, c_id, orig_in) in &state.batch_orig_ins {
                            if let Some(tr) = project.timeline.get_track_by_id_mut(t_id) {
                                if let Some(c) = tr.get_clip_by_id_mut(c_id) {
                                    c.move_to((orig_in + shift_dt).max(0.0));
                                }
                            }
                        }
                    }
                }
                DragMode::RippleIn => {
                    if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id)
                    {
                        let target_in = (state.drag_orig_in + dt)
                            .max(0.0)
                            .min(state.drag_orig_out - 0.04);
                        let (snapped_in, snap_pt) = project.timeline.snap_time_with_playhead(
                            target_in,
                            current_time,
                            Some(snap_thresh),
                            Some(cl_id),
                        );
                        state.snap_guide_x =
                            snap_pt.map(|pt| rect.left() + (pt as f32 * state.pixels_per_second));
                        let mut shift_dt = 0.0;
                        if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                cl.trim_in(snapped_in);
                                shift_dt = cl.timeline_in - state.drag_orig_in;
                                cl.move_to(state.drag_orig_in);
                            }
                        }
                        for (comp_tr_id, comp_cl_id, orig_in, _, _, _) in &state.drag_companions {
                            if let Some(tr) = project.timeline.get_track_by_id_mut(comp_tr_id) {
                                if let Some(cl) = tr.get_clip_by_id_mut(comp_cl_id) {
                                    let comp_target_in = (orig_in + shift_dt).max(0.0);
                                    cl.trim_in(comp_target_in);
                                    cl.move_to(*orig_in);
                                }
                            }
                        }
                        for (t_id, c_id, orig_in) in &state.batch_orig_ins {
                            if let Some(tr) = project.timeline.get_track_by_id_mut(t_id) {
                                if let Some(c) = tr.get_clip_by_id_mut(c_id) {
                                    c.move_to((orig_in - shift_dt).max(0.0));
                                }
                            }
                        }
                    }
                }
                DragMode::Slip => {
                    if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id)
                    {
                        let media_id = project
                            .timeline
                            .get_track_by_id(tr_id)
                            .and_then(|tr| tr.get_clip_by_id(cl_id))
                            .map(|cl| cl.media_id.clone());
                        let media_dur = media_id
                            .as_deref()
                            .and_then(|id| project.get_media(id))
                            .map(|m| m.duration);

                        let (slip_delta, cur_src_in, cur_src_out) = if let Some(tr) =
                            project.timeline.get_track_by_id_mut(tr_id)
                        {
                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                let clip_dur = cl.duration();
                                let max_dur = media_dur
                                    .unwrap_or_else(|| cl.effective_source_out().max(10000.0));
                                let max_src_in = (max_dur - clip_dur).max(0.0);
                                let new_src_in = (state.drag_orig_sin - dt).clamp(0.0, max_src_in);
                                cl.source_in = new_src_in;
                                cl.source_out = Some(new_src_in + clip_dur);
                                (
                                    new_src_in - state.drag_orig_sin,
                                    cl.source_in,
                                    cl.effective_source_out(),
                                )
                            } else {
                                (0.0, 0.0, 0.0)
                            }
                        } else {
                            (0.0, 0.0, 0.0)
                        };

                        for (comp_tr_id, comp_cl_id, _, _, orig_sin, _) in &state.drag_companions {
                            if let Some(comp_tr) = project.timeline.get_track_by_id_mut(comp_tr_id)
                            {
                                if let Some(comp_cl) = comp_tr.get_clip_by_id_mut(comp_cl_id) {
                                    let comp_dur = comp_cl.duration();
                                    let new_c_sin = (orig_sin + slip_delta).max(0.0);
                                    comp_cl.source_in = new_c_sin;
                                    comp_cl.source_out = Some(new_c_sin + comp_dur);
                                }
                            }
                        }

                        let tip_rect = Rect::from_center_size(
                            Pos2::new(pos.x, pos.y - 18.0),
                            Vec2::new(140.0, 20.0),
                        );
                        painter.rect_filled(tip_rect, 4.0, Color32::from_rgb(0x18, 0x18, 0x22));
                        painter.rect_stroke(
                            tip_rect,
                            4.0,
                            Stroke::new(1.0_f32, Color32::from_rgb(0xe0, 0x7a, 0x38)),
                        );
                        painter.text(
                            tip_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("Slip: {:.2}s - {:.2}s", cur_src_in, cur_src_out),
                            egui::FontId::proportional(11.0),
                            Color32::WHITE,
                        );
                    }
                }
                DragMode::FadeIn => {
                    if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id)
                    {
                        if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                let raw = ((pos.x
                                    - (rect.left()
                                        + cl.timeline_in as f32 * state.pixels_per_second))
                                    / state.pixels_per_second)
                                    as f64;
                                cl.fade_in = raw.clamp(0.0, cl.duration() / 2.0);
                                let tip_rect = Rect::from_center_size(
                                    Pos2::new(pos.x, pos.y - 18.0),
                                    Vec2::new(96.0, 20.0),
                                );
                                painter.rect_filled(
                                    tip_rect,
                                    4.0,
                                    Color32::from_rgb(0x18, 0x18, 0x22),
                                );
                                painter.rect_stroke(
                                    tip_rect,
                                    4.0,
                                    Stroke::new(1.0_f32, Color32::from_rgb(0x38, 0xa0, 0xf0)),
                                );
                                painter.text(
                                    tip_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    format!("Fade In: {:.2}s", cl.fade_in),
                                    egui::FontId::proportional(11.0),
                                    Color32::WHITE,
                                );
                            }
                        }
                    }
                }
                DragMode::FadeOut => {
                    if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id)
                    {
                        if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                let cl_right = rect.left()
                                    + (cl.timeline_out as f32 * state.pixels_per_second);
                                let raw = ((cl_right - pos.x) / state.pixels_per_second) as f64;
                                cl.fade_out = raw.clamp(0.0, cl.duration() / 2.0);
                                let tip_rect = Rect::from_center_size(
                                    Pos2::new(pos.x, pos.y - 18.0),
                                    Vec2::new(96.0, 20.0),
                                );
                                painter.rect_filled(
                                    tip_rect,
                                    4.0,
                                    Color32::from_rgb(0x18, 0x18, 0x22),
                                );
                                painter.rect_stroke(
                                    tip_rect,
                                    4.0,
                                    Stroke::new(1.0_f32, Color32::from_rgb(0x38, 0xa0, 0xf0)),
                                );
                                painter.text(
                                    tip_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    format!("Fade Out: {:.2}s", cl.fade_out),
                                    egui::FontId::proportional(11.0),
                                    Color32::WHITE,
                                );
                            }
                        }
                    }
                }
                DragMode::VolumeRubberband => {
                    if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id)
                    {
                        if let Some(t_idx) =
                            project.timeline.tracks.iter().position(|t| &t.id == tr_id)
                        {
                            let (ty, track_h) = track_offsets[t_idx];
                            let active_top = ty + 18.0;
                            let active_bottom = ty + track_h - 6.0;
                            let range = (active_bottom - active_top).max(8.0);
                            let norm = ((active_bottom - pos.y) / range).clamp(0.0, 1.0);
                            let mut new_vol = (norm as f64) * 2.0;
                            if (new_vol - 1.0).abs() < 0.05 {
                                new_vol = 1.0;
                            }

                            let mut tip_time_str = String::new();
                            if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                                if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                    if let Some(kf_idx) = state.drag_kf_index {
                                        let dur = cl.duration().max(0.04);
                                        let cx = rect.left()
                                            + (cl.timeline_in as f32 * state.pixels_per_second);
                                        let cw = (dur as f32 * state.pixels_per_second).max(4.0);
                                        let rel_frac = ((pos.x - (cx + 2.0)) / (cw - 4.0))
                                            .clamp(0.0, 1.0)
                                            as f64;
                                        let new_t = (rel_frac * dur).clamp(0.0, dur);

                                        if let Some(kfs) = cl.keyframes.get_mut("volume") {
                                            if kf_idx < kfs.len() {
                                                let orig_t = state
                                                    .drag_orig_keyframes
                                                    .get(kf_idx)
                                                    .map(|k| k.time)
                                                    .unwrap_or(new_t);
                                                if orig_t <= 0.001 {
                                                    kfs[kf_idx].time = 0.0;
                                                } else if (orig_t - dur).abs() <= 0.001 {
                                                    kfs[kf_idx].time = dur;
                                                } else {
                                                    kfs[kf_idx].time = new_t;
                                                }
                                                kfs[kf_idx].value = new_vol;
                                                tip_time_str =
                                                    format!("{:.2}s | ", kfs[kf_idx].time);
                                            }
                                        }
                                    } else {
                                        cl.volume = new_vol;
                                    }
                                }
                            }

                            let db = if new_vol > 0.0001 {
                                20.0 * new_vol.log10()
                            } else {
                                -99.0
                            };
                            let tip_str = if db <= -60.0 {
                                format!("{}Ses: -∞ dB (Sessiz)", tip_time_str)
                            } else {
                                format!(
                                    "{}Ses: {:+.1} dB ({:.0}%)",
                                    tip_time_str,
                                    db,
                                    new_vol * 100.0
                                )
                            };
                            let tip_rect = Rect::from_center_size(
                                Pos2::new(pos.x, pos.y - 20.0),
                                Vec2::new(160.0, 22.0),
                            );
                            painter.rect_filled(tip_rect, 4.0, Color32::from_rgb(0x18, 0x1b, 0x24));
                            painter.rect_stroke(tip_rect, 4.0, Stroke::new(1.0_f32, COLOR_AMBER));
                            painter.text(
                                tip_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                tip_str,
                                egui::FontId::proportional(11.0),
                                COLOR_AMBER,
                            );
                        }
                    }
                }
                DragMode::HandPan => {
                    let dx = pos.x - state.drag_start_x;
                    ui.scroll_with_delta(Vec2::new(dx, 0.0));
                    *action = Some(CanvasAction::Pan(dx));
                    state.drag_start_x = pos.x;
                }
                _ => {}
            }
        }
    }
}
