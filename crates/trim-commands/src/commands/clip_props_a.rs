use crate::{ClipBounds, Command, CommandError, CompanionTrimBounds};
use trim_core::clip::{AudioEqPreset, ColorLabel, ColorWheelsConfig};
use trim_core::timeline::TimelineModel;

// 18. CloseGapCommand (Ripple Delete Gap)
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct CloseGapCommand {
    pub track_id: Option<String>,
    pub gap_start: f64,
    pub gap_end: f64,
    pub shifted_tracks: Vec<String>,
    pub shifted_clips: Vec<(String, String, f64)>,
    pub shift_amount: f64,
}

impl CloseGapCommand {
    pub fn new(track_id: Option<String>, gap_start: f64, gap_end: f64) -> Self {
        Self {
            track_id,
            gap_start,
            gap_end,
            shifted_tracks: Vec::new(),
            shifted_clips: Vec::new(),
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
        self.shifted_clips.clear();

        if let Some(ref tid) = self.track_id {
            let track = timeline
                .get_track_by_id_mut(tid)
                .ok_or_else(|| CommandError::TrackNotFound(tid.clone()))?;
            if track.locked {
                return Err(CommandError::InvalidOperation(
                    "Kilitli kanaldaki boşluk kapatılamaz".into(),
                ));
            }
            for c in &mut track.clips {
                if c.timeline_in >= self.gap_end - 1e-4 {
                    self.shifted_clips
                        .push((tid.clone(), c.id.clone(), c.timeline_in));
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
                            self.shifted_clips.push((
                                track.id.clone(),
                                c.id.clone(),
                                c.timeline_in,
                            ));
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
        for (tid, cid, orig_in) in &self.shifted_clips {
            if let Some(track) = timeline.get_track_by_id_mut(tid) {
                if let Some(c) = track.get_clip_by_id_mut(cid) {
                    c.move_to(*orig_in);
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
        let len = self.commands.len();
        for (undid, cmd) in self.commands.iter_mut().rev().enumerate() {
            if let Err(err) = cmd.undo(timeline) {
                for c in self.commands[len - undid..].iter_mut() {
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
    pub old_bounds: ClipBounds,
    pub new_bounds: ClipBounds,
    pub shifted_clips: Vec<(String, String, f64, f64)>, // (track_id, clip_id, old_in, new_in)
    pub companions: Vec<CompanionTrimBounds>,
}

impl RippleTrimEdgeCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_bounds: ClipBounds,
        new_bounds: ClipBounds,
        shifted_clips: Vec<(String, String, f64, f64)>,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            old_bounds,
            new_bounds,
            shifted_clips,
            companions: Vec::new(),
        }
    }

    pub fn with_companions(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_bounds: ClipBounds,
        new_bounds: ClipBounds,
        shifted_clips: Vec<(String, String, f64, f64)>,
        companions: Vec<CompanionTrimBounds>,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            old_bounds,
            new_bounds,
            shifted_clips,
            companions,
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

        for (comp_tr_id, comp_cl_id, _, new_b) in &self.companions {
            if let Some(tr) = timeline.get_track_by_id_mut(comp_tr_id) {
                if let Some(c) = tr.get_clip_by_id_mut(comp_cl_id) {
                    c.timeline_in = new_b.0;
                    c.timeline_out = new_b.1;
                    c.source_in = new_b.2;
                    c.source_out = Some(new_b.3);
                }
            }
        }

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

        for (comp_tr_id, comp_cl_id, old_b, _) in &self.companions {
            if let Some(tr) = timeline.get_track_by_id_mut(comp_tr_id) {
                if let Some(c) = tr.get_clip_by_id_mut(comp_cl_id) {
                    c.timeline_in = old_b.0;
                    c.timeline_out = old_b.1;
                    c.source_in = old_b.2;
                    c.source_out = Some(old_b.3);
                }
            }
        }

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

// -----------------------------------------------------------------------------
// 27. SetClipFadeCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct SetClipFadeCommand {
    pub track_id: String,
    pub clip_id: String,
    pub is_fade_in: bool,
    pub old_fade: f64,
    pub new_fade: f64,
}

impl SetClipFadeCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        is_fade_in: bool,
        old_fade: f64,
        new_fade: f64,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            is_fade_in,
            old_fade,
            new_fade,
        }
    }
}

impl Command for SetClipFadeCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        if self.is_fade_in {
            clip.fade_in = self.new_fade;
        } else {
            clip.fade_out = self.new_fade;
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
        if self.is_fade_in {
            clip.fade_in = self.old_fade;
        } else {
            clip.fade_out = self.old_fade;
        }
        Ok(())
    }

    fn name(&self) -> &str {
        if self.is_fade_in {
            "Fade In Ayarla"
        } else {
            "Fade Out Ayarla"
        }
    }
}

// -----------------------------------------------------------------------------
