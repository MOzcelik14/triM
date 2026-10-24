//! MediaItem and MediaType models.

use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum MediaType {
    Video,
    Audio,
    Image,
    #[default]
    Unknown,
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Video => write!(f, "video"),
            Self::Audio => write!(f, "audio"),
            Self::Image => write!(f, "image"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: String,
    pub file_path: String,
    pub name: String,
    #[serde(default)]
    pub media_type: MediaType,
    #[serde(default)]
    pub duration: f64,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub fps: f64,
    #[serde(default)]
    pub video_codec: String,
    #[serde(default)]
    pub audio_codec: String,
    #[serde(default)]
    pub audio_channels: u32,
    #[serde(default)]
    pub audio_sample_rate: u32,
    #[serde(default)]
    pub file_size_bytes: u64,
    #[serde(default)]
    pub date_added: f64,
    #[serde(default)]
    pub proxy_path: Option<String>,
}

impl Default for MediaItem {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            file_path: String::new(),
            name: String::new(),
            media_type: MediaType::Unknown,
            duration: 0.0,
            width: 0,
            height: 0,
            fps: 0.0,
            video_codec: String::new(),
            audio_codec: String::new(),
            audio_channels: 0,
            audio_sample_rate: 0,
            file_size_bytes: 0,
            date_added: 0.0,
            proxy_path: None,
        }
    }
}

impl MediaItem {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        let p = file_path.as_ref();
        let name = p
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();
        Self {
            id: Uuid::new_v4().to_string(),
            file_path: p.to_string_lossy().to_string(),
            name,
            ..Default::default()
        }
    }

    pub fn exists(&self) -> bool {
        Path::new(&self.file_path).is_file()
    }

    pub fn has_video(&self) -> bool {
        self.media_type == MediaType::Video
            || self.media_type == MediaType::Image
            || (!self.video_codec.is_empty() && self.width > 0)
    }

    pub fn has_audio(&self) -> bool {
        self.media_type == MediaType::Audio
            || !self.audio_codec.is_empty()
            || self.audio_channels > 0
    }

    /// Resolves either the proxy file path (if enabled and file exists) or original full-res media path.
    pub fn resolved_playback_path(&self, use_proxy: bool) -> &str {
        if use_proxy {
            if let Some(ref p) = self.proxy_path {
                if Path::new(p).exists() {
                    return p.as_str();
                }
            }
        }
        &self.file_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_item_serialization() {
        let mut item = MediaItem::new("/path/to/video.mp4");
        item.media_type = MediaType::Video;
        item.duration = 12.5;
        item.width = 1920;
        item.height = 1080;
        item.fps = 30.0;
        item.video_codec = "h264".into();

        let json = serde_json::to_string(&item).unwrap();
        let deserialized: MediaItem = serde_json::from_str(&json).unwrap();

        assert_eq!(item.id, deserialized.id);
        assert_eq!(item.file_path, deserialized.file_path);
        assert_eq!(item.duration, deserialized.duration);
        assert_eq!(item.media_type, deserialized.media_type);
    }

    #[test]
    fn test_media_item_proxy_path_resolution() {
        let mut item = MediaItem::new("/raw/4k_video.mp4");
        assert_eq!(item.resolved_playback_path(false), "/raw/4k_video.mp4");
        assert_eq!(item.resolved_playback_path(true), "/raw/4k_video.mp4");

        // Set nonexistent proxy path: should still fall back to raw
        item.proxy_path = Some("/nonexistent/proxy.mp4".to_string());
        assert_eq!(item.resolved_playback_path(true), "/raw/4k_video.mp4");
    }
}
