use crate::{Command, CommandError};
use trim_core::clip::Clip;
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
            let old_locked = timeline
                .get_track_by_id(&self.old_track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.old_track_id.clone()))?
                .locked;
            let new_locked = timeline
                .get_track_by_id(&self.new_track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.new_track_id.clone()))?
                .locked;
            if old_locked || new_locked {
                return Err(CommandError::InvalidOperation(
                    "Kilitli kanaldaki klip taşınamaz".into(),
                ));
            }
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
            if timeline.get_track_by_id(&self.old_track_id).is_none() {
                return Err(CommandError::TrackNotFound(self.old_track_id.clone()));
            }
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
pub type ClipBounds = (f64, f64, f64, f64);
pub type CompanionTrimBounds = (String, String, ClipBounds, ClipBounds);

#[derive(Debug, Clone)]
pub struct TrimClipCommand {
    pub track_id: String,
    pub clip_id: String,
    pub old_bounds: ClipBounds, // (tin, tout, sin, sout)
    pub new_bounds: ClipBounds,
    /// Companion clips that are trimmed along with the primary clip:
    /// (track_id, clip_id, old_bounds, new_bounds)
    pub companions: Vec<CompanionTrimBounds>,
    pub auto_trim_linked: bool,
}

impl TrimClipCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_bounds: ClipBounds,
        new_bounds: ClipBounds,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            old_bounds,
            new_bounds,
            companions: Vec::new(),
            auto_trim_linked: true,
        }
    }

    pub fn with_companions(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_bounds: ClipBounds,
        new_bounds: ClipBounds,
        companions: Vec<CompanionTrimBounds>,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            old_bounds,
            new_bounds,
            companions,
            auto_trim_linked: false,
        }
    }

    pub fn with_auto_trim_linked(mut self, auto_trim_linked: bool) -> Self {
        self.auto_trim_linked = auto_trim_linked;
        self
    }
}

impl Command for TrimClipCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let link_gid = {
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
            let gid = clip.linked_group_id.clone();
            track.sort_clips();
            gid
        };

        if self.companions.is_empty() && self.auto_trim_linked {
            if let Some(ref gid) = link_gid {
                let dt_in = self.new_bounds.0 - self.old_bounds.0;
                let dt_out = self.new_bounds.1 - self.old_bounds.1;
                let mut discovered = Vec::new();
                for tr in &timeline.tracks {
                    if !tr.locked {
                        for c in &tr.clips {
                            if (tr.id != self.track_id || c.id != self.clip_id)
                                && c.linked_group_id.as_ref() == Some(gid)
                            {
                                let old_b = (
                                    c.timeline_in,
                                    c.timeline_out,
                                    c.source_in,
                                    c.effective_source_out(),
                                );
                                let new_tin = (c.timeline_in + dt_in).max(0.0);
                                let new_tout = (c.timeline_out + dt_out).max(new_tin + 0.04);
                                let new_sin = (c.source_in + dt_in).max(0.0);
                                let new_sout =
                                    (c.effective_source_out() + dt_out).max(new_sin + 0.04);
                                discovered.push((
                                    tr.id.clone(),
                                    c.id.clone(),
                                    old_b,
                                    (new_tin, new_tout, new_sin, new_sout),
                                ));
                            }
                        }
                    }
                }
                self.companions = discovered;
            }
        }

        for (comp_tr_id, comp_cl_id, _, new_b) in &self.companions {
            if let Some(tr) = timeline.get_track_by_id_mut(comp_tr_id) {
                if let Some(c) = tr.get_clip_by_id_mut(comp_cl_id) {
                    c.timeline_in = new_b.0;
                    c.timeline_out = new_b.1;
                    c.source_in = new_b.2;
                    c.source_out = Some(new_b.3);
                    tr.sort_clips();
                }
            }
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
        track.sort_clips();

        for (comp_tr_id, comp_cl_id, old_b, _) in &self.companions {
            if let Some(tr) = timeline.get_track_by_id_mut(comp_tr_id) {
                if let Some(c) = tr.get_clip_by_id_mut(comp_cl_id) {
                    c.timeline_in = old_b.0;
                    c.timeline_out = old_b.1;
                    c.source_in = old_b.2;
                    c.source_out = Some(old_b.3);
                    tr.sort_clips();
                }
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "Klip Kırp"
    }
}

// -----------------------------------------------------------------------------
// 5. SplitClipCommand
// -----------------------------------------------------------------------------
fn partition_keyframe_map(
    src: &std::collections::HashMap<String, Vec<trim_core::keyframe::Keyframe>>,
    left_dur: f64,
) -> (
    std::collections::HashMap<String, Vec<trim_core::keyframe::Keyframe>>,
    std::collections::HashMap<String, Vec<trim_core::keyframe::Keyframe>>,
) {
    let mut left_kfs = std::collections::HashMap::new();
    let mut right_kfs = std::collections::HashMap::new();
    for (prop, list) in src {
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
    (left_kfs, right_kfs)
}

pub(super) fn split_single_clip_core(
    orig: &Clip,
    split_time: f64,
    reuse_right_id: Option<String>,
) -> (Clip, Clip, String) {
    let split_source = orig.map_timeline_to_source(split_time);
    let left_dur = split_time - orig.timeline_in;

    let (left_kfs, right_kfs) = partition_keyframe_map(&orig.keyframes, left_dur);

    // Left clip keeps original clip id
    let mut left_clip = orig.clone();
    left_clip.timeline_out = split_time;
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
    let r_id = reuse_right_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    right_clip.id = r_id.clone();
    right_clip.timeline_in = split_time;
    if !orig.reverse {
        right_clip.source_in = split_source;
    } else {
        right_clip.source_out = Some(split_source);
    }
    right_clip.fade_in = 0.0;
    right_clip.transition_in = None;
    right_clip.keyframes = right_kfs;

    for (l_mask, r_mask) in left_clip.masks.iter_mut().zip(right_clip.masks.iter_mut()) {
        let (l_mk, r_mk) = partition_keyframe_map(&l_mask.keyframes, left_dur);
        l_mask.keyframes = l_mk;
        r_mask.keyframes = r_mk;
    }

    (left_clip, right_clip, r_id)
}
