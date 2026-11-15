//! Keyframe animation and interpolation models.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Interpolation {
    #[default]
    Linear,
    Hold,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier,
}

impl Interpolation {
    pub const ALL: &'static [Self] = &[
        Self::Linear,
        Self::EaseIn,
        Self::EaseOut,
        Self::EaseInOut,
        Self::CubicBezier,
        Self::Hold,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Linear => "Doğrusal (Linear)",
            Self::EaseIn => "Yumuşak Giriş (Ease In)",
            Self::EaseOut => "Yumuşak Çıkış (Ease Out)",
            Self::EaseInOut => "Yumuşak Geçiş (Ease In/Out)",
            Self::CubicBezier => "Özel Eğri (Cubic Bezier)",
            Self::Hold => "Sabit Tut (Hold / Step)",
        }
    }

    pub fn short_icon(&self) -> &'static str {
        match self {
            Self::Linear => "╱",
            Self::EaseIn => "◞",
            Self::EaseOut => "◜",
            Self::EaseInOut => "∫",
            Self::CubicBezier => "∿",
            Self::Hold => " stepped",
        }
    }

    /// Evaluates normalized curve factor `u` in `[0.0, 1.0]` for normalized time `t` in `[0.0, 1.0]`.
    pub fn evaluate_curve(&self, t: f64, bezier_handles: [f32; 4]) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Hold => 0.0,
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => {
                let inv = 1.0 - t;
                1.0 - inv * inv
            }
            Self::EaseInOut => 3.0 * t * t - 2.0 * t * t * t,
            Self::CubicBezier => solve_cubic_bezier(
                t,
                bezier_handles[0] as f64,
                bezier_handles[1] as f64,
                bezier_handles[2] as f64,
                bezier_handles[3] as f64,
            ),
        }
    }
}

impl std::fmt::Display for Interpolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Linear => write!(f, "linear"),
            Self::Hold => write!(f, "hold"),
            Self::EaseIn => write!(f, "ease_in"),
            Self::EaseOut => write!(f, "ease_out"),
            Self::EaseInOut => write!(f, "ease_in_out"),
            Self::CubicBezier => write!(f, "cubic_bezier"),
        }
    }
}

pub fn default_bezier_handles() -> [f32; 4] {
    [0.42, 0.0, 0.58, 1.0]
}

/// Solves 2D cubic Bezier curve `B(s) = (x(s), y(s))` with P0=(0,0), P1=(x1,y1), P2=(x2,y2), P3=(1,1)
/// for `x(s) = t` using Newton-Raphson + bisection fallback, returning `y(s)`.
pub fn solve_cubic_bezier(t: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let x1 = x1.clamp(0.0, 1.0);
    let x2 = x2.clamp(0.0, 1.0);

    let sample_x = |s: f64| -> f64 {
        let inv = 1.0 - s;
        3.0 * inv * inv * s * x1 + 3.0 * inv * s * s * x2 + s * s * s
    };
    let sample_dx = |s: f64| -> f64 {
        let inv = 1.0 - s;
        3.0 * inv * inv * x1 + 6.0 * inv * s * (x2 - x1) + 3.0 * s * s * (1.0 - x2)
    };
    let sample_y = |s: f64| -> f64 {
        let inv = 1.0 - s;
        3.0 * inv * inv * s * y1 + 3.0 * inv * s * s * y2 + s * s * s
    };

    let mut s = t;
    for _ in 0..8 {
        let x_err = sample_x(s) - t;
        if x_err.abs() < 1e-6 {
            return sample_y(s);
        }
        let dx = sample_dx(s);
        if dx.abs() < 1e-6 {
            break;
        }
        s = (s - x_err / dx).clamp(0.0, 1.0);
    }

    // Bisection fallback
    let mut lo = 0.0;
    let mut hi = 1.0;
    s = t;
    for _ in 0..12 {
        let x_val = sample_x(s);
        if (x_val - t).abs() < 1e-5 {
            break;
        }
        if t > x_val {
            lo = s;
        } else {
            hi = s;
        }
        s = 0.5 * (lo + hi);
    }
    sample_y(s)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    pub time: f64,
    pub value: f64,
    #[serde(default)]
    pub interpolation: Interpolation,
    #[serde(default = "default_bezier_handles")]
    pub bezier_handles: [f32; 4],
}

impl Keyframe {
    pub fn new(time: f64, value: f64, interpolation: Interpolation) -> Self {
        Self {
            time,
            value,
            interpolation,
            bezier_handles: default_bezier_handles(),
        }
    }

    pub fn with_bezier(time: f64, value: f64, handles: [f32; 4]) -> Self {
        Self {
            time,
            value,
            interpolation: Interpolation::CubicBezier,
            bezier_handles: handles,
        }
    }
}

/// Evaluates interpolated value from a list of keyframes at relative time `t`.
pub fn interpolate_keyframes(keyframes: &[Keyframe], time: f64, default_value: f64) -> f64 {
    if keyframes.is_empty() {
        return default_value;
    }
    if keyframes.len() == 1 {
        return keyframes[0].value;
    }

    let mut sorted = keyframes.to_vec();
    sorted.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if time <= sorted[0].time {
        return sorted[0].value;
    }
    if time >= sorted.last().unwrap().time {
        return sorted.last().unwrap().value;
    }

    for i in 0..sorted.len() - 1 {
        let k0 = &sorted[i];
        let k1 = &sorted[i + 1];
        if time >= k0.time && time <= k1.time {
            let span = k1.time - k0.time;
            if span.abs() < 1e-6 {
                return k0.value;
            }
            let t = (time - k0.time) / span;
            if k0.interpolation == Interpolation::Hold {
                return k0.value;
            }
            let u = k0.interpolation.evaluate_curve(t, k0.bezier_handles);
            return k0.value + (k1.value - k0.value) * u;
        }
    }

    sorted.last().unwrap().value
}

/// Integrates a keyframe curve from `0.0` to `t_end` (used for Speed Ramping / Time Remapping).
pub fn integrate_keyframes(keyframes: &[Keyframe], t_end: f64, default_value: f64) -> f64 {
    if t_end <= 0.0 {
        return 0.0;
    }
    if keyframes.is_empty() {
        return default_value * t_end;
    }
    if keyframes.len() == 1 {
        return keyframes[0].value.max(0.05) * t_end;
    }

    // Composite Simpson's / trapezoidal integration with adaptive subdivisions
    let steps = ((t_end * 60.0).ceil() as usize).clamp(16, 600);
    let dt = t_end / steps as f64;
    let mut area = 0.0;
    let mut prev_v = interpolate_keyframes(keyframes, 0.0, default_value).max(0.05);
    for step in 1..=steps {
        let curr_t = step as f64 * dt;
        let curr_v = interpolate_keyframes(keyframes, curr_t, default_value).max(0.05);
        area += 0.5 * (prev_v + curr_v) * dt;
        prev_v = curr_v;
    }
    area
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_and_single() {
        assert_eq!(interpolate_keyframes(&[], 2.0, 1.0), 1.0);
        let single = vec![Keyframe::new(1.0, 50.0, Interpolation::Linear)];
        assert_eq!(interpolate_keyframes(&single, 0.5, 0.0), 50.0);
        assert_eq!(interpolate_keyframes(&single, 2.0, 0.0), 50.0);
    }

    #[test]
    fn test_linear_interpolation() {
        let kfs = vec![
            Keyframe::new(0.0, 0.0, Interpolation::Linear),
            Keyframe::new(2.0, 100.0, Interpolation::Linear),
        ];
        assert_eq!(interpolate_keyframes(&kfs, -0.5, 0.0), 0.0);
        assert_eq!(interpolate_keyframes(&kfs, 2.5, 0.0), 100.0);
        assert!((interpolate_keyframes(&kfs, 1.0, 0.0) - 50.0).abs() < 1e-4);
        assert!((interpolate_keyframes(&kfs, 0.5, 0.0) - 25.0).abs() < 1e-4);
    }

    #[test]
    fn test_hold_interpolation() {
        let kfs = vec![
            Keyframe::new(0.0, 10.0, Interpolation::Hold),
            Keyframe::new(2.0, 20.0, Interpolation::Hold),
        ];
        assert_eq!(interpolate_keyframes(&kfs, 1.0, 0.0), 10.0);
        assert_eq!(interpolate_keyframes(&kfs, 1.99, 0.0), 10.0);
        assert_eq!(interpolate_keyframes(&kfs, 2.0, 0.0), 20.0);
    }

    #[test]
    fn test_ease_in_out_interpolation() {
        let kfs = vec![
            Keyframe::new(0.0, 0.0, Interpolation::EaseInOut),
            Keyframe::new(1.0, 100.0, Interpolation::EaseInOut),
        ];
        // At t=0.5, smoothstep(0.5) = 3*(0.25) - 2*(0.125) = 0.5 -> 50.0
        assert!((interpolate_keyframes(&kfs, 0.5, 0.0) - 50.0).abs() < 1e-4);
        // At t=0.2, smoothstep(0.2) = 0.104 -> 10.4
        let val_at_02 = interpolate_keyframes(&kfs, 0.2, 0.0);
        assert!((val_at_02 - 10.4).abs() < 1e-3);
    }

    #[test]
    fn test_ease_in_ease_out_and_cubic_bezier() {
        let ease_in = vec![
            Keyframe::new(0.0, 0.0, Interpolation::EaseIn),
            Keyframe::new(1.0, 100.0, Interpolation::EaseIn),
        ];
        assert!((interpolate_keyframes(&ease_in, 0.5, 0.0) - 25.0).abs() < 1e-4);

        let ease_out = vec![
            Keyframe::new(0.0, 0.0, Interpolation::EaseOut),
            Keyframe::new(1.0, 100.0, Interpolation::EaseOut),
        ];
        assert!((interpolate_keyframes(&ease_out, 0.5, 0.0) - 75.0).abs() < 1e-4);

        let bez = vec![
            Keyframe::with_bezier(0.0, 0.0, [0.25, 0.1, 0.25, 1.0]),
            Keyframe::new(1.0, 100.0, Interpolation::Linear),
        ];
        let v_mid = interpolate_keyframes(&bez, 0.5, 0.0);
        assert!(v_mid > 60.0 && v_mid < 90.0);
    }

    #[test]
    fn test_integrate_keyframes_speed_ramp() {
        // Linear speed ramp from 1.0x at t=0 to 3.0x at t=2.0 -> average speed is 2.0x -> integral is 4.0s
        let kfs = vec![
            Keyframe::new(0.0, 1.0, Interpolation::Linear),
            Keyframe::new(2.0, 3.0, Interpolation::Linear),
        ];
        let dist = integrate_keyframes(&kfs, 2.0, 1.0);
        assert!((dist - 4.0).abs() < 1e-2);
    }
}
