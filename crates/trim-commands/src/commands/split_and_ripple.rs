use super::split_single_clip_core;
use crate::{Command, CommandError};
use trim_core::clip::Clip;
use trim_core::timeline::TimelineModel;

#[derive(Debug, Clone)]
pub struct SplitClipCommand {
    pub track_id: String,
    pub clip_id: String,
    pub split_time: f64,
    pub split_linked: bool,
    pub original_clip: Option<Clip>,
    pub right_clip_id: Option<String>,
    /// Linked companion clips split alongside the target: (track_id, original_clip, right_clip_id)
    pub companions: Vec<(String, Clip, Option<String>)>,
    pub new_left_group_id: Option<String>,
    pub new_right_group_id: Option<String>,
}

impl SplitClipCommand {
    pub fn new(track_id: impl Into<String>, clip_id: impl Into<String>, split_time: f64) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            split_time,
            split_linked: true,
            original_clip: None,
            right_clip_id: None,
            companions: Vec::new(),
            new_left_group_id: None,
            new_right_group_id: None,
        }
    }

    pub fn with_split_linked(mut self, split_linked: bool) -> Self {
        self.split_linked = split_linked;
        self
    }
}

impl Command for SplitClipCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        if track.locked {
            return Err(CommandError::InvalidOperation(
                "Kilitli kanaldaki klip bölünemez".into(),
            ));
        }
        let orig = track
            .get_clip_by_id(&self.clip_id)
            .cloned()
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;

        if self.split_time <= orig.timeline_in || self.split_time >= orig.timeline_out {
            return Err(CommandError::InvalidOperation(
                "Kesim noktası klip sınırları dışında".into(),
            ));
        }

        // Collect all linked companion clips on unlocked tracks
        let mut companion_targets: Vec<(String, Clip)> = Vec::new();
        if self.split_linked {
            if let Some(ref gid) = orig.linked_group_id {
                for t in &timeline.tracks {
                    if !t.locked {
                        for c in &t.clips {
                            if (t.id != self.track_id || c.id != self.clip_id)
                                && c.linked_group_id.as_ref() == Some(gid)
                                && self.split_time > c.timeline_in + 1e-4
                                && self.split_time < c.timeline_out - 1e-4
                            {
                                companion_targets.push((t.id.clone(), c.clone()));
                            }
                        }
                    }
                }
            }
        }

        // Generate or reuse link groups if any companion exists
        let (new_left_gid, new_right_gid) = if !companion_targets.is_empty() {
            let left_g = self
                .new_left_group_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let right_g = self
                .new_right_group_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            self.new_left_group_id = Some(left_g.clone());
            self.new_right_group_id = Some(right_g.clone());
            (Some(left_g), Some(right_g))
        } else {
            (orig.linked_group_id.clone(), orig.linked_group_id.clone())
        };

        let (mut left_clip, mut right_clip, r_id) =
            split_single_clip_core(&orig, self.split_time, self.right_clip_id.clone());
        left_clip.linked_group_id = new_left_gid.clone();
        right_clip.linked_group_id = new_right_gid.clone();
        self.right_clip_id = Some(r_id);
        self.original_clip = Some(orig);

        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        track.remove_clip(&self.clip_id);
        track.add_clip(left_clip);
        track.add_clip(right_clip);

        // Split all linked companion clips as well, reusing right_clip_id on redo
        let prev_companions = std::mem::take(&mut self.companions);
        for (idx, (comp_tr_id, comp_orig)) in companion_targets.into_iter().enumerate() {
            let reuse_id = prev_companions.get(idx).and_then(|(_, _, r)| r.clone());
            let (mut comp_left, mut comp_right, comp_r_id) =
                split_single_clip_core(&comp_orig, self.split_time, reuse_id);
            comp_left.linked_group_id = new_left_gid.clone();
            comp_right.linked_group_id = new_right_gid.clone();

            if let Some(comp_track) = timeline.get_track_by_id_mut(&comp_tr_id) {
                comp_track.remove_clip(&comp_orig.id);
                comp_track.add_clip(comp_left);
                comp_track.add_clip(comp_right);
            }
            self.companions
                .push((comp_tr_id, comp_orig, Some(comp_r_id)));
        }

        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        if let Some(orig) = self.original_clip.take() {
            let track = timeline
                .get_track_by_id_mut(&self.track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
            track.remove_clip(&self.clip_id);
            if let Some(r_id) = &self.right_clip_id {
                track.remove_clip(r_id);
            }
            track.add_clip(orig);
        }

        for (comp_tr_id, comp_orig, comp_r_id_opt) in self.companions.iter_mut() {
            if let Some(comp_track) = timeline.get_track_by_id_mut(comp_tr_id) {
                comp_track.remove_clip(&comp_orig.id);
                if let Some(comp_r_id) = comp_r_id_opt {
                    comp_track.remove_clip(comp_r_id);
                }
                comp_track.add_clip(comp_orig.clone());
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "Klibi Böl"
    }
}

// -----------------------------------------------------------------------------
// 5b. BatchSplitClipCommand (Smart Cut / Scene Detection)
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct BatchSplitClipCommand {
    pub track_id: String,
    pub clip_id: String,
    pub split_times: Vec<f64>,
    pub original_clip: Option<Clip>,
    pub created_clip_ids: Vec<String>,
}

impl BatchSplitClipCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        mut split_times: Vec<f64>,
    ) -> Self {
        split_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        split_times.dedup_by(|a, b| (*a - *b).abs() < 0.05);
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            split_times,
            original_clip: None,
            created_clip_ids: Vec::new(),
        }
    }
}

impl Command for BatchSplitClipCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let orig = track
            .get_clip_by_id(&self.clip_id)
            .cloned()
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;

        // Filter valid split times strictly within clip boundaries
        let valid_splits: Vec<f64> = self
            .split_times
            .iter()
            .copied()
            .filter(|&t| t > orig.timeline_in + 0.05 && t < orig.timeline_out - 0.05)
            .collect();

        if valid_splits.is_empty() {
            return Ok(());
        }

        let mut boundaries = vec![orig.timeline_in];
        boundaries.extend_from_slice(&valid_splits);
        boundaries.push(orig.timeline_out);

        let mut segments = Vec::new();
        let mut ids = Vec::new();

        for i in 0..boundaries.len() - 1 {
            let t_in = boundaries[i];
            let t_out = boundaries[i + 1];
            let dur = t_out - t_in;
            if dur < 0.04 {
                continue;
            }

            let s_in = orig.map_timeline_to_source(t_in);
            let s_out = orig.map_timeline_to_source(t_out);

            let mut seg = orig.clone();
            seg.id = if i == 0 {
                orig.id.clone()
            } else if let Some(prev_id) = self.created_clip_ids.get(i) {
                prev_id.clone()
            } else {
                uuid::Uuid::new_v4().to_string()
            };
            seg.timeline_in = t_in;
            seg.timeline_out = t_out;
            if !orig.reverse {
                seg.source_in = s_in;
                seg.source_out = Some(s_out);
            } else {
                seg.source_in = s_out;
                seg.source_out = Some(s_in);
            }
            let seg_rel_start = t_in - orig.timeline_in;
            let seg_rel_end = t_out - orig.timeline_in;
            seg.keyframes.clear();
            for (prop, list) in &orig.keyframes {
                let mut seg_list = Vec::new();
                for k in list {
                    if k.time >= seg_rel_start - 1e-6 && k.time <= seg_rel_end + 1e-6 {
                        let mut nk = k.clone();
                        nk.time = (nk.time - seg_rel_start).max(0.0);
                        seg_list.push(nk);
                    }
                }
                if !seg_list.is_empty() {
                    seg.keyframes.insert(prop.clone(), seg_list);
                }
            }
            for (mask_idx, mask) in seg.masks.iter_mut().enumerate() {
                mask.keyframes.clear();
                if let Some(orig_mask) = orig.masks.get(mask_idx) {
                    for (prop, list) in &orig_mask.keyframes {
                        let mut seg_list = Vec::new();
                        for k in list {
                            if k.time >= seg_rel_start - 1e-6 && k.time <= seg_rel_end + 1e-6 {
                                let mut nk = k.clone();
                                nk.time = (nk.time - seg_rel_start).max(0.0);
                                seg_list.push(nk);
                            }
                        }
                        if !seg_list.is_empty() {
                            mask.keyframes.insert(prop.clone(), seg_list);
                        }
                    }
                }
            }
            if i > 0 {
                seg.fade_in = 0.0;
                seg.transition_in = None;
            }
            if i < boundaries.len() - 2 {
                seg.fade_out = 0.0;
                seg.transition_out = None;
            }
            ids.push(seg.id.clone());
            segments.push(seg);
        }

        self.created_clip_ids = ids;
        self.original_clip = Some(orig);

        track.remove_clip(&self.clip_id);
        for seg in segments {
            track.add_clip(seg);
        }

        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        if let Some(orig) = self.original_clip.take() {
            let track = timeline
                .get_track_by_id_mut(&self.track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;

            for id in &self.created_clip_ids {
                track.remove_clip(id);
            }
            track.remove_clip(&orig.id);
            track.add_clip(orig);
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Sahneleri Böl (Smart Cut)"
    }
}

// -----------------------------------------------------------------------------
// 6. RippleDeleteCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct RippleDeleteCommand {
    pub track_id: String,
    pub clip_id: String,
    pub ripple_all_unlocked: bool,
    pub removed_clip: Option<Clip>,
    pub removed_companions: Vec<(String, Clip)>,
    pub shift_amount: f64,
    pub shifted_clips: Vec<(String, String, f64)>, // (track_id, clip_id, orig_in)
}

impl RippleDeleteCommand {
    pub fn new(track_id: impl Into<String>, clip_id: impl Into<String>) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            ripple_all_unlocked: true,
            removed_clip: None,
            removed_companions: Vec::new(),
            shift_amount: 0.0,
            shifted_clips: Vec::new(),
        }
    }

    pub fn with_all_unlocked(mut self, all_unlocked: bool) -> Self {
        self.ripple_all_unlocked = all_unlocked;
        self
    }
}

impl Command for RippleDeleteCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .remove_clip(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;

        let dur = clip.duration();
        let cut_point = clip.timeline_out;
        let linked_gid = clip.linked_group_id.clone();
        self.shift_amount = dur;
        self.removed_clip = Some(clip);
        self.removed_companions.clear();
        self.shifted_clips.clear();

        // Remove all linked companion clips on unlocked tracks
        if let Some(ref gid) = linked_gid {
            for t in &mut timeline.tracks {
                if !t.locked {
                    let mut idx = 0;
                    while idx < t.clips.len() {
                        if t.clips[idx].linked_group_id.as_ref() == Some(gid) {
                            let comp = t.clips.remove(idx);
                            self.removed_companions.push((t.id.clone(), comp));
                        } else {
                            idx += 1;
                        }
                    }
                }
            }
        }

        // Compute maximum safe shift so downstream clips don't overlap earlier clips on any unlocked track
        let mut safe_shift = dur;
        for t in &timeline.tracks {
            if t.locked || (!self.ripple_all_unlocked && t.id != self.track_id) {
                continue;
            }
            let earliest_after = t
                .clips
                .iter()
                .filter(|c| c.timeline_in >= cut_point - 1e-4)
                .map(|c| c.timeline_in)
                .fold(f64::INFINITY, f64::min);
            if earliest_after.is_finite() {
                let latest_before_end = t
                    .clips
                    .iter()
                    .filter(|c| c.timeline_in < cut_point - 1e-4)
                    .map(|c| c.timeline_out)
                    .fold(0.0_f64, f64::max);
                let gap = (earliest_after - latest_before_end).max(0.0);
                safe_shift = safe_shift.min(gap);
            }
        }
        self.shift_amount = safe_shift;

        if self.ripple_all_unlocked {
            for t in &mut timeline.tracks {
                if !t.locked {
                    for c in &mut t.clips {
                        if c.timeline_in >= cut_point - 1e-4 {
                            self.shifted_clips
                                .push((t.id.clone(), c.id.clone(), c.timeline_in));
                            c.move_to((c.timeline_in - safe_shift).max(0.0));
                        }
                    }
                    t.sort_clips();
                }
            }
        } else {
            let track = timeline
                .get_track_by_id_mut(&self.track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
            for c in &mut track.clips {
                if c.timeline_in >= cut_point - 1e-4 {
                    self.shifted_clips
                        .push((self.track_id.clone(), c.id.clone(), c.timeline_in));
                    c.move_to((c.timeline_in - safe_shift).max(0.0));
                }
            }
            track.sort_clips();
        }

        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        // Restore all shifted clips back to their exact original positions
        for (t_id, c_id, orig_in) in &self.shifted_clips {
            if let Some(t) = timeline.get_track_by_id_mut(t_id) {
                if let Some(c) = t.get_clip_by_id_mut(c_id) {
                    c.move_to(*orig_in);
                }
                t.sort_clips();
            }
        }

        // Restore all companion clips if removed
        for (comp_tr_id, comp_clip) in self.removed_companions.drain(..) {
            if let Some(comp_track) = timeline.get_track_by_id_mut(&comp_tr_id) {
                comp_track.add_clip(comp_clip);
            }
        }

        // Restore primary clip
        if let Some(clip) = self.removed_clip.take() {
            let track = timeline
                .get_track_by_id_mut(&self.track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
            track.add_clip(clip);
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "Boşluksuz Sil"
    }
}

// -----------------------------------------------------------------------------
// 7. MoveMultipleClipsCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct MoveMultipleClipsCommand {
    pub moves: Vec<(String, String, f64, f64)>, // (track_id, clip_id, old_in, new_in)
}

impl MoveMultipleClipsCommand {
    pub fn new(moves: Vec<(String, String, f64, f64)>) -> Self {
        Self { moves }
    }
}

impl Command for MoveMultipleClipsCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        for (tr_id, cl_id, _, new_in) in &self.moves {
            if let Some(track) = timeline.get_track_by_id_mut(tr_id) {
                if let Some(clip) = track.get_clip_by_id_mut(cl_id) {
                    clip.move_to(*new_in);
                }
                track.sort_clips();
            }
        }
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        for (tr_id, cl_id, old_in, _) in &self.moves {
            if let Some(track) = timeline.get_track_by_id_mut(tr_id) {
                if let Some(clip) = track.get_clip_by_id_mut(cl_id) {
                    clip.move_to(*old_in);
                }
                track.sort_clips();
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Çoklu Klip Taşı"
    }
}

// -----------------------------------------------------------------------------
// 8. RemoveMultipleClipsCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct RemoveMultipleClipsCommand {
    pub targets: Vec<(String, String)>, // (track_id, clip_id)
    pub removed_clips: Vec<(String, Clip)>,
}

impl RemoveMultipleClipsCommand {
    pub fn new(targets: Vec<(String, String)>) -> Self {
        Self {
            targets,
            removed_clips: Vec::new(),
        }
    }
}

impl Command for RemoveMultipleClipsCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        self.removed_clips.clear();
        for (tr_id, cl_id) in &self.targets {
            if let Some(track) = timeline.get_track_by_id_mut(tr_id) {
                if let Some(clip) = track.remove_clip(cl_id) {
                    self.removed_clips.push((tr_id.clone(), clip));
                }
            }
        }
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        for (tr_id, clip) in self.removed_clips.drain(..) {
            if let Some(track) = timeline.get_track_by_id_mut(&tr_id) {
                track.add_clip(clip);
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Çoklu Klip Sil"
    }
}

// -----------------------------------------------------------------------------
// 9. DetachAudioCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct DetachAudioCommand {
    pub video_track_id: String,
    pub video_clip_id: String,
    pub created_audio_track_id: Option<String>,
    pub created_audio_clip_id: Option<String>,
    pub was_track_created: bool,
    pub prev_muted: Option<bool>,
}

impl DetachAudioCommand {
    pub fn new(video_track_id: impl Into<String>, video_clip_id: impl Into<String>) -> Self {
        Self {
            video_track_id: video_track_id.into(),
            video_clip_id: video_clip_id.into(),
            created_audio_track_id: None,
            created_audio_clip_id: None,
            was_track_created: false,
            prev_muted: None,
        }
    }
}

impl Command for DetachAudioCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let (media_id, tin, tout, sin, sout, name) = {
            let track = timeline
                .get_track_by_id_mut(&self.video_track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.video_track_id.clone()))?;
            if track.locked {
                return Err(CommandError::InvalidOperation(
                    "Kilitli kanaldaki klipin sesi ayrılamaz".into(),
                ));
            }
            let clip = track
                .get_clip_by_id_mut(&self.video_clip_id)
                .ok_or_else(|| CommandError::ClipNotFound(self.video_clip_id.clone()))?;
            if self.prev_muted.is_none() {
                self.prev_muted = Some(clip.muted);
            }
            clip.muted = true;
            (
                clip.media_id.clone(),
                clip.timeline_in,
                clip.timeline_out,
                clip.source_in,
                clip.source_out,
                clip.name.clone(),
            )
        };

        // Find corresponding audio track (V1->A1, V2->A2, etc.) or create matching track
        let track_count_before = timeline.tracks.len();
        let audio_track_id =
            timeline.find_or_create_corresponding_audio_track(&self.video_track_id);
        self.was_track_created = timeline.tracks.len() > track_count_before;

        let mut audio_clip = Clip::new(media_id, tin, tout);
        audio_clip.source_in = sin;
        audio_clip.source_out = sout;
        audio_clip.name = format!("{} (Ses)", name);
        if let Some(ref prev_id) = self.created_audio_clip_id {
            audio_clip.id = prev_id.clone();
        } else {
            self.created_audio_clip_id = Some(audio_clip.id.clone());
        }
        self.created_audio_track_id = Some(audio_track_id.clone());

        if let Some(at) = timeline.get_track_by_id_mut(&audio_track_id) {
            at.add_clip(audio_clip);
        }
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        // Restore video clip's previous muted state
        if let Some(track) = timeline.get_track_by_id_mut(&self.video_track_id) {
            if let Some(clip) = track.get_clip_by_id_mut(&self.video_clip_id) {
                clip.muted = self.prev_muted.unwrap_or(false);
            }
        }

        // Remove created audio clip
        if let (Some(at_id), Some(ac_id)) =
            (&self.created_audio_track_id, &self.created_audio_clip_id)
        {
            if let Some(at) = timeline.get_track_by_id_mut(at_id) {
                at.remove_clip(ac_id);
            }
        }

        // If audio track was newly created, remove it
        if self.was_track_created {
            if let Some(at_id) = &self.created_audio_track_id {
                timeline.remove_track(at_id);
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Sesi Ayır"
    }
}

// -----------------------------------------------------------------------------
