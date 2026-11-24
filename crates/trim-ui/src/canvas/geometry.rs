use super::state::{TimelineCanvasState, SNAP_THRESHOLD_PX};
use egui::{Pos2, Rect, Vec2};
use trim_core::project::Project;

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
