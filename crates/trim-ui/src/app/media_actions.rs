//! Media pool ingestion, 720p proxy generation, voiceover recording, and smart AI/video cuts.

use crate::app::{ActiveVoiceover, TrimApp};
use crate::file_dialog::{FileDialogConfig, FileDialogRequest};
use egui::{ColorImage, TextureOptions};
use std::sync::mpsc::channel;
use trim_commands::{AddClipCommand, MoveMultipleClipsCommand};
use trim_core::clip::Clip;
use trim_core::media::MediaType;
use trim_media::FFprobeAnalyzer;

impl TrimApp {
    pub fn toggle_proxy(&mut self) {
        self.use_proxies = !self.use_proxies;
        self.video_engine.use_proxies = self.use_proxies;
        self.video_engine.clear_cache();
        self.last_rendered_dirty = true;
        let state_str = if self.use_proxies {
            "AÇIK (720p H.264)"
        } else {
            "KAPALI (Orijinal 4K/UHD)"
        };
        self.status_message = format!("⚡ Akıllı Proxy Önizleme: {}", state_str);
    }

    pub fn generate_proxy_for_media(&mut self, media_id: &str) {
        let media = match self.project.get_media(media_id) {
            Some(m) => m,
            None => return,
        };
        if media.media_type != MediaType::Video {
            self.status_message =
                "Proxy yalnızca video medyaları için oluşturulabilir.".to_string();
            return;
        }

        let src = std::path::PathBuf::from(&media.file_path);
        let out = trim_media::default_proxy_path_for(&src);

        if out.exists() {
            if let Some(m) = self.project.get_media_mut(media_id) {
                m.proxy_path = Some(out.to_string_lossy().to_string());
            }
            self.status_message = format!(
                "⚡ Proxy zaten mevcut: {}",
                out.file_name().unwrap().to_string_lossy()
            );
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

                let mut clip = Clip::new(
                    &media_id,
                    vo.start_timeline_time,
                    vo.start_timeline_time + dur,
                );
                clip.name = "Seslendirme Kaydı".to_string();

                let cmd = Box::new(AddClipCommand::new(vo.track_id.clone(), clip));
                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                self.project.mark_dirty();
                self.status_message = format!(
                    "🎤 Seslendirme başarıyla kaydedildi ({:.1}s) ve kanala eklendi!",
                    dur
                );
            }
            Err(e) => {
                self.status_message = format!("Ses kaydı kaydedilemedi: {}", e);
            }
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
        let config = FileDialogConfig::pick_file("Medya Dosyası Seç").add_filter(
            "Tüm Medya Dosyaları",
            &[
                "mp4", "mov", "mkv", "webm", "mp3", "wav", "aac", "png", "jpg", "jpeg",
            ],
        );
        self.file_dialog_bridge
            .open(FileDialogRequest::ImportMedia, config);
    }

    pub fn prompt_custom_lut(&mut self, clip_id: String) {
        let config = FileDialogConfig::pick_file("3D LUT Dosyası Seç (.cube)")
            .add_filter("3D LUT (.cube)", &["cube"]);
        self.file_dialog_bridge
            .open(FileDialogRequest::SelectCustomLut { clip_id }, config);
    }

    pub fn update_media_bin_thumbnails(&mut self, ctx: &egui::Context) {
        // 1. Poll completed background thumbnails
        while let Some((m_id, thumb_path)) = self.thumbnail_gen.poll_thumbnail() {
            if let std::collections::hash_map::Entry::Vacant(e) =
                self.media_bin_textures.entry(m_id.clone())
            {
                if let Ok(dyn_img) = image::open(&thumb_path) {
                    let rgba = dyn_img.to_rgba8();
                    let color_img = ColorImage::from_rgba_unmultiplied(
                        [rgba.width() as usize, rgba.height() as usize],
                        rgba.as_raw(),
                    );
                    let tex = ctx.load_texture(
                        format!("thumb_{}", m_id),
                        color_img,
                        TextureOptions::LINEAR,
                    );
                    e.insert(tex);
                    ctx.request_repaint();
                }
            }
        }

        // 2. Queue missing thumbnails to background worker (zero UI blocking)
        for (m_id, item) in &self.project.media_pool {
            if !self.media_bin_textures.contains_key(m_id) {
                self.thumbnail_gen.request_thumbnail(item);
            }
        }
    }

    pub fn smart_scene_cut_selected_clip(&mut self) {
        let selected = self.canvas_state.selected_clip_pairs.first().cloned();
        let Some((tr_id, cl_id)) = selected else {
            self.status_message =
                "Lütfen önce sahne tespiti yapılacak bir video klibi seçin".to_string();
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
                    self.status_message =
                        "Klipte belirgin sahne geçişi algılanmadı (Eşik: 0.40)".to_string();
                    return;
                }

                let timeline_cuts: Vec<f64> = source_cuts
                    .iter()
                    .map(|&s| clip.timeline_in + (s - clip.source_in) / clip.speed)
                    .filter(|&t| t > clip.timeline_in + 0.05 && t < clip.timeline_out - 0.05)
                    .collect();

                let count = timeline_cuts.len();
                if count == 0 {
                    self.status_message =
                        "Klip sınırları içinde yeni sahne kesimi bulunamadı".to_string();
                    return;
                }

                let cmd = Box::new(trim_commands::BatchSplitClipCommand::new(
                    &tr_id,
                    &cl_id,
                    timeline_cuts,
                ));
                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                self.project.mark_dirty();
                self.status_message = format!(
                    "Akıllı Kesim Tamamlandı: {} yeni sahne algılandı ve bölündü",
                    count
                );
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

        self.silence_dialog
            .open(tr_id, cl_id, name, clip.duration());
    }

    pub fn detect_silence_for_dialog(
        &mut self,
        track_id: &str,
        clip_id: &str,
        noise_db: f32,
        min_dur: f64,
    ) {
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

        match trim_media::detect_silence_regions(
            &media_path,
            source_in,
            source_out,
            noise_db,
            min_dur,
        ) {
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

    pub fn insert_source_to_timeline(&mut self, is_overwrite: bool) {
        let Some(ref m_id) = self.source_state.media_id.clone() else {
            self.status_message = "Kaynak monitöründe seçili klip yok".to_string();
            return;
        };
        let (media_duration, media_type, media_name, media_fps, media_w, media_h) =
            match self.project.get_media(m_id) {
                Some(m) => (
                    m.duration,
                    m.media_type,
                    m.name.clone(),
                    m.fps,
                    m.width,
                    m.height,
                ),
                None => return,
            };

        let start_in = self
            .source_state
            .mark_in
            .unwrap_or(0.0)
            .clamp(0.0, media_duration);
        let end_out = self
            .source_state
            .mark_out
            .unwrap_or(media_duration)
            .clamp(start_in, media_duration);
        let clip_dur = if media_type == MediaType::Image {
            5.0
        } else {
            (end_out - start_in).max(0.1)
        };

        let target_track_id = if media_type == MediaType::Audio {
            self.project
                .timeline
                .get_track_by_id(&self.target_audio_track)
                .map(|t| t.id.clone())
                .or_else(|| {
                    self.project
                        .timeline
                        .get_audio_tracks()
                        .first()
                        .map(|t| t.id.clone())
                })
        } else {
            self.project
                .timeline
                .get_track_by_id(&self.target_video_track)
                .map(|t| t.id.clone())
                .or_else(|| {
                    self.project
                        .timeline
                        .get_video_tracks()
                        .first()
                        .map(|t| t.id.clone())
                })
        };

        let Some(target_track_id) = target_track_id else {
            log::warn!("Hedef kanal bulunamadı (medya tipi: {:?})", media_type);
            self.status_message = "Hedef kanal bulunamadı".to_string();
            return;
        };

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
                        moves.push((
                            target_track_id.clone(),
                            c.id.clone(),
                            c.timeline_in,
                            c.timeline_in + clip_dur,
                        ));
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

        if self
            .project
            .timeline
            .tracks
            .iter()
            .all(|t| t.clips.is_empty())
        {
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
}
