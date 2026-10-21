//! Poster frame thumbnail extraction and disk caching.

use std::fs;
use std::path::{Path, PathBuf};
use trim_core::media::{MediaItem, MediaType};

#[derive(Debug, Clone)]
pub struct ThumbnailGenerator {
    pub cache_dir: PathBuf,
}

impl Default for ThumbnailGenerator {
    fn default() -> Self {
        let c_dir = trim_core::paths::cache_dir().join("thumbnails");
        Self { cache_dir: c_dir }
    }
}

impl ThumbnailGenerator {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    pub fn get_thumbnail_path(&self, item: &MediaItem) -> PathBuf {
        let key = format!("{}_{}_{}x{}", item.file_path, item.duration, item.width, item.height);
        let hash = format!("{:x}", simple_hash(&key));
        self.cache_dir.join(format!("{}.jpg", hash))
    }

    pub fn generate(&self, item: &MediaItem) -> Option<PathBuf> {
        let target = self.get_thumbnail_path(item);
        if target.is_file() && target.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            return Some(target);
        }

        let src = Path::new(&item.file_path);
        if !src.is_file() {
            return None;
        }

        let _ = fs::create_dir_all(&self.cache_dir);

        match item.media_type {
            MediaType::Image => {
                let status = crate::ffmpeg::ffmpeg_cmd()
                    .args([
                        "-v",
                        "quiet",
                        "-y",
                        "-i",
                        item.file_path.as_str(),
                        "-vf",
                        "scale=240:-1",
                        "-q:v",
                        "3",
                        target.to_str().unwrap_or(""),
                    ])
                    .status()
                    .ok()?;
                if status.success() && target.is_file() {
                    Some(target)
                } else {
                    None
                }
            }
            MediaType::Video => {
                let seek_time = (item.duration * 0.1).clamp(0.0, 1.0);
                let status = crate::ffmpeg::ffmpeg_cmd()
                    .args([
                        "-v",
                        "quiet",
                        "-y",
                        "-ss",
                        &format!("{:.2}", seek_time),
                        "-i",
                        item.file_path.as_str(),
                        "-vframes",
                        "1",
                        "-vf",
                        "scale=240:-1",
                        "-q:v",
                        "3",
                        target.to_str().unwrap_or(""),
                    ])
                    .status()
                    .ok()?;
                if status.success() && target.is_file() {
                    Some(target)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

fn simple_hash(s: &str) -> u128 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h1 = DefaultHasher::new();
    s.hash(&mut h1);
    let mut h2 = DefaultHasher::new();
    s.chars().rev().collect::<String>().hash(&mut h2);
    ((h1.finish() as u128) << 64) | (h2.finish() as u128)
}
