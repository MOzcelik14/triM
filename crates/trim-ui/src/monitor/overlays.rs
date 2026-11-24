use super::ActiveTextOverlay;
use egui::{Color32, Pos2, Rect, Stroke, Vec2};

pub fn draw_text_overlays_on_rect(
    painter: &egui::Painter,
    frame_rect: Rect,
    text_overlays: &[ActiveTextOverlay],
) -> Vec<(usize, Rect)> {
    let mut rendered = Vec::new();
    for (idx, ov) in text_overlays.iter().enumerate() {
        if ov.alpha <= 0.001 || ov.text.is_empty() {
            continue;
        }
        let scaled_font_size = (ov.font_size * (frame_rect.height() / 1080.0)).clamp(10.0, 160.0);
        let family = crate::theme::resolve_egui_font_family(&ov.font_family);
        let font = egui::FontId::new(scaled_font_size, family);
        let target_x = frame_rect.left() + frame_rect.width() * ov.pos_x.clamp(0.0, 1.0);
        let target_y = frame_rect.top() + frame_rect.height() * ov.pos_y.clamp(0.0, 1.0);

        let words: Vec<&str> = ov.text.split_whitespace().collect();
        let is_multi_word = words.len() > 1;
        let (active_idx, word_local_prog) = if is_multi_word {
            let weights: Vec<f32> = words
                .iter()
                .map(|w| (w.chars().count().max(2) as f32).powf(0.75))
                .collect();
            let total_w: f32 = weights.iter().sum::<f32>().max(1.0);
            let target_w = ov.progress.clamp(0.0, 0.9999) * total_w;
            let mut acc = 0.0_f32;
            let mut found_idx = words.len() - 1;
            let mut local_p = 0.0_f32;
            for (wi, &w_len) in weights.iter().enumerate() {
                if target_w <= acc + w_len || wi + 1 == weights.len() {
                    found_idx = wi;
                    local_p = ((target_w - acc) / w_len.max(0.001)).clamp(0.0, 1.0);
                    break;
                }
                acc += w_len;
            }
            (found_idx, local_p)
        } else {
            (0, ov.progress.clamp(0.0, 1.0))
        };

        let mut final_col = ov.text_color;
        if ov.alpha < 0.999 {
            final_col = Color32::from_rgba_premultiplied(
                (final_col.r() as f32 * ov.alpha) as u8,
                (final_col.g() as f32 * ov.alpha) as u8,
                (final_col.b() as f32 * ov.alpha) as u8,
                (final_col.a() as f32 * ov.alpha) as u8,
            );
        }

        let text_rect = match ov.motion_style {
            trim_core::clip::MotionTitleStyle::Typewriter => {
                let total_chars = ov.text.chars().count();
                let type_prog = (ov.progress / 0.75).clamp(0.0, 1.0);
                let visible_count = (type_prog * total_chars as f32).ceil() as usize;
                let mut display_text: String = ov.text.chars().take(visible_count).collect();
                let blink = ((ov.progress * 12.0) as u32).is_multiple_of(2);
                if blink && type_prog < 1.0 {
                    display_text.push('▎');
                }

                let galley = painter.layout_no_wrap(display_text, font, final_col);
                let tr = egui::Align2::CENTER_CENTER
                    .anchor_size(Pos2::new(target_x, target_y), galley.size());

                if let Some(bg_col) = ov.bg_color {
                    let pad = 6.0 * (frame_rect.height() / 1080.0).max(0.5);
                    painter.rect_filled(tr.expand(pad), 4.0, bg_col);
                }
                if ov.has_shadow {
                    painter.galley(
                        tr.min + Vec2::new(2.0, 2.0),
                        galley.clone(),
                        Color32::from_black_alpha((180.0 * ov.alpha) as u8),
                    );
                }
                painter.galley(tr.min, galley, final_col);
                tr
            }
            trim_core::clip::MotionTitleStyle::LowerThird => {
                let slide_factor = (ov.progress * 4.0).clamp(0.0, 1.0);
                let ease_in = 1.0 - (1.0 - slide_factor).powi(3);
                let slide_offset_x = (1.0 - ease_in) * -120.0;
                let cur_x = target_x + slide_offset_x;

                let main_font = egui::FontId::proportional(scaled_font_size * 1.05);
                let main_galley = painter.layout_no_wrap(ov.text.clone(), main_font, final_col);

                let sub_str = ov.subtitle_text.clone().unwrap_or_default();
                let sub_font = egui::FontId::proportional(scaled_font_size * 0.70);
                let sub_col = Color32::from_rgba_premultiplied(
                    (210.0 * ov.alpha) as u8,
                    (220.0 * ov.alpha) as u8,
                    (235.0 * ov.alpha) as u8,
                    (255.0 * ov.alpha) as u8,
                );
                let sub_galley = painter.layout_no_wrap(sub_str.clone(), sub_font, sub_col);

                let has_sub = !sub_str.is_empty();
                let content_w = main_galley.size().x.max(sub_galley.size().x) + 26.0;
                let content_h = if has_sub {
                    main_galley.size().y + sub_galley.size().y + 10.0
                } else {
                    main_galley.size().y + 12.0
                };

                let box_rect = Rect::from_min_size(
                    Pos2::new(cur_x, target_y - content_h * 0.5),
                    Vec2::new(content_w, content_h),
                );

                let bg_box = ov
                    .bg_color
                    .unwrap_or(Color32::from_rgba_unmultiplied(16, 20, 30, 220));
                painter.rect_filled(box_rect, 4.0, bg_box);

                let bar_w = 4.0 * (frame_rect.height() / 1080.0).max(1.0);
                let bar_rect = Rect::from_min_max(
                    box_rect.min,
                    Pos2::new(box_rect.left() + bar_w, box_rect.bottom()),
                );
                painter.rect_filled(bar_rect, 2.0, ov.highlight_color);

                let text_left = box_rect.left() + bar_w + 8.0;
                let main_pos = Pos2::new(text_left, box_rect.top() + 4.0);
                if ov.has_shadow {
                    painter.galley(
                        main_pos + Vec2::new(1.5, 1.5),
                        main_galley.clone(),
                        Color32::from_black_alpha((180.0 * ov.alpha) as u8),
                    );
                }
                painter.galley(main_pos, main_galley, final_col);

                if has_sub {
                    let sub_pos =
                        Pos2::new(text_left, box_rect.bottom() - sub_galley.size().y - 4.0);
                    painter.galley(sub_pos, sub_galley, sub_col);
                }

                box_rect
            }
            trim_core::clip::MotionTitleStyle::CinematicZoom => {
                let zoom = 0.94 + 0.10 * ov.progress;
                let zoom_font = egui::FontId::proportional(scaled_font_size * zoom);
                let galley = painter.layout_no_wrap(ov.text.clone(), zoom_font, final_col);
                let tr = egui::Align2::CENTER_CENTER
                    .anchor_size(Pos2::new(target_x, target_y), galley.size());

                if let Some(bg_col) = ov.bg_color {
                    let pad = 8.0 * (frame_rect.height() / 1080.0).max(0.5);
                    painter.rect_filled(tr.expand(pad), 6.0, bg_col);
                }
                if ov.has_shadow {
                    painter.galley(
                        tr.min + Vec2::new(2.5, 2.5),
                        galley.clone(),
                        Color32::from_black_alpha((180.0 * ov.alpha) as u8),
                    );
                }
                painter.galley(tr.min, galley, final_col);
                tr
            }
            trim_core::clip::MotionTitleStyle::Ticker => {
                let galley = painter.layout_no_wrap(ov.text.clone(), font.clone(), final_col);
                let text_w = galley.size().x;

                let bar_h = (scaled_font_size * 1.6).max(28.0);
                let bar_rect = Rect::from_min_max(
                    Pos2::new(frame_rect.left(), target_y - bar_h * 0.5),
                    Pos2::new(frame_rect.right(), target_y + bar_h * 0.5),
                );

                let banner_bg = ov
                    .bg_color
                    .unwrap_or(Color32::from_rgba_unmultiplied(12, 14, 20, 230));
                painter.rect_filled(bar_rect, 0.0, banner_bg);

                let badge_label = ov
                    .subtitle_text
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .unwrap_or("BİLGİ");
                let badge_font = egui::FontId::proportional((scaled_font_size * 0.72).max(10.0));
                let badge_galley =
                    painter.layout_no_wrap(badge_label.to_string(), badge_font, Color32::WHITE);
                let min_badge_w = 72.0 * (frame_rect.height() / 1080.0).max(0.65);
                let badge_w = (badge_galley.size().x + 22.0).max(min_badge_w);
                let badge_rect = Rect::from_min_max(
                    bar_rect.min,
                    Pos2::new(bar_rect.left() + badge_w, bar_rect.bottom()),
                );

                let ticker_clip_rect = Rect::from_min_max(
                    Pos2::new(
                        (badge_rect.right() + 8.0).min(bar_rect.right()),
                        bar_rect.top(),
                    ),
                    bar_rect.max,
                );
                let scroll_distance = ticker_clip_rect.width() + text_w;
                let cur_x = ticker_clip_rect.right() - (ov.progress * scroll_distance);
                let text_pos = Pos2::new(cur_x, target_y - galley.size().y * 0.5);
                painter
                    .with_clip_rect(ticker_clip_rect)
                    .galley(text_pos, galley, final_col);

                painter.rect_filled(badge_rect, 0.0, Color32::from_rgb(220, 38, 38));
                painter.galley(
                    badge_rect.center() - badge_galley.size() * 0.5,
                    badge_galley,
                    Color32::WHITE,
                );
                bar_rect
            }
            trim_core::clip::MotionTitleStyle::BoxReveal => {
                let galley = painter.layout_no_wrap(ov.text.clone(), font, final_col);
                let tr = egui::Align2::CENTER_CENTER
                    .anchor_size(Pos2::new(target_x, target_y), galley.size());

                let pad = 8.0 * (frame_rect.height() / 1080.0).max(0.5);
                let full_box = tr.expand(pad);

                let unfold = (ov.progress * 3.0).clamp(0.0, 1.0);
                let ease_w = full_box.width() * unfold;
                let animated_box =
                    Rect::from_center_size(full_box.center(), Vec2::new(ease_w, full_box.height()));

                let bg_col = ov
                    .bg_color
                    .unwrap_or(Color32::from_rgba_unmultiplied(20, 24, 34, 230));
                painter.rect_filled(animated_box, 4.0, bg_col);
                painter.rect_stroke(animated_box, 4.0, Stroke::new(1.5_f32, ov.highlight_color));

                if unfold >= 0.7 {
                    let text_alpha = ((unfold - 0.7) / 0.3 * ov.alpha).clamp(0.0, 1.0);
                    let text_col = Color32::from_rgba_premultiplied(
                        (final_col.r() as f32 * text_alpha) as u8,
                        (final_col.g() as f32 * text_alpha) as u8,
                        (final_col.b() as f32 * text_alpha) as u8,
                        (final_col.a() as f32 * text_alpha) as u8,
                    );
                    painter.galley(tr.min, galley, text_col);
                }
                full_box
            }
            trim_core::clip::MotionTitleStyle::RollingCredits => {
                let galley = painter.layout_no_wrap(ov.text.clone(), font.clone(), final_col);
                let total_h = galley.size().y + 32.0;
                let scroll_y = frame_rect.bottom()
                    - ov.progress.clamp(0.0, 1.0) * (frame_rect.height() + total_h);
                let tr = egui::Align2::CENTER_TOP
                    .anchor_size(Pos2::new(target_x, scroll_y), galley.size());
                let clipped = painter.with_clip_rect(frame_rect);
                if ov.has_shadow {
                    clipped.galley(
                        tr.min + Vec2::new(2.0, 2.0),
                        galley.clone(),
                        Color32::from_black_alpha((180.0 * ov.alpha) as u8),
                    );
                }
                clipped.galley(tr.min, galley, final_col);
                if let Some(sub) = &ov.subtitle_text {
                    if !sub.trim().is_empty() {
                        let sub_font = egui::FontId::new(
                            (scaled_font_size * 0.62).max(9.0),
                            font.family.clone(),
                        );
                        let sub_galley = painter.layout_no_wrap(sub.clone(), sub_font, final_col);
                        let sub_tr = egui::Align2::CENTER_TOP
                            .anchor_size(Pos2::new(target_x, tr.bottom() + 6.0), sub_galley.size());
                        clipped.galley(sub_tr.min, sub_galley, final_col);
                    }
                }
                tr
            }
            trim_core::clip::MotionTitleStyle::ChapterCard => {
                let galley = painter.layout_no_wrap(ov.text.clone(), font.clone(), final_col);
                let tr = egui::Align2::CENTER_CENTER
                    .anchor_size(Pos2::new(target_x, target_y), galley.size());
                let rule_w = (galley.size().x * 1.15).max(80.0);
                let rule_col = Color32::from_rgba_premultiplied(
                    ov.highlight_color.r(),
                    ov.highlight_color.g(),
                    ov.highlight_color.b(),
                    (190.0 * ov.alpha) as u8,
                );
                painter.line_segment(
                    [
                        Pos2::new(target_x - rule_w * 0.5, tr.top() - 8.0),
                        Pos2::new(target_x + rule_w * 0.5, tr.top() - 8.0),
                    ],
                    Stroke::new(1.5_f32, rule_col),
                );
                if ov.has_shadow {
                    painter.galley(
                        tr.min + Vec2::new(2.0, 2.0),
                        galley.clone(),
                        Color32::from_black_alpha((180.0 * ov.alpha) as u8),
                    );
                }
                painter.galley(tr.min, galley, final_col);
                let mut bottom_y = tr.bottom() + 8.0;
                if let Some(sub) = &ov.subtitle_text {
                    if !sub.trim().is_empty() {
                        let sub_font = egui::FontId::new(
                            (scaled_font_size * 0.55).max(9.0),
                            font.family.clone(),
                        );
                        let sub_galley = painter.layout_no_wrap(sub.clone(), sub_font, final_col);
                        let sub_tr = egui::Align2::CENTER_TOP
                            .anchor_size(Pos2::new(target_x, tr.bottom() + 6.0), sub_galley.size());
                        painter.galley(sub_tr.min, sub_galley, final_col);
                        bottom_y = sub_tr.bottom() + 8.0;
                    }
                }
                painter.line_segment(
                    [
                        Pos2::new(target_x - rule_w * 0.5, bottom_y),
                        Pos2::new(target_x + rule_w * 0.5, bottom_y),
                    ],
                    Stroke::new(1.5_f32, rule_col),
                );
                tr.expand(12.0)
            }
            trim_core::clip::MotionTitleStyle::None => match ov.caption_style {
                trim_core::clip::CaptionStyle::PopUpBouncy => {
                    let word_spring = (1.0 - (word_local_prog * 2.0 - 1.0).abs()).max(0.0);
                    let mut job = egui::text::LayoutJob::default();
                    for (i, word) in words.iter().enumerate() {
                        let is_act = i == active_idx;
                        let col = if is_act {
                            ov.highlight_color
                        } else {
                            final_col
                        };
                        let f = if is_act {
                            egui::FontId::proportional(
                                scaled_font_size * (1.14 + 0.14 * word_spring),
                            )
                        } else {
                            font.clone()
                        };
                        job.append(
                            word,
                            0.0,
                            egui::TextFormat {
                                font_id: f,
                                color: col,
                                ..Default::default()
                            },
                        );
                        if i + 1 < words.len() {
                            job.append(
                                " ",
                                0.0,
                                egui::TextFormat {
                                    font_id: font.clone(),
                                    color: final_col,
                                    ..Default::default()
                                },
                            );
                        }
                    }
                    let galley = painter.layout_job(job);
                    let tr = egui::Align2::CENTER_CENTER
                        .anchor_size(Pos2::new(target_x, target_y), galley.size());

                    if let Some(bg_col) = ov.bg_color {
                        let pad = 6.0 * (frame_rect.height() / 1080.0).max(0.5);
                        painter.rect_filled(tr.expand(pad), 6.0, bg_col);
                    }
                    if ov.has_shadow {
                        painter.galley(
                            tr.min + Vec2::new(2.0, 2.0),
                            galley.clone(),
                            Color32::from_black_alpha((180.0 * ov.alpha) as u8),
                        );
                    }
                    painter.galley(tr.min, galley, ov.highlight_color);
                    tr
                }
                trim_core::clip::CaptionStyle::WordHighlightYellow
                | trim_core::clip::CaptionStyle::WordHighlightNeon => {
                    let (hl_col, pill_bg) = if ov.caption_style
                        == trim_core::clip::CaptionStyle::WordHighlightYellow
                    {
                        (
                            Color32::from_rgb(255, 230, 0),
                            Color32::from_rgba_unmultiplied(45, 38, 0, (175.0 * ov.alpha) as u8),
                        )
                    } else {
                        (
                            Color32::from_rgb(0, 255, 102),
                            Color32::from_rgba_unmultiplied(0, 48, 22, (175.0 * ov.alpha) as u8),
                        )
                    };
                    let word_spring = (1.0 - (word_local_prog * 2.0 - 1.0).abs()).max(0.0);

                    let mut job = egui::text::LayoutJob::default();
                    for (i, word) in words.iter().enumerate() {
                        let is_act = i == active_idx;
                        let col = if is_act { hl_col } else { final_col };
                        let f = if is_act {
                            egui::FontId::proportional(
                                scaled_font_size * (1.10 + 0.08 * word_spring),
                            )
                        } else {
                            font.clone()
                        };
                        job.append(
                            word,
                            0.0,
                            egui::TextFormat {
                                font_id: f,
                                color: col,
                                background: if is_act {
                                    pill_bg
                                } else {
                                    Color32::TRANSPARENT
                                },
                                ..Default::default()
                            },
                        );
                        if i + 1 < words.len() {
                            job.append(
                                " ",
                                0.0,
                                egui::TextFormat {
                                    font_id: font.clone(),
                                    color: final_col,
                                    ..Default::default()
                                },
                            );
                        }
                    }
                    let galley = painter.layout_job(job);
                    let tr = egui::Align2::CENTER_CENTER
                        .anchor_size(Pos2::new(target_x, target_y), galley.size());

                    if let Some(bg_col) = ov.bg_color {
                        let pad = 6.0 * (frame_rect.height() / 1080.0).max(0.5);
                        painter.rect_filled(tr.expand(pad), 4.0, bg_col);
                    }
                    if ov.has_shadow {
                        painter.galley(
                            tr.min + Vec2::new(2.0, 2.0),
                            galley.clone(),
                            Color32::from_black_alpha((180.0 * ov.alpha) as u8),
                        );
                    }
                    painter.galley(tr.min, galley, final_col);
                    tr
                }
                trim_core::clip::CaptionStyle::BoxBackground
                | trim_core::clip::CaptionStyle::Standard => {
                    let galley = if ov.letter_spacing.abs() > 0.1 {
                        let mut job = egui::text::LayoutJob::default();
                        let scale_ls = ov.letter_spacing * (frame_rect.height() / 1080.0).max(0.35);
                        let chars: Vec<char> = ov.text.chars().collect();
                        for (ci, ch) in chars.iter().enumerate() {
                            let s = ch.to_string();
                            job.append(
                                &s,
                                if ci == 0 { 0.0 } else { scale_ls },
                                egui::TextFormat {
                                    font_id: font.clone(),
                                    color: final_col,
                                    ..Default::default()
                                },
                            );
                        }
                        painter.layout_job(job)
                    } else {
                        painter.layout_no_wrap(ov.text.clone(), font, final_col)
                    };
                    let tr = egui::Align2::CENTER_CENTER
                        .anchor_size(Pos2::new(target_x, target_y), galley.size());

                    if let Some(bg_col) = ov.bg_color {
                        let pad = 6.0 * (frame_rect.height() / 1080.0).max(0.5);
                        let mut box_col = bg_col;
                        if ov.alpha < 0.999 {
                            box_col = Color32::from_rgba_premultiplied(
                                (box_col.r() as f32 * ov.alpha) as u8,
                                (box_col.g() as f32 * ov.alpha) as u8,
                                (box_col.b() as f32 * ov.alpha) as u8,
                                (box_col.a() as f32 * ov.alpha) as u8,
                            );
                        }
                        painter.rect_filled(tr.expand(pad), 4.0, box_col);
                    } else if ov.caption_style == trim_core::clip::CaptionStyle::BoxBackground {
                        let pad = 6.0 * (frame_rect.height() / 1080.0).max(0.5);
                        painter.rect_filled(
                            tr.expand(pad),
                            4.0,
                            Color32::from_rgba_unmultiplied(0, 0, 0, 180),
                        );
                    }

                    if ov.has_shadow {
                        let sd = (ov.shadow_distance * (frame_rect.height() / 540.0).max(0.5))
                            .clamp(0.5, 24.0);
                        let shadow_offset = Vec2::new(sd, sd);
                        painter.galley(
                            tr.min + shadow_offset,
                            galley.clone(),
                            Color32::from_black_alpha((160.0 * ov.alpha) as u8),
                        );
                    }
                    if ov.stroke_width > 0.1 {
                        let sw = (ov.stroke_width * (frame_rect.height() / 540.0).max(0.5))
                            .clamp(0.8, 12.0);
                        let sc = Color32::from_rgba_premultiplied(
                            (ov.stroke_color.r() as f32 * ov.alpha) as u8,
                            (ov.stroke_color.g() as f32 * ov.alpha) as u8,
                            (ov.stroke_color.b() as f32 * ov.alpha) as u8,
                            (255.0 * ov.alpha) as u8,
                        );
                        for (dx, dy) in [
                            (-sw, 0.0),
                            (sw, 0.0),
                            (0.0, -sw),
                            (0.0, sw),
                            (-sw * 0.7, -sw * 0.7),
                            (sw * 0.7, -sw * 0.7),
                            (-sw * 0.7, sw * 0.7),
                            (sw * 0.7, sw * 0.7),
                        ] {
                            painter.galley(tr.min + Vec2::new(dx, dy), galley.clone(), sc);
                        }
                    }
                    painter.galley(tr.min, galley, final_col);
                    tr
                }
            },
        };

        rendered.push((idx, text_rect));
    }
    rendered
}
