use crate::{Command, CommandError};
use trim_core::clip::{
    BrightnessContrastPreset, Clip, ColorWheelsConfig, LutPreset, WhiteBalancePreset,
};
use trim_core::color::{CustomCurvesConfig, HslCurvesConfig, HslQualifierConfig};
use trim_core::timeline::TimelineModel;

// 28. SetTextOverlayPositionCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct SetTextOverlayPositionCommand {
    pub track_id: String,
    pub clip_id: String,
    pub old_pos: (f32, f32),
    pub new_pos: (f32, f32),
}

impl SetTextOverlayPositionCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_pos: (f32, f32),
        new_pos: (f32, f32),
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            old_pos,
            new_pos,
        }
    }
}

impl Command for SetTextOverlayPositionCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        if let Some(ref mut txt) = clip.text_overlay {
            txt.pos_x = self.new_pos.0;
            txt.pos_y = self.new_pos.1;
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
        if let Some(ref mut txt) = clip.text_overlay {
            txt.pos_x = self.old_pos.0;
            txt.pos_y = self.old_pos.1;
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Metin Konumunu Değiştir"
    }
}

// -----------------------------------------------------------------------------
// 29. SetTextOverlayContentCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct SetTextOverlayContentCommand {
    pub track_id: String,
    pub clip_id: String,
    pub old_content: String,
    pub new_content: String,
    pub old_name: String,
    pub new_name: String,
}

impl SetTextOverlayContentCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_content: impl Into<String>,
        new_content: impl Into<String>,
        old_name: impl Into<String>,
        new_name: impl Into<String>,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            old_content: old_content.into(),
            new_content: new_content.into(),
            old_name: old_name.into(),
            new_name: new_name.into(),
        }
    }
}

impl Command for SetTextOverlayContentCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        if let Some(ref mut txt) = clip.text_overlay {
            txt.content = self.new_content.clone();
        }
        clip.name = self.new_name.clone();
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        if let Some(ref mut txt) = clip.text_overlay {
            txt.content = self.old_content.clone();
        }
        clip.name = self.old_name.clone();
        Ok(())
    }

    fn name(&self) -> &str {
        "Metin İçeriğini Değiştir"
    }
}

// -----------------------------------------------------------------------------
// 30. SetClipColorGradingCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
pub struct ClipColorGradingState {
    pub camera_log_profile: trim_core::color::CameraLogProfile,
    pub film_format: trim_core::color::FilmStockFormat,
    pub highlight_rolloff: f32,
    pub diffusion: f32,
    pub mist: f32,
    pub gate_weave: f32,
    pub brightness: f64,
    pub contrast: f64,
    pub saturation: f64,
    pub gamma: f64,
    pub temperature: f64,
    pub tint: f64,
    pub wb_preset: WhiteBalancePreset,
    pub bc_preset: BrightnessContrastPreset,
    pub lut_preset: LutPreset,
    pub custom_lut_path: Option<String>,
    pub color_wheels: Option<ColorWheelsConfig>,
    pub blur: f32,
    pub sharpen: f32,
    pub vignette: f32,
    pub film_grain: f32,
    pub halation: f32,
    pub bloom: f32,
    pub chromatic_aberration: f32,
    pub custom_curves: Option<CustomCurvesConfig>,
    pub hsl_curves: Option<HslCurvesConfig>,
    pub hsl_qualifier: Option<HslQualifierConfig>,
}

impl ClipColorGradingState {
    pub fn from_clip(clip: &Clip) -> Self {
        Self {
            camera_log_profile: clip.camera_log_profile,
            film_format: clip.film_format,
            highlight_rolloff: clip.highlight_rolloff,
            diffusion: clip.diffusion,
            mist: clip.mist,
            gate_weave: clip.gate_weave,
            brightness: clip.brightness,
            contrast: clip.contrast,
            saturation: clip.saturation,
            gamma: clip.gamma,
            temperature: clip.temperature,
            tint: clip.tint,
            wb_preset: clip.wb_preset,
            bc_preset: clip.bc_preset,
            lut_preset: clip.lut_preset,
            custom_lut_path: clip.custom_lut_path.clone(),
            color_wheels: clip.color_wheels.clone(),
            blur: clip.blur,
            sharpen: clip.sharpen,
            vignette: clip.vignette,
            film_grain: clip.film_grain,
            halation: clip.halation,
            bloom: clip.bloom,
            chromatic_aberration: clip.chromatic_aberration,
            custom_curves: clip.custom_curves.clone(),
            hsl_curves: clip.hsl_curves.clone(),
            hsl_qualifier: clip.hsl_qualifier.clone(),
        }
    }

    pub fn apply_to_clip(&self, clip: &mut Clip) {
        clip.camera_log_profile = self.camera_log_profile;
        clip.film_format = self.film_format;
        clip.highlight_rolloff = self.highlight_rolloff;
        clip.diffusion = self.diffusion;
        clip.mist = self.mist;
        clip.gate_weave = self.gate_weave;
        clip.brightness = self.brightness;
        clip.contrast = self.contrast;
        clip.saturation = self.saturation;
        clip.gamma = self.gamma;
        clip.temperature = self.temperature;
        clip.tint = self.tint;
        clip.wb_preset = self.wb_preset;
        clip.bc_preset = self.bc_preset;
        clip.lut_preset = self.lut_preset;
        clip.custom_lut_path = self.custom_lut_path.clone();
        clip.color_wheels = self.color_wheels.clone();
        clip.blur = self.blur;
        clip.sharpen = self.sharpen;
        clip.vignette = self.vignette;
        clip.film_grain = self.film_grain;
        clip.halation = self.halation;
        clip.bloom = self.bloom;
        clip.chromatic_aberration = self.chromatic_aberration;
        clip.custom_curves = self.custom_curves.clone();
        clip.hsl_curves = self.hsl_curves.clone();
        clip.hsl_qualifier = self.hsl_qualifier.clone();
    }
}

#[derive(Debug, Clone)]
pub struct SetClipColorGradingCommand {
    pub track_id: String,
    pub clip_id: String,
    pub old_state: ClipColorGradingState,
    pub new_state: ClipColorGradingState,
}

impl SetClipColorGradingCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_state: ClipColorGradingState,
        new_state: ClipColorGradingState,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            old_state,
            new_state,
        }
    }
}

impl Command for SetClipColorGradingCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        self.new_state.apply_to_clip(clip);
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        self.old_state.apply_to_clip(clip);
        Ok(())
    }

    fn name(&self) -> &str {
        "Renk Düzenlemesini Değiştir"
    }
}

// -----------------------------------------------------------------------------
// 31. SetClipBlendModeCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct SetClipBlendModeCommand {
    pub track_id: String,
    pub clip_id: String,
    pub old_blend_mode: trim_core::BlendMode,
    pub new_blend_mode: trim_core::BlendMode,
}

impl SetClipBlendModeCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_blend_mode: trim_core::BlendMode,
        new_blend_mode: trim_core::BlendMode,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            old_blend_mode,
            new_blend_mode,
        }
    }
}

impl Command for SetClipBlendModeCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.blend_mode = self.new_blend_mode;
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.blend_mode = self.old_blend_mode;
        Ok(())
    }

    fn name(&self) -> &str {
        "Karışım Modunu Değiştir"
    }
}

// -----------------------------------------------------------------------------
// 32. SetClipMasksCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct SetClipMasksCommand {
    pub track_id: String,
    pub clip_id: String,
    pub old_masks: Vec<trim_core::Mask>,
    pub new_masks: Vec<trim_core::Mask>,
}

impl SetClipMasksCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        old_masks: Vec<trim_core::Mask>,
        new_masks: Vec<trim_core::Mask>,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            old_masks,
            new_masks,
        }
    }
}

impl Command for SetClipMasksCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.masks = self.new_masks.clone();
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        clip.masks = self.old_masks.clone();
        Ok(())
    }

    fn name(&self) -> &str {
        "Maskeleri Güncelle"
    }
}

// -----------------------------------------------------------------------------
// 33. AddAdjustmentLayerCommand
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct AddAdjustmentLayerCommand {
    pub track_id: String,
    pub clip: Clip,
}

impl AddAdjustmentLayerCommand {
    pub fn new(track_id: impl Into<String>, clip: Clip) -> Self {
        Self {
            track_id: track_id.into(),
            clip,
        }
    }
}

impl Command for AddAdjustmentLayerCommand {
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
        "Ayar Katmanı Ekle"
    }
}

// -----------------------------------------------------------------------------
// 34. ModifyClipCommand (Generic Before/After Clip Snapshot)
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct ModifyClipCommand {
    pub label: String,
    pub track_id: String,
    pub clip_id: String,
    pub before: Clip,
    pub after: Clip,
}

impl ModifyClipCommand {
    pub fn new(
        label: impl Into<String>,
        track_id: impl Into<String>,
        before: Clip,
        after: Clip,
    ) -> Self {
        let clip_id = after.id.clone();
        Self {
            label: label.into(),
            track_id: track_id.into(),
            clip_id,
            before,
            after,
        }
    }
}

impl Command for ModifyClipCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        if track.locked {
            return Err(CommandError::InvalidOperation(
                "Kilitli kanaldaki klip düzenlenemez".into(),
            ));
        }
        let clip = track
            .get_clip_by_id_mut(&self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        *clip = self.after.clone();
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
        *clip = self.before.clone();
        track.sort_clips();
        Ok(())
    }

    fn name(&self) -> &str {
        &self.label
    }
}

// ---------------------------------------------------------------------------
// 32. RollTrimCommand (adjecent edit-point roll)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RollTrimCommand {
    pub track_id: String,
    pub left_clip_id: String,
    pub right_clip_id: String,
    pub delta_secs: f64,
    prev_left: Option<(f64, f64, f64, Option<f64>)>,
    prev_right: Option<(f64, f64, f64, Option<f64>)>,
}

impl RollTrimCommand {
    pub fn new(
        track_id: impl Into<String>,
        left_clip_id: impl Into<String>,
        right_clip_id: impl Into<String>,
        delta_secs: f64,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            left_clip_id: left_clip_id.into(),
            right_clip_id: right_clip_id.into(),
            delta_secs,
            prev_left: None,
            prev_right: None,
        }
    }

    /// Computes the clamped roll delta so neither clip shrinks below `0.04s` or goes negative in source/timeline.
    pub fn clamp_delta(
        l_in: f64,
        l_out: f64,
        r_in: f64,
        r_out: f64,
        rs_in: f64,
        r_speed: f64,
        delta: f64,
    ) -> f64 {
        const MIN_DUR: f64 = 0.04;
        let r_spd = r_speed.max(0.05);
        let min_d_left = -(l_out - l_in - MIN_DUR).max(0.0);
        let min_d_right_src = -(rs_in / r_spd).max(0.0);
        let min_d_right_tl = -r_in.max(0.0);
        let min_d = min_d_left.max(min_d_right_src).max(min_d_right_tl);

        let max_d_right = (r_out - r_in - MIN_DUR).max(0.0);
        delta.clamp(min_d, max_d_right)
    }
}

impl Command for RollTrimCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        if track.locked {
            return Err(CommandError::InvalidOperation(
                "Kilitli kanaldaki klip kırpılamaz".into(),
            ));
        }
        let l_idx = track
            .clips
            .iter()
            .position(|c| c.id == self.left_clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.left_clip_id.clone()))?;
        let r_idx = track
            .clips
            .iter()
            .position(|c| c.id == self.right_clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.right_clip_id.clone()))?;

        let l = &track.clips[l_idx];
        let r = &track.clips[r_idx];
        self.prev_left = Some((l.timeline_in, l.timeline_out, l.source_in, l.source_out));
        self.prev_right = Some((r.timeline_in, r.timeline_out, r.source_in, r.source_out));

        let d = Self::clamp_delta(
            l.timeline_in,
            l.timeline_out,
            r.timeline_in,
            r.timeline_out,
            r.source_in,
            r.speed,
            self.delta_secs,
        );
        let l_spd = l.speed.max(0.05);
        let r_spd = r.speed.max(0.05);

        let left = &mut track.clips[l_idx];
        left.timeline_out += d;
        if let Some(ref mut so) = left.source_out {
            *so += d * l_spd;
        }
        let right = &mut track.clips[r_idx];
        right.timeline_in += d;
        right.source_in = (right.source_in + d * r_spd).max(0.0);
        track.sort_clips();
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        if let Some((ti, to, si, so)) = self.prev_left {
            if let Some(c) = track.get_clip_by_id_mut(&self.left_clip_id) {
                c.timeline_in = ti;
                c.timeline_out = to;
                c.source_in = si;
                c.source_out = so;
            }
        }
        if let Some((ti, to, si, so)) = self.prev_right {
            if let Some(c) = track.get_clip_by_id_mut(&self.right_clip_id) {
                c.timeline_in = ti;
                c.timeline_out = to;
                c.source_in = si;
                c.source_out = so;
            }
        }
        track.sort_clips();
        Ok(())
    }

    fn name(&self) -> &str {
        "Roll Trim"
    }
}

// ---------------------------------------------------------------------------
// 33. SlideTrimCommand (slides middle clip while adjusting adjoining clips)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SlideTrimCommand {
    pub track_id: String,
    pub clip_id: String,
    pub left_clip_id: Option<String>,
    pub right_clip_id: Option<String>,
    pub delta_secs: f64,
    prev_mid: Option<(f64, f64, f64, Option<f64>)>,
    prev_left: Option<(f64, f64, f64, Option<f64>)>,
    prev_right: Option<(f64, f64, f64, Option<f64>)>,
}

impl SlideTrimCommand {
    pub fn new(
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        left_clip_id: Option<String>,
        right_clip_id: Option<String>,
        delta_secs: f64,
    ) -> Self {
        Self {
            track_id: track_id.into(),
            clip_id: clip_id.into(),
            left_clip_id,
            right_clip_id,
            delta_secs,
            prev_mid: None,
            prev_left: None,
            prev_right: None,
        }
    }
}

impl Command for SlideTrimCommand {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        const MIN_DUR: f64 = 0.04;
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        if track.locked {
            return Err(CommandError::InvalidOperation(
                "Kilitli kanaldaki klip kaydırılamaz".into(),
            ));
        }

        let mid_idx = track
            .clips
            .iter()
            .position(|c| c.id == self.clip_id)
            .ok_or_else(|| CommandError::ClipNotFound(self.clip_id.clone()))?;
        let left_idx = self
            .left_clip_id
            .as_ref()
            .and_then(|id| track.clips.iter().position(|c| &c.id == id));
        let right_idx = self
            .right_clip_id
            .as_ref()
            .and_then(|id| track.clips.iter().position(|c| &c.id == id));

        let mid = &track.clips[mid_idx];
        self.prev_mid = Some((
            mid.timeline_in,
            mid.timeline_out,
            mid.source_in,
            mid.source_out,
        ));

        let mut min_d = -mid.timeline_in.max(0.0);
        let mut max_d = f64::INFINITY;

        if let Some(li) = left_idx {
            let l = &track.clips[li];
            self.prev_left = Some((l.timeline_in, l.timeline_out, l.source_in, l.source_out));
            min_d = min_d.max(-(l.timeline_out - l.timeline_in - MIN_DUR).max(0.0));
        }
        if let Some(ri) = right_idx {
            let r = &track.clips[ri];
            self.prev_right = Some((r.timeline_in, r.timeline_out, r.source_in, r.source_out));
            let r_spd = r.speed.max(0.05);
            min_d = min_d.max(-(r.source_in / r_spd).max(0.0));
            max_d = max_d.min((r.timeline_out - r.timeline_in - MIN_DUR).max(0.0));
        }

        let d = self.delta_secs.clamp(min_d, max_d);

        track.clips[mid_idx].timeline_in += d;
        track.clips[mid_idx].timeline_out += d;
        if let Some(li) = left_idx {
            let l_spd = track.clips[li].speed.max(0.05);
            track.clips[li].timeline_out += d;
            if let Some(ref mut so) = track.clips[li].source_out {
                *so += d * l_spd;
            }
        }
        if let Some(ri) = right_idx {
            let r_spd = track.clips[ri].speed.max(0.05);
            track.clips[ri].timeline_in += d;
            track.clips[ri].source_in = (track.clips[ri].source_in + d * r_spd).max(0.0);
        }
        track.sort_clips();
        Ok(())
    }

    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        let track = timeline
            .get_track_by_id_mut(&self.track_id)
            .ok_or_else(|| CommandError::TrackNotFound(self.track_id.clone()))?;
        if let Some((ti, to, si, so)) = self.prev_mid {
            if let Some(c) = track.get_clip_by_id_mut(&self.clip_id) {
                c.timeline_in = ti;
                c.timeline_out = to;
                c.source_in = si;
                c.source_out = so;
            }
        }
        if let (Some(ref lid), Some((ti, to, si, so))) = (&self.left_clip_id, self.prev_left) {
            if let Some(c) = track.get_clip_by_id_mut(lid) {
                c.timeline_in = ti;
                c.timeline_out = to;
                c.source_in = si;
                c.source_out = so;
            }
        }
        if let (Some(ref rid), Some((ti, to, si, so))) = (&self.right_clip_id, self.prev_right) {
            if let Some(c) = track.get_clip_by_id_mut(rid) {
                c.timeline_in = ti;
                c.timeline_out = to;
                c.source_in = si;
                c.source_out = so;
            }
        }
        track.sort_clips();
        Ok(())
    }

    fn name(&self) -> &str {
        "Slide Trim"
    }
}
