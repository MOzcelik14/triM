use super::TrimApp;
use crate::track_header::{render_track_header, TrackHeaderAction, TRACK_HEADER_WIDTH};
use egui::{Color32, RichText, Stroke, Vec2};
use trim_commands::commands::*;
use trim_core::clip::Clip;

impl TrimApp {
    pub(super) fn render_timeline_track_headers(
        &mut self,
        ui: &mut egui::Ui,
        current_scroll_y: f32,
        top_bar_h: f32,
        total_tracks_h: f32,
        content_viewport_h: f32,
    ) {
        // 2. Pinned Track Headers (Left, Fixed comfortable width 216px)
        let header_w = TRACK_HEADER_WIDTH;
        let mut add_video_requested = false;
        let mut add_audio_requested = false;
        let mut add_adjustment_layer_requested = false;
        let mut reset_heights_requested = false;

        ui.allocate_ui(Vec2::new(header_w, ui.available_height()), |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let (corner_rect, _) =
                    ui.allocate_exact_size(Vec2::new(header_w, top_bar_h), egui::Sense::hover());
                let cp = ui.painter_at(corner_rect);
                cp.rect_filled(corner_rect, 0.0, Color32::from_rgb(0x18, 0x19, 0x22));
                cp.line_segment(
                    [corner_rect.left_bottom(), corner_rect.right_bottom()],
                    Stroke::new(1.0_f32, Color32::from_rgb(0x2a, 0x2c, 0x38)),
                );
                cp.line_segment(
                    [corner_rect.right_top(), corner_rect.right_bottom()],
                    Stroke::new(1.0_f32, Color32::from_rgb(0x2a, 0x2c, 0x38)),
                );

                let mut corner_ui = ui.new_child(egui::UiBuilder::new().max_rect(corner_rect));
                corner_ui.spacing_mut().item_spacing = Vec2::ZERO;
                corner_ui.horizontal_centered(|cui| {
                    cui.add_space(8.0);
                    cui.label(
                        RichText::new("≡ KANALLAR")
                            .size(10.5)
                            .strong()
                            .color(Color32::from_rgb(0x94, 0x98, 0xa8)),
                    );

                    cui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |cui| {
                        cui.add_space(6.0);
                        cui.menu_button(
                            RichText::new("+ Kanal")
                                .size(10.0)
                                .color(Color32::from_rgb(0x81, 0x8c, 0xf8))
                                .strong(),
                            |menu_ui| {
                                if menu_ui.button("▸ Video Kanalı Ekle").clicked() {
                                    add_video_requested = true;
                                    menu_ui.close_menu();
                                }
                                if menu_ui.button("♪ Ses Kanalı Ekle").clicked() {
                                    add_audio_requested = true;
                                    menu_ui.close_menu();
                                }
                                if menu_ui.button("◈ Ayar Katmanı Ekle").clicked() {
                                    add_adjustment_layer_requested = true;
                                    menu_ui.close_menu();
                                }
                                menu_ui.separator();
                                if menu_ui.button("↕ Yükseklikleri Sıfırla").clicked() {
                                    reset_heights_requested = true;
                                    menu_ui.close_menu();
                                }
                            },
                        );
                    });
                });

                let headers_scroll_out = egui::ScrollArea::vertical()
                    .id_salt("timeline_track_headers_scroll")
                    .vertical_scroll_offset(current_scroll_y)
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::ZERO;
                            let mut pending_header_actions = Vec::new();
                            for track in &self.project.timeline.tracks {
                                let is_patch = self.target_video_track == track.id
                                    || self.target_audio_track == track.id;
                                let tr_h = self.canvas_state.get_track_height(&track.id);
                                let is_recording_vo = self
                                    .active_voiceover
                                    .as_ref()
                                    .map(|vo| vo.track_id == track.id)
                                    .unwrap_or(false);
                                let (_, hdr_act) =
                                    render_track_header(ui, track, tr_h, is_patch, is_recording_vo);
                                if let Some(act) = hdr_act {
                                    pending_header_actions.push((track.id.clone(), act));
                                }
                                ui.add_space(self.canvas_state.track_spacing);
                            }

                            for (t_id, act) in pending_header_actions {
                                match act {
                                    TrackHeaderAction::ToggleMute => {
                                        if let Some(t) =
                                            self.project.timeline.get_track_by_id_mut(&t_id)
                                        {
                                            t.muted = !t.muted;
                                            self.project.mark_dirty();
                                        }
                                    }
                                    TrackHeaderAction::ToggleSolo => {
                                        if let Some(t) =
                                            self.project.timeline.get_track_by_id_mut(&t_id)
                                        {
                                            t.solo = !t.solo;
                                            self.project.mark_dirty();
                                        }
                                    }
                                    TrackHeaderAction::ToggleLock => {
                                        if let Some(t) =
                                            self.project.timeline.get_track_by_id_mut(&t_id)
                                        {
                                            t.locked = !t.locked;
                                            self.project.mark_dirty();
                                        }
                                    }
                                    TrackHeaderAction::TogglePatchTarget => {
                                        if let Some(t) =
                                            self.project.timeline.get_track_by_id(&t_id)
                                        {
                                            if t.track_type == trim_core::track::TrackType::Video {
                                                self.target_video_track = t_id.clone();
                                                self.target_audio_track = self
                                                    .project
                                                    .timeline
                                                    .find_or_create_corresponding_audio_track(
                                                        &t_id,
                                                    );
                                            } else {
                                                self.target_audio_track = t_id.clone();
                                            }
                                        }
                                    }
                                    TrackHeaderAction::RecordVoiceover => {
                                        let is_currently_rec = self
                                            .active_voiceover
                                            .as_ref()
                                            .map(|vo| vo.track_id == t_id)
                                            .unwrap_or(false);
                                        if is_currently_rec {
                                            self.finish_voiceover();
                                        } else {
                                            if self.active_voiceover.is_some() {
                                                self.finish_voiceover();
                                            }
                                            self.start_voiceover(t_id.clone());
                                        }
                                    }
                                    TrackHeaderAction::VolumeChanged(vol) => {
                                        if let Some(t) =
                                            self.project.timeline.get_track_by_id_mut(&t_id)
                                        {
                                            t.volume = vol;
                                            self.project.mark_dirty();
                                        }
                                    }
                                    TrackHeaderAction::ResizeTrack(new_h) => {
                                        self.canvas_state.set_track_height(&t_id, new_h);
                                    }
                                    TrackHeaderAction::DeleteTrack => {
                                        let cmd = Box::new(RemoveTrackCommand::new(t_id.clone()));
                                        match self.undo_stack.push(cmd, &mut self.project.timeline)
                                        {
                                            Ok(_) => {
                                                self.canvas_state
                                                    .selected_clip_pairs
                                                    .retain(|(tr_id, _)| tr_id != &t_id);
                                                self.status_message = "Kanal silindi".to_string();
                                                self.project.mark_dirty();
                                            }
                                            Err(e) => {
                                                self.status_message =
                                                    format!("Kanal silinemedi: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        });
                    });

                // Forward mouse wheel scrolling over track headers to canvas scroll state
                if ui
                    .input(|i| i.pointer.hover_pos())
                    .map(|p| headers_scroll_out.inner_rect.contains(p))
                    .unwrap_or(false)
                {
                    let scroll_delta = ui.input(|i| i.raw_scroll_delta);
                    if scroll_delta.y != 0.0 {
                        let max_scroll = (total_tracks_h - content_viewport_h).max(0.0);
                        let new_y = (self.canvas_state.scroll_offset.y - scroll_delta.y)
                            .clamp(0.0, max_scroll);
                        self.canvas_state.pending_scroll_y = Some(new_y);
                    }
                }
            });
        });

        if add_video_requested {
            self.add_video_track();
        }
        if add_audio_requested {
            self.add_audio_track();
        }
        if add_adjustment_layer_requested {
            let t0 = self.current_time.max(0.0);
            let t1 = t0 + 5.0;
            let target_track_id = self
                .project
                .timeline
                .get_video_tracks()
                .into_iter()
                .rev()
                .find(|t| {
                    !t.locked
                        && !t
                            .clips
                            .iter()
                            .any(|c| c.timeline_in < t1 && c.timeline_out > t0)
                })
                .map(|t| t.id.clone())
                .unwrap_or_else(|| self.project.timeline.add_new_video_track());
            let adj = Clip::new_adjustment_layer(t0, t1);
            let adj_id = adj.id.clone();
            let cmd = Box::new(trim_commands::AddAdjustmentLayerCommand::new(
                &target_track_id,
                adj,
            ));
            if self
                .undo_stack
                .push(cmd, &mut self.project.timeline)
                .is_ok()
            {
                self.canvas_state.selected_clip_pairs = vec![(target_track_id, adj_id)];
                self.project.mark_dirty();
                self.last_rendered_monitor_frame = None;
                self.video_engine.last_rendered_frame = None;
                self.scopes_panel.mark_dirty();
                self.status_message = "✨ Ayar Katmanı eklendi".to_string();
            }
        }
        if reset_heights_requested {
            self.canvas_state.track_heights.clear();
        }
    }
}
