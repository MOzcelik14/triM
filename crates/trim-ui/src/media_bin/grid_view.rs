use super::render_move_to_bin_menu;
use super::types::*;
use egui::{Color32, Pos2, Rect, ScrollArea, Sense, Stroke, TextureHandle, Ui, Vec2};
use std::collections::HashMap;
use trim_core::media::MediaType;
use trim_core::project::Project;

#[allow(clippy::too_many_arguments)]
pub(super) fn render_media_grid(
    ui: &mut Ui,
    project: &Project,
    selected_media_id: &mut Option<String>,
    thumb_size: f32,
    textures: &HashMap<String, TextureHandle>,
    matching_bins: &[&trim_core::project::MediaBinFolder],
    matching_sequences: &[&trim_core::sequence::Sequence],
    filtered_items: &[(&String, &trim_core::media::MediaItem)],
    media_usage_counts: &HashMap<&str, usize>,
    action: &mut Option<MediaBinAction>,
) {
    let card_w = thumb_size;
    let card_h = card_w * 0.5625 + 38.0; // 16:9 thumbnail + 38px text header/specs

    ScrollArea::vertical().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);

            // --- 7A. Folder Cards (Grid) ---
            for folder in matching_bins {
                let count = project
                    .media_pool
                    .values()
                    .filter(|m| m.bin_id.as_deref() == Some(&folder.id))
                    .count();
                let (card_rect, card_resp) =
                    ui.allocate_exact_size(Vec2::new(card_w, card_h), Sense::click());
                let painter = ui.painter_at(card_rect);

                let bg = if card_resp.hovered() {
                    Color32::from_rgb(0x28, 0x24, 0x1c)
                } else {
                    Color32::from_rgb(0x1a, 0x18, 0x14)
                };
                painter.rect_filled(card_rect, 3.0, bg);
                painter.rect_stroke(
                    card_rect,
                    3.0,
                    Stroke::new(
                        1.0_f32,
                        if card_resp.hovered() {
                            Color32::from_rgb(0xf5, 0x9e, 0x0b)
                        } else {
                            Color32::from_rgb(0x3a, 0x32, 0x24)
                        },
                    ),
                );

                // Folder top area
                let thumb_h = card_w * 0.5625;
                let thumb_rect = Rect::from_min_size(card_rect.min, Vec2::new(card_w, thumb_h));
                painter.rect_filled(thumb_rect, 2.0, Color32::from_rgb(0x14, 0x12, 0x10));

                // Big folder symbol
                painter.text(
                    thumb_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "⊞",
                    egui::FontId::proportional(26.0),
                    Color32::from_rgb(0xfb, 0xbf, 0x24),
                );

                // Item count badge
                let count_str = format!("{} öge", count);
                let badge_rect = Rect::from_min_max(
                    Pos2::new(thumb_rect.right() - 44.0, thumb_rect.bottom() - 15.0),
                    Pos2::new(thumb_rect.right() - 3.0, thumb_rect.bottom() - 3.0),
                );
                painter.rect_filled(
                    badge_rect,
                    2.0,
                    Color32::from_rgba_premultiplied(0, 0, 0, 180),
                );
                painter.text(
                    badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    count_str,
                    egui::FontId::monospace(8.5),
                    Color32::from_rgb(0xfb, 0xbf, 0x24),
                );

                // Folder name
                let text_y = thumb_rect.bottom() + 3.0;
                let mut display_name = folder.name.clone();
                if display_name.chars().count() > 16 {
                    display_name = display_name.chars().take(14).collect();
                    display_name.push('…');
                }
                painter.text(
                    Pos2::new(card_rect.left() + 4.0, text_y),
                    egui::Align2::LEFT_TOP,
                    display_name,
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(0xf8, 0xfa, 0xfc),
                );

                painter.text(
                    Pos2::new(card_rect.left() + 4.0, card_rect.bottom() - 3.0),
                    egui::Align2::LEFT_BOTTOM,
                    "Klasör (Bin)",
                    egui::FontId::monospace(8.5),
                    Color32::from_rgb(0x94, 0xa3, 0xb8),
                );

                if card_resp.double_clicked() {
                    *action = Some(MediaBinAction::EnterBin(Some(folder.id.clone())));
                }

                card_resp.context_menu(|ui| {
                    ui.set_min_width(200.0);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    ui.spacing_mut().button_padding = Vec2::new(8.0, 4.0);

                    if ui.button("↗ Klasörü Aç (Çift Tık)").clicked() {
                        *action = Some(MediaBinAction::EnterBin(Some(folder.id.clone())));
                        ui.close_menu();
                    }
                    if ui.button("✎ Yeniden Adlandır...").clicked() {
                        *action = Some(MediaBinAction::RenameBinPrompt(folder.id.clone()));
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("✕ Klasörü Sil").clicked() {
                        *action = Some(MediaBinAction::DeleteBin(folder.id.clone()));
                        ui.close_menu();
                    }
                });
            }

            // --- 7A.2. Sequence Cards (Grid) ---
            for seq in matching_sequences {
                let is_active = project.active_sequence_id.as_deref() == Some(&seq.id);
                let (card_rect, card_resp) =
                    ui.allocate_exact_size(Vec2::new(card_w, card_h), Sense::click_and_drag());
                let painter = ui.painter_at(card_rect);

                let bg = if is_active {
                    Color32::from_rgb(0x28, 0x24, 0x3c)
                } else if card_resp.hovered() {
                    Color32::from_rgb(0x22, 0x1e, 0x30)
                } else {
                    Color32::from_rgb(0x18, 0x16, 0x22)
                };
                painter.rect_filled(card_rect, 3.0, bg);
                painter.rect_stroke(
                    card_rect,
                    3.0,
                    Stroke::new(
                        1.0_f32,
                        if is_active {
                            Color32::from_rgb(0x8b, 0x5c, 0xf6)
                        } else if card_resp.hovered() {
                            Color32::from_rgb(0xa7, 0x8b, 0xfa)
                        } else {
                            Color32::from_rgb(0x38, 0x30, 0x48)
                        },
                    ),
                );

                // Top thumbnail area
                let thumb_h = card_w * 0.5625;
                let thumb_rect = Rect::from_min_size(card_rect.min, Vec2::new(card_w, thumb_h));
                painter.rect_filled(thumb_rect, 2.0, Color32::from_rgb(0x12, 0x10, 0x18));

                // Sequence symbol ≡
                painter.text(
                    thumb_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "≡",
                    egui::FontId::proportional(26.0),
                    Color32::from_rgb(0xa7, 0x8b, 0xfa),
                );

                // Duration badge in lower-right
                let dur_sec = seq.timeline.duration();
                let dur_str = if dur_sec >= 3600.0 {
                    format!(
                        "{:02}:{:02}:{:02}",
                        (dur_sec / 3600.0) as u32,
                        ((dur_sec % 3600.0) / 60.0) as u32,
                        (dur_sec % 60.0) as u32
                    )
                } else {
                    format!(
                        "{:02}:{:02}",
                        (dur_sec / 60.0) as u32,
                        (dur_sec % 60.0) as u32
                    )
                };
                let dur_badge_rect = Rect::from_min_max(
                    Pos2::new(thumb_rect.right() - 44.0, thumb_rect.bottom() - 14.0),
                    Pos2::new(thumb_rect.right() - 2.0, thumb_rect.bottom() - 2.0),
                );
                painter.rect_filled(
                    dur_badge_rect,
                    2.0,
                    Color32::from_rgba_premultiplied(0, 0, 0, 200),
                );
                painter.text(
                    dur_badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    dur_str,
                    egui::FontId::monospace(8.5),
                    Color32::from_rgb(0xa7, 0x8b, 0xfa),
                );

                // Sequence name
                let text_y = thumb_rect.bottom() + 3.0;
                let mut display_name = seq.name.clone();
                if display_name.chars().count() > 16 {
                    display_name = display_name.chars().take(14).collect();
                    display_name.push('…');
                }
                painter.text(
                    Pos2::new(card_rect.left() + 4.0, text_y),
                    egui::Align2::LEFT_TOP,
                    display_name,
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(0xf3, 0xf0, 0xff),
                );

                // Specs below name
                painter.text(
                    Pos2::new(card_rect.left() + 4.0, card_rect.bottom() - 3.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("{} kanal", seq.timeline.tracks.len()),
                    egui::FontId::monospace(8.5),
                    Color32::from_rgb(0x9d, 0x94, 0xb8),
                );

                // Interactions
                if card_resp.clicked() {
                    *selected_media_id = Some(format!("seq:{}", seq.id));
                }
                if card_resp.double_clicked() {
                    *action = Some(MediaBinAction::OpenSequence(seq.id.clone()));
                }
                if card_resp.dragged() {
                    card_resp.dnd_set_drag_payload(format!("seq:{}", seq.id));
                }

                // Context menu
                card_resp.context_menu(|ui| {
                    ui.set_min_width(220.0);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    ui.spacing_mut().button_padding = Vec2::new(8.0, 4.0);

                    if ui.button("Zaman Çizgisinde Aç (Çift Tık)").clicked() {
                        *action = Some(MediaBinAction::OpenSequence(seq.id.clone()));
                        ui.close_menu();
                    }
                    if ui.button("Zaman Çizgisine İç İçe Ekle (Nest)").clicked() {
                        *action = Some(MediaBinAction::AddToTimeline(format!("seq:{}", seq.id)));
                        ui.close_menu();
                    }
                    if ui.button("+ Sekansı Çoğalt (Duplicate)").clicked() {
                        *action = Some(MediaBinAction::DuplicateSequence(seq.id.clone()));
                        ui.close_menu();
                    }
                    if project.sequences.len() > 1 {
                        ui.separator();
                        if ui.button("✕ Sekansı Sil").clicked() {
                            *action = Some(MediaBinAction::DeleteSequence(seq.id.clone()));
                            ui.close_menu();
                        }
                    }
                });
            }

            // --- 7B. Media Cards (Grid) ---
            for &(m_id, item) in filtered_items {
                let is_selected = selected_media_id.as_deref() == Some(m_id.as_str());
                let (card_rect, card_resp) =
                    ui.allocate_exact_size(Vec2::new(card_w, card_h), Sense::click_and_drag());
                let painter = ui.painter_at(card_rect);

                // Card background
                let bg = if is_selected {
                    Color32::from_rgb(0x28, 0x38, 0x50)
                } else if card_resp.hovered() {
                    Color32::from_rgb(0x25, 0x25, 0x30)
                } else {
                    Color32::from_rgb(0x18, 0x18, 0x20)
                };

                painter.rect_filled(card_rect, 3.0, bg);
                painter.rect_stroke(
                    card_rect,
                    3.0,
                    Stroke::new(
                        if is_selected { 1.5_f32 } else { 1.0_f32 },
                        if is_selected {
                            Color32::from_rgb(0x38, 0x88, 0xe8)
                        } else {
                            Color32::from_rgb(0x2a, 0x2a, 0x34)
                        },
                    ),
                );

                // Thumbnail image area (16:9 ratio)
                let thumb_h = card_w * 0.5625;
                let thumb_rect = Rect::from_min_size(card_rect.min, Vec2::new(card_w, thumb_h));
                painter.rect_filled(thumb_rect, 2.0, Color32::from_rgb(0x0e, 0x0e, 0x12));

                if let Some(tex) = textures.get(m_id) {
                    painter.image(
                        tex.id(),
                        thumb_rect,
                        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    // Fallback placeholder
                    let label = match item.media_type {
                        MediaType::Video => "VİDEO",
                        MediaType::Audio => "SES",
                        MediaType::Image => "RESİM",
                        MediaType::Sequence => "SEKANS",
                        MediaType::Unknown => "DOSYA",
                    };
                    painter.text(
                        thumb_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        label,
                        egui::FontId::proportional(12.0),
                        Color32::from_rgb(0x70, 0x70, 0x85),
                    );
                }

                // Top-Left media type badge
                let (type_code, badge_bg) = match item.media_type {
                    MediaType::Video => ("VID", Color32::from_rgb(0x28, 0x5a, 0x9e)),
                    MediaType::Audio => ("SES", Color32::from_rgb(0x28, 0x80, 0x50)),
                    MediaType::Image => ("IMG", Color32::from_rgb(0x9e, 0x5a, 0x28)),
                    MediaType::Sequence => ("SEK", Color32::from_rgb(0x7c, 0x3a, 0xed)),
                    MediaType::Unknown => ("DOS", Color32::from_rgb(0x40, 0x40, 0x48)),
                };
                let badge_rect = Rect::from_min_size(
                    Pos2::new(thumb_rect.left() + 3.0, thumb_rect.top() + 3.0),
                    Vec2::new(22.0, 13.0),
                );
                painter.rect_filled(badge_rect, 2.0, badge_bg);
                painter.text(
                    badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    type_code,
                    egui::FontId::monospace(8.5),
                    Color32::WHITE,
                );

                // Top-Right timeline usage count badge (Nx)
                let use_cnt = media_usage_counts.get(m_id.as_str()).copied().unwrap_or(0);
                if use_cnt > 0 {
                    let u_rect = Rect::from_min_size(
                        Pos2::new(thumb_rect.right() - 25.0, thumb_rect.top() + 3.0),
                        Vec2::new(22.0, 13.0),
                    );
                    painter.rect_filled(u_rect, 2.0, Color32::from_rgb(0x16, 0x8a, 0x4a));
                    painter.text(
                        u_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{}x", use_cnt),
                        egui::FontId::monospace(8.5),
                        Color32::WHITE,
                    );
                }

                // Duration badge overlay at bottom right corner of thumbnail
                let dur_sec = item.duration;
                let dur_str = if dur_sec >= 3600.0 {
                    format!(
                        "{:02}:{:02}:{:02}",
                        (dur_sec / 3600.0) as u32,
                        ((dur_sec % 3600.0) / 60.0) as u32,
                        (dur_sec % 60.0) as u32
                    )
                } else {
                    format!(
                        "{:02}:{:02}",
                        (dur_sec / 60.0) as u32,
                        (dur_sec % 60.0) as u32
                    )
                };
                let dur_badge_rect = Rect::from_min_max(
                    Pos2::new(thumb_rect.right() - 42.0, thumb_rect.bottom() - 14.0),
                    Pos2::new(thumb_rect.right() - 2.0, thumb_rect.bottom() - 2.0),
                );
                painter.rect_filled(
                    dur_badge_rect,
                    2.0,
                    Color32::from_rgba_premultiplied(0, 0, 0, 200),
                );
                painter.text(
                    dur_badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    dur_str,
                    egui::FontId::monospace(8.5),
                    Color32::WHITE,
                );

                // Name label below thumbnail (truncated if needed)
                let text_y = thumb_rect.bottom() + 3.0;
                let mut display_name = item.name.clone();
                if display_name.chars().count() > 16 {
                    display_name = display_name.chars().take(14).collect();
                    display_name.push('…');
                }
                painter.text(
                    Pos2::new(card_rect.left() + 4.0, text_y),
                    egui::Align2::LEFT_TOP,
                    display_name,
                    egui::FontId::proportional(10.5),
                    Color32::from_rgb(0xe0, 0xe0, 0xee),
                );

                // Format / Specs below name
                let specs_str = if item.width > 0 {
                    if item.proxy_path.is_some() {
                        format!("{}x{} {:.0}fps [PX]", item.width, item.height, item.fps)
                    } else {
                        format!("{}x{} {:.0}fps", item.width, item.height, item.fps)
                    }
                } else {
                    format!("{:.1}s", item.duration)
                };
                painter.text(
                    Pos2::new(card_rect.left() + 4.0, card_rect.bottom() - 3.0),
                    egui::Align2::LEFT_BOTTOM,
                    specs_str,
                    egui::FontId::monospace(8.5),
                    Color32::from_rgb(0x75, 0x75, 0x88),
                );

                // Interactions
                if card_resp.clicked() {
                    *selected_media_id = Some(m_id.clone());
                }

                if card_resp.dragged() {
                    card_resp.dnd_set_drag_payload(m_id.clone());
                }

                // Double click loads into Source Monitor
                if card_resp.double_clicked() {
                    *selected_media_id = Some(m_id.clone());
                    *action = Some(MediaBinAction::OpenInSourceMonitor(m_id.clone()));
                }

                // Context Menu
                card_resp.context_menu(|ui| {
                    ui.set_min_width(220.0);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    ui.spacing_mut().button_padding = Vec2::new(8.0, 4.0);
                    if ui.button("Kaynak Monitöründe Aç (Çift Tık)").clicked() {
                        *action = Some(MediaBinAction::OpenInSourceMonitor(m_id.clone()));
                        ui.close_menu();
                    }
                    if ui.button("Zaman Çizgisine Ekle").clicked() {
                        *action = Some(MediaBinAction::AddToTimeline(m_id.clone()));
                        ui.close_menu();
                    }

                    // Move to Bin submenu
                    if let Some(mv) = render_move_to_bin_menu(ui, project, item, m_id) {
                        *action = Some(mv);
                    }

                    if item.media_type == MediaType::Video {
                        let proxy_label = if item.proxy_path.is_some() {
                            "◈ Proxy Mevcut (Yeniden Oluştur)"
                        } else {
                            "◈ 720p Proxy Oluştur"
                        };
                        if ui.button(proxy_label).clicked() {
                            *action = Some(MediaBinAction::GenerateProxy(m_id.clone()));
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button("Havuzdan Kaldır").clicked() {
                        *action = Some(MediaBinAction::DeleteMedia(m_id.clone()));
                        ui.close_menu();
                    }
                });
            }
        });
    });
}
