//! Main triM. desktop application implementing `eframe::App` with Premiere Pro 4-Quadrant layout.

use crate::canvas::TimelineCanvasState;
use crate::dialogs::*;
use crate::file_dialog::FileDialogBridge;
use crate::i18n::Language;
use crate::media_bin::{LeftPanelTab, MediaFilterType};
use crate::scopes_panel::ScopesPanelState;
use crate::shortcuts::ShortcutProfile;
use crate::theme::*;
use crate::toolbar::EditTool;
use crate::vu_meter::VuMeterState;

use egui::{Color32, Key, RichText, TextureHandle};
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use trim_commands::commands::*;
use trim_commands::UndoStack;
use trim_core::preferences::UserPreferences;
use trim_core::project::Project;
use trim_export::queue::{RenderQueue, RenderQueueEvent};
use trim_export::runner::{ExportMessage, ExportRunner};
use trim_media::{
    AudioMixer, AudioStreamPlayer, ThumbnailGenerator, VideoEngine, VoiceoverRecorder,
    WaveformGenerator,
};
use trim_project::AutosaveManager;
use trim_timeline::{
    find_next_edit_point, find_prev_edit_point, ripple_trim_head, ripple_trim_tail,
    split_at_playhead,
};

pub mod dialogs_dispatch;
pub mod media_actions;
pub mod monitors;
pub mod playback;
pub mod project_io;
pub mod side_panels;
pub mod timeline_actions;
pub mod timeline_panel;
pub mod top_header;

pub struct ActiveVoiceover {
    pub track_id: String,
    pub start_timeline_time: f64,
    pub recorder: VoiceoverRecorder,
}

#[derive(Default)]
pub struct SourceMonitorState {
    pub media_id: Option<String>,
    pub current_time: f64,
    pub mark_in: Option<f64>,
    pub mark_out: Option<f64>,
    pub is_playing: bool,
    pub clock_start: Option<Instant>,
    pub clock_start_time: f64,
    pub texture: Option<TextureHandle>,
    pub last_decoded_time: Option<f64>,
    pub last_decoded_id: Option<String>,
    pub decoder: trim_media::AsyncFrameDecoder,
}

#[derive(Debug, Clone)]
pub enum BinDialogMode {
    Create,
    Rename(String),
}

#[derive(Debug, Clone)]
pub struct BinDialogState {
    pub mode: BinDialogMode,
    pub name_input: String,
}

pub struct TrimApp {
    pub project: Project,
    pub undo_stack: UndoStack,
    pub current_time: f64,
    pub is_playing: bool,
    pub playback_speed: f64,
    pub clock_start: Option<Instant>,
    pub clock_start_time: f64,

    pub canvas_state: TimelineCanvasState,
    pub vu_state: VuMeterState,
    pub waveform_gen: WaveformGenerator,
    pub thumbnail_gen: ThumbnailGenerator,
    pub video_engine: VideoEngine,
    pub audio_mixer: AudioMixer,
    pub audio_player: AudioStreamPlayer,
    pub audio_worker: trim_media::audio::AudioPlaybackWorker,
    pub last_rendered_time: Option<f64>,
    pub last_rendered_dirty: bool,

    // Source Monitor state
    pub source_state: SourceMonitorState,

    // NLE Track Target Patching & Tools
    pub target_video_track: String,
    pub target_audio_track: String,
    pub current_edit_tool: EditTool,
    pub snap_enabled: bool,

    // Media Bin state
    pub media_bin_search: String,
    pub media_bin_is_grid: bool,
    pub media_bin_thumb_size: f32,
    pub media_bin_current_bin: Option<String>,
    pub media_bin_filter: MediaFilterType,
    pub media_bin_textures: HashMap<String, TextureHandle>,
    pub bin_dialog: Option<BinDialogState>,

    pub export_dialog: ExportDialog,
    pub speed_dialog: SpeedDialog,
    pub silence_dialog: SilenceDialog,
    pub transcribe_dialog: TranscribeDialog,
    pub screen_record_dialog: ScreenRecordDialog,
    pub title_dialog: TitleDialog,
    pub recovery_dialog: RecoveryDialog,
    pub audio_mixer_dialog: AudioMixerDialog,
    pub welcome_dialog: WelcomeDialog,
    pub splash_screen: SplashScreen,
    pub shortcuts_dialog: ShortcutsDialog,
    pub preferences_dialog: PreferencesDialog,
    pub preferences: UserPreferences,
    pub relink_dialog: RelinkDialog,
    pub render_queue_dialog: RenderQueueDialog,
    pub autosave_mgr: AutosaveManager,

    pub export_runner: ExportRunner,
    pub export_rx: Option<Receiver<ExportMessage>>,
    pub render_queue: RenderQueue,

    pub monitor_texture: Option<TextureHandle>,
    pub monitor_gpu_texture_id: Option<egui::TextureId>,
    pub gpu_compositor: Option<crate::gpu_compositor::GpuCompositor>,
    pub last_rendered_monitor_frame: Option<std::sync::Arc<image::RgbaImage>>,
    pub selected_media_id: Option<String>,
    pub status_message: String,
    pub last_autosave: Instant,
    pub show_about_dialog: bool,
    pub show_source_monitor: bool,
    pub show_safe_zones: bool,
    pub active_left_tab: LeftPanelTab,
    pub active_voiceover: Option<ActiveVoiceover>,
    pub use_proxies: bool,
    pub shortcut_profile: ShortcutProfile,
    pub proxy_tx: std::sync::mpsc::Sender<(String, Result<String, String>)>,
    pub proxy_rx: Option<Receiver<(String, Result<String, String>)>>,
    pub proxy_generating_ids: Vec<String>,
    pub close_confirm_dialog: CloseConfirmDialog,
    pub confirmed_exit: bool,
    pub last_saved_undo_len: usize,
    pub file_dialog_bridge: FileDialogBridge,
    pub clipboard_clips: Vec<(
        trim_core::track::TrackType,
        usize,
        f64,
        trim_core::clip::Clip,
    )>,
    pub ffmpeg_missing_alert: bool,
    pub is_monitor_fullscreen: bool,
    pub monitor_text_drag_orig_pos: Option<(String, String, (f32, f32))>,
    pub monitor_text_edit_orig_content: Option<(String, String, String, String)>,
    pub scopes_panel: ScopesPanelState,
    pub split_wipe_enabled: bool,
    pub split_wipe_pos: f32,
    pub multicam_mode: bool,
    pub multicam_active_angle: usize,
    pub paste_attrs_dialog_open: bool,
    pub paste_attr_color: bool,
    pub paste_attr_transform: bool,
    pub paste_attr_audio: bool,
    pub paste_attr_speed: bool,
    pub last_scope_rgba: Option<(Vec<u8>, u32, u32)>,
    pub wb_eyedropper_target: Option<(String, String)>,
    pub bg_task_tx: std::sync::mpsc::Sender<BackgroundTaskMsg>,
    pub bg_task_rx: std::sync::mpsc::Receiver<BackgroundTaskMsg>,
}

pub enum BackgroundTaskMsg {
    SceneDetectDone {
        track_id: String,
        clip_id: String,
        result: Result<Vec<f64>, String>,
    },
    SilenceDetectDone {
        source_in: f64,
        source_out: f64,
        padding: f64,
        result: Result<Vec<(f64, f64)>, String>,
    },
    TranscribeDone {
        result: Result<Vec<trim_core::subtitles::SubtitleEntry>, String>,
    },
    BundleDone {
        target_dir: std::path::PathBuf,
        result: Result<(usize, u64), String>,
    },
    MotionTrackDone {
        track_id: String,
        clip_id: String,
        mask_index: usize,
        old_masks: Vec<trim_core::mask::Mask>,
        result: Result<Vec<(f64, f64, f64)>, String>,
    },
    MediaAnalyzed {
        bin_id: Option<String>,
        result: Result<trim_core::MediaItem, String>,
    },
    ProjectAudioProbed {
        media_id: String,
        codec: String,
        channels: u32,
        sample_rate: u32,
        file_path: String,
    },
}

impl Default for TrimApp {
    fn default() -> Self {
        let project = Project::default();
        let v_tracks = project.timeline.get_video_tracks();
        let target_video_track = v_tracks
            .iter()
            .find(|t| t.name == "V1")
            .copied()
            .or_else(|| v_tracks.last().copied())
            .map(|t| t.id.clone())
            .unwrap_or_else(|| "v1".to_string());

        let a_tracks = project.timeline.get_audio_tracks();
        let target_audio_track = a_tracks
            .iter()
            .find(|t| t.name == "A1")
            .copied()
            .or_else(|| a_tracks.first().copied())
            .map(|t| t.id.clone())
            .unwrap_or_else(|| "a1".to_string());
        let (bg_task_tx, bg_task_rx) = std::sync::mpsc::channel();
        let (proxy_tx, proxy_rx) = std::sync::mpsc::channel();

        Self {
            project,
            undo_stack: UndoStack::new(),
            current_time: 0.0,
            is_playing: false,
            playback_speed: 1.0,
            clock_start: None,
            clock_start_time: 0.0,
            canvas_state: TimelineCanvasState::default(),
            vu_state: VuMeterState::default(),
            waveform_gen: WaveformGenerator::default(),
            thumbnail_gen: ThumbnailGenerator::default(),
            video_engine: VideoEngine::default(),
            audio_mixer: {
                // initialized below via Default
                AudioMixer::default()
            },
            audio_player: AudioStreamPlayer::default(),
            audio_worker: trim_media::audio::AudioPlaybackWorker::new(AudioMixer::default()),
            last_rendered_time: None,
            last_rendered_dirty: false,
            source_state: SourceMonitorState::default(),
            target_video_track,
            target_audio_track,
            current_edit_tool: EditTool::Select,
            snap_enabled: true,
            media_bin_search: String::new(),
            media_bin_is_grid: true,
            media_bin_thumb_size: 105.0,
            media_bin_current_bin: None,
            media_bin_filter: MediaFilterType::All,
            media_bin_textures: HashMap::new(),
            bin_dialog: None,
            export_dialog: ExportDialog::default(),
            speed_dialog: SpeedDialog::default(),
            silence_dialog: SilenceDialog::default(),
            transcribe_dialog: TranscribeDialog::default(),
            screen_record_dialog: ScreenRecordDialog::default(),
            title_dialog: TitleDialog::default(),
            recovery_dialog: RecoveryDialog::default(),
            audio_mixer_dialog: AudioMixerDialog::default(),
            welcome_dialog: WelcomeDialog::default(),
            splash_screen: SplashScreen::default(),
            shortcuts_dialog: ShortcutsDialog::default(),
            preferences_dialog: PreferencesDialog::default(),
            preferences: UserPreferences::default(),
            relink_dialog: RelinkDialog::default(),
            render_queue_dialog: RenderQueueDialog::default(),
            autosave_mgr: AutosaveManager::default(),
            export_runner: ExportRunner::default(),
            export_rx: None,
            render_queue: RenderQueue::default(),
            monitor_texture: None,
            monitor_gpu_texture_id: None,
            gpu_compositor: None,
            last_rendered_monitor_frame: None,
            selected_media_id: None,
            status_message: "Hazır".to_string(),
            last_autosave: Instant::now(),
            show_about_dialog: false,
            show_source_monitor: false,
            show_safe_zones: false,
            active_left_tab: LeftPanelTab::Project,
            active_voiceover: None,
            use_proxies: false,
            shortcut_profile: ShortcutProfile::TrimDefault,
            proxy_tx,
            proxy_rx: Some(proxy_rx),
            proxy_generating_ids: Vec::new(),
            close_confirm_dialog: CloseConfirmDialog::default(),
            confirmed_exit: false,
            last_saved_undo_len: 0,
            file_dialog_bridge: FileDialogBridge::default(),
            clipboard_clips: Vec::new(),
            ffmpeg_missing_alert: false,
            is_monitor_fullscreen: false,
            monitor_text_drag_orig_pos: None,
            monitor_text_edit_orig_content: None,
            scopes_panel: ScopesPanelState::default(),
            split_wipe_enabled: false,
            split_wipe_pos: 0.5,
            multicam_mode: false,
            multicam_active_angle: 0,
            paste_attrs_dialog_open: false,
            paste_attr_color: true,
            paste_attr_transform: true,
            paste_attr_audio: true,
            paste_attr_speed: false,
            last_scope_rgba: None,
            wb_eyedropper_target: None,
            bg_task_tx,
            bg_task_rx,
        }
    }
}

impl TrimApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_theme(&cc.egui_ctx);
        let prefs = UserPreferences::load();
        crate::theme::apply_studio_theme(&cc.egui_ctx, &prefs.theme_id);
        cc.egui_ctx.set_pixels_per_point(prefs.ui_scale);

        // Pre-warm hardware encoder detection in background so preferences/export dialogs are instant
        std::thread::spawn(|| {
            let _ = trim_media::detect_available_encoders();
        });

        let mut app = Self {
            gpu_compositor: cc
                .gl
                .as_ref()
                .and_then(|gl| crate::gpu_compositor::GpuCompositor::new(gl.clone())),
            snap_enabled: prefs.snap_enabled_by_default,
            use_proxies: prefs.use_proxies_by_default,
            preferences: prefs.clone(),
            ..Self::default()
        };

        let budget_mb = prefs.max_memory_budget_mb.max(128);
        app.audio_mixer.set_max_cache_bytes(budget_mb * 1024 * 1024);
        app.video_engine
            .set_frame_cache_capacity((budget_mb * 300 / 512).max(50));
        app.shortcut_profile = match prefs.shortcut_profile.as_str() {
            "Adobe Premiere Pro" | "Premiere" => ShortcutProfile::PremierePro,
            "DaVinci Resolve" | "Resolve" => ShortcutProfile::DaVinciResolve,
            _ => ShortcutProfile::TrimDefault,
        };
        app.audio_mixer
            .limiter
            .set_ceiling_dbfs(if prefs.master_limiter_enabled {
                prefs.limiter_ceiling_db
            } else {
                0.0
            });

        if !prefs.show_splash_screen {
            app.splash_screen.dismiss();
        }
        if !prefs.show_welcome_on_startup {
            app.welcome_dialog.close();
        }

        // Check crash recovery
        if let Some(rec_path) = app.autosave_mgr.check_recovery(&app.project) {
            app.recovery_dialog.prompt(rec_path);
        }

        // Check FFmpeg & FFprobe media engine availability
        let ffmpeg_ok = trim_media::is_ffmpeg_available() && trim_media::is_ffprobe_available();
        if !ffmpeg_ok {
            log::warn!("FFmpeg or FFprobe binaries not detected at startup!");
            app.ffmpeg_missing_alert = true;
        }

        app
    }

    /// Returns active UI language from user preferences.
    pub fn lang(&self) -> Language {
        Language::from_code(&self.preferences.language)
    }
}

impl eframe::App for TrimApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Intercept window close request if there are unsaved changes
        if ctx.input(|i| i.viewport().close_requested()) && !self.confirmed_exit {
            if self.has_unsaved_changes() || self.close_confirm_dialog.is_open {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                if !self.close_confirm_dialog.is_open {
                    let name = self.project.settings.name.clone();
                    self.close_confirm_dialog
                        .prompt(PendingCloseAction::ExitApp, name);
                }
            } else {
                self.confirmed_exit = true;
            }
        }

        self.update_playback_tick();
        self.update_monitor_frame(ctx, frame);
        self.update_source_monitor_frame(ctx);
        self.update_media_bin_thumbnails(ctx);

        if self.is_playing || self.source_state.is_playing {
            ctx.request_repaint();
        } else if self.video_engine.is_decoding_seek {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        // Window drag & drop files from OS file manager
        let dropped_paths: Vec<std::path::PathBuf> = ctx.input(|i| {
            let mut paths = Vec::new();
            for file in &i.raw.dropped_files {
                paths.extend(crate::app::media_actions::parse_dropped_paths(file));
            }
            paths
        });
        for path in dropped_paths {
            self.import_media_path(&path);
        }

        // Autosave check
        let autosave_secs = self.preferences.autosave_interval_secs;
        if autosave_secs > 0
            && self.last_autosave.elapsed().as_secs() >= autosave_secs
            && self.project.is_dirty
        {
            let _ = self.autosave_mgr.save_snapshot(&self.project);
            self.last_autosave = Instant::now();
        }

        // Poll background OS file dialogs non-blockingly
        self.poll_file_dialogs(ctx);

        // Check background export messages
        let mut export_finished = false;
        if let Some(rx) = &self.export_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ExportMessage::Progress(pct) => {
                        self.export_dialog.progress = pct;
                        self.export_dialog.status_text = format!("{:.1}%", pct);
                    }
                    ExportMessage::Status(s) => {
                        self.export_dialog.status_text = s;
                    }
                    ExportMessage::Finished(res) => {
                        self.export_dialog.is_exporting = false;
                        self.export_dialog.is_open = false;
                        export_finished = true;
                        match res {
                            Ok(p) => {
                                self.status_message = format!("Dışa aktarma tamamlandı: {}", p)
                            }
                            Err(e) => self.status_message = format!("Dışa aktarma hatası: {}", e),
                        }
                    }
                }
            }
        }
        if export_finished {
            self.export_rx = None;
        }

        // Poll sequential background render queue
        let queue_events = self.render_queue.poll();
        for ev in queue_events {
            match ev {
                RenderQueueEvent::JobProgress { .. } => {
                    ctx.request_repaint();
                }
                RenderQueueEvent::JobCompleted { output_path, .. } => {
                    self.status_message = format!("Kuyruk işi tamamlandı: {}", output_path);
                    ctx.request_repaint();
                }
                RenderQueueEvent::JobFailed { error, .. } => {
                    self.status_message = format!("Kuyruk işinde hata: {}", error);
                    ctx.request_repaint();
                }
                RenderQueueEvent::QueueFinished => {
                    self.status_message = "Tüm render kuyruğu tamamlandı!".to_string();
                    ctx.request_repaint();
                }
            }
        }

        // Check background proxy generation messages
        if let Some(rx) = &self.proxy_rx {
            while let Ok((m_id, res)) = rx.try_recv() {
                self.proxy_generating_ids.retain(|id| id != &m_id);
                match res {
                    Ok(p_path) => {
                        if let Some(m) = self.project.get_media_mut(&m_id) {
                            m.proxy_path = Some(p_path);
                        }
                        self.status_message = "⚡ 720p Proxy başarıyla oluşturuldu!".to_string();
                    }
                    Err(e) => {
                        self.status_message = format!("Proxy hatası: {}", e);
                    }
                }
            }
        }

        // Poll asynchronous background tasks (scene cut, silence detect, Whisper transcription)
        let bg_msgs: Vec<BackgroundTaskMsg> = self.bg_task_rx.try_iter().collect();
        for msg in bg_msgs {
            ctx.request_repaint();
            match msg {
                BackgroundTaskMsg::SceneDetectDone {
                    track_id,
                    clip_id,
                    result,
                } => match result {
                    Ok(source_cuts) => {
                        if source_cuts.is_empty() {
                            self.status_message =
                                "Klipte belirgin sahne geçişi algılanmadı (Eşik: 0.40)".to_string();
                        } else if let Some(clip) = self
                            .project
                            .timeline
                            .get_track_by_id(&track_id)
                            .and_then(|t| t.get_clip_by_id(&clip_id))
                            .cloned()
                        {
                            let timeline_cuts: Vec<f64> = source_cuts
                                .iter()
                                .map(|&s| clip.timeline_in + (s - clip.source_in) / clip.speed)
                                .filter(|&t| {
                                    t > clip.timeline_in + 0.05 && t < clip.timeline_out - 0.05
                                })
                                .collect();
                            let count = timeline_cuts.len();
                            if count == 0 {
                                self.status_message =
                                    "Klip sınırları içinde yeni sahne kesimi bulunamadı"
                                        .to_string();
                            } else {
                                let cmd = Box::new(trim_commands::BatchSplitClipCommand::new(
                                    &track_id,
                                    &clip_id,
                                    timeline_cuts,
                                ));
                                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                self.project.mark_dirty();
                                self.status_message = format!(
                                    "Akıllı Kesim Tamamlandı: {} yeni sahne algılandı ve bölündü",
                                    count
                                );
                            }
                        }
                    }
                    Err(e) => {
                        self.status_message = format!("Sahne algılama başarısız: {}", e);
                    }
                },
                BackgroundTaskMsg::SilenceDetectDone {
                    source_in,
                    source_out,
                    padding,
                    result,
                } => match result {
                    Ok(silences) => {
                        let speech = trim_media::calculate_speech_intervals(
                            source_in, source_out, &silences, padding,
                        );
                        self.silence_dialog.set_analysis_results(silences, speech);
                    }
                    Err(e) => {
                        self.silence_dialog.status_text = format!("Hata: {}", e);
                        self.silence_dialog.is_analyzing = false;
                    }
                },
                BackgroundTaskMsg::TranscribeDone { result } => match result {
                    Ok(entries) => {
                        let count = entries.len();
                        let subtitle_clips =
                            trim_core::subtitles::subtitle_entries_to_clips(&entries, 42.0);
                        let target_tr = self
                            .project
                            .timeline
                            .get_video_tracks()
                            .first()
                            .map(|t| t.id.clone())
                            .unwrap_or_else(|| self.project.timeline.add_new_video_track());
                        let cmds: Vec<Box<dyn trim_commands::Command>> = subtitle_clips
                            .into_iter()
                            .map(|c| {
                                Box::new(AddClipCommand::new(&target_tr, c))
                                    as Box<dyn trim_commands::Command>
                            })
                            .collect();
                        if !cmds.is_empty() {
                            let macro_cmd = Box::new(trim_commands::MacroCommand::new(
                                "Otomatik Deşifre Altyazıları",
                                cmds,
                            ));
                            let _ = self.undo_stack.push(macro_cmd, &mut self.project.timeline);
                            self.project.mark_dirty();
                        }
                        self.transcribe_dialog.close();
                        self.status_message =
                            format!("{} adet altyazı başarıyla oluşturuldu ve eklendi", count);
                    }
                    Err(e) => {
                        self.transcribe_dialog.is_transcribing = false;
                        self.transcribe_dialog.status_message = e;
                    }
                },
                BackgroundTaskMsg::BundleDone { target_dir, result } => match result {
                    Ok((count, bytes)) => {
                        let mb = bytes as f64 / (1024.0 * 1024.0);
                        self.status_message = format!(
                            "Proje başarıyla paketlendi: {} dosya ({:.1} MB) -> {:?}",
                            count, mb, target_dir
                        );
                    }
                    Err(e) => {
                        self.status_message = format!("Paketleme hatası: {}", e);
                    }
                },
                BackgroundTaskMsg::MotionTrackDone {
                    track_id,
                    clip_id,
                    mask_index,
                    old_masks,
                    result,
                } => match result {
                    Ok(samples) => {
                        let mut new_masks = old_masks.clone();
                        if let Some(target_mask) = new_masks.get_mut(mask_index) {
                            let mut kf_x = Vec::with_capacity(samples.len());
                            let mut kf_y = Vec::with_capacity(samples.len());
                            for (t, cx, cy) in &samples {
                                kf_x.push(trim_core::keyframe::Keyframe::new(
                                    *t,
                                    (cx + 0.5).clamp(0.02, 0.98),
                                    trim_core::keyframe::Interpolation::EaseInOut,
                                ));
                                kf_y.push(trim_core::keyframe::Keyframe::new(
                                    *t,
                                    (cy + 0.5).clamp(0.02, 0.98),
                                    trim_core::keyframe::Interpolation::EaseInOut,
                                ));
                            }
                            target_mask.keyframes.insert("pos_x".to_string(), kf_x);
                            target_mask.keyframes.insert("pos_y".to_string(), kf_y);
                        }
                        let cmd = Box::new(trim_commands::SetClipMasksCommand::new(
                            track_id, clip_id, old_masks, new_masks,
                        ));
                        if self
                            .undo_stack
                            .push(cmd, &mut self.project.timeline)
                            .is_ok()
                        {
                            self.project.mark_dirty();
                            self.last_rendered_monitor_frame = None;
                            self.video_engine.last_rendered_frame = None;
                            self.status_message = format!(
                                "🎯 Maske hareket takibi tamamlandı ({} anahtar kare oluşturuldu)",
                                samples.len()
                            );
                        }
                    }
                    Err(err) => {
                        self.status_message = format!("⚠ Hareket takibi başarısız: {}", err);
                    }
                },
                BackgroundTaskMsg::MediaAnalyzed { bin_id, result } => match result {
                    Ok(item) => self.finish_import_media_item(item, bin_id),
                    Err(e) => {
                        self.status_message = format!("Medya analizi başarısız: {}", e);
                    }
                },
                BackgroundTaskMsg::ProjectAudioProbed {
                    media_id,
                    codec,
                    channels,
                    sample_rate,
                    file_path,
                } => {
                    if let Some(item) = self.project.media_pool.get_mut(&media_id) {
                        item.audio_codec = codec;
                        item.audio_channels = channels;
                        item.audio_sample_rate = sample_rate;
                    }
                    self.audio_mixer.preload_audio_async(&file_path);
                }
            }
        }

        if self.active_voiceover.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

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

        if self.is_monitor_fullscreen {
            self.render_fullscreen_monitor(ctx);
            self.splash_screen.show(ctx);
            suppress_linux_ime_for_text_input(ctx);
            return;
        }

        self.render_top_header(ctx);

        // -------------------------------------------------------------
        // Bottom Status Bar
        // -------------------------------------------------------------
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let dirty = if self.has_unsaved_changes() { " *" } else { "" };
                ui.label(
                    RichText::new(format!(
                        "Proje: {}{} | Sekans: {}x{} @ {:.0}fps | {}",
                        self.project.settings.name,
                        dirty,
                        self.project.timeline.width,
                        self.project.timeline.height,
                        self.project.timeline.fps,
                        self.status_message
                    ))
                    .size(10.5)
                    .color(Color32::from_rgb(0x85, 0x85, 0x95)),
                );
            });
        });

        self.render_timeline_panel(ctx);

        self.render_side_panels(ctx);

        // -------------------------------------------------------------
        // UPPER-CENTER: Monitors (Fills the remaining upper workspace)
        // -------------------------------------------------------------
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.show_source_monitor {
                ui.columns(2, |cols| {
                    self.render_source_monitor(&mut cols[0]);
                    self.render_program_monitor(&mut cols[1]);
                });
            } else {
                self.render_program_monitor(ui);
            }
        });

        self.handle_modal_dialogs(ctx);

        // -------------------------------------------------------------
        // Startup Splash Screen (DaVinci Resolve / Kdenlive Style)
        // -------------------------------------------------------------
        self.splash_screen.show(ctx);

        suppress_linux_ime_for_text_input(ctx);
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        if let (Some(comp), Some(gl_ctx)) = (self.gpu_compositor.as_mut(), gl) {
            comp.destroy(gl_ctx);
        }
    }
}

/// On Linux (X11 + ibus/fcitx) winit routes keystrokes through the input method whenever a
/// text field requests IME. egui-winit deliberately ignores `Ime::Commit` events on Linux
/// (https://github.com/emilk/egui/issues/5008), so every non-ASCII character such as
/// `ç ğ ı ö ş ü İ` is silently swallowed. Disabling IME for the window while a text field has
/// focus makes winit deliver the layout's characters directly as regular keyboard text.
///
/// This must run at the very end of `update`: egui-winit applies the platform output (which
/// enables IME) before viewport commands are processed, so our command wins in the same frame.
fn suppress_linux_ime_for_text_input(ctx: &egui::Context) {
    if cfg!(target_os = "linux") && ctx.wants_keyboard_input() {
        ctx.send_viewport_cmd(egui::ViewportCommand::IMEAllowed(false));
    }
}
