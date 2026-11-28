//! OpenGL 3.3+ / GLES 3.0 (`glow`) off-screen FBO fragment shader compositor for real-time NLE preview.

use eframe::glow::{self, HasContext};
use std::collections::HashMap;
use std::sync::Arc;
use trim_core::clip::Clip;
use trim_core::project::Project;
use trim_media::{calculate_preview_size, parse_cube_lut_file, CompositorLayer, CubeLut3D};

pub const GPU_PREVIEW_SCALE: u32 = 2; // 1280x720 for 16:9 (2x logical 640x360 coordinate space)

mod shaders;
mod uniforms;

use shaders::{FRAGMENT_SHADER_BODY, VERTEX_SHADER_BODY};
pub use uniforms::LayerUniforms;

/// Hardware-accelerated OpenGL (`glow`) off-screen FBO compositor.
pub struct GpuCompositor {
    gl: Arc<glow::Context>,
    program: glow::Program,
    vao: glow::VertexArray,
    fbo: glow::Framebuffer,
    fbo_texture: glow::Texture,
    backdrop_fbo: glow::Framebuffer,
    backdrop_texture: glow::Texture,
    fbo_internal_format: i32,
    supports_srgb_framebuffer: bool,
    fbo_texture_registered: bool,
    egui_texture_id: Option<egui::TextureId>,
    fbo_width: u32,
    fbo_height: u32,
    layer_texture: glow::Texture,
    layer_tex_dims: (u32, u32),
    lut_texture: glow::Texture,
    lut_cache: trim_core::lru::LruCache<String, Option<CubeLut3D>>,
    uploaded_lut_path: Option<String>,
    uploaded_lut_size: u32,
    mask_texture: glow::Texture,
    mask_tex_dims: (u32, u32),
    mask_cache: HashMap<String, (u64, Vec<u8>, u32, u32)>,
    uploaded_mask_key: Option<(String, u64, u32, u32)>,
    scope_fbo: glow::Framebuffer,
    scope_texture: glow::Texture,
    scope_pbos: [Option<glow::Buffer>; 2],
    scope_pbo_idx: usize,
    scope_pbo_ready: [bool; 2],
    uniform_cache: HashMap<&'static str, Option<glow::UniformLocation>>,
    max_tex_size: i32,
    gl_destroyed: bool,
}

impl GpuCompositor {
    /// Initializes the OpenGL shader program, VAO, off-screen FBO, and layer textures.
    /// Logs errors via `log::error!` and returns `None` without panicking if compilation/linking fails.
    pub fn new(gl: Arc<glow::Context>) -> Option<Self> {
        unsafe {
            let ver = gl.version();
            if (!ver.is_embedded && (ver.major < 3 || (ver.major == 3 && ver.minor < 3)))
                || (ver.is_embedded && ver.major < 3)
            {
                log::warn!(
                    "GPU Compositor: OpenGL/GLES sürümü ({}.{}) 3.3 Core / ES 3.0 altında; CPU fallback etkinleştiriliyor.",
                    ver.major,
                    ver.minor
                );
                return None;
            }

            let header = if ver.is_embedded {
                "#version 300 es\nprecision highp float;\n"
            } else {
                "#version 330 core\n"
            };

            let vs_src = format!("{header}{VERTEX_SHADER_BODY}");
            let fs_src = format!("{header}{FRAGMENT_SHADER_BODY}");

            let vs = match gl.create_shader(glow::VERTEX_SHADER) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("GPU Compositor: Vertex shader oluşturulamadı: {}", e);
                    return None;
                }
            };
            gl.shader_source(vs, &vs_src);
            gl.compile_shader(vs);
            if !gl.get_shader_compile_status(vs) {
                let info = gl.get_shader_info_log(vs);
                log::error!("GPU Compositor: Vertex shader derleme hatası: {}", info);
                gl.delete_shader(vs);
                return None;
            }

            let fs = match gl.create_shader(glow::FRAGMENT_SHADER) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("GPU Compositor: Fragment shader oluşturulamadı: {}", e);
                    gl.delete_shader(vs);
                    return None;
                }
            };
            gl.shader_source(fs, &fs_src);
            gl.compile_shader(fs);
            if !gl.get_shader_compile_status(fs) {
                let info = gl.get_shader_info_log(fs);
                log::error!("GPU Compositor: Fragment shader derleme hatası: {}", info);
                gl.delete_shader(vs);
                gl.delete_shader(fs);
                return None;
            }

            let program = match gl.create_program() {
                Ok(p) => p,
                Err(e) => {
                    log::error!("GPU Compositor: GL program oluşturulamadı: {}", e);
                    gl.delete_shader(vs);
                    gl.delete_shader(fs);
                    return None;
                }
            };
            gl.attach_shader(program, vs);
            gl.attach_shader(program, fs);
            gl.link_program(program);
            gl.delete_shader(vs);
            gl.delete_shader(fs);

            if !gl.get_program_link_status(program) {
                let info = gl.get_program_info_log(program);
                log::error!("GPU Compositor: GL program bağlama hatası: {}", info);
                gl.delete_program(program);
                return None;
            }

            let vao = match gl.create_vertex_array() {
                Ok(v) => v,
                Err(e) => {
                    log::error!("GPU Compositor: VAO oluşturulamadı: {}", e);
                    gl.delete_program(program);
                    return None;
                }
            };

            let fbo_width = 640 * GPU_PREVIEW_SCALE;
            let fbo_height = 360 * GPU_PREVIEW_SCALE;

            let exts = gl.supported_extensions();
            let supports_srgb_framebuffer =
                exts.iter().any(|ext| ext.ends_with("ARB_framebuffer_sRGB"));
            let fbo_internal_format = glow::RGBA8 as i32;

            let fbo_texture = match Self::create_texture_with_format(
                &gl,
                fbo_width,
                fbo_height,
                fbo_internal_format,
            ) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("GPU Compositor: FBO dokusu oluşturulamadı: {}", e);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return None;
                }
            };

            let fbo = match gl.create_framebuffer() {
                Ok(f) => f,
                Err(e) => {
                    log::error!("GPU Compositor: Framebuffer oluşturulamadı: {}", e);
                    gl.delete_texture(fbo_texture);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return None;
                }
            };

            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(fbo_texture),
                0,
            );
            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            if status != glow::FRAMEBUFFER_COMPLETE {
                log::error!(
                    "GPU Compositor: Framebuffer tamamlanamadı (status: 0x{:X})",
                    status
                );
                gl.delete_framebuffer(fbo);
                gl.delete_texture(fbo_texture);
                gl.delete_vertex_array(vao);
                gl.delete_program(program);
                return None;
            }

            let backdrop_texture = match Self::create_texture_with_format(
                &gl,
                fbo_width,
                fbo_height,
                fbo_internal_format,
            ) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("GPU Compositor: Backdrop dokusu oluşturulamadı: {}", e);
                    gl.delete_framebuffer(fbo);
                    gl.delete_texture(fbo_texture);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return None;
                }
            };

            let backdrop_fbo = match gl.create_framebuffer() {
                Ok(f) => f,
                Err(e) => {
                    log::error!("GPU Compositor: Backdrop FBO oluşturulamadı: {}", e);
                    gl.delete_texture(backdrop_texture);
                    gl.delete_framebuffer(fbo);
                    gl.delete_texture(fbo_texture);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return None;
                }
            };

            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(backdrop_fbo));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(backdrop_texture),
                0,
            );
            let bd_status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            if bd_status != glow::FRAMEBUFFER_COMPLETE {
                log::error!(
                    "GPU Compositor: Backdrop FBO tamamlanamadı (status: 0x{:X})",
                    bd_status
                );
                gl.delete_framebuffer(backdrop_fbo);
                gl.delete_texture(backdrop_texture);
                gl.delete_framebuffer(fbo);
                gl.delete_texture(fbo_texture);
                gl.delete_vertex_array(vao);
                gl.delete_program(program);
                return None;
            }

            let layer_texture = match Self::create_rgba_texture(&gl, 640, 360) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("GPU Compositor: Katman dokusu oluşturulamadı: {}", e);
                    gl.delete_framebuffer(backdrop_fbo);
                    gl.delete_texture(backdrop_texture);
                    gl.delete_framebuffer(fbo);
                    gl.delete_texture(fbo_texture);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return None;
                }
            };

            let lut_texture = match Self::create_rgba_texture(&gl, 16, 4) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("GPU Compositor: LUT dokusu oluşturulamadı: {}", e);
                    gl.delete_texture(layer_texture);
                    gl.delete_framebuffer(backdrop_fbo);
                    gl.delete_texture(backdrop_texture);
                    gl.delete_framebuffer(fbo);
                    gl.delete_texture(fbo_texture);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return None;
                }
            };

            let mask_texture = match Self::create_rgba_texture(&gl, 320, 180) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("GPU Compositor: Maske dokusu oluşturulamadı: {}", e);
                    gl.delete_texture(lut_texture);
                    gl.delete_texture(layer_texture);
                    gl.delete_framebuffer(backdrop_fbo);
                    gl.delete_texture(backdrop_texture);
                    gl.delete_framebuffer(fbo);
                    gl.delete_texture(fbo_texture);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return None;
                }
            };

            let scope_texture = match Self::create_rgba_texture(&gl, 320, 180) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("GPU Compositor: Scope dokusu oluşturulamadı: {}", e);
                    gl.delete_texture(mask_texture);
                    gl.delete_texture(lut_texture);
                    gl.delete_texture(layer_texture);
                    gl.delete_framebuffer(backdrop_fbo);
                    gl.delete_texture(backdrop_texture);
                    gl.delete_framebuffer(fbo);
                    gl.delete_texture(fbo_texture);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return None;
                }
            };
            let scope_fbo = match gl.create_framebuffer() {
                Ok(f) => f,
                Err(e) => {
                    log::error!("GPU Compositor: Scope FBO oluşturulamadı: {}", e);
                    gl.delete_texture(scope_texture);
                    gl.delete_texture(mask_texture);
                    gl.delete_texture(lut_texture);
                    gl.delete_texture(layer_texture);
                    gl.delete_framebuffer(backdrop_fbo);
                    gl.delete_texture(backdrop_texture);
                    gl.delete_framebuffer(fbo);
                    gl.delete_texture(fbo_texture);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return None;
                }
            };
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(scope_fbo));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(scope_texture),
                0,
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            let scope_pbo_bytes = (320usize * 180usize * 4usize) as i32;
            let mut scope_pbos = [None, None];
            if let (Ok(b0), Ok(b1)) = (gl.create_buffer(), gl.create_buffer()) {
                for b in [b0, b1] {
                    gl.bind_buffer(glow::PIXEL_PACK_BUFFER, Some(b));
                    gl.buffer_data_size(
                        glow::PIXEL_PACK_BUFFER,
                        scope_pbo_bytes,
                        glow::STREAM_READ,
                    );
                }
                gl.bind_buffer(glow::PIXEL_PACK_BUFFER, None);
                scope_pbos = [Some(b0), Some(b1)];
            }

            let max_tex_size = gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE).max(256);

            Some(Self {
                gl,
                program,
                vao,
                fbo,
                fbo_texture,
                backdrop_fbo,
                backdrop_texture,
                fbo_internal_format,
                supports_srgb_framebuffer,
                fbo_texture_registered: false,
                egui_texture_id: None,
                fbo_width,
                fbo_height,
                layer_texture,
                layer_tex_dims: (640, 360),
                lut_texture,
                lut_cache: trim_core::lru::LruCache::with_capacity(16),
                uploaded_lut_path: None,
                uploaded_lut_size: 0,
                mask_texture,
                mask_tex_dims: (320, 180),
                mask_cache: HashMap::new(),
                uploaded_mask_key: None,
                scope_fbo,
                scope_texture,
                scope_pbos,
                scope_pbo_idx: 0,
                scope_pbo_ready: [false, false],
                uniform_cache: HashMap::with_capacity(64),
                max_tex_size,
                gl_destroyed: false,
            })
        }
    }

    unsafe fn create_rgba_texture(
        gl: &glow::Context,
        width: u32,
        height: u32,
    ) -> Result<glow::Texture, String> {
        Self::create_texture_with_format(gl, width, height, glow::RGBA8 as i32)
    }

    unsafe fn create_texture_with_format(
        gl: &glow::Context,
        width: u32,
        height: u32,
        internal_format: i32,
    ) -> Result<glow::Texture, String> {
        let tex = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            internal_format,
            width as i32,
            height as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            None,
        );
        gl.bind_texture(glow::TEXTURE_2D, None);
        Ok(tex)
    }

    unsafe fn ensure_fbo_dimensions(&mut self, target_w: u32, target_h: u32) {
        let clamped_w = target_w.max(16);
        let clamped_h = target_h.max(16);
        if self.fbo_width == clamped_w && self.fbo_height == clamped_h {
            return;
        }
        self.fbo_width = clamped_w;
        self.fbo_height = clamped_h;
        for tex in [self.fbo_texture, self.backdrop_texture] {
            self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                self.fbo_internal_format,
                self.fbo_width as i32,
                self.fbo_height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                None,
            );
        }
        self.gl.bind_texture(glow::TEXTURE_2D, None);
        for (label, fb) in [("fbo", self.fbo), ("backdrop_fbo", self.backdrop_fbo)] {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fb));
            let status = self.gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status != glow::FRAMEBUFFER_COMPLETE {
                log::warn!(
                    "GPU Compositor: {label} resize {}x{} incomplete (0x{:X})",
                    clamped_w,
                    clamped_h,
                    status
                );
            }
        }
        self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    }

    unsafe fn upload_layer_frame(&mut self, frame: &image::RgbaImage, clip: &Clip) {
        let w = frame.width();
        let h = frame.height();
        let has_curves = clip
            .custom_curves
            .as_ref()
            .is_some_and(|c| !c.is_identity());
        let has_hsl = clip.hsl_curves.as_ref().is_some_and(|c| !c.is_identity());
        let has_qual = clip.hsl_qualifier.as_ref().is_some_and(|q| q.enabled);

        let mut modified_buf: Option<Vec<u8>> = None;
        if has_curves || has_hsl || has_qual {
            let mut buf = frame.as_raw().clone();
            for px in buf.as_chunks_mut::<4>().0 {
                let mut rgb = [
                    px[0] as f32 / 255.0,
                    px[1] as f32 / 255.0,
                    px[2] as f32 / 255.0,
                ];
                if let Some(curves) = &clip.custom_curves {
                    rgb = curves.apply_rgb(rgb[0], rgb[1], rgb[2]);
                }
                if let Some(hsl) = &clip.hsl_curves {
                    rgb = hsl.apply_rgb(rgb[0], rgb[1], rgb[2]);
                }
                if let Some(qual) = &clip.hsl_qualifier {
                    if qual.enabled {
                        rgb = qual.apply_rgb(rgb[0], rgb[1], rgb[2]);
                    }
                }
                px[0] = (rgb[0] * 255.0).round() as u8;
                px[1] = (rgb[1] * 255.0).round() as u8;
                px[2] = (rgb[2] * 255.0).round() as u8;
            }
            modified_buf = Some(buf);
        }

        let raw: &[u8] = modified_buf.as_deref().unwrap_or_else(|| frame.as_raw());

        self.gl.active_texture(glow::TEXTURE0);
        self.gl
            .bind_texture(glow::TEXTURE_2D, Some(self.layer_texture));
        self.gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

        if self.layer_tex_dims == (w, h) {
            self.gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                w as i32,
                h as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(raw),
            );
        } else {
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                w as i32,
                h as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(raw),
            );
            self.layer_tex_dims = (w, h);
        }
    }

    unsafe fn upload_track_matte_frame(&mut self, matte_frame: &image::RgbaImage) -> bool {
        let w = matte_frame.width();
        let h = matte_frame.height();
        if w == 0 || h == 0 {
            return false;
        }
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for (src, dst) in matte_frame
            .as_raw()
            .as_chunks::<4>()
            .0
            .iter()
            .zip(rgba.as_chunks_mut::<4>().0.iter_mut())
        {
            let luma = (0.299 * src[0] as f32 + 0.587 * src[1] as f32 + 0.114 * src[2] as f32)
                * (src[3] as f32 / 255.0);
            let m = luma.round().clamp(0.0, 255.0) as u8;
            dst[0] = m;
            dst[1] = 255;
            dst[2] = 0;
            dst[3] = 255;
        }
        self.gl.active_texture(glow::TEXTURE3);
        self.gl
            .bind_texture(glow::TEXTURE_2D, Some(self.mask_texture));
        self.gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        if self.mask_tex_dims != (w, h) {
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                w as i32,
                h as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(&rgba),
            );
            self.mask_tex_dims = (w, h);
        } else {
            self.gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                w as i32,
                h as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(&rgba),
            );
        }
        self.uploaded_mask_key = None;
        self.gl.active_texture(glow::TEXTURE0);
        true
    }

    unsafe fn prepare_custom_lut(&mut self, lut_path: Option<&str>) -> (bool, i32) {
        let Some(raw_path) = lut_path else {
            return (false, 0);
        };
        if raw_path.is_empty() {
            return (false, 0);
        }
        let norm_buf = trim_core::paths::normalize_path(raw_path);
        let path = norm_buf.to_string_lossy();

        if self.uploaded_lut_path.as_deref() == Some(path.as_ref()) && self.uploaded_lut_size >= 2 {
            self.gl.active_texture(glow::TEXTURE1);
            self.gl
                .bind_texture(glow::TEXTURE_2D, Some(self.lut_texture));
            return (true, self.uploaded_lut_size as i32);
        }

        let path_key = path.to_string();
        if self.lut_cache.get(&path_key).is_none() {
            let parsed = parse_cube_lut_file(&norm_buf).map(|a| (*a).clone());
            self.lut_cache.insert(path_key.clone(), parsed);
        }

        let Some(Some(lut)) = self.lut_cache.get(&path_key) else {
            return (false, 0);
        };

        let n = lut.size as usize;
        let Some(tex_w_usize) = n.checked_mul(n) else {
            return (false, 0);
        };
        let Some(expected_bytes) = tex_w_usize.checked_mul(n).and_then(|v| v.checked_mul(4)) else {
            return (false, 0);
        };
        if n == 0
            || tex_w_usize > self.max_tex_size as usize
            || n > self.max_tex_size as usize
            || lut.rgba_strip.len() < expected_bytes
        {
            log::warn!(
                "GPU Compositor: Geçersiz veya GPU limitini aşan 3D LUT boyutu (n={}, bytes={})",
                n,
                lut.rgba_strip.len()
            );
            return (false, 0);
        }
        let tex_w = tex_w_usize as i32;
        let tex_h = n as i32;

        self.gl.active_texture(glow::TEXTURE1);
        self.gl
            .bind_texture(glow::TEXTURE_2D, Some(self.lut_texture));
        self.gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        self.gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            tex_w,
            tex_h,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            Some(&lut.rgba_strip[..expected_bytes]),
        );
        self.uploaded_lut_path = Some(path.to_string());
        self.uploaded_lut_size = lut.size;

        (true, tex_h)
    }

    unsafe fn prepare_adjustment_layer_lut(&mut self, clip: &Clip) -> (bool, i32) {
        let has_curves = clip
            .custom_curves
            .as_ref()
            .is_some_and(|c| !c.is_identity());
        let has_hsl = clip.hsl_curves.as_ref().is_some_and(|c| !c.is_identity());
        let has_qual = clip.hsl_qualifier.as_ref().is_some_and(|q| q.enabled);

        if !has_curves && !has_hsl && !has_qual {
            return self.prepare_custom_lut(clip.custom_lut_path.as_deref());
        }

        let cache_key = format!(
            "__adj__:{}:{:?}:{:?}:{:?}",
            clip.custom_lut_path.as_deref().unwrap_or(""),
            clip.custom_curves,
            clip.hsl_curves,
            clip.hsl_qualifier
        );

        if self.uploaded_lut_path.as_deref() == Some(cache_key.as_str())
            && self.uploaded_lut_size >= 2
        {
            self.gl.active_texture(glow::TEXTURE1);
            self.gl
                .bind_texture(glow::TEXTURE_2D, Some(self.lut_texture));
            return (true, self.uploaded_lut_size as i32);
        }

        if self.lut_cache.get(&cache_key).is_none() {
            let base_lut = clip
                .custom_lut_path
                .as_deref()
                .filter(|p| !p.is_empty())
                .map(trim_core::paths::normalize_path)
                .and_then(|p| parse_cube_lut_file(&p));
            let baked = bake_adjustment_curves_lut(clip, base_lut.as_deref(), 17);
            self.lut_cache.insert(cache_key.clone(), Some(baked));
        }

        let Some(Some(lut)) = self.lut_cache.get(&cache_key) else {
            return (false, 0);
        };
        let n = lut.size as i32;
        let tex_w = n * n;

        self.gl.active_texture(glow::TEXTURE1);
        self.gl
            .bind_texture(glow::TEXTURE_2D, Some(self.lut_texture));
        self.gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        self.gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            tex_w,
            n,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            Some(&lut.rgba_strip),
        );
        self.uploaded_lut_path = Some(cache_key);
        self.uploaded_lut_size = lut.size;
        (true, n)
    }

    unsafe fn prepare_clip_masks(
        &mut self,
        clip: &Clip,
        current_time: f64,
        logical_w: u32,
        logical_h: u32,
    ) -> bool {
        let has_enabled = clip.masks.iter().any(|m| m.enabled);
        if !has_enabled {
            return false;
        }
        let clip_time = (current_time - clip.timeline_in).max(0.0);
        let mask_w = (logical_w / 2).max(160);
        let mask_h = (logical_h / 2).max(90);

        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        mask_w.hash(&mut hasher);
        mask_h.hash(&mut hasher);
        let any_kf = clip.masks.iter().any(|m| !m.keyframes.is_empty());
        if any_kf {
            (clip_time * 60.0).round().to_bits().hash(&mut hasher);
        }
        for m in &clip.masks {
            m.id.hash(&mut hasher);
            m.enabled.hash(&mut hasher);
            (m.shape as u8).hash(&mut hasher);
            (m.target as u8).hash(&mut hasher);
            m.invert.hash(&mut hasher);
            m.pos_x.to_bits().hash(&mut hasher);
            m.pos_y.to_bits().hash(&mut hasher);
            m.scale_x.to_bits().hash(&mut hasher);
            m.scale_y.to_bits().hash(&mut hasher);
            m.rotation.to_bits().hash(&mut hasher);
            m.feather.to_bits().hash(&mut hasher);
            m.opacity.to_bits().hash(&mut hasher);
            m.expansion.to_bits().hash(&mut hasher);
            m.vertices.len().hash(&mut hasher);
            for v in &m.vertices {
                v.pos[0].to_bits().hash(&mut hasher);
                v.pos[1].to_bits().hash(&mut hasher);
                v.in_tangent[0].to_bits().hash(&mut hasher);
                v.in_tangent[1].to_bits().hash(&mut hasher);
                v.out_tangent[0].to_bits().hash(&mut hasher);
                v.out_tangent[1].to_bits().hash(&mut hasher);
            }
            m.keyframes.len().hash(&mut hasher);
            for (prop, kfs) in &m.keyframes {
                prop.hash(&mut hasher);
                for kf in kfs {
                    kf.time.to_bits().hash(&mut hasher);
                    kf.value.to_bits().hash(&mut hasher);
                }
            }
        }
        let key_hash = hasher.finish();

        let needs_raster = match self.mask_cache.get(&clip.id) {
            Some((h, _, w, h_dim)) => *h != key_hash || *w != mask_w || *h_dim != mask_h,
            None => true,
        };
        if needs_raster {
            let rgba =
                trim_core::mask::rasterize_masks_rgba(&clip.masks, mask_w, mask_h, clip_time);
            if self.mask_cache.len() >= 32 {
                self.mask_cache.clear();
            }
            self.mask_cache
                .insert(clip.id.clone(), (key_hash, rgba, mask_w, mask_h));
        }

        let desired_key = (clip.id.clone(), key_hash, mask_w, mask_h);
        if self.uploaded_mask_key.as_ref() == Some(&desired_key) {
            self.gl.active_texture(glow::TEXTURE3);
            self.gl
                .bind_texture(glow::TEXTURE_2D, Some(self.mask_texture));
            self.gl.active_texture(glow::TEXTURE0);
            return true;
        }

        if let Some((_, rgba, w, h)) = self.mask_cache.get(&clip.id) {
            self.gl.active_texture(glow::TEXTURE3);
            self.gl
                .bind_texture(glow::TEXTURE_2D, Some(self.mask_texture));
            self.gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            if self.mask_tex_dims != (*w, *h) {
                self.gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA8 as i32,
                    *w as i32,
                    *h as i32,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    Some(rgba),
                );
                self.mask_tex_dims = (*w, *h);
            } else {
                self.gl.tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    0,
                    0,
                    *w as i32,
                    *h as i32,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(rgba),
                );
            }
            self.uploaded_mask_key = Some(desired_key);
            self.gl.active_texture(glow::TEXTURE0);
            true
        } else {
            false
        }
    }

    /// Composites all visible `layers` onto the off-screen FBO using the GLSL shader pipeline
    /// (supporting ping-pong backdrop reads for blend modes, adjustment layers, and masks)
    /// and returns the bound `egui::TextureId`.
    pub fn render_layers(
        &mut self,
        project: &Project,
        layers: &[CompositorLayer],
        current_time: f64,
        frame: &mut eframe::Frame,
    ) -> Option<egui::TextureId> {
        if layers.is_empty() {
            return None;
        }

        let (logical_w, logical_h) =
            calculate_preview_size(project.timeline.width, project.timeline.height);
        let target_fbo_w = logical_w * GPU_PREVIEW_SCALE;
        let target_fbo_h = logical_h * GPU_PREVIEW_SCALE;

        unsafe {
            self.ensure_fbo_dimensions(target_fbo_w, target_fbo_h);

            let mut prev_viewport = [0i32; 4];
            self.gl
                .get_parameter_i32_slice(glow::VIEWPORT, &mut prev_viewport);
            let prev_scissor = self.gl.is_enabled(glow::SCISSOR_TEST);
            let prev_unpack_align = self.gl.get_parameter_i32(glow::UNPACK_ALIGNMENT);

            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
            if self.supports_srgb_framebuffer {
                self.gl.disable(glow::FRAMEBUFFER_SRGB);
            }
            self.gl
                .viewport(0, 0, self.fbo_width as i32, self.fbo_height as i32);
            self.gl.disable(glow::SCISSOR_TEST);
            self.gl.disable(glow::DEPTH_TEST);

            self.gl.clear_color(0.0, 0.0, 0.0, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);

            self.gl.enable(glow::BLEND);
            self.gl.blend_func_separate(
                glow::SRC_ALPHA,
                glow::ONE_MINUS_SRC_ALPHA,
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
            );

            self.gl.use_program(Some(self.program));
            self.gl.bind_vertex_array(Some(self.vao));

            if let Some(loc) = self.gl.get_uniform_location(self.program, "u_frame_tex") {
                self.gl.uniform_1_i32(Some(&loc), 0);
            }
            if let Some(loc) = self.gl.get_uniform_location(self.program, "u_lut_tex") {
                self.gl.uniform_1_i32(Some(&loc), 1);
            }
            if let Some(loc) = self.gl.get_uniform_location(self.program, "u_backdrop_tex") {
                self.gl.uniform_1_i32(Some(&loc), 2);
            }
            if let Some(loc) = self.gl.get_uniform_location(self.program, "u_mask_tex") {
                self.gl.uniform_1_i32(Some(&loc), 3);
            }

            let mut skip_next = false;
            for (idx, layer) in layers.iter().enumerate() {
                if skip_next {
                    skip_next = false;
                    continue;
                }
                let Some(u) = LayerUniforms::from_clip(
                    &layer.clip,
                    layer.frame.width(),
                    layer.frame.height(),
                    logical_w,
                    logical_h,
                    current_time,
                ) else {
                    continue;
                };

                let needs_backdrop = u.blend_mode != 0 || u.is_adjustment_layer;
                if needs_backdrop {
                    self.gl
                        .bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.fbo));
                    self.gl
                        .bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(self.backdrop_fbo));
                    self.gl.blit_framebuffer(
                        0,
                        0,
                        self.fbo_width as i32,
                        self.fbo_height as i32,
                        0,
                        0,
                        self.fbo_width as i32,
                        self.fbo_height as i32,
                        glow::COLOR_BUFFER_BIT,
                        glow::NEAREST,
                    );
                    self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
                    self.gl.disable(glow::BLEND);
                    self.gl.active_texture(glow::TEXTURE2);
                    self.gl
                        .bind_texture(glow::TEXTURE_2D, Some(self.backdrop_texture));
                    self.gl.active_texture(glow::TEXTURE0);
                } else {
                    self.gl.enable(glow::BLEND);
                }

                if !u.is_adjustment_layer {
                    self.upload_layer_frame(&layer.frame, &layer.clip);
                }
                let has_mask = if layer.clip.track_matte_above && idx + 1 < layers.len() {
                    skip_next = true;
                    self.upload_track_matte_frame(&layers[idx + 1].frame)
                } else {
                    self.prepare_clip_masks(&layer.clip, current_time, logical_w, logical_h)
                };
                let (custom_lut_enabled, lut_size) = if u.is_adjustment_layer {
                    self.prepare_adjustment_layer_lut(&layer.clip)
                } else {
                    self.prepare_custom_lut(layer.clip.custom_lut_path.as_deref())
                };

                let gl = &self.gl;
                let prog = self.program;
                let uniform_cache = &mut self.uniform_cache;
                let mut u_loc = |name: &'static str| -> Option<glow::UniformLocation> {
                    *uniform_cache
                        .entry(name)
                        .or_insert_with(|| gl.get_uniform_location(prog, name))
                };

                if let Some(loc) = u_loc("u_has_mask") {
                    gl.uniform_1_i32(Some(&loc), i32::from(has_mask));
                }
                if let Some(loc) = u_loc("u_blend_mode") {
                    gl.uniform_1_i32(Some(&loc), u.blend_mode);
                }
                if let Some(loc) = u_loc("u_is_adjustment_layer") {
                    gl.uniform_1_i32(Some(&loc), i32::from(u.is_adjustment_layer));
                }
                if let Some(loc) = u_loc("u_canvas_size") {
                    gl.uniform_2_f32(Some(&loc), u.canvas_size[0], u.canvas_size[1]);
                }
                if let Some(loc) = u_loc("u_clip_center") {
                    gl.uniform_2_f32(Some(&loc), u.clip_center[0], u.clip_center[1]);
                }
                if let Some(loc) = u_loc("u_clip_size") {
                    gl.uniform_2_f32(Some(&loc), u.clip_size[0], u.clip_size[1]);
                }
                if let Some(loc) = u_loc("u_rotation_rad") {
                    gl.uniform_1_f32(Some(&loc), u.rotation_rad);
                }
                if let Some(loc) = u_loc("u_opacity") {
                    gl.uniform_1_f32(Some(&loc), u.opacity);
                }
                if let Some(loc) = u_loc("u_crop") {
                    gl.uniform_4_f32(Some(&loc), u.crop[0], u.crop[1], u.crop[2], u.crop[3]);
                }
                if let Some(loc) = u_loc("u_wipe_mode") {
                    gl.uniform_1_i32(Some(&loc), u.wipe_mode);
                }
                if let Some(loc) = u_loc("u_fade_factor") {
                    gl.uniform_1_f32(Some(&loc), u.fade_factor);
                }
                if let Some(loc) = u_loc("u_dip_white") {
                    gl.uniform_1_f32(Some(&loc), u.dip_white);
                }
                if let Some(loc) = u_loc("u_ck_enabled") {
                    gl.uniform_1_i32(Some(&loc), i32::from(u.ck_enabled));
                }
                if let Some(loc) = u_loc("u_ck_color") {
                    gl.uniform_3_f32(Some(&loc), u.ck_color[0], u.ck_color[1], u.ck_color[2]);
                }
                if let Some(loc) = u_loc("u_ck_similarity") {
                    gl.uniform_1_f32(Some(&loc), u.ck_similarity);
                }
                if let Some(loc) = u_loc("u_ck_smoothness") {
                    gl.uniform_1_f32(Some(&loc), u.ck_smoothness);
                }
                if let Some(loc) = u_loc("u_cw_enabled") {
                    gl.uniform_1_i32(Some(&loc), i32::from(u.cw_enabled));
                }
                if let Some(loc) = u_loc("u_cw_lift") {
                    gl.uniform_3_f32(Some(&loc), u.cw_lift[0], u.cw_lift[1], u.cw_lift[2]);
                }
                if let Some(loc) = u_loc("u_cw_gamma") {
                    gl.uniform_3_f32(Some(&loc), u.cw_gamma[0], u.cw_gamma[1], u.cw_gamma[2]);
                }
                if let Some(loc) = u_loc("u_cw_gain") {
                    gl.uniform_3_f32(Some(&loc), u.cw_gain[0], u.cw_gain[1], u.cw_gain[2]);
                }
                if let Some(loc) = u_loc("u_wb_gains") {
                    gl.uniform_3_f32(Some(&loc), u.wb_gains[0], u.wb_gains[1], u.wb_gains[2]);
                }
                if let Some(loc) = u_loc("u_contrast") {
                    gl.uniform_1_f32(Some(&loc), u.contrast);
                }
                if let Some(loc) = u_loc("u_brightness") {
                    gl.uniform_1_f32(Some(&loc), u.brightness);
                }
                if let Some(loc) = u_loc("u_gamma") {
                    gl.uniform_1_f32(Some(&loc), u.gamma);
                }
                if let Some(loc) = u_loc("u_saturation") {
                    gl.uniform_1_f32(Some(&loc), u.saturation);
                }
                if let Some(loc) = u_loc("u_camera_log_profile") {
                    gl.uniform_1_i32(Some(&loc), u.camera_log_profile);
                }
                if let Some(loc) = u_loc("u_lut_preset") {
                    gl.uniform_1_i32(Some(&loc), u.lut_preset);
                }
                if let Some(loc) = u_loc("u_custom_lut_enabled") {
                    gl.uniform_1_i32(Some(&loc), i32::from(custom_lut_enabled));
                }
                if let Some(loc) = u_loc("u_lut_size") {
                    gl.uniform_1_i32(Some(&loc), lut_size);
                }
                if let Some(loc) = u_loc("u_blur") {
                    gl.uniform_1_f32(Some(&loc), u.blur);
                }
                if let Some(loc) = u_loc("u_sharpen") {
                    gl.uniform_1_f32(Some(&loc), u.sharpen);
                }
                if let Some(loc) = u_loc("u_vignette") {
                    gl.uniform_1_f32(Some(&loc), u.vignette);
                }
                if let Some(loc) = u_loc("u_film_grain") {
                    gl.uniform_1_f32(Some(&loc), u.film_grain);
                }
                if let Some(loc) = u_loc("u_halation") {
                    gl.uniform_1_f32(Some(&loc), u.halation);
                }
                if let Some(loc) = u_loc("u_bloom") {
                    gl.uniform_1_f32(Some(&loc), u.bloom);
                }
                if let Some(loc) = u_loc("u_chromatic_aberration") {
                    gl.uniform_1_f32(Some(&loc), u.chromatic_aberration);
                }
                if let Some(loc) = u_loc("u_film_format") {
                    gl.uniform_1_i32(Some(&loc), u.film_format);
                }
                if let Some(loc) = u_loc("u_highlight_rolloff") {
                    gl.uniform_1_f32(Some(&loc), u.highlight_rolloff);
                }
                if let Some(loc) = u_loc("u_diffusion") {
                    gl.uniform_1_f32(Some(&loc), u.diffusion);
                }
                if let Some(loc) = u_loc("u_mist") {
                    gl.uniform_1_f32(Some(&loc), u.mist);
                }
                if let Some(loc) = u_loc("u_gate_weave") {
                    gl.uniform_1_f32(Some(&loc), u.gate_weave);
                }
                if let Some(loc) = u_loc("u_cp_enabled") {
                    gl.uniform_1_i32(Some(&loc), i32::from(u.cp_enabled));
                }
                if let Some(loc) = u_loc("u_cp_tl") {
                    gl.uniform_2_f32(Some(&loc), u.cp_tl[0], u.cp_tl[1]);
                }
                if let Some(loc) = u_loc("u_cp_tr") {
                    gl.uniform_2_f32(Some(&loc), u.cp_tr[0], u.cp_tr[1]);
                }
                if let Some(loc) = u_loc("u_cp_br") {
                    gl.uniform_2_f32(Some(&loc), u.cp_br[0], u.cp_br[1]);
                }
                if let Some(loc) = u_loc("u_cp_bl") {
                    gl.uniform_2_f32(Some(&loc), u.cp_bl[0], u.cp_bl[1]);
                }
                if let Some(loc) = u_loc("u_glitch_intensity") {
                    gl.uniform_1_f32(Some(&loc), u.glitch_intensity);
                }
                if let Some(loc) = u_loc("u_light_leak") {
                    gl.uniform_1_f32(Some(&loc), u.light_leak);
                }
                if let Some(loc) = u_loc("u_time_seed") {
                    gl.uniform_1_f32(Some(&loc), u.time_seed);
                }

                gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            }

            self.gl.bind_vertex_array(None);
            self.gl.use_program(None);
            self.gl.active_texture(glow::TEXTURE3);
            self.gl.bind_texture(glow::TEXTURE_2D, None);
            self.gl.active_texture(glow::TEXTURE2);
            self.gl.bind_texture(glow::TEXTURE_2D, None);
            self.gl.active_texture(glow::TEXTURE1);
            self.gl.bind_texture(glow::TEXTURE_2D, None);
            self.gl.active_texture(glow::TEXTURE0);
            self.gl.bind_texture(glow::TEXTURE_2D, None);
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            self.gl
                .pixel_store_i32(glow::UNPACK_ALIGNMENT, prev_unpack_align);
            if prev_scissor {
                self.gl.enable(glow::SCISSOR_TEST);
            }
            self.gl.viewport(
                prev_viewport[0],
                prev_viewport[1],
                prev_viewport[2],
                prev_viewport[3],
            );
        }

        if self.egui_texture_id.is_none() {
            let tex_id = frame.register_native_glow_texture(self.fbo_texture);
            self.fbo_texture_registered = true;
            self.egui_texture_id = Some(tex_id);
        }

        self.egui_texture_id
    }

    /// Reads back the current composited FBO frame into an RGBA8 buffer for real-time color scopes.
    pub fn readback_fbo_rgba(&self) -> Option<(Vec<u8>, u32, u32)> {
        if self.fbo_width == 0 || self.fbo_height == 0 {
            return None;
        }
        let w = self.fbo_width;
        let h = self.fbo_height;
        let byte_len = (w as usize) * (h as usize) * 4;
        let mut buf = vec![0u8; byte_len];
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
            self.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            self.gl.read_pixels(
                0,
                0,
                w as i32,
                h as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(&mut buf),
            );
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
        Some((buf, w, h))
    }

    /// Reads back a 320x180 GPU-downsampled copy of the composited FBO for low-overhead real-time scopes.
    /// Uses double-buffered OpenGL PBO (`PIXEL_PACK_BUFFER`) DMA when available, falling back to
    /// synchronous `gl.read_pixels` on contexts without PBO mapping.
    pub fn readback_fbo_downsampled_rgba(&mut self) -> Option<(Vec<u8>, u32, u32)> {
        if self.fbo_width == 0 || self.fbo_height == 0 {
            return None;
        }
        let sw = 320u32;
        let sh = 180u32;
        let byte_len = (sw as usize) * (sh as usize) * 4;
        let mut buf = vec![0u8; byte_len];
        unsafe {
            self.gl
                .bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.fbo));
            self.gl
                .bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(self.scope_fbo));
            self.gl.blit_framebuffer(
                0,
                0,
                self.fbo_width as i32,
                self.fbo_height as i32,
                0,
                0,
                sw as i32,
                sh as i32,
                glow::COLOR_BUFFER_BIT,
                glow::LINEAR,
            );
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(self.scope_fbo));
            self.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);

            let write_idx = self.scope_pbo_idx;
            let read_idx = 1 - write_idx;
            let mut got_pbo_data = false;

            if let (Some(write_pbo), Some(read_pbo)) =
                (self.scope_pbos[write_idx], self.scope_pbos[read_idx])
            {
                // Start async DMA transfer for current frame into `write_pbo`.
                self.gl
                    .bind_buffer(glow::PIXEL_PACK_BUFFER, Some(write_pbo));
                self.gl.read_pixels(
                    0,
                    0,
                    sw as i32,
                    sh as i32,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelPackData::BufferOffset(0),
                );
                self.scope_pbo_ready[write_idx] = true;

                // Read previous frame's completed PBO if ready.
                if self.scope_pbo_ready[read_idx] {
                    self.gl.bind_buffer(glow::PIXEL_PACK_BUFFER, Some(read_pbo));
                    let ptr = self.gl.map_buffer_range(
                        glow::PIXEL_PACK_BUFFER,
                        0,
                        byte_len as i32,
                        glow::MAP_READ_BIT,
                    );
                    if !ptr.is_null() {
                        std::ptr::copy_nonoverlapping(ptr, buf.as_mut_ptr(), byte_len);
                        self.gl.unmap_buffer(glow::PIXEL_PACK_BUFFER);
                        got_pbo_data = true;
                    }
                }
                self.gl.bind_buffer(glow::PIXEL_PACK_BUFFER, None);
                self.scope_pbo_idx = read_idx;
            }

            if !got_pbo_data {
                self.gl.read_pixels(
                    0,
                    0,
                    sw as i32,
                    sh as i32,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelPackData::Slice(&mut buf),
                );
            }

            self.gl.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
            self.gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, None);
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
        Some((buf, sw, sh))
    }

    /// Explicitly deletes all OpenGL resources while the GL context is still current
    /// (called from `eframe::App::on_exit`).
    pub fn destroy(&mut self, gl: &glow::Context) {
        if self.gl_destroyed {
            return;
        }
        self.gl_destroyed = true;
        unsafe {
            for pbo in self.scope_pbos.iter().flatten() {
                gl.delete_buffer(*pbo);
            }
            gl.delete_framebuffer(self.scope_fbo);
            gl.delete_texture(self.scope_texture);
            gl.delete_texture(self.mask_texture);
            gl.delete_texture(self.layer_texture);
            gl.delete_texture(self.lut_texture);
            gl.delete_framebuffer(self.backdrop_fbo);
            gl.delete_texture(self.backdrop_texture);
            gl.delete_framebuffer(self.fbo);
            gl.delete_texture(self.fbo_texture);
            gl.delete_vertex_array(self.vao);
            gl.delete_program(self.program);
        }
    }
}

impl Drop for GpuCompositor {
    fn drop(&mut self) {
        if !self.gl_destroyed {
            let gl = std::sync::Arc::clone(&self.gl);
            self.destroy(&gl);
        }
    }
}

pub fn bake_adjustment_curves_lut(
    clip: &Clip,
    base_lut: Option<&CubeLut3D>,
    size: u32,
) -> CubeLut3D {
    let n = size.max(2) as usize;
    let denom = (n - 1) as f32;
    let mut rgba_strip = vec![0u8; n * n * n * 4];
    for b_idx in 0..n {
        for g_idx in 0..n {
            for r_idx in 0..n {
                let mut rgb = [
                    r_idx as f32 / denom,
                    g_idx as f32 / denom,
                    b_idx as f32 / denom,
                ];
                if let Some(lut) = base_lut {
                    rgb = lut.sample(rgb[0], rgb[1], rgb[2]);
                }
                if let Some(curves) = &clip.custom_curves {
                    rgb = curves.apply_rgb(rgb[0], rgb[1], rgb[2]);
                }
                if let Some(hsl) = &clip.hsl_curves {
                    rgb = hsl.apply_rgb(rgb[0], rgb[1], rgb[2]);
                }
                if let Some(qual) = &clip.hsl_qualifier {
                    if qual.enabled {
                        rgb = qual.apply_rgb(rgb[0], rgb[1], rgb[2]);
                    }
                }
                let x = b_idx * n + r_idx;
                let y = g_idx;
                let off = (y * (n * n) + x) * 4;
                rgba_strip[off] = (rgb[0].clamp(0.0, 1.0) * 255.0).round() as u8;
                rgba_strip[off + 1] = (rgb[1].clamp(0.0, 1.0) * 255.0).round() as u8;
                rgba_strip[off + 2] = (rgb[2].clamp(0.0, 1.0) * 255.0).round() as u8;
                rgba_strip[off + 3] = 255;
            }
        }
    }
    CubeLut3D {
        size: n as u32,
        rgba_strip,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trim_core::clip::{
        BrightnessContrastPreset, ChromaKeyConfig, Clip, ColorWheelsConfig, LutPreset,
        WhiteBalancePreset,
    };

    #[test]
    fn test_layer_uniforms_affine_transform_and_grading() {
        let mut clip = Clip::new("media_1", 0.0, 10.0);
        clip.scale = 1.5;
        clip.pos_x = 40.0;
        clip.pos_y = -20.0;
        clip.rotation = 90.0;
        clip.opacity = 0.8;
        clip.apply_wb_preset(WhiteBalancePreset::Warm);
        clip.apply_bc_preset(BrightnessContrastPreset::CinematicContrast);
        clip.contrast = 1.2;
        clip.brightness = 0.1;
        clip.saturation = 1.3;
        clip.lut_preset = LutPreset::Kodak2383;
        clip.chromakey = Some(ChromaKeyConfig {
            enabled: true,
            key_color: [0, 255, 0],
            similarity: 0.25,
            smoothness: 0.08,
        });
        clip.color_wheels = Some(ColorWheelsConfig {
            lift: [0.1, 0.0, -0.1],
            gamma: [1.1, 1.0, 0.9],
            gain: [1.2, 1.0, 0.8],
            luminance: [0.05, 0.0, -0.05],
        });

        clip.crop_left = 0.1;
        clip.crop_top = 0.05;
        clip.crop_right = 0.15;
        clip.crop_bottom = 0.2;

        let u = LayerUniforms::from_clip(&clip, 640, 360, 640, 360, 2.0)
            .expect("Should produce active layer uniforms");

        assert_eq!(u.canvas_size, [640.0, 360.0]);
        assert_eq!(u.clip_size, [960.0, 540.0]);
        assert_eq!(u.clip_center, [360.0, 160.0]);
        assert!((u.rotation_rad - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert!((u.opacity - 0.8).abs() < 1e-5);
        assert_eq!(u.crop, [0.1, 0.05, 0.15, 0.2]);
        assert!(u.ck_enabled);
        assert_eq!(u.ck_color, [0.0, 1.0, 0.0]);
        assert!((u.ck_similarity - 0.25).abs() < 1e-5);
        assert!((u.ck_smoothness - 0.08).abs() < 1e-5);
        assert!(u.cw_enabled);
        assert!((u.cw_lift[0] - 0.15).abs() < 1e-5);
        assert!((u.cw_gamma[0] - 1.1).abs() < 1e-5);
        assert!((u.cw_gain[0] - 1.15).abs() < 1e-5);
        assert!(u.wb_gains[0] > 1.0 && u.wb_gains[2] < 1.0);
        assert!((u.gamma - 0.90).abs() < 1e-4);
        assert_eq!(u.lut_preset, LutPreset::Kodak2383.shader_id());
    }

    #[test]
    fn test_layer_uniforms_zero_opacity_returns_none() {
        let mut clip = Clip::new("media_1", 0.0, 5.0);
        clip.opacity = 0.0;
        assert!(LayerUniforms::from_clip(&clip, 640, 360, 640, 360, 1.0).is_none());
    }

    #[test]
    fn test_bake_adjustment_curves_lut_applies_custom_curves() {
        let mut clip = Clip::new("adj", 0.0, 5.0);
        let curves = trim_core::color::CustomCurvesConfig {
            master: vec![[0.0, 0.2], [1.0, 0.8]],
            ..Default::default()
        };
        clip.custom_curves = Some(curves);

        let lut = bake_adjustment_curves_lut(&clip, None, 17);
        assert_eq!(lut.size, 17);
        // Black (0,0,0) should map close to 0.2 -> ~51
        let sampled_black = lut.sample(0.0, 0.0, 0.0);
        assert!(
            (sampled_black[0] - 0.2).abs() < 0.02,
            "Expected ~0.2, got {}",
            sampled_black[0]
        );
    }
}
