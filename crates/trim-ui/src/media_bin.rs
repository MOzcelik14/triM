//! Premiere Pro style Media Bin panel with Grid/List views, search filtering,
//! thumbnail cards with duration badges, and Source Monitor double-click loading.

use egui::{
    Button, Color32, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, TextureHandle, Ui, Vec2,
};
use std::collections::HashMap;
use trim_core::media::MediaType;
use trim_core::project::Project;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LeftPanelTab {
    #[default]
    Project,
    Effects,
}

pub enum MediaBinAction {
    ImportRequested,
    AddToTimeline(String),       // media_id
    OpenInSourceMonitor(String), // media_id
    GenerateProxy(String),       // media_id
    DeleteMedia(String),         // media_id
}

pub fn render_media_bin(
    ui: &mut Ui,
    project: &Project,
    selected_media_id: &mut Option<String>,
    search_query: &mut String,
    is_grid_view: &mut bool,
    thumb_size: &mut f32,
    textures: &HashMap<String, TextureHandle>,
) -> Option<MediaBinAction> {
    let mut action = None;

    // 2. Sub-Toolbar: Search Input + View Mode Switcher + Zoom Slider
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(
            RichText::new("Ara:")
                .size(11.0)
                .color(Color32::from_rgb(0x80, 0x80, 0x90)),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if *is_grid_view {
                ui.add_sized(
                    [44.0, 16.0],
                    egui::Slider::new(thumb_size, 70.0..=160.0).show_value(false),
                )
                .on_hover_text("Küçük Resim Boyutu");
            }

            let list_btn = Button::new(RichText::new("LST").size(10.0).strong().color(
                if !*is_grid_view {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(0x70, 0x70, 0x80)
                },
            ))
            .fill(if !*is_grid_view {
                Color32::from_rgb(0x28, 0x5a, 0x9e)
            } else {
                Color32::from_rgb(0x1a, 0x1a, 0x20)
            });
            if ui
                .add_sized([28.0, 18.0], list_btn)
                .on_hover_text("Liste Görünümü")
                .clicked()
            {
                *is_grid_view = false;
            }

            let grid_btn = Button::new(RichText::new("GRD").size(10.0).strong().color(
                if *is_grid_view {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(0x70, 0x70, 0x80)
                },
            ))
            .fill(if *is_grid_view {
                Color32::from_rgb(0x28, 0x5a, 0x9e)
            } else {
                Color32::from_rgb(0x1a, 0x1a, 0x20)
            });
            if ui
                .add_sized([28.0, 18.0], grid_btn)
                .on_hover_text("Izgara / Küçük Resim Görünümü")
                .clicked()
            {
                *is_grid_view = true;
            }

            if !search_query.is_empty()
                && ui
                    .add_sized([16.0, 18.0], Button::new("×"))
                    .on_hover_text("Aramayı temizle")
                    .clicked()
            {
                search_query.clear();
            }

            let text_edit = egui::TextEdit::singleline(search_query)
                .hint_text("Klip adı...")
                .desired_width(ui.available_width().max(30.0));
            ui.add(text_edit);
        });
    });

    ui.add_space(2.0);
    ui.separator();

    if project.media_pool.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(
                RichText::new("Medya Havuzu Boş")
                    .color(Color32::from_rgb(0x90, 0x90, 0xa5))
                    .size(13.0)
                    .strong(),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new("Dosyaları buraya sürükleyin\nveya '+ İçe Aktar' butonuna basın.")
                    .color(Color32::from_rgb(0x65, 0x65, 0x75))
                    .size(11.0),
            );
            ui.add_space(14.0);
            let btn_imp = Button::new(
                RichText::new("+ Medya Ekle")
                    .size(12.0)
                    .color(Color32::WHITE),
            )
            .fill(Color32::from_rgb(0xe0, 0x7a, 0x38));
            if ui.add(btn_imp).clicked() {
                action = Some(MediaBinAction::ImportRequested);
            }
        });
        return action;
    }

    // Filter media by search
    let query_lower = search_query.trim().to_lowercase();
    let filtered_items: Vec<(&String, &trim_core::media::MediaItem)> = project
        .media_pool
        .iter()
        .filter(|(_, item)| {
            if query_lower.is_empty() {
                true
            } else {
                item.name.to_lowercase().contains(&query_lower)
            }
        })
        .collect();

    if filtered_items.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(
                RichText::new("Eşleşen medya bulunamadı")
                    .color(Color32::from_rgb(0x80, 0x80, 0x90))
                    .size(11.5),
            );
        });
        return action;
    }

    // 3. Grid View vs List View
    if *is_grid_view {
        // Thumbnail Card Grid
        let card_w = *thumb_size;
        let card_h = card_w * 0.5625 + 38.0; // 16:9 thumbnail + 38px text header/duration
        let avail_w = ui.available_width().max(card_w + 12.0);
        let _columns = ((avail_w / (card_w + 10.0)).floor() as usize).max(1);

        ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);

                for (m_id, item) in filtered_items {
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
                    if display_name.len() > 16 {
                        display_name.truncate(14);
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

                    // Double click loads into Source Monitor!
                    if card_resp.double_clicked() {
                        *selected_media_id = Some(m_id.clone());
                        action = Some(MediaBinAction::OpenInSourceMonitor(m_id.clone()));
                    }

                    // Context Menu
                    card_resp.context_menu(|ui| {
                        ui.set_min_width(220.0);
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        ui.spacing_mut().button_padding = Vec2::new(8.0, 4.0);
                        if ui.button("Kaynak Monitöründe Aç (Çift Tık)").clicked() {
                            action = Some(MediaBinAction::OpenInSourceMonitor(m_id.clone()));
                            ui.close_menu();
                        }
                        if ui.button("Zaman Çizgisine Ekle").clicked() {
                            action = Some(MediaBinAction::AddToTimeline(m_id.clone()));
                            ui.close_menu();
                        }
                        if item.media_type == MediaType::Video {
                            let proxy_label = if item.proxy_path.is_some() {
                                "⚡ Proxy Mevcut (Yeniden Oluştur)"
                            } else {
                                "⚡ 720p Proxy Oluştur"
                            };
                            if ui.button(proxy_label).clicked() {
                                action = Some(MediaBinAction::GenerateProxy(m_id.clone()));
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        if ui.button("Havuzdan Kaldır").clicked() {
                            action = Some(MediaBinAction::DeleteMedia(m_id.clone()));
                            ui.close_menu();
                        }
                    });
                }
            });
        });
    } else {
        // List View
        ScrollArea::vertical().show(ui, |ui| {
            for (m_id, item) in filtered_items {
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
                    action = Some(MediaBinAction::OpenInSourceMonitor(m_id.clone()));
                }

                resp.context_menu(|ui| {
                    ui.set_min_width(220.0);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    ui.spacing_mut().button_padding = Vec2::new(8.0, 4.0);
                    if ui.button("Kaynak Monitöründe Aç").clicked() {
                        action = Some(MediaBinAction::OpenInSourceMonitor(m_id.clone()));
                        ui.close_menu();
                    }
                    if ui.button("Zaman Çizgisine Ekle").clicked() {
                        action = Some(MediaBinAction::AddToTimeline(m_id.clone()));
                        ui.close_menu();
                    }
                    if item.media_type == MediaType::Video {
                        let proxy_label = if item.proxy_path.is_some() {
                            "⚡ Proxy Mevcut (Yeniden Oluştur)"
                        } else {
                            "⚡ 720p Proxy Oluştur"
                        };
                        if ui.button(proxy_label).clicked() {
                            action = Some(MediaBinAction::GenerateProxy(m_id.clone()));
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button("Havuzdan Kaldır").clicked() {
                        action = Some(MediaBinAction::DeleteMedia(m_id.clone()));
                        ui.close_menu();
                    }
                });
            }
        });
    }

    action
}
