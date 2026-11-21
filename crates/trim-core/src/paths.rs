//! Cross-platform standard directories (Windows, Linux, macOS).

use std::path::PathBuf;

/// Returns the user's home directory across operating systems.
/// On Windows, checks `%USERPROFILE%`, then `%HOME%`.
/// On Unix, checks `$HOME`.
pub fn home_dir() -> PathBuf {
    if cfg!(windows) {
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            if !userprofile.is_empty() {
                return PathBuf::from(userprofile);
            }
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home);
        }
    }
    std::env::temp_dir().join("trim_home_fallback")
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

/// Returns the standard user data directory (looks, presets, templates).
/// - Windows: `%APPDATA%\triM`
/// - Linux / macOS: `$XDG_DATA_HOME/trim` or `$HOME/.local/share/trim`
pub fn data_dir() -> PathBuf {
    if cfg!(windows) {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return PathBuf::from(appdata).join("triM");
            }
        }
        return home_dir().join(".trim");
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("trim");
        }
    }
    home_dir().join(".local").join("share").join("trim")
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

/// Normalizes mixed Unix (`/`) and Windows (`\`) path separators into the host OS path format.
pub fn normalize_path<P: AsRef<std::path::Path>>(path: P) -> PathBuf {
    let raw = path.as_ref().to_string_lossy();
    if cfg!(target_os = "windows") {
        PathBuf::from(raw.replace('/', "\\"))
    } else {
        PathBuf::from(raw.replace('\\', "/"))
    }
}

/// Formats a filesystem path for safe embedding inside an FFmpeg filtergraph option (e.g., `lut3d=file='...'`).
/// Converts Windows backslashes (`\`) to forward slashes (`/`) and escapes colons (`:`) and single quotes (`'`)
/// so Windows drive letters (`C:/...`) are not misinterpreted as FFmpeg filter option delimiters.
pub fn normalize_filtergraph_path<P: AsRef<std::path::Path>>(path: P) -> String {
    let forward = path.as_ref().to_string_lossy().replace('\\', "/");
    let mut out = String::with_capacity(forward.len() + 4);
    for ch in forward.chars() {
        match ch {
            '\0' | '\r' | '\n' => {}
            ':' => out.push_str("\\:"),
            '\'' => out.push_str("\\'"),
            _ => out.push(ch),
        }
    }
    out
}

/// Escapes an arbitrary string or path for safe embedding inside single quotes (`'...'`)
/// in an FFmpeg filtergraph option (e.g. `lut3d=file='...'`, `drawtext=textfile='...'`).
/// FFmpeg's `av_get_token` treats `\` as literal inside single quotes, so `'` must be
/// escaped as `'\''` (close quote, escaped quote, reopen quote). Control characters
/// (`\0`, `\r`, `\n`) are stripped to prevent filtergraph line breaks or truncation.
pub fn escape_ffmpeg_filter_single_quoted(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 8);
    for ch in raw.chars() {
        match ch {
            '\0' | '\r' | '\n' => {}
            '\'' => out.push_str("'\\''"),
            _ => out.push(ch),
        }
    }
    out
}

/// Deterministic 64-bit FNV-1a hash (stable across Rust compiler versions and platforms)
/// for persistent disk cache keys (thumbnails, waveforms, proxies).
pub fn stable_hash_64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Returns `(mtime_secs, file_size_bytes)` for cache invalidation when source media changes.
pub fn file_mtime_and_size(path: &std::path::Path) -> (u64, u64) {
    let Ok(meta) = std::fs::metadata(path) else {
        return (0, 0);
    };
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (mtime, meta.len())
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
        assert!(
            s.contains("trim"),
            "Expected path to contain 'trim', got {:?}",
            cfg
        );
    }

    #[test]
    fn test_cache_dir_ends_with_trim() {
        let cache = cache_dir();
        let s = cache.to_string_lossy().to_lowercase();
        assert!(
            s.contains("trim"),
            "Expected path to contain 'trim', got {:?}",
            cache
        );
    }

    #[test]
    fn test_videos_dir() {
        let v = videos_dir();
        let s = v.to_string_lossy();
        assert!(
            s.ends_with("Videos"),
            "Expected path to end with 'Videos', got {:?}",
            v
        );
    }

    #[test]
    fn test_normalize_path_and_filtergraph_escaping() {
        let win_lut = r"C:\Users\editor\LUTs\cinematic_warm.cube";
        let escaped = normalize_filtergraph_path(win_lut);
        assert_eq!(escaped, "C\\:/Users/editor/LUTs/cinematic_warm.cube");

        let unix_lut = "/home/editor/luts/ film's_lut.cube";
        let escaped_unix = normalize_filtergraph_path(unix_lut);
        assert_eq!(escaped_unix, "/home/editor/luts/ film\\'s_lut.cube");

        let rel_mixed = r"media\clip_01.mp4";
        let norm = normalize_path(rel_mixed);
        if cfg!(target_os = "windows") {
            assert_eq!(norm, PathBuf::from(r"media\clip_01.mp4"));
        } else {
            assert_eq!(norm, PathBuf::from("media/clip_01.mp4"));
        }
    }
}
