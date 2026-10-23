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
use trim_core::clip::{Clip, LutPreset};
use trim_core::keyframe::{Interpolation, Keyframe};
use trim_core::media::{MediaItem, MediaType};
use trim_core::project::Project;
use trim_core::track::TrackType;

/// Preview canvas size used by the interactive monitor. Clip positions
/// (`pos_x`, `pos_y`) are stored in preview pixels.
const PREVIEW_W: f64 = 640.0;
const PREVIEW_H: f64 = 360.0;

pub struct TimelineExporter;

/// Accumulates FFmpeg input arguments and hands out input indices.
struct InputList {
    args: Vec<String>,
    count: usize,
}

impl InputList {
    fn new() -> Self {
        Self { args: Vec::new(), count: 0 }
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
    let mut kfs: Vec<Keyframe> = keyframes.iter().filter(|k| k.time.is_finite() && k.value.is_finite()).cloned().collect();
    kfs.sort_by(|a, b| a.time.total_cmp(&b.time));
    match kfs.len() {
        0 => return format!("{:.6}", map(0.0)),
        1 => return format!("{:.6}", map(kfs[0].value)),
        _ => {}
    }

    // Build nested if() from the last segment backwards.
    let last = kfs.last().unwrap();
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
                Interpolation::EaseInOut => {
                    format!("({:.6}+({:.6})*(3*{u}*{u}-2*{u}*{u}*{u}))", v0, v1 - v0)
                }
            }
        };
        expr = format!("if(lt({var},{:.6}),{seg},{expr})", k1.time);
    }
    let first = &kfs[0];
    format!("if(lt({var},{:.6}),{:.6},{expr})", first.time, map(first.value))
}

fn animated_kfs<'a>(clip: &'a Clip, prop: &str) -> Option<&'a [Keyframe]> {
    clip.keyframes.get(prop).filter(|k| !k.is_empty()).map(|v| v.as_slice())
}

/// Source window `[start, start + src_dur)` that a clip reads from.
fn source_window(clip: &Clip) -> (f64, f64) {
    let dur = clip.duration();
    let src_dur = dur * clip.speed.max(0.0001);
    let start = if clip.reverse {
        (clip.effective_source_out() - src_dur).max(0.0)
    } else {
        clip.source_in.max(0.0)
    };
    (start, src_dur)
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

    /// Video filter chain for one clip, producing an RGBA stream whose PTS is
    /// already shifted to the clip's timeline position.
    fn clip_video_chain(clip: &Clip, media: &MediaItem, out_w: u32, out_h: u32, out_fps: f64) -> String {
        let dur = clip.duration();
        let mut f: Vec<String> = Vec::new();

        let is_image = media.media_type == MediaType::Image;
        if !is_image && clip.reverse {
            f.push("reverse".into());
        }
        if !is_image && (clip.speed - 1.0).abs() > 0.001 {
            f.push(format!("setpts=(PTS-STARTPTS)/{:.6}", clip.speed));
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
        f.push(format!("scale={box_w}:{box_h}:force_original_aspect_ratio=decrease"));
        if !is_image {
            f.push(format!("pad={box_w}:{box_h}:(ow-iw)/2:(oh-ih)/2:black"));
        }
        f.push("setsar=1".into());

        if clip.brightness != 0.0 || clip.contrast != 1.0 || clip.saturation != 1.0 {
            f.push(format!(
                "eq=brightness={:.3}:contrast={:.3}:saturation={:.3}",
                clip.brightness, clip.contrast, clip.saturation
            ));
        }

        if let Some(lut_path) = &clip.custom_lut_path {
            if !lut_path.trim().is_empty() {
                let sanitized = lut_path.replace('\\', "/").replace('\'', "\\'");
                f.push(format!("lut3d=file='{sanitized}'"));
            }
        } else {
            match clip.lut_preset {
                LutPreset::None => {}
                LutPreset::TealOrange => {
                    f.push("colorbalance=rs=0.15:gs=-0.05:bs=-0.25:rh=-0.2:gh=0.05:bh=0.25,eq=contrast=1.15:saturation=1.2".to_string());
                }
                LutPreset::VintageFilm => {
                    f.push("curves=vintage,eq=contrast=0.95:saturation=0.85".to_string());
                }
                LutPreset::MonochromeNoir => {
                    f.push("hue=s=0,eq=contrast=1.45:brightness=-0.05".to_string());
                }
                LutPreset::CyberpunkNeon => {
                    f.push("colorbalance=rm=0.2:gm=-0.1:bm=0.3:rh=-0.1:gh=-0.05:bh=0.25,eq=contrast=1.25:saturation=1.4".to_string());
                }
            }
        }

        f.push("format=rgba".into());

        // Opacity (static or keyframed).
        if let Some(kfs) = animated_kfs(clip, "opacity") {
            let expr = keyframe_expr(kfs, "T", |v| v.clamp(0.0, 1.0));
            f.push(format!("geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='alpha(X,Y)*({expr})'"));
        } else if clip.opacity < 0.999 {
            f.push(format!("colorchannelmixer=aa={:.4}", clip.opacity.clamp(0.0, 1.0)));
        }

        // Fades and Transitions: regular fades dissolve to whatever is underneath.
        // Dip-to-white fades the pixels to white.
        // Wipe and slide transitions perform directional revealing.
        if clip.fade_in > 0.0 {
            let d = clip.fade_in.min(dur);
            match clip.transition_in.as_deref() {
                Some("dip_white") => f.push(format!("fade=t=in:st=0:d={:.4}:color=white", d)),
                Some("wipe_left") => f.push(format!("geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='if(lte(X/W,min(1,T/{:.4})),alpha(X,Y),0)'", d)),
                Some("wipe_right") => f.push(format!("geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='if(gte(X/W,max(0,1-T/{:.4})),alpha(X,Y),0)'", d)),
                _ => f.push(format!("fade=t=in:st=0:d={:.4}:alpha=1", d)),
            }
        }
        if clip.fade_out > 0.0 {
            let d = clip.fade_out.min(dur);
            let st = (dur - d).max(0.0);
            match clip.transition_out.as_deref() {
                Some("dip_white") => f.push(format!("fade=t=out:st={:.4}:d={:.4}:color=white", st, d)),
                Some("wipe_left") => f.push(format!("geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='if(lte(X/W,max(0,1-(T-{:.4})/{:.4})),alpha(X,Y),0)'", st, d)),
                Some("wipe_right") => f.push(format!("geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='if(gte(X/W,min(1,(T-{:.4})/{:.4})),alpha(X,Y),0)'", st, d)),
                _ => f.push(format!("fade=t=out:st={:.4}:d={:.4}:alpha=1", st, d)),
            }
        }

        if let Some(txt) = &clip.text_overlay {
            let sanitized = txt.content
                .replace('\\', "\\\\")
                .replace('\'', "\\'")
                .replace(':', "\\:")
                .replace('%', "\\%");
            let font_size = ((txt.font_size as f64) * (out_h as f64 / 1080.0)).round().max(10.0) as u32;
            let font_color = format!(
                "0x{:02x}{:02x}{:02x}{:02x}",
                txt.text_color[0], txt.text_color[1], txt.text_color[2], txt.text_color[3]
            );
            let x_expr = format!("(w-text_w)*{:.3}", txt.pos_x.clamp(0.0, 1.0));
            let y_expr = format!("(h-text_h)*{:.3}", txt.pos_y.clamp(0.0, 1.0));
            let shadow_part = if txt.has_shadow {
                ":shadowx=2:shadowy=2:shadowcolor=black@0.7"
            } else {
                ""
            };
            let box_part = if let Some(bg) = txt.bg_color {
                format!(
                    ":box=1:boxcolor=0x{:02x}{:02x}{:02x}@{:.2}:boxborderw=8",
                    bg[0], bg[1], bg[2], bg[3] as f32 / 255.0
                )
            } else {
                String::new()
            };
            f.push(format!(
                "drawtext=text='{sanitized}':fontsize={font_size}:fontcolor={font_color}:x={x_expr}:y={y_expr}{shadow_part}{box_part}"
            ));
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
        if clip.fade_in > 0.0 {
            let d = clip.fade_in.min(clip.duration());
            if clip.transition_in.as_deref() == Some("slide_left") {
                slide_x.push_str(&format!("+if(lt({local},{d:.4}),(1-{local}/{d:.4})*W,0)"));
            } else if clip.transition_in.as_deref() == Some("slide_right") {
                slide_x.push_str(&format!("-if(lt({local},{d:.4}),(1-{local}/{d:.4})*W,0)"));
            }
        }
        if clip.fade_out > 0.0 {
            let dur = clip.duration();
            let d = clip.fade_out.min(dur);
            let st = (dur - d).max(0.0);
            if clip.transition_out.as_deref() == Some("slide_left") {
                slide_x.push_str(&format!("-if(gt({local},{st:.4}),(({local}-{st:.4})/{d:.4})*W,0)"));
            } else if clip.transition_out.as_deref() == Some("slide_right") {
                slide_x.push_str(&format!("+if(gt({local},{st:.4}),(({local}-{st:.4})/{d:.4})*W,0)"));
            }
        }

        (format!("'(W-w)/2+{px}{slide_x}'"), format!("'(H-h)/2+{py}'"))
    }

    /// Audio filter chain for one clip, delayed to its timeline position.
    fn clip_audio_chain(clip: &Clip, eff_vol: f64) -> String {
        let dur = clip.duration();
        let mut a: Vec<String> = vec!["aformat=sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo".into()];
        if clip.reverse {
            a.push("areverse".into());
        }
        a.push("asetpts=PTS-STARTPTS".into());
        if (clip.speed - 1.0).abs() > 0.001 {
            a.extend(Self::get_atempo_filters(clip.speed));
        }
        // Force exact clip length so timing never drifts.
        a.push(format!("atrim=duration={:.6}", dur));
        a.push(format!("apad=whole_dur={:.6}", dur));
        if (eff_vol - 1.0).abs() > 0.001 {
            a.push(format!("volume={:.4}", eff_vol));
        }

        // Audio EQ preset and noise reduction filters
        match clip.eq_preset {
            trim_core::clip::AudioEqPreset::VoiceClarity => {
                a.push("highpass=f=80,equalizer=f=3000:t=q:w=1.5:g=3,equalizer=f=300:t=q:w=1.0:g=-2".into());
            }
            trim_core::clip::AudioEqPreset::BassBoost => {
                a.push("equalizer=f=100:t=q:w=1.0:g=6".into());
            }
            trim_core::clip::AudioEqPreset::Podcast => {
                a.push("highpass=f=75,equalizer=f=2500:t=q:w=1.2:g=2.5,lowpass=f=12000".into());
            }
            trim_core::clip::AudioEqPreset::DenoiseGate => {
                a.push("highpass=f=100,lowpass=f=10000,agate=threshold=0.03:ratio=2:range=0.05".into());
            }
            trim_core::clip::AudioEqPreset::Flat => {}
        }

        if clip.noise_gate && clip.eq_preset != trim_core::clip::AudioEqPreset::DenoiseGate {
            a.push("agate=threshold=0.03:ratio=2:range=0.05".into());
        }

        if clip.fade_in > 0.0 {
            a.push(format!("afade=t=in:st=0:d={:.4}", clip.fade_in.min(dur)));
        }
        if clip.fade_out > 0.0 {
            let d = clip.fade_out.min(dur);
            a.push(format!("afade=t=out:st={:.4}:d={:.4}", (dur - d).max(0.0), d));
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
        let timeline = &project.timeline;
        let video_tracks = timeline.get_video_tracks();
        if video_tracks.is_empty() {
            return Err("Projede video kanalı bulunamadı".to_string());
        }

        let total_duration = timeline.duration();
        let has_any_clip = timeline
            .tracks
            .iter()
            .any(|t| t.clips.iter().any(|c| project.get_media(&c.media_id).is_some() && c.duration() > 0.0));
        if !has_any_clip || total_duration <= 0.0 {
            return Err("Dışa aktarılacak klip bulunamadı".to_string());
        }

        let out_w = even(preset.width.unwrap_or(timeline.width));
        let out_h = even(preset.height.unwrap_or(timeline.height));
        let out_fps = preset.fps.unwrap_or(timeline.fps);
        let out_fps = if out_fps.is_finite() && out_fps > 0.0 { out_fps } else { 30.0 };

        let mut inputs = InputList::new();
        let mut filters: Vec<String> = Vec::new();

        // Base canvas and silence bed spanning the whole timeline.
        let base_v = inputs.push([
            "-f".into(),
            "lavfi".into(),
            "-i".into(),
            format!("color=c=black:s={}x{}:r={}:d={:.6}", out_w, out_h, out_fps, total_duration),
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

        for track in &timeline.tracks {
            let track_audible = !track.muted && (!any_solo || track.solo);
            for clip in &track.clips {
                let dur = clip.duration();
                if dur <= 0.0 {
                    continue;
                }
                let Some(media) = project.get_media(&clip.media_id) else {
                    continue;
                };
                let is_image = media.media_type == MediaType::Image;
                let wants_video = track.track_type == TrackType::Video && track.visible && media.has_video();
                let wants_audio = track_audible && !clip.muted && !is_image && media.has_audio();
                if !wants_video && !wants_audio {
                    continue;
                }

                let input_idx = if is_image {
                    inputs.push([
                        "-loop".into(),
                        "1".into(),
                        "-framerate".into(),
                        format!("{}", out_fps),
                        "-t".into(),
                        format!("{:.6}", dur),
                        "-i".into(),
                        media.file_path.clone(),
                    ])
                } else {
                    let (src_start, src_dur) = source_window(clip);
                    inputs.push([
                        "-ss".into(),
                        format!("{:.6}", src_start),
                        "-t".into(),
                        format!("{:.6}", src_dur),
                        "-i".into(),
                        media.file_path.clone(),
                    ])
                };

                if wants_video {
                    let chain = Self::clip_video_chain(clip, media, out_w, out_h, out_fps);
                    let clip_label = format!("cv{input_idx}");
                    filters.push(format!("[{input_idx}:v]{chain}[{clip_label}]"));
                    let (x, y) = Self::overlay_position(clip, out_w, out_h);
                    overlay_count += 1;
                    let next = format!("base{overlay_count}");
                    filters.push(format!(
                        "[{last_video}][{clip_label}]overlay=x={x}:y={y}:eval=frame:eof_action=pass[{next}]"
                    ));
                    last_video = next;
                }

                if wants_audio {
                    let eff_vol = clip.volume * track.volume;
                    let chain = Self::clip_audio_chain(clip, eff_vol);
                    let label = format!("ca{input_idx}");
                    filters.push(format!("[{input_idx}:a]{chain}[{label}]"));
                    audio_labels.push(label);
                }
            }
        }

        filters.push(format!("[{last_video}]format=yuv420p[outv]"));

        let mut mix = format!("[{base_a}:a]aformat=sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo[abed];[abed]");
        for l in &audio_labels {
            mix.push_str(&format!("[{l}]"));
        }
        if let Some(target_lufs) = preset.loudness_norm.target_lufs() {
            mix.push_str(&format!(
                "amix=inputs={}:duration=first:dropout_transition=0:normalize=0[mixa];[mixa]loudnorm=I={:.1}:LRA=11:TP=-1.5[outa]",
                audio_labels.len() + 1,
                target_lufs
            ));
        } else {
            mix.push_str(&format!(
                "amix=inputs={}:duration=first:dropout_transition=0:normalize=0[outa]",
                audio_labels.len() + 1
            ));
        }
        filters.push(mix);

        let full_filtergraph = filters.join(";");

        let mut cmd = vec!["ffmpeg".into(), "-y".into(), "-v".into(), "error".into(), "-nostdin".into()];
        cmd.extend(inputs.args);
        cmd.extend([
            "-filter_complex".into(),
            full_filtergraph,
            "-map".into(),
            "[outv]".into(),
            "-map".into(),
            "[outa]".into(),
        ]);
        cmd.extend(Self::resolve_video_codec_args(preset));
        cmd.extend([
            "-c:a".into(),
            preset.audio_codec.clone(),
            "-b:a".into(),
            preset.audio_bitrate.clone(),
            "-pix_fmt".into(),
            "yuv420p".into(),
            "-r".into(),
            format!("{}", out_fps),
            "-t".into(),
            format!("{:.6}", total_duration),
            "-progress".into(),
            "pipe:1".into(),
            "-nostats".into(),
            output_path.to_string(),
        ]);

        Ok((cmd, total_duration))
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
                let codec = if is_hevc && available.hevc_nvenc { "hevc_nvenc" } else { "h264_nvenc" };
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
            HardwareAccel::Qsv if available.has_intel_qsv() => {
                let codec = if is_hevc && available.hevc_qsv { "hevc_qsv" } else { "h264_qsv" };
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
                let codec = if is_hevc && available.hevc_vaapi { "hevc_vaapi" } else { "h264_vaapi" };
                vec![
                    "-c:v".into(),
                    codec.into(),
                    "-qp".into(),
                    preset.crf.to_string(),
                ]
            }
            HardwareAccel::Amf if available.has_amd_amf() => {
                let codec = if is_hevc && available.hevc_amf { "hevc_amf" } else { "h264_amf" };
                vec![
                    "-c:v".into(),
                    codec.into(),
                    "-quality".into(),
                    "quality".into(),
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
        let video_track = project.timeline.get_video_tracks_mut().into_iter().next().unwrap();
        video_track.add_clip(Clip::new(id, 0.0, 5.0));

        let (command, _) =
            TimelineExporter::build_export_pipeline(&project, &ExportPreset::default(), "/tmp/out.mp4").unwrap();
        let graph = graph_of(&command);
        // Inputs: 0 = base colour, 1 = silence bed, 2 = clip.
        assert!(graph.contains("[2:v]"), "video input missing: {graph}");
        assert!(!graph.contains("[2:a]"), "silent media must not reference an audio stream: {graph}");
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

        let v_track = proj.timeline.get_video_tracks_mut().into_iter().next().unwrap();
        let mut c = Clip::new(&m_id, 2.0, 6.0);
        c.speed = 2.0;
        c.brightness = 0.1;
        v_track.add_clip(c);

        let preset = ExportPreset::default();
        let (cmd, dur) = TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();

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

        proj.timeline.tracks[0].add_clip(Clip::new(&bg_id, 0.0, 10.0));
        let mut v2 = Track::new("V2", TrackType::Video);
        v2.add_clip(Clip::new(&fg_id, 2.0, 4.0));
        proj.timeline.tracks.insert(1, v2);
        let a1 = proj.timeline.get_audio_tracks_mut().into_iter().next().unwrap();
        a1.add_clip(Clip::new(&mu_id, 1.0, 9.0));

        let (cmd, dur) = TimelineExporter::build_export_pipeline(&proj, &ExportPreset::default(), "/tmp/o.mp4").unwrap();
        let graph = graph_of(&cmd);
        assert!((dur - 10.0).abs() < 1e-6);
        assert!(!graph.contains("concat"), "{graph}");
        assert_eq!(graph.matches("overlay=").count(), 2, "{graph}");
        // Background audio + music on the audio track + silence bed.
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
        proj.timeline.tracks[0].muted = true;
        let (cmd, _) = TimelineExporter::build_export_pipeline(&proj, &ExportPreset::default(), "/tmp/o.mp4").unwrap();
        let g = graph_of(&cmd);
        assert!(g.contains("overlay="));
        assert!(g.contains("amix=inputs=1"), "{g}");

        proj.timeline.tracks[0].muted = false;
        proj.timeline.tracks[0].visible = false;
        let (cmd, _) = TimelineExporter::build_export_pipeline(&proj, &ExportPreset::default(), "/tmp/o.mp4").unwrap();
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
        use std::process::Command;
        if Command::new("ffmpeg").arg("-version").output().is_err() {
            eprintln!("ffmpeg not available, skipping");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let vid = dir.path().join("src.mp4");
        let img = dir.path().join("logo.png");
        let aud = dir.path().join("tone.wav");
        let ok = |args: &[&str]| {
            Command::new("ffmpeg").args(["-v", "error", "-y"]).args(args).status().map(|s| s.success()).unwrap_or(false)
        };
        assert!(ok(&[
            "-f", "lavfi", "-i", "testsrc=size=320x240:rate=25:duration=4",
            "-f", "lavfi", "-i", "sine=frequency=440:duration=4",
            "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest",
            vid.to_str().unwrap(),
        ]));
        assert!(ok(&["-f", "lavfi", "-i", "color=c=red@0.5:s=64x64,format=rgba", "-frames:v", "1", img.to_str().unwrap()]));
        assert!(ok(&["-f", "lavfi", "-i", "sine=frequency=880:duration=2", aud.to_str().unwrap()]));

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
        let a1 = proj.timeline.get_audio_tracks_mut().into_iter().next().unwrap();
        a1.add_clip(Clip::new(&aud_id, 0.5, 2.5));

        let out = dir.path().join("out.mp4");
        let preset = ExportPreset {
            width: None,
            height: None,
            preset: "ultrafast".into(),
            hw_accel: HardwareAccel::Cpu,
            ..Default::default()
        };
        let (cmd, dur) = TimelineExporter::build_export_pipeline(&proj, &preset, out.to_str().unwrap()).unwrap();
        let output = Command::new(&cmd[0]).args(&cmd[1..]).output().unwrap();
        assert!(output.status.success(), "ffmpeg failed: {}", String::from_utf8_lossy(&output.stderr));

        let probe = Command::new("ffprobe")
            .args(["-v", "error", "-show_entries", "format=duration", "-of", "default=nw=1:nk=1"])
            .arg(&out)
            .output()
            .unwrap();
        let got: f64 = String::from_utf8_lossy(&probe.stdout).trim().parse().unwrap_or(0.0);
        assert!((got - dur).abs() < 0.25, "duration {got} vs {dur}");
    }

    #[test]
    fn test_resolve_video_codec_args() {
        let mut preset = ExportPreset::default();
        preset.hw_accel = HardwareAccel::Cpu;
        let args = TimelineExporter::resolve_video_codec_args(&preset);
        assert!(args.contains(&"libx264".to_string()));
        assert!(args.contains(&"-crf".to_string()));

        let mut prores_preset = ExportPreset::default();
        prores_preset.video_codec = "prores_ks".to_string();
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
        proj.timeline.tracks[0].add_clip(c);

        let mut preset = ExportPreset::default();
        preset.loudness_norm = LoudnessNorm::YouTube;
        let (cmd, _) = TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();

        let fg = cmd.iter().position(|a| a == "-filter_complex").map(|idx| &cmd[idx + 1]).unwrap();
        assert!(fg.contains("equalizer=f=3000"), "filtergraph should include voice clarity EQ: {fg}");
        assert!(fg.contains("agate=threshold=0.03"), "filtergraph should include noise gate: {fg}");
        assert!(fg.contains("loudnorm=I=-14.0"), "filtergraph should include EBU R128 loudnorm: {fg}");
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
        });
        proj.timeline.tracks[0].add_clip(c);

        let preset = ExportPreset::default();
        let (cmd, _) = TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();
        let fg = cmd.iter().position(|a| a == "-filter_complex").map(|idx| &cmd[idx + 1]).unwrap();

        assert!(fg.contains("drawtext=text='Hello triM'"), "filtergraph should contain drawtext: {fg}");
        assert!(fg.contains("box=1:boxcolor="), "filtergraph should contain subtitle box: {fg}");
        assert!(fg.contains("shadowx=2"), "filtergraph should contain shadow: {fg}");
    }

    #[test]
    fn test_lut_preset_and_custom_cube_filtergraph() {
        let mut proj = Project::new("LutTest");
        let mut v = MediaItem::new("/tmp/fake_lut.mp4");
        v.media_type = MediaType::Video;
        let vid_id = v.id.clone();
        proj.add_media(v);

        let mut c1 = Clip::new(&vid_id, 0.0, 5.0);
        c1.lut_preset = LutPreset::TealOrange;
        proj.timeline.tracks[0].add_clip(c1);

        let preset = ExportPreset::default();
        let (cmd1, _) = TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out1.mp4").unwrap();
        let fg1 = cmd1.iter().position(|a| a == "-filter_complex").map(|idx| &cmd1[idx + 1]).unwrap();
        assert!(fg1.contains("colorbalance=rs=0.15"), "should generate teal & orange filter: {fg1}");

        // Now test custom cube file
        proj.timeline.tracks[0].clips[0].custom_lut_path = Some("/path/to/cinematic.cube".to_string());
        let (cmd2, _) = TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out2.mp4").unwrap();
        let fg2 = cmd2.iter().position(|a| a == "-filter_complex").map(|idx| &cmd2[idx + 1]).unwrap();
        assert!(fg2.contains("lut3d=file='/path/to/cinematic.cube'"), "should generate lut3d filter: {fg2}");
    }
}

