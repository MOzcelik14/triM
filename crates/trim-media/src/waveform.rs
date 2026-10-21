//! Audio waveform peak extraction and caching.

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct WaveformGenerator {
    pub cache_dir: PathBuf,
    pub points_per_second: u32,
    memory_cache: Arc<RwLock<HashMap<String, Vec<[f32; 2]>>>>,
}

impl Default for WaveformGenerator {
    fn default() -> Self {
        let c_dir = trim_core::paths::cache_dir().join("waveforms");
        Self {
            cache_dir: c_dir,
            points_per_second: 100,
            memory_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl WaveformGenerator {
    pub fn new(cache_dir: PathBuf, points_per_second: u32) -> Self {
        Self {
            cache_dir,
            points_per_second,
            memory_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn get_cache_path(&self, file_path: &Path) -> PathBuf {
        let meta = file_path.metadata().ok();
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let mtime = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let key = format!("{:?}_{}_{}_{}", file_path, size, mtime, self.points_per_second);
        // Simple hash
        let hash = format!("{:x}", md5_hash(&key));
        self.cache_dir.join(format!("{}.dat", hash))
    }

    /// Retrieves cached waveform or generates from media file.
    /// Returns a list of `[min_peak, max_peak]` in range `[-1.0, 1.0]`.
    pub fn get_waveform<P: AsRef<Path>>(&self, file_path: P) -> Option<Vec<[f32; 2]>> {
        let path = file_path.as_ref();
        let path_str = path.to_string_lossy().to_string();

        // 1. Instant RAM lookup (0 ms)
        if let Ok(cache) = self.memory_cache.read() {
            if let Some(peaks) = cache.get(&path_str) {
                return Some(peaks.clone());
            }
        }

        if !path.is_file() {
            return None;
        }

        let cache_path = self.get_cache_path(path);
        if cache_path.is_file() {
            if let Ok(data) = fs::read(&cache_path) {
                if data.len() % 8 == 0 && !data.is_empty() {
                    let mut peaks = Vec::with_capacity(data.len() / 8);
                    for chunk in data.as_chunks::<8>().0 {
                        let min = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
                        let max = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
                        peaks.push([min, max]);
                    }
                    if let Ok(mut cache) = self.memory_cache.write() {
                        cache.insert(path_str, peaks.clone());
                    }
                    return Some(peaks);
                }
            }
        }

        // Generate downsampled mono PCM
        let sample_rate = self.points_per_second * 40;
        let mut child = crate::ffmpeg::ffmpeg_cmd()
            .args([
                "-v",
                "quiet",
                "-y",
                "-i",
                path.to_str().unwrap_or(""),
                "-vn",
                "-ac",
                "1",
                "-ar",
                &sample_rate.to_string(),
                "-f",
                "s16le",
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;

        let mut raw_pcm = Vec::new();
        if let Some(mut stdout) = child.stdout.take() {
            let _ = stdout.read_to_end(&mut raw_pcm);
        }
        let _ = child.wait();

        if raw_pcm.len() < 80 {
            return None;
        }

        // Convert raw bytes to i16 samples
        let num_samples = raw_pcm.len() / 2;
        let mut samples = Vec::with_capacity(num_samples);
        for chunk in raw_pcm.as_chunks::<2>().0 {
            samples.push(i16::from_le_bytes([chunk[0], chunk[1]]));
        }

        // Group into buckets of 40
        let buckets = samples.len() / 40;
        let mut peaks: Vec<[f32; 2]> = Vec::with_capacity(buckets);
        for chunk in samples[..buckets * 40].as_chunks::<40>().0 {
            let mut min_val = i16::MAX;
            let mut max_val = i16::MIN;
            for &s in chunk {
                if s < min_val {
                    min_val = s;
                }
                if s > max_val {
                    max_val = s;
                }
            }
            peaks.push([min_val as f32 / 32768.0, max_val as f32 / 32768.0]);
        }

        // Save to cache
        let _ = fs::create_dir_all(&self.cache_dir);
        let mut out_bytes = Vec::with_capacity(peaks.len() * 8);
        for &[min, max] in &peaks {
            out_bytes.extend_from_slice(&min.to_le_bytes());
            out_bytes.extend_from_slice(&max.to_le_bytes());
        }
        let _ = fs::write(&cache_path, out_bytes);

        if let Ok(mut cache) = self.memory_cache.write() {
            cache.insert(path_str, peaks.clone());
        }

        Some(peaks)
    }
}

fn md5_hash(s: &str) -> u128 {
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
    fn test_waveform_nonexistent_file() {
        let gen = WaveformGenerator::default();
        let wf = gen.get_waveform("/nonexistent/audio.mp3");
        assert!(wf.is_none());
    }

    #[test]
    fn test_waveform_cache_path_generation() {
        let gen = WaveformGenerator::default();
        let path = Path::new("/path/to/media.mp4");
        let cache_p = gen.get_cache_path(path);
        assert!(cache_p.to_string_lossy().ends_with(".dat"));
    }
}
