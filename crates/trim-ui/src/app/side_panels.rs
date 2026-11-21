use crate::inspector::{render_inspector_panel, InspectorAction};
use crate::media_bin::{render_media_bin, LeftPanelTab, MediaBinAction};

use egui::{Button, Color32, RichText, Stroke};

use trim_commands::commands::*;
use trim_core::clip::Clip;
use trim_core::media::MediaType;
use trim_core::track::TrackType;

use super::{BinDialogMode, BinDialogState, TrimApp};

impl TrimApp {
    pub(super) fn render_side_panels(&mut self, ctx: &egui::Context) {
        let lang = self.lang();
        // -------------------------------------------------------------
        // UPPER WORKSPACE:
        // Left: Media Bin / Inspector Panel (Resizable SidePanel)
        // Center: Video Monitors (Source / Program Monitor)
        // -------------------------------------------------------------
        egui::SidePanel::left("media_bin_panel")
            .resizable(true)
            .min_width(240.0)
            .default_width(320.0)
            .max_width(480.0)
            .show(ctx, |ui| {
                // Header Tabs: Proje | Efektler
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let is_proj = self.active_left_tab == LeftPanelTab::Project;
                    let is_eff = self.active_left_tab == LeftPanelTab::Effects;

                    let proj_btn = Button::new(RichText::new(lang.t("tab_project")).size(11.5).strong().color(
                        if is_proj {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(0x80, 0x80, 0x90)
                        },
                    ))
                    .fill(if is_proj {
                        Color32::from_rgb(0x28, 0x5a, 0x9e)
                    } else {
                        Color32::from_rgb(0x1a, 0x1a, 0x22)
                    })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if is_proj {
                            Color32::from_rgb(0x4a, 0x8a, 0xe8)
                        } else {
                            Color32::from_rgb(0x2c, 0x2c, 0x38)
                        },
                    ));
                    if ui.add(proj_btn).clicked() {
                        self.active_left_tab = LeftPanelTab::Project;
                    }

                    let eff_btn = Button::new(RichText::new(lang.t("tab_effects")).size(11.5).strong().color(
                        if is_eff {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(0x80, 0x80, 0x90)
                        },
                    ))
                    .fill(if is_eff {
                        Color32::from_rgb(0x28, 0x5a, 0x9e)
                    } else {
                        Color32::from_rgb(0x1a, 0x1a, 0x22)
                    })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if is_eff {
                            Color32::from_rgb(0x4a, 0x8a, 0xe8)
                        } else {
                            Color32::from_rgb(0x2c, 0x2c, 0x38)
                        },
                    ));
                    if ui.add(eff_btn).clicked() {
                        self.active_left_tab = LeftPanelTab::Effects;
                    }

                    if is_proj {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let import_btn = Button::new(
                                RichText::new("+ İçe Aktar")
                                    .color(Color32::WHITE)
                                    .size(11.0)
                                    .strong(),
                            )
                            .fill(Color32::from_rgb(0x28, 0x5a, 0x9e));
                            if ui
                                .add(import_btn)
                                .on_hover_text("Medyayı içe aktar (Ctrl+I)")
                                .clicked()
                            {
                                self.prompt_import_media();
                            }
                        });
                    }
                });

                ui.add_space(4.0);
                ui.separator();

                match self.active_left_tab {
                    LeftPanelTab::Project => {
                        if let Some(act) = render_media_bin(
                            ui,
                            &self.project,
                            &mut self.selected_media_id,
                            &mut self.media_bin_search,
                            &mut self.media_bin_is_grid,
                            &mut self.media_bin_thumb_size,
                            &self.media_bin_current_bin,
                            &mut self.media_bin_filter,
                            &self.media_bin_textures,
                        ) {
                            match act {
                                MediaBinAction::ImportRequested => self.prompt_import_media(),
                                MediaBinAction::OpenInSourceMonitor(m_id) => {
                                    self.source_state.media_id = Some(m_id);
                                    self.source_state.current_time = 0.0;
                                    self.source_state.mark_in = None;
                                    self.source_state.mark_out = None;
                                    self.source_state.is_playing = false;
                                    self.source_state.last_decoded_time = None;
                                    self.status_message =
                                        "Klip Kaynak Monitöründe Açıldı".to_string();
                                }
                                MediaBinAction::AddToTimeline(m_id) => {
                                    let seq_id_opt = if let Some(stripped) = m_id.strip_prefix("seq:") {
                                        Some(stripped.to_string())
                                    } else if self.project.sequences.iter().any(|s| s.id == m_id) {
                                        Some(m_id.clone())
                                    } else {
                                        None
                                    };

                                    if let Some(seq_id) = seq_id_opt {
                                        if self.project.active_sequence_id.as_deref() == Some(&seq_id) {
                                            self.status_message = "Bir sekans kendi içine yerleştirilemez (Nested Loop Engellendi)".to_string();
                                        } else if let Some(seq) = self.project.sequences.iter().find(|s| s.id == seq_id).cloned() {
                                            let target_video_track = if self
                                                .project
                                                .timeline
                                                .get_track_by_id(&self.target_video_track)
                                                .is_some()
                                            {
                                                Some(self.target_video_track.clone())
                                            } else {
                                                self.project
                                                    .timeline
                                                    .get_video_tracks()
                                                    .first()
                                                    .map(|t| t.id.clone())
                                            };

                                            if let Some(track_id) = target_video_track {
                                                let dur = seq.timeline.duration().max(0.1);
                                                let has_audio = seq.timeline.get_audio_tracks().iter().any(|t| !t.clips.is_empty());
                                                let link_group = if has_audio {
                                                    Some(uuid::Uuid::new_v4().to_string())
                                                } else {
                                                    None
                                                };

                                                let mut clip = Clip::new_nested_sequence(&seq.id, &seq.name, self.current_time, self.current_time + dur);
                                                clip.linked_group_id = link_group.clone();
                                                let cmd = Box::new(AddClipCommand::new(track_id.clone(), clip));

                                                if let Some(gid) = link_group {
                                                    let audio_track_id = self
                                                        .project
                                                        .timeline
                                                        .find_or_create_corresponding_audio_track(&track_id);
                                                    let mut audio_clip = Clip::new_nested_sequence(&seq.id, format!("{} (Ses)", seq.name), self.current_time, self.current_time + dur);
                                                    audio_clip.linked_group_id = Some(gid);
                                                    let cmd_a = Box::new(AddClipCommand::new(audio_track_id, audio_clip));
                                                    let macro_cmd = Box::new(MacroCommand::new("İç İçe Sekans ve Ses Ekle", vec![cmd, cmd_a]));
                                                    let _ = self.undo_stack.push(macro_cmd, &mut self.project.timeline);
                                                } else {
                                                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                                }
                                                self.status_message = format!("İç içe sekans eklendi: {}", seq.name);
                                                self.project.mark_dirty();
                                            }
                                        }
                                    } else {
                                        let media_info = self.project.get_media(&m_id).map(|media| {
                                        let clip_dur = if media.media_type == MediaType::Image {
                                            5.0
                                        } else {
                                            media.duration.max(0.1)
                                        };
                                        let has_audio =
                                            media.has_audio() || media.audio_channels > 0;
                                        (
                                            clip_dur,
                                            media.media_type,
                                            media.fps,
                                            media.width,
                                            media.height,
                                            media.name.clone(),
                                            has_audio,
                                        )
                                    });

                                    if let Some((
                                        clip_dur,
                                        media_type,
                                        media_fps,
                                        media_w,
                                        media_h,
                                        media_name,
                                        has_audio,
                                    )) = media_info
                                    {
                                        let target_track = if media_type == MediaType::Audio {
                                            if self
                                                .project
                                                .timeline
                                                .get_track_by_id(&self.target_audio_track)
                                                .is_some()
                                            {
                                                Some(self.target_audio_track.clone())
                                            } else {
                                                self.project
                                                    .timeline
                                                    .get_audio_tracks()
                                                    .first()
                                                    .map(|t| t.id.clone())
                                            }
                                        } else if self
                                            .project
                                            .timeline
                                            .get_track_by_id(&self.target_video_track)
                                            .is_some()
                                        {
                                            Some(self.target_video_track.clone())
                                        } else {
                                            self.project
                                                .timeline
                                                .get_video_tracks()
                                                .first()
                                                .map(|t| t.id.clone())
                                        };

                                        if let Some(track_id) = target_track {
                                            let is_locked = self
                                                .project
                                                .timeline
                                                .get_track_by_id(&track_id)
                                                .map(|t| t.locked)
                                                .unwrap_or(false);
                                            if is_locked {
                                                self.status_message =
                                                    "Hedef kanal kilitli, klip eklenemez"
                                                        .to_string();
                                            } else {
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
                                                let is_video_track = self
                                                    .project
                                                    .timeline
                                                    .get_track_by_id(&track_id)
                                                    .map(|t| t.track_type == TrackType::Video)
                                                    .unwrap_or(false);

                                                let link_group = if has_audio && is_video_track {
                                                    Some(uuid::Uuid::new_v4().to_string())
                                                } else {
                                                    None
                                                };

                                                let mut clip = Clip::new(
                                                    &m_id,
                                                    self.current_time,
                                                    self.current_time + clip_dur,
                                                );
                                                clip.name = media_name.clone();
                                                clip.linked_group_id = link_group.clone();
                                                let cmd = Box::new(AddClipCommand::new(
                                                    track_id.clone(),
                                                    clip,
                                                ));

                                                if let Some(gid) = link_group {
                                                    let audio_track_id = self
                                                        .project
                                                        .timeline
                                                        .find_or_create_corresponding_audio_track(
                                                            &track_id,
                                                        );
                                                    let mut audio_clip = Clip::new(
                                                        &m_id,
                                                        self.current_time,
                                                        self.current_time + clip_dur,
                                                    );
                                                    audio_clip.name =
                                                        format!("{} (Ses)", media_name);
                                                    audio_clip.linked_group_id = Some(gid);
                                                    let cmd_a = Box::new(AddClipCommand::new(
                                                        audio_track_id,
                                                        audio_clip,
                                                    ));
                                                    let macro_cmd = Box::new(MacroCommand::new(
                                                        "Klip ve Ses Ekle",
                                                        vec![cmd, cmd_a],
                                                    ));
                                                    let _ = self
                                                        .undo_stack
                                                        .push(macro_cmd, &mut self.project.timeline);
                                                } else {
                                                    let _ = self
                                                        .undo_stack
                                                        .push(cmd, &mut self.project.timeline);
                                                }

                                                self.project.mark_dirty();
                                            }
                                        }
                                    }
                                }
                            }
                            MediaBinAction::DeleteMedia(m_id) => {
                                    let is_used = self
                                        .project
                                        .timeline
                                        .tracks
                                        .iter()
                                        .any(|t| t.clips.iter().any(|c| c.media_id == m_id));
                                    if is_used {
                                        self.status_message =
                                            "Zaman çizgisinde kullanılan bir medya silinemez"
                                                .to_string();
                                    } else {
                                        self.project.media_pool.remove(&m_id);
                                        self.media_bin_textures.remove(&m_id);
                                        self.media_bin_textures.remove(&format!("strip:{m_id}"));
                                        if self.source_state.media_id.as_deref() == Some(&m_id) {
                                            self.source_state.media_id = None;
                                        }
                                        self.project.mark_dirty();
                                    }
                                }
                                MediaBinAction::GenerateProxy(m_id) => {
                                    self.generate_proxy_for_media(&m_id);
                                }
                                MediaBinAction::CreateBinPrompt => {
                                    self.bin_dialog = Some(BinDialogState {
                                        mode: BinDialogMode::Create,
                                        name_input: "Yeni Klasör".to_string(),
                                    });
                                }
                                MediaBinAction::RenameBinPrompt(bin_id) => {
                                    let current_name = self
                                        .project
                                        .get_bin(&bin_id)
                                        .map(|b| b.name.clone())
                                        .unwrap_or_default();
                                    self.bin_dialog = Some(BinDialogState {
                                        mode: BinDialogMode::Rename(bin_id),
                                        name_input: current_name,
                                    });
                                }
                                MediaBinAction::DeleteBin(bin_id) => {
                                    if let Some(removed) = self.project.remove_bin(&bin_id) {
                                        if self.media_bin_current_bin.as_deref() == Some(&bin_id) {
                                            self.media_bin_current_bin = None;
                                        }
                                        self.status_message = format!(
                                            "'{}' klasörü silindi (içindekiler ana havuza aktarıldı)",
                                            removed.name
                                        );
                                    }
                                }
                                MediaBinAction::EnterBin(bin_id) => {
                                    self.media_bin_current_bin = bin_id;
                                }
                                MediaBinAction::MoveToBin { media_id, bin_id } => {
                                    if let Some(item) = self.project.get_media_mut(&media_id) {
                                        item.bin_id = bin_id.clone();
                                        self.project.mark_dirty();
                                        let target_name = bin_id
                                            .as_deref()
                                            .and_then(|id| self.project.get_bin(id))
                                            .map(|b| b.name.as_str())
                                            .unwrap_or("Ana Havuz");
                                        self.status_message =
                                            format!("Medya '{}' klasörüne taşındı", target_name);
                                    }
                                }
                                MediaBinAction::OpenSequence(seq_id) => {
                                    self.project.switch_sequence(&seq_id);
                                    self.video_engine.clear_cache();
                                    self.audio_mixer.clear_cache();
                                    self.canvas_state.selected_clip_pairs.clear();
                                    self.current_time = 0.0;
                                    self.status_message = "Sekans açıldı".to_string();
                                }
                                MediaBinAction::CreateSequencePrompt => {
                                    let next_num = self.project.sequences.len() + 1;
                                    let new_name = format!("Sekans {:02}", next_num);
                                    self.project.create_sequence(new_name);
                                    self.video_engine.clear_cache();
                                    self.audio_mixer.clear_cache();
                                    self.canvas_state.selected_clip_pairs.clear();
                                    self.current_time = 0.0;
                                    self.status_message = "Yeni sekans oluşturuldu".to_string();
                                }
                                MediaBinAction::DuplicateSequence(seq_id) => {
                                    if self.project.duplicate_sequence(&seq_id).is_some() {
                                        self.status_message = "Sekans çoğaltıldı".to_string();
                                    }
                                }
                                MediaBinAction::DeleteSequence(seq_id) => {
                                    if self.project.remove_sequence(&seq_id) {
                                        self.video_engine.clear_cache();
                                        self.audio_mixer.clear_cache();
                                        self.canvas_state.selected_clip_pairs.clear();
                                        self.current_time = 0.0;
                                        self.status_message = "Sekans silindi".to_string();
                                    }
                                }
                                MediaBinAction::GenerateSoundscape(preset) => {
                                    self.generate_and_insert_soundscape(preset, None, 18, None);
                                }
                            }
                        }
                    }
                    LeftPanelTab::Effects => {
                        let selected_clip_id = self
                            .canvas_state
                            .selected_clip_pairs
                            .first()
                            .map(|(_, c)| c.clone());
                        let inspector_before: Option<(String, bool, trim_core::clip::Clip)> =
                            selected_clip_id.as_deref().and_then(|cid| {
                                self.project.timeline.tracks.iter().find_map(|t| {
                                    t.get_clip_by_id(cid)
                                        .map(|c| (t.id.clone(), t.locked, c.clone()))
                                })
                            });
                        let track_locked = inspector_before.as_ref().is_some_and(|(_, l, _)| *l);
                        let inspector_action = ui
                            .add_enabled_ui(!track_locked, |ui| {
                                render_inspector_panel(
                                    ui,
                                    &mut self.project,
                                    selected_clip_id.as_deref(),
                                    self.current_time,
                                )
                            })
                            .inner;
                        if let Some(act) = inspector_action {
                            match act {
                                InspectorAction::ClipModified => {
                                    if let Some((tr_id, false, before)) = inspector_before {
                                        if let Some(after) = self
                                            .project
                                            .timeline
                                            .get_track_by_id(&tr_id)
                                            .and_then(|t| t.get_clip_by_id(&before.id))
                                            .cloned()
                                        {
                                            if let Some(cl) = self
                                                .project
                                                .timeline
                                                .get_track_by_id_mut(&tr_id)
                                                .and_then(|t| t.get_clip_by_id_mut(&before.id))
                                            {
                                                *cl = before.clone();
                                            }
                                            let cmd = Box::new(ModifyClipCommand::new(
                                                "Denetçi Düzenlemesi",
                                                tr_id,
                                                before,
                                                after,
                                            ));
                                            let _ = self
                                                .undo_stack
                                                .push(cmd, &mut self.project.timeline);
                                        }
                                    }
                                    self.project.mark_dirty();
                                    self.last_rendered_monitor_frame = None;
                                    self.last_rendered_dirty = true;
                                    self.scopes_panel.mark_dirty();
                                    self.video_engine.last_rendered_frame = None;
                                }
                                InspectorAction::SetTransform {
                                    track_id,
                                    clip_id,
                                    old_transform,
                                    new_transform,
                                } => {
                                    let cmd = Box::new(SetClipTransformCommand::new(
                                        track_id,
                                        clip_id,
                                        old_transform,
                                        new_transform,
                                    ));
                                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                    self.project.mark_dirty();
                                    self.last_rendered_monitor_frame = None;
                                    self.video_engine.last_rendered_frame = None;
                                }
                                InspectorAction::SetColorWheels {
                                    track_id,
                                    clip_id,
                                    old_wheels,
                                    new_wheels,
                                } => {
                                    let cmd = Box::new(SetClipColorWheelsCommand::new(
                                        track_id, clip_id, old_wheels, new_wheels,
                                    ));
                                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                    self.project.mark_dirty();
                                    self.last_rendered_monitor_frame = None;
                                    self.video_engine.last_rendered_frame = None;
                                }
                                InspectorAction::SetEqPreset {
                                    track_id,
                                    clip_id,
                                    old_preset,
                                    new_preset,
                                } => {
                                    let cmd = Box::new(SetClipEqPresetCommand::new(
                                        track_id, clip_id, old_preset, new_preset,
                                    ));
                                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                    self.project.mark_dirty();
                                }
                                InspectorAction::SetCrop {
                                    track_id,
                                    clip_id,
                                    old_crop,
                                    new_crop,
                                } => {
                                    let cmd = Box::new(SetClipCropCommand::new(
                                        track_id, clip_id, old_crop, new_crop,
                                    ));
                                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                    self.project.mark_dirty();
                                    self.last_rendered_monitor_frame = None;
                                    self.video_engine.last_rendered_frame = None;
                                }
                                InspectorAction::ChangeSpeed(spd, rev) => {
                                    if let Some((tr_id, cl_id)) =
                                        self.canvas_state.selected_clip_pairs.first()
                                    {
                                        if let Some(tr) =
                                            self.project.timeline.get_track_by_id_mut(tr_id)
                                        {
                                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                                cl.change_speed(spd);
                                                cl.reverse = rev;
                                                tr.sort_clips();
                                                self.project.mark_dirty();
                                                self.last_rendered_monitor_frame = None;
                                                self.video_engine.last_rendered_frame = None;
                                            }
                                        }
                                    }
                                }
                                InspectorAction::TriggerSceneDetect => {
                                    self.smart_scene_cut_selected_clip();
                                }
                                InspectorAction::TriggerSilenceDetect => {
                                    self.open_silence_cut_dialog_for_selected();
                                }
                                InspectorAction::TrackMaskMotion {
                                    track_id,
                                    clip_id,
                                    mask_index,
                                } => {
                                    self.track_mask_motion_for_clip(&track_id, &clip_id, mask_index);
                                }
                                InspectorAction::AutoDuckAudio { track_id, clip_id } => {
                                    self.auto_duck_audio_clip(&track_id, &clip_id);
                                }
                                InspectorAction::TriggerTranscribe => {
                                    self.open_transcribe_dialog_for_selected();
                                }
                                InspectorAction::ImportSubtitles => {
                                    self.import_subtitles_dialog();
                                }
                                InspectorAction::ExportSubtitles => {
                                    self.export_subtitles_dialog();
                                }
                                InspectorAction::SelectCustomLut(clip_id) => {
                                    self.prompt_custom_lut(clip_id);
                                }
                                InspectorAction::SelectCustomFont(clip_id) => {
                                    self.prompt_custom_font(clip_id);
                                }
                                InspectorAction::GenerateTitleFoley { track_id, clip_id } => {
                                    self.generate_title_foley_for_clip(&track_id, &clip_id);
                                }
                                InspectorAction::ExportCubeLut { track_id, clip_id } => {
                                    self.prompt_export_cube_lut(&track_id, &clip_id);
                                }
                                InspectorAction::StartWbEyedropper { track_id, clip_id } => {
                                    if self.wb_eyedropper_target.is_some() {
                                        self.wb_eyedropper_target = None;
                                        self.status_message =
                                            "WB Damlalığı kapatıldı.".to_string();
                                    } else {
                                        self.wb_eyedropper_target = Some((track_id, clip_id));
                                        self.status_message =
                                            "🎯 WB Damlalığı Aktif: Program Monitörü üzerinde nötr beyaz/gri bir noktaya tıklayın.".to_string();
                                    }
                                }
                                InspectorAction::SetColorGrading {
                                    track_id,
                                    clip_id,
                                    old_state,
                                    new_state,
                                } => {
                                    let cmd = Box::new(SetClipColorGradingCommand::new(
                                        track_id, clip_id, *old_state, *new_state,
                                    ));
                                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                    self.project.mark_dirty();
                                    self.last_rendered_monitor_frame = None;
                                    self.video_engine.last_rendered_frame = None;
                                    self.scopes_panel.mark_dirty();
                                }
                                InspectorAction::SetBlendMode {
                                    track_id,
                                    clip_id,
                                    old_blend_mode,
                                    new_blend_mode,
                                } => {
                                    let cmd = Box::new(trim_commands::SetClipBlendModeCommand::new(
                                        track_id,
                                        clip_id,
                                        old_blend_mode,
                                        new_blend_mode,
                                    ));
                                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                    self.project.mark_dirty();
                                    self.last_rendered_monitor_frame = None;
                                    self.video_engine.last_rendered_frame = None;
                                    self.scopes_panel.mark_dirty();
                                }
                                InspectorAction::SetMasks {
                                    track_id,
                                    clip_id,
                                    old_masks,
                                    new_masks,
                                } => {
                                    let cmd = Box::new(trim_commands::SetClipMasksCommand::new(
                                        track_id, clip_id, old_masks, new_masks,
                                    ));
                                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                    self.project.mark_dirty();
                                    self.last_rendered_monitor_frame = None;
                                    self.video_engine.last_rendered_frame = None;
                                    self.scopes_panel.mark_dirty();
                                }
                                InspectorAction::AutoWhiteBalance { track_id, clip_id } => {
                                    let clip_snapshot = self
                                        .project
                                        .timeline
                                        .get_track_by_id(&track_id)
                                        .and_then(|t| t.get_clip_by_id(&clip_id))
                                        .cloned();
                                    if let Some(clip_ref) = clip_snapshot {
                                        let old_state = ClipColorGradingState::from_clip(&clip_ref);
                                        if let Some((temp, tint)) = self
                                            .video_engine
                                            .analyze_clip_auto_wb(
                                                &self.project,
                                                &clip_ref,
                                                self.current_time,
                                            )
                                        {
                                            let mut new_state = old_state.clone();
                                            new_state.wb_preset =
                                                trim_core::clip::WhiteBalancePreset::Auto;
                                            new_state.temperature = temp;
                                            new_state.tint = tint;
                                            let cmd = Box::new(SetClipColorGradingCommand::new(
                                                track_id, clip_id, old_state, new_state,
                                            ));
                                            let _ = self
                                                .undo_stack
                                                .push(cmd, &mut self.project.timeline);
                                            self.project.mark_dirty();
                                            self.last_rendered_monitor_frame = None;
                                            self.video_engine.last_rendered_frame = None;
                                            self.scopes_panel.mark_dirty();
                                            self.status_message = format!(
                                                "✨ Otomatik Beyaz Dengesi uygulandı (Sıcaklık: {:+.1}, Ton: {:+.1})",
                                                temp, tint
                                            );
                                        } else {
                                            self.status_message =
                                                "⚠ Otomatik Beyaz Dengesi için kare analiz edilemedi."
                                                    .to_string();
                                        }
                                    }
                                }
                                InspectorAction::ShotMatch { track_id, clip_id } => {
                                    let ref_opt = self
                                        .scopes_panel
                                        .active_reference_still
                                        .and_then(|idx| self.scopes_panel.reference_stills.get(idx))
                                        .map(|s| (s.rgba.clone(), s.width, s.height));
                                    let cur_opt = self
                                        .last_scope_rgba
                                        .as_ref()
                                        .map(|(rgba, w, h)| (rgba.clone(), *w, *h))
                                        .or_else(|| {
                                            self.last_rendered_monitor_frame.as_ref().map(|f| {
                                                (f.as_raw().clone(), f.width(), f.height())
                                            })
                                        });
                                    if let Some((src_rgba, sw, sh)) = cur_opt {
                                        let (ref_rgba, rw, rh) = ref_opt.unwrap_or_else(|| {
                                            ([128u8, 128, 128, 255].repeat(16 * 16), 16, 16)
                                        });
                                        let (b, c, s, temp, tint) =
                                            trim_core::color::analyze_shot_match_rgba(
                                                &src_rgba,
                                                sw,
                                                sh,
                                                &ref_rgba,
                                                rw,
                                                rh,
                                            );
                                        let before_after = self
                                            .project
                                            .timeline
                                            .get_track_by_id(&track_id)
                                            .filter(|tr| !tr.locked)
                                            .and_then(|tr| tr.get_clip_by_id(&clip_id))
                                            .map(|cl| {
                                                let before = cl.clone();
                                                let mut after = cl.clone();
                                                after.brightness =
                                                    (after.brightness + b).clamp(-1.0, 1.0);
                                                after.contrast = c.clamp(0.2, 2.5);
                                                after.saturation = s.clamp(0.0, 2.5);
                                                after.temperature =
                                                    (after.temperature + temp).clamp(-100.0, 100.0);
                                                after.tint = (after.tint + tint).clamp(-100.0, 100.0);
                                                (before, after)
                                            });
                                        if let Some((before, after)) = before_after {
                                            let cmd = Box::new(ModifyClipCommand::new(
                                                "Çekim Eşleştirme",
                                                track_id,
                                                before,
                                                after,
                                            ));
                                            let _ = self
                                                .undo_stack
                                                .push(cmd, &mut self.project.timeline);
                                        }
                                        self.project.mark_dirty();
                                        self.last_rendered_monitor_frame = None;
                                        self.last_rendered_dirty = true;
                                        self.scopes_panel.mark_dirty();
                                        self.video_engine.last_rendered_frame = None;
                                        self.scopes_panel.mark_dirty();
                                        self.status_message = format!(
                                            "🎯 Shot Match uygulandı (Parlaklık: {:+.2}, Kontrast: {:.2}, Sıcaklık: {:+.1})",
                                            b, c, temp
                                        );
                                    } else {
                                        self.status_message =
                                            "⚠ Shot Match için aktif önizleme karesi bulunamadı"
                                                .to_string();
                                    }
                                }
                            }
                        }
                    }
                }
            });

        // -------------------------------------------------------------
        // UPPER-RIGHT: Color Scopes (Independent resizable panel)
        // -------------------------------------------------------------
        if self.scopes_panel.is_open {
            egui::SidePanel::right("scopes_panel")
                .resizable(true)
                .default_width(310.0)
                .min_width(240.0)
                .max_width(540.0)
                .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(8.0))
                .show(ctx, |ui| {
                    self.scopes_panel.show(ui);
                });
            if let Some(still_idx) = self.scopes_panel.pending_apply_still_idx.take() {
                if let Some(src_clip) = self
                    .scopes_panel
                    .reference_stills
                    .get(still_idx)
                    .and_then(|s| s.grade_snapshot.clone())
                {
                    if let Some((tr_id, cl_id)) =
                        self.canvas_state.selected_clip_pairs.first().cloned()
                    {
                        if let Some(tr) = self.project.timeline.get_track_by_id_mut(&tr_id) {
                            if let Some(cl) = tr.get_clip_by_id_mut(&cl_id) {
                                cl.brightness = src_clip.brightness;
                                cl.contrast = src_clip.contrast;
                                cl.saturation = src_clip.saturation;
                                cl.gamma = src_clip.gamma;
                                cl.temperature = src_clip.temperature;
                                cl.tint = src_clip.tint;
                                cl.vignette = src_clip.vignette;
                                cl.sharpen = src_clip.sharpen;
                                cl.blur = src_clip.blur;
                                cl.film_grain = src_clip.film_grain;
                                cl.wb_preset = src_clip.wb_preset;
                                cl.bc_preset = src_clip.bc_preset;
                                cl.lut_preset = src_clip.lut_preset;
                                cl.custom_lut_path = src_clip.custom_lut_path;
                                cl.color_wheels = src_clip.color_wheels;
                                cl.custom_curves = src_clip.custom_curves;
                                cl.hsl_curves = src_clip.hsl_curves;
                                cl.hsl_qualifier = src_clip.hsl_qualifier;
                                cl.halation = src_clip.halation;
                                cl.bloom = src_clip.bloom;
                                cl.chromatic_aberration = src_clip.chromatic_aberration;
                                self.project.mark_dirty();
                                self.last_rendered_monitor_frame = None;
                                self.video_engine.last_rendered_frame = None;
                                self.scopes_panel.mark_dirty();
                                self.status_message =
                                    "🎨 Referans karenin renk derecelendirmesi (Grade) seçili klibe uygulandı"
                                        .to_string();
                            }
                        }
                    } else {
                        self.status_message =
                            "Referans rengini uygulamak için zaman çizgisinde bir klip seçin"
                                .to_string();
                    }
                }
            }
        }
    }
}
