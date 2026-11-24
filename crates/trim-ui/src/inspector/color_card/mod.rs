mod presets;

use super::{
    inspector_card, prop_row, render_badge, InspectorAction, ACCENT_AMBER, ACCENT_BLUE,
    ACCENT_TEAL, LABEL_COLOR, LABEL_COL_W, SUBCARD_BG,
};
use crate::color_wheel::render_color_wheels_panel;
use egui::{
    Button, Color32, DragValue, Frame, Layout, Margin, Pos2, RichText, Rounding, Slider, Stroke,
    TextEdit, Ui, Vec2,
};
use presets::{draw_bc_preset_thumbnail, draw_look_preset_thumbnail};
use trim_commands::commands::ClipColorGradingState;
use trim_core::clip::{
    BrightnessContrastPreset, CameraLogProfile, ChromaKeyConfig, Clip, ColorWheelsConfig,
    FilmStockFormat, LookCategory, LutPreset, WhiteBalancePreset,
};
use trim_core::color::{CustomCurvesConfig, HslCurvesConfig, HslQualifierConfig};

pub(super) fn render_color_card(
    ui: &mut Ui,
    track_id: &str,
    clip: &mut Clip,
    action: &mut Option<InspectorAction>,
) {
    let has_color_changes = clip.has_color_adjustments()
        || clip.wb_preset != WhiteBalancePreset::Custom
        || clip.bc_preset != BrightnessContrastPreset::Neutral;

    let cg_drag_id = ui.make_persistent_id(format!("cg_drag_{}", clip.id));
    let mut orig_cg: Option<ClipColorGradingState> = ui.data_mut(|d| d.get_temp(cg_drag_id));

    let record_cg_start = |ui: &mut Ui, orig: &mut Option<ClipColorGradingState>, clip: &Clip| {
        if orig.is_none() {
            let snap = ClipColorGradingState::from_clip(clip);
            *orig = Some(snap.clone());
            ui.data_mut(|d| d.insert_temp(cg_drag_id, snap));
        }
    };

    let check_cg_stop = |ui: &mut Ui,
                         orig: &mut Option<ClipColorGradingState>,
                         clip: &Clip,
                         action: &mut Option<InspectorAction>| {
        if let Some(old_state) = orig.take() {
            let new_state = ClipColorGradingState::from_clip(clip);
            if old_state != new_state {
                *action = Some(InspectorAction::SetColorGrading {
                    track_id: track_id.to_string(),
                    clip_id: clip.id.clone(),
                    old_state: Box::new(old_state),
                    new_state: Box::new(new_state),
                });
            }
            ui.data_mut(|d| d.remove::<ClipColorGradingState>(cg_drag_id));
        }
    };

    let reset_colors = std::cell::Cell::new(false);
    let trigger_shot_match = std::cell::Cell::new(false);
    inspector_card(
        ui,
        "◈",
        "Renk Düzenleme, Presetler & 3D LUT",
        ACCENT_BLUE,
        |ui| {
            if ui
                .add(
                    Button::new(
                        RichText::new("⌖ Eşle")
                            .size(10.0)
                            .color(Color32::from_rgb(0xe8, 0xa8, 0x38)),
                    )
                    .fill(Color32::from_rgb(0x28, 0x22, 0x18))
                    .rounding(Rounding::same(4.0)),
                )
                .on_hover_text(
                    "Shot Match: Seçili referans kareye (veya nötr ton dengesine) göre pozlama, kontrast ve sıcaklığı otomatik eşle",
                )
                .clicked()
            {
                trigger_shot_match.set(true);
            }
            if has_color_changes
                && ui
                    .add(
                        Button::new(
                            RichText::new("↺ Sıfırla")
                                .size(10.0)
                                .color(Color32::from_rgb(0xc8, 0xd0, 0xe4)),
                        )
                        .fill(Color32::from_rgb(0x22, 0x26, 0x34))
                        .rounding(Rounding::same(4.0)),
                    )
                    .on_hover_text(
                        "Tüm beyaz dengesi, kontrast, renk görünümü ve LUT ayarlarını sıfırla",
                    )
                    .clicked()
            {
                reset_colors.set(true);
            }
        },
        |ui| {
            if trigger_shot_match.get() {
                *action = Some(InspectorAction::ShotMatch {
                    track_id: track_id.to_string(),
                    clip_id: clip.id.clone(),
                });
            }
            if reset_colors.get() {
                let old_state = ClipColorGradingState::from_clip(clip);
                clip.reset_color_grading();
                clip.color_wheels = None;
                clip.custom_curves = None;
                clip.hsl_curves = None;
                clip.hsl_qualifier = None;
                clip.halation = 0.0;
                clip.bloom = 0.0;
                clip.chromatic_aberration = 0.0;
                let new_state = ClipColorGradingState::from_clip(clip);
                *action = Some(InspectorAction::SetColorGrading {
                    track_id: track_id.to_string(),
                    clip_id: clip.id.clone(),
                    old_state: Box::new(old_state),
                    new_state: Box::new(new_state),
                });
            }

            // =================================================================
            // 1. CAMERA LOG PROFILE & WHITE BALANCE (BEYAZ DENGESİ)
            // =================================================================
            Frame::none()
                .fill(SUBCARD_BG)
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x24, 0x30)))
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(8.0, 7.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("▸ Kamera Log Giriş Profili")
                                .size(11.0)
                                .strong()
                                .color(Color32::from_rgb(0xb8, 0xc2, 0xd8)),
                        );
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            render_badge(ui, clip.camera_log_profile.short_label(), ACCENT_BLUE);
                        });
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let pill_w = ((ui.available_width() - 4.0 * 4.0) / 5.0).max(36.0);
                        for profile in CameraLogProfile::ALL {
                            let is_sel = clip.camera_log_profile == *profile;
                            let btn = Button::new(
                                RichText::new(profile.short_label())
                                    .size(10.0)
                                    .strong()
                                    .color(if is_sel {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(0xc0, 0xc8, 0xdc)
                                    }),
                            )
                            .fill(if is_sel {
                                ACCENT_BLUE
                            } else {
                                Color32::from_rgb(0x17, 0x1a, 0x24)
                            })
                            .stroke(Stroke::new(
                                1.0_f32,
                                if is_sel {
                                    ACCENT_BLUE
                                } else {
                                    Color32::from_rgb(0x26, 0x2b, 0x3a)
                                },
                            ))
                            .rounding(Rounding::same(4.0));
                            if ui
                                .add_sized([pill_w, 22.0], btn)
                                .on_hover_text(profile.label())
                                .clicked()
                                && !is_sel
                            {
                                let old_state = ClipColorGradingState::from_clip(clip);
                                clip.camera_log_profile = *profile;
                                let new_state = ClipColorGradingState::from_clip(clip);
                                *action = Some(InspectorAction::SetColorGrading {
                                    track_id: track_id.to_string(),
                                    clip_id: clip.id.clone(),
                                    old_state: Box::new(old_state),
                                    new_state: Box::new(new_state),
                                });
                            }
                        }
                    });

                    ui.add_space(5.0);
                    ui.separator();
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("▸ Beyaz Dengesi (White Balance)")
                                .size(11.5)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            if (clip.temperature.abs() > 0.05
                                || clip.tint.abs() > 0.05
                                || clip.wb_preset != WhiteBalancePreset::Custom)
                                && ui
                                    .add(
                                        Button::new(
                                            RichText::new("↺ Sıfırla")
                                                .size(9.5)
                                                .color(LABEL_COLOR),
                                        )
                                        .fill(Color32::from_rgb(0x1a, 0x1e, 0x29))
                                        .rounding(Rounding::same(3.0)),
                                    )
                                    .clicked()
                            {
                                let old_state = ClipColorGradingState::from_clip(clip);
                                clip.wb_preset = WhiteBalancePreset::Custom;
                                clip.temperature = 0.0;
                                clip.tint = 0.0;
                                let new_state = ClipColorGradingState::from_clip(clip);
                                *action = Some(InspectorAction::SetColorGrading {
                                    track_id: track_id.to_string(),
                                    clip_id: clip.id.clone(),
                                    old_state: Box::new(old_state),
                                    new_state: Box::new(new_state),
                                });
                            }
                            render_badge(ui, clip.wb_preset.short_label(), ACCENT_AMBER);
                        });
                    });

                    ui.add_space(4.0);

                    // Auto White Balance + Monitor Eyedropper Buttons (2 equal columns)
                    let is_auto = clip.wb_preset == WhiteBalancePreset::Auto;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let half_w = ((ui.available_width() - 6.0) * 0.5).max(80.0);
                        let auto_btn = Button::new(
                            RichText::new("◈ Otomatik WB (Auto)")
                                .size(10.5)
                                .strong()
                                .color(if is_auto {
                                    Color32::WHITE
                                } else {
                                    Color32::from_rgb(0xf5, 0xb9, 0x71)
                                }),
                        )
                        .fill(if is_auto {
                            ACCENT_AMBER
                        } else {
                            Color32::from_rgba_unmultiplied(0xe0, 0x7a, 0x38, 34)
                        })
                        .stroke(Stroke::new(1.0_f32, ACCENT_AMBER))
                        .rounding(Rounding::same(5.0));

                        if ui
                            .add_sized([half_w, 26.0], auto_btn)
                            .on_hover_text(
                                "Mevcut video karesinin renk dağılımını analiz ederek otomatik sıcaklık ve renk tonu hesaplar",
                            )
                            .clicked()
                        {
                            *action = Some(InspectorAction::AutoWhiteBalance {
                                track_id: track_id.to_string(),
                                clip_id: clip.id.clone(),
                            });
                        }

                        let pick_btn = Button::new(
                            RichText::new("⌖ WB Damlalık")
                                .size(10.5)
                                .strong()
                                .color(Color32::from_rgb(0x88, 0xd4, 0xff)),
                        )
                        .fill(Color32::from_rgba_unmultiplied(0x38, 0x7a, 0xe0, 38))
                        .stroke(Stroke::new(1.0_f32, ACCENT_BLUE))
                        .rounding(Rounding::same(5.0));

                        if ui
                            .add_sized([half_w, 26.0], pick_btn)
                            .on_hover_text(
                                "Program Monitörü üzerinden nötr beyaz/gri bir piksel seçerek Beyaz Dengesini kalibre et",
                            )
                            .clicked()
                        {
                            *action = Some(InspectorAction::StartWbEyedropper {
                                track_id: track_id.to_string(),
                                clip_id: clip.id.clone(),
                            });
                        }
                    });

                    ui.add_space(4.0);

                    // WB Presets Grid (2 equal columns)
                    let wb_presets = [
                        WhiteBalancePreset::Daylight,
                        WhiteBalancePreset::Tungsten,
                        WhiteBalancePreset::Cloudy,
                        WhiteBalancePreset::Shade,
                        WhiteBalancePreset::Fluorescent,
                        WhiteBalancePreset::Warm,
                        WhiteBalancePreset::Cool,
                    ];

                    let wb_col_w = ((ui.available_width() - 6.0) * 0.5).max(80.0);
                    egui::Grid::new(format!("wb_presets_grid_{}", clip.id))
                        .num_columns(2)
                        .spacing([6.0, 4.0])
                        .show(ui, |ui| {
                            for (idx, preset) in wb_presets.iter().enumerate() {
                                let is_sel = clip.wb_preset == *preset;
                                let sw = preset.swatch_rgb();
                                let sw_col = Color32::from_rgb(sw[0], sw[1], sw[2]);
                                let btn = Button::new(
                                    RichText::new(format!("● {}", preset.short_label()))
                                        .size(10.5)
                                        .strong()
                                        .color(if is_sel { Color32::WHITE } else { sw_col }),
                                )
                                .fill(if is_sel {
                                    Color32::from_rgb(0x28, 0x38, 0x54)
                                } else {
                                    Color32::from_rgb(0x17, 0x1a, 0x24)
                                })
                                .stroke(Stroke::new(
                                    1.0_f32,
                                    if is_sel {
                                        ACCENT_BLUE
                                    } else {
                                        Color32::from_rgb(0x26, 0x2b, 0x3a)
                                    },
                                ))
                                .rounding(Rounding::same(4.0));

                                if ui
                                    .add_sized([wb_col_w, 22.0], btn)
                                    .on_hover_text(preset.label())
                                    .clicked()
                                {
                                    let old_state = ClipColorGradingState::from_clip(clip);
                                    clip.apply_wb_preset(*preset);
                                    let new_state = ClipColorGradingState::from_clip(clip);
                                    *action = Some(InspectorAction::SetColorGrading {
                                        track_id: track_id.to_string(),
                                        clip_id: clip.id.clone(),
                                        old_state: Box::new(old_state),
                                        new_state: Box::new(new_state),
                                    });
                                }
                                if idx % 2 == 1 {
                                    ui.end_row();
                                }
                            }
                        });

                    ui.add_space(4.0);

                    // Manual Temperature Slider (-100..=100)
                    prop_row(ui, "Sıcaklık (Temp)", |ui| {
                        let s_resp = ui.add(
                            Slider::new(&mut clip.temperature, -100.0..=100.0)
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        let d_resp = ui.add(
                            DragValue::new(&mut clip.temperature)
                                .speed(0.5)
                                .range(-100.0..=100.0)
                                .fixed_decimals(1),
                        );
                        if s_resp.drag_started() || d_resp.drag_started() {
                            record_cg_start(ui, &mut orig_cg, clip);
                        }
                        if s_resp.changed() || d_resp.changed() {
                            clip.wb_preset = WhiteBalancePreset::Custom;
                            *action = Some(InspectorAction::ClipModified);
                        }
                        if s_resp.drag_stopped()
                            || d_resp.drag_stopped()
                            || d_resp.lost_focus()
                        {
                            check_cg_stop(ui, &mut orig_cg, clip, action);
                        }
                    });

                    // Manual Tint Slider (-100..=100)
                    prop_row(ui, "Renk Tonu (Tint)", |ui| {
                        let s_resp = ui.add(
                            Slider::new(&mut clip.tint, -100.0..=100.0)
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        let d_resp = ui.add(
                            DragValue::new(&mut clip.tint)
                                .speed(0.5)
                                .range(-100.0..=100.0)
                                .fixed_decimals(1),
                        );
                        if s_resp.drag_started() || d_resp.drag_started() {
                            record_cg_start(ui, &mut orig_cg, clip);
                        }
                        if s_resp.changed() || d_resp.changed() {
                            clip.wb_preset = WhiteBalancePreset::Custom;
                            *action = Some(InspectorAction::ClipModified);
                        }
                        if s_resp.drag_stopped()
                            || d_resp.drag_stopped()
                            || d_resp.lost_focus()
                        {
                            check_cg_stop(ui, &mut orig_cg, clip, action);
                        }
                    });
                });

            ui.add_space(5.0);

            // =================================================================
            // 2. BRIGHTNESS & CONTRAST PRESETS + TONAL SLIDERS
            // =================================================================
            Frame::none()
                .fill(SUBCARD_BG)
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x24, 0x30)))
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(8.0, 7.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("▸ Parlaklık & Kontrast Hazır Ayarları")
                                .size(11.5)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            render_badge(ui, clip.bc_preset.short_label(), ACCENT_TEAL);
                        });
                    });

                    ui.add_space(4.0);

                    // 2-column visual preset cards for BrightnessContrastPreset::ALL
                    let avail_w = ui.available_width();
                    let col_w = ((avail_w - 6.0) * 0.5).max(95.0);
                    egui::Grid::new(format!("bc_presets_grid_{}", clip.id))
                        .num_columns(2)
                        .spacing([6.0, 5.0])
                        .show(ui, |ui| {
                            for (idx, preset) in BrightnessContrastPreset::ALL.iter().enumerate() {
                                let is_sel = clip.bc_preset == *preset;
                                let (rect, resp) = ui.allocate_exact_size(
                                    Vec2::new(col_w, 28.0),
                                    egui::Sense::click(),
                                );
                                let bg = if is_sel {
                                    Color32::from_rgb(0x1f, 0x33, 0x4d)
                                } else if resp.hovered() {
                                    Color32::from_rgb(0x1c, 0x20, 0x2c)
                                } else {
                                    Color32::from_rgb(0x15, 0x18, 0x22)
                                };
                                let border = if is_sel {
                                    ACCENT_TEAL
                                } else if resp.hovered() {
                                    Color32::from_rgb(0x3a, 0x42, 0x58)
                                } else {
                                    Color32::from_rgb(0x24, 0x29, 0x38)
                                };
                                ui.painter().rect_filled(rect, Rounding::same(5.0), bg);
                                ui.painter().rect_stroke(
                                    rect,
                                    Rounding::same(5.0),
                                    Stroke::new(if is_sel { 1.5_f32 } else { 1.0_f32 }, border),
                                );

                                let thumb_rect = egui::Rect::from_min_size(
                                    egui::pos2(rect.left() + 5.0, rect.top() + 5.0),
                                    Vec2::new(28.0, 18.0),
                                );
                                draw_bc_preset_thumbnail(ui, thumb_rect, *preset);

                                ui.painter().text(
                                    egui::pos2(thumb_rect.right() + 6.0, rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    preset.short_label(),
                                    egui::FontId::proportional(10.5),
                                    if is_sel {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(0xd0, 0xd7, 0xe8)
                                    },
                                );

                                if resp.on_hover_text(preset.label()).clicked() {
                                    let old_state = ClipColorGradingState::from_clip(clip);
                                    clip.apply_bc_preset(*preset);
                                    let new_state = ClipColorGradingState::from_clip(clip);
                                    *action = Some(InspectorAction::SetColorGrading {
                                        track_id: track_id.to_string(),
                                        clip_id: clip.id.clone(),
                                        old_state: Box::new(old_state),
                                        new_state: Box::new(new_state),
                                    });
                                }

                                if idx % 2 == 1 {
                                    ui.end_row();
                                }
                            }
                        });

                    ui.add_space(5.0);

                    prop_row(ui, "Parlaklık", |ui| {
                        let s_resp = ui.add(
                            Slider::new(&mut clip.brightness, -1.0..=1.0)
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        let d_resp = ui.add(
                            DragValue::new(&mut clip.brightness)
                                .speed(0.01)
                                .range(-1.0..=1.0)
                                .fixed_decimals(2),
                        );
                        if s_resp.drag_started() || d_resp.drag_started() {
                            record_cg_start(ui, &mut orig_cg, clip);
                        }
                        if s_resp.changed() || d_resp.changed() {
                            *action = Some(InspectorAction::ClipModified);
                        }
                        if s_resp.drag_stopped() || d_resp.drag_stopped() || d_resp.lost_focus() {
                            check_cg_stop(ui, &mut orig_cg, clip, action);
                        }
                    });

                    prop_row(ui, "Kontrast", |ui| {
                        let s_resp = ui.add(
                            Slider::new(&mut clip.contrast, 0.0..=3.0)
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        let d_resp = ui.add(
                            DragValue::new(&mut clip.contrast)
                                .speed(0.02)
                                .range(0.0..=3.0)
                                .fixed_decimals(2),
                        );
                        if s_resp.drag_started() || d_resp.drag_started() {
                            record_cg_start(ui, &mut orig_cg, clip);
                        }
                        if s_resp.changed() || d_resp.changed() {
                            *action = Some(InspectorAction::ClipModified);
                        }
                        if s_resp.drag_stopped() || d_resp.drag_stopped() || d_resp.lost_focus() {
                            check_cg_stop(ui, &mut orig_cg, clip, action);
                        }
                    });

                    prop_row(ui, "Gama (Gamma)", |ui| {
                        let s_resp = ui.add(
                            Slider::new(&mut clip.gamma, 0.2..=3.0)
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        let d_resp = ui.add(
                            DragValue::new(&mut clip.gamma)
                                .speed(0.02)
                                .range(0.2..=3.0)
                                .fixed_decimals(2),
                        );
                        if s_resp.drag_started() || d_resp.drag_started() {
                            record_cg_start(ui, &mut orig_cg, clip);
                        }
                        if s_resp.changed() || d_resp.changed() {
                            *action = Some(InspectorAction::ClipModified);
                        }
                        if s_resp.drag_stopped() || d_resp.drag_stopped() || d_resp.lost_focus() {
                            check_cg_stop(ui, &mut orig_cg, clip, action);
                        }
                    });

                    prop_row(ui, "Doygunluk", |ui| {
                        let s_resp = ui.add(
                            Slider::new(&mut clip.saturation, 0.0..=3.0)
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        let d_resp = ui.add(
                            DragValue::new(&mut clip.saturation)
                                .speed(0.02)
                                .range(0.0..=3.0)
                                .fixed_decimals(2),
                        );
                        if s_resp.drag_started() || d_resp.drag_started() {
                            record_cg_start(ui, &mut orig_cg, clip);
                        }
                        if s_resp.changed() || d_resp.changed() {
                            *action = Some(InspectorAction::ClipModified);
                        }
                        if s_resp.drag_stopped() || d_resp.drag_stopped() || d_resp.lost_focus() {
                            check_cg_stop(ui, &mut orig_cg, clip, action);
                        }
                    });

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new("▸ Görsel Efektler (Blur / Keskinlik / Vinyet / Gren)")
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(0xb8, 0xc2, 0xd8)),
                    );

                    prop_row(ui, "Bulanıklık (Blur)", |ui| {
                        let s_resp = ui.add(
                            Slider::new(&mut clip.blur, 0.0..=20.0)
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        let d_resp = ui.add(
                            DragValue::new(&mut clip.blur)
                                .speed(0.1)
                                .range(0.0..=20.0)
                                .fixed_decimals(1),
                        );
                        if s_resp.drag_started() || d_resp.drag_started() {
                            record_cg_start(ui, &mut orig_cg, clip);
                        }
                        if s_resp.changed() || d_resp.changed() {
                            *action = Some(InspectorAction::ClipModified);
                        }
                        if s_resp.drag_stopped() || d_resp.drag_stopped() || d_resp.lost_focus() {
                            check_cg_stop(ui, &mut orig_cg, clip, action);
                        }
                    });

                    prop_row(ui, "Keskinlik (Sharpen)", |ui| {
                        let s_resp = ui.add(
                            Slider::new(&mut clip.sharpen, 0.0..=2.0)
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        let d_resp = ui.add(
                            DragValue::new(&mut clip.sharpen)
                                .speed(0.02)
                                .range(0.0..=2.0)
                                .fixed_decimals(2),
                        );
                        if s_resp.drag_started() || d_resp.drag_started() {
                            record_cg_start(ui, &mut orig_cg, clip);
                        }
                        if s_resp.changed() || d_resp.changed() {
                            *action = Some(InspectorAction::ClipModified);
                        }
                        if s_resp.drag_stopped() || d_resp.drag_stopped() || d_resp.lost_focus() {
                            check_cg_stop(ui, &mut orig_cg, clip, action);
                        }
                    });

                    prop_row(ui, "Vinyet (Vignette)", |ui| {
                        let s_resp = ui.add(
                            Slider::new(&mut clip.vignette, 0.0..=1.0)
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        let d_resp = ui.add(
                            DragValue::new(&mut clip.vignette)
                                .speed(0.02)
                                .range(0.0..=1.0)
                                .fixed_decimals(2),
                        );
                        if s_resp.drag_started() || d_resp.drag_started() {
                            record_cg_start(ui, &mut orig_cg, clip);
                        }
                        if s_resp.changed() || d_resp.changed() {
                            *action = Some(InspectorAction::ClipModified);
                        }
                        if s_resp.drag_stopped() || d_resp.drag_stopped() || d_resp.lost_focus() {
                            check_cg_stop(ui, &mut orig_cg, clip, action);
                        }
                    });

                    prop_row(ui, "Film Greni (Grain)", |ui| {
                        let s_resp = ui.add(
                            Slider::new(&mut clip.film_grain, 0.0..=1.0)
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        let d_resp = ui.add(
                            DragValue::new(&mut clip.film_grain)
                                .speed(0.02)
                                .range(0.0..=1.0)
                                .fixed_decimals(2),
                        );
                        if s_resp.drag_started() || d_resp.drag_started() {
                            record_cg_start(ui, &mut orig_cg, clip);
                        }
                        if s_resp.changed() || d_resp.changed() {
                            *action = Some(InspectorAction::ClipModified);
                        }
                        if s_resp.drag_stopped() || d_resp.drag_stopped() || d_resp.lost_focus() {
                            check_cg_stop(ui, &mut orig_cg, clip, action);
                        }
                    });
                });

            ui.add_space(5.0);

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
                    let orig_cw: Option<Option<ColorWheelsConfig>> =
                        ui.data_mut(|d| d.get_temp(cw_id));

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
                            let mut c = Color32::from_rgb(
                                ck.key_color[0],
                                ck.key_color[1],
                                ck.key_color[2],
                            );
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

            // =================================================================
            // 5. CUSTOM RGB / LUMA SPLINE CURVES
            // =================================================================
            ui.add_space(6.0);
            Frame::none()
                .fill(SUBCARD_BG)
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x24, 0x30)))
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
                    let mut has_cc = clip.custom_curves.is_some();
                    ui.horizontal(|ui| {
                        if ui
                            .checkbox(
                                &mut has_cc,
                                RichText::new("∿ Özel RGB / Luma Eğrileri (Spline Curves)")
                                    .size(11.0)
                                    .strong(),
                            )
                            .changed()
                        {
                            clip.custom_curves = if has_cc {
                                Some(CustomCurvesConfig::default())
                            } else {
                                None
                            };
                            *action = Some(InspectorAction::ClipModified);
                        }
                    });
                    if let Some(cc) = &mut clip.custom_curves {
                        let ch_id = ui.make_persistent_id(format!("cc_ch_{}", clip.id));
                        let mut active_ch: usize = ui.data_mut(|d| d.get_temp(ch_id).unwrap_or(0));
                        ui.horizontal(|ui| {
                            for (idx, lbl, col) in [
                                (0usize, "Master (Y)", Color32::WHITE),
                                (1usize, "Kırmızı (R)", Color32::from_rgb(240, 85, 85)),
                                (2usize, "Yeşil (G)", Color32::from_rgb(75, 220, 110)),
                                (3usize, "Mavi (B)", Color32::from_rgb(85, 150, 250)),
                            ] {
                                if ui
                                    .selectable_label(
                                        active_ch == idx,
                                        RichText::new(lbl).size(10.0).color(col).strong(),
                                    )
                                    .clicked()
                                {
                                    active_ch = idx;
                                    ui.data_mut(|d| d.insert_temp(ch_id, active_ch));
                                }
                            }
                            if ui
                                .small_button(RichText::new("↺").size(10.0))
                                .on_hover_text("Seçili kanalı sıfırla")
                                .clicked()
                            {
                                let default_pts = vec![[0.0, 0.0], [1.0, 1.0]];
                                match active_ch {
                                    0 => cc.master = default_pts,
                                    1 => cc.red = default_pts,
                                    2 => cc.green = default_pts,
                                    _ => cc.blue = default_pts,
                                }
                                *action = Some(InspectorAction::ClipModified);
                            }
                        });

                        let (rect, resp) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width().min(220.0), 110.0),
                            egui::Sense::click_and_drag(),
                        );
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(rect, 4.0, Color32::from_rgb(0x0b, 0x0d, 0x12));
                        painter.rect_stroke(
                            rect,
                            4.0,
                            Stroke::new(1.0_f32, Color32::from_rgb(0x24, 0x2a, 0x38)),
                        );
                        // Grid lines (25%, 50%, 75% + diagonal)
                        for g in [0.25_f32, 0.5, 0.75] {
                            let gx = rect.left() + g * rect.width();
                            let gy = rect.top() + g * rect.height();
                            let gc = Color32::from_rgba_unmultiplied(255, 255, 255, 16);
                            painter.line_segment(
                                [Pos2::new(gx, rect.top()), Pos2::new(gx, rect.bottom())],
                                Stroke::new(1.0_f32, gc),
                            );
                            painter.line_segment(
                                [Pos2::new(rect.left(), gy), Pos2::new(rect.right(), gy)],
                                Stroke::new(1.0_f32, gc),
                            );
                        }
                        painter.line_segment(
                            [rect.left_bottom(), rect.right_top()],
                            Stroke::new(
                                1.0_f32,
                                Color32::from_rgba_unmultiplied(255, 255, 255, 30),
                            ),
                        );

                        let pts = match active_ch {
                            0 => &mut cc.master,
                            1 => &mut cc.red,
                            2 => &mut cc.green,
                            _ => &mut cc.blue,
                        };
                        let stroke_col = match active_ch {
                            0 => Color32::WHITE,
                            1 => Color32::from_rgb(240, 85, 85),
                            2 => Color32::from_rgb(75, 220, 110),
                            _ => Color32::from_rgb(85, 150, 250),
                        };

                        // Handle click / drag interaction on control points
                        if let Some(pos) = resp.interact_pointer_pos() {
                            let nx = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                            let ny = (1.0 - (pos.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
                            if resp.secondary_clicked() && pts.len() > 2 {
                                // Remove nearest interior point
                                if let Some((best_i, _)) = pts
                                    .iter()
                                    .enumerate()
                                    .skip(1)
                                    .take(pts.len().saturating_sub(2))
                                    .min_by(|(_, a), (_, b)| {
                                        let da = (a[0] - nx).hypot(a[1] - ny);
                                        let db = (b[0] - nx).hypot(b[1] - ny);
                                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                                    })
                                {
                                    pts.remove(best_i);
                                    *action = Some(InspectorAction::ClipModified);
                                }
                            } else if resp.dragged() || resp.clicked() {
                                // Find closest point within threshold or insert a new one
                                let mut best_idx = None;
                                let mut best_dist = 0.09_f32;
                                for (i, p) in pts.iter().enumerate() {
                                    let d = (p[0] - nx).hypot(p[1] - ny);
                                    if d < best_dist {
                                        best_dist = d;
                                        best_idx = Some(i);
                                    }
                                }
                                if let Some(idx) = best_idx {
                                    if idx == 0 {
                                        pts[0] = [0.0, ny];
                                    } else if idx + 1 == pts.len() {
                                        let last = pts.len() - 1;
                                        pts[last] = [1.0, ny];
                                    } else {
                                        let min_x = pts[idx - 1][0] + 0.02;
                                        let max_x = pts[idx + 1][0] - 0.02;
                                        let clamped_x = if min_x <= max_x {
                                            nx.clamp(min_x, max_x)
                                        } else {
                                            0.5 * (pts[idx - 1][0] + pts[idx + 1][0])
                                        };
                                        pts[idx] = [clamped_x, ny];
                                    }
                                    *action = Some(InspectorAction::ClipModified);
                                } else if resp.clicked()
                                    && pts.len() < 8
                                    && pts.iter().all(|p| (p[0] - nx).abs() >= 0.03)
                                {
                                    pts.push([nx, ny]);
                                    pts.sort_by(|a, b| {
                                        a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal)
                                    });
                                    *action = Some(InspectorAction::ClipModified);
                                }
                            }
                        }

                        // Draw evaluated spline curve
                        let steps = 48usize;
                        let mut prev_pos = None;
                        for s in 0..=steps {
                            let x = s as f32 / steps as f32;
                            let y = CustomCurvesConfig::eval_curve(pts, x);
                            let p = Pos2::new(
                                rect.left() + x * rect.width(),
                                rect.bottom() - y * rect.height(),
                            );
                            if let Some(prev) = prev_pos {
                                painter.line_segment([prev, p], Stroke::new(1.8_f32, stroke_col));
                            }
                            prev_pos = Some(p);
                        }
                        for p in pts.iter() {
                            let cp = Pos2::new(
                                rect.left() + p[0] * rect.width(),
                                rect.bottom() - p[1] * rect.height(),
                            );
                            painter.circle_filled(cp, 4.0, stroke_col);
                            painter.circle_stroke(cp, 4.0, Stroke::new(1.0_f32, Color32::BLACK));
                        }
                    }
                });

            // =================================================================
            // 6. 6-VECTOR HSL CURVES (HUE VS HUE / SAT / LUMA & LUMA VS SAT)
            // =================================================================
            ui.add_space(6.0);
            Frame::none()
                .fill(SUBCARD_BG)
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x24, 0x30)))
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
                    let mut has_hsl = clip.hsl_curves.is_some();
                    if ui
                        .checkbox(
                            &mut has_hsl,
                            RichText::new("◈ 6-Vektör HSL Eğrileri (Hue vs Hue/Sat/Luma)")
                                .size(11.0)
                                .strong(),
                        )
                        .changed()
                    {
                        clip.hsl_curves = if has_hsl {
                            Some(HslCurvesConfig::default())
                        } else {
                            None
                        };
                        *action = Some(InspectorAction::ClipModified);
                    }
                    if let Some(hsl) = &mut clip.hsl_curves {
                        let tab_id = ui.make_persistent_id(format!("hsl_tab_{}", clip.id));
                        let mut tab: usize = ui.data_mut(|d| d.get_temp(tab_id).unwrap_or(0));
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            let tab_w = ((ui.available_width() - 4.0 * 3.0) / 4.0).max(44.0);
                            for (idx, lbl) in [
                                (0usize, "Hue vs Hue"),
                                (1usize, "Hue vs Sat"),
                                (2usize, "Hue vs Luma"),
                                (3usize, "Luma vs Sat"),
                            ] {
                                let is_sel = tab == idx;
                                let btn = Button::new(
                                    RichText::new(lbl).size(9.5).strong().color(if is_sel {
                                        Color32::WHITE
                                    } else {
                                        LABEL_COLOR
                                    }),
                                )
                                .fill(if is_sel {
                                    ACCENT_BLUE
                                } else {
                                    Color32::from_rgb(0x18, 0x1c, 0x26)
                                })
                                .rounding(Rounding::same(4.0));
                                if ui.add_sized([tab_w, 20.0], btn).clicked() {
                                    tab = idx;
                                    ui.data_mut(|d| d.insert_temp(tab_id, tab));
                                }
                            }
                        });
                        ui.add_space(4.0);
                        if tab < 3 {
                            let vec_labels = [
                                ("● Kırmızı (R)", Color32::from_rgb(235, 75, 75)),
                                ("● Sarı (Y)", Color32::from_rgb(235, 205, 65)),
                                ("● Yeşil (G)", Color32::from_rgb(75, 215, 105)),
                                ("● Camgöbeği (C)", Color32::from_rgb(65, 210, 235)),
                                ("● Mavi (B)", Color32::from_rgb(85, 135, 245)),
                                ("● Macenta (M)", Color32::from_rgb(225, 85, 215)),
                            ];
                            let arr = match tab {
                                0 => &mut hsl.hue_vs_hue,
                                1 => &mut hsl.hue_vs_sat,
                                _ => &mut hsl.hue_vs_luma,
                            };
                            for (i, (name, col)) in vec_labels.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(LABEL_COL_W, 20.0),
                                        Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            ui.colored_label(
                                                *col,
                                                RichText::new(*name).size(10.5).strong(),
                                            );
                                        },
                                    );
                                    ui.spacing_mut().slider_width =
                                        (ui.available_width() - 56.0).max(60.0);
                                    if ui
                                        .add(
                                            Slider::new(&mut arr[i], -1.0..=1.0)
                                                .show_value(true)
                                                .trailing_fill(true)
                                                .fixed_decimals(2),
                                        )
                                        .changed()
                                    {
                                        *action = Some(InspectorAction::ClipModified);
                                    }
                                });
                            }
                        } else {
                            let zone_labels = [
                                "Siyah (Shadows)",
                                "Koyu Ton (Dark)",
                                "Orta Ton (Mid)",
                                "Açık Ton (Light)",
                                "Parlak (Highlights)",
                            ];
                            for (i, lbl) in zone_labels.iter().enumerate() {
                                prop_row(ui, lbl, |ui| {
                                    if ui
                                        .add(
                                            Slider::new(&mut hsl.luma_vs_sat[i], -1.0..=1.0)
                                                .show_value(true)
                                                .trailing_fill(true)
                                                .fixed_decimals(2),
                                        )
                                        .changed()
                                    {
                                        *action = Some(InspectorAction::ClipModified);
                                    }
                                });
                            }
                        }
                    }
                });

            // =================================================================
            // 7. 3D HSL QUALIFIER (SECONDARY COLOR)
            // =================================================================
            ui.add_space(6.0);
            Frame::none()
                .fill(SUBCARD_BG)
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x24, 0x30)))
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
                    let mut has_q = clip.hsl_qualifier.is_some();
                    if ui
                        .checkbox(
                            &mut has_q,
                            RichText::new("⌖ 3D HSL Seçici (İkincil Renk / Qualifier)")
                                .size(11.0)
                                .strong(),
                        )
                        .changed()
                    {
                        clip.hsl_qualifier = if has_q {
                            Some(HslQualifierConfig {
                                enabled: true,
                                ..HslQualifierConfig::default()
                            })
                        } else {
                            None
                        };
                        *action = Some(InspectorAction::ClipModified);
                    }
                    if let Some(q) = &mut clip.hsl_qualifier {
                        q.enabled = true;
                        ui.add_space(2.0);
                        if ui
                            .checkbox(
                                &mut q.show_matte,
                                RichText::new("◧ Matte Önizleme (Seçilmeyenleri Gri Yap)")
                                    .size(10.5)
                                    .color(ACCENT_AMBER),
                            )
                            .changed()
                        {
                            *action = Some(InspectorAction::ClipModified);
                        }
                        prop_row(ui, "Renk (Hue°)", |ui| {
                            if ui
                                .add(
                                    Slider::new(&mut q.hue_center, 0.0..=360.0)
                                        .suffix("°")
                                        .trailing_fill(true),
                                )
                                .changed()
                            {
                                *action = Some(InspectorAction::ClipModified);
                            }
                        });
                        prop_row(ui, "Hue Genişliği", |ui| {
                            if ui
                                .add(
                                    Slider::new(&mut q.hue_width, 5.0..=180.0)
                                        .suffix("°")
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
                                    Slider::new(&mut q.hue_softness, 1.0..=90.0)
                                        .suffix("°")
                                        .trailing_fill(true),
                                )
                                .changed()
                            {
                                *action = Some(InspectorAction::ClipModified);
                            }
                        });
                        prop_row(ui, "Doygunluk Aralığı", |ui| {
                            let mut ch = false;
                            ch |= ui
                                .add(DragValue::new(&mut q.sat_min).speed(0.02).range(0.0..=1.0))
                                .changed();
                            ui.label("–");
                            ch |= ui
                                .add(DragValue::new(&mut q.sat_max).speed(0.02).range(0.0..=1.0))
                                .changed();
                            if ch {
                                *action = Some(InspectorAction::ClipModified);
                            }
                        });
                        prop_row(ui, "Parlaklık Aralığı", |ui| {
                            let mut ch = false;
                            ch |= ui
                                .add(DragValue::new(&mut q.luma_min).speed(0.02).range(0.0..=1.0))
                                .changed();
                            ui.label("–");
                            ch |= ui
                                .add(DragValue::new(&mut q.luma_max).speed(0.02).range(0.0..=1.0))
                                .changed();
                            if ch {
                                *action = Some(InspectorAction::ClipModified);
                            }
                        });
                        prop_row(ui, "Seçim Hue Kaydır", |ui| {
                            if ui
                                .add(
                                    Slider::new(&mut q.hue_shift, -180.0..=180.0)
                                        .suffix("°")
                                        .trailing_fill(true),
                                )
                                .changed()
                            {
                                *action = Some(InspectorAction::ClipModified);
                            }
                        });
                        prop_row(ui, "Seçim Doygunluk", |ui| {
                            if ui
                                .add(Slider::new(&mut q.sat_scale, 0.0..=2.5).trailing_fill(true))
                                .changed()
                            {
                                *action = Some(InspectorAction::ClipModified);
                            }
                        });
                        prop_row(ui, "Seçim Parlaklık", |ui| {
                            if ui
                                .add(
                                    Slider::new(&mut q.luma_offset, -0.5..=0.5).trailing_fill(true),
                                )
                                .changed()
                            {
                                *action = Some(InspectorAction::ClipModified);
                            }
                        });
                    }
                });

            // =================================================================
            // 8. ANALOG FILM & LENS FX (35mm/16mm/8mm, HALATION, BLOOM, DIFFUSION, MIST, WEAVE)
            // =================================================================
            ui.add_space(6.0);
            Frame::none()
                .fill(SUBCARD_BG)
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x24, 0x30)))
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("▸ Analog Film Emülasyonu & Lens Işık Motoru")
                                .size(11.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            render_badge(ui, clip.film_format.short_label(), ACCENT_AMBER);
                        });
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let pill_w = ((ui.available_width() - 4.0 * 3.0) / 4.0).max(40.0);
                        for fmt in FilmStockFormat::ALL {
                            let is_sel = clip.film_format == *fmt;
                            let btn = Button::new(
                                RichText::new(fmt.short_label())
                                    .size(10.0)
                                    .strong()
                                    .color(if is_sel {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(0xd4, 0xd8, 0xe8)
                                    }),
                            )
                            .fill(if is_sel {
                                ACCENT_AMBER
                            } else {
                                Color32::from_rgb(0x17, 0x1a, 0x24)
                            })
                            .stroke(Stroke::new(
                                1.0_f32,
                                if is_sel {
                                    ACCENT_AMBER
                                } else {
                                    Color32::from_rgb(0x26, 0x2b, 0x3a)
                                },
                            ))
                            .rounding(Rounding::same(4.0));
                            if ui
                                .add_sized([pill_w, 22.0], btn)
                                .on_hover_text(fmt.label())
                                .clicked()
                                && !is_sel
                            {
                                let old_state = ClipColorGradingState::from_clip(clip);
                                clip.film_format = *fmt;
                                let new_state = ClipColorGradingState::from_clip(clip);
                                *action = Some(InspectorAction::SetColorGrading {
                                    track_id: track_id.to_string(),
                                    clip_id: clip.id.clone(),
                                    old_state: Box::new(old_state),
                                    new_state: Box::new(new_state),
                                });
                            }
                        }
                    });
                    ui.add_space(4.0);
                    prop_row(ui, "Parlaklık Kırılması", |ui| {
                        if ui
                            .add(
                                Slider::new(&mut clip.highlight_rolloff, 0.0..=1.0)
                                    .show_value(true)
                                    .trailing_fill(true)
                                    .fixed_decimals(2),
                            )
                            .on_hover_text("Highlight Roll-off Knee: Yüksek ışıkları analog film gibi yumuşakça sıkıştırır")
                            .changed()
                        {
                            *action = Some(InspectorAction::ClipModified);
                        }
                    });
                    prop_row(ui, "Halation (Film)", |ui| {
                        if ui
                            .add(
                                Slider::new(&mut clip.halation, 0.0..=1.0)
                                    .show_value(true)
                                    .trailing_fill(true)
                                    .fixed_decimals(2),
                            )
                            .on_hover_text("Parlak kenarlarda kırmızı-turuncu analog film halesi")
                            .changed()
                        {
                            *action = Some(InspectorAction::ClipModified);
                        }
                    });
                    prop_row(ui, "Bloom (Işıma)", |ui| {
                        if ui
                            .add(
                                Slider::new(&mut clip.bloom, 0.0..=1.0)
                                    .show_value(true)
                                    .trailing_fill(true)
                                    .fixed_decimals(2),
                            )
                            .on_hover_text("Yüksek ışıklarda yumuşak sinematik parlama")
                            .changed()
                        {
                            *action = Some(InspectorAction::ClipModified);
                        }
                    });
                    prop_row(ui, "Diffusion (Lens)", |ui| {
                        if ui
                            .add(
                                Slider::new(&mut clip.diffusion, 0.0..=1.0)
                                    .show_value(true)
                                    .trailing_fill(true)
                                    .fixed_decimals(2),
                            )
                            .on_hover_text("Pro-Mist / Lens Difüzyonu: Kontrastı yumuşatarak orta tonları kadifeleştirir")
                            .changed()
                        {
                            *action = Some(InspectorAction::ClipModified);
                        }
                    });
                    prop_row(ui, "Mist (Atmosfer)", |ui| {
                        if ui
                            .add(
                                Slider::new(&mut clip.mist, 0.0..=1.0)
                                    .show_value(true)
                                    .trailing_fill(true)
                                    .fixed_decimals(2),
                            )
                            .on_hover_text("Atmosferik Sis: Parlak bölgelerin etrafında serin sinematik pus oluşturur")
                            .changed()
                        {
                            *action = Some(InspectorAction::ClipModified);
                        }
                    });
                    prop_row(ui, "35mm Gate Weave", |ui| {
                        if ui
                            .add(
                                Slider::new(&mut clip.gate_weave, 0.0..=1.0)
                                    .show_value(true)
                                    .trailing_fill(true)
                                    .fixed_decimals(2),
                            )
                            .on_hover_text("Analog projektör mekanik kare titremesi (Gate Weave)")
                            .changed()
                        {
                            *action = Some(InspectorAction::ClipModified);
                        }
                    });
                    prop_row(ui, "Renk Sapması", |ui| {
                        if ui
                            .add(
                                Slider::new(&mut clip.chromatic_aberration, 0.0..=1.0)
                                    .show_value(true)
                                    .trailing_fill(true)
                                    .fixed_decimals(2),
                            )
                            .on_hover_text("Chromatic Aberration: R/B kanal saçaklanması")
                            .changed()
                        {
                            *action = Some(InspectorAction::ClipModified);
                        }
                    });
                });
        },
    );
}
