use super::TrimApp;
use crate::canvas::CanvasAction;
use trim_commands::commands::*;
use trim_core::clip::Clip;
use trim_core::media::MediaType;
use trim_core::track::TrackType;
use trim_timeline::split_at_playhead;

impl TrimApp {
    pub(super) fn handle_canvas_action(&mut self, ctx: &egui::Context, act: CanvasAction) {
        match act {
            CanvasAction::Seek(t) => self.seek(t),
            CanvasAction::SelectClips(pairs) => {
                self.canvas_state.selected_clip_pairs = pairs;
            }
            CanvasAction::SelectClip(tr_id, cl_id) => {
                self.canvas_state.selected_clip_pairs = vec![(tr_id, cl_id)];
            }
            CanvasAction::SetClipVolume {
                track_id,
                clip_id,
                old_volume,
                new_volume,
            } => {
                let cmd = Box::new(SetClipVolumeCommand::new(
                    track_id, clip_id, old_volume, new_volume,
                ));
                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                self.project.mark_dirty();
            }
            CanvasAction::SetClipFade {
                track_id,
                clip_id,
                is_fade_in,
                old_fade,
                new_fade,
            } => {
                let cmd = Box::new(trim_commands::SetClipFadeCommand::new(
                    track_id, clip_id, is_fade_in, old_fade, new_fade,
                ));
                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                self.project.mark_dirty();
            }
            CanvasAction::SetClipKeyframes {
                track_id,
                clip_id,
                property,
                old_keyframes,
                new_keyframes,
            } => {
                let cmd = Box::new(SetClipKeyframesCommand::new(
                    track_id,
                    clip_id,
                    property,
                    old_keyframes,
                    new_keyframes,
                ));
                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                self.project.mark_dirty();
            }
            CanvasAction::ClearSelection => {
                self.canvas_state.selected_clip_pairs.clear();
            }
            CanvasAction::ClipMoved {
                old_track_id,
                new_track_id,
                clip_id,
                old_in,
                new_in,
            } => {
                let cmd = Box::new(MoveClipCommand::new(
                    old_track_id,
                    new_track_id.clone(),
                    clip_id.clone(),
                    old_in,
                    new_in,
                ));
                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                self.canvas_state.selected_clip_pairs = vec![(new_track_id, clip_id)];
                self.project.mark_dirty();
            }
            CanvasAction::BatchClipsMoved(moves) => {
                let cmd = Box::new(MoveMultipleClipsCommand::new(moves));
                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                self.project.mark_dirty();
            }
            CanvasAction::ClipTrimmed {
                track_id,
                clip_id,
                old_bounds,
                new_bounds,
                companions,
            } => {
                let cmd = Box::new(TrimClipCommand::with_companions(
                    track_id, clip_id, old_bounds, new_bounds, companions,
                ));
                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                self.project.mark_dirty();
            }
            CanvasAction::ClipRippleTrimmed {
                track_id,
                clip_id,
                old_bounds,
                new_bounds,
                shifted_clips,
                companions,
            } => {
                let cmd = Box::new(RippleTrimEdgeCommand::with_companions(
                    track_id,
                    clip_id,
                    old_bounds,
                    new_bounds,
                    shifted_clips,
                    companions,
                ));
                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                self.project.mark_dirty();
            }
            CanvasAction::ClipModified => {
                self.project.mark_dirty();
            }
            CanvasAction::PromptSpeed(tr_id, cl_id) => {
                if let Some(tr) = self.project.timeline.get_track_by_id(&tr_id) {
                    if let Some(cl) = tr.get_clip_by_id(&cl_id) {
                        self.speed_dialog
                            .open(tr_id, cl_id, cl.speed, cl.reverse, cl.duration());
                    }
                }
            }
            CanvasAction::DetachAudio(tr_id, cl_id) => {
                let cmd = Box::new(DetachAudioCommand::new(tr_id, cl_id));
                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                self.project.mark_dirty();
            }
            CanvasAction::SplitAtPlayhead => {
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
            CanvasAction::RazorCut {
                track_id,
                clip_id,
                cut_time,
                split_linked,
            } => {
                let cmd = Box::new(
                    SplitClipCommand::new(track_id, clip_id, cut_time)
                        .with_split_linked(split_linked),
                );
                match self.undo_stack.push(cmd, &mut self.project.timeline) {
                    Ok(()) => {
                        self.status_message = format!("Klip {:.2}. saniyede kesildi", cut_time);
                        self.project.mark_dirty();
                    }
                    Err(e) => {
                        self.status_message = format!("Kesme hatası: {}", e);
                    }
                }
            }
            CanvasAction::CreateTitleAtTime(t) => {
                self.seek(t);
                self.title_dialog.open();
            }
            CanvasAction::Pan(_) => {}
            CanvasAction::CopySelected => {
                self.copy_selected_clips();
                ctx.output_mut(|o| {
                    o.copied_text = format!("trim://clips/{}", self.clipboard_clips.len());
                });
            }
            CanvasAction::PasteClipboard => {
                self.paste_clipboard_clips();
            }
            CanvasAction::DeleteSelected => {
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
                } else if !self.canvas_state.selected_clip_pairs.is_empty() {
                    self.status_message = "Kilitli kanaldaki klipler silinemez".to_string();
                }
            }
            CanvasAction::RippleDeleteSelected => {
                if let Some((tr_id, cl_id)) = self.canvas_state.selected_clip_pairs.first().cloned()
                {
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
            CanvasAction::MediaDropped {
                media_id,
                track_id,
                time,
            } => {
                let seq_id_opt = if let Some(stripped) = media_id.strip_prefix("seq:") {
                    Some(stripped.to_string())
                } else if self.project.sequences.iter().any(|s| s.id == media_id) {
                    Some(media_id.clone())
                } else {
                    None
                };

                if let Some(seq_id) = seq_id_opt {
                    if self.project.active_sequence_id.as_deref() == Some(&seq_id) {
                        self.status_message =
                            "Bir sekans kendi içine yerleştirilemez (Nested Loop Engellendi)"
                                .to_string();
                    } else if let Some(seq) = self
                        .project
                        .sequences
                        .iter()
                        .find(|s| s.id == seq_id)
                        .cloned()
                    {
                        let is_video_track = self
                            .project
                            .timeline
                            .get_track_by_id(&track_id)
                            .map(|t| t.track_type == TrackType::Video)
                            .unwrap_or(false);

                        let dur = seq.timeline.duration().max(0.1);
                        let has_audio = seq
                            .timeline
                            .get_audio_tracks()
                            .iter()
                            .any(|t| !t.clips.is_empty());

                        let (final_video_tr, final_audio_tr) = if is_video_track {
                            let a_tr = self
                                .project
                                .timeline
                                .find_or_create_corresponding_audio_track(&track_id);
                            (track_id.clone(), a_tr)
                        } else {
                            let v_tr = self
                                .project
                                .timeline
                                .find_or_create_corresponding_video_track(&track_id);
                            (v_tr, track_id.clone())
                        };

                        let link_group = if has_audio {
                            Some(uuid::Uuid::new_v4().to_string())
                        } else {
                            None
                        };

                        let mut clip =
                            Clip::new_nested_sequence(&seq.id, &seq.name, time, time + dur);
                        clip.linked_group_id = link_group.clone();
                        let clip_id = clip.id.clone();
                        let cmd = Box::new(AddClipCommand::new(final_video_tr.clone(), clip));

                        if let Some(gid) = link_group {
                            let mut audio_clip = Clip::new_nested_sequence(
                                &seq.id,
                                format!("{} (Ses)", seq.name),
                                time,
                                time + dur,
                            );
                            audio_clip.linked_group_id = Some(gid);
                            let audio_clip_id = audio_clip.id.clone();
                            let cmd_a =
                                Box::new(AddClipCommand::new(final_audio_tr.clone(), audio_clip));
                            let macro_cmd = Box::new(MacroCommand::new(
                                "İç İçe Sekans ve Ses Ekle",
                                vec![cmd, cmd_a],
                            ));
                            let _ = self.undo_stack.push(macro_cmd, &mut self.project.timeline);
                            self.canvas_state.selected_clip_pairs =
                                vec![(final_video_tr, clip_id), (final_audio_tr, audio_clip_id)];
                        } else {
                            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                            self.canvas_state.selected_clip_pairs = vec![(final_video_tr, clip_id)];
                        }
                        self.status_message = format!("İç içe sekans eklendi: {}", seq.name);
                        self.project.mark_dirty();
                    }
                } else if let Some(media) = self.project.get_media(&media_id).cloned() {
                    let clip_dur = if media.media_type == MediaType::Image {
                        5.0
                    } else {
                        media.duration.max(0.1)
                    };
                    let media_fps = media.fps;
                    let media_w = media.width;
                    let media_h = media.height;
                    let media_name = media.name.clone();
                    let has_audio = media.has_audio() || media.audio_channels > 0;
                    let is_video_track = self
                        .project
                        .timeline
                        .get_track_by_id(&track_id)
                        .map(|t| t.track_type == TrackType::Video)
                        .unwrap_or(false);

                    if self
                        .project
                        .timeline
                        .tracks
                        .iter()
                        .all(|t| t.clips.is_empty())
                    {
                        if media_fps > 1.0 {
                            self.project.set_fps(media_fps);
                        }
                        if media_w > 0 && media_h > 0 {
                            self.project.set_resolution(media_w, media_h);
                        }
                    }

                    let is_video_media = media.media_type == MediaType::Video
                        || (media.media_type != MediaType::Audio
                            && media.media_type != MediaType::Image);

                    if is_video_media {
                        let (final_video_tr, final_audio_tr) = if is_video_track {
                            let a_tr = self
                                .project
                                .timeline
                                .find_or_create_corresponding_audio_track(&track_id);
                            (track_id.clone(), a_tr)
                        } else {
                            let v_tr = self
                                .project
                                .timeline
                                .find_or_create_corresponding_video_track(&track_id);
                            (v_tr, track_id.clone())
                        };

                        let link_group = if has_audio {
                            Some(uuid::Uuid::new_v4().to_string())
                        } else {
                            None
                        };

                        let mut clip = Clip::new(&media_id, time, time + clip_dur);
                        clip.name = media_name.clone();
                        clip.linked_group_id = link_group.clone();
                        let clip_id = clip.id.clone();
                        let cmd = Box::new(AddClipCommand::new(final_video_tr.clone(), clip));

                        if let Some(gid) = link_group {
                            let mut audio_clip = Clip::new(&media_id, time, time + clip_dur);
                            audio_clip.name = format!("{} (Ses)", media_name);
                            audio_clip.linked_group_id = Some(gid);
                            let audio_clip_id = audio_clip.id.clone();
                            let cmd_a =
                                Box::new(AddClipCommand::new(final_audio_tr.clone(), audio_clip));
                            let macro_cmd =
                                Box::new(MacroCommand::new("Klip ve Ses Ekle", vec![cmd, cmd_a]));
                            let _ = self.undo_stack.push(macro_cmd, &mut self.project.timeline);
                            self.canvas_state.selected_clip_pairs =
                                vec![(final_video_tr, clip_id), (final_audio_tr, audio_clip_id)];
                        } else {
                            let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                            self.canvas_state.selected_clip_pairs = vec![(final_video_tr, clip_id)];
                        }
                    } else if media.media_type == MediaType::Audio {
                        let final_audio_tr = if is_video_track {
                            self.project
                                .timeline
                                .find_or_create_corresponding_audio_track(&track_id)
                        } else {
                            track_id.clone()
                        };
                        let mut audio_clip = Clip::new(&media_id, time, time + clip_dur);
                        audio_clip.name = media_name.clone();
                        let audio_clip_id = audio_clip.id.clone();
                        let cmd = Box::new(AddClipCommand::new(final_audio_tr.clone(), audio_clip));
                        let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                        self.canvas_state.selected_clip_pairs =
                            vec![(final_audio_tr, audio_clip_id)];
                    } else {
                        // Image or other visual-only media
                        let final_video_tr = if is_video_track {
                            track_id.clone()
                        } else {
                            self.project
                                .timeline
                                .find_or_create_corresponding_video_track(&track_id)
                        };
                        let mut clip = Clip::new(&media_id, time, time + clip_dur);
                        clip.name = media_name.clone();
                        let clip_id = clip.id.clone();
                        let cmd = Box::new(AddClipCommand::new(final_video_tr.clone(), clip));
                        let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                        self.canvas_state.selected_clip_pairs = vec![(final_video_tr, clip_id)];
                    }

                    self.project.mark_dirty();
                }
            }
            CanvasAction::CloseGap {
                track_id,
                gap_start,
                gap_end,
            } => {
                let dur = (gap_end - gap_start).max(0.0);
                let cmd = Box::new(CloseGapCommand::new(track_id, gap_start, gap_end));
                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                self.canvas_state.selected_gap = None;
                self.status_message = format!("⏩ Boşluk kapatıldı ({:.2}s)", dur);
                self.project.mark_dirty();
            }
            CanvasAction::SetClipColor {
                track_id,
                clip_id,
                color,
            } => {
                let cmd = Box::new(SetClipColorCommand::new(track_id, clip_id, color));
                let _ = self.undo_stack.push(cmd, &mut self.project.timeline);
                self.status_message = format!("🎨 Klip rengi güncellendi: {}", color.label());
                self.project.mark_dirty();
            }
            CanvasAction::ToggleLinkSelected => {
                self.toggle_link_selected_clips();
            }
            CanvasAction::AlignSelectedByAudio => {
                self.align_selected_clips_by_audio();
            }
            CanvasAction::AlignSelectedByTimecode => {
                self.align_selected_clips_by_timecode();
            }
            CanvasAction::OpenSequence(seq_id) => {
                self.project.switch_sequence(&seq_id);
                self.video_engine.clear_cache();
                self.audio_mixer.clear_cache();
                self.canvas_state.selected_clip_pairs.clear();
                self.current_time = 0.0;
                self.status_message = "Sekans açıldı".to_string();
            }
            CanvasAction::CreateCompoundClip => {
                self.create_compound_clip_from_selected();
            }
            CanvasAction::JoinThroughEdits => {
                self.join_selected_through_edits();
            }
            CanvasAction::OpenPasteAttributesDialog => {
                self.paste_attrs_dialog_open = true;
            }
            CanvasAction::ClipRollTrimmed {
                track_id,
                left_clip_id,
                right_clip_id,
                delta_secs,
            } => {
                let cmd = Box::new(trim_commands::RollTrimCommand::new(
                    track_id,
                    left_clip_id,
                    right_clip_id,
                    delta_secs,
                ));
                if self
                    .undo_stack
                    .push(cmd, &mut self.project.timeline)
                    .is_ok()
                {
                    self.project.mark_dirty();
                    self.last_rendered_dirty = true;
                    self.status_message = format!("Roll Trim uygulandı ({:+.2}s)", delta_secs);
                }
            }
            CanvasAction::ClipSlideTrimmed {
                track_id,
                clip_id,
                left_clip_id,
                right_clip_id,
                delta_secs,
            } => {
                let cmd = Box::new(trim_commands::SlideTrimCommand::new(
                    track_id,
                    clip_id,
                    left_clip_id,
                    right_clip_id,
                    delta_secs,
                ));
                if self
                    .undo_stack
                    .push(cmd, &mut self.project.timeline)
                    .is_ok()
                {
                    self.project.mark_dirty();
                    self.last_rendered_dirty = true;
                    self.status_message = format!("Slide Trim uygulandı ({:+.2}s)", delta_secs);
                }
            }
        }
    }
}
