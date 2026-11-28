use crate::app::TrimApp;
use trim_commands::{AddClipCommand, RemoveMultipleClipsCommand};
use trim_core::clip::Clip;

impl TrimApp {
    /// Nests all currently selected clips into a new Compound Clip (sub-sequence) and replaces them on the timeline.
    pub fn create_compound_clip_from_selected(&mut self) {
        if self.canvas_state.selected_clip_pairs.is_empty() {
            self.set_status(
                "Bileşik klip (Compound Clip) oluşturmak için en az 1 klip seçin",
                "Select at least 1 clip to create a Compound Clip",
            );
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
            self.set_status(
                "Seçili kliplerden bileşik klip oluşturulamadı",
                "Could not create compound clip from selected clips",
            );
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

        let pairs_to_remove: Vec<(String, String)> = self
            .canvas_state
            .selected_clip_pairs
            .iter()
            .filter(|(tr_id, _)| {
                self.project
                    .timeline
                    .get_track_by_id(tr_id)
                    .is_some_and(|t| !t.locked)
            })
            .cloned()
            .collect();
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
            self.status_message = match self.lang() {
                crate::i18n::Language::Turkish => {
                    format!("📦 Bileşik klip oluşturuldu: {}", seq_name)
                }
                crate::i18n::Language::English => {
                    format!("📦 Compound clip created: {}", seq_name)
                }
            };
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
            self.set_status(
                "Birleştirilecek kesik klip (Through-Edit) bulunamadı (Aynı kaynaktan ardışık iki kesik klip seçin)",
                "No through-edit cut found to join (select two consecutive clips from the same source)",
            );
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
            self.status_message = match self.lang() {
                crate::i18n::Language::Turkish => {
                    format!("🔗 {} kesik nokta tek klipte birleştirildi", count)
                }
                crate::i18n::Language::English => {
                    format!("🔗 Joined {} through-edit cut(s) into one clip", count)
                }
            };
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
            self.set_status(
                "Panoda özellikleri kopyalanmış klip yok (Önce Ctrl+C yapın)",
                "No clip attributes in clipboard (press Ctrl+C first)",
            );
            return;
        };
        if self.canvas_state.selected_clip_pairs.is_empty() {
            self.set_status(
                "Özelliklerin uygulanacağı hedef klip(leri) seçin",
                "Select target clip(s) to paste attributes",
            );
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
            self.status_message = match self.lang() {
                crate::i18n::Language::Turkish => {
                    format!("📋 Seçili özellikler {} klibe yapıştırıldı", applied_count)
                }
                crate::i18n::Language::English => {
                    format!("📋 Pasted selected attributes to {} clip(s)", applied_count)
                }
            };
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
        self.status_message = match self.lang() {
            crate::i18n::Language::Turkish => format!(
                "🎥 Multicam Kamera {} seçildi ({:.2}s)",
                angle_idx + 1,
                cut_t
            ),
            crate::i18n::Language::English => format!(
                "🎥 Multicam Camera {} selected ({:.2}s)",
                angle_idx + 1,
                cut_t
            ),
        };
    }
}
