//! Project and ProjectSettings data models with versioning and format compatibility.

use crate::media::MediaItem;
use crate::timeline::TimelineModel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSettings {
    #[serde(default = "default_project_name")]
    pub name: String,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    #[serde(default = "default_fps")]
    pub fps: f64,
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
}

fn default_project_name() -> String {
    "İsimsiz Proje".to_string()
}
fn default_width() -> u32 {
    1920
}
fn default_height() -> u32 {
    1080
}
fn default_fps() -> f64 {
    30.0
}
fn default_sample_rate() -> u32 {
    48000
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            name: default_project_name(),
            width: 1920,
            height: 1080,
            fps: 30.0,
            sample_rate: 48000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub settings: ProjectSettings,
    #[serde(default)]
    pub media_pool: HashMap<String, MediaItem>,
    #[serde(default)]
    pub timeline: TimelineModel,
    #[serde(skip)]
    pub file_path: Option<String>,
    #[serde(skip)]
    pub is_dirty: bool,
}

fn default_format() -> String {
    "trim".to_string()
}
fn default_version() -> u32 {
    1
}

impl Default for Project {
    fn default() -> Self {
        Self {
            format: "trim".to_string(),
            version: 1,
            settings: ProjectSettings::default(),
            media_pool: HashMap::new(),
            timeline: TimelineModel::default(),
            file_path: None,
            is_dirty: false,
        }
    }
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        let mut proj = Self::default();
        proj.settings.name = name.into();
        proj
    }

    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    pub fn clear_dirty(&mut self) {
        self.is_dirty = false;
    }

    pub fn add_media(&mut self, item: MediaItem) {
        self.media_pool.insert(item.id.clone(), item);
        self.mark_dirty();
    }

    pub fn remove_media(&mut self, media_id: &str) -> Option<MediaItem> {
        let item = self.media_pool.remove(media_id);
        if item.is_some() {
            self.mark_dirty();
        }
        item
    }

    pub fn get_media(&self, media_id: &str) -> Option<&MediaItem> {
        self.media_pool.get(media_id)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        let mut proj: Self = serde_json::from_str(json_str)?;
        // Sync timeline dimensions from settings if needed
        proj.timeline.width = proj.settings.width;
        proj.timeline.height = proj.settings.height;
        proj.timeline.fps = proj.settings.fps;
        Ok(proj)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_serialization_roundtrip() {
        let mut proj = Project::new("Test Movie");
        let media = MediaItem::new("/videos/clip1.mp4");
        let m_id = media.id.clone();
        proj.add_media(media);

        let json = proj.to_json().unwrap();
        assert!(json.contains("\"format\": \"trim\""));

        let restored = Project::from_json(&json).unwrap();
        assert_eq!(restored.settings.name, "Test Movie");
        assert_eq!(restored.media_pool.len(), 1);
        assert!(restored.get_media(&m_id).is_some());
    }

    #[test]
    fn test_legacy_cutline_format_compatibility() {
        let legacy_json = r#"{
            "format": "cutline",
            "version": 1,
            "settings": {
                "name": "Legacy Cutline Project",
                "width": 1280,
                "height": 720,
                "fps": 24.0
            },
            "media_pool": {},
            "timeline": {
                "tracks": [],
                "markers": []
            }
        }"#;

        let proj = Project::from_json(legacy_json).unwrap();
        assert_eq!(proj.settings.name, "Legacy Cutline Project");
        assert_eq!(proj.settings.width, 1280);
        assert_eq!(proj.settings.fps, 24.0);
    }
}
