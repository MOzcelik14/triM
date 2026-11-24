use super::super::{inspector_card, prop_row, InspectorAction, ACCENT_AMBER, LABEL_COLOR};
use egui::{
    Button, Color32, DragValue, Layout, RichText, Rounding, Slider, Stroke, TextEdit, Ui, Vec2,
};
use trim_core::clip::{CaptionStyle, Clip, MotionTitleStyle, TextOverlay};

pub(in crate::inspector) fn render_titler_card(
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
