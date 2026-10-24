//! Export presets for rendering timelines.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HardwareAccel {
    #[default]
    Auto,
    Cpu,
    Nvenc,
    Qsv,
    Amf,
    Vaapi,
}

impl HardwareAccel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Auto => "Otomatik (En Hızlı Donanım GPU)",
            Self::Cpu => "Yazılım / İşlemci (CPU x264/x265)",
            Self::Nvenc => "NVIDIA NVENC (Donanım GPU)",
            Self::Qsv => "Intel QuickSync (QSV GPU)",
            Self::Amf => "AMD AMF (Radeon GPU)",
            Self::Vaapi => "Linux VAAPI (Donanım GPU)",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Auto => "Otomatik (GPU)",
            Self::Cpu => "CPU",
            Self::Nvenc => "NVENC",
            Self::Qsv => "QSV",
            Self::Amf => "AMF",
            Self::Vaapi => "VAAPI",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LoudnessNorm {
    #[default]
    Off,
    YouTube,      // -14 LUFS
    Podcast,      // -16 LUFS
    BroadcastEbu, // -23 LUFS
}

impl LoudnessNorm {
    pub const ALL: &'static [Self] = &[Self::Off, Self::YouTube, Self::Podcast, Self::BroadcastEbu];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Off => "Kapalı (Orijinal Seviye)",
            Self::YouTube => "YouTube / Spotify (-14 LUFS)",
            Self::Podcast => "Podcast Yayını (-16 LUFS)",
            Self::BroadcastEbu => "Avrupa Yayın EBU R128 (-23 LUFS)",
        }
    }

    pub fn target_lufs(&self) -> Option<f32> {
        match self {
            Self::Off => None,
            Self::YouTube => Some(-14.0),
            Self::Podcast => Some(-16.0),
            Self::BroadcastEbu => Some(-23.0),
        }
    }
}

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
    #[serde(default)]
    pub hw_accel: HardwareAccel,
    #[serde(default)]
    pub loudness_norm: LoudnessNorm,
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
            hw_accel: HardwareAccel::Auto,
            loudness_norm: LoudnessNorm::YouTube,
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
            hw_accel: HardwareAccel::Auto,
            loudness_norm: LoudnessNorm::YouTube,
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
            hw_accel: HardwareAccel::Auto,
            loudness_norm: LoudnessNorm::YouTube,
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
            hw_accel: HardwareAccel::Auto,
            loudness_norm: LoudnessNorm::Off,
        },
        ExportPreset {
            name: "4K UHD Sinematik (YouTube / TV)".to_string(),
            width: Some(3840),
            height: Some(2160),
            fps: Some(60.0),
            video_codec: "libx264".to_string(),
            audio_codec: "aac".to_string(),
            preset: "medium".to_string(),
            crf: 16,
            audio_bitrate: "320k".to_string(),
            hw_accel: HardwareAccel::Auto,
            loudness_norm: LoudnessNorm::YouTube,
        },
        ExportPreset {
            name: "TikTok / Instagram Reels (1080x1920 9:16 Dikey)".to_string(),
            width: Some(1080),
            height: Some(1920),
            fps: Some(30.0),
            video_codec: "libx264".to_string(),
            audio_codec: "aac".to_string(),
            preset: "medium".to_string(),
            crf: 19,
            audio_bitrate: "256k".to_string(),
            hw_accel: HardwareAccel::Auto,
            loudness_norm: LoudnessNorm::YouTube,
        },
        ExportPreset {
            name: "HEVC / H.265 1080p (Yüksek Sıkıştırma)".to_string(),
            width: Some(1920),
            height: Some(1080),
            fps: Some(30.0),
            video_codec: "libx265".to_string(),
            audio_codec: "aac".to_string(),
            preset: "medium".to_string(),
            crf: 22,
            audio_bitrate: "192k".to_string(),
            hw_accel: HardwareAccel::Auto,
            loudness_norm: LoudnessNorm::YouTube,
        },
        ExportPreset {
            name: "Apple ProRes 422 (Kayıpsız Düzenleme)".to_string(),
            width: Some(1920),
            height: Some(1080),
            fps: Some(30.0),
            video_codec: "prores_ks".to_string(),
            audio_codec: "pcm_s16le".to_string(),
            preset: "medium".to_string(),
            crf: 10,
            audio_bitrate: "384k".to_string(),
            hw_accel: HardwareAccel::Cpu,
            loudness_norm: LoudnessNorm::Off,
        },
    ]
}
