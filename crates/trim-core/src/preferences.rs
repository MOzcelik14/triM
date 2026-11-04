//! User preferences and global configuration management for triM.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application settings and user preferences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Interface language ("tr" or "en")
    pub language: String,
    /// Whether to show the startup splash screen
    pub show_splash_screen: bool,
    /// Whether to show the Project Hub / Welcome dialog on boot
    pub show_welcome_on_startup: bool,
    /// Default keyboard shortcut profile: "triM", "Premiere", "Resolve"
    pub shortcut_profile: String,
    /// Whether magnetic snapping is enabled by default
    pub snap_enabled_by_default: bool,
    /// Snapping sensitivity threshold in pixels
    pub snap_distance_px: f32,
    /// Autosave interval in seconds (0 = disabled, default 60)
    pub autosave_interval_secs: u64,
    /// Whether to use 720p Smart Proxy by default
    pub use_proxies_by_default: bool,
    /// Whether to automatically generate proxies for imported 4K+ media
    pub auto_generate_proxies: bool,
    /// Default still image / title duration in seconds
    pub default_still_duration_secs: f64,
    /// UI scaling factor (default 1.0)
    pub ui_scale: f32,
    /// Master limiter ceiling level in dBFS (default -0.1)
    pub limiter_ceiling_db: f32,
    /// Whether master bus limiter is enabled
    pub master_limiter_enabled: bool,
    /// Whether to show audio waveforms on timeline clips
    pub show_waveforms: bool,
    /// Whether to return playhead to start position on playback stop
    pub return_playhead_on_stop: bool,
    /// Preferred hardware export encoder ("auto", "nvenc", "qsv", "amf", "vaapi", "cpu")
    pub preferred_hw_accel: String,
    /// Default timeline track height in pixels
    pub default_track_height: f32,
    /// Maximum RAM memory budget in megabytes for frame and audio PCM caches (256, 512, 1024, 2048)
    #[serde(default = "default_memory_budget_mb")]
    pub max_memory_budget_mb: usize,
}

fn default_memory_budget_mb() -> usize {
    512
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            language: "tr".to_string(),
            show_splash_screen: true,
            show_welcome_on_startup: true,
            shortcut_profile: "triM".to_string(),
            snap_enabled_by_default: true,
            snap_distance_px: 12.0,
            autosave_interval_secs: 60,
            use_proxies_by_default: false,
            auto_generate_proxies: false,
            default_still_duration_secs: 5.0,
            ui_scale: 1.0,
            limiter_ceiling_db: -0.1,
            master_limiter_enabled: true,
            show_waveforms: true,
            return_playhead_on_stop: true,
            preferred_hw_accel: "auto".to_string(),
            default_track_height: 56.0,
            max_memory_budget_mb: 512,
        }
    }
}

impl UserPreferences {
    /// Filepath of the persistent preferences file.
    pub fn file_path() -> PathBuf {
        crate::paths::config_dir().join("preferences.json")
    }

    /// Load user preferences from standard config directory, falling back to defaults if not found.
    pub fn load() -> Self {
        let path = Self::file_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(prefs) = serde_json::from_str::<Self>(&content) {
                return prefs;
            }
        }
        Self::default()
    }

    /// Save user preferences to disk as formatted JSON.
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::file_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(&path, json)
    }

    /// Calculate total disk cache size in bytes (thumbnails, waveforms, frame cache).
    pub fn cache_size_bytes() -> u64 {
        let cache_dir = crate::paths::cache_dir();
        if !cache_dir.exists() {
            return 0;
        }
        let mut total = 0u64;
        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        total += meta.len();
                    } else if meta.is_dir() {
                        if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                            for sub in sub_entries.flatten() {
                                if let Ok(m) = sub.metadata() {
                                    total += m.len();
                                }
                            }
                        }
                    }
                }
            }
        }
        total
    }

    /// Clear all files in the cache directory.
    pub fn clear_cache() -> Result<(), std::io::Error> {
        let cache_dir = crate::paths::cache_dir();
        if cache_dir.exists() {
            std::fs::remove_dir_all(&cache_dir)?;
            std::fs::create_dir_all(&cache_dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preferences_defaults() {
        let prefs = UserPreferences::default();
        assert_eq!(prefs.language, "tr");
        assert!(prefs.show_splash_screen);
        assert!(prefs.show_welcome_on_startup);
        assert_eq!(prefs.shortcut_profile, "triM");
        assert_eq!(prefs.autosave_interval_secs, 60);
        assert_eq!(prefs.ui_scale, 1.0);
    }

    #[test]
    fn test_preferences_serialization_roundtrip() {
        let prefs = UserPreferences::default();
        let json = serde_json::to_string(&prefs).expect("Serialization failed");
        let parsed: UserPreferences = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(prefs, parsed);
    }
}
