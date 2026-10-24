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
                "quiet",
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
            return Err(ProbeError::ProcessFailed(stderr.to_string()));
        }

        let json_val: serde_json::Value = serde_json::from_slice(&output.stdout)?;

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
                if !is_attached_pic {
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

        let mut duration = format
            .and_then(|f| f.get("duration"))
            .and_then(|d| d.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        if duration <= 0.0 {
            if let Some(vs) = video_stream {
                if let Some(d) = vs
                    .get("duration")
                    .and_then(|d| d.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                {
                    duration = d;
                }
            }
        }
        if duration <= 0.0 {
            if let Some(as_) = audio_stream {
                if let Some(d) = as_
                    .get("duration")
                    .and_then(|d| d.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                {
                    duration = d;
                }
            }
        }

        let file_size_bytes = format
            .and_then(|f| f.get("size"))
            .and_then(|s| s.as_str())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        let media_type = if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            if duration <= 0.0 {
                duration = 5.0; // Default image duration
            }
            MediaType::Image
        } else if video_stream.is_some() {
            MediaType::Video
        } else if audio_stream.is_some() || AUDIO_EXTENSIONS.contains(&ext.as_str()) {
            MediaType::Audio
        } else {
            MediaType::Unknown
        };

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

            let fps_str = vs
                .get("r_frame_rate")
                .or_else(|| vs.get("avg_frame_rate"))
                .and_then(|r| r.as_str())
                .unwrap_or("30/1");

            fps = parse_fps(fps_str);
        }

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

fn parse_fps(fps_str: &str) -> f64 {
    if fps_str.contains('/') {
        let parts: Vec<&str> = fps_str.split('/').collect();
        if parts.len() == 2 {
            if let (Ok(num), Ok(den)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                if den > 0.0 {
                    let val = num / den;
                    if val > 0.0 && val <= 240.0 {
                        return val;
                    }
                }
            }
        }
    } else if let Ok(val) = fps_str.parse::<f64>() {
        if val > 0.0 && val <= 240.0 {
            return val;
        }
    }
    30.0
}
