//! Real-time professional color scopes computation (RGB Parade, Luma Waveform,
//! Vectorscope, Histogram, False Color, Zebra Exposure).

use serde::{Deserialize, Serialize};

pub const WAVEFORM_W: usize = 128;
pub const WAVEFORM_H: usize = 128;
pub const PARADE_CH_W: usize = 96;
pub const VECTORSCOPE_SIZE: usize = 128;

/// Enabled scope views and analyzer configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopeConfig {
    pub rgb_parade: bool,
    pub luma_waveform: bool,
    pub vectorscope: bool,
    pub histogram: bool,
    pub histogram_luma: bool,
    pub false_color: bool,
    pub zebra_exposure: bool,
    /// Zebra exposure threshold in normalized IRE (0.50 ..= 1.00, default 0.90 = 90 IRE).
    pub zebra_threshold: f32,
}

impl Default for ScopeConfig {
    fn default() -> Self {
        Self {
            rgb_parade: true,
            luma_waveform: true,
            vectorscope: true,
            histogram: true,
            histogram_luma: true,
            false_color: false,
            zebra_exposure: false,
            zebra_threshold: 0.90,
        }
    }
}

/// Computed scope buffers derived from actual RGBA image pixels.
#[derive(Debug, Clone)]
pub struct ScopeData {
    pub width: u32,
    pub height: u32,
    pub sample_count: u32,
    /// `[3][WAVEFORM_H][PARADE_CH_W]` flattened per channel (`R`, `G`, `B`), row 0 = bottom (0 IRE), row H-1 = top (100 IRE).
    pub parade_r: Vec<u16>,
    pub parade_g: Vec<u16>,
    pub parade_b: Vec<u16>,
    pub parade_peak: u16,
    /// `WAVEFORM_H * WAVEFORM_W`, row 0 = bottom (0 IRE), row H-1 = top (100 IRE).
    pub luma_waveform: Vec<u16>,
    pub luma_peak: u16,
    /// `VECTORSCOPE_SIZE * VECTORSCOPE_SIZE` hit counts + accumulated RGB tint.
    pub vectorscope_hits: Vec<u16>,
    pub vectorscope_rgb: Vec<[u32; 3]>,
    pub vectorscope_peak: u16,
    /// 256-bin histograms for R, G, B, and Rec.709 Luma.
    pub hist_r: [u32; 256],
    pub hist_g: [u32; 256],
    pub hist_b: [u32; 256],
    pub hist_luma: [u32; 256],
    pub hist_peak: u32,
    /// Average IRE (0.0 ..= 100.0) and peak IRE across sampled pixels.
    pub avg_ire: f32,
    pub peak_ire: f32,
    pub clipped_high_pct: f32,
    pub crushed_low_pct: f32,
}

impl Default for ScopeData {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            sample_count: 0,
            parade_r: vec![0; PARADE_CH_W * WAVEFORM_H],
            parade_g: vec![0; PARADE_CH_W * WAVEFORM_H],
            parade_b: vec![0; PARADE_CH_W * WAVEFORM_H],
            parade_peak: 1,
            luma_waveform: vec![0; WAVEFORM_W * WAVEFORM_H],
            luma_peak: 1,
            vectorscope_hits: vec![0; VECTORSCOPE_SIZE * VECTORSCOPE_SIZE],
            vectorscope_rgb: vec![[0; 3]; VECTORSCOPE_SIZE * VECTORSCOPE_SIZE],
            vectorscope_peak: 1,
            hist_r: [0; 256],
            hist_g: [0; 256],
            hist_b: [0; 256],
            hist_luma: [0; 256],
            hist_peak: 1,
            avg_ire: 0.0,
            peak_ire: 0.0,
            clipped_high_pct: 0.0,
            crushed_low_pct: 0.0,
        }
    }
}

/// Computes Rec.709 normalized luminance in `[0.0, 1.0]` from 8-bit RGB.
#[inline]
pub fn rec709_luma_norm(r: u8, g: u8, b: u8) -> f32 {
    (0.2126 * (r as f32) + 0.7152 * (g as f32) + 0.0722 * (b as f32)) / 255.0
}

/// Computes Rec.709 Cb and Cr in `[-0.5, 0.5]` from 8-bit RGB.
#[inline]
pub fn rec709_cb_cr(r: u8, g: u8, b: u8) -> (f32, f32) {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;
    let cb = -0.1146 * rf - 0.3854 * gf + 0.5000 * bf;
    let cr = 0.5000 * rf - 0.4542 * gf - 0.0458 * bf;
    (cb, cr)
}

/// Standard 75% Rec.709 color-bar target positions `(label, cb, cr)` for Vectorscope graticule.
pub fn vectorscope_targets_75() -> [(&'static str, f32, f32); 6] {
    let cols: [(&str, u8, u8, u8); 6] = [
        ("R", 191, 0, 0),
        ("Mg", 191, 0, 191),
        ("B", 0, 0, 191),
        ("Cy", 0, 191, 191),
        ("G", 0, 191, 0),
        ("Yl", 191, 191, 0),
    ];
    cols.map(|(lbl, r, g, b)| {
        let (cb, cr) = rec709_cb_cr(r, g, b);
        (lbl, cb, cr)
    })
}

/// Computes all enabled numeric scopes from an RGBA8 frame buffer.
/// Automatically subsamples large frames so computation stays under ~2ms in real time.
pub fn compute_scopes_rgba(rgba: &[u8], width: u32, height: u32) -> ScopeData {
    let mut data = ScopeData {
        width,
        height,
        ..Default::default()
    };

    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 || rgba.len() < w * h * 4 {
        return data;
    }

    // Target max ~320x180 samples for real-time speed
    let step_x = (w / 320).max(1);
    let step_y = (h / 180).max(1);

    let mut sum_luma = 0.0_f64;
    let mut peak_luma = 0.0_f32;
    let mut clipped_high = 0_u32;
    let mut crushed_low = 0_u32;
    let mut count = 0_u32;

    let mut y = 0;
    while y < h {
        let mut x = 0;
        let row_off = y * w * 4;
        let wf_x = (x * WAVEFORM_W) / w;
        let _ = wf_x;

        while x < w {
            let idx = row_off + x * 4;
            let r = rgba[idx];
            let g = rgba[idx + 1];
            let b = rgba[idx + 2];

            let luma = rec709_luma_norm(r, g, b).clamp(0.0, 1.0);
            let luma_u8 = (luma * 255.0).round().clamp(0.0, 255.0) as usize;

            sum_luma += luma as f64;
            if luma > peak_luma {
                peak_luma = luma;
            }
            if luma >= 0.98 {
                clipped_high += 1;
            } else if luma <= 0.02 {
                crushed_low += 1;
            }
            count += 1;

            // Histogram
            data.hist_r[r as usize] += 1;
            data.hist_g[g as usize] += 1;
            data.hist_b[b as usize] += 1;
            data.hist_luma[luma_u8] += 1;

            // RGB Parade
            let px = ((x * PARADE_CH_W) / w).min(PARADE_CH_W - 1);
            let pr_y = ((r as usize) * (WAVEFORM_H - 1)) / 255;
            let pg_y = ((g as usize) * (WAVEFORM_H - 1)) / 255;
            let pb_y = ((b as usize) * (WAVEFORM_H - 1)) / 255;
            let ir = pr_y * PARADE_CH_W + px;
            let ig = pg_y * PARADE_CH_W + px;
            let ib = pb_y * PARADE_CH_W + px;
            data.parade_r[ir] = data.parade_r[ir].saturating_add(1);
            data.parade_g[ig] = data.parade_g[ig].saturating_add(1);
            data.parade_b[ib] = data.parade_b[ib].saturating_add(1);

            // Luma Waveform
            let wx = ((x * WAVEFORM_W) / w).min(WAVEFORM_W - 1);
            let wy = ((luma * (WAVEFORM_H - 1) as f32).round() as usize).min(WAVEFORM_H - 1);
            let iw = wy * WAVEFORM_W + wx;
            data.luma_waveform[iw] = data.luma_waveform[iw].saturating_add(1);

            // Vectorscope (Cb on X [-0.5..0.5], Cr on Y [-0.5..0.5], +Cr up)
            let (cb, cr) = rec709_cb_cr(r, g, b);
            let vx =
                (((cb + 0.5).clamp(0.0, 1.0)) * (VECTORSCOPE_SIZE - 1) as f32).round() as usize;
            let vy =
                (((0.5 - cr).clamp(0.0, 1.0)) * (VECTORSCOPE_SIZE - 1) as f32).round() as usize;
            let iv = vy.min(VECTORSCOPE_SIZE - 1) * VECTORSCOPE_SIZE + vx.min(VECTORSCOPE_SIZE - 1);
            data.vectorscope_hits[iv] = data.vectorscope_hits[iv].saturating_add(1);
            data.vectorscope_rgb[iv][0] += r as u32;
            data.vectorscope_rgb[iv][1] += g as u32;
            data.vectorscope_rgb[iv][2] += b as u32;

            x += step_x;
        }
        y += step_y;
    }

    data.sample_count = count;
    if count > 0 {
        data.avg_ire = ((sum_luma / count as f64) * 100.0) as f32;
        data.peak_ire = peak_luma * 100.0;
        data.clipped_high_pct = (clipped_high as f32 / count as f32) * 100.0;
        data.crushed_low_pct = (crushed_low as f32 / count as f32) * 100.0;
    }

    data.parade_peak = data
        .parade_r
        .iter()
        .chain(data.parade_g.iter())
        .chain(data.parade_b.iter())
        .copied()
        .max()
        .unwrap_or(1)
        .max(1);
    data.luma_peak = data.luma_waveform.iter().copied().max().unwrap_or(1).max(1);
    data.vectorscope_peak = data
        .vectorscope_hits
        .iter()
        .copied()
        .max()
        .unwrap_or(1)
        .max(1);
    data.hist_peak = data
        .hist_r
        .iter()
        .chain(data.hist_g.iter())
        .chain(data.hist_b.iter())
        .chain(data.hist_luma.iter())
        .copied()
        .max()
        .unwrap_or(1)
        .max(1);

    data
}

/// Maps normalized IRE `[0.0, 1.0]` to an industry-standard False Color RGB triplet.
#[inline]
pub fn false_color_map_ire(ire: f32) -> [u8; 3] {
    let pct = ire * 100.0;
    if pct < 4.0 {
        [110, 0, 190] // Crushed blacks (Purple)
    } else if pct < 15.0 {
        [25, 65, 225] // Deep shadows (Blue)
    } else if pct < 35.0 {
        [0, 175, 215] // Shadows (Cyan)
    } else if pct < 42.0 {
        [85, 110, 90] // Dark midtones (Muted Olive)
    } else if pct < 52.0 {
        [40, 205, 65] // 18% Middle Grey target (Green)
    } else if pct < 60.0 {
        [135, 135, 135] // Neutral midtones
    } else if pct < 72.0 {
        [248, 135, 175] // Skin tone exposure target (Pink)
    } else if pct < 88.0 {
        [185, 185, 185] // Highlights
    } else if pct < 97.0 {
        [250, 220, 25] // Near clipping (Yellow)
    } else {
        [245, 30, 30] // Clipped whites (Red)
    }
}

/// Generates an RGBA8 False Color visualization image from source RGBA8 pixels.
pub fn compute_false_color_rgba(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let len = (width as usize) * (height as usize) * 4;
    let mut out = vec![0u8; len];
    let n = rgba.len().min(len) / 4;
    for i in 0..n {
        let base = i * 4;
        let ire = rec709_luma_norm(rgba[base], rgba[base + 1], rgba[base + 2]);
        let [fr, fg, fb] = false_color_map_ire(ire);
        out[base] = fr;
        out[base + 1] = fg;
        out[base + 2] = fb;
        out[base + 3] = 255;
    }
    out
}

/// Generates an RGBA8 Zebra Exposure visualization image from source RGBA8 pixels.
/// Pixels at or above `threshold` (`0.0..=1.0`, e.g. `0.90` = 90 IRE) show diagonal warning stripes.
pub fn compute_zebra_rgba(rgba: &[u8], width: u32, height: u32, threshold: f32) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let len = w * h * 4;
    let mut out = vec![0u8; len];
    if rgba.len() < len {
        return out;
    }
    let thresh = threshold.clamp(0.1, 1.0);
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) * 4;
            let r = rgba[idx];
            let g = rgba[idx + 1];
            let b = rgba[idx + 2];
            let ire = rec709_luma_norm(r, g, b);
            if ire >= thresh {
                let stripe = ((x + y) / 4).is_multiple_of(2);
                if stripe {
                    out[idx] = 255;
                    out[idx + 1] = 220;
                    out[idx + 2] = 30;
                } else {
                    out[idx] = 22;
                    out[idx + 1] = 22;
                    out[idx + 2] = 26;
                }
                out[idx + 3] = 255;
            } else {
                // Dimmed underlying image for crisp zebra readability
                out[idx] = (r as f32 * 0.72) as u8;
                out[idx + 1] = (g as f32 * 0.72) as u8;
                out[idx + 2] = (b as f32 * 0.72) as u8;
                out[idx + 3] = 255;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scopes_pure_white_and_black() {
        // 4x4 pure white frame
        let white = vec![255u8; 4 * 4 * 4];
        let s_w = compute_scopes_rgba(&white, 4, 4);
        assert_eq!(s_w.sample_count, 16);
        assert!((s_w.avg_ire - 100.0).abs() < 0.1);
        assert!((s_w.clipped_high_pct - 100.0).abs() < 0.1);
        assert_eq!(s_w.hist_luma[255], 16);
        assert_eq!(s_w.hist_r[255], 16);

        // 4x4 pure black frame
        let mut black = vec![0u8; 4 * 4 * 4];
        for px in black.as_chunks_mut::<4>().0 {
            px[3] = 255;
        }
        let s_b = compute_scopes_rgba(&black, 4, 4);
        assert!((s_b.avg_ire - 0.0).abs() < 0.1);
        assert!((s_b.crushed_low_pct - 100.0).abs() < 0.1);
        assert_eq!(s_b.hist_luma[0], 16);
    }

    #[test]
    fn test_scopes_vectorscope_pure_red_cb_cr() {
        // Pure red (255, 0, 0) -> Cb = -0.1146, Cr = +0.5 -> top-left-ish quadrant
        let mut red = vec![0u8; 4 * 4 * 4];
        for px in red.as_chunks_mut::<4>().0 {
            px[0] = 255;
            px[3] = 255;
        }
        let s = compute_scopes_rgba(&red, 4, 4);
        assert_eq!(s.hist_r[255], 16);
        assert_eq!(s.hist_g[0], 16);
        assert_eq!(s.hist_b[0], 16);
        // Peak vectorscope hit should be near y=0 (+Cr), x < center (-Cb)
        let (max_idx, &max_val) = s
            .vectorscope_hits
            .iter()
            .enumerate()
            .max_by_key(|&(_, v)| v)
            .unwrap();
        assert_eq!(max_val, 16);
        let vy = max_idx / VECTORSCOPE_SIZE;
        let vx = max_idx % VECTORSCOPE_SIZE;
        assert!(vy < 5, "Expected Cr=+0.5 near top row, got vy={vy}");
        assert!(vx < VECTORSCOPE_SIZE / 2, "Expected Cb<0, got vx={vx}");
    }

    #[test]
    fn test_false_color_and_zebra_exposure() {
        // Pixel 0: pure black (crushed), Pixel 1: mid grey ~50% (green), Pixel 2: pure white (red/clipped)
        let pixels = vec![
            0, 0, 0, 255, // crushed
            120, 120, 120, 255, // ~47 IRE -> green
            255, 255, 255, 255, // 100 IRE -> red + zebra
        ];
        let fc = compute_false_color_rgba(&pixels, 3, 1);
        assert_eq!(&fc[0..3], &[110, 0, 190]); // purple
        assert_eq!(&fc[4..7], &[40, 205, 65]); // green
        assert_eq!(&fc[8..11], &[245, 30, 30]); // red

        let zb = compute_zebra_rgba(&pixels, 3, 1, 0.90);
        // Pixel 2 at x=2,y=0 -> (2+0)/4 = 0 (even) -> yellow stripe
        assert_eq!(&zb[8..11], &[255, 220, 30]);
        // Pixel 1 below threshold -> dimmed original
        assert_eq!(zb[4], (120.0_f32 * 0.72) as u8);
    }
}
