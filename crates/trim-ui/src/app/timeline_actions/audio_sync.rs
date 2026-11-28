use crate::app::TrimApp;

impl TrimApp {
    /// Aligns selected clips across different tracks using cross-correlation of their audio waveforms.
    pub fn align_selected_clips_by_audio(&mut self) {
        if self.canvas_state.selected_clip_pairs.len() < 2 {
            self.set_status(
                "Ses ile hizalamak için farklı kanallarda en az 2 klip seçin",
                "Select at least 2 clips on different tracks to sync by audio",
            );
            return;
        }

        let (ref_tr_id, ref_cl_id) = self.canvas_state.selected_clip_pairs[0].clone();
        let Some(ref_clip) = self
            .project
            .timeline
            .get_track_by_id(&ref_tr_id)
            .and_then(|t| t.get_clip_by_id(&ref_cl_id))
            .cloned()
        else {
            return;
        };

        let Some(ref_media) = self.project.get_media(&ref_clip.media_id).cloned() else {
            self.set_status(
                "Referans klibin medya dosyası bulunamadı",
                "Reference clip media file not found",
            );
            return;
        };

        let pps = self.waveform_gen.points_per_second;
        let ref_wf = self
            .waveform_gen
            .get_waveform(&ref_media.file_path)
            .or_else(|| self.waveform_gen.generate_sync(&ref_media.file_path));
        let Some(ref_full_wf) = ref_wf else {
            self.set_status(
                "Referans klibin ses dalga formu çıkarılamadı",
                "Could not extract reference clip audio waveform",
            );
            return;
        };

        let ref_s_in = ((ref_clip.source_in * pps as f64) as usize).min(ref_full_wf.len());
        let ref_s_out =
            ((ref_clip.effective_source_out() * pps as f64) as usize).min(ref_full_wf.len());
        if ref_s_out <= ref_s_in + 4 {
            self.set_status(
                "Referans klipte yeterli ses verisi yok",
                "Reference clip does not have enough audio data",
            );
            return;
        }
        let ref_slice = &ref_full_wf[ref_s_in..ref_s_out];

        let mut moves: Vec<(String, String, f64, f64)> = Vec::new();
        let pairs = self.canvas_state.selected_clip_pairs[1..].to_vec();
        for (tgt_tr_id, tgt_cl_id) in pairs {
            if tgt_tr_id == ref_tr_id {
                continue;
            }
            let Some(tgt_clip) = self
                .project
                .timeline
                .get_track_by_id(&tgt_tr_id)
                .and_then(|t| t.get_clip_by_id(&tgt_cl_id))
                .cloned()
            else {
                continue;
            };
            let Some(tgt_media) = self.project.get_media(&tgt_clip.media_id).cloned() else {
                continue;
            };
            let Some(tgt_full_wf) = self
                .waveform_gen
                .get_waveform(&tgt_media.file_path)
                .or_else(|| self.waveform_gen.generate_sync(&tgt_media.file_path))
            else {
                continue;
            };
            let tgt_s_in = ((tgt_clip.source_in * pps as f64) as usize).min(tgt_full_wf.len());
            let tgt_s_out =
                ((tgt_clip.effective_source_out() * pps as f64) as usize).min(tgt_full_wf.len());
            if tgt_s_out <= tgt_s_in + 4 {
                continue;
            }
            let tgt_slice = &tgt_full_wf[tgt_s_in..tgt_s_out];

            if let Some(offset_sec) =
                trim_media::waveform::compute_audio_sync_offset(ref_slice, tgt_slice, pps)
            {
                let new_in = (ref_clip.timeline_in + offset_sec).max(0.0);
                if (new_in - tgt_clip.timeline_in).abs() > 1e-3 {
                    moves.push((tgt_tr_id, tgt_cl_id, tgt_clip.timeline_in, new_in));
                }
            }
        }

        if moves.is_empty() {
            self.set_status(
                "🎙️ Klipler zaten ses dalga formuna göre hizalı",
                "🎙️ Clips are already aligned by audio waveform",
            );
            return;
        }

        let count = moves.len();
        let cmds: Vec<Box<dyn trim_commands::Command>> = moves
            .into_iter()
            .map(|(tr_id, cl_id, old_in, new_in)| {
                Box::new(trim_commands::MoveClipCommand::new(
                    tr_id.clone(),
                    tr_id,
                    cl_id,
                    old_in,
                    new_in,
                )) as Box<dyn trim_commands::Command>
            })
            .collect();
        let cmd = Box::new(trim_commands::MacroCommand::new(
            "Ses Dalga Formuna Göre Hizala",
            cmds,
        ));
        if self
            .undo_stack
            .push(cmd, &mut self.project.timeline)
            .is_ok()
        {
            self.video_engine.clear_cache();
            self.audio_mixer.clear_cache();
            self.project.mark_dirty();
            self.status_message = match self.lang() {
                crate::i18n::Language::Turkish => format!(
                    "🎙️ {} klip ses dalga formuna göre otomatik senkronize edildi",
                    count
                ),
                crate::i18n::Language::English => {
                    format!("🎙️ Auto-synced {} clip(s) by audio waveform", count)
                }
            };
        }
    }

    pub(crate) fn track_mask_motion_for_clip(
        &mut self,
        track_id: &str,
        clip_id: &str,
        mask_index: usize,
    ) {
        let Some((media_id, source_in, duration, speed, old_masks)) = self
            .project
            .timeline
            .get_track_by_id(track_id)
            .and_then(|t| t.get_clip_by_id(clip_id))
            .map(|c| {
                (
                    c.media_id.clone(),
                    c.source_in,
                    c.duration(),
                    c.speed,
                    c.masks.clone(),
                )
            })
        else {
            return;
        };

        let Some(mask) = old_masks.get(mask_index) else {
            return;
        };
        let Some(media_path) = self
            .project
            .get_media(&media_id)
            .map(|m| m.file_path.clone())
        else {
            self.set_status(
                "⚠ Hareket takibi için medya kaynağı bulunamadı",
                "⚠ Media source not found for motion tracking",
            );
            return;
        };

        let init_cx = (mask.pos_x - 0.5) as f32;
        let init_cy = (mask.pos_y - 0.5) as f32;
        let sx = mask.scale_x as f32;
        let sy = mask.scale_y as f32;

        self.bg_task_active_count += 1;
        self.bg_task_cancel_flag
            .store(false, std::sync::atomic::Ordering::Release);
        let tx = self.bg_task_tx.clone();
        let tid = track_id.to_string();
        let cid = clip_id.to_string();
        self.set_status(
            "🎯 Maske hareket takibi arka planda çalışıyor...",
            "🎯 Mask motion tracking running in background...",
        );
        std::thread::spawn(move || {
            let res = trim_media::video::track_region_motion(
                &media_path,
                source_in,
                duration,
                speed,
                init_cx,
                init_cy,
                sx,
                sy,
            )
            .map_err(|e| e.to_string());
            let _ = tx.send(crate::app::BackgroundTaskMsg::MotionTrackDone {
                track_id: tid,
                clip_id: cid,
                mask_index,
                old_masks,
                result: res,
            });
        });
    }

    pub(crate) fn auto_duck_audio_clip(&mut self, track_id: &str, clip_id: &str) {
        let Some((tl_in, tl_out, base_vol, old_kfs)) = self
            .project
            .timeline
            .get_track_by_id(track_id)
            .and_then(|t| t.get_clip_by_id(clip_id))
            .map(|c| {
                (
                    c.timeline_in,
                    c.timeline_out,
                    c.volume.clamp(0.1, 3.0),
                    c.keyframes.get("volume").cloned().unwrap_or_default(),
                )
            })
        else {
            return;
        };

        let clip_dur = (tl_out - tl_in).max(0.0);
        if clip_dur <= 0.1 {
            return;
        }

        // Gather active dialogue/speech intervals from subtitle/caption clips first, then other unmuted clips
        let mut raw_intervals: Vec<(f64, f64)> = Vec::new();
        for tr in &self.project.timeline.tracks {
            if tr.muted {
                continue;
            }
            for other in &tr.clips {
                if other.id == clip_id || other.muted || other.text_overlay.is_none() {
                    continue;
                }
                let s = (other.timeline_in - tl_in).clamp(0.0, clip_dur);
                let e = (other.timeline_out - tl_in).clamp(0.0, clip_dur);
                if e - s > 0.05 {
                    raw_intervals.push((s, e));
                }
            }
        }

        if raw_intervals.is_empty() {
            for tr in &self.project.timeline.tracks {
                if tr.id == track_id || tr.muted {
                    continue;
                }
                for other in &tr.clips {
                    if other.muted || other.is_adjustment_layer() || other.volume <= 0.01 {
                        continue;
                    }
                    let s = (other.timeline_in - tl_in).clamp(0.0, clip_dur);
                    let e = (other.timeline_out - tl_in).clamp(0.0, clip_dur);
                    if e - s > 0.08 {
                        raw_intervals.push((s, e));
                    }
                }
            }
        }

        if raw_intervals.is_empty() {
            self.set_status(
                "ℹ Diğer kanallarda veya altyazılarda çakışan konuşma bölgesi bulunamadı",
                "ℹ No overlapping speech regions found on other tracks or subtitles",
            );
            return;
        }

        raw_intervals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut merged: Vec<(f64, f64)> = Vec::new();
        for (s, e) in raw_intervals {
            if let Some(last) = merged.last_mut() {
                if s <= last.1 + 0.35 {
                    last.1 = last.1.max(e);
                    continue;
                }
            }
            merged.push((s, e));
        }

        let duck_vol = (base_vol * 0.25).max(0.05);
        let mut new_kfs: Vec<trim_core::keyframe::Keyframe> = Vec::new();
        for (s, e) in &merged {
            let pre = (s - 0.22).max(0.0);
            let post = (e + 0.28).min(clip_dur);
            if pre > 0.01 {
                new_kfs.push(trim_core::keyframe::Keyframe::new(
                    pre,
                    base_vol,
                    trim_core::keyframe::Interpolation::EaseInOut,
                ));
            }
            new_kfs.push(trim_core::keyframe::Keyframe::new(
                *s,
                duck_vol,
                trim_core::keyframe::Interpolation::EaseInOut,
            ));
            new_kfs.push(trim_core::keyframe::Keyframe::new(
                *e,
                duck_vol,
                trim_core::keyframe::Interpolation::EaseInOut,
            ));
            if post < clip_dur - 0.01 {
                new_kfs.push(trim_core::keyframe::Keyframe::new(
                    post,
                    base_vol,
                    trim_core::keyframe::Interpolation::EaseInOut,
                ));
            }
        }
        new_kfs.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let cmd = Box::new(trim_commands::SetClipKeyframesCommand::new(
            track_id.to_string(),
            clip_id.to_string(),
            "volume".to_string(),
            old_kfs,
            new_kfs,
        ));
        if self
            .undo_stack
            .push(cmd, &mut self.project.timeline)
            .is_ok()
        {
            self.audio_mixer.clear_cache();
            self.project.mark_dirty();
            self.status_message = match self.lang() {
                crate::i18n::Language::Turkish => format!(
                    "🔉 {} konuşma bölgesi için otomatik ses kısma (Auto-Ducking) uygulandı",
                    merged.len()
                ),
                crate::i18n::Language::English => format!(
                    "🔉 Applied Auto-Ducking for {} speech region(s)",
                    merged.len()
                ),
            };
        }
    }

    /// Aligns selected clips across different tracks using their media container's SMPTE `timecode_start`.
    pub fn align_selected_clips_by_timecode(&mut self) {
        if self.canvas_state.selected_clip_pairs.len() < 2 {
            self.set_status(
                "Timecode ile hizalamak için en az 2 klip seçin",
                "Select at least 2 clips to sync by timecode",
            );
            return;
        }

        let mut items: Vec<(String, String, f64, f64)> = Vec::new();
        for (tr_id, cl_id) in &self.canvas_state.selected_clip_pairs {
            let Some(tr) = self.project.timeline.get_track_by_id(tr_id) else {
                continue;
            };
            if tr.locked {
                continue;
            }
            let Some(cl) = tr.get_clip_by_id(cl_id) else {
                continue;
            };
            let Some(media) = self.project.get_media(&cl.media_id) else {
                continue;
            };
            let Some(ref tc_str) = media.timecode_start else {
                continue;
            };
            let fps = if media.fps > 0.0 {
                media.fps
            } else {
                self.project.timeline.fps.max(1.0)
            };
            if let Some(base_tc_secs) = trim_core::time::timecode_to_seconds(tc_str, fps) {
                let clip_tc_secs = base_tc_secs + cl.source_in;
                items.push((tr_id.clone(), cl_id.clone(), cl.timeline_in, clip_tc_secs));
            }
        }

        if items.len() < 2 {
            self.set_status(
                "Seçili kliplerden en az 2'sinde geçerli SMPTE timecode bulunamadı",
                "Could not find valid SMPTE timecode on at least 2 selected clips",
            );
            return;
        }

        let Some(&(_, _, anchor_tl, anchor_tc)) = items.iter().min_by(|a, b| a.3.total_cmp(&b.3))
        else {
            return;
        };

        let mut cmds: Vec<Box<dyn trim_commands::Command>> = Vec::new();
        for (tr_id, cl_id, old_in, clip_tc) in items {
            let new_in = (anchor_tl + (clip_tc - anchor_tc)).max(0.0);
            if (new_in - old_in).abs() > 1e-4 {
                cmds.push(Box::new(trim_commands::MoveClipCommand::new(
                    tr_id.clone(),
                    tr_id,
                    cl_id,
                    old_in,
                    new_in,
                )));
            }
        }

        if cmds.is_empty() {
            self.set_status(
                "Klipler zaten timecode'a göre hizalı",
                "Clips are already aligned by timecode",
            );
            return;
        }

        let count = cmds.len();
        let macro_cmd = Box::new(trim_commands::MacroCommand::new(
            "Multicam Timecode Sync",
            cmds,
        ));
        if self
            .undo_stack
            .push(macro_cmd, &mut self.project.timeline)
            .is_ok()
        {
            self.video_engine.clear_cache();
            self.audio_mixer.clear_cache();
            self.project.mark_dirty();
            self.last_rendered_dirty = true;
            self.status_message = match self.lang() {
                crate::i18n::Language::Turkish => {
                    format!("⏱ {} klip SMPTE Timecode ile senkronize edildi", count + 1)
                }
                crate::i18n::Language::English => {
                    format!("⏱ Synced {} clips via SMPTE Timecode", count + 1)
                }
            };
        }
    }
}
