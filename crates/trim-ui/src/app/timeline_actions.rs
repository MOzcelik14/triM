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
}
