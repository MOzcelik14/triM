use super::types::*;
use egui::{Button, Color32, RichText, Stroke, Ui};
use trim_core::media::MediaType;
use trim_core::project::Project;

pub(super) struct FilteredMediaBin<'a> {
    pub matching_bins: Vec<&'a trim_core::project::MediaBinFolder>,
    pub matching_sequences: Vec<&'a trim_core::sequence::Sequence>,
    pub filtered_items: Vec<(&'a String, &'a trim_core::media::MediaItem)>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_toolbar_and_filter<'a>(
    ui: &mut Ui,
    project: &'a Project,
    search_query: &mut String,
    is_grid_view: &mut bool,
    thumb_size: &mut f32,
    current_bin_id: &Option<String>,
    filter_type: &mut MediaFilterType,
    action: &mut Option<MediaBinAction>,
) -> Option<FilteredMediaBin<'a>> {
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
            RichText::new("+ Klasör")
                .size(10.5)
                .color(Color32::from_rgb(0xfb, 0xbf, 0x24)),
        )
        .fill(Color32::from_rgb(0x22, 0x24, 0x30))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x3a, 0x3e, 0x50)));
        if ui
            .add_sized([68.0, 19.0], new_bin_btn)
            .on_hover_text("Yeni Medya Klasörü (Bin) Oluştur")
            .clicked()
        {
            *action = Some(MediaBinAction::CreateBinPrompt);
        }

        let new_seq_btn = Button::new(
            RichText::new("+ Sekans")
                .size(10.5)
                .color(Color32::from_rgb(0xa7, 0x8b, 0xfa)),
        )
        .fill(Color32::from_rgb(0x22, 0x20, 0x30))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x3e, 0x38, 0x54)));
        if ui
            .add_sized([72.0, 19.0], new_seq_btn)
            .on_hover_text("Yeni Sekans (Timeline) Oluştur")
            .clicked()
        {
            *action = Some(MediaBinAction::CreateSequencePrompt);
        }

        ui.menu_button(
            RichText::new("+ SFX")
                .size(10.5)
                .color(Color32::from_rgb(0x34, 0xd3, 0x99)),
            |ui| {
                ui.set_min_width(210.0);
                ui.label(
                    RichText::new("Prosedürel SFX / Ambiyans Üret")
                        .size(10.5)
                        .strong()
                        .color(Color32::from_rgb(0x9c, 0xa3, 0xb8)),
                );
                ui.separator();
                for preset in trim_media::SoundscapePreset::BIN_PRESETS {
                    let item_text = format!(
                        "{}  ({:.1}s)",
                        preset.label(),
                        preset.default_duration()
                    );
                    if ui
                        .button(RichText::new(item_text).size(11.0))
                        .on_hover_text(
                            "Dahili DSP ile .wav sentezler, medya havuzuna ve aktif ses kanalına ekler",
                        )
                        .clicked()
                    {
                        *action = Some(MediaBinAction::GenerateSoundscape(*preset));
                        ui.close_menu();
                    }
                }
            },
        )
        .response
        .on_hover_text("Prosedürel Ses Efekti & Ambiyans Sentezleyici (Riser, Sub Drop, Projektör, Yağmur, Whoosh)");

        ui.add_space(2.0);
        ui.separator();
        ui.add_space(2.0);

        let filter_chips = [
            (MediaFilterType::All, "Tümü"),
            (MediaFilterType::Video, "▸ Video"),
            (MediaFilterType::Audio, "♪ Ses"),
            (MediaFilterType::Image, "■ Görsel"),
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
                        RichText::new("← Geri")
                            .size(10.5)
                            .color(Color32::from_rgb(0x60, 0xa5, 0xfa)),
                    )
                    .fill(Color32::TRANSPARENT);
                    if ui
                        .add(back_btn)
                        .on_hover_text("Ana Havuza (Kök Dizin) Dön")
                        .clicked()
                    {
                        *action = Some(MediaBinAction::EnterBin(None));
                    }

                    ui.label(
                        RichText::new("⊞")
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
                        *action = Some(MediaBinAction::EnterBin(None));
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

    let is_os_dragging = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
    if is_os_dragging {
        let avail_h = ui.available_height().max(240.0);
        let top_space = ((avail_h - 200.0) * 0.38).clamp(24.0, 160.0);
        ui.add_space(top_space);

        ui.vertical_centered(|ui| {
            egui::Frame::none()
                .fill(Color32::from_rgba_unmultiplied(18, 48, 96, 230))
                .stroke(Stroke::new(2.0_f32, Color32::from_rgb(56, 160, 240)))
                .rounding(egui::Rounding::same(12.0))
                .inner_margin(egui::Margin::symmetric(28.0, 24.0))
                .show(ui, |ui| {
                    ui.set_max_width(340.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("📥").size(36.0));
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("Dosyaları Buraya Bırakın")
                                .color(Color32::WHITE)
                                .size(15.0)
                                .strong(),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Medya havuzuna doğrudan eklenecek")
                                .color(Color32::from_rgb(180, 215, 255))
                                .size(11.5),
                        );
                    });
                });
        });
        return None;
    }

    // 4. Empty State check
    if project.media_pool.is_empty() && project.bins.is_empty() {
        let avail_h = ui.available_height();
        let top_space = ((avail_h - 220.0) * 0.38).clamp(24.0, 160.0);
        ui.add_space(top_space);

        ui.vertical_centered(|ui| {
            egui::Frame::none()
                .fill(Color32::from_rgba_unmultiplied(24, 27, 38, 180))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 40)))
                .rounding(egui::Rounding::same(10.0))
                .inner_margin(egui::Margin::symmetric(24.0, 20.0))
                .show(ui, |ui| {
                    ui.set_max_width(340.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("🎬").size(30.0));
                        ui.add_space(6.0);

                        ui.label(
                            RichText::new("Medya Havuzu Boş")
                                .color(Color32::from_rgb(0xf1, 0xf5, 0xf9))
                                .size(14.0)
                                .strong(),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Dosyaları buraya sürükleyip bırakın\nveya aşağıdaki butonlarla başlayın.")
                                .color(Color32::from_rgb(0x94, 0xa3, 0xb8))
                                .size(11.0),
                        );
                        ui.add_space(16.0);

                        let btn_w1 = 116.0;
                        let btn_w2 = 126.0;
                        let spacing = 10.0;
                        let total_w = btn_w1 + btn_w2 + spacing;
                        let pad = ((ui.available_width() - total_w) / 2.0).max(0.0);

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = spacing;
                            ui.add_space(pad);

                            let btn_imp = Button::new(
                                RichText::new("+ Medya Ekle")
                                    .size(11.5)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(0x25, 0x63, 0xeb))
                            .rounding(egui::Rounding::same(5.0));

                            if ui.add_sized([btn_w1, 28.0], btn_imp).clicked() {
                                *action = Some(MediaBinAction::ImportRequested);
                            }

                            let btn_fld = Button::new(
                                RichText::new("📁+ Klasör Oluştur")
                                    .size(11.5)
                                    .strong()
                                    .color(Color32::from_rgb(0xfb, 0xbf, 0x24)),
                            )
                            .fill(Color32::from_rgb(0x1e, 0x24, 0x33))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x3b, 0x43, 0x58)))
                            .rounding(egui::Rounding::same(5.0));

                            if ui.add_sized([btn_w2, 28.0], btn_fld).clicked() {
                                *action = Some(MediaBinAction::CreateBinPrompt);
                            }
                        });
                    });
                });
        });
        return None;
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
        let avail_h = ui.available_height();
        let top_space = ((avail_h - 180.0) * 0.35).clamp(20.0, 120.0);
        ui.add_space(top_space);

        ui.vertical_centered(|ui| {
            if current_bin_id.is_some() {
                egui::Frame::none()
                    .fill(Color32::from_rgba_unmultiplied(24, 27, 38, 170))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 35)))
                    .rounding(egui::Rounding::same(10.0))
                    .inner_margin(egui::Margin::symmetric(24.0, 18.0))
                    .show(ui, |ui| {
                        ui.set_max_width(340.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new("⊞")
                                    .size(26.0)
                                    .color(Color32::from_rgb(0xfb, 0xbf, 0x24)),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("Bu Klasörde Medya Yok")
                                    .color(Color32::from_rgb(0xf1, 0xf5, 0xf9))
                                    .size(13.5)
                                    .strong(),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("Dosyaları buraya sürükleyebilir veya medyaları\nsağ tıklayıp '▸ Klasöre Taşı' ile ekleyebilirsiniz.")
                                    .color(Color32::from_rgb(0x94, 0xa3, 0xb8))
                                    .size(11.0),
                            );
                            ui.add_space(14.0);

                            let btn_w = 145.0;
                            let pad = ((ui.available_width() - btn_w) / 2.0).max(0.0);
                            ui.horizontal(|ui| {
                                ui.add_space(pad);
                                let btn_imp = Button::new(
                                    RichText::new("+ Buraya Medya Ekle")
                                        .size(11.5)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(0x25, 0x63, 0xeb))
                                .rounding(egui::Rounding::same(5.0));

                                if ui.add_sized([btn_w, 28.0], btn_imp).clicked() {
                                    *action = Some(MediaBinAction::ImportRequested);
                                }
                            });
                        });
                    });
            } else {
                egui::Frame::none()
                    .fill(Color32::from_rgba_unmultiplied(24, 27, 38, 150))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 30)))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(20.0, 16.0))
                    .show(ui, |ui| {
                        ui.set_max_width(300.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new("⌖")
                                    .size(24.0)
                                    .color(Color32::from_rgb(0x94, 0xa3, 0xb8)),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("Eşleşen Medya Bulunamadı")
                                    .color(Color32::from_rgb(0xf1, 0xf5, 0xf9))
                                    .size(13.0)
                                    .strong(),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("Arama kriterlerinize veya seçili filtreye uygun\nbir klip ya da klasör yok.")
                                    .color(Color32::from_rgb(0x94, 0xa3, 0xb8))
                                    .size(11.0),
                            );
                        });
                    });
            }
        });
        return None;
    }

    Some(FilteredMediaBin {
        matching_bins,
        matching_sequences,
        filtered_items,
    })
}
