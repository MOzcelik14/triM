//! Project loading, saving, bundling, subtitle import/export, and NLE timeline interchange (EDL/FCPXML).

use crate::app::TrimApp;
use std::path::Path;
use trim_core::media::MediaType;
use trim_core::project::Project;
use trim_media::FFprobeAnalyzer;
use trim_project::{bundle_project, load_project, save_project};

impl TrimApp {
    pub fn open_project_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("triM. Projesi Aç")
            .add_filter(
                "triM. Projeleri (*.trim, *.cutline)",
                &["trim", "cutline", "json"],
            )
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
                    self.resolve_default_target_tracks();
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
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext == "trim" || ext == "cutline" || ext == "json" {
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
        } else if let Some(path) = rfd::FileDialog::new()
            .set_title("Projeyi Kaydet")
            .add_filter("triM. Projesi (*.trim)", &["trim"])
            .save_file()
        {
            match save_project(&mut self.project, path) {
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
                    self.last_saved_undo_len = self.undo_stack.len();
                    self.project.is_dirty = false;
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
                                self.status_message = format!(
                                    "{} adet altyazı zaman çizgisine başarıyla eklendi",
                                    count
                                );
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

        if let Some(path) = rfd::FileDialog::new()
            .set_title("Altyazıları SRT Olarak Kaydet")
            .set_file_name(&default_name)
            .add_filter("SubRip Altyazı (*.srt)", &["srt"])
            .save_file()
        {
            match std::fs::write(&path, srt_text) {
                Ok(()) => {
                    self.status_message =
                        format!("{} adet altyazı dışa aktarıldı: {:?}", entries.len(), path);
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
