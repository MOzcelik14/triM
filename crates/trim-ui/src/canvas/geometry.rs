use super::state::*;
use super::state::{TimelineCanvasState, SNAP_THRESHOLD_PX};
use crate::toolbar::EditTool;
use egui::{Pos2, Rect, Vec2};
use trim_core::project::Project;
use trim_core::track::TrackType;

/// Calculates total height required by all tracks including spacing.
pub fn calculate_total_tracks_height(
    tracks: &[trim_core::track::Track],
    canvas_state: &TimelineCanvasState,
) -> f32 {
    if tracks.is_empty() {
        canvas_state.track_height
    } else {
        tracks
            .iter()
            .map(|t| canvas_state.get_track_height(&t.id) + canvas_state.track_spacing)
            .sum::<f32>()
    }
}

/// Clamps vertical scroll offset within valid bounds `[0.0, max_scroll]`.
pub fn clamp_scroll_offset(offset_y: f32, content_height: f32, viewport_height: f32) -> f32 {
    let max_scroll = (content_height - viewport_height).max(0.0);
    offset_y.clamp(0.0, max_scroll)
}

/// Checks if vertical scrolling is needed for the given content and viewport heights.
pub fn is_vertical_scroll_needed(content_height: f32, viewport_height: f32) -> bool {
    content_height > viewport_height
}

/// Converts timeline seconds to an X pixel coordinate relative to `rect_left`.
pub fn time_to_x(rect_left: f32, time_secs: f64, pixels_per_second: f32) -> f32 {
    rect_left + (time_secs.max(0.0) as f32 * pixels_per_second.max(1.0))
}

/// Converts an X pixel coordinate to non-negative timeline seconds relative to `rect_left`.
pub fn x_to_time(rect_left: f32, pos_x: f32, pixels_per_second: f32) -> f64 {
    ((pos_x - rect_left) / pixels_per_second.max(1.0)).max(0.0) as f64
}

/// Computes magnetic snap threshold in seconds from pixel zoom (`SNAP_THRESHOLD_PX`),
/// clamped to `[0.02, 0.25]` seconds.
pub fn compute_snap_threshold_secs(pixels_per_second: f32) -> f64 {
    ((SNAP_THRESHOLD_PX / pixels_per_second.max(1.0)) as f64).clamp(0.02, 0.25)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipHitZone {
    LeftEdge,
    Body,
    RightEdge,
}

/// Hit-tests a horizontal pixel coordinate against a clip's `[clip_left, clip_right]` bounds.
pub fn hit_test_clip_horizontal(
    pos_x: f32,
    clip_left: f32,
    clip_right: f32,
    edge_handle_px: f32,
) -> Option<ClipHitZone> {
    if pos_x < clip_left || pos_x > clip_right || clip_right <= clip_left {
        return None;
    }
    let handle = edge_handle_px.min((clip_right - clip_left) * 0.35);
    if pos_x <= clip_left + handle {
        Some(ClipHitZone::LeftEdge)
    } else if pos_x >= clip_right - handle {
        Some(ClipHitZone::RightEdge)
    } else {
        Some(ClipHitZone::Body)
    }
}

pub fn find_gap_at_pos(
    project: &Project,
    state: &TimelineCanvasState,
    canvas_rect: Rect,
    pos: Pos2,
) -> Option<(String, f64, f64)> {
    let track_offsets = state.calculate_track_offsets(&project.timeline.tracks, canvas_rect.top());
    for (t_idx, track) in project.timeline.tracks.iter().enumerate() {
        let (ty, track_h) = track_offsets[t_idx];
        let track_rect = Rect::from_min_size(
            Pos2::new(canvas_rect.left(), ty),
            Vec2::new(canvas_rect.width(), track_h),
        );
        if !track_rect.contains(pos) {
            continue;
        }

        let click_time = ((pos.x - canvas_rect.left()) / state.pixels_per_second) as f64;
        if click_time < 0.0 {
            return None;
        }

        let mut prev_out = 0.0;
        for clip in &track.clips {
            if clip.timeline_in - prev_out > 0.04
                && click_time >= prev_out
                && click_time <= clip.timeline_in
            {
                return Some((track.id.clone(), prev_out, clip.timeline_in));
            }
            prev_out = prev_out.max(clip.timeline_out);
        }
        return None;
    }
    None
}

pub(crate) fn find_clip_at_pos(
    project: &Project,
    state: &TimelineCanvasState,
    canvas_rect: Rect,
    pos: Pos2,
) -> Option<(String, String, DragMode)> {
    let track_offsets = state.calculate_track_offsets(&project.timeline.tracks, canvas_rect.top());
    for (t_idx, track) in project.timeline.tracks.iter().enumerate() {
        let (ty, track_h) = track_offsets[t_idx];
        let track_rect = Rect::from_min_size(
            Pos2::new(canvas_rect.left(), ty),
            Vec2::new(canvas_rect.width(), track_h),
        );

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

                if state.active_tool == EditTool::Razor {
                    return Some((track.id.clone(), clip.id.clone(), DragMode::None));
                }

                if state.active_tool == EditTool::Slip {
                    return Some((track.id.clone(), clip.id.clone(), DragMode::Slip));
                }
                if state.active_tool == EditTool::Roll {
                    return Some((track.id.clone(), clip.id.clone(), DragMode::Roll));
                }
                if state.active_tool == EditTool::Slide {
                    return Some((track.id.clone(), clip.id.clone(), DragMode::Slide));
                }

                if state.active_tool == EditTool::Ripple {
                    if pos.x < clip_rect.center().x {
                        return Some((track.id.clone(), clip.id.clone(), DragMode::RippleIn));
                    } else {
                        return Some((track.id.clone(), clip.id.clone(), DragMode::RippleOut));
                    }
                }

                // Check fade handles (prioritized in the top 18px of the clip)
                let fi_w =
                    (clip.fade_in as f32 * state.pixels_per_second).min(clip_rect.width() * 0.5);
                let fi_x = clip_rect.left() + if clip.fade_in > 0.0 { fi_w } else { 7.0 };
                let fo_w =
                    (clip.fade_out as f32 * state.pixels_per_second).min(clip_rect.width() * 0.5);
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

                // Check volume rubberband on Audio tracks
                if track.track_type == TrackType::Audio && clip_rect.width() >= 12.0 {
                    let active_top = clip_rect.top() + 18.0;
                    let active_bottom = clip_rect.bottom() - 6.0;
                    let range = (active_bottom - active_top).max(8.0);
                    let dur = clip.duration().max(0.04);
                    let rel_frac = ((pos.x - (clip_rect.left() + 2.0)) / (clip_rect.width() - 4.0))
                        .clamp(0.0, 1.0) as f64;
                    let rel_t = (rel_frac * dur).clamp(0.0, dur);

                    let is_pen = state.active_tool == EditTool::Pen;
                    let hit_threshold = if is_pen { 12.0 } else { 6.0 };

                    // Check if close to any keyframe diamond handle
                    let mut hit_kf = false;
                    if let Some(kfs) = clip.keyframes.get("volume") {
                        for kf in kfs {
                            let kf_frac = (kf.time / dur).clamp(0.0, 1.0) as f32;
                            let kf_x = clip_rect.left() + 2.0 + kf_frac * (clip_rect.width() - 4.0);
                            let kf_norm = (kf.value / 2.0).clamp(0.0, 1.0) as f32;
                            let kf_y = active_bottom - (kf_norm * range);
                            if pos.distance(Pos2::new(kf_x, kf_y)) <= 8.0 {
                                hit_kf = true;
                                break;
                            }
                        }
                    }

                    if hit_kf {
                        return Some((
                            track.id.clone(),
                            clip.id.clone(),
                            DragMode::VolumeRubberband,
                        ));
                    }

                    let vol = if clip.keyframes.get("volume").is_some_and(|k| !k.is_empty()) {
                        clip.get_property_at_time("volume", rel_t)
                    } else {
                        clip.volume
                    };
                    let norm = (vol / 2.0).clamp(0.0, 1.0) as f32;
                    let vol_y = active_bottom - (norm * range);

                    if (pos.y - vol_y).abs() <= hit_threshold {
                        return Some((
                            track.id.clone(),
                            clip.id.clone(),
                            DragMode::VolumeRubberband,
                        ));
                    }
                }

                return Some((track.id.clone(), clip.id.clone(), DragMode::Move));
            }
        }

        return None;
    }
    None
}
