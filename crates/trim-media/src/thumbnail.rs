//! Poster frame thumbnail extraction, background worker pooling, and disk caching.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use trim_core::media::{MediaItem, MediaType};

#[derive(Clone)]
pub struct ThumbnailGenerator {
    pub cache_dir: PathBuf,
    req_tx: Sender<MediaItem>,
    res_rx: Arc<Mutex<Receiver<(String, PathBuf)>>>,
    in_flight: Arc<Mutex<HashSet<String>>>,
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
        let (req_tx, req_rx) = channel::<MediaItem>();
        let req_rx = Arc::new(Mutex::new(req_rx));
        let (res_tx, res_rx) = channel::<(String, PathBuf)>();
        let in_flight = Arc::new(Mutex::new(HashSet::<String>::new()));

        // Bounded worker pool (2 concurrent workers)
        let num_workers = 2usize;
        for w_idx in 0..num_workers {
            let rx = req_rx.clone();
            let res_tx = res_tx.clone();
            let in_flight_worker = in_flight.clone();
            let cache_dir_clone = cache_dir.clone();

            if let Err(e) = std::thread::Builder::new()
                .name(format!("thumbnail-worker-{}", w_idx))
                .spawn(move || {
                    loop {
                        let item = {
                            let Ok(lock) = rx.lock() else { break };
                            match lock.recv() {
                                Ok(it) => it,
                                Err(_) => break,
                            }
                        };
                        let target = get_thumbnail_path_static(&cache_dir_clone, &item);
                        let ok = if target.is_file()
                            && target.metadata().map(|m| m.len() > 0).unwrap_or(false)
                        {
                            true
                        } else {
                            generate_thumbnail_disk(&cache_dir_clone, &item).is_some()
                        };

                        if ok {
                            let _ = res_tx.send((item.id.clone(), target));
                        }
                        if let Ok(mut set) = in_flight_worker.lock() {
                            set.remove(&item.id);
                        }
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
        }
    }

    pub fn get_thumbnail_path(&self, item: &MediaItem) -> PathBuf {
        get_thumbnail_path_static(&self.cache_dir, item)
    }

    /// Submits a background thumbnail generation request if not already cached or in-flight.
    pub fn request_thumbnail(&self, item: &MediaItem) {
        let target = self.get_thumbnail_path(item);
        if target.is_file() && target.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            if let Ok(mut set) = self.in_flight.lock() {
                if !set.contains(&item.id) {
                    let _ = self.req_tx.send(item.clone());
                    set.insert(item.id.clone());
                }
            }
            return;
        }

        let mut set = match self.in_flight.lock() {
            Ok(s) => s,
            Err(_) => return,
        };
        if set.contains(&item.id) {
            return;
        }
        set.insert(item.id.clone());
        let _ = self.req_tx.send(item.clone());
    }

    /// Non-blocking poll for newly generated thumbnails from the background worker.
    pub fn poll_thumbnail(&self) -> Option<(String, PathBuf)> {
        self.res_rx.lock().ok()?.try_recv().ok()
    }

    /// Synchronous generation fallback for headless/export workflows.
    pub fn generate(&self, item: &MediaItem) -> Option<PathBuf> {
        generate_thumbnail_disk(&self.cache_dir, item)
    }
}

fn get_thumbnail_path_static(cache_dir: &Path, item: &MediaItem) -> PathBuf {
    let key = format!(
        "{}_{}_{}x{}",
        item.file_path, item.duration, item.width, item.height
    );
    let hash = format!("{:x}", simple_hash(&key));
    cache_dir.join(format!("{}.jpg", hash))
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
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h1 = DefaultHasher::new();
    s.hash(&mut h1);
    let mut h2 = DefaultHasher::new();
    s.chars().rev().collect::<String>().hash(&mut h2);
    ((h1.finish() as u128) << 64) | (h2.finish() as u128)
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

        // Polling ready receiver should not hang
        let ready = gen.poll_thumbnail();
        assert!(ready.is_none());
    }
}
