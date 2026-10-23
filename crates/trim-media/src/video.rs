//! Video frame extraction, high-speed asynchronous streaming, software composition, and color grading.

use image::{ImageBuffer, Rgba, RgbaImage};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, RwLock};
use trim_core::clip::Clip;
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
pub type FrameCacheMap = Arc<RwLock<HashMap<(String, u64), Arc<RgbaImage>>>>;

/// Fast, single-frame raw RGBA extraction via ffmpeg pipe.
pub fn decode_frame_raw<P: AsRef<Path>>(file_path: P, source_time: f64) -> Option<RgbaImage> {
    let p = file_path.as_ref();
    let p_str = p.to_string_lossy().to_string();

    let mut child = crate::ffmpeg::ffmpeg_cmd()
        .args([
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
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let mut buf = vec![0u8; PREVIEW_FRAME_BYTES];
    if let Some(mut stdout) = child.stdout.take() {
        if stdout.read_exact(&mut buf).is_err() {
            let _ = child.kill();
            return None;
        }
    }
    let _ = child.wait();

    ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(PREVIEW_WIDTH, PREVIEW_HEIGHT, buf)
}

/// Continuous background rawvideo pipe streamer for real-time 140+ FPS playback.
pub struct PlaybackStreamer {
    active_path: Option<String>,
    active_clip_id: Option<String>,
    frame_rx: Option<Receiver<(f64, Arc<RgbaImage>)>>,
    stop_flag: Option<Arc<AtomicBool>>,
    child_handle: Option<std::thread::JoinHandle<()>>,
    current_frame: Option<Arc<RgbaImage>>,
    last_pts: f64,
    pending_future_frame: Option<(f64, Arc<RgbaImage>)>,
    is_reversed: bool,
}

impl Default for PlaybackStreamer {
    fn default() -> Self {
        Self {
            active_path: None,
            active_clip_id: None,
            frame_rx: None,
            stop_flag: None,
            child_handle: None,
            current_frame: None,
            last_pts: 0.0,
            pending_future_frame: None,
            is_reversed: false,
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

    /// Starts or restarts continuous video streaming for given file from `source_start_time`.
    /// When `reverse` is true, video decodes backwards from `source_start_time` down to `source_in`.
    pub fn start(&mut self, clip_id: &str, file_path: &str, source_start_time: f64, fps: f64, reverse: bool, source_in: f64) {
        self.stop();

        let path_str = file_path.to_string();
        let clip_str = clip_id.to_string();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = stop_flag.clone();
        self.pending_future_frame = None;
        self.is_reversed = reverse;

        // Bounded queue of 15 frames (~0.5 second buffer)
        let (tx, rx): (SyncSender<DecodedFrame>, Receiver<DecodedFrame>) = sync_channel(15);

        let thread_path = path_str.clone();
        let thread_fps = if fps > 1.0 { fps } else { 30.0 };

        let handle = std::thread::spawn(move || {
            let vf_str = if reverse {
                format!(
                    "reverse,fps=fps={thread_fps:.3},scale=640:360:force_original_aspect_ratio=decrease,pad=640:360:(ow-iw)/2:(oh-ih)/2:black"
                )
            } else {
                format!(
                    "fps=fps={thread_fps:.3},scale=640:360:force_original_aspect_ratio=decrease,pad=640:360:(ow-iw)/2:(oh-ih)/2:black"
                )
            };

            let mut cmd = crate::ffmpeg::ffmpeg_cmd();
            cmd.arg("-v").arg("quiet");

            if reverse {
                let start = source_in.max(0.0);
                let dur = (source_start_time - start).max(0.05);
                cmd.args(["-ss", &format!("{:.3}", start), "-t", &format!("{:.3}", dur)]);
            } else {
                cmd.args(["-ss", &format!("{:.3}", source_start_time.max(0.0))]);
            }

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

            let mut frame_idx = 0u64;

            while !stop_clone.load(Ordering::Relaxed) {
                let mut buf = vec![0u8; PREVIEW_FRAME_BYTES];
                if stdout.read_exact(&mut buf).is_err() {
                    break;
                }

                let pts = if reverse {
                    source_start_time - (frame_idx as f64 / thread_fps)
                } else {
                    source_start_time + (frame_idx as f64 / thread_fps)
                };
                frame_idx += 1;

                if let Some(img) = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(PREVIEW_WIDTH, PREVIEW_HEIGHT, buf) {
                    if tx.send((pts, Arc::new(img))).is_err() {
                        break;
                    }
                } else {
                    break;
                }
            }

            let _ = child.kill();
            let _ = child.wait();
        });

        self.active_path = Some(path_str);
        self.active_clip_id = Some(clip_str);
        self.frame_rx = Some(rx);
        self.stop_flag = Some(stop_flag);
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

    /// Stops the background streaming process.
    pub fn stop(&mut self) {
        if let Some(flag) = &self.stop_flag {
            flag.store(true, Ordering::Relaxed);
        }
        self.frame_rx = None;
        self.stop_flag = None;
        self.child_handle = None;
        self.active_path = None;
        self.active_clip_id = None;
        self.pending_future_frame = None;
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
    latest_req_id: u64,
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
            frame_cache: Arc::new(RwLock::new(HashMap::new())),
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

    pub fn get_image<P: AsRef<Path>>(&mut self, path: P) -> Option<Arc<RgbaImage>> {
        let p_str = path.as_ref().to_string_lossy().to_string();
        if !self.image_cache.contains_key(&p_str) {
            if let Ok(dyn_img) = image::open(&p_str) {
                self.image_cache.insert(p_str.clone(), Arc::new(dyn_img.to_rgba8()));
            } else {
                return None;
            }
        }
        self.image_cache.get(&p_str).cloned()
    }

    /// Primary timeline rendering entry point with zero UI lag.
    pub fn render_timeline_frame(
        &mut self,
        project: &Project,
        timeline_time: f64,
        is_playing: bool,
    ) -> Option<Arc<RgbaImage>> {
        let (canvas_w, canvas_h) = calculate_preview_size(project.timeline.width, project.timeline.height);

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
                        if cache.len() > 300 {
                            cache.clear();
                        }
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
                && clip.brightness == 0.0
                && clip.contrast == 1.0
                && clip.saturation == 1.0
                && clip.lut_preset == trim_core::clip::LutPreset::None
                && clip.custom_lut_path.is_none()
                && clip.scale == 1.0
                && clip.pos_x == 0.0
                && clip.pos_y == 0.0
                && !clip.reverse
                && (clip.speed - 1.0).abs() < 0.01;

            if media_item.media_type == MediaType::Image {
                if self.is_decoding_seek {
                    self.async_decoder.latest_req_id += 1;
                    self.is_decoding_seek = false;
                }

                let is_title_card = media_item.file_path.contains("titles");
                let img_opt = if clip.text_overlay.is_some() && is_title_card {
                    Some(Arc::new(ImageBuffer::from_pixel(canvas_w, canvas_h, Rgba([0, 0, 0, 0]))))
                } else {
                    self.get_image(&media_item.file_path)
                };

                if let Some(img) = img_opt {
                    if is_standard_transform {
                        self.last_rendered_frame = Some(img.clone());
                        return Some(img);
                    } else {
                        let mut canvas = ImageBuffer::from_pixel(canvas_w, canvas_h, Rgba([0, 0, 0, 255]));
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
                        Some(id) => id != clip.id || (src_t - self.streamer.last_pts()).abs() > 0.3 || self.streamer.is_reversed() != clip.reverse,
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
                            let mut canvas = ImageBuffer::from_pixel(PREVIEW_WIDTH, PREVIEW_HEIGHT, Rgba([0, 0, 0, 255]));
                            draw_clip_onto_canvas(&mut canvas, clip, &frame, timeline_time);
                            let arc_canvas = Arc::new(canvas);
                            self.last_rendered_frame = Some(arc_canvas.clone());
                            return Some(arc_canvas);
                        }
                    } else if let Some(last_raw) = &self.last_raw_source_frame {
                        if is_standard_transform {
                            return Some(last_raw.clone());
                        } else {
                            let mut canvas = ImageBuffer::from_pixel(PREVIEW_WIDTH, PREVIEW_HEIGHT, Rgba([0, 0, 0, 255]));
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

                    if let Ok(cache) = self.frame_cache.read() {
                        if let Some(cached) = cache.get(&key) {
                            self.last_raw_source_frame = Some(cached.clone());
                            self.is_decoding_seek = false;
                            if is_standard_transform {
                                self.last_rendered_frame = Some(cached.clone());
                                return Some(cached.clone());
                            } else {
                                let mut canvas = ImageBuffer::from_pixel(canvas_w, canvas_h, Rgba([0, 0, 0, 255]));
                                draw_clip_onto_canvas(&mut canvas, clip, cached, timeline_time);
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
                                let mut canvas = ImageBuffer::from_pixel(canvas_w, canvas_h, Rgba([0, 0, 0, 255]));
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
                            let mut canvas = ImageBuffer::from_pixel(canvas_w, canvas_h, Rgba([0, 0, 0, 255]));
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
                        Some(Arc::new(ImageBuffer::from_pixel(canvas_w, canvas_h, Rgba([0, 0, 0, 0]))))
                    } else {
                        self.get_image(&media_item.file_path)
                    }
                }
                _ => {
                    let eff_path = media_item.resolved_playback_path(self.use_proxies);
                    let src_t = clip.map_timeline_to_source(timeline_time);
                    let ts_key = (src_t * 30.0).round() as u64;
                    let key = (eff_path.to_string(), ts_key);

                    let streamer_frame = if is_playing && self.streamer.active_clip_id() == Some(&clip.id) {
                        self.streamer.get_frame_for_time(src_t)
                    } else {
                        None
                    };

                    if let Some(sf) = streamer_frame {
                        self.last_raw_source_frame = Some(sf.clone());
                        Some(sf)
                    } else {
                        let cached = self.frame_cache.read().ok().and_then(|c| c.get(&key).cloned());
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
#[derive(Default)]
pub struct VideoFrameDecoder {
    image_cache: HashMap<String, RgbaImage>,
    frame_cache: HashMap<(String, u64), RgbaImage>,
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

    pub fn decode_frame<P: AsRef<Path>>(&mut self, file_path: P, source_time: f64) -> Option<RgbaImage> {
        let p = file_path.as_ref();
        let p_str = p.to_string_lossy().to_string();
        let ts_key = (source_time * 30.0).round() as u64;

        if let Some(cached) = self.frame_cache.get(&(p_str.clone(), ts_key)) {
            return Some(cached.clone());
        }

        if let Some(img) = decode_frame_raw(p, source_time) {
            if self.frame_cache.len() > 150 {
                self.frame_cache.clear();
            }
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
fn draw_clip_onto_canvas(canvas: &mut RgbaImage, clip: &Clip, frame: &RgbaImage, current_time: f64) {
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

    let is_wipe_or_slide = clip.transition_in.as_deref().is_some_and(|t| t.starts_with("wipe") || t.starts_with("slide"))
        || clip.transition_out.as_deref().is_some_and(|t| t.starts_with("wipe") || t.starts_with("slide"));

    let eff_opacity = if is_wipe_or_slide {
        animated_opacity.clamp(0.0, 1.0)
    } else {
        (animated_opacity * fade_factor).clamp(0.0, 1.0)
    };

    if eff_opacity <= 0.001 {
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
    let dip_white_amt = if has_dip_white { (1.0 - fade_factor) as f32 } else { 0.0 };

    let wipe_left_active = (clip.transition_in.as_deref() == Some("wipe_left") && in_progress < 1.0)
        || (clip.transition_out.as_deref() == Some("wipe_left") && out_progress < 1.0);
    let wipe_right_active = (clip.transition_in.as_deref() == Some("wipe_right") && in_progress < 1.0)
        || (clip.transition_out.as_deref() == Some("wipe_right") && out_progress < 1.0);

    let has_chromakey = clip.chromakey.as_ref().is_some_and(|ck| ck.enabled);
    let has_color_wheels = clip.color_wheels.is_some();
    let has_color_adj = clip.brightness != 0.0
        || clip.contrast != 1.0
        || clip.saturation != 1.0
        || clip.lut_preset != trim_core::clip::LutPreset::None
        || has_chromakey
        || has_color_wheels;
    let has_special_trans = has_dip_white || wipe_left_active || wipe_right_active || slide_x_offset.abs() > 0.1;

    let is_fullscreen_fit = final_w == cw as i64
        && final_h == ch as i64
        && final_x == 0
        && final_y == 0
        && (fw - cw).abs() < 0.001
        && (fh - ch).abs() < 0.001;

    if is_fullscreen_fit && !has_color_adj && !has_special_trans {
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

            if let Some(cw) = &clip.color_wheels {
                let luma = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
                let shadow_w = (1.0 - luma * 2.0).clamp(0.0, 1.0);
                let hi_w = ((luma - 0.5) * 2.0).clamp(0.0, 1.0);
                let mid_w = (1.0 - shadow_w - hi_w).clamp(0.0, 1.0);

                r += cw.lift[0] * 128.0 * shadow_w;
                g += cw.lift[1] * 128.0 * shadow_w;
                b += cw.lift[2] * 128.0 * shadow_w;

                r *= 1.0 + (cw.gamma[0] - 1.0) * mid_w;
                g *= 1.0 + (cw.gamma[1] - 1.0) * mid_w;
                b *= 1.0 + (cw.gamma[2] - 1.0) * mid_w;

                r *= 1.0 + (cw.gain[0] - 1.0) * hi_w;
                g *= 1.0 + (cw.gain[1] - 1.0) * hi_w;
                b *= 1.0 + (cw.gain[2] - 1.0) * hi_w;

                r = r.clamp(0.0, 255.0);
                g = g.clamp(0.0, 255.0);
                b = b.clamp(0.0, 255.0);
            }

            // Color adjustments
            if has_color_adj {
                let contrast = clip.contrast as f32;
                let brightness = clip.brightness as f32;
                let saturation = clip.saturation as f32;

                r = (contrast * (r - 128.0) + 128.0 + (brightness * 255.0)).clamp(0.0, 255.0);
                g = (contrast * (g - 128.0) + 128.0 + (brightness * 255.0)).clamp(0.0, 255.0);
                b = (contrast * (b - 128.0) + 128.0 + (brightness * 255.0)).clamp(0.0, 255.0);

                if saturation != 1.0 {
                    let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                    r = (luma + saturation * (r - luma)).clamp(0.0, 255.0);
                    g = (luma + saturation * (g - luma)).clamp(0.0, 255.0);
                    b = (luma + saturation * (b - luma)).clamp(0.0, 255.0);
                }

                match clip.lut_preset {
                    trim_core::clip::LutPreset::None => {}
                    trim_core::clip::LutPreset::TealOrange => {
                        let luma = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
                        if luma < 0.5 {
                            let shadow_w = (1.0 - luma * 2.0).clamp(0.0, 1.0);
                            r = (r * (1.0 - 0.22 * shadow_w)).clamp(0.0, 255.0);
                            g = (g * (1.0 + 0.08 * shadow_w)).clamp(0.0, 255.0);
                            b = (b * (1.0 + 0.32 * shadow_w)).clamp(0.0, 255.0);
                        } else {
                            let hi_w = ((luma - 0.5) * 2.0).clamp(0.0, 1.0);
                            r = (r * (1.0 + 0.25 * hi_w)).clamp(0.0, 255.0);
                            g = (g * (1.0 + 0.08 * hi_w)).clamp(0.0, 255.0);
                            b = (b * (1.0 - 0.22 * hi_w)).clamp(0.0, 255.0);
                        }
                    }
                    trim_core::clip::LutPreset::VintageFilm => {
                        r = (r * 1.06 + 14.0).clamp(0.0, 255.0);
                        g = (g * 1.02 + 10.0).clamp(0.0, 255.0);
                        b = (b * 0.90 + 6.0).clamp(0.0, 255.0);
                    }
                    trim_core::clip::LutPreset::MonochromeNoir => {
                        let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                        let norm = luma / 255.0;
                        let curved = if norm < 0.5 {
                            2.0 * norm * norm
                        } else {
                            1.0 - 2.0 * (1.0 - norm) * (1.0 - norm)
                        };
                        let gray = (curved * 255.0).clamp(0.0, 255.0);
                        r = gray;
                        g = gray;
                        b = gray;
                    }
                    trim_core::clip::LutPreset::CyberpunkNeon => {
                        r = (r * 1.22 + 8.0).clamp(0.0, 255.0);
                        g = (g * 0.88).clamp(0.0, 255.0);
                        b = (b * 1.32 + 16.0).clamp(0.0, 255.0);
                        let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                        r = (luma + 1.25 * (r - luma)).clamp(0.0, 255.0);
                        g = (luma + 1.25 * (g - luma)).clamp(0.0, 255.0);
                        b = (luma + 1.25 * (b - luma)).clamp(0.0, 255.0);
                    }
                }
            }

            // Dip to white
            if dip_white_amt > 0.0 {
                r = r + (255.0 - r) * dip_white_amt;
                g = g + (255.0 - g) * dip_white_amt;
                b = b + (255.0 - b) * dip_white_amt;
            }

            // Alpha composite over canvas
            let dst_px = canvas.get_pixel_mut(cx as u32, cy as u32);
            let dr = dst_px[0] as f32;
            let dg = dst_px[1] as f32;
            let db = dst_px[2] as f32;

            dst_px[0] = (r * a + dr * (1.0 - a)).round() as u8;
            dst_px[1] = (g * a + dg * (1.0 - a)).round() as u8;
            dst_px[2] = (b * a + db * (1.0 - a)).round() as u8;
            dst_px[3] = 255;
        }
    }
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

    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(width, height, Rgba(bg_color_rgba));
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
        use trim_core::clip::Clip;
        use trim_core::media::MediaItem;
        use trim_core::project::Project;
        use tempfile::tempdir;

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

        let track = project.timeline.get_video_tracks_mut().into_iter().next().unwrap();
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
        streamer.stop();
        assert!(!streamer.is_active());
    }
}

