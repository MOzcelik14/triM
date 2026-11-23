use crate::canvas::{calculate_total_tracks_height, render_timeline_canvas, CanvasAction};
use crate::ruler::{render_time_ruler, RulerAction};
use crate::toolbar::render_vertical_toolbar;
use crate::track_header::{render_track_header, TrackHeaderAction, TRACK_HEADER_WIDTH};
use crate::vu_meter::VuMeterWidget;

use egui::{Button, Color32, RichText, Stroke, Vec2};

use trim_commands::commands::*;
use trim_core::clip::Clip;
use trim_core::media::MediaType;
use trim_core::time::seconds_to_timecode;
use trim_core::track::TrackType;
use trim_timeline::split_at_playhead;

use super::TrimApp;

impl TrimApp {
    pub(super) fn render_timeline_panel(&mut self, ctx: &egui::Context) {
        // -------------------------------------------------------------
        // BOTTOM TIMELINE PANEL: Guaranteed permanently anchored at the bottom
        // -------------------------------------------------------------
        egui::TopBottomPanel::bottom("timeline_panel")
            .resizable(true)
            .min_height(200.0)
            .default_height(340.0)
            .max_height((ctx.screen_rect().height() - 150.0).max(400.0))
            .show(ctx, |ui| {
                // Sequence Timeline Header Bar
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;

                    self.project.ensure_sequences();

                    let mut seq_to_switch = None;
                    let mut seq_to_create = false;
                    let mut seq_to_remove = None;
                    let mut seq_to_duplicate = None;

                    for seq in &self.project.sequences {
                        let is_active = self.project.active_sequence_id.as_deref() == Some(&seq.id);

                        let fill_col = if is_active {
                            Color32::from_rgb(0x28, 0x2c, 0x3c)
                        } else {
                            Color32::from_rgb(0x18, 0x1a, 0x22)
                        };

                        let border_stroke = if is_active {
                            Stroke::new(1.5_f32, Color32::from_rgb(0x38, 0x7a, 0xe0))
                        } else {
                            Stroke::new(1.0_f32, Color32::from_rgb(0x28, 0x2c, 0x38))
                        };

                        let text_col = if is_active {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(0x9a, 0xa2, 0xb8)
                        };

                        let tab_btn = Button::new(
                            RichText::new(&seq.name)
                                .size(11.5)
                                .strong()
                                .color(text_col),
                        )
                        .fill(fill_col)
                        .stroke(border_stroke);

                        let resp = ui.add(tab_btn);
                        if resp.clicked() && !is_active {
                            seq_to_switch = Some(seq.id.clone());
                        }

                        resp.context_menu(|ui| {
                            if ui.button("+ Sekansı Çoğalt (Duplicate)").clicked() {
                                seq_to_duplicate = Some(seq.id.clone());
                                ui.close_menu();
                            }
                            if self.project.sequences.len() > 1 && ui.button("✕ Bu Sekansı Kapat").clicked() {
                                seq_to_remove = Some(seq.id.clone());
                                ui.close_menu();
                            }
                        });
                    }

                    // Button to add new sequence
                    let add_seq_btn = Button::new(
                        RichText::new("+")
                            .size(12.0)
                            .strong()
                            .color(Color32::from_rgb(0xa0, 0xa8, 0xc0)),
                    )
                    .fill(Color32::from_rgb(0x18, 0x1a, 0x22))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x28, 0x2c, 0x38)));

                    if ui.add(add_seq_btn).on_hover_text("Yeni Sekans Oluştur").clicked() {
                        seq_to_create = true;
                    }

                    if let Some(target_id) = seq_to_switch {
                        self.project.switch_sequence(&target_id);
                        self.video_engine.clear_cache();
                        self.audio_mixer.clear_cache();
                        self.canvas_state.selected_clip_pairs.clear();
                        self.current_time = 0.0;
                    }

                    if seq_to_create {
                        let next_num = self.project.sequences.len() + 1;
                        let new_name = format!("Sekans {:02}", next_num);
                        self.project.create_sequence(new_name);
                        self.video_engine.clear_cache();
                        self.audio_mixer.clear_cache();
                        self.canvas_state.selected_clip_pairs.clear();
                        self.current_time = 0.0;
                    }

                    if let Some(rem_id) = seq_to_remove {
                        self.project.remove_sequence(&rem_id);
                        self.video_engine.clear_cache();
                        self.audio_mixer.clear_cache();
                        self.canvas_state.selected_clip_pairs.clear();
                        self.current_time = 0.0;
                    }

                    if let Some(dup_id) = seq_to_duplicate {
                        self.project.duplicate_sequence(&dup_id);
                    }

                    ui.add_space(8.0);
                    let current_tc =
                        seconds_to_timecode(self.current_time, self.project.timeline.fps);
                    ui.label(
                        RichText::new(&current_tc)
                            .monospace()
                            .size(15.0)
                            .color(Color32::from_rgb(0x38, 0xa0, 0xf0))
                            .strong(),
                    );

                    ui.add_space(10.0);
                    let snap_btn =
                        Button::new(RichText::new("Mıknatıs").size(11.0).strong().color(
                            if self.snap_enabled {
                                Color32::WHITE
                            } else {
                                Color32::from_rgb(0x90, 0x94, 0xa6)
                            },
                        ))
                        .fill(if self.snap_enabled {
                            Color32::from_rgb(0x28, 0x5a, 0x9e)
                        } else {
                            Color32::from_rgb(0x20, 0x20, 0x26)
                        })
                        .stroke(Stroke::new(
                            1.0_f32,
                            if self.snap_enabled {
                                Color32::from_rgb(0x4a, 0x8a, 0xe8)
                            } else {
                                Color32::from_rgb(0x30, 0x32, 0x3e)
                            },
                        ));
                    if ui
                        .add(snap_btn)
                        .on_hover_text("Mıknatıs / Hizalama (Snapping - N)")
                        .clicked()
                    {
                        self.toggle_snap();
                    }

                    if ui
                        .button(RichText::new("İşaretçi").size(11.0))
                        .on_hover_text("İşaretçi Ekle / Sil (M)")
                        .clicked()
                    {
                        self.add_or_toggle_marker();
                    }

                    if ui
                        .button(RichText::new("+ Video").size(11.0))
                        .on_hover_text("Yeni Video Kanalı Ekle (Üste)")
                        .clicked()
                    {
                        self.add_video_track();
                    }

                    if ui
                        .button(RichText::new("+ Ses").size(11.0))
                        .on_hover_text("Yeni Ses Kanalı Ekle (Alta)")
                        .clicked()
                    {
                        self.add_audio_track();
                    }

                    if ui
                        .button(
                            RichText::new("+ Başlık")
                                .size(11.0)
                                .color(Color32::from_rgb(0x81, 0x8c, 0xf8)),
                        )
                        .on_hover_text("Zaman Çizgisine Yeni Başlık Klibi Ekle (Title Clip Modal)")
                        .clicked()
                    {
                        self.title_dialog.open();
                    }

                    ui.add_space(6.0);
                    let proxy_btn = Button::new(
                        RichText::new(if self.use_proxies {
                            "◈ Proxy: Açık"
                        } else {
                            "◈ Proxy: Kapalı"
                        })
                        .size(11.0)
                        .color(if self.use_proxies {
                            Color32::from_rgb(0x38, 0xa0, 0xf0)
                        } else {
                            Color32::from_rgb(0x70, 0x70, 0x80)
                        }),
                    )
                    .fill(if self.use_proxies {
                        Color32::from_rgb(0x18, 0x28, 0x3e)
                    } else {
                        Color32::from_rgb(0x20, 0x20, 0x26)
                    });
                    if ui
                        .add(proxy_btn)
                        .on_hover_text("Düşük Donanımlar İçin Akıllı 720p Proxy Önizleme")
                        .clicked()
                    {
                        self.toggle_proxy();
                    }

                    ui.add_space(4.0);
                    let rec_btn = Button::new(
                        RichText::new("● Ekran Kaydet")
                            .size(11.0)
                            .color(Color32::from_rgb(0xe0, 0x80, 0x40)),
                    )
                    .fill(Color32::from_rgb(0x28, 0x20, 0x1e));
                    if ui
                        .add(rec_btn)
                        .on_hover_text("Dahili Ekran ve Web Kamerası Kaydedici")
                        .clicked()
                    {
                        self.open_screen_record_dialog();
                    }

                    if let Some(ref vo) = self.active_voiceover {
                        let vo_dur = vo.recorder.elapsed_seconds();
                        let vo_rec_btn = Button::new(
                            RichText::new(format!("● SES KAYDI: {:.1}s [⏹ Durdur]", vo_dur))
                                .size(11.0)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(0xd0, 0x24, 0x24));
                        if ui.add(vo_rec_btn).clicked() {
                            self.finish_voiceover();
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        ui.add_sized(
                            [70.0, 18.0],
                            egui::Slider::new(
                                &mut self.canvas_state.pixels_per_second,
                                15.0..=350.0,
                            )
                            .show_value(false),
                        );
                        ui.label(
                            RichText::new("Zoom:")
                                .size(11.0)
                                .color(Color32::from_rgb(0x80, 0x80, 0x90)),
                        );
                    });
                });

                ui.add_space(2.0);
                ui.separator();

                // Main Timeline Row: [Toolbar] [Track Headers] [Canvas + Ruler] [Master VU Meter]
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;

                    let top_bar_h = 26.0;
                    let total_tracks_h = calculate_total_tracks_height(&self.project.timeline.tracks, &self.canvas_state);
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

                    let current_scroll_x = self.canvas_state.pending_scroll_x.unwrap_or(self.canvas_state.scroll_offset.x);
                    let current_scroll_y = self.canvas_state.scroll_offset.y;

                    // 1. Vertical Toolbar (Fixed width 42px for 32x32 tools)
                    ui.allocate_ui(Vec2::new(42.0, ui.available_height()), |ui| {
                        render_vertical_toolbar(ui, &mut self.current_edit_tool);
                    });

                    ui.add_space(2.0);
                    ui.separator();

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
                            let (corner_rect, _) = ui.allocate_exact_size(
                                Vec2::new(header_w, top_bar_h),
                                egui::Sense::hover(),
                            );
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

                            let mut corner_ui =
                                ui.new_child(egui::UiBuilder::new().max_rect(corner_rect));
                            corner_ui.spacing_mut().item_spacing = Vec2::ZERO;
                            corner_ui.horizontal_centered(|cui| {
                                cui.add_space(8.0);
                                cui.label(
                                    RichText::new("≡ KANALLAR")
                                        .size(10.5)
                                        .strong()
                                        .color(Color32::from_rgb(0x94, 0x98, 0xa8)),
                                );

                                cui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |cui| {
                                        cui.add_space(6.0);
                                        cui.menu_button(
                                            RichText::new("+ Kanal")
                                                .size(10.0)
                                                .color(Color32::from_rgb(0x81, 0x8c, 0xf8))
                                                .strong(),
                                            |menu_ui| {
                                                if menu_ui.button("▸ Video Kanalı Ekle").clicked()
                                                {
                                                    add_video_requested = true;
                                                    menu_ui.close_menu();
                                                }
                                                if menu_ui.button("♪ Ses Kanalı Ekle").clicked()
                                                {
                                                    add_audio_requested = true;
                                                    menu_ui.close_menu();
                                                }
                                                if menu_ui.button("◈ Ayar Katmanı Ekle").clicked()
                                                {
                                                    add_adjustment_layer_requested = true;
                                                    menu_ui.close_menu();
                                                }
                                                menu_ui.separator();
                                                if menu_ui
                                                    .button("↕ Yükseklikleri Sıfırla")
                                                    .clicked()
                                                {
                                                    reset_heights_requested = true;
                                                    menu_ui.close_menu();
                                                }
                                            },
                                        );
                                    },
                                );
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
                            if ui.input(|i| i.pointer.hover_pos()).map(|p| headers_scroll_out.inner_rect.contains(p)).unwrap_or(false) {
                                let scroll_delta = ui.input(|i| i.raw_scroll_delta);
                                if scroll_delta.y != 0.0 {
                                    let max_scroll = (total_tracks_h - content_viewport_h).max(0.0);
                                    let new_y = (self.canvas_state.scroll_offset.y - scroll_delta.y).clamp(0.0, max_scroll);
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
                            .find(|t| !t.locked && !t.clips.iter().any(|c| c.timeline_in < t1 && c.timeline_out > t0))
                            .map(|t| t.id.clone())
                            .unwrap_or_else(|| self.project.timeline.add_new_video_track());
                        let adj = Clip::new_adjustment_layer(t0, t1);
                        let adj_id = adj.id.clone();
                        let cmd = Box::new(trim_commands::AddAdjustmentLayerCommand::new(
                            &target_track_id,
                            adj,
                        ));
                        if self.undo_stack.push(cmd, &mut self.project.timeline).is_ok() {
                            self.canvas_state.selected_clip_pairs =
                                vec![(target_track_id, adj_id)];
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
                                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                                    let markers = self.project.timeline.markers.clone();
                                    let (_ruler_resp, ruler_act) = render_time_ruler(
                                        ui,
                                        self.canvas_state.pixels_per_second,
                                        current_scroll_x,
                                        self.project.timeline.duration(),
                                        self.project.timeline.fps,
                                        self.current_time,
                                        &markers,
                                        self.project.timeline.in_point,
                                        self.project.timeline.out_point,
                                        Some(&|t| {
                                            self.video_engine
                                                .is_timeline_time_cached(&self.project, t)
                                        }),
                                    );
                                    if !self.is_playing && !self.video_engine.is_decoding_seek {
                                        let step_t = (1.0 / self.project.timeline.fps.max(24.0))
                                            .clamp(0.03, 0.1);
                                        for k in 1..=6 {
                                            let probe_t = self.current_time + k as f64 * step_t;
                                            if self
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
                            self.canvas_state.active_tool = self.current_edit_tool;
                            self.project.timeline.snap_enabled = self.snap_enabled;
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
                                    self.current_time,
                                    &self.waveform_gen,
                                    &self.media_bin_textures,
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
                            let (ctrl, raw_scroll, zoom_delta) = ui.input(|i| (
                                i.modifiers.ctrl || i.modifiers.command,
                                i.raw_scroll_delta,
                                i.zoom_delta(),
                            ));

                            if is_hovering_canvas && ctrl && (raw_scroll.y != 0.0 || zoom_delta != 1.0) {
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
                                    let time = (content_x / self.canvas_state.pixels_per_second).max(0.0) as f64;
                                    self.zoom_timeline(factor, Some(time), Some(view_x));
                                }
                            }

                            if let Some(act) = canvas_scroll_out.inner {
                                    match act {
                                        CanvasAction::Seek(t) => self.seek(t),
                                        CanvasAction::SelectClips(pairs) => {
                                            self.canvas_state.selected_clip_pairs = pairs;
                                        }
                                        CanvasAction::SelectClip(tr_id, cl_id) => {
                                            self.canvas_state.selected_clip_pairs =
                                                vec![(tr_id, cl_id)];
                                        }
                                        CanvasAction::SetClipVolume {
                                            track_id,
                                            clip_id,
                                            old_volume,
                                            new_volume,
                                        } => {
                                            let cmd = Box::new(SetClipVolumeCommand::new(
                                                track_id, clip_id, old_volume, new_volume,
                                            ));
                                            let _ = self
                                                .undo_stack
                                                .push(cmd, &mut self.project.timeline);
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::SetClipFade {
                                            track_id,
                                            clip_id,
                                            is_fade_in,
                                            old_fade,
                                            new_fade,
                                        } => {
                                            let cmd = Box::new(trim_commands::SetClipFadeCommand::new(
                                                track_id,
                                                clip_id,
                                                is_fade_in,
                                                old_fade,
                                                new_fade,
                                            ));
                                            let _ = self
                                                .undo_stack
                                                .push(cmd, &mut self.project.timeline);
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::SetClipKeyframes {
                                            track_id,
                                            clip_id,
                                            property,
                                            old_keyframes,
                                            new_keyframes,
                                        } => {
                                            let cmd = Box::new(SetClipKeyframesCommand::new(
                                                track_id,
                                                clip_id,
                                                property,
                                                old_keyframes,
                                                new_keyframes,
                                            ));
                                            let _ = self
                                                .undo_stack
                                                .push(cmd, &mut self.project.timeline);
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::ClearSelection => {
                                            self.canvas_state.selected_clip_pairs.clear();
                                        }
                                        CanvasAction::ClipMoved {
                                            old_track_id,
                                            new_track_id,
                                            clip_id,
                                            old_in,
                                            new_in,
                                        } => {
                                            let cmd = Box::new(MoveClipCommand::new(
                                                old_track_id,
                                                new_track_id.clone(),
                                                clip_id.clone(),
                                                old_in,
                                                new_in,
                                            ));
                                            let _ = self
                                                .undo_stack
                                                .push(cmd, &mut self.project.timeline);
                                            self.canvas_state.selected_clip_pairs =
                                                vec![(new_track_id, clip_id)];
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::BatchClipsMoved(moves) => {
                                            let cmd =
                                                Box::new(MoveMultipleClipsCommand::new(moves));
                                            let _ = self
                                                .undo_stack
                                                .push(cmd, &mut self.project.timeline);
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::ClipTrimmed {
                                            track_id,
                                            clip_id,
                                            old_bounds,
                                            new_bounds,
                                            companions,
                                        } => {
                                            let cmd = Box::new(TrimClipCommand::with_companions(
                                                track_id,
                                                clip_id,
                                                old_bounds,
                                                new_bounds,
                                                companions,
                                            ));
                                            let _ = self
                                                .undo_stack
                                                .push(cmd, &mut self.project.timeline);
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::ClipRippleTrimmed {
                                            track_id,
                                            clip_id,
                                            old_bounds,
                                            new_bounds,
                                            shifted_clips,
                                            companions,
                                        } => {
                                            let cmd = Box::new(
                                                RippleTrimEdgeCommand::with_companions(
                                                    track_id,
                                                    clip_id,
                                                    old_bounds,
                                                    new_bounds,
                                                    shifted_clips,
                                                    companions,
                                                ),
                                            );
                                            let _ = self
                                                .undo_stack
                                                .push(cmd, &mut self.project.timeline);
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::ClipModified => {
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::PromptSpeed(tr_id, cl_id) => {
                                            if let Some(tr) =
                                                self.project.timeline.get_track_by_id(&tr_id)
                                            {
                                                if let Some(cl) = tr.get_clip_by_id(&cl_id) {
                                                    self.speed_dialog.open(
                                                        tr_id,
                                                        cl_id,
                                                        cl.speed,
                                                        cl.reverse,
                                                        cl.duration(),
                                                    );
                                                }
                                            }
                                        }
                                        CanvasAction::DetachAudio(tr_id, cl_id) => {
                                            let cmd =
                                                Box::new(DetachAudioCommand::new(tr_id, cl_id));
                                            let _ = self
                                                .undo_stack
                                                .push(cmd, &mut self.project.timeline);
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::SplitAtPlayhead => {
                                            let preferred_track = self
                                                .canvas_state
                                                .selected_clip_pairs
                                                .first()
                                                .map(|(t, _)| t.as_str());
                                            let preferred_clip = self
                                                .canvas_state
                                                .selected_clip_pairs
                                                .first()
                                                .map(|(_, c)| c.as_str());
                                            split_at_playhead(
                                                &mut self.project.timeline,
                                                &mut self.undo_stack,
                                                preferred_track,
                                                preferred_clip,
                                                self.current_time,
                                            );
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::RazorCut {
                                            track_id,
                                            clip_id,
                                            cut_time,
                                            split_linked,
                                        } => {
                                            let cmd = Box::new(
                                                SplitClipCommand::new(track_id, clip_id, cut_time)
                                                    .with_split_linked(split_linked),
                                            );
                                            match self
                                                .undo_stack
                                                .push(cmd, &mut self.project.timeline)
                                            {
                                                Ok(()) => {
                                                    self.status_message = format!(
                                                        "Klip {:.2}. saniyede kesildi",
                                                        cut_time
                                                    );
                                                    self.project.mark_dirty();
                                                }
                                                Err(e) => {
                                                    self.status_message =
                                                        format!("Kesme hatası: {}", e);
                                                }
                                            }
                                        }
                                        CanvasAction::CreateTitleAtTime(t) => {
                                            self.seek(t);
                                            self.title_dialog.open();
                                        }
                                        CanvasAction::Pan(_) => {}
                                        CanvasAction::CopySelected => {
                                            self.copy_selected_clips();
                                            ctx.output_mut(|o| {
                                                o.copied_text = format!("trim://clips/{}", self.clipboard_clips.len());
                                            });
                                        }
                                        CanvasAction::PasteClipboard => {
                                            self.paste_clipboard_clips();
                                        }
                                        CanvasAction::DeleteSelected => {
                                            let unlocked_pairs: Vec<(String, String)> = self
                                                .canvas_state
                                                .selected_clip_pairs
                                                .iter()
                                                .filter(|(tr_id, _)| {
                                                    !self
                                                        .project
                                                        .timeline
                                                        .get_track_by_id(tr_id)
                                                        .map(|t| t.locked)
                                                        .unwrap_or(false)
                                                })
                                                .cloned()
                                                .collect();
                                            if !unlocked_pairs.is_empty() {
                                                let cmd = Box::new(
                                                    RemoveMultipleClipsCommand::new(unlocked_pairs),
                                                );
                                                let _ = self
                                                    .undo_stack
                                                    .push(cmd, &mut self.project.timeline);
                                                self.canvas_state.selected_clip_pairs.clear();
                                                self.project.mark_dirty();
                                            } else if !self
                                                .canvas_state
                                                .selected_clip_pairs
                                                .is_empty()
                                            {
                                                self.status_message =
                                                    "Kilitli kanaldaki klipler silinemez"
                                                        .to_string();
                                            }
                                        }
                                        CanvasAction::RippleDeleteSelected => {
                                            if let Some((tr_id, cl_id)) = self
                                                .canvas_state
                                                .selected_clip_pairs
                                                .first()
                                                .cloned()
                                            {
                                                let is_locked = self
                                                    .project
                                                    .timeline
                                                    .get_track_by_id(&tr_id)
                                                    .map(|t| t.locked)
                                                    .unwrap_or(false);
                                                if !is_locked {
                                                    let cmd = Box::new(RippleDeleteCommand::new(
                                                        tr_id, cl_id,
                                                    ));
                                                    let _ = self
                                                        .undo_stack
                                                        .push(cmd, &mut self.project.timeline);
                                                    self.canvas_state.selected_clip_pairs.clear();
                                                    self.project.mark_dirty();
                                                } else {
                                                    self.status_message =
                                                        "Kilitli kanaldaki klip silinemez"
                                                            .to_string();
                                                }
                                            }
                                        }
                                        CanvasAction::MediaDropped {
                                            media_id,
                                            track_id,
                                            time,
                                        } => {
                                            let seq_id_opt = if let Some(stripped) = media_id.strip_prefix("seq:") {
                                                Some(stripped.to_string())
                                            } else if self.project.sequences.iter().any(|s| s.id == media_id) {
                                                Some(media_id.clone())
                                            } else {
                                                None
                                            };

                                            if let Some(seq_id) = seq_id_opt {
                                                if self.project.active_sequence_id.as_deref() == Some(&seq_id) {
                                                    self.status_message = "Bir sekans kendi içine yerleştirilemez (Nested Loop Engellendi)".to_string();
                                                } else if let Some(seq) = self.project.sequences.iter().find(|s| s.id == seq_id).cloned() {
                                                    let is_video_track = self
                                                        .project
                                                        .timeline
                                                        .get_track_by_id(&track_id)
                                                        .map(|t| t.track_type == TrackType::Video)
                                                        .unwrap_or(false);

                                                    let dur = seq.timeline.duration().max(0.1);
                                                    let has_audio = seq.timeline.get_audio_tracks().iter().any(|t| !t.clips.is_empty());

                                                    let (final_video_tr, final_audio_tr) = if is_video_track {
                                                        let a_tr = self.project.timeline.find_or_create_corresponding_audio_track(&track_id);
                                                        (track_id.clone(), a_tr)
                                                    } else {
                                                        let v_tr = self.project.timeline.find_or_create_corresponding_video_track(&track_id);
                                                        (v_tr, track_id.clone())
                                                    };

                                                    let link_group = if has_audio {
                                                        Some(uuid::Uuid::new_v4().to_string())
                                                    } else {
                                                        None
                                                    };

                                                    let mut clip = Clip::new_nested_sequence(&seq.id, &seq.name, time, time + dur);
                                                    clip.linked_group_id = link_group.clone();
                                                    let clip_id = clip.id.clone();
                                                    let cmd = Box::new(AddClipCommand::new(final_video_tr.clone(), clip));

                                                    if let Some(gid) = link_group {
                                                        let mut audio_clip = Clip::new_nested_sequence(&seq.id, format!("{} (Ses)", seq.name), time, time + dur);
                                                        audio_clip.linked_group_id = Some(gid);
                                                        let audio_clip_id = audio_clip.id.clone();
                                                        let cmd_a = Box::new(AddClipCommand::new(final_audio_tr.clone(), audio_clip));
                                                        let macro_cmd = Box::new(MacroCommand::new("İç İçe Sekans ve Ses Ekle", vec![cmd, cmd_a]));
                                                        let _ = self.undo_stack.push(macro_cmd, &mut self.project.timeline);
                                                        self.canvas_state.selected_clip_pairs = vec![
                                                            (final_video_tr, clip_id),
                                                            (final_audio_tr, audio_clip_id),
                                                        ];
                                                    } else {
                                                        let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                                        self.canvas_state.selected_clip_pairs = vec![
                                                            (final_video_tr, clip_id),
                                                        ];
                                                    }
                                                    self.status_message = format!("İç içe sekans eklendi: {}", seq.name);
                                                    self.project.mark_dirty();
                                                }
                                            } else if let Some(media) = self.project.get_media(&media_id).cloned() {
                                                let clip_dur =
                                                    if media.media_type == MediaType::Image {
                                                        5.0
                                                    } else {
                                                        media.duration.max(0.1)
                                                    };
                                                let media_fps = media.fps;
                                                let media_w = media.width;
                                                let media_h = media.height;
                                                let media_name = media.name.clone();
                                                let has_audio =
                                                    media.has_audio() || media.audio_channels > 0;
                                                let is_video_track = self
                                                    .project
                                                    .timeline
                                                    .get_track_by_id(&track_id)
                                                    .map(|t| t.track_type == TrackType::Video)
                                                    .unwrap_or(false);

                                                if self
                                                    .project
                                                    .timeline
                                                    .tracks
                                                    .iter()
                                                    .all(|t| t.clips.is_empty())
                                                {
                                                    if media_fps > 1.0 {
                                                        self.project.set_fps(media_fps);
                                                    }
                                                    if media_w > 0 && media_h > 0 {
                                                        self.project
                                                            .set_resolution(media_w, media_h);
                                                    }
                                                }

                                                let is_video_media = media.media_type == MediaType::Video
                                                    || (media.media_type != MediaType::Audio && media.media_type != MediaType::Image);

                                                if is_video_media {
                                                    let (final_video_tr, final_audio_tr) = if is_video_track {
                                                        let a_tr = self.project.timeline.find_or_create_corresponding_audio_track(&track_id);
                                                        (track_id.clone(), a_tr)
                                                    } else {
                                                        let v_tr = self.project.timeline.find_or_create_corresponding_video_track(&track_id);
                                                        (v_tr, track_id.clone())
                                                    };

                                                    let link_group = if has_audio {
                                                        Some(uuid::Uuid::new_v4().to_string())
                                                    } else {
                                                        None
                                                    };

                                                    let mut clip = Clip::new(&media_id, time, time + clip_dur);
                                                    clip.name = media_name.clone();
                                                    clip.linked_group_id = link_group.clone();
                                                    let clip_id = clip.id.clone();
                                                    let cmd = Box::new(AddClipCommand::new(
                                                        final_video_tr.clone(),
                                                        clip,
                                                    ));

                                                    if let Some(gid) = link_group {
                                                        let mut audio_clip = Clip::new(&media_id, time, time + clip_dur);
                                                        audio_clip.name = format!("{} (Ses)", media_name);
                                                        audio_clip.linked_group_id = Some(gid);
                                                        let audio_clip_id = audio_clip.id.clone();
                                                        let cmd_a = Box::new(AddClipCommand::new(
                                                            final_audio_tr.clone(),
                                                            audio_clip,
                                                        ));
                                                        let macro_cmd = Box::new(MacroCommand::new(
                                                            "Klip ve Ses Ekle",
                                                            vec![cmd, cmd_a],
                                                        ));
                                                        let _ = self.undo_stack.push(
                                                            macro_cmd,
                                                            &mut self.project.timeline,
                                                        );
                                                        self.canvas_state.selected_clip_pairs = vec![
                                                            (final_video_tr, clip_id),
                                                            (final_audio_tr, audio_clip_id),
                                                        ];
                                                    } else {
                                                        let _ = self
                                                            .undo_stack
                                                            .push(cmd, &mut self.project.timeline);
                                                        self.canvas_state.selected_clip_pairs = vec![
                                                            (final_video_tr, clip_id),
                                                        ];
                                                    }
                                                } else if media.media_type == MediaType::Audio {
                                                    let final_audio_tr = if is_video_track {
                                                        self.project.timeline.find_or_create_corresponding_audio_track(&track_id)
                                                    } else {
                                                        track_id.clone()
                                                    };
                                                    let mut audio_clip = Clip::new(&media_id, time, time + clip_dur);
                                                    audio_clip.name = media_name.clone();
                                                    let audio_clip_id = audio_clip.id.clone();
                                                    let cmd = Box::new(AddClipCommand::new(
                                                        final_audio_tr.clone(),
                                                        audio_clip,
                                                    ));
                                                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                                    self.canvas_state.selected_clip_pairs = vec![
                                                        (final_audio_tr, audio_clip_id),
                                                    ];
                                                } else {
                                                    // Image or other visual-only media
                                                    let final_video_tr = if is_video_track {
                                                        track_id.clone()
                                                    } else {
                                                        self.project.timeline.find_or_create_corresponding_video_track(&track_id)
                                                    };
                                                    let mut clip = Clip::new(&media_id, time, time + clip_dur);
                                                    clip.name = media_name.clone();
                                                    let clip_id = clip.id.clone();
                                                    let cmd = Box::new(AddClipCommand::new(
                                                        final_video_tr.clone(),
                                                        clip,
                                                    ));
                                                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                                    self.canvas_state.selected_clip_pairs = vec![
                                                        (final_video_tr, clip_id),
                                                    ];
                                                }

                                                self.project.mark_dirty();
                                            }
                                        }
                                        CanvasAction::CloseGap {
                                            track_id,
                                            gap_start,
                                            gap_end,
                                        } => {
                                            let dur = (gap_end - gap_start).max(0.0);
                                            let cmd = Box::new(CloseGapCommand::new(
                                                track_id, gap_start, gap_end,
                                            ));
                                            let _ = self
                                                .undo_stack
                                                .push(cmd, &mut self.project.timeline);
                                            self.canvas_state.selected_gap = None;
                                            self.status_message =
                                                format!("⏩ Boşluk kapatıldı ({:.2}s)", dur);
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::SetClipColor {
                                            track_id,
                                            clip_id,
                                            color,
                                        } => {
                                            let cmd = Box::new(SetClipColorCommand::new(
                                                track_id, clip_id, color,
                                            ));
                                            let _ = self
                                                .undo_stack
                                                .push(cmd, &mut self.project.timeline);
                                            self.status_message = format!(
                                                "🎨 Klip rengi güncellendi: {}",
                                                color.label()
                                            );
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::ToggleLinkSelected => {
                                            self.toggle_link_selected_clips();
                                        }
                                        CanvasAction::AlignSelectedByAudio => {
                                            self.align_selected_clips_by_audio();
                                        }
                                        CanvasAction::OpenSequence(seq_id) => {
                                            self.project.switch_sequence(&seq_id);
                                            self.video_engine.clear_cache();
                                            self.audio_mixer.clear_cache();
                                            self.canvas_state.selected_clip_pairs.clear();
                                            self.current_time = 0.0;
                                            self.status_message = "Sekans açıldı".to_string();
                                        }
                                        CanvasAction::CreateCompoundClip => {
                                            self.create_compound_clip_from_selected();
                                        }
                                        CanvasAction::JoinThroughEdits => {
                                            self.join_selected_through_edits();
                                        }
                                        CanvasAction::OpenPasteAttributesDialog => {
                                            self.paste_attrs_dialog_open = true;
                                        }
                                    }
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
                                ui.add(VuMeterWidget::new(&mut self.vu_state));
                            });
                        },
                    );
                });
            });
    }
}
