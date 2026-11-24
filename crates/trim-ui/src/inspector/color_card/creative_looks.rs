use super::presets::draw_look_preset_thumbnail;
use crate::color_wheel::render_color_wheels_panel;
use crate::inspector::{
    prop_row, render_badge, InspectorAction, ACCENT_BLUE, ACCENT_TEAL, LABEL_COLOR, SUBCARD_BG,
};
use egui::{
    Button, Color32, Frame, Layout, Margin, RichText, Rounding, Slider, Stroke, TextEdit, Ui, Vec2,
};
use trim_commands::commands::ClipColorGradingState;
use trim_core::clip::{ChromaKeyConfig, Clip, ColorWheelsConfig, LookCategory, LutPreset};

pub(super) fn render_creative_looks_section(
    ui: &mut Ui,
    track_id: &str,
    clip: &mut Clip,
    action: &mut Option<InspectorAction>,
) {
    // =================================================================
    // 3. CREATIVE COLOR LOOKS (Cinematic / Film Emulation / Creative)
    // =================================================================
    Frame::none()
                .fill(SUBCARD_BG)
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x24, 0x30)))
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(8.0, 7.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("▸ Yaratıcı Renk Görünümleri (Color Looks)")
                                .size(11.5)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            render_badge(ui, clip.lut_preset.short_label(), ACCENT_BLUE);
                        });
                    });

                    ui.add_space(4.0);

                    // Category filter tabs + Favorites & Search stored in egui data
                    let cat_tab_id = ui.make_persistent_id(format!("look_cat_tab_{}", clip.id));
                    let mut active_tab: u8 =
                        ui.data_mut(|d| d.get_temp::<u8>(cat_tab_id)).unwrap_or(0);
                    let fav_id = egui::Id::new("trim_favorite_lut_presets_v1");
                    let mut fav_presets: Vec<LutPreset> = ui
                        .data_mut(|d| d.get_persisted::<Vec<LutPreset>>(fav_id))
                        .unwrap_or_else(|| {
                            vec![
                                LutPreset::TealOrange,
                                LutPreset::Kodak2383,
                                LutPreset::MonochromeNoir,
                                LutPreset::GoldenHour,
                            ]
                        });
                    let search_id = ui.make_persistent_id(format!("look_search_q_{}", clip.id));
                    let mut search_q: String = ui
                        .data_mut(|d| d.get_temp::<String>(search_id))
                        .unwrap_or_default();

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let tab_w = ((ui.available_width() - 4.0 * 4.0) / 5.0).max(38.0);
                        let tabs: [(u8, &str); 5] = [
                            (0, "Tümü"),
                            (1, "Sinematik"),
                            (2, "Film"),
                            (3, "Yaratıcı"),
                            (4, "★ Favori"),
                        ];
                        for (tab_id, title) in tabs {
                            let is_active = active_tab == tab_id;
                            let btn = Button::new(RichText::new(title).size(10.0).strong().color(
                                if is_active {
                                    Color32::WHITE
                                } else {
                                    LABEL_COLOR
                                },
                            ))
                            .fill(if is_active {
                                ACCENT_BLUE
                            } else {
                                Color32::from_rgb(0x18, 0x1c, 0x26)
                            })
                            .rounding(Rounding::same(4.0));
                            if ui.add_sized([tab_w, 22.0], btn).clicked() {
                                active_tab = tab_id;
                                ui.data_mut(|d| d.insert_temp(cat_tab_id, active_tab));
                            }
                        }
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let has_q = !search_q.is_empty();
                        let edit_w = if has_q {
                            (ui.available_width() - 26.0).max(60.0)
                        } else {
                            ui.available_width()
                        };
                        let resp = ui.add(
                            TextEdit::singleline(&mut search_q)
                                .hint_text("Görünüm veya Film LUT ara...")
                                .desired_width(edit_w),
                        );
                        if resp.changed() {
                            ui.data_mut(|d| d.insert_temp(search_id, search_q.clone()));
                        }
                        if has_q && ui.small_button("✕").clicked() {
                            search_q.clear();
                            ui.data_mut(|d| d.insert_temp(search_id, String::new()));
                        }
                    });

                    ui.add_space(5.0);

                    let base_looks: Vec<LutPreset> = match active_tab {
                        1 => {
                            let mut v = vec![LutPreset::None];
                            v.extend_from_slice(LutPreset::by_category(LookCategory::Cinematic));
                            v
                        }
                        2 => {
                            let mut v = vec![LutPreset::None];
                            v.extend_from_slice(LutPreset::by_category(
                                LookCategory::FilmEmulation,
                            ));
                            v
                        }
                        3 => {
                            let mut v = vec![LutPreset::None];
                            v.extend_from_slice(LutPreset::by_category(LookCategory::Creative));
                            v
                        }
                        4 => {
                            let mut v = vec![LutPreset::None];
                            v.extend(fav_presets.iter().copied());
                            v
                        }
                        _ => LutPreset::ALL.to_vec(),
                    };

                    let q_lower = search_q.trim().to_lowercase();
                    let filtered_looks: Vec<LutPreset> = if q_lower.is_empty() {
                        base_looks
                    } else {
                        base_looks
                            .into_iter()
                            .filter(|p| {
                                *p == LutPreset::None
                                    || p.label().to_lowercase().contains(&q_lower)
                                    || p.short_label().to_lowercase().contains(&q_lower)
                            })
                            .collect()
                    };

                    // Visual Preset Thumbnail Grid (2 columns)
                    let avail_w = ui.available_width();
                    let col_w = ((avail_w - 6.0) * 0.5).max(95.0);
                    egui::Grid::new(format!("color_looks_grid_{}_{}", clip.id, active_tab))
                        .num_columns(2)
                        .spacing([6.0, 5.0])
                        .show(ui, |ui| {
                            for (idx, preset) in filtered_looks.iter().enumerate() {
                                let is_sel = clip.lut_preset == *preset;
                                let is_fav = fav_presets.contains(preset);
                                let (rect, resp) = ui.allocate_exact_size(
                                    Vec2::new(col_w, 34.0),
                                    egui::Sense::click(),
                                );

                                let bg = if is_sel {
                                    Color32::from_rgb(0x1e, 0x2f, 0x4d)
                                } else if resp.hovered() {
                                    Color32::from_rgb(0x1b, 0x20, 0x2d)
                                } else {
                                    Color32::from_rgb(0x14, 0x17, 0x21)
                                };
                                let border = if is_sel {
                                    ACCENT_BLUE
                                } else if resp.hovered() {
                                    Color32::from_rgb(0x3a, 0x44, 0x5c)
                                } else {
                                    Color32::from_rgb(0x23, 0x28, 0x36)
                                };

                                ui.painter().rect_filled(rect, Rounding::same(5.0), bg);
                                ui.painter().rect_stroke(
                                    rect,
                                    Rounding::same(5.0),
                                    Stroke::new(if is_sel { 1.5_f32 } else { 1.0_f32 }, border),
                                );

                                let thumb_rect = egui::Rect::from_min_size(
                                    egui::pos2(rect.left() + 5.0, rect.top() + 5.0),
                                    Vec2::new(34.0, 24.0),
                                );
                                draw_look_preset_thumbnail(ui, thumb_rect, *preset);

                                let cat_txt =
                                    preset.category().map(|c| c.label()).unwrap_or("Standart");
                                ui.painter().text(
                                    egui::pos2(thumb_rect.right() + 6.0, rect.top() + 11.0),
                                    egui::Align2::LEFT_CENTER,
                                    preset.short_label(),
                                    egui::FontId::proportional(10.5),
                                    if is_sel {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(0xd8, 0xdf, 0xf0)
                                    },
                                );
                                ui.painter().text(
                                    egui::pos2(thumb_rect.right() + 6.0, rect.top() + 23.5),
                                    egui::Align2::LEFT_CENTER,
                                    cat_txt,
                                    egui::FontId::proportional(8.8),
                                    Color32::from_rgb(0x7c, 0x86, 0x9e),
                                );

                                let star_rect = egui::Rect::from_min_size(
                                    egui::pos2(rect.right() - 18.0, rect.top() + 2.0),
                                    Vec2::new(16.0, 16.0),
                                );
                                if *preset != LutPreset::None {
                                    ui.painter().text(
                                        star_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        if is_fav { "★" } else { "☆" },
                                        egui::FontId::proportional(10.5),
                                        if is_fav {
                                            Color32::from_rgb(0xf5, 0xb9, 0x42)
                                        } else {
                                            Color32::from_rgb(0x64, 0x6e, 0x84)
                                        },
                                    );
                                }

                                let resp = resp.on_hover_text(preset.label());
                                if resp.clicked() {
                                    let clicked_star = *preset != LutPreset::None
                                        && resp
                                            .interact_pointer_pos()
                                            .is_some_and(|p| star_rect.expand(2.0).contains(p));
                                    if clicked_star {
                                        if is_fav {
                                            fav_presets.retain(|x| x != preset);
                                        } else {
                                            fav_presets.push(*preset);
                                        }
                                        ui.data_mut(|d| {
                                            d.insert_persisted(fav_id, fav_presets.clone())
                                        });
                                    } else {
                                        let old_state = ClipColorGradingState::from_clip(clip);
                                        clip.lut_preset = *preset;
                                        let new_state = ClipColorGradingState::from_clip(clip);
                                        if old_state != new_state {
                                            *action = Some(InspectorAction::SetColorGrading {
                                                track_id: track_id.to_string(),
                                                clip_id: clip.id.clone(),
                                                old_state: Box::new(old_state),
                                                new_state: Box::new(new_state),
                                            });
                                        }
                                    }
                                }

                                if idx % 2 == 1 {
                                    ui.end_row();
                                }
                            }
                        });

                    ui.add_space(5.0);

                    prop_row(ui, "Özel .cube LUT", |ui| {
                        if let Some(p) = clip.custom_lut_path.clone() {
                            let fname = trim_core::paths::normalize_path(&p)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| p.clone());
                            if ui
                                .add(
                                    Button::new(
                                        RichText::new("✕").size(10.0).color(Color32::WHITE),
                                    )
                                    .fill(Color32::from_rgb(0x3a, 0x20, 0x28))
                                    .rounding(Rounding::same(4.0)),
                                )
                                .on_hover_text("Özel LUT'u Kaldır")
                                .clicked()
                            {
                                let old_state = ClipColorGradingState::from_clip(clip);
                                clip.custom_lut_path = None;
                                let new_state = ClipColorGradingState::from_clip(clip);
                                *action = Some(InspectorAction::SetColorGrading {
                                    track_id: track_id.to_string(),
                                    clip_id: clip.id.clone(),
                                    old_state: Box::new(old_state),
                                    new_state: Box::new(new_state),
                                });
                            }
                            ui.label(
                                RichText::new(fname)
                                    .size(11.0)
                                    .color(Color32::from_rgb(0x58, 0xb0, 0xff)),
                            );
                        } else if ui
                            .add_sized(
                                [ui.available_width(), 22.0],
                                Button::new(
                                    RichText::new("+ .cube Dosyası Seç...")
                                        .size(10.5)
                                        .color(Color32::from_rgb(0xc8, 0xd0, 0xe4)),
                                )
                                .fill(Color32::from_rgb(0x1f, 0x23, 0x30))
                                .rounding(Rounding::same(4.0)),
                            )
                            .clicked()
                        {
                            *action = Some(InspectorAction::SelectCustomLut(clip.id.clone()));
                        }
                    });

                    ui.add_space(3.0);
                    let export_lut_btn = Button::new(
                        RichText::new("↗ Mevcut Renk Ayarını .cube LUT Olarak Kaydet...")
                            .size(10.5)
                            .strong()
                            .color(Color32::from_rgb(0x98, 0xe2, 0xc6)),
                    )
                    .fill(Color32::from_rgba_unmultiplied(0x2d, 0xb8, 0x9a, 28))
                    .stroke(Stroke::new(1.0_f32, ACCENT_TEAL))
                    .rounding(Rounding::same(5.0))
                    .min_size(Vec2::new(ui.available_width(), 24.0));

                    if ui
                        .add(export_lut_btn)
                        .on_hover_text(
                            "Klibin tüm renk, Log profili, çark, eğri ve görünüm ayarlarını 33×33×33 standart .cube 3D LUT dosyası olarak dışa aktar",
                        )
                        .clicked()
                    {
                        *action = Some(InspectorAction::ExportCubeLut {
                            track_id: track_id.to_string(),
                            clip_id: clip.id.clone(),
                        });
                    }
                });

    // Subcard: 3-Point Color Wheels
    ui.add_space(4.0);
    Frame::none()
        .fill(SUBCARD_BG)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x24, 0x30)))
        .rounding(Rounding::same(6.0))
        .inner_margin(Margin::symmetric(8.0, 6.0))
        .show(ui, |ui| {
            let cw_id = ui.make_persistent_id(format!("cw_drag_{}", clip.id));
            let orig_cw: Option<Option<ColorWheelsConfig>> = ui.data_mut(|d| d.get_temp(cw_id));

            let mut has_cw = clip.color_wheels.is_some();
            if ui
                .checkbox(
                    &mut has_cw,
                    RichText::new("Üç Noktalı Renk Çarkları (Lift / Gamma / Gain)")
                        .size(11.0)
                        .strong(),
                )
                .changed()
            {
                let old_wheels = clip.color_wheels.clone();
                let new_wheels = if has_cw {
                    Some(ColorWheelsConfig::default())
                } else {
                    None
                };
                clip.color_wheels = new_wheels.clone();
                *action = Some(InspectorAction::SetColorWheels {
                    track_id: track_id.to_string(),
                    clip_id: clip.id.clone(),
                    old_wheels,
                    new_wheels,
                });
            }

            if let Some(cw) = &mut clip.color_wheels {
                ui.add_space(4.0);
                let primary_down = ui.input(|i| i.pointer.primary_down());
                let primary_released = ui.input(|i| i.pointer.primary_released());

                if primary_down && orig_cw.is_none() {
                    ui.data_mut(|d| d.insert_temp(cw_id, Some(cw.clone())));
                }

                if render_color_wheels_panel(ui, cw) {
                    *action = Some(InspectorAction::ClipModified);
                }

                if primary_released {
                    if let Some(Some(old)) = orig_cw {
                        if &old != cw {
                            *action = Some(InspectorAction::SetColorWheels {
                                track_id: track_id.to_string(),
                                clip_id: clip.id.clone(),
                                old_wheels: Some(old),
                                new_wheels: Some(cw.clone()),
                            });
                        }
                    }
                    ui.data_mut(|d| d.remove::<Option<ColorWheelsConfig>>(cw_id));
                }
            }
        });

    // Subcard: Chroma Key
    ui.add_space(2.0);
    Frame::none()
        .fill(SUBCARD_BG)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x24, 0x30)))
        .rounding(Rounding::same(6.0))
        .inner_margin(Margin::symmetric(8.0, 6.0))
        .show(ui, |ui| {
            let mut has_ck = clip.chromakey.is_some();
            if ui
                .checkbox(
                    &mut has_ck,
                    RichText::new("◈ Yeşil / Mavi Perde (Chroma Key)")
                        .size(11.0)
                        .strong(),
                )
                .changed()
            {
                clip.chromakey = if has_ck {
                    Some(ChromaKeyConfig::default())
                } else {
                    None
                };
                *action = Some(InspectorAction::ClipModified);
            }

            if let Some(ck) = &mut clip.chromakey {
                ui.add_space(4.0);
                prop_row(ui, "Perde Rengi", |ui| {
                    let mut c =
                        Color32::from_rgb(ck.key_color[0], ck.key_color[1], ck.key_color[2]);
                    if ui.color_edit_button_srgba(&mut c).changed() {
                        ck.key_color = [c.r(), c.g(), c.b()];
                        *action = Some(InspectorAction::ClipModified);
                    }
                });

                prop_row(ui, "Benzerlik", |ui| {
                    if ui
                        .add(
                            Slider::new(&mut ck.similarity, 0.01..=1.0)
                                .show_value(true)
                                .trailing_fill(true),
                        )
                        .changed()
                    {
                        *action = Some(InspectorAction::ClipModified);
                    }
                });

                prop_row(ui, "Yumuşaklık", |ui| {
                    if ui
                        .add(
                            Slider::new(&mut ck.smoothness, 0.001..=0.5)
                                .show_value(true)
                                .trailing_fill(true),
                        )
                        .changed()
                    {
                        *action = Some(InspectorAction::ClipModified);
                    }
                });
            }
        });
}
