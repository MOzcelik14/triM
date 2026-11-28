use super::helpers::*;
use super::{TimelineExporter, PREVIEW_H, PREVIEW_W};
use trim_core::clip::{Clip, MotionTitleStyle};
use trim_core::media::{MediaItem, MediaType};

impl TimelineExporter {
    pub(super) fn clip_color_filters_in(
        clip: &Clip,
        temp_dir: Option<&std::path::Path>,
    ) -> Vec<String> {
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
    pub(super) fn write_mask_pgm(
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
    pub(super) fn clip_video_chain(
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
                                "drawtext=text='{sub_sanitized}'{font_part}:fontsize={sub_size}:fontcolor=0xc8d7e6dc:x={x_expr}:y={sub_y}{shadow_part}:expansion=none"
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
    pub(super) fn overlay_position(clip: &Clip, out_w: u32, out_h: u32) -> (String, String) {
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

    /// Builds an FFmpeg `xfade` video transition filter for `TransitionKind`.
    pub fn build_xfade_transition_filter(
        kind: trim_core::clip::TransitionKind,
        duration: f64,
        offset: f64,
    ) -> String {
        format!(
            "xfade=transition={}:duration={:.4}:offset={:.4}",
            kind.xfade_name(),
            duration.max(0.01),
            offset.max(0.0)
        )
    }

    /// Builds an FFmpeg `acrossfade` audio transition filter.
    pub fn build_acrossfade_filter(duration: f64) -> String {
        format!("acrossfade=d={:.4}:c1=tri:c2=tri", duration.max(0.01))
    }
}
