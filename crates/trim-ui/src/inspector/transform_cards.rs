use super::{
    inspector_card, prop_row, render_kf_btn, sync_kf_value, InspectorAction, ACCENT_AMBER,
    LABEL_COLOR,
};
use egui::{Button, Color32, DragValue, Pos2, RichText, Rounding, Slider, Stroke, Ui, Vec2};
use trim_core::clip::{Clip, PanZoomPreset};
use trim_core::keyframe::Interpolation;

pub(super) fn render_transform_card(
    ui: &mut Ui,
    track_id: &str,
    clip: &mut Clip,
    clip_time: f64,
    action: &mut Option<InspectorAction>,
) {
    let is_modified = clip.pos_x.abs() > 0.01
        || clip.pos_y.abs() > 0.01
        || (clip.scale - 1.0).abs() > 0.005
        || clip.rotation.abs() > 0.01
        || (clip.opacity - 1.0).abs() > 0.005;

    let tf_id = ui.make_persistent_id(format!("tf_drag_{}", clip.id));
    let mut orig_tf: Option<(f64, f64, f64, f64, f64)> = ui.data_mut(|d| d.get_temp(tf_id));

    let record_start = |ui: &mut Ui, orig: &mut Option<(f64, f64, f64, f64, f64)>, clip: &Clip| {
        if orig.is_none() {
            let snap = (
                clip.pos_x,
                clip.pos_y,
                clip.scale,
                clip.rotation,
                clip.opacity,
            );
            *orig = Some(snap);
            ui.data_mut(|d| d.insert_temp(tf_id, snap));
        }
    };

    let check_stop = |ui: &mut Ui,
                      orig: &mut Option<(f64, f64, f64, f64, f64)>,
                      clip: &Clip,
                      action: &mut Option<InspectorAction>| {
        if let Some(old) = *orig {
            let new = (
                clip.pos_x,
                clip.pos_y,
                clip.scale,
                clip.rotation,
                clip.opacity,
            );
            if (old.0 - new.0).abs() > 1e-4
                || (old.1 - new.1).abs() > 1e-4
                || (old.2 - new.2).abs() > 1e-4
                || (old.3 - new.3).abs() > 1e-4
                || (old.4 - new.4).abs() > 1e-4
            {
                *action = Some(InspectorAction::SetTransform {
                    track_id: track_id.to_string(),
                    clip_id: clip.id.clone(),
                    old_transform: old,
                    new_transform: new,
                });
            }
            *orig = None;
            ui.data_mut(|d| d.remove::<(f64, f64, f64, f64, f64)>(tf_id));
        }
    };

    let reset_clicked = std::cell::Cell::new(false);
    inspector_card(
        ui,
        "◈",
        "Video Dönüşümü & Keyframe",
        ACCENT_AMBER,
        |ui| {
            if is_modified
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
                    .on_hover_text("Konum, ölçek ve opaklığı varsayılana sıfırla")
                    .clicked()
            {
                reset_clicked.set(true);
            }
        },
        |ui| {
            if reset_clicked.get() {
                let old = (
                    clip.pos_x,
                    clip.pos_y,
                    clip.scale,
                    clip.rotation,
                    clip.opacity,
                );
                let new = (0.0, 0.0, 1.0, 0.0, 1.0);
                clip.pos_x = 0.0;
                clip.pos_y = 0.0;
                clip.scale = 1.0;
                clip.rotation = 0.0;
                clip.opacity = 1.0;
                *action = Some(InspectorAction::SetTransform {
                    track_id: track_id.to_string(),
                    clip_id: clip.id.clone(),
                    old_transform: old,
                    new_transform: new,
                });
            }

            prop_row(ui, "Konum X", |ui| {
                let r = ui.add(DragValue::new(&mut clip.pos_x).speed(1.0).suffix(" px"));
                if r.drag_started() {
                    record_start(ui, &mut orig_tf, clip);
                }
                if r.changed() {
                    sync_kf_value(clip, "pos_x", clip_time, clip.pos_x);
                    *action = Some(InspectorAction::ClipModified);
                }
                if r.drag_stopped() || r.lost_focus() {
                    check_stop(ui, &mut orig_tf, clip, action);
                }
                render_kf_btn(ui, clip, "pos_x", clip_time, clip.pos_x, action);
            });

            prop_row(ui, "Konum Y", |ui| {
                let r = ui.add(DragValue::new(&mut clip.pos_y).speed(1.0).suffix(" px"));
                if r.drag_started() {
                    record_start(ui, &mut orig_tf, clip);
                }
                if r.changed() {
                    sync_kf_value(clip, "pos_y", clip_time, clip.pos_y);
                    *action = Some(InspectorAction::ClipModified);
                }
                if r.drag_stopped() || r.lost_focus() {
                    check_stop(ui, &mut orig_tf, clip, action);
                }
                render_kf_btn(ui, clip, "pos_y", clip_time, clip.pos_y, action);
            });

            prop_row(ui, "Ölçek", |ui| {
                ui.spacing_mut().slider_width = (ui.available_width() - 84.0).max(48.0);
                let r1 = ui.add(
                    Slider::new(&mut clip.scale, 0.1..=4.0)
                        .show_value(false)
                        .trailing_fill(true),
                );
                if r1.drag_started() {
                    record_start(ui, &mut orig_tf, clip);
                }
                if r1.changed() {
                    sync_kf_value(clip, "scale", clip_time, clip.scale);
                    *action = Some(InspectorAction::ClipModified);
                }
                if r1.drag_stopped() || r1.lost_focus() {
                    check_stop(ui, &mut orig_tf, clip, action);
                }

                let r2 = ui.add(
                    DragValue::new(&mut clip.scale)
                        .speed(0.01)
                        .range(0.05..=10.0)
                        .fixed_decimals(2),
                );
                if r2.drag_started() {
                    record_start(ui, &mut orig_tf, clip);
                }
                if r2.changed() {
                    sync_kf_value(clip, "scale", clip_time, clip.scale);
                    *action = Some(InspectorAction::ClipModified);
                }
                if r2.drag_stopped() || r2.lost_focus() {
                    check_stop(ui, &mut orig_tf, clip, action);
                }
                render_kf_btn(ui, clip, "scale", clip_time, clip.scale, action);
            });

            prop_row(ui, "Döndürme", |ui| {
                let r = ui.add(
                    DragValue::new(&mut clip.rotation)
                        .speed(0.5)
                        .suffix("°")
                        .fixed_decimals(1),
                );
                if r.drag_started() {
                    record_start(ui, &mut orig_tf, clip);
                }
                if r.changed() {
                    sync_kf_value(clip, "rotation", clip_time, clip.rotation);
                    *action = Some(InspectorAction::ClipModified);
                }
                if r.drag_stopped() || r.lost_focus() {
                    check_stop(ui, &mut orig_tf, clip, action);
                }
                render_kf_btn(ui, clip, "rotation", clip_time, clip.rotation, action);
            });

            prop_row(ui, "Opaklık", |ui| {
                ui.spacing_mut().slider_width = (ui.available_width() - 84.0).max(48.0);
                let r1 = ui.add(
                    Slider::new(&mut clip.opacity, 0.0..=1.0)
                        .show_value(false)
                        .trailing_fill(true),
                );
                if r1.drag_started() {
                    record_start(ui, &mut orig_tf, clip);
                }
                if r1.changed() {
                    sync_kf_value(clip, "opacity", clip_time, clip.opacity);
                    *action = Some(InspectorAction::ClipModified);
                }
                if r1.drag_stopped() || r1.lost_focus() {
                    check_stop(ui, &mut orig_tf, clip, action);
                }

                let r2 = ui.add(
                    DragValue::new(&mut clip.opacity)
                        .speed(0.01)
                        .range(0.0..=1.0)
                        .fixed_decimals(2),
                );
                if r2.drag_started() {
                    record_start(ui, &mut orig_tf, clip);
                }
                if r2.changed() {
                    sync_kf_value(clip, "opacity", clip_time, clip.opacity);
                    *action = Some(InspectorAction::ClipModified);
                }
                if r2.drag_stopped() || r2.lost_focus() {
                    check_stop(ui, &mut orig_tf, clip, action);
                }
                render_kf_btn(ui, clip, "opacity", clip_time, clip.opacity, action);
            });

            prop_row(ui, "Ken Burns", |ui| {
                egui::ComboBox::from_id_salt("ken_burns_preset")
                    .selected_text("Şablon Seç / Uygula...")
                    .width(ui.available_width().max(110.0))
                    .show_ui(ui, |ui| {
                        for preset in PanZoomPreset::ALL {
                            if ui.button(preset.label()).clicked() {
                                clip.apply_pan_zoom_preset(*preset);
                                *action = Some(InspectorAction::ClipModified);
                            }
                        }
                    });
            });
        },
    );
}

pub(super) fn render_crop_card(
    ui: &mut Ui,
    track_id: &str,
    clip: &mut Clip,
    action: &mut Option<InspectorAction>,
) {
    let crop_id = ui.make_persistent_id(format!("crop_drag_{}", clip.id));
    let mut orig_crop: Option<(f32, f32, f32, f32)> = ui.data_mut(|d| d.get_temp(crop_id));

    let record_start = |ui: &mut Ui, orig: &mut Option<(f32, f32, f32, f32)>, clip: &Clip| {
        if orig.is_none() {
            let snap = (
                clip.crop_left,
                clip.crop_right,
                clip.crop_top,
                clip.crop_bottom,
            );
            *orig = Some(snap);
            ui.data_mut(|d| d.insert_temp(crop_id, snap));
        }
    };

    let check_stop = |ui: &mut Ui,
                      orig: &mut Option<(f32, f32, f32, f32)>,
                      clip: &Clip,
                      action: &mut Option<InspectorAction>| {
        if let Some(old) = *orig {
            let new = (
                clip.crop_left,
                clip.crop_right,
                clip.crop_top,
                clip.crop_bottom,
            );
            if (old.0 - new.0).abs() > 1e-4
                || (old.1 - new.1).abs() > 1e-4
                || (old.2 - new.2).abs() > 1e-4
                || (old.3 - new.3).abs() > 1e-4
            {
                *action = Some(InspectorAction::SetCrop {
                    track_id: track_id.to_string(),
                    clip_id: clip.id.clone(),
                    old_crop: old,
                    new_crop: new,
                });
            }
            *orig = None;
            ui.data_mut(|d| d.remove::<(f32, f32, f32, f32)>(crop_id));
        }
    };

    let is_modified = clip.has_crop();
    let reset_clicked = std::cell::Cell::new(false);

    inspector_card(
        ui,
        "✂",
        "Video Kırpma (Crop)",
        ACCENT_AMBER,
        |ui| {
            if is_modified
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
                    .on_hover_text("Kırpmayı sıfırla")
                    .clicked()
            {
                reset_clicked.set(true);
            }
        },
        |ui| {
            if reset_clicked.get() {
                let old = (
                    clip.crop_left,
                    clip.crop_right,
                    clip.crop_top,
                    clip.crop_bottom,
                );
                let new = (0.0, 0.0, 0.0, 0.0);
                clip.crop_left = 0.0;
                clip.crop_right = 0.0;
                clip.crop_top = 0.0;
                clip.crop_bottom = 0.0;
                *action = Some(InspectorAction::SetCrop {
                    track_id: track_id.to_string(),
                    clip_id: clip.id.clone(),
                    old_crop: old,
                    new_crop: new,
                });
            }

            // Sol
            prop_row(ui, "Sol", |ui| {
                let max_l = (1.0 - clip.crop_right - 0.01).max(0.0);
                let r1 = ui.add(
                    Slider::new(&mut clip.crop_left, 0.0..=max_l)
                        .show_value(false)
                        .trailing_fill(true),
                );
                if r1.drag_started() {
                    record_start(ui, &mut orig_crop, clip);
                }
                if r1.changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
                if r1.drag_stopped() || r1.lost_focus() {
                    check_stop(ui, &mut orig_crop, clip, action);
                }

                let mut pct = (clip.crop_left * 100.0).round();
                let r2 = ui.add(
                    DragValue::new(&mut pct)
                        .speed(0.5)
                        .range(0.0..=(max_l * 100.0))
                        .suffix(" %")
                        .fixed_decimals(0),
                );
                if r2.drag_started() {
                    record_start(ui, &mut orig_crop, clip);
                }
                if r2.changed() {
                    clip.crop_left = (pct / 100.0).clamp(0.0, max_l);
                    *action = Some(InspectorAction::ClipModified);
                }
                if r2.drag_stopped() || r2.lost_focus() {
                    check_stop(ui, &mut orig_crop, clip, action);
                }
            });

            // Sağ
            prop_row(ui, "Sağ", |ui| {
                let max_r = (1.0 - clip.crop_left - 0.01).max(0.0);
                let r1 = ui.add(
                    Slider::new(&mut clip.crop_right, 0.0..=max_r)
                        .show_value(false)
                        .trailing_fill(true),
                );
                if r1.drag_started() {
                    record_start(ui, &mut orig_crop, clip);
                }
                if r1.changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
                if r1.drag_stopped() || r1.lost_focus() {
                    check_stop(ui, &mut orig_crop, clip, action);
                }

                let mut pct = (clip.crop_right * 100.0).round();
                let r2 = ui.add(
                    DragValue::new(&mut pct)
                        .speed(0.5)
                        .range(0.0..=(max_r * 100.0))
                        .suffix(" %")
                        .fixed_decimals(0),
                );
                if r2.drag_started() {
                    record_start(ui, &mut orig_crop, clip);
                }
                if r2.changed() {
                    clip.crop_right = (pct / 100.0).clamp(0.0, max_r);
                    *action = Some(InspectorAction::ClipModified);
                }
                if r2.drag_stopped() || r2.lost_focus() {
                    check_stop(ui, &mut orig_crop, clip, action);
                }
            });

            // Üst
            prop_row(ui, "Üst", |ui| {
                let max_t = (1.0 - clip.crop_bottom - 0.01).max(0.0);
                let r1 = ui.add(
                    Slider::new(&mut clip.crop_top, 0.0..=max_t)
                        .show_value(false)
                        .trailing_fill(true),
                );
                if r1.drag_started() {
                    record_start(ui, &mut orig_crop, clip);
                }
                if r1.changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
                if r1.drag_stopped() || r1.lost_focus() {
                    check_stop(ui, &mut orig_crop, clip, action);
                }

                let mut pct = (clip.crop_top * 100.0).round();
                let r2 = ui.add(
                    DragValue::new(&mut pct)
                        .speed(0.5)
                        .range(0.0..=(max_t * 100.0))
                        .suffix(" %")
                        .fixed_decimals(0),
                );
                if r2.drag_started() {
                    record_start(ui, &mut orig_crop, clip);
                }
                if r2.changed() {
                    clip.crop_top = (pct / 100.0).clamp(0.0, max_t);
                    *action = Some(InspectorAction::ClipModified);
                }
                if r2.drag_stopped() || r2.lost_focus() {
                    check_stop(ui, &mut orig_crop, clip, action);
                }
            });

            // Alt
            prop_row(ui, "Alt", |ui| {
                let max_b = (1.0 - clip.crop_top - 0.01).max(0.0);
                let r1 = ui.add(
                    Slider::new(&mut clip.crop_bottom, 0.0..=max_b)
                        .show_value(false)
                        .trailing_fill(true),
                );
                if r1.drag_started() {
                    record_start(ui, &mut orig_crop, clip);
                }
                if r1.changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
                if r1.drag_stopped() || r1.lost_focus() {
                    check_stop(ui, &mut orig_crop, clip, action);
                }

                let mut pct = (clip.crop_bottom * 100.0).round();
                let r2 = ui.add(
                    DragValue::new(&mut pct)
                        .speed(0.5)
                        .range(0.0..=(max_b * 100.0))
                        .suffix(" %")
                        .fixed_decimals(0),
                );
                if r2.drag_started() {
                    record_start(ui, &mut orig_crop, clip);
                }
                if r2.changed() {
                    clip.crop_bottom = (pct / 100.0).clamp(0.0, max_b);
                    *action = Some(InspectorAction::ClipModified);
                }
                if r2.drag_stopped() || r2.lost_focus() {
                    check_stop(ui, &mut orig_crop, clip, action);
                }
            });
        },
    );
}

pub(super) fn render_speed_card(
    ui: &mut Ui,
    clip: &mut Clip,
    clip_time: f64,
    action: &mut Option<InspectorAction>,
) {
    let curr_speed = clip.get_property_at_time("speed", clip_time);
    let has_ramp = clip.has_speed_ramp();
    inspector_card(
        ui,
        "▸",
        "Hız, Yön & Akıllı Kurgu",
        ACCENT_AMBER,
        |ui| {
            let badge_txt = if has_ramp {
                format!("∿ {:.2}x", curr_speed)
            } else {
                format!("{:.2}x", curr_speed)
            };
            ui.label(
                RichText::new(badge_txt)
                    .size(10.5)
                    .strong()
                    .color(ACCENT_AMBER),
            );
        },
        |ui| {
            prop_row(ui, "Oynatma Hızı", |ui| {
                let mut s = if has_ramp { curr_speed } else { clip.speed };
                if ui
                    .add(
                        DragValue::new(&mut s)
                            .speed(0.05)
                            .range(0.1..=10.0)
                            .suffix("x"),
                    )
                    .changed()
                {
                    if has_ramp {
                        clip.add_keyframe("speed", clip_time, s, Interpolation::EaseInOut);
                        *action = Some(InspectorAction::ClipModified);
                    } else {
                        clip.change_speed(s);
                        *action = Some(InspectorAction::ChangeSpeed(s, clip.reverse));
                    }
                }

                for preset in [0.5, 1.0, 1.5, 2.0] {
                    let is_active = !has_ramp && (clip.speed - preset).abs() < 0.03;
                    let pill = Button::new(
                        RichText::new(format!("{:.1}x", preset))
                            .size(10.0)
                            .strong()
                            .color(if is_active {
                                Color32::WHITE
                            } else {
                                LABEL_COLOR
                            }),
                    )
                    .fill(if is_active {
                        ACCENT_AMBER
                    } else {
                        Color32::from_rgb(0x1f, 0x23, 0x30)
                    })
                    .rounding(Rounding::same(4.0));
                    if ui.add_sized([28.0, 20.0], pill).clicked() {
                        clip.keyframes.remove("speed");
                        clip.change_speed(preset);
                        *action = Some(InspectorAction::ChangeSpeed(preset, clip.reverse));
                    }
                }
                render_kf_btn(ui, clip, "speed", clip_time, curr_speed, action);
            });

            prop_row(ui, "Stabilizasyon", |ui| {
                let s_resp = ui.add(
                    Slider::new(&mut clip.stabilization_strength, 0.0..=1.0)
                        .show_value(false)
                        .trailing_fill(true),
                );
                let mut pct_stab = (clip.stabilization_strength * 100.0).round() as i32;
                let d_resp = ui.add(
                    DragValue::new(&mut pct_stab)
                        .speed(1.0)
                        .range(0..=100)
                        .suffix("%"),
                );
                if d_resp.changed() {
                    clip.stabilization_strength = (pct_stab as f32 / 100.0).clamp(0.0, 1.0);
                }
                if s_resp.changed() || d_resp.changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Hız Eğrisi (Ramp)", |ui| {
                let ramp_label = if has_ramp {
                    "∿ Dinamik Hız Eğrisi Aktif"
                } else {
                    "Sabit Hız (Şablon Seç...)"
                };
                egui::ComboBox::from_id_salt(format!("speed_ramp_{}", clip.id))
                    .selected_text(ramp_label)
                    .width(ui.available_width().max(110.0))
                    .show_ui(ui, |ui| {
                        for (key, lbl) in [
                            ("none", "Sabit Hız (Eğriyi Temizle)"),
                            ("bullet_time", "▸ Bullet Time (Hızlı → Yavaş → Hızlı)"),
                            ("action_burst", "▸ Aksiyon Patlaması (Ortada Hızlı)"),
                            ("ramp_up", "▸ İvmelenen (Yavaştan Hızlıya)"),
                            ("ramp_down", "▸ Frenleyen (Hızlıdan Yavaşa)"),
                        ] {
                            if ui.button(lbl).clicked() {
                                clip.apply_speed_ramp_preset(key);
                                *action = Some(InspectorAction::ClipModified);
                            }
                        }
                    });
            });

            if has_ramp {
                let dur = clip.duration().max(0.1);
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 40.0),
                    egui::Sense::hover(),
                );
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 4.0, Color32::from_rgb(0x12, 0x15, 0x1e));
                painter.rect_stroke(
                    rect,
                    4.0,
                    Stroke::new(1.0_f32, Color32::from_rgb(0x24, 0x2a, 0x38)),
                );
                // 1.0x reference line
                let y_1x = rect.bottom() - (1.0 / 3.5) * rect.height();
                painter.line_segment(
                    [Pos2::new(rect.left(), y_1x), Pos2::new(rect.right(), y_1x)],
                    Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 30)),
                );

                let steps = 40usize;
                let mut pts = Vec::with_capacity(steps + 1);
                for i in 0..=steps {
                    let frac = i as f32 / steps as f32;
                    let t = (frac as f64) * dur;
                    let sp = clip.get_property_at_time("speed", t).clamp(0.1, 3.5) as f32;
                    let px = rect.left() + frac * rect.width();
                    let py = rect.bottom() - (sp / 3.5) * (rect.height() - 4.0) - 2.0;
                    pts.push(Pos2::new(px, py));
                }
                for w in pts.windows(2) {
                    painter.line_segment([w[0], w[1]], Stroke::new(1.8_f32, ACCENT_AMBER));
                }
                // Playhead marker on speed curve
                let ph_frac = (clip_time / dur).clamp(0.0, 1.0) as f32;
                let ph_x = rect.left() + ph_frac * rect.width();
                painter.line_segment(
                    [Pos2::new(ph_x, rect.top()), Pos2::new(ph_x, rect.bottom())],
                    Stroke::new(1.2_f32, Color32::WHITE),
                );
            }

            prop_row(ui, "Oynatma Yönü", |ui| {
                let mut rev = clip.reverse;
                if ui.checkbox(&mut rev, "Ters Oynat (Reverse)").changed() {
                    clip.reverse = rev;
                    *action = Some(InspectorAction::ChangeSpeed(clip.speed, rev));
                }
                if ui
                    .checkbox(&mut clip.pitch_lock, "Ses Perdesini Koru (Pitch Lock)")
                    .on_hover_text("Hız değiştiğinde sesin incelmesini/kalınlaşmasını önler")
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            if !clip.media_id.is_empty() {
                ui.add_space(4.0);
                let btn_scene = Button::new(
                    RichText::new("✂  Sahneleri Algıla ve Böl (Smart Cut)")
                        .size(11.0)
                        .strong()
                        .color(Color32::from_rgb(0xf5, 0x9e, 0x42)),
                )
                .fill(Color32::from_rgba_unmultiplied(0xe0, 0x7a, 0x38, 26))
                .stroke(Stroke::new(
                    1.0_f32,
                    Color32::from_rgba_unmultiplied(0xe0, 0x7a, 0x38, 120),
                ))
                .rounding(Rounding::same(5.0))
                .min_size(Vec2::new(ui.available_width(), 26.0));
                if ui.add(btn_scene).clicked() {
                    *action = Some(InspectorAction::TriggerSceneDetect);
                }

                let btn_silence = Button::new(
                    RichText::new("∿  Sessizlikleri Otomatik Kes (Jump-Cut)")
                        .size(11.0)
                        .strong()
                        .color(Color32::from_rgb(0x4a, 0xde, 0x80)),
                )
                .fill(Color32::from_rgba_unmultiplied(0x2d, 0xb8, 0x9a, 24))
                .stroke(Stroke::new(
                    1.0_f32,
                    Color32::from_rgba_unmultiplied(0x2d, 0xb8, 0x9a, 120),
                ))
                .rounding(Rounding::same(5.0))
                .min_size(Vec2::new(ui.available_width(), 26.0));
                if ui.add(btn_silence).clicked() {
                    *action = Some(InspectorAction::TriggerSilenceDetect);
                }
            }
        },
    );
}
