use serde::{Deserialize, Serialize};

/// Converts normalized RGB `[0.0, 1.0]` to `(hue_deg [0..360], sat [0..1], luma [0..1])`.
#[inline]
pub fn rgb_to_hsl_luma(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let rf = r.clamp(0.0, 1.0);
    let gf = g.clamp(0.0, 1.0);
    let bf = b.clamp(0.0, 1.0);
    let max_c = rf.max(gf).max(bf);
    let min_c = rf.min(gf).min(bf);
    let delta = max_c - min_c;
    let luma = 0.2126 * rf + 0.7152 * gf + 0.0722 * bf;
    let hue = if delta <= 1e-5 {
        0.0
    } else if (max_c - rf).abs() < 1e-5 {
        60.0 * (((gf - bf) / delta).rem_euclid(6.0))
    } else if (max_c - gf).abs() < 1e-5 {
        60.0 * (((bf - rf) / delta) + 2.0)
    } else {
        60.0 * (((rf - gf) / delta) + 4.0)
    };
    let sat = if max_c <= 1e-5 { 0.0 } else { delta / max_c };
    (hue, sat, luma)
}

/// Rotates hue (in degrees) and scales saturation/luma on a normalized RGB triplet.
#[inline]
pub fn shift_rgb_hue_sat_luma(
    r: f32,
    g: f32,
    b: f32,
    hue_shift_deg: f32,
    sat_mult: f32,
    luma_add: f32,
) -> [f32; 3] {
    let mut rf = r;
    let mut gf = g;
    let mut bf = b;
    if hue_shift_deg.abs() > 1e-3 {
        let rad = hue_shift_deg.to_radians();
        let cos_a = rad.cos();
        let sin_a = rad.sin();
        let s1 = (1.0 - cos_a) / 3.0;
        let s2 = (1.0 / 3.0_f32).sqrt() * sin_a;
        let m00 = cos_a + s1;
        let m01 = s1 - s2;
        let m02 = s1 + s2;
        let m10 = s1 + s2;
        let m11 = cos_a + s1;
        let m12 = s1 - s2;
        let m20 = s1 - s2;
        let m21 = s1 + s2;
        let m22 = cos_a + s1;
        let nr = rf * m00 + gf * m01 + bf * m02;
        let ng = rf * m10 + gf * m11 + bf * m12;
        let nb = rf * m20 + gf * m21 + bf * m22;
        rf = nr;
        gf = ng;
        bf = nb;
    }
    let luma = 0.2126 * rf + 0.7152 * gf + 0.0722 * bf;
    rf = luma + (rf - luma) * sat_mult + luma_add;
    gf = luma + (gf - luma) * sat_mult + luma_add;
    bf = luma + (bf - luma) * sat_mult + luma_add;
    [rf.clamp(0.0, 1.0), gf.clamp(0.0, 1.0), bf.clamp(0.0, 1.0)]
}

fn default_curve_points() -> Vec<[f32; 2]> {
    vec![[0.0, 0.0], [1.0, 1.0]]
}

/// Custom Spline Curves for Master Luma, Red, Green, and Blue channels (DaVinci Custom Curves).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomCurvesConfig {
    #[serde(default = "default_curve_points")]
    pub master: Vec<[f32; 2]>,
    #[serde(default = "default_curve_points")]
    pub red: Vec<[f32; 2]>,
    #[serde(default = "default_curve_points")]
    pub green: Vec<[f32; 2]>,
    #[serde(default = "default_curve_points")]
    pub blue: Vec<[f32; 2]>,
}

impl Default for CustomCurvesConfig {
    fn default() -> Self {
        Self {
            master: default_curve_points(),
            red: default_curve_points(),
            green: default_curve_points(),
            blue: default_curve_points(),
        }
    }
}

impl CustomCurvesConfig {
    pub fn is_identity_channel(pts: &[[f32; 2]]) -> bool {
        pts.len() == 2
            && (pts[0][0] - 0.0).abs() < 1e-4
            && (pts[0][1] - 0.0).abs() < 1e-4
            && (pts[1][0] - 1.0).abs() < 1e-4
            && (pts[1][1] - 1.0).abs() < 1e-4
    }

    pub fn is_identity(&self) -> bool {
        Self::is_identity_channel(&self.master)
            && Self::is_identity_channel(&self.red)
            && Self::is_identity_channel(&self.green)
            && Self::is_identity_channel(&self.blue)
    }

    /// Evaluates a sorted control-point curve at `x` in `[0.0, 1.0]` using smooth Hermite interpolation.
    #[inline]
    pub fn eval_curve(pts: &[[f32; 2]], x: f32) -> f32 {
        if pts.is_empty() {
            return x.clamp(0.0, 1.0);
        }
        let x = x.clamp(0.0, 1.0);
        if pts.len() == 1 {
            return pts[0][1].clamp(0.0, 1.0);
        }
        if x <= pts[0][0] {
            return pts[0][1].clamp(0.0, 1.0);
        }
        let last = pts.len() - 1;
        if x >= pts[last][0] {
            return pts[last][1].clamp(0.0, 1.0);
        }

        // Find segment i where pts[i][0] <= x <= pts[i+1][0]
        let mut idx = 0;
        for i in 0..last {
            if x >= pts[i][0] && x <= pts[i + 1][0] {
                idx = i;
                break;
            }
        }
        let p1 = pts[idx];
        let p2 = pts[idx + 1];
        let dx = (p2[0] - p1[0]).max(1e-5);
        let t = ((x - p1[0]) / dx).clamp(0.0, 1.0);

        if pts.len() == 2 {
            return (p1[1] + t * (p2[1] - p1[1])).clamp(0.0, 1.0);
        }

        // Secant slopes with Fritsch-Carlson clamping to prevent overshoot
        let sec = (p2[1] - p1[1]) / dx;
        let m1 = if idx > 0 {
            let p0 = pts[idx - 1];
            let s0 = (p1[1] - p0[1]) / (p1[0] - p0[0]).max(1e-5);
            if s0 * sec <= 0.0 {
                0.0
            } else {
                0.5 * (s0 + sec)
            }
        } else {
            sec
        };
        let m2 = if idx + 2 <= last {
            let p3 = pts[idx + 2];
            let s2 = (p3[1] - p2[1]) / (p3[0] - p2[0]).max(1e-5);
            if sec * s2 <= 0.0 {
                0.0
            } else {
                0.5 * (sec + s2)
            }
        } else {
            sec
        };

        let t2 = t * t;
        let t3 = t2 * t;
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        let y = h00 * p1[1] + h10 * dx * m1 + h01 * p2[1] + h11 * dx * m2;
        y.clamp(0.0, 1.0)
    }

    #[inline]
    pub fn apply_rgb(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        let mr = Self::eval_curve(&self.master, r);
        let mg = Self::eval_curve(&self.master, g);
        let mb = Self::eval_curve(&self.master, b);
        [
            Self::eval_curve(&self.red, mr),
            Self::eval_curve(&self.green, mg),
            Self::eval_curve(&self.blue, mb),
        ]
    }

    /// Generates an FFmpeg `curves=...` filter string if any curve is non-identity.
    pub fn ffmpeg_filter(&self) -> Option<String> {
        if self.is_identity() {
            return None;
        }
        let fmt_pts = |pts: &[[f32; 2]]| -> String {
            let mut sorted = pts.to_vec();
            sorted.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal));
            sorted.dedup_by(|a, b| (a[0] - b[0]).abs() < 0.01);
            sorted
                .iter()
                .map(|p| format!("{:.3}/{:.3}", p[0].clamp(0.0, 1.0), p[1].clamp(0.0, 1.0)))
                .collect::<Vec<_>>()
                .join(" ")
        };
        let mut parts = Vec::new();
        if !Self::is_identity_channel(&self.master) {
            parts.push(format!("master='{}'", fmt_pts(&self.master)));
        }
        if !Self::is_identity_channel(&self.red) {
            parts.push(format!("r='{}'", fmt_pts(&self.red)));
        }
        if !Self::is_identity_channel(&self.green) {
            parts.push(format!("g='{}'", fmt_pts(&self.green)));
        }
        if !Self::is_identity_channel(&self.blue) {
            parts.push(format!("b='{}'", fmt_pts(&self.blue)));
        }
        if parts.is_empty() {
            None
        } else {
            Some(format!("curves={}", parts.join(":")))
        }
    }
}

/// DaVinci-style 6-vector HSL Curves (`Hue vs Hue`, `Hue vs Sat`, `Hue vs Luma`, `Luma vs Sat`).
/// Color vectors correspond to `[Red 0°, Yellow 60°, Green 120°, Cyan 180°, Blue 240°, Magenta 300°]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HslCurvesConfig {
    /// Hue shift in degrees `[-60.0..=60.0]` for `[R, Y, G, C, B, M]`.
    #[serde(default)]
    pub hue_vs_hue: [f32; 6],
    /// Saturation delta `[-1.0..=1.0]` for `[R, Y, G, C, B, M]`.
    #[serde(default)]
    pub hue_vs_sat: [f32; 6],
    /// Luminance delta `[-0.5..=0.5]` for `[R, Y, G, C, B, M]`.
    #[serde(default)]
    pub hue_vs_luma: [f32; 6],
    /// Saturation delta `[-1.0..=1.0]` across 5 luma zones `[Shadows, DarkMid, Mid, BrightMid, Highlights]`.
    #[serde(default)]
    pub luma_vs_sat: [f32; 5],
}

impl Default for HslCurvesConfig {
    fn default() -> Self {
        Self {
            hue_vs_hue: [0.0; 6],
            hue_vs_sat: [0.0; 6],
            hue_vs_luma: [0.0; 6],
            luma_vs_sat: [0.0; 5],
        }
    }
}

impl HslCurvesConfig {
    pub fn is_identity(&self) -> bool {
        self.hue_vs_hue.iter().all(|v| v.abs() < 1e-4)
            && self.hue_vs_sat.iter().all(|v| v.abs() < 1e-4)
            && self.hue_vs_luma.iter().all(|v| v.abs() < 1e-4)
            && self.luma_vs_sat.iter().all(|v| v.abs() < 1e-4)
    }

    #[inline]
    fn sample_hue_ring(values: &[f32; 6], hue_deg: f32) -> f32 {
        let h = hue_deg.rem_euclid(360.0) / 60.0;
        let idx0 = (h.floor() as usize) % 6;
        let idx1 = (idx0 + 1) % 6;
        let frac = h - h.floor();
        let smooth = frac * frac * (3.0 - 2.0 * frac);
        values[idx0] * (1.0 - smooth) + values[idx1] * smooth
    }

    #[inline]
    fn sample_luma_5(values: &[f32; 5], luma: f32) -> f32 {
        let pos = (luma.clamp(0.0, 1.0)) * 4.0;
        let idx0 = (pos.floor() as usize).min(4);
        let idx1 = (idx0 + 1).min(4);
        let frac = pos - pos.floor();
        let smooth = frac * frac * (3.0 - 2.0 * frac);
        values[idx0] * (1.0 - smooth) + values[idx1] * smooth
    }

    #[inline]
    pub fn apply_rgb(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        if self.is_identity() {
            return [r, g, b];
        }
        let (hue, sat, luma) = rgb_to_hsl_luma(r, g, b);
        // Weight hue-based adjustments by chroma saturation so neutral greys aren't tinted
        let chroma_weight = (sat * 3.0).clamp(0.0, 1.0);
        let d_hue = Self::sample_hue_ring(&self.hue_vs_hue, hue) * chroma_weight;
        let d_sat_hue = Self::sample_hue_ring(&self.hue_vs_sat, hue) * chroma_weight;
        let d_luma = Self::sample_hue_ring(&self.hue_vs_luma, hue) * chroma_weight * 0.45;
        let d_sat_luma = Self::sample_luma_5(&self.luma_vs_sat, luma);

        let sat_mult = (1.0 + d_sat_hue + d_sat_luma).max(0.0);
        shift_rgb_hue_sat_luma(r, g, b, d_hue, sat_mult, d_luma)
    }
}

/// 3D HSL Qualifier for secondary selective color grading and matte visualization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HslQualifierConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Center hue in degrees `[0.0, 360.0]`.
    #[serde(default = "default_qual_hue_center")]
    pub hue_center: f32,
    /// Half-width in degrees `[5.0, 180.0]`.
    #[serde(default = "default_qual_hue_width")]
    pub hue_width: f32,
    /// Feather softness in degrees `[1.0, 60.0]`.
    #[serde(default = "default_qual_hue_softness")]
    pub hue_softness: f32,
    #[serde(default = "default_qual_sat_min")]
    pub sat_min: f32,
    #[serde(default = "default_qual_one")]
    pub sat_max: f32,
    #[serde(default = "default_qual_luma_min")]
    pub luma_min: f32,
    #[serde(default = "default_qual_one")]
    pub luma_max: f32,
    #[serde(default)]
    pub invert: bool,
    /// When true, displays non-qualified pixels as desaturated monochrome matte in the monitor.
    #[serde(default)]
    pub show_matte: bool,
    /// Secondary hue shift in degrees `[-180..=180]`.
    #[serde(default)]
    pub hue_shift: f32,
    /// Secondary saturation multiplier `[0.0..=2.5]`.
    #[serde(default = "default_qual_one")]
    pub sat_scale: f32,
    /// Secondary luminance offset `[-0.5..=0.5]`.
    #[serde(default)]
    pub luma_offset: f32,
}

fn default_qual_hue_center() -> f32 {
    25.0 // Skin-tone / warm orange default
}
fn default_qual_hue_width() -> f32 {
    35.0
}
fn default_qual_hue_softness() -> f32 {
    15.0
}
fn default_qual_sat_min() -> f32 {
    0.12
}
fn default_qual_luma_min() -> f32 {
    0.05
}
fn default_qual_one() -> f32 {
    1.0
}

impl Default for HslQualifierConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            hue_center: default_qual_hue_center(),
            hue_width: default_qual_hue_width(),
            hue_softness: default_qual_hue_softness(),
            sat_min: default_qual_sat_min(),
            sat_max: 1.0,
            luma_min: default_qual_luma_min(),
            luma_max: 1.0,
            invert: false,
            show_matte: false,
            hue_shift: 0.0,
            sat_scale: 1.0,
            luma_offset: 0.0,
        }
    }
}

impl HslQualifierConfig {
    /// Creates an active HSL Qualifier centered on a sampled 8-bit RGB pixel (Eyedropper pick).
    pub fn from_rgb_sample(rgb: [u8; 3]) -> Self {
        let (hue, sat, luma) = rgb_to_hsl_luma(
            rgb[0] as f32 / 255.0,
            rgb[1] as f32 / 255.0,
            rgb[2] as f32 / 255.0,
        );
        Self {
            enabled: true,
            hue_center: hue,
            hue_width: 28.0,
            hue_softness: 14.0,
            sat_min: (sat - 0.25).max(0.05),
            sat_max: (sat + 0.35).min(1.0),
            luma_min: (luma - 0.30).max(0.0),
            luma_max: (luma + 0.35).min(1.0),
            ..Default::default()
        }
    }

    /// Computes the `[0.0, 1.0]` selection matte for a normalized RGB pixel.
    #[inline]
    pub fn compute_matte(&self, r: f32, g: f32, b: f32) -> f32 {
        let (hue, sat, luma) = rgb_to_hsl_luma(r, g, b);
        // Shortest angular distance on 360° circle
        let mut dh = (hue - self.hue_center).abs() % 360.0;
        if dh > 180.0 {
            dh = 360.0 - dh;
        }
        let hw = self.hue_width.max(1.0);
        let hs = self.hue_softness.max(1.0);
        let m_hue = if dh <= hw {
            1.0
        } else if dh >= hw + hs {
            0.0
        } else {
            1.0 - (dh - hw) / hs
        };

        let soft_sl = 0.08_f32;
        let m_sat_lo = ((sat - (self.sat_min - soft_sl)) / soft_sl).clamp(0.0, 1.0);
        let m_sat_hi = (((self.sat_max + soft_sl) - sat) / soft_sl).clamp(0.0, 1.0);
        let m_lum_lo = ((luma - (self.luma_min - soft_sl)) / soft_sl).clamp(0.0, 1.0);
        let m_lum_hi = (((self.luma_max + soft_sl) - luma) / soft_sl).clamp(0.0, 1.0);

        let raw = (m_hue * m_sat_lo * m_sat_hi * m_lum_lo * m_lum_hi).clamp(0.0, 1.0);
        if self.invert {
            1.0 - raw
        } else {
            raw
        }
    }

    #[inline]
    pub fn apply_rgb(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        if !self.enabled {
            return [r, g, b];
        }
        let matte = self.compute_matte(r, g, b);
        let graded =
            shift_rgb_hue_sat_luma(r, g, b, self.hue_shift, self.sat_scale, self.luma_offset);
        if self.show_matte {
            // Highlight Matte mode: qualified pixels show graded color, unselected show dimmed greyscale
            let grey = (0.2126 * r + 0.7152 * g + 0.0722 * b) * 0.35;
            [
                grey * (1.0 - matte) + graded[0] * matte,
                grey * (1.0 - matte) + graded[1] * matte,
                grey * (1.0 - matte) + graded[2] * matte,
            ]
        } else {
            [
                r * (1.0 - matte) + graded[0] * matte,
                g * (1.0 - matte) + graded[1] * matte,
                b * (1.0 - matte) + graded[2] * matte,
            ]
        }
    }
}
