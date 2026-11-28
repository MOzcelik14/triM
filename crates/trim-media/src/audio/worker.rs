use super::{AudioMixer, AudioStreamPlayer, SAMPLE_RATE};
use std::sync::{Arc, Condvar, Mutex};
use trim_core::project::Project;

const DEFAULT_SINK_LATENCY_SECS: f64 = 0.025;
const WORKER_CHUNK_SECS: f64 = 0.020;

/// Plans the next timeline audio interval `(lo_t, hi_t, reverse_dir)` for a playback tick.
pub fn plan_audio_chunk(
    cur_t: f64,
    speed: f64,
    chunk_secs: f64,
    timeline_dur: f64,
) -> Option<(f64, f64, bool)> {
    if speed.abs() <= 0.05 || chunk_secs <= 0.0 || timeline_dur <= 0.0 {
        return None;
    }
    let delta = chunk_secs * speed;
    if speed >= 0.0 {
        if cur_t >= timeline_dur {
            return None;
        }
        let next_t = (cur_t + delta).min(timeline_dur);
        if next_t <= cur_t + 1e-6 {
            return None;
        }
        Some((cur_t, next_t, false))
    } else {
        if cur_t <= 0.0 {
            return None;
        }
        let next_t = (cur_t + delta).max(0.0);
        if cur_t <= next_t + 1e-6 {
            return None;
        }
        Some((next_t, cur_t, true))
    }
}

/// Applies reverse swap, JKL speed resampling, and master volume scaling to a mixed PCM chunk.
pub fn process_playback_chunk(
    mut samples: Vec<i16>,
    mut db_l: f32,
    mut db_r: f32,
    reverse_dir: bool,
    abs_speed: f64,
    master_vol: f32,
) -> (Vec<i16>, f32, f32) {
    if reverse_dir && samples.len() >= 2 {
        let pairs = samples.len() / 2;
        for idx in 0..(pairs / 2) {
            let opp = pairs - 1 - idx;
            samples.swap(idx * 2, opp * 2);
            samples.swap(idx * 2 + 1, opp * 2 + 1);
        }
    }
    if (abs_speed - 1.0).abs() > 0.05 && samples.len() >= 4 {
        let src_frames = samples.len() / 2;
        let dst_frames = ((src_frames as f64) / abs_speed).round().max(1.0) as usize;
        let mut resampled = Vec::with_capacity(dst_frames * 2);
        for df in 0..dst_frames {
            let sf = ((df as f64) * abs_speed)
                .floor()
                .min((src_frames - 1) as f64) as usize;
            resampled.push(samples[sf * 2]);
            resampled.push(samples[sf * 2 + 1]);
        }
        samples = resampled;
    }
    if (master_vol - 1.0).abs() > 1e-4 {
        for s in &mut samples {
            *s = ((*s as f32) * master_vol)
                .round()
                .clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        }
        if master_vol <= 0.0001 {
            db_l = -60.0;
            db_r = -60.0;
        } else {
            let gain_db = 20.0 * master_vol.log10();
            db_l = (db_l + gain_db).clamp(-60.0, 0.0);
            db_r = (db_r + gain_db).clamp(-60.0, 0.0);
        }
    }
    (samples, db_l, db_r)
}

/// Computes master playhead position from stereo frames written to the audio sink.
pub fn compute_audio_clock_time(
    base_time: f64,
    speed: f64,
    frames_written: u64,
    sample_rate: u32,
    sink_latency_secs: f64,
) -> f64 {
    if sample_rate == 0 || frames_written == 0 {
        return base_time;
    }
    let elapsed_audio = ((frames_written as f64 / sample_rate as f64) - sink_latency_secs).max(0.0);
    (base_time + elapsed_audio * speed).max(0.0)
}

struct AudioWorkerState {
    active: bool,
    generation: u64,
    project: Option<Project>,
    cursor_time: f64,
    base_time: f64,
    speed: f64,
    master_volume: f32,
    frames_written: u64,
    sink_alive: bool,
    vu_l: f32,
    vu_r: f32,
    shutdown: bool,
}

/// Dedicated background audio mixing & sink worker so `AudioMixer::render_audio_interval`
/// never blocks the UI thread and playback can use the audio sink as the master clock.
pub struct AudioPlaybackWorker {
    shared: Arc<(Mutex<AudioWorkerState>, Condvar)>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl AudioPlaybackWorker {
    pub fn new(mut mixer: AudioMixer) -> Self {
        let shared = Arc::new((
            Mutex::new(AudioWorkerState {
                active: false,
                generation: 0,
                project: None,
                cursor_time: 0.0,
                base_time: 0.0,
                speed: 1.0,
                master_volume: 1.0,
                frames_written: 0,
                sink_alive: false,
                vu_l: -60.0,
                vu_r: -60.0,
                shutdown: false,
            }),
            Condvar::new(),
        ));
        let shared_worker = Arc::clone(&shared);
        let handle = std::thread::Builder::new()
            .name("audio-playback-worker".to_string())
            .spawn(move || {
                let mut player = AudioStreamPlayer::new();
                let mut last_gen = 0u64;
                loop {
                    let (gen, project_opt, cur_t, speed, master_vol) = {
                        let (lock, cv) = &*shared_worker;
                        let Ok(mut st) = lock.lock() else { break };
                        while !st.shutdown && !st.active {
                            if st.generation != last_gen {
                                last_gen = st.generation;
                                drop(st);
                                player.stop();
                                mixer.reset_limiter();
                                st = match lock.lock() {
                                    Ok(g) => g,
                                    Err(_) => return,
                                };
                            }
                            if st.shutdown {
                                break;
                            }
                            st = match cv.wait(st) {
                                Ok(g) => g,
                                Err(_) => return,
                            };
                        }
                        if st.shutdown {
                            break;
                        }
                        (
                            st.generation,
                            st.project.clone(),
                            st.cursor_time,
                            st.speed,
                            st.master_volume,
                        )
                    };

                    if gen != last_gen {
                        player.stop();
                        player.prepare();
                        mixer.reset_limiter();
                        last_gen = gen;
                    }

                    let Some(project) = project_opt else {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        continue;
                    };

                    let dur = project.timeline.duration();
                    let Some((lo_t, hi_t, reverse_dir)) =
                        plan_audio_chunk(cur_t, speed, WORKER_CHUNK_SECS, dur)
                    else {
                        if let Ok(mut st) = shared_worker.0.lock() {
                            if st.generation == gen {
                                st.active = false;
                            }
                        }
                        continue;
                    };

                    let (raw_samples, db_l, db_r) =
                        mixer.render_audio_interval(&project, lo_t, hi_t);
                    let (samples, out_l, out_r) = process_playback_chunk(
                        raw_samples,
                        db_l,
                        db_r,
                        reverse_dir,
                        speed.abs(),
                        master_vol,
                    );
                    let written_frames = (samples.len() / 2) as u64;
                    let sink_ok = player.write_samples_checked(&samples);

                    if let Ok(mut st) = shared_worker.0.lock() {
                        if st.generation == gen && st.active {
                            st.cursor_time = if reverse_dir { lo_t } else { hi_t };
                            if sink_ok {
                                st.frames_written =
                                    st.frames_written.saturating_add(written_frames);
                                st.sink_alive = true;
                            } else {
                                st.sink_alive = false;
                            }
                            st.vu_l = out_l;
                            st.vu_r = out_r;
                        }
                    }

                    // Pace the worker near real-time (20ms chunks) while keeping ~20ms lead
                    std::thread::sleep(std::time::Duration::from_millis(18));
                }
                player.stop();
            })
            .ok();

        Self { shared, handle }
    }

    pub fn start(&self, project: Project, start_time: f64, speed: f64, master_volume: f32) {
        let (lock, cv) = &*self.shared;
        if let Ok(mut st) = lock.lock() {
            st.generation = st.generation.wrapping_add(1);
            st.active = true;
            st.project = Some(project);
            st.cursor_time = start_time;
            st.base_time = start_time;
            st.speed = speed;
            st.master_volume = master_volume;
            st.frames_written = 0;
            st.sink_alive = false;
            st.vu_l = -60.0;
            st.vu_r = -60.0;
            cv.notify_all();
        }
    }

    pub fn update_speed_and_volume(&self, speed: f64, master_volume: f32) {
        let (lock, _) = &*self.shared;
        if let Ok(mut st) = lock.lock() {
            if st.active {
                if (st.speed - speed).abs() > 1e-6 {
                    let current_clock = compute_audio_clock_time(
                        st.base_time,
                        st.speed,
                        st.frames_written,
                        SAMPLE_RATE,
                        DEFAULT_SINK_LATENCY_SECS,
                    );
                    st.base_time = current_clock;
                    st.cursor_time = current_clock;
                    st.frames_written = 0;
                    st.speed = speed;
                }
                st.master_volume = master_volume;
            }
        }
    }

    pub fn stop(&self) {
        let (lock, cv) = &*self.shared;
        if let Ok(mut st) = lock.lock() {
            st.active = false;
            st.generation = st.generation.wrapping_add(1);
            st.sink_alive = false;
            st.frames_written = 0;
            cv.notify_all();
        }
    }

    /// Returns master audio clock time in seconds when the audio sink is actively consuming PCM.
    pub fn audio_clock_time(&self) -> Option<f64> {
        let (lock, _) = &*self.shared;
        let st = lock.lock().ok()?;
        if !st.active || !st.sink_alive || st.frames_written == 0 {
            return None;
        }
        Some(compute_audio_clock_time(
            st.base_time,
            st.speed,
            st.frames_written,
            SAMPLE_RATE,
            DEFAULT_SINK_LATENCY_SECS,
        ))
    }

    pub fn vu_levels(&self) -> Option<(f32, f32)> {
        let (lock, _) = &*self.shared;
        let st = lock.lock().ok()?;
        if !st.active {
            return None;
        }
        Some((st.vu_l, st.vu_r))
    }
}

impl Drop for AudioPlaybackWorker {
    fn drop(&mut self) {
        let (lock, cv) = &*self.shared;
        if let Ok(mut st) = lock.lock() {
            st.shutdown = true;
            st.active = false;
            cv.notify_all();
        }
        if let Some(h) = self.handle.take() {
            for _ in 0..25 {
                if h.is_finished() {
                    let _ = h.join();
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            if h.is_finished() {
                let _ = h.join();
            }
        }
    }
}
