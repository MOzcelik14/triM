use crate::inspector::{
    prop_row, render_badge, InspectorAction, ACCENT_AMBER, ACCENT_BLUE, LABEL_COLOR, LABEL_COL_W,
    SUBCARD_BG,
};
use egui::{
    Button, Color32, DragValue, Frame, Layout, Margin, Pos2, RichText, Rounding, Slider, Stroke,
    Ui, Vec2,
};
use trim_commands::commands::ClipColorGradingState;
use trim_core::clip::{Clip, FilmStockFormat};
use trim_core::color::{CustomCurvesConfig, HslCurvesConfig, HslQualifierConfig};

pub(super) fn render_curves_and_film_section(
    ui: &mut Ui,
    track_id: &str,
    clip: &mut Clip,
    action: &mut Option<InspectorAction>,
) {
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
                    Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 30)),
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
                        let btn =
                            Button::new(RichText::new(lbl).size(9.5).strong().color(if is_sel {
                                Color32::WHITE
                            } else {
                                LABEL_COLOR
                            }))
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
                            ui.spacing_mut().slider_width = (ui.available_width() - 56.0).max(60.0);
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
                        .add(Slider::new(&mut q.luma_offset, -0.5..=0.5).trailing_fill(true))
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
}
