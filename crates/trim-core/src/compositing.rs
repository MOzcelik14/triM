//! Layer compositing models and mathematical blend modes shared across
//! CPU preview, GPU shaders, and FFmpeg export parity.

use serde::{Deserialize, Serialize};

/// Distinguishes standard media clips from media-less Adjustment Layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClipKind {
    #[default]
    Media,
    AdjustmentLayer,
}

/// Industry-standard layer blending modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    SoftLight,
    HardLight,
    Add,
    Subtract,
    Difference,
    Darken,
    Lighten,
}

impl BlendMode {
    pub const ALL: &'static [Self] = &[
        Self::Normal,
        Self::Multiply,
        Self::Screen,
        Self::Overlay,
        Self::SoftLight,
        Self::HardLight,
        Self::Add,
        Self::Subtract,
        Self::Difference,
        Self::Darken,
        Self::Lighten,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Multiply => "Çarpma (Multiply)",
            Self::Screen => "Ekran (Screen)",
            Self::Overlay => "Kaplama (Overlay)",
            Self::SoftLight => "Yumuşak Işık (Soft Light)",
            Self::HardLight => "Sert Işık (Hard Light)",
            Self::Add => "Ekleme (Add)",
            Self::Subtract => "Çıkarma (Subtract)",
            Self::Difference => "Fark (Difference)",
            Self::Darken => "Koyulaştır (Darken)",
            Self::Lighten => "Aydınlat (Lighten)",
        }
    }

    /// Integer index used by the GPU fragment shader (`u_blend_mode`).
    #[inline]
    pub fn shader_id(&self) -> i32 {
        match self {
            Self::Normal => 0,
            Self::Multiply => 1,
            Self::Screen => 2,
            Self::Overlay => 3,
            Self::SoftLight => 4,
            Self::HardLight => 5,
            Self::Add => 6,
            Self::Subtract => 7,
            Self::Difference => 8,
            Self::Darken => 9,
            Self::Lighten => 10,
        }
    }

    /// FFmpeg `blend` filter mode string (`all_mode=...`).
    #[inline]
    pub fn ffmpeg_mode(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Multiply => "multiply",
            Self::Screen => "screen",
            Self::Overlay => "overlay",
            Self::SoftLight => "softlight",
            Self::HardLight => "hardlight",
            Self::Add => "addition",
            Self::Subtract => "subtract",
            Self::Difference => "difference",
            Self::Darken => "darken",
            Self::Lighten => "lighten",
        }
    }
}

/// Evaluates a single normalized color channel `[0.0, 1.0]` under the given `BlendMode`.
/// Formulas match standard Photoshop / Premiere / FFmpeg `blend` definitions and GLSL shader.
#[inline]
pub fn blend_channel(base: f32, top: f32, mode: BlendMode) -> f32 {
    let b = base.clamp(0.0, 1.0);
    let t = top.clamp(0.0, 1.0);
    match mode {
        BlendMode::Normal => t,
        BlendMode::Multiply => b * t,
        BlendMode::Screen => 1.0 - (1.0 - b) * (1.0 - t),
        BlendMode::Overlay => {
            if b < 0.5 {
                2.0 * b * t
            } else {
                1.0 - 2.0 * (1.0 - b) * (1.0 - t)
            }
        }
        BlendMode::SoftLight => {
            // W3C / Pegtop-compatible smooth Soft Light formula
            if t <= 0.5 {
                b - (1.0 - 2.0 * t) * b * (1.0 - b)
            } else {
                let d = if b <= 0.25 {
                    ((16.0 * b - 12.0) * b + 4.0) * b
                } else {
                    b.sqrt()
                };
                b + (2.0 * t - 1.0) * (d - b)
            }
        }
        BlendMode::HardLight => {
            if t < 0.5 {
                2.0 * b * t
            } else {
                1.0 - 2.0 * (1.0 - b) * (1.0 - t)
            }
        }
        BlendMode::Add => (b + t).min(1.0),
        BlendMode::Subtract => (b - t).max(0.0),
        BlendMode::Difference => (b - t).abs(),
        BlendMode::Darken => b.min(t),
        BlendMode::Lighten => b.max(t),
    }
}

/// Blends two normalized RGB triplets `[0.0, 1.0]`.
#[inline]
pub fn blend_rgb(base: [f32; 3], top: [f32; 3], mode: BlendMode) -> [f32; 3] {
    [
        blend_channel(base[0], top[0], mode),
        blend_channel(base[1], top[1], mode),
        blend_channel(base[2], top[2], mode),
    ]
}

/// Composites `top` RGBA `[0.0, 1.0]` over opaque/translucent `base` RGBA using `top_opacity` and `mode`.
#[inline]
pub fn blend_rgba_over(
    base: [f32; 4],
    top: [f32; 4],
    top_opacity: f32,
    mode: BlendMode,
) -> [f32; 4] {
    let a = (top[3] * top_opacity).clamp(0.0, 1.0);
    if a <= 1e-5 {
        return base;
    }
    let blended = blend_rgb([base[0], base[1], base[2]], [top[0], top[1], top[2]], mode);
    let out_r = base[0] * (1.0 - a) + blended[0] * a;
    let out_g = base[1] * (1.0 - a) + blended[1] * a;
    let out_b = base[2] * (1.0 - a) + blended[2] * a;
    let out_a = (a + base[3] * (1.0 - a)).clamp(0.0, 1.0);
    [out_r, out_g, out_b, out_a]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blend_modes_mathematical_accuracy() {
        let b = 0.4_f32;
        let t = 0.6_f32;

        assert!((blend_channel(b, t, BlendMode::Normal) - 0.6).abs() < 1e-5);
        assert!((blend_channel(b, t, BlendMode::Multiply) - 0.24).abs() < 1e-5);
        assert!((blend_channel(b, t, BlendMode::Screen) - (1.0 - 0.6 * 0.4)).abs() < 1e-5);
        // Overlay: b < 0.5 -> 2 * 0.4 * 0.6 = 0.48
        assert!((blend_channel(b, t, BlendMode::Overlay) - 0.48).abs() < 1e-5);
        // HardLight: t >= 0.5 -> 1 - 2 * 0.6 * 0.4 = 0.52
        assert!((blend_channel(b, t, BlendMode::HardLight) - 0.52).abs() < 1e-5);
        assert!((blend_channel(0.7, 0.6, BlendMode::Add) - 1.0).abs() < 1e-5);
        assert!((blend_channel(0.3, 0.6, BlendMode::Subtract) - 0.0).abs() < 1e-5);
        assert!((blend_channel(b, t, BlendMode::Difference) - 0.2).abs() < 1e-5);
        assert!((blend_channel(b, t, BlendMode::Darken) - 0.4).abs() < 1e-5);
        assert!((blend_channel(b, t, BlendMode::Lighten) - 0.6).abs() < 1e-5);
        // SoftLight at t=0.5 should return base unchanged
        assert!((blend_channel(b, 0.5, BlendMode::SoftLight) - b).abs() < 1e-5);
    }

    #[test]
    fn test_blend_rgba_over_opacity() {
        let base = [0.5, 0.5, 0.5, 1.0];
        let top = [1.0, 0.0, 0.0, 1.0];
        // Multiply with 50% opacity: blended=[0.5,0,0], mixed 50% with base=[0.5,0.5,0.5] -> [0.5, 0.25, 0.25, 1.0]
        let out = blend_rgba_over(base, top, 0.5, BlendMode::Multiply);
        assert!((out[0] - 0.5).abs() < 1e-5);
        assert!((out[1] - 0.25).abs() < 1e-5);
        assert!((out[2] - 0.25).abs() < 1e-5);
        assert!((out[3] - 1.0).abs() < 1e-5);
    }
}
