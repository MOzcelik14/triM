//! Audio silence detection and speech interval calculation using FFmpeg.

use crate::ffmpeg::ffmpeg_cmd;
use std::path::Path;

/// Detects silent regions in an audio/video file within [source_in, source_out]
/// using FFmpeg's `silencedetect` audio filter.
/// Returns a list of `(silence_start, silence_end)` in source seconds.
pub fn detect_silence_regions(
    media_path: &Path,
    source_in: f64,
    source_out: f64,
    noise_db: f32,
    min_duration: f64,
) -> Result<Vec<(f64, f64)>, String> {
    if !media_path.exists() {
        return Err(format!("Dosya bulunamadı: {}", media_path.display()));
    }

    let noise = noise_db.clamp(-90.0, -10.0);
    let min_dur = min_duration.clamp(0.1, 10.0);

    let mut cmd = ffmpeg_cmd();
    cmd.arg("-hide_banner")
        .arg("-nostats")
        .arg("-ss")
        .arg(format!("{:.3}", source_in.max(0.0)));

    if source_out > source_in {
        cmd.arg("-to").arg(format!("{:.3}", source_out));
    }

    cmd.arg("-i")
        .arg(media_path)
        .arg("-vn")
        .arg("-af")
        .arg(format!("silencedetect=noise={:.1}dB:d={:.2}", noise, min_dur))
        .arg("-f")
        .arg("null")
        .arg("-");

    log::info!("Running silence detection on {}: {:?}", media_path.display(), cmd);

    let output = cmd
        .output()
        .map_err(|e| format!("FFmpeg sessizlik algılama çalıştırılamadı: {}", e))?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout_str, stderr_str);

    let silences = parse_silence_timestamps(&combined, source_in);
    log::info!(
        "Detected {} silent regions in {}",
        silences.len(),
        media_path.display()
    );
    Ok(silences)
}

/// Parses FFmpeg `silence_start:` and `silence_end:` messages from stderr/stdout into
/// absolute source intervals `(start, end)`.
pub fn parse_silence_timestamps(output: &str, source_in: f64) -> Vec<(f64, f64)> {
    let mut silences = Vec::new();
    let mut current_start: Option<f64> = None;

    for line in output.lines() {
        if let Some(pos) = line.find("silence_start:") {
            let after = &line[pos + "silence_start:".len()..];
            let num_str = after.split_whitespace().next().unwrap_or("");
            if let Ok(val) = num_str.parse::<f64>() {
                current_start = Some(source_in + val);
            }
        } else if let Some(pos) = line.find("silence_end:") {
            let after = &line[pos + "silence_end:".len()..];
            let num_str = after.split_whitespace().next().unwrap_or("");
            if let Ok(end_val) = num_str.parse::<f64>() {
                let abs_end = source_in + end_val;
                if let Some(start) = current_start.take() {
                    if abs_end > start {
                        silences.push((start, abs_end));
                    }
                }
            }
        }
    }

    silences.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    silences
}

/// Subtracts detected silence regions from the clip source range `[source_in, source_out]`
/// and expands speech boundaries slightly with `padding_seconds` (cushion)
/// to avoid clipping word start and end consonants.
/// Returns speech intervals `(speech_in, speech_out)`.
pub fn calculate_speech_intervals(
    source_in: f64,
    source_out: f64,
    silences: &[(f64, f64)],
    padding_seconds: f64,
) -> Vec<(f64, f64)> {
    if source_out <= source_in {
        return Vec::new();
    }

    if silences.is_empty() {
        return vec![(source_in, source_out)];
    }

    let mut speech_intervals = Vec::new();
    let mut current_speech_start = source_in;

    for &(sil_start, sil_end) in silences {
        let clamped_sil_start = sil_start.clamp(source_in, source_out);
        let clamped_sil_end = sil_end.clamp(source_in, source_out);

        // Add padding around speech: keep extra audio before silence starts
        let speech_end = (clamped_sil_start + padding_seconds).min(source_out);
        let next_speech_start = (clamped_sil_end - padding_seconds).max(source_in);

        if speech_end > current_speech_start + 0.1 {
            speech_intervals.push((current_speech_start, speech_end));
        }

        current_speech_start = next_speech_start;
    }

    if current_speech_start + 0.1 < source_out {
        speech_intervals.push((current_speech_start, source_out));
    }

    // Merge any overlapping speech intervals created by padding
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for interval in speech_intervals {
        if let Some(last) = merged.last_mut() {
            if interval.0 <= last.1 {
                last.1 = last.1.max(interval.1);
                continue;
            }
        }
        merged.push(interval);
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_silence_timestamps() {
        let sample = r#"
[silencedetect @ 0x55dc98a280c0] silence_start: 3.456
[silencedetect @ 0x55dc98a280c0] silence_end: 5.678 | silence_duration: 2.222
[silencedetect @ 0x55dc98a280c0] silence_start: 8.100
[silencedetect @ 0x55dc98a280c0] silence_end: 10.200 | silence_duration: 2.100
"#;
        let silences = parse_silence_timestamps(sample, 0.0);
        assert_eq!(silences.len(), 2);
        assert!((silences[0].0 - 3.456).abs() < 0.001);
        assert!((silences[0].1 - 5.678).abs() < 0.001);
        assert!((silences[1].0 - 8.100).abs() < 0.001);
        assert!((silences[1].1 - 10.200).abs() < 0.001);
    }

    #[test]
    fn test_calculate_speech_intervals() {
        let silences = vec![(3.0, 5.0), (8.0, 10.0)];
        let speech = calculate_speech_intervals(0.0, 12.0, &silences, 0.0);
        assert_eq!(speech.len(), 3);
        assert!((speech[0].0 - 0.0).abs() < 0.001);
        assert!((speech[0].1 - 3.0).abs() < 0.001);
        assert!((speech[1].0 - 5.0).abs() < 0.001);
        assert!((speech[1].1 - 8.0).abs() < 0.001);
        assert!((speech[2].0 - 10.0).abs() < 0.001);
        assert!((speech[2].1 - 12.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_speech_intervals_with_padding() {
        let silences = vec![(3.0, 5.0)];
        let speech = calculate_speech_intervals(0.0, 8.0, &silences, 0.1);
        assert_eq!(speech.len(), 2);
        assert!((speech[0].0 - 0.0).abs() < 0.001);
        assert!((speech[0].1 - 3.1).abs() < 0.001);
        assert!((speech[1].0 - 4.9).abs() < 0.001);
        assert!((speech[1].1 - 8.0).abs() < 0.001);
    }
}
