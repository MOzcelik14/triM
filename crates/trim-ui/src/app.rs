//! Main triM. desktop application implementing `eframe::App`.

use crate::canvas::{render_timeline_canvas, CanvasAction, TimelineCanvasState};
use crate::dialogs::*;
use crate::inspector::{render_inspector_panel, InspectorAction};
use crate::media_bin::{render_media_bin, MediaBinAction};
use crate::monitor::{MonitorAction, MonitorWidget};
use crate::ruler::{render_time_ruler, RulerAction};
use crate::theme::*;
use crate::track_header::{render_track_header, TrackHeaderAction};
use crate::transport::{render_transport_bar, TransportAction};
use crate::vu_meter::{VuMeterState, VuMeterWidget};

use egui::{Color32, ColorImage, Key, RichText, ScrollArea, Stroke, TextureHandle, TextureOptions};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::time::Instant;
use trim_commands::commands::*;
use trim_commands::UndoStack;
use trim_core::clip::Clip;
use trim_core::marker::Marker;
use trim_core::media::MediaType;
use trim_core::project::Project;
use trim_core::time::seconds_to_timecode;
use trim_export::runner::{ExportMessage, ExportRunner};
use trim_media::{generate_title_image, AudioMixer, AudioStreamPlayer, FFprobeAnalyzer, ThumbnailGenerator, VideoEngine, WaveformGenerator};
use trim_project::{load_project, save_project, AutosaveManager};
use trim_timeline::{find_next_edit_point, find_prev_edit_point, ripple_trim_head, ripple_trim_tail, split_at_playhead};

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

    pub export_dialog: ExportDialog,
    pub speed_dialog: SpeedDialog,
    pub title_dialog: TitleDialog,
    pub recovery_dialog: RecoveryDialog,
    pub autosave_mgr: AutosaveManager,

    pub export_runner: ExportRunner,
    pub export_rx: Option<Receiver<ExportMessage>>,

    pub monitor_texture: Option<TextureHandle>,
    pub selected_media_id: Option<String>,
    pub status_message: String,
    pub last_autosave: Instant,
    pub show_about_dialog: bool,
}

impl TrimApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_theme(&cc.egui_ctx);

        let mut app = Self {
            project: Project::default(),
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
            export_dialog: ExportDialog::default(),
            speed_dialog: SpeedDialog::default(),
            title_dialog: TitleDialog::default(),
            recovery_dialog: RecoveryDialog::default(),
            autosave_mgr: AutosaveManager::default(),
            export_runner: ExportRunner::default(),
            export_rx: None,
            monitor_texture: None,
            selected_media_id: None,
            status_message: "Hazır".to_string(),
            last_autosave: Instant::now(),
            show_about_dialog: false,
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
        self.clock_start = Some(Instant::now());
        self.clock_start_time = self.current_time;
    }

    pub fn pause(&mut self) {
        self.is_playing = false;
        self.playback_speed = 1.0;
        self.clock_start = None;
        self.video_engine.streamer.stop();
        self.audio_player.stop();
        self.vu_state.reset();
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
                self.project.add_media(item);
                if has_audio {
                    self.audio_mixer.preload_audio_async(&file_path);
                }
                self.status_message = format!("Medya içe aktarıldı: {}", name);
            }
            Err(e) => {
                self.status_message = format!("Medya analizi başarısız: {}", e);
            }
        }
    }

    pub fn prompt_import_media(&mut self) {
        // Native file picker using rfd or std::process
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Medya Dosyası Seç")
            .add_filter("Tüm Medya Dosyaları", &["mp4", "mov", "mkv", "webm", "mp3", "wav", "aac", "png", "jpg", "jpeg"])
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
                    self.current_time = 0.0;
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
                    self.current_time = 0.0;
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
        } else {
            self.import_media_file(path);
            let has_clips = self.project.timeline.tracks.iter().any(|t| !t.clips.is_empty());
            if !has_clips {
                let p_str = path.to_string_lossy().to_string();
                if let Some((m_id, media)) = self.project.media_pool.iter().find(|(_, m)| m.file_path == p_str) {
                    let clip_dur = if media.media_type == MediaType::Image {
                        5.0
                    } else {
                        media.duration.max(0.1)
                    };
                    let target_track = if media.media_type == MediaType::Audio {
                        self.project.timeline.get_audio_tracks().first().map(|t| t.id.clone())
                    } else {
                        self.project.timeline.get_video_tracks().first().map(|t| t.id.clone())
                    };

                    if let Some(track_id) = target_track {
                        let mut clip = Clip::new(m_id.clone(), 0.0, clip_dur);
                        clip.name = media.name.clone();
                        let cmd = Box::new(AddClipCommand::new(track_id, clip));
                        let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                        self.project.mark_dirty();
                    }
                }
            }
        }
    }

    pub fn save_project_dialog(&mut self) {
        if let Some(fp) = self.project.file_path.clone() {
            let _ = save_project(&mut self.project, fp);
            self.status_message = "Proje kaydedildi".to_string();
        } else if let Some(path) = rfd::FileDialog::new()
            .set_title("Projeyi Kaydet")
            .add_filter("triM. Projesi (*.trim)", &["trim"])
            .save_file()
        {
            let _ = save_project(&mut self.project, path);
            self.status_message = "Proje kaydedildi".to_string();
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
            // A timeline gap or the end of the last clip must remove the
            // previous GPU texture instead of freezing its final frame.
            self.monitor_texture = None;
        }
    }
}

impl eframe::App for TrimApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_playback_tick();
        self.update_monitor_frame(ctx);

        if self.is_playing || self.video_engine.is_decoding_seek {
            ctx.request_repaint();
        }

        // Window drag & drop files from OS file manager
        ctx.input(|i| {
            for file in &i.raw.dropped_files {
                if let Some(path) = &file.path {
                    self.import_media_file(path);
                }
            }
        });

        // Check periodic autosave (every 2 minutes)
        if self.last_autosave.elapsed().as_secs() >= 120 {
            let _ = self.autosave_mgr.save_snapshot(&self.project);
            self.last_autosave = Instant::now();
        }

        // Check export channel messages
        if let Some(rx) = &self.export_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ExportMessage::Progress(pct) => self.export_dialog.progress = pct,
                    ExportMessage::Status(s) => self.export_dialog.status_text = s,
                    ExportMessage::Finished(res) => {
                        self.export_dialog.is_exporting = false;
                        match res {
                            Ok(p) => self.status_message = format!("Dışa aktarma tamamlandı: {}", p),
                            Err(e) => self.status_message = format!("Dışa aktarma hatası: {}", e),
                        }
                    }
                }
            }
        }

        // -------------------------------------------------------------
        // Global Keyboard Shortcuts
        // -------------------------------------------------------------
        ctx.input(|i| {
            if i.key_pressed(Key::Space) {
                self.toggle_play();
            } else if i.key_pressed(Key::S) && !i.modifiers.ctrl {
                let preferred_track = self.canvas_state.selected_clip_pairs.first().map(|(t, _)| t.as_str());
                let preferred_clip = self.canvas_state.selected_clip_pairs.first().map(|(_, c)| c.as_str());
                split_at_playhead(&mut self.project.timeline, &mut self.undo_stack, preferred_track, preferred_clip, self.current_time);
            } else if i.key_pressed(Key::Q) {
                let preferred_track = self.canvas_state.selected_clip_pairs.first().map(|(t, _)| t.as_str());
                let preferred_clip = self.canvas_state.selected_clip_pairs.first().map(|(_, c)| c.as_str());
                ripple_trim_head(&mut self.project.timeline, &mut self.undo_stack, preferred_track, preferred_clip, self.current_time);
            } else if i.key_pressed(Key::W) {
                let preferred_track = self.canvas_state.selected_clip_pairs.first().map(|(t, _)| t.as_str());
                let preferred_clip = self.canvas_state.selected_clip_pairs.first().map(|(_, c)| c.as_str());
                ripple_trim_tail(&mut self.project.timeline, &mut self.undo_stack, preferred_track, preferred_clip, self.current_time);
            } else if i.key_pressed(Key::M) {
                self.add_or_toggle_marker();
            } else if i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace) {
                if i.modifiers.shift {
                    if let Some((tr_id, cl_id)) = self.canvas_state.selected_clip_pairs.first().cloned() {
                        let cmd = Box::new(RippleDeleteCommand::new(tr_id, cl_id));
                        let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                        self.canvas_state.selected_clip_pairs.clear();
                    }
                } else if !self.canvas_state.selected_clip_pairs.is_empty() {
                    let cmd = Box::new(RemoveMultipleClipsCommand::new(self.canvas_state.selected_clip_pairs.clone()));
                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                    self.canvas_state.selected_clip_pairs.clear();
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
            } else if i.key_pressed(Key::J) {
                self.shuttle_reverse();
            } else if i.key_pressed(Key::K) {
                self.pause();
            } else if i.key_pressed(Key::L) {
                self.shuttle_forward();
            } else if i.modifiers.ctrl && i.key_pressed(Key::Z) {
                let _ = self.undo_stack.undo(&mut self.project.timeline);
            } else if (i.modifiers.ctrl && i.key_pressed(Key::Y)) || (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::Z)) {
                let _ = self.undo_stack.redo(&mut self.project.timeline);
            } else if i.modifiers.ctrl && i.key_pressed(Key::S) {
                self.save_project_dialog();
            } else if i.modifiers.ctrl && i.key_pressed(Key::O) {
                self.open_project_dialog();
            } else if i.modifiers.ctrl && i.key_pressed(Key::E) {
                self.export_dialog.is_open = true;
            } else if i.modifiers.ctrl && i.key_pressed(Key::T) {
                self.title_dialog.open();
            }
        });

        // -------------------------------------------------------------
        // Top Menu Bar & Main Toolbar
        // -------------------------------------------------------------
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Dosya", |ui| {
                    if ui.button("Yeni Proje (Ctrl+N)").clicked() {
                        self.project = Project::default();
                        self.undo_stack.clear();
                        self.current_time = 0.0;
                        ui.close_menu();
                    }
                    if ui.button("Proje Aç (Ctrl+O)").clicked() {
                        self.open_project_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Kaydet (Ctrl+S)").clicked() {
                        self.save_project_dialog();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Medya İçe Aktar...").clicked() {
                        self.prompt_import_media();
                        ui.close_menu();
                    }
                    if ui.button("Başlık / Metin Ekle (Ctrl+T)").clicked() {
                        self.title_dialog.open();
                        ui.close_menu();
                    }
                    if ui.button("Videoyu Dışa Aktar (Ctrl+E)").clicked() {
                        self.export_dialog.is_open = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Düzen", |ui| {
                    if ui.button("Geri Al (Ctrl+Z)").clicked() {
                        let _ = self.undo_stack.undo(&mut self.project.timeline);
                        ui.close_menu();
                    }
                    if ui.button("Yinele (Ctrl+Y)").clicked() {
                        let _ = self.undo_stack.redo(&mut self.project.timeline);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Oynatma Çizgisinden Kes / Böl (S)").clicked() {
                        let preferred_track = self.canvas_state.selected_clip_pairs.first().map(|(t, _)| t.as_str());
                        let preferred_clip = self.canvas_state.selected_clip_pairs.first().map(|(_, c)| c.as_str());
                        split_at_playhead(&mut self.project.timeline, &mut self.undo_stack, preferred_track, preferred_clip, self.current_time);
                        ui.close_menu();
                    }
                    if ui.button("Başı Çizgiye Kırp ve Kaydır (Q)").clicked() {
                        let preferred_track = self.canvas_state.selected_clip_pairs.first().map(|(t, _)| t.as_str());
                        let preferred_clip = self.canvas_state.selected_clip_pairs.first().map(|(_, c)| c.as_str());
                        ripple_trim_head(&mut self.project.timeline, &mut self.undo_stack, preferred_track, preferred_clip, self.current_time);
                        ui.close_menu();
                    }
                    if ui.button("Sonu Çizgiye Kırp ve Kaydır (W)").clicked() {
                        let preferred_track = self.canvas_state.selected_clip_pairs.first().map(|(t, _)| t.as_str());
                        let preferred_clip = self.canvas_state.selected_clip_pairs.first().map(|(_, c)| c.as_str());
                        ripple_trim_tail(&mut self.project.timeline, &mut self.undo_stack, preferred_track, preferred_clip, self.current_time);
                        ui.close_menu();
                    }
                });

                ui.menu_button("Yardım", |ui| {
                    if ui.button("triM. Hakkında").clicked() {
                        self.show_about_dialog = true;
                        ui.close_menu();
                    }
                });
            });

            ui.separator();

            // Toolbar buttons
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                if ui.button("Yeni Proje").clicked() {
                    self.project = Project::default();
                    self.undo_stack.clear();
                }
                if ui.button("Proje Aç").clicked() {
                    self.open_project_dialog();
                }
                if ui.button("Kaydet").clicked() {
                    self.save_project_dialog();
                }
                ui.separator();
                if ui.button("+ Medya Ekle").clicked() {
                    self.prompt_import_media();
                }
                if ui.button("Başlık / Metin").clicked() {
                    self.title_dialog.open();
                }
                ui.separator();
                if ui.button("Geri Al").clicked() {
                    let _ = self.undo_stack.undo(&mut self.project.timeline);
                }
                if ui.button("Yinele").clicked() {
                    let _ = self.undo_stack.redo(&mut self.project.timeline);
                }
                ui.separator();
                let exp_btn = egui::Button::new(RichText::new("Videoyu Dışa Aktar").color(Color32::WHITE).strong())
                    .fill(Color32::from_rgb(0xe0, 0x7a, 0x38));
                if ui.add(exp_btn).clicked() {
                    self.export_dialog.is_open = true;
                }
            });
        });

        // Bottom Status Bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let dirty = if self.project.is_dirty { " *" } else { "" };
                ui.label(RichText::new(format!(
                    "Proje: {}{} | {}x{} @ {:.0}fps | {}",
                    self.project.settings.name,
                    dirty,
                    self.project.timeline.width,
                    self.project.timeline.height,
                    self.project.timeline.fps,
                    self.status_message
                )).size(11.0).color(Color32::from_rgb(0x90, 0x90, 0xa0)));
            });
        });

        // Bottom Timeline Panel
        egui::TopBottomPanel::bottom("timeline_panel")
            .resizable(true)
            .min_height(140.0)
            .default_height(220.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Pinned Left Track Headers column (Fixed width 120px)
                    ui.allocate_ui(egui::vec2(120.0, ui.available_height()), |ui| {
                        ui.vertical(|ui| {
                            // Top corner header aligned with time ruler (height 26px)
                            let (corner_rect, _) = ui.allocate_exact_size(egui::vec2(120.0, 26.0), egui::Sense::hover());
                            ui.painter().rect_filled(corner_rect, 0.0, Color32::from_rgb(0x18, 0x18, 0x1c));
                            ui.painter().rect_stroke(corner_rect, 0.0, Stroke::new(1.0_f32, Color32::from_rgb(0x28, 0x28, 0x30)));
                            ui.painter().text(
                                corner_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "KANALLAR",
                                egui::FontId::proportional(11.0),
                                Color32::from_rgb(0x90, 0x90, 0xa0),
                            );

                            ui.add_space(10.0);

                            // Left Track Headers column
                            let mut pending_header_actions = Vec::new();
                            for track in &self.project.timeline.tracks {
                                let (_, hdr_act) = render_track_header(ui, track, self.canvas_state.track_height);
                                if let Some(act) = hdr_act {
                                    pending_header_actions.push((track.id.clone(), act));
                                }
                                ui.add_space(self.canvas_state.track_spacing);
                            }

                            for (t_id, act) in pending_header_actions {
                                if let Some(t) = self.project.timeline.get_track_by_id_mut(&t_id) {
                                    match act {
                                        TrackHeaderAction::ToggleMute => t.muted = !t.muted,
                                        TrackHeaderAction::ToggleSolo => t.solo = !t.solo,
                                        TrackHeaderAction::VolumeChanged(vol) => t.volume = vol,
                                    }
                                }
                            }
                        });
                    });

                    // Right Area: Synchronized horizontal scroll for Time Ruler + Timeline Canvas
                    ScrollArea::horizontal().show(ui, |ui| {
                        ui.vertical(|ui| {
                            // 1. Time Ruler
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

                            // 2. Timeline Canvas
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
                                    CanvasAction::ClipMoved { track_id, clip_id, old_in, new_in } => {
                                        let cmd = Box::new(MoveClipCommand::new(track_id, clip_id, old_in, new_in));
                                        let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
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
                                    CanvasAction::DeleteSelected => {
                                        if !self.canvas_state.selected_clip_pairs.is_empty() {
                                            let cmd = Box::new(RemoveMultipleClipsCommand::new(self.canvas_state.selected_clip_pairs.clone()));
                                            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                            self.canvas_state.selected_clip_pairs.clear();
                                            self.project.mark_dirty();
                                        }
                                    }
                                    CanvasAction::RippleDeleteSelected => {
                                        if let Some((tr_id, cl_id)) = self.canvas_state.selected_clip_pairs.first().cloned() {
                                            let cmd = Box::new(RippleDeleteCommand::new(tr_id, cl_id));
                                            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                            self.canvas_state.selected_clip_pairs.clear();
                                            self.project.mark_dirty();
                                        }
                                    }
                                    CanvasAction::MediaDropped { media_id, track_id, time } => {
                                        if let Some(media) = self.project.get_media(&media_id) {
                                            let clip_dur = if media.media_type == MediaType::Image {
                                                5.0
                                            } else {
                                                media.duration.max(0.1)
                                            };
                                            let mut clip = Clip::new(media_id, time, time + clip_dur);
                                            clip.name = media.name.clone();
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
            });

        // Central Panels (Left: Media Bin, Center: Monitor, Right: Inspector)
        egui::SidePanel::left("left_bin_panel")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                if let Some(act) = render_media_bin(ui, &self.project, &mut self.selected_media_id) {
                    match act {
                        MediaBinAction::ImportRequested => self.prompt_import_media(),
                        MediaBinAction::AddToTimeline(m_id) => {
                            if let Some(media) = self.project.get_media(&m_id) {
                                let clip_dur = if media.media_type == MediaType::Image {
                                    5.0
                                } else {
                                    media.duration.max(0.1)
                                };
                                let target_track = if media.media_type == MediaType::Audio {
                                    self.project.timeline.get_audio_tracks().first().map(|t| t.id.clone())
                                } else {
                                    self.project.timeline.get_video_tracks().first().map(|t| t.id.clone())
                                };

                                if let Some(track_id) = target_track {
                                    let mut clip = Clip::new(m_id, self.current_time, self.current_time + clip_dur);
                                    clip.name = media.name.clone();
                                    let cmd = Box::new(AddClipCommand::new(track_id, clip));
                                    let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                    self.project.mark_dirty();
                                }
                            }
                        }
                    }
                }
            });

        egui::SidePanel::right("right_inspector_panel")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                let selected_id = self.canvas_state.selected_clip_pairs.first().map(|(_, c)| c.as_str());
                if let Some(act) = render_inspector_panel(ui, &mut self.project, selected_id, self.current_time) {
                    match act {
                        InspectorAction::ClipModified => self.project.mark_dirty(),
                        InspectorAction::ChangeSpeed(sp, rev) => {
                            if let Some((tr_id, cl_id)) = self.canvas_state.selected_clip_pairs.first().cloned() {
                                let cmd = Box::new(ChangeClipSpeedCommand::new(tr_id, cl_id, sp, rev));
                                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                self.project.mark_dirty();
                            }
                        }
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let total_w = ui.available_width();
            let total_h = ui.available_height();
            let transport_bar_h = 36.0;
            let monitor_h = (total_h - transport_bar_h - 14.0).max(120.0);
            let monitor_w = (total_w - 52.0).max(160.0);

            // Monitor & VU Meter row
            ui.horizontal(|ui| {
                ui.allocate_ui(egui::vec2(monitor_w, monitor_h), |ui| {
                    let has_clips = self.project.timeline.tracks.iter().any(|t| !t.clips.is_empty());
                    let timecode = seconds_to_timecode(self.current_time, self.project.timeline.fps);
                    let duration_tc = seconds_to_timecode(self.project.timeline.duration(), self.project.timeline.fps);
                    let aspect = (self.project.timeline.width as f32 / self.project.timeline.height as f32).max(0.1);

                    let top_clip = self.project.timeline.get_top_visible_video_clip_at(self.current_time);
                    let media_label = top_clip.and_then(|(_, c)| self.project.get_media(&c.media_id)).map(|m| {
                        format!("{}x{} • {:.0}fps", m.width, m.height, m.fps)
                    });

                    let (_resp, mon_act) = MonitorWidget::new(
                        self.monitor_texture.as_ref(),
                        has_clips,
                        aspect,
                        &timecode,
                        &duration_tc,
                        self.is_playing,
                        self.playback_speed,
                        media_label.as_deref(),
                        self.video_engine.is_decoding_seek,
                    ).show(ui);

                    if let Some(act) = mon_act {
                        match act {
                            MonitorAction::NewProject => {
                                self.project = Project::default();
                                self.undo_stack.clear();
                            }
                            MonitorAction::OpenProject => self.open_project_dialog(),
                            MonitorAction::ImportMedia => self.prompt_import_media(),
                        }
                    }
                });

                // Vertical VU Meter
                ui.add(VuMeterWidget::new(&mut self.vu_state));
            });

            ui.add_space(4.0);
            ui.separator();

            // Transport bar
            let dur = self.project.timeline.duration();
            let fps = self.project.timeline.fps;
            if let Some(t_act) = render_transport_bar(ui, self.current_time, dur, fps, self.is_playing, self.playback_speed) {
                match t_act {
                    TransportAction::TogglePlay => self.toggle_play(),
                    TransportAction::Stop => self.stop(),
                    TransportAction::StepFrame(d) => self.step_frame(d),
                }
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

        if let Some(act) = self.title_dialog.show(ctx) {
            match act {
                TitleDialogAction::CreateTitle { text, font_size, text_color, bg_color, duration } => {
                    if let Some(out_png) = generate_title_image(&text, font_size, text_color, bg_color, self.project.timeline.width, self.project.timeline.height) {
                        if let Ok(mut item) = FFprobeAnalyzer::analyze(&out_png) {
                            item.name = text.clone();
                            item.duration = duration;
                            let m_id = item.id.clone();
                            self.project.add_media(item);

                            if let Some(v_track) = self.project.timeline.get_video_tracks().first() {
                                let clip = Clip::new(m_id, self.current_time, self.current_time + duration);
                                let cmd = Box::new(AddClipCommand::new(v_track.id.clone(), clip));
                                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                                self.project.mark_dirty();
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
                        self.status_message = "Proje otomatik kayıttan kurtarıldı".to_string();
                    }
                }
                RecoveryAction::Discard(p) => {
                    let _ = std::fs::remove_file(p);
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
                        ui.label("v0.3.0 (Rust Native Desktop)");
                        ui.add_space(8.0);
                        ui.label("Hafif, hızlı, modern Linux video düzenleyici.");
                        ui.label("Geliştirici: MOzcelik14");
                        ui.add_space(14.0);
                        if ui.button("Kapat").clicked() {
                            self.show_about_dialog = false;
                        }
                    });
                });
        }

        // Request continuous repaint if playing for smooth 60 FPS
        if self.is_playing {
            ctx.request_repaint();
        }
    }
}
