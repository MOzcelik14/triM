use super::{
    inspector_card, prop_row, render_badge, render_kf_btn, render_mask_kf_btn, sync_kf_value,
    InspectorAction, ACCENT_AMBER, ACCENT_BLUE, ACCENT_TEAL, LABEL_COLOR, SUBCARD_BG,
};
use egui::{
    Button, Color32, DragValue, Frame, Layout, Margin, Pos2, RichText, Rounding, Slider, Stroke,
    TextEdit, Ui, Vec2,
};
use trim_core::clip::{
    AudioCompressorConfig, AudioEqPreset, AudioVisualizerConfig, CaptionStyle, Clip,
    CornerPinConfig, MotionTitleStyle, ParametricEqConfig, TextOverlay, VisualizerStyle,
};

pub(super) fn render_transitions_card(
    ui: &mut Ui,
    clip: &mut Clip,
    action: &mut Option<InspectorAction>,
) {
    inspector_card(
        ui,
        "◈",
        "Giriş / Çıkış ve Geçişler",
        ACCENT_BLUE,
        |_| {},
        |ui| {
            prop_row(ui, "Giriş Kararma", |ui| {
                if ui
                    .add(
                        DragValue::new(&mut clip.fade_in)
                            .speed(0.1)
                            .range(0.0..=60.0)
                            .suffix(" sn"),
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Giriş Geçişi", |ui| {
                egui::ComboBox::from_id_salt("trans_in")
                    .selected_text(Clip::transition_label(clip.transition_in.as_deref()))
                    .width(ui.available_width().max(110.0))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(clip.transition_in.is_none(), "Yok (Düz)")
                            .clicked()
                        {
                            clip.transition_in = None;
                            *action = Some(InspectorAction::ClipModified);
                        }
                        for (key, label) in Clip::SUPPORTED_TRANSITIONS {
                            let is_sel = clip.transition_in.as_deref() == Some(*key);
                            if ui.selectable_label(is_sel, *label).clicked() {
                                clip.transition_in = Some(key.to_string());
                                *action = Some(InspectorAction::ClipModified);
                            }
                        }
                    });
            });

            prop_row(ui, "Çıkış Kararma", |ui| {
                if ui
                    .add(
                        DragValue::new(&mut clip.fade_out)
                            .speed(0.1)
                            .range(0.0..=60.0)
                            .suffix(" sn"),
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Çıkış Geçişi", |ui| {
                egui::ComboBox::from_id_salt("trans_out")
                    .selected_text(Clip::transition_label(clip.transition_out.as_deref()))
                    .width(ui.available_width().max(110.0))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(clip.transition_out.is_none(), "Yok (Düz)")
                            .clicked()
                        {
                            clip.transition_out = None;
                            *action = Some(InspectorAction::ClipModified);
                        }
                        for (key, label) in Clip::SUPPORTED_TRANSITIONS {
                            let is_sel = clip.transition_out.as_deref() == Some(*key);
                            if ui.selectable_label(is_sel, *label).clicked() {
                                clip.transition_out = Some(key.to_string());
                                *action = Some(InspectorAction::ClipModified);
                            }
                        }
                    });
            });
        },
    );
}

pub(super) fn render_audio_card(
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

pub(super) fn render_titler_card(
    ui: &mut Ui,
    track_id: &str,
    clip: &mut Clip,
    action: &mut Option<InspectorAction>,
) {
    let clip_id = clip.id.clone();
    let track_id_owned = track_id.to_string();
    inspector_card(
        ui,
        "T",
        "Metin & Altyazı (Titler)",
        ACCENT_AMBER,
        |_| {},
        |ui| {
            let mut has_overlay = clip.text_overlay.is_some();
            if ui
                .checkbox(
                    &mut has_overlay,
                    RichText::new("Metin Katmanı Etkin").size(11.0).strong(),
                )
                .changed()
            {
                clip.text_overlay = if has_overlay {
                    Some(TextOverlay::default())
                } else {
                    None
                };
                *action = Some(InspectorAction::ClipModified);
            }

            if let Some(txt) = &mut clip.text_overlay {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Yazı İçeriği:").size(11.0).color(LABEL_COLOR));
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        let tr_btn = Button::new(
                            RichText::new("TR → EN Harf Dönüştür")
                                .size(9.5)
                                .strong()
                                .color(Color32::from_rgb(0xfb, 0xbf, 0x24)),
                        )
                        .fill(Color32::from_rgb(0x1f, 0x23, 0x30))
                        .rounding(Rounding::same(4.0));
                        if ui
                            .add(tr_btn)
                            .on_hover_text(
                                "Özel yabancı fontlarda bozulan Türkçe harfleri (Ğ→G, Ş→S, İ→I, Ç→C, Ö→O, Ü→U) otomatik ASCII karşılıklarına dönüştürür.",
                            )
                            .clicked()
                        {
                            txt.content = TextOverlay::convert_turkish_to_ascii(&txt.content);
                            if let Some(sub) = &txt.subtitle_text {
                                txt.subtitle_text =
                                    Some(TextOverlay::convert_turkish_to_ascii(sub));
                            }
                            *action = Some(InspectorAction::ClipModified);
                        }
                    });
                });
                if ui
                    .add(
                        TextEdit::multiline(&mut txt.content)
                            .desired_rows(2)
                            .desired_width(ui.available_width()),
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }

                prop_row(ui, "Yazı Tipi (Font)", |ui| {
                    let mut changed = false;
                    let display_font = if txt.is_custom_font_file() {
                        std::path::Path::new(&txt.font_family)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(&txt.font_family)
                            .to_string()
                    } else {
                        txt.font_family.clone()
                    };
                    egui::ComboBox::from_id_salt("inspector_font_family")
                        .selected_text(display_font)
                        .show_ui(ui, |ui| {
                            for &font in trim_core::clip::POPULAR_FONT_FAMILIES {
                                if ui
                                    .selectable_value(&mut txt.font_family, font.to_string(), font)
                                    .changed()
                                {
                                    changed = true;
                                }
                            }
                        });

                    ui.add_space(4.0);
                    let bold_btn =
                        Button::new(RichText::new("B").size(11.0).strong()).fill(if txt.bold {
                            Color32::from_rgb(0x28, 0x5a, 0x9e)
                        } else {
                            Color32::from_rgb(0x1f, 0x23, 0x30)
                        });
                    if ui.add_sized([22.0, 18.0], bold_btn).clicked() {
                        txt.bold = !txt.bold;
                        changed = true;
                    }

                    let italic_btn =
                        Button::new(RichText::new("I").size(11.0).italics()).fill(if txt.italic {
                            Color32::from_rgb(0x28, 0x5a, 0x9e)
                        } else {
                            Color32::from_rgb(0x1f, 0x23, 0x30)
                        });
                    if ui.add_sized([22.0, 18.0], italic_btn).clicked() {
                        txt.italic = !txt.italic;
                        changed = true;
                    }

                    if changed {
                        *action = Some(InspectorAction::ClipModified);
                    }
                });

                prop_row(ui, "Özel Font", |ui| {
                    let is_custom = txt.is_custom_font_file();
                    let btn_w = if is_custom {
                        (ui.available_width() - 28.0).max(60.0)
                    } else {
                        ui.available_width()
                    };
                    let font_btn = Button::new(
                        RichText::new("+ .ttf / .otf Dosyası Seç...")
                            .size(10.0)
                            .color(Color32::from_rgb(0xd8, 0xde, 0xf0)),
                    )
                    .fill(Color32::from_rgb(0x1f, 0x23, 0x30))
                    .rounding(Rounding::same(4.0));
                    if ui
                        .add_sized([btn_w, 20.0], font_btn)
                        .on_hover_text("Bilgisayarınızdaki özel bir .ttf veya .otf font dosyasını doğrudan yükler")
                        .clicked()
                    {
                        *action = Some(InspectorAction::SelectCustomFont(clip_id.clone()));
                    }
                    if is_custom
                        && ui
                            .small_button("✕")
                            .on_hover_text(
                                "Özel font dosyasını kaldır ve varsayılan Inter fontuna dön",
                            )
                            .clicked()
                    {
                        txt.font_family = "Inter".to_string();
                        *action = Some(InspectorAction::ClipModified);
                    }
                });

                prop_row(ui, "Yazı Boyutu", |ui| {
                    if ui
                        .add(
                            DragValue::new(&mut txt.font_size)
                                .speed(1.0)
                                .range(12.0..=140.0)
                                .suffix(" pt"),
                        )
                        .changed()
                    {
                        *action = Some(InspectorAction::ClipModified);
                    }
                });

                prop_row(ui, "Hızlı Konum", |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let btn_w = ((ui.available_width() - 4.0 * 2.0) / 3.0).max(40.0);
                    for (lbl, py, bg) in [
                        ("Üst", 0.15_f32, None),
                        ("Orta", 0.50_f32, None),
                        ("Alt Yazı", 0.85_f32, Some([0, 0, 0, 180])),
                    ] {
                        if ui
                            .add_sized(
                                [btn_w, 20.0],
                                Button::new(RichText::new(lbl).size(10.5))
                                    .fill(Color32::from_rgb(0x1f, 0x23, 0x30))
                                    .rounding(Rounding::same(4.0)),
                            )
                            .clicked()
                        {
                            txt.pos_x = 0.5;
                            txt.pos_y = py;
                            txt.bg_color = bg;
                            *action = Some(InspectorAction::ClipModified);
                        }
                    }
                });

                prop_row(ui, "Yatay (X)", |ui| {
                    if ui
                        .add(
                            Slider::new(&mut txt.pos_x, 0.0..=1.0)
                                .show_value(false)
                                .trailing_fill(true),
                        )
                        .changed()
                    {
                        *action = Some(InspectorAction::ClipModified);
                    }
                });

                prop_row(ui, "Dikey (Y)", |ui| {
                    if ui
                        .add(
                            Slider::new(&mut txt.pos_y, 0.0..=1.0)
                                .show_value(false)
                                .trailing_fill(true),
                        )
                        .changed()
                    {
                        *action = Some(InspectorAction::ClipModified);
                    }
                });

                prop_row(ui, "Yazı Rengi", |ui| {
                    let mut col = Color32::from_rgba_unmultiplied(
                        txt.text_color[0],
                        txt.text_color[1],
                        txt.text_color[2],
                        txt.text_color[3],
                    );
                    if ui.color_edit_button_srgba(&mut col).changed() {
                        txt.text_color = [col.r(), col.g(), col.b(), col.a()];
                        *action = Some(InspectorAction::ClipModified);
                    }
                    let colors = [
                        ("Beyaz", [255, 255, 255, 255]),
                        ("Sarı", [245, 166, 35, 255]),
                        ("Turkuaz", [60, 210, 230, 255]),
                        ("Kırmızı", [235, 75, 75, 255]),
                    ];
                    for (c_name, c_val) in colors {
                        let is_c = txt.text_color == c_val;
                        if ui
                            .selectable_label(is_c, RichText::new(c_name).size(10.0))
                            .clicked()
                        {
                            txt.text_color = c_val;
                            *action = Some(InspectorAction::ClipModified);
                        }
                    }
                });

                prop_row(ui, "Kontur (Stroke)", |ui| {
                    ui.spacing_mut().slider_width = (ui.available_width() - 84.0).max(48.0);
                    if ui
                        .add(
                            Slider::new(&mut txt.stroke_width, 0.0..=12.0)
                                .suffix(" px")
                                .trailing_fill(true)
                                .fixed_decimals(1),
                        )
                        .changed()
                    {
                        *action = Some(InspectorAction::ClipModified);
                    }
                    let mut sc = Color32::from_rgba_unmultiplied(
                        txt.stroke_color[0],
                        txt.stroke_color[1],
                        txt.stroke_color[2],
                        txt.stroke_color[3],
                    );
                    if ui.color_edit_button_srgba(&mut sc).changed() {
                        txt.stroke_color = [sc.r(), sc.g(), sc.b(), sc.a()];
                        *action = Some(InspectorAction::ClipModified);
                    }
                });

                prop_row(ui, "Harf Aralığı", |ui| {
                    if ui
                        .add(
                            Slider::new(&mut txt.letter_spacing, -4.0..=24.0)
                                .suffix(" px")
                                .trailing_fill(true)
                                .fixed_decimals(1),
                        )
                        .changed()
                    {
                        *action = Some(InspectorAction::ClipModified);
                    }
                });

                prop_row(ui, "Stil & Kutu", |ui| {
                    if ui.checkbox(&mut txt.has_shadow, "Gölge").changed() {
                        *action = Some(InspectorAction::ClipModified);
                    }
                    if txt.has_shadow
                        && ui
                            .add(
                                DragValue::new(&mut txt.shadow_distance)
                                    .speed(0.2)
                                    .range(0.0..=20.0)
                                    .suffix("px"),
                            )
                            .changed()
                    {
                        *action = Some(InspectorAction::ClipModified);
                    }
                    let mut has_box = txt.bg_color.is_some();
                    if ui.checkbox(&mut has_box, "Arka Plan").changed() {
                        txt.bg_color = if has_box { Some([0, 0, 0, 180]) } else { None };
                        *action = Some(InspectorAction::ClipModified);
                    }
                });

                prop_row(ui, "Animasyon", |ui| {
                    egui::ComboBox::from_id_salt("motion_style_combo")
                        .selected_text(txt.motion_style.label())
                        .width(ui.available_width().max(110.0))
                        .show_ui(ui, |ui| {
                            for style in MotionTitleStyle::ALL {
                                if ui
                                    .selectable_value(&mut txt.motion_style, *style, style.label())
                                    .clicked()
                                {
                                    *action = Some(InspectorAction::ClipModified);
                                }
                            }
                        });
                });

                if matches!(
                    txt.motion_style,
                    MotionTitleStyle::LowerThird
                        | MotionTitleStyle::RollingCredits
                        | MotionTitleStyle::ChapterCard
                        | MotionTitleStyle::Ticker
                ) {
                    prop_row(ui, "Alt Başlık", |ui| {
                        let mut sub = txt.subtitle_text.clone().unwrap_or_default();
                        if ui
                            .add(
                                TextEdit::singleline(&mut sub)
                                    .hint_text("Ünvan / Oyuncular / Rozet Metni")
                                    .desired_width(ui.available_width()),
                            )
                            .changed()
                        {
                            txt.subtitle_text = if sub.trim().is_empty() {
                                None
                            } else {
                                Some(sub)
                            };
                            *action = Some(InspectorAction::ClipModified);
                        }
                    });
                }

                prop_row(ui, "Altyazı Stili", |ui| {
                    egui::ComboBox::from_id_salt("caption_style_combo")
                        .selected_text(txt.caption_style.label())
                        .width(ui.available_width().max(110.0))
                        .show_ui(ui, |ui| {
                            for style in CaptionStyle::ALL {
                                if ui
                                    .selectable_value(&mut txt.caption_style, *style, style.label())
                                    .clicked()
                                {
                                    *action = Some(InspectorAction::ClipModified);
                                }
                            }
                        });
                });

                ui.add_space(4.0);
                let foley_label = if txt.motion_style == MotionTitleStyle::Typewriter {
                    "♪ Daktilo Tuş + 35mm Projektör Foley Sesi Üret ve Ekle"
                } else {
                    "♪ Sinematik Başlık Foley Sesi Üret ve Ekle"
                };
                let btn_foley = Button::new(
                    RichText::new(foley_label)
                        .size(10.5)
                        .strong()
                        .color(Color32::from_rgb(0xfb, 0xbf, 0x24)),
                )
                .fill(Color32::from_rgba_unmultiplied(0xe0, 0x7a, 0x38, 28))
                .stroke(Stroke::new(
                    1.0_f32,
                    Color32::from_rgba_unmultiplied(0xe0, 0x7a, 0x38, 120),
                ))
                .rounding(Rounding::same(5.0))
                .min_size(Vec2::new(ui.available_width(), 24.0));
                if ui
                    .add(btn_foley)
                    .on_hover_text(
                        "Metin uzunluğuna ve klip süresine göre prosedürel daktilo tıkırtısı + 35mm projektör foley sesini (.wav) sentezler ve alt ses kanalına yerleştirir.",
                    )
                    .clicked()
                {
                    *action = Some(InspectorAction::GenerateTitleFoley {
                        track_id: track_id_owned.clone(),
                        clip_id: clip_id.clone(),
                    });
                }
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let half_w = ((ui.available_width() - 6.0) * 0.5).max(80.0);
                if ui
                    .add_sized(
                        [half_w, 24.0],
                        Button::new(RichText::new("↙ SRT İçe Aktar").size(10.5))
                            .fill(Color32::from_rgb(0x1f, 0x23, 0x30))
                            .rounding(Rounding::same(5.0)),
                    )
                    .clicked()
                {
                    *action = Some(InspectorAction::ImportSubtitles);
                }
                if ui
                    .add_sized(
                        [half_w, 24.0],
                        Button::new(RichText::new("↗ SRT Dışa Aktar").size(10.5))
                            .fill(Color32::from_rgb(0x1f, 0x23, 0x30))
                            .rounding(Rounding::same(5.0)),
                    )
                    .clicked()
                {
                    *action = Some(InspectorAction::ExportSubtitles);
                }
            });

            let btn_whisper = Button::new(
                RichText::new("◈  Lokal Whisper ile Otomatik Deşifre Et")
                    .size(11.0)
                    .strong()
                    .color(Color32::from_rgb(0x60, 0xb0, 0xff)),
            )
            .fill(Color32::from_rgba_unmultiplied(0x38, 0x7a, 0xe0, 26))
            .stroke(Stroke::new(
                1.0_f32,
                Color32::from_rgba_unmultiplied(0x38, 0x7a, 0xe0, 120),
            ))
            .rounding(Rounding::same(5.0))
            .min_size(Vec2::new(ui.available_width(), 26.0));
            if ui.add(btn_whisper).clicked() {
                *action = Some(InspectorAction::TriggerTranscribe);
            }
        },
    );
}

pub(super) fn render_compositing_card(
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

pub(super) fn render_masks_card(
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
