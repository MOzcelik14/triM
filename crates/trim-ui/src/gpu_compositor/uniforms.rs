use trim_core::clip::Clip;

/// Pre-computed per-layer shader uniforms derived from `Clip` state at `timeline_time`.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerUniforms {
    pub canvas_size: [f32; 2],
    pub clip_center: [f32; 2],
    pub clip_size: [f32; 2],
    pub rotation_rad: f32,
    pub opacity: f32,
    pub wipe_mode: i32,
    pub fade_factor: f32,
    pub dip_white: f32,
    pub ck_enabled: bool,
    pub ck_color: [f32; 3],
    pub ck_similarity: f32,
    pub ck_smoothness: f32,
    pub cw_enabled: bool,
    pub cw_lift: [f32; 3],
    pub cw_gamma: [f32; 3],
    pub cw_gain: [f32; 3],
    pub wb_gains: [f32; 3],
    pub temperature: f32,
    pub tint: f32,
    pub contrast: f32,
    pub brightness: f32,
    pub gamma: f32,
    pub saturation: f32,
    pub blur: f32,
    pub sharpen: f32,
    pub vignette: f32,
    pub film_grain: f32,
    pub halation: f32,
    pub bloom: f32,
    pub chromatic_aberration: f32,
    pub film_format: i32,
    pub highlight_rolloff: f32,
    pub diffusion: f32,
    pub mist: f32,
    pub gate_weave: f32,
    pub cp_enabled: bool,
    pub cp_tl: [f32; 2],
    pub cp_tr: [f32; 2],
    pub cp_br: [f32; 2],
    pub cp_bl: [f32; 2],
    pub glitch_intensity: f32,
    pub light_leak: f32,
    pub time_seed: f32,
    pub camera_log_profile: i32,
    pub lut_preset: i32,
    pub crop: [f32; 4],
    pub blend_mode: i32,
    pub is_adjustment_layer: bool,
}

impl LayerUniforms {
    pub fn from_clip(
        clip: &Clip,
        frame_w: u32,
        frame_h: u32,
        logical_canvas_w: u32,
        logical_canvas_h: u32,
        current_time: f64,
    ) -> Option<Self> {
        let cw = logical_canvas_w.max(1) as f64;
        let ch = logical_canvas_h.max(1) as f64;
        let fw = frame_w as f64;
        let fh = frame_h as f64;
        if fw <= 0.0 || fh <= 0.0 {
            return None;
        }

        let clip_time = (current_time - clip.timeline_in).max(0.0);
        let animated_opacity = clip.get_property_at_time("opacity", clip_time);
        let animated_scale = clip.get_property_at_time("scale", clip_time);
        let animated_pos_x = clip.get_property_at_time("pos_x", clip_time);
        let animated_pos_y = clip.get_property_at_time("pos_y", clip_time);
        let animated_rot_deg = clip.get_property_at_time("rotation", clip_time);

        let dt = current_time - clip.timeline_in;
        let rem = clip.timeline_out - current_time;

        let in_progress = if clip.fade_in > 0.0 && dt < clip.fade_in {
            (dt / clip.fade_in).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let out_progress = if clip.fade_out > 0.0 && rem < clip.fade_out {
            (rem / clip.fade_out).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let fade_factor = in_progress.min(out_progress);

        let is_wipe_or_slide =
            clip.transition_in.as_deref().is_some_and(|t| {
                t.starts_with("wipe") || t.starts_with("slide") || t == "whip_pan"
            }) || clip.transition_out.as_deref().is_some_and(|t| {
                t.starts_with("wipe") || t.starts_with("slide") || t == "whip_pan"
            });

        let eff_opacity = if is_wipe_or_slide {
            animated_opacity.clamp(0.0, 1.0)
        } else {
            (animated_opacity * fade_factor).clamp(0.0, 1.0)
        };

        if eff_opacity <= 0.001 {
            return None;
        }

        let scale_fit = (cw / fw).min(ch / fh);
        let mut eff_scale = scale_fit * animated_scale.max(0.05);
        if clip.transition_in.as_deref() == Some("zoom_in") && in_progress < 1.0 {
            eff_scale *= 0.3 + 0.7 * in_progress;
        } else if clip.transition_in.as_deref() == Some("zoom_out") && in_progress < 1.0 {
            eff_scale *= 1.7 - 0.7 * in_progress;
        }
        if clip.transition_out.as_deref() == Some("zoom_out") && out_progress < 1.0 {
            eff_scale *= 0.3 + 0.7 * out_progress;
        } else if clip.transition_out.as_deref() == Some("zoom_in") && out_progress < 1.0 {
            eff_scale *= 1.7 - 0.7 * out_progress;
        }

        let final_w = (fw * eff_scale).round().max(1.0);
        let final_h = (fh * eff_scale).round().max(1.0);

        let mut slide_x_offset = 0.0;
        if in_progress < 1.0 {
            if matches!(
                clip.transition_in.as_deref(),
                Some("slide_left") | Some("whip_pan")
            ) {
                let p = if clip.transition_in.as_deref() == Some("whip_pan") {
                    (1.0 - in_progress).powi(2)
                } else {
                    1.0 - in_progress
                };
                slide_x_offset += p * cw;
            } else if clip.transition_in.as_deref() == Some("slide_right") {
                slide_x_offset -= (1.0 - in_progress) * cw;
            }
        }
        if out_progress < 1.0 {
            if matches!(
                clip.transition_out.as_deref(),
                Some("slide_left") | Some("whip_pan")
            ) {
                let p = if clip.transition_out.as_deref() == Some("whip_pan") {
                    (1.0 - out_progress).powi(2)
                } else {
                    1.0 - out_progress
                };
                slide_x_offset -= p * cw;
            } else if clip.transition_out.as_deref() == Some("slide_right") {
                slide_x_offset += (1.0 - out_progress) * cw;
            }
        }

        let final_x = ((cw - final_w) * 0.5 + animated_pos_x + slide_x_offset).round();
        let final_y = ((ch - final_h) * 0.5 + animated_pos_y).round();
        let center_x = (final_x + final_w * 0.5) as f32;
        let center_y = (final_y + final_h * 0.5) as f32;

        let has_dip_white = (clip.transition_in.as_deref() == Some("dip_white")
            && fade_factor < 1.0)
            || (clip.transition_out.as_deref() == Some("dip_white") && fade_factor < 1.0);
        let dip_white = if has_dip_white {
            (1.0 - fade_factor) as f32
        } else {
            0.0
        };

        let wipe_left = (clip.transition_in.as_deref() == Some("wipe_left") && in_progress < 1.0)
            || (clip.transition_out.as_deref() == Some("wipe_left") && out_progress < 1.0);
        let wipe_right = (clip.transition_in.as_deref() == Some("wipe_right") && in_progress < 1.0)
            || (clip.transition_out.as_deref() == Some("wipe_right") && out_progress < 1.0);
        let luma_wipe = (clip.transition_in.as_deref() == Some("luma_wipe") && in_progress < 1.0)
            || (clip.transition_out.as_deref() == Some("luma_wipe") && out_progress < 1.0);
        let wipe_mode = if wipe_left {
            1
        } else if wipe_right {
            2
        } else if luma_wipe {
            3
        } else {
            0
        };

        let mut extra_blur = 0.0_f32;
        if (clip.transition_in.as_deref() == Some("blur_dissolve") && in_progress < 1.0)
            || (clip.transition_out.as_deref() == Some("blur_dissolve") && out_progress < 1.0)
        {
            extra_blur += ((1.0 - fade_factor) as f32) * 14.0;
        }
        if (clip.transition_in.as_deref() == Some("whip_pan") && in_progress < 1.0)
            || (clip.transition_out.as_deref() == Some("whip_pan") && out_progress < 1.0)
        {
            extra_blur += ((1.0 - fade_factor) as f32) * 18.0;
        }

        let glitch_active = (clip.transition_in.as_deref() == Some("glitch") && in_progress < 1.0)
            || (clip.transition_out.as_deref() == Some("glitch") && out_progress < 1.0);
        let glitch_intensity = if glitch_active {
            (1.0 - fade_factor) as f32
        } else {
            0.0
        };

        let light_leak_active = (clip.transition_in.as_deref() == Some("light_leak")
            && in_progress < 1.0)
            || (clip.transition_out.as_deref() == Some("light_leak") && out_progress < 1.0);
        let light_leak = if light_leak_active {
            (1.0 - fade_factor) as f32
        } else {
            0.0
        };

        let (ck_enabled, ck_color, ck_similarity, ck_smoothness) = match &clip.chromakey {
            Some(ck) if ck.enabled => (
                true,
                [
                    ck.key_color[0] as f32 / 255.0,
                    ck.key_color[1] as f32 / 255.0,
                    ck.key_color[2] as f32 / 255.0,
                ],
                ck.similarity.clamp(0.01, 1.0),
                ck.smoothness.clamp(0.001, 0.5),
            ),
            _ => (false, [0.0, 1.0, 0.0], 0.3, 0.05),
        };

        let (cw_enabled, cw_lift, cw_gamma, cw_gain) = match &clip.color_wheels {
            Some(cw) => (
                true,
                [
                    cw.lift[0] + cw.luminance[0],
                    cw.lift[1] + cw.luminance[0],
                    cw.lift[2] + cw.luminance[0],
                ],
                [
                    (cw.gamma[0] + cw.luminance[1]).max(0.0),
                    (cw.gamma[1] + cw.luminance[1]).max(0.0),
                    (cw.gamma[2] + cw.luminance[1]).max(0.0),
                ],
                [
                    (cw.gain[0] + cw.luminance[2]).max(0.0),
                    (cw.gain[1] + cw.luminance[2]).max(0.0),
                    (cw.gain[2] + cw.luminance[2]).max(0.0),
                ],
            ),
            None => (false, [0.0; 3], [1.0; 3], [1.0; 3]),
        };

        let temperature = clip.temperature as f32;
        let tint = clip.tint as f32;
        let wb_gains = trim_core::color::white_balance_gains(temperature, tint);
        let lut_preset = clip.lut_preset.shader_id();
        let (cp_enabled, cp_tl, cp_tr, cp_br, cp_bl) = match &clip.corner_pin {
            Some(cp) if cp.enabled => (
                true,
                cp.top_left,
                cp.top_right,
                cp.bottom_right,
                cp.bottom_left,
            ),
            _ => (false, [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]),
        };

        Some(Self {
            canvas_size: [cw as f32, ch as f32],
            clip_center: [center_x, center_y],
            clip_size: [final_w as f32, final_h as f32],
            rotation_rad: (animated_rot_deg as f32).to_radians(),
            opacity: eff_opacity as f32,
            wipe_mode,
            fade_factor: fade_factor as f32,
            dip_white,
            ck_enabled,
            ck_color,
            ck_similarity,
            ck_smoothness,
            cw_enabled,
            cw_lift,
            cw_gamma,
            cw_gain,
            wb_gains,
            temperature,
            tint,
            contrast: clip.contrast as f32,
            brightness: clip.brightness as f32,
            gamma: clip.gamma as f32,
            saturation: clip.saturation as f32,
            blur: (clip.blur.clamp(0.0, 20.0) + extra_blur).clamp(0.0, 25.0),
            sharpen: clip.sharpen.clamp(0.0, 2.0),
            vignette: clip.vignette.clamp(0.0, 1.0),
            film_grain: clip.film_grain.clamp(0.0, 1.0),
            halation: clip.halation.clamp(0.0, 1.0),
            bloom: clip.bloom.clamp(0.0, 1.0),
            chromatic_aberration: clip.chromatic_aberration.clamp(0.0, 1.0),
            film_format: clip.film_format.shader_id(),
            highlight_rolloff: clip.highlight_rolloff.clamp(0.0, 1.0),
            diffusion: clip.diffusion.clamp(0.0, 1.0),
            mist: clip.mist.clamp(0.0, 1.0),
            gate_weave: clip.gate_weave.clamp(0.0, 1.0),
            cp_enabled,
            cp_tl,
            cp_tr,
            cp_br,
            cp_bl,
            glitch_intensity,
            light_leak,
            time_seed: (current_time as f32).fract(),
            camera_log_profile: clip.camera_log_profile.shader_id(),
            lut_preset,
            crop: [
                clip.crop_left.clamp(0.0, 1.0),
                clip.crop_top.clamp(0.0, 1.0),
                clip.crop_right.clamp(0.0, 1.0),
                clip.crop_bottom.clamp(0.0, 1.0),
            ],
            blend_mode: clip.blend_mode.shader_id(),
            is_adjustment_layer: clip.is_adjustment_layer(),
        })
    }
}
