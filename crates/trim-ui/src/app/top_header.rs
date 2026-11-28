use crate::dialogs::*;
use crate::i18n::Language;
use crate::shortcuts::ShortcutProfile;

use egui::{Button, Color32, RichText};

use super::TrimApp;

impl TrimApp {
    pub(super) fn render_top_header(&mut self, ctx: &egui::Context) {
        // -------------------------------------------------------------
        // Top Header: Premiere Pro Navigation Bar & Window Controls
        // -------------------------------------------------------------
        let lang = self.lang();
        egui::TopBottomPanel::top("top_header").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                // triM. Logo Branding
                ui.label(
                    RichText::new("triM.")
                        .size(15.0)
                        .strong()
                        .color(Color32::from_rgb(0xe0, 0x7a, 0x38)),
                );
                ui.add_space(4.0);

                // Dosya / File
                ui.menu_button(lang.t("menu_file"), |ui| {
                    if ui.button(lang.t("menu_welcome")).clicked() {
                        self.dialogs.welcome_dialog.open();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_new_project")).clicked() {
                        if self.has_unsaved_changes() {
                            let name = self.project.settings.name.clone();
                            self.dialogs
                                .close_confirm_dialog
                                .prompt(PendingCloseAction::NewProject, name);
                        } else {
                            self.reset_to_new_project();
                        }
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_open_project")).clicked() {
                        if self.has_unsaved_changes() {
                            let name = self.project.settings.name.clone();
                            self.dialogs
                                .close_confirm_dialog
                                .prompt(PendingCloseAction::OpenProject, name);
                        } else {
                            self.open_project_dialog();
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(lang.t("menu_save")).clicked() {
                        self.save_project_dialog();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_save_as")).clicked() {
                        self.save_project_as_dialog();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_bundle")).clicked() {
                        self.bundle_project_dialog();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_snapshots")).clicked() {
                        let snaps = self.persistence.autosave_mgr.list_snapshots();
                        self.dialogs.recovery_dialog.open_snapshot_browser(snaps);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(lang.t("menu_import_media")).clicked() {
                        self.prompt_import_media();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_relink")).clicked() {
                        self.dialogs.relink_dialog.open(&self.project);
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_import_interchange")).clicked() {
                        self.import_interchange_dialog();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_import_subtitles")).clicked() {
                        self.import_subtitles_dialog();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_export_subtitles")).clicked() {
                        self.export_subtitles_dialog();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_export_edl")).clicked() {
                        self.export_edl_dialog();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_export_fcpxml")).clicked() {
                        self.export_fcpxml_dialog();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_export_studio")).clicked() {
                        self.dialogs.export_dialog.open(&self.project);
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_render_queue")).clicked() {
                        self.dialogs.render_queue_dialog.open();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(lang.t("menu_exit")).clicked() {
                        if self.has_unsaved_changes() && !self.persistence.confirmed_exit {
                            let name = self.project.settings.name.clone();
                            self.dialogs
                                .close_confirm_dialog
                                .prompt(PendingCloseAction::ExitApp, name);
                        } else {
                            self.persistence.confirmed_exit = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        ui.close_menu();
                    }
                });

                // Düzen / Edit
                ui.menu_button(lang.t("menu_edit"), |ui| {
                    let can_undo = self.undo_stack.can_undo();
                    if ui
                        .add_enabled(can_undo, Button::new(lang.t("menu_undo")))
                        .clicked()
                    {
                        self.perform_undo();
                        ui.close_menu();
                    }
                    let can_redo = self.undo_stack.can_redo();
                    if ui
                        .add_enabled(can_redo, Button::new(lang.t("menu_redo")))
                        .clicked()
                    {
                        self.perform_redo();
                        ui.close_menu();
                    }
                    ui.separator();
                    let has_sel = !self.canvas_state.selected_clip_pairs.is_empty();
                    if ui
                        .add_enabled(has_sel, Button::new(lang.t("menu_cut")))
                        .clicked()
                    {
                        self.copy_selected_clips();
                        self.delete_selected_clips();
                        ctx.output_mut(|o| {
                            o.copied_text =
                                format!("trim://clips/{}", self.tools.clipboard_clips.len());
                        });
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(has_sel, Button::new(lang.t("menu_copy")))
                        .clicked()
                    {
                        self.copy_selected_clips();
                        ctx.output_mut(|o| {
                            o.copied_text =
                                format!("trim://clips/{}", self.tools.clipboard_clips.len());
                        });
                        ui.close_menu();
                    }
                    let can_paste = !self.tools.clipboard_clips.is_empty();
                    if ui
                        .add_enabled(can_paste, Button::new(lang.t("menu_paste")))
                        .clicked()
                    {
                        self.paste_clipboard_clips();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(lang.t("menu_delete")).clicked() {
                        self.delete_selected_clips();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_ripple_delete")).clicked() {
                        self.ripple_delete_selected_clips();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(lang.t("menu_select_all")).clicked() {
                        self.select_all_clips();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_deselect")).clicked() {
                        self.canvas_state.selected_clip_pairs.clear();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(lang.t("menu_link_clips")).clicked() {
                        self.toggle_link_selected_clips();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_silence_cut")).clicked() {
                        self.open_silence_cut_dialog_for_selected();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_transcribe")).clicked() {
                        self.open_transcribe_dialog_for_selected();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_screen_record")).clicked() {
                        self.open_screen_record_dialog();
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.menu_button(lang.t("menu_shortcut_profile"), |ui| {
                        for p in ShortcutProfile::ALL {
                            if ui
                                .selectable_label(self.tools.shortcut_profile == *p, p.label())
                                .clicked()
                            {
                                self.tools.shortcut_profile = *p;
                                self.tools.keymap_config =
                                    crate::shortcuts::KeymapConfig::for_profile(*p);
                                if let Err(e) = self
                                    .tools
                                    .keymap_config
                                    .save_to_path(&crate::shortcuts::KeymapConfig::config_path())
                                {
                                    log::warn!("Failed to save keybindings: {}", e);
                                }
                                self.status_message = match self.lang() {
                                    crate::i18n::Language::Turkish => {
                                        format!("Kısayol Profili: {}", p.label())
                                    }
                                    crate::i18n::Language::English => {
                                        format!("Shortcut Profile: {}", p.label())
                                    }
                                };
                                ui.close_menu();
                            }
                        }
                    });
                    ui.separator();
                    if ui.button(lang.t("menu_preferences")).clicked() {
                        self.dialogs.preferences_dialog.open(&self.preferences);
                        ui.close_menu();
                    }
                });

                // Sekans / Sequence
                ui.menu_button(lang.t("menu_sequence"), |ui| {
                    if ui.button(lang.t("menu_split_playhead")).clicked() {
                        self.split_at_playhead_command();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_smart_cut")).clicked() {
                        self.smart_scene_cut_selected_clip();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_marker")).clicked() {
                        self.add_marker_at_playhead();
                        ui.close_menu();
                    }
                    let snap_label = match lang {
                        Language::English => {
                            if self.tools.snap_enabled {
                                "Magnetic Snapping (N): Enabled"
                            } else {
                                "Magnetic Snapping (N): Disabled"
                            }
                        }
                        Language::Turkish => {
                            if self.tools.snap_enabled {
                                "Manyetik Yapışma (N): Açık"
                            } else {
                                "Manyetik Yapışma (N): Kapalı"
                            }
                        }
                    };
                    if ui.button(snap_label).clicked() {
                        self.toggle_snap();
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.menu_button(lang.t("menu_aspect_ratio"), |ui| {
                        let curr = self.project.timeline.aspect_ratio();
                        for r in [
                            trim_core::AspectRatio::Wide16x9,
                            trim_core::AspectRatio::Vertical9x16,
                            trim_core::AspectRatio::Square1x1,
                            trim_core::AspectRatio::Portrait4x5,
                            trim_core::AspectRatio::Classic4x3,
                            trim_core::AspectRatio::Cinema21x9,
                        ] {
                            if ui.selectable_label(curr == r, r.label()).clicked() {
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
                                ui.close_menu();
                            }
                        }
                    });
                    if ui
                        .checkbox(&mut self.monitor.show_safe_zones, lang.t("menu_safe_zones"))
                        .clicked()
                    {
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .checkbox(&mut self.media.use_proxies, lang.t("menu_proxies"))
                        .clicked()
                    {
                        self.media.video_engine.use_proxies = self.media.use_proxies;
                        self.media.video_engine.clear_cache();
                        self.media.last_rendered_dirty = true;
                        let state_str = match lang {
                            Language::English => {
                                if self.media.use_proxies {
                                    "Enabled"
                                } else {
                                    "Disabled"
                                }
                            }
                            Language::Turkish => {
                                if self.media.use_proxies {
                                    "Açık"
                                } else {
                                    "Kapalı"
                                }
                            }
                        };
                        self.status_message = match self.lang() {
                            Language::Turkish | Language::English => {
                                format!("⚡ Proxy: {}", state_str)
                            }
                        };
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(lang.t("menu_go_home")).clicked() {
                        self.seek(0.0);
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_go_end")).clicked() {
                        let dur = self.project.timeline.duration();
                        self.seek(dur);
                        ui.close_menu();
                    }
                });

                // Kanallar / Tracks
                ui.menu_button(lang.t("menu_tracks"), |ui| {
                    if ui.button(lang.t("menu_add_video_track")).clicked() {
                        self.add_video_track();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_add_audio_track")).clicked() {
                        self.add_audio_track();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(lang.t("menu_reset_track_heights")).clicked() {
                        self.canvas_state.track_heights.clear();
                        ui.close_menu();
                    }
                });

                // Pencere / Window
                ui.menu_button(lang.t("menu_window"), |ui| {
                    if ui
                        .checkbox(
                            &mut self.monitor.show_source_monitor,
                            lang.t("menu_source_monitor"),
                        )
                        .clicked()
                    {
                        ui.close_menu();
                    }
                    if ui
                        .checkbox(
                            &mut self.monitor.scopes_panel.is_open,
                            "📊 Renk Kapsamları (Scopes)",
                        )
                        .clicked()
                    {
                        if self.monitor.scopes_panel.is_open {
                            self.monitor.scopes_panel.mark_dirty();
                        }
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_audio_mixer")).clicked() {
                        self.dialogs.audio_mixer_dialog.toggle();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_reset_audio_device")).clicked() {
                        self.playback.audio_player.reset_device();
                        self.set_status(
                            "Ses çıkış cihazı yeniden başlatıldı",
                            "Audio output device reset",
                        );
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_title_editor")).clicked() {
                        self.dialogs.title_dialog.open();
                        ui.close_menu();
                    }
                    if ui.button(lang.t("menu_export_studio")).clicked() {
                        self.dialogs.export_dialog.open(&self.project);
                        ui.close_menu();
                    }
                });

                // Yardım / Help
                ui.menu_button(lang.t("menu_help"), |ui| {
                    if ui.button(lang.t("menu_shortcuts_f1")).clicked() {
                        self.dialogs.shortcuts_dialog.open();
                        ui.close_menu();
                    }
                    let update_lbl = match lang {
                        Language::Turkish => "↺ Güncellemeleri Kontrol Et (GitHub Releases)",
                        Language::English => "↺ Check for Updates (GitHub Releases)",
                    };
                    if ui.button(update_lbl).clicked() {
                        ctx.open_url(egui::OpenUrl::new_tab(
                            "https://github.com/MOzcelik14/triM/releases/latest",
                        ));
                        self.status_message = match self.lang() {
                            crate::i18n::Language::Turkish => format!(
                                "↺ Yüklü sürüm: v{} — GitHub Releases sayfası tarayıcıda açıldı.",
                                env!("CARGO_PKG_VERSION")
                            ),
                            crate::i18n::Language::English => format!(
                                "↺ Installed version: v{} — Opened GitHub Releases in browser.",
                                env!("CARGO_PKG_VERSION")
                            ),
                        };
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(lang.t("menu_about")).clicked() {
                        self.dialogs.show_about_dialog = true;
                        ui.close_menu();
                    }
                });

                // Right Side Controls: Settings + Quick Export + Mixer + Monitor view + Title
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let settings_btn = Button::new(RichText::new("⚙").size(13.0)).fill(
                        if self.dialogs.preferences_dialog.is_open {
                            Color32::from_rgb(0x28, 0x5a, 0x9e)
                        } else {
                            Color32::from_rgb(0x22, 0x22, 0x2c)
                        },
                    );
                    if ui
                        .add(settings_btn)
                        .on_hover_text(lang.t("menu_preferences"))
                        .clicked()
                    {
                        self.dialogs.preferences_dialog.open(&self.preferences);
                    }

                    let quick_exp_btn = Button::new(
                        RichText::new(lang.t("btn_export"))
                            .color(Color32::WHITE)
                            .size(11.5)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(0x28, 0x5a, 0x9e));
                    if ui
                        .add(quick_exp_btn)
                        .on_hover_text(lang.t("menu_export_studio"))
                        .clicked()
                    {
                        self.dialogs.export_dialog.open(&self.project);
                    }

                    let mixer_btn = Button::new(RichText::new(lang.t("btn_mixer")).size(11.0))
                        .fill(if self.dialogs.audio_mixer_dialog.is_open {
                            Color32::from_rgb(0x2a, 0x54, 0x3d)
                        } else {
                            Color32::from_rgb(0x22, 0x22, 0x2c)
                        });
                    if ui
                        .add(mixer_btn)
                        .on_hover_text(lang.t("menu_audio_mixer"))
                        .clicked()
                    {
                        self.dialogs.audio_mixer_dialog.toggle();
                    }

                    let mon_label = match (lang, self.monitor.show_source_monitor) {
                        (Language::English, true) => "Single Monitor",
                        (Language::English, false) => "Dual Monitor",
                        (Language::Turkish, true) => "Tek Monitör",
                        (Language::Turkish, false) => "İkili Monitör",
                    };
                    let mon_btn = Button::new(RichText::new(mon_label).size(11.0)).fill(
                        if self.monitor.show_source_monitor {
                            Color32::from_rgb(0x2a, 0x3d, 0x54)
                        } else {
                            Color32::from_rgb(0x22, 0x22, 0x2c)
                        },
                    );
                    if ui
                        .add(mon_btn)
                        .on_hover_text(lang.t("menu_source_monitor"))
                        .clicked()
                    {
                        self.monitor.show_source_monitor = !self.monitor.show_source_monitor;
                    }

                    ui.separator();
                    for (layout, tr_lbl, en_lbl) in [
                        (super::WorkspaceLayout::Audio, "Ses", "Audio"),
                        (super::WorkspaceLayout::Color, "Renk", "Color"),
                        (super::WorkspaceLayout::Edit, "Kurgu", "Edit"),
                    ] {
                        let lbl = match lang {
                            Language::Turkish => tr_lbl,
                            Language::English => en_lbl,
                        };
                        let active = self.workspace_layout == layout;
                        let btn =
                            Button::new(RichText::new(lbl).size(11.0).strong()).fill(if active {
                                Color32::from_rgb(0x28, 0x5a, 0x9e)
                            } else {
                                Color32::from_rgb(0x22, 0x22, 0x2c)
                            });
                        if ui.add(btn).clicked() {
                            self.apply_workspace_layout(layout);
                        }
                    }

                    let dirty_str = if self.has_unsaved_changes() { " *" } else { "" };
                    let title_text = format!(
                        "{} — triM. v{}{}",
                        self.project.settings.name,
                        env!("CARGO_PKG_VERSION"),
                        dirty_str
                    );
                    ui.label(
                        RichText::new(title_text)
                            .size(11.5)
                            .color(Color32::from_rgb(0x90, 0x90, 0xa4)),
                    );
                });
            });
        });
    }
}
