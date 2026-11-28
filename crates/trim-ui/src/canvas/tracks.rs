use crate::theme::*;
use egui::{Color32, Pos2, Rect, Stroke, Ui, Vec2};
use std::collections::HashSet;
use trim_core::project::Project;
use trim_core::track::TrackType;
use trim_media::WaveformGenerator;

use super::gaps::render_track_gaps;
use super::state::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn render_tracks_and_clips(
    ui: &mut Ui,
    painter: &egui::Painter,
    rect: Rect,
    view_clip_rect: Rect,
    project: &Project,
    state: &mut TimelineCanvasState,
    track_offsets: &[(f32, f32)],
    selected_ids: &HashSet<String>,
    waveform_gen: &WaveformGenerator,
    textures: &std::collections::HashMap<String, egui::TextureHandle>,
) {
    let mut linked_group_clips: std::collections::HashMap<&str, Vec<(&str, f64)>> =
        std::collections::HashMap::new();
    for t in &project.timeline.tracks {
        for c in &t.clips {
            if let Some(ref gid) = c.linked_group_id {
                linked_group_clips
                    .entry(gid.as_str())
                    .or_default()
                    .push((t.id.as_str(), c.timeline_in));
            }
        }
    }
    // 1. Draw track lanes
    for (t_idx, track) in project.timeline.tracks.iter().enumerate() {
        let (ty, track_h) = track_offsets[t_idx];
        if ty + track_h < view_clip_rect.top() - 32.0 || ty > view_clip_rect.bottom() + 32.0 {
            continue;
        }
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

        render_track_gaps(ui, painter, rect, view_clip_rect, track, ty, track_h, state);

        // Draw clips on this track
        let view_clip_rect = ui.clip_rect();
        for clip in &track.clips {
            let cx = rect.left() + (clip.timeline_in as f32 * state.pixels_per_second);
            let cw = (clip.duration() as f32 * state.pixels_per_second).max(4.0);
            let clip_rect = Rect::from_min_size(Pos2::new(cx, ty), Vec2::new(cw, track_h));
            if clip_rect.right() < view_clip_rect.left() - 32.0
                || clip_rect.left() > view_clip_rect.right() + 32.0
            {
                continue;
            }

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
            } else if clip.is_adjustment_layer() {
                if is_sel {
                    Color32::from_rgb(0x9b, 0x3b, 0xd8)
                } else {
                    Color32::from_rgb(0x6e, 0x28, 0x9c)
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

            let is_offline = match project.get_media(&clip.media_id) {
                Some(m) => !state.is_media_online(&m.id, &m.file_path),
                None => clip.text_overlay.is_none() && !clip.is_adjustment_layer(),
            };

            let fill_color = if is_offline {
                Color32::from_rgb(0x42, 0x16, 0x1a)
            } else {
                base_color
            };
            painter.rect_filled(clip_rect, 4.0, fill_color);

            if is_offline {
                let mut x = clip_rect.left() - clip_rect.height();
                while x < clip_rect.right() {
                    let p1 = Pos2::new(x.max(clip_rect.left()), clip_rect.bottom());
                    let p2 = Pos2::new(
                        (x + clip_rect.height()).min(clip_rect.right()),
                        clip_rect.top(),
                    );
                    if p2.x > clip_rect.left() && p1.x < clip_rect.right() {
                        painter.line_segment(
                            [p1, p2],
                            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(230, 50, 50, 60)),
                        );
                    }
                    x += 14.0;
                }
            }

            // Colored top accent stripe if label is set
            if let Some([r, g, b]) = clip.color_label.color_rgb() {
                let stripe_rect =
                    Rect::from_min_size(clip_rect.left_top(), Vec2::new(clip_rect.width(), 3.5));
                painter.rect_filled(stripe_rect, 2.0, Color32::from_rgb(r, g, b));
            }

            // Video Filmstrip Thumbnails (Video tracks only)
            if track.track_type == TrackType::Video && clip_rect.width() >= 28.0 {
                let strip_tex = textures.get(&format!("strip:{}", clip.media_id));
                let poster_tex = textures.get(&clip.media_id);
                if let Some((tex, is_strip)) = strip_tex
                    .map(|t| (t, true))
                    .or_else(|| poster_tex.map(|t| (t, false)))
                {
                    let body_top = clip_rect.top() + 18.0;
                    let body_bottom = clip_rect.bottom() - 2.0;
                    if body_bottom > body_top + 8.0 {
                        let film_h = body_bottom - body_top;
                        let film_w = (film_h * 1.777_f32).clamp(16.0, 100.0);
                        let total_w = clip_rect.width() - 4.0;
                        let num_frames = ((total_w / film_w).floor() as usize).max(1);
                        let media_dur = project
                            .get_media(&clip.media_id)
                            .map(|m| m.duration)
                            .unwrap_or_else(|| clip.effective_source_out())
                            .max(0.1);
                        let tile_count = trim_media::thumbnail::FILMSTRIP_TILE_COUNT as f32;

                        let start_x = clip_rect.left() + 2.0;
                        let first_visible = ((view_clip_rect.left() - 32.0 - start_x) / film_w)
                            .floor()
                            .max(0.0) as usize;
                        let max_visible_right =
                            (clip_rect.right() - 3.0).min(view_clip_rect.right() + 32.0);
                        let mut frame_x = start_x + (first_visible as f32) * film_w;
                        for i in first_visible..num_frames {
                            if frame_x >= max_visible_right {
                                break;
                            }
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
                            let uv = if is_strip {
                                let center_px = (frame_x - clip_rect.left()) + curr_w * 0.5;
                                let tl_time = clip.timeline_in
                                    + (center_px / state.pixels_per_second.max(1.0)) as f64;
                                let src_t = clip.map_timeline_to_source(tl_time);
                                let frac = (src_t / media_dur).clamp(0.0, 0.9999) as f32;
                                let tile_idx =
                                    (frac * tile_count).floor().clamp(0.0, tile_count - 1.0);
                                let u0 = tile_idx / tile_count;
                                let u1 = (tile_idx + uv_max_x) / tile_count;
                                Rect::from_min_max(Pos2::new(u0, 0.0), Pos2::new(u1, 1.0))
                            } else {
                                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(uv_max_x, 1.0))
                            };

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

                            let vis_left = (view_clip_rect.left() - clip_rect.left()).max(0.0);
                            let vis_right =
                                (view_clip_rect.right() - clip_rect.left()).min(clip_rect.width());

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
                    let active_top = clip_rect.top() + 18.0;
                    let active_bottom = clip_rect.bottom() - 6.0;
                    let range = (active_bottom - active_top).max(8.0);

                    let line_color = if is_sel {
                        Color32::from_rgba_premultiplied(0xf5, 0x9e, 0x0b, 220)
                    } else {
                        Color32::from_rgba_premultiplied(0x38, 0xa0, 0xf0, 190)
                    };

                    let hover_p = ui.input(|i| i.pointer.hover_pos());

                    if let Some(kfs) = clip.keyframes.get("volume").filter(|k| !k.is_empty()) {
                        let dur = clip.duration().max(0.04);
                        let mut pts: Vec<Pos2> = Vec::with_capacity(kfs.len() + 2);

                        let v_start =
                            (clip.get_property_at_time("volume", 0.0) / 2.0).clamp(0.0, 1.0) as f32;
                        pts.push(Pos2::new(
                            clip_rect.left() + 2.0,
                            active_bottom - (v_start * range),
                        ));

                        for kf in kfs {
                            let kf_frac = (kf.time / dur).clamp(0.0, 1.0) as f32;
                            let kf_x = clip_rect.left() + 2.0 + kf_frac * (clip_rect.width() - 4.0);
                            let kf_norm = (kf.value / 2.0).clamp(0.0, 1.0) as f32;
                            let kf_y = active_bottom - (kf_norm * range);
                            pts.push(Pos2::new(kf_x, kf_y));
                        }

                        let v_end =
                            (clip.get_property_at_time("volume", dur) / 2.0).clamp(0.0, 1.0) as f32;
                        pts.push(Pos2::new(
                            clip_rect.right() - 2.0,
                            active_bottom - (v_end * range),
                        ));

                        for w in pts.windows(2) {
                            painter.line_segment([w[0], w[1]], Stroke::new(1.5_f32, line_color));
                        }

                        // Draw diamond handles on each keyframe
                        for (idx, kf) in kfs.iter().enumerate() {
                            let kf_frac = (kf.time / dur).clamp(0.0, 1.0) as f32;
                            let kf_x = clip_rect.left() + 2.0 + kf_frac * (clip_rect.width() - 4.0);
                            let kf_norm = (kf.value / 2.0).clamp(0.0, 1.0) as f32;
                            let kf_y = active_bottom - (kf_norm * range);
                            let kf_pos = Pos2::new(kf_x, kf_y);

                            let is_node_hovered =
                                hover_p.is_some_and(|p| p.distance(kf_pos) <= 6.0);
                            let is_active_node = state.drag_mode == DragMode::VolumeRubberband
                                && state.drag_clip_id.as_deref() == Some(&clip.id)
                                && state.drag_kf_index == Some(idx);

                            let r = if is_node_hovered || is_active_node {
                                5.0
                            } else {
                                3.5
                            };
                            let col = if is_node_hovered || is_active_node {
                                Color32::WHITE
                            } else {
                                line_color
                            };

                            let diamond = vec![
                                Pos2::new(kf_pos.x, kf_pos.y - r),
                                Pos2::new(kf_pos.x + r, kf_pos.y),
                                Pos2::new(kf_pos.x, kf_pos.y + r),
                                Pos2::new(kf_pos.x - r, kf_pos.y),
                            ];
                            painter.add(egui::Shape::convex_polygon(
                                diamond,
                                col,
                                Stroke::new(1.0_f32, Color32::from_rgb(0x18, 0x18, 0x24)),
                            ));
                        }
                    } else {
                        let norm = (clip.volume / 2.0).clamp(0.0, 1.0) as f32;
                        let vol_y = active_bottom - (norm * range);

                        painter.line_segment(
                            [
                                Pos2::new(clip_rect.left() + 2.0, vol_y),
                                Pos2::new(clip_rect.right() - 2.0, vol_y),
                            ],
                            Stroke::new(1.5_f32, line_color),
                        );

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
            let border_stroke = if is_offline {
                Stroke::new(2.0_f32, Color32::from_rgb(0xe0, 0x38, 0x38))
            } else if is_sel {
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
            let title_prefix = if clip.is_adjustment_layer() {
                "◈ "
            } else if clip.is_nested_sequence() {
                "≡ "
            } else if clip.text_overlay.is_some() {
                "T "
            } else {
                ""
            };
            let offline_badge = if is_offline {
                crate::i18n::pick(state.lang, " [!] ÇEVRİMDIŞI", " [!] OFFLINE")
            } else {
                ""
            };
            let title_str = format!(
                "{}{}{}{}{}{}{}  [{:.2}s]{}",
                title_prefix,
                clip.name,
                offline_badge,
                lock_badge,
                link_badge,
                trans_badge,
                kf_badge,
                clip.duration(),
                speed_badge
            );

            let header_w = (clip_rect.width() - 4.0).max(0.0);
            if header_w > 0.0 {
                let header_rect = Rect::from_min_size(
                    Pos2::new(clip_rect.left() + 2.0, clip_rect.top() + 2.0),
                    Vec2::new(header_w, 16.0),
                );
                painter.rect_filled(
                    header_rect,
                    2.0,
                    Color32::from_rgba_premultiplied(16, 18, 24, 150),
                );

                let right_pad = if clip.linked_group_id.is_some() && clip_rect.width() > 65.0 {
                    50.0
                } else {
                    4.0
                };
                let text_max_x = (clip_rect.right() - right_pad).max(clip_rect.left() + 4.0);
                let text_clip_rect = Rect::from_min_max(
                    Pos2::new(clip_rect.left() + 2.0, clip_rect.top()),
                    Pos2::new(text_max_x, clip_rect.bottom()),
                );

                if text_clip_rect.width() >= 12.0 {
                    let text_painter = painter.with_clip_rect(text_clip_rect);
                    text_painter.text(
                        Pos2::new(clip_rect.left() + 8.0, clip_rect.top() + 4.0),
                        egui::Align2::LEFT_TOP,
                        title_str,
                        egui::FontId::proportional(11.0),
                        Color32::WHITE,
                    );
                }
            }

            // Sync offset badge for linked clips (O(1) lookup via per-frame map)
            if let Some(ref gid) = clip.linked_group_id {
                let companion_in = linked_group_clips.get(gid.as_str()).and_then(|entries| {
                    entries
                        .iter()
                        .find(|(tid, _)| *tid != track.id.as_str())
                        .map(|(_, t_in)| *t_in)
                });

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
}
