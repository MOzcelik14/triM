use super::{BackgroundTaskMsg, TrimApp};
use crate::i18n::Language;
use std::time::Instant;
use trim_commands::commands::*;
use trim_export::queue::RenderQueueEvent;
use trim_export::runner::ExportMessage;

impl TrimApp {
    pub(super) fn poll_background_work(&mut self, ctx: &egui::Context) {
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

        // Poll background OS file dialogs non-blockingly and reap finished helper processes
        self.poll_file_dialogs(ctx);
        self.helper_processes.poll_reap();

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
                            Err(e) => {
                                self.status_message = match self.lang() {
                                    Language::Turkish => format!(
                                        "Dışa aktarma hatası: {} (FFmpeg kurulumunu, medya dosyalarını ve hedef disk alanı/izinlerini kontrol edin)",
                                        e
                                    ),
                                    Language::English => format!(
                                        "Export failed: {} (check FFmpeg installation, source media, and target disk space/permissions)",
                                        e
                                    ),
                                };
                            }
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
                                if let Err(e) =
                                    self.undo_stack.push(cmd, &mut self.project.timeline)
                                {
                                    self.status_message = format!("⚠ Sahne kesimi başarısız: {e}");
                                } else {
                                    self.project.mark_dirty();
                                    self.status_message = format!(
                                        "Akıllı Kesim Tamamlandı: {} yeni sahne algılandı ve bölündü",
                                        count
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        self.status_message = match self.lang() {
                            Language::Turkish => format!(
                                "Sahne algılama başarısız: {} (FFmpeg ve medya dosyasının okunabilirliğini kontrol edin)",
                                e
                            ),
                            Language::English => format!(
                                "Scene detection failed: {} (check FFmpeg and media file readability)",
                                e
                            ),
                        };
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
                            if let Err(e) =
                                self.undo_stack.push(macro_cmd, &mut self.project.timeline)
                            {
                                self.status_message = format!("⚠ Altyazı eklenemedi: {e}");
                            } else {
                                self.project.mark_dirty();
                                self.status_message = format!(
                                    "{} adet altyazı başarıyla oluşturuldu ve eklendi",
                                    count
                                );
                            }
                        } else {
                            self.status_message =
                                format!("{} adet altyazı başarıyla oluşturuldu ve eklendi", count);
                        }
                        self.transcribe_dialog.close();
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
                        self.status_message = match self.lang() {
                            Language::Turkish => format!(
                                "Paketleme hatası: {} (hedef klasör izinlerini ve boş disk alanını kontrol edin)",
                                e
                            ),
                            Language::English => format!(
                                "Bundle failed: {} (check target folder permissions and available disk space)",
                                e
                            ),
                        };
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
    }
}
