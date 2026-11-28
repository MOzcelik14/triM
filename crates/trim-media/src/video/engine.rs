use super::compositor::draw_clip_onto_canvas;
use super::lut::*;
use super::streamer::*;
use image::{ImageBuffer, Rgba, RgbaImage};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use trim_core::clip::Clip;
use trim_core::lru::LruCache;
use trim_core::media::MediaType;
use trim_core::project::Project;

pub struct VideoEngine {
    pub streamer: PlaybackStreamer,
    pub extra_streamers: HashMap<String, PlaybackStreamer>,
    pub async_decoder: AsyncFrameDecoder,
    pub frame_cache: FrameCacheMap,
    pub image_cache: HashMap<String, Arc<RgbaImage>>,
    pub last_raw_source_frame: Option<Arc<RgbaImage>>,
    pub clip_last_frames: HashMap<String, Arc<RgbaImage>>,
    pub last_rendered_frame: Option<Arc<RgbaImage>>,
    pub is_decoding_seek: bool,
    pub use_proxies: bool,
    pub proxy_mode: trim_core::preferences::ProxyMode,
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
            extra_streamers: HashMap::new(),
            async_decoder: AsyncFrameDecoder::new(),
            frame_cache: Arc::new(RwLock::new(LruCache::with_capacity(300))),
            image_cache: HashMap::new(),
            last_raw_source_frame: None,
            clip_last_frames: HashMap::new(),
            last_rendered_frame: None,
            is_decoding_seek: false,
            use_proxies: false,
            proxy_mode: trim_core::preferences::ProxyMode::Auto,
        }
    }

    /// Resolves whether a media item should play via its 720p proxy or original source
    /// according to `proxy_mode` (`Auto` enables proxy for >=1440p or when `use_proxies` is on).
    pub fn effective_playback_path<'a>(&self, media: &'a trim_core::media::MediaItem) -> &'a str {
        use trim_core::preferences::ProxyMode;
        let use_p = match self.proxy_mode {
            ProxyMode::Off => false,
            ProxyMode::Always => true,
            ProxyMode::Auto => crate::proxy::should_use_proxy(
                media.width,
                media.height,
                self.use_proxies,
                media.proxy_path.is_some(),
            ),
        };
        media.resolved_playback_path(use_p)
    }

    pub fn clear_cache(&mut self) {
        self.streamer.stop();
        for mut s in self.extra_streamers.drain().map(|(_, s)| s) {
            s.stop();
        }
        if let Ok(mut fc) = self.frame_cache.write() {
            fc.clear();
        }
        self.image_cache.clear();
        self.last_raw_source_frame = None;
        self.clip_last_frames.clear();
        self.last_rendered_frame = None;
        self.is_decoding_seek = false;
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_layer_source_frame(
        &mut self,
        clip: &trim_core::clip::Clip,
        eff_path: &str,
        src_t: f64,
        is_playing: bool,
        stream_fps: f64,
        use_primary_streamer: bool,
        target_w: u32,
        target_h: u32,
    ) -> Option<Arc<RgbaImage>> {
        let ts_key = (src_t * 1000.0).round() as u64;
        let key = (eff_path.to_string(), ts_key);

        let streamer_frame = if is_playing {
            let target_streamer = if use_primary_streamer {
                &mut self.streamer
            } else {
                self.extra_streamers.entry(clip.id.clone()).or_default()
            };
            if target_streamer.active_clip_id() == Some(&clip.id) {
                if target_streamer.should_restart(src_t, clip.reverse) {
                    target_streamer.start_sized(
                        &clip.id,
                        eff_path,
                        src_t,
                        stream_fps,
                        clip.reverse,
                        clip.source_in,
                        target_w,
                        target_h,
                    );
                }
                target_streamer.get_frame_for_time(src_t)
            } else if use_primary_streamer && target_streamer.is_active() {
                // Primary streamer busy with another clip; use dedicated per-clip streamer
                let extra = self.extra_streamers.entry(clip.id.clone()).or_default();
                if extra.active_clip_id() != Some(&clip.id)
                    || extra.should_restart(src_t, clip.reverse)
                {
                    extra.start_sized(
                        &clip.id,
                        eff_path,
                        src_t,
                        stream_fps,
                        clip.reverse,
                        clip.source_in,
                        target_w,
                        target_h,
                    );
                }
                extra.get_frame_for_time(src_t)
            } else {
                target_streamer.start_sized(
                    &clip.id,
                    eff_path,
                    src_t,
                    stream_fps,
                    clip.reverse,
                    clip.source_in,
                    target_w,
                    target_h,
                );
                target_streamer.get_frame_for_time(src_t)
            }
        } else {
            if self.streamer.is_active() {
                self.streamer.stop();
            }
            for s in self.extra_streamers.values_mut() {
                if s.is_active() {
                    s.stop();
                }
            }
            None
        };

        if let Some(sf) = streamer_frame {
            if self.clip_last_frames.len() > 16 {
                self.clip_last_frames.clear();
            }
            self.clip_last_frames.insert(clip.id.clone(), sf.clone());
            self.last_raw_source_frame = Some(sf.clone());
            return Some(sf);
        }

        let cached = self
            .frame_cache
            .write()
            .ok()
            .and_then(|mut c| c.get(&key).cloned());
        if let Some(c) = cached {
            self.clip_last_frames.insert(clip.id.clone(), c.clone());
            self.last_raw_source_frame = Some(c.clone());
            self.is_decoding_seek = false;
            return Some(c);
        }

        if let Some(prev) = self.clip_last_frames.get(&clip.id).cloned() {
            if !self.is_decoding_seek {
                self.async_decoder
                    .request_seek_sized(eff_path, src_t, target_w, target_h);
                self.is_decoding_seek = true;
            }
            return Some(prev);
        }

        if let Some(frame) = decode_frame_raw_sized(eff_path, src_t, target_w, target_h) {
            let arc = Arc::new(frame);
            if let Ok(mut cache) = self.frame_cache.write() {
                cache.insert(key, arc.clone());
            }
            if self.clip_last_frames.len() > 16 {
                self.clip_last_frames.clear();
            }
            self.clip_last_frames.insert(clip.id.clone(), arc.clone());
            self.last_raw_source_frame = Some(arc.clone());
            self.is_decoding_seek = false;
            Some(arc)
        } else {
            None
        }
    }

    /// Dynamically sets the maximum number of frames held in the LRU frame cache.
    pub fn set_frame_cache_capacity(&self, capacity: usize) {
        if let Ok(mut fc) = self.frame_cache.write() {
            fc.set_max_entries(capacity);
        }
    }

    /// Returns `(has_active_video, is_cached)` for a given `timeline_time` without mutating LRU order.
    pub fn is_timeline_time_cached(&self, project: &Project, timeline_time: f64) -> (bool, bool) {
        let Ok(cache) = self.frame_cache.read() else {
            return (false, false);
        };
        let mut has_video = false;
        let mut all_ready = true;
        for track in &project.timeline.tracks {
            if track.track_type != trim_core::track::TrackType::Video || track.muted {
                continue;
            }
            for clip in &track.clips {
                if timeline_time < clip.timeline_in || timeline_time >= clip.timeline_out {
                    continue;
                }
                if clip.is_adjustment_layer() {
                    continue;
                }
                has_video = true;
                let Some(media) = project.get_media(&clip.media_id) else {
                    continue;
                };
                if media.media_type == MediaType::Image {
                    continue;
                }
                let eff_path = self.effective_playback_path(media).to_string();
                let src_t = clip.map_timeline_to_source(timeline_time);
                let ts_key = (src_t * 1000.0).round() as u64;
                let hit = cache.peek(&(eff_path.clone(), ts_key)).is_some()
                    || cache
                        .peek(&(eff_path.clone(), ts_key.saturating_sub(1)))
                        .is_some()
                    || cache.peek(&(eff_path, ts_key + 1)).is_some();
                if !hit {
                    all_ready = false;
                }
            }
        }
        (has_video, has_video && all_ready)
    }

    /// Queues an asynchronous background decode for the first uncached video clip at `timeline_time`
    /// when idle (not currently seeking or streaming). Returns `true` if a background decode was queued.
    pub fn prewarm_frame_at_time(&mut self, project: &Project, timeline_time: f64) -> bool {
        if self.is_decoding_seek || self.streamer.is_active() {
            return false;
        }
        for track in &project.timeline.tracks {
            if track.track_type != trim_core::track::TrackType::Video || track.muted {
                continue;
            }
            for clip in &track.clips {
                if timeline_time < clip.timeline_in || timeline_time >= clip.timeline_out {
                    continue;
                }
                if clip.is_adjustment_layer() {
                    continue;
                }
                let Some(media) = project.get_media(&clip.media_id) else {
                    continue;
                };
                if media.media_type != MediaType::Video {
                    continue;
                }
                let eff_path = self.effective_playback_path(media).to_string();
                let src_t = clip.map_timeline_to_source(timeline_time);
                let ts_key = (src_t * 1000.0).round() as u64;
                let already_cached = self
                    .frame_cache
                    .read()
                    .ok()
                    .is_some_and(|c| c.peek(&(eff_path.clone(), ts_key)).is_some());
                if !already_cached {
                    let _ = self.async_decoder.request_seek(&eff_path, src_t);
                    self.is_decoding_seek = true;
                    return true;
                }
            }
        }
        false
    }

    pub fn get_image<P: AsRef<Path>>(&mut self, path: P) -> Option<Arc<RgbaImage>> {
        const MAX_IMAGE_CACHE_ENTRIES: usize = 64;
        let p_str = path.as_ref().to_string_lossy().to_string();
        if !self.image_cache.contains_key(&p_str) {
            let mut reader = image::ImageReader::open(&p_str).ok()?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(16_384);
            limits.max_image_height = Some(16_384);
            limits.max_alloc = Some(512 * 1024 * 1024);
            reader.limits(limits);
            let dyn_img = reader.decode().ok()?;
            if self.image_cache.len() >= MAX_IMAGE_CACHE_ENTRIES {
                if let Some(evict_key) = self.image_cache.keys().next().cloned() {
                    self.image_cache.remove(&evict_key);
                }
            }
            self.image_cache
                .insert(p_str.clone(), Arc::new(dyn_img.to_rgba8()));
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
            let eff_path = self.effective_playback_path(media_item);
            let src_t = clip.map_timeline_to_source(sample_tl_time);
            let ts_key = (src_t * 1000.0).round() as u64;
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
                    let key = (res.path.clone(), (res.time * 1000.0).round() as u64);
                    if let Ok(mut cache) = self.frame_cache.write() {
                        cache.insert(key, frame.clone());
                    }
                    self.last_raw_source_frame = Some(frame);
                }
            }
        }

        let mut layers = Vec::with_capacity(visible_clips.len());
        let active_ids: std::collections::HashSet<&str> =
            visible_clips.iter().map(|(_, c)| c.id.as_str()).collect();
        self.extra_streamers.retain(|id, s| {
            if active_ids.contains(id.as_str()) {
                true
            } else {
                s.stop();
                false
            }
        });

        for (layer_idx, (_track, clip)) in visible_clips.iter().enumerate() {
            if clip.is_adjustment_layer() {
                let sentinel = Arc::new(ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0])));
                layers.push(CompositorLayer {
                    clip: (*clip).clone(),
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
                    let eff_path = self.effective_playback_path(media_item).to_string();
                    let src_t = clip.map_timeline_to_source(timeline_time);
                    let (pw, ph) =
                        calculate_preview_size(project.timeline.width, project.timeline.height);
                    self.resolve_layer_source_frame(
                        clip,
                        &eff_path,
                        src_t,
                        is_playing,
                        project.timeline.fps,
                        layer_idx == 0,
                        pw,
                        ph,
                    )
                }
            };

            if let Some(frame) = frame_img {
                layers.push(CompositorLayer {
                    clip: (*clip).clone(),
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
                    let key = (res.path.clone(), (res.time * 1000.0).round() as u64);
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
                && !clip.corner_pin.as_ref().is_some_and(|cp| cp.enabled)
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
                let eff_path = self.effective_playback_path(media_item);
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
                        self.streamer.start_sized(
                            &clip.id,
                            eff_path,
                            src_t,
                            project.timeline.fps,
                            clip.reverse,
                            clip.source_in,
                            canvas_w,
                            canvas_h,
                        );
                    }

                    if let Some(frame) = self.streamer.get_frame_for_time(src_t) {
                        self.last_raw_source_frame = Some(frame.clone());
                        if is_standard_transform {
                            self.last_rendered_frame = Some(frame.clone());
                            return Some(frame);
                        } else {
                            let mut canvas =
                                ImageBuffer::from_pixel(canvas_w, canvas_h, Rgba([0, 0, 0, 255]));
                            draw_clip_onto_canvas(&mut canvas, clip, &frame, timeline_time);
                            let arc_canvas = Arc::new(canvas);
                            self.last_rendered_frame = Some(arc_canvas.clone());
                            return Some(arc_canvas);
                        }
                    } else if let Some(last_raw) = &self.last_raw_source_frame {
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
                } else {
                    // Scrubbing/Paused Mode: Stop streamer, check cache, or request async seek
                    if self.streamer.is_active() {
                        self.streamer.stop();
                    }

                    let ts_key = (src_t * 1000.0).round() as u64;
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
                        if let Some(frame) =
                            decode_frame_raw_sized(eff_path, src_t, canvas_w, canvas_h)
                        {
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
                        self.async_decoder
                            .request_seek_sized(eff_path, src_t, canvas_w, canvas_h);
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
        let active_ids: std::collections::HashSet<&str> =
            visible_clips.iter().map(|(_, c)| c.id.as_str()).collect();
        self.extra_streamers.retain(|id, s| {
            if active_ids.contains(id.as_str()) {
                true
            } else {
                s.stop();
                false
            }
        });

        for (layer_idx, (_track, clip)) in visible_clips.iter().enumerate() {
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
                    let eff_path = self.effective_playback_path(media_item).to_string();
                    let src_t = clip.map_timeline_to_source(timeline_time);
                    self.resolve_layer_source_frame(
                        clip,
                        &eff_path,
                        src_t,
                        is_playing,
                        project.timeline.fps,
                        layer_idx == 0,
                        canvas_w,
                        canvas_h,
                    )
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
