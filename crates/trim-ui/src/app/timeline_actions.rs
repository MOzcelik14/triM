//! Timeline tracks, clip splitting, ripple deletion, linking, markers, and on-monitor text overlays.

use crate::app::TrimApp;
use trim_commands::{
    AddClipCommand, AddMarkerCommand, AddTrackCommand, CloseGapCommand, LinkClipsCommand,
    RemoveMarkerCommand, RemoveMultipleClipsCommand, RippleDeleteCommand, SetClipColorCommand,
    UnlinkClipsCommand,
};
use trim_core::clip::{Clip, ColorLabel, TextOverlay};
use trim_core::marker::Marker;
use trim_media::{generate_title_image, FFprobeAnalyzer};
use trim_timeline::split_at_playhead;

impl TrimApp {
    /// Resolves and assigns target tracks (V1 and A1), falling back to first available tracks with explicit logging.
    pub fn resolve_default_target_tracks(&mut self) {
        let v_tracks = self.project.timeline.get_video_tracks();
        self.target_video_track = v_tracks
            .iter()
            .find(|t| t.name == "V1")
            .copied()
            .or_else(|| v_tracks.first().copied())
            .map(|t| t.id.clone())
            .unwrap_or_else(|| {
                log::warn!("Hedef video kanalı (V1) bulunamadı");
                String::new()
            });

        let a_tracks = self.project.timeline.get_audio_tracks();
        self.target_audio_track = a_tracks
            .iter()
            .find(|t| t.name == "A1")
            .copied()
            .or_else(|| a_tracks.first().copied())
            .map(|t| t.id.clone())
            .unwrap_or_else(|| {
                log::warn!("Hedef ses kanalı (A1) bulunamadı");
                String::new()
            });
    }

    /// Creates a dedicated text/title clip on the timeline at `time`, placing it
    /// on an upper video track so it overlays any underlying video footage.
    pub fn create_text_clip_at_time(
        &mut self,
        time: f64,
        pos_x: f32,
        pos_y: f32,
        text: &str,
    ) -> Option<(String, String)> {
        let duration = 5.0; // Standard 5.0s title clip duration
        let out_png = generate_title_image(
            "",
            48,
            [255, 255, 255],
            [0, 0, 0, 0], // Fully transparent canvas
            "center",
            false,
            self.project.timeline.width,
            self.project.timeline.height,
        )?;

        let mut item = if let Ok(it) = FFprobeAnalyzer::analyze(&out_png) {
            it
        } else {
            let mut it = trim_core::media::MediaItem::new(out_png.to_str().unwrap());
            it.media_type = trim_core::media::MediaType::Image;
            it
        };
        let label = if text.trim().is_empty() {
            "Metin"
        } else {
            text
        };
        item.name = format!("Metin: {}", label);
        item.duration = duration;
        let m_id = item.id.clone();
        self.project.add_media(item);

        let current_video_tracks = self.project.timeline.get_video_tracks();
        let target_has_clip = self
            .project
            .timeline
            .get_track_by_id(&self.target_video_track)
            .map(|t| t.find_clip_at(time).is_some())
            .unwrap_or(false);

        let target_track_id = if target_has_clip {
            // Find an upper track that does not have a clip at this time and is not locked
            let upper_track = current_video_tracks
                .iter()
                .filter(|t| t.id != self.target_video_track && !t.locked)
                .find(|t| t.find_clip_at(time).is_none());

            if let Some(t) = upper_track {
                t.id.clone()
            } else {
                // Auto-create an upper video track (e.g. V2, V3) stacked on top
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
                self.status_message =
                    format!("Üst video kanalı otomatik oluşturuldu: {}", new_track_name);
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

        if target_track_id.is_empty() {
            return None;
        }

        let mut new_clip = Clip::new(&m_id, time, time + duration);
        new_clip.name = format!("Metin: {}", label);
        new_clip.text_overlay = Some(TextOverlay {
            content: label.to_string(),
            pos_x,
            pos_y,
            font_size: 48.0,
            text_color: [255, 255, 255, 255],
            has_shadow: true,
            ..Default::default()
        });
        let cl_id = new_clip.id.clone();
        let cmd = Box::new(AddClipCommand::new(target_track_id.clone(), new_clip));
        let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
        self.project.mark_dirty();
        self.last_rendered_monitor_frame = None;
        self.video_engine.last_rendered_frame = None;
        Some((target_track_id, cl_id))
    }

    pub fn handle_monitor_text_click(&mut self, pos_x: f32, pos_y: f32) {
        // 1. Check if the user clicked on an existing text overlay at the current playhead
        let mut hit_text_clip = None;
        for track in self.project.timeline.get_video_tracks() {
            if let Some(clip) = track.find_clip_at(self.current_time) {
                if let Some(txt) = &clip.text_overlay {
                    let dx = (pos_x - txt.pos_x).abs();
                    let dy = (pos_y - txt.pos_y).abs();
                    if dx < 0.20 && dy < 0.12 {
                        hit_text_clip = Some((track.id.clone(), clip.id.clone()));
                        break;
                    }
                }
            }
        }

        if let Some((tr_id, cl_id)) = hit_text_clip {
            // User clicked on an existing text overlay: select it for editing / dragging
            self.canvas_state.selected_clip_pairs = vec![(tr_id, cl_id)];
            self.status_message = "Mevcut Metin Klibi Seçildi".to_string();
        } else {
            // User clicked on the monitor with Type Tool:
            // ALWAYS create a dedicated text clip on the timeline!
            if let Some((tr_id, cl_id)) =
                self.create_text_clip_at_time(self.current_time, pos_x, pos_y, "Metin")
            {
                self.canvas_state.selected_clip_pairs = vec![(tr_id, cl_id)];
                self.status_message = format!(
                    "Zaman Çizgisine Yeni Metin Klibi Eklendi (5.0s) - X: {:.0}%, Y: {:.0}%",
                    pos_x * 100.0,
                    pos_y * 100.0
                );
            }
        }
    }

    /// Selects the text clip under the playhead whose anchor matches (`pos_x`, `pos_y`).
    /// Topmost visible track wins when several text clips share the same anchor.
    pub fn handle_monitor_text_select(&mut self, pos_x: f32, pos_y: f32) {
        let mut found = None;
        for track in self.project.timeline.get_video_tracks().iter().rev() {
            if !track.visible {
                continue;
            }
            if let Some(clip) = track.find_clip_at(self.current_time) {
                if let Some(txt) = &clip.text_overlay {
                    if (txt.pos_x - pos_x).abs() < 1e-3 && (txt.pos_y - pos_y).abs() < 1e-3 {
                        found = Some((track.id.clone(), clip.id.clone()));
                        break;
                    }
                }
            }
        }
        if let Some(pair) = found {
            if self.canvas_state.selected_clip_pairs.first() != Some(&pair) {
                self.canvas_state.selected_clip_pairs = vec![pair];
                self.status_message = "Metin Klibi Seçildi (görüntüde sürükleyerek taşıyın)".into();
            }
        }
    }

    pub fn handle_monitor_text_drag(&mut self, pos_x: f32, pos_y: f32) {
        if let Some((tr_id, cl_id)) = self.canvas_state.selected_clip_pairs.first().cloned() {
            if let Some(tr) = self.project.timeline.get_track_by_id_mut(&tr_id) {
                if let Some(cl) = tr.get_clip_by_id_mut(&cl_id) {
                    if let Some(txt) = &mut cl.text_overlay {
                        txt.pos_x = pos_x;
                        txt.pos_y = pos_y;
                        self.project.mark_dirty();
                        self.last_rendered_monitor_frame = None;
                        self.video_engine.last_rendered_frame = None;
                    }
                }
            }
        }
    }

    pub fn handle_monitor_text_content_changed(&mut self, content: String) {
        if let Some((tr_id, cl_id)) = self.canvas_state.selected_clip_pairs.first().cloned() {
            if let Some(tr) = self.project.timeline.get_track_by_id_mut(&tr_id) {
                if let Some(cl) = tr.get_clip_by_id_mut(&cl_id) {
                    if let Some(txt) = &mut cl.text_overlay {
                        txt.content = content.clone();
                        if !content.trim().is_empty() {
                            cl.name = format!("Metin: {}", content);
                        }
                        self.project.mark_dirty();
                        self.last_rendered_monitor_frame = None;
                        self.video_engine.last_rendered_frame = None;
                    }
                }
            }
        }
    }

    pub fn toggle_link_selected_clips(&mut self) {
        if self.canvas_state.selected_clip_pairs.len() < 2 {
            if let Some((tr_id, cl_id)) = self.canvas_state.selected_clip_pairs.first() {
                if let Some(tr) = self.project.timeline.get_track_by_id(tr_id) {
                    if let Some(cl) = tr.get_clip_by_id(cl_id) {
                        if cl.linked_group_id.is_some() {
                            let cmd = Box::new(UnlinkClipsCommand::new(vec![(
                                tr_id.clone(),
                                cl_id.clone(),
                            )]));
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
        let first_gid = self
            .project
            .timeline
            .get_track_by_id(&first_pair.0)
            .and_then(|t| t.get_clip_by_id(&first_pair.1))
            .and_then(|c| c.linked_group_id.clone());

        let all_linked = first_gid.is_some()
            && self
                .canvas_state
                .selected_clip_pairs
                .iter()
                .all(|(tr_id, cl_id)| {
                    self.project
                        .timeline
                        .get_track_by_id(tr_id)
                        .and_then(|t| t.get_clip_by_id(cl_id))
                        .and_then(|c| c.linked_group_id.as_ref())
                        == first_gid.as_ref()
                });

        if all_linked {
            let cmd = Box::new(UnlinkClipsCommand::new(
                self.canvas_state.selected_clip_pairs.clone(),
            ));
            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
            self.status_message = "Kliplerin bağı kaldırıldı (Unlink)".to_string();
        } else {
            let cmd = Box::new(LinkClipsCommand::new(
                self.canvas_state.selected_clip_pairs.clone(),
            ));
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
            .filter_map(|t| {
                t.name
                    .strip_prefix('V')
                    .and_then(|s| s.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(0);
        let track = trim_core::track::Track::new(
            format!("V{}", max_idx + 1),
            trim_core::track::TrackType::Video,
        );
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
            .filter_map(|t| {
                t.name
                    .strip_prefix('A')
                    .and_then(|s| s.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(0);
        let track = trim_core::track::Track::new(
            format!("A{}", max_idx + 1),
            trim_core::track::TrackType::Audio,
        );
        let name = track.name.clone();
        let len = self.project.timeline.tracks.len();
        let cmd = Box::new(AddTrackCommand::new(track, len));
        let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
        self.status_message = format!("Ses kanalı eklendi: {}", name);
        self.project.mark_dirty();
    }

    pub fn delete_selected_clips(&mut self) {
        if !self.canvas_state.selected_clip_pairs.is_empty() {
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
    }

    pub fn ripple_delete_selected_clips(&mut self) {
        if let Some((tr_id, cl_id)) = self.canvas_state.selected_clip_pairs.first().cloned() {
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

    pub fn add_or_toggle_marker(&mut self) {
        if let Some(existing) = self
            .project
            .timeline
            .get_marker_at(self.current_time, 0.15)
            .cloned()
        {
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

    pub fn add_marker_at_playhead(&mut self) {
        self.add_or_toggle_marker();
    }

    pub fn close_selected_gap(&mut self, all_tracks: bool) {
        if let Some((tr_id, gap_start, gap_end)) = self.canvas_state.selected_gap.take() {
            let dur = (gap_end - gap_start).max(0.0);
            if dur <= 1e-4 {
                return;
            }
            let target_tr = if all_tracks { None } else { Some(tr_id) };
            let cmd = Box::new(CloseGapCommand::new(target_tr, gap_start, gap_end));
            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
            self.status_message = format!("⏩ Boşluk kapatıldı ({:.2}s)", dur);
            self.project.mark_dirty();
        }
    }

    pub fn set_selected_clips_color(&mut self, color: ColorLabel) {
        for (tr_id, cl_id) in &self.canvas_state.selected_clip_pairs {
            let cmd = Box::new(SetClipColorCommand::new(
                tr_id.clone(),
                cl_id.clone(),
                color,
            ));
            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
        }
        self.status_message = format!("🎨 Klip rengi güncellendi: {}", color.label());
        self.project.mark_dirty();
    }

    /// Copies currently selected timeline clips into the internal clipboard preserving relative track hierarchy.
    pub fn copy_selected_clips(&mut self) {
        if self.canvas_state.selected_clip_pairs.is_empty() {
            self.status_message = "Kopyalanacak klip seçilmedi".to_string();
            return;
        }

        let mut v_tracks = self.project.timeline.get_video_tracks();
        v_tracks.sort_by_key(|t| {
            t.name
                .strip_prefix('V')
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0)
        });
        let v_ids: Vec<String> = v_tracks.iter().map(|t| t.id.clone()).collect();

        let mut a_tracks = self.project.timeline.get_audio_tracks();
        a_tracks.sort_by_key(|t| {
            t.name
                .strip_prefix('A')
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0)
        });
        let a_ids: Vec<String> = a_tracks.iter().map(|t| t.id.clone()).collect();

        let mut clips_to_copy = Vec::new();
        let mut min_in = f64::MAX;
        let mut min_v_idx = usize::MAX;
        let mut min_a_idx = usize::MAX;

        for (tr_id, cl_id) in &self.canvas_state.selected_clip_pairs {
            if let Some(track) = self.project.timeline.get_track_by_id(tr_id) {
                if let Some(clip) = track.get_clip_by_id(cl_id) {
                    if clip.timeline_in < min_in {
                        min_in = clip.timeline_in;
                    }
                    match track.track_type {
                        trim_core::track::TrackType::Video => {
                            let idx = v_ids.iter().position(|id| id == tr_id).unwrap_or(0);
                            min_v_idx = min_v_idx.min(idx);
                            clips_to_copy.push((
                                trim_core::track::TrackType::Video,
                                idx,
                                clip.clone(),
                            ));
                        }
                        trim_core::track::TrackType::Audio => {
                            let idx = a_ids.iter().position(|id| id == tr_id).unwrap_or(0);
                            min_a_idx = min_a_idx.min(idx);
                            clips_to_copy.push((
                                trim_core::track::TrackType::Audio,
                                idx,
                                clip.clone(),
                            ));
                        }
                    }
                }
            }
        }

        if clips_to_copy.is_empty() {
            return;
        }

        self.clipboard_clips = clips_to_copy
            .into_iter()
            .map(|(tt, idx, clip)| {
                let rel_offset = (clip.timeline_in - min_in).max(0.0);
                let rel_tr = match tt {
                    trim_core::track::TrackType::Video => idx.saturating_sub(min_v_idx),
                    trim_core::track::TrackType::Audio => idx.saturating_sub(min_a_idx),
                };
                (tt, rel_tr, rel_offset, clip)
            })
            .collect();

        self.status_message = format!("📋 Kopyalandı: {} klip", self.clipboard_clips.len());
    }

    /// Pastes clips from the clipboard starting at the playhead (`current_time`) preserving track hierarchy.
    pub fn paste_clipboard_clips(&mut self) {
        if self.clipboard_clips.is_empty() {
            self.status_message = "Panoda yapıştırılacak klip yok".to_string();
            return;
        }

        let paste_time = self.current_time;
        let mut new_selection = Vec::new();
        let mut commands: Vec<Box<dyn trim_commands::Command>> = Vec::new();
        let mut linked_id_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        let mut v_tracks = self.project.timeline.get_video_tracks();
        v_tracks.sort_by_key(|t| {
            t.name
                .strip_prefix('V')
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0)
        });
        let mut v_ids: Vec<String> = v_tracks.iter().map(|t| t.id.clone()).collect();

        let mut a_tracks = self.project.timeline.get_audio_tracks();
        a_tracks.sort_by_key(|t| {
            t.name
                .strip_prefix('A')
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0)
        });
        let mut a_ids: Vec<String> = a_tracks.iter().map(|t| t.id.clone()).collect();

        let base_v_idx = v_ids
            .iter()
            .position(|id| id == &self.target_video_track)
            .unwrap_or(0);
        let base_a_idx = a_ids
            .iter()
            .position(|id| id == &self.target_audio_track)
            .unwrap_or(0);

        let mut max_out = paste_time;
        for (track_type, rel_track, rel_offset, clip) in &self.clipboard_clips {
            let final_track_id = match track_type {
                trim_core::track::TrackType::Video => {
                    let desired_idx = base_v_idx + rel_track;
                    while v_ids.len() <= desired_idx {
                        let new_id = self.project.timeline.add_new_video_track();
                        v_ids.push(new_id);
                    }
                    let tr_id = &v_ids[desired_idx];
                    if let Some(tr) = self.project.timeline.get_track_by_id(tr_id) {
                        if !tr.locked {
                            tr_id.clone()
                        } else {
                            v_ids
                                .iter()
                                .find(|id| {
                                    self.project
                                        .timeline
                                        .get_track_by_id(id)
                                        .map(|t| !t.locked)
                                        .unwrap_or(false)
                                })
                                .cloned()
                                .unwrap_or_else(|| tr_id.clone())
                        }
                    } else {
                        tr_id.clone()
                    }
                }
                trim_core::track::TrackType::Audio => {
                    let desired_idx = base_a_idx + rel_track;
                    while a_ids.len() <= desired_idx {
                        let new_id = self.project.timeline.add_new_audio_track();
                        a_ids.push(new_id);
                    }
                    let tr_id = &a_ids[desired_idx];
                    if let Some(tr) = self.project.timeline.get_track_by_id(tr_id) {
                        if !tr.locked {
                            tr_id.clone()
                        } else {
                            a_ids
                                .iter()
                                .find(|id| {
                                    self.project
                                        .timeline
                                        .get_track_by_id(id)
                                        .map(|t| !t.locked)
                                        .unwrap_or(false)
                                })
                                .cloned()
                                .unwrap_or_else(|| tr_id.clone())
                        }
                    } else {
                        tr_id.clone()
                    }
                }
            };

            let mut new_clip = clip.clone();
            let new_clip_id = uuid::Uuid::new_v4().to_string();
            new_clip.id = new_clip_id.clone();
            new_clip.timeline_in = paste_time + rel_offset;
            new_clip.timeline_out = new_clip.timeline_in + clip.duration();
            max_out = max_out.max(new_clip.timeline_out);

            if let Some(old_grp) = &clip.linked_group_id {
                let new_grp = linked_id_map
                    .entry(old_grp.clone())
                    .or_insert_with(|| uuid::Uuid::new_v4().to_string())
                    .clone();
                new_clip.linked_group_id = Some(new_grp);
            }

            new_selection.push((final_track_id.clone(), new_clip_id));
            commands.push(Box::new(AddClipCommand::new(final_track_id, new_clip)));
        }

        if commands.is_empty() {
            return;
        }

        let count = commands.len();
        let macro_cmd = Box::new(trim_commands::MacroCommand::new(
            "Klip(leri) Yapıştır",
            commands,
        ));
        if self
            .undo_stack
            .push(macro_cmd, &mut self.project.timeline)
            .is_ok()
        {
            self.canvas_state.selected_clip_pairs = new_selection;
            self.current_time = max_out;
            self.last_rendered_dirty = true;
            self.video_engine.clear_cache();
            self.audio_mixer.clear_cache();
            self.project.mark_dirty();
            self.status_message = format!(
                "📋 Yapıştırıldı: {} klip ({:.2}s -> {:.2}s)",
                count, paste_time, max_out
            );
        }
    }

    /// Zooms the timeline in or out by `factor`, centered at `focus_time` (or playhead if None).
    /// If `focus_view_x` is provided, ensures the point remains under the cursor screen position.
    pub fn zoom_timeline(
        &mut self,
        factor: f32,
        focus_time: Option<f64>,
        focus_view_x: Option<f32>,
    ) {
        let old_pps = self.canvas_state.pixels_per_second;
        let new_pps = (old_pps * factor).clamp(15.0, 350.0);
        if (new_pps - old_pps).abs() < 1e-3 {
            return;
        }
        self.canvas_state.pixels_per_second = new_pps;

        let scroll_x = self.canvas_state.scroll_offset.x;
        let (target_time, view_x) = match (focus_time, focus_view_x) {
            (Some(t), Some(vx)) => (t, vx),
            (Some(t), None) => {
                let vx = ((t as f32 * old_pps) - scroll_x).max(0.0);
                (t, vx)
            }
            (None, _) => {
                let t = self.current_time;
                let vx = ((t as f32 * old_pps) - scroll_x).max(0.0);
                (t, vx)
            }
        };

        let new_scroll_x = ((target_time as f32 * new_pps) - view_x).max(0.0);
        self.canvas_state.pending_scroll_x = Some(new_scroll_x);
    }

    /// Aligns selected clips across different tracks using cross-correlation of their audio waveforms.
    pub fn align_selected_clips_by_audio(&mut self) {
        if self.canvas_state.selected_clip_pairs.len() < 2 {
            self.status_message =
                "Ses ile hizalamak için farklı kanallarda en az 2 klip seçin".to_string();
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
            self.status_message = "Referans klibin medya dosyası bulunamadı".to_string();
            return;
        };

        let pps = self.waveform_gen.points_per_second;
        let ref_wf = self
            .waveform_gen
            .get_waveform(&ref_media.file_path)
            .or_else(|| self.waveform_gen.generate_sync(&ref_media.file_path));
        let Some(ref_full_wf) = ref_wf else {
            self.status_message = "Referans klibin ses dalga formu çıkarılamadı".to_string();
            return;
        };

        let ref_s_in = ((ref_clip.source_in * pps as f64) as usize).min(ref_full_wf.len());
        let ref_s_out =
            ((ref_clip.effective_source_out() * pps as f64) as usize).min(ref_full_wf.len());
        if ref_s_out <= ref_s_in + 4 {
            self.status_message = "Referans klipte yeterli ses verisi yok".to_string();
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
            self.status_message = "🎙️ Klipler zaten ses dalga formuna göre hizalı".to_string();
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
            self.status_message = format!(
                "🎙️ {} klip ses dalga formuna göre otomatik senkronize edildi",
                count
            );
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
            self.status_message = "⚠ Hareket takibi için medya kaynağı bulunamadı".to_string();
            return;
        };

        let init_cx = (mask.pos_x - 0.5) as f32;
        let init_cy = (mask.pos_y - 0.5) as f32;
        let sx = mask.scale_x as f32;
        let sy = mask.scale_y as f32;

        match trim_media::video::track_region_motion(
            &media_path,
            source_in,
            duration,
            speed,
            init_cx,
            init_cy,
            sx,
            sy,
        ) {
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
                    track_id.to_string(),
                    clip_id.to_string(),
                    old_masks,
                    new_masks,
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
        }
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
            self.status_message =
                "ℹ Diğer kanallarda veya altyazılarda çakışan konuşma bölgesi bulunamadı"
                    .to_string();
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
            self.status_message = format!(
                "🔉 {} konuşma bölgesi için otomatik ses kısma (Auto-Ducking) uygulandı",
                merged.len()
            );
        }
    }

    /// Nests all currently selected clips into a new Compound Clip (sub-sequence) and replaces them on the timeline.
    pub fn create_compound_clip_from_selected(&mut self) {
        if self.canvas_state.selected_clip_pairs.is_empty() {
            self.status_message =
                "Bileşik klip (Compound Clip) oluşturmak için en az 1 klip seçin".to_string();
            return;
        }

        let mut min_in = f64::MAX;
        let mut max_out = 0.0_f64;
        let mut v_clips = Vec::new();
        let mut a_clips = Vec::new();
        let mut first_v_track = None;

        for (tr_id, cl_id) in &self.canvas_state.selected_clip_pairs {
            if let Some(tr) = self.project.timeline.get_track_by_id(tr_id) {
                if tr.locked {
                    continue;
                }
                if let Some(cl) = tr.get_clip_by_id(cl_id) {
                    min_in = min_in.min(cl.timeline_in);
                    max_out = max_out.max(cl.timeline_out);
                    match tr.track_type {
                        trim_core::track::TrackType::Video => {
                            if first_v_track.is_none() {
                                first_v_track = Some(tr_id.clone());
                            }
                            v_clips.push(cl.clone());
                        }
                        trim_core::track::TrackType::Audio => {
                            a_clips.push(cl.clone());
                        }
                    }
                }
            }
        }

        if (v_clips.is_empty() && a_clips.is_empty()) || max_out <= min_in {
            self.status_message = "Seçili kliplerden bileşik klip oluşturulamadı".to_string();
            return;
        }

        self.project.sync_active_sequence_to_list();
        let comp_num = self.project.sequences.len() + 1;
        let seq_id = format!("seq_{}", uuid::Uuid::new_v4());
        let seq_name = format!("Bileşik Klip {:02}", comp_num);

        let mut comp_tl = trim_core::timeline::TimelineModel {
            width: self.project.timeline.width,
            height: self.project.timeline.height,
            fps: self.project.timeline.fps,
            ..Default::default()
        };
        comp_tl.set_aspect_ratio(self.project.timeline.aspect_ratio());

        for mut c in v_clips {
            c.id = uuid::Uuid::new_v4().to_string();
            c.timeline_in = (c.timeline_in - min_in).max(0.0);
            c.timeline_out = (c.timeline_out - min_in).max(c.timeline_in + 0.05);
            let placed = comp_tl
                .tracks
                .iter_mut()
                .filter(|t| t.track_type == trim_core::track::TrackType::Video)
                .find(|t| {
                    !t.clips.iter().any(|other| {
                        c.timeline_in < other.timeline_out && c.timeline_out > other.timeline_in
                    })
                });
            if let Some(tr) = placed {
                tr.add_clip(c);
            } else {
                let new_v_id = comp_tl.add_new_video_track();
                if let Some(tr) = comp_tl.get_track_by_id_mut(&new_v_id) {
                    tr.add_clip(c);
                }
            }
        }

        let has_audio = !a_clips.is_empty();
        for mut c in a_clips {
            c.id = uuid::Uuid::new_v4().to_string();
            c.timeline_in = (c.timeline_in - min_in).max(0.0);
            c.timeline_out = (c.timeline_out - min_in).max(c.timeline_in + 0.05);
            let placed = comp_tl
                .tracks
                .iter_mut()
                .filter(|t| t.track_type == trim_core::track::TrackType::Audio)
                .find(|t| {
                    !t.clips.iter().any(|other| {
                        c.timeline_in < other.timeline_out && c.timeline_out > other.timeline_in
                    })
                });
            if let Some(tr) = placed {
                tr.add_clip(c);
            } else {
                let new_a_id = comp_tl.add_new_audio_track();
                if let Some(tr) = comp_tl.get_track_by_id_mut(&new_a_id) {
                    tr.add_clip(c);
                }
            }
        }

        self.project.sequences.push(trim_core::sequence::Sequence {
            id: seq_id.clone(),
            name: seq_name.clone(),
            timeline: comp_tl,
        });

        let pairs_to_remove = self.canvas_state.selected_clip_pairs.clone();
        let mut cmds: Vec<Box<dyn trim_commands::Command>> =
            vec![Box::new(RemoveMultipleClipsCommand::new(pairs_to_remove))];

        let target_v = first_v_track.unwrap_or_else(|| self.target_video_track.clone());
        let link_gid = if has_audio {
            Some(uuid::Uuid::new_v4().to_string())
        } else {
            None
        };

        let mut nested_v = Clip::new_nested_sequence(&seq_id, &seq_name, min_in, max_out);
        nested_v.linked_group_id = link_gid.clone();
        let nested_v_id = nested_v.id.clone();
        cmds.push(Box::new(AddClipCommand::new(target_v.clone(), nested_v)));

        if let Some(gid) = link_gid {
            let target_a = self
                .project
                .timeline
                .find_or_create_corresponding_audio_track(&target_v);
            let mut nested_a =
                Clip::new_nested_sequence(&seq_id, format!("{} (Ses)", seq_name), min_in, max_out);
            nested_a.linked_group_id = Some(gid);
            cmds.push(Box::new(AddClipCommand::new(target_a, nested_a)));
        }

        let macro_cmd = Box::new(trim_commands::MacroCommand::new(
            "Bileşik Klip Oluştur",
            cmds,
        ));
        if self
            .undo_stack
            .push(macro_cmd, &mut self.project.timeline)
            .is_ok()
        {
            self.canvas_state.selected_clip_pairs = vec![(target_v, nested_v_id)];
            self.video_engine.clear_cache();
            self.audio_mixer.clear_cache();
            self.project.mark_dirty();
            self.status_message = format!("📦 Bileşik klip oluşturuldu: {}", seq_name);
        }
    }

    /// Joins contiguous split clips (Through-Edits) from the same source media back into a single clip.
    pub fn join_selected_through_edits(&mut self) {
        let selected_set: std::collections::HashSet<(String, String)> = self
            .canvas_state
            .selected_clip_pairs
            .iter()
            .cloned()
            .collect();

        type TrimBounds = (f64, f64, f64, f64);
        type ThroughEditJoin = (String, String, String, TrimBounds, TrimBounds);
        let mut joins: Vec<ThroughEditJoin> = Vec::new();

        for tr in &self.project.timeline.tracks {
            if tr.locked || tr.clips.len() < 2 {
                continue;
            }
            for i in 0..(tr.clips.len() - 1) {
                let a = &tr.clips[i];
                let b = &tr.clips[i + 1];
                let is_relevant = if selected_set.is_empty() {
                    (a.timeline_out - self.current_time).abs() < 0.1
                } else {
                    selected_set.contains(&(tr.id.clone(), a.id.clone()))
                        || selected_set.contains(&(tr.id.clone(), b.id.clone()))
                };
                if !is_relevant {
                    continue;
                }
                let contiguous_tl = (b.timeline_in - a.timeline_out).abs() < 0.05;
                let same_media = a.media_id == b.media_id;
                let same_speed = (a.speed - b.speed).abs() < 0.01 && a.reverse == b.reverse;
                let contiguous_src = (b.source_in - a.effective_source_out()).abs() < 0.12;

                if contiguous_tl && same_media && same_speed && contiguous_src {
                    let old_bounds = (
                        a.timeline_in,
                        a.timeline_out,
                        a.source_in,
                        a.effective_source_out(),
                    );
                    let new_bounds = (
                        a.timeline_in,
                        b.timeline_out,
                        a.source_in,
                        b.effective_source_out(),
                    );
                    joins.push((
                        tr.id.clone(),
                        a.id.clone(),
                        b.id.clone(),
                        old_bounds,
                        new_bounds,
                    ));
                    break;
                }
            }
        }

        if joins.is_empty() {
            self.status_message =
                "Birleştirilecek kesik klip (Through-Edit) bulunamadı (Aynı kaynaktan ardışık iki kesik klip seçin)"
                    .to_string();
            return;
        }

        let count = joins.len();
        let mut cmds: Vec<Box<dyn trim_commands::Command>> = Vec::new();
        let mut new_sel = Vec::new();
        for (tr_id, keep_id, remove_id, old_b, new_b) in joins {
            cmds.push(Box::new(RemoveMultipleClipsCommand::new(vec![(
                tr_id.clone(),
                remove_id,
            )])));
            cmds.push(Box::new(trim_commands::TrimClipCommand::new(
                tr_id.clone(),
                keep_id.clone(),
                old_b,
                new_b,
            )));
            new_sel.push((tr_id, keep_id));
        }

        let macro_cmd = Box::new(trim_commands::MacroCommand::new(
            "Kesik Klipleri Birleştir (Join Through-Edits)",
            cmds,
        ));
        if self
            .undo_stack
            .push(macro_cmd, &mut self.project.timeline)
            .is_ok()
        {
            self.canvas_state.selected_clip_pairs = new_sel;
            self.video_engine.clear_cache();
            self.audio_mixer.clear_cache();
            self.project.mark_dirty();
            self.status_message = format!("🔗 {} kesik nokta tek klipte birleştirildi", count);
        }
    }

    /// Selectively pastes Color, Transform, Audio, or Speed attributes from the first clipboard clip onto selected clips.
    pub fn apply_paste_attributes(
        &mut self,
        copy_color: bool,
        copy_transform: bool,
        copy_audio: bool,
        copy_speed: bool,
    ) {
        let Some((_, _, _, src)) = self.clipboard_clips.first().cloned() else {
            self.status_message =
                "Panoda özellikleri kopyalanmış klip yok (Önce Ctrl+C yapın)".to_string();
            return;
        };
        if self.canvas_state.selected_clip_pairs.is_empty() {
            self.status_message = "Özelliklerin uygulanacağı hedef klip(leri) seçin".to_string();
            return;
        }

        let mut applied_count = 0;
        let selected = self.canvas_state.selected_clip_pairs.clone();
        for (tr_id, cl_id) in selected {
            if let Some(tr) = self.project.timeline.get_track_by_id_mut(&tr_id) {
                if tr.locked {
                    continue;
                }
                if let Some(cl) = tr.get_clip_by_id_mut(&cl_id) {
                    if copy_color {
                        cl.brightness = src.brightness;
                        cl.contrast = src.contrast;
                        cl.saturation = src.saturation;
                        cl.gamma = src.gamma;
                        cl.temperature = src.temperature;
                        cl.tint = src.tint;
                        cl.vignette = src.vignette;
                        cl.sharpen = src.sharpen;
                        cl.blur = src.blur;
                        cl.film_grain = src.film_grain;
                        cl.wb_preset = src.wb_preset;
                        cl.bc_preset = src.bc_preset;
                        cl.lut_preset = src.lut_preset;
                        cl.custom_lut_path = src.custom_lut_path.clone();
                        cl.color_wheels = src.color_wheels.clone();
                        cl.custom_curves = src.custom_curves.clone();
                        cl.hsl_curves = src.hsl_curves.clone();
                        cl.hsl_qualifier = src.hsl_qualifier.clone();
                        cl.halation = src.halation;
                        cl.bloom = src.bloom;
                        cl.chromatic_aberration = src.chromatic_aberration;
                    }
                    if copy_transform {
                        cl.pos_x = src.pos_x;
                        cl.pos_y = src.pos_y;
                        cl.scale = src.scale;
                        cl.rotation = src.rotation;
                        cl.opacity = src.opacity;
                        cl.crop_left = src.crop_left;
                        cl.crop_right = src.crop_right;
                        cl.crop_top = src.crop_top;
                        cl.crop_bottom = src.crop_bottom;
                        cl.corner_pin = src.corner_pin.clone();
                        cl.blend_mode = src.blend_mode;
                        cl.track_matte_above = src.track_matte_above;
                    }
                    if copy_audio {
                        cl.volume = src.volume;
                        cl.pan = src.pan;
                        cl.eq_preset = src.eq_preset;
                        cl.noise_gate = src.noise_gate;
                        cl.voice_denoise = src.voice_denoise;
                        cl.reverb_amount = src.reverb_amount;
                        cl.echo_amount = src.echo_amount;
                        cl.parametric_eq = src.parametric_eq.clone();
                        cl.compressor = src.compressor.clone();
                    }
                    if copy_speed {
                        cl.change_speed(src.speed);
                        cl.reverse = src.reverse;
                        cl.pitch_lock = src.pitch_lock;
                    }
                    applied_count += 1;
                }
                tr.sort_clips();
            }
        }

        if applied_count > 0 {
            self.video_engine.clear_cache();
            self.audio_mixer.clear_cache();
            self.last_rendered_monitor_frame = None;
            self.scopes_panel.mark_dirty();
            self.project.mark_dirty();
            self.status_message =
                format!("📋 Seçili özellikler {} klibe yapıştırıldı", applied_count);
        }
    }

    /// Cuts across active video tracks at the playhead and activates the chosen Multicam angle (0..3).
    pub fn handle_multicam_angle_switch(&mut self, angle_idx: usize) {
        self.multicam_active_angle = angle_idx.min(3);
        let cut_t = self.current_time;

        let mut v_tracks: Vec<String> = self
            .project
            .timeline
            .get_video_tracks()
            .iter()
            .map(|t| t.id.clone())
            .collect();
        // get_video_tracks() is ordered top-to-bottom; reverse so V1 is index 0, V2 is index 1, etc.
        v_tracks.reverse();

        if v_tracks.is_empty() {
            return;
        }

        // First, split any active video clips at playhead if cut_t is strictly inside them
        let mut split_cmds: Vec<Box<dyn trim_commands::Command>> = Vec::new();
        for tr_id in &v_tracks {
            if let Some(tr) = self.project.timeline.get_track_by_id(tr_id) {
                if tr.locked {
                    continue;
                }
                if let Some(cl) = tr.find_clip_at(cut_t) {
                    if cut_t > cl.timeline_in + 0.08 && cut_t < cl.timeline_out - 0.08 {
                        split_cmds.push(Box::new(
                            trim_commands::SplitClipCommand::new(
                                tr_id.clone(),
                                cl.id.clone(),
                                cut_t,
                            )
                            .with_split_linked(false),
                        ));
                    }
                }
            }
        }
        if !split_cmds.is_empty() {
            let macro_split = Box::new(trim_commands::MacroCommand::new(
                "Multicam Açı Kesimi",
                split_cmds,
            ));
            let _ = self
                .undo_stack
                .push(macro_split, &mut self.project.timeline);
        }

        // Now set opacity on the clips active at `cut_t + 0.001` so that `angle_idx` is visible (1.0)
        // and any upper tracks covering it are transparent (0.0)
        let probe_t = cut_t + 0.001;
        for (idx, tr_id) in v_tracks.iter().enumerate() {
            if let Some(tr) = self.project.timeline.get_track_by_id_mut(tr_id) {
                if tr.locked {
                    continue;
                }
                if let Some(cl) = tr.find_clip_at_mut(probe_t) {
                    if idx == self.multicam_active_angle {
                        cl.opacity = 1.0;
                    } else if idx > self.multicam_active_angle {
                        cl.opacity = 0.0;
                    }
                }
            }
        }

        self.video_engine.clear_cache();
        self.last_rendered_monitor_frame = None;
        self.scopes_panel.mark_dirty();
        self.project.mark_dirty();
        self.status_message = format!(
            "🎥 Multicam Kamera {} seçildi ({:.2}s)",
            angle_idx + 1,
            cut_t
        );
    }
}
