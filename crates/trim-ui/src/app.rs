//! Main triM. desktop application implementing `eframe::App` with Premiere Pro 4-Quadrant layout.

use crate::canvas::{
    calculate_total_tracks_height, render_timeline_canvas, CanvasAction, TimelineCanvasState,
};
use crate::dialogs::*;
use crate::file_dialog::FileDialogBridge;
use crate::inspector::{render_inspector_panel, InspectorAction};
use crate::media_bin::{render_media_bin, LeftPanelTab, MediaBinAction, MediaFilterType};
use crate::monitor::{ActiveTextOverlay, MonitorAction, MonitorMode, MonitorWidget};
use crate::ruler::{render_time_ruler, RulerAction};
use crate::shortcuts::ShortcutProfile;
use crate::theme::*;
use crate::toolbar::{render_vertical_toolbar, EditTool};
use crate::track_header::{render_track_header, TrackHeaderAction, TRACK_HEADER_WIDTH};
use crate::vu_meter::{VuMeterState, VuMeterWidget};

use egui::{Button, Color32, Key, RichText, Stroke, TextureHandle, Vec2};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Instant;

use trim_commands::commands::*;
use trim_commands::UndoStack;
use trim_core::clip::{Clip, TextOverlay};
use trim_core::media::MediaType;
use trim_core::project::Project;
use trim_core::time::seconds_to_timecode;
use trim_core::track::TrackType;
use trim_export::runner::{ExportMessage, ExportRunner};
use trim_media::{
    generate_title_image, AudioMixer, AudioStreamPlayer, FFprobeAnalyzer, ThumbnailGenerator,
    VideoEngine, VoiceoverRecorder, WaveformGenerator,
};
use trim_project::{load_project, AutosaveManager};
use trim_timeline::{
    find_next_edit_point, find_prev_edit_point, ripple_trim_head, ripple_trim_tail,
    split_at_playhead,
};

pub mod media_actions;
pub mod playback;
pub mod project_io;
pub mod timeline_actions;

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
    pub shortcuts_dialog: ShortcutsDialog,
    pub relink_dialog: RelinkDialog,
    pub autosave_mgr: AutosaveManager,

    pub export_runner: ExportRunner,
    pub export_rx: Option<Receiver<ExportMessage>>,

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
    pub proxy_rx: Option<Receiver<(String, Result<String, String>)>>,
    pub proxy_generating_ids: Vec<String>,
    pub close_confirm_dialog: CloseConfirmDialog,
    pub confirmed_exit: bool,
    pub last_saved_undo_len: usize,
    pub file_dialog_bridge: FileDialogBridge,
    pub clipboard_clips: Vec<(trim_core::track::TrackType, f64, trim_core::clip::Clip)>,
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
            audio_mixer: AudioMixer::default(),
            audio_player: AudioStreamPlayer::default(),
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
            shortcuts_dialog: ShortcutsDialog::default(),
            relink_dialog: RelinkDialog::default(),
            autosave_mgr: AutosaveManager::default(),
            export_runner: ExportRunner::default(),
            export_rx: None,
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
            proxy_rx: None,
            proxy_generating_ids: Vec::new(),
            close_confirm_dialog: CloseConfirmDialog::default(),
            confirmed_exit: false,
            last_saved_undo_len: 0,
            file_dialog_bridge: FileDialogBridge::default(),
            clipboard_clips: Vec::new(),
        }
    }
}

impl TrimApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_theme(&cc.egui_ctx);
        let mut app = Self {
            gpu_compositor: cc
                .gl
                .as_ref()
                .and_then(|gl| crate::gpu_compositor::GpuCompositor::new(gl.clone())),
            ..Self::default()
        };

        // Check crash recovery
        if let Some(rec_path) = app.autosave_mgr.check_recovery(&app.project) {
            app.recovery_dialog.prompt(rec_path);
        }

        app
    }

    pub fn render_source_monitor(&mut self, source_ui: &mut egui::Ui) {
        let source_media = self
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
        let source_tc = seconds_to_timecode(self.source_state.current_time, source_fps);
        let source_dur_tc = seconds_to_timecode(source_dur, source_fps);
        let source_aspect = source_media
            .map(|m| (m.width as f32 / m.height.max(1) as f32).max(0.1))
            .unwrap_or(16.0 / 9.0);
        let source_label =
            source_media.map(|m| format!("{}x{} • {:.0}fps", m.width, m.height, m.fps));

        let (_s_resp, s_act) = MonitorWidget::new(
            MonitorMode::Source,
            source_title,
            self.source_state.texture.as_ref(),
            source_media.is_some(),
            source_aspect,
            self.source_state.current_time,
            source_dur,
            self.source_state.mark_in,
            self.source_state.mark_out,
            &source_tc,
            &source_dur_tc,
            self.source_state.is_playing,
            1.0,
            source_label.as_deref(),
            false,
        )
        .show(source_ui);

        if let Some(act) = s_act {
            match act {
                MonitorAction::ImportMedia => self.prompt_import_media(),
                MonitorAction::TogglePlay => {
                    self.source_state.is_playing = !self.source_state.is_playing
                }
                MonitorAction::StepFrame(d) => {
                    self.source_state.is_playing = false;
                    let new_t = (self.source_state.current_time + (d as f64 / source_fps.max(1.0)))
                        .clamp(0.0, source_dur);
                    self.source_state.current_time = new_t;
                }
                MonitorAction::MarkIn => {
                    self.source_state.mark_in = Some(self.source_state.current_time);
                    self.status_message =
                        format!("Kaynak Giriş: {:.2}s", self.source_state.current_time);
                }
                MonitorAction::MarkOut => {
                    self.source_state.mark_out = Some(self.source_state.current_time);
                    self.status_message =
                        format!("Kaynak Çıkış: {:.2}s", self.source_state.current_time);
                }
                MonitorAction::GoToIn => {
                    self.source_state.current_time = self.source_state.mark_in.unwrap_or(0.0);
                }
                MonitorAction::GoToOut => {
                    self.source_state.current_time =
                        self.source_state.mark_out.unwrap_or(source_dur);
                }
                MonitorAction::InsertToTimeline => self.insert_source_to_timeline(false),
                MonitorAction::OverwriteToTimeline => self.insert_source_to_timeline(true),
                MonitorAction::Seek(t) => {
                    self.source_state.is_playing = false;
                    self.source_state.current_time = t.clamp(0.0, source_dur);
                }
                _ => {}
            }
        }
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
        let prog_tc = seconds_to_timecode(self.current_time, prog_fps);
        let prog_dur_tc = seconds_to_timecode(prog_dur, prog_fps);
        let prog_aspect =
            (self.project.timeline.width as f32 / self.project.timeline.height as f32).max(0.1);

        let top_clip = self
            .project
            .timeline
            .get_top_visible_video_clip_at(self.current_time);
        let prog_label = top_clip
            .and_then(|(_, c)| self.project.get_media(&c.media_id))
            .map(|m| format!("{}x{} • {:.0}fps", m.width, m.height, m.fps));

        let mut active_text_overlays = Vec::new();
        for track in self.project.timeline.get_video_tracks().iter().rev() {
            if !track.visible {
                continue;
            }
            if let Some(c) = track.find_clip_at(self.current_time) {
                if let Some(txt) = &c.text_overlay {
                    let dt = self.current_time - c.timeline_in;
                    let rem = c.timeline_out - self.current_time;
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
                    active_text_overlays.push(ActiveTextOverlay {
                        text: txt.content.clone(),
                        font_size: txt.font_size,
                        text_color,
                        bg_color,
                        pos_x: txt.pos_x,
                        pos_y: txt.pos_y,
                        has_shadow: txt.has_shadow,
                        alpha: a.clamp(0.0, 1.0),
                        caption_style: txt.caption_style,
                        highlight_color,
                        progress,
                        motion_style: txt.motion_style,
                        subtitle_text: txt.subtitle_text.clone(),
                    });
                }
            }
        }

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

        let (_p_resp, p_act) = MonitorWidget::new(
            MonitorMode::Program,
            "Sekans 01",
            self.monitor_texture.as_ref(),
            has_clips,
            prog_aspect,
            self.current_time,
            prog_dur,
            None,
            None,
            &prog_tc,
            &prog_dur_tc,
            self.is_playing,
            self.playback_speed,
            prog_label.as_deref(),
            self.video_engine.is_decoding_seek,
        )
        .custom_texture_id(self.monitor_gpu_texture_id)
        .safe_zones(self.show_safe_zones)
        .aspect_ratio_preset(Some(self.project.timeline.aspect_ratio()))
        .text_overlays(active_text_overlays)
        .active_tool(self.current_edit_tool)
        .inline_text_edit(selected_text_content)
        .show(prog_ui);

        if let Some(act) = p_act {
            match act {
                MonitorAction::NewProject => {
                    if self.has_unsaved_changes() {
                        let name = self.project.settings.name.clone();
                        self.close_confirm_dialog
                            .prompt(PendingCloseAction::NewProject, name);
                    } else {
                        self.reset_to_new_project();
                    }
                }
                MonitorAction::OpenProject => {
                    if self.has_unsaved_changes() {
                        let name = self.project.settings.name.clone();
                        self.close_confirm_dialog
                            .prompt(PendingCloseAction::OpenProject, name);
                    } else {
                        self.open_project_dialog();
                    }
                }
                MonitorAction::ImportMedia => self.prompt_import_media(),
                MonitorAction::TogglePlay => self.toggle_play(),
                MonitorAction::StepFrame(d) => self.step_frame(d),
                MonitorAction::MarkIn => {
                    self.project.timeline.set_in_point(self.current_time);
                    self.project.mark_dirty();
                    self.status_message = format!("Giriş Noktası (In): {:.2}s", self.current_time);
                }
                MonitorAction::MarkOut => {
                    self.project.timeline.set_out_point(self.current_time);
                    self.project.mark_dirty();
                    self.status_message = format!("Çıkış Noktası (Out): {:.2}s", self.current_time);
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
                    self.show_safe_zones = !self.show_safe_zones;
                }
                MonitorAction::SetAspectRatio(r) => {
                    self.project.timeline.set_aspect_ratio(r);
                    self.project.sync_settings_from_timeline();
                    self.project.mark_dirty();
                    self.status_message = format!("Format Değiştirildi: {}", r.label());
                }
                MonitorAction::TextToolClick { pos_x, pos_y } => {
                    self.handle_monitor_text_click(pos_x, pos_y);
                }
                MonitorAction::TextPositionDrag { pos_x, pos_y } => {
                    self.handle_monitor_text_drag(pos_x, pos_y);
                }
                MonitorAction::TextContentEdited(content) => {
                    self.handle_monitor_text_content_changed(content);
                }
                _ => {}
            }
        }
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
        ctx.input(|i| {
            for file in &i.raw.dropped_files {
                if let Some(path) = &file.path {
                    self.import_media_file(path);
                }
            }
        });

        // Autosave check
        if self.last_autosave.elapsed().as_secs() >= 60 && self.project.is_dirty {
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

        if self.active_voiceover.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        // -------------------------------------------------------------
        // Global Keyboard Shortcuts
        // -------------------------------------------------------------
        let mut do_copy = false;
        let mut do_cut = false;
        let mut do_paste = false;

        if !ctx.wants_keyboard_input() && !self.welcome_dialog.is_open {
            ctx.input(|i| {
                let is_copy = i.events.iter().any(|e| matches!(e, egui::Event::Copy))
                    || ((i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(Key::C));
                let is_paste = i.events.iter().any(|e| matches!(e, egui::Event::Paste(_)))
                    || ((i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(Key::V));
                let is_cut = i.events.iter().any(|e| matches!(e, egui::Event::Cut))
                    || ((i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(Key::X));

                if is_copy {
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
                    } else {
                        self.current_edit_tool = EditTool::TrackSelect;
                    }
                    self.canvas_state.active_tool = self.current_edit_tool;
                    self.status_message = "İz Seçme Aracı (A) etkin".to_string();
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
                        self.close_selected_gap(is_ripple);
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
                    let _ = self.undo_stack.redo(&mut self.project.timeline);
                    self.canvas_state
                        .selected_clip_pairs
                        .retain(|(tr_id, cl_id)| {
                            self.project
                                .timeline
                                .get_track_by_id(tr_id)
                                .and_then(|t| t.get_clip_by_id(cl_id))
                                .is_some()
                        });
                    self.project.mark_dirty();
                } else if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(Key::Z) {
                    let _ = self.undo_stack.undo(&mut self.project.timeline);
                    self.canvas_state
                        .selected_clip_pairs
                        .retain(|(tr_id, cl_id)| {
                            self.project
                                .timeline
                                .get_track_by_id(tr_id)
                                .and_then(|t| t.get_clip_by_id(cl_id))
                                .is_some()
                        });
                    self.project.mark_dirty();
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
                } else if i.modifiers.ctrl && (i.key_pressed(Key::E) || i.key_pressed(Key::M)) {
                    self.export_dialog.open(&self.project);
                } else if i.modifiers.ctrl && i.key_pressed(Key::T) {
                    self.title_dialog.open();
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
        }

        // -------------------------------------------------------------
        // Top Header: Premiere Pro Navigation Bar & Window Controls
        // -------------------------------------------------------------
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

                // Dosya
                ui.menu_button("Dosya", |ui| {
                    if ui.button("Başlangıç Ekranı...").clicked() {
                        self.welcome_dialog.open();
                        ui.close_menu();
                    }
                    if ui.button("Yeni Boş Proje (Ctrl+N)").clicked() {
                        if self.has_unsaved_changes() {
                            let name = self.project.settings.name.clone();
                            self.close_confirm_dialog
                                .prompt(PendingCloseAction::NewProject, name);
                        } else {
                            self.reset_to_new_project();
                        }
                        ui.close_menu();
                    }
                    if ui.button("Proje Aç... (Ctrl+O)").clicked() {
                        if self.has_unsaved_changes() {
                            let name = self.project.settings.name.clone();
                            self.close_confirm_dialog
                                .prompt(PendingCloseAction::OpenProject, name);
                        } else {
                            self.open_project_dialog();
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Kaydet (Ctrl+S)").clicked() {
                        self.save_project_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Farklı Kaydet...").clicked() {
                        self.save_project_as_dialog();
                        ui.close_menu();
                    }
                    if ui
                        .button("Projeyi Paketle / Arşivle... (Taşınabilir)")
                        .clicked()
                    {
                        self.bundle_project_dialog();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Medya İçe Aktar... (Ctrl+I)").clicked() {
                        self.prompt_import_media();
                        ui.close_menu();
                    }
                    if ui
                        .button("Çevrimdışı Medyaları Yeniden Bağla (Relink)...")
                        .clicked()
                    {
                        self.relink_dialog.open(&self.project);
                        ui.close_menu();
                    }
                    if ui.button("Projeyi İçe Aktar (FCPXML / EDL)...").clicked() {
                        self.import_interchange_dialog();
                        ui.close_menu();
                    }
                    if ui
                        .button("Altyazı Dosyası İçe Aktar (.srt, .vtt)...")
                        .clicked()
                    {
                        self.import_subtitles_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Altyazıları Dışa Aktar (.srt)...").clicked() {
                        self.export_subtitles_dialog();
                        ui.close_menu();
                    }
                    if ui
                        .button("Projeyi EDL Olarak Dışa Aktar (.edl)...")
                        .clicked()
                    {
                        self.export_edl_dialog();
                        ui.close_menu();
                    }
                    if ui
                        .button("Projeyi FCPXML Olarak Dışa Aktar (Premiere/DaVinci)...")
                        .clicked()
                    {
                        self.export_fcpxml_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Dışa Aktarma Stüdyosu... (Ctrl+M)").clicked() {
                        self.export_dialog.open(&self.project);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Çıkış (Ctrl+Q)").clicked() {
                        if self.has_unsaved_changes() && !self.confirmed_exit {
                            let name = self.project.settings.name.clone();
                            self.close_confirm_dialog
                                .prompt(PendingCloseAction::ExitApp, name);
                        } else {
                            self.confirmed_exit = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        ui.close_menu();
                    }
                });

                // Düzen
                ui.menu_button("Düzen", |ui| {
                    let can_undo = self.undo_stack.can_undo();
                    if ui
                        .add_enabled(can_undo, Button::new("Geri Al (Ctrl+Z)"))
                        .clicked()
                    {
                        let _ = self.undo_stack.undo(&mut self.project.timeline);
                        self.canvas_state
                            .selected_clip_pairs
                            .retain(|(tr_id, cl_id)| {
                                self.project
                                    .timeline
                                    .get_track_by_id(tr_id)
                                    .and_then(|t| t.get_clip_by_id(cl_id))
                                    .is_some()
                            });
                        self.project.mark_dirty();
                        ui.close_menu();
                    }
                    let can_redo = self.undo_stack.can_redo();
                    if ui
                        .add_enabled(can_redo, Button::new("Yinele (Ctrl+Y)"))
                        .clicked()
                    {
                        let _ = self.undo_stack.redo(&mut self.project.timeline);
                        self.canvas_state
                            .selected_clip_pairs
                            .retain(|(tr_id, cl_id)| {
                                self.project
                                    .timeline
                                    .get_track_by_id(tr_id)
                                    .and_then(|t| t.get_clip_by_id(cl_id))
                                    .is_some()
                            });
                        self.project.mark_dirty();
                        ui.close_menu();
                    }
                    ui.separator();
                    let has_sel = !self.canvas_state.selected_clip_pairs.is_empty();
                    if ui.add_enabled(has_sel, Button::new("Kes (Ctrl+X)")).clicked() {
                        self.copy_selected_clips();
                        self.delete_selected_clips();
                        ctx.output_mut(|o| {
                            o.copied_text = format!("trim://clips/{}", self.clipboard_clips.len());
                        });
                        ui.close_menu();
                    }
                    if ui.add_enabled(has_sel, Button::new("Kopyala (Ctrl+C)")).clicked() {
                        self.copy_selected_clips();
                        ctx.output_mut(|o| {
                            o.copied_text = format!("trim://clips/{}", self.clipboard_clips.len());
                        });
                        ui.close_menu();
                    }
                    let can_paste = !self.clipboard_clips.is_empty();
                    if ui.add_enabled(can_paste, Button::new("Yapıştır (Ctrl+V)")).clicked() {
                        self.paste_clipboard_clips();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Seçilenleri Sil (Delete)").clicked() {
                        self.delete_selected_clips();
                        ui.close_menu();
                    }
                    if ui.button("Dalgalı Sil (Shift+Delete)").clicked() {
                        self.ripple_delete_selected_clips();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Tüm Klipleri Seç (Ctrl+A)").clicked() {
                        self.select_all_clips();
                        ui.close_menu();
                    }
                    if ui.button("Seçimi Kaldır (Ctrl+D)").clicked() {
                        self.canvas_state.selected_clip_pairs.clear();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Klipleri Bağla / Ayır (Ctrl+L)").clicked() {
                        self.toggle_link_selected_clips();
                        ui.close_menu();
                    }
                    if ui
                        .button("Sessizlikleri Otomatik Kes (Jump-Cut)...")
                        .clicked()
                    {
                        self.open_silence_cut_dialog_for_selected();
                        ui.close_menu();
                    }
                    if ui
                        .button("🎙️ Lokal Whisper ile Otomatik Deşifre Et...")
                        .clicked()
                    {
                        self.open_transcribe_dialog_for_selected();
                        ui.close_menu();
                    }
                    if ui.button("📹 Ekran ve Web Kamerası Kaydet...").clicked() {
                        self.open_screen_record_dialog();
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.menu_button("Klavye Kısayol Profili", |ui| {
                        for p in ShortcutProfile::ALL {
                            if ui
                                .selectable_label(self.shortcut_profile == *p, p.label())
                                .clicked()
                            {
                                self.shortcut_profile = *p;
                                self.status_message = format!("Kısayol Profili: {}", p.label());
                                ui.close_menu();
                            }
                        }
                    });
                });

                // Sekans
                ui.menu_button("Sekans", |ui| {
                    if ui.button("Oynatma Çizgisinde Kes (S / Ctrl+K)").clicked() {
                        self.split_at_playhead_command();
                        ui.close_menu();
                    }
                    if ui
                        .button("🎬 Seçili Klipte Sahneleri Algıla ve Böl (Smart Cut)")
                        .clicked()
                    {
                        self.smart_scene_cut_selected_clip();
                        ui.close_menu();
                    }
                    if ui.button("İşaretçi (Marker) Ekle / Kaldır (M)").clicked() {
                        self.add_marker_at_playhead();
                        ui.close_menu();
                    }
                    if ui
                        .button(format!(
                            "Manyetik Yapışma (N): {}",
                            if self.snap_enabled {
                                "Açık"
                            } else {
                                "Kapalı"
                            }
                        ))
                        .clicked()
                    {
                        self.toggle_snap();
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.menu_button("En-Boy Oranı / Format", |ui| {
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
                                self.status_message = format!("Format Değiştirildi: {}", r.label());
                                ui.close_menu();
                            }
                        }
                    });
                    if ui
                        .checkbox(&mut self.show_safe_zones, "Güvenli Alan Rehber Çizgileri")
                        .clicked()
                    {
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .checkbox(
                            &mut self.use_proxies,
                            "⚡ Akıllı Proxy Önizleme (720p H.264)",
                        )
                        .clicked()
                    {
                        self.video_engine.use_proxies = self.use_proxies;
                        self.video_engine.clear_cache();
                        self.last_rendered_dirty = true;
                        let state_str = if self.use_proxies {
                            "Açık"
                        } else {
                            "Kapalı"
                        };
                        self.status_message = format!("⚡ Proxy Önizleme: {}", state_str);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Başa Git (Home)").clicked() {
                        self.seek(0.0);
                        ui.close_menu();
                    }
                    if ui.button("Sona Git (End)").clicked() {
                        let dur = self.project.timeline.duration();
                        self.seek(dur);
                        ui.close_menu();
                    }
                });

                // Kanallar
                ui.menu_button("Kanallar", |ui| {
                    if ui.button("Video Kanalı Ekle (+ Video)").clicked() {
                        self.add_video_track();
                        ui.close_menu();
                    }
                    if ui.button("Ses Kanalı Ekle (+ Ses)").clicked() {
                        self.add_audio_track();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .button("Tüm Kanal Yüksekliklerini Sıfırla (56px)")
                        .clicked()
                    {
                        self.canvas_state.track_heights.clear();
                        ui.close_menu();
                    }
                });

                // Pencere
                ui.menu_button("Pencere", |ui| {
                    if ui
                        .checkbox(
                            &mut self.show_source_monitor,
                            "Kaynak Monitörü (İkili Görünüm)",
                        )
                        .clicked()
                    {
                        ui.close_menu();
                    }
                    if ui.button("Çok Kanallı Ses Mikseri").clicked() {
                        self.audio_mixer_dialog.toggle();
                        ui.close_menu();
                    }
                    if ui.button("Metin / Başlık Editörü").clicked() {
                        self.title_dialog.open();
                        ui.close_menu();
                    }
                    if ui.button("Dışa Aktarma Stüdyosu").clicked() {
                        self.export_dialog.open(&self.project);
                        ui.close_menu();
                    }
                });

                // Yardım
                ui.menu_button("Yardım", |ui| {
                    if ui.button("Klavye Kısayolları (F1)").clicked() {
                        self.shortcuts_dialog.open();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("triM. Hakkında").clicked() {
                        self.show_about_dialog = true;
                        ui.close_menu();
                    }
                });

                // Right Side Controls: Quick Export + Mixer + Monitor view + Title
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let quick_exp_btn = Button::new(
                        RichText::new("Dışa Aktar")
                            .color(Color32::WHITE)
                            .size(11.5)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(0x28, 0x5a, 0x9e));
                    if ui
                        .add(quick_exp_btn)
                        .on_hover_text("Dışa Aktarma Stüdyosu (Ctrl+M)")
                        .clicked()
                    {
                        self.export_dialog.open(&self.project);
                    }

                    let mixer_btn = Button::new(RichText::new("Mikser").size(11.0)).fill(
                        if self.audio_mixer_dialog.is_open {
                            Color32::from_rgb(0x2a, 0x54, 0x3d)
                        } else {
                            Color32::from_rgb(0x22, 0x22, 0x2c)
                        },
                    );
                    if ui
                        .add(mixer_btn)
                        .on_hover_text("Çok Kanallı Ses Mikserini Aç/Kapat")
                        .clicked()
                    {
                        self.audio_mixer_dialog.toggle();
                    }

                    let mon_label = if self.show_source_monitor {
                        "Tek Monitör"
                    } else {
                        "İkili Monitör"
                    };
                    let mon_btn = Button::new(RichText::new(mon_label).size(11.0)).fill(
                        if self.show_source_monitor {
                            Color32::from_rgb(0x2a, 0x3d, 0x54)
                        } else {
                            Color32::from_rgb(0x22, 0x22, 0x2c)
                        },
                    );
                    if ui
                        .add(mon_btn)
                        .on_hover_text(
                            "Monitör Görünümünü Değiştir (İkili Monitör / Tek Sekans Monitörü)",
                        )
                        .clicked()
                    {
                        self.show_source_monitor = !self.show_source_monitor;
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
                    ui.spacing_mut().item_spacing.x = 6.0;

                    let seq_tab_btn = Button::new(
                        RichText::new("Sekans 01")
                            .size(12.0)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(0x24, 0x24, 0x2e))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x38, 0x7a, 0xe0)));
                    let _ = ui.add(seq_tab_btn);

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
                            "⚡ Proxy: Açık"
                        } else {
                            "⚡ Proxy: Kapalı"
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
                        RichText::new("📹 Ekran Kaydet")
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
                            RichText::new(format!("🔴 SES KAYDI: {:.1}s [⏹ Durdur]", vo_dur))
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
                                15.0..=300.0,
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

                    let current_scroll_x = self.canvas_state.scroll_offset.x;
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
                                    RichText::new("🎛 KANALLAR")
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
                                                if menu_ui.button("📹 Video Kanalı Ekle").clicked()
                                                {
                                                    add_video_requested = true;
                                                    menu_ui.close_menu();
                                                }
                                                if menu_ui.button("🎵 Ses Kanalı Ekle").clicked()
                                                {
                                                    add_audio_requested = true;
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
                    if reset_heights_requested {
                        self.canvas_state.track_heights.clear();
                    }

                    // 3. Scrollable Timeline Canvas + Time Ruler
                    let vu_w = 58.0;
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
                                    );

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
                                        } => {
                                            let cmd = Box::new(TrimClipCommand::new(
                                                track_id, clip_id, old_bounds, new_bounds,
                                            ));
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
                                        } => {
                                            let cmd = Box::new(SplitClipCommand::new(
                                                track_id, clip_id, cut_time,
                                            ));
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
                                            if let Some(media) = self.project.get_media(&media_id) {
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

                                                let link_group = if has_audio && is_video_track {
                                                    Some(uuid::Uuid::new_v4().to_string())
                                                } else {
                                                    None
                                                };

                                                let mut clip =
                                                    Clip::new(&media_id, time, time + clip_dur);
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
                                                    let mut audio_clip =
                                                        Clip::new(&media_id, time, time + clip_dur);
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
                                                    let _ = self.undo_stack.push(
                                                        macro_cmd,
                                                        &mut self.project.timeline,
                                                    );
                                                } else {
                                                    let _ = self
                                                        .undo_stack
                                                        .push(cmd, &mut self.project.timeline);
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
                                    }
                                }
                            });
                        });

                    // 4. Master VU Meter (Far Right)
                    ui.allocate_ui(Vec2::new(vu_w, ui.available_height()), |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(3.0);
                            ui.add(VuMeterWidget::new(&mut self.vu_state));
                        });
                    });
                });
            });

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

                    let proj_btn = Button::new(RichText::new("Proje").size(11.5).strong().color(
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

                    let eff_btn = Button::new(RichText::new("Efektler").size(11.5).strong().color(
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
                            }
                        }
                    }
                    LeftPanelTab::Effects => {
                        let selected_clip_id = self
                            .canvas_state
                            .selected_clip_pairs
                            .first()
                            .map(|(_, c)| c.clone());
                        if let Some(act) = render_inspector_panel(
                            ui,
                            &mut self.project,
                            selected_clip_id.as_deref(),
                            self.current_time,
                        ) {
                            match act {
                                InspectorAction::ClipModified => {
                                    self.project.mark_dirty();
                                    self.last_rendered_monitor_frame = None;
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
                            }
                        }
                    }
                }
            });

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

        // -------------------------------------------------------------
        // Modals & Dialogs
        // -------------------------------------------------------------
        if let Some(act) = self.export_dialog.show(ctx) {
            match act {
                ExportDialogAction::StartExport(preset, out_path, range) => {
                    let (tx, rx) = channel();
                    self.export_rx = Some(rx);
                    self.export_runner
                        .start_export(self.project.clone(), preset, out_path, range, tx);
                }
                ExportDialogAction::CancelExport => {
                    self.export_runner.cancel();
                    self.export_dialog.is_exporting = false;
                    self.export_rx = None;
                    self.status_message = "Dışa aktarma iptal edildi".to_string();
                }
                ExportDialogAction::OpenOutputFolder(path) => {
                    let p = std::path::Path::new(&path);
                    let folder = if p.is_file() {
                        p.parent().unwrap_or(p)
                    } else {
                        p
                    };
                    #[cfg(target_os = "windows")]
                    let _ = std::process::Command::new("explorer").arg(folder).spawn();
                    #[cfg(target_os = "macos")]
                    let _ = std::process::Command::new("open").arg(folder).spawn();
                    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
                    let _ = std::process::Command::new("xdg-open").arg(folder).spawn();
                }
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
                                        ..Default::default()
                                    });
                                    if motion_style
                                        == trim_core::clip::MotionTitleStyle::CinematicZoom
                                        || motion_style
                                            == trim_core::clip::MotionTitleStyle::LowerThird
                                    {
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
                        self.resolve_default_target_tracks();
                        for item in self.project.media_pool.values() {
                            if item.media_type == MediaType::Audio || item.audio_channels > 0 {
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
        // Welcome Screen / Project Launcher Modal
        // (Do not overlap when Crash Recovery modal is active)
        // -------------------------------------------------------------
        let welcome_act = if self.recovery_dialog.is_open {
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
                        let temp_wav = std::env::temp_dir().join(format!(
                            "trim_whisper_{}.wav",
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0)
                        ));
                        let media_p = PathBuf::from(&media_path);
                        match trim_media::extract_audio_for_whisper(&media_p, &temp_wav) {
                            Ok(()) => {
                                match trim_media::run_whisper_transcription(
                                    &whisper_bin,
                                    &temp_wav,
                                    &model_name,
                                    Some(&language),
                                ) {
                                    Ok(entries) => {
                                        let count = entries.len();
                                        let subtitle_clips =
                                            trim_core::subtitles::subtitle_entries_to_clips(
                                                &entries, 42.0,
                                            );
                                        let target_tr = self
                                            .project
                                            .timeline
                                            .get_video_tracks()
                                            .first()
                                            .map(|t| t.id.clone())
                                            .unwrap_or_else(|| {
                                                self.project.timeline.add_new_video_track()
                                            });
                                        for c in subtitle_clips {
                                            let cmd = Box::new(AddClipCommand::new(&target_tr, c));
                                            let _ = self
                                                .undo_stack
                                                .push(cmd, &mut self.project.timeline);
                                        }
                                        self.transcribe_dialog.close();
                                        self.status_message = format!(
                                            "{} adet altyazı başarıyla oluşturuldu ve eklendi",
                                            count
                                        );
                                        self.project.mark_dirty();
                                    }
                                    Err(e) => {
                                        self.transcribe_dialog.is_transcribing = false;
                                        self.transcribe_dialog.status_message =
                                            format!("Deşifre hatası: {}", e);
                                    }
                                }
                                let _ = std::fs::remove_file(temp_wav);
                            }
                            Err(e) => {
                                self.transcribe_dialog.is_transcribing = false;
                                self.transcribe_dialog.status_message =
                                    format!("Ses ayıklama hatası: {}", e);
                            }
                        }
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

            let gpu_status = if self.gpu_compositor.is_some() {
                "OpenGL 3.3 FBO Shader (Aktif)"
            } else {
                "CPU Yazılım Birleştirici (Fallback)"
            };

            egui::Window::new("triM. Hakkında")
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .order(egui::Order::Foreground)
                .fixed_size(Vec2::new(450.0, 340.0))
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .frame(about_frame)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        // Top Branding Header
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("triM.")
                                    .size(30.0)
                                    .strong()
                                    .color(Color32::from_rgb(0xe0, 0x7a, 0x38)),
                            );
                            ui.add_space(6.0);
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
                                        .size(11.0)
                                        .strong()
                                        .color(Color32::from_rgb(0xf2, 0x9b, 0x5e)),
                                    );
                                });
                            ui.add_space(4.0);
                            egui::Frame::none()
                                .fill(Color32::from_rgb(0x1b, 0x24, 0x34))
                                .rounding(egui::Rounding::same(6.0))
                                .stroke(Stroke::new(
                                    1.0_f32,
                                    Color32::from_rgb(0x32, 0x4e, 0x78),
                                ))
                                .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new("RUST NATIVE NLE")
                                            .size(10.0)
                                            .strong()
                                            .color(Color32::from_rgb(0x68, 0xb0, 0xf8)),
                                    );
                                });
                        });

                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(
                                "Düşük gecikmeli, bağımsız ve modern masaüstü video kurgu stüdyosu.",
                            )
                            .size(12.0)
                            .color(Color32::from_rgb(0x9a, 0x9e, 0xb2)),
                        );

                        ui.add_space(12.0);

                        // Technical Specs Box
                        egui::Frame::none()
                            .fill(Color32::from_rgb(0x11, 0x12, 0x17))
                            .rounding(egui::Rounding::same(8.0))
                            .stroke(Stroke::new(
                                1.0_f32,
                                Color32::from_rgb(0x25, 0x28, 0x34),
                            ))
                            .inner_margin(egui::Margin::symmetric(14.0, 12.0))
                            .show(ui, |ui| {
                                ui.set_min_width(400.0);
                                egui::Grid::new("about_specs_grid")
                                    .num_columns(2)
                                    .spacing([16.0, 7.0])
                                    .show(ui, |ui| {
                                        let rows = [
                                            ("Görüntü Hattı:", gpu_status),
                                            (
                                                "Ses DSP Motoru:",
                                                "48 kHz Stereo · 5 ms Lookahead Limiter",
                                            ),
                                            (
                                                "Medya Çekirdeği:",
                                                "FFmpeg / FFprobe · 720p H.264 Proxy",
                                            ),
                                            ("Derleme & Dil:", "Rust 2021 · egui / glow"),
                                            ("Lisans:", "GPL-3.0-or-later (Açık Kaynak)"),
                                            ("Geliştirici:", "MOzcelik14"),
                                        ];
                                        for (k, v) in rows {
                                            ui.label(
                                                RichText::new(k)
                                                    .size(11.0)
                                                    .strong()
                                                    .color(Color32::from_rgb(0x82, 0x86, 0x9a)),
                                            );
                                            ui.label(
                                                RichText::new(v)
                                                    .size(11.0)
                                                    .monospace()
                                                    .color(Color32::from_rgb(0xdc, 0xe0, 0xee)),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });

                        ui.add_space(14.0);

                        // Bottom Action Buttons
                        ui.horizontal(|ui| {
                            let gh_btn = Button::new(
                                RichText::new("🌐 GitHub Deposu")
                                    .size(11.5)
                                    .strong()
                                    .color(Color32::from_rgb(0xd8, 0xdc, 0xec)),
                            )
                            .min_size(Vec2::new(140.0, 32.0))
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

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let close_btn = Button::new(
                                        RichText::new("Kapat")
                                            .size(12.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .min_size(Vec2::new(100.0, 32.0))
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
                BinDialogMode::Create => ("📁 Yeni Medya Klasörü (Bin)", "Oluştur"),
                BinDialogMode::Rename(_) => ("📁 Klasörü Yeniden Adlandır", "Kaydet"),
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
    }
}
