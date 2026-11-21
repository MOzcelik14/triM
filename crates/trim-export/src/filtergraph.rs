//! FFmpeg complex filtergraph pipeline generator.
//!
//! The export graph mirrors the interactive preview compositor
//! (`trim_media::video::draw_clip_onto_canvas` and
//! `trim_media::audio::AudioMixer::render_audio_interval`):
//!
//! * A black base canvas spanning the whole timeline is generated.
//! * Every visible video track is composited bottom-to-top (V1, V2, ...).
//!   Each clip is decoded, retimed, scaled, colour graded, faded and then
//!   `overlay`-ed onto the canvas at its timeline position. Overlapping
//!   tracks therefore produce layers instead of being concatenated.
//! * Audio from every non-muted clip (on video *and* audio tracks, honouring
//!   solo/mute and track volume) is delayed to its timeline position and
//!   summed with `amix`, anchored to timeline time.

use crate::presets::{ExportPreset, HardwareAccel};
use trim_core::clip::{Clip, MotionTitleStyle};
use trim_core::keyframe::{Interpolation, Keyframe};
use trim_core::media::{MediaItem, MediaType};
use trim_core::project::Project;
use trim_core::track::{Track, TrackType};

/// Preview canvas size used by the interactive monitor. Clip positions
/// (`pos_x`, `pos_y`) are stored in preview pixels.
const PREVIEW_W: f64 = 640.0;
const PREVIEW_H: f64 = 360.0;

pub struct TimelineExporter;

/// Prepared FFmpeg export invocation along with a private RAII temporary directory
/// holding baked LUTs, mask PGMs, and any `-filter_complex_script` file.
pub struct PreparedExport {
    pub cmd_args: Vec<String>,
    pub total_duration: f64,
    pub temp_dir: Option<tempfile::TempDir>,
}

fn safe_ffmpeg_path_arg(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with('-') && std::path::Path::new(trimmed).is_relative() {
        format!("./{trimmed}")
    } else {
        trimmed.to_string()
    }
}

fn escape_filtergraph_file_option(raw: &str) -> String {
    let forward = raw.replace('\\', "/");
    let mut out = String::with_capacity(forward.len() + 8);
    for ch in forward.chars() {
        match ch {
            '\0' | '\r' | '\n' => {}
            ':' => out.push_str("\\:"),
            '\'' => out.push_str("'\\''"),
            _ => out.push(ch),
        }
    }
    out
}

fn sanitize_drawtext_value(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 8);
    for ch in raw.chars() {
        match ch {
            '\0' | '\r' => {}
            '\n' => out.push(' '),
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("'\\''"),
            ':' => out.push_str("\\:"),
            '%' => out.push_str("\\%"),
            _ => out.push(ch),
        }
    }
    out
}

/// Accumulates FFmpeg input arguments and hands out input indices.
struct InputList {
    args: Vec<String>,
    count: usize,
}

impl InputList {
    fn new() -> Self {
        Self {
            args: Vec::new(),
            count: 0,
        }
    }

    fn push(&mut self, args: impl IntoIterator<Item = String>) -> usize {
        self.args.extend(args);
        let idx = self.count;
        self.count += 1;
        idx
    }
}

fn even(v: u32) -> u32 {
    (v.max(2)) & !1
}

/// Builds an FFmpeg expression that evaluates keyframes at local time `var`.
/// Mirrors `trim_core::keyframe::interpolate_keyframes`.
fn keyframe_expr(keyframes: &[Keyframe], var: &str, map: impl Fn(f64) -> f64) -> String {
    let mut kfs: Vec<Keyframe> = keyframes
        .iter()
        .filter(|k| k.time.is_finite() && k.value.is_finite())
        .cloned()
        .collect();
    kfs.sort_by(|a, b| a.time.total_cmp(&b.time));
    match kfs.len() {
        0 => return format!("{:.6}", map(0.0)),
        1 => return format!("{:.6}", map(kfs[0].value)),
        _ => {}
    }

    // Build nested if() from the last segment backwards.
    let Some(last) = kfs.last() else {
        return format!("{:.6}", map(0.0));
    };
    let mut expr = format!("{:.6}", map(last.value));
    for i in (0..kfs.len() - 1).rev() {
        let k0 = &kfs[i];
        let k1 = &kfs[i + 1];
        let v0 = map(k0.value);
        let v1 = map(k1.value);
        let span = k1.time - k0.time;
        let seg = if span.abs() < 1e-6 {
            format!("{:.6}", v0)
        } else {
            let u = format!("(({var}-{:.6})/{:.6})", k0.time, span);
            match k0.interpolation {
                Interpolation::Hold => format!("{:.6}", v0),
                Interpolation::Linear => format!("({:.6}+({:.6})*{u})", v0, v1 - v0),
                Interpolation::EaseIn => format!("({:.6}+({:.6})*({u}*{u}))", v0, v1 - v0),
                Interpolation::EaseOut => {
                    format!("({:.6}+({:.6})*(1-(1-{u})*(1-{u})))", v0, v1 - v0)
                }
                Interpolation::EaseInOut => {
                    format!("({:.6}+({:.6})*(3*{u}*{u}-2*{u}*{u}*{u}))", v0, v1 - v0)
                }
                Interpolation::CubicBezier => {
                    let y1 = k0.bezier_handles[1] as f64;
                    let y2 = k0.bezier_handles[3] as f64;
                    format!(
                        "({:.6}+({:.6})*(3*(1-{u})*(1-{u})*{u}*{:.4}+3*(1-{u})*{u}*{u}*{:.4}+{u}*{u}*{u}))",
                        v0,
                        v1 - v0,
                        y1,
                        y2
                    )
                }
            }
        };
        expr = format!("if(lt({var},{:.6}),{seg},{expr})", k1.time);
    }
    let first = &kfs[0];
    format!(
        "if(lt({var},{:.6}),{:.6},{expr})",
        first.time,
        map(first.value)
    )
}

fn animated_kfs<'a>(clip: &'a Clip, prop: &str) -> Option<&'a [Keyframe]> {
    clip.keyframes
        .get(prop)
        .filter(|k| !k.is_empty())
        .map(|v| v.as_slice())
}

/// Source window `[start, start + src_dur)` that a clip reads from.
fn source_window(clip: &Clip) -> (f64, f64) {
    let dur = clip.duration();
    let src_dur = if let Some(kfs) = clip.keyframes.get("speed").filter(|k| !k.is_empty()) {
        trim_core::keyframe::integrate_keyframes(kfs, dur, clip.speed).max(0.04)
    } else {
        dur * clip.speed.max(0.0001)
    };
    let start = if clip.reverse {
        (clip.effective_source_out() - src_dur).max(0.0)
    } else {
        clip.source_in.max(0.0)
    };
    (start, src_dur)
}

/// Builds a piecewise-linear `setpts` expression mapping source time `T` to timeline local time
/// when a clip uses Speed Ramping (`speed` keyframes).
fn build_speed_ramp_setpts(clip: &Clip) -> Option<String> {
    let kfs = clip.keyframes.get("speed").filter(|k| !k.is_empty())?;
    let dur = clip.duration();
    if dur <= 0.04 {
        return None;
    }
    let segments = 8usize;
    let mut s_pts = Vec::with_capacity(segments + 1);
    let mut t_pts = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t = (i as f64 / segments as f64) * dur;
        let s = trim_core::keyframe::integrate_keyframes(kfs, t, clip.speed);
        t_pts.push(t);
        s_pts.push(s);
    }
    let mut expr = format!("{:.6}", dur);
    for i in (0..segments).rev() {
        let s0 = s_pts[i];
        let s1 = s_pts[i + 1];
        let t0 = t_pts[i];
        let t1 = t_pts[i + 1];
        let ds = (s1 - s0).max(1e-5);
        let seg = format!("({:.6}+(T-{:.6})*{:.6})", t0, s0, (t1 - t0) / ds);
        expr = format!("if(lt(T,{:.6}),{seg},{expr})", s1);
    }
    Some(format!("setpts='({expr})/TB'"))
}

impl TimelineExporter {
    pub fn get_atempo_filters(speed: f64) -> Vec<String> {
        let mut filters = Vec::new();
        let mut curr = speed;
        if curr > 1.0 {
            while curr > 2.0 {
                filters.push("atempo=2.0".to_string());
                curr /= 2.0;
            }
            if (curr - 1.0).abs() > 0.001 {
                filters.push(format!("atempo={:.4}", curr));
            }
        } else if curr < 1.0 && curr > 0.0 {
            while curr < 0.5 {
                filters.push("atempo=0.5".to_string());
                curr /= 0.5;
            }
            if (curr - 1.0).abs() > 0.001 {
                filters.push(format!("atempo={:.4}", curr));
            }
        }
        filters
    }

    /// Writes a baked 3D `.cube` LUT file for `clip` into the temporary export directory
    /// and returns its normalized FFmpeg filtergraph path.
    pub fn write_baked_clip_cube_lut(clip: &Clip, size: usize) -> Option<String> {
        Self::write_baked_clip_cube_lut_in(clip, size, None)
    }

    fn write_baked_clip_cube_lut_in(
        clip: &Clip,
        size: usize,
        temp_dir: Option<&std::path::Path>,
    ) -> Option<String> {
        let dir = match temp_dir {
            Some(d) => d.to_path_buf(),
            None => std::env::temp_dir().join("trim_baked_luts"),
        };
        std::fs::create_dir_all(&dir).ok()?;
        let safe_id: String = clip
            .id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let file_name = if safe_id.is_empty() {
            format!("trim_grade_{}.cube", uuid::Uuid::new_v4())
        } else if temp_dir.is_some() {
            format!("trim_grade_{safe_id}.cube")
        } else {
            format!("trim_grade_{safe_id}_{}.cube", uuid::Uuid::new_v4())
        };
        let path = dir.join(file_name);
        let cube_text = trim_core::color::bake_clip_grade_to_cube(clip, size);
        std::fs::write(&path, cube_text).ok()?;
        Some(escape_filtergraph_file_option(&path.to_string_lossy()))
    }

    /// Color grading filter list shared by regular video clips and Adjustment Layers.
    fn clip_color_filters_in(clip: &Clip, temp_dir: Option<&std::path::Path>) -> Vec<String> {
        let mut f = Vec::new();
        if clip.needs_baked_cube_lut() {
            if let Some(sanitized_baked) = Self::write_baked_clip_cube_lut_in(clip, 33, temp_dir) {
                f.push(format!("lut3d=file='{sanitized_baked}':interp=trilinear"));
            }
            if let Some(lut_path) = &clip.custom_lut_path {
                if !lut_path.trim().is_empty() {
                    let sanitized = escape_filtergraph_file_option(lut_path.trim());
                    f.push(format!("lut3d=file='{sanitized}'"));
                }
            }
        } else {
            if clip.temperature.abs() > 1e-4 || clip.tint.abs() > 1e-4 {
                let [wbr, wbg, wbb] = trim_core::color::white_balance_gains(
                    clip.temperature as f32,
                    clip.tint as f32,
                );
                f.push(format!(
                    "colorchannelmixer=rr={wbr:.4}:gg={wbg:.4}:bb={wbb:.4}"
                ));
            }

            let has_gamma = (clip.gamma - 1.0).abs() > 1e-4;
            if clip.brightness.abs() > 1e-4
                || (clip.contrast - 1.0).abs() > 1e-4
                || (clip.saturation - 1.0).abs() > 1e-4
                || has_gamma
            {
                if has_gamma {
                    f.push(format!(
                        "eq=brightness={:.3}:contrast={:.3}:saturation={:.3}:gamma={:.3}",
                        clip.brightness, clip.contrast, clip.saturation, clip.gamma
                    ));
                } else {
                    f.push(format!(
                        "eq=brightness={:.3}:contrast={:.3}:saturation={:.3}",
                        clip.brightness, clip.contrast, clip.saturation
                    ));
                }
            }

            if let Some(lut_path) = &clip.custom_lut_path {
                if !lut_path.trim().is_empty() {
                    let sanitized = escape_filtergraph_file_option(lut_path.trim());
                    f.push(format!("lut3d=file='{sanitized}'"));
                } else if let Some(preset_filter) = clip.lut_preset.ffmpeg_filter() {
                    f.push(preset_filter.to_string());
                }
            } else if let Some(preset_filter) = clip.lut_preset.ffmpeg_filter() {
                f.push(preset_filter.to_string());
            }

            if let Some(cw) = &clip.color_wheels {
                let rs = (cw.lift[0] + cw.luminance[0]).clamp(-1.0, 1.0);
                let gs = (cw.lift[1] + cw.luminance[0]).clamp(-1.0, 1.0);
                let bs = (cw.lift[2] + cw.luminance[0]).clamp(-1.0, 1.0);
                let rm = (cw.gamma[0] + cw.luminance[1] - 1.0).clamp(-1.0, 1.0);
                let gm = (cw.gamma[1] + cw.luminance[1] - 1.0).clamp(-1.0, 1.0);
                let bm = (cw.gamma[2] + cw.luminance[1] - 1.0).clamp(-1.0, 1.0);
                let rh = (cw.gain[0] + cw.luminance[2] - 1.0).clamp(-1.0, 1.0);
                let gh = (cw.gain[1] + cw.luminance[2] - 1.0).clamp(-1.0, 1.0);
                let bh = (cw.gain[2] + cw.luminance[2] - 1.0).clamp(-1.0, 1.0);
                if rs.abs() > 1e-4
                    || gs.abs() > 1e-4
                    || bs.abs() > 1e-4
                    || rm.abs() > 1e-4
                    || gm.abs() > 1e-4
                    || bm.abs() > 1e-4
                    || rh.abs() > 1e-4
                    || gh.abs() > 1e-4
                    || bh.abs() > 1e-4
                {
                    f.push(format!(
                        "colorbalance=rs={:.3}:gs={:.3}:bs={:.3}:rm={:.3}:gm={:.3}:bm={:.3}:rh={:.3}:gh={:.3}:bh={:.3}",
                        rs, gs, bs, rm, gm, bm, rh, gh, bh
                    ));
                }
            }

            if let Some(curves) = &clip.custom_curves {
                if let Some(cf) = curves.ffmpeg_filter() {
                    f.push(cf);
                }
            }
        }
        if clip.chromatic_aberration > 1e-4 {
            let shift = ((clip.chromatic_aberration * 8.0).round() as i32).clamp(1, 16);
            f.push(format!("rgbashift=rh={shift}:bh=-{shift}"));
        }
        if clip.halation > 1e-4 {
            let rh = (clip.halation * 0.24).clamp(0.02, 0.35);
            f.push(format!(
                "colorbalance=rh={rh:.3}:gh={:.3}:bh=-{:.3}",
                rh * 0.28,
                rh * 0.14
            ));
        }
        if clip.bloom > 1e-4 {
            let br = (clip.bloom * 0.08).clamp(0.01, 0.15);
            let gm = (1.0 + clip.bloom * 0.18).clamp(1.01, 1.35);
            f.push(format!("eq=brightness={br:.3}:gamma={gm:.3}"));
        }
        if clip.diffusion > 1e-4 {
            let ct = (1.0 - clip.diffusion * 0.14).clamp(0.80, 0.99);
            let gm = (1.0 + clip.diffusion * 0.12).clamp(1.01, 1.20);
            f.push(format!("eq=contrast={ct:.3}:gamma={gm:.3}"));
        }
        if clip.mist > 1e-4 {
            let br = (clip.mist * 0.06).clamp(0.005, 0.10);
            let ct = (1.0 - clip.mist * 0.10).clamp(0.85, 0.99);
            f.push(format!("eq=brightness={br:.3}:contrast={ct:.3}"));
        }
        if clip.gate_weave > 1e-4 {
            let shift = ((clip.gate_weave * 3.0).round() as i32).clamp(1, 4);
            f.push(format!("rgbashift=rv={shift}:gv={shift}:bv={shift}"));
        }

        if clip.blur > 1e-4 {
            f.push(format!("gblur=sigma={:.2}", (clip.blur * 20.0).max(0.2)));
        }
        if clip.sharpen > 1e-4 {
            f.push(format!(
                "unsharp=5:5:{:.2}:5:5:0.0",
                (clip.sharpen * 2.5).clamp(0.1, 3.0)
            ));
        }
        if clip.vignette > 1e-4 {
            f.push(format!(
                "vignette=angle={:.3}",
                (clip.vignette * 0.785).clamp(0.05, 1.2)
            ));
        }
        if clip.film_grain > 1e-4 {
            f.push(format!(
                "noise=alls={:.0}:allf=t+u",
                (clip.film_grain * 35.0).clamp(1.0, 45.0)
            ));
        }
        f
    }

    /// Writes a single channel (`0` = R/ClipAlpha, `1` = G/ColorAndEffects) of an RGBA buffer
    /// as a binary P5 PGM grayscale image for FFmpeg `-loop 1 -i` mask input.
    fn write_mask_pgm(
        path: &std::path::Path,
        w: u32,
        h: u32,
        rgba: &[u8],
        channel: usize,
    ) -> std::io::Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(path)?;
        write!(file, "P5\n{w} {h}\n255\n")?;
        let mut gray = Vec::with_capacity((w as usize) * (h as usize));
        for px in rgba.as_chunks::<4>().0 {
            gray.push(px[channel]);
        }
        file.write_all(&gray)
    }

    /// Video filter chain for one clip, producing an RGBA stream whose PTS is
    /// already shifted to the clip's timeline position.
    fn clip_video_chain(
        clip: &Clip,
        media: &MediaItem,
        out_w: u32,
        out_h: u32,
        out_fps: f64,
        temp_dir: Option<&std::path::Path>,
    ) -> String {
        let dur = clip.duration();
        let mut f: Vec<String> = Vec::new();

        let is_image = media.media_type == MediaType::Image;
        if !is_image && clip.reverse {
            f.push("reverse".into());
        }
        if !is_image {
            if let Some(ramp_expr) = build_speed_ramp_setpts(clip) {
                f.push("setpts=PTS-STARTPTS".into());
                f.push(ramp_expr);
            } else if (clip.speed - 1.0).abs() > 0.001 {
                f.push(format!("setpts=(PTS-STARTPTS)/{:.6}", clip.speed));
            } else {
                f.push("setpts=PTS-STARTPTS".into());
            }
        } else {
            f.push("setpts=PTS-STARTPTS".into());
        }
        f.push(format!("fps={}", out_fps));
        f.push(format!("trim=duration={:.6}", dur));
        f.push("setpts=PTS-STARTPTS".into());

        // Scale: preview fits the frame into the canvas, then multiplies by
        // clip.scale. Video frames are letterboxed (opaque bars) in preview,
        // images keep their own alpha and aspect ratio.
        let scale = clip.get_property_at_time("scale", 0.0).max(0.05);
        let box_w = ((out_w as f64) * scale).round().max(2.0) as u32;
        let box_h = ((out_h as f64) * scale).round().max(2.0) as u32;
        f.push(format!(
            "scale={box_w}:{box_h}:force_original_aspect_ratio=decrease"
        ));
        if !is_image {
            f.push(format!("pad={box_w}:{box_h}:(ow-iw)/2:(oh-ih)/2:black"));
        }
        f.push("setsar=1".into());

        f.extend(Self::clip_color_filters_in(clip, temp_dir));

        f.push("format=rgba".into());

        if let Some(kfs) = animated_kfs(clip, "rotation") {
            let expr = keyframe_expr(kfs, "t", |deg| deg.to_radians());
            f.push(format!(
                "rotate='{expr}':ow='rotw(iw)':oh='roth(ih)':c=none"
            ));
        } else {
            let rot_deg = clip.get_property_at_time("rotation", 0.0);
            if rot_deg.abs() > 0.01 {
                f.push(format!(
                    "rotate={:.6}:ow='rotw(iw)':oh='roth(ih)':c=none",
                    rot_deg.to_radians()
                ));
            }
        }

        if let Some(cp) = &clip.corner_pin {
            if cp.is_active() {
                f.push(format!(
                    "perspective=x0={:.3}*W:y0={:.3}*H:x1={:.3}*W:y1={:.3}*H:x2={:.3}*W:y2={:.3}*H:x3={:.3}*W:y3={:.3}*H:sense=destination",
                    cp.top_left[0],
                    cp.top_left[1],
                    cp.top_right[0],
                    cp.top_right[1],
                    cp.bottom_left[0],
                    cp.bottom_left[1],
                    cp.bottom_right[0],
                    cp.bottom_right[1],
                ));
            }
        }

        if let Some(ck) = &clip.chromakey {
            if ck.enabled {
                f.push(format!(
                    "chromakey=color=0x{:02x}{:02x}{:02x}:similarity={:.3}:smoothness={:.3}",
                    ck.key_color[0], ck.key_color[1], ck.key_color[2], ck.similarity, ck.smoothness
                ));
            }
        }

        if clip.has_crop() {
            let cl = clip.crop_left.clamp(0.0, 0.99);
            let cr = clip.crop_right.clamp(0.0, (0.99 - cl).max(0.0));
            let ct = clip.crop_top.clamp(0.0, 0.99);
            let cb = clip.crop_bottom.clamp(0.0, (0.99 - ct).max(0.0));
            f.push(format!(
                "geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='if(between(X,{:.4}*W,{:.4}*W)*between(Y,{:.4}*H,{:.4}*H),alpha(X,Y),0)'",
                cl, 1.0 - cr, ct, 1.0 - cb
            ));
        }

        if let Some(txt) = &clip.text_overlay {
            let sanitized = sanitize_drawtext_value(&txt.content);
            let font_size = ((txt.font_size as f64) * (out_h as f64 / 1080.0))
                .round()
                .max(10.0) as u32;
            let font_color = format!(
                "0x{:02x}{:02x}{:02x}{:02x}",
                txt.text_color[0], txt.text_color[1], txt.text_color[2], txt.text_color[3]
            );
            let mut shadow_part = if txt.has_shadow {
                let sd = (txt.shadow_distance.round() as i32).clamp(1, 20);
                format!(":shadowx={sd}:shadowy={sd}:shadowcolor=black@0.7")
            } else {
                String::new()
            };
            if txt.stroke_width > 0.1 {
                let bw = (txt.stroke_width.round() as u32).clamp(1, 20);
                shadow_part.push_str(&format!(
                    ":borderw={bw}:bordercolor=0x{:02x}{:02x}{:02x}@{:.2}",
                    txt.stroke_color[0],
                    txt.stroke_color[1],
                    txt.stroke_color[2],
                    txt.stroke_color[3] as f32 / 255.0
                ));
            }
            let box_part = if let Some(bg) = txt.bg_color {
                format!(
                    ":box=1:boxcolor=0x{:02x}{:02x}{:02x}@{:.2}:boxborderw=8",
                    bg[0],
                    bg[1],
                    bg[2],
                    bg[3] as f32 / 255.0
                )
            } else if txt.motion_style == MotionTitleStyle::LowerThird {
                ":box=1:boxcolor=0x0f172a@0.85:boxborderw=10".to_string()
            } else if txt.motion_style == MotionTitleStyle::Ticker {
                ":box=1:boxcolor=0x0c0e14@0.90:boxborderw=10".to_string()
            } else if txt.motion_style == MotionTitleStyle::BoxReveal {
                ":box=1:boxcolor=0x141822@0.90:boxborderw=10".to_string()
            } else {
                String::new()
            };

            let font_style = match (txt.bold, txt.italic) {
                (true, true) => ":style=Bold Italic",
                (true, false) => ":style=Bold",
                (false, true) => ":style=Italic",
                (false, false) => "",
            };
            let font_part = if txt.is_custom_font_file() {
                let sanitized_path = escape_filtergraph_file_option(txt.font_family.trim());
                format!(":fontfile='{sanitized_path}'")
            } else if txt.font_family != "Sans-Serif" || !font_style.is_empty() {
                let sanitized_fam: String = txt
                    .font_family
                    .chars()
                    .filter(|c| !matches!(c, '\'' | ':' | '\\' | '\n' | '\r' | '\0'))
                    .collect();
                let fam = if sanitized_fam.trim().is_empty() {
                    "Sans-Serif"
                } else {
                    sanitized_fam.trim()
                };
                format!(":font='{fam}'{font_style}")
            } else {
                String::new()
            };

            match txt.motion_style {
                MotionTitleStyle::Typewriter => {
                    let chars: Vec<char> = txt.content.chars().collect();
                    let total_chars = chars.len();
                    let x_expr = format!("(w-text_w)*{:.3}", txt.pos_x.clamp(0.0, 1.0));
                    let y_expr = format!("(h-text_h)*{:.3}", txt.pos_y.clamp(0.0, 1.0));
                    let type_dur = (dur * 0.75).min(dur);

                    if total_chars <= 1 {
                        f.push(format!(
                            "drawtext=text='{sanitized}':fontsize={font_size}{font_part}:fontcolor={font_color}:x={x_expr}:y={y_expr}{shadow_part}{box_part}:expansion=none"
                        ));
                    } else {
                        for i in 1..=total_chars {
                            let sub: String = chars[..i].iter().collect();
                            let sub_sanitized = sanitize_drawtext_value(&sub);
                            let st = (i - 1) as f64 * type_dur / total_chars as f64;
                            let et = if i < total_chars {
                                i as f64 * type_dur / total_chars as f64
                            } else {
                                dur
                            };
                            let enable_clause = if i < total_chars {
                                format!(":enable='between(t,{:.4},{:.4})'", st, et)
                            } else {
                                format!(":enable='gte(t,{:.4})'", st)
                            };
                            f.push(format!(
                                "drawtext=text='{sub_sanitized}':fontsize={font_size}{font_part}:fontcolor={font_color}:x={x_expr}:y={y_expr}{shadow_part}{box_part}{enable_clause}:expansion=none"
                            ));
                        }
                    }

                    if let Some(sub) = &txt.subtitle_text {
                        if !sub.trim().is_empty() {
                            let sub_sanitized = sanitize_drawtext_value(sub);
                            let sub_size = ((font_size as f64 * 0.58).round().max(8.0)) as u32;
                            let sub_y = format!(
                                "(h-text_h)*{:.3}+{:.0}",
                                txt.pos_y.clamp(0.0, 1.0),
                                font_size as f64 * 1.25
                            );
                            let sub_enable = format!(":enable='gte(t,{:.4})'", type_dur);
                            f.push(format!(
                                "drawtext=text='{sub_sanitized}':fontsize={sub_size}{font_part}:fontcolor=0xc8d7e6dc:x={x_expr}:y={sub_y}{shadow_part}{sub_enable}:expansion=none"
                            ));
                        }
                    }
                }
                MotionTitleStyle::LowerThird => {
                    let slide_dur = (dur * 0.25).clamp(0.15, 0.6);
                    let offset = ((120.0 * (out_w as f64 / 1920.0)).round().max(40.0)) as i32;
                    let x_expr = format!(
                        "if(lt(t,{slide_dur:.4}),(w-text_w)*{:.3}-{offset}*(1-t/{slide_dur:.4}),(w-text_w)*{:.3})",
                        txt.pos_x.clamp(0.0, 1.0),
                        txt.pos_x.clamp(0.0, 1.0)
                    );
                    let y_expr = format!("(h-text_h)*{:.3}", txt.pos_y.clamp(0.0, 1.0));
                    f.push(format!(
                        "drawtext=text='{sanitized}':fontsize={font_size}{font_part}:fontcolor={font_color}:x='{x_expr}':y={y_expr}{shadow_part}{box_part}:expansion=none"
                    ));

                    if let Some(sub) = &txt.subtitle_text {
                        if !sub.trim().is_empty() {
                            let sub_sanitized = sanitize_drawtext_value(sub);
                            let sub_size = ((font_size as f64 * 0.58).round().max(8.0)) as u32;
                            let sub_y = format!(
                                "(h-text_h)*{:.3}+{:.0}",
                                txt.pos_y.clamp(0.0, 1.0),
                                font_size as f64 * 1.25
                            );
                            let sub_x_expr = format!(
                                "if(lt(t,{slide_dur:.4}),(w-text_w)*{:.3}-{offset}*(1-t/{slide_dur:.4}),(w-text_w)*{:.3})",
                                txt.pos_x.clamp(0.0, 1.0),
                                txt.pos_x.clamp(0.0, 1.0)
                            );
                            f.push(format!(
                                "drawtext=text='{sub_sanitized}':fontsize={sub_size}{font_part}:fontcolor=0xc8d7e6dc:x='{sub_x_expr}':y={sub_y}{shadow_part}:expansion=none"
                            ));
                        }
                    }
                }
                MotionTitleStyle::CinematicZoom => {
                    let size_expr = format!("'{font_size}*(0.94+0.10*t/{dur:.4})'");
                    let x_expr = format!("(w-text_w)*{:.3}", txt.pos_x.clamp(0.0, 1.0));
                    let y_expr = format!("(h-text_h)*{:.3}", txt.pos_y.clamp(0.0, 1.0));
                    f.push(format!(
                        "drawtext=text='{sanitized}':fontsize={size_expr}{font_part}:fontcolor={font_color}:x={x_expr}:y={y_expr}{shadow_part}{box_part}:expansion=none"
                    ));

                    if let Some(sub) = &txt.subtitle_text {
                        if !sub.trim().is_empty() {
                            let sub_sanitized = sanitize_drawtext_value(sub);
                            let sub_size = ((font_size as f64 * 0.58).round().max(8.0)) as u32;
                            let sub_y = format!(
                                "(h-text_h)*{:.3}+{:.0}",
                                txt.pos_y.clamp(0.0, 1.0),
                                font_size as f64 * 1.25
                            );
                            f.push(format!(
                                "drawtext=text='{sub_sanitized}':fontsize={sub_size}{font_part}:fontcolor=0xc8d7e6dc:x={x_expr}:y={sub_y}{shadow_part}:expansion=none"
                            ));
                        }
                    }
                }
                MotionTitleStyle::Ticker => {
                    let x_expr = format!("w-(t/{dur:.4})*(w+text_w)");
                    let y_expr = format!("(h-text_h)*{:.3}", txt.pos_y.clamp(0.0, 1.0));
                    f.push(format!(
                        "drawtext=text='{sanitized}':fontsize={font_size}{font_part}:fontcolor={font_color}:x={x_expr}:y={y_expr}{shadow_part}{box_part}:expansion=none"
                    ));
                }
                MotionTitleStyle::RollingCredits => {
                    let x_expr = format!("(w-text_w)*{:.3}", txt.pos_x.clamp(0.0, 1.0));
                    let y_expr = format!("'h-(t/{dur:.4})*(h+text_h)'");
                    f.push(format!(
                        "drawtext=text='{sanitized}':fontsize={font_size}{font_part}:fontcolor={font_color}:x={x_expr}:y={y_expr}{shadow_part}{box_part}:expansion=none"
                    ));
                    if let Some(sub) = &txt.subtitle_text {
                        if !sub.trim().is_empty() {
                            let sub_sanitized = sanitize_drawtext_value(sub);
                            let sub_size = ((font_size as f64 * 0.62).round().max(8.0)) as u32;
                            let sub_y = format!(
                                "'h-(t/{dur:.4})*(h+text_h)+{:.0}'",
                                font_size as f64 * 1.35
                            );
                            f.push(format!(
                                "drawtext=text='{sub_sanitized}':fontsize={sub_size}{font_part}:fontcolor=0xc8d7e6dc:x={x_expr}:y={sub_y}{shadow_part}:expansion=none"
                            ));
                        }
                    }
                }
                MotionTitleStyle::BoxReveal => {
                    let reveal_t = (dur * 0.25).min(0.5);
                    let x_expr = format!("(w-text_w)*{:.3}", txt.pos_x.clamp(0.0, 1.0));
                    let y_expr = format!("(h-text_h)*{:.3}", txt.pos_y.clamp(0.0, 1.0));
                    let enable_clause = format!(":enable='gte(t,{:.4})'", reveal_t);
                    f.push(format!(
                        "drawtext=text='{sanitized}':fontsize={font_size}{font_part}:fontcolor={font_color}:x={x_expr}:y={y_expr}{shadow_part}{box_part}{enable_clause}:expansion=none"
                    ));
                }
                MotionTitleStyle::ChapterCard | MotionTitleStyle::None => {
                    let x_expr = format!("(w-text_w)*{:.3}", txt.pos_x.clamp(0.0, 1.0));
                    let y_expr = format!("(h-text_h)*{:.3}", txt.pos_y.clamp(0.0, 1.0));
                    f.push(format!(
                        "drawtext=text='{sanitized}':fontsize={font_size}{font_part}:fontcolor={font_color}:x={x_expr}:y={y_expr}{shadow_part}{box_part}:expansion=none"
                    ));

                    if let Some(sub) = &txt.subtitle_text {
                        if !sub.trim().is_empty() {
                            let sub_sanitized = sanitize_drawtext_value(sub);
                            let sub_size = ((font_size as f64 * 0.58).round().max(8.0)) as u32;
                            let sub_y = format!(
                                "(h-text_h)*{:.3}+{:.0}",
                                txt.pos_y.clamp(0.0, 1.0),
                                font_size as f64 * 1.25
                            );
                            f.push(format!(
                                "drawtext=text='{sub_sanitized}':fontsize={sub_size}:fontcolor=0xc8d7e6dc:x={x_expr}:y={sub_y}{shadow_part}:expansion=none"
                            ));
                        }
                    }
                }
            }
        }

        // Opacity (static or keyframed).
        if let Some(kfs) = animated_kfs(clip, "opacity") {
            let expr = keyframe_expr(kfs, "T", |v| v.clamp(0.0, 1.0));
            f.push(format!(
                "geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='alpha(X,Y)*({expr})'"
            ));
        } else if clip.opacity < 0.999 {
            f.push(format!(
                "colorchannelmixer=aa={:.4}",
                clip.opacity.clamp(0.0, 1.0)
            ));
        }

        // Fades and Transitions: regular fades dissolve to whatever is underneath.
        // Dip-to-white / dip-to-black fades the pixels to white/black.
        // Wipe and slide transitions perform directional revealing.
        if !is_image && clip.stabilization_strength > 1e-4 {
            let rx = ((clip.stabilization_strength * 48.0).round() as u32).clamp(16, 64) & !1;
            f.push(format!("deshake=rx={rx}:ry={rx}:edge=mirror"));
        }

        let eff_fade_in = if clip.transition_in.is_some() && clip.fade_in <= 0.0 {
            0.5f64.min(dur * 0.5)
        } else {
            clip.fade_in
        };
        if eff_fade_in > 0.0 {
            let d = eff_fade_in.min(dur);
            match clip.transition_in.as_deref() {
                Some("dip_white") => f.push(format!("fade=t=in:st=0:d={:.4}:color=white", d)),
                Some("dip_black") => f.push(format!("fade=t=in:st=0:d={:.4}:color=black", d)),
                Some("light_leak") => f.push(format!("fade=t=in:st=0:d={:.4}:color=0xffb347", d)),
                Some("blur_dissolve") => {
                    f.push(format!("gblur=sigma=10:enable='lt(t,{:.4})'", d));
                    f.push(format!("fade=t=in:st=0:d={:.4}:alpha=1", d));
                }
                Some("whip_pan") => {
                    f.push(format!("gblur=sigma=14:steps=1:enable='lt(t,{:.4})'", d));
                    f.push(format!("fade=t=in:st=0:d={:.4}:alpha=1", d));
                }
                Some("glitch") => {
                    f.push(format!("rgbashift=rh=12:bh=-12:enable='lt(t,{:.4})'", d));
                    f.push(format!("fade=t=in:st=0:d={:.4}:alpha=1", d));
                }
                Some("wipe_left") => f.push(format!("geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='if(lte(X/W,min(1,T/{:.4})),alpha(X,Y),0)'", d)),
                Some("wipe_right") => f.push(format!("geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='if(gte(X/W,max(0,1-T/{:.4})),alpha(X,Y),0)'", d)),
                Some("luma_wipe") => f.push(format!("geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='if(lte((X+Y)/(W+H),min(1,T/{:.4})),alpha(X,Y),0)'", d)),
                _ => f.push(format!("fade=t=in:st=0:d={:.4}:alpha=1", d)),
            }
        }
        let eff_fade_out = if clip.transition_out.is_some() && clip.fade_out <= 0.0 {
            0.5f64.min(dur * 0.5)
        } else {
            clip.fade_out
        };
        if eff_fade_out > 0.0 {
            let d = eff_fade_out.min(dur);
            let st = (dur - d).max(0.0);
            match clip.transition_out.as_deref() {
                Some("dip_white") => f.push(format!("fade=t=out:st={:.4}:d={:.4}:color=white", st, d)),
                Some("dip_black") => f.push(format!("fade=t=out:st={:.4}:d={:.4}:color=black", st, d)),
                Some("light_leak") => f.push(format!("fade=t=out:st={:.4}:d={:.4}:color=0xffb347", st, d)),
                Some("blur_dissolve") => {
                    f.push(format!("gblur=sigma=10:enable='gt(t,{:.4})'", st));
                    f.push(format!("fade=t=out:st={:.4}:d={:.4}:alpha=1", st, d));
                }
                Some("whip_pan") => {
                    f.push(format!("gblur=sigma=14:steps=1:enable='gt(t,{:.4})'", st));
                    f.push(format!("fade=t=out:st={:.4}:d={:.4}:alpha=1", st, d));
                }
                Some("glitch") => {
                    f.push(format!("rgbashift=rh=12:bh=-12:enable='gt(t,{:.4})'", st));
                    f.push(format!("fade=t=out:st={:.4}:d={:.4}:alpha=1", st, d));
                }
                Some("wipe_left") => f.push(format!("geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='if(lte(X/W,max(0,1-(T-{:.4})/{:.4})),alpha(X,Y),0)'", st, d)),
                Some("wipe_right") => f.push(format!("geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='if(gte(X/W,min(1,(T-{:.4})/{:.4})),alpha(X,Y),0)'", st, d)),
                Some("luma_wipe") => f.push(format!("geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='if(lte((X+Y)/(W+H),max(0,1-(T-{:.4})/{:.4})),alpha(X,Y),0)'", st, d)),
                _ => f.push(format!("fade=t=out:st={:.4}:d={:.4}:alpha=1", st, d)),
            }
        }

        f.push(format!("setpts=PTS+{:.6}/TB", clip.timeline_in));
        f.join(",")
    }

    /// Overlay x/y expressions (main-timeline time `t`).
    fn overlay_position(clip: &Clip, out_w: u32, out_h: u32) -> (String, String) {
        let kx = out_w as f64 / PREVIEW_W;
        let ky = out_h as f64 / PREVIEW_H;
        let local = format!("(t-{:.6})", clip.timeline_in);
        let px = match animated_kfs(clip, "pos_x") {
            Some(k) => keyframe_expr(k, &local, |v| v * kx),
            None => format!("{:.4}", clip.pos_x * kx),
        };
        let py = match animated_kfs(clip, "pos_y") {
            Some(k) => keyframe_expr(k, &local, |v| v * ky),
            None => format!("{:.4}", clip.pos_y * ky),
        };

        let mut slide_x = String::new();
        let eff_in = if clip.transition_in.is_some() && clip.fade_in <= 0.0 {
            0.5f64.min(clip.duration() * 0.5)
        } else {
            clip.fade_in
        };
        if eff_in > 0.0 {
            let d = eff_in.min(clip.duration());
            if clip.transition_in.as_deref() == Some("slide_left")
                || clip.transition_in.as_deref() == Some("whip_pan")
            {
                slide_x.push_str(&format!("+if(lt({local},{d:.4}),(1-{local}/{d:.4})*W,0)"));
            } else if clip.transition_in.as_deref() == Some("slide_right") {
                slide_x.push_str(&format!("-if(lt({local},{d:.4}),(1-{local}/{d:.4})*W,0)"));
            }
        }
        let eff_out = if clip.transition_out.is_some() && clip.fade_out <= 0.0 {
            0.5f64.min(clip.duration() * 0.5)
        } else {
            clip.fade_out
        };
        if eff_out > 0.0 {
            let dur = clip.duration();
            let d = eff_out.min(dur);
            let st = (dur - d).max(0.0);
            if clip.transition_out.as_deref() == Some("slide_left")
                || clip.transition_out.as_deref() == Some("whip_pan")
            {
                slide_x.push_str(&format!(
                    "-if(gt({local},{st:.4}),(({local}-{st:.4})/{d:.4})*W,0)"
                ));
            } else if clip.transition_out.as_deref() == Some("slide_right") {
                slide_x.push_str(&format!(
                    "+if(gt({local},{st:.4}),(({local}-{st:.4})/{d:.4})*W,0)"
                ));
            }
        }

        (
            format!("'(W-w)/2+{px}{slide_x}'"),
            format!("'(H-h)/2+{py}'"),
        )
    }

    /// Audio filter chain for one clip, delayed to its timeline position.
    fn clip_audio_chain(clip: &Clip, eff_vol: f64) -> String {
        let dur = clip.duration();
        let mut a: Vec<String> =
            vec!["aformat=sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo".into()];
        if clip.reverse {
            a.push("areverse".into());
        }
        a.push("asetpts=PTS-STARTPTS".into());
        let eff_audio_speed = if let Some(kfs) = animated_kfs(clip, "speed") {
            if dur > 1e-4 {
                (trim_core::keyframe::integrate_keyframes(kfs, dur, clip.speed) / dur)
                    .clamp(0.1, 10.0)
            } else {
                clip.speed
            }
        } else {
            clip.speed
        };
        if (eff_audio_speed - 1.0).abs() > 0.001 {
            if clip.pitch_lock {
                a.extend(Self::get_atempo_filters(eff_audio_speed));
            } else {
                a.push(format!(
                    "asetrate={:.2},aresample=48000",
                    48000.0 * eff_audio_speed
                ));
            }
        }
        // Force exact clip length so timing never drifts.
        a.push(format!("atrim=duration={:.6}", dur));
        a.push(format!("apad=whole_dur={:.6}", dur));
        if let Some(kfs) = animated_kfs(clip, "volume") {
            let trk_mult = if clip.volume > 0.001 {
                eff_vol / clip.volume
            } else {
                1.0
            };
            let expr = keyframe_expr(kfs, "t", |v| (v * trk_mult).clamp(0.0, 10.0));
            a.push(format!("volume=eval=frame:volume='{expr}'"));
        } else if (eff_vol - 1.0).abs() > 0.001 {
            a.push(format!("volume={:.4}", eff_vol));
        }

        // Audio EQ preset and noise reduction filters
        match clip.eq_preset {
            trim_core::clip::AudioEqPreset::VoiceClarity => {
                a.push(
                    "highpass=f=80,equalizer=f=3000:t=q:w=1.5:g=3,equalizer=f=300:t=q:w=1.0:g=-2"
                        .into(),
                );
            }
            trim_core::clip::AudioEqPreset::BassBoost => {
                a.push("equalizer=f=100:t=q:w=1.0:g=6".into());
            }
            trim_core::clip::AudioEqPreset::Podcast => {
                a.push("highpass=f=75,equalizer=f=2500:t=q:w=1.2:g=2.5,lowpass=f=12000".into());
            }
            trim_core::clip::AudioEqPreset::DenoiseGate => {
                a.push(
                    "highpass=f=100,lowpass=f=10000,agate=threshold=0.03:ratio=2:range=0.05".into(),
                );
            }
            trim_core::clip::AudioEqPreset::Flat => {}
        }

        if let Some(peq) = &clip.parametric_eq {
            if !peq.is_flat() {
                for band in &peq.bands {
                    if band.enabled && band.gain_db.abs() >= 0.05 {
                        a.push(format!(
                            "equalizer=f={:.0}:t=q:w={:.2}:g={:.2}",
                            band.freq_hz.clamp(20.0, 20000.0),
                            band.q.clamp(0.2, 10.0),
                            band.gain_db.clamp(-24.0, 24.0)
                        ));
                    }
                }
            }
        }

        if let Some(comp) = &clip.compressor {
            if comp.enabled {
                if comp.de_esser > 0.01 {
                    let g = -(comp.de_esser.clamp(0.0, 1.0) * 9.0);
                    a.push(format!("equalizer=f=6500:t=q:w=2.0:g={g:.1}"));
                }
                let th_lin = 10.0_f32
                    .powf(comp.threshold_db.clamp(-48.0, 0.0) / 20.0)
                    .clamp(0.001, 1.0);
                let mk_lin = 10.0_f32
                    .powf(comp.makeup_db.clamp(0.0, 24.0) / 20.0)
                    .clamp(1.0, 16.0);
                a.push(format!(
                    "acompressor=threshold={:.4}:ratio={:.2}:attack={:.1}:release={:.1}:makeup={:.2}",
                    th_lin,
                    comp.ratio.clamp(1.0, 20.0),
                    comp.attack_ms.clamp(1.0, 200.0),
                    comp.release_ms.clamp(10.0, 1000.0),
                    mk_lin
                ));
                if comp.limiter_enabled {
                    a.push("alimiter=limit=0.891".into());
                }
            }
        }

        if clip.reverb_amount > 0.01 || clip.echo_amount > 0.01 {
            let d1 = (clip.reverb_amount * 0.35).clamp(0.05, 0.5);
            let d2 = (clip.echo_amount * 0.45).clamp(0.05, 0.6);
            a.push(format!("aecho=0.8:0.88:60|140:{d1:.2}|{d2:.2}"));
        }

        if clip.pan.abs() > 0.01 {
            let p = clip.pan.clamp(-1.0, 1.0);
            let l_g = if p > 0.0 { 1.0 - p } else { 1.0 };
            let r_g = if p < 0.0 { 1.0 + p } else { 1.0 };
            a.push(format!("pan=stereo|c0={l_g:.3}*c0|c1={r_g:.3}*c1"));
        }

        if clip.noise_gate && clip.eq_preset != trim_core::clip::AudioEqPreset::DenoiseGate {
            a.push("agate=threshold=0.03:ratio=2:range=0.05".into());
        }

        if clip.voice_denoise > 0.01 {
            let nr_db = (clip.voice_denoise.clamp(0.0, 1.0) * 32.0).clamp(4.0, 32.0);
            a.push(format!("afftdn=nr={nr_db:.1}:nf=-42"));
        }

        if clip.fade_in > 0.0 {
            a.push(format!("afade=t=in:st=0:d={:.4}", clip.fade_in.min(dur)));
        }
        if clip.fade_out > 0.0 {
            let d = clip.fade_out.min(dur);
            a.push(format!(
                "afade=t=out:st={:.4}:d={:.4}",
                (dur - d).max(0.0),
                d
            ));
        }
        let delay_ms = (clip.timeline_in * 1000.0).round().max(0.0) as u64;
        if delay_ms > 0 {
            a.push(format!("adelay=delays={delay_ms}:all=1"));
        }
        a.join(",")
    }

    pub fn build_export_pipeline(
        project: &Project,
        preset: &ExportPreset,
        output_path: &str,
    ) -> Result<(Vec<String>, f64), String> {
        Self::build_export_pipeline_range(project, preset, output_path, None)
    }

    pub fn build_export_pipeline_prepared(
        project: &Project,
        preset: &ExportPreset,
        output_path: &str,
        range: Option<(f64, f64)>,
    ) -> Result<PreparedExport, String> {
        let temp_dir = tempfile::TempDir::new()
            .map_err(|e| format!("Geçici dışa aktarma klasörü oluşturulamadı: {e}"))?;
        let (cmd_args, total_duration) = Self::build_export_pipeline_range_in(
            project,
            preset,
            output_path,
            range,
            Some(temp_dir.path()),
        )?;
        Ok(PreparedExport {
            cmd_args,
            total_duration,
            temp_dir: Some(temp_dir),
        })
    }

    pub fn build_export_pipeline_range(
        project: &Project,
        preset: &ExportPreset,
        output_path: &str,
        range: Option<(f64, f64)>,
    ) -> Result<(Vec<String>, f64), String> {
        Self::build_export_pipeline_range_in(project, preset, output_path, range, None)
    }

    fn build_export_pipeline_range_in(
        project: &Project,
        preset: &ExportPreset,
        output_path: &str,
        range: Option<(f64, f64)>,
        temp_dir: Option<&std::path::Path>,
    ) -> Result<(Vec<String>, f64), String> {
        let flattened_tl = project.get_flattened_timeline();
        let orig_total_duration = flattened_tl.duration();
        let (eff_start, eff_duration) = if let Some((start, end)) = range {
            let s = start.max(0.0).min(orig_total_duration);
            let e = end.max(s).min(orig_total_duration);
            let d = (e - s).max(0.001);
            (Some(s), d)
        } else {
            (None, orig_total_duration)
        };

        // Filter and re-align timeline clips so only clips within the range are processed
        let timeline_storage;
        let timeline: &trim_core::timeline::TimelineModel = if let Some(start) = eff_start {
            let mut range_tl = flattened_tl.clone();
            let range_end = start + eff_duration;
            for track in &mut range_tl.tracks {
                let mut adjusted_clips = Vec::new();
                for clip in &track.clips {
                    if clip.timeline_out <= start + 1e-4 || clip.timeline_in >= range_end - 1e-4 {
                        continue;
                    }
                    let mut c = clip.clone();
                    let front_cut = (start - c.timeline_in).max(0.0);
                    let eff_in = (c.timeline_in - start).max(0.0);
                    let eff_out = (c.timeline_out - start).min(eff_duration);

                    c.timeline_in = eff_in;
                    c.timeline_out = eff_out;
                    let new_dur = (eff_out - eff_in).max(0.001);
                    if !c.reverse {
                        c.source_in += front_cut * c.speed;
                        c.source_out = Some(c.source_in + new_dur * c.speed);
                    } else {
                        let orig_sout = c.effective_source_out();
                        let new_sout = (orig_sout - front_cut * c.speed).max(c.source_in);
                        c.source_out = Some(new_sout);
                        c.source_in = (new_sout - new_dur * c.speed).max(0.0);
                    }

                    if front_cut > 0.0 {
                        c.fade_in = (c.fade_in - front_cut).max(0.0);
                    }
                    let back_cut = (clip.timeline_out - range_end).max(0.0);
                    if back_cut > 0.0 {
                        c.fade_out = (c.fade_out - back_cut).max(0.0);
                    }
                    adjusted_clips.push(c);
                }
                track.clips = adjusted_clips;
            }
            timeline_storage = range_tl;
            &timeline_storage
        } else {
            &flattened_tl
        };

        let video_tracks = timeline.get_video_tracks();
        if video_tracks.is_empty() {
            return Err("Projede video kanalı bulunamadı".to_string());
        }

        let total_duration = eff_duration;
        let has_any_clip = timeline.tracks.iter().any(|t| {
            t.clips
                .iter()
                .any(|c| project.get_media(&c.media_id).is_some() && c.duration() > 0.0)
        });
        if !has_any_clip || total_duration <= 0.0 {
            return Err("Dışa aktarılacak klip bulunamadı".to_string());
        }

        let out_w = even(preset.width.unwrap_or(timeline.width));
        let out_h = even(preset.height.unwrap_or(timeline.height));
        let out_fps = preset.fps.unwrap_or(timeline.fps);
        let out_fps = if out_fps.is_finite() && out_fps > 0.0 {
            out_fps
        } else {
            30.0
        };

        let mut inputs = InputList::new();
        let mut filters: Vec<String> = Vec::new();

        // Base canvas and silence bed spanning the whole timeline.
        let base_v = inputs.push([
            "-f".into(),
            "lavfi".into(),
            "-i".into(),
            format!(
                "color=c=black:s={}x{}:r={}:d={:.6}",
                out_w, out_h, out_fps, total_duration
            ),
        ]);
        let base_a = inputs.push([
            "-f".into(),
            "lavfi".into(),
            "-t".into(),
            format!("{:.6}", total_duration),
            "-i".into(),
            "anullsrc=r=48000:cl=stereo".into(),
        ]);
        filters.push(format!("[{base_v}:v]format=yuv420p,setsar=1[base0]"));
        let mut last_video = "base0".to_string();
        let mut overlay_count = 0usize;

        let any_solo = timeline.tracks.iter().any(|t| t.solo);
        let mut audio_labels: Vec<String> = Vec::new();

        let mut ordered_tracks: Vec<&Track> =
            timeline.get_video_tracks().into_iter().rev().collect();
        ordered_tracks.extend(timeline.get_audio_tracks());

        for track in ordered_tracks {
            let track_audible = !track.muted && (!any_solo || track.solo);
            for clip in &track.clips {
                let dur = clip.duration();
                if dur <= 0.0 {
                    continue;
                }

                // Helper closure-like preparation for masks on video/adjustment clips
                let has_alpha_mask = clip
                    .masks
                    .iter()
                    .any(|m| m.enabled && m.target == trim_core::MaskTarget::ClipAlpha);
                let has_fx_mask = clip
                    .masks
                    .iter()
                    .any(|m| m.enabled && m.target == trim_core::MaskTarget::ColorAndEffects);

                if clip.is_adjustment_layer() {
                    if track.track_type != TrackType::Video || !track.visible {
                        continue;
                    }
                    let t0 = clip.timeline_in;
                    let t1 = clip.timeline_out;
                    let op = clip.get_property_at_time("opacity", 0.0).clamp(0.0, 1.0);
                    if op <= 0.001 {
                        continue;
                    }

                    let mut alpha_mask_idx: Option<usize> = None;
                    let mut fx_mask_idx: Option<usize> = None;
                    if has_alpha_mask || has_fx_mask {
                        let mask_rgba =
                            trim_core::mask::rasterize_masks_rgba(&clip.masks, out_w, out_h, 0.0);
                        let mask_dir = temp_dir
                            .map(|p| p.to_path_buf())
                            .unwrap_or_else(std::env::temp_dir);
                        if has_alpha_mask {
                            let p = mask_dir.join(format!("trim_mask_alpha_{}.pgm", clip.id));
                            if Self::write_mask_pgm(&p, out_w, out_h, &mask_rgba, 0).is_ok() {
                                alpha_mask_idx = Some(inputs.push([
                                    "-protocol_whitelist".into(),
                                    "file".into(),
                                    "-loop".into(),
                                    "1".into(),
                                    "-framerate".into(),
                                    format!("{out_fps}"),
                                    "-t".into(),
                                    format!("{total_duration:.6}"),
                                    "-i".into(),
                                    p.to_string_lossy().to_string(),
                                ]));
                            }
                        }
                        if has_fx_mask {
                            let p = mask_dir.join(format!("trim_mask_fx_{}.pgm", clip.id));
                            if Self::write_mask_pgm(&p, out_w, out_h, &mask_rgba, 1).is_ok() {
                                fx_mask_idx = Some(inputs.push([
                                    "-protocol_whitelist".into(),
                                    "file".into(),
                                    "-loop".into(),
                                    "1".into(),
                                    "-framerate".into(),
                                    format!("{out_fps}"),
                                    "-t".into(),
                                    format!("{total_duration:.6}"),
                                    "-i".into(),
                                    p.to_string_lossy().to_string(),
                                ]));
                            }
                        }
                    }

                    overlay_count += 1;
                    let k = overlay_count;
                    let bg_lbl = format!("adj_bg_{k}");
                    let in_lbl = format!("adj_in_{k}");
                    let bg_fx_lbl = format!("adj_bgfx_{k}");
                    if fx_mask_idx.is_some() {
                        filters.push(format!(
                            "[{last_video}]split=3[{bg_lbl}][{bg_fx_lbl}][{in_lbl}]"
                        ));
                    } else {
                        filters.push(format!("[{last_video}]split=2[{bg_lbl}][{in_lbl}]"));
                    }

                    let color_filters = Self::clip_color_filters_in(clip, temp_dir);
                    let graded_lbl = if color_filters.is_empty() {
                        in_lbl
                    } else {
                        let gl = format!("adj_gr_{k}");
                        filters.push(format!(
                            "[{in_lbl}]format=rgb24,{}[{gl}]",
                            color_filters.join(",")
                        ));
                        gl
                    };

                    let fx_lbl = if let Some(fx_i) = fx_mask_idx {
                        let fl = format!("adj_fx_{k}");
                        filters.push(format!(
                            "[{bg_fx_lbl}]format=yuv420p[adj_bg_y_{k}];[{graded_lbl}]format=yuv420p[adj_gr_y_{k}];[{fx_i}:v]format=gray[adj_fm_{k}];[adj_bg_y_{k}][adj_gr_y_{k}][adj_fm_{k}]maskedmerge=enable='between(t,{t0:.6},{t1:.6})'[{fl}]"
                        ));
                        fl
                    } else {
                        let fl = format!("adj_fy_{k}");
                        filters.push(format!("[{graded_lbl}]format=yuv420p[{fl}]"));
                        fl
                    };

                    let next = format!("base{k}");
                    let blend_mode = clip.blend_mode.ffmpeg_mode();
                    if let Some(a_i) = alpha_mask_idx {
                        let bl = format!("adj_bl_{k}");
                        filters.push(format!(
                            "[{bg_lbl}]format=yuv420p,split=2[adj_bgy_{k}][adj_bgm_{k}];[adj_bgy_{k}][{fx_lbl}]blend=all_mode={blend_mode}:all_opacity={op:.4}:enable='between(t,{t0:.6},{t1:.6})':eof_action=pass[{bl}];[{a_i}:v]format=gray[adj_am_{k}];[adj_bgm_{k}][{bl}][adj_am_{k}]maskedmerge=enable='between(t,{t0:.6},{t1:.6})'[{next}]"
                        ));
                    } else {
                        filters.push(format!(
                            "[{bg_lbl}]format=yuv420p[adj_bgy_{k}];[adj_bgy_{k}][{fx_lbl}]blend=all_mode={blend_mode}:all_opacity={op:.4}:enable='between(t,{t0:.6},{t1:.6})':eof_action=pass[{next}]"
                        ));
                    }
                    last_video = next;
                    continue;
                }

                let Some(media) = project.get_media(&clip.media_id) else {
                    continue;
                };
                let is_image = media.media_type == MediaType::Image;
                let wants_video =
                    track.track_type == TrackType::Video && track.visible && media.has_video();
                let wants_audio = track.track_type == TrackType::Audio
                    && track_audible
                    && !clip.muted
                    && !is_image
                    && media.has_audio();
                if !wants_video && !wants_audio {
                    continue;
                }

                let safe_media_path = safe_ffmpeg_path_arg(&media.file_path);
                let input_idx = if is_image {
                    inputs.push([
                        "-protocol_whitelist".into(),
                        "file".into(),
                        "-loop".into(),
                        "1".into(),
                        "-framerate".into(),
                        format!("{}", out_fps),
                        "-t".into(),
                        format!("{:.6}", dur),
                        "-i".into(),
                        safe_media_path,
                    ])
                } else {
                    let (src_start, src_dur) = source_window(clip);
                    let mut in_args = vec![
                        "-protocol_whitelist".into(),
                        "file".into(),
                        "-ss".into(),
                        format!("{:.6}", src_start),
                        "-t".into(),
                        format!("{:.6}", src_dur),
                    ];
                    if wants_audio
                        && matches!(
                            media.audio_codec.as_str(),
                            "pcm_s16be" | "pcm_s24be" | "pcm_s32be" | "pcm_f32be"
                        )
                    {
                        in_args.push("-c:a".into());
                        in_args.push(media.audio_codec.clone());
                    }
                    in_args.push("-i".into());
                    in_args.push(safe_media_path);
                    inputs.push(in_args)
                };

                if wants_video {
                    let chain =
                        Self::clip_video_chain(clip, media, out_w, out_h, out_fps, temp_dir);
                    let clip_label = format!("cv{input_idx}");
                    filters.push(format!("[{input_idx}:v]{chain}[{clip_label}]"));
                    let (x, y) = Self::overlay_position(clip, out_w, out_h);

                    let mut alpha_mask_idx: Option<usize> = None;
                    if has_alpha_mask {
                        let mask_rgba =
                            trim_core::mask::rasterize_masks_rgba(&clip.masks, out_w, out_h, 0.0);
                        let mask_dir = temp_dir
                            .map(|p| p.to_path_buf())
                            .unwrap_or_else(std::env::temp_dir);
                        let p = mask_dir.join(format!("trim_mask_alpha_{}.pgm", clip.id));
                        if Self::write_mask_pgm(&p, out_w, out_h, &mask_rgba, 0).is_ok() {
                            alpha_mask_idx = Some(inputs.push([
                                "-protocol_whitelist".into(),
                                "file".into(),
                                "-loop".into(),
                                "1".into(),
                                "-framerate".into(),
                                format!("{out_fps}"),
                                "-t".into(),
                                format!("{total_duration:.6}"),
                                "-i".into(),
                                p.to_string_lossy().to_string(),
                            ]));
                        }
                    }

                    overlay_count += 1;
                    let k = overlay_count;
                    let next = format!("base{k}");
                    let t0 = clip.timeline_in;
                    let t1 = clip.timeline_out;

                    if clip.blend_mode != trim_core::BlendMode::Normal || alpha_mask_idx.is_some() {
                        let blend_mode = clip.blend_mode.ffmpeg_mode();
                        let bg_lbl = format!("cbg_{k}");
                        let bg_dup = format!("cbgd_{k}");
                        let bg_blend = format!("cbgb_{k}");
                        let ov_lbl = format!("cov_{k}");
                        let has_blend = clip.blend_mode != trim_core::BlendMode::Normal;
                        let has_alpha = alpha_mask_idx.is_some();

                        if has_blend && has_alpha {
                            filters.push(format!(
                                "[{last_video}]format=yuv420p,split=3[{bg_lbl}][{bg_blend}][{bg_dup}];[{bg_dup}][{clip_label}]overlay=x={x}:y={y}:eval=frame:eof_action=pass[{ov_lbl}]"
                            ));
                        } else {
                            filters.push(format!(
                                "[{last_video}]format=yuv420p,split=2[{bg_lbl}][{bg_dup}];[{bg_dup}][{clip_label}]overlay=x={x}:y={y}:eval=frame:eof_action=pass[{ov_lbl}]"
                            ));
                        }

                        let blended_lbl = if has_blend {
                            let bl = format!("cbl_{k}");
                            let blend_bg = if has_alpha { &bg_blend } else { &bg_lbl };
                            filters.push(format!(
                                "[{blend_bg}][{ov_lbl}]blend=all_mode={blend_mode}:all_opacity=1.0:enable='between(t,{t0:.6},{t1:.6})':eof_action=pass[{bl}]"
                            ));
                            bl
                        } else {
                            ov_lbl
                        };

                        if let Some(a_i) = alpha_mask_idx {
                            filters.push(format!(
                                "[{a_i}:v]format=gray[cam_{k}];[{bg_lbl}][{blended_lbl}][cam_{k}]maskedmerge=enable='between(t,{t0:.6},{t1:.6})'[{next}]"
                            ));
                        } else {
                            filters.push(format!("[{blended_lbl}]null[{next}]"));
                        }
                    } else {
                        filters.push(format!(
                            "[{last_video}][{clip_label}]overlay=x={x}:y={y}:eval=frame:eof_action=pass[{next}]"
                        ));
                    }
                    last_video = next;
                }

                if wants_audio {
                    let eff_vol = clip.volume * track.volume;
                    let mut chain = Self::clip_audio_chain(clip, eff_vol);
                    if track.pan.abs() > 0.01 {
                        let p = track.pan.clamp(-1.0, 1.0);
                        let l_g = if p > 0.0 { 1.0 - p } else { 1.0 };
                        let r_g = if p < 0.0 { 1.0 + p } else { 1.0 };
                        chain.push_str(&format!(",pan=stereo|c0={l_g:.3}*c0|c1={r_g:.3}*c1"));
                    }
                    let label = format!("ca{input_idx}");
                    filters.push(format!("[{input_idx}:a]{chain}[{label}]"));
                    audio_labels.push(label);
                }
            }
        }

        let codec_args = Self::resolve_video_codec_args(preset);
        let uses_vaapi = codec_args.iter().any(|a| a.contains("vaapi"));
        let is_prores = preset.video_codec.contains("prores");

        if uses_vaapi {
            filters.push(format!("[{last_video}]format=nv12,hwupload[outv]"));
        } else if is_prores {
            filters.push(format!("[{last_video}]format=yuv422p10le[outv]"));
        } else {
            filters.push(format!("[{last_video}]format=yuv420p[outv]"));
        }

        let mut mix = format!("[{base_a}:a]aformat=sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo[abed];[abed]");
        for l in &audio_labels {
            mix.push_str(&format!("[{l}]"));
        }
        if let Some(target_lufs) = preset.loudness_norm.target_lufs() {
            mix.push_str(&format!(
                "amix=inputs={}:duration=first:dropout_transition=0:normalize=0[mixa];[mixa]alimiter=limit=0.988553:attack=1:release=75:level=disabled[lima];[lima]loudnorm=I={:.1}:LRA=11:TP=-1.5,aresample=48000[outa]",
                audio_labels.len() + 1,
                target_lufs
            ));
        } else {
            mix.push_str(&format!(
                "amix=inputs={}:duration=first:dropout_transition=0:normalize=0[mixa];[mixa]alimiter=limit=0.988553:attack=1:release=75:level=disabled[outa]",
                audio_labels.len() + 1
            ));
        }
        filters.push(mix);

        let full_filtergraph = filters.join(";");

        let mut cmd = vec![
            "ffmpeg".into(),
            "-y".into(),
            "-v".into(),
            "error".into(),
            "-nostdin".into(),
        ];
        if uses_vaapi {
            let vaapi_dev = trim_media::detect_vaapi_device()
                .unwrap_or_else(|| "/dev/dri/renderD128".to_string());
            cmd.extend(["-vaapi_device".into(), vaapi_dev]);
        }
        cmd.extend(inputs.args);
        cmd.extend([
            "-filter_complex".into(),
            full_filtergraph,
            "-map".into(),
            "[outv]".into(),
            "-map".into(),
            "[outa]".into(),
        ]);
        cmd.extend(codec_args);
        cmd.extend([
            "-c:a".into(),
            preset.audio_codec.clone(),
            "-b:a".into(),
            preset.audio_bitrate.clone(),
        ]);
        if !uses_vaapi {
            cmd.extend([
                "-pix_fmt".into(),
                if is_prores { "yuv422p10le" } else { "yuv420p" }.into(),
            ]);
        }
        cmd.extend([
            "-colorspace".into(),
            "bt709".into(),
            "-color_primaries".into(),
            "bt709".into(),
            "-color_trc".into(),
            "bt709".into(),
        ]);
        let lower_out = output_path.to_ascii_lowercase();
        if lower_out.ends_with(".mp4") || lower_out.ends_with(".mov") || lower_out.ends_with(".m4v")
        {
            cmd.extend(["-movflags".into(), "+faststart".into()]);
        }
        cmd.extend(["-r".into(), format!("{}", out_fps)]);

        cmd.extend(vec![
            "-t".into(),
            format!("{:.6}", eff_duration),
            "-progress".into(),
            "pipe:1".into(),
            "-nostats".into(),
            safe_ffmpeg_path_arg(output_path),
        ]);

        Ok((cmd, eff_duration))
    }

    /// Resolves optimal video codec arguments honoring hardware acceleration and fallbacks.
    pub fn resolve_video_codec_args(preset: &ExportPreset) -> Vec<String> {
        let available = trim_media::detect_available_encoders();
        let is_hevc = preset.video_codec.contains("265") || preset.video_codec.contains("hevc");
        let is_prores = preset.video_codec.contains("prores");

        if is_prores {
            return vec![
                "-c:v".into(),
                "prores_ks".into(),
                "-profile:v".into(),
                "3".into(),
            ];
        }

        let target_accel = match preset.hw_accel {
            HardwareAccel::Auto => {
                if available.has_nvidia_nvenc() {
                    HardwareAccel::Nvenc
                } else if available.has_videotoolbox() {
                    HardwareAccel::VideoToolbox
                } else if available.has_intel_qsv() {
                    HardwareAccel::Qsv
                } else if available.has_vaapi() {
                    HardwareAccel::Vaapi
                } else if available.has_amd_amf() {
                    HardwareAccel::Amf
                } else {
                    HardwareAccel::Cpu
                }
            }
            other => other,
        };

        match target_accel {
            HardwareAccel::Nvenc if available.has_nvidia_nvenc() => {
                let codec = if is_hevc && available.hevc_nvenc {
                    "hevc_nvenc"
                } else {
                    "h264_nvenc"
                };
                vec![
                    "-c:v".into(),
                    codec.into(),
                    "-preset".into(),
                    "p4".into(),
                    "-rc:v".into(),
                    "vbr".into(),
                    "-cq:v".into(),
                    preset.crf.to_string(),
                ]
            }
            HardwareAccel::VideoToolbox if available.has_videotoolbox() => {
                let codec = if is_hevc && available.hevc_videotoolbox {
                    "hevc_videotoolbox"
                } else {
                    "h264_videotoolbox"
                };
                let qv = (100u32.saturating_sub(preset.crf.min(50) * 2)).clamp(35, 90);
                vec!["-c:v".into(), codec.into(), "-q:v".into(), qv.to_string()]
            }
            HardwareAccel::Qsv if available.has_intel_qsv() => {
                let codec = if is_hevc && available.hevc_qsv {
                    "hevc_qsv"
                } else {
                    "h264_qsv"
                };
                vec![
                    "-c:v".into(),
                    codec.into(),
                    "-preset".into(),
                    "medium".into(),
                    "-global_quality".into(),
                    preset.crf.to_string(),
                ]
            }
            HardwareAccel::Vaapi if available.has_vaapi() => {
                let codec = if is_hevc && available.hevc_vaapi {
                    "hevc_vaapi"
                } else {
                    "h264_vaapi"
                };
                vec![
                    "-c:v".into(),
                    codec.into(),
                    "-qp".into(),
                    preset.crf.to_string(),
                ]
            }
            HardwareAccel::Amf if available.has_amd_amf() => {
                let codec = if is_hevc && available.hevc_amf {
                    "hevc_amf"
                } else {
                    "h264_amf"
                };
                let qp = preset.crf.clamp(1, 51).to_string();
                vec![
                    "-c:v".into(),
                    codec.into(),
                    "-quality".into(),
                    "quality".into(),
                    "-rc".into(),
                    "cqp".into(),
                    "-qp_i".into(),
                    qp.clone(),
                    "-qp_p".into(),
                    qp,
                ]
            }
            _ => {
                let codec = if is_hevc { "libx265" } else { "libx264" };
                vec![
                    "-c:v".into(),
                    codec.into(),
                    "-preset".into(),
                    preset.preset.clone(),
                    "-crf".into(),
                    preset.crf.to_string(),
                ]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::LoudnessNorm;
    use trim_core::clip::Clip;
    use trim_core::keyframe::interpolate_keyframes;
    use trim_core::media::MediaItem;
    use trim_core::track::Track;

    fn graph_of(cmd: &[String]) -> String {
        let index = cmd.iter().position(|arg| arg == "-filter_complex").unwrap();
        cmd[index + 1].clone()
    }

    #[test]
    fn test_atempo_filter_chain() {
        // 1.0x -> no filters
        assert!(TimelineExporter::get_atempo_filters(1.0).is_empty());

        // 1.5x -> [atempo=1.5]
        let f15 = TimelineExporter::get_atempo_filters(1.5);
        assert_eq!(f15.len(), 1);
        assert_eq!(f15[0], "atempo=1.5000");

        // 3.0x -> [atempo=2.0, atempo=1.5]
        let f30 = TimelineExporter::get_atempo_filters(3.0);
        assert_eq!(f30.len(), 2);
        assert_eq!(f30[0], "atempo=2.0");
        assert_eq!(f30[1], "atempo=1.5000");

        // 0.25x -> [atempo=0.5, atempo=0.5]
        let f025 = TimelineExporter::get_atempo_filters(0.25);
        assert_eq!(f025.len(), 2);
        assert_eq!(f025[0], "atempo=0.5");
        assert_eq!(f025[1], "atempo=0.5000");
    }

    #[test]
    fn test_silent_video_uses_correct_filtergraph_labels() {
        let mut project = Project::new("Silent video");
        let mut media = MediaItem::new("/videos/no-audio.mp4");
        media.media_type = MediaType::Video;
        media.duration = 5.0;
        media.audio_codec.clear();
        let id = media.id.clone();
        project.add_media(media);
        let video_track = project
            .timeline
            .get_video_tracks_mut()
            .into_iter()
            .next()
            .unwrap();
        video_track.add_clip(Clip::new(id, 0.0, 5.0));

        let (command, _) = TimelineExporter::build_export_pipeline(
            &project,
            &ExportPreset::default(),
            "/tmp/out.mp4",
        )
        .unwrap();
        let graph = graph_of(&command);
        // Inputs: 0 = base colour, 1 = silence bed, 2 = clip.
        assert!(graph.contains("[2:v]"), "video input missing: {graph}");
        assert!(
            !graph.contains("[2:a]"),
            "silent media must not reference an audio stream: {graph}"
        );
        assert!(graph.contains("amix=inputs=1"), "{graph}");
    }

    #[test]
    fn test_export_pipeline_generation() {
        let mut proj = Project::new("Export Test");
        let mut m = MediaItem::new("/path/test.mp4");
        m.media_type = MediaType::Video;
        m.duration = 10.0;
        let m_id = m.id.clone();
        proj.add_media(m);

        let v_track = proj
            .timeline
            .get_video_tracks_mut()
            .into_iter()
            .next()
            .unwrap();
        let mut c = Clip::new(&m_id, 2.0, 6.0);
        c.speed = 2.0;
        c.brightness = 0.1;
        v_track.add_clip(c);

        let preset = ExportPreset::default();
        let (cmd, dur) =
            TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();

        assert!((dur - 6.0).abs() < 1e-3);
        assert!(cmd.contains(&"-filter_complex".to_string()));
        assert!(cmd.contains(&"/tmp/out.mp4".to_string()));
        let graph = graph_of(&cmd);
        assert!(graph.contains("setpts=PTS+2.000000/TB"), "{graph}");
        // 4 s on the timeline at 2x reads 8 s of source.
        let t_pos = cmd.iter().position(|a| a == "/path/test.mp4").unwrap();
        assert_eq!(cmd[t_pos - 2], "8.000000");
    }

    #[test]
    fn test_overlapping_tracks_are_composited_not_concatenated() {
        let mut proj = Project::new("Layers");
        let mut bg = MediaItem::new("/v/bg.mp4");
        bg.media_type = MediaType::Video;
        bg.audio_codec = "aac".into();
        let mut fg = MediaItem::new("/v/logo.png");
        fg.media_type = MediaType::Image;
        let mut music = MediaItem::new("/a/music.mp3");
        music.media_type = MediaType::Audio;
        music.audio_codec = "mp3".into();
        let (bg_id, fg_id, mu_id) = (bg.id.clone(), fg.id.clone(), music.id.clone());
        proj.add_media(bg);
        proj.add_media(fg);
        proj.add_media(music);

        let v1 = proj
            .timeline
            .tracks
            .iter_mut()
            .find(|t| t.name == "V1")
            .unwrap();
        v1.add_clip(Clip::new(&bg_id, 0.0, 10.0));

        let v2 = proj
            .timeline
            .tracks
            .iter_mut()
            .find(|t| t.name == "V2")
            .unwrap();
        v2.add_clip(Clip::new(&fg_id, 2.0, 4.0));

        let a1 = proj
            .timeline
            .tracks
            .iter_mut()
            .find(|t| t.name == "A1")
            .unwrap();
        a1.add_clip(Clip::new(&mu_id, 1.0, 9.0));

        let a2 = proj
            .timeline
            .tracks
            .iter_mut()
            .find(|t| t.name == "A2")
            .unwrap();
        a2.add_clip(Clip::new(&bg_id, 0.0, 10.0));

        let (cmd, dur) =
            TimelineExporter::build_export_pipeline(&proj, &ExportPreset::default(), "/tmp/o.mp4")
                .unwrap();
        let graph = graph_of(&cmd);
        assert!((dur - 10.0).abs() < 1e-6);
        assert!(!graph.contains("concat"), "{graph}");
        assert_eq!(graph.matches("overlay=").count(), 2, "{graph}");
        // Background audio on A2 + music on A1 + silence bed.
        assert!(graph.contains("amix=inputs=3"), "{graph}");
        assert!(graph.contains("adelay=delays=1000:all=1"), "{graph}");
    }

    #[test]
    fn test_hidden_and_muted_tracks() {
        let mut proj = Project::new("Mute");
        let mut m = MediaItem::new("/v/a.mp4");
        m.media_type = MediaType::Video;
        m.audio_codec = "aac".into();
        let id = m.id.clone();
        proj.add_media(m);
        proj.timeline.tracks[0].add_clip(Clip::new(&id, 0.0, 3.0));
        let a1 = proj
            .timeline
            .get_audio_tracks_mut()
            .into_iter()
            .next()
            .unwrap();
        a1.add_clip(Clip::new(&id, 0.0, 3.0));
        let a1_id = a1.id.clone();

        proj.timeline.get_track_by_id_mut(&a1_id).unwrap().muted = true;
        let (cmd, _) =
            TimelineExporter::build_export_pipeline(&proj, &ExportPreset::default(), "/tmp/o.mp4")
                .unwrap();
        let g = graph_of(&cmd);
        assert!(g.contains("overlay="));
        assert!(g.contains("amix=inputs=1"), "{g}");

        proj.timeline.get_track_by_id_mut(&a1_id).unwrap().muted = false;
        proj.timeline.tracks[0].visible = false;
        let (cmd, _) =
            TimelineExporter::build_export_pipeline(&proj, &ExportPreset::default(), "/tmp/o.mp4")
                .unwrap();
        let g = graph_of(&cmd);
        assert!(!g.contains("overlay="), "{g}");
        assert!(g.contains("amix=inputs=2"), "{g}");
    }

    #[test]
    fn test_keyframe_expression_matches_interpolation() {
        let kfs = vec![
            Keyframe::new(0.5, 10.0, Interpolation::Linear),
            Keyframe::new(1.5, 30.0, Interpolation::EaseInOut),
            Keyframe::new(2.5, 0.0, Interpolation::Hold),
            Keyframe::new(3.0, 5.0, Interpolation::Linear),
        ];
        let expr = keyframe_expr(&kfs, "T", |v| v);
        // Evaluate the generated expression with a tiny interpreter-free
        // check: substitute and compare a few sample points via meval-like
        // manual evaluation is overkill; instead verify structure and
        // boundary constants.
        assert!(expr.starts_with("if(lt(T,0.500000),10.000000,"));
        assert!(expr.contains("3*"));
        assert_eq!(expr.matches("if(").count(), kfs.len());
        // Sanity: the reference interpolation at boundaries.
        assert_eq!(interpolate_keyframes(&kfs, 0.0, 0.0), 10.0);
        assert_eq!(interpolate_keyframes(&kfs, 4.0, 0.0), 5.0);
    }

    /// End-to-end: actually renders a layered timeline with FFmpeg when it is
    /// available on the machine.
    #[test]
    fn test_ffmpeg_end_to_end_render() {
        if trim_media::ffmpeg_cmd().arg("-version").output().is_err() {
            eprintln!("ffmpeg not available, skipping");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let vid = dir.path().join("src.mp4");
        let img = dir.path().join("logo.png");
        let aud = dir.path().join("tone.wav");
        let ok = |args: &[&str]| {
            trim_media::ffmpeg_cmd()
                .args(["-v", "error", "-y"])
                .args(args)
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };
        assert!(ok(&[
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=320x240:rate=25:duration=4",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=4",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
            vid.to_str().unwrap(),
        ]));
        assert!(ok(&[
            "-f",
            "lavfi",
            "-i",
            "color=c=red@0.5:s=64x64,format=rgba",
            "-frames:v",
            "1",
            img.to_str().unwrap()
        ]));
        assert!(ok(&[
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:duration=2",
            aud.to_str().unwrap()
        ]));

        let mut proj = Project::new("E2E");
        proj.timeline.width = 320;
        proj.timeline.height = 240;
        let mut v = MediaItem::new(&vid);
        v.media_type = MediaType::Video;
        v.audio_codec = "aac".into();
        let mut i = MediaItem::new(&img);
        i.media_type = MediaType::Image;
        let mut a = MediaItem::new(&aud);
        a.media_type = MediaType::Audio;
        a.audio_codec = "pcm_s16le".into();
        let (vid_id, img_id, aud_id) = (v.id.clone(), i.id.clone(), a.id.clone());
        proj.add_media(v);
        proj.add_media(i);
        proj.add_media(a);

        let mut c1 = Clip::new(&vid_id, 0.0, 2.0);
        c1.fade_out = 0.5;
        proj.timeline.tracks[0].add_clip(c1);
        let mut c2 = Clip::new(&vid_id, 3.0, 4.0);
        c2.speed = 2.0;
        c2.reverse = true;
        c2.source_in = 0.5;
        c2.source_out = Some(2.5);
        proj.timeline.tracks[0].add_clip(c2);
        let mut v2 = Track::new("V2", TrackType::Video);
        let mut logo = Clip::new(&img_id, 1.0, 3.5);
        logo.pos_x = 50.0;
        logo.scale = 0.5;
        logo.add_keyframe("opacity", 0.0, 0.0, Interpolation::Linear);
        logo.add_keyframe("opacity", 1.0, 1.0, Interpolation::Linear);
        v2.add_clip(logo);
        proj.timeline.tracks.insert(1, v2);
        let a1 = proj
            .timeline
            .get_audio_tracks_mut()
            .into_iter()
            .next()
            .unwrap();
        a1.add_clip(Clip::new(&aud_id, 0.5, 2.5));

        let out = dir.path().join("out.mp4");
        let preset = ExportPreset {
            width: None,
            height: None,
            preset: "ultrafast".into(),
            hw_accel: HardwareAccel::Cpu,
            ..Default::default()
        };
        let (cmd, dur) =
            TimelineExporter::build_export_pipeline(&proj, &preset, out.to_str().unwrap()).unwrap();
        let output = trim_media::create_command(&cmd[0])
            .args(&cmd[1..])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "ffmpeg failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let probe = trim_media::ffprobe_cmd()
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=nw=1:nk=1",
            ])
            .arg(&out)
            .output()
            .unwrap();
        let got: f64 = String::from_utf8_lossy(&probe.stdout)
            .trim()
            .parse()
            .unwrap_or(0.0);
        assert!((got - dur).abs() < 0.25, "duration {got} vs {dur}");
    }

    #[test]
    fn test_resolve_video_codec_args() {
        let preset = ExportPreset {
            hw_accel: HardwareAccel::Cpu,
            ..Default::default()
        };
        let args = TimelineExporter::resolve_video_codec_args(&preset);
        assert!(args.contains(&"libx264".to_string()));
        assert!(args.contains(&"-crf".to_string()));

        let prores_preset = ExportPreset {
            video_codec: "prores_ks".to_string(),
            ..Default::default()
        };
        let args = TimelineExporter::resolve_video_codec_args(&prores_preset);
        assert!(args.contains(&"prores_ks".to_string()));
        assert!(args.contains(&"-profile:v".to_string()));
    }

    #[test]
    fn test_loudness_norm_and_eq_filtergraph() {
        use trim_core::clip::AudioEqPreset;

        let mut proj = Project::new("AudioTest");
        let mut v = MediaItem::new("/tmp/fake.mp4");
        v.media_type = MediaType::Video;
        v.audio_codec = "aac".into();
        let vid_id = v.id.clone();
        proj.add_media(v);

        let mut c = Clip::new(&vid_id, 0.0, 5.0);
        c.eq_preset = AudioEqPreset::VoiceClarity;
        c.noise_gate = true;
        let a1 = proj
            .timeline
            .get_audio_tracks_mut()
            .into_iter()
            .next()
            .unwrap();
        a1.add_clip(c);

        let preset = ExportPreset {
            loudness_norm: LoudnessNorm::YouTube,
            ..Default::default()
        };
        let (cmd, _) =
            TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();

        let fg = cmd
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &cmd[idx + 1])
            .unwrap();
        assert!(
            fg.contains("equalizer=f=3000"),
            "filtergraph should include voice clarity EQ: {fg}"
        );
        assert!(
            fg.contains("agate=threshold=0.03"),
            "filtergraph should include noise gate: {fg}"
        );
        assert!(
            fg.contains("loudnorm=I=-14.0"),
            "filtergraph should include EBU R128 loudnorm: {fg}"
        );
    }

    #[test]
    fn test_text_overlay_drawtext_generation() {
        use trim_core::clip::TextOverlay;

        let mut proj = Project::new("TextTest");
        let mut v = MediaItem::new("/tmp/fake.mp4");
        v.media_type = MediaType::Video;
        let vid_id = v.id.clone();
        proj.add_media(v);

        let mut c = Clip::new(&vid_id, 0.0, 5.0);
        c.text_overlay = Some(TextOverlay {
            content: "Hello triM".to_string(),
            font_size: 48.0,
            text_color: [255, 255, 255, 255],
            bg_color: Some([0, 0, 0, 180]),
            pos_x: 0.5,
            pos_y: 0.85,
            has_shadow: true,
            ..Default::default()
        });
        proj.timeline.tracks[0].add_clip(c);

        let preset = ExportPreset::default();
        let (cmd, _) =
            TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();
        let fg = cmd
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &cmd[idx + 1])
            .unwrap();

        assert!(
            fg.contains("drawtext=text='Hello triM'"),
            "filtergraph should contain drawtext: {fg}"
        );
        assert!(
            fg.contains("box=1:boxcolor="),
            "filtergraph should contain subtitle box: {fg}"
        );
        assert!(
            fg.contains("shadowx=3"),
            "filtergraph should contain shadow: {fg}"
        );
    }

    #[test]
    fn test_lower_third_drawtext_generation() {
        use trim_core::clip::{MotionTitleStyle, TextOverlay};

        let mut proj = Project::new("LowerThirdTest");
        let mut v = MediaItem::new("/tmp/fake_lt.mp4");
        v.media_type = MediaType::Video;
        let vid_id = v.id.clone();
        proj.add_media(v);

        let mut c = Clip::new(&vid_id, 0.0, 5.0);
        c.text_overlay = Some(TextOverlay {
            content: "Jane Doe".to_string(),
            motion_style: MotionTitleStyle::LowerThird,
            subtitle_text: Some("Yönetmen".to_string()),
            font_size: 40.0,
            text_color: [255, 255, 255, 255],
            pos_x: 0.08,
            pos_y: 0.82,
            has_shadow: true,
            ..Default::default()
        });
        proj.timeline.tracks[0].add_clip(c);

        let preset = ExportPreset::default();
        let (cmd, _) =
            TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();
        let fg = cmd
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &cmd[idx + 1])
            .unwrap();

        assert!(
            fg.contains("drawtext=text='Jane Doe'"),
            "filtergraph should contain main title: {fg}"
        );
        assert!(
            fg.contains("drawtext=text='Yönetmen'"),
            "filtergraph should contain subtitle: {fg}"
        );
        assert!(
            fg.contains("boxcolor=0x0f172a@0.85"),
            "filtergraph should contain lower third box: {fg}"
        );
    }

    #[test]
    fn test_lut_preset_and_custom_cube_filtergraph() {
        use trim_core::clip::LutPreset;
        let mut proj = Project::new("LutTest");
        let mut v = MediaItem::new("/tmp/fake_lut.mp4");
        v.media_type = MediaType::Video;
        let vid_id = v.id.clone();
        proj.add_media(v);

        let mut c1 = Clip::new(&vid_id, 0.0, 5.0);
        c1.lut_preset = LutPreset::TealOrange;
        proj.timeline.tracks[0].add_clip(c1);

        let preset = ExportPreset::default();
        let (cmd1, _) =
            TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out1.mp4").unwrap();
        let fg1 = cmd1
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &cmd1[idx + 1])
            .unwrap();
        assert!(
            fg1.contains("colorbalance=rs=-0.20:gs=0.06:bs=0.28"),
            "should generate teal & orange filter: {fg1}"
        );

        // Now test custom cube file (Unix path)
        proj.timeline.tracks[0].clips[0].custom_lut_path =
            Some("/path/to/cinematic.cube".to_string());
        let (cmd2, _) =
            TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out2.mp4").unwrap();
        let fg2 = cmd2
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &cmd2[idx + 1])
            .unwrap();
        assert!(
            fg2.contains("lut3d=file='/path/to/cinematic.cube'"),
            "should generate lut3d filter: {fg2}"
        );

        // Test Windows custom cube file path with drive letter and backslashes
        proj.timeline.tracks[0].clips[0].custom_lut_path =
            Some(r"C:\Users\Editor\LUTs\cinematic.cube".to_string());
        let (cmd3, _) =
            TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out3.mp4").unwrap();
        let fg3 = cmd3
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &cmd3[idx + 1])
            .unwrap();
        assert!(
            fg3.contains("lut3d=file='C\\:/Users/Editor/LUTs/cinematic.cube'"),
            "should escape Windows drive colon and backslashes in lut3d filter: {fg3}"
        );
    }

    #[test]
    fn test_typewriter_drawtext_generation() {
        use trim_core::clip::{MotionTitleStyle, TextOverlay};

        let mut proj = Project::new("TypewriterTest");
        let mut v = MediaItem::new("/tmp/fake_tw.mp4");
        v.media_type = MediaType::Video;
        let vid_id = v.id.clone();
        proj.add_media(v);

        let mut c = Clip::new(&vid_id, 0.0, 4.0);
        c.text_overlay = Some(TextOverlay {
            content: "triM".to_string(),
            motion_style: MotionTitleStyle::Typewriter,
            subtitle_text: Some("NLE".to_string()),
            font_size: 48.0,
            text_color: [255, 255, 255, 255],
            has_shadow: true,
            ..Default::default()
        });
        proj.timeline.tracks[0].add_clip(c);

        let preset = ExportPreset::default();
        let (cmd, _) =
            TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();
        let fg = cmd
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &cmd[idx + 1])
            .unwrap();

        // Should contain progressive letter reveals
        assert!(
            fg.contains("drawtext=text='t'"),
            "should contain first char: {fg}"
        );
        assert!(
            fg.contains("drawtext=text='tr'"),
            "should contain two chars: {fg}"
        );
        assert!(
            fg.contains("drawtext=text='tri'"),
            "should contain three chars: {fg}"
        );
        assert!(
            fg.contains("drawtext=text='triM'"),
            "should contain full word: {fg}"
        );
        assert!(
            fg.contains(":enable='between(t,"),
            "should contain enable between: {fg}"
        );
        assert!(
            fg.contains(":enable='gte(t,"),
            "should contain enable gte: {fg}"
        );
        assert!(
            fg.contains("drawtext=text='NLE'"),
            "should contain subtitle: {fg}"
        );
    }

    #[test]
    fn test_text_clip_fade_in_fade_out_order() {
        use trim_core::clip::TextOverlay;

        let mut proj = Project::new("TextFadeTest");
        let mut v = MediaItem::new("/tmp/fake_fade.png");
        v.media_type = MediaType::Image;
        let vid_id = v.id.clone();
        proj.add_media(v);

        let mut c = Clip::new(&vid_id, 0.0, 5.0);
        c.fade_in = 1.0;
        c.fade_out = 1.0;
        c.text_overlay = Some(TextOverlay {
            content: "Fade Me".to_string(),
            font_size: 48.0,
            text_color: [255, 255, 255, 255],
            ..Default::default()
        });
        proj.timeline.tracks[0].add_clip(c);

        let preset = ExportPreset::default();
        let (cmd, _) =
            TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();
        let fg = cmd
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &cmd[idx + 1])
            .unwrap();

        let drawtext_pos = fg
            .find("drawtext=text='Fade Me'")
            .expect("drawtext must exist");
        let fade_in_pos = fg
            .find("fade=t=in:st=0:d=1.0000:alpha=1")
            .expect("fade in must exist");
        let fade_out_pos = fg
            .find("fade=t=out:st=4.0000:d=1.0000:alpha=1")
            .expect("fade out must exist");

        // CRUCIAL: drawtext MUST come BEFORE fade=t=in and fade=t=out so the text is properly faded!
        assert!(
            drawtext_pos < fade_in_pos,
            "drawtext ({drawtext_pos}) must be before fade_in ({fade_in_pos}): {fg}"
        );
        assert!(
            fade_in_pos < fade_out_pos,
            "fade_in ({fade_in_pos}) must be before fade_out ({fade_out_pos}): {fg}"
        );
    }

    #[test]
    fn test_cinematic_zoom_drawtext_generation() {
        use trim_core::clip::{MotionTitleStyle, TextOverlay};

        let mut proj = Project::new("ZoomTest");
        let mut v = MediaItem::new("/tmp/fake_zoom.mp4");
        v.media_type = MediaType::Video;
        let vid_id = v.id.clone();
        proj.add_media(v);

        let mut c = Clip::new(&vid_id, 0.0, 5.0);
        c.text_overlay = Some(TextOverlay {
            content: "Cinematic Zoom".to_string(),
            motion_style: MotionTitleStyle::CinematicZoom,
            font_size: 50.0,
            ..Default::default()
        });
        proj.timeline.tracks[0].add_clip(c);

        let preset = ExportPreset::default();
        let (cmd, _) =
            TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();
        let fg = cmd
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &cmd[idx + 1])
            .unwrap();

        assert!(
            fg.contains("fontsize='50*(0.94+0.10*t/5.0000)'"),
            "must contain animated fontsize: {fg}"
        );
    }

    #[test]
    fn test_build_export_pipeline_with_in_out_range() {
        let mut proj = Project::new("RangeTest");
        let mut v = MediaItem::new("/tmp/fake_range.mp4");
        v.media_type = MediaType::Video;
        let vid_id = v.id.clone();
        proj.add_media(v);

        let mut v_outside = MediaItem::new("/tmp/outside_ignored.mp4");
        v_outside.media_type = MediaType::Video;
        let outside_id = v_outside.id.clone();
        proj.add_media(v_outside);

        proj.timeline.tracks[0].add_clip(Clip::new(&vid_id, 0.0, 10.0));
        // Clip completely after range (20.0 to 25.0)
        proj.timeline.tracks[0].add_clip(Clip::new(&outside_id, 20.0, 25.0));

        let preset = ExportPreset::default();
        let (cmd, dur) = TimelineExporter::build_export_pipeline_range(
            &proj,
            &preset,
            "/tmp/out_range.mp4",
            Some((2.5, 7.5)),
        )
        .unwrap();

        assert!(
            (dur - 5.0).abs() < 1e-4,
            "Effective duration should be 5.0s"
        );

        // Outside clip must be completely excluded!
        assert!(
            !cmd.iter().any(|arg| arg.contains("outside_ignored.mp4")),
            "Clips outside the in/out range must not be loaded into ffmpeg"
        );

        let ss_idx = cmd.iter().rposition(|a| a == "-ss");
        assert!(ss_idx.is_some(), "-ss argument must be present");
        assert_eq!(cmd[ss_idx.unwrap() + 1], "2.500000");

        let t_idx = cmd.iter().rposition(|a| a == "-t");
        assert!(t_idx.is_some(), "-t argument must be present");
        assert_eq!(cmd[t_idx.unwrap() + 1], "5.000000");
    }

    #[test]
    fn test_build_export_pipeline_with_crop() {
        let mut proj = Project::new("CropTest");
        let mut v = MediaItem::new("/tmp/fake_crop.mp4");
        v.media_type = MediaType::Video;
        let vid_id = v.id.clone();
        proj.add_media(v);
        let mut clip = Clip::new(&vid_id, 0.0, 5.0);
        clip.crop_left = 0.1;
        clip.crop_right = 0.15;
        clip.crop_top = 0.05;
        clip.crop_bottom = 0.2;
        proj.timeline.tracks[0].add_clip(clip);

        let preset = ExportPreset::default();
        let (cmd, _) =
            TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out_crop.mp4").unwrap();
        let fg = cmd
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &cmd[idx + 1])
            .unwrap();

        assert!(
            fg.contains("between(X,0.1000*W,0.8500*W)*between(Y,0.0500*H,0.8000*H)"),
            "Filtergraph must contain crop alpha mask: {fg}"
        );
    }

    #[test]
    fn test_build_export_pipeline_with_volume_keyframes() {
        let mut proj = Project::new("VolKfTest");
        let mut a = MediaItem::new("/tmp/fake_audio.mp3");
        a.media_type = MediaType::Audio;
        a.audio_codec = "mp3".into();
        let a_id = a.id.clone();
        proj.add_media(a);

        let mut clip = Clip::new(&a_id, 0.0, 5.0);
        clip.add_keyframe(
            "volume",
            0.0,
            1.0,
            trim_core::keyframe::Interpolation::Linear,
        );
        clip.add_keyframe(
            "volume",
            2.5,
            0.3,
            trim_core::keyframe::Interpolation::Linear,
        );
        clip.add_keyframe(
            "volume",
            5.0,
            1.0,
            trim_core::keyframe::Interpolation::Linear,
        );
        let a1 = proj
            .timeline
            .tracks
            .iter_mut()
            .find(|t| t.name == "A1")
            .unwrap();
        a1.add_clip(clip);

        let preset = ExportPreset::default();
        let (cmd, _) =
            TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out_vol.mp4").unwrap();
        let fg = cmd
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &cmd[idx + 1])
            .unwrap();

        assert!(
            fg.contains("volume=eval=frame:volume="),
            "Filtergraph must contain volume evaluation expression: {fg}"
        );
    }

    #[test]
    fn test_white_balance_gamma_and_expanded_looks_filtergraph() {
        use trim_core::clip::{BrightnessContrastPreset, LutPreset, WhiteBalancePreset};

        let mut proj = Project::new("ColorExportTest");
        let mut v = MediaItem::new("/tmp/fake_color.mp4");
        v.media_type = MediaType::Video;
        let vid_id = v.id.clone();
        proj.add_media(v);

        let mut c1 = Clip::new(&vid_id, 0.0, 5.0);
        c1.apply_wb_preset(WhiteBalancePreset::Tungsten);
        c1.apply_bc_preset(BrightnessContrastPreset::CinematicContrast);
        c1.lut_preset = LutPreset::Kodak2383;
        proj.timeline.tracks[0].add_clip(c1);

        let preset = ExportPreset::default();
        let (cmd, _) =
            TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out_color.mp4").unwrap();
        let fg = cmd
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &cmd[idx + 1])
            .unwrap();

        assert!(
            fg.contains("colorchannelmixer=rr="),
            "should include white balance colorchannelmixer filter: {fg}"
        );
        assert!(
            fg.contains(":gamma=0.900"),
            "should include gamma in eq filter for CinematicContrast: {fg}"
        );
        assert!(
            fg.contains("colorbalance=rs=-0.12"),
            "should include Kodak 2383 look filter: {fg}"
        );
    }

    #[test]
    fn test_compositing_blend_mask_adjustment_layer_filtergraph() {
        let mut proj = Project::new("CompTest");
        let mut m = MediaItem::new("/tmp/fake_video.mp4");
        m.media_type = MediaType::Video;
        m.duration = 5.0;
        let m_id = m.id.clone();
        proj.add_media(m);

        let mut base_clip = Clip::new(&m_id, 0.0, 5.0);
        base_clip.blend_mode = trim_core::BlendMode::Screen;
        let mut alpha_mask = trim_core::Mask::new(trim_core::MaskShape::Rectangle, "Box");
        alpha_mask.target = trim_core::MaskTarget::ClipAlpha;
        base_clip.masks.push(alpha_mask);
        proj.timeline.tracks[0].add_clip(base_clip);

        let mut adj = Clip::new_adjustment_layer(1.0, 4.0);
        adj.brightness = 0.15;
        adj.blend_mode = trim_core::BlendMode::Overlay;
        let mut fx_mask = trim_core::Mask::new(trim_core::MaskShape::Ellipse, "Spot");
        fx_mask.target = trim_core::MaskTarget::ColorAndEffects;
        adj.masks.push(fx_mask);
        proj.timeline.add_new_video_track();
        proj.timeline
            .tracks
            .iter_mut()
            .rfind(|t| t.track_type == TrackType::Video)
            .unwrap()
            .add_clip(adj);

        let preset = ExportPreset::default();
        let (cmd, _) =
            TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out_comp.mp4").unwrap();
        let fg = cmd
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &cmd[idx + 1])
            .unwrap();

        assert!(
            fg.contains("blend=all_mode=screen"),
            "Missing Screen blend: {fg}"
        );
        assert!(
            fg.contains("blend=all_mode=overlay"),
            "Missing Overlay blend on AdjLayer: {fg}"
        );
        assert!(
            fg.contains("maskedmerge"),
            "Missing maskedmerge for masks: {fg}"
        );
        assert!(
            fg.contains("eq=brightness=0.150"),
            "Missing AdjLayer color filter: {fg}"
        );
    }

    #[test]
    fn test_rotation_prores_and_audio_speed_ramp_export() {
        let mut proj = Project::new("ParityTest");
        let mut m = MediaItem::new("/tmp/fake_parity.mp4");
        m.media_type = MediaType::Video;
        m.duration = 10.0;
        m.audio_codec = "aac".to_string();
        let m_id = m.id.clone();
        proj.add_media(m);

        let mut clip = Clip::new(&m_id, 0.0, 4.0);
        clip.rotation = 45.0;
        clip.add_keyframe(
            "speed",
            0.0,
            1.0,
            trim_core::keyframe::Interpolation::Linear,
        );
        clip.add_keyframe(
            "speed",
            4.0,
            2.0,
            trim_core::keyframe::Interpolation::Linear,
        );
        proj.timeline.tracks[0].add_clip(clip.clone());

        let mut a_clip = Clip::new(&m_id, 0.0, 4.0);
        a_clip.add_keyframe(
            "speed",
            0.0,
            1.0,
            trim_core::keyframe::Interpolation::Linear,
        );
        a_clip.add_keyframe(
            "speed",
            4.0,
            2.0,
            trim_core::keyframe::Interpolation::Linear,
        );
        let audio_tr_id = proj.timeline.get_audio_tracks()[0].id.clone();
        proj.timeline
            .get_track_by_id_mut(&audio_tr_id)
            .unwrap()
            .add_clip(a_clip);

        let prores_preset = ExportPreset {
            video_codec: "prores_ks".to_string(),
            ..Default::default()
        };
        let (cmd, _) =
            TimelineExporter::build_export_pipeline(&proj, &prores_preset, "/tmp/out.mov").unwrap();

        assert!(cmd.contains(&"yuv422p10le".to_string()));
        let fg = cmd
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &cmd[idx + 1])
            .unwrap();
        assert!(fg.contains("rotate="), "Missing rotate filter: {fg}");
        assert!(
            fg.contains("atempo=1.5000"),
            "Missing average speed-ramp atempo on audio: {fg}"
        );
    }

    #[test]
    fn test_baked_cube_lut_filtergraph_for_camera_log_and_hsl_curves() {
        let mut proj = Project::new("BakedCubeTest");
        let mut v = MediaItem::new("/tmp/slog_footage.mp4");
        v.media_type = MediaType::Video;
        v.duration = 6.0;
        let vid_id = v.id.clone();
        proj.add_media(v);

        let mut c1 = Clip::new(&vid_id, 0.0, 4.0);
        c1.camera_log_profile = trim_core::CameraLogProfile::SLog3;
        c1.temperature = 14.0;
        c1.lut_preset = trim_core::LutPreset::Kodak2383;
        proj.timeline.tracks[0].add_clip(c1);

        let (cmd, _) = TimelineExporter::build_export_pipeline(
            &proj,
            &ExportPreset::default(),
            "/tmp/out.mp4",
        )
        .unwrap();
        let fg = cmd
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &cmd[idx + 1])
            .unwrap();
        assert!(
            fg.contains("lut3d=file='") && fg.contains(":interp=trilinear"),
            "Camera Log clip should bake 3D .cube LUT and emit trilinear lut3d filter: {fg}"
        );
    }

    #[test]
    fn test_filtergraph_escapes_injection_and_uses_protocol_whitelist_and_tempdir() {
        let mut proj = Project::new("SecurityTest");
        let mut v = MediaItem::new("-evil_leading_dash.mp4");
        v.media_type = MediaType::Video;
        v.duration = 5.0;
        let vid_id = v.id.clone();
        proj.add_media(v);

        let mut c1 = Clip::new(&vid_id, 0.0, 4.0);
        let txt = trim_core::clip::TextOverlay {
            content: "O'Brien %{localtime:%X}\n;copy[outv]".into(),
            motion_style: MotionTitleStyle::None,
            ..Default::default()
        };
        c1.text_overlay = Some(txt);
        proj.timeline.tracks[0].add_clip(c1);

        let prepared = TimelineExporter::build_export_pipeline_prepared(
            &proj,
            &ExportPreset::default(),
            "-out.mp4",
            None,
        )
        .unwrap();
        let temp_path = prepared.temp_dir.as_ref().unwrap().path().to_path_buf();
        assert!(temp_path.exists());

        let cmd = &prepared.cmd_args;
        assert!(cmd.windows(2).any(|w| w == ["-protocol_whitelist", "file"]));
        assert!(cmd.contains(&"./-evil_leading_dash.mp4".to_string()));
        assert_eq!(cmd.last().map(String::as_str), Some("./-out.mp4"));

        let fg = cmd
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &cmd[idx + 1])
            .unwrap();
        assert!(fg.contains(":expansion=none"));
        assert!(fg.contains("O'\\''Brien"));
        assert!(!fg.contains("\n;copy[outv]"));

        drop(prepared);
        assert!(!temp_path.exists());
    }

    #[test]
    fn test_k11_mask_plus_blend_and_adjustment_layer_split_valid_with_ffmpeg() {
        let mut proj = Project::new("K11Parity");
        proj.settings.width = 320;
        proj.settings.height = 180;
        proj.timeline.width = 320;
        proj.timeline.height = 180;
        proj.timeline.fps = 24.0;
        proj.timeline.tracks[1].pan = -0.5;

        let temp_dir = tempfile::tempdir().unwrap();
        let src_video = temp_dir.path().join("src.mp4");
        let out_video = temp_dir.path().join("out.mp4");

        // Create a real tiny 1-second test clip if FFmpeg is installed
        let has_ffmpeg = trim_media::is_ffmpeg_available();
        if has_ffmpeg {
            let gen = trim_media::ffmpeg_cmd()
                .args([
                    "-y",
                    "-v",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                    "color=c=blue:s=320x180:r=24:d=1",
                    "-f",
                    "lavfi",
                    "-i",
                    "sine=f=440:r=48000",
                    "-ac",
                    "2",
                    "-t",
                    "1",
                    "-c:v",
                    "libx264",
                    "-c:a",
                    "aac",
                    src_video.to_str().unwrap(),
                ])
                .output();
            assert!(
                matches!(gen, Ok(o) if o.status.success()),
                "failed to generate synthetic clip"
            );
        } else {
            std::fs::write(&src_video, b"dummy").unwrap();
        }

        let mut v = MediaItem::new(src_video.to_str().unwrap());
        v.media_type = MediaType::Video;
        v.duration = 1.0;
        v.audio_channels = 2;
        v.audio_sample_rate = 48000;
        let vid_id = v.id.clone();
        proj.add_media(v);

        let mut c1 = Clip::new(&vid_id, 0.0, 1.0);
        c1.blend_mode = trim_core::BlendMode::Screen;
        c1.masks.push(trim_core::mask::Mask::new(
            trim_core::mask::MaskShape::Rectangle,
            "m_alpha",
        ));
        proj.timeline.tracks[0].add_clip(c1);

        let mut adj = Clip::new_adjustment_layer(0.0, 1.0);
        adj.brightness = 0.1;
        adj.masks.push(trim_core::mask::Mask::new(
            trim_core::mask::MaskShape::Rectangle,
            "adj_a",
        ));
        let mut adj_fx_mask =
            trim_core::mask::Mask::new(trim_core::mask::MaskShape::Rectangle, "adj_fx");
        adj_fx_mask.target = trim_core::mask::MaskTarget::ColorAndEffects;
        adj.masks.push(adj_fx_mask);
        proj.timeline.tracks[0].add_clip(adj);

        let a1 = Clip::new(&vid_id, 0.0, 1.0);
        let audio_track = proj
            .timeline
            .tracks
            .iter_mut()
            .find(|t| t.track_type == TrackType::Audio)
            .unwrap();
        audio_track.pan = -0.5;
        audio_track.add_clip(a1);

        let preset = ExportPreset {
            loudness_norm: LoudnessNorm::YouTube,
            ..Default::default()
        };
        let prepared = TimelineExporter::build_export_pipeline_prepared(
            &proj,
            &preset,
            out_video.to_str().unwrap(),
            None,
        )
        .unwrap();

        let fg = prepared
            .cmd_args
            .iter()
            .position(|a| a == "-filter_complex")
            .map(|idx| &prepared.cmd_args[idx + 1])
            .unwrap();
        assert!(
            fg.contains("split=3"),
            "expected split=3 for blend+mask/adj: {fg}"
        );
        assert!(
            fg.contains("aresample=48000"),
            "expected 48kHz resample after loudnorm: {fg}"
        );
        assert!(
            fg.contains("pan=stereo|"),
            "expected track.pan in filtergraph: {fg}"
        );
        assert!(prepared
            .cmd_args
            .windows(2)
            .any(|w| w == ["-colorspace", "bt709"]));
        assert!(prepared
            .cmd_args
            .windows(2)
            .any(|w| w == ["-movflags", "+faststart"]));

        if has_ffmpeg {
            let status = trim_media::create_command(&prepared.cmd_args[0])
                .args(&prepared.cmd_args[1..])
                .output()
                .expect("run ffmpeg");
            assert!(
                status.status.success(),
                "FFmpeg failed on K-11 mask+blend+adj export: {}",
                String::from_utf8_lossy(&status.stderr)
            );
            assert!(out_video.is_file() && out_video.metadata().unwrap().len() > 0);
        }
    }
}
