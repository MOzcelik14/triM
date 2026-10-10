//! Export presets for rendering timelines.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportPreset {
    pub name: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub video_codec: String,
    pub audio_codec: String,
    pub preset: String,
    pub crf: u32,
    pub audio_bitrate: String,
}

impl Default for ExportPreset {
    fn default() -> Self {
        Self {
            name: "1080p Yüksek Kalite (YouTube)".to_string(),
            width: Some(1920),
            height: Some(1080),
            fps: Some(30.0),
            video_codec: "libx264".to_string(),
            audio_codec: "aac".to_string(),
            preset: "medium".to_string(),
            crf: 18,
            audio_bitrate: "256k".to_string(),
        }
    }
}

pub fn get_default_presets() -> Vec<ExportPreset> {
    vec![
        ExportPreset {
            name: "1080p Yüksek Kalite (YouTube)".to_string(),
            width: Some(1920),
            height: Some(1080),
            fps: Some(30.0),
            video_codec: "libx264".to_string(),
            audio_codec: "aac".to_string(),
            preset: "medium".to_string(),
            crf: 18,
            audio_bitrate: "256k".to_string(),
        },
        ExportPreset {
            name: "720p Hızlı (Düşük Boyut)".to_string(),
            width: Some(1280),
            height: Some(720),
            fps: Some(30.0),
            video_codec: "libx264".to_string(),
            audio_codec: "aac".to_string(),
            preset: "veryfast".to_string(),
            crf: 23,
            audio_bitrate: "128k".to_string(),
        },
        ExportPreset {
            name: "Kaynak ile Eşleşen".to_string(),
            width: None,
            height: None,
            fps: None,
            video_codec: "libx264".to_string(),
            audio_codec: "aac".to_string(),
            preset: "medium".to_string(),
            crf: 18,
            audio_bitrate: "192k".to_string(),
        },
        ExportPreset {
            name: "HEVC / H.265 1080p".to_string(),
            width: Some(1920),
            height: Some(1080),
            fps: Some(30.0),
            video_codec: "libx265".to_string(),
            audio_codec: "aac".to_string(),
            preset: "medium".to_string(),
            crf: 22,
            audio_bitrate: "192k".to_string(),
        },
    ]
}
