//! Video frame extraction, high-speed asynchronous streaming, software composition, and color grading.

use image::{ImageBuffer, Rgba, RgbaImage};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, RwLock};
use trim_core::clip::Clip;
use trim_core::media::MediaType;
use trim_core::project::Project;

pub const PREVIEW_WIDTH: u32 = 640;
pub const PREVIEW_HEIGHT: u32 = 360;
pub const PREVIEW_FRAME_BYTES: usize = (PREVIEW_WIDTH * PREVIEW_HEIGHT * 4) as usize;

/// Fast, single-frame raw RGBA extraction via ffmpeg pipe.
pub fn decode_frame_raw<P: AsRef<Path>>(file_path: P, source_time: f64) -> Option<RgbaImage> {
    let p = file_path.as_ref();
    let p_str = p.to_string_lossy().to_string();

    let mut child = Command::new("ffmpeg")
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

    pub fn active_clip_id(&self) -> Option<&str> {
        self.active_clip_id.as_deref()
    }

    /// Starts or restarts continuous video streaming for given file from `source_start_time`.
    pub fn start(&mut self, clip_id: &str, file_path: &str, source_start_time: f64, fps: f64) {
        self.stop();

        let path_str = file_path.to_string();
        let clip_str = clip_id.to_string();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = stop_flag.clone();
        self.pending_future_frame = None;

        // Bounded queue of 15 frames (~0.5 second buffer)
        let (tx, rx): (SyncSender<(f64, Arc<RgbaImage>)>, Receiver<(f64, Arc<RgbaImage>)>) = sync_channel(15);

        let thread_path = path_str.clone();
        let thread_fps = if fps > 1.0 { fps } else { 30.0 };

        let handle = std::thread::spawn(move || {
            let mut child = match Command::new("ffmpeg")
                .args([
                    "-v",
                    "quiet",
                    "-ss",
                    &format!("{:.3}", source_start_time.max(0.0)),
                    "-i",
                    &thread_path,
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
            {
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

                let pts = source_start_time + (frame_idx as f64 / thread_fps);
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
        // First check if pending future frame is now due
        if let Some((pts, _)) = &self.pending_future_frame {
            if *pts <= target_source_time + 0.015 {
                let (p, frame) = self.pending_future_frame.take().unwrap();
                self.current_frame = Some(frame);
                self.last_pts = p;
            }
        }

        if self.pending_future_frame.is_none() {
            if let Some(rx) = &self.frame_rx {
                while let Ok((pts, frame)) = rx.try_recv() {
                    if pts <= target_source_time + 0.015 {
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
    pub frame_cache: Arc<RwLock<HashMap<(String, u64), Arc<RgbaImage>>>>,
    pub image_cache: HashMap<String, RgbaImage>,
    pub last_rendered_frame: Option<Arc<RgbaImage>>,
    pub is_decoding_seek: bool,
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
            last_rendered_frame: None,
            is_decoding_seek: false,
        }
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

    /// Primary timeline rendering entry point with zero UI lag.
    pub fn render_timeline_frame(
        &mut self,
        project: &Project,
        timeline_time: f64,
        is_playing: bool,
    ) -> Option<Arc<RgbaImage>> {
        let visible_clips = project.timeline.get_visible_video_clips_at(timeline_time);
        if visible_clips.is_empty() {
            // Never retain the final frame when the playhead leaves a clip.
            // Invalidate outstanding seek requests so late results cannot
            // repopulate a blank monitor after a timeline gap.
            self.streamer.stop();
            self.last_rendered_frame = None;
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
                    self.last_rendered_frame = Some(frame);
                }
            }
        }

        // Fast Path: Single full-screen video/image clip without opacity or complex effects
        if visible_clips.len() == 1 {
            let (_track, clip) = visible_clips[0];
            let media_item = match project.get_media(&clip.media_id) {
                Some(m) => m,
                None => return None,
            };

            let is_standard_transform = clip.opacity >= 0.99
                && clip.fade_in <= 0.0
                && clip.fade_out <= 0.0
                && clip.brightness == 0.0
                && clip.contrast == 1.0
                && clip.saturation == 1.0
                && clip.scale == 1.0
                && clip.pos_x == 0.0
                && clip.pos_y == 0.0;

            if is_standard_transform {
                if media_item.media_type == MediaType::Image {
                    if let Some(img) = self.get_image(&media_item.file_path) {
                        let arc_img = Arc::new(img.clone());
                        self.last_rendered_frame = Some(arc_img.clone());
                        return Some(arc_img);
                    }
                } else {
                    let src_t = clip.map_timeline_to_source(timeline_time);

                    if is_playing {
                        // Playback Mode: Use streaming rawvideo pipe
                        let needs_restart = match self.streamer.active_clip_id() {
                            Some(id) => id != clip.id,
                            None => true,
                        };

                        if needs_restart {
                            self.streamer.start(
                                &clip.id,
                                &media_item.file_path,
                                src_t,
                                project.timeline.fps,
                            );
                        }

                        if let Some(frame) = self.streamer.get_frame_for_time(src_t) {
                            self.last_rendered_frame = Some(frame.clone());
                            return Some(frame);
                        }
                    } else {
                        // Scrubbing/Paused Mode: Stop streamer, check cache, or request async seek
                        if self.streamer.is_active() {
                            self.streamer.stop();
                        }

                        let ts_key = (src_t * 30.0).round() as u64;
                        let key = (media_item.file_path.clone(), ts_key);

                        if let Ok(cache) = self.frame_cache.read() {
                            if let Some(cached) = cache.get(&key) {
                                self.last_rendered_frame = Some(cached.clone());
                                self.is_decoding_seek = false;
                                return Some(cached.clone());
                            }
                        }

                        if self.last_rendered_frame.is_none() {
                            // Immediate first frame decode so monitor is never blank on initial clip placement
                            if let Some(frame) = decode_frame_raw(&media_item.file_path, src_t) {
                                let arc = Arc::new(frame);
                                if let Ok(mut cache) = self.frame_cache.write() {
                                    cache.insert(key, arc.clone());
                                }
                                self.last_rendered_frame = Some(arc.clone());
                                self.is_decoding_seek = false;
                                return Some(arc);
                            }
                        }

                        // Trigger non-blocking async seek if not already in flight
                        if !self.is_decoding_seek {
                            self.async_decoder.request_seek(&media_item.file_path, src_t);
                            self.is_decoding_seek = true;
                        }
                    }

                    return self.last_rendered_frame.clone();
                }
            }
        }

        // Layered Composite Path: Multiple clips, transitions, opacity, fades, keyframes
        let mut canvas = ImageBuffer::from_pixel(PREVIEW_WIDTH, PREVIEW_HEIGHT, Rgba([14, 14, 16, 255]));

        for (_track, clip) in visible_clips {
            let media_item = match project.get_media(&clip.media_id) {
                Some(m) => m,
                None => continue,
            };

            let frame_img = match media_item.media_type {
                MediaType::Image => self.get_image(&media_item.file_path).cloned(),
                _ => {
                    let src_t = clip.map_timeline_to_source(timeline_time);
                    let ts_key = (src_t * 30.0).round() as u64;
                    let key = (media_item.file_path.clone(), ts_key);

                    let cached = self.frame_cache.read().ok().and_then(|c| c.get(&key).cloned());
                    if let Some(c) = cached {
                        Some((*c).clone())
                    } else if self.last_rendered_frame.is_none() {
                        if let Some(frame) = decode_frame_raw(&media_item.file_path, src_t) {
                            let arc = Arc::new(frame.clone());
                            if let Ok(mut cache) = self.frame_cache.write() {
                                cache.insert(key, arc);
                            }
                            self.last_rendered_frame = Some(Arc::new(frame.clone()));
                            self.is_decoding_seek = false;
                            Some(frame)
                        } else {
                            None
                        }
                    } else {
                        if !self.is_decoding_seek {
                            self.async_decoder.request_seek(&media_item.file_path, src_t);
                            self.is_decoding_seek = true;
                        }
                        self.last_rendered_frame.as_ref().map(|f| (**f).clone())
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
    frame_cache: HashMap<(String, u64), RgbaImage>,
}

impl Default for VideoFrameDecoder {
    fn default() -> Self {
        Self {
            image_cache: HashMap::new(),
            frame_cache: HashMap::new(),
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

    let mut eff_opacity = animated_opacity.clamp(0.0, 1.0);
    let dt = current_time - clip.timeline_in;
    let rem = clip.timeline_out - current_time;

    let mut fade_factor = 1.0;
    if clip.fade_in > 0.0 && dt < clip.fade_in {
        let f = (dt / clip.fade_in).clamp(0.0, 1.0);
        eff_opacity *= f;
        fade_factor = f;
    }
    if clip.fade_out > 0.0 && rem < clip.fade_out {
        let f = (rem / clip.fade_out).clamp(0.0, 1.0);
        eff_opacity *= f;
        fade_factor = f;
    }

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
    let eff_scale = scale_fit * animated_scale.max(0.05);

    let final_w = (fw * eff_scale).round() as i64;
    let final_h = (fh * eff_scale).round() as i64;

    let final_x = ((cw - final_w as f64) / 2.0 + animated_pos_x).round() as i64;
    let final_y = ((ch - final_h as f64) / 2.0 + animated_pos_y).round() as i64;

    let has_dip_white = (clip.transition_in.as_deref() == Some("dip_white") && fade_factor < 1.0)
        || (clip.transition_out.as_deref() == Some("dip_white") && fade_factor < 1.0);
    let dip_white_amt = if has_dip_white { (1.0 - fade_factor) as f32 } else { 0.0 };

    let has_color_adj = clip.brightness != 0.0 || clip.contrast != 1.0 || clip.saturation != 1.0;

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

            let sx = ((x as f64 / final_w as f64) * fw) as u32;
            let sx = sx.min(frame.width() - 1);

            let src_px = frame.get_pixel(sx, sy);
            let mut r = src_px[0] as f32;
            let mut g = src_px[1] as f32;
            let mut b = src_px[2] as f32;
            let a = (src_px[3] as f32 / 255.0) * eff_opacity as f32;

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

/// Generates a synthetic title image and saves to ~/.local/share/trim/titles/.
pub fn generate_title_image(
    text: &str,
    font_size: u32,
    text_color_rgb: [u8; 3],
    bg_color_rgba: [u8; 4],
    width: u32,
    height: u32,
) -> Option<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let titles_dir = PathBuf::from(home).join(".local").join("share").join("trim").join("titles");
    let _ = std::fs::create_dir_all(&titles_dir);

    let id = uuid::Uuid::new_v4().to_string();
    let out_file = titles_dir.join(format!("title_{}.png", id));

    let img = ImageBuffer::from_pixel(width, height, Rgba(bg_color_rgba));

    let hex_color = format!("{:02x}{:02x}{:02x}", text_color_rgb[0], text_color_rgb[1], text_color_rgb[2]);

    let status = Command::new("ffmpeg")
        .args([
            "-v",
            "quiet",
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("color=c=black@0.0:s={}x{}:r=1:d=1", width, height),
            "-vf",
            &format!(
                "drawtext=text='{}':fontcolor=0x{}:fontsize={}:x=(w-text_w)/2:y=(h-text_h)/2",
                text.replace('\'', "\\'"),
                hex_color,
                font_size
            ),
            "-vframes",
            "1",
            out_file.to_str().unwrap_or(""),
        ])
        .status();

    if status.map(|s| s.success()).unwrap_or(false) && out_file.is_file() {
        Some(out_file)
    } else {
        let _ = img.save(&out_file);
        Some(out_file)
    }
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
            streamer.start("test_clip", dummy, 1.0, 30.0);
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
}
