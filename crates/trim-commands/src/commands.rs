//! Complete implementation of all 14 timeline editing commands.

use crate::{Command, CommandError};
use trim_core::clip::Clip;
use trim_core::marker::Marker;
use trim_core::timeline::TimelineModel;
use trim_core::track::{Track, TrackType};

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
// 3. MoveClipCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct MoveClipCommand {
    pub track_id: String,
    pub clip_id: String,
    pub old_in: f64,
    pub new_in: f64,
}

impl MoveClipCommand {
    pub fn new(track_id: impl Into<String>, clip_id: impl Into<String>, old_in: f64, new_in: f64) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            old_in,
            new_in,
        }
    }
}

impl Command for MoveClipCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.move_to(self.new_in);
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
        clip.move_to(self.old_in);
        track.sort_clips();
        Ok(())
    }

    fn name(&self) -> &str {
        "Klip Taşı"
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
            return Err(CommandError::InvalidOperation("Kesim noktası klip sınırları dışında".into()));
        }

        let split_source = orig.map_timeline_to_source(self.split_time);

        // Left clip keeps original clip id
        let mut left_clip = orig.clone();
        left_clip.timeline_out = self.split_time;
        left_clip.source_out = Some(split_source);

        // Right clip gets new id
        let mut right_clip = orig.clone();
        right_clip.id = uuid::Uuid::new_v4().to_string();
        right_clip.timeline_in = self.split_time;
        right_clip.source_in = split_source;
        self.right_clip_id = Some(right_clip.id.clone());

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

        // Find existing audio track or create new one
        let audio_track_id = if let Some(at) = timeline.get_audio_tracks().first() {
            self.was_track_created = false;
            at.id.clone()
        } else {
            let new_at = Track::new("A1", TrackType::Audio);
            let at_id = new_at.id.clone();
            timeline.add_track(new_at);
            self.was_track_created = true;
            at_id
        };

        let mut audio_clip = Clip::new(media_id, tin, tout);
        audio_clip.source_in = sin;
        audio_clip.source_out = sout;
        audio_clip.name = format!("{} (Ses)", name);
        self.created_audio_clip_id = Some(audio_clip.id.clone());
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
        if let (Some(at_id), Some(ac_id)) = (&self.created_audio_track_id, &self.created_audio_clip_id) {
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
    pub fn new(track_id: impl Into<String>, clip_id: impl Into<String>, playhead_time: f64) -> Self {
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
            return Err(CommandError::InvalidOperation("Geçersiz kırpma konumu".into()));
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
    pub fn new(track_id: impl Into<String>, clip_id: impl Into<String>, playhead_time: f64) -> Self {
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
            return Err(CommandError::InvalidOperation("Geçersiz kırpma konumu".into()));
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
        timeline
            .markers
            .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
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
            timeline
                .markers
                .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
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

        clip.speed = self.new_speed;
        clip.reverse = self.new_reverse;

        let src_span = clip.effective_source_out() - clip.source_in;
        let new_dur = (src_span / self.new_speed).max(0.04);
        clip.timeline_out = clip.timeline_in + new_dur;
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
        track.sort_clips();
        Ok(())
    }

    fn name(&self) -> &str {
        "Hız ve Yön Değiştir"
    }
}
