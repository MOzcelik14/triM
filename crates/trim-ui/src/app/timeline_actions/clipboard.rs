use crate::app::TrimApp;
use trim_commands::AddClipCommand;

impl TrimApp {
    /// Copies currently selected timeline clips into the internal clipboard preserving relative track hierarchy.
    pub fn copy_selected_clips(&mut self) {
        if self.canvas_state.selected_clip_pairs.is_empty() {
            self.set_status("Kopyalanacak klip seçilmedi", "No clip selected to copy");
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

        self.status_message = match self.lang() {
            crate::i18n::Language::Turkish => {
                format!("📋 Kopyalandı: {} klip", self.clipboard_clips.len())
            }
            crate::i18n::Language::English => {
                format!("📋 Copied: {} clip(s)", self.clipboard_clips.len())
            }
        };
    }

    /// Pastes clips from the clipboard starting at the playhead (`current_time`) preserving track hierarchy.
    pub fn paste_clipboard_clips(&mut self) {
        if self.clipboard_clips.is_empty() {
            self.set_status(
                "Panoda yapıştırılacak klip yok",
                "No clips in clipboard to paste",
            );
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
            self.status_message = match self.lang() {
                crate::i18n::Language::Turkish => format!(
                    "📋 Yapıştırıldı: {} klip ({:.2}s -> {:.2}s)",
                    count, paste_time, max_out
                ),
                crate::i18n::Language::English => format!(
                    "📋 Pasted: {} clip(s) ({:.2}s -> {:.2}s)",
                    count, paste_time, max_out
                ),
            };
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
}
