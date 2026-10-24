//! Keyframe animation and interpolation models.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum Interpolation {
    #[default]
    Linear,
    Hold,
    EaseInOut,
}

impl std::fmt::Display for Interpolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Linear => write!(f, "linear"),
            Self::Hold => write!(f, "hold"),
            Self::EaseInOut => write!(f, "ease_in_out"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    pub time: f64,
    pub value: f64,
    #[serde(default)]
    pub interpolation: Interpolation,
}

impl Keyframe {
    pub fn new(time: f64, value: f64, interpolation: Interpolation) -> Self {
        Self {
            time,
            value,
            interpolation,
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

            return match k0.interpolation {
                Interpolation::Hold => k0.value,
                Interpolation::EaseInOut => {
                    // Smoothstep Hermite polynomial: 3*t^2 - 2*t^3
                    let u = 3.0 * t * t - 2.0 * t * t * t;
                    k0.value + (k1.value - k0.value) * u
                }
                Interpolation::Linear => k0.value + (k1.value - k0.value) * t,
            };
        }
    }

    sorted.last().unwrap().value
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
}
