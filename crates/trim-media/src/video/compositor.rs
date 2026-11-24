use super::lut::{parse_cube_lut_file, CubeLut3D, VideoFrameDecoder};
use image::{ImageBuffer, Rgba, RgbaImage};
use std::path::PathBuf;
use trim_core::clip::Clip;

pub(super) fn draw_clip_onto_canvas(
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
                    custom_lut.as_deref(),
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

    // Zoom-in / Zoom-out transitions
    if clip.transition_in.as_deref() == Some("zoom_in") && in_progress < 1.0 {
        eff_scale *= 0.3 + 0.7 * in_progress;
    } else if clip.transition_in.as_deref() == Some("zoom_out") && in_progress < 1.0 {
        eff_scale *= 1.7 - 0.7 * in_progress;
    }
    if clip.transition_out.as_deref() == Some("zoom_out") && out_progress < 1.0 {
        eff_scale *= 0.3 + 0.7 * out_progress;
    } else if clip.transition_out.as_deref() == Some("zoom_in") && out_progress < 1.0 {
        eff_scale *= 1.7 - 0.7 * out_progress;
    }

    let final_w = (fw * eff_scale).round() as i64;
    let final_h = (fh * eff_scale).round() as i64;

    let mut slide_x_offset = 0.0;
    if in_progress < 1.0 {
        if matches!(
            clip.transition_in.as_deref(),
            Some("slide_left") | Some("whip_pan")
        ) {
            let p = if clip.transition_in.as_deref() == Some("whip_pan") {
                (1.0 - in_progress).powi(2)
            } else {
                1.0 - in_progress
            };
            slide_x_offset += p * cw;
        } else if clip.transition_in.as_deref() == Some("slide_right") {
            slide_x_offset -= (1.0 - in_progress) * cw;
        }
    }
    if out_progress < 1.0 {
        if matches!(
            clip.transition_out.as_deref(),
            Some("slide_left") | Some("whip_pan")
        ) {
            let p = if clip.transition_out.as_deref() == Some("whip_pan") {
                (1.0 - out_progress).powi(2)
            } else {
                1.0 - out_progress
            };
            slide_x_offset -= p * cw;
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
    let luma_wipe_active = (clip.transition_in.as_deref() == Some("luma_wipe")
        && in_progress < 1.0)
        || (clip.transition_out.as_deref() == Some("luma_wipe") && out_progress < 1.0);
    let glitch_active = (clip.transition_in.as_deref() == Some("glitch") && in_progress < 1.0)
        || (clip.transition_out.as_deref() == Some("glitch") && out_progress < 1.0);
    let light_leak_active = (clip.transition_in.as_deref() == Some("light_leak")
        && in_progress < 1.0)
        || (clip.transition_out.as_deref() == Some("light_leak") && out_progress < 1.0);
    let blur_dissolve_active = (clip.transition_in.as_deref() == Some("blur_dissolve")
        && in_progress < 1.0)
        || (clip.transition_out.as_deref() == Some("blur_dissolve") && out_progress < 1.0);
    let whip_pan_active = (clip.transition_in.as_deref() == Some("whip_pan") && in_progress < 1.0)
        || (clip.transition_out.as_deref() == Some("whip_pan") && out_progress < 1.0);

    let eff_blur = clip.blur.clamp(0.0, 20.0)
        + if blur_dissolve_active || whip_pan_active {
            ((1.0 - fade_factor) as f32) * 10.0
        } else {
            0.0
        };

    let has_chromakey = clip.chromakey.as_ref().is_some_and(|ck| ck.enabled);
    let has_color_adj = clip.has_color_adjustments() || has_chromakey;
    let has_clip_grading = clip.has_color_adjustments();
    let has_blend_or_mask = clip.blend_mode != trim_core::BlendMode::Normal || has_masks;
    let has_special_trans = has_dip_white
        || wipe_left_active
        || wipe_right_active
        || luma_wipe_active
        || glitch_active
        || light_leak_active
        || eff_blur > 0.01
        || slide_x_offset.abs() > 0.1;

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

        if alpha >= 0.999 && src_raw.as_chunks::<4>().0.iter().all(|p| p[3] == 255) {
            dst_raw.copy_from_slice(src_raw);
            return;
        }

        let alpha_q8 = (alpha.clamp(0.0, 1.0) * 256.0).round() as u32;
        for (dst, src) in dst_raw
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(src_raw.as_chunks::<4>().0.iter())
        {
            let sa = (src[3] as u32 * alpha_q8) >> 8;
            let inv_sa = 256 - sa;
            dst[0] = ((src[0] as u32 * sa + dst[0] as u32 * inv_sa + 128) >> 8) as u8;
            dst[1] = ((src[1] as u32 * sa + dst[1] as u32 * inv_sa + 128) >> 8) as u8;
            dst[2] = ((src[2] as u32 * sa + dst[2] as u32 * inv_sa + 128) >> 8) as u8;
            dst[3] = 255;
        }
        return;
    }

    let fw_u32 = frame.width().max(1);
    let fh_u32 = frame.height().max(1);
    let blur_step = (eff_blur.round() as i32).clamp(0, 12);
    let sharpen_amt = clip.sharpen.clamp(0.0, 2.0);
    let vignette_amt = clip.vignette.clamp(0.0, 1.0);
    let grain_amt = clip.film_grain.clamp(0.0, 1.0);
    let halation_amt = clip.halation.clamp(0.0, 1.0);
    let bloom_amt = clip.bloom.clamp(0.0, 1.0);
    let ca_amt = clip.chromatic_aberration.clamp(0.0, 1.0);
    let active_cp = clip.corner_pin.as_ref().filter(|cp| cp.is_active());

    let y_start = (-final_y).max(0);
    let y_end = (ch as i64 - final_y).min(final_h);
    let x_start = (-final_x).max(0);
    let x_end = (cw as i64 - final_x).min(final_w);

    for y in y_start..y_end {
        let cy = final_y + y;

        let norm_y = y as f32 / final_h.max(1) as f32;
        let sy_base = ((y as f64 / final_h as f64) * fh) as u32;
        let sy_base = sy_base.min(fh_u32 - 1);

        for x in x_start..x_end {
            let cx = final_x + x;

            let norm_x = x as f64 / final_w.max(1) as f64;
            if wipe_left_active && norm_x > fade_factor {
                continue;
            }
            if wipe_right_active && norm_x < (1.0 - fade_factor) {
                continue;
            }

            let (sx, sy) = if let Some(cp) = active_cp {
                let Some([u_cp, v_cp]) = invert_bilinear_quad(
                    [norm_x as f32, norm_y],
                    cp.top_left,
                    cp.top_right,
                    cp.bottom_right,
                    cp.bottom_left,
                ) else {
                    continue;
                };
                if !(0.0..=1.0).contains(&u_cp) || !(0.0..=1.0).contains(&v_cp) {
                    continue;
                }
                (
                    ((u_cp * fw as f32) as u32).min(fw_u32 - 1),
                    ((v_cp * fh as f32) as u32).min(fh_u32 - 1),
                )
            } else {
                let sx = ((x as f64 / final_w as f64) * fw) as u32;
                (sx.min(fw_u32 - 1), sy_base)
            };

            let src_px = frame.get_pixel(sx, sy);
            let mut r = src_px[0] as f32;
            let mut g = src_px[1] as f32;
            let mut b = src_px[2] as f32;
            let mut a = (src_px[3] as f32 / 255.0) * eff_opacity as f32;

            if ca_amt > 0.001 {
                let radial = ((norm_x as f32 - 0.5).abs() + (norm_y - 0.5).abs()).clamp(0.0, 1.0);
                let ca_px = ((ca_amt * radial * 10.0).round() as i32).clamp(1, 14);
                let sx_r = (sx as i32 + ca_px).clamp(0, fw_u32 as i32 - 1) as u32;
                let sx_b = (sx as i32 - ca_px).clamp(0, fw_u32 as i32 - 1) as u32;
                r = frame.get_pixel(sx_r, sy)[0] as f32;
                b = frame.get_pixel(sx_b, sy)[2] as f32;
            }

            if glitch_active {
                let shift_px = (((1.0 - fade_factor) * 12.0 * (norm_y as f64 * 36.0).sin()).round()
                    as i32)
                    .clamp(-16, 16);
                let sx_r = (sx as i32 + shift_px).clamp(0, fw_u32 as i32 - 1) as u32;
                let sx_b = (sx as i32 - shift_px).clamp(0, fw_u32 as i32 - 1) as u32;
                r = frame.get_pixel(sx_r, sy)[0] as f32;
                b = frame.get_pixel(sx_b, sy)[2] as f32;
            }

            if blur_step > 0 {
                let s_l = (sx as i32 - blur_step).clamp(0, fw_u32 as i32 - 1) as u32;
                let s_r = (sx as i32 + blur_step).clamp(0, fw_u32 as i32 - 1) as u32;
                let s_u = (sy as i32 - blur_step).clamp(0, fh_u32 as i32 - 1) as u32;
                let s_d = (sy as i32 + blur_step).clamp(0, fh_u32 as i32 - 1) as u32;
                let p1 = frame.get_pixel(s_l, sy);
                let p2 = frame.get_pixel(s_r, sy);
                let p3 = frame.get_pixel(sx, s_u);
                let p4 = frame.get_pixel(sx, s_d);
                r = r * 0.28 + (p1[0] as f32 + p2[0] as f32 + p3[0] as f32 + p4[0] as f32) * 0.18;
                g = g * 0.28 + (p1[1] as f32 + p2[1] as f32 + p3[1] as f32 + p4[1] as f32) * 0.18;
                b = b * 0.28 + (p1[2] as f32 + p2[2] as f32 + p3[2] as f32 + p4[2] as f32) * 0.18;
            }

            if halation_amt > 0.001 || bloom_amt > 0.001 {
                let rad = 4i32;
                let s_l = (sx as i32 - rad).clamp(0, fw_u32 as i32 - 1) as u32;
                let s_r = (sx as i32 + rad).clamp(0, fw_u32 as i32 - 1) as u32;
                let s_u = (sy as i32 - rad).clamp(0, fh_u32 as i32 - 1) as u32;
                let s_d = (sy as i32 + rad).clamp(0, fh_u32 as i32 - 1) as u32;
                let p1 = frame.get_pixel(s_l, sy);
                let p2 = frame.get_pixel(s_r, sy);
                let p3 = frame.get_pixel(sx, s_u);
                let p4 = frame.get_pixel(sx, s_d);
                let nr = (p1[0] as f32 + p2[0] as f32 + p3[0] as f32 + p4[0] as f32) * 0.25;
                let ng = (p1[1] as f32 + p2[1] as f32 + p3[1] as f32 + p4[1] as f32) * 0.25;
                let nb = (p1[2] as f32 + p2[2] as f32 + p3[2] as f32 + p4[2] as f32) * 0.25;
                let n_luma = (0.2126 * nr + 0.7152 * ng + 0.0722 * nb) / 255.0;
                if halation_amt > 0.001 && n_luma > 0.65 {
                    let h_str = ((n_luma - 0.65) / 0.35).clamp(0.0, 1.0) * halation_amt * 110.0;
                    r = (r + h_str).clamp(0.0, 255.0);
                    g = (g + h_str * 0.28).clamp(0.0, 255.0);
                    b = (b + h_str * 0.06).clamp(0.0, 255.0);
                }
                if bloom_amt > 0.001 && n_luma > 0.58 {
                    let b_str = ((n_luma - 0.58) / 0.42).clamp(0.0, 1.0) * bloom_amt * 0.65;
                    r = (r + nr * b_str).clamp(0.0, 255.0);
                    g = (g + ng * b_str).clamp(0.0, 255.0);
                    b = (b + nb * b_str).clamp(0.0, 255.0);
                }
            }

            if sharpen_amt > 0.001 {
                let s_l = sx.saturating_sub(1);
                let s_r = (sx + 1).min(fw_u32 - 1);
                let s_u = sy.saturating_sub(1);
                let s_d = (sy + 1).min(fh_u32 - 1);
                let p1 = frame.get_pixel(s_l, sy);
                let p2 = frame.get_pixel(s_r, sy);
                let p3 = frame.get_pixel(sx, s_u);
                let p4 = frame.get_pixel(sx, s_d);
                let avg_r = (p1[0] as f32 + p2[0] as f32 + p3[0] as f32 + p4[0] as f32) * 0.25;
                let avg_g = (p1[1] as f32 + p2[1] as f32 + p3[1] as f32 + p4[1] as f32) * 0.25;
                let avg_b = (p1[2] as f32 + p2[2] as f32 + p3[2] as f32 + p4[2] as f32) * 0.25;
                r = (r + (r - avg_r) * sharpen_amt * 1.5).clamp(0.0, 255.0);
                g = (g + (g - avg_g) * sharpen_amt * 1.5).clamp(0.0, 255.0);
                b = (b + (b - avg_b) * sharpen_amt * 1.5).clamp(0.0, 255.0);
            }

            if luma_wipe_active {
                let luma_n = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
                let thresh = (1.0 - fade_factor) as f32;
                let luma_a = ((luma_n - (thresh - 0.15)) / 0.30).clamp(0.0, 1.0);
                a *= (fade_factor as f32 * 0.2 + luma_a * 0.8).clamp(0.0, 1.0);
            }

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

            let [mut gr, mut gg, mut gb] = apply_clip_grading_rgb(
                r,
                g,
                b,
                clip,
                has_clip_grading,
                &color_params,
                custom_lut.as_deref(),
            );

            if vignette_amt > 0.001 {
                let dx = (norm_x as f32 - 0.5) * 1.42;
                let dy = (norm_y - 0.5) * 1.42;
                let vig = (1.0 - (dx * dx + dy * dy) * vignette_amt * 1.15).clamp(0.0, 1.0);
                gr *= vig;
                gg *= vig;
                gb *= vig;
            }

            if grain_amt > 0.001 {
                let hash_v =
                    (((cx as f32 * 12.9898 + cy as f32 * 78.233 + clip_time as f32 * 37.0).sin()
                        * 43_758.547)
                        .fract()
                        - 0.5)
                        * grain_amt
                        * 48.0;
                gr = (gr + hash_v).clamp(0.0, 255.0);
                gg = (gg + hash_v).clamp(0.0, 255.0);
                gb = (gb + hash_v).clamp(0.0, 255.0);
            }

            r += (gr - r) * color_mask;
            g += (gg - g) * color_mask;
            b += (gb - b) * color_mask;

            if light_leak_active {
                let leak_amt = (1.0 - fade_factor) as f32;
                r = (r + (255.0 - r) * leak_amt * 0.75).clamp(0.0, 255.0);
                g = (g + (135.0 - g).max(0.0) * leak_amt * 0.45).clamp(0.0, 255.0);
            }

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

/// Inverse bilinear interpolation mapping a screen point `p` inside quad `(tl, tr, br, bl)` back to `[u, v]` in `[0..1]`.
#[inline]
pub fn invert_bilinear_quad(
    p: [f32; 2],
    tl: [f32; 2],
    tr: [f32; 2],
    br: [f32; 2],
    bl: [f32; 2],
) -> Option<[f32; 2]> {
    let mut u = 0.5_f32;
    let mut v = 0.5_f32;
    for _ in 0..5 {
        let qx = (1.0 - u) * (1.0 - v) * tl[0]
            + u * (1.0 - v) * tr[0]
            + u * v * br[0]
            + (1.0 - u) * v * bl[0]
            - p[0];
        let qy = (1.0 - u) * (1.0 - v) * tl[1]
            + u * (1.0 - v) * tr[1]
            + u * v * br[1]
            + (1.0 - u) * v * bl[1]
            - p[1];
        if qx.abs() < 1e-4 && qy.abs() < 1e-4 {
            break;
        }
        let dqx_du = (1.0 - v) * (tr[0] - tl[0]) + v * (br[0] - bl[0]);
        let dqx_dv = (1.0 - u) * (bl[0] - tl[0]) + u * (br[0] - tr[0]);
        let dqy_du = (1.0 - v) * (tr[1] - tl[1]) + v * (br[1] - bl[1]);
        let dqy_dv = (1.0 - u) * (bl[1] - tl[1]) + u * (br[1] - tr[1]);
        let det = dqx_du * dqy_dv - dqx_dv * dqy_du;
        if det.abs() < 1e-6 {
            return None;
        }
        u -= (dqy_dv * qx - dqx_dv * qy) / det;
        v -= (-dqy_du * qx + dqx_du * qy) / det;
    }
    Some([u, v])
}

/// Applies clip color wheels, color grade parameters, curves, HSL qualifier, and optional 3D LUT to an RGB pixel in `[0.0, 255.0]`.
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
    if clip.camera_log_profile != trim_core::CameraLogProfile::Rec709 {
        let [lr, lg, lb] = clip
            .camera_log_profile
            .apply_rgb(r / 255.0, g / 255.0, b / 255.0);
        r = lr * 255.0;
        g = lg * 255.0;
        b = lb * 255.0;
    }

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
        if let Some(curves) = &clip.custom_curves {
            if !curves.is_identity() {
                [nr, ng, nb] = curves.apply_rgb(nr, ng, nb);
            }
        }
        if let Some(hsl) = &clip.hsl_curves {
            if !hsl.is_identity() {
                [nr, ng, nb] = hsl.apply_rgb(nr, ng, nb);
            }
        }
        if let Some(qual) = &clip.hsl_qualifier {
            if qual.enabled {
                [nr, ng, nb] = qual.apply_rgb(nr, ng, nb);
            }
        }
        if clip.film_format != trim_core::FilmStockFormat::None || clip.highlight_rolloff > 1e-4 {
            [nr, ng, nb] = clip
                .film_format
                .apply_rgb(nr, ng, nb, clip.highlight_rolloff);
        }
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

/// Tracks a normalized bounding region `(center_x, center_y, scale_x, scale_y)` across a video clip
/// using block-matching SAD (Sum of Absolute Differences) on downscaled luma frames.
/// Returns a list of `(clip_relative_time, norm_center_x, norm_center_y)` samples suitable for mask keyframes.
#[allow(clippy::too_many_arguments)]
pub fn track_region_motion(
    video_path: &str,
    source_in: f64,
    duration: f64,
    speed: f64,
    init_cx: f32,
    init_cy: f32,
    scale_x: f32,
    scale_y: f32,
) -> Result<Vec<(f64, f64, f64)>, String> {
    let norm_path = trim_core::paths::normalize_path(video_path);
    if !norm_path.exists() {
        return Err("Kaynak video dosyası bulunamadı.".to_string());
    }

    let step_sec = (duration / 20.0).clamp(0.15, 0.5);
    let sample_count = ((duration / step_sec).ceil() as usize).clamp(2, 32);
    let mut decoder = VideoFrameDecoder::new();

    let mut samples = Vec::with_capacity(sample_count + 1);
    let mut cur_cx = init_cx.clamp(-0.45, 0.45);
    let mut cur_cy = init_cy.clamp(-0.45, 0.45);

    let mut prev_luma: Option<(Vec<u8>, u32, u32)> = None;

    for idx in 0..=sample_count {
        let clip_t = (idx as f64 * step_sec).min(duration);
        let src_t = (source_in + clip_t * speed.max(0.05)).max(0.0);
        let Some(frame) = decoder.decode_frame(video_path, src_t) else {
            continue;
        };
        let w = frame.width().max(1);
        let h = frame.height().max(1);
        let raw = frame.as_raw();
        let mut luma = Vec::with_capacity((w * h) as usize);
        for px in raw.as_chunks::<4>().0 {
            let y = ((px[0] as u32 * 77 + px[1] as u32 * 150 + px[2] as u32 * 29) >> 8) as u8;
            luma.push(y);
        }

        if let Some((ref ref_luma, rw, rh)) = prev_luma {
            if rw == w && rh == h {
                let patch_rx = ((scale_x.clamp(0.08, 0.5) * w as f32 * 0.35) as i32).clamp(6, 36);
                let patch_ry = ((scale_y.clamp(0.08, 0.5) * h as f32 * 0.35) as i32).clamp(6, 36);
                let prev_px = ((cur_cx + 0.5) * w as f32).round() as i32;
                let prev_py = ((cur_cy + 0.5) * h as f32).round() as i32;
                let search_r = 24i32;

                let mut best_dx = 0i32;
                let mut best_dy = 0i32;
                let mut best_sad = u64::MAX;

                for dy in (-search_r..=search_r).step_by(2) {
                    for dx in (-search_r..=search_r).step_by(2) {
                        let mut sad = 0u64;
                        let mut count = 0u64;
                        for py in (-patch_ry..=patch_ry).step_by(2) {
                            let r_y = prev_py + py;
                            let c_y = r_y + dy;
                            if r_y < 0 || r_y >= h as i32 || c_y < 0 || c_y >= h as i32 {
                                continue;
                            }
                            for px in (-patch_rx..=patch_rx).step_by(2) {
                                let r_x = prev_px + px;
                                let c_x = r_x + dx;
                                if r_x < 0 || r_x >= w as i32 || c_x < 0 || c_x >= w as i32 {
                                    continue;
                                }
                                let rv = ref_luma[(r_y as u32 * w + r_x as u32) as usize] as i32;
                                let cv = luma[(c_y as u32 * w + c_x as u32) as usize] as i32;
                                sad += (rv - cv).unsigned_abs() as u64;
                                count += 1;
                            }
                        }
                        if count > 8 {
                            let norm_sad = (sad * 1024) / count;
                            if norm_sad < best_sad {
                                best_sad = norm_sad;
                                best_dx = dx;
                                best_dy = dy;
                            }
                        }
                    }
                }

                // 1-pixel refinement around best_dx, best_dy
                let coarse_dx = best_dx;
                let coarse_dy = best_dy;
                for dy in (coarse_dy - 1)..=(coarse_dy + 1) {
                    for dx in (coarse_dx - 1)..=(coarse_dx + 1) {
                        let mut sad = 0u64;
                        let mut count = 0u64;
                        for py in (-patch_ry..=patch_ry).step_by(2) {
                            let r_y = prev_py + py;
                            let c_y = r_y + dy;
                            if r_y < 0 || r_y >= h as i32 || c_y < 0 || c_y >= h as i32 {
                                continue;
                            }
                            for px in (-patch_rx..=patch_rx).step_by(2) {
                                let r_x = prev_px + px;
                                let c_x = r_x + dx;
                                if r_x < 0 || r_x >= w as i32 || c_x < 0 || c_x >= w as i32 {
                                    continue;
                                }
                                let rv = ref_luma[(r_y as u32 * w + r_x as u32) as usize] as i32;
                                let cv = luma[(c_y as u32 * w + c_x as u32) as usize] as i32;
                                sad += (rv - cv).unsigned_abs() as u64;
                                count += 1;
                            }
                        }
                        if count > 8 {
                            let norm_sad = (sad * 1024) / count;
                            if norm_sad < best_sad {
                                best_sad = norm_sad;
                                best_dx = dx;
                                best_dy = dy;
                            }
                        }
                    }
                }

                cur_cx = (cur_cx + best_dx as f32 / w as f32).clamp(-0.48, 0.48);
                cur_cy = (cur_cy + best_dy as f32 / h as f32).clamp(-0.48, 0.48);
            }
        }

        samples.push((clip_t, cur_cx as f64, cur_cy as f64));
        prev_luma = Some((luma, w, h));
    }

    if samples.is_empty() {
        return Err("Hareket takibi için kare çözümlenemedi.".to_string());
    }
    Ok(samples)
}
