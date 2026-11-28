//! Unified color grading, White Balance (Temperature/Tint), Auto WB analysis,
//! Brightness/Contrast/Gamma presets, and categorized Creative Color Looks.

mod curves_and_hsl;
mod presets;

pub use curves_and_hsl::*;
pub use presets::*;

/// Smooth S-curve contrast helper on `[0.0, 1.0]`, where `amount` is in `[0.0, 1.0]`.
#[inline]
pub fn s_curve(x: f32, amount: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let smooth = x * x * (3.0 - 2.0 * x);
    x + amount * (smooth - x)
}

/// Soft filmic shoulder/toe curve on `[0.0, 1.0]`.
#[inline]
pub fn filmic_tone(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let toe = x * x * (3.0 - 2.0 * x);
    let shoulder = 1.0 - (1.0 - x) * (1.0 - x);
    0.02 + 0.96 * (0.65 * toe + 0.35 * shoulder)
}

/// Computes `(wb_r, wb_g, wb_b)` multipliers from `temperature` (`-100..=100`) and `tint` (`-100..=100`).
#[inline]
pub fn white_balance_gains(temperature: f32, tint: f32) -> [f32; 3] {
    let temp = (temperature / 100.0).clamp(-1.0, 1.0);
    let tnt = (tint / 100.0).clamp(-1.0, 1.0);
    let r = (1.0 + temp * 0.28 + tnt * 0.10).max(0.1);
    let g = (1.0 - temp * 0.04 - tnt * 0.18).max(0.1);
    let b = (1.0 - temp * 0.28 + tnt * 0.10).max(0.1);
    [r, g, b]
}

/// Complete per-clip color grading parameters evaluated for a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorGradeParams {
    pub temperature: f32,
    pub tint: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub gamma: f32,
    pub saturation: f32,
    pub lut_preset: LutPreset,
}

impl Default for ColorGradeParams {
    fn default() -> Self {
        Self {
            temperature: 0.0,
            tint: 0.0,
            brightness: 0.0,
            contrast: 1.0,
            gamma: 1.0,
            saturation: 1.0,
            lut_preset: LutPreset::None,
        }
    }
}

/// Applies White Balance (Temperature/Tint), Contrast, Brightness, Gamma, Saturation, and Look Preset
/// to a normalized `[0.0, 1.0]` RGB pixel.
#[inline]
pub fn apply_color_grade_normalized(
    mut r: f32,
    mut g: f32,
    mut b: f32,
    params: &ColorGradeParams,
) -> [f32; 3] {
    // 1. White Balance (Temperature & Tint)
    if params.temperature.abs() > 1e-4 || params.tint.abs() > 1e-4 {
        let [wbr, wbg, wbb] = white_balance_gains(params.temperature, params.tint);
        r = (r * wbr).clamp(0.0, 1.0);
        g = (g * wbg).clamp(0.0, 1.0);
        b = (b * wbb).clamp(0.0, 1.0);
    }

    // 2. Contrast & Brightness
    let mid = 128.0 / 255.0;
    r = (params.contrast * (r - mid) + mid + params.brightness).clamp(0.0, 1.0);
    g = (params.contrast * (g - mid) + mid + params.brightness).clamp(0.0, 1.0);
    b = (params.contrast * (b - mid) + mid + params.brightness).clamp(0.0, 1.0);

    // 3. Gamma (midtone power curve matching FFmpeg eq=gamma)
    if (params.gamma - 1.0).abs() > 1e-4 {
        let inv_gamma = 1.0 / params.gamma.max(0.1);
        r = r.max(0.0).powf(inv_gamma).clamp(0.0, 1.0);
        g = g.max(0.0).powf(inv_gamma).clamp(0.0, 1.0);
        b = b.max(0.0).powf(inv_gamma).clamp(0.0, 1.0);
    }

    // 4. Saturation
    if (params.saturation - 1.0).abs() > 1e-4 {
        let luma = 0.299 * r + 0.587 * g + 0.114 * b;
        r = (luma + params.saturation * (r - luma)).clamp(0.0, 1.0);
        g = (luma + params.saturation * (g - luma)).clamp(0.0, 1.0);
        b = (luma + params.saturation * (b - luma)).clamp(0.0, 1.0);
    }

    // 5. Creative Color Look Preset
    if params.lut_preset != LutPreset::None {
        [r, g, b] = params.lut_preset.apply_rgb(r, g, b);
    }

    [r, g, b]
}

/// Analyzes an RGBA8 image buffer to compute corrective `(temperature, tint)` values
/// in `[-100.0, 100.0]` using a midtone-weighted Gray-World + chromatic cast estimator.
///
/// - If the image has a cool/blue cast (`B > R`), returns a positive `temperature` (warms the image).
/// - If the image has a warm/orange cast (`R > B`), returns a negative `temperature` (cools the image).
/// - If the image has a green cast (`G > avg(R,B)`), returns a positive `tint` (shifts toward magenta).
/// - If the image has a magenta cast (`G < avg(R,B)`), returns a negative `tint` (shifts toward green).
pub fn analyze_auto_white_balance_rgba(rgba: &[u8], width: u32, height: u32) -> (f64, f64) {
    let expected_len = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4);
    if rgba.len() < expected_len || expected_len == 0 {
        return (0.0, 0.0);
    }

    // Subsample large frames for sub-millisecond performance
    let total_pixels = (width as usize) * (height as usize);
    let step_px = (total_pixels / 16384).max(1);

    let mut sum_r = 0.0_f64;
    let mut sum_g = 0.0_f64;
    let mut sum_b = 0.0_f64;
    let mut weight_sum = 0.0_f64;

    for px_idx in (0..total_pixels).step_by(step_px) {
        let i = px_idx * 4;
        let a = rgba[i + 3];
        if a < 32 {
            continue;
        }
        let r = rgba[i] as f64;
        let g = rgba[i + 1] as f64;
        let b = rgba[i + 2] as f64;

        let max_c = r.max(g).max(b);
        let min_c = r.min(g).min(b);
        // Exclude clipped highlights and deep shadow noise
        if !(16.0..=246.0).contains(&max_c) {
            continue;
        }

        let luma = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
        // Midtone bell weight (peaks at luma = 0.5)
        let luma_w = (4.0 * luma * (1.0 - luma)).max(0.05);
        // Favor near-neutral / moderately saturated pixels over extreme neon colors
        let chroma = (max_c - min_c) / 255.0;
        let neutrality_w = (1.0 - chroma * 0.75).max(0.15);
        let w = luma_w * neutrality_w;

        sum_r += r * w;
        sum_g += g * w;
        sum_b += b * w;
        weight_sum += w;
    }

    if weight_sum <= 1e-6 {
        return (0.0, 0.0);
    }

    let mean_r = (sum_r / weight_sum).max(1.0);
    let mean_g = (sum_g / weight_sum).max(1.0);
    let mean_b = (sum_b / weight_sum).max(1.0);

    // Log-ratio chromaticity imbalance:
    // If mean_b > mean_r (blue cast), ln(mean_b / mean_r) > 0 -> positive temperature needed
    let rb_log = (mean_b / mean_r).ln();
    // Target: r * (1 + 0.28 * t) / (b * (1 - 0.28 * t)) ≈ 1 => 0.56 * t ≈ ln(b / r)
    let raw_temp = (rb_log / 0.56) * 100.0;

    // Green vs Red-Blue geometric mean:
    // If mean_g > sqrt(mean_r * mean_b) (green cast), positive tint (magenta) needed
    let rb_geom = (mean_r * mean_b).sqrt().max(1.0);
    let gm_log = (mean_g / rb_geom).ln();
    let raw_tint = (gm_log / 0.28) * 100.0;

    // Apply conservative damping (0.85) so Auto WB feels natural and never over-corrects
    let temp = ((raw_temp * 0.85).clamp(-100.0, 100.0) * 2.0).round() / 2.0;
    let tint = ((raw_tint * 0.85).clamp(-100.0, 100.0) * 2.0).round() / 2.0;

    (temp, tint)
}

/// Computes corrective `(temperature, tint)` values in `[-100.0, 100.0]` from a single
/// 8-bit RGB pixel picked with the White Balance Eyedropper so that the sampled pixel becomes neutral gray.
pub fn compute_wb_from_neutral_rgb(r: u8, g: u8, b: u8) -> (f64, f64) {
    let rf = (r as f64).max(4.0);
    let gf = (g as f64).max(4.0);
    let bf = (b as f64).max(4.0);

    // Blue > Red => positive temperature (warms); Red > Blue => negative temperature (cools)
    let rb_log = (bf / rf).ln();
    let raw_temp = (rb_log / 0.56) * 100.0;

    // Green > sqrt(R*B) => positive tint (shifts toward magenta)
    let rb_geom = (rf * bf).sqrt().max(4.0);
    let gm_log = (gf / rb_geom).ln();
    let raw_tint = (gm_log / 0.28) * 100.0;

    let temp = (raw_temp.clamp(-100.0, 100.0) * 2.0).round() / 2.0;
    let tint = (raw_tint.clamp(-100.0, 100.0) * 2.0).round() / 2.0;
    (temp, tint)
}

/// Bakes a clip's complete color grading pipeline (Camera Log profile, 3-Point Color Wheels,
/// White Balance, Contrast/Brightness/Gamma/Saturation, Creative Look Preset, Custom RGB/Luma Curves,
/// 6-Vector HSL Curves, and 3D HSL Qualifier) into a standard Adobe/Resolve `.cube` 3D LUT string.
pub fn bake_clip_grade_to_cube(clip: &crate::clip::Clip, size: usize) -> String {
    use std::fmt::Write;
    let n = size.clamp(17, 65);
    let title = if clip.name.trim().is_empty() {
        "triM Color Grade".to_string()
    } else {
        clip.name
            .chars()
            .filter(|c| *c != '"' && *c != '\n' && *c != '\r')
            .collect::<String>()
    };
    let mut out = String::with_capacity(n * n * n * 28 + 160);
    let _ = writeln!(out, "# Generated by triM. Color Engine");
    let _ = writeln!(out, "TITLE \"{title}\"");
    let _ = writeln!(out, "LUT_3D_SIZE {n}");
    let _ = writeln!(out, "DOMAIN_MIN 0.0 0.0 0.0");
    let _ = writeln!(out, "DOMAIN_MAX 1.0 1.0 1.0");

    let denom = (n - 1) as f32;
    for b_idx in 0..n {
        let b_in = b_idx as f32 / denom;
        for g_idx in 0..n {
            let g_in = g_idx as f32 / denom;
            for r_idx in 0..n {
                let r_in = r_idx as f32 / denom;
                let [ro, go, bo] = clip.evaluate_color_pipeline_rgb(r_in, g_in, b_in);
                let _ = writeln!(
                    out,
                    "{:.6} {:.6} {:.6}",
                    ro.clamp(0.0, 1.0),
                    go.clamp(0.0, 1.0),
                    bo.clamp(0.0, 1.0)
                );
            }
        }
    }
    out
}

/// Computes automatic Shot Match grading parameters `(brightness, contrast, saturation, temperature, tint)`
/// to match `src_rgba` statistics to `ref_rgba` statistics.
pub fn analyze_shot_match_rgba(
    src_rgba: &[u8],
    src_w: u32,
    src_h: u32,
    ref_rgba: &[u8],
    ref_w: u32,
    ref_h: u32,
) -> (f64, f64, f64, f64, f64) {
    fn frame_stats(rgba: &[u8], w: u32, h: u32) -> (f64, f64, f64, f64, f64) {
        let total = (w as usize) * (h as usize);
        if total == 0 || rgba.len() < total * 4 {
            return (0.5, 0.2, 0.25, 0.5, 0.5);
        }
        let step = (total / 8192).max(1);
        let mut sum_l = 0.0;
        let mut sum_l2 = 0.0;
        let mut sum_sat = 0.0;
        let mut sum_r = 0.0;
        let mut sum_b = 0.0;
        let mut cnt = 0.0;
        for i in (0..total).step_by(step) {
            let idx = i * 4;
            let r = rgba[idx] as f64 / 255.0;
            let g = rgba[idx + 1] as f64 / 255.0;
            let b = rgba[idx + 2] as f64 / 255.0;
            let l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            let sat = r.max(g).max(b) - r.min(g).min(b);
            sum_l += l;
            sum_l2 += l * l;
            sum_sat += sat;
            sum_r += r;
            sum_b += b;
            cnt += 1.0;
        }
        if cnt <= 0.0 {
            return (0.5, 0.2, 0.25, 0.5, 0.5);
        }
        let mean_l = sum_l / cnt;
        let var_l = (sum_l2 / cnt - mean_l * mean_l).max(1e-4);
        (
            mean_l,
            var_l.sqrt(),
            (sum_sat / cnt).max(0.02),
            sum_r / cnt,
            sum_b / cnt,
        )
    }

    let (src_l, src_std, src_sat, src_r, src_b) = frame_stats(src_rgba, src_w, src_h);
    let (ref_l, ref_std, ref_sat, ref_r, ref_b) = frame_stats(ref_rgba, ref_w, ref_h);

    let brightness = ((ref_l - src_l) * 0.85).clamp(-0.4, 0.4);
    let contrast = ((ref_std / src_std.max(0.05)).clamp(0.6, 1.6) * 100.0).round() / 100.0;
    let saturation = ((ref_sat / src_sat.max(0.05)).clamp(0.5, 1.8) * 100.0).round() / 100.0;
    let d_rb = ((ref_r - ref_b) - (src_r - src_b)) * 140.0;
    let temperature = d_rb.clamp(-80.0, 80.0).round();
    let tint = 0.0;

    (brightness, contrast, saturation, temperature, tint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_white_balance_detects_blue_and_warm_casts() {
        // 1. Neutral gray image -> (0.0, 0.0)
        let neutral = [128u8, 128, 128, 255].repeat(64 * 64);
        let (t_n, tint_n) = analyze_auto_white_balance_rgba(&neutral, 64, 64);
        assert!(t_n.abs() < 1.0, "neutral temp should be ~0, got {t_n}");
        assert!(
            tint_n.abs() < 1.0,
            "neutral tint should be ~0, got {tint_n}"
        );

        // 2. Cool/Bluish image (R=95, G=125, B=175) -> positive temperature (warm correction)
        let cool_img = [95u8, 125, 175, 255].repeat(64 * 64);
        let (t_cool, _) = analyze_auto_white_balance_rgba(&cool_img, 64, 64);
        assert!(
            t_cool > 25.0,
            "cool image should yield positive warming temperature, got {t_cool}"
        );

        // 3. Warm/Orange tungsten cast (R=185, G=130, B=85) -> negative temperature (cool correction)
        let warm_img = [185u8, 130, 85, 255].repeat(64 * 64);
        let (t_warm, _) = analyze_auto_white_balance_rgba(&warm_img, 64, 64);
        assert!(
            t_warm < -25.0,
            "warm image should yield negative cooling temperature, got {t_warm}"
        );

        // 4. Green fluorescent cast (R=110, G=165, B=110) -> positive magenta tint
        let green_img = [110u8, 165, 110, 255].repeat(64 * 64);
        let (_, tint_green) = analyze_auto_white_balance_rgba(&green_img, 64, 64);
        assert!(
            tint_green > 20.0,
            "greenish image should yield positive magenta tint, got {tint_green}"
        );
    }

    #[test]
    fn test_all_lut_presets_produce_distinct_valid_colors() {
        let shadow_sample = [0.25_f32, 0.30, 0.35];
        let highlight_sample = [0.75_f32, 0.68, 0.58];
        for preset in LutPreset::ALL {
            let out_s = preset.apply_rgb(shadow_sample[0], shadow_sample[1], shadow_sample[2]);
            let out_h = preset.apply_rgb(
                highlight_sample[0],
                highlight_sample[1],
                highlight_sample[2],
            );
            for c in out_s.into_iter().chain(out_h) {
                assert!((0.0..=1.0).contains(&c));
            }
            if *preset != LutPreset::None {
                let diff_s = (out_s[0] - shadow_sample[0]).abs()
                    + (out_s[1] - shadow_sample[1]).abs()
                    + (out_s[2] - shadow_sample[2]).abs();
                let diff_h = (out_h[0] - highlight_sample[0]).abs()
                    + (out_h[1] - highlight_sample[1]).abs()
                    + (out_h[2] - highlight_sample[2]).abs();
                let total_diff = diff_s + diff_h;
                assert!(
                    total_diff > 0.02,
                    "Preset {:?} must meaningfully transform pixel values (diff={})",
                    preset,
                    total_diff
                );
                assert!(
                    preset.ffmpeg_filter().is_some(),
                    "Preset {:?} must have an FFmpeg export filter",
                    preset
                );
            }
        }
    }

    #[test]
    fn test_brightness_contrast_presets_and_wb_presets() {
        for wb in WhiteBalancePreset::ALL {
            if *wb != WhiteBalancePreset::Auto {
                let (temp, tint) = wb
                    .temperature_tint()
                    .expect("Static WB preset must have temp/tint");
                assert!((-100.0..=100.0).contains(&temp));
                assert!((-100.0..=100.0).contains(&tint));
            }
        }

        for bc in BrightnessContrastPreset::ALL {
            let (b, c, g) = bc.params();
            assert!((-1.0..=1.0).contains(&b));
            assert!((0.1..=3.0).contains(&c));
            assert!((0.2..=3.0).contains(&g));
        }
    }

    #[test]
    fn test_camera_log_profiles_and_wb_eyedropper_and_cube_baking() {
        for profile in CameraLogProfile::ALL {
            let [rs, gs, bs] = profile.apply_rgb(0.25, 0.25, 0.25);
            let [rh, gh, bh] = profile.apply_rgb(0.75, 0.75, 0.75);
            for v in [rs, gs, bs, rh, gh, bh] {
                assert!((0.0..=1.0).contains(&v));
            }
            if *profile != CameraLogProfile::Rec709 {
                let diff = (rs - 0.25).abs() + (rh - 0.75).abs();
                assert!(
                    diff > 0.05,
                    "Log profile {:?} should expand contrast between shadows and highlights (diff={})",
                    profile,
                    diff
                );
            }
        }

        let (temp_n, tint_n) = compute_wb_from_neutral_rgb(128, 128, 128);
        assert!(temp_n.abs() < 0.1 && tint_n.abs() < 0.1);
        let (temp_blue, _) = compute_wb_from_neutral_rgb(90, 125, 175);
        assert!(
            temp_blue > 25.0,
            "blue pixel should require positive warming temp"
        );
        let (temp_warm, _) = compute_wb_from_neutral_rgb(185, 125, 85);
        assert!(
            temp_warm < -25.0,
            "warm pixel should require negative cooling temp"
        );

        let mut clip = crate::clip::Clip::new("m1", 0.0, 5.0);
        clip.name = "Noir Scene 01".to_string();
        clip.camera_log_profile = CameraLogProfile::SLog3;
        clip.temperature = 18.0;
        clip.contrast = 1.2;
        clip.lut_preset = LutPreset::Kodak2383;
        assert!(clip.needs_baked_cube_lut());

        let cube = bake_clip_grade_to_cube(&clip, 17);
        assert!(cube.contains("TITLE \"Noir Scene 01\""));
        assert!(cube.contains("LUT_3D_SIZE 17"));
        let data_lines = cube
            .lines()
            .filter(|l| {
                !l.starts_with('#')
                    && !l.starts_with("TITLE")
                    && !l.starts_with("LUT_3D_SIZE")
                    && !l.starts_with("DOMAIN_")
                    && !l.trim().is_empty()
            })
            .count();
        assert_eq!(data_lines, 17 * 17 * 17);
    }
}
