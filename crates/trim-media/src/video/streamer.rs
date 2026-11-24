use image::{ImageBuffer, Rgba, RgbaImage};
use std::io::Read;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex, RwLock};
use trim_core::lru::LruCache;

pub const PREVIEW_WIDTH: u32 = 640;
pub const PREVIEW_HEIGHT: u32 = 360;
pub const PREVIEW_FRAME_BYTES: usize = (PREVIEW_WIDTH * PREVIEW_HEIGHT * 4) as usize;

/// Calculates canvas preview dimensions scaled to fit the monitor while preserving aspect ratio.
pub fn calculate_preview_size(width: u32, height: u32) -> (u32, u32) {
    let w = width.max(16);
    let h = height.max(16);
    if w >= h {
        let cw = 640;
        let ch = ((640.0 * h as f64 / w as f64).round() as u32).max(180) & !1;
        (cw, ch)
    } else {
        let ch = 640;
        let cw = ((640.0 * w as f64 / h as f64).round() as u32).max(180) & !1;
        (cw, ch)
    }
}

pub type DecodedFrame = (f64, Arc<RgbaImage>);
pub type FrameCacheMap = Arc<RwLock<LruCache<(String, u64), Arc<RgbaImage>>>>;

/// Fast, single-frame raw RGBA extraction via ffmpeg pipe at 640x360 preview resolution.
pub fn decode_frame_raw<P: AsRef<Path>>(file_path: P, source_time: f64) -> Option<RgbaImage> {
    decode_frame_raw_sized(file_path, source_time, PREVIEW_WIDTH, PREVIEW_HEIGHT)
}

/// Fast, single-frame raw RGBA extraction via ffmpeg pipe at a specified preview resolution.
fn decode_frame_raw_sized_with_hwaccel(
    p_str: &str,
    source_time: f64,
    tw: u32,
    th: u32,
    hwaccel: Option<&str>,
) -> Option<RgbaImage> {
    let vf = format!(
        "scale={tw}:{th}:force_original_aspect_ratio=decrease,pad={tw}:{th}:(ow-iw)/2:(oh-ih)/2:black"
    );

    let mut cmd = crate::ffmpeg::ffmpeg_cmd();
    cmd.args(["-v", "quiet", "-y", "-protocol_whitelist", "file"]);
    if let Some(hw) = hwaccel {
        cmd.args(["-hwaccel", hw]);
    }
    cmd.args([
        "-ss",
        &format!("{:.3}", source_time.max(0.0)),
        "-i",
        p_str,
        "-vframes",
        "1",
        "-vf",
        &vf,
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgba",
        "-",
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::null());

    let mut guard = crate::ffmpeg::ChildGuard::spawn(&mut cmd).ok()?;
    let frame_bytes = (tw as usize).checked_mul(th as usize)?.checked_mul(4)?;
    let mut buf = vec![0u8; frame_bytes];
    {
        let mut stdout = guard.take_stdout()?;
        if stdout.read_exact(&mut buf).is_err() {
            let _ = guard.kill();
            return None;
        }
    }
    let _ = guard.wait();

    ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(tw, th, buf)
}

pub fn decode_frame_raw_sized<P: AsRef<Path>>(
    file_path: P,
    source_time: f64,
    target_w: u32,
    target_h: u32,
) -> Option<RgbaImage> {
    let p_str = file_path.as_ref().to_string_lossy().to_string();
    let tw = target_w.max(16);
    let th = target_h.max(16);
    if let Some(hw) = crate::ffmpeg::probe_working_hwaccel().as_ffmpeg_flag() {
        if let Some(img) =
            decode_frame_raw_sized_with_hwaccel(&p_str, source_time, tw, th, Some(hw))
        {
            return Some(img);
        }
        log::warn!(
            "hwaccel '{}' single-frame decode failed for {}; falling back to software decode",
            hw,
            p_str
        );
    }
    decode_frame_raw_sized_with_hwaccel(&p_str, source_time, tw, th, None)
}

/// Continuous background rawvideo pipe streamer for real-time preview playback
/// (measured via `trim-media/examples/perf_bench.rs`; see `docs/benchmarks/baseline_v2.3.0.md`).
pub struct PlaybackStreamer {
    active_path: Option<String>,
    active_clip_id: Option<String>,
    frame_rx: Option<Receiver<(f64, Arc<RgbaImage>)>>,
    stop_flag: Option<Arc<AtomicBool>>,
    pub(super) active_child: Option<Arc<Mutex<Option<crate::ffmpeg::ChildGuard>>>>,
    child_handle: Option<std::thread::JoinHandle<()>>,
    current_frame: Option<Arc<RgbaImage>>,
    last_pts: f64,
    pending_future_frame: Option<(f64, Arc<RgbaImage>)>,
    is_reversed: bool,
    started_at: std::time::Instant,
    frames_delivered: u64,
    frame_pool: crate::frame_pool::FrameBufferPool,
}

impl Drop for PlaybackStreamer {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Default for PlaybackStreamer {
    fn default() -> Self {
        Self {
            active_path: None,
            active_clip_id: None,
            frame_rx: None,
            stop_flag: None,
            active_child: None,
            child_handle: None,
            current_frame: None,
            last_pts: 0.0,
            pending_future_frame: None,
            is_reversed: false,
            started_at: std::time::Instant::now(),
            frames_delivered: 0,
            frame_pool: crate::frame_pool::FrameBufferPool::new(16),
        }
    }
}

impl PlaybackStreamer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.frame_rx.is_some()
    }

    pub fn is_reversed(&self) -> bool {
        self.is_reversed
    }

    pub fn active_clip_id(&self) -> Option<&str> {
        self.active_clip_id.as_deref()
    }

    pub fn last_pts(&self) -> f64 {
        self.last_pts
    }

    pub fn frames_delivered(&self) -> u64 {
        self.frames_delivered
    }

    pub fn started_at(&self) -> std::time::Instant {
        self.started_at
    }

    /// Determines if the streamer should restart given a new target PTS and reverse state.
    pub fn should_restart(&self, target_pts: f64, reverse: bool) -> bool {
        if self.is_reversed != reverse {
            return true;
        }
        // If still initializing, give it up to 1.2s grace period to decode and deliver the first frame
        if self.frames_delivered == 0 {
            return self.started_at.elapsed().as_secs_f64() >= 1.2;
        }
        // After frames are actively being delivered, restart if playback drifted by > 0.6s (user seek)
        (target_pts - self.last_pts).abs() > 0.6
    }

    /// Starts or restarts continuous video streaming for given file from `source_start_time`.
    /// When `reverse` is false, video streams forward continuously via rawvideo pipe.
    /// When `reverse` is true, video decodes backwards in small ~2.0s forward chunks reversed in RAM,
    /// completely avoiding FFmpeg's memory-bloating reverse filter and eliminating any RAM leak (<60MB max).
    pub fn start(
        &mut self,
        clip_id: &str,
        file_path: &str,
        source_start_time: f64,
        fps: f64,
        reverse: bool,
        source_in: f64,
    ) {
        self.start_sized(
            clip_id,
            file_path,
            source_start_time,
            fps,
            reverse,
            source_in,
            PREVIEW_WIDTH,
            PREVIEW_HEIGHT,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start_sized(
        &mut self,
        clip_id: &str,
        file_path: &str,
        source_start_time: f64,
        fps: f64,
        reverse: bool,
        source_in: f64,
        target_w: u32,
        target_h: u32,
    ) {
        self.stop();

        let tw = target_w.max(16);
        let th = target_h.max(16);
        let frame_bytes = (tw as usize).saturating_mul(th as usize).saturating_mul(4);

        let path_str = file_path.to_string();
        let clip_str = clip_id.to_string();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = stop_flag.clone();
        let active_child: Arc<Mutex<Option<crate::ffmpeg::ChildGuard>>> =
            Arc::new(Mutex::new(None));
        let child_holder = active_child.clone();

        self.pending_future_frame = None;
        self.current_frame = None;
        self.is_reversed = reverse;
        self.started_at = std::time::Instant::now();
        self.frames_delivered = 0;

        // Bounded queue of 60 frames (~2.0 seconds buffer)
        let (tx, rx): (SyncSender<DecodedFrame>, Receiver<DecodedFrame>) = sync_channel(60);

        let thread_path = path_str.clone();
        let thread_fps = if fps > 1.0 { fps } else { 30.0 };
        let pool = self.frame_pool.clone();

        let handle = std::thread::spawn(move || {
            let vf_str = format!(
                "fps=fps={thread_fps:.3},scale={tw}:{th}:force_original_aspect_ratio=decrease,pad={tw}:{th}:(ow-iw)/2:(oh-ih)/2:black"
            );

            if !reverse {
                // Forward streaming: continuous single FFmpeg process (hwaccel with software fallback)
                let hw_flag = crate::ffmpeg::probe_working_hwaccel().as_ffmpeg_flag();
                for hw_opt in if hw_flag.is_some() {
                    [hw_flag, None]
                } else {
                    [None, None]
                } {
                    if stop_clone.load(Ordering::Acquire) {
                        break;
                    }
                    let mut cmd = crate::ffmpeg::ffmpeg_cmd();
                    cmd.arg("-v").arg("quiet");
                    cmd.args(["-protocol_whitelist", "file"]);
                    if let Some(hw) = hw_opt {
                        cmd.args(["-hwaccel", hw]);
                    }
                    cmd.args(["-ss", &format!("{:.3}", source_start_time.max(0.0))]);
                    cmd.args([
                        "-i",
                        &thread_path,
                        "-vf",
                        &vf_str,
                        "-f",
                        "rawvideo",
                        "-pix_fmt",
                        "rgba",
                        "-",
                    ]);
                    cmd.stdout(Stdio::piped()).stderr(Stdio::null());

                    let mut guard = match crate::ffmpeg::ChildGuard::spawn(&mut cmd) {
                        Ok(g) => g,
                        Err(_) => {
                            if hw_opt.is_some() {
                                log::warn!("hwaccel stream spawn failed; falling back to software");
                                continue;
                            }
                            return;
                        }
                    };

                    let mut stdout = match guard.take_stdout() {
                        Some(s) => s,
                        None => return,
                    };

                    if let Ok(mut lock) = child_holder.lock() {
                        if stop_clone.load(Ordering::Acquire) {
                            return;
                        }
                        *lock = Some(guard);
                    }

                    let mut frame_idx = 0u64;
                    while !stop_clone.load(Ordering::Acquire) {
                        let mut buf = pool.checkout(frame_bytes);
                        if stdout.read_exact(&mut buf).is_err() {
                            pool.checkin(buf);
                            break;
                        }

                        let pts = source_start_time + (frame_idx as f64 / thread_fps);
                        frame_idx += 1;

                        if let Some(img) = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(tw, th, buf) {
                            if tx.send((pts, Arc::new(img))).is_err() {
                                if let Ok(mut lock) = child_holder.lock() {
                                    drop(lock.take());
                                }
                                return;
                            }
                        } else {
                            break;
                        }
                    }

                    if let Ok(mut lock) = child_holder.lock() {
                        drop(lock.take());
                    }

                    if frame_idx > 0 || hw_opt.is_none() || stop_clone.load(Ordering::Acquire) {
                        break;
                    }
                    log::warn!(
                        "hwaccel '{}' stream produced 0 frames for {}; falling back to software decode",
                        hw_opt.unwrap_or(""),
                        thread_path
                    );
                }
            } else {
                // Reverse streaming: chunked forward decode in 2.0s segments, reversed in memory.
                // Absolutely ZERO ffmpeg reverse filter buffering = ZERO memory leaks (<60MB max).
                let chunk_dur = 2.0;
                let mut curr_end = source_start_time;

                while !stop_clone.load(Ordering::Acquire) && curr_end > source_in + 0.005 {
                    let curr_start = (curr_end - chunk_dur).max(source_in);
                    let actual_dur = curr_end - curr_start;
                    if actual_dur <= 0.001 {
                        break;
                    }

                    let mut cmd = crate::ffmpeg::ffmpeg_cmd();
                    cmd.arg("-v").arg("quiet");
                    cmd.args(["-protocol_whitelist", "file"]);
                    cmd.args([
                        "-ss",
                        &format!("{:.3}", curr_start.max(0.0)),
                        "-t",
                        &format!("{:.3}", actual_dur),
                    ]);
                    cmd.args([
                        "-i",
                        &thread_path,
                        "-vf",
                        &vf_str,
                        "-f",
                        "rawvideo",
                        "-pix_fmt",
                        "rgba",
                        "-",
                    ]);
                    cmd.stdout(Stdio::piped()).stderr(Stdio::null());

                    let mut guard = match crate::ffmpeg::ChildGuard::spawn(&mut cmd) {
                        Ok(g) => g,
                        Err(_) => break,
                    };

                    let mut stdout = match guard.take_stdout() {
                        Some(s) => s,
                        None => {
                            break;
                        }
                    };

                    if let Ok(mut lock) = child_holder.lock() {
                        if stop_clone.load(Ordering::Acquire) {
                            break;
                        }
                        *lock = Some(guard);
                    }

                    let mut chunk_frames = Vec::new();
                    let mut frame_idx = 0u64;

                    while !stop_clone.load(Ordering::Acquire) {
                        let mut buf = pool.checkout(frame_bytes);
                        if stdout.read_exact(&mut buf).is_err() {
                            pool.checkin(buf);
                            break;
                        }

                        let pts = curr_start + (frame_idx as f64 / thread_fps);
                        frame_idx += 1;

                        if let Some(img) = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(tw, th, buf) {
                            chunk_frames.push((pts, Arc::new(img)));
                        } else {
                            break;
                        }
                    }

                    if let Ok(mut lock) = child_holder.lock() {
                        if let Some(mut g) = lock.take() {
                            if stop_clone.load(Ordering::Acquire) {
                                let _ = g.kill();
                            } else {
                                let _ = g.wait();
                            }
                        }
                    }

                    if stop_clone.load(Ordering::Acquire) {
                        break;
                    }

                    // Reverse the frames so they flow backward in time
                    chunk_frames.reverse();

                    let mut send_err = false;
                    for item in chunk_frames {
                        if stop_clone.load(Ordering::Relaxed) {
                            send_err = true;
                            break;
                        }
                        if tx.send(item).is_err() {
                            send_err = true;
                            break;
                        }
                    }

                    if send_err {
                        break;
                    }

                    curr_end = curr_start;
                }
            }
        });

        self.active_path = Some(path_str);
        self.active_clip_id = Some(clip_str);
        self.frame_rx = Some(rx);
        self.stop_flag = Some(stop_flag);
        self.active_child = Some(active_child);
        self.child_handle = Some(handle);
        self.last_pts = source_start_time;
    }

    /// Fetches the frame matching target time from the stream receiver with accurate PTS pacing.
    pub fn get_frame_for_time(&mut self, target_source_time: f64) -> Option<Arc<RgbaImage>> {
        let is_rev = self.is_reversed;
        // First check if pending future frame is now due
        if let Some((pts, _)) = &self.pending_future_frame {
            let is_due = if is_rev {
                *pts >= target_source_time - 0.005
            } else {
                *pts <= target_source_time + 0.005
            };
            if is_due {
                if let Some((p, frame)) = self.pending_future_frame.take() {
                    if let Some(old) = self.current_frame.replace(frame) {
                        self.frame_pool.recycle_arc(old);
                    }
                    self.last_pts = p;
                    self.frames_delivered += 1;
                }
            }
        }

        if self.pending_future_frame.is_none() {
            if let Some(rx) = &self.frame_rx {
                while let Ok((pts, frame)) = rx.try_recv() {
                    let is_due = if is_rev {
                        pts >= target_source_time - 0.005
                    } else {
                        pts <= target_source_time + 0.005
                    };
                    if is_due {
                        if let Some(old) = self.current_frame.replace(frame) {
                            self.frame_pool.recycle_arc(old);
                        }
                        self.last_pts = pts;
                        self.frames_delivered += 1;
                    } else {
                        // Future frame: retain until timeline reaches this timestamp
                        self.pending_future_frame = Some((pts, frame));
                        break;
                    }
                }
            }
        }
        self.current_frame.clone()
    }

    /// Returns the current frame alongside its decoded presentation timestamp (PTS).
    /// Shares the underlying `Arc<RgbaImage>` without copying pixel buffers.
    pub fn get_frame_for_time_with_pts(
        &mut self,
        target_source_time: f64,
    ) -> Option<(f64, Arc<RgbaImage>)> {
        let frame = self.get_frame_for_time(target_source_time)?;
        Some((self.last_pts, frame))
    }

    /// Returns the PTS of the currently held frame.
    pub fn current_frame_pts(&self) -> f64 {
        self.last_pts
    }

    /// Stops the background streaming process and immediately terminates any active FFmpeg child process.
    pub fn stop(&mut self) {
        if let Some(flag) = &self.stop_flag {
            flag.store(true, Ordering::Release);
        }
        if let Some(child_arc) = &self.active_child {
            if let Ok(mut lock) = child_arc.lock() {
                if let Some(mut child) = lock.take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }
        self.active_child = None;
        self.frame_rx = None;
        self.stop_flag = None;
        if let Some(handle) = self.child_handle.take() {
            let _ = handle.join();
        }
        self.active_path = None;
        self.active_clip_id = None;
        if let Some((_, old)) = self.pending_future_frame.take() {
            self.frame_pool.recycle_arc(old);
        }
        if let Some(old) = self.current_frame.take() {
            self.frame_pool.recycle_arc(old);
        }
        self.frames_delivered = 0;
    }

    /// Returns the OS process ID of the active FFmpeg child process, if currently running.
    pub fn child_pid(&self) -> Option<u32> {
        self.active_child
            .as_ref()?
            .lock()
            .ok()?
            .as_ref()
            .and_then(|g| g.id())
    }
}

pub struct SeekCommand {
    pub id: u64,
    pub path: String,
    pub time: f64,
    pub target_w: u32,
    pub target_h: u32,
}

pub struct SeekResult {
    pub id: u64,
    pub path: String,
    pub time: f64,
    pub frame: Option<Arc<RgbaImage>>,
}

/// Computes frame matching tolerance based on frame duration (`0.5 / fps`),
/// clamped to `[0.004, 0.030]` seconds so high-FPS streams never accept adjacent frames
/// and low-FPS/VFR streams remain safely bounded.
pub fn frame_tolerance_for_fps(fps: f64) -> f64 {
    let safe_fps = if fps.is_finite() && fps > 0.0 {
        fps
    } else {
        30.0
    };
    (0.5 / safe_fps).clamp(0.004, 0.030)
}

/// Selects the frame whose PTS is closest to `target_time` within `tolerance`.
pub fn select_closest_frame_within_tolerance<'a, T>(
    frames: impl Iterator<Item = &'a (f64, T)>,
    target_time: f64,
    tolerance: f64,
) -> Option<(f64, &'a T)> {
    let mut best: Option<(f64, f64, &'a T)> = None;
    for (pts, item) in frames {
        let diff = (*pts - target_time).abs();
        if diff <= tolerance {
            match best {
                Some((best_diff, _, _)) if diff >= best_diff => {}
                _ => best = Some((diff, *pts, item)),
            }
        }
    }
    best.map(|(_, pts, item)| (pts, item))
}

/// Persistent single-stream seek decoder maintaining temporal locality during timeline scrubbing.
///
/// Instead of spawning a brand-new FFmpeg process for every scrub tick (cold seek ~140ms overhead),
/// `PersistentSeekDecoder` keeps an active `PlaybackStreamer` around the scrub position. Sequential
/// forward steps are read straight from the existing stdout pipe in < 1ms, and small backward jumps
/// are served from an internal ring buffer. All processes are RAII-governed by `ChildGuard`.
pub struct PersistentSeekDecoder {
    streamer: PlaybackStreamer,
    active_path: String,
    active_w: u32,
    active_h: u32,
    active_fps: f64,
    last_requested_time: f64,
    recent_frames: std::collections::VecDeque<(f64, Arc<RgbaImage>)>,
}

impl Default for PersistentSeekDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl PersistentSeekDecoder {
    pub fn new() -> Self {
        Self {
            streamer: PlaybackStreamer::new(),
            active_path: String::new(),
            active_w: 0,
            active_h: 0,
            active_fps: 30.0,
            last_requested_time: -1.0,
            recent_frames: std::collections::VecDeque::with_capacity(32),
        }
    }

    fn push_recent(&mut self, pts: f64, frame: Arc<RgbaImage>) {
        if self.recent_frames.len() >= 30 {
            self.recent_frames.pop_front();
        }
        self.recent_frames.push_back((pts, frame));
    }

    pub fn seek(
        &mut self,
        path: &str,
        time: f64,
        target_w: u32,
        target_h: u32,
    ) -> Option<Arc<RgbaImage>> {
        let fps = if self.active_path == path && self.active_fps > 0.0 {
            self.active_fps
        } else {
            30.0
        };
        self.seek_with_fps(path, time, fps, target_w, target_h)
    }

    pub fn seek_with_fps(
        &mut self,
        path: &str,
        time: f64,
        fps: f64,
        target_w: u32,
        target_h: u32,
    ) -> Option<Arc<RgbaImage>> {
        let effective_fps = if fps.is_finite() && fps > 0.0 {
            fps
        } else {
            30.0
        };
        let tol = frame_tolerance_for_fps(effective_fps);

        // 1. Check recent ring buffer for closest frame within FPS-scaled tolerance
        if self.active_path == path && self.active_w == target_w && self.active_h == target_h {
            if let Some((_, frame)) =
                select_closest_frame_within_tolerance(self.recent_frames.iter(), time, tol)
            {
                self.last_requested_time = time;
                return Some(frame.clone());
            }
        }

        let is_same_clip =
            self.active_path == path && self.active_w == target_w && self.active_h == target_h;

        // 2. Check forward reference locality: if same clip and advancing within 1.5s forward,
        // read forward from the existing pipe without restarting FFmpeg.
        // Only return once the stream actually reaches `time` within `tol` (never return stale previous frame).
        if is_same_clip
            && time >= self.last_requested_time
            && (time - self.last_requested_time) <= 1.5
        {
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(60);
            while std::time::Instant::now() < deadline {
                if let Some((pts, frame)) = self.streamer.get_frame_for_time_with_pts(time) {
                    if (pts - time).abs() <= tol || pts >= time - tol {
                        self.last_requested_time = time;
                        self.push_recent(pts, frame.clone());
                        return Some(frame);
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }

        // 3. Jump seek: restart the persistent streamer at the new timestamp.
        self.streamer.stop();
        self.streamer.start_sized(
            "seek_stream",
            path,
            time,
            effective_fps,
            false,
            0.0,
            target_w,
            target_h,
        );
        self.active_path = path.to_string();
        self.active_w = target_w;
        self.active_h = target_h;
        self.active_fps = effective_fps;
        self.recent_frames.clear();

        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(350);
        while std::time::Instant::now() < deadline {
            if let Some((pts, frame)) = self.streamer.get_frame_for_time_with_pts(time) {
                self.last_requested_time = time;
                self.push_recent(pts, frame.clone());
                return Some(frame);
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        // 4. Safe fallback: if stream timed out, decode 1 raw frame as fallback
        if let Some(img) = decode_frame_raw_sized(path, time, target_w, target_h) {
            let arc = Arc::new(img);
            self.last_requested_time = time;
            self.push_recent(time, arc.clone());
            Some(arc)
        } else {
            None
        }
    }
}

/// Asynchronous single-frame seek worker preventing UI thread lockup during timeline scrubbing.
pub struct AsyncFrameDecoder {
    seek_tx: std::sync::mpsc::Sender<SeekCommand>,
    result_rx: Receiver<SeekResult>,
    pub latest_req_id: u64,
}

impl Default for AsyncFrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncFrameDecoder {
    pub fn new() -> Self {
        let (seek_tx, seek_rx) = std::sync::mpsc::channel::<SeekCommand>();
        let (result_tx, result_rx) = std::sync::mpsc::channel::<SeekResult>();

        std::thread::spawn(move || {
            let mut persistent = PersistentSeekDecoder::new();
            while let Ok(mut cmd) = seek_rx.recv() {
                // Request coalescing: discard stale seek requests queued during rapid dragging
                while let Ok(newer) = seek_rx.try_recv() {
                    cmd = newer;
                }

                let maybe_img = persistent.seek(&cmd.path, cmd.time, cmd.target_w, cmd.target_h);
                let res = SeekResult {
                    id: cmd.id,
                    path: cmd.path,
                    time: cmd.time,
                    frame: maybe_img,
                };
                let _ = result_tx.send(res);
            }
        });

        Self {
            seek_tx,
            result_rx,
            latest_req_id: 0,
        }
    }

    /// Issues an async seek request for `file_path` at `source_time` at default 640x360 resolution.
    pub fn request_seek(&mut self, file_path: &str, source_time: f64) -> u64 {
        self.request_seek_sized(file_path, source_time, PREVIEW_WIDTH, PREVIEW_HEIGHT)
    }

    /// Issues an async seek request for `file_path` at `source_time` at a specific preview resolution.
    pub fn request_seek_sized(
        &mut self,
        file_path: &str,
        source_time: f64,
        target_w: u32,
        target_h: u32,
    ) -> u64 {
        self.latest_req_id += 1;
        let id = self.latest_req_id;
        let _ = self.seek_tx.send(SeekCommand {
            id,
            path: file_path.to_string(),
            time: source_time,
            target_w,
            target_h,
        });
        id
    }

    /// Polls for newly completed seek frame results without blocking.
    pub fn poll_result(&mut self) -> Option<SeekResult> {
        let mut latest = None;
        while let Ok(res) = self.result_rx.try_recv() {
            latest = Some(res);
        }
        latest
    }
}
