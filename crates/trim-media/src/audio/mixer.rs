//! Audio decoding, multi-track PCM mixing, lookahead brickwall limiting, and dual VU meter RMS level analysis.

pub use crate::dsp::limiter::LookaheadLimiter;
use std::collections::HashSet;
use std::io::Read;
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use trim_core::lru::LruCache;
use trim_core::media::MediaType;
use trim_core::project::Project;

pub const SAMPLE_RATE: u32 = 48000;
pub const AUDIO_CHUNK_SECS: f64 = 10.0;
pub const MAX_PCM_CACHE_BYTES: usize = 512 * 1024 * 1024; // 512 MB hard cap

pub type PcmAudioBuffer = Option<Arc<Vec<[i16; 2]>>>;
pub type PcmCache = Arc<RwLock<LruCache<String, PcmAudioBuffer>>>;

#[derive(Clone, Default)]
struct ClipDspState {
    env_level: f32,
    hp_prev_in: [f32; 2],
    hp_prev_out: [f32; 2],
    eq_state: [[[f32; 4]; 2]; 6],
    comp_env: f32,
    deess_prev: [f32; 2],
    delay_buf: Vec<[f32; 2]>,
    delay_pos: usize,
}

#[derive(Clone)]
pub struct AudioMixer {
    pub(super) pcm_cache: PcmCache,
    in_flight: Arc<(Mutex<HashSet<String>>, Condvar)>,
    clip_dsp_states: Arc<Mutex<std::collections::HashMap<String, ClipDspState>>>,
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

struct InFlightGuard {
    key: String,
    in_flight: Arc<(Mutex<HashSet<String>>, Condvar)>,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        let (lock, cvar) = &*self.in_flight;
        if let Ok(mut set) = lock.lock() {
            set.remove(&self.key);
            cvar.notify_all();
        }
    }
}

impl AudioMixer {
    pub fn new() -> Self {
        let limiter = LookaheadLimiter::new(SAMPLE_RATE);
        let lookahead_len = limiter.lookahead_samples();
        Self {
            pcm_cache: Arc::new(RwLock::new(LruCache::with_byte_limit(MAX_PCM_CACHE_BYTES))),
            in_flight: Arc::new((Mutex::new(HashSet::new()), Condvar::new())),
            clip_dsp_states: Arc::new(Mutex::new(std::collections::HashMap::new())),
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
        if let Ok(mut states) = self.clip_dsp_states.lock() {
            states.clear();
        }
    }

    /// Dynamically sets the maximum memory limit in bytes for the PCM audio buffer cache.
    pub fn set_max_cache_bytes(&self, bytes: usize) {
        if let Ok(mut c) = self.pcm_cache.write() {
            c.set_max_bytes(bytes);
        }
    }

    /// Returns current memory footprint of decoded PCM audio in bytes.
    pub fn current_cache_bytes(&self) -> usize {
        self.pcm_cache.read().map(|c| c.total_bytes()).unwrap_or(0)
    }

    /// Inserts pre-decoded PCM stereo frames directly into the bounded cache (used by tests and synth generators).
    pub fn insert_cached_pcm(&self, key: impl Into<String>, pcm: Vec<[i16; 2]>) {
        let size_bytes = pcm.len() * 4;
        if let Ok(mut c) = self.pcm_cache.write() {
            c.insert_with_size(key.into(), Some(Arc::new(pcm)), size_bytes);
        }
    }

    /// Asynchronously preloads audio PCM via a bounded 2-worker background pool upon media import.
    pub fn preload_audio_async<P: AsRef<Path>>(&self, file_path: P) {
        type PreloadJob = (
            std::path::PathBuf,
            PcmCache,
            Arc<(Mutex<HashSet<String>>, Condvar)>,
        );
        static PRELOAD_TX: std::sync::OnceLock<std::sync::mpsc::Sender<PreloadJob>> =
            std::sync::OnceLock::new();

        let tx = PRELOAD_TX.get_or_init(|| {
            let (tx, rx) = std::sync::mpsc::channel::<PreloadJob>();
            let rx = Arc::new(Mutex::new(rx));
            for _ in 0..2 {
                let rx_clone = Arc::clone(&rx);
                std::thread::spawn(move || loop {
                    let job = {
                        let Ok(guard) = rx_clone.lock() else { break };
                        guard.recv()
                    };
                    let Ok((path, cache_clone, in_flight_clone)) = job else {
                        break;
                    };
                    let path_str = path.to_string_lossy().to_string();
                    let already_cached = cache_clone
                        .read()
                        .ok()
                        .and_then(|c| c.peek(&path_str).and_then(|v| v.as_ref()).cloned())
                        .is_some();
                    if !already_cached {
                        let _guard = {
                            let (lock, _) = &*in_flight_clone;
                            if let Ok(mut set) = lock.lock() {
                                if set.contains(&path_str) {
                                    None
                                } else {
                                    set.insert(path_str.clone());
                                    Some(InFlightGuard {
                                        key: path_str.clone(),
                                        in_flight: Arc::clone(&in_flight_clone),
                                    })
                                }
                            } else {
                                None
                            }
                        };
                        if _guard.is_some() {
                            let pcm = decode_audio_to_pcm(&path).map(Arc::new);
                            let size_bytes = pcm.as_ref().map(|a| a.len() * 4).unwrap_or(64);
                            if let Ok(mut c) = cache_clone.write() {
                                c.insert_with_size(path_str.clone(), pcm, size_bytes);
                            }
                        }
                    }
                });
            }
            tx
        });

        let path = file_path.as_ref().to_path_buf();
        let path_str = path.to_string_lossy().to_string();
        if let Ok(c) = self.pcm_cache.read() {
            if c.peek(&path_str).is_some() {
                return;
            }
        }
        let _ = tx.send((path, self.pcm_cache.clone(), self.in_flight.clone()));
    }

    /// Loads and caches 48kHz 16-bit stereo PCM for given media file.
    pub fn get_audio_pcm<P: AsRef<Path>>(&self, file_path: P) -> Option<Arc<Vec<[i16; 2]>>> {
        let path = file_path.as_ref();
        let path_str = path.to_string_lossy().to_string();
        if let Ok(mut c) = self.pcm_cache.write() {
            if let Some(cached) = c.get(&path_str) {
                return cached.clone();
            }
        }

        // Wait if another thread is decoding this exact file, then atomically claim in-flight
        // ownership under the same lock so no TOCTOU race can occur.
        let _guard = {
            let (lock, cvar) = &*self.in_flight;
            let mut set = lock.lock().ok()?;
            while set.contains(&path_str) {
                set = cvar.wait(set).unwrap_or_else(|e| e.into_inner());
            }
            if let Ok(mut c) = self.pcm_cache.write() {
                if let Some(cached) = c.get(&path_str) {
                    return cached.clone();
                }
            }
            set.insert(path_str.clone());
            InFlightGuard {
                key: path_str.clone(),
                in_flight: Arc::clone(&self.in_flight),
            }
        };

        let pcm = decode_audio_to_pcm(path).map(Arc::new);
        let size_bytes = pcm.as_ref().map(|a| a.len() * 4).unwrap_or(64);
        if let Ok(mut c) = self.pcm_cache.write() {
            c.insert_with_size(path_str, pcm.clone(), size_bytes);
        }
        pcm
    }

    /// Loads and caches a single 10-second 48kHz 16-bit stereo PCM chunk (`chunk_idx`)
    /// under key `"<path>#chunk:<idx>"` in the byte-bounded LRU cache.
    pub fn get_audio_pcm_chunk<P: AsRef<Path>>(
        &self,
        file_path: P,
        chunk_idx: u64,
    ) -> Option<Arc<Vec<[i16; 2]>>> {
        let path = file_path.as_ref();
        let path_str = path.to_string_lossy();
        let key = format!("{}#chunk:{}", path_str, chunk_idx);
        if let Ok(mut c) = self.pcm_cache.write() {
            if let Some(cached) = c.get(&key) {
                return cached.clone();
            }
        }

        let _guard = {
            let (lock, cvar) = &*self.in_flight;
            let mut set = lock.lock().ok()?;
            while set.contains(&key) {
                set = cvar.wait(set).unwrap_or_else(|e| e.into_inner());
            }
            if let Ok(mut c) = self.pcm_cache.write() {
                if let Some(cached) = c.get(&key) {
                    return cached.clone();
                }
            }
            set.insert(key.clone());
            InFlightGuard {
                key: key.clone(),
                in_flight: Arc::clone(&self.in_flight),
            }
        };

        let start_secs = chunk_idx as f64 * AUDIO_CHUNK_SECS;
        let pcm = decode_audio_chunk_to_pcm(path, start_secs, AUDIO_CHUNK_SECS).map(Arc::new);
        let size_bytes = pcm.as_ref().map(|a| a.len() * 4).unwrap_or(64);
        if let Ok(mut c) = self.pcm_cache.write() {
            c.insert_with_size(key, pcm.clone(), size_bytes);
        }
        pcm
    }

    /// Returns a PCM buffer covering `[s_min, s_max]` along with its starting global sample offset.
    /// Reuses a full-file cached entry if already present (e.g. `insert_cached_pcm`), otherwise
    /// fetches only the overlapping 10-second chunk(s) so long files stay within a small RAM budget.
    pub fn get_audio_pcm_window<P: AsRef<Path>>(
        &self,
        file_path: P,
        s_min: f64,
        s_max: f64,
    ) -> Option<(Arc<Vec<[i16; 2]>>, usize)> {
        let path = file_path.as_ref();
        let path_str = path.to_string_lossy().to_string();
        if let Ok(mut c) = self.pcm_cache.write() {
            if let Some(cached) = c.get(&path_str) {
                return cached.as_ref().map(|arc| (Arc::clone(arc), 0));
            }
        }

        let chunk_samples = (AUDIO_CHUNK_SECS * SAMPLE_RATE as f64).round() as usize;
        let start_chunk = (s_min.max(0.0) / AUDIO_CHUNK_SECS).floor() as u64;
        let end_chunk = (s_max.max(0.0) / AUDIO_CHUNK_SECS).floor() as u64;

        let first = self.get_audio_pcm_chunk(path, start_chunk)?;
        let base_offset = (start_chunk as usize).saturating_mul(chunk_samples);
        if end_chunk <= start_chunk {
            return Some((first, base_offset));
        }

        let mut stitched = (*first).clone();
        for c_idx in (start_chunk + 1)..=end_chunk {
            if let Some(next_chunk) = self.get_audio_pcm_chunk(path, c_idx) {
                stitched.extend_from_slice(&next_chunk);
            }
        }
        Some((Arc::new(stitched), base_offset))
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
            let is_audio_track = track.track_type == trim_core::track::TrackType::Audio;
            let is_video_track = track.track_type == trim_core::track::TrackType::Video;
            if !is_audio_track && !is_video_track {
                continue;
            }
            if track.muted || (is_video_track && !track.visible) {
                continue;
            }
            if has_solo && !track.solo {
                continue;
            }
            let track_vol = (track.volume * flattened_tl.bus_gains.gain_for(track.bus)) as f32;

            for clip in &track.clips {
                if clip.muted {
                    continue;
                }
                if clip.timeline_out <= w_start || clip.timeline_in >= w_end {
                    continue;
                }

                // For video tracks, only mix audio if this video clip was never given a companion
                // audio clip on an Audio track (e.g. imported before Sony ipcm audio detection).
                if is_video_track {
                    if clip.linked_group_id.is_some() {
                        continue;
                    }
                    let has_audio_companion = flattened_tl
                        .tracks
                        .iter()
                        .filter(|t| t.track_type == trim_core::track::TrackType::Audio)
                        .flat_map(|t| t.clips.iter())
                        .any(|ac| {
                            ac.media_id == clip.media_id
                                && ac.timeline_out > clip.timeline_in
                                && ac.timeline_in < clip.timeline_out
                        });
                    if has_audio_companion {
                        continue;
                    }
                }

                let media_item = match project.get_media(&clip.media_id) {
                    Some(m) => m,
                    None => continue,
                };

                if media_item.media_type == MediaType::Image {
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

                let (pcm, base_offset) =
                    match self.get_audio_pcm_window(&media_item.file_path, s_min, s_max) {
                        Some(p) if !p.0.is_empty() => p,
                        _ => continue,
                    };

                let idx_min = (s_min * SAMPLE_RATE as f64).round() as isize - base_offset as isize;
                let idx_max = (s_max * SAMPLE_RATE as f64).round() as isize - base_offset as isize;

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
                let denoise_strength = if clip.voice_denoise > 0.01 {
                    clip.voice_denoise.clamp(0.0, 1.0)
                } else if clip.noise_gate
                    || clip.eq_preset == trim_core::clip::AudioEqPreset::DenoiseGate
                {
                    0.35
                } else {
                    0.0
                };
                let step = if src_len >= 2 && chunk_len >= 2 {
                    (src_len - 1) as f64 / (chunk_len - 1) as f64
                } else {
                    0.0
                };

                let mut dsp_state = self
                    .clip_dsp_states
                    .lock()
                    .ok()
                    .and_then(|mut map| map.remove(&clip.id))
                    .unwrap_or_default();

                let mut env_level = dsp_state.env_level;
                let mut hp_prev_in = dsp_state.hp_prev_in;
                let mut hp_prev_out = dsp_state.hp_prev_out;

                // Precompute active Biquad peaking EQ coefficients (up to 6 bands)
                let mut eq_coeffs = [[0.0_f32; 5]; 6];
                let mut eq_state = dsp_state.eq_state; // [band][ch][x1, x2, y1, y2]
                let mut active_eq_count = 0usize;
                if let Some(peq) = &clip.parametric_eq {
                    if !peq.is_flat() {
                        for band in &peq.bands {
                            if active_eq_count >= eq_coeffs.len() {
                                break;
                            }
                            if !band.enabled || band.gain_db.abs() < 0.05 {
                                continue;
                            }
                            let a_amp = 10.0_f32.powf(band.gain_db.clamp(-24.0, 24.0) / 40.0);
                            let w0 = 2.0 * std::f32::consts::PI * band.freq_hz.clamp(20.0, 20000.0)
                                / SAMPLE_RATE as f32;
                            let cos_w0 = w0.cos();
                            let alpha = w0.sin() / (2.0 * band.q.clamp(0.2, 10.0));
                            let a0 = (1.0 + alpha / a_amp).max(1e-6);
                            let b0 = (1.0 + alpha * a_amp) / a0;
                            let b1 = (-2.0 * cos_w0) / a0;
                            let b2 = (1.0 - alpha * a_amp) / a0;
                            let a1 = (-2.0 * cos_w0) / a0;
                            let a2 = (1.0 - alpha / a_amp) / a0;
                            eq_coeffs[active_eq_count] = [b0, b1, b2, a1, a2];
                            active_eq_count += 1;
                        }
                    }
                }

                // Compressor / De-Esser state
                let active_comp = clip.compressor.as_ref().filter(|c| c.enabled);
                let mut comp_env = dsp_state.comp_env;
                let mut deess_prev = dsp_state.deess_prev;
                let (
                    comp_thresh,
                    comp_ratio,
                    comp_atk,
                    comp_rel,
                    comp_makeup,
                    comp_limit,
                    deess_amt,
                ) = if let Some(c) = active_comp {
                    let th = 10.0_f32.powf(c.threshold_db.clamp(-48.0, 0.0) / 20.0);
                    let rat = c.ratio.clamp(1.0, 20.0);
                    let atk = (-1.0
                        / ((c.attack_ms.clamp(1.0, 200.0) * 0.001) * SAMPLE_RATE as f32))
                        .exp();
                    let rel = (-1.0
                        / ((c.release_ms.clamp(10.0, 1000.0) * 0.001) * SAMPLE_RATE as f32))
                        .exp();
                    let mk = 10.0_f32.powf(c.makeup_db.clamp(0.0, 24.0) / 20.0);
                    (
                        th,
                        rat,
                        atk,
                        rel,
                        mk,
                        c.limiter_enabled,
                        c.de_esser.clamp(0.0, 1.0),
                    )
                } else {
                    (1.0, 1.0, 0.0, 0.0, 1.0, false, 0.0)
                };

                // Stereo pan gains (clip.pan + track.pan in [-1.0..=1.0])
                let combined_pan = (clip.pan + track.pan).clamp(-1.0, 1.0) as f32;
                let (pan_l, pan_r) = if combined_pan < 0.0 {
                    (1.0_f32, (1.0 + combined_pan).clamp(0.0, 1.0))
                } else if combined_pan > 0.0 {
                    ((1.0 - combined_pan).clamp(0.0, 1.0), 1.0_f32)
                } else {
                    (1.0_f32, 1.0_f32)
                };

                // Compact comb/echo delay line for Reverb & Echo preview
                let rev_amt = clip.reverb_amount.clamp(0.0, 1.0);
                let echo_amt = clip.echo_amount.clamp(0.0, 1.0);
                let has_space_fx = rev_amt > 0.01 || echo_amt > 0.01;
                let mut delay_buf = if has_space_fx {
                    if dsp_state.delay_buf.len() == 4800 {
                        std::mem::take(&mut dsp_state.delay_buf)
                    } else {
                        vec![[0.0_f32; 2]; 4800] // 100ms circular tap buffer
                    }
                } else {
                    Vec::new()
                };
                let mut delay_pos = if has_space_fx && !delay_buf.is_empty() {
                    dsp_state.delay_pos % delay_buf.len()
                } else {
                    0
                };

                for k in 0..chunk_len {
                    let dst_idx = dst_start + k;
                    if dst_idx >= num_samples {
                        break;
                    }

                    let (mut l, mut r) = if src_len >= 2 && chunk_len >= 2 {
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

                    if denoise_strength > 0.001 {
                        // First-order 90Hz high-pass filter to remove low-frequency rumble
                        let alpha_hp = 0.988_f32;
                        let yl = alpha_hp * (hp_prev_out[0] + l - hp_prev_in[0]);
                        let yr = alpha_hp * (hp_prev_out[1] + r - hp_prev_in[1]);
                        hp_prev_in = [l, r];
                        hp_prev_out = [yl, yr];
                        l = l + (yl - l) * denoise_strength;
                        r = r + (yr - r) * denoise_strength;

                        // Smooth downward expander / spectral noise gate
                        let peak = l.abs().max(r.abs());
                        let coeff = if peak > env_level { 0.08 } else { 0.002 };
                        env_level += coeff * (peak - env_level);
                        let thresh = 0.008 + 0.032 * denoise_strength;
                        if env_level < thresh {
                            let ratio = (env_level / thresh.max(1e-5)).clamp(0.0, 1.0);
                            let atten = ratio.powf(1.0 + 1.8 * denoise_strength);
                            l *= atten;
                            r *= atten;
                        }
                    }

                    // 6-Band Parametric Biquad EQ
                    for b_idx in 0..active_eq_count {
                        let [b0, b1, b2, a1, a2] = eq_coeffs[b_idx];
                        let st_l = &mut eq_state[b_idx][0];
                        let yl = b0 * l + b1 * st_l[0] + b2 * st_l[1] - a1 * st_l[2] - a2 * st_l[3];
                        st_l[1] = st_l[0];
                        st_l[0] = l;
                        st_l[3] = st_l[2];
                        st_l[2] = yl;
                        l = yl;

                        let st_r = &mut eq_state[b_idx][1];
                        let yr = b0 * r + b1 * st_r[0] + b2 * st_r[1] - a1 * st_r[2] - a2 * st_r[3];
                        st_r[1] = st_r[0];
                        st_r[0] = r;
                        st_r[3] = st_r[2];
                        st_r[2] = yr;
                        r = yr;
                    }

                    // Dynamic Compressor + De-Esser + Brickwall Limiter
                    if active_comp.is_some() {
                        if deess_amt > 0.01 {
                            let hf_l = l - deess_prev[0];
                            let hf_r = r - deess_prev[1];
                            deess_prev = [l, r];
                            if hf_l.abs().max(hf_r.abs()) > 0.12 {
                                l -= hf_l * deess_amt * 0.55;
                                r -= hf_r * deess_amt * 0.55;
                            }
                        }
                        let peak = l.abs().max(r.abs());
                        comp_env = if peak > comp_env {
                            comp_atk * comp_env + (1.0 - comp_atk) * peak
                        } else {
                            comp_rel * comp_env + (1.0 - comp_rel) * peak
                        };
                        let mut c_gain = comp_makeup;
                        if comp_env > comp_thresh && comp_thresh > 1e-5 {
                            let over = comp_env / comp_thresh;
                            let compressed = over.powf((1.0 / comp_ratio) - 1.0);
                            c_gain *= compressed;
                        }
                        l *= c_gain;
                        r *= c_gain;
                        if comp_limit {
                            l = l.clamp(-0.891, 0.891);
                            r = r.clamp(-0.891, 0.891);
                        }
                    }

                    // Reverb / Echo spatial feedback
                    if has_space_fx && !delay_buf.is_empty() {
                        let d_len = delay_buf.len();
                        let tap1 = delay_buf[(delay_pos + d_len - 1600) % d_len];
                        let tap2 = delay_buf[(delay_pos + d_len - 3700) % d_len];
                        let wet_l = tap1[0] * rev_amt * 0.35 + tap2[0] * echo_amt * 0.45;
                        let wet_r = tap1[1] * rev_amt * 0.35 + tap2[1] * echo_amt * 0.45;
                        delay_buf[delay_pos] = [l + wet_l * 0.35, r + wet_r * 0.35];
                        delay_pos = (delay_pos + 1) % d_len;
                        l += wet_l;
                        r += wet_r;
                    }

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

                    out[dst_idx][0] += l * gain * pan_l;
                    out[dst_idx][1] += r * gain * pan_r;
                }

                if let Ok(mut map) = self.clip_dsp_states.lock() {
                    if map.len() >= 64 && !map.contains_key(&clip.id) {
                        if let Some(evict_key) = map.keys().find(|k| *k != &clip.id).cloned() {
                            map.remove(&evict_key);
                        }
                    }
                    map.insert(
                        clip.id.clone(),
                        ClipDspState {
                            env_level,
                            hp_prev_in,
                            hp_prev_out,
                            eq_state,
                            comp_env,
                            deess_prev,
                            delay_buf,
                            delay_pos,
                        },
                    );
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

    /// Renders stereo PCM audio and VU levels for a single standalone media file (used by Source Monitor).
    pub fn render_source_media_interval(
        &self,
        file_path: &str,
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

        let Some(pcm) = self.get_audio_pcm(file_path) else {
            return (vec![0i16; num_samples * 2], -60.0, -60.0);
        };
        if pcm.is_empty() {
            return (vec![0i16; num_samples * 2], -60.0, -60.0);
        }

        let start_idx = (t_start.max(0.0) * SAMPLE_RATE as f64).round() as usize;
        let mut interleaved = Vec::with_capacity(num_samples * 2);
        let mut sum_sq_l = 0.0_f64;
        let mut sum_sq_r = 0.0_f64;
        let inv_i16 = 1.0_f64 / 32768.0_f64;

        for i in 0..num_samples {
            let idx = start_idx + i;
            let [l, r] = if idx < pcm.len() { pcm[idx] } else { [0, 0] };
            interleaved.push(l);
            interleaved.push(r);
            let nl = (l as f64) * inv_i16;
            let nr = (r as f64) * inv_i16;
            sum_sq_l += nl * nl;
            sum_sq_r += nr * nr;
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

fn run_ffmpeg_pcm_decode(
    path_str: &str,
    forced_input_codec: Option<&str>,
    stream_map: Option<&str>,
) -> Vec<[i16; 2]> {
    let mut cmd = crate::ffmpeg::ffmpeg_cmd();
    cmd.args([
        "-v",
        "error",
        "-y",
        "-ignore_unknown",
        "-err_detect",
        "ignore_err",
    ]);
    cmd.args(["-protocol_whitelist", "file"]);
    if let Some(codec) = forced_input_codec {
        cmd.args(["-c:a", codec]);
    }
    cmd.args(["-i", path_str, "-vn", "-dn", "-sn"]);
    if let Some(map_spec) = stream_map {
        cmd.args(["-map", map_spec]);
    }
    cmd.args([
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

    const MAX_PCM_DECODE_BYTES: u64 = 512 * 1024 * 1024;
    let mut pcm = Vec::new();
    if let Ok(mut guard) = crate::ffmpeg::ChildGuard::spawn(&mut cmd) {
        if let Some(mut stdout) = guard.take_stdout() {
            let mut buf = [0u8; 65536];
            let mut leftover = [0u8; 4];
            let mut leftover_len = 0usize;
            let mut total_read = 0u64;
            while total_read < MAX_PCM_DECODE_BYTES {
                let to_read = ((MAX_PCM_DECODE_BYTES - total_read) as usize).min(buf.len());
                match stdout.read(&mut buf[..to_read]) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        total_read += n as u64;
                        let mut slice = &buf[..n];
                        if leftover_len > 0 {
                            let need = 4 - leftover_len;
                            if slice.len() >= need {
                                leftover[leftover_len..4].copy_from_slice(&slice[..need]);
                                let l = i16::from_le_bytes([leftover[0], leftover[1]]);
                                let r = i16::from_le_bytes([leftover[2], leftover[3]]);
                                pcm.push([l, r]);
                                slice = &slice[need..];
                                leftover_len = 0;
                            } else {
                                leftover[leftover_len..leftover_len + slice.len()]
                                    .copy_from_slice(slice);
                                leftover_len += slice.len();
                                continue;
                            }
                        }
                        let (chunks, rem) = slice.as_chunks::<4>();
                        pcm.reserve(chunks.len());
                        for c in chunks {
                            pcm.push([
                                i16::from_le_bytes([c[0], c[1]]),
                                i16::from_le_bytes([c[2], c[3]]),
                            ]);
                        }
                        if !rem.is_empty() {
                            leftover[..rem.len()].copy_from_slice(rem);
                            leftover_len = rem.len();
                        }
                    }
                }
            }
        }
        let _ = guard.kill();
        let _ = guard.wait();
    }
    pcm
}

fn run_ffmpeg_pcm_decode_ret(
    path_str: &str,
    forced_codec: Option<&str>,
    stream_map: Option<&str>,
) -> Vec<[i16; 2]> {
    run_ffmpeg_pcm_decode(path_str, forced_codec, stream_map)
}

fn decode_audio_chunk_to_pcm(
    path: &Path,
    start_secs: f64,
    duration_secs: f64,
) -> Option<Vec<[i16; 2]>> {
    let path_str = path.to_str().unwrap_or("");
    if path_str.is_empty() || duration_secs <= 0.0 {
        return None;
    }
    let mut cmd = crate::ffmpeg::ffmpeg_cmd();
    cmd.args([
        "-v",
        "quiet",
        "-protocol_whitelist",
        "file",
        "-ss",
        &format!("{:.3}", start_secs.max(0.0)),
        "-t",
        &format!("{:.3}", duration_secs),
        "-i",
        path_str,
        "-vn",
        "-acodec",
        "pcm_s16le",
        "-f",
        "s16le",
        "-ac",
        "2",
        "-ar",
        &SAMPLE_RATE.to_string(),
        "-",
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::null());

    let mut guard = crate::ffmpeg::ChildGuard::spawn(&mut cmd).ok()?;
    let mut raw = Vec::new();
    if let Some(mut stdout) = guard.take_stdout() {
        let _ = stdout.read_to_end(&mut raw);
    }
    let _ = guard.wait();

    if raw.len() < 4 {
        return None;
    }
    let samples: Vec<[i16; 2]> = raw
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| {
            [
                i16::from_le_bytes([c[0], c[1]]),
                i16::from_le_bytes([c[2], c[3]]),
            ]
        })
        .collect();
    if samples.is_empty() {
        None
    } else {
        Some(samples)
    }
}

pub(super) fn decode_audio_to_pcm(path: &Path) -> Option<Vec<[i16; 2]>> {
    let path_str = path.to_str().unwrap_or("");
    if path_str.is_empty() {
        return None;
    }

    // 1. Check if the file has an MP4/MOV raw PCM atom (such as Sony XAVC S `ipcm` / `lpcm` / `twos` / `in24`)
    // that requires an explicit input decoder on older FFmpeg builds (e.g. Pop!_OS 22.04 FFmpeg 4.4).
    let detected_pcm = crate::analyzer::detect_forced_pcm_input_codec(path);

    let mut pcm = if let Some(info) = detected_pcm {
        let b = run_ffmpeg_pcm_decode_ret(path_str, Some(info.codec), Some("0:a:0?"));
        if !b.is_empty() {
            b
        } else {
            run_ffmpeg_pcm_decode_ret(path_str, None, Some("0:a:0?"))
        }
    } else {
        run_ffmpeg_pcm_decode_ret(path_str, None, Some("0:a:0?"))
    };

    if !pcm.is_empty() {
        return Some(pcm);
    }

    // Skip expensive multi-codec fallback when ffprobe confirms there is no audio stream
    if detected_pcm.is_none()
        && crate::analyzer::FFprobeAnalyzer::analyze(path).is_ok_and(|info| !info.has_audio())
    {
        return None;
    }

    // 2. Fallback: general audio stream selection without explicit stream map
    pcm = run_ffmpeg_pcm_decode_ret(path_str, None, None);

    // 3. Fallback: iterate over Sony / QuickTime uncompressed PCM input decoders
    if pcm.is_empty() {
        for &codec in &[
            "pcm_s16be",
            "pcm_s24be",
            "pcm_s16le",
            "pcm_s24le",
            "pcm_s32be",
            "pcm_f32be",
        ] {
            if detected_pcm.is_some_and(|d| d.codec == codec) {
                pcm = run_ffmpeg_pcm_decode_ret(path_str, Some(codec), None);
            } else {
                pcm = run_ffmpeg_pcm_decode_ret(path_str, Some(codec), Some("0:a:0?"));
                if pcm.is_empty() {
                    pcm = run_ffmpeg_pcm_decode_ret(path_str, Some(codec), None);
                }
            }
            if !pcm.is_empty() {
                break;
            }
        }
    }

    if pcm.is_empty() {
        None
    } else {
        Some(pcm)
    }
}
