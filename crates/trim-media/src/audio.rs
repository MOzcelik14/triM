//! Audio decoding, multi-track PCM mixing, lookahead brickwall limiting, and dual VU meter RMS level analysis.

pub use crate::dsp::limiter::LookaheadLimiter;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, RwLock};
use trim_core::lru::LruCache;
use trim_core::media::MediaType;
use trim_core::project::Project;

pub const SAMPLE_RATE: u32 = 48000;
pub const MAX_PCM_CACHE_BYTES: usize = 512 * 1024 * 1024; // 512 MB hard cap

pub type PcmAudioBuffer = Option<Arc<Vec<[i16; 2]>>>;
pub type PcmCache = Arc<RwLock<LruCache<String, PcmAudioBuffer>>>;

#[derive(Clone)]
pub struct AudioMixer {
    pcm_cache: PcmCache,
    pub limiter: LookaheadLimiter,
    last_mixed_end_time: Option<f64>,
    mix_scratch: Vec<[f32; 2]>,
    prime_scratch: Vec<[f32; 2]>,
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for AudioMixer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioMixer")
            .field("limiter", &self.limiter)
            .finish_non_exhaustive()
    }
}

impl AudioMixer {
    pub fn new() -> Self {
        let limiter = LookaheadLimiter::new(SAMPLE_RATE);
        let lookahead_len = limiter.lookahead_samples();
        Self {
            pcm_cache: Arc::new(RwLock::new(LruCache::with_byte_limit(MAX_PCM_CACHE_BYTES))),
            limiter,
            last_mixed_end_time: None,
            // Pre-allocate 1 second (48,000 stereo frames) to avoid hot-path allocations during playback ticks
            mix_scratch: vec![[0.0, 0.0]; SAMPLE_RATE as usize],
            prime_scratch: vec![[0.0, 0.0]; lookahead_len],
        }
    }

    /// Resets the master lookahead limiter state and continuity tracking (e.g., on pause or seek).
    pub fn reset_limiter(&mut self) {
        self.limiter.reset();
        self.last_mixed_end_time = None;
    }

    /// Inserts pre-decoded PCM stereo frames directly into the bounded cache (used by tests and synth generators).
    pub fn insert_cached_pcm(&self, key: impl Into<String>, pcm: Vec<[i16; 2]>) {
        let size_bytes = pcm.len() * 4;
        if let Ok(mut c) = self.pcm_cache.write() {
            c.insert_with_size(key.into(), Some(Arc::new(pcm)), size_bytes);
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

    /// Sums raw normalized `f32` (`-1.0..=1.0` nominal per track) stereo samples for `[w_start, w_end]`
    /// into `out` with zero heap allocations in the mixing loop.
    fn mix_raw_window(&self, project: &Project, w_start: f64, w_end: f64, out: &mut [[f32; 2]]) {
        out.fill([0.0, 0.0]);
        let num_samples = out.len();
        if num_samples == 0 || w_end <= w_start {
            return;
        }

        let flattened_tl = project.get_flattened_timeline();
        let has_solo = flattened_tl.tracks.iter().any(|t| t.solo);
        let inv_i16 = 1.0_f32 / 32768.0_f32;

        for track in &flattened_tl.tracks {
            if track.track_type != trim_core::track::TrackType::Audio {
                continue;
            }
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
                if clip.timeline_out <= w_start || clip.timeline_in >= w_end {
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

                let overlap_start = w_start.max(clip.timeline_in);
                let overlap_end = w_end.min(clip.timeline_out);
                if overlap_end <= overlap_start {
                    continue;
                }

                let dst_start = ((overlap_start - w_start) * SAMPLE_RATE as f64).round() as usize;
                let dst_end = ((overlap_end - w_start) * SAMPLE_RATE as f64)
                    .round()
                    .min(num_samples as f64) as usize;
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

                let src_len = c_max - c_min;
                let eff_vol = (clip.volume as f32) * track_vol;
                let has_vol_kfs = clip.keyframes.get("volume").is_some_and(|k| !k.is_empty());
                let has_dynamic_gain = clip.fade_in > 0.0 || clip.fade_out > 0.0 || has_vol_kfs;
                let step = if src_len >= 2 && chunk_len >= 2 {
                    (src_len - 1) as f64 / (chunk_len - 1) as f64
                } else {
                    0.0
                };

                for k in 0..chunk_len {
                    let dst_idx = dst_start + k;
                    if dst_idx >= num_samples {
                        break;
                    }

                    let (l, r) = if src_len >= 2 && chunk_len >= 2 {
                        let rel = (k as f64 * step).min((src_len - 1) as f64);
                        let eff_rel = if clip.reverse {
                            (src_len - 1) as f64 - rel
                        } else {
                            rel
                        };
                        let i0_rel = (eff_rel.floor() as usize).min(src_len - 1);
                        let i1_rel = (i0_rel + 1).min(src_len - 1);
                        let frac = (eff_rel - i0_rel as f64) as f32;
                        let s0 = pcm[c_min + i0_rel];
                        let s1 = pcm[c_min + i1_rel];
                        let l_val = (s0[0] as f32 + (s1[0] as f32 - s0[0] as f32) * frac) * inv_i16;
                        let r_val = (s0[1] as f32 + (s1[1] as f32 - s0[1] as f32) * frac) * inv_i16;
                        (l_val, r_val)
                    } else {
                        let s = pcm[c_min];
                        (s[0] as f32 * inv_i16, s[1] as f32 * inv_i16)
                    };

                    let gain = if has_dynamic_gain {
                        let cur_t = overlap_start + (k as f64 / SAMPLE_RATE as f64);
                        let dt = (cur_t - clip.timeline_in).max(0.0);
                        let rem = (clip.timeline_out - cur_t).max(0.0);

                        let dyn_vol = if has_vol_kfs {
                            (clip.get_property_at_time("volume", dt) as f32) * track_vol
                        } else {
                            eff_vol
                        };

                        let mut g = dyn_vol;
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

                    out[dst_idx][0] += l * gain;
                    out[dst_idx][1] += r * gain;
                }
            }
        }
    }

    /// Mixes multi-track audio for `[t_start, t_end]` and processes it through the persistent
    /// stereo-linked `LookaheadLimiter` with 5 ms lookahead compensation so output frames remain
    /// sample-aligned to `[t_start, t_end]` (zero AV sync drift).
    pub fn mix_timeline_chunk(
        &mut self,
        project: &Project,
        t_start: f64,
        t_end: f64,
    ) -> &[[f32; 2]] {
        if t_end <= t_start {
            return &[];
        }

        let num_samples = ((t_end - t_start) * SAMPLE_RATE as f64).round() as usize;
        if num_samples == 0 {
            return &[];
        }

        if self.mix_scratch.len() < num_samples {
            self.mix_scratch.resize(num_samples, [0.0, 0.0]);
        }

        let lookahead_samples = self.limiter.lookahead_samples();
        let lookahead_sec = lookahead_samples as f64 / SAMPLE_RATE as f64;

        let is_contiguous = self
            .last_mixed_end_time
            .is_some_and(|last_t| (t_start - last_t).abs() < 1e-3);

        if !is_contiguous {
            if self.prime_scratch.len() != lookahead_samples {
                self.prime_scratch.resize(lookahead_samples, [0.0, 0.0]);
            }
            // Prime lookahead buffer with [t_start, t_start + 5ms]
            let mut prime_buf = std::mem::take(&mut self.prime_scratch);
            self.mix_raw_window(
                project,
                t_start,
                t_start + lookahead_sec,
                &mut prime_buf[..lookahead_samples],
            );
            self.limiter.prime(&prime_buf[..lookahead_samples]);
            self.prime_scratch = prime_buf;
        }

        // Mix the lookahead-shifted window [t_start + 5ms, t_end + 5ms] and push through limiter.
        // The limiter's 5ms delay line outputs the exact time-aligned [t_start, t_end] samples.
        let mut scratch = std::mem::take(&mut self.mix_scratch);
        self.mix_raw_window(
            project,
            t_start + lookahead_sec,
            t_end + lookahead_sec,
            &mut scratch[..num_samples],
        );
        self.limiter
            .process_buffer_in_place(&mut scratch[..num_samples]);
        self.mix_scratch = scratch;
        self.last_mixed_end_time = Some(t_end);

        &self.mix_scratch[..num_samples]
    }

    /// Renders and mixes multi-track PCM audio for the interval `[t_start, t_end]`
    /// through the master `LookaheadLimiter`.
    /// Returns `(interleaved_samples_i16, left_dbfs, right_dbfs)`.
    pub fn render_audio_interval(
        &mut self,
        project: &Project,
        t_start: f64,
        t_end: f64,
    ) -> (Vec<i16>, f32, f32) {
        let limited = self.mix_timeline_chunk(project, t_start, t_end);
        let num_samples = limited.len();
        if num_samples == 0 {
            return (Vec::new(), -60.0, -60.0);
        }

        let mut interleaved = Vec::with_capacity(num_samples * 2);
        let mut sum_sq_l = 0.0_f64;
        let mut sum_sq_r = 0.0_f64;

        for &[l, r] in limited {
            let cl_l = (l * 32767.0).round().clamp(-32768.0, 32767.0) as i16;
            let cl_r = (r * 32767.0).round().clamp(-32768.0, 32767.0) as i16;
            interleaved.push(cl_l);
            interleaved.push(cl_r);

            let norm_l = l as f64;
            let norm_r = r as f64;
            sum_sq_l += norm_l * norm_l;
            sum_sq_r += norm_r * norm_r;
        }

        let rms_l = (sum_sq_l / num_samples as f64).sqrt();
        let rms_r = (sum_sq_r / num_samples as f64).sqrt();

        let db_l = if rms_l > 1e-4 {
            (-60.0_f32).max((20.0 * (rms_l as f32).log10()).min(0.0))
        } else {
            -60.0
        };

        let db_r = if rms_r > 1e-4 {
            (-60.0_f32).max((20.0 * (rms_r as f32).log10()).min(0.0))
        } else {
            -60.0
        };

        (interleaved, db_l, db_r)
    }

    pub fn clear_cache(&mut self) {
        if let Ok(mut c) = self.pcm_cache.write() {
            c.clear();
        }
        self.reset_limiter();
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

/// Cross-platform audio stream player writing 48kHz 16-bit stereo PCM.
/// - Linux: PipeWire (`pw-cat`) -> PulseAudio (`paplay`) -> ALSA (`aplay`) -> `ffplay`
/// - Windows: `ffplay.exe` (`CREATE_NO_WINDOW`, low-delay WASAPI/DirectSound output) with silent fallback
pub struct AudioStreamPlayer {
    child: Option<crate::ffmpeg::ChildGuard>,
    backend_unavailable: bool,
}

impl Default for AudioStreamPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioStreamPlayer {
    pub fn new() -> Self {
        Self {
            child: None,
            backend_unavailable: false,
        }
    }

    #[cfg(target_os = "windows")]
    fn spawn_platform_player() -> std::io::Result<crate::ffmpeg::ChildGuard> {
        let mut ffplay = crate::ffmpeg::ffplay_cmd();
        ffplay
            .args([
                "-nodisp",
                "-autoexit",
                "-loglevel",
                "quiet",
                "-fflags",
                "nobuffer",
                "-flags",
                "low_delay",
                "-f",
                "s16le",
                "-ar",
                "48000",
                "-ac",
                "2",
                "-i",
                "pipe:0",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        crate::ffmpeg::ChildGuard::spawn(&mut ffplay)
    }

    #[cfg(not(target_os = "windows"))]
    fn spawn_platform_player() -> std::io::Result<crate::ffmpeg::ChildGuard> {
        let mut pw_cmd = crate::ffmpeg::create_command("pw-cat");
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
                let mut paplay_cmd = crate::ffmpeg::create_command("paplay");
                paplay_cmd
                    .args([
                        "--raw",
                        "--format=s16le",
                        "--rate=48000",
                        "--channels=2",
                        "--latency-msec=25",
                    ])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                crate::ffmpeg::ChildGuard::spawn(&mut paplay_cmd)
            })
            .or_else(|_| {
                let mut aplay_cmd = crate::ffmpeg::create_command("aplay");
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
    }

    fn ensure_started(&mut self) -> Option<&mut std::process::ChildStdin> {
        if self.child.is_none() && !self.backend_unavailable {
            match Self::spawn_platform_player() {
                Ok(c) => {
                    self.child = Some(c);
                }
                Err(_) => {
                    // Prevent repeated process-spawn attempts during the same playback session
                    self.backend_unavailable = true;
                }
            }
        }

        self.child.as_mut().and_then(|c| c.stdin.as_mut())
    }

    /// Pre-spawns audio output process to eliminate startup delay upon clicking play.
    pub fn prepare(&mut self) {
        self.backend_unavailable = false;
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
        self.backend_unavailable = false;
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
    use std::f32::consts::TAU;
    use trim_core::clip::Clip;
    use trim_core::media::MediaItem;
    use trim_core::track::{Track, TrackType};

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

    #[test]
    fn test_audio_mixer_multitrack_overdrive_brickwall_limited() {
        let mut mixer = AudioMixer::new();
        let ceiling = mixer.limiter.ceiling();

        // Create a synthetic 0 dBFS stereo sine wave (L = 1.0, R = 0.5) of 0.2s (9600 frames)
        let pcm_frames: Vec<[i16; 2]> = (0..9600)
            .map(|i| {
                let s = (TAU * 440.0 * (i as f32 / SAMPLE_RATE as f32)).sin();
                [(s * 32767.0) as i16, (s * 16383.5) as i16]
            })
            .collect();

        let fake_path = "/tmp/synthetic_overdrive_test.wav";
        mixer.insert_cached_pcm(fake_path, pcm_frames);

        let mut project = Project::new("OverdriveTest");
        let mut media = MediaItem::new(fake_path);
        media.media_type = MediaType::Audio;
        media.duration = 0.2;
        let media_id = media.id.clone();
        project.add_media(media);

        // Add 2 full-scale audio tracks (summing to +6 dBFS = 2.0x overdrive on Left, 1.0x on Right)
        project.timeline.tracks.clear();
        for idx in 0..2 {
            let mut tr = Track::new(format!("A{}", idx + 1), TrackType::Audio);
            tr.add_clip(Clip::new(&media_id, 0.0, 0.2));
            project.timeline.add_track(tr);
        }

        // Mix in two contiguous 50ms chunks to verify seamless lookahead continuity and AV sync
        let chunk1 = mixer.mix_timeline_chunk(&project, 0.0, 0.05).to_vec();
        let chunk2 = mixer.mix_timeline_chunk(&project, 0.05, 0.10).to_vec();

        for frame in chunk1.iter().chain(chunk2.iter()) {
            let peak = frame[0].abs().max(frame[1].abs());
            assert!(
                peak <= ceiling + 1e-5,
                "Mixed peak {} exceeded -0.1 dBFS ceiling {}",
                peak,
                ceiling
            );
            // Verify 2:1 Left/Right stereo balance is preserved under gain reduction
            assert!(
                (frame[0] - 2.0 * frame[1]).abs() < 1e-3,
                "Stereo balance distorted: L={}, R={}",
                frame[0],
                frame[1],
            );
            if frame[0].abs() > 0.05 {
                let ratio = frame[0] / frame[1];
                assert!(
                    (ratio - 2.0).abs() < 5e-3,
                    "Stereo balance distorted: L={}, R={}, ratio={}",
                    frame[0],
                    frame[1],
                    ratio
                );
            }
        }
    }
}
