//! Interactive timeline canvas rendering tracks, clips, waveforms, fades, keyframes, playhead and editing gestures.

use crate::theme::*;
use crate::toolbar::EditTool;
use egui::{Color32, Pos2, Rect, Response, RichText, Sense, Stroke, Ui, Vec2};
use std::collections::HashSet;
use trim_core::clip::ColorLabel;
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
    VolumeRubberband,
    Scrub,
    Rubberband,
    HandPan,
    Slip,
}

pub const TRACK_TOP_PADDING: f32 = 0.0;
pub const SNAP_THRESHOLD_PX: f32 = 10.0;

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
    pub drag_orig_vol: f64,
    pub batch_orig_ins: Vec<(String, String, f64)>, // (track_id, clip_id, orig_in)

    pub rubberband_start: Option<Pos2>,
    pub rubberband_rect: Option<Rect>,
    pub snap_guide_x: Option<f32>,
    pub drag_target_track_id: Option<String>,
    pub track_heights: std::collections::HashMap<String, f32>,
    pub selected_gap: Option<(String, f64, f64)>, // (track_id, gap_start, gap_end)
    pub scroll_offset: Vec2,
    pub pending_scroll_y: Option<f32>,
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
            drag_orig_vol: 1.0,
            batch_orig_ins: Vec::new(),
            rubberband_start: None,
            rubberband_rect: None,
            snap_guide_x: None,
            drag_target_track_id: None,
            track_heights: std::collections::HashMap::new(),
            selected_gap: None,
            scroll_offset: Vec2::ZERO,
            pending_scroll_y: None,
        }
    }
}

impl TimelineCanvasState {
    pub fn get_track_height(&self, track_id: &str) -> f32 {
        self.track_heights
            .get(track_id)
            .copied()
            .unwrap_or(self.track_height)
    }

    pub fn set_track_height(&mut self, track_id: &str, height: f32) {
        self.track_heights
            .insert(track_id.to_string(), height.clamp(32.0, 160.0));
    }

    pub fn calculate_track_offsets(
        &self,
        tracks: &[trim_core::track::Track],
        top: f32,
    ) -> Vec<(f32, f32)> {
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

pub enum CanvasAction {
    Seek(f64),
    SelectClip(String, String),
    SelectClips(Vec<(String, String)>),
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
    CloseGap {
        track_id: Option<String>,
        gap_start: f64,
        gap_end: f64,
    },
    SetClipColor {
        track_id: String,
        clip_id: String,
        color: trim_core::clip::ColorLabel,
    },
    SetClipVolume {
        track_id: String,
        clip_id: String,
        old_volume: f64,
        new_volume: f64,
    },
    ToggleLinkSelected,
}

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

    // 1. Draw track lanes
    for (t_idx, track) in project.timeline.tracks.iter().enumerate() {
        let (ty, track_h) = track_offsets[t_idx];
        let track_rect =
            Rect::from_min_size(Pos2::new(rect.left(), ty), Vec2::new(rect.width(), track_h));

        let lane_color = if t_idx % 2 == 0 {
            Color32::from_rgb(0x1c, 0x1c, 0x22)
        } else {
            Color32::from_rgb(0x20, 0x20, 0x26)
        };
        painter.rect_filled(track_rect, 0.0, lane_color);
        painter.line_segment(
            [
                Pos2::new(track_rect.left(), track_rect.bottom()),
                Pos2::new(track_rect.right(), track_rect.bottom()),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(0x2a, 0x2a, 0x32)),
        );

        // Draw gaps on this track
        let mut prev_t = 0.0;
        for clip in &track.clips {
            if clip.timeline_in - prev_t > 0.04 {
                let g_in = prev_t;
                let g_out = clip.timeline_in;
                let gx = rect.left() + (g_in as f32 * state.pixels_per_second);
                let gw = ((g_out - g_in) as f32 * state.pixels_per_second).max(2.0);
                let gap_rect = Rect::from_min_size(Pos2::new(gx, ty), Vec2::new(gw, track_h));

                let is_gap_sel =
                    state.selected_gap.as_ref() == Some(&(track.id.clone(), g_in, g_out));
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

        // Draw clips on this track
        for clip in &track.clips {
            let cx = rect.left() + (clip.timeline_in as f32 * state.pixels_per_second);
            let cw = (clip.duration() as f32 * state.pixels_per_second).max(4.0);
            let clip_rect = Rect::from_min_size(Pos2::new(cx, ty), Vec2::new(cw, track_h));

            let is_sel = selected_ids.contains(&clip.id);

            // Clip body fill
            let base_color = if let Some([r, g, b]) = clip.color_label.color_rgb() {
                if is_sel {
                    Color32::from_rgb(
                        (r as f32 * 1.15).min(255.0) as u8,
                        (g as f32 * 1.15).min(255.0) as u8,
                        (b as f32 * 1.15).min(255.0) as u8,
                    )
                } else {
                    Color32::from_rgb(
                        (r as f32 * 0.70) as u8,
                        (g as f32 * 0.70) as u8,
                        (b as f32 * 0.70) as u8,
                    )
                }
            } else if clip.text_overlay.is_some() {
                if is_sel {
                    Color32::from_rgb(0x8a, 0x48, 0xa5)
                } else {
                    Color32::from_rgb(0x6a, 0x32, 0x82)
                }
            } else {
                match track.track_type {
                    TrackType::Video => {
                        if is_sel {
                            COLOR_CLIP_VIDEO_SELECTED
                        } else {
                            COLOR_CLIP_VIDEO
                        }
                    }
                    TrackType::Audio => {
                        if is_sel {
                            COLOR_CLIP_AUDIO_SELECTED
                        } else {
                            COLOR_CLIP_AUDIO
                        }
                    }
                }
            };
            painter.rect_filled(clip_rect, 4.0, base_color);

            // Colored top accent stripe if label is set
            if let Some([r, g, b]) = clip.color_label.color_rgb() {
                let stripe_rect =
                    Rect::from_min_size(clip_rect.left_top(), Vec2::new(clip_rect.width(), 3.5));
                painter.rect_filled(stripe_rect, 2.0, Color32::from_rgb(r, g, b));
            }

            // Video Filmstrip Thumbnails (Video tracks only)
            if track.track_type == TrackType::Video && clip_rect.width() >= 28.0 {
                if let Some(tex) = textures.get(&clip.media_id) {
                    let body_top = clip_rect.top() + 18.0;
                    let body_bottom = clip_rect.bottom() - 2.0;
                    if body_bottom > body_top + 8.0 {
                        let film_h = body_bottom - body_top;
                        let film_w = (film_h * 1.777_f32).clamp(16.0, 100.0);
                        let total_w = clip_rect.width() - 4.0;
                        let num_frames = ((total_w / film_w).floor() as usize).max(1);

                        let mut frame_x = clip_rect.left() + 2.0;
                        for i in 0..num_frames {
                            let curr_w = if i == num_frames - 1 {
                                (clip_rect.right() - 2.0 - frame_x).min(film_w).max(2.0)
                            } else {
                                film_w
                            };
                            let frame_rect = Rect::from_min_size(
                                Pos2::new(frame_x, body_top),
                                Vec2::new(curr_w, film_h),
                            );

                            let uv_max_x = (curr_w / film_w).clamp(0.0, 1.0);
                            let uv =
                                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(uv_max_x, 1.0));

                            painter.image(
                                tex.id(),
                                frame_rect,
                                uv,
                                Color32::from_rgba_premultiplied(225, 225, 235, 210),
                            );

                            if i > 0 {
                                painter.line_segment(
                                    [frame_rect.left_top(), frame_rect.left_bottom()],
                                    Stroke::new(
                                        1.0_f32,
                                        Color32::from_rgba_premultiplied(0, 0, 0, 180),
                                    ),
                                );
                            }

                            frame_x += film_w;
                            if frame_x >= clip_rect.right() - 3.0 {
                                break;
                            }
                        }
                    }
                }
            }

            // Waveform (Audio tracks only)
            if track.track_type == TrackType::Audio {
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
                            let wf_col = Color32::from_rgba_premultiplied(0x64, 0xe6, 0xa0, 0xb4);

                            let vis_left = (rect.left() - clip_rect.left()).max(0.0);
                            let vis_right =
                                (rect.right() - clip_rect.left()).min(clip_rect.width());

                            if vis_right > vis_left {
                                let mut px = vis_left;
                                let mut wf_shapes = Vec::with_capacity(
                                    ((vis_right - vis_left) / 2.0).ceil() as usize,
                                );
                                while px < vis_right {
                                    let norm = px / clip_rect.width();
                                    let idx = (norm as f64 * slice.len() as f64) as usize;
                                    if idx < slice.len() {
                                        let [v_min, v_max] = slice[idx];
                                        let x_pos = clip_rect.left() + px;
                                        let y1 = cy + v_min * max_amp;
                                        let y2 = cy + v_max * max_amp;
                                        wf_shapes.push(egui::Shape::line_segment(
                                            [Pos2::new(x_pos, y1), Pos2::new(x_pos, y2)],
                                            Stroke::new(1.0_f32, wf_col),
                                        ));
                                    }
                                    px += 2.0;
                                }
                                painter.extend(wf_shapes);
                            }
                        }
                    }
                }

                // Volume Rubberband (Audio tracks only)
                if clip_rect.width() >= 12.0 {
                    let norm = (clip.volume / 2.0).clamp(0.0, 1.0) as f32;
                    let active_top = clip_rect.top() + 18.0;
                    let active_bottom = clip_rect.bottom() - 6.0;
                    let range = (active_bottom - active_top).max(8.0);
                    let vol_y = active_bottom - (norm * range);

                    let line_color = if is_sel {
                        Color32::from_rgba_premultiplied(0xf5, 0x9e, 0x0b, 220)
                    } else {
                        Color32::from_rgba_premultiplied(0x38, 0xa0, 0xf0, 190)
                    };
                    painter.line_segment(
                        [
                            Pos2::new(clip_rect.left() + 2.0, vol_y),
                            Pos2::new(clip_rect.right() - 2.0, vol_y),
                        ],
                        Stroke::new(1.5_f32, line_color),
                    );

                    let hover_p = ui.input(|i| i.pointer.hover_pos());
                    let is_hovered = hover_p
                        .map(|p| clip_rect.contains(p) && (p.y - vol_y).abs() <= 5.0)
                        .unwrap_or(false);

                    let is_active_vol = state.drag_mode == DragMode::VolumeRubberband
                        && state.drag_clip_id.as_deref() == Some(&clip.id);

                    let node_x = if is_active_vol || is_hovered {
                        hover_p
                            .map(|p| p.x)
                            .unwrap_or(clip_rect.center().x)
                            .clamp(clip_rect.left() + 8.0, clip_rect.right() - 8.0)
                    } else {
                        clip_rect.center().x
                    };

                    let node_r = if is_hovered || is_active_vol {
                        4.5
                    } else {
                        3.0
                    };
                    let node_col = if is_hovered || is_active_vol {
                        Color32::WHITE
                    } else {
                        line_color
                    };
                    painter.circle_filled(Pos2::new(node_x, vol_y), node_r, node_col);
                    painter.circle_stroke(
                        Pos2::new(node_x, vol_y),
                        node_r,
                        Stroke::new(1.0_f32, Color32::from_rgb(0x18, 0x18, 0x24)),
                    );
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
                    [
                        clip_rect.left_bottom(),
                        Pos2::new(clip_rect.left() + fi_w, clip_rect.top()),
                    ],
                    Stroke::new(
                        1.5_f32,
                        Color32::from_rgba_premultiplied(255, 255, 255, 190),
                    ),
                );
            }
            let fi_handle_pos = Pos2::new(
                clip_rect.left() + if clip.fade_in > 0.0 { fi_w } else { 7.0 },
                clip_rect.top() + 7.0,
            );
            let fi_hovered = ui
                .input(|i| i.pointer.hover_pos())
                .map(|p| p.distance(fi_handle_pos) <= 8.0)
                .unwrap_or(false);
            let fi_radius = if fi_hovered { 5.5 } else { 3.5 };
            let fi_color = if fi_hovered {
                Color32::from_rgb(0x38, 0xa0, 0xf0)
            } else if clip.fade_in > 0.0 {
                Color32::WHITE
            } else {
                Color32::from_rgb(0xd0, 0xd0, 0xd8)
            };
            painter.circle_filled(fi_handle_pos, fi_radius, fi_color);
            painter.circle_stroke(
                fi_handle_pos,
                fi_radius,
                Stroke::new(1.0_f32, Color32::from_rgb(0x1a, 0x1a, 0x22)),
            );

            // Fade Out wedge overlay & interactive handle dot
            let fo_w =
                (clip.fade_out as f32 * state.pixels_per_second).min(clip_rect.width() * 0.5);
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
                    [
                        Pos2::new(clip_rect.right() - fo_w, clip_rect.top()),
                        clip_rect.right_bottom(),
                    ],
                    Stroke::new(
                        1.5_f32,
                        Color32::from_rgba_premultiplied(255, 255, 255, 190),
                    ),
                );
            }
            let fo_handle_pos = Pos2::new(
                clip_rect.right() - if clip.fade_out > 0.0 { fo_w } else { 7.0 },
                clip_rect.top() + 7.0,
            );
            let fo_hovered = ui
                .input(|i| i.pointer.hover_pos())
                .map(|p| p.distance(fo_handle_pos) <= 8.0)
                .unwrap_or(false);
            let fo_radius = if fo_hovered { 5.5 } else { 3.5 };
            let fo_color = if fo_hovered {
                Color32::from_rgb(0x38, 0xa0, 0xf0)
            } else if clip.fade_out > 0.0 {
                Color32::WHITE
            } else {
                Color32::from_rgb(0xd0, 0xd0, 0xd8)
            };
            painter.circle_filled(fo_handle_pos, fo_radius, fo_color);
            painter.circle_stroke(
                fo_handle_pos,
                fo_radius,
                Stroke::new(1.0_f32, Color32::from_rgb(0x1a, 0x1a, 0x22)),
            );

            // Clip border
            let border_stroke = if is_sel {
                Stroke::new(2.0_f32, COLOR_AMBER)
            } else {
                Stroke::new(1.0_f32, Color32::from_rgb(0x38, 0x38, 0x48))
            };
            painter.rect_stroke(clip_rect, 4.0, border_stroke);

            // Trim handles cues
            painter.rect_filled(
                Rect::from_min_size(
                    clip_rect.left_top(),
                    Vec2::new(state.trim_handle_width, clip_rect.height()),
                ),
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

            // Visual transition bands (Cross dissolve, dip, slide etc.)
            if clip.transition_in.is_some() {
                let trans_w = ((clip.fade_in.max(0.5) as f32) * state.pixels_per_second)
                    .clamp(8.0, (clip_rect.width() * 0.45).max(8.0));
                let trans_rect = Rect::from_min_size(
                    clip_rect.left_top(),
                    Vec2::new(trans_w, clip_rect.height()),
                );
                painter.rect_filled(
                    trans_rect,
                    2.0,
                    Color32::from_rgba_premultiplied(0x9b, 0x59, 0xb6, 55),
                );
                painter.line_segment(
                    [trans_rect.left_bottom(), trans_rect.right_top()],
                    Stroke::new(1.0_f32, Color32::from_rgb(0xc3, 0x9b, 0xd3)),
                );
            }
            if clip.transition_out.is_some() {
                let trans_w = ((clip.fade_out.max(0.5) as f32) * state.pixels_per_second)
                    .clamp(8.0, (clip_rect.width() * 0.45).max(8.0));
                let trans_rect = Rect::from_min_size(
                    Pos2::new(clip_rect.right() - trans_w, clip_rect.top()),
                    Vec2::new(trans_w, clip_rect.height()),
                );
                painter.rect_filled(
                    trans_rect,
                    2.0,
                    Color32::from_rgba_premultiplied(0x9b, 0x59, 0xb6, 55),
                );
                painter.line_segment(
                    [trans_rect.left_top(), trans_rect.right_bottom()],
                    Stroke::new(1.0_f32, Color32::from_rgb(0xc3, 0x9b, 0xd3)),
                );
            }

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
            let link_badge = if clip.linked_group_id.is_some() {
                " 🔗"
            } else {
                ""
            };
            let trans_badge = match (
                clip.transition_in.as_deref(),
                clip.transition_out.as_deref(),
            ) {
                (Some(_), Some(_)) => " ⧗⧗",
                (Some(_), None) => " ⧗-",
                (None, Some(_)) => " -⧗",
                (None, None) => "",
            };
            let title_prefix = if clip.text_overlay.is_some() {
                "🅃 "
            } else {
                ""
            };
            let title_str = format!(
                "{}{}{}{}{}{}  [{:.2}s]{}",
                title_prefix,
                clip.name,
                lock_badge,
                link_badge,
                trans_badge,
                kf_badge,
                clip.duration(),
                speed_badge
            );

            let header_rect = Rect::from_min_size(
                Pos2::new(clip_rect.left() + 2.0, clip_rect.top() + 2.0),
                Vec2::new((clip_rect.width() - 4.0).max(4.0), 16.0),
            );
            painter.rect_filled(
                header_rect,
                2.0,
                Color32::from_rgba_premultiplied(16, 18, 24, 150),
            );

            painter.text(
                Pos2::new(clip_rect.left() + 10.0, clip_rect.top() + 4.0),
                egui::Align2::LEFT_TOP,
                title_str,
                egui::FontId::proportional(11.0),
                Color32::WHITE,
            );

            // Sync offset badge for linked clips
            if let Some(ref gid) = clip.linked_group_id {
                let companion_in = project
                    .timeline
                    .tracks
                    .iter()
                    .filter(|t| t.id != track.id)
                    .find_map(|t| {
                        t.clips
                            .iter()
                            .find(|c| c.linked_group_id.as_ref() == Some(gid))
                    })
                    .map(|c| c.timeline_in);

                if let Some(comp_in) = companion_in {
                    let sync_diff = clip.timeline_in - comp_in;
                    if sync_diff.abs() > 0.03 && clip_rect.width() > 65.0 {
                        let sync_str = format!("{:+0.2}s", sync_diff);
                        let badge_rect = Rect::from_min_size(
                            Pos2::new(clip_rect.right() - 48.0, clip_rect.top() + 4.0),
                            Vec2::new(42.0, 14.0),
                        );
                        painter.rect_filled(badge_rect, 3.0, Color32::from_rgb(0xdc, 0x35, 0x45));
                        painter.text(
                            badge_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            sync_str,
                            egui::FontId::proportional(9.5),
                            Color32::WHITE,
                        );
                    }
                }
            }

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
                            painter.add(egui::Shape::convex_polygon(
                                dia,
                                COLOR_AMBER,
                                Stroke::new(0.5_f32, Color32::BLACK),
                            ));
                        }
                    }
                }
            }
        }
    }

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
    let pointer = ui.input(|i| i.pointer.clone());
    let snap_thresh =
        ((SNAP_THRESHOLD_PX / state.pixels_per_second.max(1.0)) as f64).clamp(0.02, 0.25);

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
        EditTool::Slip => {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        _ => {
            if matches!(
                state.drag_mode,
                DragMode::FadeIn | DragMode::FadeOut | DragMode::TrimIn | DragMode::TrimOut | DragMode::Slip
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
                                DragMode::TrimIn | DragMode::TrimOut | DragMode::Slip => {
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
                    let clicked_t =
                        ((pos.x - rect.left()) / state.pixels_per_second).max(0.0) as f64;
                    let snapped_t = project
                        .timeline
                        .snap_time(clicked_t, Some(snap_thresh), None);
                    if let Some((tr_id, cl_id, _)) = find_clip_at_pos(project, state, rect, pos) {
                        state.selected_clip_pairs = vec![(tr_id, cl_id)];
                    }
                    action = Some(CanvasAction::Seek(snapped_t));
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
                                            state.batch_orig_ins.push((
                                                t_id.clone(),
                                                c_id.clone(),
                                                c.timeline_in,
                                            ));
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
                    action = Some(CanvasAction::SelectClips(state.selected_clip_pairs.clone()));
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
                    action = Some(CanvasAction::SelectClips(linked_pairs));
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

                if let Some(tr) = project.timeline.get_track_by_id_mut(&tr_id) {
                    if let Some(cl) = tr.get_clip_by_id_mut(&cl_id) {
                        state.drag_orig_in = cl.timeline_in;
                        state.drag_orig_out = cl.timeline_out;
                        state.drag_orig_sin = cl.source_in;
                        state.drag_orig_sout = cl.effective_source_out();
                        state.drag_orig_vol = cl.volume;

                        if mode == DragMode::VolumeRubberband
                            && (ui.input(|i| {
                                i.pointer
                                    .button_double_clicked(egui::PointerButton::Primary)
                            }) || is_alt)
                        {
                            let old_v = cl.volume;
                            cl.volume = 1.0;
                            action = Some(CanvasAction::SetClipVolume {
                                track_id: tr_id.clone(),
                                clip_id: cl_id.clone(),
                                old_volume: old_v,
                                new_volume: 1.0,
                            });
                        }
                    }
                }

                state.batch_orig_ins.clear();
                for (t_id, c_id) in &state.selected_clip_pairs {
                    if let Some(tr) = project.timeline.get_track_by_id(t_id) {
                        if let Some(cl) = tr.get_clip_by_id(c_id) {
                            state
                                .batch_orig_ins
                                .push((t_id.clone(), c_id.clone(), cl.timeline_in));
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
                    action = Some(CanvasAction::Seek(snapped_t));
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
                    action = Some(CanvasAction::Seek(snapped));
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
                        if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                cl.trim_in(snapped_in);
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
                        if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                cl.trim_out(snapped_out);
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

                        if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                let clip_dur = cl.duration();
                                let max_dur = media_dur
                                    .unwrap_or_else(|| cl.effective_source_out().max(10000.0));
                                let max_src_in = (max_dur - clip_dur).max(0.0);
                                let new_src_in = (state.drag_orig_sin - dt).clamp(0.0, max_src_in);
                                cl.source_in = new_src_in;
                                cl.source_out = Some(new_src_in + clip_dur);

                                let tip_rect = Rect::from_center_size(
                                    Pos2::new(pos.x, pos.y - 18.0),
                                    Vec2::new(140.0, 20.0),
                                );
                                painter.rect_filled(
                                    tip_rect,
                                    4.0,
                                    Color32::from_rgb(0x18, 0x18, 0x22),
                                );
                                painter.rect_stroke(
                                    tip_rect,
                                    4.0,
                                    Stroke::new(1.0_f32, Color32::from_rgb(0xe0, 0x7a, 0x38)),
                                );
                                painter.text(
                                    tip_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    format!("Slip: {:.2}s - {:.2}s", cl.source_in, cl.effective_source_out()),
                                    egui::FontId::proportional(11.0),
                                    Color32::WHITE,
                                );
                            }
                        }
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

                            if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                                if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                    cl.volume = new_vol;
                                }
                            }

                            let db = if new_vol > 0.0001 {
                                20.0 * new_vol.log10()
                            } else {
                                -99.0
                            };
                            let tip_str = if db <= -60.0 {
                                "Ses: -∞ dB (Sessiz)".to_string()
                            } else {
                                format!("Ses: {:+.1} dB ({:.0}%)", db, new_vol * 100.0)
                            };
                            let tip_rect = Rect::from_center_size(
                                Pos2::new(pos.x, pos.y - 20.0),
                                Vec2::new(140.0, 22.0),
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
                                    moves.push((
                                        t_id.clone(),
                                        c_id.clone(),
                                        *orig_in,
                                        cl.timeline_in,
                                    ));
                                }
                            }
                        }
                    }
                    if !moves.is_empty() {
                        action = Some(CanvasAction::BatchClipsMoved(moves));
                    }
                } else if let (Some(tr_id), Some(cl_id)) =
                    (&state.drag_track_id, &state.drag_clip_id)
                {
                    let target_track_id = state
                        .drag_target_track_id
                        .clone()
                        .unwrap_or_else(|| tr_id.clone());
                    if let Some(tr) = project.timeline.get_track_by_id(tr_id) {
                        if let Some(cl) = tr.get_clip_by_id(cl_id) {
                            if (cl.timeline_in - state.drag_orig_in).abs() > 1e-4
                                || &target_track_id != tr_id
                            {
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
            DragMode::TrimIn | DragMode::TrimOut | DragMode::Slip => {
                if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id) {
                    if let Some(tr) = project.timeline.get_track_by_id(tr_id) {
                        if let Some(cl) = tr.get_clip_by_id(cl_id) {
                            let new_bounds = (
                                cl.timeline_in,
                                cl.timeline_out,
                                cl.source_in,
                                cl.effective_source_out(),
                            );
                            let old_bounds = (
                                state.drag_orig_in,
                                state.drag_orig_out,
                                state.drag_orig_sin,
                                state.drag_orig_sout,
                            );
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
            DragMode::VolumeRubberband => {
                if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id) {
                    if let Some(tr) = project.timeline.get_track_by_id(tr_id) {
                        if let Some(cl) = tr.get_clip_by_id(cl_id) {
                            if (cl.volume - state.drag_orig_vol).abs() > 1e-4 {
                                action = Some(CanvasAction::SetClipVolume {
                                    track_id: tr_id.clone(),
                                    clip_id: cl_id.clone(),
                                    old_volume: state.drag_orig_vol,
                                    new_volume: cl.volume,
                                });
                            }
                        }
                    }
                }
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
        ui.set_min_width(260.0);
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        ui.spacing_mut().button_padding = Vec2::new(10.0, 5.0);
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);

        // 1. If a gap is selected (or hovered right click on a gap)
        if state.selected_gap.is_none() && state.selected_clip_pairs.is_empty() {
            if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                if let Some(gap) = find_gap_at_pos(project, state, rect, pos) {
                    state.selected_gap = Some(gap);
                }
            }
        }

        if let Some((tr_id, gap_start, gap_end)) = state.selected_gap.clone() {
            let gap_dur = (gap_end - gap_start).max(0.0);
            ui.label(
                RichText::new(format!("⏩ Zaman Çizgisi Boşluğu ({:.2}s)", gap_dur))
                    .color(COLOR_AMBER)
                    .strong(),
            );
            ui.label(
                RichText::new(format!("Aralık: {:.2}s - {:.2}s", gap_start, gap_end))
                    .small()
                    .color(Color32::from_rgb(0xa0, 0xa0, 0xb0)),
            );
            ui.separator();

            let is_locked = project
                .timeline
                .get_track_by_id(&tr_id)
                .map(|t| t.locked)
                .unwrap_or(false);

            ui.add_enabled_ui(!is_locked, |ui| {
                if ui
                    .button("⏩ Boşluğu Kapat (Ripple Delete Gap)  [Del / Shift+Del]")
                    .clicked()
                {
                    action = Some(CanvasAction::CloseGap {
                        track_id: Some(tr_id.clone()),
                        gap_start,
                        gap_end,
                    });
                    ui.close_menu();
                }
            });

            if ui.button("⏩ Tüm Kanallardaki Boşluğu Kapat").clicked() {
                action = Some(CanvasAction::CloseGap {
                    track_id: None,
                    gap_start,
                    gap_end,
                });
                ui.close_menu();
            }

            ui.separator();
            if ui.button("Boşluk Seçimini Kaldır (Esc)").clicked() {
                state.selected_gap = None;
                ui.close_menu();
            }
            return;
        }

        // 2. If clip(s) are selected
        if let Some((tr_id, cl_id)) = state.selected_clip_pairs.first().cloned() {
            let is_locked = project
                .timeline
                .get_track_by_id(&tr_id)
                .map(|t| t.locked)
                .unwrap_or(false);

            if is_locked {
                ui.label(
                    RichText::new("[K] Kanal Kilitli (Düzenlenemez)")
                        .color(Color32::from_rgb(0xe0, 0x7a, 0x38))
                        .italics(),
                );
                ui.separator();
            }

            ui.add_enabled_ui(!is_locked, |ui| {
                if ui.button("Hız ve Süre (Speed & Duration)...").clicked() {
                    action = Some(CanvasAction::PromptSpeed(tr_id.clone(), cl_id.clone()));
                    ui.close_menu();
                }

                if let Some(tr) = project.timeline.get_track_by_id(&tr_id) {
                    if tr.track_type == TrackType::Video
                        && ui.button("Sesi Ayır (Detach Audio)").clicked()
                    {
                        action = Some(CanvasAction::DetachAudio(tr_id.clone(), cl_id.clone()));
                        ui.close_menu();
                    }
                }

                if ui.button("Klibi Böl (S)").clicked() {
                    action = Some(CanvasAction::SplitAtPlayhead);
                    ui.close_menu();
                }

                ui.separator();

                // Color labels submenu
                ui.menu_button("🎨 Renk Etiketi (Color Label)", |ui| {
                    ui.set_min_width(200.0);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    ui.spacing_mut().button_padding = Vec2::new(8.0, 4.0);

                    for color in ColorLabel::ALL {
                        let mut btn_text = RichText::new(color.label());
                        if let Some([r, g, b]) = color.color_rgb() {
                            btn_text = btn_text.color(Color32::from_rgb(r, g, b));
                        }
                        if ui.button(btn_text).clicked() {
                            action = Some(CanvasAction::SetClipColor {
                                track_id: tr_id.clone(),
                                clip_id: cl_id.clone(),
                                color: *color,
                            });
                            ui.close_menu();
                        }
                    }
                });

                // Link / Unlink toggle button
                let is_linked = project
                    .timeline
                    .get_track_by_id(&tr_id)
                    .and_then(|t| t.get_clip_by_id(&cl_id))
                    .and_then(|c| c.linked_group_id.as_ref())
                    .is_some();
                if is_linked {
                    if ui
                        .button("🔗 Klip Bağını Kaldır (Unlink)  [Ctrl+L]")
                        .clicked()
                    {
                        action = Some(CanvasAction::ToggleLinkSelected);
                        ui.close_menu();
                    }
                } else if state.selected_clip_pairs.len() > 1
                    && ui.button("🔗 Klipleri Bağla (Link)  [Ctrl+L]").clicked()
                {
                    action = Some(CanvasAction::ToggleLinkSelected);
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

                if state.active_tool == EditTool::Slip {
                    return Some((track.id.clone(), clip.id.clone(), DragMode::Slip));
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
                    let norm = (clip.volume / 2.0).clamp(0.0, 1.0) as f32;
                    let active_top = clip_rect.top() + 18.0;
                    let active_bottom = clip_rect.bottom() - 6.0;
                    let range = (active_bottom - active_top).max(8.0);
                    let vol_y = active_bottom - (norm * range);

                    if (pos.y - vol_y).abs() <= 5.0 {
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

#[cfg(test)]
mod tests {
    use super::*;
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
}
