use super::render_move_to_bin_menu;
use super::types::*;
use egui::{Color32, Rect, ScrollArea, Sense, Stroke, Ui, Vec2};
use std::collections::HashMap;
use trim_core::media::MediaType;
use trim_core::project::Project;

#[allow(clippy::too_many_arguments)]
pub(super) fn render_media_list(
    ui: &mut Ui,
    project: &Project,
    selected_media_id: &mut Option<String>,
    matching_bins: &[&trim_core::project::MediaBinFolder],
    matching_sequences: &[&trim_core::sequence::Sequence],
    filtered_items: &[(&String, &trim_core::media::MediaItem)],
    media_usage_counts: &HashMap<&str, usize>,
    action: &mut Option<MediaBinAction>,
) {
    ScrollArea::vertical().show(ui, |ui| {
        // --- 7A. Folder Rows (List) ---
        for folder in matching_bins {
            let count = project
                .media_pool
                .values()
                .filter(|m| m.bin_id.as_deref() == Some(&folder.id))
                .count();
            let item_height = 36.0;
            let (rect, resp) = ui
                .allocate_exact_size(Vec2::new(ui.available_width(), item_height), Sense::click());
            let painter = ui.painter_at(rect);

            let bg = if resp.hovered() {
                Color32::from_rgb(0x28, 0x24, 0x1e)
            } else {
                Color32::from_rgb(0x1a, 0x18, 0x14)
            };
            painter.rect_filled(rect, 2.0, bg);
            painter.rect_stroke(
                rect,
                2.0,
                Stroke::new(
                    1.0_f32,
                    if resp.hovered() {
                        Color32::from_rgb(0xf5, 0x9e, 0x0b)
                    } else {
                        Color32::from_rgb(0x32, 0x2b, 0x20)
                    },
                ),
            );

            // Folder Icon
            let pill_rect = Rect::from_center_size(
                egui::pos2(rect.left() + 18.0, rect.center().y),
                Vec2::new(24.0, 18.0),
            );
            painter.text(
                pill_rect.center(),
                egui::Align2::CENTER_CENTER,
                "⊞",
                egui::FontId::proportional(14.0),
                Color32::from_rgb(0xfb, 0xbf, 0x24),
            );

            // Name
            painter.text(
                egui::pos2(rect.left() + 36.0, rect.top() + 6.0),
                egui::Align2::LEFT_TOP,
                &folder.name,
                egui::FontId::proportional(11.5),
                Color32::from_rgb(0xf8, 0xfa, 0xfc),
            );

            // Subtext / Count
            painter.text(
                egui::pos2(rect.left() + 36.0, rect.bottom() - 5.0),
                egui::Align2::LEFT_BOTTOM,
                format!("Klasör (Bin)  •  {} öge", count),
                egui::FontId::monospace(9.5),
                Color32::from_rgb(0x94, 0xa3, 0xb8),
            );

            if resp.double_clicked() {
                *action = Some(MediaBinAction::EnterBin(Some(folder.id.clone())));
            }

            resp.context_menu(|ui| {
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

        // --- 7A.2. Sequence Rows (List) ---
        for seq in matching_sequences {
            let is_active = project.active_sequence_id.as_deref() == Some(&seq.id);
            let row_height = 36.0;
            let (rect, resp) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), row_height),
                Sense::click_and_drag(),
            );
            let painter = ui.painter_at(rect);

            let bg = if is_active {
                Color32::from_rgb(0x28, 0x24, 0x3c)
            } else if resp.hovered() {
                Color32::from_rgb(0x22, 0x1e, 0x30)
            } else {
                Color32::TRANSPARENT
            };
            painter.rect_filled(rect, 2.0, bg);

            // Icon
            painter.text(
                egui::pos2(rect.left() + 18.0, rect.center().y),
                egui::Align2::CENTER_CENTER,
                "≡",
                egui::FontId::proportional(14.0),
                Color32::from_rgb(0xa7, 0x8b, 0xfa),
            );

            // Name
            painter.text(
                egui::pos2(rect.left() + 36.0, rect.top() + 6.0),
                egui::Align2::LEFT_TOP,
                &seq.name,
                egui::FontId::proportional(11.5),
                Color32::from_rgb(0xf3, 0xf0, 0xff),
            );

            // Subtext / specs
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
            painter.text(
                egui::pos2(rect.left() + 36.0, rect.bottom() - 5.0),
                egui::Align2::LEFT_BOTTOM,
                format!(
                    "Sekans  •  {}  •  {} kanal",
                    dur_str,
                    seq.timeline.tracks.len()
                ),
                egui::FontId::monospace(9.5),
                Color32::from_rgb(0x9d, 0x94, 0xb8),
            );

            if resp.clicked() {
                *selected_media_id = Some(format!("seq:{}", seq.id));
            }
            if resp.double_clicked() {
                *action = Some(MediaBinAction::OpenSequence(seq.id.clone()));
            }
            if resp.dragged() {
                resp.dnd_set_drag_payload(format!("seq:{}", seq.id));
            }

            resp.context_menu(|ui| {
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

        // --- 7B. Media Rows (List) ---
        for &(m_id, item) in filtered_items {
            let is_selected = selected_media_id.as_deref() == Some(m_id.as_str());
            let item_height = 36.0;
            let (rect, resp) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), item_height),
                Sense::click_and_drag(),
            );
            let painter = ui.painter_at(rect);

            let bg = if is_selected {
                Color32::from_rgb(0x28, 0x38, 0x50)
            } else if resp.hovered() {
                Color32::from_rgb(0x25, 0x25, 0x30)
            } else {
                Color32::from_rgb(0x18, 0x18, 0x20)
            };

            painter.rect_filled(rect, 2.0, bg);
            if is_selected {
                painter.rect_stroke(
                    rect,
                    2.0,
                    Stroke::new(1.0_f32, Color32::from_rgb(0x38, 0x88, 0xe8)),
                );
            }

            // Type Badge Pill
            let (type_code, badge_bg) = match item.media_type {
                MediaType::Video => ("VID", Color32::from_rgb(0x28, 0x5a, 0x9e)),
                MediaType::Audio => ("SES", Color32::from_rgb(0x28, 0x80, 0x50)),
                MediaType::Image => ("IMG", Color32::from_rgb(0x9e, 0x5a, 0x28)),
                MediaType::Sequence => ("SEK", Color32::from_rgb(0x7c, 0x3a, 0xed)),
                MediaType::Unknown => ("DOS", Color32::from_rgb(0x40, 0x40, 0x48)),
            };
            let pill_rect = Rect::from_center_size(
                egui::pos2(rect.left() + 18.0, rect.center().y),
                Vec2::new(24.0, 14.0),
            );
            painter.rect_filled(pill_rect, 2.0, badge_bg);
            painter.text(
                pill_rect.center(),
                egui::Align2::CENTER_CENTER,
                type_code,
                egui::FontId::monospace(8.5),
                Color32::WHITE,
            );

            // Name
            painter.text(
                egui::pos2(rect.left() + 36.0, rect.top() + 6.0),
                egui::Align2::LEFT_TOP,
                &item.name,
                egui::FontId::proportional(11.5),
                Color32::from_rgb(0xe0, 0xe0, 0xea),
            );

            // Usage count badge in List view
            let use_cnt = media_usage_counts.get(m_id.as_str()).copied().unwrap_or(0);
            if use_cnt > 0 {
                let u_rect = Rect::from_center_size(
                    egui::pos2(rect.right() - 18.0, rect.center().y),
                    Vec2::new(24.0, 14.0),
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

            // Duration & Specs
            let dur_str = format!("{:.1}s", item.duration);
            let res_str = if item.width > 0 {
                if item.proxy_path.is_some() {
                    format!("{}x{} {:.0}fps [PX]", item.width, item.height, item.fps)
                } else {
                    format!("{}x{} {:.0}fps", item.width, item.height, item.fps)
                }
            } else {
                String::new()
            };
            painter.text(
                egui::pos2(rect.left() + 30.0, rect.bottom() - 5.0),
                egui::Align2::LEFT_BOTTOM,
                format!("{}  {}", dur_str, res_str),
                egui::FontId::monospace(9.5),
                Color32::from_rgb(0x80, 0x80, 0x90),
            );

            if resp.clicked() {
                *selected_media_id = Some(m_id.clone());
            }

            if resp.dragged() {
                resp.dnd_set_drag_payload(m_id.clone());
            }

            if resp.double_clicked() {
                *action = Some(MediaBinAction::OpenInSourceMonitor(m_id.clone()));
            }

            resp.context_menu(|ui| {
                ui.set_min_width(220.0);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                ui.spacing_mut().button_padding = Vec2::new(8.0, 4.0);
                if ui.button("Kaynak Monitöründe Aç").clicked() {
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
}
