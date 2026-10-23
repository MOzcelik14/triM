//! Main triM. desktop application implementing `eframe::App` with Premiere Pro 4-Quadrant layout.

use crate::canvas::{render_timeline_canvas, CanvasAction, TimelineCanvasState};
use crate::dialogs::*;
use crate::inspector::{render_inspector_panel, InspectorAction};
use crate::media_bin::{render_media_bin, LeftPanelTab, MediaBinAction};
use crate::monitor::{ActiveTextOverlay, MonitorAction, MonitorMode, MonitorWidget};
use crate::ruler::{render_time_ruler, RulerAction};
use crate::shortcuts::ShortcutProfile;
use crate::theme::*;
use crate::toolbar::{render_vertical_toolbar, EditTool};
use crate::track_header::{render_track_header, TrackHeaderAction};
use crate::vu_meter::{VuMeterState, VuMeterWidget};

use egui::{
    Button, Color32, ColorImage, Key, RichText, ScrollArea, Stroke, TextureHandle,
    TextureOptions, Vec2,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Instant;

use trim_commands::commands::*;
use trim_commands::UndoStack;
use trim_core::clip::{Clip, TextOverlay};
use trim_core::marker::Marker;
use trim_core::media::MediaType;
use trim_core::project::Project;
use trim_core::time::seconds_to_timecode;
use trim_export::runner::{ExportMessage, ExportRunner};
use trim_media::{
    decode_frame_raw, generate_title_image, AudioMixer, AudioStreamPlayer, FFprobeAnalyzer,
    ThumbnailGenerator, VideoEngine, VoiceoverRecorder, WaveformGenerator,
};
use trim_project::{bundle_project, load_project, save_project, AutosaveManager};
use trim_timeline::{
    find_next_edit_point, find_prev_edit_point, ripple_trim_head, ripple_trim_tail,
    split_at_playhead,
};

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
    pub media_bin_textures: HashMap<String, TextureHandle>,

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
    pub autosave_mgr: AutosaveManager,

    pub export_runner: ExportRunner,
    pub export_rx: Option<Receiver<ExportMessage>>,

    pub monitor_texture: Option<TextureHandle>,
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
}

impl TrimApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_theme(&cc.egui_ctx);

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

        let mut app = Self {
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
            media_bin_textures: HashMap::new(),
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
            autosave_mgr: AutosaveManager::default(),
            export_runner: ExportRunner::default(),
            export_rx: None,
            monitor_texture: None,
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
        };

        // Check crash recovery
        if let Some(rec_path) = app.autosave_mgr.check_recovery(&app.project) {
            app.recovery_dialog.prompt(rec_path);
        }

        app
    }

    pub fn toggle_play(&mut self) {
        if self.is_playing {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn play(&mut self) {
        if self.current_time >= self.project.timeline.duration() && self.project.timeline.duration() > 0.0 {
            self.current_time = 0.0;
        }
        self.is_playing = true;
        self.audio_player.prepare();
        self.clock_start = Some(Instant::now());
        self.clock_start_time = self.current_time;
    }

    pub fn pause(&mut self) {
        if self.active_voiceover.is_some() {
            self.finish_voiceover();
        }
        self.is_playing = false;
        self.playback_speed = 1.0;
        self.clock_start = None;
        self.video_engine.streamer.stop();
        self.audio_player.stop();
        self.vu_state.reset();
    }

    pub fn toggle_proxy(&mut self) {
        self.use_proxies = !self.use_proxies;
        self.video_engine.use_proxies = self.use_proxies;
        self.video_engine.clear_cache();
        self.last_rendered_dirty = true;
        let state_str = if self.use_proxies { "AÇIK (720p H.264)" } else { "KAPALI (Orijinal 4K/UHD)" };
        self.status_message = format!("⚡ Akıllı Proxy Önizleme: {}", state_str);
    }

    pub fn generate_proxy_for_media(&mut self, media_id: &str) {
        let media = match self.project.get_media(media_id) {
            Some(m) => m,
            None => return,
        };
        if media.media_type != MediaType::Video {
            self.status_message = "Proxy yalnızca video medyaları için oluşturulabilir.".to_string();
            return;
        }

        let src = std::path::PathBuf::from(&media.file_path);
        let out = trim_media::default_proxy_path_for(&src);

        if out.exists() {
            if let Some(m) = self.project.get_media_mut(media_id) {
                m.proxy_path = Some(out.to_string_lossy().to_string());
            }
            self.status_message = format!("⚡ Proxy zaten mevcut: {}", out.file_name().unwrap().to_string_lossy());
            return;
        }

        let m_id = media_id.to_string();
        self.proxy_generating_ids.push(m_id.clone());
        let (tx, rx) = channel();
        self.proxy_rx = Some(rx);

        std::thread::spawn(move || {
            let res = trim_media::generate_proxy(&src, &out);
            let payload = res.map(|_| out.to_string_lossy().to_string());
            let _ = tx.send((m_id, payload));
        });

        self.status_message = "⚡ 720p Proxy arka planda oluşturuluyor...".to_string();
    }

    pub fn start_voiceover(&mut self, track_id: String) {
        if let Some(t) = self.project.timeline.get_track_by_id(&track_id) {
            if t.locked {
                self.status_message = "Kilitli kanala ses kaydı yapılamaz.".to_string();
                return;
            }
        }

        let vo_dir = trim_core::paths::cache_dir().join("voiceovers");
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let wav_path = vo_dir.join(format!("voiceover_{}.wav", timestamp));

        match trim_media::VoiceoverRecorder::start(&wav_path) {
            Ok(rec) => {
                let start_time = self.current_time;
                self.active_voiceover = Some(ActiveVoiceover {
                    track_id,
                    start_timeline_time: start_time,
                    recorder: rec,
                });
                self.play();
                self.status_message = "🔴 Canlı seslendirme kaydediliyor... Durdurmak için [⏹] veya Boşluk tuşuna basın.".to_string();
            }
            Err(e) => {
                self.status_message = format!("Ses kaydı başlatılamadı: {}", e);
            }
        }
    }

    pub fn finish_voiceover(&mut self) {
        let mut vo = match self.active_voiceover.take() {
            Some(v) => v,
            None => return,
        };

        match vo.recorder.stop() {
            Ok(dur) => {
                let p_str = vo.recorder.output_path().to_string_lossy().to_string();
                let mut media_item = trim_core::media::MediaItem::new(&p_str);
                media_item.media_type = MediaType::Audio;
                media_item.duration = dur;
                media_item.name = format!("Ses Kaydı ({:.1}s)", dur);
                let media_id = media_item.id.clone();
                self.project.add_media(media_item);

                let mut clip = Clip::new(&media_id, vo.start_timeline_time, vo.start_timeline_time + dur);
                clip.name = "Seslendirme Kaydı".to_string();

                let cmd = Box::new(AddClipCommand::new(vo.track_id.clone(), clip));
                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                self.status_message = format!("🎤 Seslendirme başarıyla kaydedildi ({:.1}s) ve kanala eklendi!", dur);
            }
            Err(e) => {
                self.status_message = format!("Ses kaydı kaydedilemedi: {}", e);
            }
        }
    }

    pub fn stop(&mut self) {
        self.pause();
        self.seek(0.0);
    }

    pub fn seek(&mut self, time: f64) {
        self.video_engine.streamer.stop();
        self.audio_player.stop();
        let dur = self.project.timeline.duration();
        self.current_time = time.clamp(0.0, dur.max(0.0));
        if self.is_playing {
            self.clock_start = Some(Instant::now());
            self.clock_start_time = self.current_time;
        }
    }

    pub fn step_frame(&mut self, delta: i32) {
        self.pause();
        let fps = self.project.timeline.fps.max(1.0);
        let new_t = self.current_time + (delta as f64 / fps);
        self.seek(new_t);
    }

    pub fn shuttle_forward(&mut self) {
        let speeds = [1.0, 2.0, 4.0, 8.0];
        if !self.is_playing || self.playback_speed <= 0.0 {
            self.playback_speed = 1.0;
            self.play();
        } else {
            let next = speeds.iter().find(|&&s| s > self.playback_speed + 0.1).copied().unwrap_or(8.0);
            self.playback_speed = next;
            self.clock_start = Some(Instant::now());
            self.clock_start_time = self.current_time;
        }
    }

    pub fn shuttle_reverse(&mut self) {
        let speeds = [-1.0, -2.0, -4.0, -8.0];
        if !self.is_playing || self.playback_speed >= 0.0 {
            self.playback_speed = -1.0;
            self.play();
        } else {
            let next = speeds.iter().find(|&&s| s < self.playback_speed - 0.1).copied().unwrap_or(-8.0);
            self.playback_speed = next;
            self.clock_start = Some(Instant::now());
            self.clock_start_time = self.current_time;
        }
    }

    pub fn add_or_toggle_marker(&mut self) {
        if let Some(existing) = self.project.timeline.get_marker_at(self.current_time, 0.15).cloned() {
            let cmd = Box::new(RemoveMarkerCommand::new(existing.id));
            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
            self.project.mark_dirty();
            self.status_message = format!("İşaretçi silindi ({:.2}s)", self.current_time);
        } else {
            let marker = Marker::new(self.current_time);
            let cmd = Box::new(AddMarkerCommand::new(marker));
            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
            self.project.mark_dirty();
            self.status_message = format!("İşaretçi eklendi ({:.2}s)", self.current_time);
        }
    }

    pub fn import_media_file<P: AsRef<std::path::Path>>(&mut self, path: P) {
        match FFprobeAnalyzer::analyze(path) {
            Ok(item) => {
                let name = item.name.clone();
                let file_path = item.file_path.clone();
                let has_audio = item.media_type == MediaType::Audio || item.audio_channels > 0;
                let m_id = item.id.clone();
                self.project.add_media(item);
                if has_audio {
                    self.audio_mixer.preload_audio_async(&file_path);
                }
                // Automatically set as active in Source Monitor
                self.source_state.media_id = Some(m_id);
                self.source_state.current_time = 0.0;
                self.source_state.mark_in = None;
                self.source_state.mark_out = None;
                self.source_state.last_decoded_time = None;
                self.status_message = format!("Medya içe aktarıldı: {}", name);
            }
            Err(e) => {
                self.status_message = format!("Medya analizi başarısız: {}", e);
            }
        }
    }

    pub fn prompt_import_media(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Medya Dosyası Seç")
            .add_filter(
                "Tüm Medya Dosyaları",
                &["mp4", "mov", "mkv", "webm", "mp3", "wav", "aac", "png", "jpg", "jpeg"],
            )
            .pick_file()
        {
            self.import_media_file(path);
        }
    }

    pub fn open_project_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("triM. Projesi Aç")
            .add_filter("triM. Projeleri (*.trim, *.cutline)", &["trim", "cutline", "json"])
            .pick_file()
        {
            match load_project(&path) {
                Ok(proj) => {
                    self.project = proj;
                    self.undo_stack.clear();
                    self.canvas_state.selected_clip_pairs.clear();
                    self.current_time = 0.0;
                    self.media_bin_textures.clear();
                    self.video_engine.clear_cache();
                    self.audio_mixer.clear_cache();
                    self.audio_player.stop();
                    self.is_playing = false;
                    self.monitor_texture = None;
                    self.source_state.media_id = None;
                    self.source_state.texture = None;
                    self.source_state.is_playing = false;
                    self.target_video_track = self.project.timeline.get_video_tracks().iter().find(|t| t.name == "V1").map(|t| t.id.clone()).unwrap_or_default();
                    self.target_audio_track = self.project.timeline.get_audio_tracks().iter().find(|t| t.name == "A1").map(|t| t.id.clone()).unwrap_or_default();
                    self.autosave_mgr.cleanup(&self.project);
                    for item in self.project.media_pool.values() {
                        if item.media_type == MediaType::Audio || item.audio_channels > 0 {
                            self.audio_mixer.preload_audio_async(&item.file_path);
                        }
                    }
                    self.status_message = format!("Proje yüklendi: {:?}", path);
                }
                Err(e) => {
                    self.status_message = format!("Proje yüklenemedi: {}", e);
                }
            }
        }
    }

    pub fn open_path(&mut self, path: &Path) {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        if ext == "trim" || ext == "cutline" || ext == "json" {
            match load_project(path) {
                Ok(proj) => {
                    self.project = proj;
                    self.undo_stack.clear();
                    self.canvas_state.selected_clip_pairs.clear();
                    self.current_time = 0.0;
                    self.media_bin_textures.clear();
                    self.video_engine.clear_cache();
                    self.audio_mixer.clear_cache();
                    self.audio_player.stop();
                    self.is_playing = false;
                    self.monitor_texture = None;
                    self.source_state.media_id = None;
                    self.source_state.texture = None;
                    self.source_state.is_playing = false;
                    self.target_video_track = self.project.timeline.get_video_tracks().iter().find(|t| t.name == "V1").map(|t| t.id.clone()).unwrap_or_default();
                    self.target_audio_track = self.project.timeline.get_audio_tracks().iter().find(|t| t.name == "A1").map(|t| t.id.clone()).unwrap_or_default();
                    self.autosave_mgr.cleanup(&self.project);
                    for item in self.project.media_pool.values() {
                        if item.media_type == MediaType::Audio || item.audio_channels > 0 {
                            self.audio_mixer.preload_audio_async(&item.file_path);
                        }
                    }
                    self.welcome_dialog.close();
                    self.status_message = format!("Proje yüklendi: {:?}", path);
                }
                Err(e) => {
                    self.status_message = format!("Proje yüklenemedi: {}", e);
                }
            }
        } else {
            self.import_media_file(path);
        }
    }

    pub fn save_project_dialog(&mut self) -> bool {
        if let Some(fp) = self.project.file_path.clone() {
            match save_project(&mut self.project, fp) {
                Ok(()) => {
                    self.autosave_mgr.cleanup(&self.project);
                    self.status_message = "Proje kaydedildi".to_string();
                    true
                }
                Err(e) => {
                    self.status_message = format!("Kayıt hatası: {}", e);
                    false
                }
            }
        } else if let Some(path) = rfd::FileDialog::new()
            .set_title("Projeyi Kaydet")
            .add_filter("triM. Projesi (*.trim)", &["trim"])
            .save_file()
        {
            match save_project(&mut self.project, path) {
                Ok(()) => {
                    self.autosave_mgr.cleanup(&self.project);
                    self.status_message = "Proje kaydedildi".to_string();
                    true
                }
                Err(e) => {
                    self.status_message = format!("Kayıt hatası: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn save_project_as_dialog(&mut self) -> bool {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Projeyi Farklı Kaydet")
            .add_filter("triM. Projesi (*.trim)", &["trim"])
            .save_file()
        {
            match save_project(&mut self.project, path.clone()) {
                Ok(()) => {
                    self.autosave_mgr.cleanup(&self.project);
                    self.status_message = format!("Proje kaydedildi: {:?}", path);
                    true
                }
                Err(e) => {
                    self.status_message = format!("Kayıt hatası: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn bundle_project_dialog(&mut self) {
        let default_name = format!("{}_Paket", self.project.settings.name);
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("Projeyi ve Medyaları Paketleyeceğiniz Klasörü Seçin")
            .pick_folder()
        {
            let target_dir = folder.join(&default_name);
            match bundle_project(&self.project, &target_dir) {
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
            }
        }
    }

    pub fn import_subtitles_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Altyazı Dosyası Seç (.srt, .vtt)")
            .add_filter("Altyazı Dosyaları (*.srt, *.vtt)", &["srt", "vtt"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let parsed = if ext == "vtt" {
                        trim_core::parse_vtt(&content)
                    } else {
                        trim_core::parse_srt(&content)
                    };

                    match parsed {
                        Ok(entries) => {
                            if entries.is_empty() {
                                self.status_message = "Altyazı dosyasında geçerli satır bulunamadı".to_string();
                                return;
                            }
                            let clips = trim_core::subtitle_entries_to_clips(&entries, 48.0);
                            let count = clips.len();

                            let target_track_id = if let Some(v_track) = self
                                .project
                                .timeline
                                .tracks
                                .iter()
                                .find(|t| t.track_type == trim_core::track::TrackType::Video && t.name.to_lowercase().contains("altyazı"))
                            {
                                v_track.id.clone()
                            } else {
                                self.project.timeline.add_new_video_track()
                            };

                            if let Some(tr) = self.project.timeline.get_track_by_id_mut(&target_track_id) {
                                if !tr.name.to_lowercase().contains("altyazı") {
                                    tr.name = format!("{} (Altyazı)", tr.name);
                                }
                                for clip in clips {
                                    tr.add_clip(clip);
                                }
                                self.project.mark_dirty();
                                self.status_message = format!("{} adet altyazı zaman çizgisine başarıyla eklendi", count);
                            }
                        }
                        Err(e) => {
                            self.status_message = format!("Altyazı ayrıştırma hatası: {}", e);
                        }
                    }
                }
                Err(e) => {
                    self.status_message = format!("Dosya okunamadı: {}", e);
                }
            }
        }
    }

    pub fn export_subtitles_dialog(&mut self) {
        let mut all_clips = Vec::new();
        for t in &self.project.timeline.tracks {
            for c in &t.clips {
                if c.text_overlay.as_ref().is_some_and(|txt| !txt.content.trim().is_empty()) {
                    all_clips.push(c.clone());
                }
            }
        }

        if all_clips.is_empty() {
            self.status_message = "Dışa aktarılacak metin veya altyazı klibi bulunamadı".to_string();
            return;
        }

        let entries = trim_core::clips_to_subtitle_entries(&all_clips);
        let srt_text = trim_core::export_srt(&entries);
        let default_name = format!("{}_altyazi.srt", self.project.settings.name);

        if let Some(path) = rfd::FileDialog::new()
            .set_title("Altyazıları SRT Olarak Kaydet")
            .set_file_name(&default_name)
            .add_filter("SubRip Altyazı (*.srt)", &["srt"])
            .save_file()
        {
            match std::fs::write(&path, srt_text) {
                Ok(()) => {
                    self.status_message = format!("{} adet altyazı dışa aktarıldı: {:?}", entries.len(), path);
                }
                Err(e) => {
                    self.status_message = format!("Altyazı kayıt hatası: {}", e);
                }
            }
        }
    }

    pub fn export_edl_dialog(&mut self) {
        let edl_text = trim_export::generate_edl(&self.project);
        let default_name = format!("{}.edl", self.project.settings.name);

        if let Some(path) = rfd::FileDialog::new()
            .set_title("Projeyi CMX 3600 EDL Olarak Dışa Aktar")
            .set_file_name(&default_name)
            .add_filter("CMX 3600 EDL (*.edl)", &["edl"])
            .save_file()
        {
            match std::fs::write(&path, edl_text) {
                Ok(()) => {
                    self.status_message = format!("EDL başarıyla dışa aktarıldı: {:?}", path);
                }
                Err(e) => {
                    self.status_message = format!("EDL kayıt hatası: {}", e);
                }
            }
        }
    }

    pub fn export_fcpxml_dialog(&mut self) {
        let xml_text = trim_export::generate_fcpxml(&self.project);
        let default_name = format!("{}.xml", self.project.settings.name);

        if let Some(path) = rfd::FileDialog::new()
            .set_title("Projeyi Final Cut Pro XML (Premiere/DaVinci) Olarak Dışa Aktar")
            .set_file_name(&default_name)
            .add_filter("Final Cut Pro XML (*.xml)", &["xml"])
            .save_file()
        {
            match std::fs::write(&path, xml_text) {
                Ok(()) => {
                    self.status_message = format!("FCPXML başarıyla dışa aktarıldı: {:?}", path);
                }
                Err(e) => {
                    self.status_message = format!("FCPXML kayıt hatası: {}", e);
                }
            }
        }
    }

    pub fn smart_scene_cut_selected_clip(&mut self) {
        let selected = self.canvas_state.selected_clip_pairs.first().cloned();
        let Some((tr_id, cl_id)) = selected else {
            self.status_message = "Lütfen önce sahne tespiti yapılacak bir video klibi seçin".to_string();
            return;
        };

        let clip_info = if let Some(tr) = self.project.timeline.get_track_by_id(&tr_id) {
            tr.get_clip_by_id(&cl_id).cloned()
        } else {
            None
        };

        let Some(clip) = clip_info else {
            self.status_message = "Seçili klip bulunamadı".to_string();
            return;
        };

        let media_path = if let Some(media) = self.project.get_media(&clip.media_id) {
            std::path::PathBuf::from(&media.file_path)
        } else {
            std::path::PathBuf::from(&clip.media_id)
        };

        if !media_path.exists() {
            self.status_message = "Klibin kaynak video dosyası bulunamadı".to_string();
            return;
        }

        let source_in = clip.source_in;
        let source_out = clip.effective_source_out();

        self.status_message = "Sahneler taranıyor (FFmpeg)...".to_string();

        match trim_media::detect_scene_changes(&media_path, source_in, source_out, 0.40) {
            Ok(source_cuts) => {
                if source_cuts.is_empty() {
                    self.status_message = "Klipte belirgin sahne geçişi algılanmadı (Eşik: 0.40)".to_string();
                    return;
                }

                let timeline_cuts: Vec<f64> = source_cuts
                    .iter()
                    .map(|&s| clip.timeline_in + (s - clip.source_in) / clip.speed)
                    .filter(|&t| t > clip.timeline_in + 0.05 && t < clip.timeline_out - 0.05)
                    .collect();

                let count = timeline_cuts.len();
                if count == 0 {
                    self.status_message = "Klip sınırları içinde yeni sahne kesimi bulunamadı".to_string();
                    return;
                }

                let cmd = Box::new(trim_commands::BatchSplitClipCommand::new(&tr_id, &cl_id, timeline_cuts));
                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                self.project.mark_dirty();
                self.status_message = format!("Akıllı Kesim Tamamlandı: {} yeni sahne algılandı ve bölündü", count);
            }
            Err(e) => {
                self.status_message = format!("Sahne algılama başarısız: {}", e);
            }
        }
    }

    pub fn open_silence_cut_dialog_for_selected(&mut self) {
        let selected = self.canvas_state.selected_clip_pairs.first().cloned();
        let Some((tr_id, cl_id)) = selected else {
            self.status_message = "Lütfen önce sessizlikleri kesilecek bir klip seçin".to_string();
            return;
        };

        let clip_info = if let Some(tr) = self.project.timeline.get_track_by_id(&tr_id) {
            tr.get_clip_by_id(&cl_id).cloned()
        } else {
            None
        };

        let Some(clip) = clip_info else {
            self.status_message = "Seçili klip bulunamadı".to_string();
            return;
        };

        let name = if !clip.name.is_empty() {
            clip.name.clone()
        } else if let Some(media) = self.project.get_media(&clip.media_id) {
            media.name.clone()
        } else {
            "Klip".to_string()
        };

        self.silence_dialog.open(tr_id, cl_id, name, clip.duration());
    }

    pub fn detect_silence_for_dialog(&mut self, track_id: &str, clip_id: &str, noise_db: f32, min_dur: f64) {
        let clip_info = if let Some(tr) = self.project.timeline.get_track_by_id(track_id) {
            tr.get_clip_by_id(clip_id).cloned()
        } else {
            None
        };
        let Some(clip) = clip_info else {
            self.silence_dialog.status_text = "Klip bulunamadı".to_string();
            self.silence_dialog.is_analyzing = false;
            return;
        };

        let media_path = if let Some(media) = self.project.get_media(&clip.media_id) {
            std::path::PathBuf::from(&media.file_path)
        } else {
            std::path::PathBuf::from(&clip.media_id)
        };

        if !media_path.exists() {
            self.silence_dialog.status_text = "Kaynak medya dosyası bulunamadı".to_string();
            self.silence_dialog.is_analyzing = false;
            return;
        }

        let source_in = clip.source_in;
        let source_out = clip.effective_source_out();
        let padding = self.silence_dialog.padding;

        match trim_media::detect_silence_regions(&media_path, source_in, source_out, noise_db, min_dur) {
            Ok(silences) => {
                let speech = trim_media::calculate_speech_intervals(source_in, source_out, &silences, padding);
                self.silence_dialog.set_analysis_results(silences, speech);
            }
            Err(e) => {
                self.silence_dialog.status_text = format!("Hata: {}", e);
                self.silence_dialog.is_analyzing = false;
            }
        }
    }

    pub fn open_transcribe_dialog_for_selected(&mut self) {
        let selected = self.canvas_state.selected_clip_pairs.first().cloned();
        let Some((tr_id, cl_id)) = selected else {
            self.status_message = "Lütfen önce deşifre edilecek bir klip seçin".to_string();
            return;
        };

        let clip_info = if let Some(tr) = self.project.timeline.get_track_by_id(&tr_id) {
            tr.get_clip_by_id(&cl_id).cloned()
        } else {
            None
        };
        let Some(clip) = clip_info else {
            self.status_message = "Klip bulunamadı".to_string();
            return;
        };

        let media_path = if let Some(media) = self.project.get_media(&clip.media_id) {
            media.file_path.clone()
        } else {
            clip.media_id.clone()
        };

        let name = if !clip.name.is_empty() {
            clip.name.clone()
        } else {
            "Klip".to_string()
        };

        self.transcribe_dialog.open(media_path, name);
    }

    pub fn open_screen_record_dialog(&mut self) {
        self.screen_record_dialog.open();
    }

    pub fn toggle_link_selected_clips(&mut self) {
        if self.canvas_state.selected_clip_pairs.len() < 2 {
            if let Some((tr_id, cl_id)) = self.canvas_state.selected_clip_pairs.first() {
                if let Some(tr) = self.project.timeline.get_track_by_id(tr_id) {
                    if let Some(cl) = tr.get_clip_by_id(cl_id) {
                        if cl.linked_group_id.is_some() {
                            let cmd = Box::new(UnlinkClipsCommand::new(vec![(tr_id.clone(), cl_id.clone())]));
                            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                            self.status_message = "Klip bağı kaldırıldı".to_string();
                            self.project.mark_dirty();
                            return;
                        }
                    }
                }
            }
            self.status_message = "Bağlamak için en az 2 klip seçin (Shift + Tık)".to_string();
            return;
        }

        let first_pair = &self.canvas_state.selected_clip_pairs[0];
        let first_gid = self.project.timeline.get_track_by_id(&first_pair.0)
            .and_then(|t| t.get_clip_by_id(&first_pair.1))
            .and_then(|c| c.linked_group_id.clone());

        let all_linked = first_gid.is_some() && self.canvas_state.selected_clip_pairs.iter().all(|(tr_id, cl_id)| {
            self.project.timeline.get_track_by_id(tr_id)
                .and_then(|t| t.get_clip_by_id(cl_id))
                .and_then(|c| c.linked_group_id.as_ref()) == first_gid.as_ref()
        });

        if all_linked {
            let cmd = Box::new(UnlinkClipsCommand::new(self.canvas_state.selected_clip_pairs.clone()));
            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
            self.status_message = "Kliplerin bağı kaldırıldı (Unlink)".to_string();
        } else {
            let cmd = Box::new(LinkClipsCommand::new(self.canvas_state.selected_clip_pairs.clone()));
            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
            self.status_message = "Klipler birbirine bağlandı (Link)".to_string();
        }
        self.project.mark_dirty();
    }

    pub fn add_video_track(&mut self) {
        let max_idx = self
            .project
            .timeline
            .get_video_tracks()
            .iter()
            .filter_map(|t| t.name.strip_prefix('V').and_then(|s| s.parse::<usize>().ok()))
            .max()
            .unwrap_or(0);
        let track = trim_core::track::Track::new(format!("V{}", max_idx + 1), trim_core::track::TrackType::Video);
        let name = track.name.clone();
        let cmd = Box::new(AddTrackCommand::new(track, 0));
        let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
        self.status_message = format!("Video kanalı eklendi: {}", name);
        self.project.mark_dirty();
    }

    pub fn add_audio_track(&mut self) {
        let max_idx = self
            .project
            .timeline
            .get_audio_tracks()
            .iter()
            .filter_map(|t| t.name.strip_prefix('A').and_then(|s| s.parse::<usize>().ok()))
            .max()
            .unwrap_or(0);
        let track = trim_core::track::Track::new(format!("A{}", max_idx + 1), trim_core::track::TrackType::Audio);
        let name = track.name.clone();
        let len = self.project.timeline.tracks.len();
        let cmd = Box::new(AddTrackCommand::new(track, len));
        let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
        self.status_message = format!("Ses kanalı eklendi: {}", name);
        self.project.mark_dirty();
    }

    pub fn reset_to_new_project(&mut self) {
        self.project = Project::default();
        self.undo_stack.clear();
        self.canvas_state.selected_clip_pairs.clear();
        self.current_time = 0.0;
        self.media_bin_textures.clear();
        self.video_engine.clear_cache();
        self.audio_mixer.clear_cache();
        self.audio_player.stop();
        self.is_playing = false;
        self.monitor_texture = None;
        self.source_state.media_id = None;
        self.source_state.texture = None;
        self.source_state.is_playing = false;
        self.target_video_track = self.project.timeline.get_video_tracks().iter().find(|t| t.name == "V1").map(|t| t.id.clone()).unwrap_or_default();
        self.target_audio_track = self.project.timeline.get_audio_tracks().iter().find(|t| t.name == "A1").map(|t| t.id.clone()).unwrap_or_default();
        self.status_message = "Yeni boş proje oluşturuldu".to_string();
    }

    pub fn delete_selected_clips(&mut self) {
        if !self.canvas_state.selected_clip_pairs.is_empty() {
            let unlocked_pairs: Vec<(String, String)> = self.canvas_state.selected_clip_pairs
                .iter()
                .filter(|(tr_id, _)| !self.project.timeline.get_track_by_id(tr_id).map(|t| t.locked).unwrap_or(false))
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
    }

    pub fn ripple_delete_selected_clips(&mut self) {
        if let Some((tr_id, cl_id)) = self.canvas_state.selected_clip_pairs.first().cloned() {
            let is_locked = self.project.timeline.get_track_by_id(&tr_id).map(|t| t.locked).unwrap_or(false);
            if !is_locked {
                let cmd = Box::new(RippleDeleteCommand::new(tr_id, cl_id));
                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                self.canvas_state.selected_clip_pairs.clear();
                self.project.mark_dirty();
            } else {
                self.status_message = "Kilitli kanaldaki klip silinemez".to_string();
            }
        }
    }

    pub fn select_all_clips(&mut self) {
        let mut all = Vec::new();
        for tr in &self.project.timeline.tracks {
            for cl in &tr.clips {
                all.push((tr.id.clone(), cl.id.clone()));
            }
        }
        self.canvas_state.selected_clip_pairs = all;
    }

    pub fn split_at_playhead_command(&mut self) {
        let preferred_track = self.canvas_state.selected_clip_pairs.first().map(|(t, _)| t.as_str());
        let preferred_clip = self.canvas_state.selected_clip_pairs.first().map(|(_, c)| c.as_str());
        split_at_playhead(&mut self.project.timeline, &mut self.undo_stack, preferred_track, preferred_clip, self.current_time);
        self.project.mark_dirty();
    }

    pub fn add_marker_at_playhead(&mut self) {
        self.add_or_toggle_marker();
    }

    pub fn insert_source_to_timeline(&mut self, is_overwrite: bool) {
        let Some(ref m_id) = self.source_state.media_id.clone() else {
            self.status_message = "Kaynak monitöründe seçili klip yok".to_string();
            return;
        };
        let (media_duration, media_type, media_name, media_fps, media_w, media_h) = match self.project.get_media(m_id) {
            Some(m) => (m.duration, m.media_type, m.name.clone(), m.fps, m.width, m.height),
            None => return,
        };

        let start_in = self.source_state.mark_in.unwrap_or(0.0).clamp(0.0, media_duration);
        let end_out = self.source_state.mark_out.unwrap_or(media_duration).clamp(start_in, media_duration);
        let clip_dur = if media_type == MediaType::Image {
            5.0
        } else {
            (end_out - start_in).max(0.1)
        };

        let target_track_id = if media_type == MediaType::Audio {
            if self.project.timeline.get_track_by_id(&self.target_audio_track).is_some() {
                self.target_audio_track.clone()
            } else {
                self.project.timeline.get_audio_tracks().first().map(|t| t.id.clone()).unwrap_or_default()
            }
        } else if self.project.timeline.get_track_by_id(&self.target_video_track).is_some() {
            self.target_video_track.clone()
        } else {
            self.project.timeline.get_video_tracks().first().map(|t| t.id.clone()).unwrap_or_default()
        };

        if target_track_id.is_empty() {
            self.status_message = "Hedef kanal bulunamadı".to_string();
            return;
        }

        if let Some(track) = self.project.timeline.get_track_by_id(&target_track_id) {
            if track.locked {
                self.status_message = "Hedef kanal kilitli, ekleme yapılamaz".to_string();
                return;
            }
        }

        let insert_time = self.current_time;

        // Ripple insert: shift downstream clips forward on target track
        if !is_overwrite {
            let mut moves = Vec::new();
            if let Some(track) = self.project.timeline.get_track_by_id(&target_track_id) {
                for c in &track.clips {
                    if c.timeline_in >= insert_time {
                        moves.push((target_track_id.clone(), c.id.clone(), c.timeline_in, c.timeline_in + clip_dur));
                    }
                }
            }
            if !moves.is_empty() {
                let cmd = Box::new(MoveMultipleClipsCommand::new(moves));
                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
            }
        }

        let mut new_clip = Clip::new(m_id.clone(), insert_time, insert_time + clip_dur);
        new_clip.name = media_name;
        new_clip.source_in = start_in;
        new_clip.source_out = Some(end_out);

        if self.project.timeline.tracks.iter().all(|t| t.clips.is_empty()) {
            if media_fps > 1.0 {
                self.project.timeline.fps = media_fps;
            }
            if media_w > 0 && media_h > 0 {
                self.project.timeline.width = media_w;
                self.project.timeline.height = media_h;
            }
        }

        let cmd = Box::new(AddClipCommand::new(target_track_id, new_clip));
        let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
        self.project.mark_dirty();

        self.seek(insert_time + clip_dur);
        self.status_message = if is_overwrite {
            "Klip üzerine yazıldı (Overwrite)".to_string()
        } else {
            "Klip eklendi (Insert)".to_string()
        };
    }

    pub fn export_current_frame(&mut self) {
        if let Some(frame) = self.video_engine.last_rendered_frame.as_ref() {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let filename = format!("kare_{}_{:06}.png", ts, (self.current_time * 1000.0) as u64);
            let out_path = std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join(&filename);
            if frame.save(&out_path).is_ok() {
                if let Ok(mut item) = FFprobeAnalyzer::analyze(&out_path) {
                    item.name = filename;
                    item.duration = 5.0;
                    self.project.add_media(item);
                    self.status_message = format!("Kare dışa aktarıldı ve havuza eklendi: {}", out_path.display());
                } else {
                    self.status_message = format!("Kare kaydedildi: {}", out_path.display());
                }
            } else {
                self.status_message = "Kare kaydedilemedi".to_string();
            }
        } else {
            self.status_message = "Dışa aktarılacak aktif kare yok".to_string();
        }
    }

    pub fn update_playback_tick(&mut self) {
        if self.is_playing {
            if let Some(start) = self.clock_start {
                let elapsed = start.elapsed().as_secs_f64() * self.playback_speed;
                let cur = self.clock_start_time + elapsed;
                let dur = self.project.timeline.duration();

                if dur > 0.0 && (cur >= dur || cur < 0.0) {
                    self.current_time = cur.clamp(0.0, dur);
                    self.pause();
                } else {
                    let prev_t = self.current_time;
                    self.current_time = cur.max(0.0);

                    // Render mixed audio chunk & update VU levels & output to speakers
                    if (self.playback_speed - 1.0).abs() < 0.01 {
                        let (samples, db_l, db_r) = self.audio_mixer.render_audio_interval(&self.project, prev_t, self.current_time);
                        self.vu_state.set_levels(db_l, db_r);
                        self.audio_player.write_samples(&samples);
                    }
                }
            }
        }
        self.vu_state.tick_decay();
    }

    pub fn update_monitor_frame(&mut self, ctx: &egui::Context) {
        let frame_opt = self.video_engine.render_timeline_frame(
            &self.project,
            self.current_time,
            self.is_playing,
        );

        if let Some(frame) = frame_opt {
            if let Some(last) = &self.last_rendered_monitor_frame {
                if std::sync::Arc::ptr_eq(last, &frame) {
                    return;
                }
            }
            self.last_rendered_monitor_frame = Some(frame.clone());
            let color_image = ColorImage::from_rgba_unmultiplied(
                [frame.width() as usize, frame.height() as usize],
                frame.as_raw(),
            );
            if let Some(tex) = &mut self.monitor_texture {
                tex.set(color_image, TextureOptions::LINEAR);
            } else {
                self.monitor_texture = Some(ctx.load_texture("monitor_preview", color_image, TextureOptions::LINEAR));
            }
        } else {
            self.last_rendered_monitor_frame = None;
            self.monitor_texture = None;
        }
    }

    pub fn update_source_monitor_frame(&mut self, ctx: &egui::Context) {
        let Some(ref m_id) = self.source_state.media_id else {
            self.source_state.texture = None;
            return;
        };
        let Some(media) = self.project.get_media(m_id) else {
            self.source_state.texture = None;
            return;
        };

        if self.source_state.is_playing {
            let dt = 1.0 / 30.0;
            self.source_state.current_time += dt;
            if self.source_state.current_time >= media.duration {
                self.source_state.current_time = self.source_state.mark_in.unwrap_or(0.0);
                self.source_state.is_playing = false;
            }
            ctx.request_repaint();
        }

        let needs_decode = match (&self.source_state.last_decoded_id, self.source_state.last_decoded_time) {
            (Some(id), Some(t)) => id != m_id || (t - self.source_state.current_time).abs() > 0.03,
            _ => true,
        };

        if needs_decode {
            let frame_opt = if media.media_type == MediaType::Image {
                image::open(&media.file_path).ok().map(|img| img.to_rgba8())
            } else if media.media_type == MediaType::Video {
                decode_frame_raw(&media.file_path, self.source_state.current_time)
            } else {
                None
            };

            if let Some(frame) = frame_opt {
                let color_image = ColorImage::from_rgba_unmultiplied(
                    [frame.width() as usize, frame.height() as usize],
                    frame.as_raw(),
                );
                if let Some(tex) = &mut self.source_state.texture {
                    tex.set(color_image, TextureOptions::LINEAR);
                } else {
                    self.source_state.texture = Some(ctx.load_texture("source_preview", color_image, TextureOptions::LINEAR));
                }
                self.source_state.last_decoded_id = Some(m_id.clone());
                self.source_state.last_decoded_time = Some(self.source_state.current_time);
            }
        }
    }

    pub fn update_media_bin_thumbnails(&mut self, ctx: &egui::Context) {
        for (m_id, item) in &self.project.media_pool {
            if !self.media_bin_textures.contains_key(m_id) {
                if let Some(thumb_path) = self.thumbnail_gen.generate(item) {
                    if let Ok(dyn_img) = image::open(&thumb_path) {
                        let rgba = dyn_img.to_rgba8();
                        let color_img = ColorImage::from_rgba_unmultiplied(
                            [rgba.width() as usize, rgba.height() as usize],
                            rgba.as_raw(),
                        );
                        let tex = ctx.load_texture(format!("thumb_{}", m_id), color_img, TextureOptions::LINEAR);
                        self.media_bin_textures.insert(m_id.clone(), tex);
                    }
                }
            }
        }
    }

    pub fn render_source_monitor(&mut self, source_ui: &mut egui::Ui) {
        let source_media = self.source_state.media_id.as_ref().and_then(|id| self.project.get_media(id));
        let source_title = source_media.map(|m| m.name.as_str()).unwrap_or("Seçili Klip Yok");
        let source_dur = source_media.map(|m| m.duration).unwrap_or(0.0);
        let source_fps = source_media.map(|m| m.fps).unwrap_or(self.project.timeline.fps);
        let source_tc = seconds_to_timecode(self.source_state.current_time, source_fps);
        let source_dur_tc = seconds_to_timecode(source_dur, source_fps);
        let source_aspect = source_media
            .map(|m| (m.width as f32 / m.height.max(1) as f32).max(0.1))
            .unwrap_or(16.0 / 9.0);
        let source_label = source_media.map(|m| {
            format!("{}x{} • {:.0}fps", m.width, m.height, m.fps)
        });

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
        ).show(source_ui);

        if let Some(act) = s_act {
            match act {
                MonitorAction::ImportMedia => self.prompt_import_media(),
                MonitorAction::TogglePlay => self.source_state.is_playing = !self.source_state.is_playing,
                MonitorAction::StepFrame(d) => {
                    self.source_state.is_playing = false;
                    let new_t = (self.source_state.current_time + (d as f64 / source_fps.max(1.0))).clamp(0.0, source_dur);
                    self.source_state.current_time = new_t;
                }
                MonitorAction::MarkIn => {
                    self.source_state.mark_in = Some(self.source_state.current_time);
                    self.status_message = format!("Kaynak Giriş: {:.2}s", self.source_state.current_time);
                }
                MonitorAction::MarkOut => {
                    self.source_state.mark_out = Some(self.source_state.current_time);
                    self.status_message = format!("Kaynak Çıkış: {:.2}s", self.source_state.current_time);
                }
                MonitorAction::GoToIn => {
                    self.source_state.current_time = self.source_state.mark_in.unwrap_or(0.0);
                }
                MonitorAction::GoToOut => {
                    self.source_state.current_time = self.source_state.mark_out.unwrap_or(source_dur);
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
        let has_clips = self.project.timeline.tracks.iter().any(|t| !t.clips.is_empty());
        let prog_dur = self.project.timeline.duration();
        let prog_fps = self.project.timeline.fps;
        let prog_tc = seconds_to_timecode(self.current_time, prog_fps);
        let prog_dur_tc = seconds_to_timecode(prog_dur, prog_fps);
        let prog_aspect = (self.project.timeline.width as f32 / self.project.timeline.height as f32).max(0.1);

        let top_clip = self.project.timeline.get_top_visible_video_clip_at(self.current_time);
        let prog_label = top_clip.and_then(|(_, c)| self.project.get_media(&c.media_id)).map(|m| {
            format!("{}x{} • {:.0}fps", m.width, m.height, m.fps)
        });

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
                        txt.text_color[0], txt.text_color[1], txt.text_color[2], txt.text_color[3]
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
                    });
                }
            }
        }

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
        .safe_zones(self.show_safe_zones)
        .aspect_ratio_preset(Some(self.project.timeline.aspect_ratio()))
        .text_overlays(active_text_overlays)
        .show(prog_ui);

        if let Some(act) = p_act {
            match act {
                MonitorAction::NewProject => {
                    if self.project.is_dirty {
                        let name = self.project.settings.name.clone();
                        self.close_confirm_dialog.prompt(PendingCloseAction::NewProject, name);
                    } else {
                        self.reset_to_new_project();
                    }
                }
                MonitorAction::OpenProject => {
                    if self.project.is_dirty {
                        let name = self.project.settings.name.clone();
                        self.close_confirm_dialog.prompt(PendingCloseAction::OpenProject, name);
                    } else {
                        self.open_project_dialog();
                    }
                }
                MonitorAction::ImportMedia => self.prompt_import_media(),
                MonitorAction::TogglePlay => self.toggle_play(),
                MonitorAction::StepFrame(d) => self.step_frame(d),
                MonitorAction::MarkIn => {
                    self.status_message = format!("Sekans Giriş: {:.2}s", self.current_time);
                }
                MonitorAction::MarkOut => {
                    self.status_message = format!("Sekans Çıkış: {:.2}s", self.current_time);
                }
                MonitorAction::GoToIn => self.seek(0.0),
                MonitorAction::GoToOut => self.seek(prog_dur),
                MonitorAction::ExportFrame => self.export_current_frame(),
                MonitorAction::Seek(t) => self.seek(t),
                MonitorAction::ToggleSafeZones => {
                    self.show_safe_zones = !self.show_safe_zones;
                }
                MonitorAction::SetAspectRatio(r) => {
                    self.project.timeline.set_aspect_ratio(r);
                    self.status_message = format!("Format Değiştirildi: {}", r.label());
                }
                _ => {}
            }
        }
    }
}

impl eframe::App for TrimApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Intercept window close request if there are unsaved changes
        if ctx.input(|i| i.viewport().close_requested()) && !self.confirmed_exit && self.project.is_dirty {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            let name = self.project.settings.name.clone();
            self.close_confirm_dialog.prompt(PendingCloseAction::ExitApp, name);
        }

        self.update_playback_tick();
        self.update_monitor_frame(ctx);
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
                            Ok(p) => self.status_message = format!("Dışa aktarma tamamlandı: {}", p),
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
        if !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                if i.key_pressed(Key::Space) {
                    self.toggle_play();
                } else if i.key_pressed(Key::Comma) {
                    self.insert_source_to_timeline(false);
                } else if i.key_pressed(Key::Period) {
                    self.insert_source_to_timeline(true);
                } else if i.key_pressed(Key::I) && !i.modifiers.ctrl {
                    if self.source_state.media_id.is_some() {
                        self.source_state.mark_in = Some(self.source_state.current_time);
                        self.status_message = format!("Kaynak Giriş: {:.2}s", self.source_state.current_time);
                    }
                } else if i.key_pressed(Key::O) && !i.modifiers.ctrl {
                    if self.source_state.media_id.is_some() {
                        self.source_state.mark_out = Some(self.source_state.current_time);
                        self.status_message = format!("Kaynak Çıkış: {:.2}s", self.source_state.current_time);
                    }
                } else if i.modifiers.ctrl && i.key_pressed(Key::K) && self.shortcut_profile == ShortcutProfile::PremierePro {
                    let preferred_track = self.canvas_state.selected_clip_pairs.first().map(|(t, _)| t.as_str());
                    let preferred_clip = self.canvas_state.selected_clip_pairs.first().map(|(_, c)| c.as_str());
                    split_at_playhead(&mut self.project.timeline, &mut self.undo_stack, preferred_track, preferred_clip, self.current_time);
                } else if i.modifiers.ctrl && (i.key_pressed(Key::Backslash) || i.key_pressed(Key::B)) && self.shortcut_profile == ShortcutProfile::DaVinciResolve {
                    let preferred_track = self.canvas_state.selected_clip_pairs.first().map(|(t, _)| t.as_str());
                    let preferred_clip = self.canvas_state.selected_clip_pairs.first().map(|(_, c)| c.as_str());
                    split_at_playhead(&mut self.project.timeline, &mut self.undo_stack, preferred_track, preferred_clip, self.current_time);
                } else if i.key_pressed(Key::V) && !i.modifiers.ctrl && self.shortcut_profile != ShortcutProfile::DaVinciResolve {
                    self.current_edit_tool = EditTool::Select;
                } else if i.key_pressed(Key::A) && !i.modifiers.ctrl {
                    if self.shortcut_profile == ShortcutProfile::DaVinciResolve {
                        self.current_edit_tool = EditTool::Select;
                    } else {
                        self.current_edit_tool = EditTool::TrackSelect;
                    }
                } else if i.key_pressed(Key::B) && !i.modifiers.ctrl {
                    if self.shortcut_profile == ShortcutProfile::DaVinciResolve {
                        self.current_edit_tool = EditTool::Razor;
                    } else {
                        self.current_edit_tool = EditTool::Ripple;
                    }
                } else if i.key_pressed(Key::T) && !i.modifiers.ctrl && self.shortcut_profile == ShortcutProfile::DaVinciResolve {
                    self.current_edit_tool = EditTool::Ripple;
                } else if i.key_pressed(Key::C) && !i.modifiers.ctrl {
                    self.current_edit_tool = EditTool::Razor;
                } else if i.key_pressed(Key::S) && !i.modifiers.ctrl && self.shortcut_profile == ShortcutProfile::TrimDefault {
                    let preferred_track = self.canvas_state.selected_clip_pairs.first().map(|(t, _)| t.as_str());
                    let preferred_clip = self.canvas_state.selected_clip_pairs.first().map(|(_, c)| c.as_str());
                    split_at_playhead(&mut self.project.timeline, &mut self.undo_stack, preferred_track, preferred_clip, self.current_time);
                } else if i.key_pressed(Key::Q) && !i.modifiers.ctrl {
                    let preferred_track = self.canvas_state.selected_clip_pairs.first().map(|(t, _)| t.as_str());
                    let preferred_clip = self.canvas_state.selected_clip_pairs.first().map(|(_, c)| c.as_str());
                    ripple_trim_head(&mut self.project.timeline, &mut self.undo_stack, preferred_track, preferred_clip, self.current_time);
                } else if i.key_pressed(Key::W) && !i.modifiers.ctrl {
                    let preferred_track = self.canvas_state.selected_clip_pairs.first().map(|(t, _)| t.as_str());
                    let preferred_clip = self.canvas_state.selected_clip_pairs.first().map(|(_, c)| c.as_str());
                    ripple_trim_tail(&mut self.project.timeline, &mut self.undo_stack, preferred_track, preferred_clip, self.current_time);
                } else if i.key_pressed(Key::M) && !i.modifiers.ctrl {
                    self.add_or_toggle_marker();
                } else if i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace) {
                    let is_ripple = i.modifiers.shift
                        || (self.shortcut_profile == ShortcutProfile::DaVinciResolve && i.key_pressed(Key::Delete));
                    if is_ripple {
                        if let Some((tr_id, cl_id)) = self.canvas_state.selected_clip_pairs.first().cloned() {
                            let is_locked = self.project.timeline.get_track_by_id(&tr_id).map(|t| t.locked).unwrap_or(false);
                            if !is_locked {
                                let cmd = Box::new(RippleDeleteCommand::new(tr_id, cl_id));
                                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                self.canvas_state.selected_clip_pairs.clear();
                            } else {
                                self.status_message = "Kilitli kanaldaki klip silinemez".to_string();
                            }
                        }
                    } else if !self.canvas_state.selected_clip_pairs.is_empty() {
                        let unlocked_pairs: Vec<(String, String)> = self.canvas_state.selected_clip_pairs
                            .iter()
                            .filter(|(tr_id, _)| !self.project.timeline.get_track_by_id(tr_id).map(|t| t.locked).unwrap_or(false))
                            .cloned()
                            .collect();
                        if !unlocked_pairs.is_empty() {
                            let cmd = Box::new(RemoveMultipleClipsCommand::new(unlocked_pairs));
                            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                            self.canvas_state.selected_clip_pairs.clear();
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
                } else if i.key_pressed(Key::J) && !i.modifiers.ctrl {
                    self.shuttle_reverse();
                } else if i.key_pressed(Key::K) && !i.modifiers.ctrl {
                    self.pause();
                } else if i.key_pressed(Key::L) && !i.modifiers.ctrl {
                    self.shuttle_forward();
                } else if (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::Z)) || (i.modifiers.ctrl && i.key_pressed(Key::Y)) {
                    let _ = self.undo_stack.redo(&mut self.project.timeline);
                    self.canvas_state.selected_clip_pairs.retain(|(tr_id, cl_id)| {
                        self.project.timeline.get_track_by_id(tr_id).and_then(|t| t.get_clip_by_id(cl_id)).is_some()
                    });
                    self.project.mark_dirty();
                } else if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(Key::Z) {
                    let _ = self.undo_stack.undo(&mut self.project.timeline);
                    self.canvas_state.selected_clip_pairs.retain(|(tr_id, cl_id)| {
                        self.project.timeline.get_track_by_id(tr_id).and_then(|t| t.get_clip_by_id(cl_id)).is_some()
                    });
                    self.project.mark_dirty();
                } else if i.key_pressed(Key::F1) {
                    self.shortcuts_dialog.toggle();
                } else if i.modifiers.ctrl && i.key_pressed(Key::S) {
                    self.save_project_dialog();
                } else if i.modifiers.ctrl && i.key_pressed(Key::O) {
                    if self.project.is_dirty {
                        let name = self.project.settings.name.clone();
                        self.close_confirm_dialog.prompt(PendingCloseAction::OpenProject, name);
                    } else {
                        self.open_project_dialog();
                    }
                } else if i.modifiers.ctrl && i.key_pressed(Key::N) {
                    if self.project.is_dirty {
                        let name = self.project.settings.name.clone();
                        self.close_confirm_dialog.prompt(PendingCloseAction::NewProject, name);
                    } else {
                        self.reset_to_new_project();
                    }
                } else if i.modifiers.ctrl && i.key_pressed(Key::Q) {
                    if self.project.is_dirty && !self.confirmed_exit {
                        let name = self.project.settings.name.clone();
                        self.close_confirm_dialog.prompt(PendingCloseAction::ExitApp, name);
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
        }

        // -------------------------------------------------------------
        // Top Header: Premiere Pro Navigation Bar & Window Controls
        // -------------------------------------------------------------
        egui::TopBottomPanel::top("top_header").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                // triM. Logo Branding
                ui.label(RichText::new("triM.").size(15.0).strong().color(Color32::from_rgb(0xe0, 0x7a, 0x38)));
                ui.add_space(4.0);

                // Dosya
                ui.menu_button("Dosya", |ui| {
                    if ui.button("Başlangıç Ekranı...").clicked() {
                        self.welcome_dialog.open();
                        ui.close_menu();
                    }
                    if ui.button("Yeni Boş Proje (Ctrl+N)").clicked() {
                        if self.project.is_dirty {
                            let name = self.project.settings.name.clone();
                            self.close_confirm_dialog.prompt(PendingCloseAction::NewProject, name);
                        } else {
                            self.reset_to_new_project();
                        }
                        ui.close_menu();
                    }
                    if ui.button("Proje Aç... (Ctrl+O)").clicked() {
                        if self.project.is_dirty {
                            let name = self.project.settings.name.clone();
                            self.close_confirm_dialog.prompt(PendingCloseAction::OpenProject, name);
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
                    if ui.button("Projeyi Paketle / Arşivle... (Taşınabilir)").clicked() {
                        self.bundle_project_dialog();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Medya İçe Aktar... (Ctrl+I)").clicked() {
                        self.prompt_import_media();
                        ui.close_menu();
                    }
                    if ui.button("Altyazı Dosyası İçe Aktar (.srt, .vtt)...").clicked() {
                        self.import_subtitles_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Altyazıları Dışa Aktar (.srt)...").clicked() {
                        self.export_subtitles_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Projeyi EDL Olarak Dışa Aktar (.edl)...").clicked() {
                        self.export_edl_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Projeyi FCPXML Olarak Dışa Aktar (Premiere/DaVinci)...").clicked() {
                        self.export_fcpxml_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Dışa Aktarma Stüdyosu... (Ctrl+M)").clicked() {
                        self.export_dialog.open(&self.project);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Çıkış (Ctrl+Q)").clicked() {
                        if self.project.is_dirty && !self.confirmed_exit {
                            let name = self.project.settings.name.clone();
                            self.close_confirm_dialog.prompt(PendingCloseAction::ExitApp, name);
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
                    if ui.add_enabled(can_undo, Button::new("Geri Al (Ctrl+Z)")).clicked() {
                        let _ = self.undo_stack.undo(&mut self.project.timeline);
                        self.canvas_state.selected_clip_pairs.retain(|(tr_id, cl_id)| {
                            self.project.timeline.get_track_by_id(tr_id).and_then(|t| t.get_clip_by_id(cl_id)).is_some()
                        });
                        self.project.mark_dirty();
                        ui.close_menu();
                    }
                    let can_redo = self.undo_stack.can_redo();
                    if ui.add_enabled(can_redo, Button::new("Yinele (Ctrl+Y)")).clicked() {
                        let _ = self.undo_stack.redo(&mut self.project.timeline);
                        self.canvas_state.selected_clip_pairs.retain(|(tr_id, cl_id)| {
                            self.project.timeline.get_track_by_id(tr_id).and_then(|t| t.get_clip_by_id(cl_id)).is_some()
                        });
                        self.project.mark_dirty();
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
                    if ui.button("Sessizlikleri Otomatik Kes (Jump-Cut)...").clicked() {
                        self.open_silence_cut_dialog_for_selected();
                        ui.close_menu();
                    }
                    if ui.button("🎙️ Lokal Whisper ile Otomatik Deşifre Et...").clicked() {
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
                            if ui.selectable_label(self.shortcut_profile == *p, p.label()).clicked() {
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
                    if ui.button("🎬 Seçili Klipte Sahneleri Algıla ve Böl (Smart Cut)").clicked() {
                        self.smart_scene_cut_selected_clip();
                        ui.close_menu();
                    }
                    if ui.button("İşaretçi (Marker) Ekle / Kaldır (M)").clicked() {
                        self.add_marker_at_playhead();
                        ui.close_menu();
                    }
                    if ui.button(format!("Manyetik Yapışma: {}", if self.snap_enabled { "Açık" } else { "Kapalı" })).clicked() {
                        self.snap_enabled = !self.snap_enabled;
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
                                self.status_message = format!("Format Değiştirildi: {}", r.label());
                                ui.close_menu();
                            }
                        }
                    });
                    if ui.checkbox(&mut self.show_safe_zones, "Güvenli Alan Rehber Çizgileri").clicked() {
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.checkbox(&mut self.use_proxies, "⚡ Akıllı Proxy Önizleme (720p H.264)").clicked() {
                        self.video_engine.use_proxies = self.use_proxies;
                        self.video_engine.clear_cache();
                        self.last_rendered_dirty = true;
                        let state_str = if self.use_proxies { "Açık" } else { "Kapalı" };
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
                    if ui.button("Tüm Kanal Yüksekliklerini Sıfırla (56px)").clicked() {
                        self.canvas_state.track_heights.clear();
                        ui.close_menu();
                    }
                });

                // Pencere
                ui.menu_button("Pencere", |ui| {
                    if ui.checkbox(&mut self.show_source_monitor, "Kaynak Monitörü (İkili Görünüm)").clicked() {
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
                    let quick_exp_btn = Button::new(RichText::new("Dışa Aktar").color(Color32::WHITE).size(11.5).strong())
                        .fill(Color32::from_rgb(0x28, 0x5a, 0x9e));
                    if ui.add(quick_exp_btn).on_hover_text("Dışa Aktarma Stüdyosu (Ctrl+M)").clicked() {
                        self.export_dialog.open(&self.project);
                    }

                    let mixer_btn = Button::new(RichText::new("Mikser").size(11.0))
                        .fill(if self.audio_mixer_dialog.is_open { Color32::from_rgb(0x2a, 0x54, 0x3d) } else { Color32::from_rgb(0x22, 0x22, 0x2c) });
                    if ui.add(mixer_btn).on_hover_text("Çok Kanallı Ses Mikserini Aç/Kapat").clicked() {
                        self.audio_mixer_dialog.toggle();
                    }

                    let mon_label = if self.show_source_monitor { "Tek Monitör" } else { "İkili Monitör" };
                    let mon_btn = Button::new(RichText::new(mon_label).size(11.0))
                        .fill(if self.show_source_monitor { Color32::from_rgb(0x2a, 0x3d, 0x54) } else { Color32::from_rgb(0x22, 0x22, 0x2c) });
                    if ui.add(mon_btn).on_hover_text("Monitör Görünümünü Değiştir (İkili Monitör / Tek Sekans Monitörü)").clicked() {
                        self.show_source_monitor = !self.show_source_monitor;
                    }

                    let dirty_str = if self.project.is_dirty { " *" } else { "" };
                    let title_text = format!("{} - triM Proje{}", self.project.settings.name, dirty_str);
                    ui.label(RichText::new(title_text).size(11.5).color(Color32::from_rgb(0x90, 0x90, 0xa4)));
                });
            });
        });

        // -------------------------------------------------------------
        // Bottom Status Bar
        // -------------------------------------------------------------
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let dirty = if self.project.is_dirty { " *" } else { "" };
                ui.label(RichText::new(format!(
                    "Proje: {}{} | Sekans: {}x{} @ {:.0}fps | {}",
                    self.project.settings.name,
                    dirty,
                    self.project.timeline.width,
                    self.project.timeline.height,
                    self.project.timeline.fps,
                    self.status_message
                )).size(10.5).color(Color32::from_rgb(0x85, 0x85, 0x95)));
            });
        });

        // -------------------------------------------------------------
        // BOTTOM TIMELINE PANEL: Guaranteed permanently anchored at the bottom
        // -------------------------------------------------------------
        egui::TopBottomPanel::bottom("timeline_panel")
            .resizable(true)
            .min_height(200.0)
            .default_height(340.0)
            .max_height(600.0)
            .show(ctx, |ui| {
                // Sequence Timeline Header Bar
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;

                    let seq_tab_btn = Button::new(RichText::new("Sekans 01").size(12.0).strong().color(Color32::WHITE))
                        .fill(Color32::from_rgb(0x24, 0x24, 0x2e))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x38, 0x7a, 0xe0)));
                    let _ = ui.add(seq_tab_btn);

                    ui.add_space(8.0);
                    let current_tc = seconds_to_timecode(self.current_time, self.project.timeline.fps);
                    ui.label(
                        RichText::new(&current_tc)
                            .monospace()
                            .size(15.0)
                            .color(Color32::from_rgb(0x38, 0xa0, 0xf0))
                            .strong(),
                    );

                    ui.add_space(10.0);
                    let snap_btn = Button::new(
                        RichText::new(if self.snap_enabled { "[Mıknatıs: Açık]" } else { "[Mıknatıs: Kapalı]" })
                            .size(11.0)
                            .color(if self.snap_enabled { Color32::from_rgb(0x50, 0xd0, 0x70) } else { Color32::from_rgb(0x70, 0x70, 0x80) })
                    )
                    .fill(if self.snap_enabled { Color32::from_rgb(0x1a, 0x2e, 0x20) } else { Color32::from_rgb(0x20, 0x20, 0x26) });
                    if ui.add(snap_btn).on_hover_text("Mıknatıs / Hizalama (Snapping)").clicked() {
                        self.snap_enabled = !self.snap_enabled;
                    }

                    if ui.button(RichText::new("[İşaretçi]").size(11.0)).on_hover_text("İşaretçi Ekle / Sil (M)").clicked() {
                        self.add_or_toggle_marker();
                    }

                    if ui.button(RichText::new("[+ Video]").size(11.0)).on_hover_text("Yeni Video Kanalı Ekle (Üste)").clicked() {
                        self.add_video_track();
                    }

                    if ui.button(RichText::new("[+ Ses]").size(11.0)).on_hover_text("Yeni Ses Kanalı Ekle (Alta)").clicked() {
                        self.add_audio_track();
                    }

                    ui.add_space(6.0);
                    let proxy_btn = Button::new(
                        RichText::new(if self.use_proxies { "⚡ Proxy: Açık" } else { "⚡ Proxy: Kapalı" })
                            .size(11.0)
                            .color(if self.use_proxies { Color32::from_rgb(0x38, 0xa0, 0xf0) } else { Color32::from_rgb(0x70, 0x70, 0x80) })
                    )
                    .fill(if self.use_proxies { Color32::from_rgb(0x18, 0x28, 0x3e) } else { Color32::from_rgb(0x20, 0x20, 0x26) });
                    if ui.add(proxy_btn).on_hover_text("Düşük Donanımlar İçin Akıllı 720p Proxy Önizleme").clicked() {
                        self.toggle_proxy();
                    }

                    ui.add_space(4.0);
                    let rec_btn = Button::new(
                        RichText::new("📹 Ekran Kaydet")
                            .size(11.0)
                            .color(Color32::from_rgb(0xe0, 0x80, 0x40)),
                    )
                    .fill(Color32::from_rgb(0x28, 0x20, 0x1e));
                    if ui.add(rec_btn).on_hover_text("Dahili Ekran ve Web Kamerası Kaydedici").clicked() {
                        self.open_screen_record_dialog();
                    }

                    if let Some(ref vo) = self.active_voiceover {
                        let vo_dur = vo.recorder.elapsed_seconds();
                        let vo_rec_btn = Button::new(
                            RichText::new(format!("🔴 SES KAYDI: {:.1}s [⏹ Durdur]", vo_dur))
                                .size(11.0)
                                .strong()
                                .color(Color32::WHITE)
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
                            egui::Slider::new(&mut self.canvas_state.pixels_per_second, 15.0..=300.0).show_value(false),
                        );
                        ui.label(RichText::new("Zoom:").size(11.0).color(Color32::from_rgb(0x80, 0x80, 0x90)));
                    });
                });

                ui.add_space(2.0);
                ui.separator();

                // Main Timeline Row: [Toolbar] [Track Headers] [Canvas + Ruler] [Master VU Meter]
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;

                    // 1. Vertical Toolbar (Fixed width 34px)
                    ui.allocate_ui(Vec2::new(34.0, ui.available_height()), |ui| {
                        render_vertical_toolbar(ui, &mut self.current_edit_tool);
                    });

                    ui.add_space(2.0);
                    ui.separator();

                    // 2. Pinned Track Headers (Left, Fixed width 142px)
                    let header_w = 142.0;
                    ui.allocate_ui(Vec2::new(header_w, ui.available_height()), |ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::ZERO;
                            let (corner_rect, _) = ui.allocate_exact_size(Vec2::new(header_w, 26.0), egui::Sense::hover());
                            let cp = ui.painter_at(corner_rect);
                            cp.rect_filled(corner_rect, 0.0, Color32::from_rgb(0x18, 0x18, 0x1c));
                            cp.rect_stroke(corner_rect, 0.0, Stroke::new(1.0_f32, Color32::from_rgb(0x28, 0x28, 0x30)));
                            cp.text(
                                corner_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "KANALLAR",
                                egui::FontId::proportional(11.0),
                                Color32::from_rgb(0x90, 0x90, 0xa0),
                            );

                            let mut pending_header_actions = Vec::new();
                            for track in &self.project.timeline.tracks {
                                let is_patch = self.target_video_track == track.id || self.target_audio_track == track.id;
                                let tr_h = self.canvas_state.get_track_height(&track.id);
                                let is_recording_vo = self.active_voiceover.as_ref().map(|vo| vo.track_id == track.id).unwrap_or(false);
                                let (_, hdr_act) = render_track_header(ui, track, tr_h, is_patch, is_recording_vo);
                                if let Some(act) = hdr_act {
                                    pending_header_actions.push((track.id.clone(), act));
                                }
                                ui.add_space(self.canvas_state.track_spacing);
                            }

                            for (t_id, act) in pending_header_actions {
                                match act {
                                    TrackHeaderAction::ToggleMute => {
                                        if let Some(t) = self.project.timeline.get_track_by_id_mut(&t_id) {
                                            t.muted = !t.muted;
                                        }
                                    }
                                    TrackHeaderAction::ToggleSolo => {
                                        if let Some(t) = self.project.timeline.get_track_by_id_mut(&t_id) {
                                            t.solo = !t.solo;
                                        }
                                    }
                                    TrackHeaderAction::ToggleLock => {
                                        if let Some(t) = self.project.timeline.get_track_by_id_mut(&t_id) {
                                            t.locked = !t.locked;
                                        }
                                    }
                                    TrackHeaderAction::TogglePatchTarget => {
                                        if let Some(t) = self.project.timeline.get_track_by_id(&t_id) {
                                            if t.track_type == trim_core::track::TrackType::Video {
                                                self.target_video_track = t_id.clone();
                                            } else {
                                                self.target_audio_track = t_id.clone();
                                            }
                                        }
                                    }
                                    TrackHeaderAction::RecordVoiceover => {
                                        let is_currently_rec = self.active_voiceover.as_ref().map(|vo| vo.track_id == t_id).unwrap_or(false);
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
                                        if let Some(t) = self.project.timeline.get_track_by_id_mut(&t_id) {
                                            t.volume = vol;
                                        }
                                    }
                                    TrackHeaderAction::ResizeTrack(new_h) => {
                                        self.canvas_state.set_track_height(&t_id, new_h);
                                    }
                                    TrackHeaderAction::DeleteTrack => {
                                        let cmd = Box::new(RemoveTrackCommand::new(t_id.clone()));
                                        match self.undo_stack.push(cmd, &mut self.project.timeline) {
                                            Ok(_) => {
                                                self.canvas_state.selected_clip_pairs.retain(|(tr_id, _)| tr_id != &t_id);
                                                self.status_message = "Kanal silindi".to_string();
                                                self.project.mark_dirty();
                                            }
                                            Err(e) => {
                                                self.status_message = format!("Kanal silinemedi: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        });
                    });

                    // 3. Scrollable Timeline Canvas
                    let vu_w = 46.0;
                    let avail_canvas_w = (ui.available_width() - vu_w).max(100.0);
                    ui.allocate_ui(Vec2::new(avail_canvas_w, ui.available_height()), |ui| {
                        ScrollArea::horizontal().show(ui, |ui| {
                            ui.spacing_mut().item_spacing = Vec2::ZERO;
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::ZERO;
                                let markers = self.project.timeline.markers.clone();
                                let (_ruler_resp, ruler_act) = render_time_ruler(
                                    ui,
                                    self.canvas_state.pixels_per_second,
                                    0.0,
                                    self.project.timeline.duration(),
                                    self.project.timeline.fps,
                                    self.current_time,
                                    &markers,
                                );

                                if let Some(act) = ruler_act {
                                    match act {
                                        RulerAction::Seek(t) => self.seek(t),
                                        RulerAction::MarkerClicked(m_id) => {
                                            if let Some(m) = self.project.timeline.markers.iter().find(|m| m.id == m_id) {
                                                self.seek(m.time);
                                            }
                                        }
                                    }
                                }

                                self.canvas_state.active_tool = self.current_edit_tool;
                                let (_canvas_resp, canvas_act) = render_timeline_canvas(
                                    ui,
                                    &mut self.project,
                                    &mut self.canvas_state,
                                    self.current_time,
                                    &self.waveform_gen,
                                );

                                if let Some(act) = canvas_act {
                                    match act {
                                        CanvasAction::Seek(t) => self.seek(t),
                                        CanvasAction::SelectClip(tr_id, cl_id) => {
                                            self.canvas_state.selected_clip_pairs = vec![(tr_id, cl_id)];
                                        }
                                        CanvasAction::ClearSelection => {
                                            self.canvas_state.selected_clip_pairs.clear();
                                        }
                                        CanvasAction::ClipMoved { old_track_id, new_track_id, clip_id, old_in, new_in } => {
                                            let cmd = Box::new(MoveClipCommand::new(old_track_id, new_track_id.clone(), clip_id.clone(), old_in, new_in));
                                            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                            self.canvas_state.selected_clip_pairs = vec![(new_track_id, clip_id)];
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::BatchClipsMoved(moves) => {
                                            let cmd = Box::new(MoveMultipleClipsCommand::new(moves));
                                            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::ClipTrimmed { track_id, clip_id, old_bounds, new_bounds } => {
                                            let cmd = Box::new(TrimClipCommand::new(track_id, clip_id, old_bounds, new_bounds));
                                            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::ClipModified => {
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::PromptSpeed(tr_id, cl_id) => {
                                            if let Some(tr) = self.project.timeline.get_track_by_id(&tr_id) {
                                                if let Some(cl) = tr.get_clip_by_id(&cl_id) {
                                                    self.speed_dialog.open(tr_id, cl_id, cl.speed, cl.reverse, cl.duration());
                                                }
                                            }
                                        }
                                        CanvasAction::DetachAudio(tr_id, cl_id) => {
                                            let cmd = Box::new(DetachAudioCommand::new(tr_id, cl_id));
                                            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                            self.project.mark_dirty();
                                        }
                                        CanvasAction::SplitAtPlayhead => {
                                            let preferred_track = self.canvas_state.selected_clip_pairs.first().map(|(t, _)| t.as_str());
                                            let preferred_clip = self.canvas_state.selected_clip_pairs.first().map(|(_, c)| c.as_str());
                                            split_at_playhead(&mut self.project.timeline, &mut self.undo_stack, preferred_track, preferred_clip, self.current_time);
                                        }
                                        CanvasAction::RazorCut { track_id, clip_id, cut_time } => {
                                            let cmd = Box::new(SplitClipCommand::new(track_id, clip_id, cut_time));
                                            match self.undo_stack.push(cmd, &mut self.project.timeline) {
                                                Ok(()) => {
                                                    self.status_message = format!("Klip {:.2}. saniyede kesildi", cut_time);
                                                    self.project.mark_dirty();
                                                }
                                                Err(e) => {
                                                    self.status_message = format!("Kesme hatası: {}", e);
                                                }
                                            }
                                        }
                                        CanvasAction::CreateTitleAtTime(t) => {
                                            self.seek(t);
                                            self.title_dialog.open();
                                        }
                                        CanvasAction::Pan(_) => {}
                                        CanvasAction::DeleteSelected => {
                                            let unlocked_pairs: Vec<(String, String)> = self.canvas_state.selected_clip_pairs
                                                .iter()
                                                .filter(|(tr_id, _)| !self.project.timeline.get_track_by_id(tr_id).map(|t| t.locked).unwrap_or(false))
                                                .cloned()
                                                .collect();
                                            if !unlocked_pairs.is_empty() {
                                                let cmd = Box::new(RemoveMultipleClipsCommand::new(unlocked_pairs));
                                                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                                self.canvas_state.selected_clip_pairs.clear();
                                                self.project.mark_dirty();
                                            } else if !self.canvas_state.selected_clip_pairs.is_empty() {
                                                self.status_message = "Kilitli kanaldaki klipler silinemez".to_string();
                                            }
                                        }
                                        CanvasAction::RippleDeleteSelected => {
                                            if let Some((tr_id, cl_id)) = self.canvas_state.selected_clip_pairs.first().cloned() {
                                                let is_locked = self.project.timeline.get_track_by_id(&tr_id).map(|t| t.locked).unwrap_or(false);
                                                if !is_locked {
                                                    let cmd = Box::new(RippleDeleteCommand::new(tr_id, cl_id));
                                                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                                    self.canvas_state.selected_clip_pairs.clear();
                                                    self.project.mark_dirty();
                                                } else {
                                                    self.status_message = "Kilitli kanaldaki klip silinemez".to_string();
                                                }
                                            }
                                        }
                                        CanvasAction::MediaDropped { media_id, track_id, time } => {
                                            if let Some(media) = self.project.get_media(&media_id) {
                                                let clip_dur = if media.media_type == MediaType::Image {
                                                    5.0
                                                } else {
                                                    media.duration.max(0.1)
                                                };
                                                let media_fps = media.fps;
                                                let media_w = media.width;
                                                let media_h = media.height;
                                                let media_name = media.name.clone();

                                                if self.project.timeline.tracks.iter().all(|t| t.clips.is_empty()) {
                                                    if media_fps > 1.0 {
                                                        self.project.timeline.fps = media_fps;
                                                    }
                                                    if media_w > 0 && media_h > 0 {
                                                        self.project.timeline.width = media_w;
                                                        self.project.timeline.height = media_h;
                                                    }
                                                }
                                                let mut clip = Clip::new(media_id, time, time + clip_dur);
                                                clip.name = media_name;
                                                let cmd = Box::new(AddClipCommand::new(track_id, clip));
                                                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                                self.project.mark_dirty();
                                            }
                                        }
                                    }
                                }
                            });
                        });
                    });

                    // 4. Master VU Meter (Far Right)
                    ui.allocate_ui(Vec2::new(vu_w, ui.available_height()), |ui| {
                        ui.add(VuMeterWidget::new(&mut self.vu_state));
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

                    let proj_btn = Button::new(RichText::new("Proje").size(11.5).strong().color(if is_proj { Color32::WHITE } else { Color32::from_rgb(0x80, 0x80, 0x90) }))
                        .fill(if is_proj { Color32::from_rgb(0x28, 0x5a, 0x9e) } else { Color32::from_rgb(0x1a, 0x1a, 0x22) })
                        .stroke(Stroke::new(1.0_f32, if is_proj { Color32::from_rgb(0x4a, 0x8a, 0xe8) } else { Color32::from_rgb(0x2c, 0x2c, 0x38) }));
                    if ui.add(proj_btn).clicked() {
                        self.active_left_tab = LeftPanelTab::Project;
                    }

                    let eff_btn = Button::new(RichText::new("Efektler").size(11.5).strong().color(if is_eff { Color32::WHITE } else { Color32::from_rgb(0x80, 0x80, 0x90) }))
                        .fill(if is_eff { Color32::from_rgb(0x28, 0x5a, 0x9e) } else { Color32::from_rgb(0x1a, 0x1a, 0x22) })
                        .stroke(Stroke::new(1.0_f32, if is_eff { Color32::from_rgb(0x4a, 0x8a, 0xe8) } else { Color32::from_rgb(0x2c, 0x2c, 0x38) }));
                    if ui.add(eff_btn).clicked() {
                        self.active_left_tab = LeftPanelTab::Effects;
                    }

                    if is_proj {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let import_btn = Button::new(RichText::new("+ İçe Aktar").color(Color32::WHITE).size(11.0).strong())
                                .fill(Color32::from_rgb(0x28, 0x5a, 0x9e));
                            if ui.add(import_btn).on_hover_text("Medyayı içe aktar (Ctrl+I)").clicked() {
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
                                    self.status_message = "Klip Kaynak Monitöründe Açıldı".to_string();
                                }
                                MediaBinAction::AddToTimeline(m_id) => {
                                    if let Some(media) = self.project.get_media(&m_id) {
                                        let clip_dur = if media.media_type == MediaType::Image {
                                            5.0
                                        } else {
                                            media.duration.max(0.1)
                                        };
                                        let media_type = media.media_type;
                                        let media_fps = media.fps;
                                        let media_w = media.width;
                                        let media_h = media.height;
                                        let media_name = media.name.clone();

                                        let target_track = if media_type == MediaType::Audio {
                                            if self.project.timeline.get_track_by_id(&self.target_audio_track).is_some() {
                                                Some(self.target_audio_track.clone())
                                            } else {
                                                self.project.timeline.get_audio_tracks().first().map(|t| t.id.clone())
                                            }
                                        } else if self.project.timeline.get_track_by_id(&self.target_video_track).is_some() {
                                            Some(self.target_video_track.clone())
                                        } else {
                                            self.project.timeline.get_video_tracks().first().map(|t| t.id.clone())
                                        };

                                        if let Some(track_id) = target_track {
                                            let is_locked = self.project.timeline.get_track_by_id(&track_id).map(|t| t.locked).unwrap_or(false);
                                            if is_locked {
                                                self.status_message = "Hedef kanal kilitli, klip eklenemez".to_string();
                                            } else {
                                                if self.project.timeline.tracks.iter().all(|t| t.clips.is_empty()) {
                                                    if media_fps > 1.0 {
                                                        self.project.timeline.fps = media_fps;
                                                    }
                                                    if media_w > 0 && media_h > 0 {
                                                        self.project.timeline.width = media_w;
                                                        self.project.timeline.height = media_h;
                                                    }
                                                }
                                                let mut clip = Clip::new(m_id, self.current_time, self.current_time + clip_dur);
                                                clip.name = media_name;
                                                let cmd = Box::new(AddClipCommand::new(track_id, clip));
                                                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                                self.project.mark_dirty();
                                            }
                                        }
                                    }
                                }
                                MediaBinAction::DeleteMedia(m_id) => {
                                    let is_used = self.project.timeline.tracks.iter().any(|t| t.clips.iter().any(|c| c.media_id == m_id));
                                    if is_used {
                                        self.status_message = "Zaman çizgisinde kullanılan bir medya silinemez".to_string();
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
                            }
                        }
                    }
                    LeftPanelTab::Effects => {
                        let selected_clip_id = self.canvas_state.selected_clip_pairs.first().map(|(_, c)| c.clone());
                        if let Some(act) = render_inspector_panel(
                            ui,
                            &mut self.project,
                            selected_clip_id.as_deref(),
                            self.current_time,
                        ) {
                            match act {
                                InspectorAction::ClipModified => {
                                    self.project.mark_dirty();
                                }
                                InspectorAction::ChangeSpeed(spd, rev) => {
                                    if let Some((tr_id, cl_id)) = self.canvas_state.selected_clip_pairs.first() {
                                        if let Some(tr) = self.project.timeline.get_track_by_id_mut(tr_id) {
                                            if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                                                cl.change_speed(spd);
                                                cl.reverse = rev;
                                                tr.sort_clips();
                                                self.project.mark_dirty();
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
                ExportDialogAction::StartExport(preset, out_path) => {
                    let (tx, rx) = channel();
                    self.export_rx = Some(rx);
                    self.export_runner.start_export(self.project.clone(), preset, out_path, tx);
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

        if let Some(act) = self.speed_dialog.show(ctx) {
            match act {
                SpeedDialogAction::Apply { track_id, clip_id, speed, reverse } => {
                    let cmd = Box::new(ChangeClipSpeedCommand::new(track_id, clip_id, speed, reverse));
                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                    self.project.mark_dirty();
                }
            }
        }

        if let Some(act) = self.silence_dialog.show(ctx) {
            match act {
                SilenceDialogAction::DetectRequested { track_id, clip_id, noise_db, min_duration } => {
                    self.detect_silence_for_dialog(&track_id, &clip_id, noise_db, min_duration);
                }
                SilenceDialogAction::ApplyCut { track_id, clip_id, speech_intervals } => {
                    let cmd = Box::new(AutoSilenceCutCommand::new(track_id, clip_id, speech_intervals));
                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                    self.canvas_state.selected_clip_pairs.clear();
                    self.status_message = "Sessizlikler otomatik kesildi (Jump-cut uygulandı)".to_string();
                    self.project.mark_dirty();
                }
            }
        }

        if let Some(act) = self.title_dialog.show(ctx) {
            match act {
                TitleDialogAction::CreateTitle { text, font_size, text_color, bg_color, position, has_shadow, duration } => {
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
                            let target_has_clip = self.project.timeline.get_track_by_id(&self.target_video_track)
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
                                        .filter_map(|t| t.name.strip_prefix('V').and_then(|s| s.parse::<usize>().ok()))
                                        .max()
                                        .unwrap_or(1);
                                    let new_track_name = format!("V{}", max_v + 1);
                                    let new_track = trim_core::track::Track::new(&new_track_name, trim_core::track::TrackType::Video);
                                    let new_id = new_track.id.clone();
                                    let _ = self.undo_stack.push(Box::new(AddTrackCommand::new(new_track, 0)), &mut self.project.timeline);
                                    self.status_message = format!("Üst video kanalı otomatik oluşturuldu: {}", new_track_name);
                                    new_id
                                }
                            } else if self.project.timeline.get_track_by_id(&self.target_video_track).is_some() {
                                self.target_video_track.clone()
                            } else {
                                current_video_tracks.first().map(|t| t.id.clone()).unwrap_or_default()
                            };

                            if !target_track_id.is_empty() {
                                let is_locked = self.project.timeline.get_track_by_id(&target_track_id).map(|t| t.locked).unwrap_or(false);
                                if is_locked {
                                    self.status_message = "Hedef video kanalı kilitli, metin klibi eklenemez".to_string();
                                } else {
                                    let mut clip = Clip::new(m_id, self.current_time, self.current_time + duration);
                                    clip.name = format!("Başlık: {}", text);
                                    clip.text_overlay = Some(TextOverlay {
                                        content: text.clone(),
                                        font_size: font_size as f32,
                                        text_color: [text_color[0], text_color[1], text_color[2], 255],
                                        bg_color: if bg_color[3] > 0 { Some(bg_color) } else { None },
                                        pos_x: 0.5,
                                        pos_y: match position.as_str() {
                                            "lower_third" => 0.80,
                                            "subtitle" => 0.87,
                                            "top" => 0.12,
                                            _ => 0.50,
                                        },
                                        has_shadow,
                                        ..Default::default()
                                    });
                                    let cmd = Box::new(AddClipCommand::new(target_track_id, clip));
                                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                    self.project.mark_dirty();
                                    self.status_message = format!("Metin klibi eklendi ({}s)", duration);
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
                    if let Ok(proj) = load_project(&p) {
                        self.project = proj;
                        self.undo_stack.clear();
                        self.canvas_state.selected_clip_pairs.clear();
                        self.current_time = 0.0;
                        self.media_bin_textures.clear();
                        self.video_engine.clear_cache();
                        self.audio_mixer.clear_cache();
                        self.audio_player.stop();
                        self.is_playing = false;
                        self.monitor_texture = None;
                        self.source_state.media_id = None;
                        self.source_state.texture = None;
                        self.source_state.is_playing = false;
                        self.target_video_track = self.project.timeline.get_video_tracks().iter().find(|t| t.name == "V1").map(|t| t.id.clone()).unwrap_or_default();
                        self.target_audio_track = self.project.timeline.get_audio_tracks().iter().find(|t| t.name == "A1").map(|t| t.id.clone()).unwrap_or_default();
                        for item in self.project.media_pool.values() {
                            if item.media_type == MediaType::Audio || item.audio_channels > 0 {
                                self.audio_mixer.preload_audio_async(&item.file_path);
                            }
                        }
                        let _ = std::fs::remove_file(&p);
                        self.status_message = "Proje otomatik kayıttan kurtarıldı".to_string();
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
        // -------------------------------------------------------------
        if let Some(act) = self.welcome_dialog.show(ctx) {
            match act {
                WelcomeAction::CreateNewProject { name, width, height, fps } => {
                    if self.project.is_dirty {
                        let current_name = self.project.settings.name.clone();
                        self.close_confirm_dialog.prompt(PendingCloseAction::NewProject, current_name);
                        self.welcome_dialog.close();
                    } else {
                        self.reset_to_new_project();
                        self.project.settings.name = name;
                        self.project.timeline.width = width;
                        self.project.timeline.height = height;
                        self.project.timeline.fps = fps;
                        self.status_message = format!("Yeni proje hazır: {}", self.project.settings.name);
                        self.welcome_dialog.close();
                    }
                }
                WelcomeAction::OpenProjectRequested => {
                    if self.project.is_dirty {
                        let name = self.project.settings.name.clone();
                        self.close_confirm_dialog.prompt(PendingCloseAction::OpenProject, name);
                        self.welcome_dialog.close();
                    } else {
                        self.open_project_dialog();
                        self.welcome_dialog.close();
                    }
                }
                WelcomeAction::OpenRecentProject(path) => {
                    if self.project.is_dirty {
                        let name = self.project.settings.name.clone();
                        self.close_confirm_dialog.prompt(PendingCloseAction::OpenPath(PathBuf::from(path)), name);
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
                    if self.save_project_dialog() {
                        match pending {
                            PendingCloseAction::ExitApp => {
                                self.confirmed_exit = true;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            PendingCloseAction::NewProject => self.reset_to_new_project(),
                            PendingCloseAction::OpenProject => self.open_project_dialog(),
                            PendingCloseAction::OpenPath(path) => self.open_path(&path),
                        }
                    }
                }
                CloseConfirmAction::DiscardAndProceed(pending) => {
                    self.project.is_dirty = false;
                    match pending {
                        PendingCloseAction::ExitApp => {
                            self.confirmed_exit = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        PendingCloseAction::NewProject => self.reset_to_new_project(),
                        PendingCloseAction::OpenProject => self.open_project_dialog(),
                        PendingCloseAction::OpenPath(path) => self.open_path(&path),
                    }
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
                TranscribeDialogAction::StartTranscribe { media_path, model_name, language } => {
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
                                match trim_media::run_whisper_transcription(&whisper_bin, &temp_wav, &model_name, Some(&language)) {
                                    Ok(entries) => {
                                        let count = entries.len();
                                        let subtitle_clips = trim_core::subtitles::subtitle_entries_to_clips(&entries, 42.0);
                                        let target_tr = self.project.timeline.get_video_tracks().first().map(|t| t.id.clone()).unwrap_or_default();
                                        for c in subtitle_clips {
                                            let cmd = Box::new(AddClipCommand::new(&target_tr, c));
                                            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                        }
                                        self.transcribe_dialog.close();
                                        self.status_message = format!("{} adet altyazı başarıyla oluşturuldu ve eklendi", count);
                                        self.project.mark_dirty();
                                    }
                                    Err(e) => {
                                        self.transcribe_dialog.is_transcribing = false;
                                        self.transcribe_dialog.status_message = format!("Deşifre hatası: {}", e);
                                    }
                                }
                                let _ = std::fs::remove_file(temp_wav);
                            }
                            Err(e) => {
                                self.transcribe_dialog.is_transcribing = false;
                                self.transcribe_dialog.status_message = format!("Ses ayıklama hatası: {}", e);
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
                ScreenRecordAction::RecordingSaved { file_path, auto_add_to_timeline } => {
                    if let Ok(mut item) = FFprobeAnalyzer::analyze(&file_path) {
                        item.name = file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Ekran Kaydı".to_string());
                        let m_id = item.id.clone();
                        let dur = item.duration;
                        self.project.add_media(item);

                        if auto_add_to_timeline && dur > 0.1 {
                            let current_video_tracks = self.project.timeline.get_video_tracks();
                            if let Some(target_tr) = current_video_tracks.first() {
                                let clip = Clip::new(m_id, self.current_time, self.current_time + dur);
                                let cmd = Box::new(AddClipCommand::new(&target_tr.id, clip));
                                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                self.project.mark_dirty();
                            }
                        }
                        self.status_message = format!("Ekran kaydı içe aktarıldı: {:?}", file_path.file_name().unwrap_or_default());
                    }
                }
            }
        }

        if self.show_about_dialog {
            egui::Window::new("triM. Hakkında")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading(RichText::new("triM.").size(24.0).color(Color32::from_rgb(0xe0, 0x7a, 0x38)));
                        ui.label(format!("v{} (Rust Native Desktop)", env!("CARGO_PKG_VERSION")));
                        ui.add_space(8.0);
                        ui.label("Hafif, hızlı, modern masaüstü video düzenleyici.");
                        ui.label("Geliştirici: MOzcelik14");
                        ui.add_space(14.0);
                        if ui.button("Kapat").clicked() {
                            self.show_about_dialog = false;
                        }
                    });
                });
        }
    }
}
