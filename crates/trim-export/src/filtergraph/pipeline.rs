use super::helpers::*;
use super::{PreparedExport, TimelineExporter};
use crate::presets::{ExportPreset, HardwareAccel};
use trim_core::clip::Clip;
use trim_core::media::MediaType;
use trim_core::project::Project;
use trim_core::track::{Track, TrackType};

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

    pub(super) fn write_baked_clip_cube_lut_in(
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

        let _video_tracks = timeline.get_video_tracks();

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
                        if has_alpha_mask {
                            let p = mask_dir.join(format!("trim_mask_alpha_{safe_id}.pgm"));
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
                            let p = mask_dir.join(format!("trim_mask_fx_{safe_id}.pgm"));
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
                        let p = mask_dir.join(format!("trim_mask_alpha_{safe_id}.pgm"));
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
        if let Some(target_lufs) = preset
            .loudness_norm
            .target_lufs()
            .filter(|_| !audio_labels.is_empty())
        {
            mix.push_str(&format!(
                "amix=inputs={}:duration=first:dropout_transition=0:normalize=0[mixa];[mixa]alimiter=limit=0.988553:attack=1:release=75:level=disabled[lima];[lima]loudnorm=I={:.1}:LRA=11:TP=-1.5,aresample=48000,aformat=sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo[outa]",
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
