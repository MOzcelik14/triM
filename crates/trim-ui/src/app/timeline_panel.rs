use crate::canvas::{calculate_total_tracks_height, render_timeline_canvas};
use crate::ruler::{render_time_ruler, RulerAction};
use crate::toolbar::render_vertical_toolbar;
use crate::vu_meter::VuMeterWidget;

use egui::{Color32, Vec2};

use super::TrimApp;

impl TrimApp {
    pub(super) fn render_timeline_panel(&mut self, ctx: &egui::Context) {
        self.canvas_state.lang = self.lang();
        // -------------------------------------------------------------
        // BOTTOM TIMELINE PANEL: Guaranteed permanently anchored at the bottom
        // -------------------------------------------------------------
        egui::TopBottomPanel::bottom("timeline_panel")
            .resizable(true)
            .min_height(200.0)
            .default_height(340.0)
            .max_height((ctx.screen_rect().height() - 150.0).max(400.0))
            .show(ctx, |ui| {
                self.render_timeline_header(ui);

                ui.add_space(2.0);
                ui.separator();

                // Main Timeline Row: [Toolbar] [Track Headers] [Canvas + Ruler] [Master VU Meter]
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;

                    let top_bar_h = 26.0;
                    let total_tracks_h = calculate_total_tracks_height(
                        &self.project.timeline.tracks,
                        &self.canvas_state,
                    );
                    let avail_h = ui.available_height();
                    let content_viewport_h = (avail_h - top_bar_h).max(40.0);

                    // If tracks fit within available height, no vertical scrolling is needed
                    if total_tracks_h <= content_viewport_h {
                        if self.canvas_state.scroll_offset.y != 0.0 {
                            self.canvas_state.scroll_offset.y = 0.0;
                            self.canvas_state.pending_scroll_y = Some(0.0);
                        }
                    } else {
                        let max_scroll = total_tracks_h - content_viewport_h;
                        let clamped_y = self.canvas_state.scroll_offset.y.clamp(0.0, max_scroll);
                        if clamped_y != self.canvas_state.scroll_offset.y {
                            self.canvas_state.scroll_offset.y = clamped_y;
                            self.canvas_state.pending_scroll_y = Some(clamped_y);
                        }
                    }

                    let current_scroll_x = self
                        .canvas_state
                        .pending_scroll_x
                        .unwrap_or(self.canvas_state.scroll_offset.x);
                    let current_scroll_y = self.canvas_state.scroll_offset.y;

                    // 1. Vertical Toolbar (Fixed width 42px for 32x32 tools)
                    ui.allocate_ui(Vec2::new(42.0, ui.available_height()), |ui| {
                        let lang = self.lang();
                        render_vertical_toolbar(ui, &mut self.tools.current_edit_tool, lang);
                    });

                    ui.add_space(2.0);
                    ui.separator();

                    self.render_timeline_track_headers(
                        ui,
                        current_scroll_y,
                        top_bar_h,
                        total_tracks_h,
                        content_viewport_h,
                    );

                    // 3. Scrollable Timeline Canvas + Time Ruler
                    let vu_w = 88.0;
                    let avail_canvas_w = (ui.available_width() - vu_w).max(100.0);
                    ui.allocate_ui(Vec2::new(avail_canvas_w, ui.available_height()), |ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::ZERO;

                            // 1. Time Ruler (Follower of canvas X scroll)
                            egui::ScrollArea::horizontal()
                                .id_salt("timeline_ruler_scroll")
                                .max_width(avail_canvas_w)
                                .horizontal_scroll_offset(current_scroll_x)
                                .scroll_bar_visibility(
                                    egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                                )
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                                    let markers = self.project.timeline.markers.clone();
                                    let (_ruler_resp, ruler_act) = render_time_ruler(
                                        ui,
                                        self.canvas_state.pixels_per_second,
                                        current_scroll_x,
                                        self.project.timeline.duration(),
                                        self.project.timeline.fps,
                                        self.playback.current_time,
                                        &markers,
                                        self.project.timeline.in_point,
                                        self.project.timeline.out_point,
                                        Some(&|t| {
                                            self.media
                                                .video_engine
                                                .is_timeline_time_cached(&self.project, t)
                                        }),
                                    );
                                    if !self.playback.is_playing
                                        && !self.media.video_engine.is_decoding_seek
                                    {
                                        let step_t = (1.0 / self.project.timeline.fps.max(24.0))
                                            .clamp(0.03, 0.1);
                                        for k in 1..=6 {
                                            let probe_t =
                                                self.playback.current_time + k as f64 * step_t;
                                            if self
                                                .media
                                                .video_engine
                                                .prewarm_frame_at_time(&self.project, probe_t)
                                            {
                                                break;
                                            }
                                        }
                                    }

                                    if let Some(act) = ruler_act {
                                        match act {
                                            RulerAction::Seek(t) => self.seek(t),
                                            RulerAction::MarkerClicked(m_id) => {
                                                if let Some(m) = self
                                                    .project
                                                    .timeline
                                                    .markers
                                                    .iter()
                                                    .find(|m| m.id == m_id)
                                                {
                                                    self.seek(m.time);
                                                }
                                            }
                                        }
                                    }
                                });

                            // 2. Timeline Canvas (Driver of both X and Y scroll)
                            self.canvas_state.active_tool = self.tools.current_edit_tool;
                            self.project.timeline.snap_enabled = self.tools.snap_enabled;
                            let mut canvas_scroll = egui::ScrollArea::both()
                                .id_salt("timeline_canvas_both_scroll")
                                .auto_shrink([false, false]);

                            let prev_scroll_y = self.canvas_state.scroll_offset.y;
                            if let Some(target_x) = self.canvas_state.pending_scroll_x.take() {
                                canvas_scroll = canvas_scroll.horizontal_scroll_offset(target_x);
                            }
                            if let Some(target_y) = self.canvas_state.pending_scroll_y.take() {
                                canvas_scroll = canvas_scroll.vertical_scroll_offset(target_y);
                            }

                            let canvas_scroll_out = canvas_scroll.show(ui, |ui| {
                                ui.spacing_mut().item_spacing = Vec2::ZERO;
                                let (_canvas_resp, canvas_act) = render_timeline_canvas(
                                    ui,
                                    &mut self.project,
                                    &mut self.canvas_state,
                                    self.playback.current_time,
                                    &self.media.waveform_gen,
                                    &self.media.media_bin_textures,
                                );
                                canvas_act
                            });

                            self.canvas_state.scroll_offset = canvas_scroll_out.state.offset;

                            // Handle Ctrl + MouseWheel timeline zooming centered under the cursor
                            let canvas_rect = canvas_scroll_out.inner_rect;
                            let is_hovering_canvas = ui
                                .input(|i| i.pointer.hover_pos())
                                .map(|p| canvas_rect.contains(p))
                                .unwrap_or(false);
                            let (ctrl, raw_scroll, zoom_delta) = ui.input(|i| {
                                (
                                    i.modifiers.ctrl || i.modifiers.command,
                                    i.raw_scroll_delta,
                                    i.zoom_delta(),
                                )
                            });

                            if is_hovering_canvas
                                && ctrl
                                && (raw_scroll.y != 0.0 || zoom_delta != 1.0)
                            {
                                self.canvas_state.pending_scroll_y = Some(prev_scroll_y);
                                let factor = if zoom_delta != 1.0 {
                                    zoom_delta
                                } else if raw_scroll.y > 0.0 {
                                    1.15
                                } else {
                                    1.0 / 1.15
                                };
                                if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                                    let view_x = pos.x - canvas_rect.left();
                                    let content_x = self.canvas_state.scroll_offset.x + view_x;
                                    let time = (content_x / self.canvas_state.pixels_per_second)
                                        .max(0.0)
                                        as f64;
                                    self.zoom_timeline(factor, Some(time), Some(view_x));
                                }
                            }

                            if let Some(act) = canvas_scroll_out.inner {
                                self.handle_canvas_action(ctx, act);
                            }
                        });
                    });

                    // 4. Divider + Master VU Meter (Far Right)
                    let full_h = ui.available_height();
                    ui.allocate_ui_with_layout(
                        Vec2::new(vu_w, full_h),
                        egui::Layout::left_to_right(egui::Align::Min),
                        |ui| {
                            ui.spacing_mut().item_spacing = Vec2::ZERO;
                            ui.add_space(4.0);
                            let sep_rect = ui.allocate_space(Vec2::new(1.0, full_h)).1;
                            ui.painter().rect_filled(
                                sep_rect,
                                0.0,
                                Color32::from_rgb(0x2c, 0x2e, 0x38),
                            );
                            ui.add_space(6.0);
                            ui.allocate_ui(Vec2::new((vu_w - 15.0).max(68.0), full_h), |ui| {
                                ui.add(VuMeterWidget::new(&mut self.playback.vu_state));
                            });
                        },
                    );
                });
            });
    }
}
