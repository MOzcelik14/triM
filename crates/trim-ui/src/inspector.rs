//! Clip Properties and Inspector panel in Turkish.

use egui::{Button, DragValue, Grid, RichText, ScrollArea, Ui};
use trim_core::clip::{AudioEqPreset, Clip, TextOverlay};
use trim_core::keyframe::Interpolation;
use trim_core::project::Project;

pub enum InspectorAction {
    ClipModified,
    ChangeSpeed(f64, bool),
}

pub fn render_inspector_panel(
    ui: &mut Ui,
    project: &mut Project,
    selected_clip_id: Option<&str>,
    playhead_time: f64,
) -> Option<InspectorAction> {
    let mut action = None;

    ui.heading(RichText::new("Özellikler / Denetçi").size(14.0));
    ui.add_space(6.0);

    let target_clip_and_track = if let Some(clip_id) = selected_clip_id {
        let track_info = project
            .timeline
            .tracks
            .iter()
            .find(|t| t.get_clip_by_id(clip_id).is_some())
            .map(|t| (t.name.clone(), t.track_type == trim_core::track::TrackType::Audio));
        if let Some((t_name, is_audio)) = track_info {
            project
                .timeline
                .tracks
                .iter_mut()
                .find_map(|t| t.get_clip_by_id_mut(clip_id))
                .map(|c| (t_name, is_audio, c))
        } else {
            None
        }
    } else {
        None
    };

    if target_clip_and_track.is_none() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                RichText::new("Özelliklerini görüntülemek ve düzenlemek için\nzaman çizgisinden bir klip seçin.")
                    .color(egui::Color32::from_rgb(0x80, 0x80, 0x90))
                    .size(12.0),
            );
        });
        return None;
    }

    let (track_name, is_audio, clip) = target_clip_and_track.unwrap();
    let clip_time = (playhead_time - clip.timeline_in).max(0.0);

    ScrollArea::vertical().show(ui, |ui| {
        // 1. Klip Bilgisi
        render_info_group(ui, clip, &track_name, &mut action);
        ui.add_space(6.0);

        // 2. Zamanlama
        render_timing_group(ui, clip, &mut action);
        ui.add_space(6.0);

        // If clip has text overlay, prioritize Titler right after timing!
        if !is_audio && clip.text_overlay.is_some() {
            render_titler_group(ui, clip, &mut action);
            ui.add_space(6.0);
        }

        // If audio track, prioritize Audio & EQ and skip visual-only groups
        if is_audio {
            render_audio_group(ui, clip, &mut action);
            ui.add_space(6.0);

            render_speed_group(ui, clip, &mut action);
            ui.add_space(6.0);

            render_transitions_group(ui, clip, &mut action);
        } else {
            // 3. Video Dönüşümü & Anahtar Kareler
            render_transform_group(ui, clip, clip_time, &mut action);
            ui.add_space(6.0);

            // Ses & EQ Profili
            render_audio_group(ui, clip, &mut action);
            ui.add_space(6.0);

            // 4. Hız ve Yön
            render_speed_group(ui, clip, &mut action);
            ui.add_space(6.0);

            // 5. Renk ve Filtreler
            render_color_group(ui, clip, &mut action);
            ui.add_space(6.0);

            // 6. Giriş / Çıkış ve Geçişler
            render_transitions_group(ui, clip, &mut action);

            // If no text overlay yet, offer enabling it at the bottom
            if clip.text_overlay.is_none() {
                ui.add_space(6.0);
                render_titler_group(ui, clip, &mut action);
            }
        }
    });

    action
}

fn render_info_group(ui: &mut Ui, clip: &mut Clip, track_name: &str, action: &mut Option<InspectorAction>) {
    ui.group(|ui| {
        ui.strong("Klip Bilgisi");
        Grid::new("info_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("İsim:");
            if ui.text_edit_singleline(&mut clip.name).changed() {
                *action = Some(InspectorAction::ClipModified);
            }
            ui.end_row();

            ui.label("Kanal:");
            ui.label(track_name);
            ui.end_row();

            ui.label("Süre:");
            ui.label(format!("{:.2} s", clip.duration()));
            ui.end_row();
        });
    });
}

fn render_timing_group(ui: &mut Ui, clip: &mut Clip, action: &mut Option<InspectorAction>) {
    ui.group(|ui| {
        ui.strong("Zamanlama");
        Grid::new("timing_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("Başlangıç:");
            let old_dur = clip.duration();
            if ui.add(DragValue::new(&mut clip.timeline_in).speed(0.1).range(0.0..=99999.0).suffix(" s")).changed() {
                clip.timeline_in = clip.timeline_in.max(0.0);
                clip.timeline_out = clip.timeline_in + old_dur;
                *action = Some(InspectorAction::ClipModified);
            }
            ui.end_row();

            ui.label("Bitiş:");
            if ui.add(DragValue::new(&mut clip.timeline_out).speed(0.1).range(0.0..=99999.0).suffix(" s")).changed() {
                clip.timeline_out = clip.timeline_out.max(clip.timeline_in + 0.04);
                clip.source_out = Some(clip.source_in + (clip.timeline_out - clip.timeline_in) * clip.speed);
                *action = Some(InspectorAction::ClipModified);
            }
            ui.end_row();
        });
    });
}

fn render_transform_group(ui: &mut Ui, clip: &mut Clip, clip_time: f64, action: &mut Option<InspectorAction>) {
    ui.group(|ui| {
        ui.strong("Video Dönüşümü & Anahtar Kareler");
        Grid::new("trans_grid").num_columns(3).spacing([6.0, 6.0]).show(ui, |ui| {
            // Pos X
            ui.label("Konum X:");
            if ui.add(DragValue::new(&mut clip.pos_x).speed(1.0).suffix(" px")).changed() {
                sync_kf_value(clip, "pos_x", clip_time, clip.pos_x);
                *action = Some(InspectorAction::ClipModified);
            }
            render_kf_btn(ui, clip, "pos_x", clip_time, clip.pos_x, action);
            ui.end_row();

            // Pos Y
            ui.label("Konum Y:");
            if ui.add(DragValue::new(&mut clip.pos_y).speed(1.0).suffix(" px")).changed() {
                sync_kf_value(clip, "pos_y", clip_time, clip.pos_y);
                *action = Some(InspectorAction::ClipModified);
            }
            render_kf_btn(ui, clip, "pos_y", clip_time, clip.pos_y, action);
            ui.end_row();

            // Scale
            ui.label("Ölçek:");
            if ui.add(DragValue::new(&mut clip.scale).speed(0.01).range(0.05..=10.0)).changed() {
                sync_kf_value(clip, "scale", clip_time, clip.scale);
                *action = Some(InspectorAction::ClipModified);
            }
            render_kf_btn(ui, clip, "scale", clip_time, clip.scale, action);
            ui.end_row();

            // Opacity
            ui.label("Opaklık:");
            if ui.add(DragValue::new(&mut clip.opacity).speed(0.01).range(0.0..=1.0)).changed() {
                sync_kf_value(clip, "opacity", clip_time, clip.opacity);
                *action = Some(InspectorAction::ClipModified);
            }
            render_kf_btn(ui, clip, "opacity", clip_time, clip.opacity, action);
            ui.end_row();
        });
    });
}

fn render_speed_group(ui: &mut Ui, clip: &mut Clip, action: &mut Option<InspectorAction>) {
    ui.group(|ui| {
        ui.strong("Hız ve Yön");
        Grid::new("speed_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("Hız:");
            let mut s = clip.speed;
            if ui.add(DragValue::new(&mut s).speed(0.05).range(0.1..=10.0).suffix("x")).changed() {
                *action = Some(InspectorAction::ChangeSpeed(s, clip.reverse));
            }
            ui.end_row();

            ui.label("Hazır:");
            ui.horizontal(|ui| {
                for preset in [0.5, 1.0, 2.0] {
                    if ui.button(format!("{:.1}x", preset)).clicked() {
                        *action = Some(InspectorAction::ChangeSpeed(preset, clip.reverse));
                    }
                }
            });
            ui.end_row();

            ui.label("");
            let mut rev = clip.reverse;
            if ui.checkbox(&mut rev, "Geriye Doğru Oynat").changed() {
                *action = Some(InspectorAction::ChangeSpeed(clip.speed, rev));
            }
            ui.end_row();
        });
    });
}

fn render_color_group(ui: &mut Ui, clip: &mut Clip, action: &mut Option<InspectorAction>) {
    ui.group(|ui| {
        ui.strong("Renk ve Filtreler");
        Grid::new("color_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("Parlaklık:");
            if ui.add(DragValue::new(&mut clip.brightness).speed(0.05).range(-1.0..=1.0)).changed() {
                *action = Some(InspectorAction::ClipModified);
            }
            ui.end_row();

            ui.label("Kontrast:");
            if ui.add(DragValue::new(&mut clip.contrast).speed(0.05).range(0.0..=3.0)).changed() {
                *action = Some(InspectorAction::ClipModified);
            }
            ui.end_row();

            ui.label("Doygunluk:");
            if ui.add(DragValue::new(&mut clip.saturation).speed(0.05).range(0.0..=3.0)).changed() {
                *action = Some(InspectorAction::ClipModified);
            }
            ui.end_row();

            ui.label("");
            if ui.button("Renkleri Sıfırla").clicked() {
                clip.brightness = 0.0;
                clip.contrast = 1.0;
                clip.saturation = 1.0;
                *action = Some(InspectorAction::ClipModified);
            }
            ui.end_row();
        });
    });
}

fn render_transitions_group(ui: &mut Ui, clip: &mut Clip, action: &mut Option<InspectorAction>) {
    ui.group(|ui| {
        ui.strong("Giriş / Çıkış ve Geçişler");
        Grid::new("fade_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("Giriş Kararması:");
            if ui.add(DragValue::new(&mut clip.fade_in).speed(0.1).range(0.0..=60.0).suffix(" s")).changed() {
                *action = Some(InspectorAction::ClipModified);
            }
            ui.end_row();

            ui.label("Giriş Geçişi:");
            egui::ComboBox::from_id_salt("trans_in")
                .selected_text(Clip::transition_label(clip.transition_in.as_deref()))
                .show_ui(ui, |ui| {
                    if ui.selectable_label(clip.transition_in.is_none(), "Yok (Düz)").clicked() {
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
            ui.end_row();

            ui.label("Çıkış Kararması:");
            if ui.add(DragValue::new(&mut clip.fade_out).speed(0.1).range(0.0..=60.0).suffix(" s")).changed() {
                *action = Some(InspectorAction::ClipModified);
            }
            ui.end_row();

            ui.label("Çıkış Geçişi:");
            egui::ComboBox::from_id_salt("trans_out")
                .selected_text(Clip::transition_label(clip.transition_out.as_deref()))
                .show_ui(ui, |ui| {
                    if ui.selectable_label(clip.transition_out.is_none(), "Yok (Düz)").clicked() {
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
            ui.end_row();
        });
    });
}

fn render_audio_group(ui: &mut Ui, clip: &mut Clip, action: &mut Option<InspectorAction>) {
    ui.group(|ui| {
        ui.strong("Ses & EQ Profili");
        Grid::new("audio_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("Ses Düzeyi:");
            if ui.add(DragValue::new(&mut clip.volume).speed(0.05).range(0.0..=3.0)).changed() {
                *action = Some(InspectorAction::ClipModified);
            }
            ui.end_row();

            ui.label("EQ Profili:");
            egui::ComboBox::from_id_salt("clip_eq_preset")
                .selected_text(clip.eq_preset.label())
                .show_ui(ui, |ui| {
                    for p in AudioEqPreset::ALL {
                        if ui.selectable_label(clip.eq_preset == *p, p.label()).clicked() {
                            clip.eq_preset = *p;
                            *action = Some(InspectorAction::ClipModified);
                        }
                    }
                });
            ui.end_row();

            ui.label("Gürültü:");
            if ui.checkbox(&mut clip.noise_gate, "Gürültü Kapısı (Noise Gate)").changed() {
                *action = Some(InspectorAction::ClipModified);
            }
            ui.end_row();

            ui.label("");
            if ui.checkbox(&mut clip.muted, "Klibi Sessize Al").changed() {
                *action = Some(InspectorAction::ClipModified);
            }
            ui.end_row();
        });
    });
}

fn render_titler_group(ui: &mut Ui, clip: &mut Clip, action: &mut Option<InspectorAction>) {
    ui.group(|ui| {
        ui.strong("Metin & Altyazı (Titler)");
        let mut has_overlay = clip.text_overlay.is_some();
        if ui.checkbox(&mut has_overlay, "Metin Katmanı Etkin").changed() {
            if has_overlay {
                clip.text_overlay = Some(TextOverlay::default());
            } else {
                clip.text_overlay = None;
            }
            *action = Some(InspectorAction::ClipModified);
        }

        if let Some(txt) = &mut clip.text_overlay {
            ui.add_space(4.0);
            ui.label(RichText::new("Yazı İçeriği:").size(11.0).color(egui::Color32::from_rgb(0xa0, 0xa0, 0xb0)));
            if ui.text_edit_multiline(&mut txt.content).changed() {
                *action = Some(InspectorAction::ClipModified);
            }

            ui.add_space(4.0);
            Grid::new("titler_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Yazı Boyutu:");
                if ui.add(DragValue::new(&mut txt.font_size).speed(1.0).range(12.0..=140.0).suffix(" pt")).changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
                ui.end_row();

                ui.label("Konum Şablonu:");
                ui.horizontal(|ui| {
                    if ui.button("Alt Yazı").clicked() {
                        txt.pos_x = 0.5;
                        txt.pos_y = 0.85;
                        txt.bg_color = Some([0, 0, 0, 180]);
                        *action = Some(InspectorAction::ClipModified);
                    }
                    if ui.button("Orta").clicked() {
                        txt.pos_x = 0.5;
                        txt.pos_y = 0.50;
                        txt.bg_color = None;
                        *action = Some(InspectorAction::ClipModified);
                    }
                    if ui.button("Üst").clicked() {
                        txt.pos_x = 0.5;
                        txt.pos_y = 0.15;
                        txt.bg_color = None;
                        *action = Some(InspectorAction::ClipModified);
                    }
                });
                ui.end_row();

                ui.label("Yatay Konum (X):");
                if ui.add(egui::Slider::new(&mut txt.pos_x, 0.0..=1.0).show_value(false)).changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
                ui.end_row();

                ui.label("Dikey Konum (Y):");
                if ui.add(egui::Slider::new(&mut txt.pos_y, 0.0..=1.0).show_value(false)).changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
                ui.end_row();

                ui.label("Yazı Rengi:");
                ui.horizontal(|ui| {
                    let colors = [
                        ("Beyaz", [255, 255, 255, 255]),
                        ("Sarı", [245, 166, 35, 255]),
                        ("Turkuaz", [60, 210, 230, 255]),
                        ("Kırmızı", [235, 75, 75, 255]),
                    ];
                    for (c_name, c_val) in colors {
                        let is_c = txt.text_color == c_val;
                        if ui.selectable_label(is_c, c_name).clicked() {
                            txt.text_color = c_val;
                            *action = Some(InspectorAction::ClipModified);
                        }
                    }
                });
                ui.end_row();

                ui.label("Gölge / Kontur:");
                if ui.checkbox(&mut txt.has_shadow, "Açık").changed() {
                    *action = Some(InspectorAction::ClipModified);
                }
                ui.end_row();

                ui.label("Altyazı Kutusu:");
                let mut has_box = txt.bg_color.is_some();
                if ui.checkbox(&mut has_box, "Yarı Saydam Kutu").changed() {
                    if has_box {
                        txt.bg_color = Some([0, 0, 0, 180]);
                    } else {
                        txt.bg_color = None;
                    }
                    *action = Some(InspectorAction::ClipModified);
                }
                ui.end_row();
            });
        }
    });
}

fn render_kf_btn(
    ui: &mut Ui,
    clip: &mut Clip,
    prop: &str,
    clip_time: f64,
    val: f64,
    action: &mut Option<InspectorAction>,
) {
    let has_kf = clip
        .keyframes
        .get(prop)
        .is_some_and(|list| list.iter().any(|k| (k.time - clip_time).abs() <= 0.05));

    let (btn_txt, btn_col) = if has_kf {
        ("KF*", egui::Color32::from_rgb(0xe0, 0x7a, 0x38))
    } else {
        ("KF", egui::Color32::from_rgb(0x70, 0x70, 0x80))
    };

    if ui.add(Button::new(RichText::new(btn_txt).size(10.0).strong().color(btn_col)).min_size(egui::vec2(26.0, 18.0))).clicked() {
        if has_kf {
            clip.remove_keyframe(prop, clip_time, 0.05);
        } else {
            clip.add_keyframe(prop, clip_time, val, Interpolation::Linear);
        }
        *action = Some(InspectorAction::ClipModified);
    }
}

fn sync_kf_value(clip: &mut Clip, prop: &str, clip_time: f64, val: f64) {
    if let Some(kfs) = clip.keyframes.get_mut(prop) {
        if let Some(kf) = kfs.iter_mut().find(|k| (k.time - clip_time).abs() <= 0.05) {
            kf.value = val;
        }
    }
}
