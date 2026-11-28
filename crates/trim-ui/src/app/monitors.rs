use crate::dialogs::*;
use crate::monitor::{ActiveTextOverlay, MonitorAction, MonitorMode, MonitorWidget};
use crate::toolbar::EditTool;

use egui::Color32;
use std::time::Instant;

use trim_commands::commands::*;
use trim_core::time::seconds_to_timecode;

use super::TrimApp;

impl TrimApp {
    pub fn create_subclip_from_source_monitor(&mut self) {
        let Some(media_id) = self.monitor.source_state.media_id.clone() else {
            return;
        };
        let Some(media) = self.project.get_media(&media_id).cloned() else {
            return;
        };
        let sin = self.monitor.source_state.mark_in.unwrap_or(0.0);
        let sout = self.monitor.source_state.mark_out.unwrap_or(media.duration);
        let idx = self.project.subclips.len() + 1;
        let name = format!("{} [Alt-Klip {}]", media.name, idx);
        if let Some(sub) = self
            .project
            .create_subclip_from_range(&media_id, name, sin, sout)
        {
            self.status_message = match self.lang() {
                crate::i18n::Language::Turkish => format!(
                    "✂ Alt-Klip oluşturuldu: {} ({:.2}s – {:.2}s)",
                    sub.name, sub.source_in, sub.source_out
                ),
                crate::i18n::Language::English => format!(
                    "✂ Subclip created: {} ({:.2}s – {:.2}s)",
                    sub.name, sub.source_in, sub.source_out
                ),
            };
        }
    }

    pub fn render_source_monitor(&mut self, source_ui: &mut egui::Ui) {
        let source_media = self
            .monitor
            .source_state
            .media_id
            .as_ref()
            .and_then(|id| self.project.get_media(id));
        let source_title = source_media
            .map(|m| m.name.as_str())
            .unwrap_or("Seçili Klip Yok");
        let source_dur = source_media.map(|m| m.duration).unwrap_or(0.0);
        let source_fps = source_media
            .map(|m| m.fps)
            .unwrap_or(self.project.timeline.fps);
        let source_tc = seconds_to_timecode(self.monitor.source_state.current_time, source_fps);
        let source_dur_tc = seconds_to_timecode(source_dur, source_fps);
        let source_aspect = source_media
            .map(|m| (m.width as f32 / m.height.max(1) as f32).max(0.1))
            .unwrap_or(16.0 / 9.0);
        let source_label =
            source_media.map(|m| format!("{}x{} • {:.0}fps", m.width, m.height, m.fps));

        let (_s_resp, s_act) = MonitorWidget::new(
            MonitorMode::Source,
            source_title,
            self.monitor.source_state.texture.as_ref(),
            source_media.is_some(),
            source_aspect,
            self.monitor.source_state.current_time,
            source_dur,
            self.monitor.source_state.mark_in,
            self.monitor.source_state.mark_out,
            &source_tc,
            &source_dur_tc,
            self.monitor.source_state.is_playing,
            1.0,
            source_label.as_deref(),
            false,
        )
        .show(source_ui);

        if let Some(act) = s_act {
            match act {
                MonitorAction::ImportMedia => self.prompt_import_media(),
                MonitorAction::TogglePlay => {
                    if self.monitor.source_state.is_playing {
                        self.monitor.source_state.is_playing = false;
                        self.monitor.source_state.clock_start = None;
                        self.playback.audio_player.stop();
                        self.playback.vu_state.reset();
                    } else {
                        if self.playback.is_playing {
                            self.pause();
                        }
                        if source_dur > 0.0 && self.monitor.source_state.current_time >= source_dur
                        {
                            self.monitor.source_state.current_time =
                                self.monitor.source_state.mark_in.unwrap_or(0.0);
                        }
                        self.monitor.source_state.is_playing = true;
                        self.monitor.source_state.clock_start = Some(Instant::now());
                        self.monitor.source_state.clock_start_time =
                            self.monitor.source_state.current_time;
                        self.playback.audio_player.prepare();
                    }
                }
                MonitorAction::StepFrame(d) => {
                    if self.monitor.source_state.is_playing {
                        self.monitor.source_state.is_playing = false;
                        self.monitor.source_state.clock_start = None;
                        self.playback.audio_player.stop();
                        self.playback.vu_state.reset();
                    }
                    let new_t = (self.monitor.source_state.current_time
                        + (d as f64 / source_fps.max(1.0)))
                    .clamp(0.0, source_dur);
                    self.monitor.source_state.current_time = new_t;
                }
                MonitorAction::MarkIn => {
                    self.monitor.source_state.mark_in =
                        Some(self.monitor.source_state.current_time);
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => {
                            format!(
                                "Kaynak Giriş: {:.2}s",
                                self.monitor.source_state.current_time
                            )
                        }
                        crate::i18n::Language::English => {
                            format!("Source In: {:.2}s", self.monitor.source_state.current_time)
                        }
                    };
                }
                MonitorAction::MarkOut => {
                    self.monitor.source_state.mark_out =
                        Some(self.monitor.source_state.current_time);
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => {
                            format!(
                                "Kaynak Çıkış: {:.2}s",
                                self.monitor.source_state.current_time
                            )
                        }
                        crate::i18n::Language::English => {
                            format!("Source Out: {:.2}s", self.monitor.source_state.current_time)
                        }
                    };
                }
                MonitorAction::GoToIn => {
                    self.monitor.source_state.current_time =
                        self.monitor.source_state.mark_in.unwrap_or(0.0);
                    if self.monitor.source_state.is_playing {
                        self.monitor.source_state.clock_start = Some(Instant::now());
                        self.monitor.source_state.clock_start_time =
                            self.monitor.source_state.current_time;
                    }
                }
                MonitorAction::GoToOut => {
                    self.monitor.source_state.current_time =
                        self.monitor.source_state.mark_out.unwrap_or(source_dur);
                    if self.monitor.source_state.is_playing {
                        self.monitor.source_state.clock_start = Some(Instant::now());
                        self.monitor.source_state.clock_start_time =
                            self.monitor.source_state.current_time;
                    }
                }
                MonitorAction::InsertToTimeline => self.insert_source_to_timeline(false),
                MonitorAction::OverwriteToTimeline => self.insert_source_to_timeline(true),
                MonitorAction::CreateSubclip => self.create_subclip_from_source_monitor(),
                MonitorAction::Seek(t) => {
                    if self.monitor.source_state.is_playing {
                        self.monitor.source_state.is_playing = false;
                        self.monitor.source_state.clock_start = None;
                        self.playback.audio_player.stop();
                        self.playback.vu_state.reset();
                    }
                    self.monitor.source_state.current_time = t.clamp(0.0, source_dur);
                }
                _ => {}
            }
        }
    }

    pub fn collect_active_text_overlays(&self) -> Vec<ActiveTextOverlay> {
        let mut active_text_overlays = Vec::new();
        for track in self.project.timeline.get_video_tracks().iter().rev() {
            if !track.visible {
                continue;
            }
            if let Some(c) = track.find_clip_at(self.playback.current_time) {
                if let Some(txt) = &c.text_overlay {
                    let dt = self.playback.current_time - c.timeline_in;
                    let rem = c.timeline_out - self.playback.current_time;
                    let mut a = c.opacity as f32;
                    if c.fade_in > 0.0 && dt < c.fade_in {
                        a *= (dt / c.fade_in) as f32;
                    }
                    if c.fade_out > 0.0 && rem < c.fade_out {
                        a *= (rem / c.fade_out) as f32;
                    }
                    let text_color = Color32::from_rgba_unmultiplied(
                        txt.text_color[0],
                        txt.text_color[1],
                        txt.text_color[2],
                        txt.text_color[3],
                    );
                    let dur = c.duration().max(0.01);
                    let progress = (dt / dur).clamp(0.0, 1.0) as f32;
                    let highlight_color = Color32::from_rgba_unmultiplied(
                        txt.highlight_color[0],
                        txt.highlight_color[1],
                        txt.highlight_color[2],
                        txt.highlight_color[3],
                    );
                    let bg_color = txt.bg_color.map(|b| {
                        Color32::from_rgba_unmultiplied(b[0], b[1], b[2], (b[3] as f32 * a) as u8)
                    });
                    let stroke_color = Color32::from_rgba_unmultiplied(
                        txt.stroke_color[0],
                        txt.stroke_color[1],
                        txt.stroke_color[2],
                        txt.stroke_color[3],
                    );
                    active_text_overlays.push(ActiveTextOverlay {
                        text: txt.content.clone(),
                        font_size: txt.font_size,
                        text_color,
                        bg_color,
                        pos_x: txt.pos_x,
                        pos_y: txt.pos_y,
                        has_shadow: txt.has_shadow,
                        shadow_distance: txt.shadow_distance,
                        stroke_width: txt.stroke_width,
                        stroke_color,
                        letter_spacing: txt.letter_spacing,
                        alpha: a.clamp(0.0, 1.0),
                        caption_style: txt.caption_style,
                        highlight_color,
                        progress,
                        motion_style: txt.motion_style,
                        subtitle_text: txt.subtitle_text.clone(),
                        font_family: txt.font_family.clone(),
                        bold: txt.bold,
                        italic: txt.italic,
                    });
                }
            }
        }
        active_text_overlays
    }

    pub fn render_program_monitor(&mut self, prog_ui: &mut egui::Ui) {
        let has_clips = self
            .project
            .timeline
            .tracks
            .iter()
            .any(|t| !t.clips.is_empty());
        let prog_dur = self.project.timeline.duration();
        let prog_fps = self.project.timeline.fps;
        let prog_tc = seconds_to_timecode(self.playback.current_time, prog_fps);
        let prog_dur_tc = seconds_to_timecode(prog_dur, prog_fps);
        let prog_aspect =
            (self.project.timeline.width as f32 / self.project.timeline.height as f32).max(0.1);

        let top_clip = self
            .project
            .timeline
            .get_top_visible_video_clip_at(self.playback.current_time);
        let prog_label = top_clip
            .and_then(|(_, c)| self.project.get_media(&c.media_id))
            .map(|m| format!("{}x{} • {:.0}fps", m.width, m.height, m.fps));

        let active_text_overlays = self.collect_active_text_overlays();

        let selected_text_content =
            if let Some((tr_id, cl_id)) = self.canvas_state.selected_clip_pairs.first() {
                self.project
                    .timeline
                    .get_track_by_id(tr_id)
                    .and_then(|t| t.get_clip_by_id(cl_id))
                    .and_then(|c| c.text_overlay.as_ref())
                    .map(|txt| txt.content.clone())
            } else {
                top_clip
                    .and_then(|(_, c)| c.text_overlay.as_ref())
                    .map(|txt| txt.content.clone())
            };

        let active_mask_overlay =
            self.canvas_state
                .selected_clip_pairs
                .first()
                .and_then(|(tr_id, cl_id)| {
                    self.project
                        .timeline
                        .get_track_by_id(tr_id)
                        .and_then(|t| t.get_clip_by_id(cl_id))
                        .filter(|c| c.masks.iter().any(|m| m.enabled))
                        .map(|c| crate::monitor::ActiveMaskOverlay {
                            clip_id: c.id.clone(),
                            masks: c.masks.clone(),
                            clip_time: (self.playback.current_time - c.timeline_in).max(0.0),
                        })
                });

        let ref_tex = self
            .monitor
            .scopes_panel
            .active_reference_still
            .and_then(|i| self.monitor.scopes_panel.reference_stills.get(i))
            .and_then(|s| s.tex.as_ref())
            .or(self.monitor.source_state.texture.as_ref());

        let mut v_tracks = self.project.timeline.get_video_tracks();
        v_tracks.reverse();
        let multicam_labels: [String; 4] = std::array::from_fn(|idx| {
            v_tracks
                .get(idx)
                .and_then(|tr| {
                    tr.find_clip_at(self.playback.current_time)
                        .map(|cl| format!("{} • {}", tr.name, cl.name))
                        .or_else(|| Some(format!("{} (Boş)", tr.name)))
                })
                .unwrap_or_else(|| format!("V{} (Yok)", idx + 1))
        });

        let (_p_resp, p_act) = MonitorWidget::new(
            MonitorMode::Program,
            "Sekans 01",
            self.monitor.monitor_texture.as_ref(),
            has_clips,
            prog_aspect,
            self.playback.current_time,
            prog_dur,
            None,
            None,
            &prog_tc,
            &prog_dur_tc,
            self.playback.is_playing,
            self.playback.playback_speed,
            prog_label.as_deref(),
            self.media.video_engine.is_decoding_seek,
        )
        .custom_texture_id(self.monitor.monitor_gpu_texture_id)
        .safe_zones(self.monitor.show_safe_zones)
        .aspect_ratio_preset(Some(self.project.timeline.aspect_ratio()))
        .text_overlays(active_text_overlays)
        .active_tool(self.tools.current_edit_tool)
        .inline_text_edit(selected_text_content)
        .mask_overlay(active_mask_overlay)
        .split_wipe(
            self.monitor.split_wipe_enabled,
            self.monitor.split_wipe_pos,
            ref_tex,
        )
        .multicam(
            self.monitor.multicam_mode,
            self.monitor.multicam_active_angle,
            multicam_labels.to_vec(),
        )
        .wb_eyedropper(self.monitor.wb_eyedropper_target.is_some())
        .show(prog_ui);

        if let Some(act) = p_act {
            match act {
                MonitorAction::NewProject => {
                    if self.has_unsaved_changes() {
                        let name = self.project.settings.name.clone();
                        self.dialogs
                            .close_confirm_dialog
                            .prompt(PendingCloseAction::NewProject, name);
                    } else {
                        self.reset_to_new_project();
                    }
                }
                MonitorAction::OpenProject => {
                    if self.has_unsaved_changes() {
                        let name = self.project.settings.name.clone();
                        self.dialogs
                            .close_confirm_dialog
                            .prompt(PendingCloseAction::OpenProject, name);
                    } else {
                        self.open_project_dialog();
                    }
                }
                MonitorAction::ImportMedia => self.prompt_import_media(),
                MonitorAction::TogglePlay => self.toggle_play(),
                MonitorAction::StepFrame(d) => self.step_frame(d),
                MonitorAction::MarkIn => {
                    self.project
                        .timeline
                        .set_in_point(self.playback.current_time);
                    self.project.mark_dirty();
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => {
                            format!("Giriş Noktası (In): {:.2}s", self.playback.current_time)
                        }
                        crate::i18n::Language::English => {
                            format!("In Point: {:.2}s", self.playback.current_time)
                        }
                    };
                }
                MonitorAction::MarkOut => {
                    self.project
                        .timeline
                        .set_out_point(self.playback.current_time);
                    self.project.mark_dirty();
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => {
                            format!("Çıkış Noktası (Out): {:.2}s", self.playback.current_time)
                        }
                        crate::i18n::Language::English => {
                            format!("Out Point: {:.2}s", self.playback.current_time)
                        }
                    };
                }
                MonitorAction::GoToIn => {
                    let t = self.project.timeline.in_point.unwrap_or(0.0);
                    self.seek(t);
                }
                MonitorAction::GoToOut => {
                    let t = self.project.timeline.out_point.unwrap_or(prog_dur);
                    self.seek(t);
                }
                MonitorAction::ExportFrame => self.export_current_frame(),
                MonitorAction::Seek(t) => self.seek(t),
                MonitorAction::ToggleSafeZones => {
                    self.monitor.show_safe_zones = !self.monitor.show_safe_zones;
                }
                MonitorAction::SetAspectRatio(r) => {
                    self.project.timeline.set_aspect_ratio(r);
                    self.project.sync_settings_from_timeline();
                    self.project.mark_dirty();
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => {
                            format!("Format Değiştirildi: {}", r.label())
                        }
                        crate::i18n::Language::English => {
                            format!("Aspect Ratio Changed: {}", r.label())
                        }
                    };
                }
                MonitorAction::TextToolClick { pos_x, pos_y } => {
                    self.handle_monitor_text_click(pos_x, pos_y);
                }
                MonitorAction::TextPositionDrag { pos_x, pos_y } => {
                    if self.monitor.monitor_text_drag_orig_pos.is_none() {
                        if let Some((tr_id, cl_id)) = self.canvas_state.selected_clip_pairs.first()
                        {
                            if let Some(tr) = self.project.timeline.get_track_by_id(tr_id) {
                                if let Some(cl) = tr.get_clip_by_id(cl_id) {
                                    if let Some(ref txt) = cl.text_overlay {
                                        self.monitor.monitor_text_drag_orig_pos = Some((
                                            tr_id.clone(),
                                            cl_id.clone(),
                                            (txt.pos_x, txt.pos_y),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    self.handle_monitor_text_drag(pos_x, pos_y);
                }
                MonitorAction::TextPositionDragEnded => {
                    if let Some((tr_id, cl_id, orig_pos)) =
                        self.monitor.monitor_text_drag_orig_pos.take()
                    {
                        if let Some(tr) = self.project.timeline.get_track_by_id(&tr_id) {
                            if let Some(cl) = tr.get_clip_by_id(&cl_id) {
                                if let Some(ref txt) = cl.text_overlay {
                                    let cur_pos = (txt.pos_x, txt.pos_y);
                                    if (cur_pos.0 - orig_pos.0).abs() > 1e-4
                                        || (cur_pos.1 - orig_pos.1).abs() > 1e-4
                                    {
                                        let cmd = Box::new(
                                            trim_commands::SetTextOverlayPositionCommand::new(
                                                tr_id, cl_id, orig_pos, cur_pos,
                                            ),
                                        );
                                        let _ =
                                            self.undo_stack.push(cmd, &mut self.project.timeline);
                                        self.project.mark_dirty();
                                    }
                                }
                            }
                        }
                    }
                }
                MonitorAction::TextSelectAt { pos_x, pos_y } => {
                    self.handle_monitor_text_select(pos_x, pos_y);
                }
                MonitorAction::TextDoubleClick { pos_x, pos_y } => {
                    self.handle_monitor_text_select(pos_x, pos_y);
                    self.tools.current_edit_tool = EditTool::Type;
                    self.canvas_state.active_tool = EditTool::Type;
                    self.set_status("Metin Düzenleme Modu (T)", "Text Edit Mode (T)");
                }
                MonitorAction::ToggleFullscreen => {
                    self.monitor.is_monitor_fullscreen = !self.monitor.is_monitor_fullscreen;
                }
                MonitorAction::MasksEdited {
                    clip_id,
                    old_masks,
                    new_masks,
                    commit_undo,
                } => {
                    if let Some((tr_id, _)) = self
                        .canvas_state
                        .selected_clip_pairs
                        .iter()
                        .find(|(_, cid)| *cid == clip_id)
                        .cloned()
                    {
                        if commit_undo {
                            let cmd = Box::new(trim_commands::SetClipMasksCommand::new(
                                tr_id, clip_id, old_masks, new_masks,
                            ));
                            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                        } else if let Some(cl) = self
                            .project
                            .timeline
                            .get_track_by_id_mut(&tr_id)
                            .and_then(|t| t.get_clip_by_id_mut(&clip_id))
                        {
                            cl.masks = new_masks;
                        }
                        self.project.mark_dirty();
                        self.monitor.last_rendered_monitor_frame = None;
                        self.media.video_engine.last_rendered_frame = None;
                        self.monitor.scopes_panel.mark_dirty();
                    }
                }
                MonitorAction::TextContentEdited(content) => {
                    if self.monitor.monitor_text_edit_orig_content.is_none() {
                        if let Some((tr_id, cl_id)) = self.canvas_state.selected_clip_pairs.first()
                        {
                            if let Some(tr) = self.project.timeline.get_track_by_id(tr_id) {
                                if let Some(cl) = tr.get_clip_by_id(cl_id) {
                                    if let Some(ref txt) = cl.text_overlay {
                                        self.monitor.monitor_text_edit_orig_content = Some((
                                            tr_id.clone(),
                                            cl_id.clone(),
                                            txt.content.clone(),
                                            cl.name.clone(),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    self.handle_monitor_text_content_changed(content);
                }
                MonitorAction::TextEditingCompleted => {
                    if let Some((tr_id, cl_id, orig_content, orig_name)) =
                        self.monitor.monitor_text_edit_orig_content.take()
                    {
                        if let Some(tr) = self.project.timeline.get_track_by_id(&tr_id) {
                            if let Some(cl) = tr.get_clip_by_id(&cl_id) {
                                if let Some(ref txt) = cl.text_overlay {
                                    if txt.content != orig_content {
                                        let cmd = Box::new(
                                            trim_commands::SetTextOverlayContentCommand::new(
                                                tr_id,
                                                cl_id,
                                                orig_content,
                                                txt.content.clone(),
                                                orig_name,
                                                cl.name.clone(),
                                            ),
                                        );
                                        let _ =
                                            self.undo_stack.push(cmd, &mut self.project.timeline);
                                        self.project.mark_dirty();
                                    }
                                }
                            }
                        }
                    }
                }
                MonitorAction::ToggleSplitWipe => {
                    self.monitor.split_wipe_enabled = !self.monitor.split_wipe_enabled;
                    if self.monitor.split_wipe_enabled {
                        self.set_status(
                            "◧ Bölünmüş Ekran (Before/After Wipe) Açık",
                            "◧ Split Screen (Before/After Wipe) On",
                        );
                    } else {
                        self.set_status(
                            "◧ Bölünmüş Ekran (Before/After Wipe) Kapalı",
                            "◧ Split Screen (Before/After Wipe) Off",
                        );
                    }
                }
                MonitorAction::SplitWipePosChanged(pos) => {
                    self.monitor.split_wipe_pos = pos.clamp(0.05, 0.95);
                }
                MonitorAction::ToggleMulticamView => {
                    self.monitor.multicam_mode = !self.monitor.multicam_mode;
                    if self.monitor.multicam_mode {
                        self.set_status(
                            "⊞ Multicam 2x2 Modu Açık (1–4 tuşlarıyla kamera açısı kesin)",
                            "⊞ Multicam 2x2 Mode On (use keys 1–4 to cut angles)",
                        );
                    } else {
                        self.set_status("⊞ Multicam 2x2 Modu Kapalı", "⊞ Multicam 2x2 Mode Off");
                    }
                }
                MonitorAction::MulticamAngleSelected(angle) => {
                    self.handle_multicam_angle_switch(angle);
                }
                MonitorAction::HoverPixelSample { x_norm, y_norm }
                    if self.monitor.scopes_panel.is_open =>
                {
                    if let Some((rgba, rw, rh)) = &self.monitor.last_scope_rgba {
                        self.monitor.scopes_panel.active_pixel_sample =
                            trim_core::scopes::sample_pixel_rgba(rgba, *rw, *rh, x_norm, y_norm);
                    } else if let Some(frame) = &self.monitor.last_rendered_monitor_frame {
                        self.monitor.scopes_panel.active_pixel_sample =
                            trim_core::scopes::sample_pixel_rgba(
                                frame.as_raw(),
                                frame.width(),
                                frame.height(),
                                x_norm,
                                y_norm,
                            );
                    }
                }
                MonitorAction::CancelWbEyedropper => {
                    self.monitor.wb_eyedropper_target = None;
                    self.set_status("WB Damlalığı iptal edildi.", "WB Eyedropper cancelled.");
                }
                MonitorAction::PickWhiteBalancePixel { x_norm, y_norm } => {
                    if let Some((track_id, clip_id)) = self.monitor.wb_eyedropper_target.take() {
                        let sample = if let Some((rgba, rw, rh)) = &self.monitor.last_scope_rgba {
                            trim_core::scopes::sample_pixel_rgba(rgba, *rw, *rh, x_norm, y_norm)
                        } else if let Some(frame) = &self.monitor.last_rendered_monitor_frame {
                            trim_core::scopes::sample_pixel_rgba(
                                frame.as_raw(),
                                frame.width(),
                                frame.height(),
                                x_norm,
                                y_norm,
                            )
                        } else {
                            None
                        };
                        if let Some(ro) = sample {
                            let (temp, tint) = trim_core::color::compute_wb_from_neutral_rgb(
                                ro.rgb[0], ro.rgb[1], ro.rgb[2],
                            );
                            if let Some(clip_ref) = self
                                .project
                                .timeline
                                .get_track_by_id(&track_id)
                                .and_then(|t| t.get_clip_by_id(&clip_id))
                                .cloned()
                            {
                                let old_state = ClipColorGradingState::from_clip(&clip_ref);
                                let mut new_state = old_state.clone();
                                new_state.wb_preset = trim_core::clip::WhiteBalancePreset::Custom;
                                new_state.temperature = temp;
                                new_state.tint = tint;
                                let cmd = Box::new(SetClipColorGradingCommand::new(
                                    track_id, clip_id, old_state, new_state,
                                ));
                                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                self.project.mark_dirty();
                                self.monitor.last_rendered_monitor_frame = None;
                                self.media.video_engine.last_rendered_frame = None;
                                self.monitor.scopes_panel.mark_dirty();
                                self.status_message = match self.lang() {
                                    crate::i18n::Language::Turkish => format!(
                                        "🎯 WB Damlalığı uygulandı (RGB: {},{},{} → Sıcaklık: {:+.1}, Ton: {:+.1})",
                                        ro.rgb[0], ro.rgb[1], ro.rgb[2], temp, tint
                                    ),
                                    crate::i18n::Language::English => format!(
                                        "🎯 WB Eyedropper applied (RGB: {},{},{} → Temp: {:+.1}, Tint: {:+.1})",
                                        ro.rgb[0], ro.rgb[1], ro.rgb[2], temp, tint
                                    ),
                                };
                            }
                        } else {
                            self.set_status(
                                "⚠ Piksel rengi okunamadı (aktif kare yok).",
                                "⚠ Could not sample pixel color (no active frame).",
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
