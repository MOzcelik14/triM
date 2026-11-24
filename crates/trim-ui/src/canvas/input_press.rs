use crate::toolbar::EditTool;
use egui::{Pos2, Rect, Response, Ui, Vec2};
use trim_core::project::Project;

use super::geometry::*;
use super::state::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_primary_press(
    ui: &mut Ui,
    response: &Response,
    rect: Rect,
    project: &mut Project,
    state: &mut TimelineCanvasState,
    track_offsets: &[(f32, f32)],
    current_time: f64,
    pointer: &egui::PointerState,
    snap_thresh: f64,
    action: &mut Option<CanvasAction>,
) -> bool {
    // 7. User Interactions: Mouse press, drag, release

    if response.hovered() && pointer.primary_pressed() {
        if let Some(pos) = pointer.interact_pos() {
            // Check tool-specific actions first
            match state.active_tool {
                EditTool::Razor => {
                    let is_alt = ui.input(|i| i.modifiers.alt);
                    let hit = find_clip_at_pos(project, state, rect, pos);
                    if let Some((tr_id, cl_id, _)) = hit {
                        if let Some(tr) = project.timeline.get_track_by_id(&tr_id) {
                            if !tr.locked {
                                if let Some(cl) = tr.get_clip_by_id(&cl_id) {
                                    let raw_cut_t =
                                        ((pos.x - rect.left()) / state.pixels_per_second) as f64;
                                    let (cut_t, _) = project.timeline.snap_time_with_playhead(
                                        raw_cut_t,
                                        current_time,
                                        Some(snap_thresh),
                                        Some(&cl_id),
                                    );
                                    if cut_t > cl.timeline_in + 0.04
                                        && cut_t < cl.timeline_out - 0.04
                                    {
                                        *action = Some(CanvasAction::RazorCut {
                                            track_id: tr_id,
                                            clip_id: cl_id,
                                            cut_time: cut_t,
                                            split_linked: !is_alt,
                                        });
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                    return true;
                }
                EditTool::Type => {
                    let clicked_t =
                        ((pos.x - rect.left()) / state.pixels_per_second).max(0.0) as f64;
                    let snapped_t = project
                        .timeline
                        .snap_time(clicked_t, Some(snap_thresh), None);
                    if let Some((tr_id, cl_id, _)) = find_clip_at_pos(project, state, rect, pos) {
                        state.selected_clip_pairs = vec![(tr_id, cl_id)];
                    }
                    *action = Some(CanvasAction::Seek(snapped_t));
                    return true;
                }
                EditTool::Hand => {
                    state.drag_mode = DragMode::HandPan;
                    state.drag_start_x = pos.x;
                    return true;
                }
                EditTool::TrackSelect => {
                    let hit = find_clip_at_pos(project, state, rect, pos);
                    if let Some((tr_id, cl_id, _)) = hit {
                        if let Some(tr) = project.timeline.get_track_by_id(&tr_id) {
                            if !tr.locked {
                                if let Some(cl) = tr.get_clip_by_id(&cl_id) {
                                    let start_t = cl.timeline_in;
                                    let is_shift = ui.input(|i| i.modifiers.shift);
                                    let mut forward_clips = Vec::new();
                                    for t in &project.timeline.tracks {
                                        if !t.locked && (is_shift || t.id == tr_id) {
                                            for c in &t.clips {
                                                if c.timeline_in >= start_t - 1e-4 {
                                                    forward_clips
                                                        .push((t.id.clone(), c.id.clone()));
                                                }
                                            }
                                        }
                                    }
                                    state.selected_clip_pairs = forward_clips;
                                    state.drag_mode = DragMode::Move;
                                    state.drag_clip_id = Some(cl_id);
                                    state.drag_track_id = Some(tr_id.clone());
                                    state.drag_target_track_id = Some(tr_id);
                                    state.drag_start_x = pos.x;
                                    state.drag_orig_in = cl.timeline_in;
                                    state.drag_orig_out = cl.timeline_out;
                                    state.drag_orig_sin = cl.source_in;
                                    state.drag_orig_sout = cl.effective_source_out();
                                    state.batch_orig_ins.clear();
                                    for (t_id, c_id) in &state.selected_clip_pairs {
                                        if let Some(tr_c) = project.timeline.get_track_by_id(t_id) {
                                            if let Some(c) = tr_c.get_clip_by_id(c_id) {
                                                state.batch_orig_ins.push((
                                                    t_id.clone(),
                                                    c_id.clone(),
                                                    c.timeline_in,
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                            return true;
                        }
                    }
                    return true;
                }
                _ => {}
            }

            let hit = find_clip_at_pos(project, state, rect, pos);
            let is_multi = ui.input(|i| i.modifiers.shift || i.modifiers.ctrl);

            if let Some((tr_id, cl_id, mode)) = hit {
                state.selected_gap = None;
                let pair = (tr_id.clone(), cl_id.clone());
                let is_alt = ui.input(|i| i.modifiers.alt);
                if is_multi {
                    if let Some(pos_idx) = state.selected_clip_pairs.iter().position(|p| p == &pair)
                    {
                        state.selected_clip_pairs.remove(pos_idx);
                    } else {
                        state.selected_clip_pairs.push(pair.clone());
                    }
                    *action = Some(CanvasAction::SelectClips(state.selected_clip_pairs.clone()));
                } else if !state.selected_clip_pairs.contains(&pair) {
                    let mut linked_pairs = vec![pair.clone()];
                    if !is_alt {
                        if let Some(tr) = project.timeline.get_track_by_id(&tr_id) {
                            if let Some(cl) = tr.get_clip_by_id(&cl_id) {
                                if let Some(ref gid) = cl.linked_group_id {
                                    for t in &project.timeline.tracks {
                                        for c in &t.clips {
                                            if c.linked_group_id.as_ref() == Some(gid)
                                                && (t.id != tr_id || c.id != cl_id)
                                            {
                                                linked_pairs.push((t.id.clone(), c.id.clone()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    state.selected_clip_pairs = linked_pairs.clone();
                    *action = Some(CanvasAction::SelectClips(linked_pairs));
                } else if !is_alt {
                    // Even if already selected, ensure companion linked clips are in selected_clip_pairs
                    if let Some(tr) = project.timeline.get_track_by_id(&tr_id) {
                        if let Some(cl) = tr.get_clip_by_id(&cl_id) {
                            if let Some(ref gid) = cl.linked_group_id {
                                for t in &project.timeline.tracks {
                                    for c in &t.clips {
                                        if c.linked_group_id.as_ref() == Some(gid) {
                                            let p = (t.id.clone(), c.id.clone());
                                            if !state.selected_clip_pairs.contains(&p) {
                                                state.selected_clip_pairs.push(p);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                state.drag_mode = mode;
                state.drag_clip_id = Some(cl_id.clone());
                state.drag_track_id = Some(tr_id.clone());
                state.drag_target_track_id = Some(tr_id.clone());
                state.drag_start_x = pos.x;

                let t_pos = project.timeline.tracks.iter().position(|t| t.id == tr_id);
                let (ty, track_h) = t_pos
                    .map(|idx| track_offsets[idx])
                    .unwrap_or((rect.top(), state.track_height));

                if let Some(tr) = project.timeline.get_track_by_id_mut(&tr_id) {
                    if let Some(cl) = tr.get_clip_by_id_mut(&cl_id) {
                        state.drag_orig_in = cl.timeline_in;
                        state.drag_orig_out = cl.timeline_out;
                        state.drag_orig_sin = cl.source_in;
                        state.drag_orig_sout = cl.effective_source_out();
                        state.drag_orig_vol = cl.volume;
                        state.drag_orig_fade = if mode == DragMode::FadeIn {
                            cl.fade_in
                        } else {
                            cl.fade_out
                        };
                        state.drag_orig_keyframes =
                            cl.keyframes.get("volume").cloned().unwrap_or_default();
                        state.drag_kf_index = None;

                        if ui.input(|i| {
                            i.pointer
                                .button_double_clicked(egui::PointerButton::Primary)
                        }) {
                            if let Some(ref seq_id) = cl.nested_sequence_id {
                                *action = Some(CanvasAction::OpenSequence(seq_id.clone()));
                                state.drag_mode = DragMode::None;
                            }
                        }

                        if mode == DragMode::VolumeRubberband {
                            let dur = cl.duration().max(0.04);
                            let active_top = ty + 18.0;
                            let active_bottom = ty + track_h - 6.0;
                            let range = (active_bottom - active_top).max(8.0);
                            let cx =
                                rect.left() + (cl.timeline_in as f32 * state.pixels_per_second);
                            let cw = (dur as f32 * state.pixels_per_second).max(4.0);
                            let clip_rect =
                                Rect::from_min_size(Pos2::new(cx, ty), Vec2::new(cw, track_h));

                            // 1. Check if user clicked an existing keyframe diamond
                            let mut clicked_kf_idx = None;
                            if let Some(kfs) = cl.keyframes.get("volume") {
                                for (idx, kf) in kfs.iter().enumerate() {
                                    let kf_frac = (kf.time / dur).clamp(0.0, 1.0) as f32;
                                    let kf_x = clip_rect.left()
                                        + 2.0
                                        + kf_frac * (clip_rect.width() - 4.0);
                                    let kf_norm = (kf.value / 2.0).clamp(0.0, 1.0) as f32;
                                    let kf_y = active_bottom - (kf_norm * range);
                                    if pos.distance(Pos2::new(kf_x, kf_y)) <= 8.0 {
                                        clicked_kf_idx = Some(idx);
                                        break;
                                    }
                                }
                            }

                            if let Some(idx) = clicked_kf_idx {
                                if is_alt {
                                    // Alt+click on keyframe: remove it!
                                    let old_kfs = state.drag_orig_keyframes.clone();
                                    let mut new_kfs = old_kfs.clone();
                                    new_kfs.remove(idx);
                                    cl.keyframes.insert("volume".to_string(), new_kfs.clone());
                                    *action = Some(CanvasAction::SetClipKeyframes {
                                        track_id: tr_id.clone(),
                                        clip_id: cl_id.clone(),
                                        property: "volume".to_string(),
                                        old_keyframes: old_kfs,
                                        new_keyframes: new_kfs,
                                    });
                                    state.drag_mode = DragMode::None;
                                } else {
                                    state.drag_kf_index = Some(idx);
                                }
                            } else if state.active_tool == EditTool::Pen
                                || ui.input(|i| i.modifiers.ctrl)
                            {
                                // Pen tool or Ctrl+click: insert a new keyframe!
                                let rel_frac = ((pos.x - (clip_rect.left() + 2.0))
                                    / (clip_rect.width() - 4.0))
                                    .clamp(0.0, 1.0)
                                    as f64;
                                let rel_t = (rel_frac * dur).clamp(0.0, dur);
                                let norm = ((active_bottom - pos.y) / range).clamp(0.0, 1.0);
                                let click_vol = ((norm as f64) * 2.0).clamp(0.0, 2.0);

                                let old_kfs = state.drag_orig_keyframes.clone();
                                let mut new_kfs = if old_kfs.is_empty() {
                                    vec![
                                        trim_core::keyframe::Keyframe::new(
                                            0.0,
                                            cl.volume,
                                            trim_core::keyframe::Interpolation::Linear,
                                        ),
                                        trim_core::keyframe::Keyframe::new(
                                            rel_t,
                                            click_vol,
                                            trim_core::keyframe::Interpolation::Linear,
                                        ),
                                        trim_core::keyframe::Keyframe::new(
                                            dur,
                                            cl.volume,
                                            trim_core::keyframe::Interpolation::Linear,
                                        ),
                                    ]
                                } else {
                                    let mut k = old_kfs.clone();
                                    k.push(trim_core::keyframe::Keyframe::new(
                                        rel_t,
                                        click_vol,
                                        trim_core::keyframe::Interpolation::Linear,
                                    ));
                                    k
                                };
                                new_kfs.sort_by(|a, b| {
                                    a.time
                                        .partial_cmp(&b.time)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                });
                                let added_idx = new_kfs
                                    .iter()
                                    .position(|k| (k.time - rel_t).abs() < 1e-4)
                                    .unwrap_or(0);
                                cl.keyframes.insert("volume".to_string(), new_kfs);
                                state.drag_kf_index = Some(added_idx);
                            } else if ui.input(|i| {
                                i.pointer
                                    .button_double_clicked(egui::PointerButton::Primary)
                            }) {
                                let old_v = cl.volume;
                                cl.volume = 1.0;
                                cl.keyframes.remove("volume");
                                *action = Some(CanvasAction::SetClipVolume {
                                    track_id: tr_id.clone(),
                                    clip_id: cl_id.clone(),
                                    old_volume: old_v,
                                    new_volume: 1.0,
                                });
                            }
                        }
                    }
                }

                state.drag_companions.clear();
                let is_alt = ui.input(|i| i.modifiers.alt);
                if !is_alt
                    && matches!(
                        mode,
                        DragMode::TrimIn
                            | DragMode::TrimOut
                            | DragMode::Slip
                            | DragMode::RippleIn
                            | DragMode::RippleOut
                    )
                {
                    if let Some(tr) = project.timeline.get_track_by_id(&tr_id) {
                        if let Some(cl) = tr.get_clip_by_id(&cl_id) {
                            if let Some(ref gid) = cl.linked_group_id {
                                for other_tr in &project.timeline.tracks {
                                    if !other_tr.locked {
                                        for other_c in &other_tr.clips {
                                            if (other_tr.id != tr_id || other_c.id != cl_id)
                                                && other_c.linked_group_id.as_ref() == Some(gid)
                                            {
                                                state.drag_companions.push((
                                                    other_tr.id.clone(),
                                                    other_c.id.clone(),
                                                    other_c.timeline_in,
                                                    other_c.timeline_out,
                                                    other_c.source_in,
                                                    other_c.effective_source_out(),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                state.batch_orig_ins.clear();
                if mode == DragMode::RippleIn || mode == DragMode::RippleOut {
                    let boundary = state.drag_orig_out - 1e-4;
                    for tr in &project.timeline.tracks {
                        if !tr.locked
                            && (tr.id == tr_id
                                || state
                                    .drag_companions
                                    .iter()
                                    .any(|(comp_tr, _, _, _, _, _)| comp_tr == &tr.id))
                        {
                            for c in &tr.clips {
                                let is_companion = state
                                    .drag_companions
                                    .iter()
                                    .any(|(_, comp_cl, _, _, _, _)| comp_cl == &c.id);
                                if c.id != cl_id && !is_companion && c.timeline_in >= boundary {
                                    state.batch_orig_ins.push((
                                        tr.id.clone(),
                                        c.id.clone(),
                                        c.timeline_in,
                                    ));
                                }
                            }
                        }
                    }
                } else {
                    for (t_id, c_id) in &state.selected_clip_pairs {
                        if let Some(tr) = project.timeline.get_track_by_id(t_id) {
                            if let Some(cl) = tr.get_clip_by_id(c_id) {
                                state.batch_orig_ins.push((
                                    t_id.clone(),
                                    c_id.clone(),
                                    cl.timeline_in,
                                ));
                            }
                        }
                    }
                }
            } else {
                // Clicked empty area - check if user clicked an inter-clip gap
                let gap_hit = find_gap_at_pos(project, state, rect, pos);
                if let Some(gap) = gap_hit {
                    state.selected_gap = Some(gap);
                    state.selected_clip_pairs.clear();
                    let clicked_t =
                        ((pos.x - rect.left()) / state.pixels_per_second).max(0.0) as f64;
                    let snapped_t = project
                        .timeline
                        .snap_time(clicked_t, Some(snap_thresh), None);
                    *action = Some(CanvasAction::Seek(snapped_t));
                } else if is_multi {
                    state.selected_gap = None;
                    state.drag_mode = DragMode::Rubberband;
                    state.rubberband_start = Some(pos);
                    state.rubberband_rect = Some(Rect::from_min_max(pos, pos));
                } else {
                    state.selected_gap = None;
                    state.selected_clip_pairs.clear();
                    state.drag_mode = DragMode::Scrub;
                    let clicked_t =
                        ((pos.x - rect.left()) / state.pixels_per_second).max(0.0) as f64;
                    let snapped_t = project
                        .timeline
                        .snap_time(clicked_t, Some(snap_thresh), None);
                    *action = Some(CanvasAction::Seek(snapped_t));
                }
            }
        }
    }
    false
}
