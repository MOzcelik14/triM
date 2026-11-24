use crate::{Command, CommandError};
use trim_core::clip::Clip;
use trim_core::marker::Marker;
use trim_core::timeline::TimelineModel;

// 10. RippleTrimHeadCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct RippleTrimHeadCommand {
    pub track_id: String,
    pub clip_id: String,
    pub playhead_time: f64,
    pub ripple_all_unlocked: bool,
    pub orig_bounds: Option<(f64, f64, f64, f64)>,
    pub orig_clip: Option<Clip>,
    pub companion_bounds: Option<(String, String, f64, f64, f64, f64)>,
    pub orig_companion: Option<(String, Clip)>,
    pub shift_amount: f64,
    pub shifted_clips: Vec<(String, String, f64)>,
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
            ripple_all_unlocked: true,
            orig_bounds: None,
            orig_clip: None,
            companion_bounds: None,
            orig_companion: None,
            shift_amount: 0.0,
            shifted_clips: Vec::new(),
        }
    }

    pub fn with_all_unlocked(mut self, all_unlocked: bool) -> Self {
        self.ripple_all_unlocked = all_unlocked;
        self
    }
}

impl Command for RippleTrimHeadCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        if track.locked {
            return Err(CommandError::InvalidOperation(
                "Kilitli kanaldaki klip kırpılamaz".into(),
            ));
        }
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;

        let orig_in = clip.timeline_in;
        let orig_out = clip.timeline_out;
        let orig_sin = clip.source_in;
        let orig_sout = clip.effective_source_out();
        let linked_gid = clip.linked_group_id.clone();

        let dt = self.playhead_time - orig_in;
        if !dt.is_finite() || dt <= 0.04 || dt >= clip.duration() - 0.04 {
            return Err(CommandError::InvalidOperation(
                "Geçersiz kırpma konumu".into(),
            ));
        }

        self.orig_bounds = Some((orig_in, orig_out, orig_sin, orig_sout));
        self.orig_clip = Some(clip.clone());

        clip.trim_in(self.playhead_time);
        clip.move_to(orig_in); // shorten duration
        self.shift_amount = dt;
        self.shifted_clips.clear();
        self.orig_companion = None;

        // Also trim companion clip if linked
        if let Some(ref gid) = linked_gid {
            for t in &mut timeline.tracks {
                if t.id != self.track_id && !t.locked {
                    if let Some(comp) = t
                        .clips
                        .iter_mut()
                        .find(|c| c.linked_group_id.as_ref() == Some(gid))
                    {
                        self.companion_bounds = Some((
                            t.id.clone(),
                            comp.id.clone(),
                            comp.timeline_in,
                            comp.timeline_out,
                            comp.source_in,
                            comp.effective_source_out(),
                        ));
                        self.orig_companion = Some((t.id.clone(), comp.clone()));
                        comp.trim_in(self.playhead_time);
                        comp.move_to(orig_in);
                        break;
                    }
                }
            }
        }

        let ignore_c_id = self.clip_id.clone();
        let ignore_comp_id = self
            .companion_bounds
            .as_ref()
            .map(|(_, id, _, _, _, _)| id.clone());

        if self.ripple_all_unlocked {
            for t in &mut timeline.tracks {
                if !t.locked {
                    for c in &mut t.clips {
                        if c.id != ignore_c_id
                            && Some(&c.id) != ignore_comp_id.as_ref()
                            && c.timeline_in >= orig_out - 1e-4
                        {
                            self.shifted_clips
                                .push((t.id.clone(), c.id.clone(), c.timeline_in));
                            c.move_to((c.timeline_in - dt).max(0.0));
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
                if c.id != ignore_c_id && c.timeline_in >= orig_out - 1e-4 {
                    self.shifted_clips
                        .push((self.track_id.clone(), c.id.clone(), c.timeline_in));
                    c.move_to((c.timeline_in - dt).max(0.0));
                }
            }
            track.sort_clips();
        }

        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        for (t_id, c_id, orig_in) in &self.shifted_clips {
            if let Some(t) = timeline.get_track_by_id_mut(t_id) {
                if let Some(c) = t.get_clip_by_id_mut(c_id) {
                    c.move_to(*orig_in);
                }
                t.sort_clips();
            }
        }

        if let Some((comp_tr_id, orig_comp)) = self.orig_companion.take() {
            self.companion_bounds = None;
            if let Some(t) = timeline.get_track_by_id_mut(&comp_tr_id) {
                if let Some(comp) = t.get_clip_by_id_mut(&orig_comp.id) {
                    *comp = orig_comp;
                }
                t.sort_clips();
            }
        }

        if let Some(orig_clip) = self.orig_clip.take() {
            self.orig_bounds = None;
            let track = timeline
                .get_track_by_id_mut(&self.track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
            if let Some(clip) = track.get_clip_by_id_mut(&self.clip_id) {
                *clip = orig_clip;
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
    pub ripple_all_unlocked: bool,
    pub orig_bounds: Option<(f64, f64, f64, f64)>,
    pub companion_bounds: Option<(String, String, f64, f64, f64, f64)>,
    pub shift_amount: f64,
    pub shifted_clips: Vec<(String, String, f64)>,
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
            ripple_all_unlocked: true,
            orig_bounds: None,
            companion_bounds: None,
            shift_amount: 0.0,
            shifted_clips: Vec::new(),
        }
    }

    pub fn with_all_unlocked(mut self, all_unlocked: bool) -> Self {
        self.ripple_all_unlocked = all_unlocked;
        self
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
        let linked_gid = clip.linked_group_id.clone();
        self.orig_bounds = Some((orig_in, orig_out, orig_sin, orig_sout));

        let dt = orig_out - self.playhead_time;
        if !dt.is_finite() || dt <= 0.04 || dt >= clip.duration() - 0.04 {
            return Err(CommandError::InvalidOperation(
                "Geçersiz kırpma konumu".into(),
            ));
        }

        clip.trim_out(self.playhead_time);
        self.shift_amount = dt;
        self.shifted_clips.clear();

        // Also trim companion clip if linked
        if let Some(ref gid) = linked_gid {
            for t in &mut timeline.tracks {
                if t.id != self.track_id && !t.locked {
                    if let Some(comp) = t
                        .clips
                        .iter_mut()
                        .find(|c| c.linked_group_id.as_ref() == Some(gid))
                    {
                        self.companion_bounds = Some((
                            t.id.clone(),
                            comp.id.clone(),
                            comp.timeline_in,
                            comp.timeline_out,
                            comp.source_in,
                            comp.effective_source_out(),
                        ));
                        comp.trim_out(self.playhead_time);
                        break;
                    }
                }
            }
        }

        let ignore_c_id = self.clip_id.clone();
        let ignore_comp_id = self
            .companion_bounds
            .as_ref()
            .map(|(_, id, _, _, _, _)| id.clone());

        if self.ripple_all_unlocked {
            for t in &mut timeline.tracks {
                if !t.locked {
                    for c in &mut t.clips {
                        if c.id != ignore_c_id
                            && Some(&c.id) != ignore_comp_id.as_ref()
                            && c.timeline_in >= orig_out - 1e-4
                        {
                            self.shifted_clips
                                .push((t.id.clone(), c.id.clone(), c.timeline_in));
                            c.move_to((c.timeline_in - dt).max(0.0));
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
                if c.id != ignore_c_id && c.timeline_in >= orig_out - 1e-4 {
                    self.shifted_clips
                        .push((self.track_id.clone(), c.id.clone(), c.timeline_in));
                    c.move_to((c.timeline_in - dt).max(0.0));
                }
            }
            track.sort_clips();
        }

        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        for (t_id, c_id, orig_in) in &self.shifted_clips {
            if let Some(t) = timeline.get_track_by_id_mut(t_id) {
                if let Some(c) = t.get_clip_by_id_mut(c_id) {
                    c.move_to(*orig_in);
                }
                t.sort_clips();
            }
        }

        if let Some((comp_tr_id, comp_id, cin, cout, csin, csout)) = self.companion_bounds.take() {
            if let Some(t) = timeline.get_track_by_id_mut(&comp_tr_id) {
                if let Some(comp) = t.get_clip_by_id_mut(&comp_id) {
                    comp.timeline_in = cin;
                    comp.timeline_out = cout;
                    comp.source_in = csin;
                    comp.source_out = Some(csout);
                }
                t.sort_clips();
            }
        }

        if let Some((oin, oout, osin, osout)) = self.orig_bounds.take() {
            let track = timeline
                .get_track_by_id_mut(&self.track_id)
                .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
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
            new_speed: if new_speed.is_finite() {
                new_speed.clamp(0.1, 10.0)
            } else {
                1.0
            },
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
    pub shifted_clips: Vec<(String, f64)>,
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
            shifted_clips: Vec::new(),
        }
    }
}

impl Command for AutoSilenceCutCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        if track.locked {
            return Err(CommandError::InvalidOperation(
                "Kilitli kanal düzenlenemez".into(),
            ));
        }

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
        self.shifted_clips.clear();

        if shift > 1e-4 {
            for c in &mut track.clips {
                if c.timeline_in >= old_end_tl - 1e-4 {
                    self.shifted_clips.push((c.id.clone(), c.timeline_in));
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

        // Restore exact positions of shifted subsequent clips
        for (cid, orig_in) in &self.shifted_clips {
            if let Some(c) = track.clips.iter_mut().find(|c| &c.id == cid) {
                c.move_to(*orig_in);
            }
        }
        self.shifted_clips.clear();

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
