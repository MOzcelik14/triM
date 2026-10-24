//! Audio decoding, multi-track PCM mixing, and dual VU meter RMS level analysis.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use trim_core::lru::LruCache;
use trim_core::media::MediaType;
use trim_core::project::Project;

use std::sync::{Arc, RwLock};

pub const SAMPLE_RATE: u32 = 48000;
pub const MAX_PCM_CACHE_BYTES: usize = 512 * 1024 * 1024; // 512 MB hard cap

pub type PcmAudioBuffer = Option<Arc<Vec<[i16; 2]>>>;
pub type PcmCache = Arc<RwLock<LruCache<String, PcmAudioBuffer>>>;

#[derive(Clone)]
pub struct AudioMixer {
    pcm_cache: PcmCache,
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for AudioMixer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioMixer").finish_non_exhaustive()
    }
}

impl AudioMixer {
    pub fn new() -> Self {
        Self {
            pcm_cache: Arc::new(RwLock::new(LruCache::with_byte_limit(MAX_PCM_CACHE_BYTES))),
        }
    }

    /// Asynchronously preloads audio PCM in a background thread upon media import.
    pub fn preload_audio_async<P: AsRef<Path>>(&self, file_path: P) {
        let path = file_path.as_ref().to_path_buf();
        let path_str = path.to_string_lossy().to_string();
        let cache_clone = self.pcm_cache.clone();

        std::thread::spawn(move || {
            if let Ok(c) = cache_clone.read() {
                if c.contains_key(&path_str) {
                    return;
                }
            }
            let pcm = decode_audio_to_pcm(&path).map(Arc::new);
            let size_bytes = pcm.as_ref().map(|a| a.len() * 4).unwrap_or(32);
            if let Ok(mut c) = cache_clone.write() {
                c.insert_with_size(path_str, pcm, size_bytes);
            }
        });
    }

    /// Loads and caches 48kHz 16-bit stereo PCM for given media file.
    pub fn get_audio_pcm<P: AsRef<Path>>(&self, file_path: P) -> Option<Arc<Vec<[i16; 2]>>> {
        let path = file_path.as_ref();
        let path_str = path.to_string_lossy().to_string();

        if let Ok(mut c) = self.pcm_cache.write() {
            if let Some(opt) = c.get(&path_str) {
                return opt.clone();
            }
        }

        let pcm = decode_audio_to_pcm(path).map(Arc::new);
        let size_bytes = pcm.as_ref().map(|a| a.len() * 4).unwrap_or(32);
        if let Ok(mut c) = self.pcm_cache.write() {
            c.insert_with_size(path_str, pcm.clone(), size_bytes);
        }
        pcm
    }

    /// Renders and mixes multi-track PCM audio for the interval [t_start, t_end].
    /// Returns `(interleaved_samples_i16, left_dbfs, right_dbfs)`.
    pub fn render_audio_interval(
        &mut self,
        project: &Project,
        t_start: f64,
        t_end: f64,
    ) -> (Vec<i16>, f32, f32) {
        if t_end <= t_start {
            return (Vec::new(), -60.0, -60.0);
        }

        let num_samples = ((t_end - t_start) * SAMPLE_RATE as f64).round() as usize;
        if num_samples == 0 {
            return (Vec::new(), -60.0, -60.0);
        }

        let mut mixed: Vec<[f32; 2]> = vec![[0.0, 0.0]; num_samples];
        let has_solo = project.timeline.tracks.iter().any(|t| t.solo);

        for track in &project.timeline.tracks {
            if track.muted {
                continue;
            }
            if has_solo && !track.solo {
                continue;
            }
            let track_vol = track.volume as f32;

            for clip in &track.clips {
                if clip.muted {
                    continue;
                }
                if clip.timeline_out <= t_start || clip.timeline_in >= t_end {
                    continue;
                }

                let media_item = match project.get_media(&clip.media_id) {
                    Some(m) => m,
                    None => continue,
                };

                if media_item.media_type == MediaType::Image {
                    continue;
                }

                let pcm = match self.get_audio_pcm(&media_item.file_path) {
                    Some(p) => p,
                    None => continue,
                };

                if pcm.is_empty() {
                    continue;
                }

                let overlap_start = t_start.max(clip.timeline_in);
                let overlap_end = t_end.min(clip.timeline_out);
                if overlap_end <= overlap_start {
                    continue;
                }

                let dst_start = ((overlap_start - t_start) * SAMPLE_RATE as f64).round() as usize;
                let dst_end = ((overlap_end - t_start) * SAMPLE_RATE as f64).round() as usize;
                let chunk_len = dst_end.saturating_sub(dst_start);
                if chunk_len == 0 {
                    continue;
                }

                let src_start = clip.map_timeline_to_source(overlap_start);
                let src_end = clip.map_timeline_to_source(overlap_end);
                let s_min = src_start.min(src_end);
                let s_max = src_start.max(src_end);

                let idx_min = (s_min * SAMPLE_RATE as f64).round() as isize;
                let idx_max = (s_max * SAMPLE_RATE as f64).round() as isize;

                let pcm_len = pcm.len() as isize;
                if idx_min >= pcm_len || idx_max <= 0 {
                    continue;
                }

                let c_min = 0.max(idx_min.min(pcm_len)) as usize;
                let c_max = 0.max(idx_max.min(pcm_len)) as usize;
                if c_max <= c_min {
                    continue;
                }

                let mut source_chunk: Vec<[f32; 2]> = pcm[c_min..c_max]
                    .iter()
                    .map(|&[l, r]| [l as f32, r as f32])
                    .collect();

                if clip.reverse {
                    source_chunk.reverse();
                }

                // Resample if length differs due to speed or rounding
                let resampled_chunk: Vec<[f32; 2]> = if source_chunk.len() == chunk_len {
                    source_chunk
                } else if source_chunk.len() >= 2 && chunk_len >= 2 {
                    let mut res = Vec::with_capacity(chunk_len);
                    let step = (source_chunk.len() - 1) as f64 / (chunk_len - 1) as f64;
                    for k in 0..chunk_len {
                        let idx = (k as f64 * step).min((source_chunk.len() - 1) as f64);
                        let i0 = idx.floor() as usize;
                        let i1 = (i0 + 1).min(source_chunk.len() - 1);
                        let frac = (idx - i0 as f64) as f32;
                        let l = source_chunk[i0][0]
                            + (source_chunk[i1][0] - source_chunk[i0][0]) * frac;
                        let r = source_chunk[i0][1]
                            + (source_chunk[i1][1] - source_chunk[i0][1]) * frac;
                        res.push([l, r]);
                    }
                    res
                } else {
                    vec![source_chunk.first().copied().unwrap_or([0.0, 0.0]); chunk_len]
                };

                let eff_vol = (clip.volume as f32) * track_vol;
                let has_fade = clip.fade_in > 0.0 || clip.fade_out > 0.0;

                for (k, &[l, r]) in resampled_chunk.iter().enumerate() {
                    let dst_idx = dst_start + k;
                    if dst_idx >= mixed.len() {
                        break;
                    }

                    let gain = if has_fade {
                        let cur_t = overlap_start + (k as f64 / SAMPLE_RATE as f64);
                        let dt = (cur_t - clip.timeline_in).max(0.0);
                        let rem = (clip.timeline_out - cur_t).max(0.0);

                        let mut g = eff_vol;
                        if clip.fade_in > 0.0 && dt < clip.fade_in {
                            g *= (dt / clip.fade_in) as f32;
                        }
                        if clip.fade_out > 0.0 && rem < clip.fade_out {
                            g *= (rem / clip.fade_out) as f32;
                        }
                        g
                    } else {
                        eff_vol
                    };

                    mixed[dst_idx][0] += l * gain;
                    mixed[dst_idx][1] += r * gain;
                }
            }
        }

        // Clip to i16 and calculate RMS
        let mut interleaved = Vec::with_capacity(num_samples * 2);
        let mut sum_sq_l = 0.0f64;
        let mut sum_sq_r = 0.0f64;

        for &[l, r] in &mixed {
            let cl_l = l.clamp(-32768.0, 32767.0) as i16;
            let cl_r = r.clamp(-32768.0, 32767.0) as i16;
            interleaved.push(cl_l);
            interleaved.push(cl_r);

            let norm_l = cl_l as f64 / 32768.0;
            let norm_r = cl_r as f64 / 32768.0;
            sum_sq_l += norm_l * norm_l;
            sum_sq_r += norm_r * norm_r;
        }

        let rms_l = (sum_sq_l / num_samples as f64).sqrt();
        let rms_r = (sum_sq_r / num_samples as f64).sqrt();

        let db_l = if rms_l > 1e-4 {
            (-60.0f32).max((20.0 * (rms_l as f32).log10()).min(0.0))
        } else {
            -60.0
        };

        let db_r = if rms_r > 1e-4 {
            (-60.0f32).max((20.0 * (rms_r as f32).log10()).min(0.0))
        } else {
            -60.0
        };

        (interleaved, db_l, db_r)
    }

    pub fn clear_cache(&mut self) {
        if let Ok(mut c) = self.pcm_cache.write() {
            c.clear();
        }
    }
}

fn decode_audio_to_pcm(path: &Path) -> Option<Vec<[i16; 2]>> {
    let mut cmd = crate::ffmpeg::ffmpeg_cmd();
    cmd.args([
        "-v",
        "quiet",
        "-y",
        "-i",
        path.to_str().unwrap_or(""),
        "-vn",
        "-f",
        "s16le",
        "-ac",
        "2",
        "-ar",
        &SAMPLE_RATE.to_string(),
        "pipe:1",
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::null());

    let mut guard = crate::ffmpeg::ChildGuard::spawn(&mut cmd).ok()?;

    let mut raw_bytes = Vec::new();
    if let Some(mut stdout) = guard.take_stdout() {
        let _ = stdout.read_to_end(&mut raw_bytes);
    }
    let status = guard.wait().ok()?;

    if !status.success() || raw_bytes.len() < 4 {
        return None;
    }

    let num_pairs = raw_bytes.len() / 4;
    let mut pcm = Vec::with_capacity(num_pairs);
    for chunk in raw_bytes.as_chunks::<4>().0 {
        let left = i16::from_le_bytes([chunk[0], chunk[1]]);
        let right = i16::from_le_bytes([chunk[2], chunk[3]]);
        pcm.push([left, right]);
    }

    Some(pcm)
}

/// Linux native audio stream player writing 48kHz 16-bit stereo PCM via PipeWire/ALSA.
pub struct AudioStreamPlayer {
    child: Option<crate::ffmpeg::ChildGuard>,
}

impl Default for AudioStreamPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioStreamPlayer {
    pub fn new() -> Self {
        Self { child: None }
    }

    fn ensure_started(&mut self) -> Option<&mut std::process::ChildStdin> {
        if self.child.is_none() {
            let spawned = if cfg!(windows) {
                // Windows: ffplay with CREATE_NO_WINDOW
                crate::ffmpeg::ChildGuard::spawn(
                    crate::ffmpeg::ffplay_cmd()
                        .args([
                            "-nodisp",
                            "-autoexit",
                            "-loglevel",
                            "quiet",
                            "-f",
                            "s16le",
                            "-ar",
                            "48000",
                            "-ac",
                            "2",
                            "-i",
                            "-",
                        ])
                        .stdin(Stdio::piped())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null()),
                )
            } else {
                // Linux: PipeWire pw-cat (20ms) -> ALSA aplay -> ffplay
                let mut pw_cmd = Command::new("pw-cat");
                pw_cmd
                    .args([
                        "-p",
                        "--format=s16",
                        "--rate=48000",
                        "--channels=2",
                        "--latency=20ms",
                        "-",
                    ])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());

                crate::ffmpeg::ChildGuard::spawn(&mut pw_cmd)
                    .or_else(|_| {
                        let mut aplay_cmd = Command::new("aplay");
                        aplay_cmd
                            .args([
                                "-q",
                                "-t",
                                "raw",
                                "-c",
                                "2",
                                "-f",
                                "S16_LE",
                                "-r",
                                "48000",
                                "--buffer-time=25000",
                                "--period-time=5000",
                                "-",
                            ])
                            .stdin(Stdio::piped())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null());
                        crate::ffmpeg::ChildGuard::spawn(&mut aplay_cmd)
                    })
                    .or_else(|_| {
                        let mut ffplay = crate::ffmpeg::ffplay_cmd();
                        ffplay
                            .args([
                                "-nodisp",
                                "-autoexit",
                                "-loglevel",
                                "quiet",
                                "-f",
                                "s16le",
                                "-ar",
                                "48000",
                                "-ac",
                                "2",
                                "-i",
                                "-",
                            ])
                            .stdin(Stdio::piped())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null());
                        crate::ffmpeg::ChildGuard::spawn(&mut ffplay)
                    })
            };

            if let Ok(c) = spawned {
                self.child = Some(c);
            }
        }

        self.child.as_mut().and_then(|c| c.stdin.as_mut())
    }

    /// Pre-spawns audio output process to eliminate startup delay upon clicking play.
    pub fn prepare(&mut self) {
        let _ = self.ensure_started();
    }

    /// Writes raw 16-bit stereo PCM samples to the active audio device.
    pub fn write_samples(&mut self, samples: &[i16]) {
        if samples.is_empty() {
            return;
        }

        if let Some(stdin) = self.ensure_started() {
            let mut byte_buf = Vec::with_capacity(samples.len() * 2);
            for &s in samples {
                byte_buf.extend_from_slice(&s.to_le_bytes());
            }

            if stdin.write_all(&byte_buf).is_err() {
                self.stop();
            }
        }
    }

    /// Stops audio playback immediately and kills the child audio process.
    pub fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
        }
    }
}

impl Drop for AudioStreamPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_mixer_empty_timeline() {
        let mut mixer = AudioMixer::new();
        let project = Project::new("Empty");
        let (samples, db_l, db_r) = mixer.render_audio_interval(&project, 0.0, 1.0);
        assert_eq!(samples.len(), SAMPLE_RATE as usize * 2);
        // All zero samples
        assert!(samples.iter().all(|&s| s == 0));
        assert_eq!(db_l, -60.0);
        assert_eq!(db_r, -60.0);
    }

    #[test]
    fn test_audio_stream_player_lifecycle() {
        let mut player = AudioStreamPlayer::new();
        player.write_samples(&[]);
        assert!(player.child.is_none());
        player.stop();
        assert!(player.child.is_none());
    }

    #[test]
    fn test_audio_mixer_pcm_cache_bounded() {
        let mixer = AudioMixer::new();
        // Insert a 256MB synthetic buffer
        let samples_count = 64 * 1024 * 1024; // 64M stereo pairs = 64M * 4 bytes = 256MB
        let buf = Arc::new(vec![[0i16, 0i16]; samples_count]);
        {
            let mut cache = mixer.pcm_cache.write().unwrap();
            cache.insert_with_size(
                "file1.wav".to_string(),
                Some(buf.clone()),
                samples_count * 4,
            );
            assert_eq!(cache.len(), 1);
            assert_eq!(cache.total_bytes(), 256 * 1024 * 1024);
        }

        // Insert another 256MB synthetic buffer -> 512MB total
        {
            let mut cache = mixer.pcm_cache.write().unwrap();
            cache.insert_with_size(
                "file2.wav".to_string(),
                Some(buf.clone()),
                samples_count * 4,
            );
            assert_eq!(cache.len(), 2);
            assert_eq!(cache.total_bytes(), 512 * 1024 * 1024);
        }

        // Insert a third buffer (64MB) -> exceeds 512MB limit, evicts least recently used ("file1.wav")
        let small_count = 16 * 1024 * 1024; // 64MB
        let small_buf = Arc::new(vec![[0i16, 0i16]; small_count]);
        {
            let mut cache = mixer.pcm_cache.write().unwrap();
            cache.insert_with_size("file3.wav".to_string(), Some(small_buf), small_count * 4);
            assert_eq!(cache.len(), 2);
            assert!(!cache.contains_key(&"file1.wav".to_string()));
            assert!(cache.contains_key(&"file2.wav".to_string()));
            assert!(cache.contains_key(&"file3.wav".to_string()));
            assert_eq!(cache.total_bytes(), 320 * 1024 * 1024);
        }
    }
}
