//! Clip Properties and Inspector panel in Turkish.

use egui::{Button, DragValue, Grid, RichText, ScrollArea, Ui};
use trim_core::clip::Clip;
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
        let track_name = project
            .timeline
            .tracks
            .iter()
            .find(|t| t.get_clip_by_id(clip_id).is_some())
            .map(|t| t.name.clone());
        if let Some(t_name) = track_name {
            project
                .timeline
                .tracks
                .iter_mut()
                .find_map(|t| t.get_clip_by_id_mut(clip_id))
                .map(|c| (t_name, c))
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

    let (track_name, clip) = target_clip_and_track.unwrap();
    let clip_time = (playhead_time - clip.timeline_in).max(0.0);

    ScrollArea::vertical().show(ui, |ui| {
        // 1. Klip Bilgisi
        ui.group(|ui| {
            ui.strong("Klip Bilgisi");
            Grid::new("info_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("İsim:");
                if ui.text_edit_singleline(&mut clip.name).changed() {
                    action = Some(InspectorAction::ClipModified);
                }
                ui.end_row();

                ui.label("Kanal:");
                ui.label(&track_name);
                ui.end_row();

                ui.label("Süre:");
                ui.label(format!("{:.2} s", clip.duration()));
                ui.end_row();
            });
        });

        ui.add_space(6.0);

        // 2. Zamanlama
        ui.group(|ui| {
            ui.strong("Zamanlama");
            Grid::new("timing_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Başlangıç:");
                let old_dur = clip.duration();
                if ui.add(DragValue::new(&mut clip.timeline_in).speed(0.1).range(0.0..=99999.0).suffix(" s")).changed() {
                    clip.timeline_in = clip.timeline_in.max(0.0);
                    clip.timeline_out = clip.timeline_in + old_dur;
                    action = Some(InspectorAction::ClipModified);
                }
                ui.end_row();

                ui.label("Bitiş:");
                if ui.add(DragValue::new(&mut clip.timeline_out).speed(0.1).range(0.0..=99999.0).suffix(" s")).changed() {
                    clip.timeline_out = clip.timeline_out.max(clip.timeline_in + 0.04);
                    clip.source_out = Some(clip.source_in + (clip.timeline_out - clip.timeline_in) * clip.speed);
                    action = Some(InspectorAction::ClipModified);
                }
                ui.end_row();
            });
        });

        ui.add_space(6.0);

        // 3. Video Dönüşümü & Anahtar Kareler
        ui.group(|ui| {
            ui.strong("Video Dönüşümü & Anahtar Kareler");
            Grid::new("trans_grid").num_columns(3).spacing([6.0, 6.0]).show(ui, |ui| {
                // Pos X
                ui.label("Konum X:");
                if ui.add(DragValue::new(&mut clip.pos_x).speed(1.0).suffix(" px")).changed() {
                    sync_kf_value(clip, "pos_x", clip_time, clip.pos_x);
                    action = Some(InspectorAction::ClipModified);
                }
                render_kf_btn(ui, clip, "pos_x", clip_time, clip.pos_x, &mut action);
                ui.end_row();

                // Pos Y
                ui.label("Konum Y:");
                if ui.add(DragValue::new(&mut clip.pos_y).speed(1.0).suffix(" px")).changed() {
                    sync_kf_value(clip, "pos_y", clip_time, clip.pos_y);
                    action = Some(InspectorAction::ClipModified);
                }
                render_kf_btn(ui, clip, "pos_y", clip_time, clip.pos_y, &mut action);
                ui.end_row();

                // Scale
                ui.label("Ölçek:");
                if ui.add(DragValue::new(&mut clip.scale).speed(0.05).range(0.05..=10.0).suffix("x")).changed() {
                    sync_kf_value(clip, "scale", clip_time, clip.scale);
                    action = Some(InspectorAction::ClipModified);
                }
                render_kf_btn(ui, clip, "scale", clip_time, clip.scale, &mut action);
                ui.end_row();

                // Opacity
                ui.label("Opaklık:");
                if ui.add(DragValue::new(&mut clip.opacity).speed(0.05).range(0.0..=1.0)).changed() {
                    sync_kf_value(clip, "opacity", clip_time, clip.opacity);
                    action = Some(InspectorAction::ClipModified);
                }
                render_kf_btn(ui, clip, "opacity", clip_time, clip.opacity, &mut action);
                ui.end_row();
            });
        });

        ui.add_space(6.0);

        // 4. Hız ve Yön
        ui.group(|ui| {
            ui.strong("Hız ve Yön");
            Grid::new("speed_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Hız:");
                let mut sp = clip.speed;
                if ui.add(DragValue::new(&mut sp).speed(0.1).range(0.1..=10.0).suffix("x")).changed() {
                    action = Some(InspectorAction::ChangeSpeed(sp, clip.reverse));
                }
                ui.end_row();

                ui.label("Hazır:");
                ui.horizontal(|ui| {
                    if ui.button("0.5x").clicked() {
                        action = Some(InspectorAction::ChangeSpeed(0.5, clip.reverse));
                    }
                    if ui.button("1.0x").clicked() {
                        action = Some(InspectorAction::ChangeSpeed(1.0, clip.reverse));
                    }
                    if ui.button("2.0x").clicked() {
                        action = Some(InspectorAction::ChangeSpeed(2.0, clip.reverse));
                    }
                });
                ui.end_row();

                ui.label("");
                let mut rev = clip.reverse;
                if ui.checkbox(&mut rev, "Geriye Doğru Oynat").changed() {
                    action = Some(InspectorAction::ChangeSpeed(clip.speed, rev));
                }
                ui.end_row();
            });
        });

        ui.add_space(6.0);

        // 5. Renk ve Filtreler
        ui.group(|ui| {
            ui.strong("Renk ve Filtreler");
            Grid::new("color_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Parlaklık:");
                if ui.add(DragValue::new(&mut clip.brightness).speed(0.05).range(-1.0..=1.0)).changed() {
                    action = Some(InspectorAction::ClipModified);
                }
                ui.end_row();

                ui.label("Kontrast:");
                if ui.add(DragValue::new(&mut clip.contrast).speed(0.05).range(0.0..=3.0)).changed() {
                    action = Some(InspectorAction::ClipModified);
                }
                ui.end_row();

                ui.label("Doygunluk:");
                if ui.add(DragValue::new(&mut clip.saturation).speed(0.05).range(0.0..=3.0)).changed() {
                    action = Some(InspectorAction::ClipModified);
                }
                ui.end_row();

                ui.label("");
                if ui.button("Renkleri Sıfırla").clicked() {
                    clip.brightness = 0.0;
                    clip.contrast = 1.0;
                    clip.saturation = 1.0;
                    action = Some(InspectorAction::ClipModified);
                }
                ui.end_row();
            });
        });

        ui.add_space(6.0);

        // 6. Giriş / Çıkış ve Geçişler
        ui.group(|ui| {
            ui.strong("Giriş / Çıkış ve Geçişler");
            Grid::new("fade_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Giriş Kararması:");
                if ui.add(DragValue::new(&mut clip.fade_in).speed(0.1).range(0.0..=60.0).suffix(" s")).changed() {
                    action = Some(InspectorAction::ClipModified);
                }
                ui.end_row();

                ui.label("Giriş Geçişi:");
                egui::ComboBox::from_id_salt("trans_in")
                    .selected_text(match clip.transition_in.as_deref() {
                        Some("dip_black") => "Siyaha Geçiş",
                        Some("dip_white") => "Beyaza Geçiş",
                        _ => "Yok (Düz)",
                    })
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(clip.transition_in.is_none(), "Yok (Düz)").clicked() {
                            clip.transition_in = None;
                            action = Some(InspectorAction::ClipModified);
                        }
                        if ui.selectable_label(clip.transition_in.as_deref() == Some("dip_black"), "Siyaha Geçiş").clicked() {
                            clip.transition_in = Some("dip_black".into());
                            action = Some(InspectorAction::ClipModified);
                        }
                        if ui.selectable_label(clip.transition_in.as_deref() == Some("dip_white"), "Beyaza Geçiş").clicked() {
                            clip.transition_in = Some("dip_white".into());
                            action = Some(InspectorAction::ClipModified);
                        }
                    });
                ui.end_row();

                ui.label("Çıkış Kararması:");
                if ui.add(DragValue::new(&mut clip.fade_out).speed(0.1).range(0.0..=60.0).suffix(" s")).changed() {
                    action = Some(InspectorAction::ClipModified);
                }
                ui.end_row();

                ui.label("Çıkış Geçişi:");
                egui::ComboBox::from_id_salt("trans_out")
                    .selected_text(match clip.transition_out.as_deref() {
                        Some("dip_black") => "Siyaha Geçiş",
                        Some("dip_white") => "Beyaza Geçiş",
                        _ => "Yok (Düz)",
                    })
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(clip.transition_out.is_none(), "Yok (Düz)").clicked() {
                            clip.transition_out = None;
                            action = Some(InspectorAction::ClipModified);
                        }
                        if ui.selectable_label(clip.transition_out.as_deref() == Some("dip_black"), "Siyaha Geçiş").clicked() {
                            clip.transition_out = Some("dip_black".into());
                            action = Some(InspectorAction::ClipModified);
                        }
                        if ui.selectable_label(clip.transition_out.as_deref() == Some("dip_white"), "Beyaza Geçiş").clicked() {
                            clip.transition_out = Some("dip_white".into());
                            action = Some(InspectorAction::ClipModified);
                        }
                    });
                ui.end_row();
            });
        });

        ui.add_space(6.0);

        // 7. Ses
        ui.group(|ui| {
            ui.strong("Ses");
            Grid::new("audio_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Ses Düzeyi:");
                if ui.add(DragValue::new(&mut clip.volume).speed(0.05).range(0.0..=3.0)).changed() {
                    action = Some(InspectorAction::ClipModified);
                }
                ui.end_row();

                ui.label("");
                if ui.checkbox(&mut clip.muted, "Klibi Sessize Al").changed() {
                    action = Some(InspectorAction::ClipModified);
                }
                ui.end_row();
            });
        });
    });

    action
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
