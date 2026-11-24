use super::compositor::draw_clip_onto_canvas;
use super::streamer::decode_frame_raw;
use image::{ImageBuffer, Rgba, RgbaImage};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use trim_core::clip::Clip;
use trim_core::lru::LruCache;
use trim_core::media::MediaType;
use trim_core::project::Project;

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

static CUBE_LUT_CACHE: std::sync::OnceLock<Mutex<LruCache<String, Option<Arc<CubeLut3D>>>>> =
    std::sync::OnceLock::new();

pub fn parse_cube_lut_file<P: AsRef<Path>>(path: P) -> Option<Arc<CubeLut3D>> {
    let norm = trim_core::paths::normalize_path(path);
    let mtime_key = std::fs::metadata(&norm)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let cache_key = format!("{}#{}", norm.to_string_lossy(), mtime_key);

    let cache = CUBE_LUT_CACHE.get_or_init(|| Mutex::new(LruCache::with_capacity(16)));
    if let Ok(mut guard) = cache.lock() {
        if let Some(cached) = guard.get(&cache_key) {
            return cached.clone();
        }
    }

    const MAX_CUBE_LUT_BYTES: u64 = 16 * 1024 * 1024;
    let parsed = std::fs::symlink_metadata(&norm)
        .ok()
        .filter(|m| !m.file_type().is_symlink() && m.is_file() && m.len() <= MAX_CUBE_LUT_BYTES)
        .and_then(|_| std::fs::read_to_string(&norm).ok())
        .and_then(|content| parse_cube_lut_str(&content))
        .map(Arc::new);

    if let Ok(mut guard) = cache.lock() {
        guard.insert(cache_key, parsed.clone());
    }
    parsed
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
        let ts_key = (source_time * 1000.0).round() as u64;

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
