//! Project loading, saving, bundling, subtitle import/export, and NLE timeline interchange (EDL/FCPXML).

use crate::app::TrimApp;
use crate::dialogs::PendingCloseAction;
use crate::file_dialog::{FileDialogConfig, FileDialogRequest};
use std::path::Path;
use trim_core::media::MediaType;
use trim_core::project::Project;
use trim_media::FFprobeAnalyzer;
use trim_project::{bundle_project, ensure_project_extension, load_project, save_project};

impl TrimApp {
    pub fn open_project_dialog(&mut self) {
        let config = FileDialogConfig::pick_file("triM. Projesi Aç").add_filter(
            "triM. Projeleri (*.trim, *.cutline)",
            &["trim", "cutline", "json"],
        );
        self.dialogs
            .file_dialog_bridge
            .open(FileDialogRequest::OpenProject, config);
    }

    pub fn load_project_from_path(&mut self, path: &Path) {
        match load_project(path) {
            Ok(proj) => {
                self.project = proj;
                self.undo_stack.clear();
                self.persistence.last_saved_undo_len = 0;
                self.canvas_state.selected_clip_pairs.clear();
                self.playback.current_time = 0.0;
                self.media.media_bin_textures.clear();
                self.media.video_engine.clear_cache();
                self.playback.audio_mixer.clear_cache();
                self.playback.audio_player.stop();
                self.playback.is_playing = false;
                self.monitor.monitor_texture = None;
                self.monitor.monitor_gpu_texture_id = None;
                self.monitor.source_state.media_id = None;
                self.monitor.source_state.texture = None;
                self.monitor.source_state.is_playing = false;
                self.monitor.source_state.clock_start = None;
                self.resolve_default_target_tracks();
                self.persistence.autosave_mgr.cleanup(&self.project);
                let mut probe_targets = Vec::new();
                for item in self.project.media_pool.values() {
                    if item.media_type == MediaType::Video && !item.has_audio() {
                        probe_targets.push((item.id.clone(), item.file_path.clone()));
                    } else if item.has_audio() {
                        self.playback
                            .audio_mixer
                            .preload_audio_async(&item.file_path);
                    }
                }
                if !probe_targets.is_empty() {
                    let tx = self.bg_tasks.tx.clone();
                    std::thread::spawn(move || {
                        for (media_id, file_path) in probe_targets {
                            if let Ok(probed) = FFprobeAnalyzer::analyze(&file_path) {
                                if probed.has_audio() {
                                    let _ = tx.send(
                                        crate::app::BackgroundTaskMsg::ProjectAudioProbed {
                                            media_id,
                                            codec: probed.audio_codec,
                                            channels: probed.audio_channels,
                                            sample_rate: probed.audio_sample_rate,
                                            file_path,
                                        },
                                    );
                                }
                            }
                        }
                    });
                }
                self.dialogs.welcome_dialog.close();
                let offline = self.project.get_offline_media_ids();
                if !offline.is_empty() {
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => format!(
                            "Proje yüklendi: {:?} ({} adet çevrimdışı medya tespit edildi)",
                            path,
                            offline.len()
                        ),
                        crate::i18n::Language::English => format!(
                            "Project loaded: {:?} ({} offline media file(s) detected)",
                            path,
                            offline.len()
                        ),
                    };
                    self.dialogs.relink_dialog.open(&self.project);
                } else {
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => format!("Proje yüklendi: {:?}", path),
                        crate::i18n::Language::English => format!("Project loaded: {:?}", path),
                    };
                }
            }
            Err(e) => {
                self.status_message = match self.lang() {
                    crate::i18n::Language::Turkish => format!("Proje yüklenemedi: {}", e),
                    crate::i18n::Language::English => format!("Failed to load project: {}", e),
                };
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
            let target = ensure_project_extension(fp);
            match save_project(&mut self.project, target.clone()) {
                Ok(()) => {
                    self.persistence.last_saved_undo_len = self.undo_stack.len();
                    self.project.is_dirty = false;
                    self.persistence.autosave_mgr.cleanup(&self.project);
                    self.dialogs
                        .welcome_dialog
                        .add_recent(&self.project.settings.name, &target.to_string_lossy());
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => format!("Proje kaydedildi: {:?}", target),
                        crate::i18n::Language::English => format!("Project saved: {:?}", target),
                    };
                    true
                }
                Err(e) => {
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => format!("Kayıt hatası: {}", e),
                        crate::i18n::Language::English => format!("Save error: {}", e),
                    };
                    false
                }
            }
        } else {
            let default_name = format!("{}.trim", self.project.settings.name);
            let config = FileDialogConfig::save_file("Projeyi Kaydet", Some(default_name))
                .add_filter("triM. Projesi (*.trim)", &["trim"]);
            self.dialogs
                .file_dialog_bridge
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
            let default_name = format!("{}.trim", self.project.settings.name);
            let config = FileDialogConfig::save_file("Projeyi Kaydet", Some(default_name))
                .add_filter("triM. Projesi (*.trim)", &["trim"]);
            self.dialogs.file_dialog_bridge.open(
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
                self.persistence.confirmed_exit = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            PendingCloseAction::NewProject => self.reset_to_new_project(),
            PendingCloseAction::OpenProject => self.open_project_dialog(),
            PendingCloseAction::OpenPath(path) => self.open_path(&path),
        }
    }

    pub fn save_project_as_dialog(&mut self) -> bool {
        let default_name = format!("{}.trim", self.project.settings.name);
        let config = FileDialogConfig::save_file("Projeyi Farklı Kaydet", Some(default_name))
            .add_filter("triM. Projesi (*.trim)", &["trim"]);
        self.dialogs
            .file_dialog_bridge
            .open(FileDialogRequest::SaveProjectAs, config)
    }

    pub fn bundle_project_dialog(&mut self) {
        let default_folder_name = format!("{}_Paket", self.project.settings.name);
        let config =
            FileDialogConfig::pick_folder("Projeyi ve Medyaları Paketleyeceğiniz Klasörü Seçin");
        self.dialogs.file_dialog_bridge.open(
            FileDialogRequest::BundleProject {
                default_folder_name,
            },
            config,
        );
    }

    pub fn import_subtitles_dialog(&mut self) {
        let config = FileDialogConfig::pick_file("Altyazı Dosyası Seç (.srt, .vtt)")
            .add_filter("Altyazı Dosyaları (*.srt, *.vtt)", &["srt", "vtt"]);
        self.dialogs
            .file_dialog_bridge
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
                            self.set_status(
                                "Altyazı dosyasında geçerli satır bulunamadı",
                                "No valid subtitle entries found in file",
                            );
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
                            self.status_message = match self.lang() {
                                crate::i18n::Language::Turkish => format!(
                                    "{} adet altyazı zaman çizgisine başarıyla eklendi",
                                    count
                                ),
                                crate::i18n::Language::English => {
                                    format!("Added {} subtitle(s) to timeline", count)
                                }
                            };
                        }
                    }
                    Err(e) => {
                        self.status_message = match self.lang() {
                            crate::i18n::Language::Turkish => {
                                format!("Altyazı ayrıştırma hatası: {}", e)
                            }
                            crate::i18n::Language::English => {
                                format!("Subtitle parse error: {}", e)
                            }
                        };
                    }
                }
            }
            Err(e) => {
                self.status_message = match self.lang() {
                    crate::i18n::Language::Turkish => format!("Dosya okunamadı: {}", e),
                    crate::i18n::Language::English => format!("Failed to read file: {}", e),
                };
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
            self.set_status(
                "Dışa aktarılacak metin veya altyazı klibi bulunamadı",
                "No text or subtitle clips found to export",
            );
            return;
        }

        let entries = trim_core::clips_to_subtitle_entries(&all_clips);
        let srt_text = trim_core::export_srt(&entries);
        let default_name = format!("{}_altyazi.srt", self.project.settings.name);

        let config =
            FileDialogConfig::save_file("Altyazıları SRT Olarak Kaydet", Some(default_name))
                .add_filter("SubRip Altyazı (*.srt)", &["srt"]);
        self.dialogs.file_dialog_bridge.open(
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
        self.dialogs
            .file_dialog_bridge
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
        self.dialogs
            .file_dialog_bridge
            .open(FileDialogRequest::ExportFcpxml { xml_text }, config);
    }

    pub fn import_interchange_dialog(&mut self) {
        let config = FileDialogConfig::pick_file("Projeyi İçe Aktar (FCPXML / EDL)")
            .add_filter("NLE Değişim Dosyaları (*.xml, *.edl)", &["xml", "edl"])
            .add_filter("Final Cut Pro XML (*.xml)", &["xml"])
            .add_filter("CMX 3600 EDL (*.edl)", &["edl"]);
        self.dialogs
            .file_dialog_bridge
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
                        self.persistence.last_saved_undo_len = 0;
                        self.canvas_state.selected_clip_pairs.clear();
                        self.playback.current_time = 0.0;
                        self.media.media_bin_textures.clear();
                        self.media.video_engine.clear_cache();
                        self.playback.audio_mixer.clear_cache();
                        self.playback.audio_player.stop();
                        self.playback.is_playing = false;
                        self.monitor.monitor_texture = None;
                        self.monitor.monitor_gpu_texture_id = None;
                        self.monitor.source_state.media_id = None;
                        self.monitor.source_state.texture = None;
                        self.monitor.source_state.is_playing = false;
                        self.monitor.source_state.clock_start = None;
                        self.resolve_default_target_tracks();
                        self.persistence.autosave_mgr.cleanup(&self.project);
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
                                self.playback
                                    .audio_mixer
                                    .preload_audio_async(&item.file_path);
                            }
                        }
                        self.dialogs.welcome_dialog.close();
                        let clip_count: usize = self
                            .project
                            .timeline
                            .tracks
                            .iter()
                            .map(|t| t.clips.len())
                            .sum();
                        self.status_message = match self.lang() {
                            crate::i18n::Language::Turkish => format!(
                                "Proje başarıyla içe aktarıldı: {} kanal, {} klip ({})",
                                self.project.timeline.tracks.len(),
                                clip_count,
                                file_name
                            ),
                            crate::i18n::Language::English => format!(
                                "Project imported: {} track(s), {} clip(s) ({})",
                                self.project.timeline.tracks.len(),
                                clip_count,
                                file_name
                            ),
                        };
                    }
                    Err(e) => {
                        self.status_message = match self.lang() {
                            crate::i18n::Language::Turkish => {
                                format!("İçe aktarma hatası: {}", e)
                            }
                            crate::i18n::Language::English => format!("Import error: {}", e),
                        };
                    }
                }
            }
            Err(e) => {
                self.status_message = match self.lang() {
                    crate::i18n::Language::Turkish => format!("Dosya okunamadı: {}", e),
                    crate::i18n::Language::English => format!("Failed to read file: {}", e),
                };
            }
        }
    }

    /// Non-blocking poll of any active background OS file dialog.
    pub fn poll_file_dialogs(&mut self, ctx: &egui::Context) {
        let Some(resp) = self.dialogs.file_dialog_bridge.poll() else {
            if self.dialogs.file_dialog_bridge.is_pending() {
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
                let target = ensure_project_extension(&path);
                match save_project(&mut self.project, &target) {
                    Ok(()) => {
                        self.persistence.last_saved_undo_len = self.undo_stack.len();
                        self.project.is_dirty = false;
                        self.persistence.autosave_mgr.cleanup(&self.project);
                        self.dialogs
                            .welcome_dialog
                            .add_recent(&self.project.settings.name, &target.to_string_lossy());
                        self.status_message = match self.lang() {
                            crate::i18n::Language::Turkish => {
                                format!("Proje kaydedildi: {:?}", target)
                            }
                            crate::i18n::Language::English => {
                                format!("Project saved: {:?}", target)
                            }
                        };
                        if let Some(pending) = after_save {
                            self.execute_pending_close_action(ctx, pending);
                        }
                    }
                    Err(e) => {
                        self.status_message = match self.lang() {
                            crate::i18n::Language::Turkish => format!("Kayıt hatası: {}", e),
                            crate::i18n::Language::English => format!("Save error: {}", e),
                        };
                    }
                }
            }
            FileDialogRequest::SaveProjectAs => {
                let target = ensure_project_extension(&path);
                match save_project(&mut self.project, &target) {
                    Ok(()) => {
                        self.persistence.last_saved_undo_len = self.undo_stack.len();
                        self.project.is_dirty = false;
                        self.persistence.autosave_mgr.cleanup(&self.project);
                        self.dialogs
                            .welcome_dialog
                            .add_recent(&self.project.settings.name, &target.to_string_lossy());
                        self.status_message = match self.lang() {
                            crate::i18n::Language::Turkish => {
                                format!("Proje kaydedildi: {:?}", target)
                            }
                            crate::i18n::Language::English => {
                                format!("Project saved: {:?}", target)
                            }
                        };
                    }
                    Err(e) => {
                        self.status_message = match self.lang() {
                            crate::i18n::Language::Turkish => format!("Kayıt hatası: {}", e),
                            crate::i18n::Language::English => format!("Save error: {}", e),
                        };
                    }
                }
            }
            FileDialogRequest::BundleProject {
                default_folder_name,
            } => {
                let target_dir = path.join(&default_folder_name);
                let proj_clone = self.project.clone();
                let tx = self.bg_tasks.tx.clone();
                self.bg_tasks.active_count += 1;
                self.bg_tasks
                    .cancel_flag
                    .store(false, std::sync::atomic::Ordering::Release);
                self.set_status(
                    "Proje arka planda paketleniyor...",
                    "Bundling project in background...",
                );
                std::thread::spawn(move || {
                    let res = bundle_project(&proj_clone, &target_dir).map_err(|e| e.to_string());
                    let _ = tx.send(crate::app::BackgroundTaskMsg::BundleDone {
                        target_dir,
                        result: res,
                    });
                });
            }
            FileDialogRequest::ImportSubtitles => {
                self.import_subtitles_from_path(&path);
            }
            FileDialogRequest::ExportSubtitles {
                srt_text,
                entry_count,
            } => match std::fs::write(&path, srt_text) {
                Ok(()) => {
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => {
                            format!("{} adet altyazı dışa aktarıldı: {:?}", entry_count, path)
                        }
                        crate::i18n::Language::English => {
                            format!("Exported {} subtitle(s): {:?}", entry_count, path)
                        }
                    };
                }
                Err(e) => {
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => {
                            format!("Altyazı kayıt hatası: {}", e)
                        }
                        crate::i18n::Language::English => {
                            format!("Subtitle save error: {}", e)
                        }
                    };
                }
            },
            FileDialogRequest::ExportEdl { edl_text } => match std::fs::write(&path, edl_text) {
                Ok(()) => {
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => {
                            format!("EDL başarıyla dışa aktarıldı: {:?}", path)
                        }
                        crate::i18n::Language::English => {
                            format!("EDL exported successfully: {:?}", path)
                        }
                    };
                }
                Err(e) => {
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => format!("EDL kayıt hatası: {}", e),
                        crate::i18n::Language::English => format!("EDL save error: {}", e),
                    };
                }
            },
            FileDialogRequest::ExportFcpxml { xml_text } => match std::fs::write(&path, xml_text) {
                Ok(()) => {
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => {
                            format!("FCPXML başarıyla dışa aktarıldı: {:?}", path)
                        }
                        crate::i18n::Language::English => {
                            format!("FCPXML exported successfully: {:?}", path)
                        }
                    };
                }
                Err(e) => {
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => format!("FCPXML kayıt hatası: {}", e),
                        crate::i18n::Language::English => format!("FCPXML save error: {}", e),
                    };
                }
            },
            FileDialogRequest::ImportInterchange => {
                self.import_interchange_from_path(&path);
            }
            FileDialogRequest::ImportMedia => {
                self.import_media_file(path);
            }
            FileDialogRequest::SelectExportOutput => {
                self.dialogs.export_dialog.output_path = path.to_string_lossy().to_string();
            }
            FileDialogRequest::SelectCustomLut { clip_id } => {
                for track in &mut self.project.timeline.tracks {
                    if let Some(clip) = track.get_clip_by_id_mut(&clip_id) {
                        clip.custom_lut_path = Some(path.to_string_lossy().to_string());
                        self.project.mark_dirty();
                        self.media.last_rendered_dirty = true;
                        break;
                    }
                }
            }
            FileDialogRequest::SelectCustomFont { clip_id } => {
                crate::theme::register_custom_font_file(ctx, &path);
                let font_path_str = path.to_string_lossy().to_string();
                let font_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Özel Font")
                    .to_string();
                for track in &mut self.project.timeline.tracks {
                    if let Some(clip) = track.get_clip_by_id_mut(&clip_id) {
                        if let Some(txt) = &mut clip.text_overlay {
                            txt.font_family = font_path_str;
                            self.project.mark_dirty();
                            self.media.last_rendered_dirty = true;
                            self.status_message = match self.lang() {
                                crate::i18n::Language::Turkish => {
                                    format!("🔤 Özel font dosyası yüklendi: {}", font_name)
                                }
                                crate::i18n::Language::English => {
                                    format!("🔤 Custom font loaded: {}", font_name)
                                }
                            };
                        }
                        break;
                    }
                }
            }
            FileDialogRequest::ExportCubeLut {
                cube_text,
                clip_name,
            } => {
                let mut target = path;
                if target.extension().is_none() {
                    target.set_extension("cube");
                }
                match std::fs::write(&target, cube_text) {
                    Ok(()) => {
                        self.status_message = match self.lang() {
                            crate::i18n::Language::Turkish => format!(
                                "💾 '{}' renk ayarı 3D .cube LUT olarak kaydedildi: {}",
                                clip_name,
                                target.display()
                            ),
                            crate::i18n::Language::English => format!(
                                "💾 '{}' color grade exported as 3D .cube LUT: {}",
                                clip_name,
                                target.display()
                            ),
                        };
                    }
                    Err(e) => {
                        self.status_message = match self.lang() {
                            crate::i18n::Language::Turkish => {
                                format!("⚠ .cube LUT dosyası kaydedilemedi: {e}")
                            }
                            crate::i18n::Language::English => {
                                format!("⚠ Failed to save .cube LUT file: {e}")
                            }
                        };
                    }
                }
            }
            FileDialogRequest::RelinkFolder => {
                let count = self.project.relink_media_from_folder(&path);
                if count > 0 {
                    self.media.video_engine.clear_cache();
                    self.playback.audio_mixer.clear_cache();
                    self.media.last_rendered_dirty = true;
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => {
                            format!("{} medya dosyası başarıyla yeniden bağlandı.", count)
                        }
                        crate::i18n::Language::English => {
                            format!("Relinked {} media file(s) successfully.", count)
                        }
                    };
                    self.dialogs.relink_dialog.refresh(&self.project);
                } else {
                    self.set_status(
                        "Seçilen klasörde eşleşen çevrimdışı medya bulunamadı.",
                        "No matching offline media found in selected folder.",
                    );
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
                    self.media.video_engine.clear_cache();
                    self.playback.audio_mixer.clear_cache();
                    self.media.last_rendered_dirty = true;
                    self.status_message = match self.lang() {
                        crate::i18n::Language::Turkish => {
                            format!("Medya yeniden bağlandı: {}", media_name)
                        }
                        crate::i18n::Language::English => {
                            format!("Media relinked: {}", media_name)
                        }
                    };
                    self.dialogs.relink_dialog.refresh(&self.project);
                }
            }
            FileDialogRequest::ExportCurrentFrame {
                frame,
                default_name,
            } => {
                if frame.save(&path).is_ok() {
                    let filename = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&default_name)
                        .to_string();
                    if let Ok(mut item) = FFprobeAnalyzer::analyze(&path) {
                        item.name = filename;
                        item.duration = 5.0;
                        self.project.add_media(item);
                        self.status_message = match self.lang() {
                            crate::i18n::Language::Turkish => {
                                format!("Kare dışa aktarıldı ve havuza eklendi: {}", path.display())
                            }
                            crate::i18n::Language::English => {
                                format!("Frame exported and added to bin: {}", path.display())
                            }
                        };
                    } else {
                        self.status_message = match self.lang() {
                            crate::i18n::Language::Turkish => {
                                format!("Kare kaydedildi: {}", path.display())
                            }
                            crate::i18n::Language::English => {
                                format!("Frame saved: {}", path.display())
                            }
                        };
                    }
                } else {
                    self.set_status("Kare kaydedilemedi", "Failed to save frame");
                }
            }
        }
    }

    pub fn relink_folder_dialog(&mut self) {
        let config = FileDialogConfig::pick_folder("Medyaları Yeniden Bağlamak İçin Klasör Seçin");
        self.dialogs
            .file_dialog_bridge
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
        self.dialogs
            .file_dialog_bridge
            .open(FileDialogRequest::RelinkSingleMedia { media_id }, config);
    }

    pub fn reset_to_new_project(&mut self) {
        self.project = Project::default();
        self.undo_stack.clear();
        self.persistence.last_saved_undo_len = 0;
        self.canvas_state.selected_clip_pairs.clear();
        self.playback.current_time = 0.0;
        self.media.media_bin_textures.clear();
        self.media.video_engine.clear_cache();
        self.playback.audio_mixer.clear_cache();
        self.playback.audio_player.stop();
        self.playback.is_playing = false;
        self.monitor.monitor_texture = None;
        self.monitor.monitor_gpu_texture_id = None;
        self.monitor.source_state.media_id = None;
        self.monitor.source_state.texture = None;
        self.monitor.source_state.is_playing = false;
        self.resolve_default_target_tracks();
        self.set_status("Yeni boş proje oluşturuldu", "New empty project created");
    }

    /// Determines whether the project has any unsaved modifications, clips, media, or undo steps.
    pub fn has_unsaved_changes(&self) -> bool {
        if self.project.is_dirty {
            return true;
        }
        if self.undo_stack.len() != self.persistence.last_saved_undo_len {
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
        let frame_arc = self
            .media
            .video_engine
            .last_rendered_frame
            .clone()
            .or_else(|| {
                self.media.video_engine.render_timeline_frame(
                    &self.project,
                    self.playback.current_time,
                    false,
                )
            });
        if let Some(frame) = frame_arc {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let filename = format!(
                "kare_{}_{:06}.png",
                ts,
                (self.playback.current_time * 1000.0) as u64
            );
            let config =
                FileDialogConfig::save_file("Kareyi PNG Olarak Kaydet", Some(filename.clone()))
                    .add_filter("PNG Görüntüsü (*.png)", &["png"]);
            self.dialogs.file_dialog_bridge.open(
                FileDialogRequest::ExportCurrentFrame {
                    frame,
                    default_name: filename,
                },
                config,
            );
        } else {
            self.set_status(
                "Dışa aktarılacak aktif kare yok",
                "No active frame to export",
            );
        }
    }
}
