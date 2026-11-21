pub(super) const VERTEX_SHADER_BODY: &str = r#"
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

pub(super) const FRAGMENT_SHADER_BODY: &str = r#"
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
uniform int u_camera_log_profile;
uniform int u_lut_preset;
uniform int u_custom_lut_enabled;
uniform int u_lut_size;
uniform float u_blur;
uniform float u_sharpen;
uniform float u_vignette;
uniform float u_film_grain;
uniform float u_halation;
uniform float u_bloom;
uniform float u_chromatic_aberration;
uniform int u_film_format;
uniform float u_highlight_rolloff;
uniform float u_diffusion;
uniform float u_mist;
uniform float u_gate_weave;
uniform int u_cp_enabled;
uniform vec2 u_cp_tl;
uniform vec2 u_cp_tr;
uniform vec2 u_cp_br;
uniform vec2 u_cp_bl;
uniform float u_glitch_intensity;
uniform float u_light_leak;
uniform float u_time_seed;

float log_to_linear_chan(float x, int p) {
    if (p == 1) { // Sony S-Log3
        return (x >= 0.100537775)
            ? ((pow(10.0, (x - 0.790453) / 0.344676) - 0.009468) / 0.555556)
            : ((x - 0.092864) / 8.735631);
    }
    if (p == 2) { // Canon C-Log
        return (x >= 0.100686685)
            ? ((pow(10.0, (x - 0.384316) / 0.245281) - 0.064829) / 5.555556)
            : ((x - 0.092864) / 8.799461);
    }
    if (p == 3) { // Panasonic V-Log
        float cut = 171.2102946929 / 1023.0;
        return (x >= cut)
            ? (pow(10.0, (x * 1023.0 - 420.0) / 261.5) * 0.19 - 0.01)
            : (((x * 1023.0 - 95.0) * 0.01125) / (171.2102946929 - 95.0));
    }
    if (p == 4) { // ARRI Log-C
        return (x > 0.14)
            ? ((pow(10.0, 3.89616 * x - 2.27752) - 0.0929) / 0.9892)
            : ((x - 0.0929) / 6.025);
    }
    return x;
}

float rec709_oetf_chan(float l) {
    float v = clamp(l, 0.0, 1.0);
    return (v < 0.018) ? (4.5 * v) : (1.099 * pow(v, 0.45) - 0.099);
}

vec3 apply_camera_log_profile(vec3 c, int p) {
    if (p <= 0) return c;
    return clamp(vec3(
        rec709_oetf_chan(log_to_linear_chan(c.r, p)),
        rec709_oetf_chan(log_to_linear_chan(c.g, p)),
        rec709_oetf_chan(log_to_linear_chan(c.b, p))
    ), 0.0, 1.0);
}

vec2 invert_bilinear_quad(vec2 p, vec2 p00, vec2 p10, vec2 p11, vec2 p01) {
    vec2 u = 0.5 * vec2(1.0);
    for (int iter = 0; iter < 6; iter++) {
        float a = 1.0 - u.x;
        float b = 1.0 - u.y;
        vec2 f = a * b * p00 + u.x * b * p10 + u.x * u.y * p11 + a * u.y * p01 - p;
        if (dot(f, f) < 1e-7) break;
        vec2 dfdu = -b * p00 + b * p10 + u.y * p11 - u.y * p01;
        vec2 dfdv = -a * p00 - u.x * p10 + u.x * p11 + a * p01;
        float det = dfdu.x * dfdv.y - dfdu.y * dfdv.x;
        if (abs(det) < 1e-7) return vec2(-1.0);
        float inv_det = 1.0 / det;
        u.x -= (dfdv.y * f.x - dfdv.x * f.y) * inv_det;
        u.y -= (-dfdu.y * f.x + dfdu.x * f.y) * inv_det;
    }
    return u;
}

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
    vec2 local_uv = v_canvas_uv;

    if (u_is_adjustment_layer == 1) {
        vec4 bd = texture(u_backdrop_tex, v_canvas_uv);
        rgb = bd.rgb;
        a = u_opacity;
    } else {
        vec2 uv;
        if (u_cp_enabled == 1) {
            uv = invert_bilinear_quad(v_canvas_uv, u_cp_tl, u_cp_tr, u_cp_br, u_cp_bl);
        } else {
            vec2 canvas_px = v_canvas_uv * u_canvas_size;
            vec2 rel = canvas_px - u_clip_center;

            float cos_r = cos(-u_rotation_rad);
            float sin_r = sin(-u_rotation_rad);
            vec2 rot_rel = vec2(
                rel.x * cos_r - rel.y * sin_r,
                rel.x * sin_r + rel.y * cos_r
            );

            uv = (rot_rel / max(u_clip_size, vec2(1.0))) + vec2(0.5);
        }
        if (u_gate_weave > 0.001) {
            float wx = sin(u_time_seed * 41.0) * 0.0035 * u_gate_weave;
            float wy = cos(u_time_seed * 67.0) * 0.0045 * u_gate_weave;
            uv += vec2(wx, wy);
        }
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
        local_uv = uv;

        vec2 texel_step = vec2(1.0) / max(u_clip_size, vec2(1.0));
        vec4 texel = texture(u_frame_tex, uv);
        rgb = texel.rgb;
        a = texel.a * u_opacity;

        if (u_chromatic_aberration > 0.001) {
            vec2 ca_vec = (uv - vec2(0.5)) * (u_chromatic_aberration * 0.025);
            float r_ca = texture(u_frame_tex, clamp(uv + ca_vec, 0.0, 1.0)).r;
            float b_ca = texture(u_frame_tex, clamp(uv - ca_vec, 0.0, 1.0)).b;
            rgb = vec3(r_ca, rgb.g, b_ca);
        }

        if (u_glitch_intensity > 0.001) {
            float shift = u_glitch_intensity * 0.028 * sin(uv.y * 42.0 + u_time_seed * 17.0);
            float r_g = texture(u_frame_tex, clamp(uv + vec2(shift, 0.0), 0.0, 1.0)).r;
            float b_g = texture(u_frame_tex, clamp(uv - vec2(shift, 0.0), 0.0, 1.0)).b;
            rgb = vec3(r_g, rgb.g, b_g);
        }

        if (u_blur > 0.01) {
            vec2 r1 = texel_step * u_blur * 1.4;
            vec2 r2 = texel_step * u_blur * 2.8;
            vec3 acc = rgb * 0.24;
            acc += texture(u_frame_tex, clamp(uv + vec2(r1.x, 0.0), 0.0, 1.0)).rgb * 0.12;
            acc += texture(u_frame_tex, clamp(uv - vec2(r1.x, 0.0), 0.0, 1.0)).rgb * 0.12;
            acc += texture(u_frame_tex, clamp(uv + vec2(0.0, r1.y), 0.0, 1.0)).rgb * 0.12;
            acc += texture(u_frame_tex, clamp(uv - vec2(0.0, r1.y), 0.0, 1.0)).rgb * 0.12;
            acc += texture(u_frame_tex, clamp(uv + vec2(r2.x, r2.y), 0.0, 1.0)).rgb * 0.07;
            acc += texture(u_frame_tex, clamp(uv + vec2(-r2.x, r2.y), 0.0, 1.0)).rgb * 0.07;
            acc += texture(u_frame_tex, clamp(uv + vec2(r2.x, -r2.y), 0.0, 1.0)).rgb * 0.07;
            acc += texture(u_frame_tex, clamp(uv + vec2(-r2.x, -r2.y), 0.0, 1.0)).rgb * 0.07;
            rgb = acc;
        }

        if (u_sharpen > 0.001) {
            vec3 n_avg = (
                texture(u_frame_tex, clamp(uv + vec2(texel_step.x, 0.0), 0.0, 1.0)).rgb +
                texture(u_frame_tex, clamp(uv - vec2(texel_step.x, 0.0), 0.0, 1.0)).rgb +
                texture(u_frame_tex, clamp(uv + vec2(0.0, texel_step.y), 0.0, 1.0)).rgb +
                texture(u_frame_tex, clamp(uv - vec2(0.0, texel_step.y), 0.0, 1.0)).rgb
            ) * 0.25;
            rgb = clamp(rgb + (rgb - n_avg) * (u_sharpen * 1.8), 0.0, 1.0);
        }

        if (u_wipe_mode == 3) {
            float luma_w = dot(rgb, vec3(0.299, 0.587, 0.114));
            float thresh = 1.0 - u_fade_factor;
            float luma_alpha = smoothstep(thresh - 0.15, thresh + 0.15, luma_w);
            a *= clamp(mix(u_fade_factor, luma_alpha, 0.8), 0.0, 1.0);
        }

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

    if (u_camera_log_profile > 0) {
        rgb = apply_camera_log_profile(rgb, u_camera_log_profile);
    }

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

    if (u_film_format > 0 || u_highlight_rolloff > 0.001) {
        float fmt_c = (u_film_format == 3) ? 0.17 : ((u_film_format == 2) ? 0.11 : ((u_film_format == 1) ? 0.06 : 0.0));
        float fmt_r = (u_film_format == 3) ? 0.30 : ((u_film_format == 2) ? 0.22 : ((u_film_format == 1) ? 0.14 : 0.0));
        float warm_s = (u_film_format == 3) ? 0.045 : ((u_film_format == 2) ? 0.028 : ((u_film_format == 1) ? 0.015 : 0.0));
        float sh_lift = (u_film_format == 3) ? 0.05 : ((u_film_format == 2) ? 0.035 : ((u_film_format == 1) ? 0.02 : 0.0));
        float sat_sc = (u_film_format == 3) ? 0.84 : ((u_film_format == 2) ? 0.91 : ((u_film_format == 1) ? 0.96 : 1.0));

        vec3 inv_c = vec3(1.0) - rgb;
        rgb.r = clamp(rgb.r + warm_s + sh_lift * inv_c.r * inv_c.r, 0.0, 1.0);
        rgb.g = clamp(rgb.g + warm_s * 0.25 + sh_lift * 0.85 * inv_c.g * inv_c.g, 0.0, 1.0);
        rgb.b = clamp(rgb.b - warm_s * 0.75 + sh_lift * 1.15 * inv_c.b * inv_c.b, 0.0, 1.0);

        if (fmt_c > 0.001) {
            rgb = clamp((rgb - vec3(0.5)) * (1.0 + fmt_c) + vec3(0.5), 0.0, 1.0);
        }

        float roll = clamp(fmt_r + u_highlight_rolloff * 0.65, 0.0, 1.0);
        if (roll > 0.001) {
            float knee = 0.62 - roll * 0.18;
            float denom = max(0.001, 1.0 - knee);
            float k_exp = 1.0 + roll * 1.5;
            float e_denom = max(0.0001, 1.0 - exp(-k_exp));
            vec3 over = max(vec3(0.0), rgb - vec3(knee));
            vec3 t = over / denom;
            vec3 comp = vec3(knee) + denom * (vec3(1.0) - exp(-t * k_exp)) / e_denom;
            vec3 mask = step(vec3(knee), rgb);
            rgb = clamp(mix(rgb, mix(rgb, comp, roll), mask), 0.0, 1.0);
        }

        if (abs(sat_sc - 1.0) > 0.001) {
            float fl = dot(rgb, vec3(0.2126, 0.7152, 0.0722));
            rgb = clamp(mix(vec3(fl), rgb, sat_sc), 0.0, 1.0);
        }
    }

    if (u_halation > 0.001) {
        float luma_h = dot(rgb, vec3(0.299, 0.587, 0.114));
        float hi_mask = clamp((luma_h - 0.55) / 0.45, 0.0, 1.0);
        float glow = hi_mask * hi_mask * u_halation;
        rgb = clamp(rgb + vec3(0.28, 0.09, -0.04) * glow, 0.0, 1.0);
    }

    if (u_bloom > 0.001) {
        float luma_b = dot(rgb, vec3(0.299, 0.587, 0.114));
        float hi_mask = clamp((luma_b - 0.45) / 0.55, 0.0, 1.0);
        float glow = hi_mask * u_bloom * 0.22;
        rgb = clamp(rgb + vec3(glow), 0.0, 1.0);
    }

    if (u_diffusion > 0.001) {
        float luma_d = dot(rgb, vec3(0.2126, 0.7152, 0.0722));
        vec3 soft_lift = max(rgb, vec3(luma_d * 0.92 + 0.06));
        rgb = clamp(mix(rgb, soft_lift, u_diffusion * 0.35), 0.0, 1.0);
    }

    if (u_mist > 0.001) {
        float luma_m = dot(rgb, vec3(0.2126, 0.7152, 0.0722));
        float mist_mask = smoothstep(0.25, 0.92, luma_m);
        vec3 mist_glow = vec3(0.94, 0.96, 1.0) * mist_mask * u_mist * 0.22;
        rgb = clamp(1.0 - (1.0 - rgb) * (1.0 - mist_glow), 0.0, 1.0);
    }

    if (u_vignette > 0.001) {
        vec2 vd = (local_uv - vec2(0.5)) * vec2(1.42, 1.42);
        float r2 = dot(vd, vd);
        float vig = clamp(1.0 - r2 * u_vignette * 1.15, 0.0, 1.0);
        rgb *= vig;
    }

    if (u_film_grain > 0.001) {
        float n_hash = fract(sin(dot(v_canvas_uv * u_canvas_size + vec2(u_time_seed * 37.0, u_time_seed * 91.0), vec2(12.9898, 78.233))) * 43758.5453);
        float grain = (n_hash - 0.5) * u_film_grain * 0.22;
        rgb = clamp(rgb + vec3(grain), 0.0, 1.0);
    }

    if (u_has_mask == 1) {
        rgb = mix(ungraded_rgb, rgb, mask_val.g);
    }

    if (u_light_leak > 0.001) {
        float leak_grad = clamp(1.1 - distance(local_uv, vec2(0.2, 0.3)), 0.0, 1.0);
        vec3 leak_color = vec3(1.0, 0.52, 0.14) * u_light_leak * (0.45 + 0.55 * leak_grad);
        rgb = clamp(1.0 - (1.0 - rgb) * (1.0 - leak_color), 0.0, 1.0);
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
