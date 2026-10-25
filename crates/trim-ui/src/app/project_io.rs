//! Project loading, saving, bundling, subtitle import/export, and NLE timeline interchange (EDL/FCPXML).

use crate::app::TrimApp;
use crate::dialogs::PendingCloseAction;
use crate::file_dialog::{FileDialogConfig, FileDialogRequest};
use std::path::Path;
use trim_core::media::MediaType;
use trim_core::project::Project;
use trim_media::FFprobeAnalyzer;
use trim_project::{bundle_project, load_project, save_project};

impl TrimApp {
    pub fn open_project_dialog(&mut self) {
        let config = FileDialogConfig::pick_file("triM. Projesi Aç").add_filter(
            "triM. Projeleri (*.trim, *.cutline)",
            &["trim", "cutline", "json"],
        );
        self.file_dialog_bridge
            .open(FileDialogRequest::OpenProject, config);
    }

    pub fn load_project_from_path(&mut self, path: &Path) {
        match load_project(path) {
            Ok(proj) => {
                self.project = proj;
                self.undo_stack.clear();
                self.last_saved_undo_len = 0;
                self.canvas_state.selected_clip_pairs.clear();
                self.current_time = 0.0;
                self.media_bin_textures.clear();
                self.video_engine.clear_cache();
                self.audio_mixer.clear_cache();
                self.audio_player.stop();
                self.is_playing = false;
                self.monitor_texture = None;
                self.monitor_gpu_texture_id = None;
                self.source_state.media_id = None;
                self.source_state.texture = None;
                self.source_state.is_playing = false;
                self.resolve_default_target_tracks();
                self.autosave_mgr.cleanup(&self.project);
                for item in self.project.media_pool.values() {
                    if item.media_type == MediaType::Audio || item.audio_channels > 0 {
                        self.audio_mixer.preload_audio_async(&item.file_path);
                    }
                }
                self.welcome_dialog.close();
                let offline = self.project.get_offline_media_ids();
                if !offline.is_empty() {
                    self.status_message = format!(
                        "Proje yüklendi: {:?} ({} adet çevrimdışı medya tespit edildi)",
                        path,
                        offline.len()
                    );
                    self.relink_dialog.open(&self.project);
                } else {
                    self.status_message = format!("Proje yüklendi: {:?}", path);
                }
            }
            Err(e) => {
                self.status_message = format!("Proje yüklenemedi: {}", e);
            }
        }
    }

    pub fn open_path(&mut self, path: &Path) {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext == "trim" || ext == "cutline" || ext == "json" {
            self.load_project_from_path(path);
        } else {
            self.import_media_file(path);
        }
    }

    pub fn save_project_dialog(&mut self) -> bool {
        if let Some(fp) = self.project.file_path.clone() {
            match save_project(&mut self.project, fp) {
                Ok(()) => {
                    self.last_saved_undo_len = self.undo_stack.len();
                    self.project.is_dirty = false;
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
            let config = FileDialogConfig::save_file("Projeyi Kaydet", None)
                .add_filter("triM. Projesi (*.trim)", &["trim"]);
            self.file_dialog_bridge
                .open(FileDialogRequest::SaveProject { after_save: None }, config);
            false
        }
    }

    pub fn save_project_and_proceed(&mut self, ctx: &egui::Context, pending: PendingCloseAction) {
        if self.project.file_path.is_some() {
            if self.save_project_dialog() {
                self.execute_pending_close_action(ctx, pending);
            }
        } else {
            let config = FileDialogConfig::save_file("Projeyi Kaydet", None)
                .add_filter("triM. Projesi (*.trim)", &["trim"]);
            self.file_dialog_bridge.open(
                FileDialogRequest::SaveProject {
                    after_save: Some(pending),
                },
                config,
            );
        }
    }

    pub fn execute_pending_close_action(
        &mut self,
        ctx: &egui::Context,
        pending: PendingCloseAction,
    ) {
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

    pub fn save_project_as_dialog(&mut self) -> bool {
        let config = FileDialogConfig::save_file("Projeyi Farklı Kaydet", None)
            .add_filter("triM. Projesi (*.trim)", &["trim"]);
        self.file_dialog_bridge
            .open(FileDialogRequest::SaveProjectAs, config)
    }

    pub fn bundle_project_dialog(&mut self) {
        let default_folder_name = format!("{}_Paket", self.project.settings.name);
        let config =
            FileDialogConfig::pick_folder("Projeyi ve Medyaları Paketleyeceğiniz Klasörü Seçin");
        self.file_dialog_bridge.open(
            FileDialogRequest::BundleProject {
                default_folder_name,
            },
            config,
        );
    }

    pub fn import_subtitles_dialog(&mut self) {
        let config = FileDialogConfig::pick_file("Altyazı Dosyası Seç (.srt, .vtt)")
            .add_filter("Altyazı Dosyaları (*.srt, *.vtt)", &["srt", "vtt"]);
        self.file_dialog_bridge
            .open(FileDialogRequest::ImportSubtitles, config);
    }

    pub fn import_subtitles_from_path(&mut self, path: &Path) {
        match std::fs::read_to_string(path) {
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
                            self.status_message =
                                "Altyazı dosyasında geçerli satır bulunamadı".to_string();
                            return;
                        }
                        let clips = trim_core::subtitle_entries_to_clips(&entries, 48.0);
                        let count = clips.len();

                        let target_track_id = if let Some(v_track) =
                            self.project.timeline.tracks.iter().find(|t| {
                                t.track_type == trim_core::track::TrackType::Video
                                    && t.name.to_lowercase().contains("altyazı")
                            }) {
                            v_track.id.clone()
                        } else {
                            self.project.timeline.add_new_video_track()
                        };

                        if let Some(tr) =
                            self.project.timeline.get_track_by_id_mut(&target_track_id)
                        {
                            if !tr.name.to_lowercase().contains("altyazı") {
                                tr.name = format!("{} (Altyazı)", tr.name);
                            }
                            for clip in clips {
                                tr.add_clip(clip);
                            }
                            self.project.mark_dirty();
                            self.status_message =
                                format!("{} adet altyazı zaman çizgisine başarıyla eklendi", count);
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

    pub fn export_subtitles_dialog(&mut self) {
        let mut all_clips = Vec::new();
        for t in &self.project.timeline.tracks {
            for c in &t.clips {
                if c.text_overlay
                    .as_ref()
                    .is_some_and(|txt| !txt.content.trim().is_empty())
                {
                    all_clips.push(c.clone());
                }
            }
        }

        if all_clips.is_empty() {
            self.status_message =
                "Dışa aktarılacak metin veya altyazı klibi bulunamadı".to_string();
            return;
        }

        let entries = trim_core::clips_to_subtitle_entries(&all_clips);
        let srt_text = trim_core::export_srt(&entries);
        let default_name = format!("{}_altyazi.srt", self.project.settings.name);

        let config =
            FileDialogConfig::save_file("Altyazıları SRT Olarak Kaydet", Some(default_name))
                .add_filter("SubRip Altyazı (*.srt)", &["srt"]);
        self.file_dialog_bridge.open(
            FileDialogRequest::ExportSubtitles {
                srt_text,
                entry_count: entries.len(),
            },
            config,
        );
    }

    pub fn export_edl_dialog(&mut self) {
        let edl_text = trim_export::generate_edl(&self.project);
        let default_name = format!("{}.edl", self.project.settings.name);

        let config = FileDialogConfig::save_file(
            "Projeyi CMX 3600 EDL Olarak Dışa Aktar",
            Some(default_name),
        )
        .add_filter("CMX 3600 EDL (*.edl)", &["edl"]);
        self.file_dialog_bridge
            .open(FileDialogRequest::ExportEdl { edl_text }, config);
    }

    pub fn export_fcpxml_dialog(&mut self) {
        let xml_text = trim_export::generate_fcpxml(&self.project);
        let default_name = format!("{}.xml", self.project.settings.name);

        let config = FileDialogConfig::save_file(
            "Projeyi Final Cut Pro XML (Premiere/DaVinci) Olarak Dışa Aktar",
            Some(default_name),
        )
        .add_filter("Final Cut Pro XML (*.xml)", &["xml"]);
        self.file_dialog_bridge
            .open(FileDialogRequest::ExportFcpxml { xml_text }, config);
    }

    pub fn import_interchange_dialog(&mut self) {
        let config = FileDialogConfig::pick_file("Projeyi İçe Aktar (FCPXML / EDL)")
            .add_filter("NLE Değişim Dosyaları (*.xml, *.edl)", &["xml", "edl"])
            .add_filter("Final Cut Pro XML (*.xml)", &["xml"])
            .add_filter("CMX 3600 EDL (*.edl)", &["edl"]);
        self.file_dialog_bridge
            .open(FileDialogRequest::ImportInterchange, config);
    }

    pub fn import_interchange_from_path(&mut self, path: &std::path::Path) {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("import.xml");
                match trim_export::import_interchange(&content, file_name) {
                    Ok(imported_project) => {
                        self.project = imported_project;
                        self.undo_stack.clear();
                        self.last_saved_undo_len = 0;
                        self.canvas_state.selected_clip_pairs.clear();
                        self.current_time = 0.0;
                        self.media_bin_textures.clear();
                        self.video_engine.clear_cache();
                        self.audio_mixer.clear_cache();
                        self.audio_player.stop();
                        self.is_playing = false;
                        self.monitor_texture = None;
                        self.monitor_gpu_texture_id = None;
                        self.source_state.media_id = None;
                        self.source_state.texture = None;
                        self.source_state.is_playing = false;
                        self.resolve_default_target_tracks();
                        self.autosave_mgr.cleanup(&self.project);
                        for item in self.project.media_pool.values() {
                            if item.media_type == MediaType::Audio || item.audio_channels > 0 {
                                self.audio_mixer.preload_audio_async(&item.file_path);
                            }
                        }
                        self.welcome_dialog.close();
                        let clip_count: usize = self
                            .project
                            .timeline
                            .tracks
                            .iter()
                            .map(|t| t.clips.len())
                            .sum();
                        self.status_message = format!(
                            "Proje başarıyla içe aktarıldı: {} kanal, {} klip ({})",
                            self.project.timeline.tracks.len(),
                            clip_count,
                            file_name
                        );
                    }
                    Err(e) => {
                        self.status_message = format!("İçe aktarma hatası: {}", e);
                    }
                }
            }
            Err(e) => {
                self.status_message = format!("Dosya okunamadı: {}", e);
            }
        }
    }

    /// Non-blocking poll of any active background OS file dialog.
    pub fn poll_file_dialogs(&mut self, ctx: &egui::Context) {
        let Some(resp) = self.file_dialog_bridge.poll() else {
            if self.file_dialog_bridge.is_pending() {
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            }
            return;
        };

        ctx.request_repaint();

        let Some(path) = resp.path else {
            return;
        };

        match resp.request {
            FileDialogRequest::OpenProject => {
                self.load_project_from_path(&path);
            }
            FileDialogRequest::SaveProject { after_save } => {
                match save_project(&mut self.project, path.clone()) {
                    Ok(()) => {
                        self.last_saved_undo_len = self.undo_stack.len();
                        self.project.is_dirty = false;
                        self.autosave_mgr.cleanup(&self.project);
                        self.status_message = format!("Proje kaydedildi: {:?}", path);
                        if let Some(pending) = after_save {
                            self.execute_pending_close_action(ctx, pending);
                        }
                    }
                    Err(e) => {
                        self.status_message = format!("Kayıt hatası: {}", e);
                    }
                }
            }
            FileDialogRequest::SaveProjectAs => {
                match save_project(&mut self.project, path.clone()) {
                    Ok(()) => {
                        self.last_saved_undo_len = self.undo_stack.len();
                        self.project.is_dirty = false;
                        self.autosave_mgr.cleanup(&self.project);
                        self.status_message = format!("Proje kaydedildi: {:?}", path);
                    }
                    Err(e) => {
                        self.status_message = format!("Kayıt hatası: {}", e);
                    }
                }
            }
            FileDialogRequest::BundleProject {
                default_folder_name,
            } => {
                let target_dir = path.join(&default_folder_name);
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
            FileDialogRequest::ImportSubtitles => {
                self.import_subtitles_from_path(&path);
            }
            FileDialogRequest::ExportSubtitles {
                srt_text,
                entry_count,
            } => match std::fs::write(&path, srt_text) {
                Ok(()) => {
                    self.status_message =
                        format!("{} adet altyazı dışa aktarıldı: {:?}", entry_count, path);
                }
                Err(e) => {
                    self.status_message = format!("Altyazı kayıt hatası: {}", e);
                }
            },
            FileDialogRequest::ExportEdl { edl_text } => match std::fs::write(&path, edl_text) {
                Ok(()) => {
                    self.status_message = format!("EDL başarıyla dışa aktarıldı: {:?}", path);
                }
                Err(e) => {
                    self.status_message = format!("EDL kayıt hatası: {}", e);
                }
            },
            FileDialogRequest::ExportFcpxml { xml_text } => match std::fs::write(&path, xml_text) {
                Ok(()) => {
                    self.status_message = format!("FCPXML başarıyla dışa aktarıldı: {:?}", path);
                }
                Err(e) => {
                    self.status_message = format!("FCPXML kayıt hatası: {}", e);
                }
            },
            FileDialogRequest::ImportInterchange => {
                self.import_interchange_from_path(&path);
            }
            FileDialogRequest::ImportMedia => {
                self.import_media_file(path);
            }
            FileDialogRequest::SelectExportOutput => {
                self.export_dialog.output_path = path.to_string_lossy().to_string();
            }
            FileDialogRequest::SelectCustomLut { clip_id } => {
                for track in &mut self.project.timeline.tracks {
                    if let Some(clip) = track.get_clip_by_id_mut(&clip_id) {
                        clip.custom_lut_path = Some(path.to_string_lossy().to_string());
                        self.project.mark_dirty();
                        self.last_rendered_dirty = true;
                        break;
                    }
                }
            }
            FileDialogRequest::RelinkFolder => {
                let count = self.project.relink_media_from_folder(&path);
                if count > 0 {
                    self.video_engine.clear_cache();
                    self.audio_mixer.clear_cache();
                    self.last_rendered_dirty = true;
                    self.status_message =
                        format!("{} medya dosyası başarıyla yeniden bağlandı.", count);
                    self.relink_dialog.refresh(&self.project);
                } else {
                    self.status_message =
                        "Seçilen klasörde eşleşen çevrimdışı medya bulunamadı.".to_string();
                }
            }
            FileDialogRequest::RelinkSingleMedia { media_id } => {
                let name = if let Some(item) = self.project.get_media_mut(&media_id) {
                    item.file_path = path.to_string_lossy().to_string();
                    Some(item.name.clone())
                } else {
                    None
                };
                if let Some(media_name) = name {
                    self.project.mark_dirty();
                    self.video_engine.clear_cache();
                    self.audio_mixer.clear_cache();
                    self.last_rendered_dirty = true;
                    self.status_message = format!("Medya yeniden bağlandı: {}", media_name);
                    self.relink_dialog.refresh(&self.project);
                }
            }
        }
    }

    pub fn relink_folder_dialog(&mut self) {
        let config = FileDialogConfig::pick_folder("Medyaları Yeniden Bağlamak İçin Klasör Seçin");
        self.file_dialog_bridge
            .open(FileDialogRequest::RelinkFolder, config);
    }

    pub fn relink_single_media_dialog(&mut self, media_id: impl Into<String>) {
        let media_id = media_id.into();
        let title = if let Some(m) = self.project.get_media(&media_id) {
            format!("'{}' İçin Dosya Seçin", m.name)
        } else {
            "Medya Dosyası Seçin".to_string()
        };
        let config = FileDialogConfig::pick_file(title);
        self.file_dialog_bridge
            .open(FileDialogRequest::RelinkSingleMedia { media_id }, config);
    }

    pub fn reset_to_new_project(&mut self) {
        self.project = Project::default();
        self.undo_stack.clear();
        self.last_saved_undo_len = 0;
        self.canvas_state.selected_clip_pairs.clear();
        self.current_time = 0.0;
        self.media_bin_textures.clear();
        self.video_engine.clear_cache();
        self.audio_mixer.clear_cache();
        self.audio_player.stop();
        self.is_playing = false;
        self.monitor_texture = None;
        self.monitor_gpu_texture_id = None;
        self.source_state.media_id = None;
        self.source_state.texture = None;
        self.source_state.is_playing = false;
        self.resolve_default_target_tracks();
        self.status_message = "Yeni boş proje oluşturuldu".to_string();
    }

    /// Determines whether the project has any unsaved modifications, clips, media, or undo steps.
    pub fn has_unsaved_changes(&self) -> bool {
        if self.project.is_dirty {
            return true;
        }
        if self.undo_stack.len() != self.last_saved_undo_len {
            return true;
        }
        if self.project.file_path.is_none() {
            let has_media = !self.project.media_pool.is_empty();
            let has_clips = self
                .project
                .timeline
                .tracks
                .iter()
                .any(|t| !t.clips.is_empty());
            let has_markers = !self.project.timeline.markers.is_empty();
            if has_media || has_clips || has_markers {
                return true;
            }
        }
        false
    }

    pub fn export_current_frame(&mut self) {
        let frame_arc = self.video_engine.last_rendered_frame.clone().or_else(|| {
            self.video_engine
                .render_timeline_frame(&self.project, self.current_time, false)
        });
        if let Some(frame) = frame_arc.as_ref() {
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
                    self.status_message = format!(
                        "Kare dışa aktarıldı ve havuza eklendi: {}",
                        out_path.display()
                    );
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
}
