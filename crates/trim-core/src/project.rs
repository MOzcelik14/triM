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
pub struct MediaBinFolder {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub color_rgb: Option<[u8; 3]>,
}

impl MediaBinFolder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            color_rgb: None,
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
    pub bins: Vec<MediaBinFolder>,
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
            bins: Vec::new(),
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

    pub fn get_media_mut(&mut self, media_id: &str) -> Option<&mut MediaItem> {
        self.media_pool.get_mut(media_id)
    }

    pub fn add_bin(&mut self, name: impl Into<String>) -> String {
        let folder = MediaBinFolder::new(name);
        let id = folder.id.clone();
        self.bins.push(folder);
        self.mark_dirty();
        id
    }

    pub fn remove_bin(&mut self, bin_id: &str) -> Option<MediaBinFolder> {
        if let Some(idx) = self.bins.iter().position(|b| b.id == bin_id) {
            let removed = self.bins.remove(idx);
            for item in self.media_pool.values_mut() {
                if item.bin_id.as_deref() == Some(bin_id) {
                    item.bin_id = None;
                }
            }
            self.mark_dirty();
            Some(removed)
        } else {
            None
        }
    }

    pub fn rename_bin(&mut self, bin_id: &str, new_name: impl Into<String>) -> bool {
        if let Some(bin) = self.bins.iter_mut().find(|b| b.id == bin_id) {
            bin.name = new_name.into();
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    pub fn get_bin(&self, bin_id: &str) -> Option<&MediaBinFolder> {
        self.bins.iter().find(|b| b.id == bin_id)
    }

    pub fn get_bin_mut(&mut self, bin_id: &str) -> Option<&mut MediaBinFolder> {
        self.bins.iter_mut().find(|b| b.id == bin_id)
    }

    pub fn set_resolution(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.settings.width = width;
            self.settings.height = height;
            self.timeline.width = width;
            self.timeline.height = height;
        }
    }

    pub fn set_fps(&mut self, fps: f64) {
        if fps > 0.0 {
            self.settings.fps = fps;
            self.timeline.fps = fps;
        }
    }

    pub fn sync_settings_from_timeline(&mut self) {
        if self.timeline.width > 0 && self.timeline.height > 0 {
            self.settings.width = self.timeline.width;
            self.settings.height = self.timeline.height;
        }
        if self.timeline.fps > 0.0 {
            self.settings.fps = self.timeline.fps;
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut normalized = self.clone();
        normalized.sync_settings_from_timeline();
        serde_json::to_string_pretty(&normalized)
    }

    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        let raw: serde_json::Value = serde_json::from_str(json_str)?;
        let timeline_has_dims = raw.get("timeline").and_then(|t| t.get("width")).is_some()
            && raw.get("timeline").and_then(|t| t.get("height")).is_some();

        let mut proj: Self = serde_json::from_value(raw)?;

        if timeline_has_dims {
            // If timeline had non-default resolution/fps (e.g. 3840x2160 set on timeline while
            // settings remained at default 1920x1080), preserve timeline values and sync settings.
            let timeline_non_default = proj.timeline.width != 1920
                || proj.timeline.height != 1080
                || (proj.timeline.fps - 30.0).abs() > 1e-4;
            let settings_is_default = proj.settings.width == 1920
                && proj.settings.height == 1080
                && (proj.settings.fps - 30.0).abs() <= 1e-4;

            if timeline_non_default || settings_is_default {
                proj.sync_settings_from_timeline();
            } else {
                proj.timeline.width = proj.settings.width;
                proj.timeline.height = proj.settings.height;
                proj.timeline.fps = proj.settings.fps;
            }
        } else {
            // Legacy format where dimensions only existed in settings
            proj.timeline.width = proj.settings.width;
            proj.timeline.height = proj.settings.height;
            proj.timeline.fps = proj.settings.fps;
        }
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
    fn test_uhd_resolution_preserved_on_autosave_recovery() {
        let mut proj = Project::new("UHD 4K Kurgu");
        // Simulate timeline resolution changed to 3860x2160 @ 60fps
        proj.timeline.width = 3860;
        proj.timeline.height = 2160;
        proj.timeline.fps = 60.0;

        let json = proj.to_json().unwrap();
        let restored = Project::from_json(&json).unwrap();
        assert_eq!(restored.timeline.width, 3860);
        assert_eq!(restored.timeline.height, 2160);
        assert_eq!(restored.timeline.fps, 60.0);
        assert_eq!(restored.settings.width, 3860);
        assert_eq!(restored.settings.height, 2160);
        assert_eq!(restored.settings.fps, 60.0);
    }

    #[test]
    fn test_legacy_unsynced_recovery_json_preserves_timeline_resolution() {
        // Simulate an older recovery file where settings had 1920x1080 but timeline had 3840x2160
        let unsynced_json = r#"{
            "format": "trim",
            "version": 1,
            "settings": {
                "name": "Yeni Proje",
                "width": 1920,
                "height": 1080,
                "fps": 30.0
            },
            "media_pool": {},
            "timeline": {
                "tracks": [],
                "markers": [],
                "width": 3840,
                "height": 2160,
                "fps": 59.94
            }
        }"#;

        let proj = Project::from_json(unsynced_json).unwrap();
        assert_eq!(proj.timeline.width, 3840);
        assert_eq!(proj.timeline.height, 2160);
        assert!((proj.timeline.fps - 59.94).abs() < 1e-4);
        assert_eq!(proj.settings.width, 3840);
        assert_eq!(proj.settings.height, 2160);
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
        assert_eq!(proj.timeline.width, 1280);
        assert_eq!(proj.timeline.height, 720);
        assert_eq!(proj.settings.fps, 24.0);
    }

    #[test]
    fn test_media_bin_folder_crud_and_serialization() {
        let mut proj = Project::default();
        let bin_id = proj.add_bin("Röportajlar");
        assert_eq!(proj.bins.len(), 1);
        assert_eq!(proj.bins[0].name, "Röportajlar");

        let item = MediaItem {
            id: "media-1".to_string(),
            bin_id: Some(bin_id.clone()),
            ..Default::default()
        };
        proj.add_media(item);

        assert_eq!(
            proj.get_media("media-1").unwrap().bin_id.as_deref(),
            Some(bin_id.as_str())
        );

        // Rename bin
        assert!(proj.rename_bin(&bin_id, "B-Roll"));
        assert_eq!(proj.get_bin(&bin_id).unwrap().name, "B-Roll");

        // Serialize and deserialize
        let json = proj.to_json().unwrap();
        let loaded = Project::from_json(&json).unwrap();
        assert_eq!(loaded.bins.len(), 1);
        assert_eq!(loaded.bins[0].name, "B-Roll");
        assert_eq!(
            loaded.get_media("media-1").unwrap().bin_id.as_deref(),
            Some(bin_id.as_str())
        );

        // Delete bin moves contained items to root (bin_id = None)
        let mut proj2 = loaded;
        let removed = proj2.remove_bin(&bin_id);
        assert!(removed.is_some());
        assert_eq!(proj2.bins.len(), 0);
        assert_eq!(proj2.get_media("media-1").unwrap().bin_id, None);
    }
}
