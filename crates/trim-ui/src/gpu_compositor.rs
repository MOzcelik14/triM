//! OpenGL 3.3+ / GLES 3.0 (`glow`) off-screen FBO fragment shader compositor for real-time NLE preview.

use eframe::glow::{self, HasContext};
use std::collections::HashMap;
use std::sync::Arc;
use trim_core::clip::Clip;
use trim_core::project::Project;
use trim_media::{calculate_preview_size, parse_cube_lut_file, CompositorLayer, CubeLut3D};

pub const GPU_PREVIEW_SCALE: u32 = 2; // 1280x720 for 16:9 (2x logical 640x360 coordinate space)

const VERTEX_SHADER_BODY: &str = r#"
out vec2 v_canvas_uv;

void main() {
    vec2 pos;
    if (gl_VertexID == 0) {
        pos = vec2(-1.0, -1.0);
    } else if (gl_VertexID == 1) {
        pos = vec2(1.0, -1.0);
    } else if (gl_VertexID == 2) {
        pos = vec2(-1.0, 1.0);
    } else {
        pos = vec2(1.0, 1.0);
    }
    v_canvas_uv = pos * 0.5 + 0.5;
    gl_Position = vec4(pos, 0.0, 1.0);
}
"#;

const FRAGMENT_SHADER_BODY: &str = r#"
in vec2 v_canvas_uv;
out vec4 frag_color;

uniform sampler2D u_frame_tex;
uniform sampler2D u_lut_tex;
uniform sampler2D u_backdrop_tex;
uniform sampler2D u_mask_tex;

uniform int u_has_mask;
uniform int u_blend_mode;
uniform int u_is_adjustment_layer;

uniform vec2 u_canvas_size;
uniform vec2 u_clip_center;
uniform vec2 u_clip_size;
uniform float u_rotation_rad;
uniform float u_opacity;
uniform vec4 u_crop;

uniform int u_wipe_mode;
uniform float u_fade_factor;
uniform float u_dip_white;

uniform int u_ck_enabled;
uniform vec3 u_ck_color;
uniform float u_ck_similarity;
uniform float u_ck_smoothness;

uniform int u_cw_enabled;
uniform vec3 u_cw_lift;
uniform vec3 u_cw_gamma;
uniform vec3 u_cw_gain;

uniform vec3 u_wb_gains;
uniform float u_contrast;
uniform float u_brightness;
uniform float u_gamma;
uniform float u_saturation;
uniform int u_lut_preset;
uniform int u_custom_lut_enabled;
uniform int u_lut_size;

vec3 sample_3d_lut(vec3 rgb, int n_int) {
    float n = float(n_int);
    float scale = (n - 1.0) / n;
    float offset = 0.5 / n;
    float u_slice = rgb.r * scale + offset;
    float v = rgb.g * scale + offset;
    float b_pos = rgb.b * (n - 1.0);
    float b0 = floor(b_pos);
    float b1 = min(b0 + 1.0, n - 1.0);
    float b_frac = b_pos - b0;
    vec2 uv0 = vec2((b0 + u_slice) / n, v);
    vec2 uv1 = vec2((b1 + u_slice) / n, v);
    vec3 c0 = texture(u_lut_tex, uv0).rgb;
    vec3 c1 = texture(u_lut_tex, uv1).rgb;
    return mix(c0, c1, b_frac);
}

vec3 s_curve_vec(vec3 x, float strength) {
    vec3 cx = clamp(x, 0.0, 1.0);
    vec3 s = cx * cx * (3.0 - 2.0 * cx);
    return cx + (s - cx) * strength;
}

float blend_chan(float b, float t, int mode) {
    if (mode == 1) return b * t; // Multiply
    if (mode == 2) return 1.0 - (1.0 - b) * (1.0 - t); // Screen
    if (mode == 3) return (b < 0.5) ? (2.0 * b * t) : (1.0 - 2.0 * (1.0 - b) * (1.0 - t)); // Overlay
    if (mode == 4) { // SoftLight (W3C)
        if (t <= 0.5) {
            return b - (1.0 - 2.0 * t) * b * (1.0 - b);
        } else {
            float d = (b <= 0.25) ? (((16.0 * b - 12.0) * b + 4.0) * b) : sqrt(b);
            return b + (2.0 * t - 1.0) * (d - b);
        }
    }
    if (mode == 5) return (t < 0.5) ? (2.0 * b * t) : (1.0 - 2.0 * (1.0 - b) * (1.0 - t)); // HardLight
    if (mode == 6) return min(1.0, b + t); // Add
    if (mode == 7) return max(0.0, b - t); // Subtract
    if (mode == 8) return abs(b - t); // Difference
    if (mode == 9) return min(b, t); // Darken
    if (mode == 10) return max(b, t); // Lighten
    return t; // Normal
}

vec3 apply_blend_mode(vec3 base, vec3 top, int mode) {
    if (mode == 0) return top;
    return vec3(
        blend_chan(base.r, top.r, mode),
        blend_chan(base.g, top.g, mode),
        blend_chan(base.b, top.b, mode)
    );
}

void main() {
    vec3 rgb;
    float a;

    if (u_is_adjustment_layer == 1) {
        vec4 bd = texture(u_backdrop_tex, v_canvas_uv);
        rgb = bd.rgb;
        a = u_opacity;
    } else {
        vec2 canvas_px = v_canvas_uv * u_canvas_size;
        vec2 rel = canvas_px - u_clip_center;

        float cos_r = cos(-u_rotation_rad);
        float sin_r = sin(-u_rotation_rad);
        vec2 rot_rel = vec2(
            rel.x * cos_r - rel.y * sin_r,
            rel.x * sin_r + rel.y * cos_r
        );

        vec2 uv = (rot_rel / max(u_clip_size, vec2(1.0))) + vec2(0.5);
        if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
            discard;
        }
        if (uv.x < u_crop.x || uv.x > (1.0 - u_crop.z) || uv.y < u_crop.y || uv.y > (1.0 - u_crop.w)) {
            discard;
        }

        if (u_wipe_mode == 1 && uv.x > u_fade_factor) {
            discard;
        }
        if (u_wipe_mode == 2 && uv.x < (1.0 - u_fade_factor)) {
            discard;
        }

        vec4 texel = texture(u_frame_tex, uv);
        rgb = texel.rgb;
        a = texel.a * u_opacity;

        if (u_ck_enabled == 1) {
            float dist = distance(rgb, u_ck_color);
            if (dist < u_ck_similarity) {
                discard;
            } else if (dist < u_ck_similarity + u_ck_smoothness) {
                a *= (dist - u_ck_similarity) / u_ck_smoothness;
            }
        }
    }

    vec2 mask_val = (u_has_mask == 1) ? texture(u_mask_tex, v_canvas_uv).rg : vec2(1.0, 1.0);
    a *= mask_val.r;
    if (a <= 0.001) {
        discard;
    }

    vec3 ungraded_rgb = rgb;

    if (u_cw_enabled == 1) {
        float luma = dot(rgb, vec3(0.299, 0.587, 0.114));
        float shadow_w = clamp(1.0 - luma * 2.0, 0.0, 1.0);
        float hi_w = clamp((luma - 0.5) * 2.0, 0.0, 1.0);
        float mid_w = clamp(1.0 - shadow_w - hi_w, 0.0, 1.0);

        rgb += u_cw_lift * (128.0 / 255.0) * shadow_w;
        rgb *= vec3(1.0) + (u_cw_gamma - vec3(1.0)) * mid_w;
        rgb *= vec3(1.0) + (u_cw_gain - vec3(1.0)) * hi_w;
        rgb = clamp(rgb, 0.0, 1.0);
    }

    // 1. White balance channel gains
    rgb = clamp(rgb * u_wb_gains, 0.0, 1.0);

    // 2. Contrast & Brightness
    rgb = clamp(
        u_contrast * (rgb - vec3(128.0 / 255.0)) + vec3(128.0 / 255.0) + vec3(u_brightness),
        0.0,
        1.0
    );

    // 3. Gamma
    if (abs(u_gamma - 1.0) > 0.001) {
        float inv_g = 1.0 / max(u_gamma, 0.1);
        rgb = clamp(pow(max(rgb, vec3(0.0)), vec3(inv_g)), 0.0, 1.0);
    }

    // 4. Saturation
    if (abs(u_saturation - 1.0) > 0.0001) {
        float luma_s = dot(rgb, vec3(0.299, 0.587, 0.114));
        rgb = clamp(vec3(luma_s) + u_saturation * (rgb - vec3(luma_s)), 0.0, 1.0);
    }

    // 5. Creative Color Looks
    float luma = dot(rgb, vec3(0.299, 0.587, 0.114));
    float shadow_w = clamp(1.0 - luma * 2.0, 0.0, 1.0);
    float hi_w = clamp((luma - 0.5) * 2.0, 0.0, 1.0);

    if (u_lut_preset == 1) {
        if (luma < 0.5) {
            rgb = clamp(rgb * vec3(1.0 - 0.22 * shadow_w, 1.0 + 0.08 * shadow_w, 1.0 + 0.32 * shadow_w), 0.0, 1.0);
        } else {
            rgb = clamp(rgb * vec3(1.0 + 0.25 * hi_w, 1.0 + 0.08 * hi_w, 1.0 - 0.22 * hi_w), 0.0, 1.0);
        }
    } else if (u_lut_preset == 2) {
        rgb = clamp(
            rgb * vec3(1.06, 1.02, 0.90) + vec3(14.0 / 255.0, 10.0 / 255.0, 6.0 / 255.0),
            0.0,
            1.0
        );
    } else if (u_lut_preset == 3) {
        float curved = (luma < 0.5)
            ? (2.0 * luma * luma)
            : (1.0 - 2.0 * (1.0 - luma) * (1.0 - luma));
        rgb = vec3(clamp(curved, 0.0, 1.0));
    } else if (u_lut_preset == 4) {
        rgb = clamp(
            rgb * vec3(1.22, 0.88, 1.32) + vec3(8.0 / 255.0, 0.0, 16.0 / 255.0),
            0.0,
            1.0
        );
        float luma_c = dot(rgb, vec3(0.299, 0.587, 0.114));
        rgb = clamp(vec3(luma_c) + 1.25 * (rgb - vec3(luma_c)), 0.0, 1.0);
    } else if (u_lut_preset == 5) {
        // Cinematic Warm
        rgb = s_curve_vec(rgb, 0.35);
        rgb.r = rgb.r * (1.08 + 0.10 * hi_w) + 0.02 * shadow_w;
        rgb.g = rgb.g * (1.02 + 0.04 * hi_w) + 0.01 * shadow_w;
        rgb.b = rgb.b * (0.88 - 0.08 * hi_w);
        rgb = clamp(rgb, 0.0, 1.0);
    } else if (u_lut_preset == 6) {
        // Cinematic Cool
        rgb = s_curve_vec(rgb, 0.40);
        rgb.r = rgb.r * (0.90 - 0.06 * shadow_w);
        rgb.g = rgb.g * (0.98 + 0.04 * shadow_w);
        rgb.b = rgb.b * (1.12 + 0.12 * shadow_w);
        float lm = dot(rgb, vec3(0.299, 0.587, 0.114));
        rgb = clamp(vec3(lm) + 0.90 * (rgb - vec3(lm)), 0.0, 1.0);
    } else if (u_lut_preset == 7) {
        // Blockbuster
        rgb = s_curve_vec(rgb, 0.55);
        rgb.r = rgb.r * (1.0 - 0.26 * shadow_w + 0.28 * hi_w);
        rgb.g = rgb.g * (1.0 + 0.05 * shadow_w + 0.06 * hi_w);
        rgb.b = rgb.b * (1.0 + 0.30 * shadow_w - 0.24 * hi_w);
        float lm = dot(rgb, vec3(0.299, 0.587, 0.114));
        rgb = clamp(vec3(lm) + 1.15 * (rgb - vec3(lm)), 0.0, 1.0);
    } else if (u_lut_preset == 8) {
        // Filmic Neutral
        vec3 cr = rgb / (vec3(1.0) + 0.18 * rgb) * 1.18;
        cr = s_curve_vec(cr, 0.22);
        float lm = dot(cr, vec3(0.299, 0.587, 0.114));
        rgb = clamp(vec3(lm) + 0.94 * (cr - vec3(lm)), 0.0, 1.0);
    } else if (u_lut_preset == 9) {
        // Kodak 2383
        vec3 cr = s_curve_vec(rgb, 0.50);
        cr.r = cr.r * (1.0 - 0.14 * shadow_w + 0.16 * hi_w);
        cr.g = cr.g * (1.0 + 0.03 * shadow_w + 0.05 * hi_w);
        cr.b = cr.b * (1.0 + 0.18 * shadow_w - 0.16 * hi_w);
        rgb = clamp(cr, 0.0, 1.0);
    } else if (u_lut_preset == 10) {
        // Kodak Portra
        vec3 cr = s_curve_vec(rgb, 0.18);
        cr.r = cr.r * 1.05 + 0.03;
        cr.g = cr.g * 1.02 + 0.02;
        cr.b = cr.b * 0.96 + 0.015;
        float lm = dot(cr, vec3(0.299, 0.587, 0.114));
        rgb = clamp(vec3(lm) + 0.94 * (cr - vec3(lm)), 0.0, 1.0);
    } else if (u_lut_preset == 11) {
        // Fuji Film
        vec3 cr = s_curve_vec(rgb, 0.32);
        cr.r = cr.r * (0.98 + 0.05 * hi_w);
        cr.g = cr.g * (1.04 + 0.06 * shadow_w);
        cr.b = cr.b * (1.05 + 0.08 * shadow_w);
        float lm = dot(cr, vec3(0.299, 0.587, 0.114));
        rgb = clamp(vec3(lm) + 1.08 * (cr - vec3(lm)), 0.0, 1.0);
    } else if (u_lut_preset == 12) {
        // Faded Film
        vec3 cr = rgb * 0.80 + vec3(0.085, 0.080, 0.095);
        float lm = dot(cr, vec3(0.299, 0.587, 0.114));
        rgb = clamp(vec3(lm) + 0.78 * (cr - vec3(lm)), 0.0, 1.0);
    } else if (u_lut_preset == 13) {
        // Analog Film
        vec3 cr = s_curve_vec(rgb, 0.25);
        cr.r = cr.r * 1.04 + 0.035;
        cr.g = cr.g * 1.01 + 0.025;
        cr.b = cr.b * 0.91 + 0.040 * shadow_w;
        float lm = dot(cr, vec3(0.299, 0.587, 0.114));
        rgb = clamp(vec3(lm) + 0.90 * (cr - vec3(lm)), 0.0, 1.0);
    } else if (u_lut_preset == 14) {
        // Sepia
        float sr = rgb.r * 0.393 + rgb.g * 0.769 + rgb.b * 0.189;
        float sg = rgb.r * 0.349 + rgb.g * 0.686 + rgb.b * 0.168;
        float sb = rgb.r * 0.272 + rgb.g * 0.534 + rgb.b * 0.131;
        rgb = clamp(vec3(sr, sg, sb), 0.0, 1.0);
    } else if (u_lut_preset == 15) {
        // Bleach Bypass
        vec3 desat = vec3(luma) + 0.42 * (rgb - vec3(luma));
        vec3 overlay = vec3(
            (luma < 0.5) ? (2.0 * desat.r * luma) : (1.0 - 2.0 * (1.0 - desat.r) * (1.0 - luma)),
            (luma < 0.5) ? (2.0 * desat.g * luma) : (1.0 - 2.0 * (1.0 - desat.g) * (1.0 - luma)),
            (luma < 0.5) ? (2.0 * desat.b * luma) : (1.0 - 2.0 * (1.0 - desat.b) * (1.0 - luma))
        );
        rgb = clamp(mix(desat, overlay, 0.65), 0.0, 1.0);
    } else if (u_lut_preset == 16) {
        // Muted Colors
        vec3 cr = (rgb - vec3(0.5)) * 0.92 + vec3(0.5);
        float lm = dot(cr, vec3(0.299, 0.587, 0.114));
        rgb = clamp(vec3(lm) + 0.55 * (cr - vec3(lm)), 0.0, 1.0);
    } else if (u_lut_preset == 17) {
        // Vibrant
        vec3 cr = s_curve_vec(rgb, 0.22);
        float lm = dot(cr, vec3(0.299, 0.587, 0.114));
        float max_c = max(cr.r, max(cr.g, cr.b));
        float min_c = min(cr.r, min(cr.g, cr.b));
        float cur_sat = max_c - min_c;
        float boost = 1.0 + 0.55 * (1.0 - cur_sat);
        rgb = clamp(vec3(lm) + boost * (cr - vec3(lm)), 0.0, 1.0);
    } else if (u_lut_preset == 18) {
        // Pastel
        vec3 cr = rgb * 0.82 + vec3(0.14, 0.13, 0.15);
        float lm = dot(cr, vec3(0.299, 0.587, 0.114));
        rgb = clamp(vec3(lm) + 0.72 * (cr - vec3(lm)), 0.0, 1.0);
    } else if (u_lut_preset == 19) {
        // Golden Hour
        vec3 cr = s_curve_vec(rgb, 0.25);
        cr.r = cr.r * 1.15 + 0.04;
        cr.g = cr.g * 1.04 + 0.015;
        cr.b = cr.b * 0.82;
        float lm = dot(cr, vec3(0.299, 0.587, 0.114));
        rgb = clamp(vec3(lm) + 1.12 * (cr - vec3(lm)), 0.0, 1.0);
    } else if (u_lut_preset == 20) {
        // Cold Winter
        vec3 cr = s_curve_vec(rgb, 0.30);
        cr.r = cr.r * 0.88;
        cr.g = cr.g * 0.99 + 0.01;
        cr.b = cr.b * 1.16 + 0.03;
        float lm = dot(cr, vec3(0.299, 0.587, 0.114));
        rgb = clamp(vec3(lm) + 0.82 * (cr - vec3(lm)), 0.0, 1.0);
    } else if (u_lut_preset == 21) {
        // Night Cinematic
        vec3 cr = max(rgb * 0.86 - vec3(0.03), vec3(0.0));
        cr = s_curve_vec(cr, 0.42);
        cr.r = cr.r * 0.80;
        cr.g = cr.g * 0.95 + 0.02 * shadow_w;
        cr.b = cr.b * 1.22 + 0.07 * shadow_w;
        float lm = dot(cr, vec3(0.299, 0.587, 0.114));
        rgb = clamp(vec3(lm) + 0.85 * (cr - vec3(lm)), 0.0, 1.0);
    }

    if (u_custom_lut_enabled == 1 && u_lut_size >= 2) {
        rgb = sample_3d_lut(rgb, u_lut_size);
    }

    if (u_has_mask == 1) {
        rgb = mix(ungraded_rgb, rgb, mask_val.g);
    }

    if (u_dip_white > 0.0) {
        rgb = mix(rgb, vec3(1.0), u_dip_white);
    }

    if (u_blend_mode != 0 || u_is_adjustment_layer == 1) {
        vec3 base = texture(u_backdrop_tex, v_canvas_uv).rgb;
        vec3 blended = apply_blend_mode(base, rgb, u_blend_mode);
        frag_color = vec4(mix(base, blended, a), 1.0);
    } else {
        frag_color = vec4(rgb, a);
    }
}
"#;

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

        let is_wipe_or_slide = clip
            .transition_in
            .as_deref()
            .is_some_and(|t| t.starts_with("wipe") || t.starts_with("slide"))
            || clip
                .transition_out
                .as_deref()
                .is_some_and(|t| t.starts_with("wipe") || t.starts_with("slide"));

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
        }

        let final_w = (fw * eff_scale).round().max(1.0);
        let final_h = (fh * eff_scale).round().max(1.0);

        let mut slide_x_offset = 0.0;
        if in_progress < 1.0 {
            if clip.transition_in.as_deref() == Some("slide_left") {
                slide_x_offset += (1.0 - in_progress) * cw;
            } else if clip.transition_in.as_deref() == Some("slide_right") {
                slide_x_offset -= (1.0 - in_progress) * cw;
            }
        }
        if out_progress < 1.0 {
            if clip.transition_out.as_deref() == Some("slide_left") {
                slide_x_offset -= (1.0 - out_progress) * cw;
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
        let wipe_mode = if wipe_left {
            1
        } else if wipe_right {
            2
        } else {
            0
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

/// Hardware-accelerated OpenGL (`glow`) off-screen FBO compositor.
pub struct GpuCompositor {
    gl: Arc<glow::Context>,
    program: glow::Program,
    vao: glow::VertexArray,
    fbo: glow::Framebuffer,
    fbo_texture: glow::Texture,
    backdrop_fbo: glow::Framebuffer,
    backdrop_texture: glow::Texture,
    fbo_internal_format: i32,
    supports_srgb_framebuffer: bool,
    fbo_texture_registered: bool,
    egui_texture_id: Option<egui::TextureId>,
    fbo_width: u32,
    fbo_height: u32,
    layer_texture: glow::Texture,
    layer_tex_dims: (u32, u32),
    lut_texture: glow::Texture,
    lut_cache: HashMap<String, Option<CubeLut3D>>,
    uploaded_lut_path: Option<String>,
    uploaded_lut_size: u32,
    mask_texture: glow::Texture,
    mask_tex_dims: (u32, u32),
    mask_cache: HashMap<String, (u64, Vec<u8>, u32, u32)>,
}

impl GpuCompositor {
    /// Initializes the OpenGL shader program, VAO, off-screen FBO, and layer textures.
    /// Logs errors via `log::error!` and returns `None` without panicking if compilation/linking fails.
    pub fn new(gl: Arc<glow::Context>) -> Option<Self> {
        unsafe {
            let ver = gl.version();
            if (!ver.is_embedded && (ver.major < 3 || (ver.major == 3 && ver.minor < 3)))
                || (ver.is_embedded && ver.major < 3)
            {
                log::warn!(
                    "GPU Compositor: OpenGL/GLES sürümü ({}.{}) 3.3 Core / ES 3.0 altında; CPU fallback etkinleştiriliyor.",
                    ver.major,
                    ver.minor
                );
                return None;
            }

            let header = if ver.is_embedded {
                "#version 300 es\nprecision highp float;\n"
            } else {
                "#version 330 core\n"
            };

            let vs_src = format!("{header}{VERTEX_SHADER_BODY}");
            let fs_src = format!("{header}{FRAGMENT_SHADER_BODY}");

            let vs = match gl.create_shader(glow::VERTEX_SHADER) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("GPU Compositor: Vertex shader oluşturulamadı: {}", e);
                    return None;
                }
            };
            gl.shader_source(vs, &vs_src);
            gl.compile_shader(vs);
            if !gl.get_shader_compile_status(vs) {
                let info = gl.get_shader_info_log(vs);
                log::error!("GPU Compositor: Vertex shader derleme hatası: {}", info);
                gl.delete_shader(vs);
                return None;
            }

            let fs = match gl.create_shader(glow::FRAGMENT_SHADER) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("GPU Compositor: Fragment shader oluşturulamadı: {}", e);
                    gl.delete_shader(vs);
                    return None;
                }
            };
            gl.shader_source(fs, &fs_src);
            gl.compile_shader(fs);
            if !gl.get_shader_compile_status(fs) {
                let info = gl.get_shader_info_log(fs);
                log::error!("GPU Compositor: Fragment shader derleme hatası: {}", info);
                gl.delete_shader(vs);
                gl.delete_shader(fs);
                return None;
            }

            let program = match gl.create_program() {
                Ok(p) => p,
                Err(e) => {
                    log::error!("GPU Compositor: GL program oluşturulamadı: {}", e);
                    gl.delete_shader(vs);
                    gl.delete_shader(fs);
                    return None;
                }
            };
            gl.attach_shader(program, vs);
            gl.attach_shader(program, fs);
            gl.link_program(program);
            gl.delete_shader(vs);
            gl.delete_shader(fs);

            if !gl.get_program_link_status(program) {
                let info = gl.get_program_info_log(program);
                log::error!("GPU Compositor: GL program bağlama hatası: {}", info);
                gl.delete_program(program);
                return None;
            }

            let vao = match gl.create_vertex_array() {
                Ok(v) => v,
                Err(e) => {
                    log::error!("GPU Compositor: VAO oluşturulamadı: {}", e);
                    gl.delete_program(program);
                    return None;
                }
            };

            let fbo_width = 640 * GPU_PREVIEW_SCALE;
            let fbo_height = 360 * GPU_PREVIEW_SCALE;

            let exts = gl.supported_extensions();
            let srgb_textures =
                (ver.is_embedded && ver.major >= 3) || exts.iter().any(|ext| ext.contains("sRGB"));
            let supports_srgb_framebuffer =
                exts.iter().any(|ext| ext.ends_with("ARB_framebuffer_sRGB"));
            let fbo_internal_format = if srgb_textures {
                glow::SRGB8_ALPHA8 as i32
            } else {
                glow::RGBA8 as i32
            };

            let fbo_texture = match Self::create_texture_with_format(
                &gl,
                fbo_width,
                fbo_height,
                fbo_internal_format,
            ) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("GPU Compositor: FBO dokusu oluşturulamadı: {}", e);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return None;
                }
            };

            let fbo = match gl.create_framebuffer() {
                Ok(f) => f,
                Err(e) => {
                    log::error!("GPU Compositor: Framebuffer oluşturulamadı: {}", e);
                    gl.delete_texture(fbo_texture);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return None;
                }
            };

            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(fbo_texture),
                0,
            );
            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            if status != glow::FRAMEBUFFER_COMPLETE {
                log::error!(
                    "GPU Compositor: Framebuffer tamamlanamadı (status: 0x{:X})",
                    status
                );
                gl.delete_framebuffer(fbo);
                gl.delete_texture(fbo_texture);
                gl.delete_vertex_array(vao);
                gl.delete_program(program);
                return None;
            }

            let backdrop_texture = match Self::create_texture_with_format(
                &gl,
                fbo_width,
                fbo_height,
                fbo_internal_format,
            ) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("GPU Compositor: Backdrop dokusu oluşturulamadı: {}", e);
                    gl.delete_framebuffer(fbo);
                    gl.delete_texture(fbo_texture);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return None;
                }
            };

            let backdrop_fbo = match gl.create_framebuffer() {
                Ok(f) => f,
                Err(e) => {
                    log::error!("GPU Compositor: Backdrop FBO oluşturulamadı: {}", e);
                    gl.delete_texture(backdrop_texture);
                    gl.delete_framebuffer(fbo);
                    gl.delete_texture(fbo_texture);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return None;
                }
            };

            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(backdrop_fbo));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(backdrop_texture),
                0,
            );
            let bd_status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            if bd_status != glow::FRAMEBUFFER_COMPLETE {
                log::error!(
                    "GPU Compositor: Backdrop FBO tamamlanamadı (status: 0x{:X})",
                    bd_status
                );
                gl.delete_framebuffer(backdrop_fbo);
                gl.delete_texture(backdrop_texture);
                gl.delete_framebuffer(fbo);
                gl.delete_texture(fbo_texture);
                gl.delete_vertex_array(vao);
                gl.delete_program(program);
                return None;
            }

            let layer_texture = match Self::create_rgba_texture(&gl, 640, 360) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("GPU Compositor: Katman dokusu oluşturulamadı: {}", e);
                    gl.delete_framebuffer(backdrop_fbo);
                    gl.delete_texture(backdrop_texture);
                    gl.delete_framebuffer(fbo);
                    gl.delete_texture(fbo_texture);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return None;
                }
            };

            let lut_texture = match Self::create_rgba_texture(&gl, 16, 4) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("GPU Compositor: LUT dokusu oluşturulamadı: {}", e);
                    gl.delete_texture(layer_texture);
                    gl.delete_framebuffer(backdrop_fbo);
                    gl.delete_texture(backdrop_texture);
                    gl.delete_framebuffer(fbo);
                    gl.delete_texture(fbo_texture);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return None;
                }
            };

            let mask_texture = match Self::create_rgba_texture(&gl, 320, 180) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("GPU Compositor: Maske dokusu oluşturulamadı: {}", e);
                    gl.delete_texture(lut_texture);
                    gl.delete_texture(layer_texture);
                    gl.delete_framebuffer(backdrop_fbo);
                    gl.delete_texture(backdrop_texture);
                    gl.delete_framebuffer(fbo);
                    gl.delete_texture(fbo_texture);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return None;
                }
            };

            Some(Self {
                gl,
                program,
                vao,
                fbo,
                fbo_texture,
                backdrop_fbo,
                backdrop_texture,
                fbo_internal_format,
                supports_srgb_framebuffer,
                fbo_texture_registered: false,
                egui_texture_id: None,
                fbo_width,
                fbo_height,
                layer_texture,
                layer_tex_dims: (640, 360),
                lut_texture,
                lut_cache: HashMap::new(),
                uploaded_lut_path: None,
                uploaded_lut_size: 0,
                mask_texture,
                mask_tex_dims: (320, 180),
                mask_cache: HashMap::new(),
            })
        }
    }

    unsafe fn create_rgba_texture(
        gl: &glow::Context,
        width: u32,
        height: u32,
    ) -> Result<glow::Texture, String> {
        Self::create_texture_with_format(gl, width, height, glow::RGBA8 as i32)
    }

    unsafe fn create_texture_with_format(
        gl: &glow::Context,
        width: u32,
        height: u32,
        internal_format: i32,
    ) -> Result<glow::Texture, String> {
        let tex = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            internal_format,
            width as i32,
            height as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            None,
        );
        gl.bind_texture(glow::TEXTURE_2D, None);
        Ok(tex)
    }

    unsafe fn ensure_fbo_dimensions(&mut self, target_w: u32, target_h: u32) {
        if self.fbo_width == target_w && self.fbo_height == target_h {
            return;
        }
        self.fbo_width = target_w.max(16);
        self.fbo_height = target_h.max(16);
        for tex in [self.fbo_texture, self.backdrop_texture] {
            self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                self.fbo_internal_format,
                self.fbo_width as i32,
                self.fbo_height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                None,
            );
        }
        self.gl.bind_texture(glow::TEXTURE_2D, None);
    }

    unsafe fn upload_layer_frame(&mut self, frame: &image::RgbaImage) {
        let w = frame.width();
        let h = frame.height();
        let raw = frame.as_raw();

        self.gl.active_texture(glow::TEXTURE0);
        self.gl
            .bind_texture(glow::TEXTURE_2D, Some(self.layer_texture));
        self.gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

        if self.layer_tex_dims == (w, h) {
            self.gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                w as i32,
                h as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(raw),
            );
        } else {
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                w as i32,
                h as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(raw),
            );
            self.layer_tex_dims = (w, h);
        }
    }

    unsafe fn prepare_custom_lut(&mut self, lut_path: Option<&str>) -> (bool, i32) {
        let Some(raw_path) = lut_path else {
            return (false, 0);
        };
        if raw_path.is_empty() {
            return (false, 0);
        }
        let norm_buf = trim_core::paths::normalize_path(raw_path);
        let path = norm_buf.to_string_lossy();

        if self.uploaded_lut_path.as_deref() == Some(path.as_ref()) && self.uploaded_lut_size >= 2 {
            self.gl.active_texture(glow::TEXTURE1);
            self.gl
                .bind_texture(glow::TEXTURE_2D, Some(self.lut_texture));
            return (true, self.uploaded_lut_size as i32);
        }

        if !self.lut_cache.contains_key(path.as_ref()) {
            let parsed = parse_cube_lut_file(&norm_buf);
            self.lut_cache.insert(path.to_string(), parsed);
        }

        let Some(Some(lut)) = self.lut_cache.get(path.as_ref()) else {
            return (false, 0);
        };

        let n = lut.size;
        let tex_w = (n * n) as i32;
        let tex_h = n as i32;

        self.gl.active_texture(glow::TEXTURE1);
        self.gl
            .bind_texture(glow::TEXTURE_2D, Some(self.lut_texture));
        self.gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        self.gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            tex_w,
            tex_h,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            Some(&lut.rgba_strip),
        );
        self.uploaded_lut_path = Some(path.to_string());
        self.uploaded_lut_size = n;

        (true, n as i32)
    }

    unsafe fn prepare_clip_masks(
        &mut self,
        clip: &Clip,
        current_time: f64,
        logical_w: u32,
        logical_h: u32,
    ) -> bool {
        let has_enabled = clip.masks.iter().any(|m| m.enabled);
        if !has_enabled {
            return false;
        }
        let clip_time = (current_time - clip.timeline_in).max(0.0);
        let mask_w = (logical_w / 2).max(160);
        let mask_h = (logical_h / 2).max(90);

        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        mask_w.hash(&mut hasher);
        mask_h.hash(&mut hasher);
        let any_kf = clip.masks.iter().any(|m| !m.keyframes.is_empty());
        if any_kf {
            (clip_time * 60.0).round().to_bits().hash(&mut hasher);
        }
        for m in &clip.masks {
            m.id.hash(&mut hasher);
            m.enabled.hash(&mut hasher);
            (m.shape as u8).hash(&mut hasher);
            (m.target as u8).hash(&mut hasher);
            m.invert.hash(&mut hasher);
            m.pos_x.to_bits().hash(&mut hasher);
            m.pos_y.to_bits().hash(&mut hasher);
            m.scale_x.to_bits().hash(&mut hasher);
            m.scale_y.to_bits().hash(&mut hasher);
            m.rotation.to_bits().hash(&mut hasher);
            m.feather.to_bits().hash(&mut hasher);
            m.opacity.to_bits().hash(&mut hasher);
            m.expansion.to_bits().hash(&mut hasher);
            m.vertices.len().hash(&mut hasher);
            for v in &m.vertices {
                v.pos[0].to_bits().hash(&mut hasher);
                v.pos[1].to_bits().hash(&mut hasher);
                v.in_tangent[0].to_bits().hash(&mut hasher);
                v.in_tangent[1].to_bits().hash(&mut hasher);
                v.out_tangent[0].to_bits().hash(&mut hasher);
                v.out_tangent[1].to_bits().hash(&mut hasher);
            }
            m.keyframes.len().hash(&mut hasher);
        }
        let key_hash = hasher.finish();

        let needs_raster = match self.mask_cache.get(&clip.id) {
            Some((h, _, w, h_dim)) => *h != key_hash || *w != mask_w || *h_dim != mask_h,
            None => true,
        };
        if needs_raster {
            let rgba =
                trim_core::mask::rasterize_masks_rgba(&clip.masks, mask_w, mask_h, clip_time);
            self.mask_cache
                .insert(clip.id.clone(), (key_hash, rgba, mask_w, mask_h));
        }

        if let Some((_, rgba, w, h)) = self.mask_cache.get(&clip.id) {
            self.gl.active_texture(glow::TEXTURE3);
            self.gl
                .bind_texture(glow::TEXTURE_2D, Some(self.mask_texture));
            self.gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            if self.mask_tex_dims != (*w, *h) {
                self.gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA8 as i32,
                    *w as i32,
                    *h as i32,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    Some(rgba),
                );
                self.mask_tex_dims = (*w, *h);
            } else {
                self.gl.tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    0,
                    0,
                    *w as i32,
                    *h as i32,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(rgba),
                );
            }
            self.gl.active_texture(glow::TEXTURE0);
            true
        } else {
            false
        }
    }

    /// Composites all visible `layers` onto the off-screen FBO using the GLSL shader pipeline
    /// (supporting ping-pong backdrop reads for blend modes, adjustment layers, and masks)
    /// and returns the bound `egui::TextureId`.
    pub fn render_layers(
        &mut self,
        project: &Project,
        layers: &[CompositorLayer],
        current_time: f64,
        frame: &mut eframe::Frame,
    ) -> Option<egui::TextureId> {
        if layers.is_empty() {
            return None;
        }

        let (logical_w, logical_h) =
            calculate_preview_size(project.timeline.width, project.timeline.height);
        let target_fbo_w = logical_w * GPU_PREVIEW_SCALE;
        let target_fbo_h = logical_h * GPU_PREVIEW_SCALE;

        unsafe {
            self.ensure_fbo_dimensions(target_fbo_w, target_fbo_h);

            let mut prev_viewport = [0i32; 4];
            self.gl
                .get_parameter_i32_slice(glow::VIEWPORT, &mut prev_viewport);

            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
            if self.supports_srgb_framebuffer {
                self.gl.disable(glow::FRAMEBUFFER_SRGB);
            }
            self.gl
                .viewport(0, 0, self.fbo_width as i32, self.fbo_height as i32);
            self.gl.disable(glow::SCISSOR_TEST);
            self.gl.disable(glow::DEPTH_TEST);

            self.gl.clear_color(0.0, 0.0, 0.0, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);

            self.gl.enable(glow::BLEND);
            self.gl.blend_func_separate(
                glow::SRC_ALPHA,
                glow::ONE_MINUS_SRC_ALPHA,
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
            );

            self.gl.use_program(Some(self.program));
            self.gl.bind_vertex_array(Some(self.vao));

            if let Some(loc) = self.gl.get_uniform_location(self.program, "u_frame_tex") {
                self.gl.uniform_1_i32(Some(&loc), 0);
            }
            if let Some(loc) = self.gl.get_uniform_location(self.program, "u_lut_tex") {
                self.gl.uniform_1_i32(Some(&loc), 1);
            }
            if let Some(loc) = self.gl.get_uniform_location(self.program, "u_backdrop_tex") {
                self.gl.uniform_1_i32(Some(&loc), 2);
            }
            if let Some(loc) = self.gl.get_uniform_location(self.program, "u_mask_tex") {
                self.gl.uniform_1_i32(Some(&loc), 3);
            }

            for layer in layers {
                let Some(u) = LayerUniforms::from_clip(
                    &layer.clip,
                    layer.frame.width(),
                    layer.frame.height(),
                    logical_w,
                    logical_h,
                    current_time,
                ) else {
                    continue;
                };

                let needs_backdrop = u.blend_mode != 0 || u.is_adjustment_layer;
                if needs_backdrop {
                    self.gl
                        .bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.fbo));
                    self.gl
                        .bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(self.backdrop_fbo));
                    self.gl.blit_framebuffer(
                        0,
                        0,
                        self.fbo_width as i32,
                        self.fbo_height as i32,
                        0,
                        0,
                        self.fbo_width as i32,
                        self.fbo_height as i32,
                        glow::COLOR_BUFFER_BIT,
                        glow::NEAREST,
                    );
                    self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
                    self.gl.disable(glow::BLEND);
                    self.gl.active_texture(glow::TEXTURE2);
                    self.gl
                        .bind_texture(glow::TEXTURE_2D, Some(self.backdrop_texture));
                    self.gl.active_texture(glow::TEXTURE0);
                } else {
                    self.gl.enable(glow::BLEND);
                }

                if !u.is_adjustment_layer {
                    self.upload_layer_frame(&layer.frame);
                }
                let has_mask =
                    self.prepare_clip_masks(&layer.clip, current_time, logical_w, logical_h);
                let (custom_lut_enabled, lut_size) =
                    self.prepare_custom_lut(layer.clip.custom_lut_path.as_deref());

                let gl = &self.gl;
                let prog = self.program;

                if let Some(loc) = gl.get_uniform_location(prog, "u_has_mask") {
                    gl.uniform_1_i32(Some(&loc), i32::from(has_mask));
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_blend_mode") {
                    gl.uniform_1_i32(Some(&loc), u.blend_mode);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_is_adjustment_layer") {
                    gl.uniform_1_i32(Some(&loc), i32::from(u.is_adjustment_layer));
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_canvas_size") {
                    gl.uniform_2_f32(Some(&loc), u.canvas_size[0], u.canvas_size[1]);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_clip_center") {
                    gl.uniform_2_f32(Some(&loc), u.clip_center[0], u.clip_center[1]);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_clip_size") {
                    gl.uniform_2_f32(Some(&loc), u.clip_size[0], u.clip_size[1]);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_rotation_rad") {
                    gl.uniform_1_f32(Some(&loc), u.rotation_rad);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_opacity") {
                    gl.uniform_1_f32(Some(&loc), u.opacity);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_crop") {
                    gl.uniform_4_f32(Some(&loc), u.crop[0], u.crop[1], u.crop[2], u.crop[3]);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_wipe_mode") {
                    gl.uniform_1_i32(Some(&loc), u.wipe_mode);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_fade_factor") {
                    gl.uniform_1_f32(Some(&loc), u.fade_factor);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_dip_white") {
                    gl.uniform_1_f32(Some(&loc), u.dip_white);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_ck_enabled") {
                    gl.uniform_1_i32(Some(&loc), i32::from(u.ck_enabled));
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_ck_color") {
                    gl.uniform_3_f32(Some(&loc), u.ck_color[0], u.ck_color[1], u.ck_color[2]);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_ck_similarity") {
                    gl.uniform_1_f32(Some(&loc), u.ck_similarity);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_ck_smoothness") {
                    gl.uniform_1_f32(Some(&loc), u.ck_smoothness);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_cw_enabled") {
                    gl.uniform_1_i32(Some(&loc), i32::from(u.cw_enabled));
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_cw_lift") {
                    gl.uniform_3_f32(Some(&loc), u.cw_lift[0], u.cw_lift[1], u.cw_lift[2]);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_cw_gamma") {
                    gl.uniform_3_f32(Some(&loc), u.cw_gamma[0], u.cw_gamma[1], u.cw_gamma[2]);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_cw_gain") {
                    gl.uniform_3_f32(Some(&loc), u.cw_gain[0], u.cw_gain[1], u.cw_gain[2]);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_wb_gains") {
                    gl.uniform_3_f32(Some(&loc), u.wb_gains[0], u.wb_gains[1], u.wb_gains[2]);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_contrast") {
                    gl.uniform_1_f32(Some(&loc), u.contrast);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_brightness") {
                    gl.uniform_1_f32(Some(&loc), u.brightness);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_gamma") {
                    gl.uniform_1_f32(Some(&loc), u.gamma);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_saturation") {
                    gl.uniform_1_f32(Some(&loc), u.saturation);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_lut_preset") {
                    gl.uniform_1_i32(Some(&loc), u.lut_preset);
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_custom_lut_enabled") {
                    gl.uniform_1_i32(Some(&loc), i32::from(custom_lut_enabled));
                }
                if let Some(loc) = gl.get_uniform_location(prog, "u_lut_size") {
                    gl.uniform_1_i32(Some(&loc), lut_size);
                }

                gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            }

            self.gl.bind_vertex_array(None);
            self.gl.use_program(None);
            self.gl.active_texture(glow::TEXTURE3);
            self.gl.bind_texture(glow::TEXTURE_2D, None);
            self.gl.active_texture(glow::TEXTURE2);
            self.gl.bind_texture(glow::TEXTURE_2D, None);
            self.gl.active_texture(glow::TEXTURE1);
            self.gl.bind_texture(glow::TEXTURE_2D, None);
            self.gl.active_texture(glow::TEXTURE0);
            self.gl.bind_texture(glow::TEXTURE_2D, None);
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            self.gl.viewport(
                prev_viewport[0],
                prev_viewport[1],
                prev_viewport[2],
                prev_viewport[3],
            );
        }

        if self.egui_texture_id.is_none() {
            let tex_id = frame.register_native_glow_texture(self.fbo_texture);
            self.fbo_texture_registered = true;
            self.egui_texture_id = Some(tex_id);
        }

        self.egui_texture_id
    }

    /// Reads back the current composited FBO frame into an RGBA8 buffer for real-time color scopes.
    pub fn readback_fbo_rgba(&self) -> Option<(Vec<u8>, u32, u32)> {
        if self.fbo_width == 0 || self.fbo_height == 0 {
            return None;
        }
        let w = self.fbo_width;
        let h = self.fbo_height;
        let byte_len = (w as usize) * (h as usize) * 4;
        let mut buf = vec![0u8; byte_len];
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
            self.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            self.gl.read_pixels(
                0,
                0,
                w as i32,
                h as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(&mut buf),
            );
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
        Some((buf, w, h))
    }
}

impl Drop for GpuCompositor {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_texture(self.mask_texture);
            self.gl.delete_texture(self.layer_texture);
            self.gl.delete_texture(self.lut_texture);
            self.gl.delete_framebuffer(self.backdrop_fbo);
            self.gl.delete_texture(self.backdrop_texture);
            self.gl.delete_framebuffer(self.fbo);
            if !self.fbo_texture_registered {
                self.gl.delete_texture(self.fbo_texture);
            }
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_program(self.program);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trim_core::clip::{
        BrightnessContrastPreset, ChromaKeyConfig, Clip, ColorWheelsConfig, LutPreset,
        WhiteBalancePreset,
    };

    #[test]
    fn test_layer_uniforms_affine_transform_and_grading() {
        let mut clip = Clip::new("media_1", 0.0, 10.0);
        clip.scale = 1.5;
        clip.pos_x = 40.0;
        clip.pos_y = -20.0;
        clip.rotation = 90.0;
        clip.opacity = 0.8;
        clip.apply_wb_preset(WhiteBalancePreset::Warm);
        clip.apply_bc_preset(BrightnessContrastPreset::CinematicContrast);
        clip.contrast = 1.2;
        clip.brightness = 0.1;
        clip.saturation = 1.3;
        clip.lut_preset = LutPreset::Kodak2383;
        clip.chromakey = Some(ChromaKeyConfig {
            enabled: true,
            key_color: [0, 255, 0],
            similarity: 0.25,
            smoothness: 0.08,
        });
        clip.color_wheels = Some(ColorWheelsConfig {
            lift: [0.1, 0.0, -0.1],
            gamma: [1.1, 1.0, 0.9],
            gain: [1.2, 1.0, 0.8],
            luminance: [0.05, 0.0, -0.05],
        });

        clip.crop_left = 0.1;
        clip.crop_top = 0.05;
        clip.crop_right = 0.15;
        clip.crop_bottom = 0.2;

        let u = LayerUniforms::from_clip(&clip, 640, 360, 640, 360, 2.0)
            .expect("Should produce active layer uniforms");

        assert_eq!(u.canvas_size, [640.0, 360.0]);
        assert_eq!(u.clip_size, [960.0, 540.0]);
        assert_eq!(u.clip_center, [360.0, 160.0]);
        assert!((u.rotation_rad - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert!((u.opacity - 0.8).abs() < 1e-5);
        assert_eq!(u.crop, [0.1, 0.05, 0.15, 0.2]);
        assert!(u.ck_enabled);
        assert_eq!(u.ck_color, [0.0, 1.0, 0.0]);
        assert!((u.ck_similarity - 0.25).abs() < 1e-5);
        assert!((u.ck_smoothness - 0.08).abs() < 1e-5);
        assert!(u.cw_enabled);
        assert!((u.cw_lift[0] - 0.15).abs() < 1e-5);
        assert!((u.cw_gamma[0] - 1.1).abs() < 1e-5);
        assert!((u.cw_gain[0] - 1.15).abs() < 1e-5);
        assert!(u.wb_gains[0] > 1.0 && u.wb_gains[2] < 1.0);
        assert!((u.gamma - 0.90).abs() < 1e-4);
        assert_eq!(u.lut_preset, LutPreset::Kodak2383.shader_id());
    }

    #[test]
    fn test_layer_uniforms_zero_opacity_returns_none() {
        let mut clip = Clip::new("media_1", 0.0, 5.0);
        clip.opacity = 0.0;
        assert!(LayerUniforms::from_clip(&clip, 640, 360, 640, 360, 1.0).is_none());
    }
}
