//! Poster frame thumbnail extraction, background worker pooling, and disk caching.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use trim_core::media::{MediaItem, MediaType};

pub const FILMSTRIP_TILE_COUNT: usize = 8;

#[derive(Clone)]
pub struct ThumbnailGenerator {
    pub cache_dir: PathBuf,
    req_tx: Sender<(MediaItem, bool)>,
    res_rx: Arc<Mutex<Receiver<(String, PathBuf)>>>,
    in_flight: Arc<Mutex<HashSet<String>>>,
    failed_keys: Arc<Mutex<HashSet<String>>>,
}

impl std::fmt::Debug for ThumbnailGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThumbnailGenerator")
            .field("cache_dir", &self.cache_dir)
            .finish_non_exhaustive()
    }
}

impl Default for ThumbnailGenerator {
    fn default() -> Self {
        let c_dir = trim_core::paths::cache_dir().join("thumbnails");
        Self::new(c_dir)
    }
}

impl ThumbnailGenerator {
    pub fn new(cache_dir: PathBuf) -> Self {
        let (req_tx, req_rx) = channel::<(MediaItem, bool)>();
        let req_rx = Arc::new(Mutex::new(req_rx));
        let (res_tx, res_rx) = channel::<(String, PathBuf)>();
        let in_flight = Arc::new(Mutex::new(HashSet::<String>::new()));
        let failed_keys = Arc::new(Mutex::new(HashSet::<String>::new()));

        // Bounded worker pool (2 concurrent workers)
        let num_workers = 2usize;
        for w_idx in 0..num_workers {
            let rx = req_rx.clone();
            let res_tx = res_tx.clone();
            let in_flight_worker = in_flight.clone();
            let failed_worker = failed_keys.clone();
            let cache_dir_clone = cache_dir.clone();

            if let Err(e) = std::thread::Builder::new()
                .name(format!("thumbnail-worker-{}", w_idx))
                .spawn(move || loop {
                    let (item, is_filmstrip) = {
                        let Ok(lock) = rx.lock() else { break };
                        match lock.recv() {
                            Ok(it) => it,
                            Err(_) => break,
                        }
                    };
                    let key = if is_filmstrip {
                        format!("strip:{}", item.id)
                    } else {
                        item.id.clone()
                    };
                    let target = if is_filmstrip {
                        get_filmstrip_path_static(&cache_dir_clone, &item)
                    } else {
                        get_thumbnail_path_static(&cache_dir_clone, &item)
                    };
                    let ok = if target.is_file()
                        && target.metadata().map(|m| m.len() > 0).unwrap_or(false)
                    {
                        true
                    } else if is_filmstrip {
                        generate_filmstrip_disk(&cache_dir_clone, &item).is_some()
                    } else {
                        generate_thumbnail_disk(&cache_dir_clone, &item).is_some()
                    };

                    if ok {
                        let _ = res_tx.send((key.clone(), target));
                    } else if let Ok(mut failed) = failed_worker.lock() {
                        failed.insert(key.clone());
                    }
                    if let Ok(mut set) = in_flight_worker.lock() {
                        set.remove(&key);
                    }
                })
            {
                log::error!("Failed to spawn thumbnail worker thread #{}: {}", w_idx, e);
            }
        }

        Self {
            cache_dir,
            req_tx,
            res_rx: Arc::new(Mutex::new(res_rx)),
            in_flight,
            failed_keys,
        }
    }

    pub fn get_thumbnail_path(&self, item: &MediaItem) -> PathBuf {
        get_thumbnail_path_static(&self.cache_dir, item)
    }

    pub fn get_filmstrip_path(&self, item: &MediaItem) -> PathBuf {
        get_filmstrip_path_static(&self.cache_dir, item)
    }

    /// Submits a background thumbnail generation request if not already cached or in-flight.
    pub fn request_thumbnail(&self, item: &MediaItem) {
        self.enqueue_job(item, false);
    }

    /// Submits a background 8-tile filmstrip generation request for video clips.
    pub fn request_filmstrip(&self, item: &MediaItem) {
        if item.media_type != MediaType::Video {
            return;
        }
        self.enqueue_job(item, true);
    }

    fn enqueue_job(&self, item: &MediaItem, is_filmstrip: bool) {
        let key = if is_filmstrip {
            format!("strip:{}", item.id)
        } else {
            item.id.clone()
        };
        if let Ok(failed) = self.failed_keys.lock() {
            if failed.contains(&key) {
                return;
            }
        }
        let mut set = match self.in_flight.lock() {
            Ok(s) => s,
            Err(_) => return,
        };
        if set.contains(&key) {
            return;
        }
        set.insert(key);
        let _ = self.req_tx.send((item.clone(), is_filmstrip));
    }

    /// Non-blocking poll for newly generated thumbnails/filmstrips from the background worker.
    pub fn poll_thumbnail(&self) -> Option<(String, PathBuf)> {
        self.res_rx.lock().ok()?.try_recv().ok()
    }

    /// Synchronous generation fallback for headless/export workflows.
    pub fn generate(&self, item: &MediaItem) -> Option<PathBuf> {
        generate_thumbnail_disk(&self.cache_dir, item)
    }
}

fn get_thumbnail_path_static(cache_dir: &Path, item: &MediaItem) -> PathBuf {
    let (mtime, size) = trim_core::paths::file_mtime_and_size(Path::new(&item.file_path));
    let key = format!(
        "{}_{}_{}x{}_{}_{}",
        item.file_path, item.duration, item.width, item.height, mtime, size
    );
    let hash = format!("{:x}", simple_hash(&key));
    cache_dir.join(format!("{}.jpg", hash))
}

fn get_filmstrip_path_static(cache_dir: &Path, item: &MediaItem) -> PathBuf {
    let (mtime, size) = trim_core::paths::file_mtime_and_size(Path::new(&item.file_path));
    let key = format!(
        "{}_{}_{}x{}_{}_{}",
        item.file_path, item.duration, item.width, item.height, mtime, size
    );
    let hash = format!("{:x}", simple_hash(&key));
    cache_dir.join(format!("{}_strip{}.jpg", hash, FILMSTRIP_TILE_COUNT))
}

fn generate_filmstrip_disk(cache_dir: &Path, item: &MediaItem) -> Option<PathBuf> {
    if item.media_type != MediaType::Video {
        return None;
    }
    let target = get_filmstrip_path_static(cache_dir, item);
    if target.is_file() && target.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        return Some(target);
    }

    let src = Path::new(&item.file_path);
    if !src.is_file() {
        return None;
    }

    let _ = fs::create_dir_all(cache_dir);
    let dur = item.duration.max(0.8);
    let fps_val = ((FILMSTRIP_TILE_COUNT as f64 + 0.4) / dur).clamp(0.02, 30.0);
    let vf = format!(
        "fps=fps={:.4},scale=160:90:force_original_aspect_ratio=increase,crop=160:90,tile={}x1",
        fps_val, FILMSTRIP_TILE_COUNT
    );

    let mut cmd = crate::ffmpeg::ffmpeg_cmd();
    cmd.args([
        "-v",
        "quiet",
        "-y",
        "-i",
        item.file_path.as_str(),
        "-vf",
        &vf,
        "-frames:v",
        "1",
        "-q:v",
        "4",
        target.to_str().unwrap_or(""),
    ]);
    let mut guard = crate::ffmpeg::ChildGuard::spawn(&mut cmd).ok()?;
    let status = guard.wait().ok()?;
    if status.success() && target.is_file() {
        Some(target)
    } else {
        None
    }
}

fn generate_thumbnail_disk(cache_dir: &Path, item: &MediaItem) -> Option<PathBuf> {
    let target = get_thumbnail_path_static(cache_dir, item);
    if target.is_file() && target.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        return Some(target);
    }

    let src = Path::new(&item.file_path);
    if !src.is_file() {
        return None;
    }

    let _ = fs::create_dir_all(cache_dir);

    match item.media_type {
        MediaType::Image => {
            let mut cmd = crate::ffmpeg::ffmpeg_cmd();
            cmd.args([
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
            ]);
            let mut guard = crate::ffmpeg::ChildGuard::spawn(&mut cmd).ok()?;
            let status = guard.wait().ok()?;
            if status.success() && target.is_file() {
                Some(target)
            } else {
                None
            }
        }
        MediaType::Video => {
            let seek_time = (item.duration * 0.1).clamp(0.0, 1.0);
            let mut cmd = crate::ffmpeg::ffmpeg_cmd();
            cmd.args([
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
            ]);
            let mut guard = crate::ffmpeg::ChildGuard::spawn(&mut cmd).ok()?;
            let status = guard.wait().ok()?;
            if status.success() && target.is_file() {
                Some(target)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn simple_hash(s: &str) -> u128 {
    let h1 = trim_core::paths::stable_hash_64(s.as_bytes());
    let rev: String = s.chars().rev().collect();
    let h2 = trim_core::paths::stable_hash_64(rev.as_bytes());
    ((h1 as u128) << 64) | (h2 as u128)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thumbnail_generator_bounded_pool() {
        let temp_dir = std::env::temp_dir().join("trim_thumb_pool_test");
        let gen = ThumbnailGenerator::new(temp_dir);

        let item = MediaItem::new("/nonexistent/video.mp4");
        gen.request_thumbnail(&item);
        gen.request_filmstrip(&item);

        // Polling ready receiver should not hang
        let ready = gen.poll_thumbnail();
        assert!(ready.is_none());
        assert!(gen
            .get_filmstrip_path(&item)
            .to_string_lossy()
            .ends_with("_strip8.jpg"));
    }
}
