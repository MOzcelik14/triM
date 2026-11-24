use crate::dialogs::*;
use crate::i18n::Language;
use crate::shortcuts::ShortcutProfile;

use egui::{Button, Color32, Key, RichText, Stroke, Vec2};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;

use trim_commands::commands::*;
use trim_core::clip::{Clip, TextOverlay};
use trim_core::media::MediaType;
use trim_export::queue::RenderJob;
use trim_media::{generate_title_image, FFprobeAnalyzer};
use trim_project::load_project;

use super::{BackgroundTaskMsg, BinDialogMode, TrimApp};

impl TrimApp {
    pub(super) fn handle_modal_dialogs(&mut self, ctx: &egui::Context) {
        // -------------------------------------------------------------
        // Modals & Dialogs
        // -------------------------------------------------------------
        if let Some(act) = self.export_dialog.show(ctx) {
            match act {
                ExportDialogAction::StartExport(preset, out_path, range) => {
                    let (tx, rx) = channel();
                    self.export_rx = Some(rx);
                    self.export_runner.start_export(
                        self.project.clone(),
                        preset,
                        out_path,
                        range,
                        tx,
                    );
                }
                ExportDialogAction::CancelExport => {
                    self.export_runner.cancel();
                    self.export_dialog.is_exporting = false;
                    self.export_rx = None;
                    self.status_message = "Dışa aktarma iptal edildi".to_string();
                }
                ExportDialogAction::AddToQueue(preset, out_path, range) => {
                    let job_name = Path::new(&out_path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Dışa Aktarma İşi")
                        .to_string();
                    let job =
                        RenderJob::new(job_name, self.project.clone(), preset, out_path, range);
                    self.render_queue.add_job(job);
                    self.render_queue_dialog.open();
                    self.status_message = "İş dışa aktarma kuyruğuna eklendi.".to_string();
                }
                ExportDialogAction::OpenOutputFolder(path) => {
                    let p = std::path::Path::new(&path);
                    let folder = if p.is_file() {
                        p.parent().unwrap_or(p)
                    } else {
                        p
                    };
                    if folder.is_dir() {
                        self.open_folder_tracked(folder);
                    }
                }
                ExportDialogAction::CopyYoutubeChapters => {
                    let chapters = trim_core::interchange::export_youtube_chapters(
                        &self.project.timeline.markers,
                        self.project.timeline.duration(),
                    );
                    ctx.output_mut(|o| {
                        o.copied_text = chapters;
                    });
                    self.status_message =
                        "📑 YouTube Bölüm (Chapter) zaman damgaları panoya kopyalandı!".to_string();
                }
                ExportDialogAction::ExportEdl => {
                    self.export_edl_dialog();
                }
                ExportDialogAction::ExportFcpxml => {
                    self.export_fcpxml_dialog();
                }
            }
        }

        if let Some(act) = self.render_queue_dialog.show(ctx, &mut self.render_queue) {
            match act {
                RenderQueueDialogAction::OpenOutputFolder(path) => {
                    let p = std::path::Path::new(&path);
                    let folder = if p.is_file() {
                        p.parent().unwrap_or(p)
                    } else {
                        p
                    };
                    if folder.is_dir() {
                        self.open_folder_tracked(folder);
                    }
                }
                RenderQueueDialogAction::NewExportJob => {
                    self.export_dialog.open(&self.project);
                }
                RenderQueueDialogAction::Close => {}
            }
        }

        if let Some(act) = self.relink_dialog.show(ctx) {
            match act {
                RelinkDialogAction::PickFolder => {
                    self.relink_folder_dialog();
                }
                RelinkDialogAction::PickSingleMedia { media_id } => {
                    self.relink_single_media_dialog(media_id);
                }
                RelinkDialogAction::Refresh => {
                    self.relink_dialog.refresh(&self.project);
                }
                RelinkDialogAction::Close => {}
            }
        }

        if let Some(act) = self.speed_dialog.show(ctx) {
            match act {
                SpeedDialogAction::Apply {
                    track_id,
                    clip_id,
                    speed,
                    reverse,
                } => {
                    let cmd = Box::new(ChangeClipSpeedCommand::new(
                        track_id, clip_id, speed, reverse,
                    ));
                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                    self.project.mark_dirty();
                }
            }
        }

        if let Some(act) = self.silence_dialog.show(ctx) {
            match act {
                SilenceDialogAction::DetectRequested {
                    track_id,
                    clip_id,
                    noise_db,
                    min_duration,
                } => {
                    self.detect_silence_for_dialog(&track_id, &clip_id, noise_db, min_duration);
                }
                SilenceDialogAction::ApplyCut {
                    track_id,
                    clip_id,
                    speech_intervals,
                } => {
                    let cmd = Box::new(AutoSilenceCutCommand::new(
                        track_id,
                        clip_id,
                        speech_intervals,
                    ));
                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                    self.canvas_state.selected_clip_pairs.clear();
                    self.status_message =
                        "Sessizlikler otomatik kesildi (Jump-cut uygulandı)".to_string();
                    self.project.mark_dirty();
                }
            }
        }

        if let Some(act) = self.title_dialog.show(ctx) {
            match act {
                TitleDialogAction::CreateTitle {
                    text,
                    font_size,
                    text_color,
                    bg_color,
                    position,
                    has_shadow,
                    duration,
                    motion_style,
                    subtitle_text,
                    font_family,
                    bold,
                    italic,
                } => {
                    if let Some(out_png) = generate_title_image(
                        &text,
                        font_size,
                        text_color,
                        bg_color,
                        &position,
                        has_shadow,
                        self.project.timeline.width,
                        self.project.timeline.height,
                    ) {
                        if let Ok(mut item) = FFprobeAnalyzer::analyze(&out_png) {
                            item.name = text.clone();
                            item.duration = duration;
                            let m_id = item.id.clone();
                            self.project.add_media(item);

                            let current_video_tracks = self.project.timeline.get_video_tracks();
                            let target_has_clip = self
                                .project
                                .timeline
                                .get_track_by_id(&self.target_video_track)
                                .map(|t| t.find_clip_at(self.current_time).is_some())
                                .unwrap_or(false);

                            let target_track_id = if target_has_clip {
                                let upper_track = current_video_tracks
                                    .iter()
                                    .filter(|t| t.id != self.target_video_track && !t.locked)
                                    .find(|t| t.find_clip_at(self.current_time).is_none());

                                if let Some(t) = upper_track {
                                    t.id.clone()
                                } else {
                                    let max_v = current_video_tracks
                                        .iter()
                                        .filter_map(|t| {
                                            t.name
                                                .strip_prefix('V')
                                                .and_then(|s| s.parse::<usize>().ok())
                                        })
                                        .max()
                                        .unwrap_or(1);
                                    let new_track_name = format!("V{}", max_v + 1);
                                    let new_track = trim_core::track::Track::new(
                                        &new_track_name,
                                        trim_core::track::TrackType::Video,
                                    );
                                    let new_id = new_track.id.clone();
                                    let _ = self.undo_stack.push(
                                        Box::new(AddTrackCommand::new(new_track, 0)),
                                        &mut self.project.timeline,
                                    );
                                    self.project.mark_dirty();
                                    self.status_message = format!(
                                        "Üst video kanalı otomatik oluşturuldu: {}",
                                        new_track_name
                                    );
                                    new_id
                                }
                            } else if self
                                .project
                                .timeline
                                .get_track_by_id(&self.target_video_track)
                                .is_some()
                            {
                                self.target_video_track.clone()
                            } else {
                                current_video_tracks
                                    .first()
                                    .map(|t| t.id.clone())
                                    .unwrap_or_else(|| self.project.timeline.add_new_video_track())
                            };

                            if !target_track_id.is_empty() {
                                let is_locked = self
                                    .project
                                    .timeline
                                    .get_track_by_id(&target_track_id)
                                    .map(|t| t.locked)
                                    .unwrap_or(false);
                                if is_locked {
                                    self.status_message =
                                        "Hedef video kanalı kilitli, metin klibi eklenemez"
                                            .to_string();
                                } else {
                                    let mut clip = Clip::new(
                                        m_id,
                                        self.current_time,
                                        self.current_time + duration,
                                    );
                                    clip.name = format!("Başlık: {}", text);
                                    clip.text_overlay = Some(TextOverlay {
                                        content: text.clone(),
                                        font_size: font_size as f32,
                                        text_color: [
                                            text_color[0],
                                            text_color[1],
                                            text_color[2],
                                            255,
                                        ],
                                        bg_color: if bg_color[3] > 0 {
                                            Some(bg_color)
                                        } else {
                                            None
                                        },
                                        pos_x: match position.as_str() {
                                            "lower_third" => 0.08,
                                            _ => 0.50,
                                        },
                                        pos_y: match position.as_str() {
                                            "lower_third" => 0.80,
                                            "subtitle" => 0.87,
                                            "top" => 0.12,
                                            _ => 0.50,
                                        },
                                        has_shadow,
                                        motion_style,
                                        subtitle_text,
                                        font_family,
                                        bold,
                                        italic,
                                        ..Default::default()
                                    });
                                    if matches!(
                                        motion_style,
                                        trim_core::clip::MotionTitleStyle::CinematicZoom
                                            | trim_core::clip::MotionTitleStyle::LowerThird
                                            | trim_core::clip::MotionTitleStyle::RollingCredits
                                            | trim_core::clip::MotionTitleStyle::ChapterCard
                                    ) {
                                        clip.fade_in = 0.5;
                                        clip.fade_out = 0.5;
                                    }
                                    let cmd = Box::new(AddClipCommand::new(target_track_id, clip));
                                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                    self.project.mark_dirty();
                                    self.status_message =
                                        format!("Metin klibi eklendi ({}s)", duration);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(act) = self.recovery_dialog.show(ctx) {
            match act {
                RecoveryAction::Restore(p) => {
                    if let Ok(mut proj) = load_project(&p) {
                        // If an older recovery snapshot had 1920x1080 clobbered over a 4K/custom
                        // video clip, recover the resolution from the timeline's first video clip.
                        if proj.timeline.width == 1920 && proj.timeline.height == 1080 {
                            let first_video_dims = proj
                                .timeline
                                .tracks
                                .iter()
                                .flat_map(|t| t.clips.iter())
                                .filter_map(|c| proj.media_pool.get(&c.media_id))
                                .find(|m| {
                                    m.media_type == MediaType::Video && m.width > 0 && m.height > 0
                                })
                                .map(|m| (m.width, m.height, m.fps));
                            if let Some((vw, vh, vfps)) = first_video_dims {
                                if vw != 1920 || vh != 1080 {
                                    proj.set_resolution(vw, vh);
                                    if vfps > 1.0 {
                                        proj.set_fps(vfps);
                                    }
                                }
                            }
                        }

                        self.project = proj;
                        self.project.mark_dirty();
                        self.welcome_dialog.close();
                        self.undo_stack.clear();
                        self.canvas_state.selected_clip_pairs.clear();
                        self.current_time = 0.0;
                        self.media_bin_textures.clear();
                        self.video_engine.clear_cache();
                        self.audio_mixer.clear_cache();
                        self.audio_player.stop();
                        self.is_playing = false;
                        self.monitor_texture = None;
                        self.last_rendered_dirty = true;
                        self.source_state.media_id = None;
                        self.source_state.texture = None;
                        self.source_state.is_playing = false;
                        self.source_state.clock_start = None;
                        self.resolve_default_target_tracks();
                        for item in self.project.media_pool.values_mut() {
                            if item.media_type == MediaType::Video && !item.has_audio() {
                                if let Ok(probed) = FFprobeAnalyzer::analyze(&item.file_path) {
                                    if probed.has_audio() {
                                        item.audio_codec = probed.audio_codec;
                                        item.audio_channels = probed.audio_channels;
                                        item.audio_sample_rate = probed.audio_sample_rate;
                                    }
                                }
                            }
                            if item.has_audio() {
                                self.audio_mixer.preload_audio_async(&item.file_path);
                            }
                        }
                        let _ = std::fs::remove_file(&p);
                        self.status_message = format!(
                            "Proje otomatik kayıttan kurtarıldı ({}×{} @ {:.0}fps)",
                            self.project.timeline.width,
                            self.project.timeline.height,
                            self.project.timeline.fps
                        );
                    }
                }
                RecoveryAction::Discard(p) => {
                    let _ = std::fs::remove_file(p);
                    self.autosave_mgr.cleanup(&self.project);
                }
            }
        }

        // -------------------------------------------------------------
        // Audio Mixer Modal
        // -------------------------------------------------------------
        let mixer_actions = self.audio_mixer_dialog.show(
            ctx,
            &self.project,
            self.vu_state.current_l,
            self.vu_state.current_r,
        );
        for act in mixer_actions {
            match act {
                AudioMixerAction::TrackVolumeChanged(t_id, vol) => {
                    if let Some(t) = self.project.timeline.get_track_by_id_mut(&t_id) {
                        t.volume = vol;
                        self.project.mark_dirty();
                    }
                }
                AudioMixerAction::TrackPanChanged(t_id, pan) => {
                    if let Some(t) = self.project.timeline.get_track_by_id_mut(&t_id) {
                        t.pan = pan.clamp(-1.0, 1.0);
                        self.audio_mixer.clear_cache();
                        self.project.mark_dirty();
                    }
                }
                AudioMixerAction::TrackMuteToggled(t_id) => {
                    if let Some(t) = self.project.timeline.get_track_by_id_mut(&t_id) {
                        t.muted = !t.muted;
                    }
                }
                AudioMixerAction::TrackSoloToggled(t_id) => {
                    if let Some(t) = self.project.timeline.get_track_by_id_mut(&t_id) {
                        t.solo = !t.solo;
                    }
                }
                AudioMixerAction::MasterVolumeChanged(_vol) => {
                    // Handled inside dialog
                }
            }
        }

        // -------------------------------------------------------------
        // Paste Attributes Modal (Ctrl+Alt+V)
        // -------------------------------------------------------------
        if self.paste_attrs_dialog_open {
            let mut open = self.paste_attrs_dialog_open;
            let mut do_apply = false;
            let mut do_close = false;
            egui::Window::new("≡ Özellikleri Seçerek Yapıştır (Paste Attributes)")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(Vec2::new(390.0, 220.0))
                .show(ctx, |ui| {
                    let src_name = self
                        .clipboard_clips
                        .first()
                        .map(|(_, _, _, c)| c.name.as_str())
                        .unwrap_or("(Panoda klip yok)");
                    ui.label(
                        RichText::new(format!("Kaynak Klip: {}", src_name))
                            .size(11.5)
                            .color(Color32::from_rgb(0x94, 0xa3, 0xb8)),
                    );
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.checkbox(
                        &mut self.paste_attr_color,
                        "■ Renk & Grading (Color Wheels, Eğriler, HSL, LUT, Film FX)",
                    );
                    ui.checkbox(
                        &mut self.paste_attr_transform,
                        "⊞ Dönüşüm & Kompozisyon (Konum, Ölçek, Döndürme, Kırpma, Corner Pin)",
                    );
                    ui.checkbox(
                        &mut self.paste_attr_audio,
                        "♪ Ses & Fairlight (Ses Seviyesi, Pan, 6-Band EQ, Kompresör, Reverb)",
                    );
                    ui.checkbox(
                        &mut self.paste_attr_speed,
                        "▸ Hız & Yön (Speed, Reverse, Pitch Lock)",
                    );

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let apply_btn = egui::Button::new(
                                RichText::new("Seçili Kliplere Uygula")
                                    .size(11.5)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(0x28, 0x5a, 0x9e));
                            if ui
                                .add_enabled(!self.clipboard_clips.is_empty(), apply_btn)
                                .clicked()
                            {
                                do_apply = true;
                            }
                            if ui.button("İptal").clicked() {
                                do_close = true;
                            }
                        });
                    });
                });
            if do_apply {
                self.apply_paste_attributes(
                    self.paste_attr_color,
                    self.paste_attr_transform,
                    self.paste_attr_audio,
                    self.paste_attr_speed,
                );
                self.paste_attrs_dialog_open = false;
            } else if do_close || !open {
                self.paste_attrs_dialog_open = false;
            }
        }

        // -------------------------------------------------------------
        // Welcome Screen / Project Launcher Modal
        // (Do not overlap when Crash Recovery modal is active)
        // -------------------------------------------------------------
        let welcome_act = if self.recovery_dialog.is_open || self.splash_screen.is_active {
            None
        } else {
            self.welcome_dialog.show(ctx)
        };
        if let Some(act) = welcome_act {
            match act {
                WelcomeAction::CreateNewProject {
                    name,
                    width,
                    height,
                    fps,
                } => {
                    if self.has_unsaved_changes() {
                        let current_name = self.project.settings.name.clone();
                        self.close_confirm_dialog
                            .prompt(PendingCloseAction::NewProject, current_name);
                        self.welcome_dialog.close();
                    } else {
                        self.reset_to_new_project();
                        self.project.settings.name = name;
                        self.project.set_resolution(width, height);
                        self.project.set_fps(fps);
                        self.status_message = format!(
                            "Yeni proje hazır: {} ({}×{} @ {:.0}fps)",
                            self.project.settings.name, width, height, fps
                        );
                        self.welcome_dialog.close();
                    }
                }
                WelcomeAction::OpenProjectRequested => {
                    if self.has_unsaved_changes() {
                        let name = self.project.settings.name.clone();
                        self.close_confirm_dialog
                            .prompt(PendingCloseAction::OpenProject, name);
                        self.welcome_dialog.close();
                    } else {
                        self.open_project_dialog();
                        self.welcome_dialog.close();
                    }
                }
                WelcomeAction::OpenRecentProject(path) => {
                    if self.has_unsaved_changes() {
                        let name = self.project.settings.name.clone();
                        self.close_confirm_dialog
                            .prompt(PendingCloseAction::OpenPath(PathBuf::from(path)), name);
                        self.welcome_dialog.close();
                    } else {
                        self.open_path(Path::new(&path));
                        self.welcome_dialog.close();
                    }
                }
                WelcomeAction::Close => {
                    self.welcome_dialog.close();
                }
            }
        }

        // -------------------------------------------------------------
        // Unsaved Changes Confirmation Modal
        // -------------------------------------------------------------
        if let Some(act) = self.close_confirm_dialog.show(ctx) {
            match act {
                CloseConfirmAction::SaveAndProceed(pending) => {
                    self.save_project_and_proceed(ctx, pending);
                }
                CloseConfirmAction::DiscardAndProceed(pending) => {
                    self.project.is_dirty = false;
                    self.last_saved_undo_len = self.undo_stack.len();
                    self.execute_pending_close_action(ctx, pending);
                }
                CloseConfirmAction::Cancel => {}
            }
        }

        // -------------------------------------------------------------
        // Keyboard Shortcuts Modal (F1)
        // -------------------------------------------------------------
        self.shortcuts_dialog.show(ctx, &mut self.shortcut_profile);

        // -------------------------------------------------------------
        // Preferences / Settings Modal (Ctrl+,)
        // -------------------------------------------------------------
        if let Some(act) = self.preferences_dialog.show(ctx, self.lang()) {
            match act {
                PreferencesAction::Save(new_prefs) => {
                    let old_scale = self.preferences.ui_scale;
                    let new_scale = new_prefs.ui_scale;
                    let old_proxy = self.preferences.use_proxies_by_default;
                    let new_proxy = new_prefs.use_proxies_by_default;
                    let old_snap = self.preferences.snap_enabled_by_default;
                    let new_snap = new_prefs.snap_enabled_by_default;

                    crate::theme::apply_studio_theme(ctx, &new_prefs.theme_id);
                    if (old_scale - new_scale).abs() > 0.001 {
                        ctx.set_pixels_per_point(new_scale);
                    }
                    if old_proxy != new_proxy {
                        self.use_proxies = new_proxy;
                        self.video_engine.use_proxies = new_proxy;
                        self.video_engine.clear_cache();
                        self.last_rendered_dirty = true;
                    }
                    if old_snap != new_snap {
                        self.snap_enabled = new_snap;
                    }

                    match new_prefs.shortcut_profile.as_str() {
                        "Adobe Premiere Pro" => {
                            self.shortcut_profile = ShortcutProfile::PremierePro
                        }
                        "DaVinci Resolve" => {
                            self.shortcut_profile = ShortcutProfile::DaVinciResolve
                        }
                        _ => self.shortcut_profile = ShortcutProfile::TrimDefault,
                    }

                    if self.preferences.max_memory_budget_mb != new_prefs.max_memory_budget_mb {
                        let mb = new_prefs.max_memory_budget_mb.max(128);
                        self.audio_mixer.set_max_cache_bytes(mb * 1024 * 1024);
                        self.video_engine
                            .set_frame_cache_capacity((mb * 300 / 512).max(50));
                    }

                    self.audio_mixer.limiter.set_ceiling_dbfs(
                        if new_prefs.master_limiter_enabled {
                            new_prefs.limiter_ceiling_db
                        } else {
                            0.0
                        },
                    );

                    if let Err(e) = new_prefs.save() {
                        self.status_message = format!("Tercihler kaydedilemedi: {}", e);
                    } else {
                        self.status_message = match new_prefs.language.as_str() {
                            "en" => "Preferences saved successfully.".to_string(),
                            _ => "Tercihler başarıyla kaydedildi.".to_string(),
                        };
                    }
                    self.preferences = new_prefs;
                }
                PreferencesAction::Close => {
                    crate::theme::apply_studio_theme(ctx, &self.preferences.theme_id);
                }
            }
        }

        // -------------------------------------------------------------
        // Whisper Speech-to-Text Transcription Modal
        // -------------------------------------------------------------
        if let Some(act) = self.transcribe_dialog.ui(ctx) {
            match act {
                TranscribeDialogAction::StartTranscribe {
                    media_path,
                    model_name,
                    language,
                } => {
                    let whisper_bin_opt = self.transcribe_dialog.whisper_bin.clone();
                    if let Some(whisper_bin) = whisper_bin_opt {
                        self.transcribe_dialog.is_transcribing = true;
                        self.transcribe_dialog.status_message =
                            "Ses ayıklanıyor ve deşifre ediliyor (arka planda)...".to_string();
                        let tx = self.bg_task_tx.clone();
                        std::thread::spawn(move || {
                            let temp_wav = std::env::temp_dir()
                                .join(format!("trim_whisper_{}.wav", uuid::Uuid::new_v4()));
                            let media_p = PathBuf::from(&media_path);
                            let res =
                                match trim_media::extract_audio_for_whisper(&media_p, &temp_wav) {
                                    Ok(()) => trim_media::run_whisper_transcription(
                                        &whisper_bin,
                                        &temp_wav,
                                        &model_name,
                                        Some(&language),
                                    )
                                    .map_err(|e| format!("Deşifre hatası: {}", e)),
                                    Err(e) => Err(format!("Ses ayıklama hatası: {}", e)),
                                };
                            let _ = std::fs::remove_file(&temp_wav);
                            let _ = tx.send(BackgroundTaskMsg::TranscribeDone { result: res });
                        });
                    } else {
                        self.transcribe_dialog.is_transcribing = false;
                        self.transcribe_dialog.status_message = "Whisper çalıştırılabilir dosyası bulunamadı. Lütfen whisper-cli veya openai-whisper kurun.".to_string();
                    }
                }
            }
        }

        // -------------------------------------------------------------
        // Screen & Webcam Recorder Modal
        // -------------------------------------------------------------
        if let Some(act) = self.screen_record_dialog.ui(ctx) {
            match act {
                ScreenRecordAction::RecordingSaved {
                    file_path,
                    auto_add_to_timeline,
                } => {
                    if let Ok(mut item) = FFprobeAnalyzer::analyze(&file_path) {
                        item.name = file_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Ekran Kaydı".to_string());
                        let m_id = item.id.clone();
                        let dur = item.duration;
                        self.project.add_media(item);

                        if auto_add_to_timeline && dur > 0.1 {
                            let current_video_tracks = self.project.timeline.get_video_tracks();
                            if let Some(target_tr) = current_video_tracks.first() {
                                let clip =
                                    Clip::new(m_id, self.current_time, self.current_time + dur);
                                let cmd = Box::new(AddClipCommand::new(&target_tr.id, clip));
                                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                self.project.mark_dirty();
                            }
                        }
                        self.status_message = format!(
                            "Ekran kaydı içe aktarıldı: {:?}",
                            file_path.file_name().unwrap_or_default()
                        );
                    }
                }
            }
        }

        if self.show_about_dialog {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.show_about_dialog = false;
            }

            let screen_rect = ctx.screen_rect();
            egui::Area::new(egui::Id::new("about_modal_backdrop"))
                .order(egui::Order::Middle)
                .fixed_pos(egui::Pos2::ZERO)
                .show(ctx, |ui| {
                    let (rect, resp) =
                        ui.allocate_exact_size(screen_rect.size(), egui::Sense::click_and_drag());
                    ui.painter()
                        .rect_filled(rect, 0.0, Color32::from_black_alpha(155));
                    if resp.clicked() {
                        self.show_about_dialog = false;
                    }
                });

            let about_frame = egui::Frame::window(&ctx.style())
                .fill(Color32::from_rgb(0x16, 0x17, 0x1e))
                .rounding(egui::Rounding::same(10.0))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x2e, 0x31, 0x3e)))
                .inner_margin(egui::Margin::symmetric(24.0, 20.0))
                .shadow(egui::epaint::Shadow {
                    offset: Vec2::new(0.0, 12.0),
                    blur: 32.0,
                    spread: 3.0,
                    color: Color32::from_black_alpha(180),
                });

            egui::Window::new("triM. Hakkında")
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .order(egui::Order::Foreground)
                .fixed_size(Vec2::new(520.0, 340.0))
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .frame(about_frame)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        // Top Branding Header
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("triM.")
                                    .size(32.0)
                                    .strong()
                                    .color(Color32::from_rgb(0xe0, 0x7a, 0x38)),
                            );
                            ui.add_space(8.0);
                            egui::Frame::none()
                                .fill(Color32::from_rgb(0x28, 0x1e, 0x18))
                                .rounding(egui::Rounding::same(6.0))
                                .stroke(Stroke::new(
                                    1.0_f32,
                                    Color32::from_rgb(0x8a, 0x4b, 0x24),
                                ))
                                .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "v{}",
                                            env!("CARGO_PKG_VERSION")
                                        ))
                                        .size(11.5)
                                        .strong()
                                        .color(Color32::from_rgb(0xf2, 0x9b, 0x5e)),
                                    );
                                });
                        });

                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(
                                "Modern, bağımsız ve hafif masaüstü video düzenleyicisi.",
                            )
                            .size(13.0)
                            .color(Color32::from_rgb(0x9a, 0x9e, 0xb2)),
                        );

                        ui.add_space(14.0);

                        // Developer & Copyright Card
                        egui::Frame::none()
                            .fill(Color32::from_rgb(0x11, 0x12, 0x17))
                            .rounding(egui::Rounding::same(8.0))
                            .stroke(Stroke::new(
                                1.0_f32,
                                Color32::from_rgb(0x25, 0x28, 0x34),
                            ))
                            .inner_margin(egui::Margin::symmetric(16.0, 14.0))
                            .show(ui, |ui| {
                                ui.set_min_width(470.0);
                                egui::Grid::new("about_info_grid")
                                    .num_columns(2)
                                    .spacing([18.0, 8.0])
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new("Geliştirici:")
                                                .size(12.0)
                                                .strong()
                                                .color(Color32::from_rgb(0x82, 0x86, 0x9a)),
                                        );
                                        ui.label(
                                            RichText::new("Murat Özçelik (MOzcelik14)")
                                                .size(12.0)
                                                .strong()
                                                .color(Color32::from_rgb(0xf0, 0xf4, 0xfc)),
                                        );
                                        ui.end_row();

                                        ui.label(
                                            RichText::new("Telif Hakkı:")
                                                .size(12.0)
                                                .strong()
                                                .color(Color32::from_rgb(0x82, 0x86, 0x9a)),
                                        );
                                        ui.label(
                                            RichText::new("© 2026 Murat Özçelik. Tüm hakları saklıdır.")
                                                .size(12.0)
                                                .color(Color32::from_rgb(0xdc, 0xe0, 0xee)),
                                        );
                                        ui.end_row();

                                        ui.label(
                                            RichText::new("Lisans:")
                                                .size(12.0)
                                                .strong()
                                                .color(Color32::from_rgb(0x82, 0x86, 0x9a)),
                                        );
                                        ui.label(
                                            RichText::new("GNU General Public License v3.0 veya üzeri (Açık Kaynak)")
                                                .size(12.0)
                                                .color(Color32::from_rgb(0xa0, 0xdb, 0xa0)),
                                        );
                                        ui.end_row();

                                        ui.label(
                                            RichText::new("Vizyon:")
                                                .size(12.0)
                                                .strong()
                                                .color(Color32::from_rgb(0x82, 0x86, 0x9a)),
                                        );
                                        ui.label(
                                            RichText::new("Kullanıcı gizliliğine ve doğrudan donanım performansına odaklanan özgür video stüdyosu.")
                                                .size(11.5)
                                                .color(Color32::from_rgb(0x94, 0xa3, 0xb8)),
                                        );
                                        ui.end_row();
                                    });
                            });

                        ui.add_space(14.0);

                        // Bottom Action Buttons
                        ui.horizontal(|ui| {
                            let web_btn = Button::new(
                                RichText::new("↗ Web Sitesi")
                                    .size(11.5)
                                    .strong()
                                    .color(Color32::from_rgb(0xd8, 0xdc, 0xec)),
                            )
                            .min_size(Vec2::new(105.0, 32.0))
                            .rounding(egui::Rounding::same(6.0))
                            .fill(Color32::from_rgb(0x22, 0x25, 0x32))
                            .stroke(Stroke::new(
                                1.0_f32,
                                Color32::from_rgb(0x36, 0x3a, 0x4c),
                            ));

                            if ui
                                .add(web_btn)
                                .on_hover_text("https://mozcelik14.github.io/triM")
                                .clicked()
                            {
                                ctx.open_url(egui::OpenUrl::new_tab(
                                    "https://mozcelik14.github.io/triM",
                                ));
                            }

                            ui.add_space(4.0);

                            let gh_btn = Button::new(
                                RichText::new("↗ GitHub")
                                    .size(11.5)
                                    .strong()
                                    .color(Color32::from_rgb(0xd8, 0xdc, 0xec)),
                            )
                            .min_size(Vec2::new(92.0, 32.0))
                            .rounding(egui::Rounding::same(6.0))
                            .fill(Color32::from_rgb(0x22, 0x25, 0x32))
                            .stroke(Stroke::new(
                                1.0_f32,
                                Color32::from_rgb(0x36, 0x3a, 0x4c),
                            ));

                            if ui
                                .add(gh_btn)
                                .on_hover_text("https://github.com/MOzcelik14/triM")
                                .clicked()
                            {
                                ctx.open_url(egui::OpenUrl::new_tab(
                                    "https://github.com/MOzcelik14/triM",
                                ));
                            }

                            ui.add_space(4.0);

                            let upd_btn = Button::new(
                                RichText::new("↺ Güncellemeler")
                                    .size(11.5)
                                    .strong()
                                    .color(Color32::from_rgb(0x60, 0xd0, 0x98)),
                            )
                            .min_size(Vec2::new(124.0, 32.0))
                            .rounding(egui::Rounding::same(6.0))
                            .fill(Color32::from_rgb(0x1b, 0x2a, 0x24))
                            .stroke(Stroke::new(
                                1.0_f32,
                                Color32::from_rgb(0x2d, 0x52, 0x40),
                            ));

                            if ui
                                .add(upd_btn)
                                .on_hover_text("https://github.com/MOzcelik14/triM/releases/latest")
                                .clicked()
                            {
                                ctx.open_url(egui::OpenUrl::new_tab(
                                    "https://github.com/MOzcelik14/triM/releases/latest",
                                ));
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let close_btn = Button::new(
                                        RichText::new("Kapat")
                                            .size(12.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .min_size(Vec2::new(88.0, 32.0))
                                    .rounding(egui::Rounding::same(6.0))
                                    .fill(Color32::from_rgb(0xe0, 0x7a, 0x38));

                                    if ui.add(close_btn).clicked() {
                                        self.show_about_dialog = false;
                                    }
                                },
                            );
                        });
                    });
                });
        }

        if let Some(mut state) = self.bin_dialog.clone() {
            let mut close_dialog = false;
            let mut commit_dialog = false;

            let (title, confirm_text) = match &state.mode {
                BinDialogMode::Create => ("⊞ Yeni Medya Klasörü (Bin)", "Oluştur"),
                BinDialogMode::Rename(_) => ("⊞ Klasörü Yeniden Adlandır", "Kaydet"),
            };

            let modal_frame = egui::Frame::none()
                .fill(Color32::from_rgb(0x16, 0x18, 0x22))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x38, 0x3e, 0x54)))
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::same(16.0));

            egui::Window::new("bin_modal_window")
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .order(egui::Order::Foreground)
                .fixed_size(Vec2::new(320.0, 140.0))
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .frame(modal_frame)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(title)
                                .size(14.0)
                                .strong()
                                .color(Color32::from_rgb(0xf5, 0x9e, 0x0b)),
                        );
                        ui.add_space(10.0);

                        ui.label(
                            RichText::new("Klasör Adı:")
                                .size(11.0)
                                .color(Color32::from_rgb(0x94, 0xa3, 0xb8)),
                        );
                        let text_resp = ui.add(
                            egui::TextEdit::singleline(&mut state.name_input)
                                .desired_width(ui.available_width()),
                        );
                        if text_resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                            commit_dialog = true;
                        }

                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let ok_btn = Button::new(
                                        RichText::new(confirm_text)
                                            .size(11.5)
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .fill(Color32::from_rgb(0x25, 0x63, 0xeb))
                                    .min_size(Vec2::new(75.0, 26.0))
                                    .rounding(egui::Rounding::same(4.0));

                                    if ui.add(ok_btn).clicked() {
                                        commit_dialog = true;
                                    }

                                    let cancel_btn = Button::new(
                                        RichText::new("İptal")
                                            .size(11.5)
                                            .color(Color32::from_rgb(0x94, 0xa3, 0xb8)),
                                    )
                                    .fill(Color32::from_rgb(0x22, 0x25, 0x32))
                                    .min_size(Vec2::new(65.0, 26.0))
                                    .rounding(egui::Rounding::same(4.0));

                                    if ui.add(cancel_btn).clicked()
                                        || ui.input(|i| i.key_pressed(Key::Escape))
                                    {
                                        close_dialog = true;
                                    }
                                },
                            );
                        });
                    });
                });

            if commit_dialog {
                let trimmed = state.name_input.trim();
                if !trimmed.is_empty() {
                    match state.mode {
                        BinDialogMode::Create => {
                            let bin_id = self.project.add_bin(trimmed);
                            self.media_bin_current_bin = Some(bin_id);
                            self.status_message = format!("'{}' klasörü oluşturuldu", trimmed);
                        }
                        BinDialogMode::Rename(bin_id) => {
                            self.project.rename_bin(&bin_id, trimmed);
                            self.status_message =
                                format!("Klasör '{}' olarak yeniden adlandırıldı", trimmed);
                        }
                    }
                    self.project.mark_dirty();
                }
                self.bin_dialog = None;
            } else if close_dialog {
                self.bin_dialog = None;
            } else {
                self.bin_dialog = Some(state);
            }
        }

        // -------------------------------------------------------------
        // FFmpeg Missing Diagnostic Modal
        // -------------------------------------------------------------
        if self.ffmpeg_missing_alert && !self.splash_screen.is_active {
            let mut is_open = self.ffmpeg_missing_alert;
            let title = match self.lang() {
                Language::Turkish => "⚠ triM. — FFmpeg Medya Motoru Bulunamadı",
                Language::English => "⚠ triM. — FFmpeg Media Engine Not Found",
            };
            egui::Window::new(title)
                .id(egui::Id::new("ffmpeg_missing_modal"))
                .open(&mut is_open)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .resizable(false)
                .collapsible(false)
                .fixed_size(egui::Vec2::new(500.0, 300.0))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("⚠")
                                .size(32.0)
                                .color(egui::Color32::from_rgb(240, 180, 50)),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(match self.lang() {
                                Language::Turkish => "FFmpeg veya FFprobe Bulunamadı!",
                                Language::English => "FFmpeg or FFprobe Not Detected!",
                            })
                            .font(egui::FontId::proportional(15.0))
                            .strong()
                            .color(egui::Color32::WHITE),
                        );
                    });

                    ui.add_space(10.0);
                    let info_text = {
                        #[cfg(target_os = "windows")]
                        {
                            match self.lang() {
                                Language::Turkish => "triM. video çözümleme, oynatma, dalga formu çıkarma ve dışa aktarma için FFmpeg ve FFprobe motoruna ihtiyaç duyar.\n\nNasıl çözebilirsiniz:\n1. Windows installer ile kurun (install.ps1 otomatik FFmpeg kurar)\n2. PowerShell'de şunu çalıştırın: winget install Gyan.FFmpeg\n3. Ya da ffmpeg.exe ve ffprobe.exe dosyalarını trim.exe yanına kopyalayın.",
                                Language::English => "triM. requires FFmpeg and FFprobe binaries for video decoding, preview playback, waveform extraction, and exporting.\n\nHow to resolve:\n1. Run install.ps1 (automatically bundles and configures FFmpeg)\n2. Run in PowerShell: winget install Gyan.FFmpeg\n3. Or place ffmpeg.exe & ffprobe.exe next to trim.exe.",
                            }
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            match self.lang() {
                                Language::Turkish => "triM. video çözümleme, oynatma, dalga formu çıkarma ve dışa aktarma için FFmpeg ve FFprobe motoruna ihtiyaç duyar.\n\nNasıl çözebilirsiniz:\n1. Paket yöneticisinden kurun: sudo apt install ffmpeg (veya pacman -S ffmpeg / dnf install ffmpeg)\n2. Ya da install.sh betiğini çalıştırın\n3. Ya da ffmpeg ve ffprobe ikili dosyalarını PATH'e veya trim ikilisinin yanına ekleyin.",
                                Language::English => "triM. requires FFmpeg and FFprobe binaries for video decoding, preview playback, waveform extraction, and exporting.\n\nHow to resolve:\n1. Install via package manager: sudo apt install ffmpeg (or pacman -S ffmpeg / dnf install ffmpeg)\n2. Or run install.sh\n3. Or place ffmpeg & ffprobe binaries in PATH or next to the trim executable.",
                            }
                        }
                    };
                    ui.label(
                        egui::RichText::new(info_text)
                            .size(12.0)
                            .color(egui::Color32::from_rgb(210, 215, 225)),
                    );

                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        let retry_lbl = match self.lang() {
                            Language::Turkish => "↺ Tekrar Kontrol Et",
                            Language::English => "↺ Check Again",
                        };
                        if ui.button(retry_lbl).clicked() {
                            trim_media::refresh_ffmpeg_availability();
                            if trim_media::is_ffmpeg_available() && trim_media::is_ffprobe_available() {
                                self.ffmpeg_missing_alert = false;
                                self.status_message = match self.lang() {
                                    Language::Turkish => "FFmpeg başarıyla tespit edildi! Sistem hazır.".to_string(),
                                    Language::English => "FFmpeg detected successfully! System ready.".to_string(),
                                };
                            } else {
                                self.status_message = match self.lang() {
                                    Language::Turkish => "FFmpeg hala tespit edilemedi. Lütfen FFmpeg'i kurun veya PATH'e ekleyin.".to_string(),
                                    Language::English => "FFmpeg still not detected. Please install FFmpeg or add it to PATH.".to_string(),
                                };
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let close_lbl = match self.lang() {
                                Language::Turkish => "Yine de Devam Et",
                                Language::English => "Continue Anyway",
                            };
                            if ui.button(close_lbl).clicked() {
                                self.ffmpeg_missing_alert = false;
                            }
                        });
                    });
                });
            if !is_open {
                self.ffmpeg_missing_alert = false;
            }
        }
    }
    /// Opens `folder` in the platform file manager via `ChildGuard`-backed `TrackedChildPool`
    /// so finished launchers are reaped non-blockingly and any still-running process is killed on Drop.
    fn open_folder_tracked(&mut self, folder: &std::path::Path) {
        #[cfg(target_os = "windows")]
        let program = "explorer";
        #[cfg(target_os = "macos")]
        let program = "open";
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let program = "xdg-open";
        let mut cmd = std::process::Command::new(program);
        cmd.arg(folder);
        let _ = self.helper_processes.spawn(&mut cmd);
    }
}
