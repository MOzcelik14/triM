use super::super::{
    inspector_card, prop_row, render_badge, render_kf_btn, render_mask_kf_btn, sync_kf_value,
    InspectorAction, ACCENT_BLUE, ACCENT_TEAL, LABEL_COLOR, SUBCARD_BG,
};
use egui::{
    Button, Color32, DragValue, Frame, Layout, Margin, RichText, Rounding, Stroke, Ui, Vec2,
};
use trim_core::clip::{Clip, CornerPinConfig};

pub(in crate::inspector) fn render_compositing_card(
    ui: &mut Ui,
    track_id: &str,
    clip: &mut Clip,
    clip_time: f64,
    action: &mut Option<InspectorAction>,
) {
    let accent = if clip.is_adjustment_layer() {
        Color32::from_rgb(0xb8, 0x58, 0xff)
    } else {
        ACCENT_BLUE
    };
    let title = if clip.is_adjustment_layer() {
        "Ayar Katmanı & Karışım"
    } else {
        "Karışım Modu & Perspektif"
    };
    let clip_id = clip.id.clone();
    let is_adj = clip.is_adjustment_layer();

    inspector_card(
        ui,
        "◈",
        title,
        accent,
        move |ui| {
            if is_adj {
                render_badge(ui, "AYAR KATMANI", Color32::from_rgb(0xb8, 0x58, 0xff));
            }
        },
        |ui| {
            prop_row(ui, "Karışım Modu", |ui| {
                let mut selected = clip.blend_mode;
                egui::ComboBox::from_id_salt(format!("blend_mode_{}", clip.id))
                    .selected_text(selected.label())
                    .width(ui.available_width().max(110.0))
                    .show_ui(ui, |ui| {
                        for mode in trim_core::BlendMode::ALL {
                            if ui
                                .selectable_value(&mut selected, *mode, mode.label())
                                .clicked()
                                && selected != clip.blend_mode
                            {
                                let old_blend_mode = clip.blend_mode;
                                clip.blend_mode = selected;
                                *action = Some(InspectorAction::SetBlendMode {
                                    track_id: track_id.to_string(),
                                    clip_id: clip_id.clone(),
                                    old_blend_mode,
                                    new_blend_mode: selected,
                                });
                            }
                        }
                    });
            });

            if !clip.is_adjustment_layer() {
                ui.add_space(4.0);
                if ui
                    .checkbox(
                        &mut clip.track_matte_above,
                        RichText::new("◧ Üst Kanalı Maske Olarak Kullan (Track Matte)")
                            .size(11.0),
                    )
                    .on_hover_text(
                        "Bir üst video kanalındaki klibin parlaklık/alfa değerini bu klibin görünürlük maskesi olarak kullanır (Text-in-Video vb.)",
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }

                ui.add_space(4.0);
                Frame::none()
                    .fill(SUBCARD_BG)
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x24, 0x30)))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(Margin::symmetric(8.0, 6.0))
                    .show(ui, |ui| {
                        let mut has_cp = clip.corner_pin.is_some();
                        ui.horizontal(|ui| {
                            if ui
                                .checkbox(
                                    &mut has_cp,
                                    RichText::new("⊞ 4 Köşe Perspektif (Corner Pin)")
                                        .size(11.0)
                                        .strong(),
                                )
                                .changed()
                            {
                                clip.corner_pin = if has_cp {
                                    Some(CornerPinConfig::default())
                                } else {
                                    None
                                };
                                *action = Some(InspectorAction::ClipModified);
                            }
                            if has_cp
                                && ui
                                    .small_button("↺")
                                    .on_hover_text("Köşeleri sıfırla")
                                    .clicked()
                            {
                                clip.corner_pin = Some(CornerPinConfig::default());
                                *action = Some(InspectorAction::ClipModified);
                            }
                        });
                        if let Some(cp) = &mut clip.corner_pin {
                            for (lbl, pt) in [
                                ("Sol Üst (TL)", &mut cp.top_left),
                                ("Sağ Üst (TR)", &mut cp.top_right),
                                ("Sağ Alt (BR)", &mut cp.bottom_right),
                                ("Sol Alt (BL)", &mut cp.bottom_left),
                            ] {
                                prop_row(ui, lbl, |ui| {
                                    let mut ch = false;
                                    ch |= ui
                                        .add(
                                            DragValue::new(&mut pt[0])
                                                .speed(0.005)
                                                .range(-0.5..=1.5)
                                                .prefix("X:"),
                                        )
                                        .changed();
                                    ch |= ui
                                        .add(
                                            DragValue::new(&mut pt[1])
                                                .speed(0.005)
                                                .range(-0.5..=1.5)
                                                .prefix("Y:"),
                                        )
                                        .changed();
                                    if ch {
                                        *action = Some(InspectorAction::ClipModified);
                                    }
                                });
                            }
                        }
                    });
            }

            if clip.is_adjustment_layer() {
                ui.add_space(4.0);
                let op_drag_id = ui.make_persistent_id(format!("adj_op_drag_{}", clip.id));
                let orig_op: Option<(f64, f64, f64, f64, f64)> =
                    ui.data_mut(|d| d.get_temp(op_drag_id));
                let cur_op = clip.get_property_at_time("opacity", clip_time);
                let mut op_val = cur_op;
                prop_row(ui, "Opaklık", |ui| {
                    ui.spacing_mut().slider_width = (ui.available_width() - 84.0).max(48.0);
                    let resp = ui.add(
                        egui::Slider::new(&mut op_val, 0.0..=1.0)
                            .fixed_decimals(2)
                            .trailing_fill(true)
                            .show_value(true),
                    );
                    if resp.drag_started() {
                        ui.data_mut(|d| {
                            d.insert_temp(
                                op_drag_id,
                                (
                                    clip.pos_x,
                                    clip.pos_y,
                                    clip.scale,
                                    clip.rotation,
                                    clip.opacity,
                                ),
                            )
                        });
                    }
                    if resp.changed() {
                        clip.opacity = op_val;
                        sync_kf_value(clip, "opacity", clip_time, op_val);
                        *action = Some(InspectorAction::ClipModified);
                    }
                    if resp.drag_stopped() {
                        if let Some(old_t) = orig_op {
                            let new_t = (
                                clip.pos_x,
                                clip.pos_y,
                                clip.scale,
                                clip.rotation,
                                clip.opacity,
                            );
                            *action = Some(InspectorAction::SetTransform {
                                track_id: track_id.to_string(),
                                clip_id: clip_id.clone(),
                                old_transform: old_t,
                                new_transform: new_t,
                            });
                            ui.data_mut(|d| d.remove::<(f64, f64, f64, f64, f64)>(op_drag_id));
                        }
                    }
                    render_kf_btn(ui, clip, "opacity", clip_time, op_val, action);
                });
            }
        },
    );
}

pub(in crate::inspector) fn render_masks_card(
    ui: &mut Ui,
    track_id: &str,
    clip: &mut Clip,
    clip_time: f64,
    action: &mut Option<InspectorAction>,
) {
    let clip_id = clip.id.clone();
    let count_txt = format!("{} maske", clip.masks.len());
    inspector_card(
        ui,
        "◧",
        "Maskeler",
        ACCENT_TEAL,
        move |ui| {
            ui.label(RichText::new(count_txt).size(10.5).color(LABEL_COLOR));
        },
        |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let btn_w = ((ui.available_width() - 6.0 * 2.0) / 3.0).max(58.0);
                for (lbl, shape, name_base) in [
                    (
                        "+ Dikdörtgen",
                        trim_core::MaskShape::Rectangle,
                        "Dikdörtgen",
                    ),
                    ("+ Elips", trim_core::MaskShape::Ellipse, "Elips"),
                    ("+ Bezier", trim_core::MaskShape::Bezier, "Bezier"),
                ] {
                    if ui
                        .add_sized(
                            [btn_w, 24.0],
                            Button::new(RichText::new(lbl).size(10.5).color(Color32::WHITE))
                                .fill(Color32::from_rgb(0x20, 0x26, 0x34))
                                .rounding(Rounding::same(4.0)),
                        )
                        .clicked()
                    {
                        let old_masks = clip.masks.clone();
                        let idx = clip.masks.len() + 1;
                        let new_mask = trim_core::Mask::new(shape, format!("{name_base} {idx}"));
                        clip.masks.push(new_mask);
                        *action = Some(InspectorAction::SetMasks {
                            track_id: track_id.to_string(),
                            clip_id: clip_id.clone(),
                            old_masks,
                            new_masks: clip.masks.clone(),
                        });
                    }
                }
            });

            if clip.masks.is_empty() {
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Monitörden veya yukarıdan bir maske ekleyin.")
                        .size(11.0)
                        .color(LABEL_COLOR),
                );
                return;
            }

            let mut remove_idx: Option<usize> = None;
            let mut commit_masks_snapshot: Option<Vec<trim_core::Mask>> = None;
            let drag_id = ui.make_persistent_id(format!("mask_drag_{}", clip.id));
            let orig_drag: Option<Vec<trim_core::Mask>> = ui.data_mut(|d| d.get_temp(drag_id));

            for idx in 0..clip.masks.len() {
                ui.add_space(6.0);
                let mask_id = clip.masks[idx].id.clone();
                let pre_edit = clip.masks.clone();

                Frame::none()
                    .fill(SUBCARD_BG)
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x24, 0x30)))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(Margin::symmetric(8.0, 6.0))
                    .show(ui, |ui| {
                        let m = &mut clip.masks[idx];
                        ui.horizontal(|ui| {
                            if ui.checkbox(&mut m.enabled, "").changed() {
                                commit_masks_snapshot = Some(pre_edit.clone());
                            }
                            ui.label(
                                RichText::new(format!("{} ({})", m.name, m.shape.label()))
                                    .size(11.5)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .add(
                                        Button::new(
                                            RichText::new("✕").size(10.5).color(Color32::WHITE),
                                        )
                                        .fill(Color32::from_rgb(0x3a, 0x20, 0x28))
                                        .rounding(Rounding::same(4.0)),
                                    )
                                    .on_hover_text("Maskeyi Sil")
                                    .clicked()
                                {
                                    remove_idx = Some(idx);
                                }
                                if ui
                                    .add(
                                        Button::new(
                                            RichText::new("⌖ Takip")
                                                .size(10.0)
                                                .strong()
                                                .color(Color32::WHITE),
                                        )
                                        .fill(Color32::from_rgb(0x1f, 0x36, 0x4d))
                                        .stroke(Stroke::new(1.0_f32, ACCENT_BLUE))
                                        .rounding(Rounding::same(4.0)),
                                    )
                                    .on_hover_text(
                                        "Seçili maske bölgesini klip boyunca otomatik takip ederek pos_x / pos_y keyframe'leri oluşturur",
                                    )
                                    .clicked()
                                {
                                    *action = Some(InspectorAction::TrackMaskMotion {
                                        track_id: track_id.to_string(),
                                        clip_id: clip_id.clone(),
                                        mask_index: idx,
                                    });
                                }
                            });
                        });

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            ui.allocate_ui_with_layout(
                                Vec2::new(76.0, 20.0),
                                Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.label(RichText::new("Hedef").size(11.0).color(LABEL_COLOR));
                                },
                            );
                            let mut tgt = m.target;
                            egui::ComboBox::from_id_salt(format!(
                                "mask_tgt_{}_{}",
                                clip_id, mask_id
                            ))
                            .selected_text(tgt.label())
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                for opt in [
                                    trim_core::MaskTarget::ClipAlpha,
                                    trim_core::MaskTarget::ColorAndEffects,
                                ] {
                                    if ui.selectable_value(&mut tgt, opt, opt.label()).clicked()
                                        && tgt != m.target
                                    {
                                        m.target = tgt;
                                        commit_masks_snapshot = Some(pre_edit.clone());
                                    }
                                }
                            });
                            if ui.checkbox(&mut m.invert, "Ters").changed() {
                                commit_masks_snapshot = Some(pre_edit.clone());
                            }
                        });

                        let eval = m.eval_at(clip_time);
                        let mut p_x = eval.pos_x as f64;
                        let mut p_y = eval.pos_y as f64;
                        let mut s_x = eval.scale_x as f64;
                        let mut s_y = eval.scale_y as f64;
                        let mut rot = eval.rotation_deg as f64;
                        let mut fth = eval.feather as f64;
                        let mut exp = eval.expansion as f64;
                        let mut op = eval.opacity as f64;

                        let mut drag_started = false;
                        let mut drag_stopped = false;
                        let mut any_changed = false;

                        let mut slider_row =
                            |ui: &mut Ui,
                             label: &str,
                             val: &mut f64,
                             range: std::ops::RangeInclusive<f64>,
                             prop: &str,
                             m_ref: &mut trim_core::Mask| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(76.0, 18.0),
                                        Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                RichText::new(label)
                                                    .size(10.5)
                                                    .color(LABEL_COLOR),
                                            );
                                        },
                                    );
                                    ui.spacing_mut().slider_width =
                                        (ui.available_width() - 80.0).max(48.0);
                                    let resp = ui.add(
                                        egui::Slider::new(val, range)
                                            .fixed_decimals(2)
                                            .trailing_fill(true)
                                            .show_value(true),
                                    );
                                    if resp.drag_started() {
                                        drag_started = true;
                                    }
                                    if resp.changed() {
                                        any_changed = true;
                                        match prop {
                                            "pos_x" => m_ref.pos_x = *val,
                                            "pos_y" => m_ref.pos_y = *val,
                                            "scale_x" => m_ref.scale_x = *val,
                                            "scale_y" => m_ref.scale_y = *val,
                                            "rotation" => m_ref.rotation = *val,
                                            "feather" => m_ref.feather = *val,
                                            "expansion" => m_ref.expansion = *val,
                                            "opacity" => m_ref.opacity = *val,
                                            _ => {}
                                        }
                                        if let Some(kfs) = m_ref.keyframes.get_mut(prop) {
                                            if let Some(kf) = kfs
                                                .iter_mut()
                                                .find(|k| (k.time - clip_time).abs() <= 0.05)
                                            {
                                                kf.value = *val;
                                            }
                                        }
                                    }
                                    if resp.drag_stopped() {
                                        drag_stopped = true;
                                    }
                                    if render_mask_kf_btn(ui, m_ref, prop, clip_time, *val) {
                                        commit_masks_snapshot = Some(pre_edit.clone());
                                    }
                                });
                            };

                        slider_row(ui, "Konum X", &mut p_x, 0.0..=1.0, "pos_x", m);
                        slider_row(ui, "Konum Y", &mut p_y, 0.0..=1.0, "pos_y", m);
                        slider_row(ui, "Genişlik", &mut s_x, 0.02..=2.0, "scale_x", m);
                        slider_row(ui, "Yükseklik", &mut s_y, 0.02..=2.0, "scale_y", m);
                        slider_row(ui, "Döndürme", &mut rot, -180.0..=180.0, "rotation", m);
                        slider_row(ui, "Yumuşaklık", &mut fth, 0.0..=0.5, "feather", m);
                        slider_row(ui, "Genişletme", &mut exp, -0.5..=0.5, "expansion", m);
                        slider_row(ui, "Opaklık", &mut op, 0.0..=1.0, "opacity", m);

                        if drag_started {
                            ui.data_mut(|d| d.insert_temp(drag_id, pre_edit.clone()));
                        }
                        if any_changed {
                            *action = Some(InspectorAction::ClipModified);
                        }
                        if drag_stopped {
                            if let Some(ref old_m) = orig_drag {
                                commit_masks_snapshot = Some(old_m.clone());
                                ui.data_mut(|d| d.remove::<Vec<trim_core::Mask>>(drag_id));
                            }
                        }
                    });
            }

            if let Some(rm_i) = remove_idx {
                let old_masks = clip.masks.clone();
                clip.masks.remove(rm_i);
                *action = Some(InspectorAction::SetMasks {
                    track_id: track_id.to_string(),
                    clip_id,
                    old_masks,
                    new_masks: clip.masks.clone(),
                });
            } else if let Some(old_masks) = commit_masks_snapshot {
                *action = Some(InspectorAction::SetMasks {
                    track_id: track_id.to_string(),
                    clip_id,
                    old_masks,
                    new_masks: clip.masks.clone(),
                });
            }
        },
    );
}
