use super::presets::draw_bc_preset_thumbnail;
use super::{check_cg_stop, record_cg_start};
use crate::inspector::{
    prop_row, render_badge, InspectorAction, ACCENT_AMBER, ACCENT_BLUE, ACCENT_TEAL, LABEL_COLOR,
    SUBCARD_BG,
};
use egui::{
    Button, Color32, DragValue, Frame, Layout, Margin, RichText, Rounding, Slider, Stroke, Ui, Vec2,
};
use trim_commands::commands::ClipColorGradingState;
use trim_core::clip::{BrightnessContrastPreset, CameraLogProfile, Clip, WhiteBalancePreset};

#[allow(clippy::too_many_arguments)]
pub(super) fn render_primary_section(
    ui: &mut Ui,
    track_id: &str,
    clip: &mut Clip,
    cg_drag_id: egui::Id,
    orig_cg: &mut Option<ClipColorGradingState>,
    action: &mut Option<InspectorAction>,
) {
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
                            record_cg_start(ui, cg_drag_id, orig_cg, clip);
                        }
                        if s_resp.changed() || d_resp.changed() {
                            clip.wb_preset = WhiteBalancePreset::Custom;
                            *action = Some(InspectorAction::ClipModified);
                        }
                        if s_resp.drag_stopped()
                            || d_resp.drag_stopped()
                            || d_resp.lost_focus()
                        {
                            check_cg_stop(ui, cg_drag_id, orig_cg, track_id, clip, action);
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
                            record_cg_start(ui, cg_drag_id, orig_cg, clip);
                        }
                        if s_resp.changed() || d_resp.changed() {
                            clip.wb_preset = WhiteBalancePreset::Custom;
                            *action = Some(InspectorAction::ClipModified);
                        }
                        if s_resp.drag_stopped()
                            || d_resp.drag_stopped()
                            || d_resp.lost_focus()
                        {
                            check_cg_stop(ui, cg_drag_id, orig_cg, track_id, clip, action);
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
                        let (rect, resp) =
                            ui.allocate_exact_size(Vec2::new(col_w, 28.0), egui::Sense::click());
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
                    record_cg_start(ui, cg_drag_id, orig_cg, clip);
                }
                if s_resp.changed() || d_resp.changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
                if s_resp.drag_stopped() || d_resp.drag_stopped() || d_resp.lost_focus() {
                    check_cg_stop(ui, cg_drag_id, orig_cg, track_id, clip, action);
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
                    record_cg_start(ui, cg_drag_id, orig_cg, clip);
                }
                if s_resp.changed() || d_resp.changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
                if s_resp.drag_stopped() || d_resp.drag_stopped() || d_resp.lost_focus() {
                    check_cg_stop(ui, cg_drag_id, orig_cg, track_id, clip, action);
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
                    record_cg_start(ui, cg_drag_id, orig_cg, clip);
                }
                if s_resp.changed() || d_resp.changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
                if s_resp.drag_stopped() || d_resp.drag_stopped() || d_resp.lost_focus() {
                    check_cg_stop(ui, cg_drag_id, orig_cg, track_id, clip, action);
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
                    record_cg_start(ui, cg_drag_id, orig_cg, clip);
                }
                if s_resp.changed() || d_resp.changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
                if s_resp.drag_stopped() || d_resp.drag_stopped() || d_resp.lost_focus() {
                    check_cg_stop(ui, cg_drag_id, orig_cg, track_id, clip, action);
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
                    record_cg_start(ui, cg_drag_id, orig_cg, clip);
                }
                if s_resp.changed() || d_resp.changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
                if s_resp.drag_stopped() || d_resp.drag_stopped() || d_resp.lost_focus() {
                    check_cg_stop(ui, cg_drag_id, orig_cg, track_id, clip, action);
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
                    record_cg_start(ui, cg_drag_id, orig_cg, clip);
                }
                if s_resp.changed() || d_resp.changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
                if s_resp.drag_stopped() || d_resp.drag_stopped() || d_resp.lost_focus() {
                    check_cg_stop(ui, cg_drag_id, orig_cg, track_id, clip, action);
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
                    record_cg_start(ui, cg_drag_id, orig_cg, clip);
                }
                if s_resp.changed() || d_resp.changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
                if s_resp.drag_stopped() || d_resp.drag_stopped() || d_resp.lost_focus() {
                    check_cg_stop(ui, cg_drag_id, orig_cg, track_id, clip, action);
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
                    record_cg_start(ui, cg_drag_id, orig_cg, clip);
                }
                if s_resp.changed() || d_resp.changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
                if s_resp.drag_stopped() || d_resp.drag_stopped() || d_resp.lost_focus() {
                    check_cg_stop(ui, cg_drag_id, orig_cg, track_id, clip, action);
                }
            });
        });

    ui.add_space(5.0);
}
