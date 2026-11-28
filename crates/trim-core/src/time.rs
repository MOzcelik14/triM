//! Rational / frame-accurate time representation and SMPTE timecode utilities for triM.

use serde::{Deserialize, Serialize};

/// Exact rational frame rate (e.g. `30000/1001` for 29.97 fps, `24000/1001` for 23.976 fps).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameRate {
    pub num: u32,
    pub den: u32,
}

impl Default for FrameRate {
    fn default() -> Self {
        Self { num: 30, den: 1 }
    }
}

impl FrameRate {
    pub const fn new(num: u32, den: u32) -> Self {
        let num = if num == 0 { 30 } else { num };
        let den = if den == 0 { 1 } else { den };
        Self { num, den }
    }

    /// Maps an `f64` fps value to the nearest exact broadcast/cinema rational frame rate.
    pub fn from_fps(fps: f64) -> Self {
        if !fps.is_finite() || fps <= 0.0 {
            return Self::default();
        }
        if (fps - 23.976_023_976).abs() < 0.01 {
            Self {
                num: 24000,
                den: 1001,
            }
        } else if (fps - 29.970_029_97).abs() < 0.01 {
            Self {
                num: 30000,
                den: 1001,
            }
        } else if (fps - 59.940_059_94).abs() < 0.01 {
            Self {
                num: 60000,
                den: 1001,
            }
        } else {
            Self {
                num: (fps.round() as u32).max(1),
                den: 1,
            }
        }
    }

    pub fn to_fps(self) -> f64 {
        self.num as f64 / self.den.max(1) as f64
    }

    pub fn nominal_fps(self) -> u64 {
        (self.to_fps().round() as u64).max(1)
    }

    pub fn is_drop_frame_capable(self) -> bool {
        self.den == 1001 && (self.num == 30000 || self.num == 60000)
    }

    pub fn fcpxml_frame_duration(self) -> String {
        format!("{}/{}s", self.den, self.num)
    }

    pub fn fcpxml_time_for_frames(self, frames: u64) -> String {
        if frames == 0 {
            "0s".to_string()
        } else {
            let ticks = frames.saturating_mul(self.den as u64);
            format!("{ticks}/{}s", self.num)
        }
    }
}

/// Rational frame-exact timestamp in `frames` at a `FrameRate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RationalTime {
    pub frames: i64,
    pub rate: FrameRate,
}

impl RationalTime {
    pub const fn from_frames(frames: i64, rate: FrameRate) -> Self {
        Self { frames, rate }
    }

    pub fn from_seconds(seconds: f64, rate: FrameRate) -> Self {
        let safe = if seconds.is_finite() { seconds } else { 0.0 };
        let frames = (safe * rate.to_fps()).round() as i64;
        Self { frames, rate }
    }

    pub fn to_seconds(self) -> f64 {
        (self.frames as f64 * self.rate.den as f64) / self.rate.num.max(1) as f64
    }

    pub fn snap_seconds(seconds: f64, rate: FrameRate) -> f64 {
        Self::from_seconds(seconds, rate).to_seconds()
    }
}

/// Formats seconds to non-drop-frame timecode string `HH:MM:SS:FF` at specified frame rate.
pub fn seconds_to_timecode(seconds: f64, fps: f64) -> String {
    let rate = FrameRate::from_fps(fps);
    let total_frames = (seconds.max(0.0) * rate.to_fps()).round().max(0.0) as u64;
    let fps_int = rate.nominal_fps();

    let ff = total_frames % fps_int;
    let total_seconds = total_frames / fps_int;
    let ss = total_seconds % 60;
    let mm = (total_seconds / 60) % 60;
    let hh = total_seconds / 3600;

    format!("{:02}:{:02}:{:02}:{:02}", hh, mm, ss, ff)
}

/// Formats seconds to SMPTE Drop-Frame (`HH:MM:SS;FF`) for 29.97/59.94, or NDF (`HH:MM:SS:FF`) otherwise.
pub fn seconds_to_drop_frame_timecode(seconds: f64, fps: f64) -> String {
    let rate = FrameRate::from_fps(fps);
    if !rate.is_drop_frame_capable() {
        return seconds_to_timecode(seconds, fps);
    }

    let drop_frames: u64 = if rate.num == 60000 { 4 } else { 2 };
    let fps_int = rate.nominal_fps(); // 30 or 60
    let frame_num = (seconds.max(0.0) * rate.to_fps()).round().max(0.0) as u64;

    let frames_per_10_mins = fps_int * 600 - drop_frames * 9;
    let frames_per_min = fps_int * 60 - drop_frames;

    let d = frame_num / frames_per_10_mins;
    let m = frame_num % frames_per_10_mins;
    let adjusted = if m >= drop_frames {
        let extra_mins = (m - drop_frames) / frames_per_min;
        frame_num + drop_frames * 9 * d + drop_frames * extra_mins
    } else {
        frame_num + drop_frames * 9 * d
    };

    let ff = adjusted % fps_int;
    let ss = (adjusted / fps_int) % 60;
    let mm = (adjusted / (fps_int * 60)) % 60;
    let hh = adjusted / (fps_int * 3600);

    format!("{:02}:{:02}:{:02};{:02}", hh, mm, ss, ff)
}

/// Parses timecode string (supports `:` NDF and `;` Drop-Frame, e.g. `HH:MM:SS:FF` or `HH:MM:SS;FF`).
pub fn timecode_to_seconds(tc: &str, fps: f64) -> Option<f64> {
    let rate = FrameRate::from_fps(fps);
    let is_df = tc.contains(';') && rate.is_drop_frame_capable();
    let fps_int = rate.nominal_fps();
    let normalized = tc.trim().replace(';', ":");
    let parts: Vec<&str> = normalized.split(':').collect();
    if parts.is_empty() {
        return None;
    }

    match parts.len() {
        4 => {
            let hh = parts[0].parse::<u64>().ok()?;
            let mm = parts[1].parse::<u64>().ok()?;
            let ss = parts[2].parse::<u64>().ok()?;
            let ff = parts[3].parse::<u64>().ok()?;
            let total_mins = hh.checked_mul(60)?.checked_add(mm)?;
            let raw_frames = total_mins
                .checked_mul(60)?
                .checked_add(ss)?
                .checked_mul(fps_int)?
                .checked_add(ff)?;
            let frame_num = if is_df {
                let drop_frames: u64 = if rate.num == 60000 { 4 } else { 2 };
                let dropped = drop_frames * (total_mins - total_mins / 10);
                raw_frames.saturating_sub(dropped)
            } else {
                raw_frames
            };
            Some(frame_num as f64 / rate.to_fps())
        }
        3 => {
            let mm = parts[0].parse::<u64>().ok()?;
            let ss = parts[1].parse::<u64>().ok()?;
            let ff = parts[2].parse::<u64>().ok()?;
            let secs = mm.checked_mul(60)?.checked_add(ss)?;
            let total_frames = secs.checked_mul(fps_int)?.checked_add(ff)?;
            Some(total_frames as f64 / rate.to_fps())
        }
        2 => {
            let ss = parts[0].parse::<u64>().ok()?;
            let ff = parts[1].parse::<u64>().ok()?;
            let total_frames = ss.checked_mul(fps_int)?.checked_add(ff)?;
            Some(total_frames as f64 / rate.to_fps())
        }
        1 => parts[0].parse::<f64>().ok().filter(|v| v.is_finite()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seconds_to_timecode() {
        assert_eq!(seconds_to_timecode(0.0, 30.0), "00:00:00:00");
        assert_eq!(seconds_to_timecode(1.0, 30.0), "00:00:01:00");
        assert_eq!(seconds_to_timecode(65.5, 30.0), "00:01:05:15");
        assert_eq!(seconds_to_timecode(3661.1, 30.0), "01:01:01:03");
    }

    #[test]
    fn test_timecode_to_seconds() {
        assert_eq!(timecode_to_seconds("00:00:00:00", 30.0), Some(0.0));
        let s = timecode_to_seconds("00:01:05:15", 30.0).unwrap();
        assert!((s - 65.5).abs() < 1e-4);
    }

    #[test]
    fn test_fractional_fps_timecode_roundtrip_no_drift() {
        for fps in [23.976, 29.97, 59.94] {
            for seconds in [0.0, 1.5, 65.5, 100.0, 3600.0] {
                let tc = seconds_to_timecode(seconds, fps);
                let recovered = timecode_to_seconds(&tc, fps).expect("Timecode must parse");
                let diff = (recovered - seconds).abs();
                let frame_dur = 1.0 / fps;
                assert!(
                    diff < frame_dur * 0.5,
                    "Drift at {seconds}s ({fps} fps): timecode={tc}, recovered={recovered}s (diff={diff}s > half frame {frame_dur})"
                );
            }
        }
    }

    #[test]
    fn test_rational_time_and_drop_frame_roundtrip_across_broadcast_rates() {
        for fps in [23.976, 24.0, 25.0, 29.97, 30.0, 50.0, 59.94, 60.0] {
            let rate = FrameRate::from_fps(fps);
            for frames in [0_i64, 1, 29, 30, 1798, 1800, 17982, 107892] {
                let rt = RationalTime::from_frames(frames, rate);
                let secs = rt.to_seconds();
                let back = RationalTime::from_seconds(secs, rate);
                assert_eq!(back.frames, frames, "frame drift at {fps} fps");
            }
        }

        // SMPTE 29.97 Drop-Frame: 00:00:59;29 -> next frame is 00:01:00;02
        let rate_2997 = FrameRate::from_fps(29.97);
        assert_eq!(
            seconds_to_drop_frame_timecode(
                RationalTime::from_frames(1799, rate_2997).to_seconds(),
                29.97
            ),
            "00:00:59;29"
        );
        assert_eq!(
            seconds_to_drop_frame_timecode(
                RationalTime::from_frames(1800, rate_2997).to_seconds(),
                29.97
            ),
            "00:01:00;02"
        );
        // 10th minute does NOT drop: 00:10:00;00
        assert_eq!(
            seconds_to_drop_frame_timecode(
                RationalTime::from_frames(17982, rate_2997).to_seconds(),
                29.97
            ),
            "00:10:00;00"
        );
        // Parse back drop-frame timecode
        let parsed = timecode_to_seconds("00:01:00;02", 29.97).unwrap();
        assert_eq!(RationalTime::from_seconds(parsed, rate_2997).frames, 1800);
    }
}
