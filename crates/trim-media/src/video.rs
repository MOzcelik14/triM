//! Video frame extraction, high-speed asynchronous streaming, software composition, and color grading.

use image::{ImageBuffer, Rgba, RgbaImage};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex, RwLock};
use trim_core::clip::Clip;
use trim_core::lru::LruCache;
use trim_core::media::MediaType;
use trim_core::project::Project;

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

/// Fast, single-frame raw RGBA extraction via ffmpeg pipe.
pub fn decode_frame_raw<P: AsRef<Path>>(file_path: P, source_time: f64) -> Option<RgbaImage> {
    let p = file_path.as_ref();
    let p_str = p.to_string_lossy().to_string();

    let mut cmd = crate::ffmpeg::ffmpeg_cmd();
    cmd.args([
        "-v",
        "quiet",
        "-y",
        "-ss",
        &format!("{:.3}", source_time.max(0.0)),
        "-i",
        &p_str,
        "-vframes",
        "1",
        "-vf",
        "scale=640:360:force_original_aspect_ratio=decrease,pad=640:360:(ow-iw)/2:(oh-ih)/2:black",
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgba",
        "-",
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::null());

    let mut guard = crate::ffmpeg::ChildGuard::spawn(&mut cmd).ok()?;

    let mut buf = vec![0u8; PREVIEW_FRAME_BYTES];
    if let Some(mut stdout) = guard.take_stdout() {
        if stdout.read_exact(&mut buf).is_err() {
            let _ = guard.kill();
            return None;
        }
    }
    let _ = guard.wait();

    ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(PREVIEW_WIDTH, PREVIEW_HEIGHT, buf)
}

/// Continuous background rawvideo pipe streamer for real-time 140+ FPS playback.
pub struct PlaybackStreamer {
    active_path: Option<String>,
    active_clip_id: Option<String>,
    frame_rx: Option<Receiver<(f64, Arc<RgbaImage>)>>,
    stop_flag: Option<Arc<AtomicBool>>,
    active_child: Option<Arc<Mutex<Option<std::process::Child>>>>,
    child_handle: Option<std::thread::JoinHandle<()>>,
    current_frame: Option<Arc<RgbaImage>>,
    last_pts: f64,
    pending_future_frame: Option<(f64, Arc<RgbaImage>)>,
    is_reversed: bool,
    started_at: std::time::Instant,
    frames_delivered: u64,
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
        self.stop();

        let path_str = file_path.to_string();
        let clip_str = clip_id.to_string();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = stop_flag.clone();
        let active_child: Arc<Mutex<Option<std::process::Child>>> = Arc::new(Mutex::new(None));
        let child_holder = active_child.clone();

        self.pending_future_frame = None;
        self.is_reversed = reverse;
        self.started_at = std::time::Instant::now();
        self.frames_delivered = 0;

        // Bounded queue of 60 frames (~2.0 seconds buffer)
        let (tx, rx): (SyncSender<DecodedFrame>, Receiver<DecodedFrame>) = sync_channel(60);

        let thread_path = path_str.clone();
        let thread_fps = if fps > 1.0 { fps } else { 30.0 };

        let handle = std::thread::spawn(move || {
            let vf_str = format!(
                "fps=fps={thread_fps:.3},scale=640:360:force_original_aspect_ratio=decrease,pad=640:360:(ow-iw)/2:(oh-ih)/2:black"
            );

            if !reverse {
                // Forward streaming: continuous single FFmpeg process
                let mut cmd = crate::ffmpeg::ffmpeg_cmd();
                cmd.arg("-v").arg("quiet");
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

                let mut child = match cmd.spawn() {
                    Ok(c) => c,
                    Err(_) => return,
                };

                let mut stdout = match child.stdout.take() {
                    Some(s) => s,
                    None => {
                        let _ = child.kill();
                        return;
                    }
                };

                if let Ok(mut lock) = child_holder.lock() {
                    *lock = Some(child);
                }

                let mut frame_idx = 0u64;
                while !stop_clone.load(Ordering::Relaxed) {
                    let mut buf = vec![0u8; PREVIEW_FRAME_BYTES];
                    if stdout.read_exact(&mut buf).is_err() {
                        break;
                    }

                    let pts = source_start_time + (frame_idx as f64 / thread_fps);
                    frame_idx += 1;

                    if let Some(img) = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(
                        PREVIEW_WIDTH,
                        PREVIEW_HEIGHT,
                        buf,
                    ) {
                        if tx.send((pts, Arc::new(img))).is_err() {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                if let Ok(mut lock) = child_holder.lock() {
                    if let Some(mut c) = lock.take() {
                        let _ = c.kill();
                        let _ = c.wait();
                    }
                }
            } else {
                // Reverse streaming: chunked forward decode in 2.0s segments, reversed in memory.
                // Absolutely ZERO ffmpeg reverse filter buffering = ZERO memory leaks (<60MB max).
                let chunk_dur = 2.0;
                let mut curr_end = source_start_time;

                while !stop_clone.load(Ordering::Relaxed) && curr_end > source_in + 0.005 {
                    let curr_start = (curr_end - chunk_dur).max(source_in);
                    let actual_dur = curr_end - curr_start;
                    if actual_dur <= 0.001 {
                        break;
                    }

                    let mut cmd = crate::ffmpeg::ffmpeg_cmd();
                    cmd.arg("-v").arg("quiet");
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

                    let mut child = match cmd.spawn() {
                        Ok(c) => c,
                        Err(_) => break,
                    };

                    let mut stdout = match child.stdout.take() {
                        Some(s) => s,
                        None => {
                            let _ = child.kill();
                            break;
                        }
                    };

                    if let Ok(mut lock) = child_holder.lock() {
                        *lock = Some(child);
                    }

                    let mut chunk_frames = Vec::new();
                    let mut frame_idx = 0u64;

                    while !stop_clone.load(Ordering::Relaxed) {
                        let mut buf = vec![0u8; PREVIEW_FRAME_BYTES];
                        if stdout.read_exact(&mut buf).is_err() {
                            break;
                        }

                        let pts = curr_start + (frame_idx as f64 / thread_fps);
                        frame_idx += 1;

                        if let Some(img) = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(
                            PREVIEW_WIDTH,
                            PREVIEW_HEIGHT,
                            buf,
                        ) {
                            chunk_frames.push((pts, Arc::new(img)));
                        } else {
                            break;
                        }
                    }

                    if let Ok(mut lock) = child_holder.lock() {
                        if let Some(mut c) = lock.take() {
                            let _ = c.wait();
                        }
                    }

                    if stop_clone.load(Ordering::Relaxed) {
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
                let (p, frame) = self.pending_future_frame.take().unwrap();
                self.current_frame = Some(frame);
                self.last_pts = p;
                self.frames_delivered += 1;
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
                        self.current_frame = Some(frame);
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

    /// Stops the background streaming process and immediately terminates any active FFmpeg child process.
    pub fn stop(&mut self) {
        if let Some(flag) = &self.stop_flag {
            flag.store(true, Ordering::Relaxed);
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
        self.child_handle = None;
        self.active_path = None;
        self.active_clip_id = None;
        self.pending_future_frame = None;
        self.frames_delivered = 0;
    }
}

pub struct SeekCommand {
    pub id: u64,
    pub path: String,
    pub time: f64,
}

pub struct SeekResult {
    pub id: u64,
    pub path: String,
    pub time: f64,
    pub frame: Option<Arc<RgbaImage>>,
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
            while let Ok(mut cmd) = seek_rx.recv() {
                // Request coalescing: discard stale seek requests queued during rapid dragging
                while let Ok(newer) = seek_rx.try_recv() {
                    cmd = newer;
                }

                let maybe_img = decode_frame_raw(&cmd.path, cmd.time);
                let res = SeekResult {
                    id: cmd.id,
                    path: cmd.path,
                    time: cmd.time,
                    frame: maybe_img.map(Arc::new),
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

    /// Issues an async seek request for `file_path` at `source_time`.
    pub fn request_seek(&mut self, file_path: &str, source_time: f64) -> u64 {
        self.latest_req_id += 1;
        let id = self.latest_req_id;
        let _ = self.seek_tx.send(SeekCommand {
            id,
            path: file_path.to_string(),
            time: source_time,
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

/// Decoded raw frame and clip metadata for a single visible timeline track layer.
#[derive(Clone, Debug)]
pub struct CompositorLayer {
    pub clip: Clip,
    pub frame: Arc<RgbaImage>,
}

/// Parsed 3D `.cube` LUT unwrapped into a 2D RGBA8 strip texture of dimensions `(size * size, size)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CubeLut3D {
    pub size: u32,
    pub rgba_strip: Vec<u8>,
}

impl CubeLut3D {
    /// Trilinear/slice-interpolated 3D LUT sampling for normalized RGB in `[0.0, 1.0]`.
    #[inline]
    pub fn sample(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        let n = self.size as usize;
        if n < 2 || self.rgba_strip.len() < n * n * n * 4 {
            return [r, g, b];
        }
        let max_idx = (n - 1) as f32;
        let rf = (r.clamp(0.0, 1.0) * max_idx).clamp(0.0, max_idx);
        let gf = (g.clamp(0.0, 1.0) * max_idx).clamp(0.0, max_idx);
        let bf = (b.clamp(0.0, 1.0) * max_idx).clamp(0.0, max_idx);

        let r0 = (rf.floor() as usize).min(n - 1);
        let r1 = (r0 + 1).min(n - 1);
        let dr = rf - r0 as f32;

        let g0 = (gf.floor() as usize).min(n - 1);
        let g1 = (g0 + 1).min(n - 1);
        let dg = gf - g0 as f32;

        let b0 = (bf.floor() as usize).min(n - 1);
        let b1 = (b0 + 1).min(n - 1);
        let db = bf - b0 as f32;

        let width = n * n;
        let fetch = |ri: usize, gi: usize, bi: usize| -> [f32; 3] {
            let x = bi * n + ri;
            let y = gi;
            let idx = (y * width + x) * 4;
            [
                self.rgba_strip[idx] as f32 / 255.0,
                self.rgba_strip[idx + 1] as f32 / 255.0,
                self.rgba_strip[idx + 2] as f32 / 255.0,
            ]
        };

        let c000 = fetch(r0, g0, b0);
        let c100 = fetch(r1, g0, b0);
        let c010 = fetch(r0, g1, b0);
        let c110 = fetch(r1, g1, b0);
        let c001 = fetch(r0, g0, b1);
        let c101 = fetch(r1, g0, b1);
        let c011 = fetch(r0, g1, b1);
        let c111 = fetch(r1, g1, b1);

        let mut out = [0.0f32; 3];
        for i in 0..3 {
            let c00 = c000[i] + (c100[i] - c000[i]) * dr;
            let c10 = c010[i] + (c110[i] - c010[i]) * dr;
            let c01 = c001[i] + (c101[i] - c001[i]) * dr;
            let c11 = c011[i] + (c111[i] - c011[i]) * dr;
            let c0 = c00 + (c10 - c00) * dg;
            let c1 = c01 + (c11 - c01) * dg;
            out[i] = (c0 + (c1 - c0) * db).clamp(0.0, 1.0);
        }
        out
    }
}

pub fn parse_cube_lut_str(content: &str) -> Option<CubeLut3D> {
    let mut size: Option<u32> = None;
    let mut triplets: Vec<[f32; 3]> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("TITLE") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("LUT_3D_SIZE") {
            let n: u32 = rest.split_whitespace().next()?.parse().ok()?;
            if !(2..=64).contains(&n) {
                return None;
            }
            size = Some(n);
            triplets.reserve((n * n * n) as usize);
            continue;
        }
        if trimmed.starts_with("DOMAIN_MIN")
            || trimmed.starts_with("DOMAIN_MAX")
            || trimmed.starts_with("LUT_1D_SIZE")
        {
            continue;
        }

        let mut parts = trimmed.split_whitespace();
        if let (Some(rs), Some(gs), Some(bs)) = (parts.next(), parts.next(), parts.next()) {
            if let (Ok(r), Ok(g), Ok(b)) = (rs.parse::<f32>(), gs.parse::<f32>(), bs.parse::<f32>())
            {
                triplets.push([r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]);
            }
        }
    }

    let n = size?;
    let expected = (n * n * n) as usize;
    if triplets.len() < expected {
        return None;
    }

    let width = (n * n) as usize;
    let height = n as usize;
    let mut rgba_strip = vec![255u8; width * height * 4];

    for b in 0..height {
        for g in 0..height {
            for r in 0..height {
                let src_idx = b * height * height + g * height + r;
                let dst_x = b * height + r;
                let dst_y = g;
                let dst_idx = (dst_y * width + dst_x) * 4;
                let rgb = triplets[src_idx];
                rgba_strip[dst_idx] = (rgb[0] * 255.0).round() as u8;
                rgba_strip[dst_idx + 1] = (rgb[1] * 255.0).round() as u8;
                rgba_strip[dst_idx + 2] = (rgb[2] * 255.0).round() as u8;
                rgba_strip[dst_idx + 3] = 255;
            }
        }
    }

    Some(CubeLut3D {
        size: n,
        rgba_strip,
    })
}

pub fn parse_cube_lut_file<P: AsRef<Path>>(path: P) -> Option<CubeLut3D> {
    let norm = trim_core::paths::normalize_path(path);
    let content = std::fs::read_to_string(norm).ok()?;
    parse_cube_lut_str(&content)
}

/// High-level video playback and composite rendering engine.
pub struct VideoEngine {
    pub streamer: PlaybackStreamer,
    pub async_decoder: AsyncFrameDecoder,
    pub frame_cache: FrameCacheMap,
    pub image_cache: HashMap<String, Arc<RgbaImage>>,
    pub last_raw_source_frame: Option<Arc<RgbaImage>>,
    pub last_rendered_frame: Option<Arc<RgbaImage>>,
    pub is_decoding_seek: bool,
    pub use_proxies: bool,
}

impl Default for VideoEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoEngine {
    pub fn new() -> Self {
        Self {
            streamer: PlaybackStreamer::new(),
            async_decoder: AsyncFrameDecoder::new(),
            frame_cache: Arc::new(RwLock::new(LruCache::with_capacity(300))),
            image_cache: HashMap::new(),
            last_raw_source_frame: None,
            last_rendered_frame: None,
            is_decoding_seek: false,
            use_proxies: false,
        }
    }

    pub fn clear_cache(&mut self) {
        self.streamer.stop();
        if let Ok(mut fc) = self.frame_cache.write() {
            fc.clear();
        }
        self.image_cache.clear();
        self.last_raw_source_frame = None;
        self.last_rendered_frame = None;
        self.is_decoding_seek = false;
    }

    /// Dynamically sets the maximum number of frames held in the LRU frame cache.
    pub fn set_frame_cache_capacity(&self, capacity: usize) {
        if let Ok(mut fc) = self.frame_cache.write() {
            fc.set_max_entries(capacity);
        }
    }

    pub fn get_image<P: AsRef<Path>>(&mut self, path: P) -> Option<Arc<RgbaImage>> {
        let p_str = path.as_ref().to_string_lossy().to_string();
        if !self.image_cache.contains_key(&p_str) {
            if let Ok(dyn_img) = image::open(&p_str) {
                self.image_cache
                    .insert(p_str.clone(), Arc::new(dyn_img.to_rgba8()));
            } else {
                return None;
            }
        }
        self.image_cache.get(&p_str).cloned()
    }

    /// Analyzes the raw source frame of `clip` at `timeline_time` (or theclip's midpoint if outside)
    /// and returns the estimated Auto White Balance `(temperature, tint)` parameters.
    pub fn analyze_clip_auto_wb(
        &mut self,
        project: &Project,
        clip: &Clip,
        timeline_time: f64,
    ) -> Option<(f64, f64)> {
        let media_item = project.get_media(&clip.media_id)?;
        let sample_tl_time =
            if timeline_time >= clip.timeline_in && timeline_time < clip.timeline_out {
                timeline_time
            } else {
                0.5 * (clip.timeline_in + clip.timeline_out)
            };

        let raw_img: Arc<RgbaImage> = if media_item.media_type == MediaType::Image {
            self.get_image(&media_item.file_path)?
        } else {
            let eff_path = media_item.resolved_playback_path(self.use_proxies);
            let src_t = clip.map_timeline_to_source(sample_tl_time);
            let ts_key = (src_t * 30.0).round() as u64;
            let key = (eff_path.to_string(), ts_key);

            let cached = self
                .frame_cache
                .write()
                .ok()
                .and_then(|mut c| c.get(&key).cloned());
            if let Some(frame) = cached {
                frame
            } else if let Some(ref raw) = self.last_raw_source_frame {
                Arc::clone(raw)
            } else {
                let decoded = decode_frame_raw(eff_path, src_t)?;
                let arc = Arc::new(decoded);
                if let Ok(mut cache) = self.frame_cache.write() {
                    cache.insert(key, Arc::clone(&arc));
                }
                arc
            }
        };

        Some(trim_core::color::analyze_auto_white_balance_rgba(
            raw_img.as_raw(),
            raw_img.width(),
            raw_img.height(),
        ))
    }

    /// Decodes and collects all visible video/image layers at `timeline_time` without CPU pixel compositing.
    /// Used by the OpenGL (`glow`) FBO shader compositor to upload raw frames directly to GPU textures.
    pub fn collect_timeline_layers(
        &mut self,
        project: &Project,
        timeline_time: f64,
        is_playing: bool,
    ) -> Vec<CompositorLayer> {
        let (canvas_w, canvas_h) =
            calculate_preview_size(project.timeline.width, project.timeline.height);

        let flattened_tl = project.get_flattened_timeline();
        let visible_clips = flattened_tl.get_visible_video_clips_at(timeline_time);
        if visible_clips.is_empty() {
            self.streamer.stop();
            self.last_rendered_frame = None;
            self.last_raw_source_frame = None;
            if self.is_decoding_seek {
                self.async_decoder.latest_req_id += 1;
                self.is_decoding_seek = false;
            }
            let _ = self.async_decoder.poll_result();
            return Vec::new();
        }

        if let Some(res) = self.async_decoder.poll_result() {
            if res.id == self.async_decoder.latest_req_id {
                self.is_decoding_seek = false;
                if let Some(frame) = res.frame {
                    let key = (res.path.clone(), (res.time * 30.0).round() as u64);
                    if let Ok(mut cache) = self.frame_cache.write() {
                        cache.insert(key, frame.clone());
                    }
                    self.last_raw_source_frame = Some(frame);
                }
            }
        }

        let mut layers = Vec::with_capacity(visible_clips.len());

        for (_track, clip) in visible_clips {
            if clip.is_adjustment_layer() {
                let sentinel = Arc::new(ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0])));
                layers.push(CompositorLayer {
                    clip: clip.clone(),
                    frame: sentinel,
                });
                continue;
            }
            let media_item = match project.get_media(&clip.media_id) {
                Some(m) => m,
                None => continue,
            };

            let frame_img = match media_item.media_type {
                MediaType::Image => {
                    if self.is_decoding_seek {
                        self.async_decoder.latest_req_id += 1;
                        self.is_decoding_seek = false;
                    }
                    let is_title_card = media_item.file_path.contains("titles");
                    if clip.text_overlay.is_some() && is_title_card {
                        Some(Arc::new(ImageBuffer::from_pixel(
                            canvas_w,
                            canvas_h,
                            Rgba([0, 0, 0, 0]),
                        )))
                    } else {
                        self.get_image(&media_item.file_path)
                    }
                }
                _ => {
                    let eff_path = media_item.resolved_playback_path(self.use_proxies);
                    let src_t = clip.map_timeline_to_source(timeline_time);
                    let ts_key = (src_t * 30.0).round() as u64;
                    let key = (eff_path.to_string(), ts_key);

                    let streamer_frame = if is_playing {
                        if self.streamer.active_clip_id() == Some(&clip.id) {
                            if self.streamer.should_restart(src_t, clip.reverse) {
                                self.streamer.start(
                                    &clip.id,
                                    eff_path,
                                    src_t,
                                    project.timeline.fps,
                                    clip.reverse,
                                    clip.source_in,
                                );
                            }
                            self.streamer.get_frame_for_time(src_t)
                        } else if !self.streamer.is_active() {
                            self.streamer.start(
                                &clip.id,
                                eff_path,
                                src_t,
                                project.timeline.fps,
                                clip.reverse,
                                clip.source_in,
                            );
                            self.streamer.get_frame_for_time(src_t)
                        } else {
                            None
                        }
                    } else {
                        if self.streamer.is_active() {
                            self.streamer.stop();
                        }
                        None
                    };

                    if let Some(sf) = streamer_frame {
                        self.last_raw_source_frame = Some(sf.clone());
                        Some(sf)
                    } else {
                        let cached = self
                            .frame_cache
                            .write()
                            .ok()
                            .and_then(|mut c| c.get(&key).cloned());
                        if let Some(c) = cached {
                            self.last_raw_source_frame = Some(c.clone());
                            self.is_decoding_seek = false;
                            Some(c)
                        } else if self.last_raw_source_frame.is_none() {
                            if let Some(frame) = decode_frame_raw(eff_path, src_t) {
                                let arc = Arc::new(frame);
                                if let Ok(mut cache) = self.frame_cache.write() {
                                    cache.insert(key, arc.clone());
                                }
                                self.last_raw_source_frame = Some(arc.clone());
                                self.is_decoding_seek = false;
                                Some(arc)
                            } else {
                                None
                            }
                        } else {
                            if !self.is_decoding_seek {
                                self.async_decoder.request_seek(eff_path, src_t);
                                self.is_decoding_seek = true;
                            }
                            self.last_raw_source_frame.clone()
                        }
                    }
                }
            };

            if let Some(frame) = frame_img {
                layers.push(CompositorLayer {
                    clip: clip.clone(),
                    frame,
                });
            }
        }

        layers
    }

    /// Primary timeline rendering entry point with zero UI lag.
    pub fn render_timeline_frame(
        &mut self,
        project: &Project,
        timeline_time: f64,
        is_playing: bool,
    ) -> Option<Arc<RgbaImage>> {
        let (canvas_w, canvas_h) =
            calculate_preview_size(project.timeline.width, project.timeline.height);

        let visible_clips = project.timeline.get_visible_video_clips_at(timeline_time);
        if visible_clips.is_empty() {
            // Never retain the final frame when the playhead leaves a clip.
            // Invalidate outstanding seek requests so late results cannot
            // repopulate a blank monitor after a timeline gap.
            self.streamer.stop();
            self.last_rendered_frame = None;
            self.last_raw_source_frame = None;
            if self.is_decoding_seek {
                self.async_decoder.latest_req_id += 1;
                self.is_decoding_seek = false;
            }
            let _ = self.async_decoder.poll_result();
            return None;
        }

        // Only accept a result from the latest requested seek. Results from
        // a clip we already left must not be displayed on another clip.
        if let Some(res) = self.async_decoder.poll_result() {
            if res.id == self.async_decoder.latest_req_id {
                self.is_decoding_seek = false;
                if let Some(frame) = res.frame {
                    let key = (res.path.clone(), (res.time * 30.0).round() as u64);
                    if let Ok(mut cache) = self.frame_cache.write() {
                        cache.insert(key, frame.clone());
                    }
                    self.last_raw_source_frame = Some(frame.clone());
                    self.last_rendered_frame = Some(frame);
                }
            }
        }

        // Fast Path: Single full-screen video/image clip
        if visible_clips.len() == 1 {
            let (_track, clip) = visible_clips[0];
            let media_item = project.get_media(&clip.media_id)?;

            let dt = (timeline_time - clip.timeline_in).max(0.0);
            let rem = (clip.timeline_out - timeline_time).max(0.0);
            let in_fade_in = clip.fade_in > 0.0 && dt < clip.fade_in;
            let in_fade_out = clip.fade_out > 0.0 && rem < clip.fade_out;

            let is_standard_transform = canvas_w == PREVIEW_WIDTH
                && canvas_h == PREVIEW_HEIGHT
                && clip.opacity >= 0.99
                && !in_fade_in
                && !in_fade_out
                && clip.transition_in.is_none()
                && clip.transition_out.is_none()
                && !clip.has_color_adjustments()
                && !clip.has_crop()
                && !clip.chromakey.as_ref().is_some_and(|ck| ck.enabled)
                && clip.keyframes.is_empty()
                && clip.text_overlay.is_none()
                && clip.scale == 1.0
                && clip.pos_x == 0.0
                && clip.pos_y == 0.0
                && (clip.speed - 1.0).abs() < 0.01;

            if media_item.media_type == MediaType::Image {
                if self.is_decoding_seek {
                    self.async_decoder.latest_req_id += 1;
                    self.is_decoding_seek = false;
                }

                let is_title_card = media_item.file_path.contains("titles");
                let img_opt = if clip.text_overlay.is_some() && is_title_card {
                    Some(Arc::new(ImageBuffer::from_pixel(
                        canvas_w,
                        canvas_h,
                        Rgba([0, 0, 0, 0]),
                    )))
                } else {
                    self.get_image(&media_item.file_path)
                };

                if let Some(img) = img_opt {
                    if is_standard_transform {
                        self.last_rendered_frame = Some(img.clone());
                        return Some(img);
                    } else {
                        let mut canvas =
                            ImageBuffer::from_pixel(canvas_w, canvas_h, Rgba([0, 0, 0, 255]));
                        draw_clip_onto_canvas(&mut canvas, clip, &img, timeline_time);
                        let arc_canvas = Arc::new(canvas);
                        self.last_rendered_frame = Some(arc_canvas.clone());
                        return Some(arc_canvas);
                    }
                }
            } else {
                let eff_path = media_item.resolved_playback_path(self.use_proxies);
                let src_t = clip.map_timeline_to_source(timeline_time);

                if is_playing {
                    // Playback Mode: Use streaming rawvideo pipe
                    let needs_restart = match self.streamer.active_clip_id() {
                        Some(id) => {
                            id != clip.id || self.streamer.should_restart(src_t, clip.reverse)
                        }
                        None => true,
                    };

                    if needs_restart {
                        self.streamer.start(
                            &clip.id,
                            eff_path,
                            src_t,
                            project.timeline.fps,
                            clip.reverse,
                            clip.source_in,
                        );
                    }

                    if let Some(frame) = self.streamer.get_frame_for_time(src_t) {
                        self.last_raw_source_frame = Some(frame.clone());
                        if is_standard_transform {
                            self.last_rendered_frame = Some(frame.clone());
                            return Some(frame);
                        } else {
                            let mut canvas = ImageBuffer::from_pixel(
                                PREVIEW_WIDTH,
                                PREVIEW_HEIGHT,
                                Rgba([0, 0, 0, 255]),
                            );
                            draw_clip_onto_canvas(&mut canvas, clip, &frame, timeline_time);
                            let arc_canvas = Arc::new(canvas);
                            self.last_rendered_frame = Some(arc_canvas.clone());
                            return Some(arc_canvas);
                        }
                    } else if let Some(last_raw) = &self.last_raw_source_frame {
                        if is_standard_transform {
                            return Some(last_raw.clone());
                        } else {
                            let mut canvas = ImageBuffer::from_pixel(
                                PREVIEW_WIDTH,
                                PREVIEW_HEIGHT,
                                Rgba([0, 0, 0, 255]),
                            );
                            draw_clip_onto_canvas(&mut canvas, clip, last_raw, timeline_time);
                            let arc_canvas = Arc::new(canvas);
                            self.last_rendered_frame = Some(arc_canvas.clone());
                            return Some(arc_canvas);
                        }
                    }
                } else {
                    // Scrubbing/Paused Mode: Stop streamer, check cache, or request async seek
                    if self.streamer.is_active() {
                        self.streamer.stop();
                    }

                    let ts_key = (src_t * 30.0).round() as u64;
                    let key = (eff_path.to_string(), ts_key);

                    if let Ok(mut cache) = self.frame_cache.write() {
                        if let Some(cached) = cache.get(&key) {
                            let cached = cached.clone();
                            self.last_raw_source_frame = Some(cached.clone());
                            self.is_decoding_seek = false;
                            if is_standard_transform {
                                self.last_rendered_frame = Some(cached.clone());
                                return Some(cached);
                            } else {
                                let mut canvas = ImageBuffer::from_pixel(
                                    canvas_w,
                                    canvas_h,
                                    Rgba([0, 0, 0, 255]),
                                );
                                draw_clip_onto_canvas(&mut canvas, clip, &cached, timeline_time);
                                let arc_canvas = Arc::new(canvas);
                                self.last_rendered_frame = Some(arc_canvas.clone());
                                return Some(arc_canvas);
                            }
                        }
                    }

                    if self.last_raw_source_frame.is_none() {
                        // Immediate first frame decode so monitor is never blank on initial clip placement
                        if let Some(frame) = decode_frame_raw(eff_path, src_t) {
                            let arc = Arc::new(frame);
                            if let Ok(mut cache) = self.frame_cache.write() {
                                cache.insert(key, arc.clone());
                            }
                            self.last_raw_source_frame = Some(arc.clone());
                            self.is_decoding_seek = false;
                            if is_standard_transform {
                                self.last_rendered_frame = Some(arc.clone());
                                return Some(arc);
                            } else {
                                let mut canvas = ImageBuffer::from_pixel(
                                    canvas_w,
                                    canvas_h,
                                    Rgba([0, 0, 0, 255]),
                                );
                                draw_clip_onto_canvas(&mut canvas, clip, &arc, timeline_time);
                                let arc_canvas = Arc::new(canvas);
                                self.last_rendered_frame = Some(arc_canvas.clone());
                                return Some(arc_canvas);
                            }
                        }
                    }

                    // Trigger non-blocking async seek if not already in flight
                    if !self.is_decoding_seek {
                        self.async_decoder.request_seek(eff_path, src_t);
                        self.is_decoding_seek = true;
                    }

                    if let Some(last_raw) = &self.last_raw_source_frame {
                        if is_standard_transform {
                            return Some(last_raw.clone());
                        } else {
                            let mut canvas =
                                ImageBuffer::from_pixel(canvas_w, canvas_h, Rgba([0, 0, 0, 255]));
                            draw_clip_onto_canvas(&mut canvas, clip, last_raw, timeline_time);
                            let arc_canvas = Arc::new(canvas);
                            self.last_rendered_frame = Some(arc_canvas.clone());
                            return Some(arc_canvas);
                        }
                    }
                }

                return self.last_rendered_frame.clone();
            }
        }

        // Layered Composite Path: Multiple clips, transitions, opacity, fades, keyframes
        let mut canvas = ImageBuffer::from_pixel(canvas_w, canvas_h, Rgba([0, 0, 0, 255]));

        for (_track, clip) in visible_clips {
            let media_item = match project.get_media(&clip.media_id) {
                Some(m) => m,
                None => continue,
            };

            let frame_img = match media_item.media_type {
                MediaType::Image => {
                    let is_title_card = media_item.file_path.contains("titles");
                    if clip.text_overlay.is_some() && is_title_card {
                        Some(Arc::new(ImageBuffer::from_pixel(
                            canvas_w,
                            canvas_h,
                            Rgba([0, 0, 0, 0]),
                        )))
                    } else {
                        self.get_image(&media_item.file_path)
                    }
                }
                _ => {
                    let eff_path = media_item.resolved_playback_path(self.use_proxies);
                    let src_t = clip.map_timeline_to_source(timeline_time);
                    let ts_key = (src_t * 30.0).round() as u64;
                    let key = (eff_path.to_string(), ts_key);

                    let streamer_frame = if is_playing {
                        if self.streamer.active_clip_id() == Some(&clip.id) {
                            if self.streamer.should_restart(src_t, clip.reverse) {
                                self.streamer.start(
                                    &clip.id,
                                    eff_path,
                                    src_t,
                                    project.timeline.fps,
                                    clip.reverse,
                                    clip.source_in,
                                );
                            }
                            self.streamer.get_frame_for_time(src_t)
                        } else if !self.streamer.is_active() {
                            self.streamer.start(
                                &clip.id,
                                eff_path,
                                src_t,
                                project.timeline.fps,
                                clip.reverse,
                                clip.source_in,
                            );
                            self.streamer.get_frame_for_time(src_t)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some(sf) = streamer_frame {
                        self.last_raw_source_frame = Some(sf.clone());
                        Some(sf)
                    } else {
                        let cached = self
                            .frame_cache
                            .write()
                            .ok()
                            .and_then(|mut c| c.get(&key).cloned());
                        if let Some(c) = cached {
                            self.last_raw_source_frame = Some(c.clone());
                            Some(c)
                        } else if self.last_raw_source_frame.is_none() {
                            if let Some(frame) = decode_frame_raw(eff_path, src_t) {
                                let arc = Arc::new(frame);
                                if let Ok(mut cache) = self.frame_cache.write() {
                                    cache.insert(key, arc.clone());
                                }
                                self.last_raw_source_frame = Some(arc.clone());
                                self.is_decoding_seek = false;
                                Some(arc)
                            } else {
                                None
                            }
                        } else {
                            if !self.is_decoding_seek {
                                self.async_decoder.request_seek(eff_path, src_t);
                                self.is_decoding_seek = true;
                            }
                            self.last_raw_source_frame.clone()
                        }
                    }
                }
            };

            if let Some(img) = frame_img {
                draw_clip_onto_canvas(&mut canvas, clip, &img, timeline_time);
            }
        }

        let arc_canvas = Arc::new(canvas);
        self.last_rendered_frame = Some(arc_canvas.clone());
        Some(arc_canvas)
    }
}

/// Backward compatibility wrapper for existing tests and synchronous tools.
pub struct VideoFrameDecoder {
    image_cache: HashMap<String, RgbaImage>,
    frame_cache: LruCache<(String, u64), RgbaImage>,
}

impl Default for VideoFrameDecoder {
    fn default() -> Self {
        Self {
            image_cache: HashMap::new(),
            frame_cache: LruCache::with_capacity(150),
        }
    }
}

impl VideoFrameDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_image<P: AsRef<Path>>(&mut self, path: P) -> Option<&RgbaImage> {
        let p_str = path.as_ref().to_string_lossy().to_string();
        if !self.image_cache.contains_key(&p_str) {
            if let Ok(dyn_img) = image::open(&p_str) {
                self.image_cache.insert(p_str.clone(), dyn_img.to_rgba8());
            } else {
                return None;
            }
        }
        self.image_cache.get(&p_str)
    }

    pub fn decode_frame<P: AsRef<Path>>(
        &mut self,
        file_path: P,
        source_time: f64,
    ) -> Option<RgbaImage> {
        let p = file_path.as_ref();
        let p_str = p.to_string_lossy().to_string();
        let ts_key = (source_time * 30.0).round() as u64;

        if let Some(cached) = self.frame_cache.get(&(p_str.clone(), ts_key)) {
            return Some(cached.clone());
        }

        if let Some(img) = decode_frame_raw(p, source_time) {
            self.frame_cache.insert((p_str, ts_key), img.clone());
            Some(img)
        } else {
            None
        }
    }

    pub fn render_timeline_frame(
        &mut self,
        project: &Project,
        timeline_time: f64,
        target_w: u32,
        target_h: u32,
    ) -> RgbaImage {
        let visible_clips = project.timeline.get_visible_video_clips_at(timeline_time);
        let mut canvas = ImageBuffer::from_pixel(target_w, target_h, Rgba([14, 14, 16, 255]));

        if visible_clips.is_empty() {
            return canvas;
        }

        for (_track, clip) in visible_clips {
            if clip.is_adjustment_layer() {
                let sentinel = ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0]));
                draw_clip_onto_canvas(&mut canvas, clip, &sentinel, timeline_time);
                continue;
            }
            let media_item = match project.get_media(&clip.media_id) {
                Some(m) => m,
                None => continue,
            };

            let frame_img = match media_item.media_type {
                MediaType::Image => self.get_image(&media_item.file_path).cloned(),
                _ => {
                    let src_t = clip.map_timeline_to_source(timeline_time);
                    self.decode_frame(&media_item.file_path, src_t)
                }
            };

            if let Some(img) = frame_img {
                draw_clip_onto_canvas(&mut canvas, clip, &img, timeline_time);
            }
        }

        canvas
    }
}

/// Applies transformations, opacity, color grading and draws clip onto target canvas.
fn draw_clip_onto_canvas(
    canvas: &mut RgbaImage,
    clip: &Clip,
    frame: &RgbaImage,
    current_time: f64,
) {
    let clip_time = (current_time - clip.timeline_in).max(0.0);
    let animated_opacity = clip.get_property_at_time("opacity", clip_time);
    let animated_scale = clip.get_property_at_time("scale", clip_time);
    let animated_pos_x = clip.get_property_at_time("pos_x", clip_time);
    let animated_pos_y = clip.get_property_at_time("pos_y", clip_time);

    let dt = current_time - clip.timeline_in;
    let rem = clip.timeline_out - current_time;

    let in_progress = if clip.fade_in > 0.0 && dt < clip.fade_in {
        (dt / clip.fade_in).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let out_progress = if clip.fade_out > 0.0 && rem < clip.fade_out {
        (rem / clip.fade_out).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let fade_factor = in_progress.min(out_progress);

    let is_wipe_or_slide = clip
        .transition_in
        .as_deref()
        .is_some_and(|t| t.starts_with("wipe") || t.starts_with("slide"))
        || clip
            .transition_out
            .as_deref()
            .is_some_and(|t| t.starts_with("wipe") || t.starts_with("slide"));

    let eff_opacity = if is_wipe_or_slide {
        animated_opacity.clamp(0.0, 1.0)
    } else {
        (animated_opacity * fade_factor).clamp(0.0, 1.0)
    };

    if eff_opacity <= 0.001 {
        return;
    }

    let color_params = clip.color_grade_params();
    let custom_lut = clip
        .custom_lut_path
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .and_then(parse_cube_lut_file);
    let has_masks = clip.masks.iter().any(|m| m.enabled);

    if clip.is_adjustment_layer() {
        let cw_u32 = canvas.width();
        let ch_u32 = canvas.height();
        if cw_u32 == 0 || ch_u32 == 0 {
            return;
        }
        let has_color_adj = clip.has_color_adjustments();
        let aspect = cw_u32 as f32 / ch_u32.max(1) as f32;
        for cy in 0..ch_u32 {
            let v = (cy as f32 + 0.5) / ch_u32 as f32;
            for cx in 0..cw_u32 {
                let u = (cx as f32 + 0.5) / cw_u32 as f32;
                let (alpha_mask, color_mask) = if has_masks {
                    trim_core::mask::evaluate_masks_at_uv(&clip.masks, [u, v], clip_time, aspect)
                } else {
                    (1.0, 1.0)
                };
                let eff_a = (eff_opacity as f32 * alpha_mask).clamp(0.0, 1.0);
                if eff_a <= 0.0001 {
                    continue;
                }
                let dst_px = canvas.get_pixel_mut(cx, cy);
                let dr = dst_px[0] as f32;
                let dg = dst_px[1] as f32;
                let db = dst_px[2] as f32;

                let [gr, gg, gb] = apply_clip_grading_rgb(
                    dr,
                    dg,
                    db,
                    clip,
                    has_color_adj,
                    &color_params,
                    custom_lut.as_ref(),
                );
                let mr = dr + (gr - dr) * color_mask;
                let mg = dg + (gg - dg) * color_mask;
                let mb = db + (gb - db) * color_mask;

                let [out_r, out_g, out_b, _] = trim_core::compositing::blend_rgba_over(
                    [dr / 255.0, dg / 255.0, db / 255.0, 1.0],
                    [mr / 255.0, mg / 255.0, mb / 255.0, eff_a],
                    1.0,
                    clip.blend_mode,
                );
                dst_px[0] = (out_r * 255.0).round().clamp(0.0, 255.0) as u8;
                dst_px[1] = (out_g * 255.0).round().clamp(0.0, 255.0) as u8;
                dst_px[2] = (out_b * 255.0).round().clamp(0.0, 255.0) as u8;
                dst_px[3] = 255;
            }
        }
        return;
    }

    let cw = canvas.width() as f64;
    let ch = canvas.height() as f64;
    let fw = frame.width() as f64;
    let fh = frame.height() as f64;

    if fw <= 0.0 || fh <= 0.0 {
        return;
    }

    let scale_fit = (cw / fw).min(ch / fh);
    let mut eff_scale = scale_fit * animated_scale.max(0.05);

    // Zoom-in transition
    if clip.transition_in.as_deref() == Some("zoom_in") && in_progress < 1.0 {
        eff_scale *= 0.3 + 0.7 * in_progress;
    }

    let final_w = (fw * eff_scale).round() as i64;
    let final_h = (fh * eff_scale).round() as i64;

    let mut slide_x_offset = 0.0;
    if in_progress < 1.0 {
        if clip.transition_in.as_deref() == Some("slide_left") {
            slide_x_offset += (1.0 - in_progress) * cw;
        } else if clip.transition_in.as_deref() == Some("slide_right") {
            slide_x_offset -= (1.0 - in_progress) * cw;
        }
    }
    if out_progress < 1.0 {
        if clip.transition_out.as_deref() == Some("slide_left") {
            slide_x_offset -= (1.0 - out_progress) * cw;
        } else if clip.transition_out.as_deref() == Some("slide_right") {
            slide_x_offset += (1.0 - out_progress) * cw;
        }
    }

    let final_x = ((cw - final_w as f64) / 2.0 + animated_pos_x + slide_x_offset).round() as i64;
    let final_y = ((ch - final_h as f64) / 2.0 + animated_pos_y).round() as i64;

    let has_dip_white = (clip.transition_in.as_deref() == Some("dip_white") && fade_factor < 1.0)
        || (clip.transition_out.as_deref() == Some("dip_white") && fade_factor < 1.0);
    let dip_white_amt = if has_dip_white {
        (1.0 - fade_factor) as f32
    } else {
        0.0
    };

    let wipe_left_active = (clip.transition_in.as_deref() == Some("wipe_left")
        && in_progress < 1.0)
        || (clip.transition_out.as_deref() == Some("wipe_left") && out_progress < 1.0);
    let wipe_right_active = (clip.transition_in.as_deref() == Some("wipe_right")
        && in_progress < 1.0)
        || (clip.transition_out.as_deref() == Some("wipe_right") && out_progress < 1.0);

    let has_chromakey = clip.chromakey.as_ref().is_some_and(|ck| ck.enabled);
    let has_color_adj = clip.has_color_adjustments() || has_chromakey;
    let has_clip_grading = clip.has_color_adjustments();
    let has_blend_or_mask = clip.blend_mode != trim_core::BlendMode::Normal || has_masks;
    let has_special_trans =
        has_dip_white || wipe_left_active || wipe_right_active || slide_x_offset.abs() > 0.1;

    let is_fullscreen_fit = final_w == cw as i64
        && final_h == ch as i64
        && final_x == 0
        && final_y == 0
        && (fw - cw).abs() < 0.001
        && (fh - ch).abs() < 0.001;

    if is_fullscreen_fit && !has_color_adj && !has_special_trans && !has_blend_or_mask {
        let alpha = eff_opacity as f32;
        let src_raw = frame.as_raw();
        let dst_raw: &mut [u8] = canvas.as_mut();

        for i in (0..src_raw.len()).step_by(4) {
            let sa = (src_raw[i + 3] as f32 / 255.0) * alpha;
            let inv_sa = 1.0 - sa;
            let dr = dst_raw[i] as f32;
            let dg = dst_raw[i + 1] as f32;
            let db = dst_raw[i + 2] as f32;

            dst_raw[i] = ((src_raw[i] as f32) * sa + dr * inv_sa).round() as u8;
            dst_raw[i + 1] = ((src_raw[i + 1] as f32) * sa + dg * inv_sa).round() as u8;
            dst_raw[i + 2] = ((src_raw[i + 2] as f32) * sa + db * inv_sa).round() as u8;
            dst_raw[i + 3] = 255;
        }
        return;
    }

    for y in 0..final_h {
        let cy = final_y + y;
        if cy < 0 || cy >= ch as i64 {
            continue;
        }

        let sy = ((y as f64 / final_h as f64) * fh) as u32;
        let sy = sy.min(frame.height() - 1);

        for x in 0..final_w {
            let cx = final_x + x;
            if cx < 0 || cx >= cw as i64 {
                continue;
            }

            if wipe_left_active {
                let norm_x = x as f64 / final_w.max(1) as f64;
                if norm_x > fade_factor {
                    continue;
                }
            }
            if wipe_right_active {
                let norm_x = x as f64 / final_w.max(1) as f64;
                if norm_x < (1.0 - fade_factor) {
                    continue;
                }
            }

            let sx = ((x as f64 / final_w as f64) * fw) as u32;
            let sx = sx.min(frame.width() - 1);

            let src_px = frame.get_pixel(sx, sy);
            let mut r = src_px[0] as f32;
            let mut g = src_px[1] as f32;
            let mut b = src_px[2] as f32;
            let mut a = (src_px[3] as f32 / 255.0) * eff_opacity as f32;

            if let Some(ck) = &clip.chromakey {
                if ck.enabled {
                    let kr = ck.key_color[0] as f32;
                    let kg = ck.key_color[1] as f32;
                    let kb = ck.key_color[2] as f32;
                    let dr = (r - kr) / 255.0;
                    let dg = (g - kg) / 255.0;
                    let db = (b - kb) / 255.0;
                    let dist = (dr * dr + dg * dg + db * db).sqrt();
                    let sim = ck.similarity.clamp(0.01, 1.0);
                    let smooth = ck.smoothness.clamp(0.001, 0.5);
                    if dist < sim {
                        continue;
                    } else if dist < sim + smooth {
                        let blend = (dist - sim) / smooth;
                        a *= blend;
                    }
                }
            }

            let (alpha_mask, color_mask) = if has_masks {
                let u = (cx as f32 + 0.5) / cw as f32;
                let v = (cy as f32 + 0.5) / ch as f32;
                let aspect = (cw / ch.max(1.0)) as f32;
                trim_core::mask::evaluate_masks_at_uv(&clip.masks, [u, v], clip_time, aspect)
            } else {
                (1.0, 1.0)
            };
            a *= alpha_mask;
            if a <= 0.0001 {
                continue;
            }

            let [gr, gg, gb] = apply_clip_grading_rgb(
                r,
                g,
                b,
                clip,
                has_clip_grading,
                &color_params,
                custom_lut.as_ref(),
            );
            r += (gr - r) * color_mask;
            g += (gg - g) * color_mask;
            b += (gb - b) * color_mask;

            // Dip to white
            if dip_white_amt > 0.0 {
                r = r + (255.0 - r) * dip_white_amt;
                g = g + (255.0 - g) * dip_white_amt;
                b = b + (255.0 - b) * dip_white_amt;
            }

            // Blend & composite over canvas
            let dst_px = canvas.get_pixel_mut(cx as u32, cy as u32);
            let dr = dst_px[0] as f32 / 255.0;
            let dg = dst_px[1] as f32 / 255.0;
            let db = dst_px[2] as f32 / 255.0;

            let [out_r, out_g, out_b, _] = trim_core::compositing::blend_rgba_over(
                [dr, dg, db, 1.0],
                [r / 255.0, g / 255.0, b / 255.0, a],
                1.0,
                clip.blend_mode,
            );
            dst_px[0] = (out_r * 255.0).round().clamp(0.0, 255.0) as u8;
            dst_px[1] = (out_g * 255.0).round().clamp(0.0, 255.0) as u8;
            dst_px[2] = (out_b * 255.0).round().clamp(0.0, 255.0) as u8;
            dst_px[3] = 255;
        }
    }
}

/// Applies clip color wheels, color grade parameters, and optional 3D LUT to an RGB pixel in `[0.0, 255.0]`.
#[inline]
fn apply_clip_grading_rgb(
    mut r: f32,
    mut g: f32,
    mut b: f32,
    clip: &Clip,
    has_color_adj: bool,
    color_params: &trim_core::ColorGradeParams,
    custom_lut: Option<&CubeLut3D>,
) -> [f32; 3] {
    if let Some(cw) = &clip.color_wheels {
        let luma = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
        let shadow_w = (1.0 - luma * 2.0).clamp(0.0, 1.0);
        let hi_w = ((luma - 0.5) * 2.0).clamp(0.0, 1.0);
        let mid_w = (1.0 - shadow_w - hi_w).clamp(0.0, 1.0);

        let gamma_r = (cw.gamma[0] + cw.luminance[1]).max(0.0);
        let gamma_g = (cw.gamma[1] + cw.luminance[1]).max(0.0);
        let gamma_b = (cw.gamma[2] + cw.luminance[1]).max(0.0);

        let gain_r = (cw.gain[0] + cw.luminance[2]).max(0.0);
        let gain_g = (cw.gain[1] + cw.luminance[2]).max(0.0);
        let gain_b = (cw.gain[2] + cw.luminance[2]).max(0.0);

        r += (cw.lift[0] + cw.luminance[0]) * 128.0 * shadow_w;
        g += (cw.lift[1] + cw.luminance[0]) * 128.0 * shadow_w;
        b += (cw.lift[2] + cw.luminance[0]) * 128.0 * shadow_w;

        r *= 1.0 + (gamma_r - 1.0) * mid_w;
        g *= 1.0 + (gamma_g - 1.0) * mid_w;
        b *= 1.0 + (gamma_b - 1.0) * mid_w;

        r *= 1.0 + (gain_r - 1.0) * hi_w;
        g *= 1.0 + (gain_g - 1.0) * hi_w;
        b *= 1.0 + (gain_b - 1.0) * hi_w;

        r = r.clamp(0.0, 255.0);
        g = g.clamp(0.0, 255.0);
        b = b.clamp(0.0, 255.0);
    }

    if has_color_adj {
        let [mut nr, mut ng, mut nb] = trim_core::color::apply_color_grade_normalized(
            r / 255.0,
            g / 255.0,
            b / 255.0,
            color_params,
        );
        if let Some(lut) = custom_lut {
            [nr, ng, nb] = lut.sample(nr, ng, nb);
        }
        r = (nr * 255.0).clamp(0.0, 255.0);
        g = (ng * 255.0).clamp(0.0, 255.0);
        b = (nb * 255.0).clamp(0.0, 255.0);
    }

    [r, g, b]
}

/// Generates a synthetic title canvas image and saves to ~/.config/trim/titles/.
/// The text overlay itself is rendered dynamically in vectors by the monitor widget
/// and export filtergraph, so the background file stays clean without duplicated text.
#[allow(clippy::too_many_arguments)]
pub fn generate_title_image(
    _text: &str,
    _font_size: u32,
    _text_color_rgb: [u8; 3],
    bg_color_rgba: [u8; 4],
    _position: &str,
    _has_shadow: bool,
    width: u32,
    height: u32,
) -> Option<PathBuf> {
    let titles_dir = trim_core::paths::config_dir().join("titles");
    let _ = std::fs::create_dir_all(&titles_dir);

    let id = uuid::Uuid::new_v4().to_string();
    let out_file = titles_dir.join(format!("title_{}.png", id));

    let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(width, height, Rgba(bg_color_rgba));
    img.save(&out_file).ok()?;
    Some(out_file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_frame_decoder_smoke() {
        let mut decoder = VideoFrameDecoder::new();
        let res = decoder.decode_frame("/tmp/nonexistent_video_file.mp4", 0.0);
        assert!(res.is_none());

        let dummy = "/tmp/sample_trim_test.mp4";
        if std::path::Path::new(dummy).is_file() {
            let frame = decoder.decode_frame(dummy, 0.5);
            assert!(frame.is_some());
            let img = frame.unwrap();
            assert!(img.width() > 0 && img.height() > 0);
        }

        let user_file = "/home/mozcelik/Videolar/İsimsiz Proje.mp4";
        if std::path::Path::new(user_file).is_file() {
            let frame = decoder.decode_frame(user_file, 11.467);
            assert!(frame.is_some());
        }
    }

    #[test]
    fn test_image_preview_clears_at_end_and_during_gaps() {
        use tempfile::tempdir;
        use trim_core::clip::Clip;
        use trim_core::media::MediaItem;
        use trim_core::project::Project;

        let dir = tempdir().unwrap();
        let image_path = dir.path().join("preview.png");
        RgbaImage::from_pixel(8, 8, Rgba([200, 30, 40, 255]))
            .save(&image_path)
            .unwrap();

        let mut project = Project::default();
        let mut media = MediaItem::new(&image_path);
        media.media_type = MediaType::Image;
        let media_id = media.id.clone();
        project.add_media(media);

        let track = project
            .timeline
            .get_video_tracks_mut()
            .into_iter()
            .next()
            .unwrap();
        track.add_clip(Clip::new(&media_id, 0.0, 2.0));
        track.add_clip(Clip::new(&media_id, 3.0, 4.0));

        let mut engine = VideoEngine::new();
        assert!(engine.render_timeline_frame(&project, 1.9, false).is_some());
        assert!(engine.last_rendered_frame.is_some());

        // The clip's out-point and the following gap must both be black.
        assert!(engine.render_timeline_frame(&project, 2.0, false).is_none());
        assert!(engine.last_rendered_frame.is_none());
        assert!(!engine.is_decoding_seek);
        assert!(engine.render_timeline_frame(&project, 2.5, false).is_none());

        // Seeking to another image must draw it, then clear at its out-point.
        assert!(engine.render_timeline_frame(&project, 3.0, false).is_some());
        assert!(engine.render_timeline_frame(&project, 4.0, false).is_none());
        assert!(engine.last_rendered_frame.is_none());
    }

    #[test]
    fn test_video_engine_lifecycle() {
        let engine = VideoEngine::new();
        assert!(!engine.streamer.is_active());
        let dummy = "/tmp/sample_trim_test.mp4";
        if std::path::Path::new(dummy).is_file() {
            let frame = decode_frame_raw(dummy, 0.5);
            assert!(frame.is_some());
            let f = frame.unwrap();
            assert_eq!(f.width(), PREVIEW_WIDTH);
            assert_eq!(f.height(), PREVIEW_HEIGHT);
        }
    }

    #[test]
    fn test_playback_streamer_and_async_seek() {
        let dummy = "/home/mozcelik/Videolar/İsimsiz Proje.mp4";
        if std::path::Path::new(dummy).is_file() {
            // Test PlaybackStreamer
            let mut streamer = PlaybackStreamer::new();
            streamer.start("test_clip", dummy, 1.0, 30.0, false, 0.0);
            assert!(streamer.is_active());
            let start = std::time::Instant::now();
            let mut frame = None;
            while start.elapsed().as_millis() < 1500 {
                if let Some(f) = streamer.get_frame_for_time(1.2) {
                    frame = Some(f);
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            assert!(frame.is_some());
            let f = frame.unwrap();
            assert_eq!(f.width(), PREVIEW_WIDTH);
            assert_eq!(f.height(), PREVIEW_HEIGHT);
            streamer.stop();
            assert!(!streamer.is_active());

            // Test AsyncFrameDecoder
            let mut async_dec = AsyncFrameDecoder::new();
            let req_id = async_dec.request_seek(dummy, 2.0);
            assert!(req_id > 0);
            let start = std::time::Instant::now();
            let mut got_result = None;
            while start.elapsed().as_millis() < 1200 {
                if let Some(res) = async_dec.poll_result() {
                    got_result = Some(res);
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
            assert!(got_result.is_some());
            let res = got_result.unwrap();
            let frame = res.frame.expect("should decode test frame");
            assert_eq!(frame.width(), PREVIEW_WIDTH);
            assert_eq!(frame.height(), PREVIEW_HEIGHT);
        }
    }

    #[test]
    fn test_render_timeline_real_file() {
        let path = "/home/mozcelik/Videolar/İsimsiz Proje.mp4";
        if std::path::Path::new(path).exists() {
            let mut project = Project::default();
            let media = trim_core::media::MediaItem::new(path);
            let media_id = media.id.clone();
            project.add_media(media);
            let mut track = trim_core::track::Track::new("V1", trim_core::track::TrackType::Video);
            let clip = trim_core::clip::Clip::new(&media_id, 0.0, 10.0);
            track.add_clip(clip);
            project.timeline.add_track(track);

            let mut engine = VideoEngine::new();
            let f1 = engine.render_timeline_frame(&project, 0.0, false);
            assert!(f1.is_some(), "f1 should be some for real file at time 0.0");

            // Test scrub to 2.0s
            let _ = engine.render_timeline_frame(&project, 2.0, false);
            let start = std::time::Instant::now();
            let mut seek_done = false;
            while start.elapsed().as_millis() < 1200 {
                let f = engine.render_timeline_frame(&project, 2.0, false);
                if !engine.is_decoding_seek {
                    assert!(f.is_some());
                    seek_done = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            assert!(seek_done, "Seek to 2.0s should complete asynchronously");

            // Test playback at 30fps
            let mut got_streamed = false;
            let play_start = std::time::Instant::now();
            let mut t = 2.0;
            while play_start.elapsed().as_millis() < 1500 {
                t += 0.033;
                let f = engine.render_timeline_frame(&project, t, true);
                if let Some(img) = f {
                    assert_eq!(img.width(), PREVIEW_WIDTH);
                    got_streamed = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(30));
            }
            assert!(got_streamed, "Playback should produce frames");
        }
    }

    #[test]
    fn test_render_timeline_with_fade_out() {
        let mut project = Project::default();
        let mut media = trim_core::media::MediaItem::new("/nonexistent_test.mp4");
        media.media_type = trim_core::media::MediaType::Video;
        let media_id = media.id.clone();
        project.add_media(media);

        let mut track = trim_core::track::Track::new("V1", trim_core::track::TrackType::Video);
        let mut clip = trim_core::clip::Clip::new(&media_id, 0.0, 5.0);
        clip.fade_out = 1.5;
        track.add_clip(clip);
        project.timeline.add_track(track);

        let mut engine = VideoEngine::new();
        // Visible at 4.0s (within fade-out period)
        let _ = engine.render_timeline_frame(&project, 4.0, false);
        // Beyond clip at 6.0s (empty, should return None and clean state)
        let beyond = engine.render_timeline_frame(&project, 6.0, false);
        assert!(beyond.is_none());
    }

    #[test]
    fn test_reverse_playback_streamer_flags() {
        let mut streamer = PlaybackStreamer::new();
        assert!(!streamer.is_reversed());
        streamer.start("clip_rev", "/nonexistent.mp4", 5.0, 30.0, true, 1.0);
        assert!(streamer.is_reversed());
        assert_eq!(streamer.last_pts(), 5.0);
        assert!(!streamer.should_restart(5.0, true));
        assert!(streamer.should_restart(5.0, false));
        streamer.stop();
        assert!(!streamer.is_active());
    }

    #[test]
    fn test_reverse_playback_streamer_real_frames() {
        let test_file = "/tmp/test_trim_rev.mp4";
        if std::path::Path::new(test_file).is_file() {
            let mut streamer = PlaybackStreamer::new();
            streamer.start("clip_rev_real", test_file, 4.0, 30.0, true, 0.0);
            assert!(streamer.is_active());
            assert!(streamer.is_reversed());

            let start = std::time::Instant::now();
            let mut got_frame = false;
            let mut t = 4.0;
            while start.elapsed().as_millis() < 2000 {
                t -= 0.033;
                if let Some(f) = streamer.get_frame_for_time(t) {
                    assert_eq!(f.width(), PREVIEW_WIDTH);
                    assert_eq!(f.height(), PREVIEW_HEIGHT);
                    got_frame = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            assert!(
                got_frame,
                "Streamer should deliver reversed frames from real video"
            );
            assert!(streamer.frames_delivered() > 0);
            streamer.stop();
            assert!(!streamer.is_active());
        }
    }

    #[test]
    fn test_chromakey_rendering_removes_key_color() {
        use trim_core::clip::{ChromaKeyConfig, Clip};

        let mut canvas = ImageBuffer::from_pixel(10, 10, Rgba([0, 0, 255, 255])); // Blue canvas background

        // Frame with pure green on left half (x < 5) and bright red on right half (x >= 5)
        let mut frame = ImageBuffer::from_pixel(10, 10, Rgba([0, 255, 0, 255]));
        for y in 0..10 {
            for x in 5..10 {
                frame.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }

        let mut clip = Clip::new("m1", 0.0, 5.0);
        clip.chromakey = Some(ChromaKeyConfig {
            enabled: true,
            key_color: [0, 255, 0],
            similarity: 0.25,
            smoothness: 0.05,
        });

        draw_clip_onto_canvas(&mut canvas, &clip, &frame, 1.0);

        // Keyed out area (left half) should retain the background canvas color (Blue)
        let keyed_px = canvas.get_pixel(2, 2);
        assert_eq!(keyed_px[0], 0, "Red channel should be 0 on keyed area");
        assert_eq!(
            keyed_px[1], 0,
            "Green channel should be keyed out (not green!)"
        );
        assert_eq!(keyed_px[2], 255, "Blue background should show through");

        // Non-keyed area (right half) should show the red foreground from the clip
        let red_px = canvas.get_pixel(7, 2);
        assert_eq!(red_px[0], 255, "Red foreground should be rendered");
        assert_eq!(red_px[1], 0);
        assert_eq!(red_px[2], 0);
    }

    #[test]
    fn test_color_wheels_rendering_modifies_pixels() {
        use trim_core::clip::{Clip, ColorWheelsConfig};

        let mut canvas = ImageBuffer::from_pixel(10, 10, Rgba([0, 0, 0, 255]));
        let frame = ImageBuffer::from_pixel(10, 10, Rgba([128, 128, 128, 255]));

        let mut clip = Clip::new("m1", 0.0, 5.0);
        clip.color_wheels = Some(ColorWheelsConfig {
            lift: [0.2, 0.0, -0.2],
            gamma: [1.2, 1.0, 0.8],
            gain: [1.2, 1.0, 0.8],
            luminance: [0.0, 0.1, 0.0],
        });

        draw_clip_onto_canvas(&mut canvas, &clip, &frame, 1.0);

        let modified_px = canvas.get_pixel(5, 5);
        assert!(
            modified_px[0] > modified_px[1],
            "Red should be boosted over Green"
        );
        assert!(
            modified_px[1] > modified_px[2],
            "Green should be higher than Blue"
        );
    }

    #[test]
    fn test_image_chromakey_bypasses_fast_path() {
        use trim_core::clip::{ChromaKeyConfig, Clip};
        use trim_core::media::{MediaItem, MediaType};

        let temp_dir = std::env::temp_dir();
        let img_path = temp_dir.join("test_trim_ck_bypass.png");

        // Create 640x360 green image
        let green_img: RgbaImage = ImageBuffer::from_pixel(
            PREVIEW_WIDTH,
            PREVIEW_HEIGHT,
            Rgba([0u8, 255u8, 0u8, 255u8]),
        );
        green_img
            .save(&img_path)
            .expect("Failed to save temp test image");

        let mut project = Project::default();
        let mut media = MediaItem::new(img_path.to_str().unwrap());
        media.media_type = MediaType::Image;
        let m_id = media.id.clone();
        project.add_media(media);

        let mut track = trim_core::track::Track::new("V1", trim_core::track::TrackType::Video);
        let mut clip = Clip::new(&m_id, 0.0, 5.0);
        clip.chromakey = Some(ChromaKeyConfig {
            enabled: true,
            key_color: [0, 255, 0],
            similarity: 0.3,
            smoothness: 0.05,
        });
        track.add_clip(clip);
        project.timeline.add_track(track);

        let mut engine = VideoEngine::new();
        let rendered = engine.render_timeline_frame(&project, 1.0, false);
        assert!(rendered.is_some(), "Should render a frame");

        let frame = rendered.unwrap();
        // Since green was keyed out on single clip, canvas background is black [0, 0, 0, 255]
        let px = frame.get_pixel(100, 100);
        assert_eq!(
            px[1], 0,
            "Green should be removed by Chroma Key instead of returning raw green frame"
        );

        let _ = std::fs::remove_file(img_path);
    }

    #[test]
    fn test_video_engine_frame_cache_lru_bounded() {
        let engine = VideoEngine::new();
        let dummy_frame = Arc::new(ImageBuffer::from_pixel(16, 16, Rgba([0, 0, 0, 255])));

        {
            let mut cache = engine.frame_cache.write().unwrap();
            for i in 0..400 {
                let key = ("test_media.mp4".to_string(), i);
                cache.insert(key, Arc::clone(&dummy_frame));
            }
            // Max capacity is 300, so size should be capped at 300
            assert_eq!(cache.len(), 300);
            // First 100 items (0..100) should have been evicted
            for i in 0..100 {
                let key = ("test_media.mp4".to_string(), i);
                assert!(
                    cache.get(&key).is_none(),
                    "Key {} should have been evicted",
                    i
                );
            }
            // Items 100..400 should be present
            for i in 100..400 {
                let key = ("test_media.mp4".to_string(), i);
                assert!(cache.get(&key).is_some(), "Key {} should be in cache", i);
            }
        }
    }

    #[test]
    fn test_parse_cube_lut_str_unwraps_3d_strip() {
        let cube_content = "\
# Test 2x2x2 3D LUT
TITLE \"UnitTestLUT\"
LUT_3D_SIZE 2
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
";
        let lut = parse_cube_lut_str(cube_content).expect("Should parse valid 2x2x2 .cube LUT");
        assert_eq!(lut.size, 2);
        // 2x2x2 = 8 RGBA texels = 32 bytes
        assert_eq!(lut.rgba_strip.len(), 32);
        // Entry 0: (0,0,0) -> [0, 0, 0, 255]
        assert_eq!(&lut.rgba_strip[0..4], &[0, 0, 0, 255]);
        // Entry 1: (1,0,0) -> [255, 0, 0, 255]
        assert_eq!(&lut.rgba_strip[4..8], &[255, 0, 0, 255]);
        // Entry 7: (1,1,1) -> [255, 255, 255, 255]
        assert_eq!(&lut.rgba_strip[28..32], &[255, 255, 255, 255]);
    }

    #[test]
    fn test_collect_timeline_layers_multitrack_order() {
        use tempfile::tempdir;
        use trim_core::clip::Clip;
        use trim_core::media::MediaItem;
        use trim_core::project::Project;

        let dir = tempdir().unwrap();
        let img1_path = dir.path().join("layer1.png");
        let img2_path = dir.path().join("layer2.png");
        RgbaImage::from_pixel(16, 16, Rgba([255, 0, 0, 255]))
            .save(&img1_path)
            .unwrap();
        RgbaImage::from_pixel(16, 16, Rgba([0, 255, 0, 255]))
            .save(&img2_path)
            .unwrap();

        let mut project = Project::default();
        let mut m1 = MediaItem::new(&img1_path);
        m1.media_type = MediaType::Image;
        let m1_id = m1.id.clone();
        project.add_media(m1);

        let mut m2 = MediaItem::new(&img2_path);
        m2.media_type = MediaType::Image;
        let m2_id = m2.id.clone();
        project.add_media(m2);

        let v1_id = project
            .timeline
            .tracks
            .iter()
            .find(|t| t.name == "V1")
            .unwrap()
            .id
            .clone();
        let v2_id = project
            .timeline
            .tracks
            .iter()
            .find(|t| t.name == "V2")
            .unwrap()
            .id
            .clone();

        project
            .timeline
            .get_track_by_id_mut(&v1_id)
            .unwrap()
            .add_clip(Clip::new(&m1_id, 0.0, 5.0));
        project
            .timeline
            .get_track_by_id_mut(&v2_id)
            .unwrap()
            .add_clip(Clip::new(&m2_id, 0.0, 5.0));

        let mut engine = VideoEngine::new();
        let layers = engine.collect_timeline_layers(&project, 1.0, false);
        assert_eq!(layers.len(), 2);
        // Bottom track (V1) comes first, top track (V2) comes second for alpha blending
        assert_eq!(layers[0].clip.media_id, m1_id);
        assert_eq!(layers[1].clip.media_id, m2_id);
    }

    #[test]
    fn test_advanced_color_presets_rendering() {
        use trim_core::clip::{BrightnessContrastPreset, Clip, LutPreset, WhiteBalancePreset};

        let frame = ImageBuffer::from_pixel(8, 8, Rgba([128, 128, 128, 255]));

        // 1. Warm White Balance boosts Red and reduces Blue
        let mut canvas_wb = ImageBuffer::from_pixel(8, 8, Rgba([0, 0, 0, 255]));
        let mut clip_wb = Clip::new("m1", 0.0, 5.0);
        clip_wb.apply_wb_preset(WhiteBalancePreset::Warm);
        draw_clip_onto_canvas(&mut canvas_wb, &clip_wb, &frame, 1.0);
        let px_wb = canvas_wb.get_pixel(4, 4);
        assert!(px_wb[0] > 128, "Warm WB should increase red channel");
        assert!(px_wb[2] < 128, "Warm WB should decrease blue channel");

        // 2. Dark / Moody BC preset darkens midtones via brightness + gamma
        let mut canvas_bc = ImageBuffer::from_pixel(8, 8, Rgba([0, 0, 0, 255]));
        let mut clip_bc = Clip::new("m1", 0.0, 5.0);
        clip_bc.apply_bc_preset(BrightnessContrastPreset::DarkMoody);
        draw_clip_onto_canvas(&mut canvas_bc, &clip_bc, &frame, 1.0);
        let px_bc = canvas_bc.get_pixel(4, 4);
        assert!(px_bc[0] < 128, "DarkMoody preset should darken midtones");

        // 3. Sepia look produces warm R > G > B ordering
        let mut canvas_sepia = ImageBuffer::from_pixel(8, 8, Rgba([0, 0, 0, 255]));
        let mut clip_sepia = Clip::new("m1", 0.0, 5.0);
        clip_sepia.lut_preset = LutPreset::Sepia;
        draw_clip_onto_canvas(&mut canvas_sepia, &clip_sepia, &frame, 1.0);
        let px_sepia = canvas_sepia.get_pixel(4, 4);
        assert!(px_sepia[0] > px_sepia[1] && px_sepia[1] > px_sepia[2]);
    }

    #[test]
    fn test_analyze_clip_auto_wb_on_image_clip() {
        use tempfile::tempdir;
        use trim_core::clip::Clip;
        use trim_core::media::MediaItem;
        use trim_core::project::Project;

        let dir = tempdir().unwrap();
        let img_path = dir.path().join("blue_cast.png");
        // Image with strong blue cast
        RgbaImage::from_pixel(32, 32, Rgba([90, 125, 185, 255]))
            .save(&img_path)
            .unwrap();

        let mut project = Project::default();
        let mut media = MediaItem::new(&img_path);
        media.media_type = MediaType::Image;
        let media_id = media.id.clone();
        project.add_media(media);

        let clip = Clip::new(&media_id, 0.0, 5.0);
        let mut engine = VideoEngine::new();
        let (temp, _tint) = engine
            .analyze_clip_auto_wb(&project, &clip, 1.0)
            .expect("Should analyze auto WB for image clip");
        assert!(
            temp > 10.0,
            "Blue-cast image should produce warm positive temperature compensation, got {temp}"
        );
    }

    #[test]
    fn test_cpu_blend_modes_masks_and_adjustment_layer() {
        use trim_core::{BlendMode, Clip, Mask, MaskShape, MaskTarget};

        // 1. BlendMode::Multiply over mid-gray (128) with red (255, 0, 0) => (128, 0, 0)
        let mut canvas = ImageBuffer::from_pixel(16, 16, Rgba([128, 128, 128, 255]));
        let top_frame = ImageBuffer::from_pixel(16, 16, Rgba([255, 0, 0, 255]));
        let mut top_clip = Clip::new("m_top", 0.0, 5.0);
        top_clip.blend_mode = BlendMode::Multiply;
        draw_clip_onto_canvas(&mut canvas, &top_clip, &top_frame, 1.0);
        let px = canvas.get_pixel(8, 8);
        assert!(
            px[0] > 120 && px[1] < 5 && px[2] < 5,
            "Multiply failed: {:?}",
            px
        );

        // 2. Rectangle ClipAlpha mask on top clip keeps corners untouched
        let mut canvas_mask = ImageBuffer::from_pixel(32, 32, Rgba([10, 20, 30, 255]));
        let white_frame = ImageBuffer::from_pixel(32, 32, Rgba([250, 250, 250, 255]));
        let mut masked_clip = Clip::new("m_mask", 0.0, 5.0);
        let mut rect_mask = Mask::new(MaskShape::Rectangle, "Center Box");
        rect_mask.scale_x = 0.4;
        rect_mask.scale_y = 0.4;
        rect_mask.feather = 0.0;
        rect_mask.target = MaskTarget::ClipAlpha;
        masked_clip.masks.push(rect_mask);
        draw_clip_onto_canvas(&mut canvas_mask, &masked_clip, &white_frame, 1.0);
        assert!(
            canvas_mask.get_pixel(16, 16)[0] > 240,
            "Center inside mask should be white"
        );
        assert_eq!(
            canvas_mask.get_pixel(1, 1)[0],
            10,
            "Corner outside mask should remain backdrop"
        );

        // 3. Adjustment Layer with brightness boost + ColorAndEffects mask affects only center
        let mut canvas_adj = ImageBuffer::from_pixel(32, 32, Rgba([100, 100, 100, 255]));
        let mut adj = Clip::new_adjustment_layer(0.0, 5.0);
        adj.brightness = 0.4;
        let mut adj_mask = Mask::new(MaskShape::Rectangle, "Grade Box");
        adj_mask.scale_x = 0.4;
        adj_mask.scale_y = 0.4;
        adj_mask.feather = 0.0;
        adj_mask.target = MaskTarget::ColorAndEffects;
        adj.masks.push(adj_mask);
        let sentinel = ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0]));
        draw_clip_onto_canvas(&mut canvas_adj, &adj, &sentinel, 1.0);
        assert!(
            canvas_adj.get_pixel(16, 16)[0] > 170,
            "Adjustment layer center should be brightened: {:?}",
            canvas_adj.get_pixel(16, 16)
        );
        assert_eq!(
            canvas_adj.get_pixel(1, 1)[0],
            100,
            "Adjustment layer outside ColorAndEffects mask should keep backdrop"
        );
    }
}
