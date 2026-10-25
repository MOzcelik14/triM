//! Media metadata extraction using system `ffprobe`.

use std::path::Path;
use thiserror::Error;
use trim_core::media::{MediaItem, MediaType};

#[derive(Error, Debug)]
pub enum ProbeError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("ffprobe execution failed: {0}")]
    ProcessFailed(String),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
}

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "tiff", "gif"];
const AUDIO_EXTENSIONS: &[&str] = &["mp3", "wav", "aac", "flac", "ogg", "m4a"];

pub struct FFprobeAnalyzer;

impl FFprobeAnalyzer {
    pub fn analyze<P: AsRef<Path>>(file_path: P) -> Result<MediaItem, ProbeError> {
        let path = file_path.as_ref();
        if !path.is_file() {
            return Err(ProbeError::FileNotFound(path.to_string_lossy().to_string()));
        }

        let output = crate::ffmpeg::ffprobe_cmd()
            .args([
                "-v",
                "error",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
            ])
            .arg(path)
            .output()
            .map_err(|e| ProbeError::ProcessFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let err_msg = if stderr.trim().is_empty() {
                format!("ffprobe exited with status {}", output.status)
            } else {
                stderr.to_string()
            };
            return Err(ProbeError::ProcessFailed(err_msg));
        }

        let json_val: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        Self::parse_probe_json(&json_val, path)
    }

    /// Internal parser separating JSON processing from process execution for thorough unit testing.
    pub fn parse_probe_json(
        json_val: &serde_json::Value,
        path: &Path,
    ) -> Result<MediaItem, ProbeError> {
        let streams = json_val
            .get("streams")
            .and_then(|s| s.as_array())
            .cloned()
            .unwrap_or_default();
        let format = json_val.get("format");

        let mut video_stream = None;
        let mut audio_stream = None;

        for s in &streams {
            let codec_type = s.get("codec_type").and_then(|c| c.as_str()).unwrap_or("");
            if codec_type == "video" && video_stream.is_none() {
                let is_attached_pic = s
                    .get("disposition")
                    .and_then(|d| d.get("attached_pic"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0)
                    == 1;
                // Exclude QuickTime timecode or data streams masquerading as video
                let codec_name = s.get("codec_name").and_then(|c| c.as_str()).unwrap_or("");
                let is_data = codec_name == "bin_data" || codec_name == "tmcd";
                if !is_attached_pic && !is_data {
                    video_stream = Some(s);
                }
            } else if codec_type == "audio" && audio_stream.is_none() {
                audio_stream = Some(s);
            }
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let mut width = 1920;
        let mut height = 1080;
        let mut fps = 30.0;
        let mut video_codec = String::new();

        if let Some(vs) = video_stream {
            width = vs.get("width").and_then(|w| w.as_u64()).unwrap_or(1920) as u32;
            height = vs.get("height").and_then(|h| h.as_u64()).unwrap_or(1080) as u32;
            video_codec = vs
                .get("codec_name")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();

            let r_fps_str = vs.get("r_frame_rate").and_then(|r| r.as_str());
            let avg_fps_str = vs.get("avg_frame_rate").and_then(|r| r.as_str());

            fps = if let Some(r_str) = r_fps_str {
                let parsed = parse_fps(r_str);
                if r_str == "0/0" || parsed <= 0.0 {
                    avg_fps_str.map(parse_fps).unwrap_or(parsed)
                } else {
                    parsed
                }
            } else if let Some(avg_str) = avg_fps_str {
                parse_fps(avg_str)
            } else {
                30.0
            };
        }

        let duration = extract_duration(format, video_stream, audio_stream, fps, &ext);

        let file_size_bytes = format
            .and_then(|f| f.get("size"))
            .and_then(|s| s.as_str())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        let media_type = if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            MediaType::Image
        } else if video_stream.is_some()
            || ext == "mov"
            || ext == "mp4"
            || ext == "mkv"
            || ext == "m4v"
        {
            MediaType::Video
        } else if audio_stream.is_some() || AUDIO_EXTENSIONS.contains(&ext.as_str()) {
            MediaType::Audio
        } else {
            MediaType::Unknown
        };

        let mut audio_codec = String::new();
        let mut audio_channels = 0;
        let mut audio_sample_rate = 0;

        if let Some(as_) = audio_stream {
            audio_codec = as_
                .get("codec_name")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            audio_channels = as_.get("channels").and_then(|c| c.as_u64()).unwrap_or(2) as u32;
            audio_sample_rate = as_
                .get("sample_rate")
                .and_then(|s| s.as_str())
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(48000);
        }

        let mut item = MediaItem::new(path);
        item.media_type = media_type;
        item.duration = duration;
        item.width = width;
        item.height = height;
        item.fps = fps;
        item.video_codec = video_codec;
        item.audio_codec = audio_codec;
        item.audio_channels = audio_channels;
        item.audio_sample_rate = audio_sample_rate;
        item.file_size_bytes = file_size_bytes;

        Ok(item)
    }
}

pub fn parse_time_string(val: &str) -> Option<f64> {
    let s = val.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("n/a") {
        return None;
    }
    if s.contains(':') {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 3 {
            let h = parts[0].parse::<f64>().ok()?;
            let m = parts[1].parse::<f64>().ok()?;
            let sec = parts[2].parse::<f64>().ok()?;
            let total = h * 3600.0 + m * 60.0 + sec;
            if total >= 0.0 {
                return Some(total);
            }
        } else if parts.len() == 2 {
            let m = parts[0].parse::<f64>().ok()?;
            let sec = parts[1].parse::<f64>().ok()?;
            let total = m * 60.0 + sec;
            if total >= 0.0 {
                return Some(total);
            }
        }
    }
    if let Ok(v) = s.parse::<f64>() {
        if v >= 0.0 {
            return Some(v);
        }
    }
    None
}

fn extract_duration(
    format: Option<&serde_json::Value>,
    video_stream: Option<&serde_json::Value>,
    audio_stream: Option<&serde_json::Value>,
    fps: f64,
    ext: &str,
) -> f64 {
    // 1. Format duration
    if let Some(dur) = format
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(parse_time_string)
    {
        if dur > 0.0 {
            return dur;
        }
    }

    // 2. Video stream duration
    if let Some(vs) = video_stream {
        if let Some(dur) = vs
            .get("duration")
            .and_then(|d| d.as_str())
            .and_then(parse_time_string)
        {
            if dur > 0.0 {
                return dur;
            }
        }
    }

    // 3. Audio stream duration
    if let Some(as_) = audio_stream {
        if let Some(dur) = as_
            .get("duration")
            .and_then(|d| d.as_str())
            .and_then(parse_time_string)
        {
            if dur > 0.0 {
                return dur;
            }
        }
    }

    // 4. Video stream tags (DURATION, DURATION-eng, etc.)
    if let Some(vs) = video_stream {
        if let Some(tags) = vs.get("tags") {
            for key in &["DURATION", "duration", "DURATION-eng", "duration-eng"] {
                if let Some(dur) = tags
                    .get(*key)
                    .and_then(|d| d.as_str())
                    .and_then(parse_time_string)
                {
                    if dur > 0.0 {
                        return dur;
                    }
                }
            }
        }
    }

    // 5. Format tags
    if let Some(f) = format {
        if let Some(tags) = f.get("tags") {
            for key in &["DURATION", "duration", "DURATION-eng", "duration-eng"] {
                if let Some(dur) = tags
                    .get(*key)
                    .and_then(|d| d.as_str())
                    .and_then(parse_time_string)
                {
                    if dur > 0.0 {
                        return dur;
                    }
                }
            }
        }
    }

    // 6. Fallback: nb_frames / fps (frequent in Canon camera QuickTime MOV)
    if let Some(vs) = video_stream {
        if let Some(nb_frames) = vs
            .get("nb_frames")
            .and_then(|n| n.as_str())
            .and_then(|s| s.parse::<f64>().ok())
        {
            if fps > 0.0 && nb_frames > 0.0 {
                return nb_frames / fps;
            }
        }
    }

    // Default duration for static images
    if IMAGE_EXTENSIONS.contains(&ext) {
        5.0
    } else {
        0.0
    }
}

pub fn parse_fps(fps_str: &str) -> f64 {
    let s = fps_str.trim();
    if s.contains('/') {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() == 2 {
            if let (Ok(num), Ok(den)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                if den > 0.0 && num > 0.0 {
                    let val = num / den;
                    if val > 0.0 && val <= 240.0 {
                        return val;
                    }
                }
            }
        }
    } else if let Ok(val) = s.parse::<f64>() {
        if val > 0.0 && val <= 240.0 {
            return val;
        }
    }
    30.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_time_string_various_formats() {
        assert_eq!(parse_time_string(""), None);
        assert_eq!(parse_time_string("N/A"), None);
        assert_eq!(parse_time_string("n/a"), None);
        assert_eq!(parse_time_string("12.5"), Some(12.5));
        assert_eq!(parse_time_string("00:01:30.000"), Some(90.0));
        assert_eq!(parse_time_string("01:00:00.000"), Some(3600.0));
        assert_eq!(parse_time_string("02:15.500"), Some(135.5));
    }

    #[test]
    fn test_canon_eos_mov_metadata_parsing() {
        let json_val = json!({
            "streams": [
                {
                    "codec_type": "video",
                    "codec_name": "h264",
                    "width": 3840,
                    "height": 2160,
                    "r_frame_rate": "30000/1001",
                    "avg_frame_rate": "30000/1001",
                    "tags": {
                        "DURATION": "00:00:45.500000000",
                        "creation_time": "2026-10-25T08:30:00.000000Z"
                    }
                },
                {
                    "codec_type": "audio",
                    "codec_name": "pcm_s16le",
                    "channels": 2,
                    "sample_rate": "48000"
                },
                {
                    "codec_type": "data",
                    "codec_name": "tmcd",
                    "tags": {
                        "timecode": "00:01:00:00"
                    }
                }
            ],
            "format": {
                "size": "150000000",
                "format_name": "mov,mp4,m4a,3gp,3g2,mj2"
            }
        });

        let path = Path::new("/path/to/MVI_0001.MOV");
        let item = FFprobeAnalyzer::parse_probe_json(&json_val, path).unwrap();

        assert_eq!(item.media_type, MediaType::Video);
        assert_eq!(item.width, 3840);
        assert_eq!(item.height, 2160);
        assert!((item.fps - 29.97).abs() < 0.01);
        assert_eq!(item.video_codec, "h264");
        assert_eq!(item.audio_codec, "pcm_s16le");
        assert_eq!(item.audio_channels, 2);
        assert_eq!(item.audio_sample_rate, 48000);
        assert!((item.duration - 45.5).abs() < 0.01);
    }

    #[test]
    fn test_canon_mov_fallback_nb_frames_and_avg_fps() {
        let json_val = json!({
            "streams": [
                {
                    "codec_type": "video",
                    "codec_name": "hevc",
                    "width": 1920,
                    "height": 1080,
                    "r_frame_rate": "0/0",
                    "avg_frame_rate": "60000/1001",
                    "nb_frames": "3000"
                }
            ],
            "format": {
                "duration": "N/A",
                "size": "50000000"
            }
        });

        let path = Path::new("/DCIM/100CANON/IMG_0123.MOV");
        let item = FFprobeAnalyzer::parse_probe_json(&json_val, path).unwrap();

        assert_eq!(item.media_type, MediaType::Video);
        assert!((item.fps - 59.94).abs() < 0.01);
        // 3000 frames at ~59.94006 fps = ~50.05 seconds
        assert!((item.duration - 50.05).abs() < 0.1);
    }
}
