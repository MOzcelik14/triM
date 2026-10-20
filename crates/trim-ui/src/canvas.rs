//! Interactive timeline canvas rendering tracks, clips, waveforms, fades, keyframes, playhead and editing gestures.

use crate::theme::*;
use crate::toolbar::EditTool;
use egui::{Color32, Pos2, Rect, Response, RichText, Sense, Stroke, Ui, Vec2};
use std::collections::HashSet;
use trim_core::media::MediaType;
use trim_core::project::Project;
use trim_core::track::TrackType;
use trim_media::WaveformGenerator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragMode {
    None,
    Move,
    TrimIn,
    TrimOut,
    FadeIn,
    FadeOut,
    Scrub,
    Rubberband,
    HandPan,
}

pub const TRACK_TOP_PADDING: f32 = 0.0;

#[derive(Debug, Clone)]
pub struct TimelineCanvasState {
    pub pixels_per_second: f32,
    pub track_height: f32,
    pub track_spacing: f32,
    pub trim_handle_width: f32,

    pub selected_clip_pairs: Vec<(String, String)>, // (track_id, clip_id)

    pub active_tool: EditTool,
    pub drag_mode: DragMode,
    pub drag_clip_id: Option<String>,
    pub drag_track_id: Option<String>,
    pub drag_start_x: f32,
    pub drag_orig_in: f64,
    pub drag_orig_out: f64,
    pub drag_orig_sin: f64,
    pub drag_orig_sout: f64,
    pub batch_orig_ins: Vec<(String, String, f64)>, // (track_id, clip_id, orig_in)

    pub rubberband_start: Option<Pos2>,
    pub rubberband_rect: Option<Rect>,
    pub snap_guide_x: Option<f32>,
    pub drag_target_track_id: Option<String>,
    pub track_heights: std::collections::HashMap<String, f32>,
}

impl Default for TimelineCanvasState {
    fn default() -> Self {
        Self {
            pixels_per_second: 60.0,
            track_height: 56.0,
            track_spacing: 6.0,
            trim_handle_width: 8.0,
            selected_clip_pairs: Vec::new(),
            active_tool: EditTool::Select,
            drag_mode: DragMode::None,
            drag_clip_id: None,
            drag_track_id: None,
            drag_start_x: 0.0,
            drag_orig_in: 0.0,
            drag_orig_out: 0.0,
            drag_orig_sin: 0.0,
            drag_orig_sout: 0.0,
            batch_orig_ins: Vec::new(),
            rubberband_start: None,
            rubberband_rect: None,
            snap_guide_x: None,
            drag_target_track_id: None,
            track_heights: std::collections::HashMap::new(),
        }
    }
}

impl TimelineCanvasState {
    pub fn get_track_height(&self, track_id: &str) -> f32 {
        self.track_heights.get(track_id).copied().unwrap_or(self.track_height)
    }

    pub fn set_track_height(&mut self, track_id: &str, height: f32) {
        self.track_heights.insert(track_id.to_string(), height.clamp(32.0, 160.0));
    }

    pub fn calculate_track_offsets(&self, tracks: &[trim_core::track::Track], top: f32) -> Vec<(f32, f32)> {
        let mut offsets = Vec::with_capacity(tracks.len());
        let mut current_y = top + TRACK_TOP_PADDING;
        for track in tracks {
            let h = self.get_track_height(&track.id);
            offsets.push((current_y, h));
            current_y += h + self.track_spacing;
        }
        offsets
    }
}

pub enum CanvasAction {
    Seek(f64),
    SelectClip(String, String),
    ClearSelection,
    ClipMoved {
        old_track_id: String,
        new_track_id: String,
        clip_id: String,
        old_in: f64,
        new_in: f64,
    },
    BatchClipsMoved(Vec<(String, String, f64, f64)>),
    ClipTrimmed {
        track_id: String,
        clip_id: String,
        old_bounds: (f64, f64, f64, f64),
        new_bounds: (f64, f64, f64, f64),
    },
    ClipModified,
    PromptSpeed(String, String),
    DetachAudio(String, String),
    SplitAtPlayhead,
    RazorCut {
        track_id: String,
        clip_id: String,
        cut_time: f64,
    },
    CreateTitleAtTime(f64),
    Pan(f32),
    DeleteSelected,
    RippleDeleteSelected,
    MediaDropped {
        media_id: String,
        track_id: String,
        time: f64,
    },
}


pub fn render_timeline_canvas(
    ui: &mut Ui,
    project: &mut Project,
    state: &mut TimelineCanvasState,
    current_time: f64,
    waveform_gen: &WaveformGenerator,
) -> (Response, Option<CanvasAction>) {
    let min_sec = (project.timeline.duration() + 30.0).max(60.0) as f32;
    let canvas_width = ui.available_width().max(min_sec * state.pixels_per_second);
    let canvas_height = if project.timeline.tracks.is_empty() {
        state.track_height
    } else {
        project.timeline.tracks.iter().map(|t| state.get_track_height(&t.id) + state.track_spacing).sum::<f32>()
    };

    let (rect, response) = ui.allocate_exact_size(Vec2::new(canvas_width, canvas_height), Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    // Canvas background
    painter.rect_filled(rect, 0.0, Color32::from_rgb(0x16, 0x16, 0x1a));

    let mut action = None;
    let selected_ids: HashSet<String> = state.selected_clip_pairs.iter().map(|(_, c)| c.clone()).collect();
    let track_offsets = state.calculate_track_offsets(&project.timeline.tracks, rect.top());

    // 1. Draw track lanes
    for (t_idx, track) in project.timeline.tracks.iter().enumerate() {
        let (ty, track_h) = track_offsets[t_idx];
        let track_rect = Rect::from_min_size(Pos2::new(rect.left(), ty), Vec2::new(rect.width(), track_h));

        let lane_color = if t_idx % 2 == 0 {
            Color32::from_rgb(0x1c, 0x1c, 0x22)
        } else {
            Color32::from_rgb(0x20, 0x20, 0x26)
        };
        painter.rect_filled(track_rect, 0.0, lane_color);
        painter.line_segment(
            [Pos2::new(track_rect.left(), track_rect.bottom()), Pos2::new(track_rect.right(), track_rect.bottom())],
            Stroke::new(1.0_f32, Color32::from_rgb(0x2a, 0x2a, 0x32)),
        );

        // Draw clips on this track
        for clip in &track.clips {
            let cx = rect.left() + (clip.timeline_in as f32 * state.pixels_per_second);
            let cw = (clip.duration() as f32 * state.pixels_per_second).max(4.0);
            let clip_rect = Rect::from_min_size(Pos2::new(cx, ty), Vec2::new(cw, track_h));

            let is_sel = selected_ids.contains(&clip.id);

            // Clip body fill
            let base_color = match track.track_type {
                TrackType::Video => {
                    if is_sel { COLOR_CLIP_VIDEO_SELECTED } else { COLOR_CLIP_VIDEO }
                }
                TrackType::Audio => {
                    if is_sel { COLOR_CLIP_AUDIO_SELECTED } else { COLOR_CLIP_AUDIO }
                }
            };
            painter.rect_filled(clip_rect, 4.0, base_color);

            // Waveform
            if track.track_type == TrackType::Audio || project.get_media(&clip.media_id).is_some_and(|m| m.has_audio()) {
                if let Some(media) = project.get_media(&clip.media_id) {
                    if let Some(wf) = waveform_gen.get_waveform(&media.file_path) {
                        let wf_len = wf.len();
                        let pps = waveform_gen.points_per_second as f64;
                        let s_in = (clip.source_in * pps) as usize;
                        let s_out = (clip.effective_source_out() * pps) as usize;

                        if s_out > s_in && s_in < wf_len {
                            let end_idx = s_out.min(wf_len);
                            let slice = &wf[s_in..end_idx];
                            let cy = clip_rect.center().y;
                            let max_amp = (clip_rect.height() - 16.0) / 2.0;
                            let wf_col = if track.track_type == TrackType::Audio {
                                Color32::from_rgba_premultiplied(0x64, 0xe6, 0xa0, 0xb4)
                            } else {
                                Color32::from_rgba_premultiplied(0x8c, 0xcd, 0xff, 0x8c)
                            };

                            let vis_left = (rect.left() - clip_rect.left()).max(0.0);
                            let vis_right = (rect.right() - clip_rect.left()).min(clip_rect.width());

                            if vis_right > vis_left {
                                let mut px = vis_left;
                                let mut wf_shapes = Vec::with_capacity(((vis_right - vis_left) / 2.0).ceil() as usize);
                                while px < vis_right {
                                    let norm = px / clip_rect.width();
                                    let idx = (norm as f64 * slice.len() as f64) as usize;
                                    if idx < slice.len() {
                                        let [v_min, v_max] = slice[idx];
                                        let x_pos = clip_rect.left() + px;
                                        let y1 = cy + v_min * max_amp;
                                        let y2 = cy + v_max * max_amp;
                                        wf_shapes.push(egui::Shape::line_segment([Pos2::new(x_pos, y1), Pos2::new(x_pos, y2)], Stroke::new(1.0_f32, wf_col)));
                                    }
                                    px += 2.0;
                                }
                                painter.extend(wf_shapes);
                            }
                        }
                    }
                }
            }

            // Fade In wedge overlay & interactive handle dot
            let fi_w = (clip.fade_in as f32 * state.pixels_per_second).min(clip_rect.width() * 0.5);
            if clip.fade_in > 0.0 {
                let fi_poly = vec![
                    clip_rect.left_top(),
                    Pos2::new(clip_rect.left() + fi_w, clip_rect.top()),
                    clip_rect.left_bottom(),
                ];
                painter.add(egui::Shape::convex_polygon(
                    fi_poly,
                    Color32::from_rgba_premultiplied(0, 0, 0, 120),
                    Stroke::NONE,
                ));
                painter.line_segment(
                    [clip_rect.left_bottom(), Pos2::new(clip_rect.left() + fi_w, clip_rect.top())],
                    Stroke::new(1.5_f32, Color32::from_rgba_premultiplied(255, 255, 255, 190)),
                );
            }
            let fi_handle_pos = Pos2::new(clip_rect.left() + if clip.fade_in > 0.0 { fi_w } else { 7.0 }, clip_rect.top() + 7.0);
            let fi_hovered = ui.input(|i| i.pointer.hover_pos()).map(|p| p.distance(fi_handle_pos) <= 8.0).unwrap_or(false);
            let fi_radius = if fi_hovered { 5.5 } else { 3.5 };
            let fi_color = if fi_hovered { Color32::from_rgb(0x38, 0xa0, 0xf0) } else if clip.fade_in > 0.0 { Color32::WHITE } else { Color32::from_rgb(0xd0, 0xd0, 0xd8) };
            painter.circle_filled(fi_handle_pos, fi_radius, fi_color);
            painter.circle_stroke(fi_handle_pos, fi_radius, Stroke::new(1.0_f32, Color32::from_rgb(0x1a, 0x1a, 0x22)));

            // Fade Out wedge overlay & interactive handle dot
            let fo_w = (clip.fade_out as f32 * state.pixels_per_second).min(clip_rect.width() * 0.5);
            if clip.fade_out > 0.0 {
                let fo_poly = vec![
                    Pos2::new(clip_rect.right() - fo_w, clip_rect.top()),
                    clip_rect.right_top(),
                    clip_rect.right_bottom(),
                ];
                painter.add(egui::Shape::convex_polygon(
                    fo_poly,
                    Color32::from_rgba_premultiplied(0, 0, 0, 120),
                    Stroke::NONE,
                ));
                painter.line_segment(
                    [Pos2::new(clip_rect.right() - fo_w, clip_rect.top()), clip_rect.right_bottom()],
                    Stroke::new(1.5_f32, Color32::from_rgba_premultiplied(255, 255, 255, 190)),
                );
            }
            let fo_handle_pos = Pos2::new(clip_rect.right() - if clip.fade_out > 0.0 { fo_w } else { 7.0 }, clip_rect.top() + 7.0);
            let fo_hovered = ui.input(|i| i.pointer.hover_pos()).map(|p| p.distance(fo_handle_pos) <= 8.0).unwrap_or(false);
            let fo_radius = if fo_hovered { 5.5 } else { 3.5 };
            let fo_color = if fo_hovered { Color32::from_rgb(0x38, 0xa0, 0xf0) } else if clip.fade_out > 0.0 { Color32::WHITE } else { Color32::from_rgb(0xd0, 0xd0, 0xd8) };
            painter.circle_filled(fo_handle_pos, fo_radius, fo_color);
            painter.circle_stroke(fo_handle_pos, fo_radius, Stroke::new(1.0_f32, Color32::from_rgb(0x1a, 0x1a, 0x22)));


            // Clip border
            let border_stroke = if is_sel {
                Stroke::new(2.0_f32, COLOR_AMBER)
            } else {
                Stroke::new(1.0_f32, Color32::from_rgb(0x38, 0x38, 0x48))
            };
            painter.rect_stroke(clip_rect, 4.0, border_stroke);

            // Trim handles cues
            painter.rect_filled(
                Rect::from_min_size(clip_rect.left_top(), Vec2::new(state.trim_handle_width, clip_rect.height())),
                0.0,
                Color32::from_rgba_premultiplied(255, 255, 255, 20),
            );
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(clip_rect.right() - state.trim_handle_width, clip_rect.top()),
                    Vec2::new(state.trim_handle_width, clip_rect.height()),
                ),
                0.0,
                Color32::from_rgba_premultiplied(255, 255, 255, 20),
            );

            // Clip text & badges
            let speed_badge = if (clip.speed - 1.0).abs() > 0.01 || clip.reverse {
                let mut p = Vec::new();
                if (clip.speed - 1.0).abs() > 0.01 {
                    p.push(format!("{:.1}x", clip.speed));
                }
                if clip.reverse {
                    p.push("REV".into());
                }
                format!(" [{}]", p.join(" "))
            } else {
                String::new()
            };

            let kf_badge = if clip.has_keyframes(None) { " *" } else { "" };
            let lock_badge = if track.locked { " [K]" } else { "" };
            let title_str = format!("{}{}  [{:.2}s]{}{}", clip.name, lock_badge, clip.duration(), speed_badge, kf_badge);

            painter.text(
                Pos2::new(clip_rect.left() + 10.0, clip_rect.top() + 6.0),
                egui::Align2::LEFT_TOP,
                title_str,
                egui::FontId::proportional(11.0),
                Color32::WHITE,
            );

            // Keyframe diamonds along the bottom of the clip
            if clip.has_keyframes(None) {
                let mut drawn: HashSet<u64> = HashSet::new();
                for kfs in clip.keyframes.values() {
                    for kf in kfs {
                        let key_ts = (kf.time * 100.0).round() as u64;
                        if !drawn.contains(&key_ts) {
                            drawn.insert(key_ts);
                            let kx = clip_rect.left() + (kf.time as f32 * state.pixels_per_second);
                            let ky = clip_rect.bottom() - 8.0;
                            let dia = vec![
                                Pos2::new(kx, ky - 3.5),
                                Pos2::new(kx + 3.5, ky),
                                Pos2::new(kx, ky + 3.5),
                                Pos2::new(kx - 3.5, ky),
                            ];
                            painter.add(egui::Shape::convex_polygon(dia, COLOR_AMBER, Stroke::new(0.5_f32, Color32::BLACK)));
                        }
                    }
                }
            }
        }
    }

    // 2. Vertical marker guidelines across canvas
    let tracks_bottom = track_offsets.last().map(|(ty, h)| ty + h).unwrap_or(rect.top() + TRACK_TOP_PADDING);

    // 2. Vertical marker guidelines across canvas
    for marker in &project.timeline.markers {
        let mx = rect.left() + (marker.time as f32 * state.pixels_per_second);
        if mx >= rect.left() && mx <= rect.right() {
            painter.line_segment(
                [Pos2::new(mx, rect.top()), Pos2::new(mx, tracks_bottom)],
                Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(0xe0, 0x7a, 0x38, 0x80)),
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
        painter.rect_filled(rb, 0.0, Color32::from_rgba_premultiplied(0xe0, 0x7a, 0x38, 40));
        painter.rect_stroke(rb, 0.0, Stroke::new(1.0_f32, COLOR_AMBER));
    }

    // 5b. Dragging clip ghost on target track (vertical cross-track preview)
    if state.drag_mode == DragMode::Move {
        if let (Some(tr_id), Some(cl_id), Some(target_tr_id)) = (&state.drag_track_id, &state.drag_clip_id, &state.drag_target_track_id) {
            if target_tr_id != tr_id {
                if let Some(target_idx) = project.timeline.tracks.iter().position(|t| &t.id == target_tr_id) {
                    if let Some(orig_tr) = project.timeline.get_track_by_id(tr_id) {
                        if let Some(cl) = orig_tr.get_clip_by_id(cl_id) {
                            let (ghost_ty, ghost_h) = track_offsets[target_idx];
                            let ghost_cx = rect.left() + (cl.timeline_in as f32 * state.pixels_per_second);
                            let ghost_cw = (cl.duration() as f32 * state.pixels_per_second).max(4.0);
                            let ghost_rect = Rect::from_min_size(Pos2::new(ghost_cx, ghost_ty), Vec2::new(ghost_cw, ghost_h));
                            painter.rect_filled(ghost_rect, 4.0, Color32::from_rgba_premultiplied(0x38, 0xa0, 0xf0, 80));
                            painter.rect_stroke(ghost_rect, 4.0, Stroke::new(2.0_f32, Color32::from_rgb(0x5b, 0x9e, 0xe8)));
                        }
                    }
                }
            }
        }
    }

    // 6. Drag-and-drop payload from Media Bin onto Canvas
    let pointer = ui.input(|i| i.pointer.clone());

    if let Some(payload) = response.dnd_release_payload::<String>() {
        let media_id = (*payload).clone();
        if let Some(pos) = pointer.interact_pos() {
            let drop_time = ((pos.x - rect.left()) / state.pixels_per_second).max(0.0) as f64;
            let snapped_time = project.timeline.snap_time(drop_time, Some(0.1), None);

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
                        project.timeline.get_audio_tracks().first().map(|t| t.id.clone())
                    } else {
                        project.timeline.get_video_tracks().first().map(|t| t.id.clone())
                    }
                } else {
                    None
                }.unwrap_or_else(|| project.timeline.tracks.first().map(|t| t.id.clone()).unwrap_or_default())
            });

            if !final_track_id.is_empty() {
                action = Some(CanvasAction::MediaDropped {
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
                [Pos2::new(guide_x, rect.top()), Pos2::new(guide_x, rect.bottom())],
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
            ui.ctx().set_cursor_icon(if state.drag_mode == DragMode::HandPan { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        EditTool::Type => {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        }
        _ => {
            if matches!(state.drag_mode, DragMode::FadeIn | DragMode::FadeOut | DragMode::TrimIn | DragMode::TrimOut) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
        }
    }

    // Razor Hover Guide line
    if state.active_tool == EditTool::Razor {
        if let Some(hover_pos) = pointer.hover_pos() {
            if rect.contains(hover_pos) {
                if let Some((tr_id, cl_id, _)) = find_clip_at_pos(project, state, rect, hover_pos) {
                    if let Some(tr) = project.timeline.get_track_by_id(&tr_id) {
                        if !tr.locked {
                            if let Some(cl) = tr.get_clip_by_id(&cl_id) {
                                let cut_t = ((hover_pos.x - rect.left()) / state.pixels_per_second) as f64;
                                if cut_t > cl.timeline_in + 0.04 && cut_t < cl.timeline_out - 0.04 {
                                    let t_idx = project.timeline.tracks.iter().position(|t| t.id == tr_id).unwrap_or(0);
                                    let (ty, track_h) = track_offsets[t_idx];
                                    let cx = hover_pos.x;
                                    painter.line_segment(
                                        [Pos2::new(cx, ty), Pos2::new(cx, ty + track_h)],
                                        Stroke::new(2.0_f32, Color32::from_rgb(0xff, 0x48, 0x48)),
                                    );
                                    let badge_rect = Rect::from_center_size(Pos2::new(cx, ty + 10.0), Vec2::new(36.0, 16.0));
                                    painter.rect_filled(badge_rect, 3.0, Color32::from_rgb(0xd0, 0x30, 0x30));
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

    // 7. User Interactions: Mouse press, drag, release

    if response.hovered() && pointer.primary_pressed() {
        if let Some(pos) = pointer.interact_pos() {
            // Check tool-specific actions first
            match state.active_tool {
                EditTool::Razor => {
                    let hit = find_clip_at_pos(project, state, rect, pos);
                    if let Some((tr_id, cl_id, _)) = hit {
                        if let Some(tr) = project.timeline.get_track_by_id(&tr_id) {
                            if !tr.locked {
                                if let Some(cl) = tr.get_clip_by_id(&cl_id) {
                                    let cut_t = ((pos.x - rect.left()) / state.pixels_per_second) as f64;
                                    if cut_t > cl.timeline_in + 0.04 && cut_t < cl.timeline_out - 0.04 {
                                        action = Some(CanvasAction::RazorCut {
                                            track_id: tr_id,
                                            clip_id: cl_id,
                                            cut_time: cut_t,
                                        });
                                        return (response, action);
                                    }
                                }
                            }
                        }
                    }
                    return (response, action);
                }
                EditTool::Type => {
                    let clicked_t = ((pos.x - rect.left()) / state.pixels_per_second).max(0.0) as f64;
                    let snapped_t = project.timeline.snap_time(clicked_t, Some(0.1), None);
                    action = Some(CanvasAction::CreateTitleAtTime(snapped_t));
                    return (response, action);
                }
                EditTool::Hand => {
                    state.drag_mode = DragMode::HandPan;
                    state.drag_start_x = pos.x;
                    return (response, action);
                }
                EditTool::TrackSelect => {
                    let hit = find_clip_at_pos(project, state, rect, pos);
                    if let Some((tr_id, cl_id, _)) = hit {
                        if let Some(tr) = project.timeline.get_track_by_id(&tr_id) {
                            if let Some(cl) = tr.get_clip_by_id(&cl_id) {
                                let start_t = cl.timeline_in;
                                let is_shift = ui.input(|i| i.modifiers.shift);
                                let mut forward_clips = Vec::new();
                                for t in &project.timeline.tracks {
                                    if is_shift || t.id == tr_id {
                                        for c in &t.clips {
                                            if c.timeline_in >= start_t - 1e-4 {
                                                forward_clips.push((t.id.clone(), c.id.clone()));
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
                                    if let Some(t) = project.timeline.get_track_by_id(t_id) {
                                        if let Some(c) = t.get_clip_by_id(c_id) {
                                            state.batch_orig_ins.push((t_id.clone(), c_id.clone(), c.timeline_in));
                                        }
                                    }
                                }
                                return (response, action);
                            }
                        }
                    }
                    return (response, action);
                }
                _ => {}
            }

            let hit = find_clip_at_pos(project, state, rect, pos);
            let is_multi = ui.input(|i| i.modifiers.shift || i.modifiers.ctrl);

            if let Some((tr_id, cl_id, mode)) = hit {
                let pair = (tr_id.clone(), cl_id.clone());
                if is_multi {
                    if let Some(pos_idx) = state.selected_clip_pairs.iter().position(|p| p == &pair) {
                        state.selected_clip_pairs.remove(pos_idx);
                    } else {
                        state.selected_clip_pairs.push(pair.clone());
                    }
                } else if !state.selected_clip_pairs.contains(&pair) {
                    state.selected_clip_pairs = vec![pair.clone()];
                    action = Some(CanvasAction::SelectClip(tr_id.clone(), cl_id.clone()));
                }

                state.drag_mode = mode;
                state.drag_clip_id = Some(cl_id.clone());
                state.drag_track_id = Some(tr_id.clone());
                state.drag_target_track_id = Some(tr_id.clone());
                state.drag_start_x = pos.x;

                if let Some(tr) = project.timeline.get_track_by_id(&tr_id) {
                    if let Some(cl) = tr.get_clip_by_id(&cl_id) {
                        state.drag_orig_in = cl.timeline_in;
                        state.drag_orig_out = cl.timeline_out;
                        state.drag_orig_sin = cl.source_in;
                        state.drag_orig_sout = cl.effective_source_out();
                    }
                }

                state.batch_orig_ins.clear();
                for (t_id, c_id) in &state.selected_clip_pairs {
                    if let Some(tr) = project.timeline.get_track_by_id(t_id) {
                        if let Some(cl) = tr.get_clip_by_id(c_id) {
                            state.batch_orig_ins.push((t_id.clone(), c_id.clone(), cl.timeline_in));
                        }
                    }
                }
            } else {
                // Clicked empty track area
                if is_multi {
                    state.drag_mode = DragMode::Rubberband;
                    state.rubberband_start = Some(pos);
                    state.rubberband_rect = Some(Rect::from_min_max(pos, pos));
                } else {
                    state.selected_clip_pairs.clear();
                    state.drag_mode = DragMode::Scrub;
                    let clicked_t = ((pos.x - rect.left()) / state.pixels_per_second).max(0.0) as f64;
                    let snapped_t = project.timeline.snap_time(clicked_t, Some(0.1), None);
                    action = Some(CanvasAction::Seek(snapped_t));
                }
            }
        }
    }

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
                                let cx = rect.left() + (cl.timeline_in as f32 * state.pixels_per_second);
                                let cw = (cl.duration() as f32 * state.pixels_per_second).max(4.0);
                                let cl_rect = Rect::from_min_size(Pos2::new(cx, ty), Vec2::new(cw, track_h));
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
                    let snapped = project.timeline.snap_time(t, Some(0.1), None);
                    action = Some(CanvasAction::Seek(snapped));
                }
                DragMode::Move => {
                    if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id) {
                        let target_in = (state.drag_orig_in + dt).max(0.0);
                        let snapped_in = project.timeline.snap_time(target_in, Some(0.1), Some(cl_id));
                        let actual_dt = snapped_in - state.drag_orig_in;

                        if (snapped_in - target_in).abs() > 1e-4 {
                            state.snap_guide_x = Some(rect.left() + (snapped_in as f32 * state.pixels_per_second));
                        } else {
                            state.snap_guide_x = None;
                        }

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
                            for (t_id, c_id, orig_in) in &state.batch_orig_ins {
                                if let Some(tr) = project.timeline.get_track_by_id_mut(t_id) {
                                    if let Some(cl) = tr.get_clip_by_id_mut(c_id) {
                                        cl.move_to((orig_in + actual_dt).max(0.0));
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
                    if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id) {
                        let target_in = (state.drag_orig_in + dt).max(0.0);
                        let snapped_in = project.timeline.snap_time(target_in, Some(0.1), Some(cl_id));
                        if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                cl.trim_in(snapped_in);
                            }
                        }
                    }
                }
                DragMode::TrimOut => {
                    if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id) {
                        let target_out = (state.drag_orig_out + dt).max(state.drag_orig_in + 0.04);
                        let snapped_out = project.timeline.snap_time(target_out, Some(0.1), Some(cl_id));
                        if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                cl.trim_out(snapped_out);
                            }
                        }
                    }
                }
                DragMode::FadeIn => {
                    if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id) {
                        if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                let raw = ((pos.x - (rect.left() + cl.timeline_in as f32 * state.pixels_per_second)) / state.pixels_per_second) as f64;
                                cl.fade_in = raw.clamp(0.0, cl.duration() / 2.0);
                                let tip_rect = Rect::from_center_size(Pos2::new(pos.x, pos.y - 18.0), Vec2::new(96.0, 20.0));
                                painter.rect_filled(tip_rect, 4.0, Color32::from_rgb(0x18, 0x18, 0x22));
                                painter.rect_stroke(tip_rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(0x38, 0xa0, 0xf0)));
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
                    if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id) {
                        if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                let cl_right = rect.left() + (cl.timeline_out as f32 * state.pixels_per_second);
                                let raw = ((cl_right - pos.x) / state.pixels_per_second) as f64;
                                cl.fade_out = raw.clamp(0.0, cl.duration() / 2.0);
                                let tip_rect = Rect::from_center_size(Pos2::new(pos.x, pos.y - 18.0), Vec2::new(96.0, 20.0));
                                painter.rect_filled(tip_rect, 4.0, Color32::from_rgb(0x18, 0x18, 0x22));
                                painter.rect_stroke(tip_rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(0x38, 0xa0, 0xf0)));
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
                DragMode::HandPan => {
                    let dx = pos.x - state.drag_start_x;
                    ui.scroll_with_delta(Vec2::new(dx, 0.0));
                    action = Some(CanvasAction::Pan(dx));
                    state.drag_start_x = pos.x;
                }
                _ => {}
            }
        }
    }

    // Release mouse
    if pointer.primary_released() && state.drag_mode != DragMode::None {
        match state.drag_mode {
            DragMode::Rubberband => {
                state.rubberband_rect = None;
                state.rubberband_start = None;
            }
            DragMode::Move => {
                state.snap_guide_x = None;
                if state.selected_clip_pairs.len() > 1 {
                    let mut moves = Vec::new();
                    for (t_id, c_id, orig_in) in &state.batch_orig_ins {
                        if let Some(tr) = project.timeline.get_track_by_id(t_id) {
                            if let Some(cl) = tr.get_clip_by_id(c_id) {
                                if (cl.timeline_in - orig_in).abs() > 1e-4 {
                                    moves.push((t_id.clone(), c_id.clone(), *orig_in, cl.timeline_in));
                                }
                            }
                        }
                    }
                    if !moves.is_empty() {
                        action = Some(CanvasAction::BatchClipsMoved(moves));
                    }
                } else if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id) {
                    let target_track_id = state.drag_target_track_id.clone().unwrap_or_else(|| tr_id.clone());
                    if let Some(tr) = project.timeline.get_track_by_id(tr_id) {
                        if let Some(cl) = tr.get_clip_by_id(cl_id) {
                            if (cl.timeline_in - state.drag_orig_in).abs() > 1e-4 || &target_track_id != tr_id {
                                action = Some(CanvasAction::ClipMoved {
                                    old_track_id: tr_id.clone(),
                                    new_track_id: target_track_id,
                                    clip_id: cl_id.clone(),
                                    old_in: state.drag_orig_in,
                                    new_in: cl.timeline_in,
                                });
                            }
                        }
                    }
                }
                state.drag_target_track_id = None;
            }
            DragMode::TrimIn | DragMode::TrimOut => {
                if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id) {
                    if let Some(tr) = project.timeline.get_track_by_id(tr_id) {
                        if let Some(cl) = tr.get_clip_by_id(cl_id) {
                            let new_bounds = (cl.timeline_in, cl.timeline_out, cl.source_in, cl.effective_source_out());
                            let old_bounds = (state.drag_orig_in, state.drag_orig_out, state.drag_orig_sin, state.drag_orig_sout);
                            if new_bounds != old_bounds {
                                action = Some(CanvasAction::ClipTrimmed {
                                    track_id: tr_id.clone(),
                                    clip_id: cl_id.clone(),
                                    old_bounds,
                                    new_bounds,
                                });
                            }
                        }
                    }
                }
            }
            DragMode::FadeIn | DragMode::FadeOut => {
                action = Some(CanvasAction::ClipModified);
            }
            _ => {}
        }

        state.drag_mode = DragMode::None;
        state.drag_clip_id = None;
        state.drag_track_id = None;
        state.snap_guide_x = None;
    }

    // Context Menu on right click
    response.context_menu(|ui| {
        if let Some((tr_id, cl_id)) = state.selected_clip_pairs.first().cloned() {
            let is_locked = project.timeline.get_track_by_id(&tr_id).map(|t| t.locked).unwrap_or(false);

            if is_locked {
                ui.label(RichText::new("[K] Kanal Kilitli (Düzenlenemez)").color(Color32::from_rgb(0xe0, 0x7a, 0x38)).italics());
                ui.separator();
            }

            ui.add_enabled_ui(!is_locked, |ui| {
                if ui.button("Hız ve Süre (Speed & Duration)...").clicked() {
                    action = Some(CanvasAction::PromptSpeed(tr_id.clone(), cl_id.clone()));
                    ui.close_menu();
                }

                if let Some(tr) = project.timeline.get_track_by_id(&tr_id) {
                    if tr.track_type == TrackType::Video
                        && ui.button("Sesi Ayır (Detach Audio)").clicked() {
                            action = Some(CanvasAction::DetachAudio(tr_id.clone(), cl_id.clone()));
                            ui.close_menu();
                        }
                }

                if ui.button("Klibi Böl (S)").clicked() {
                    action = Some(CanvasAction::SplitAtPlayhead);
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Sil (Del)").clicked() {
                    action = Some(CanvasAction::DeleteSelected);
                    ui.close_menu();
                }

                if ui.button("⏪ Boşluksuz Sil (Shift+Del)").clicked() {
                    action = Some(CanvasAction::RippleDeleteSelected);
                    ui.close_menu();
                }
            });
        }
    });

    (response, action)
}

fn find_clip_at_pos(
    project: &Project,
    state: &TimelineCanvasState,
    canvas_rect: Rect,
    pos: Pos2,
) -> Option<(String, String, DragMode)> {
    let track_offsets = state.calculate_track_offsets(&project.timeline.tracks, canvas_rect.top());
    for (t_idx, track) in project.timeline.tracks.iter().enumerate() {
        let (ty, track_h) = track_offsets[t_idx];
        let track_rect = Rect::from_min_size(Pos2::new(canvas_rect.left(), ty), Vec2::new(canvas_rect.width(), track_h));

        if !track_rect.contains(pos) {
            continue;
        }

        for clip in &track.clips {
            let cx = canvas_rect.left() + (clip.timeline_in as f32 * state.pixels_per_second);
            let cw = (clip.duration() as f32 * state.pixels_per_second).max(4.0);
            let clip_rect = Rect::from_min_size(Pos2::new(cx, ty), Vec2::new(cw, track_h));

            if clip_rect.contains(pos) {
                if track.locked {
                    return Some((track.id.clone(), clip.id.clone(), DragMode::None));
                }

                // Check fade handles (prioritized in the top 18px of the clip)
                let fi_w = (clip.fade_in as f32 * state.pixels_per_second).min(clip_rect.width() * 0.5);
                let fi_x = clip_rect.left() + if clip.fade_in > 0.0 { fi_w } else { 7.0 };
                let fo_w = (clip.fade_out as f32 * state.pixels_per_second).min(clip_rect.width() * 0.5);
                let fo_x = clip_rect.right() - if clip.fade_out > 0.0 { fo_w } else { 7.0 };

                if pos.y <= clip_rect.top() + 18.0 {
                    if (pos.x - fi_x).abs() <= 10.0 {
                        return Some((track.id.clone(), clip.id.clone(), DragMode::FadeIn));
                    }
                    if (pos.x - fo_x).abs() <= 10.0 {
                        return Some((track.id.clone(), clip.id.clone(), DragMode::FadeOut));
                    }
                }

                // Check trim handles (outside or below fade handles)
                if pos.x <= clip_rect.left() + state.trim_handle_width {
                    return Some((track.id.clone(), clip.id.clone(), DragMode::TrimIn));
                }
                if pos.x >= clip_rect.right() - state.trim_handle_width {
                    return Some((track.id.clone(), clip.id.clone(), DragMode::TrimOut));
                }

                return Some((track.id.clone(), clip.id.clone(), DragMode::Move));
            }
        }

        return None;
    }
    None
}
