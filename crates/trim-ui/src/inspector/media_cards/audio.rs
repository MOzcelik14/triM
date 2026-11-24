use super::super::{
    inspector_card, prop_row, InspectorAction, ACCENT_TEAL, LABEL_COLOR, SUBCARD_BG,
};
use egui::{
    Button, Color32, DragValue, Frame, Layout, Margin, Pos2, RichText, Rounding, Slider, Stroke,
    Ui, Vec2,
};
use trim_core::clip::{
    AudioCompressorConfig, AudioEqPreset, AudioVisualizerConfig, Clip, ParametricEqConfig,
    VisualizerStyle,
};

pub(in crate::inspector) fn render_audio_card(
    ui: &mut Ui,
    track_id: &str,
    clip: &mut Clip,
    action: &mut Option<InspectorAction>,
) {
    let pct = (clip.volume * 100.0).round() as i32;
    let is_muted = clip.muted;
    inspector_card(
        ui,
        "♪",
        "Ses & EQ Profili",
        ACCENT_TEAL,
        |ui| {
            ui.label(
                RichText::new(format!("{}%", pct))
                    .size(10.5)
                    .strong()
                    .color(if is_muted {
                        Color32::from_rgb(0xe8, 0x58, 0x58)
                    } else {
                        ACCENT_TEAL
                    }),
            );
        },
        |ui| {
            prop_row(ui, "Ses Düzeyi", |ui| {
                if ui
                    .add(
                        Slider::new(&mut clip.volume, 0.0..=3.0)
                            .show_value(false)
                            .trailing_fill(true),
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
                if ui
                    .add(
                        DragValue::new(&mut clip.volume)
                            .speed(0.02)
                            .range(0.0..=3.0)
                            .fixed_decimals(2),
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Gürültü Sil", |ui| {
                let s_resp = ui.add(
                    Slider::new(&mut clip.voice_denoise, 0.0..=1.0)
                        .show_value(false)
                        .trailing_fill(true),
                );
                let mut pct_dn = (clip.voice_denoise * 100.0).round() as i32;
                let d_resp = ui.add(
                    DragValue::new(&mut pct_dn)
                        .speed(1.0)
                        .range(0..=100)
                        .suffix("%"),
                );
                if d_resp.changed() {
                    clip.voice_denoise = (pct_dn as f32 / 100.0).clamp(0.0, 1.0);
                }
                if s_resp.changed() || d_resp.changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Oto. Kısma", |ui| {
                let duck_btn = Button::new(
                    RichText::new("∿ Konuşmada Müziği Kıs (Auto-Duck)")
                        .size(10.5)
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(0x1e, 0x36, 0x44))
                .stroke(Stroke::new(1.0_f32, ACCENT_TEAL))
                .rounding(Rounding::same(5.0))
                .min_size(Vec2::new(ui.available_width(), 22.0));
                if ui
                    .add(duck_btn)
                    .on_hover_text(
                        "Diğer kanallardaki diyalog/konuşma bölgelerini algılayarak bu klibin ses düzeyine otomatik kısma (ducking) keyframe'leri ekler",
                    )
                    .clicked()
                {
                    *action = Some(InspectorAction::AutoDuckAudio {
                        track_id: track_id.to_string(),
                        clip_id: clip.id.clone(),
                    });
                }
            });

            prop_row(ui, "EQ Profili", |ui| {
                egui::ComboBox::from_id_salt("clip_eq_preset")
                    .selected_text(clip.eq_preset.label())
                    .width(ui.available_width().max(110.0))
                    .show_ui(ui, |ui| {
                        for p in AudioEqPreset::ALL {
                            if ui
                                .selectable_label(clip.eq_preset == *p, p.label())
                                .clicked()
                                && clip.eq_preset != *p
                            {
                                let old_preset = clip.eq_preset;
                                clip.eq_preset = *p;
                                *action = Some(InspectorAction::SetEqPreset {
                                    track_id: track_id.to_string(),
                                    clip_id: clip.id.clone(),
                                    old_preset,
                                    new_preset: *p,
                                });
                            }
                        }
                    });
            });

            prop_row(ui, "Filtre & Durum", |ui| {
                if ui
                    .checkbox(&mut clip.noise_gate, "Gürültü Kapısı")
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
                if ui.checkbox(&mut clip.muted, "Sessiz").changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Stereo Pan", |ui| {
                ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(48.0);
                if ui
                    .add(
                        Slider::new(&mut clip.pan, -1.0..=1.0)
                            .show_value(true)
                            .trailing_fill(true)
                            .fixed_decimals(2),
                    )
                    .on_hover_text("-1.0 = Tam Sol (L), 0.0 = Merkez (C), +1.0 = Tam Sağ (R)")
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
                if ui
                    .small_button("C")
                    .on_hover_text("Merkeze Sıfırla")
                    .clicked()
                {
                    clip.pan = 0.0;
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Reverb (Oda)", |ui| {
                if ui
                    .add(
                        Slider::new(&mut clip.reverb_amount, 0.0..=1.0)
                            .show_value(true)
                            .trailing_fill(true)
                            .fixed_decimals(2),
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Echo (Yankı)", |ui| {
                if ui
                    .add(
                        Slider::new(&mut clip.echo_amount, 0.0..=1.0)
                            .show_value(true)
                            .trailing_fill(true)
                            .fixed_decimals(2),
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            // =================================================================
            // FAIRLIGHT 6-BAND PARAMETRIC EQ
            // =================================================================
            ui.add_space(4.0);
            Frame::none()
                .fill(SUBCARD_BG)
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x24, 0x30)))
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
                    let mut has_peq = clip.parametric_eq.is_some();
                    if ui
                        .checkbox(
                            &mut has_peq,
                            RichText::new("≡ 6-Bant Parametrik EQ (Fairlight)")
                                .size(11.0)
                                .strong(),
                        )
                        .changed()
                    {
                        clip.parametric_eq = if has_peq {
                            Some(ParametricEqConfig::default())
                        } else {
                            None
                        };
                        *action = Some(InspectorAction::ClipModified);
                    }
                    if let Some(peq) = &mut clip.parametric_eq {
                        // Draw frequency response curve
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 54.0),
                            egui::Sense::hover(),
                        );
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(rect, 4.0, Color32::from_rgb(0x0b, 0x0d, 0x12));
                        painter.rect_stroke(
                            rect,
                            4.0,
                            Stroke::new(1.0_f32, Color32::from_rgb(0x24, 0x2a, 0x38)),
                        );
                        let mid_y = rect.center().y;
                        painter.line_segment(
                            [
                                Pos2::new(rect.left(), mid_y),
                                Pos2::new(rect.right(), mid_y),
                            ],
                            Stroke::new(
                                1.0_f32,
                                Color32::from_rgba_unmultiplied(255, 255, 255, 26),
                            ),
                        );
                        let steps = 48usize;
                        let mut prev = None;
                        for s in 0..=steps {
                            let frac = s as f32 / steps as f32;
                            // Log frequency 20 Hz .. 20000 Hz
                            let freq = 20.0_f32 * (1000.0_f32).powf(frac);
                            let mut db_sum = 0.0_f32;
                            for b in &peq.bands {
                                if b.enabled {
                                    let log_dist = (freq / b.freq_hz.max(20.0)).log2();
                                    let bw = (1.0 / b.q.max(0.2)).max(0.15);
                                    db_sum += b.gain_db * (-0.5 * (log_dist / bw).powi(2)).exp();
                                }
                            }
                            let norm_y = (db_sum / 18.0).clamp(-1.0, 1.0);
                            let p = Pos2::new(
                                rect.left() + frac * rect.width(),
                                mid_y - norm_y * (rect.height() * 0.44),
                            );
                            if let Some(pr) = prev {
                                painter.line_segment([pr, p], Stroke::new(1.6_f32, ACCENT_TEAL));
                            }
                            prev = Some(p);
                        }

                        let band_names = ["80Hz", "200Hz", "650Hz", "2kHz", "5.5kHz", "12kHz"];
                        for (i, band) in peq.bands.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 5.0;
                                if ui.checkbox(&mut band.enabled, "").changed() {
                                    *action = Some(InspectorAction::ClipModified);
                                }
                                ui.allocate_ui_with_layout(
                                    Vec2::new(42.0, 18.0),
                                    Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(
                                                band_names.get(i).copied().unwrap_or("Band"),
                                            )
                                            .size(10.0)
                                            .color(LABEL_COLOR),
                                        );
                                    },
                                );
                                ui.spacing_mut().slider_width =
                                    (ui.available_width() - 98.0).max(46.0);
                                if ui
                                    .add(
                                        Slider::new(&mut band.gain_db, -18.0..=18.0)
                                            .suffix(" dB")
                                            .trailing_fill(true)
                                            .fixed_decimals(1),
                                    )
                                    .changed()
                                {
                                    *action = Some(InspectorAction::ClipModified);
                                }
                                if ui
                                    .add(
                                        DragValue::new(&mut band.q)
                                            .speed(0.05)
                                            .range(0.3..=8.0)
                                            .prefix("Q:"),
                                    )
                                    .changed()
                                {
                                    *action = Some(InspectorAction::ClipModified);
                                }
                            });
                        }
                    }
                });

            // =================================================================
            // FAIRLIGHT DYNAMICS (COMPRESSOR / DE-ESSER / LIMITER)
            // =================================================================
            ui.add_space(4.0);
            Frame::none()
                .fill(SUBCARD_BG)
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x24, 0x30)))
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
                    let mut has_comp = clip.compressor.is_some();
                    if ui
                        .checkbox(
                            &mut has_comp,
                            RichText::new("◈ Dinamik İşlemci (Compressor / Limiter / De-Esser)")
                                .size(11.0)
                                .strong(),
                        )
                        .changed()
                    {
                        clip.compressor = if has_comp {
                            Some(AudioCompressorConfig {
                                enabled: true,
                                ..AudioCompressorConfig::default()
                            })
                        } else {
                            None
                        };
                        *action = Some(InspectorAction::ClipModified);
                    }
                    if let Some(comp) = &mut clip.compressor {
                        comp.enabled = true;
                        prop_row(ui, "Eşik (Thresh)", |ui| {
                            if ui
                                .add(
                                    Slider::new(&mut comp.threshold_db, -48.0..=0.0)
                                        .suffix(" dB")
                                        .trailing_fill(true),
                                )
                                .changed()
                            {
                                *action = Some(InspectorAction::ClipModified);
                            }
                        });
                        prop_row(ui, "Oran (Ratio)", |ui| {
                            if ui
                                .add(
                                    Slider::new(&mut comp.ratio, 1.0..=20.0)
                                        .suffix(":1")
                                        .trailing_fill(true),
                                )
                                .changed()
                            {
                                *action = Some(InspectorAction::ClipModified);
                            }
                        });
                        prop_row(ui, "Attack / Rel", |ui| {
                            let mut ch = false;
                            ch |= ui
                                .add(
                                    DragValue::new(&mut comp.attack_ms)
                                        .speed(1.0)
                                        .range(1.0..=200.0)
                                        .suffix("ms"),
                                )
                                .changed();
                            ch |= ui
                                .add(
                                    DragValue::new(&mut comp.release_ms)
                                        .speed(5.0)
                                        .range(10.0..=1000.0)
                                        .suffix("ms"),
                                )
                                .changed();
                            if ch {
                                *action = Some(InspectorAction::ClipModified);
                            }
                        });
                        prop_row(ui, "Makeup Kazanç", |ui| {
                            if ui
                                .add(
                                    Slider::new(&mut comp.makeup_db, 0.0..=24.0)
                                        .suffix(" dB")
                                        .trailing_fill(true),
                                )
                                .changed()
                            {
                                *action = Some(InspectorAction::ClipModified);
                            }
                        });
                        prop_row(ui, "De-Esser (S/Ş)", |ui| {
                            if ui
                                .add(
                                    Slider::new(&mut comp.de_esser, 0.0..=1.0)
                                        .trailing_fill(true)
                                        .fixed_decimals(2),
                                )
                                .on_hover_text("6.5 kHz sert sibilans seslerini yumuşatır")
                                .changed()
                            {
                                *action = Some(InspectorAction::ClipModified);
                            }
                        });
                        if ui
                            .checkbox(
                                &mut comp.limiter_enabled,
                                "Brickwall Limiter (-1.0 dBFS Tavan)",
                            )
                            .changed()
                        {
                            *action = Some(InspectorAction::ClipModified);
                        }
                    }
                });

            ui.add_space(2.0);
            Frame::none()
                .fill(SUBCARD_BG)
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x24, 0x30)))
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
                    let mut has_vis = clip.audio_visualizer.is_some();
                    if ui
                        .checkbox(
                            &mut has_vis,
                            RichText::new("∿ Görsel Ses Dalgası (Audio Visualizer)")
                                .size(11.0)
                                .strong(),
                        )
                        .changed()
                    {
                        clip.audio_visualizer = if has_vis {
                            Some(AudioVisualizerConfig::default())
                        } else {
                            None
                        };
                        *action = Some(InspectorAction::ClipModified);
                    }

                    if let Some(vis) = &mut clip.audio_visualizer {
                        ui.add_space(4.0);
                        prop_row(ui, "Dalga Stili", |ui| {
                            egui::ComboBox::from_id_salt("vis_style_combo")
                                .selected_text(vis.style.label())
                                .width(ui.available_width().max(110.0))
                                .show_ui(ui, |ui| {
                                    for st in VisualizerStyle::ALL {
                                        if ui
                                            .selectable_value(&mut vis.style, *st, st.label())
                                            .clicked()
                                        {
                                            *action = Some(InspectorAction::ClipModified);
                                        }
                                    }
                                });
                        });

                        prop_row(ui, "Dikey Konum", |ui| {
                            if ui
                                .add(
                                    Slider::new(&mut vis.pos_y, 0.0..=1.0)
                                        .show_value(false)
                                        .trailing_fill(true),
                                )
                                .changed()
                            {
                                *action = Some(InspectorAction::ClipModified);
                            }
                        });

                        prop_row(ui, "Dalga Rengi", |ui| {
                            let mut col = Color32::from_rgba_unmultiplied(
                                vis.color[0],
                                vis.color[1],
                                vis.color[2],
                                vis.color[3],
                            );
                            if ui.color_edit_button_srgba(&mut col).changed() {
                                vis.color = [col.r(), col.g(), col.b(), col.a()];
                                *action = Some(InspectorAction::ClipModified);
                            }
                        });
                    }
                });
        },
    );
}
