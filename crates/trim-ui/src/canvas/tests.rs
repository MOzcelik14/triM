use super::*;
use egui::{Pos2, Rect, Vec2};
use trim_core::project::Project;
use trim_core::track::{Track, TrackType};

#[test]
fn test_calculate_total_tracks_height_empty() {
    let state = TimelineCanvasState::default();
    let tracks: Vec<Track> = Vec::new();
    assert_eq!(
        calculate_total_tracks_height(&tracks, &state),
        state.track_height
    );
}

#[test]
fn test_calculate_total_tracks_height_with_custom_heights() {
    let mut state = TimelineCanvasState::default();
    let mut t1 = Track::new("V1", TrackType::Video);
    let mut t2 = Track::new("A1", TrackType::Audio);
    t1.id = "t1".into();
    t2.id = "t2".into();
    state.set_track_height("t1", 80.0);
    // t2 uses default track_height (56.0)

    let tracks = vec![t1, t2];
    let total = calculate_total_tracks_height(&tracks, &state);
    // (80.0 + 6.0) + (56.0 + 6.0) = 148.0
    assert_eq!(total, 148.0);
}

#[test]
fn test_clamp_scroll_offset() {
    // Content smaller than viewport: max_scroll = 0.0
    assert_eq!(clamp_scroll_offset(50.0, 200.0, 300.0), 0.0);
    assert_eq!(clamp_scroll_offset(-10.0, 200.0, 300.0), 0.0);

    // Content larger than viewport: max_scroll = 500.0 - 300.0 = 200.0
    assert_eq!(clamp_scroll_offset(-20.0, 500.0, 300.0), 0.0);
    assert_eq!(clamp_scroll_offset(100.0, 500.0, 300.0), 100.0);
    assert_eq!(clamp_scroll_offset(350.0, 500.0, 300.0), 200.0);
}

#[test]
fn test_is_vertical_scroll_needed() {
    assert!(!is_vertical_scroll_needed(200.0, 300.0));
    assert!(!is_vertical_scroll_needed(300.0, 300.0));
    assert!(is_vertical_scroll_needed(301.0, 300.0));
}

#[test]
fn test_time_pixel_conversion_and_clamping() {
    let left = 120.0_f32;
    for pps in [10.0_f32, 60.0_f32, 240.0_f32] {
        let x = time_to_x(left, 5.5, pps);
        assert!((x - (left + 5.5 * pps)).abs() < 1e-4);
        let t = x_to_time(left, x, pps);
        assert!((t - 5.5).abs() < 1e-5);
    }
    // Negative X before rect_left clamps to 0.0s
    assert_eq!(x_to_time(left, 40.0, 60.0), 0.0);
    assert_eq!(time_to_x(left, -3.0, 60.0), left);
}

#[test]
fn test_compute_snap_threshold_secs_bounds() {
    // Very zoomed out (10 px/s -> 10/10 = 1.0s, clamped to 0.25s)
    assert!((compute_snap_threshold_secs(10.0) - 0.25).abs() < 1e-6);
    // Default zoom (60 px/s -> 10/60 = 0.1667s)
    assert!((compute_snap_threshold_secs(60.0) - (10.0 / 60.0)).abs() < 1e-5);
    // Very zoomed in (1000 px/s -> 10/1000 = 0.01s, clamped to 0.02s)
    assert!((compute_snap_threshold_secs(1000.0) - 0.02).abs() < 1e-6);
}

#[test]
fn test_hit_test_clip_horizontal() {
    assert_eq!(hit_test_clip_horizontal(99.0, 100.0, 200.0, 8.0), None);
    assert_eq!(hit_test_clip_horizontal(201.0, 100.0, 200.0, 8.0), None);
    assert_eq!(
        hit_test_clip_horizontal(104.0, 100.0, 200.0, 8.0),
        Some(ClipHitZone::LeftEdge)
    );
    assert_eq!(
        hit_test_clip_horizontal(150.0, 100.0, 200.0, 8.0),
        Some(ClipHitZone::Body)
    );
    assert_eq!(
        hit_test_clip_horizontal(196.0, 100.0, 200.0, 8.0),
        Some(ClipHitZone::RightEdge)
    );
}

#[test]
fn test_find_gap_at_pos_detects_gap_and_ignores_clips() {
    let mut proj = Project::new("Gap Hit");
    proj.timeline.tracks.clear();
    let mut tr = Track::new("V1", TrackType::Video);
    tr.id = "v1".into();
    let mut c1 = trim_core::clip::Clip::new("m1", 0.0, 2.0);
    c1.timeline_in = 0.0;
    c1.timeline_out = 2.0;
    let mut c2 = trim_core::clip::Clip::new("m2", 0.0, 2.0);
    c2.timeline_in = 5.0;
    c2.timeline_out = 7.0;
    tr.add_clip(c1);
    tr.add_clip(c2);
    proj.timeline.tracks.push(tr);

    let state = TimelineCanvasState::default(); // pps=60, track_height=56, TRACK_TOP_PADDING=8
    let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(800.0, 200.0));
    // Inside track 1 Y (e.g. y=30), gap is [2.0s, 5.0s] -> x in [120, 300]
    let gap = find_gap_at_pos(&proj, &state, rect, Pos2::new(210.0, 30.0));
    assert_eq!(gap, Some(("v1".to_string(), 2.0, 5.0)));

    // Clicking on clip c1 (x=60, t=1.0s) returns None
    assert_eq!(
        find_gap_at_pos(&proj, &state, rect, Pos2::new(60.0, 30.0)),
        None
    );
    // Clicking outside track vertically returns None
    assert_eq!(
        find_gap_at_pos(&proj, &state, rect, Pos2::new(210.0, 180.0)),
        None
    );
}
