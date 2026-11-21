use super::{ActiveMaskOverlay, ActiveTextOverlay, MonitorAction};
use egui::{Color32, Pos2, Rect, Response, Stroke, Ui, Vec2};

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

pub(super) fn fit_aspect(max_w: f32, max_h: f32, aspect: f32) -> (f32, f32) {
    if max_w <= 0.0 || max_h <= 0.0 {
        return (100.0, 100.0);
    }
    let h_from_w = max_w / aspect;
    if h_from_w <= max_h {
        (max_w, h_from_w)
    } else {
        (max_h * aspect, max_h)
    }
}

pub(super) fn draw_corner_guides(painter: &egui::Painter, rect: Rect) {
    let guide_len = 10.0;
    let guide_stroke = Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 45));

    // Top-Left
    painter.line_segment(
        [
            rect.left_top(),
            Pos2::new(rect.left() + guide_len, rect.top()),
        ],
        guide_stroke,
    );
    painter.line_segment(
        [
            rect.left_top(),
            Pos2::new(rect.left(), rect.top() + guide_len),
        ],
        guide_stroke,
    );

    // Top-Right
    painter.line_segment(
        [
            rect.right_top(),
            Pos2::new(rect.right() - guide_len, rect.top()),
        ],
        guide_stroke,
    );
    painter.line_segment(
        [
            rect.right_top(),
            Pos2::new(rect.right(), rect.top() + guide_len),
        ],
        guide_stroke,
    );

    // Bottom-Left
    painter.line_segment(
        [
            rect.left_bottom(),
            Pos2::new(rect.left() + guide_len, rect.bottom()),
        ],
        guide_stroke,
    );
    painter.line_segment(
        [
            rect.left_bottom(),
            Pos2::new(rect.left(), rect.bottom() - guide_len),
        ],
        guide_stroke,
    );

    // Bottom-Right
    painter.line_segment(
        [
            rect.right_bottom(),
            Pos2::new(rect.right() - guide_len, rect.bottom()),
        ],
        guide_stroke,
    );
    painter.line_segment(
        [
            rect.right_bottom(),
            Pos2::new(rect.right(), rect.bottom() - guide_len),
        ],
        guide_stroke,
    );
}

pub(super) fn draw_button(painter: &egui::Painter, rect: Rect, text: &str, is_primary: bool) {
    let bg = if is_primary {
        Color32::from_rgb(0xe0, 0x7a, 0x38)
    } else {
        Color32::from_rgb(0x24, 0x24, 0x2b)
    };
    let text_col = Color32::WHITE;

    painter.rect_filled(rect, 4.0, bg);
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(
            1.0_f32,
            if is_primary {
                Color32::from_rgb(0xcb, 0x69, 0x29)
            } else {
                Color32::from_rgb(0x38, 0x38, 0x44)
            },
        ),
    );

    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(12.0),
        text_col,
    );
}

pub(super) fn draw_safe_zones(painter: &egui::Painter, rect: Rect, is_vertical: bool) {
    let action_safe = rect.shrink2(Vec2::new(rect.width() * 0.05, rect.height() * 0.05));
    let title_safe = rect.shrink2(Vec2::new(rect.width() * 0.10, rect.height() * 0.10));

    let stroke_action = Stroke::new(
        1.0_f32,
        Color32::from_rgba_premultiplied(0x38, 0xa0, 0xe8, 60),
    );
    let stroke_title = Stroke::new(
        1.0_f32,
        Color32::from_rgba_premultiplied(0xe0, 0xaa, 0x50, 70),
    );

    painter.rect_stroke(action_safe, 0.0, stroke_action);
    painter.rect_stroke(title_safe, 0.0, stroke_title);

    // Center crosshair
    let c = rect.center();
    let cross_len = 8.0;
    let cross_stroke = Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 60));
    painter.line_segment(
        [
            Pos2::new(c.x - cross_len, c.y),
            Pos2::new(c.x + cross_len, c.y),
        ],
        cross_stroke,
    );
    painter.line_segment(
        [
            Pos2::new(c.x, c.y - cross_len),
            Pos2::new(c.x, c.y + cross_len),
        ],
        cross_stroke,
    );

    if is_vertical {
        // Vertical Shorts / Reels / TikTok UI zones overlay
        // Right side buttons zone (likes, comments, share)
        let right_ui = Rect::from_min_max(
            Pos2::new(
                rect.right() - rect.width() * 0.18,
                rect.top() + rect.height() * 0.40,
            ),
            Pos2::new(rect.right(), rect.top() + rect.height() * 0.85),
        );
        painter.rect_filled(
            right_ui,
            2.0,
            Color32::from_rgba_premultiplied(240, 60, 60, 25),
        );
        painter.rect_stroke(
            right_ui,
            2.0,
            Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(240, 60, 60, 60)),
        );

        // Bottom captions zone
        let bottom_ui = Rect::from_min_max(
            Pos2::new(rect.left(), rect.bottom() - rect.height() * 0.20),
            Pos2::new(rect.right(), rect.bottom()),
        );
        painter.rect_filled(
            bottom_ui,
            2.0,
            Color32::from_rgba_premultiplied(240, 60, 60, 20),
        );
        painter.rect_stroke(
            bottom_ui,
            2.0,
            Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(240, 60, 60, 50)),
        );

        painter.text(
            Pos2::new(rect.left() + 6.0, rect.bottom() - 14.0),
            egui::Align2::LEFT_BOTTOM,
            "Reels/TikTok UI Alanı",
            egui::FontId::proportional(9.5),
            Color32::from_rgba_premultiplied(240, 180, 180, 140),
        );
    } else {
        // 2.39:1 Anamorphic Cinemascope letterbox safe-frame guides on landscape canvas
        let scope_h = (rect.width() / 2.39).min(rect.height());
        if scope_h + 4.0 < rect.height() {
            let bar_h = (rect.height() - scope_h) * 0.5;
            let top_y = rect.top() + bar_h;
            let bot_y = rect.bottom() - bar_h;
            let scope_stroke = Stroke::new(
                1.0_f32,
                Color32::from_rgba_premultiplied(0xb7, 0xe3, 0x6a, 110),
            );
            painter.line_segment(
                [
                    Pos2::new(rect.left(), top_y),
                    Pos2::new(rect.right(), top_y),
                ],
                scope_stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(rect.left(), bot_y),
                    Pos2::new(rect.right(), bot_y),
                ],
                scope_stroke,
            );
            painter.text(
                Pos2::new(rect.right() - 8.0, top_y - 3.0),
                egui::Align2::RIGHT_BOTTOM,
                "2.39:1 Sinemaskop",
                egui::FontId::monospace(8.5),
                Color32::from_rgba_premultiplied(0xb7, 0xe3, 0x6a, 160),
            );
        }
    }
}

#[derive(Clone, Debug)]
enum MaskHandleKind {
    Center,
    Corner,
    Rotation,
    BezierVertex(usize),
    BezierInTangent(usize),
    BezierOutTangent(usize),
}

#[derive(Clone, Debug)]
struct MaskDragState {
    mask_idx: usize,
    kind: MaskHandleKind,
    old_masks: Vec<trim_core::Mask>,
}

pub(super) fn handle_mask_overlay_interaction(
    ui: &mut Ui,
    painter: &egui::Painter,
    frame_rect: Rect,
    response: &Response,
    ov: &mut ActiveMaskOverlay,
    action: &mut Option<MonitorAction>,
) {
    if ov.masks.is_empty() || frame_rect.width() <= 1.0 || frame_rect.height() <= 1.0 {
        return;
    }

    let uv_to_screen = |u: f32, v: f32| -> Pos2 {
        Pos2::new(
            frame_rect.left() + u * frame_rect.width(),
            frame_rect.top() + v * frame_rect.height(),
        )
    };
    let screen_to_uv = |p: Pos2| -> [f32; 2] {
        [
            ((p.x - frame_rect.left()) / frame_rect.width()).clamp(0.0, 1.0),
            ((p.y - frame_rect.top()) / frame_rect.height()).clamp(0.0, 1.0),
        ]
    };

    let drag_state_id = ui.make_persistent_id(format!("monitor_mask_drag_{}", ov.clip_id));
    let mut drag_state: Option<MaskDragState> = ui.data_mut(|d| d.get_temp(drag_state_id));

    let mut hit_candidates: Vec<(usize, MaskHandleKind, Pos2)> = Vec::new();
    let aspect = frame_rect.width() / frame_rect.height().max(1.0);

    for (m_idx, mask) in ov.masks.iter().enumerate() {
        if !mask.enabled {
            continue;
        }
        let p = mask.eval_at(ov.clip_time);
        let col = if mask.target == trim_core::MaskTarget::ClipAlpha {
            Color32::from_rgb(0x2d, 0xd4, 0xbf)
        } else {
            Color32::from_rgb(0xf5, 0x9e, 0x0b)
        };
        let stroke = Stroke::new(1.5_f32, col);
        let rot = p.rotation_deg.to_radians();
        let (sin_r, cos_r) = rot.sin_cos();

        let local_to_uv = |lx: f32, ly: f32| -> [f32; 2] {
            let rx = lx * cos_r - ly * sin_r;
            let ry = lx * sin_r + ly * cos_r;
            [p.pos_x + rx / aspect, p.pos_y + ry]
        };

        match mask.shape {
            trim_core::MaskShape::Rectangle => {
                let hx = (p.scale_x * 0.5) * aspect;
                let hy = p.scale_y * 0.5;
                let corners_local = [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)];
                let pts: Vec<Pos2> = corners_local
                    .iter()
                    .map(|&(lx, ly)| {
                        let [u, v] = local_to_uv(lx, ly);
                        uv_to_screen(u, v)
                    })
                    .collect();
                painter.add(egui::Shape::closed_line(pts.clone(), stroke));
                for pt in &pts {
                    painter.rect_filled(Rect::from_center_size(*pt, Vec2::splat(7.0)), 1.0, col);
                    hit_candidates.push((m_idx, MaskHandleKind::Corner, *pt));
                }
                let [ru, rv] = local_to_uv(0.0, -hy - 0.06);
                let rot_pos = uv_to_screen(ru, rv);
                let top_mid = uv_to_screen(local_to_uv(0.0, -hy)[0], local_to_uv(0.0, -hy)[1]);
                painter.line_segment([top_mid, rot_pos], Stroke::new(1.0_f32, col));
                painter.circle_filled(rot_pos, 4.5, Color32::WHITE);
                painter.circle_stroke(rot_pos, 4.5, stroke);
                hit_candidates.push((m_idx, MaskHandleKind::Rotation, rot_pos));
            }
            trim_core::MaskShape::Ellipse => {
                let hx = (p.scale_x * 0.5) * aspect;
                let hy = p.scale_y * 0.5;
                let steps = 36;
                let mut pts = Vec::with_capacity(steps);
                for s in 0..steps {
                    let ang = (s as f32 / steps as f32) * std::f32::consts::TAU;
                    let [u, v] = local_to_uv(hx * ang.cos(), hy * ang.sin());
                    pts.push(uv_to_screen(u, v));
                }
                painter.add(egui::Shape::closed_line(pts, stroke));
                for &(lx, ly) in &[(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)] {
                    let [u, v] = local_to_uv(lx, ly);
                    let pt = uv_to_screen(u, v);
                    painter.rect_filled(Rect::from_center_size(pt, Vec2::splat(6.5)), 1.0, col);
                    hit_candidates.push((m_idx, MaskHandleKind::Corner, pt));
                }
                let [ru, rv] = local_to_uv(0.0, -hy - 0.06);
                let rot_pos = uv_to_screen(ru, rv);
                let [tu, tv] = local_to_uv(0.0, -hy);
                painter.line_segment([uv_to_screen(tu, tv), rot_pos], Stroke::new(1.0_f32, col));
                painter.circle_filled(rot_pos, 4.5, Color32::WHITE);
                painter.circle_stroke(rot_pos, 4.5, stroke);
                hit_candidates.push((m_idx, MaskHandleKind::Rotation, rot_pos));
            }
            trim_core::MaskShape::Bezier => {
                let poly = trim_core::mask::flatten_bezier_polygon(&mask.vertices, 12);
                if poly.len() >= 2 {
                    let pts: Vec<Pos2> = poly.iter().map(|uv| uv_to_screen(uv[0], uv[1])).collect();
                    painter.add(egui::Shape::closed_line(pts, stroke));
                }
                for (vi, v) in mask.vertices.iter().enumerate() {
                    let vp = uv_to_screen(v.pos[0], v.pos[1]);
                    let ip = uv_to_screen(v.pos[0] + v.in_tangent[0], v.pos[1] + v.in_tangent[1]);
                    let op = uv_to_screen(v.pos[0] + v.out_tangent[0], v.pos[1] + v.out_tangent[1]);
                    painter.line_segment([vp, ip], Stroke::new(1.0_f32, Color32::from_gray(170)));
                    painter.line_segment([vp, op], Stroke::new(1.0_f32, Color32::from_gray(170)));
                    painter.circle_filled(ip, 3.0, Color32::LIGHT_GRAY);
                    painter.circle_filled(op, 3.0, Color32::LIGHT_GRAY);
                    painter.circle_filled(vp, 4.5, col);
                    hit_candidates.push((m_idx, MaskHandleKind::BezierVertex(vi), vp));
                    hit_candidates.push((m_idx, MaskHandleKind::BezierInTangent(vi), ip));
                    hit_candidates.push((m_idx, MaskHandleKind::BezierOutTangent(vi), op));
                }
            }
        }

        let center_pos = uv_to_screen(p.pos_x, p.pos_y);
        painter.circle_filled(center_pos, 5.0, col);
        painter.circle_stroke(center_pos, 5.0, Stroke::new(1.0_f32, Color32::WHITE));
        hit_candidates.push((m_idx, MaskHandleKind::Center, center_pos));
    }

    if response.drag_started() {
        if let Some(ptr) = response.interact_pointer_pos() {
            let mut best: Option<(usize, MaskHandleKind, f32)> = None;
            for (m_idx, kind, hp) in &hit_candidates {
                let d = ptr.distance(*hp);
                if d <= 12.0 && best.as_ref().is_none_or(|b| d < b.2) {
                    best = Some((*m_idx, kind.clone(), d));
                }
            }
            if let Some((m_idx, kind, _)) = best {
                drag_state = Some(MaskDragState {
                    mask_idx: m_idx,
                    kind,
                    old_masks: ov.masks.clone(),
                });
                ui.data_mut(|d| d.insert_temp(drag_state_id, drag_state.clone()));
            }
        }
    }

    if response.dragged() {
        if let (Some(ref ds), Some(ptr)) = (&drag_state, response.interact_pointer_pos()) {
            let uv = screen_to_uv(ptr);
            let mut new_masks = ov.masks.clone();
            if let Some(m) = new_masks.get_mut(ds.mask_idx) {
                match ds.kind {
                    MaskHandleKind::Center => {
                        let dx = uv[0] as f64 - m.pos_x;
                        let dy = uv[1] as f64 - m.pos_y;
                        m.pos_x = uv[0] as f64;
                        m.pos_y = uv[1] as f64;
                        if m.shape == trim_core::MaskShape::Bezier {
                            for v in &mut m.vertices {
                                v.pos[0] += dx as f32;
                                v.pos[1] += dy as f32;
                            }
                        }
                    }
                    MaskHandleKind::Corner => {
                        let sx = ((uv[0] as f64 - m.pos_x).abs() * 2.0).clamp(0.02, 2.0);
                        let sy = ((uv[1] as f64 - m.pos_y).abs() * 2.0).clamp(0.02, 2.0);
                        m.scale_x = sx;
                        m.scale_y = sy;
                    }
                    MaskHandleKind::Rotation => {
                        let cp = uv_to_screen(m.pos_x as f32, m.pos_y as f32);
                        let ang = (ptr.y - cp.y).atan2(ptr.x - cp.x).to_degrees() + 90.0;
                        m.rotation = ang as f64;
                    }
                    MaskHandleKind::BezierVertex(vi) => {
                        if let Some(v) = m.vertices.get_mut(vi) {
                            v.pos = uv;
                        }
                    }
                    MaskHandleKind::BezierInTangent(vi) => {
                        if let Some(v) = m.vertices.get_mut(vi) {
                            v.in_tangent = [uv[0] - v.pos[0], uv[1] - v.pos[1]];
                        }
                    }
                    MaskHandleKind::BezierOutTangent(vi) => {
                        if let Some(v) = m.vertices.get_mut(vi) {
                            v.out_tangent = [uv[0] - v.pos[0], uv[1] - v.pos[1]];
                        }
                    }
                }
                *action = Some(MonitorAction::MasksEdited {
                    clip_id: ov.clip_id.clone(),
                    old_masks: ds.old_masks.clone(),
                    new_masks,
                    commit_undo: false,
                });
            }
        }
    }

    if response.drag_stopped() {
        if let Some(ds) = drag_state.take() {
            ui.data_mut(|d| d.remove::<Option<MaskDragState>>(drag_state_id));
            *action = Some(MonitorAction::MasksEdited {
                clip_id: ov.clip_id.clone(),
                old_masks: ds.old_masks,
                new_masks: ov.masks.clone(),
                commit_undo: true,
            });
        }
    }
}
