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

use egui::{Color32, RichText, TextureHandle};
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use trim_commands::UndoStack;
use trim_core::preferences::UserPreferences;
use trim_core::project::Project;
use trim_export::queue::RenderQueue;
use trim_export::runner::{ExportMessage, ExportRunner};
use trim_media::{
    AudioMixer, AudioStreamPlayer, ThumbnailGenerator, VideoEngine, VoiceoverRecorder,
    WaveformGenerator,
};
use trim_project::AutosaveManager;

pub mod background_poll;
pub mod canvas_dispatch;
pub mod dialogs_dispatch;
pub mod media_actions;
pub mod monitors;
pub mod playback;
pub mod project_io;
pub mod shortcuts;
pub mod side_panels;
pub mod timeline_actions;
pub mod timeline_header;
pub mod timeline_panel;
pub mod top_header;
pub mod track_headers;

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
    pub proxy_cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub close_confirm_dialog: CloseConfirmDialog,
    pub confirmed_exit: bool,
    pub last_saved_undo_len: usize,
    pub file_dialog_bridge: FileDialogBridge,
    pub helper_processes: trim_media::ffmpeg::TrackedChildPool,
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
            proxy_cancel_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            close_confirm_dialog: CloseConfirmDialog::default(),
            confirmed_exit: false,
            last_saved_undo_len: 0,
            file_dialog_bridge: FileDialogBridge::default(),
            helper_processes: trim_media::ffmpeg::TrackedChildPool::default(),
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
            "Final Cut Pro" | "FCP" => ShortcutProfile::FinalCutPro,
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

        self.poll_background_work(ctx);
        self.handle_global_shortcuts(ctx);

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
            let lang = self.lang();
            ui.horizontal(|ui| {
                let dirty = if self.has_unsaved_changes() { " *" } else { "" };
                ui.label(
                    RichText::new(format!(
                        "{}: {}{} | {}: {}x{} @ {:.0}fps | {}",
                        lang.t("status_project"),
                        self.project.settings.name,
                        dirty,
                        lang.t("status_sequence"),
                        self.project.timeline.width,
                        self.project.timeline.height,
                        self.project.timeline.fps,
                        self.status_message
                    ))
                    .size(10.5)
                    .color(Color32::from_rgb(0x85, 0x85, 0x95)),
                );

                if !self.proxy_generating_ids.is_empty() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(lang.t("btn_cancel")).clicked() {
                            self.proxy_cancel_flag
                                .store(true, std::sync::atomic::Ordering::Release);
                            self.status_message = lang.t("status_cancelling").to_string();
                        }
                        let proxy_lbl = format!(
                            "{} ({})",
                            lang.t("status_proxy_running"),
                            self.proxy_generating_ids.len()
                        );
                        ui.label(
                            RichText::new(proxy_lbl)
                                .size(10.5)
                                .color(Color32::from_rgb(0xC8, 0xB4, 0x64)),
                        );
                        ui.spinner();
                    });
                }
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
