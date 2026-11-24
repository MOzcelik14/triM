use egui::{Color32, Response, Sense, Ui, Vec2};
use std::collections::HashSet;
use trim_core::project::Project;
use trim_media::WaveformGenerator;

use super::geometry::*;
use super::state::*;

use super::input_drag::handle_active_drag;
use super::input_press::handle_primary_press;
use super::input_release::handle_release_and_context_menu;
use super::overlays::render_overlays_and_cursor;
use super::tracks::render_tracks_and_clips;

pub fn render_timeline_canvas(
    ui: &mut Ui,
    project: &mut Project,
    state: &mut TimelineCanvasState,
    current_time: f64,
    waveform_gen: &WaveformGenerator,
    textures: &std::collections::HashMap<String, egui::TextureHandle>,
) -> (Response, Option<CanvasAction>) {
    let min_sec = (project.timeline.duration() + 30.0).max(60.0) as f32;
    let canvas_width = ui.available_width().max(min_sec * state.pixels_per_second);
    let canvas_height = if project.timeline.tracks.is_empty() {
        state.track_height
    } else {
        project
            .timeline
            .tracks
            .iter()
            .map(|t| state.get_track_height(&t.id) + state.track_spacing)
            .sum::<f32>()
    };

    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(canvas_width, canvas_height),
        Sense::click_and_drag(),
    );
    let painter = ui.painter_at(rect);

    // Canvas background
    painter.rect_filled(rect, 0.0, Color32::from_rgb(0x16, 0x16, 0x1a));

    let mut action = None;
    let selected_ids: HashSet<String> = state
        .selected_clip_pairs
        .iter()
        .map(|(_, c)| c.clone())
        .collect();
    let track_offsets = state.calculate_track_offsets(&project.timeline.tracks, rect.top());

    let view_clip_rect = ui.clip_rect();

    render_tracks_and_clips(
        ui,
        &painter,
        rect,
        view_clip_rect,
        project,
        state,
        &track_offsets,
        &selected_ids,
        waveform_gen,
        textures,
    );

    let pointer = ui.input(|i| i.pointer.clone());
    let snap_thresh = compute_snap_threshold_secs(state.pixels_per_second);

    render_overlays_and_cursor(
        ui,
        &painter,
        &response,
        rect,
        project,
        state,
        &track_offsets,
        current_time,
        &pointer,
        snap_thresh,
        &mut action,
    );

    if handle_primary_press(
        ui,
        &response,
        rect,
        project,
        state,
        &track_offsets,
        current_time,
        &pointer,
        snap_thresh,
        &mut action,
    ) {
        return (response, action);
    }

    handle_active_drag(
        ui,
        &painter,
        rect,
        project,
        state,
        &track_offsets,
        current_time,
        &pointer,
        snap_thresh,
        &mut action,
    );

    handle_release_and_context_menu(&response, rect, project, state, &pointer, &mut action);

    (response, action)
}
