//! Time and timecode utilities for triM.

/// Formats seconds to timecode string `HH:MM:SS:FF` at specified frame rate.
pub fn seconds_to_timecode(seconds: f64, fps: f64) -> String {
    let fps = if fps <= 0.0 { 30.0 } else { fps };
    let total_frames = (seconds * fps).round().max(0.0) as u64;
    let fps_int = fps.round() as u64;
    let fps_int = if fps_int == 0 { 30 } else { fps_int };

    let ff = total_frames % fps_int;
    let total_seconds = total_frames / fps_int;
    let ss = total_seconds % 60;
    let mm = (total_seconds / 60) % 60;
    let hh = total_seconds / 3600;

    format!("{:02}:{:02}:{:02}:{:02}", hh, mm, ss, ff)
}

/// Parses timecode string (e.g. `HH:MM:SS:FF` or `MM:SS:FF` or `SS:FF`) back into seconds.
pub fn timecode_to_seconds(tc: &str, fps: f64) -> Option<f64> {
    let fps = if fps <= 0.0 { 30.0 } else { fps };
    let parts: Vec<&str> = tc.trim().split(':').collect();
    if parts.is_empty() {
        return None;
    }

    match parts.len() {
        4 => {
            let hh = parts[0].parse::<u64>().ok()?;
            let mm = parts[1].parse::<u64>().ok()?;
            let ss = parts[2].parse::<u64>().ok()?;
            let ff = parts[3].parse::<u64>().ok()?;
            let total_frames = ((hh * 3600 + mm * 60 + ss) as f64 * fps).round() as u64 + ff;
            Some(total_frames as f64 / fps)
        }
        3 => {
            let mm = parts[0].parse::<u64>().ok()?;
            let ss = parts[1].parse::<u64>().ok()?;
            let ff = parts[2].parse::<u64>().ok()?;
            let total_frames = ((mm * 60 + ss) as f64 * fps).round() as u64 + ff;
            Some(total_frames as f64 / fps)
        }
        2 => {
            let ss = parts[0].parse::<u64>().ok()?;
            let ff = parts[1].parse::<u64>().ok()?;
            let total_frames = (ss as f64 * fps).round() as u64 + ff;
            Some(total_frames as f64 / fps)
        }
        1 => parts[0].parse::<f64>().ok(),
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
}
