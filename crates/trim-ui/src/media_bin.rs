//! Premiere Pro style Media Bin panel with Grid/List views, search filtering,
//! folder/bin organization, thumbnail cards with duration badges, and Source Monitor double-click loading.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaFilterType {
    #[default]
    All,
    Video,
    Audio,
    Image,
}

pub enum MediaBinAction {
    ImportRequested,
    AddToTimeline(String),       // media_id or sequence_id
    OpenInSourceMonitor(String), // media_id
    GenerateProxy(String),       // media_id
    DeleteMedia(String),         // media_id
    CreateBinPrompt,
    RenameBinPrompt(String),  // bin_id
    DeleteBin(String),        // bin_id
    EnterBin(Option<String>), // bin_id (None = root)
    MoveToBin {
        media_id: String,
        bin_id: Option<String>,
    },
    OpenSequence(String), // sequence_id
    CreateSequencePrompt,
    DuplicateSequence(String), // sequence_id
    DeleteSequence(String),    // sequence_id
}

#[allow(clippy::too_many_arguments)]
pub fn render_media_bin(
    ui: &mut Ui,
    project: &Project,
    selected_media_id: &mut Option<String>,
    search_query: &mut String,
    is_grid_view: &mut bool,
    thumb_size: &mut f32,
    current_bin_id: &Option<String>,
    filter_type: &mut MediaFilterType,
    textures: &HashMap<String, TextureHandle>,
) -> Option<MediaBinAction> {
    let mut action = None;

    // 1. Sub-Toolbar: Search Input + View Mode Switcher + Zoom Slider
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
                .hint_text("Klip veya klasör adı...")
                .desired_width(ui.available_width().max(30.0));
            ui.add(text_edit);
        });
    });

    ui.add_space(2.0);

    // 2. Action Toolbar: + Yeni Klasör Button + Media Filter Chips
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;

        let new_bin_btn = Button::new(
            RichText::new("📁+ Klasör")
                .size(10.5)
                .color(Color32::from_rgb(0xfb, 0xbf, 0x24)),
        )
        .fill(Color32::from_rgb(0x22, 0x24, 0x30))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x3a, 0x3e, 0x50)));
        if ui
            .add_sized([72.0, 19.0], new_bin_btn)
            .on_hover_text("Yeni Medya Klasörü (Bin) Oluştur")
            .clicked()
        {
            action = Some(MediaBinAction::CreateBinPrompt);
        }

        let new_seq_btn = Button::new(
            RichText::new("🎞️+ Sekans")
                .size(10.5)
                .color(Color32::from_rgb(0xa7, 0x8b, 0xfa)),
        )
        .fill(Color32::from_rgb(0x22, 0x20, 0x30))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x3e, 0x38, 0x54)));
        if ui
            .add_sized([76.0, 19.0], new_seq_btn)
            .on_hover_text("Yeni Sekans (Timeline) Oluştur")
            .clicked()
        {
            action = Some(MediaBinAction::CreateSequencePrompt);
        }

        ui.add_space(2.0);
        ui.separator();
        ui.add_space(2.0);

        let filter_chips = [
            (MediaFilterType::All, "Tümü"),
            (MediaFilterType::Video, "🎬 Video"),
            (MediaFilterType::Audio, "🎵 Ses"),
            (MediaFilterType::Image, "🖼️ Görsel"),
        ];

        for (chip_type, label) in filter_chips {
            let is_active = *filter_type == chip_type;
            let chip_btn = Button::new(RichText::new(label).size(10.0).strong().color(
                if is_active {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(0x90, 0x90, 0xa0)
                },
            ))
            .fill(if is_active {
                Color32::from_rgb(0x25, 0x63, 0xeb)
            } else {
                Color32::from_rgb(0x18, 0x1a, 0x24)
            })
            .stroke(Stroke::new(
                1.0_f32,
                if is_active {
                    Color32::from_rgb(0x3b, 0x82, 0xf6)
                } else {
                    Color32::from_rgb(0x2a, 0x2d, 0x3d)
                },
            ));

            if ui.add(chip_btn).clicked() {
                *filter_type = chip_type;
            }
        }
    });

    ui.add_space(2.0);

    // 3. Breadcrumbs / Navigation Bar (when inside a subfolder)
    if let Some(bin_id) = current_bin_id {
        let bin_name = project
            .get_bin(bin_id)
            .map(|b| b.name.as_str())
            .unwrap_or("Klasör");
        egui::Frame::none()
            .fill(Color32::from_rgb(0x1a, 0x1c, 0x26))
            .rounding(egui::Rounding::same(3.0))
            .inner_margin(egui::Margin::symmetric(6.0, 4.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let back_btn = Button::new(
                        RichText::new("⬅ Geri")
                            .size(10.5)
                            .color(Color32::from_rgb(0x60, 0xa5, 0xfa)),
                    )
                    .fill(Color32::TRANSPARENT);
                    if ui
                        .add(back_btn)
                        .on_hover_text("Ana Havuza (Kök Dizin) Dön")
                        .clicked()
                    {
                        action = Some(MediaBinAction::EnterBin(None));
                    }

                    ui.label(
                        RichText::new("📁")
                            .size(11.0)
                            .color(Color32::from_rgb(0xf5, 0x9e, 0x0b)),
                    );
                    if ui
                        .link(
                            RichText::new("Ana Havuz")
                                .size(10.5)
                                .color(Color32::from_rgb(0x94, 0xa3, 0xb8)),
                        )
                        .clicked()
                    {
                        action = Some(MediaBinAction::EnterBin(None));
                    }
                    ui.label(
                        RichText::new(">")
                            .size(10.0)
                            .color(Color32::from_rgb(0x64, 0x74, 0x8b)),
                    );
                    ui.label(
                        RichText::new(bin_name)
                            .size(11.0)
                            .strong()
                            .color(Color32::WHITE),
                    );
                });
            });
        ui.add_space(2.0);
    }

    ui.separator();

    // 4. Empty State check
    if project.media_pool.is_empty() && project.bins.is_empty() {
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
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let btn_imp = Button::new(
                    RichText::new("+ Medya Ekle")
                        .size(12.0)
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(0xe0, 0x7a, 0x38));
                if ui.add(btn_imp).clicked() {
                    action = Some(MediaBinAction::ImportRequested);
                }

                let btn_fld = Button::new(
                    RichText::new("📁+ Klasör Oluştur")
                        .size(12.0)
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(0x28, 0x5a, 0x9e));
                if ui.add(btn_fld).clicked() {
                    action = Some(MediaBinAction::CreateBinPrompt);
                }
            });
        });
        return action;
    }

    // 5. Gather matching bins and media items
    let query_lower = search_query.trim().to_lowercase();

    // Folders are only listed in the root directory (current_bin_id == None)
    let matching_bins: Vec<&trim_core::project::MediaBinFolder> = if current_bin_id.is_none() {
        project
            .bins
            .iter()
            .filter(|b| {
                if query_lower.is_empty() {
                    true
                } else {
                    b.name.to_lowercase().contains(&query_lower)
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let matching_sequences: Vec<&trim_core::sequence::Sequence> = if current_bin_id.is_none()
        && (*filter_type == MediaFilterType::All || *filter_type == MediaFilterType::Video)
    {
        project
            .sequences
            .iter()
            .filter(|s| {
                if query_lower.is_empty() {
                    true
                } else {
                    s.name.to_lowercase().contains(&query_lower)
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let filtered_items: Vec<(&String, &trim_core::media::MediaItem)> = project
        .media_pool
        .iter()
        .filter(|(_, item)| {
            // Check bin membership
            let in_current_bin = match current_bin_id {
                None => item.bin_id.is_none(),
                Some(bid) => item.bin_id.as_deref() == Some(bid.as_str()),
            };
            if !in_current_bin {
                return false;
            }

            // Check media filter type
            let matches_filter = match filter_type {
                MediaFilterType::All => true,
                MediaFilterType::Video => item.media_type == MediaType::Video,
                MediaFilterType::Audio => item.media_type == MediaType::Audio,
                MediaFilterType::Image => item.media_type == MediaType::Image,
            };
            if !matches_filter {
                return false;
            }

            // Check search query
            if query_lower.is_empty() {
                true
            } else {
                item.name.to_lowercase().contains(&query_lower)
            }
        })
        .collect();

    if matching_bins.is_empty() && matching_sequences.is_empty() && filtered_items.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            if current_bin_id.is_some() {
                ui.label(
                    RichText::new("📁 Bu klasörde henüz medya yok")
                        .color(Color32::from_rgb(0xa0, 0xa0, 0xb0))
                        .size(12.0)
                        .strong(),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Dosyaları buraya sürükleyebilir veya medyaları sağ tıklayıp\n'📁 Klasöre Taşı' ile buraya ekleyebilirsiniz.")
                        .color(Color32::from_rgb(0x70, 0x70, 0x80))
                        .size(11.0),
                );
                ui.add_space(10.0);
                let btn_imp = Button::new(
                    RichText::new("+ Buraya Medya Ekle")
                        .size(11.0)
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(0xe0, 0x7a, 0x38));
                if ui.add(btn_imp).clicked() {
                    action = Some(MediaBinAction::ImportRequested);
                }
            } else {
                ui.label(
                    RichText::new("Eşleşen medya veya klasör bulunamadı")
                        .color(Color32::from_rgb(0x80, 0x80, 0x90))
                        .size(11.5),
                );
            }
        });
        return action;
    }

    // 6. Helper for "Klasöre Taşı" submenu on media items
    let render_move_to_bin_menu = |ui: &mut Ui,
                                   item: &trim_core::media::MediaItem,
                                   m_id: &String|
     -> Option<MediaBinAction> {
        let mut move_act = None;
        ui.menu_button("📁 Klasöre Taşı", |ui| {
            ui.set_min_width(190.0);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            ui.spacing_mut().button_padding = Vec2::new(8.0, 4.0);

            let is_in_root = item.bin_id.is_none();
            let root_btn = ui.add_enabled(
                !is_in_root,
                egui::Button::new(if is_in_root {
                    "✓ Ana Havuz (Kök Dizin)"
                } else {
                    "📁 Ana Havuz (Kök Dizin)"
                }),
            );
            if root_btn.clicked() {
                move_act = Some(MediaBinAction::MoveToBin {
                    media_id: m_id.clone(),
                    bin_id: None,
                });
                ui.close_menu();
            }

            if !project.bins.is_empty() {
                ui.separator();
                for bin in &project.bins {
                    let is_in_this = item.bin_id.as_deref() == Some(&bin.id);
                    let label = if is_in_this {
                        format!("✓ {}", bin.name)
                    } else {
                        format!("📁 {}", bin.name)
                    };
                    let bin_btn = ui.add_enabled(!is_in_this, egui::Button::new(label));
                    if bin_btn.clicked() {
                        move_act = Some(MediaBinAction::MoveToBin {
                            media_id: m_id.clone(),
                            bin_id: Some(bin.id.clone()),
                        });
                        ui.close_menu();
                    }
                }
            }
        });
        move_act
    };

    // 7. Grid View vs List View
    if *is_grid_view {
        let card_w = *thumb_size;
        let card_h = card_w * 0.5625 + 38.0; // 16:9 thumbnail + 38px text header/specs

        ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);

                // --- 7A. Folder Cards (Grid) ---
                for folder in &matching_bins {
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
                        "📁",
                        egui::FontId::proportional(28.0),
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
                    if display_name.len() > 16 {
                        display_name.truncate(14);
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
                        action = Some(MediaBinAction::EnterBin(Some(folder.id.clone())));
                    }

                    card_resp.context_menu(|ui| {
                        ui.set_min_width(200.0);
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        ui.spacing_mut().button_padding = Vec2::new(8.0, 4.0);

                        if ui.button("📂 Klasörü Aç (Çift Tık)").clicked() {
                            action = Some(MediaBinAction::EnterBin(Some(folder.id.clone())));
                            ui.close_menu();
                        }
                        if ui.button("✏️ Yeniden Adlandır...").clicked() {
                            action = Some(MediaBinAction::RenameBinPrompt(folder.id.clone()));
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("🗑️ Klasörü Sil").clicked() {
                            action = Some(MediaBinAction::DeleteBin(folder.id.clone()));
                            ui.close_menu();
                        }
                    });
                }

                // --- 7A.2. Sequence Cards (Grid) ---
                for seq in &matching_sequences {
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

                    // Sequence symbol 🎞️
                    painter.text(
                        thumb_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "🎞️",
                        egui::FontId::proportional(28.0),
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
                    if display_name.len() > 16 {
                        display_name.truncate(14);
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
                        action = Some(MediaBinAction::OpenSequence(seq.id.clone()));
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
                            action = Some(MediaBinAction::OpenSequence(seq.id.clone()));
                            ui.close_menu();
                        }
                        if ui.button("Zaman Çizgisine İç İçe Ekle (Nest)").clicked() {
                            action = Some(MediaBinAction::AddToTimeline(format!("seq:{}", seq.id)));
                            ui.close_menu();
                        }
                        if ui.button("📋 Sekansı Çoğalt (Duplicate)").clicked() {
                            action = Some(MediaBinAction::DuplicateSequence(seq.id.clone()));
                            ui.close_menu();
                        }
                        if project.sequences.len() > 1 {
                            ui.separator();
                            if ui.button("🗑️ Sekansı Sil").clicked() {
                                action = Some(MediaBinAction::DeleteSequence(seq.id.clone()));
                                ui.close_menu();
                            }
                        }
                    });
                }

                // --- 7B. Media Cards (Grid) ---
                for &(m_id, item) in &filtered_items {
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

                    // Double click loads into Source Monitor
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

                        // Move to Bin submenu
                        if let Some(mv) = render_move_to_bin_menu(ui, item, m_id) {
                            action = Some(mv);
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
            // --- 7A. Folder Rows (List) ---
            for folder in &matching_bins {
                let count = project
                    .media_pool
                    .values()
                    .filter(|m| m.bin_id.as_deref() == Some(&folder.id))
                    .count();
                let item_height = 36.0;
                let (rect, resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), item_height),
                    Sense::click(),
                );
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
                    "📁",
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
                    action = Some(MediaBinAction::EnterBin(Some(folder.id.clone())));
                }

                resp.context_menu(|ui| {
                    ui.set_min_width(200.0);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    ui.spacing_mut().button_padding = Vec2::new(8.0, 4.0);

                    if ui.button("📂 Klasörü Aç (Çift Tık)").clicked() {
                        action = Some(MediaBinAction::EnterBin(Some(folder.id.clone())));
                        ui.close_menu();
                    }
                    if ui.button("✏️ Yeniden Adlandır...").clicked() {
                        action = Some(MediaBinAction::RenameBinPrompt(folder.id.clone()));
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("🗑️ Klasörü Sil").clicked() {
                        action = Some(MediaBinAction::DeleteBin(folder.id.clone()));
                        ui.close_menu();
                    }
                });
            }

            // --- 7A.2. Sequence Rows (List) ---
            for seq in &matching_sequences {
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
                    "🎞️",
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
                    format!("Sekans  •  {}  •  {} kanal", dur_str, seq.timeline.tracks.len()),
                    egui::FontId::monospace(9.5),
                    Color32::from_rgb(0x9d, 0x94, 0xb8),
                );

                if resp.clicked() {
                    *selected_media_id = Some(format!("seq:{}", seq.id));
                }
                if resp.double_clicked() {
                    action = Some(MediaBinAction::OpenSequence(seq.id.clone()));
                }
                if resp.dragged() {
                    resp.dnd_set_drag_payload(format!("seq:{}", seq.id));
                }

                resp.context_menu(|ui| {
                    ui.set_min_width(220.0);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    ui.spacing_mut().button_padding = Vec2::new(8.0, 4.0);

                    if ui.button("Zaman Çizgisinde Aç (Çift Tık)").clicked() {
                        action = Some(MediaBinAction::OpenSequence(seq.id.clone()));
                        ui.close_menu();
                    }
                    if ui.button("Zaman Çizgisine İç İçe Ekle (Nest)").clicked() {
                        action = Some(MediaBinAction::AddToTimeline(format!("seq:{}", seq.id)));
                        ui.close_menu();
                    }
                    if ui.button("📋 Sekansı Çoğalt (Duplicate)").clicked() {
                        action = Some(MediaBinAction::DuplicateSequence(seq.id.clone()));
                        ui.close_menu();
                    }
                    if project.sequences.len() > 1 {
                        ui.separator();
                        if ui.button("🗑️ Sekansı Sil").clicked() {
                            action = Some(MediaBinAction::DeleteSequence(seq.id.clone()));
                            ui.close_menu();
                        }
                    }
                });
            }

            // --- 7B. Media Rows (List) ---
            for &(m_id, item) in &filtered_items {
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

                    // Move to Bin submenu
                    if let Some(mv) = render_move_to_bin_menu(ui, item, m_id) {
                        action = Some(mv);
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
