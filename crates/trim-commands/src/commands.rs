//! Complete implementation of all 14 timeline editing commands.

use crate::{Command, CommandError};
use trim_core::clip::{AudioEqPreset, Clip, ColorLabel, ColorWheelsConfig};
use trim_core::marker::Marker;
use trim_core::timeline::TimelineModel;
use trim_core::track::Track;

// -----------------------------------------------------------------------------
// 1. AddClipCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct AddClipCommand {
    pub track_id: String,
    pub clip: Clip,
}

impl AddClipCommand {
    pub fn new(track_id: impl Into<String>, clip: Clip) -> Self {
        Self {
            track_id: track_id.into(),
            clip,
        }
    }
}

impl Command for AddClipCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        track.add_clip(self.clip.clone());
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        track.remove_clip(&self.clip.id);
        Ok(())
    }

    fn name(&self) -> &str {
        "Klip Ekle"
    }
}

// -----------------------------------------------------------------------------
// 2. RemoveClipCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct RemoveClipCommand {
    pub track_id: String,
    pub clip_id: String,
    pub removed_clip: Option<Clip>,
}

impl RemoveClipCommand {
    pub fn new(track_id: impl Into<String>, clip_id: impl Into<String>) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            removed_clip: None,
        }
    }
}

impl Command for RemoveClipCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        self.removed_clip = track.remove_clip(&self.clip_id);
        if self.removed_clip.is_none() {
            return Err(CommandError::ClipNotFound(self.clip_id.clone()));
        }
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        if let Some(clip) = self.removed_clip.take() {
            let track = timeline
                .get_track_by_id_mut(&self.track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
            track.add_clip(clip);
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Klip Sil"
    }
}

// -----------------------------------------------------------------------------
// 3. MoveClipCommand (Supports cross-track vertical moves)
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct MoveClipCommand {
    pub old_track_id: String,
    pub new_track_id: String,
    pub clip_id: String,
    pub old_in: f64,
    pub new_in: f64,
}

impl MoveClipCommand {
    pub fn new(
        old_track_id: impl Into<String>,
        new_track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_in: f64,
        new_in: f64,
    ) -> Self {
        Self {
            old_track_id: old_track_id.into(),
            new_track_id: new_track_id.into(),
            clip_id: clip_id.into(),
            old_in,
            new_in,
        }
    }
}

impl Command for MoveClipCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        if self.old_track_id == self.new_track_id {
            let track = timeline
                .get_track_by_id_mut(&self.old_track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.old_track_id.clone()))?;
            let clip = track
                .get_clip_by_id_mut(&self.clip_id)
                .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
            clip.move_to(self.new_in);
            track.sort_clips();
        } else {
            let mut clip = {
                let old_track = timeline
                    .get_track_by_id_mut(&self.old_track_id)
                    .ok_or_else(|| CommandError::TrackNotFound(self.old_track_id.clone()))?;
                old_track
                    .remove_clip(&self.clip_id)
                    .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?
            };
            clip.move_to(self.new_in);
            let new_track = timeline
                .get_track_by_id_mut(&self.new_track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.new_track_id.clone()))?;
            new_track.add_clip(clip);
        }
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        if self.old_track_id == self.new_track_id {
            let track = timeline
                .get_track_by_id_mut(&self.old_track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.old_track_id.clone()))?;
            let clip = track
                .get_clip_by_id_mut(&self.clip_id)
                .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
            clip.move_to(self.old_in);
            track.sort_clips();
        } else {
            let mut clip = {
                let new_track = timeline
                    .get_track_by_id_mut(&self.new_track_id)
                    .ok_or_else(|| CommandError::TrackNotFound(self.new_track_id.clone()))?;
                new_track
                    .remove_clip(&self.clip_id)
                    .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?
            };
            clip.move_to(self.old_in);
            let old_track = timeline
                .get_track_by_id_mut(&self.old_track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.old_track_id.clone()))?;
            old_track.add_clip(clip);
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Klip Taşı"
    }
}

// -----------------------------------------------------------------------------
// 3b. AddTrackCommand & RemoveTrackCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct AddTrackCommand {
    pub track: Track,
    pub insert_index: usize,
}

impl AddTrackCommand {
    pub fn new(track: Track, insert_index: usize) -> Self {
        Self {
            track,
            insert_index,
        }
    }
}

impl Command for AddTrackCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let idx = self.insert_index.min(timeline.tracks.len());
        timeline.tracks.insert(idx, self.track.clone());
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        timeline.remove_track(&self.track.id);
        Ok(())
    }

    fn name(&self) -> &str {
        "Kanal Ekle"
    }
}

#[derive(Debug, Clone)]
pub struct RemoveTrackCommand {
    pub track: Option<Track>,
    pub track_id: String,
    pub original_index: usize,
}

impl RemoveTrackCommand {
    pub fn new(track_id: impl Into<String>) -> Self {
        Self {
            track: None,
            track_id: track_id.into(),
            original_index: 0,
        }
    }
}

impl Command for RemoveTrackCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let pos = timeline
            .tracks
            .iter()
            .position(|t| t.id == self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        self.original_index = pos;
        let removed = timeline
            .remove_track_safe(&self.track_id)
            .map_err(|e| CommandError::InvalidOperation(e.to_string()))?;
        self.track = Some(removed);
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        if let Some(track) = self.track.take() {
            let idx = self.original_index.min(timeline.tracks.len());
            timeline.tracks.insert(idx, track);
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Kanal Sil"
    }
}

// -----------------------------------------------------------------------------
// 4. TrimClipCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct TrimClipCommand {
    pub track_id: String,
    pub clip_id: String,
    pub old_bounds: (f64, f64, f64, f64), // (tin, tout, sin, sout)
    pub new_bounds: (f64, f64, f64, f64),
}

impl TrimClipCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_bounds: (f64, f64, f64, f64),
        new_bounds: (f64, f64, f64, f64),
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            old_bounds,
            new_bounds,
        }
    }
}

impl Command for TrimClipCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.timeline_in = self.new_bounds.0;
        clip.timeline_out = self.new_bounds.1;
        clip.source_in = self.new_bounds.2;
        clip.source_out = Some(self.new_bounds.3);
        track.sort_clips();
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.timeline_in = self.old_bounds.0;
        clip.timeline_out = self.old_bounds.1;
        clip.source_in = self.old_bounds.2;
        clip.source_out = Some(self.old_bounds.3);
        track.sort_clips();
        Ok(())
    }

    fn name(&self) -> &str {
        "Klip Kırp"
    }
}

// -----------------------------------------------------------------------------
// 5. SplitClipCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct SplitClipCommand {
    pub track_id: String,
    pub clip_id: String,
    pub split_time: f64,
    pub original_clip: Option<Clip>,
    pub right_clip_id: Option<String>,
}

impl SplitClipCommand {
    pub fn new(track_id: impl Into<String>, clip_id: impl Into<String>, split_time: f64) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            split_time,
            original_clip: None,
            right_clip_id: None,
        }
    }
}

impl Command for SplitClipCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let orig = track
            .get_clip_by_id(&self.clip_id)
            .cloned()
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;

        if self.split_time <= orig.timeline_in || self.split_time >= orig.timeline_out {
            return Err(CommandError::InvalidOperation(
                "Kesim noktası klip sınırları dışında".into(),
            ));
        }

        let split_source = orig.map_timeline_to_source(self.split_time);
        let left_dur = self.split_time - orig.timeline_in;

        // Partition keyframes between left and right parts
        let mut left_kfs = std::collections::HashMap::new();
        let mut right_kfs = std::collections::HashMap::new();
        for (prop, list) in &orig.keyframes {
            let mut l_list = Vec::new();
            let mut r_list = Vec::new();
            for k in list {
                if k.time <= left_dur {
                    l_list.push(k.clone());
                } else {
                    let mut r_k = k.clone();
                    r_k.time = (r_k.time - left_dur).max(0.0);
                    r_list.push(r_k);
                }
            }
            if !l_list.is_empty() {
                left_kfs.insert(prop.clone(), l_list);
            }
            if !r_list.is_empty() {
                right_kfs.insert(prop.clone(), r_list);
            }
        }

        // Left clip keeps original clip id
        let mut left_clip = orig.clone();
        left_clip.timeline_out = self.split_time;
        if !orig.reverse {
            left_clip.source_out = Some(split_source);
        } else {
            left_clip.source_in = split_source;
        }
        left_clip.fade_out = 0.0;
        left_clip.transition_out = None;
        left_clip.keyframes = left_kfs;

        // Right clip preserves ID if re-executing (Redo), or gets new UUID on first execute
        let mut right_clip = orig.clone();
        let r_id = self
            .right_clip_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        right_clip.id = r_id.clone();
        right_clip.timeline_in = self.split_time;
        if !orig.reverse {
            right_clip.source_in = split_source;
        } else {
            right_clip.source_out = Some(split_source);
        }
        right_clip.fade_in = 0.0;
        right_clip.transition_in = None;
        right_clip.keyframes = right_kfs;
        self.right_clip_id = Some(r_id);

        self.original_clip = Some(orig);

        track.remove_clip(&self.clip_id);
        track.add_clip(left_clip);
        track.add_clip(right_clip);
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
    pub removed_clip: Option<Clip>,
    pub shift_amount: f64,
}

impl RippleDeleteCommand {
    pub fn new(track_id: impl Into<String>, clip_id: impl Into<String>) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            removed_clip: None,
            shift_amount: 0.0,
        }
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
        self.shift_amount = dur;
        self.removed_clip = Some(clip);

        // Shift subsequent clips left
        for c in &mut track.clips {
            if c.timeline_in >= cut_point - 1e-4 {
                c.move_to(c.timeline_in - dur);
            }
        }
        track.sort_clips();
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        if let Some(clip) = self.removed_clip.take() {
            let cut_point = clip.timeline_out;
            let dur = self.shift_amount;
            let track = timeline
                .get_track_by_id_mut(&self.track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;

            // Shift subsequent clips back right
            for c in &mut track.clips {
                if c.timeline_in >= cut_point - dur - 1e-4 {
                    c.move_to(c.timeline_in + dur);
                }
            }
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
}

impl DetachAudioCommand {
    pub fn new(video_track_id: impl Into<String>, video_clip_id: impl Into<String>) -> Self {
        Self {
            video_track_id: video_track_id.into(),
            video_clip_id: video_clip_id.into(),
            created_audio_track_id: None,
            created_audio_clip_id: None,
            was_track_created: false,
        }
    }
}

impl Command for DetachAudioCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let (media_id, tin, tout, sin, sout, name) = {
            let track = timeline
                .get_track_by_id_mut(&self.video_track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.video_track_id.clone()))?;
            let clip = track
                .get_clip_by_id_mut(&self.video_clip_id)
                .ok_or_else(|| CommandError::ClipNotFound(self.video_clip_id.clone()))?;
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
        // Unmute video clip
        if let Some(track) = timeline.get_track_by_id_mut(&self.video_track_id) {
            if let Some(clip) = track.get_clip_by_id_mut(&self.video_clip_id) {
                clip.muted = false;
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
// 10. RippleTrimHeadCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct RippleTrimHeadCommand {
    pub track_id: String,
    pub clip_id: String,
    pub playhead_time: f64,
    pub orig_bounds: Option<(f64, f64, f64, f64)>,
    pub shift_amount: f64,
}

impl RippleTrimHeadCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        playhead_time: f64,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            playhead_time,
            orig_bounds: None,
            shift_amount: 0.0,
        }
    }
}

impl Command for RippleTrimHeadCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;

        let orig_in = clip.timeline_in;
        let orig_out = clip.timeline_out;
        let orig_sin = clip.source_in;
        let orig_sout = clip.effective_source_out();
        self.orig_bounds = Some((orig_in, orig_out, orig_sin, orig_sout));

        let dt = self.playhead_time - orig_in;
        if dt <= 0.04 || dt >= clip.duration() - 0.04 {
            return Err(CommandError::InvalidOperation(
                "Geçersiz kırpma konumu".into(),
            ));
        }

        clip.trim_in(self.playhead_time);
        clip.move_to(orig_in); // shorten duration
        self.shift_amount = dt;

        // Shift subsequent clips left
        for c in &mut track.clips {
            if c.id != self.clip_id && c.timeline_in >= orig_out - 1e-4 {
                c.move_to(c.timeline_in - dt);
            }
        }
        track.sort_clips();
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        if let Some((oin, oout, osin, osout)) = self.orig_bounds {
            let dt = self.shift_amount;
            let track = timeline
                .get_track_by_id_mut(&self.track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;

            // Shift subsequent clips back right
            for c in &mut track.clips {
                if c.id != self.clip_id && c.timeline_in >= oout - dt - 1e-4 {
                    c.move_to(c.timeline_in + dt);
                }
            }

            if let Some(clip) = track.get_clip_by_id_mut(&self.clip_id) {
                clip.timeline_in = oin;
                clip.timeline_out = oout;
                clip.source_in = osin;
                clip.source_out = Some(osout);
            }
            track.sort_clips();
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Başı Çizgiye Kırp ve Kaydır"
    }
}

// -----------------------------------------------------------------------------
// 11. RippleTrimTailCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct RippleTrimTailCommand {
    pub track_id: String,
    pub clip_id: String,
    pub playhead_time: f64,
    pub orig_bounds: Option<(f64, f64, f64, f64)>,
    pub shift_amount: f64,
}

impl RippleTrimTailCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        playhead_time: f64,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            playhead_time,
            orig_bounds: None,
            shift_amount: 0.0,
        }
    }
}

impl Command for RippleTrimTailCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;

        let orig_in = clip.timeline_in;
        let orig_out = clip.timeline_out;
        let orig_sin = clip.source_in;
        let orig_sout = clip.effective_source_out();
        self.orig_bounds = Some((orig_in, orig_out, orig_sin, orig_sout));

        let dt = orig_out - self.playhead_time;
        if dt <= 0.04 || dt >= clip.duration() - 0.04 {
            return Err(CommandError::InvalidOperation(
                "Geçersiz kırpma konumu".into(),
            ));
        }

        clip.trim_out(self.playhead_time);
        self.shift_amount = dt;

        // Shift subsequent clips left
        for c in &mut track.clips {
            if c.id != self.clip_id && c.timeline_in >= orig_out - 1e-4 {
                c.move_to(c.timeline_in - dt);
            }
        }
        track.sort_clips();
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        if let Some((oin, oout, osin, osout)) = self.orig_bounds {
            let dt = self.shift_amount;
            let track = timeline
                .get_track_by_id_mut(&self.track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;

            // Shift subsequent clips back right
            for c in &mut track.clips {
                if c.id != self.clip_id && c.timeline_in >= oout - dt - 1e-4 {
                    c.move_to(c.timeline_in + dt);
                }
            }

            if let Some(clip) = track.get_clip_by_id_mut(&self.clip_id) {
                clip.timeline_in = oin;
                clip.timeline_out = oout;
                clip.source_in = osin;
                clip.source_out = Some(osout);
            }
            track.sort_clips();
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Sonu Çizgiye Kırp ve Kaydır"
    }
}

// -----------------------------------------------------------------------------
// 12. AddMarkerCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct AddMarkerCommand {
    pub marker: Marker,
}

impl AddMarkerCommand {
    pub fn new(marker: Marker) -> Self {
        Self { marker }
    }
}

impl Command for AddMarkerCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        timeline.markers.push(self.marker.clone());
        timeline.markers.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        timeline.remove_marker(&self.marker.id);
        Ok(())
    }

    fn name(&self) -> &str {
        "İşaretçi Ekle"
    }
}

// -----------------------------------------------------------------------------
// 13. RemoveMarkerCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct RemoveMarkerCommand {
    pub marker_id: String,
    pub removed_marker: Option<Marker>,
}

impl RemoveMarkerCommand {
    pub fn new(marker_id: impl Into<String>) -> Self {
        Self {
            marker_id: marker_id.into(),
            removed_marker: None,
        }
    }
}

impl Command for RemoveMarkerCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        self.removed_marker = timeline.remove_marker(&self.marker_id);
        if self.removed_marker.is_none() {
            return Err(CommandError::InvalidOperation("İşaretçi bulunamadı".into()));
        }
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        if let Some(m) = self.removed_marker.take() {
            timeline.markers.push(m);
            timeline.markers.sort_by(|a, b| {
                a.time
                    .partial_cmp(&b.time)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "İşaretçi Sil"
    }
}

// -----------------------------------------------------------------------------
// 14. ChangeClipSpeedCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct ChangeClipSpeedCommand {
    pub track_id: String,
    pub clip_id: String,
    pub new_speed: f64,
    pub new_reverse: bool,
    pub old_speed: f64,
    pub old_reverse: bool,
    pub old_timeline_out: f64,
    pub old_source_out: Option<f64>,
}

impl ChangeClipSpeedCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        new_speed: f64,
        new_reverse: bool,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            new_speed: new_speed.clamp(0.1, 10.0),
            new_reverse,
            old_speed: 1.0,
            old_reverse: false,
            old_timeline_out: 0.0,
            old_source_out: None,
        }
    }
}

impl Command for ChangeClipSpeedCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;

        self.old_speed = clip.speed;
        self.old_reverse = clip.reverse;
        self.old_timeline_out = clip.timeline_out;
        self.old_source_out = clip.source_out;

        clip.change_speed(self.new_speed);
        clip.reverse = self.new_reverse;
        track.sort_clips();
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;

        clip.speed = self.old_speed;
        clip.reverse = self.old_reverse;
        clip.timeline_out = self.old_timeline_out;
        clip.source_out = self.old_source_out;
        track.sort_clips();
        Ok(())
    }

    fn name(&self) -> &str {
        "Hız ve Yön Değiştir"
    }
}

// -----------------------------------------------------------------------------
// 15. AutoSilenceCutCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct AutoSilenceCutCommand {
    pub track_id: String,
    pub original_clip_id: String,
    pub speech_intervals: Vec<(f64, f64)>, // (source_in, source_out) in seconds
    pub original_clip: Option<Clip>,
    pub generated_clips: Vec<Clip>,
    pub shift_amount: f64,
}

impl AutoSilenceCutCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        speech_intervals: Vec<(f64, f64)>,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            original_clip_id: clip_id.into(),
            speech_intervals,
            original_clip: None,
            generated_clips: Vec::new(),
            shift_amount: 0.0,
        }
    }
}

impl Command for AutoSilenceCutCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;

        let orig = track
            .remove_clip(&self.original_clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.original_clip_id.clone()))?;

        let old_end_tl = orig.timeline_out;
        let mut current_tl = orig.timeline_in;
        let mut new_clips = Vec::new();

        for &(sp_in, sp_out) in &self.speech_intervals {
            let src_span = (sp_out - sp_in).max(0.04);
            let tl_dur = src_span / orig.speed.max(0.1);
            let mut sub = orig.clone();
            sub.id = uuid::Uuid::new_v4().to_string();
            sub.source_in = sp_in;
            sub.source_out = Some(sp_out);
            sub.timeline_in = current_tl;
            sub.timeline_out = current_tl + tl_dur;
            current_tl += tl_dur;
            track.add_clip(sub.clone());
            new_clips.push(sub);
        }

        let new_end_tl = current_tl;
        let shift = (old_end_tl - new_end_tl).max(0.0);
        self.shift_amount = shift;

        if shift > 1e-4 {
            for c in &mut track.clips {
                if c.timeline_in >= old_end_tl - 1e-4 {
                    c.move_to((c.timeline_in - shift).max(0.0));
                }
            }
        }

        track.sort_clips();
        self.original_clip = Some(orig);
        self.generated_clips = new_clips;
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;

        // Remove generated clips
        for c in &self.generated_clips {
            track.remove_clip(&c.id);
        }

        // Shift subsequent clips back right
        if self.shift_amount > 1e-4 {
            if let Some(orig) = &self.original_clip {
                let shifted_boundary = orig.timeline_out - self.shift_amount;
                for c in &mut track.clips {
                    if c.timeline_in >= shifted_boundary - 1e-4 {
                        c.move_to(c.timeline_in + self.shift_amount);
                    }
                }
            }
        }

        // Restore original clip
        if let Some(orig) = self.original_clip.take() {
            track.add_clip(orig);
        }

        track.sort_clips();
        self.generated_clips.clear();
        Ok(())
    }

    fn name(&self) -> &str {
        "Sessizlikleri Otomatik Kes (Jump-Cut)"
    }
}

// -----------------------------------------------------------------------------
// 16. LinkClipsCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct LinkClipsCommand {
    pub clip_pairs: Vec<(String, String)>, // (track_id, clip_id)
    pub previous_group_ids: Vec<Option<String>>,
    pub new_group_id: String,
}

impl LinkClipsCommand {
    pub fn new(clip_pairs: Vec<(String, String)>) -> Self {
        let new_group_id = uuid::Uuid::new_v4().to_string();
        Self {
            clip_pairs,
            previous_group_ids: Vec::new(),
            new_group_id,
        }
    }
}

impl Command for LinkClipsCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        self.previous_group_ids.clear();
        for (tr_id, cl_id) in &self.clip_pairs {
            let track = timeline
                .get_track_by_id_mut(tr_id)
                .ok_or_else(|| CommandError::TrackNotFound(tr_id.clone()))?;
            let clip = track
                .get_clip_by_id_mut(cl_id)
                .ok_or_else(|| CommandError::ClipNotFound(cl_id.clone()))?;
            self.previous_group_ids.push(clip.linked_group_id.clone());
            clip.linked_group_id = Some(self.new_group_id.clone());
        }
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        for ((tr_id, cl_id), old_gid) in self.clip_pairs.iter().zip(self.previous_group_ids.iter())
        {
            if let Some(track) = timeline.get_track_by_id_mut(tr_id) {
                if let Some(clip) = track.get_clip_by_id_mut(cl_id) {
                    clip.linked_group_id = old_gid.clone();
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Klipleri Bağla (Link)"
    }
}

// -----------------------------------------------------------------------------
// 17. UnlinkClipsCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct UnlinkClipsCommand {
    pub clip_pairs: Vec<(String, String)>, // (track_id, clip_id)
    pub previous_group_ids: Vec<Option<String>>,
}

impl UnlinkClipsCommand {
    pub fn new(clip_pairs: Vec<(String, String)>) -> Self {
        Self {
            clip_pairs,
            previous_group_ids: Vec::new(),
        }
    }
}

impl Command for UnlinkClipsCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        self.previous_group_ids.clear();
        for (tr_id, cl_id) in &self.clip_pairs {
            let track = timeline
                .get_track_by_id_mut(tr_id)
                .ok_or_else(|| CommandError::TrackNotFound(tr_id.clone()))?;
            let clip = track
                .get_clip_by_id_mut(cl_id)
                .ok_or_else(|| CommandError::ClipNotFound(cl_id.clone()))?;
            self.previous_group_ids.push(clip.linked_group_id.clone());
            clip.linked_group_id = None;
        }
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        for ((tr_id, cl_id), old_gid) in self.clip_pairs.iter().zip(self.previous_group_ids.iter())
        {
            if let Some(track) = timeline.get_track_by_id_mut(tr_id) {
                if let Some(clip) = track.get_clip_by_id_mut(cl_id) {
                    clip.linked_group_id = old_gid.clone();
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Klip Bağını Kaldır (Unlink)"
    }
}

// -----------------------------------------------------------------------------
// 18. CloseGapCommand (Ripple Delete Gap)
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct CloseGapCommand {
    pub track_id: Option<String>,
    pub gap_start: f64,
    pub gap_end: f64,
    pub shifted_tracks: Vec<String>,
    pub shift_amount: f64,
}

impl CloseGapCommand {
    pub fn new(track_id: Option<String>, gap_start: f64, gap_end: f64) -> Self {
        Self {
            track_id,
            gap_start,
            gap_end,
            shifted_tracks: Vec::new(),
            shift_amount: (gap_end - gap_start).max(0.0),
        }
    }
}

impl Command for CloseGapCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let dur = (self.gap_end - self.gap_start).max(0.0);
        if dur <= 1e-4 {
            return Ok(());
        }
        self.shift_amount = dur;
        self.shifted_tracks.clear();

        if let Some(ref tid) = self.track_id {
            let track = timeline
                .get_track_by_id_mut(tid)
                .ok_or_else(|| CommandError::TrackNotFound(tid.clone()))?;
            for c in &mut track.clips {
                if c.timeline_in >= self.gap_end - 1e-4 {
                    c.move_to((c.timeline_in - dur).max(0.0));
                }
            }
            track.sort_clips();
            self.shifted_tracks.push(tid.clone());
        } else {
            for track in &mut timeline.tracks {
                if !track.locked {
                    for c in &mut track.clips {
                        if c.timeline_in >= self.gap_end - 1e-4 {
                            c.move_to((c.timeline_in - dur).max(0.0));
                        }
                    }
                    track.sort_clips();
                    self.shifted_tracks.push(track.id.clone());
                }
            }
        }
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        if self.shift_amount <= 1e-4 {
            return Ok(());
        }
        for tid in &self.shifted_tracks {
            if let Some(track) = timeline.get_track_by_id_mut(tid) {
                for c in &mut track.clips {
                    if c.timeline_in >= self.gap_start - 1e-4 {
                        c.move_to(c.timeline_in + self.shift_amount);
                    }
                }
                track.sort_clips();
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Boşluğu Kapat (Ripple Delete Gap)"
    }
}

// -----------------------------------------------------------------------------
// 19. SetClipColorCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct SetClipColorCommand {
    pub track_id: String,
    pub clip_id: String,
    pub new_color: ColorLabel,
    pub old_color: ColorLabel,
}

impl SetClipColorCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        new_color: ColorLabel,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            new_color,
            old_color: ColorLabel::None,
        }
    }
}

impl Command for SetClipColorCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        self.old_color = clip.color_label;
        clip.color_label = self.new_color;
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.color_label = self.old_color;
        Ok(())
    }

    fn name(&self) -> &str {
        "Klip Rengini Değiştir"
    }
}

// -----------------------------------------------------------------------------
// 17. SetClipVolumeCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct SetClipVolumeCommand {
    pub track_id: String,
    pub clip_id: String,
    pub new_volume: f64,
    pub old_volume: f64,
}

impl SetClipVolumeCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_volume: f64,
        new_volume: f64,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            new_volume,
            old_volume,
        }
    }
}

impl Command for SetClipVolumeCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.volume = self.new_volume;
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.volume = self.old_volume;
        Ok(())
    }

    fn name(&self) -> &str {
        "Klip Ses Seviyesini Değiştir"
    }
}

// -----------------------------------------------------------------------------
// 24. MacroCommand (Compound / Batch atomic command)
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub struct MacroCommand {
    pub name: String,
    pub commands: Vec<Box<dyn Command>>,
}

impl MacroCommand {
    pub fn new(name: impl Into<String>, commands: Vec<Box<dyn Command>>) -> Self {
        Self {
            name: name.into(),
            commands,
        }
    }
}

impl Command for MacroCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        for (executed, cmd) in self.commands.iter_mut().enumerate() {
            if let Err(err) = cmd.execute(timeline) {
                for c in self.commands[..executed].iter_mut().rev() {
                    let _ = c.undo(timeline);
                }
                return Err(err);
            }
        }
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        for (undid, cmd) in self.commands.iter_mut().rev().enumerate() {
            if let Err(err) = cmd.undo(timeline) {
                for c in self.commands.iter_mut().rev().take(undid) {
                    let _ = c.execute(timeline);
                }
                return Err(err);
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// -----------------------------------------------------------------------------
// 21. SetClipTransformCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct SetClipTransformCommand {
    pub track_id: String,
    pub clip_id: String,
    pub old_pos_x: f64,
    pub old_pos_y: f64,
    pub old_scale: f64,
    pub old_rotation: f64,
    pub old_opacity: f64,
    pub new_pos_x: f64,
    pub new_pos_y: f64,
    pub new_scale: f64,
    pub new_rotation: f64,
    pub new_opacity: f64,
}

impl SetClipTransformCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_transform: (f64, f64, f64, f64, f64),
        new_transform: (f64, f64, f64, f64, f64),
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            old_pos_x: old_transform.0,
            old_pos_y: old_transform.1,
            old_scale: old_transform.2,
            old_rotation: old_transform.3,
            old_opacity: old_transform.4,
            new_pos_x: new_transform.0,
            new_pos_y: new_transform.1,
            new_scale: new_transform.2,
            new_rotation: new_transform.3,
            new_opacity: new_transform.4,
        }
    }
}

impl Command for SetClipTransformCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.pos_x = self.new_pos_x;
        clip.pos_y = self.new_pos_y;
        clip.scale = self.new_scale;
        clip.rotation = self.new_rotation;
        clip.opacity = self.new_opacity;
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.pos_x = self.old_pos_x;
        clip.pos_y = self.old_pos_y;
        clip.scale = self.old_scale;
        clip.rotation = self.old_rotation;
        clip.opacity = self.old_opacity;
        Ok(())
    }

    fn name(&self) -> &str {
        "Dönüşümü Değiştir"
    }
}

// -----------------------------------------------------------------------------
// 22. SetClipColorWheelsCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct SetClipColorWheelsCommand {
    pub track_id: String,
    pub clip_id: String,
    pub old_wheels: Option<ColorWheelsConfig>,
    pub new_wheels: Option<ColorWheelsConfig>,
}

impl SetClipColorWheelsCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_wheels: Option<ColorWheelsConfig>,
        new_wheels: Option<ColorWheelsConfig>,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            old_wheels,
            new_wheels,
        }
    }
}

impl Command for SetClipColorWheelsCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.color_wheels = self.new_wheels.clone();
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.color_wheels = self.old_wheels.clone();
        Ok(())
    }

    fn name(&self) -> &str {
        "Renk Tekerleklerini Değiştir"
    }
}

// -----------------------------------------------------------------------------
// 23. SetClipEqPresetCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct SetClipEqPresetCommand {
    pub track_id: String,
    pub clip_id: String,
    pub old_preset: AudioEqPreset,
    pub new_preset: AudioEqPreset,
}

impl SetClipEqPresetCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_preset: AudioEqPreset,
        new_preset: AudioEqPreset,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            old_preset,
            new_preset,
        }
    }
}

impl Command for SetClipEqPresetCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.eq_preset = self.new_preset;
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.eq_preset = self.old_preset;
        Ok(())
    }

    fn name(&self) -> &str {
        "EQ Profilini Değiştir"
    }
}

// -----------------------------------------------------------------------------
// 24. SetClipCropCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct SetClipCropCommand {
    pub track_id: String,
    pub clip_id: String,
    pub old_crop: (f32, f32, f32, f32),
    pub new_crop: (f32, f32, f32, f32),
}

impl SetClipCropCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_crop: (f32, f32, f32, f32),
        new_crop: (f32, f32, f32, f32),
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            old_crop,
            new_crop,
        }
    }
}

impl Command for SetClipCropCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.crop_left = self.new_crop.0;
        clip.crop_right = self.new_crop.1;
        clip.crop_top = self.new_crop.2;
        clip.crop_bottom = self.new_crop.3;
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.crop_left = self.old_crop.0;
        clip.crop_right = self.old_crop.1;
        clip.crop_top = self.old_crop.2;
        clip.crop_bottom = self.old_crop.3;
        Ok(())
    }

    fn name(&self) -> &str {
        "Kırpmayı Değiştir"
    }
}

// -----------------------------------------------------------------------------
// 25. RippleTrimEdgeCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct RippleTrimEdgeCommand {
    pub track_id: String,
    pub clip_id: String,
    pub old_bounds: (f64, f64, f64, f64),
    pub new_bounds: (f64, f64, f64, f64),
    pub shifted_clips: Vec<(String, String, f64, f64)>, // (track_id, clip_id, old_in, new_in)
}

impl RippleTrimEdgeCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_bounds: (f64, f64, f64, f64),
        new_bounds: (f64, f64, f64, f64),
        shifted_clips: Vec<(String, String, f64, f64)>,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            old_bounds,
            new_bounds,
            shifted_clips,
        }
    }
}

impl Command for RippleTrimEdgeCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;

        clip.timeline_in = self.new_bounds.0;
        clip.timeline_out = self.new_bounds.1;
        clip.source_in = self.new_bounds.2;
        clip.source_out = Some(self.new_bounds.3);

        for (tr_id, cl_id, _, new_in) in &self.shifted_clips {
            if let Some(tr) = timeline.get_track_by_id_mut(tr_id) {
                if let Some(c) = tr.get_clip_by_id_mut(cl_id) {
                    c.move_to(*new_in);
                }
            }
        }
        for track in &mut timeline.tracks {
            track.sort_clips();
        }
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;

        clip.timeline_in = self.old_bounds.0;
        clip.timeline_out = self.old_bounds.1;
        clip.source_in = self.old_bounds.2;
        clip.source_out = Some(self.old_bounds.3);

        for (tr_id, cl_id, old_in, _) in &self.shifted_clips {
            if let Some(tr) = timeline.get_track_by_id_mut(tr_id) {
                if let Some(c) = tr.get_clip_by_id_mut(cl_id) {
                    c.move_to(*old_in);
                }
            }
        }
        for track in &mut timeline.tracks {
            track.sort_clips();
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Dalgalanma Düzenleme (Ripple Trim)"
    }
}

// -----------------------------------------------------------------------------
// 26. SetClipKeyframesCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct SetClipKeyframesCommand {
    pub track_id: String,
    pub clip_id: String,
    pub property: String,
    pub old_keyframes: Vec<trim_core::keyframe::Keyframe>,
    pub new_keyframes: Vec<trim_core::keyframe::Keyframe>,
}

impl SetClipKeyframesCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        property: impl Into<String>,
        old_keyframes: Vec<trim_core::keyframe::Keyframe>,
        new_keyframes: Vec<trim_core::keyframe::Keyframe>,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            property: property.into(),
            old_keyframes,
            new_keyframes,
        }
    }
}

impl Command for SetClipKeyframesCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;

        if self.new_keyframes.is_empty() {
            clip.keyframes.remove(&self.property);
        } else {
            clip.keyframes
                .insert(self.property.clone(), self.new_keyframes.clone());
        }
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;

        if self.old_keyframes.is_empty() {
            clip.keyframes.remove(&self.property);
        } else {
            clip.keyframes
                .insert(self.property.clone(), self.old_keyframes.clone());
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Anahtar Kareleri Değiştir"
    }
}




