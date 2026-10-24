//! Smart scene change detection using FFmpeg.

use crate::ffmpeg::ffmpeg_cmd;
use std::path::Path;

/// Detects scene cut points (in source seconds) using FFmpeg's scene score filter.
pub fn detect_scene_changes(
    video_path: &Path,
    source_in: f64,
    source_out: f64,
    threshold: f32,
) -> Result<Vec<f64>, String> {
    if !video_path.exists() {
        return Err(format!("Dosya bulunamadı: {}", video_path.display()));
    }

    let thresh = threshold.clamp(0.1, 1.0);
    let mut cmd = ffmpeg_cmd();
    cmd.arg("-hide_banner")
        .arg("-nostats")
        .arg("-ss")
        .arg(format!("{:.3}", source_in.max(0.0)));

    if source_out > source_in {
        cmd.arg("-to").arg(format!("{:.3}", source_out));
    }

    cmd.arg("-i")
        .arg(video_path)
        .arg("-vf")
        .arg(format!(
            "select='gt(scene\\,{:.2})',metadata=print:file=-",
            thresh
        ))
        .arg("-f")
        .arg("null")
        .arg("-");

    log::info!(
        "Running scene detection on {}: {:?}",
        video_path.display(),
        cmd
    );

    let output = cmd
        .output()
        .map_err(|e| format!("FFmpeg sahne algılama çalıştırılamadı: {}", e))?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    // Some FFmpeg versions print metadata to stdout, others to stderr
    let combined = format!("{}\n{}", stdout_str, stderr_str);
    let cuts = parse_scene_cut_timestamps(&combined, source_in);

    log::info!(
        "Detected {} scene changes in {}",
        cuts.len(),
        video_path.display()
    );
    Ok(cuts)
}

/// Parses FFmpeg `pts_time:` lines into absolute source timestamps.
pub fn parse_scene_cut_timestamps(output: &str, source_in: f64) -> Vec<f64> {
    let mut cuts = Vec::new();
    for line in output.lines() {
        if let Some(pos) = line.find("pts_time:") {
            let after = &line[pos + "pts_time:".len()..];
            let num_str = after.split_whitespace().next().unwrap_or("");
            if let Ok(rel_time) = num_str.parse::<f64>() {
                if rel_time > 0.1 {
                    cuts.push(source_in + rel_time);
                }
            }
        }
    }
    cuts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    cuts.dedup_by(|a, b| (*a - *b).abs() < 0.25); // Avoid cuts within 250ms of each other
    cuts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scene_cut_timestamps() {
        let sample_output = r#"
[Parsed_metadata_1 @ 0x55a560288dc0] frame:0    pts:2500000 pts_time:2.5
lavfi.scene_score=0.450000
[Parsed_metadata_1 @ 0x55a560288dc0] frame:1    pts:4800000 pts_time:4.8
lavfi.scene_score=0.620000
"#;
        let cuts = parse_scene_cut_timestamps(sample_output, 10.0);
        assert_eq!(cuts.len(), 2);
        assert!((cuts[0] - 12.5).abs() < 0.001);
        assert!((cuts[1] - 14.8).abs() < 0.001);
    }
}
