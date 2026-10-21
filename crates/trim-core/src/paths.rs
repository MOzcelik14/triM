//! Cross-platform standard directories (Windows, Linux, macOS).

use std::path::PathBuf;

/// Returns the user's home directory across operating systems.
/// On Windows, checks `%USERPROFILE%`, then `%HOME%`.
/// On Unix, checks `$HOME`.
pub fn home_dir() -> PathBuf {
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        if !userprofile.is_empty() {
            return PathBuf::from(userprofile);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home);
        }
    }
    PathBuf::from(".")
}

/// Returns the standard configuration & persistent application data directory.
/// - Windows: `%APPDATA%\triM` (e.g. `C:\Users\Username\AppData\Roaming\triM`)
/// - Linux / macOS: `$HOME/.config/trim`
pub fn config_dir() -> PathBuf {
    if cfg!(windows) {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return PathBuf::from(appdata).join("triM");
            }
        }
        return home_dir().join(".trim");
    }

    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home).join(".config").join("trim");
        }
    }
    home_dir().join(".config").join("trim")
}

/// Returns the standard cache directory for temporary/reproducible data (thumbnails, waveforms, frames).
/// - Windows: `%LOCALAPPDATA%\triM` (e.g. `C:\Users\Username\AppData\Local\triM`)
/// - Linux / macOS: `$HOME/.cache/trim`
pub fn cache_dir() -> PathBuf {
    if cfg!(windows) {
        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            if !localappdata.is_empty() {
                return PathBuf::from(localappdata).join("triM");
            }
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return PathBuf::from(appdata).join("triM").join("cache");
            }
        }
        return home_dir().join(".cache").join("trim");
    }

    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home).join(".cache").join("trim");
        }
    }
    home_dir().join(".cache").join("trim")
}

/// Returns the default videos directory for exports and projects.
/// - Windows: `%USERPROFILE%\Videos`
/// - Linux / macOS: `$HOME/Videos`
pub fn videos_dir() -> PathBuf {
    let videos = home_dir().join("Videos");
    if videos.exists() {
        return videos;
    }
    // Return standard path even if not yet created
    videos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_home_dir_not_empty() {
        let home = home_dir();
        assert!(!home.as_os_str().is_empty());
    }

    #[test]
    fn test_config_dir_ends_with_trim() {
        let cfg = config_dir();
        let s = cfg.to_string_lossy().to_lowercase();
        assert!(s.contains("trim"), "Expected path to contain 'trim', got {:?}", cfg);
    }

    #[test]
    fn test_cache_dir_ends_with_trim() {
        let cache = cache_dir();
        let s = cache.to_string_lossy().to_lowercase();
        assert!(s.contains("trim"), "Expected path to contain 'trim', got {:?}", cache);
    }

    #[test]
    fn test_videos_dir() {
        let v = videos_dir();
        let s = v.to_string_lossy();
        assert!(s.ends_with("Videos"), "Expected path to end with 'Videos', got {:?}", v);
    }
}
