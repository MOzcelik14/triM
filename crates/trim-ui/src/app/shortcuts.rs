use super::TrimApp;
use crate::dialogs::PendingCloseAction;
use crate::shortcuts::ShortcutProfile;
use crate::toolbar::EditTool;
use egui::Key;
use trim_commands::commands::*;
use trim_timeline::{
    find_next_edit_point, find_prev_edit_point, ripple_trim_head, ripple_trim_tail,
    split_at_playhead,
};

impl TrimApp {
    pub(super) fn handle_global_shortcuts(&mut self, ctx: &egui::Context) {
        // -------------------------------------------------------------
        // Global Keyboard Shortcuts
        // -------------------------------------------------------------
        let mut do_copy = false;
        let mut do_cut = false;
        let mut do_paste = false;
        let mut multicam_angle_key: Option<usize> = None;

        if !ctx.wants_keyboard_input()
            && !self.welcome_dialog.is_open
            && !self.splash_screen.is_active
        {
            ctx.input(|i| {
                let is_paste_attrs = (i.modifiers.ctrl || i.modifiers.command)
                    && i.modifiers.alt
                    && i.key_pressed(Key::V);
                let is_copy = i.events.iter().any(|e| matches!(e, egui::Event::Copy))
                    || ((i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(Key::C));
                let is_paste = !is_paste_attrs
                    && (i.events.iter().any(|e| matches!(e, egui::Event::Paste(_)))
                        || ((i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(Key::V)));
                let is_cut = i.events.iter().any(|e| matches!(e, egui::Event::Cut))
                    || ((i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(Key::X));

                if is_paste_attrs {
                    self.paste_attrs_dialog_open = true;
                } else if self.multicam_mode && !i.modifiers.ctrl && !i.modifiers.alt {
                    if i.key_pressed(Key::Num1) {
                        multicam_angle_key = Some(0);
                    } else if i.key_pressed(Key::Num2) {
                        multicam_angle_key = Some(1);
                    } else if i.key_pressed(Key::Num3) {
                        multicam_angle_key = Some(2);
                    } else if i.key_pressed(Key::Num4) {
                        multicam_angle_key = Some(3);
                    }
                }

                if i.key_pressed(Key::F11) {
                    self.is_monitor_fullscreen = !self.is_monitor_fullscreen;
                } else if self.is_monitor_fullscreen && i.key_pressed(Key::Escape) {
                    self.is_monitor_fullscreen = false;
                } else if (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(Key::Comma) {
                    self.preferences_dialog.open(&self.preferences);
                } else if is_copy {
                    do_copy = true;
                } else if is_paste {
                    do_paste = true;
                } else if is_cut {
                    do_cut = true;
                } else if i.key_pressed(Key::Space) {
                    self.toggle_play();
                } else if i.key_pressed(Key::Comma) {
                    self.insert_source_to_timeline(false);
                } else if i.key_pressed(Key::Period) {
                    self.insert_source_to_timeline(true);
                } else if i.modifiers.alt && i.key_pressed(Key::X) {
                    self.project.timeline.clear_in_out();
                    self.project.mark_dirty();
                    self.status_message = "Giriş/Çıkış Noktaları Temizlendi".to_string();
                } else if i.key_pressed(Key::I) && !i.modifiers.ctrl {
                    if self.source_state.media_id.is_some() {
                        self.source_state.mark_in = Some(self.source_state.current_time);
                        self.status_message =
                            format!("Kaynak Giriş: {:.2}s", self.source_state.current_time);
                    } else {
                        self.project.timeline.set_in_point(self.current_time);
                        self.project.mark_dirty();
                        self.status_message =
                            format!("Giriş Noktası (In): {:.2}s", self.current_time);
                    }
                } else if i.key_pressed(Key::O) && !i.modifiers.ctrl {
                    if self.source_state.media_id.is_some() {
                        self.source_state.mark_out = Some(self.source_state.current_time);
                        self.status_message =
                            format!("Kaynak Çıkış: {:.2}s", self.source_state.current_time);
                    } else {
                        self.project.timeline.set_out_point(self.current_time);
                        self.project.mark_dirty();
                        self.status_message =
                            format!("Çıkış Noktası (Out): {:.2}s", self.current_time);
                    }
                } else if i.modifiers.ctrl
                    && i.key_pressed(Key::K)
                    && self.shortcut_profile == ShortcutProfile::PremierePro
                {
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
                } else if i.modifiers.ctrl
                    && (i.key_pressed(Key::Backslash) || i.key_pressed(Key::B))
                    && self.shortcut_profile == ShortcutProfile::DaVinciResolve
                {
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
                } else if i.key_pressed(Key::V)
                    && !i.modifiers.ctrl
                    && self.shortcut_profile != ShortcutProfile::DaVinciResolve
                {
                    self.current_edit_tool = EditTool::Select;
                    self.canvas_state.active_tool = EditTool::Select;
                    self.status_message = "Seçim Aracı (V) etkin".to_string();
                } else if i.key_pressed(Key::A) && !i.modifiers.ctrl {
                    if self.shortcut_profile == ShortcutProfile::DaVinciResolve {
                        self.current_edit_tool = EditTool::Select;
                        self.status_message = "Seçim Aracı (A) etkin".to_string();
                    } else {
                        self.current_edit_tool = EditTool::TrackSelect;
                        self.status_message = "İz Seçme Aracı (A) etkin".to_string();
                    }
                    self.canvas_state.active_tool = self.current_edit_tool;
                } else if i.key_pressed(Key::B) && !i.modifiers.ctrl {
                    if self.shortcut_profile == ShortcutProfile::DaVinciResolve {
                        self.current_edit_tool = EditTool::Razor;
                        self.status_message = "Jilet / Kesme Aracı (Blade - B) etkin".to_string();
                    } else {
                        self.current_edit_tool = EditTool::Ripple;
                        self.status_message = "Dalgalanma Düzenleme (Ripple - B) etkin".to_string();
                    }
                    self.canvas_state.active_tool = self.current_edit_tool;
                } else if i.key_pressed(Key::C) && !i.modifiers.ctrl {
                    self.current_edit_tool = EditTool::Razor;
                    self.canvas_state.active_tool = EditTool::Razor;
                    self.status_message = "Jilet / Kesme Aracı (Razor - C) etkin".to_string();
                } else if i.key_pressed(Key::Y) && !i.modifiers.ctrl {
                    self.current_edit_tool = EditTool::Slip;
                    self.canvas_state.active_tool = EditTool::Slip;
                    self.status_message = "Kaydırma Aracı (Slip - Y) etkin".to_string();
                } else if i.key_pressed(Key::R) && !i.modifiers.ctrl {
                    self.current_edit_tool = EditTool::Roll;
                    self.canvas_state.active_tool = EditTool::Roll;
                    self.status_message = "Yuvarlama Aracı (Roll - R) etkin".to_string();
                } else if i.key_pressed(Key::U) && !i.modifiers.ctrl {
                    self.current_edit_tool = EditTool::Slide;
                    self.canvas_state.active_tool = EditTool::Slide;
                    self.status_message = "Sürükleme Aracı (Slide - U) etkin".to_string();
                } else if i.key_pressed(Key::P) && !i.modifiers.ctrl {
                    self.current_edit_tool = EditTool::Pen;
                    self.canvas_state.active_tool = EditTool::Pen;
                    self.status_message = "Kalem / Anahtar Kare Aracı (Pen - P) etkin".to_string();
                } else if i.key_pressed(Key::H) && !i.modifiers.ctrl {
                    self.current_edit_tool = EditTool::Hand;
                    self.canvas_state.active_tool = EditTool::Hand;
                    self.status_message = "El Aracı (Hand - H) etkin".to_string();
                } else if i.key_pressed(Key::T) && !i.modifiers.ctrl {
                    self.current_edit_tool = EditTool::Type;
                    self.canvas_state.active_tool = EditTool::Type;
                    self.status_message =
                        "Metin Aracı (Type - T) etkin: Video ekranına tıklayarak yazı ekleyin"
                            .to_string();
                } else if i.key_pressed(Key::S)
                    && !i.modifiers.ctrl
                    && self.shortcut_profile == ShortcutProfile::TrimDefault
                {
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
                } else if i.key_pressed(Key::Q) && !i.modifiers.ctrl {
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
                    ripple_trim_head(
                        &mut self.project.timeline,
                        &mut self.undo_stack,
                        preferred_track,
                        preferred_clip,
                        self.current_time,
                    );
                    self.project.mark_dirty();
                } else if i.key_pressed(Key::W) && !i.modifiers.ctrl {
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
                    ripple_trim_tail(
                        &mut self.project.timeline,
                        &mut self.undo_stack,
                        preferred_track,
                        preferred_clip,
                        self.current_time,
                    );
                    self.project.mark_dirty();
                } else if i.key_pressed(Key::M) && !i.modifiers.ctrl {
                    self.add_or_toggle_marker();
                } else if i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace) {
                    let is_ripple = i.modifiers.shift
                        || (self.shortcut_profile == ShortcutProfile::DaVinciResolve
                            && i.key_pressed(Key::Delete));
                    if self.canvas_state.selected_gap.is_some() {
                        self.close_selected_gap(true);
                    } else if is_ripple {
                        if let Some((tr_id, cl_id)) =
                            self.canvas_state.selected_clip_pairs.first().cloned()
                        {
                            let is_locked = self
                                .project
                                .timeline
                                .get_track_by_id(&tr_id)
                                .map(|t| t.locked)
                                .unwrap_or(false);
                            if !is_locked {
                                let cmd = Box::new(RippleDeleteCommand::new(tr_id, cl_id));
                                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                self.canvas_state.selected_clip_pairs.clear();
                                self.project.mark_dirty();
                            } else {
                                self.status_message =
                                    "Kilitli kanaldaki klip silinemez".to_string();
                            }
                        }
                    } else if !self.canvas_state.selected_clip_pairs.is_empty() {
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
                            let cmd = Box::new(RemoveMultipleClipsCommand::new(unlocked_pairs));
                            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                            self.canvas_state.selected_clip_pairs.clear();
                            self.project.mark_dirty();
                        } else {
                            self.status_message = "Kilitli kanaldaki klipler silinemez".to_string();
                        }
                    }
                } else if i.key_pressed(Key::ArrowLeft) {
                    if i.modifiers.shift {
                        self.seek(self.current_time - 1.0);
                    } else {
                        self.step_frame(-1);
                    }
                } else if i.key_pressed(Key::ArrowRight) {
                    if i.modifiers.shift {
                        self.seek(self.current_time + 1.0);
                    } else {
                        self.step_frame(1);
                    }
                } else if i.key_pressed(Key::ArrowUp) {
                    let prev_pt = find_prev_edit_point(&self.project.timeline, self.current_time);
                    self.seek(prev_pt);
                } else if i.key_pressed(Key::ArrowDown) {
                    let next_pt = find_next_edit_point(&self.project.timeline, self.current_time);
                    self.seek(next_pt);
                } else if i.key_pressed(Key::Home) {
                    self.seek(0.0);
                } else if i.key_pressed(Key::End) {
                    self.seek(self.project.timeline.duration());
                } else if (i.modifiers.ctrl || i.modifiers.command)
                    && (i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals))
                    || (!i.modifiers.ctrl
                        && !i.modifiers.command
                        && !i.modifiers.alt
                        && (i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals)))
                {
                    self.zoom_timeline(1.25, None, None);
                } else if (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(Key::Minus)
                    || (!i.modifiers.ctrl
                        && !i.modifiers.command
                        && !i.modifiers.alt
                        && i.key_pressed(Key::Minus))
                {
                    self.zoom_timeline(0.8, None, None);
                } else if !i.modifiers.ctrl
                    && (i.key_pressed(Key::N)
                        || (i.key_pressed(Key::S)
                            && self.shortcut_profile != ShortcutProfile::TrimDefault))
                {
                    self.toggle_snap();
                } else if i.key_pressed(Key::J) && !i.modifiers.ctrl {
                    self.shuttle_reverse();
                } else if i.key_pressed(Key::K) && !i.modifiers.ctrl {
                    self.shuttle_stop();
                } else if i.key_pressed(Key::L) && !i.modifiers.ctrl {
                    self.shuttle_forward();
                } else if (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::Z))
                    || (i.modifiers.ctrl && i.key_pressed(Key::Y))
                {
                    self.perform_redo();
                } else if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(Key::Z) {
                    self.perform_undo();
                } else if i.key_pressed(Key::F1) {
                    self.shortcuts_dialog.toggle();
                } else if i.modifiers.ctrl && i.key_pressed(Key::S) {
                    self.save_project_dialog();
                } else if i.modifiers.ctrl && i.key_pressed(Key::O) {
                    if self.has_unsaved_changes() {
                        let name = self.project.settings.name.clone();
                        self.close_confirm_dialog
                            .prompt(PendingCloseAction::OpenProject, name);
                    } else {
                        self.open_project_dialog();
                    }
                } else if i.modifiers.ctrl && i.key_pressed(Key::N) {
                    if self.has_unsaved_changes() {
                        let name = self.project.settings.name.clone();
                        self.close_confirm_dialog
                            .prompt(PendingCloseAction::NewProject, name);
                    } else {
                        self.reset_to_new_project();
                    }
                } else if i.modifiers.ctrl && i.key_pressed(Key::Q) {
                    if self.has_unsaved_changes() && !self.confirmed_exit {
                        let name = self.project.settings.name.clone();
                        self.close_confirm_dialog
                            .prompt(PendingCloseAction::ExitApp, name);
                    } else {
                        self.confirmed_exit = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                } else if i.modifiers.ctrl && i.key_pressed(Key::L) {
                    self.toggle_link_selected_clips();
                } else if i.modifiers.ctrl && i.key_pressed(Key::I) {
                    self.prompt_import_media();
                } else if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::E) {
                    self.render_queue_dialog.toggle();
                } else if i.modifiers.ctrl && (i.key_pressed(Key::E) || i.key_pressed(Key::M)) {
                    self.export_dialog.open(&self.project);
                } else if i.modifiers.ctrl && i.key_pressed(Key::T) {
                    self.title_dialog.open();
                } else if (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(Key::R) {
                    if let Some((tr_id, cl_id)) =
                        self.canvas_state.selected_clip_pairs.first().cloned()
                    {
                        if let Some(cl) = self
                            .project
                            .timeline
                            .get_track_by_id(&tr_id)
                            .and_then(|t| t.get_clip_by_id(&cl_id))
                        {
                            self.speed_dialog.open(
                                tr_id,
                                cl_id,
                                cl.speed,
                                cl.reverse,
                                cl.duration(),
                            );
                        }
                    } else {
                        self.status_message =
                            "Hız değiştirmek için önce bir klip seçin".to_string();
                    }
                }
            });

            if do_copy {
                self.copy_selected_clips();
                ctx.output_mut(|o| {
                    o.copied_text = format!("trim://clips/{}", self.clipboard_clips.len());
                });
            }
            if do_cut {
                self.copy_selected_clips();
                self.delete_selected_clips();
                ctx.output_mut(|o| {
                    o.copied_text = format!("trim://clips/{}", self.clipboard_clips.len());
                });
            }
            if do_paste {
                self.paste_clipboard_clips();
            }
            if let Some(angle) = multicam_angle_key {
                self.handle_multicam_angle_switch(angle);
            }
        }
    }
}
