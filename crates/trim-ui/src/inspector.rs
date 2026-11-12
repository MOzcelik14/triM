//! Clip Properties and Sequence Inspector panel in Turkish.

use crate::color_wheel::render_color_wheels_panel;
use egui::{
    Button, Color32, DragValue, Frame, Layout, Margin, RichText, Rounding, ScrollArea, Slider,
    Stroke, TextEdit, Ui, Vec2,
};
use trim_commands::commands::ClipColorGradingState;
use trim_core::clip::{
    AudioEqPreset, AudioVisualizerConfig, BrightnessContrastPreset, CaptionStyle, ChromaKeyConfig,
    Clip, ColorLabel, ColorWheelsConfig, LookCategory, LutPreset, MotionTitleStyle, PanZoomPreset,
    TextOverlay, VisualizerStyle, WhiteBalancePreset,
};
use trim_core::keyframe::Interpolation;
use trim_core::media::MediaType;
use trim_core::project::Project;
use trim_core::track::TrackType;

pub enum InspectorAction {
    ClipModified,
    ChangeSpeed(f64, bool),
    TriggerSceneDetect,
    TriggerSilenceDetect,
    TriggerTranscribe,
    ImportSubtitles,
    ExportSubtitles,
    SelectCustomLut(String),
    AutoWhiteBalance {
        track_id: String,
        clip_id: String,
    },
    SetColorGrading {
        track_id: String,
        clip_id: String,
        old_state: ClipColorGradingState,
        new_state: ClipColorGradingState,
    },
    SetTransform {
        track_id: String,
        clip_id: String,
        old_transform: (f64, f64, f64, f64, f64),
        new_transform: (f64, f64, f64, f64, f64),
    },
    SetColorWheels {
        track_id: String,
        clip_id: String,
        old_wheels: Option<ColorWheelsConfig>,
        new_wheels: Option<ColorWheelsConfig>,
    },
    SetEqPreset {
        track_id: String,
        clip_id: String,
        old_preset: AudioEqPreset,
        new_preset: AudioEqPreset,
    },
    SetCrop {
        track_id: String,
        clip_id: String,
        old_crop: (f32, f32, f32, f32),
        new_crop: (f32, f32, f32, f32),
    },
    SetBlendMode {
        track_id: String,
        clip_id: String,
        old_blend_mode: trim_core::BlendMode,
        new_blend_mode: trim_core::BlendMode,
    },
    SetMasks {
        track_id: String,
        clip_id: String,
        old_masks: Vec<trim_core::Mask>,
        new_masks: Vec<trim_core::Mask>,
    },
}

const CARD_BG: Color32 = Color32::from_rgb(0x13, 0x15, 0x1c);
const CARD_HEADER_BG: Color32 = Color32::from_rgb(0x18, 0x1b, 0x24);
const CARD_BORDER: Color32 = Color32::from_rgb(0x25, 0x29, 0x36);
const SUBCARD_BG: Color32 = Color32::from_rgb(0x0f, 0x11, 0x17);
const LABEL_COLOR: Color32 = Color32::from_rgb(0x9c, 0xa3, 0xb8);
const ACCENT_AMBER: Color32 = Color32::from_rgb(0xe0, 0x7a, 0x38);
const ACCENT_BLUE: Color32 = Color32::from_rgb(0x38, 0x7a, 0xe0);
const ACCENT_TEAL: Color32 = Color32::from_rgb(0x2d, 0xb8, 0x9a);
const LABEL_COL_W: f32 = 88.0;

pub fn render_inspector_panel(
    ui: &mut Ui,
    project: &mut Project,
    selected_clip_id: Option<&str>,
    playhead_time: f64,
) -> Option<InspectorAction> {
    let mut action = None;

    let track_meta = selected_clip_id.and_then(|clip_id| {
        project
            .timeline
            .tracks
            .iter()
            .find(|t| t.get_clip_by_id(clip_id).is_some())
            .map(|t| {
                (
                    t.id.clone(),
                    t.name.clone(),
                    t.track_type == TrackType::Audio,
                )
            })
    });

    // Top Inspector Header Bar
    Frame::none()
        .fill(Color32::from_rgb(0x15, 0x17, 0x20))
        .stroke(Stroke::new(1.0_f32, CARD_BORDER))
        .rounding(Rounding::same(7.0))
        .inner_margin(Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Özellikler / Denetçi")
                        .size(13.5)
                        .strong()
                        .color(Color32::WHITE),
                );
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some((_, ref t_name, is_audio)) = track_meta {
                        let (badge_txt, badge_col) = if is_audio {
                            (format!("SES • {}", t_name), ACCENT_TEAL)
                        } else {
                            (format!("VİDEO • {}", t_name), ACCENT_BLUE)
                        };
                        render_badge(ui, &badge_txt, badge_col);
                    } else {
                        render_badge(ui, "SEKANS", ACCENT_AMBER);
                    }
                });
            });
        });

    ui.add_space(6.0);

    let target_clip_and_track = if let Some(clip_id) = selected_clip_id {
        if let Some((t_id, t_name, is_audio)) = track_meta {
            project
                .timeline
                .tracks
                .iter_mut()
                .find_map(|t| t.get_clip_by_id_mut(clip_id))
                .map(|c| (t_id, t_name, is_audio, c))
        } else {
            None
        }
    } else {
        None
    };

    if target_clip_and_track.is_none() {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                render_sequence_inspector(ui, project, &mut action);
            });
        return action;
    }

    let (track_id, track_name, is_audio, clip) = target_clip_and_track.unwrap();
    let clip_time = (playhead_time - clip.timeline_in).max(0.0);

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // 1. Klip Bilgisi & Zamanlama
            render_info_and_timing_card(ui, clip, &track_name, is_audio, &mut action);
            ui.add_space(6.0);

            // If clip has text overlay enabled, surface Titler near the top
            if !is_audio && clip.text_overlay.is_some() {
                render_titler_card(ui, clip, &mut action);
                ui.add_space(6.0);
            }

            if is_audio {
                render_audio_card(ui, &track_id, clip, &mut action);
                ui.add_space(6.0);

                render_speed_card(ui, clip, &mut action);
                ui.add_space(6.0);

                render_transitions_card(ui, clip, &mut action);
            } else if clip.is_adjustment_layer() {
                render_compositing_card(ui, &track_id, clip, clip_time, &mut action);
                ui.add_space(6.0);

                render_masks_card(ui, &track_id, clip, clip_time, &mut action);
                ui.add_space(6.0);

                render_color_card(ui, &track_id, clip, &mut action);
                ui.add_space(6.0);
            } else {
                // 2. Karışım Modu & Compositing
                render_compositing_card(ui, &track_id, clip, clip_time, &mut action);
                ui.add_space(6.0);

                // 2a. Video Dönüşümü & Anahtar Kareler
                render_transform_card(ui, &track_id, clip, clip_time, &mut action);
                ui.add_space(6.0);

                // 2b. Video Kırpma (Crop)
                render_crop_card(ui, &track_id, clip, &mut action);
                ui.add_space(6.0);

                // 2c. Maskeler (Masks)
                render_masks_card(ui, &track_id, clip, clip_time, &mut action);
                ui.add_space(6.0);

                // 3. Renk, LUT & Chroma Key
                render_color_card(ui, &track_id, clip, &mut action);
                ui.add_space(6.0);

                // 4. Ses & EQ Profili
                render_audio_card(ui, &track_id, clip, &mut action);
                ui.add_space(6.0);

                // 5. Hız, Yön & Akıllı Kurgu
                render_speed_card(ui, clip, &mut action);
                ui.add_space(6.0);

                // 6. Giriş / Çıkış ve Geçişler
                render_transitions_card(ui, clip, &mut action);
            }
            ui.add_space(8.0);
        });

    action
}

fn render_badge(ui: &mut Ui, text: &str, color: Color32) {
    Frame::none()
        .fill(Color32::from_rgba_unmultiplied(
            color.r(),
            color.g(),
            color.b(),
            36,
        ))
        .stroke(Stroke::new(1.0_f32, color))
        .rounding(Rounding::same(4.0))
        .inner_margin(Margin::symmetric(6.0, 2.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(10.0).strong().color(color));
        });
}

fn inspector_card(
    ui: &mut Ui,
    icon: &str,
    title: &str,
    accent: Color32,
    header_right: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui),
) {
    Frame::none()
        .fill(CARD_BG)
        .stroke(Stroke::new(1.0_f32, CARD_BORDER))
        .rounding(Rounding::same(8.0))
        .show(ui, |ui| {
            // Card Header Strip
            Frame::none()
                .fill(CARD_HEADER_BG)
                .rounding(Rounding {
                    nw: 7.0,
                    ne: 7.0,
                    sw: 0.0,
                    se: 0.0,
                })
                .inner_margin(Margin::symmetric(10.0, 6.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(icon).size(12.0).color(accent));
                        ui.label(
                            RichText::new(title)
                                .size(11.5)
                                .strong()
                                .color(Color32::from_rgb(0xe8, 0xec, 0xf4)),
                        );
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), header_right);
                    });
                });

            // Thin divider line
            let rect = ui.available_rect_before_wrap();
            ui.painter().line_segment(
                [
                    egui::pos2(rect.left(), rect.top()),
                    egui::pos2(rect.right(), rect.top()),
                ],
                Stroke::new(1.0_f32, CARD_BORDER),
            );

            // Card Body
            Frame::none()
                .inner_margin(Margin::symmetric(10.0, 8.0))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    body(ui);
                });
        });
}

fn prop_row<R>(ui: &mut Ui, label: &str, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.allocate_ui_with_layout(
            Vec2::new(LABEL_COL_W, 20.0),
            Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(RichText::new(label).size(11.0).color(LABEL_COLOR));
            },
        );
        add_contents(ui)
    })
    .inner
}

fn render_sequence_inspector(
    ui: &mut Ui,
    project: &mut Project,
    action: &mut Option<InspectorAction>,
) {
    let first_video_media = project
        .media_pool
        .values()
        .find(|m| m.media_type == MediaType::Video && m.width > 0 && m.height > 0)
        .map(|m| (m.width, m.height, m.fps, m.name.clone()));

    let ar_label = project.timeline.aspect_ratio().short_label();
    inspector_card(
        ui,
        "🎬",
        "Sekans & Proje Ayarları",
        ACCENT_AMBER,
        |ui| {
            ui.label(
                RichText::new(ar_label)
                    .size(10.5)
                    .strong()
                    .color(ACCENT_AMBER),
            );
        },
        |ui| {
            prop_row(ui, "Proje Adı", |ui| {
                if ui
                    .add(
                        TextEdit::singleline(&mut project.settings.name)
                            .desired_width(ui.available_width()),
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Çözünürlük", |ui| {
                let mut w = project.timeline.width;
                let mut h = project.timeline.height;
                let w_changed = ui
                    .add(
                        DragValue::new(&mut w)
                            .speed(2.0)
                            .range(128..=7680)
                            .suffix(" px"),
                    )
                    .changed();
                ui.label(RichText::new("×").size(11.0).color(LABEL_COLOR));
                let h_changed = ui
                    .add(
                        DragValue::new(&mut h)
                            .speed(2.0)
                            .range(128..=7680)
                            .suffix(" px"),
                    )
                    .changed();
                if w_changed || h_changed {
                    project.set_resolution(w, h);
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Hazır Boyut", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    for (lbl, pw, ph) in [
                        ("4K", 3840_u32, 2160_u32),
                        ("1080p", 1920, 1080),
                        ("9:16", 1080, 1920),
                        ("1:1", 1080, 1080),
                    ] {
                        let is_active =
                            project.timeline.width == pw && project.timeline.height == ph;
                        let btn = Button::new(RichText::new(lbl).size(10.5).strong().color(
                            if is_active {
                                Color32::WHITE
                            } else {
                                Color32::from_rgb(0xb8, 0xc0, 0xd4)
                            },
                        ))
                        .fill(if is_active {
                            ACCENT_AMBER
                        } else {
                            Color32::from_rgb(0x1f, 0x23, 0x30)
                        })
                        .rounding(Rounding::same(4.0));
                        if ui
                            .add(btn)
                            .on_hover_text(format!("{}×{} px", pw, ph))
                            .clicked()
                        {
                            project.set_resolution(pw, ph);
                            *action = Some(InspectorAction::ClipModified);
                        }
                    }
                });
            });

            if let Some((vw, vh, vfps, vname)) = first_video_media {
                if project.timeline.width != vw || project.timeline.height != vh {
                    ui.add_space(2.0);
                    let btn_sync = Button::new(
                        RichText::new(format!(
                            "📐 Kaynak Videoya Eşitle ({}×{} @ {:.0}fps)",
                            vw,
                            vh,
                            if vfps > 1.0 {
                                vfps
                            } else {
                                project.timeline.fps
                            }
                        ))
                        .size(11.0)
                        .strong()
                        .color(Color32::from_rgb(0xf5, 0x9e, 0x42)),
                    )
                    .fill(Color32::from_rgba_unmultiplied(0xe0, 0x7a, 0x38, 28))
                    .stroke(Stroke::new(1.0_f32, ACCENT_AMBER))
                    .rounding(Rounding::same(5.0))
                    .min_size(Vec2::new(ui.available_width(), 26.0));
                    if ui
                        .add(btn_sync)
                        .on_hover_text(format!("'{}' medyasının çözünürlüğünü uygula", vname))
                        .clicked()
                    {
                        project.set_resolution(vw, vh);
                        if vfps > 1.0 {
                            project.set_fps(vfps);
                        }
                        *action = Some(InspectorAction::ClipModified);
                    }
                }
            }

            prop_row(ui, "Kare Hızı", |ui| {
                let mut fps = project.timeline.fps;
                if ui
                    .add(
                        DragValue::new(&mut fps)
                            .speed(0.5)
                            .range(1.0..=240.0)
                            .suffix(" fps"),
                    )
                    .changed()
                {
                    project.set_fps(fps);
                    *action = Some(InspectorAction::ClipModified);
                }
                for preset_fps in [24.0, 30.0, 60.0] {
                    let is_curr = (project.timeline.fps - preset_fps).abs() < 0.05;
                    let pill = Button::new(
                        RichText::new(format!("{:.0}", preset_fps))
                            .size(10.0)
                            .strong()
                            .color(if is_curr { Color32::WHITE } else { LABEL_COLOR }),
                    )
                    .fill(if is_curr {
                        ACCENT_BLUE
                    } else {
                        Color32::from_rgb(0x1f, 0x23, 0x30)
                    })
                    .rounding(Rounding::same(4.0));
                    if ui.add(pill).clicked() {
                        project.set_fps(preset_fps);
                        *action = Some(InspectorAction::ClipModified);
                    }
                }
            });
        },
    );

    ui.add_space(6.0);

    let v_tracks = project.timeline.get_video_tracks().len();
    let a_tracks = project.timeline.get_audio_tracks().len();
    let clip_count: usize = project.timeline.tracks.iter().map(|t| t.clips.len()).sum();
    let total_dur = project.timeline.duration();

    inspector_card(
        ui,
        "📊",
        "Zaman Çizgisi Özeti",
        ACCENT_BLUE,
        |_| {},
        |ui| {
            prop_row(ui, "Kanallar", |ui| {
                ui.label(
                    RichText::new(format!("{} Video  •  {} Ses", v_tracks, a_tracks))
                        .size(11.0)
                        .strong()
                        .color(Color32::WHITE),
                );
            });
            prop_row(ui, "Klip Sayısı", |ui| {
                ui.label(
                    RichText::new(format!("{} adet", clip_count))
                        .size(11.0)
                        .color(Color32::from_rgb(0xd0, 0xd6, 0xe6)),
                );
            });
            prop_row(ui, "Toplam Süre", |ui| {
                ui.label(
                    RichText::new(format!("{:.2} sn", total_dur))
                        .size(11.0)
                        .strong()
                        .color(ACCENT_TEAL),
                );
            });
        },
    );

    ui.add_space(10.0);

    Frame::none()
        .fill(SUBCARD_BG)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x1f, 0x22, 0x2e)))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::symmetric(12.0, 14.0))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("🎞  Klip Denetçisi")
                        .size(12.0)
                        .strong()
                        .color(Color32::from_rgb(0xb8, 0xc0, 0xd4)),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(
                        "Dönüşüm, 3D LUT, renk çarkları, ses EQ veya geçiş ayarlarını düzenlemek için zaman çizgisinden bir klip seçin.",
                    )
                    .size(11.0)
                    .color(Color32::from_rgb(0x78, 0x80, 0x94)),
                );
            });
        });
}

fn render_info_and_timing_card(
    ui: &mut Ui,
    clip: &mut Clip,
    track_name: &str,
    is_audio: bool,
    action: &mut Option<InspectorAction>,
) {
    let accent = if is_audio { ACCENT_TEAL } else { ACCENT_BLUE };
    let clip_dur = clip.duration();
    inspector_card(
        ui,
        "🎞",
        "Klip Bilgisi & Zamanlama",
        accent,
        |ui| {
            ui.label(
                RichText::new(format!("{:.2} sn", clip_dur))
                    .size(10.5)
                    .strong()
                    .color(accent),
            );
        },
        |ui| {
            prop_row(ui, "Klip Adı", |ui| {
                if ui
                    .add(TextEdit::singleline(&mut clip.name).desired_width(ui.available_width()))
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Kanal & Süre", |ui| {
                render_badge(ui, track_name, accent);
                ui.label(
                    RichText::new(format!("Toplam {:.2} s", clip.duration()))
                        .size(11.0)
                        .color(Color32::from_rgb(0xd8, 0xde, 0xec)),
                );
            });

            prop_row(ui, "Başlangıç (In)", |ui| {
                let old_dur = clip.duration();
                if ui
                    .add(
                        DragValue::new(&mut clip.timeline_in)
                            .speed(0.05)
                            .range(0.0..=99999.0)
                            .suffix(" sn"),
                    )
                    .changed()
                {
                    clip.timeline_in = clip.timeline_in.max(0.0);
                    clip.timeline_out = clip.timeline_in + old_dur;
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Bitiş (Out)", |ui| {
                if ui
                    .add(
                        DragValue::new(&mut clip.timeline_out)
                            .speed(0.05)
                            .range(0.0..=99999.0)
                            .suffix(" sn"),
                    )
                    .changed()
                {
                    clip.timeline_out = clip.timeline_out.max(clip.timeline_in + 0.04);
                    clip.source_out =
                        Some(clip.source_in + (clip.timeline_out - clip.timeline_in) * clip.speed);
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Renk Etiketi", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    for color in ColorLabel::ALL {
                        let is_active = clip.color_label == *color;
                        let sw_color = if let Some([r, g, b]) = color.color_rgb() {
                            Color32::from_rgb(r, g, b)
                        } else {
                            Color32::from_rgb(80, 88, 105)
                        };
                        let (rect, resp) =
                            ui.allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::click());
                        let painter = ui.painter();
                        if resp.hovered() || is_active {
                            painter.circle_stroke(
                                rect.center(),
                                9.0,
                                Stroke::new(
                                    1.5_f32,
                                    if is_active {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_gray(180)
                                    },
                                ),
                            );
                        }
                        painter.circle_filled(rect.center(), 6.5, sw_color);
                        if resp.on_hover_text(color.label()).clicked() {
                            clip.color_label = *color;
                            *action = Some(InspectorAction::ClipModified);
                        }
                    }
                });
            });

            prop_row(ui, "A/V Senkron", |ui| {
                if clip.linked_group_id.is_some() {
                    render_badge(
                        ui,
                        "🔗 Bağlı (A/V Senkron)",
                        Color32::from_rgb(59, 130, 246),
                    );
                } else {
                    render_badge(ui, "Bağımsız Klip", Color32::from_rgb(100, 116, 139));
                }
            });
        },
    );
}

fn render_transform_card(
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
        "📐",
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
                render_kf_btn(ui, clip, "pos_x", clip_time, clip.pos_x, action);
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
            });

            prop_row(ui, "Konum Y", |ui| {
                render_kf_btn(ui, clip, "pos_y", clip_time, clip.pos_y, action);
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
            });

            prop_row(ui, "Ölçek", |ui| {
                render_kf_btn(ui, clip, "scale", clip_time, clip.scale, action);
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
            });

            prop_row(ui, "Döndürme", |ui| {
                render_kf_btn(ui, clip, "rotation", clip_time, clip.rotation, action);
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
            });

            prop_row(ui, "Opaklık", |ui| {
                render_kf_btn(ui, clip, "opacity", clip_time, clip.opacity, action);
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

fn render_crop_card(
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
        "✂️",
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

fn render_speed_card(ui: &mut Ui, clip: &mut Clip, action: &mut Option<InspectorAction>) {
    let curr_speed = clip.speed;
    inspector_card(
        ui,
        "⚡",
        "Hız, Yön & Akıllı Kurgu",
        ACCENT_AMBER,
        |ui| {
            ui.label(
                RichText::new(format!("{:.2}x", curr_speed))
                    .size(10.5)
                    .strong()
                    .color(ACCENT_AMBER),
            );
        },
        |ui| {
            prop_row(ui, "Oynatma Hızı", |ui| {
                let mut s = clip.speed;
                if ui
                    .add(
                        DragValue::new(&mut s)
                            .speed(0.05)
                            .range(0.1..=10.0)
                            .suffix("x"),
                    )
                    .changed()
                {
                    clip.change_speed(s);
                    *action = Some(InspectorAction::ChangeSpeed(s, clip.reverse));
                }

                for preset in [0.5, 1.0, 1.5, 2.0] {
                    let is_active = (clip.speed - preset).abs() < 0.03;
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
                    if ui.add(pill).clicked() {
                        clip.change_speed(preset);
                        *action = Some(InspectorAction::ChangeSpeed(preset, clip.reverse));
                    }
                }
            });

            prop_row(ui, "Oynatma Yönü", |ui| {
                let mut rev = clip.reverse;
                if ui.checkbox(&mut rev, "Ters Oynat (Reverse)").changed() {
                    clip.reverse = rev;
                    *action = Some(InspectorAction::ChangeSpeed(clip.speed, rev));
                }
            });

            if !clip.media_id.is_empty() {
                ui.add_space(4.0);
                let btn_scene = Button::new(
                    RichText::new("🎬  Sahneleri Algıla ve Böl (Smart Cut)")
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
                    RichText::new("⚡  Sessizlikleri Otomatik Kes (Jump-Cut)")
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

fn draw_look_preset_thumbnail(ui: &mut Ui, rect: egui::Rect, preset: LutPreset) {
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        Rounding::same(4.0),
        Color32::from_rgb(0x0b, 0x0d, 0x12),
    );

    // Render 4 horizontal bands representing sky/cool, skin/warm, foliage/mid, and deep shadow
    // passed directly through `preset.apply_rgb` for an authentic live look preview.
    let sample_colors: [[f32; 3]; 4] = [
        [0.24, 0.56, 0.86], // Sky / Cyan-Blue highlight
        [0.88, 0.62, 0.44], // Skin / Warm mid-high
        [0.36, 0.64, 0.38], // Foliage / Green midtone
        [0.18, 0.20, 0.28], // Shadow
    ];
    let band_w = rect.width() / 4.0;
    for (idx, base) in sample_colors.iter().enumerate() {
        let [r, g, b] = preset.apply_rgb(base[0], base[1], base[2]);
        let col = Color32::from_rgb(
            (r * 255.0).round() as u8,
            (g * 255.0).round() as u8,
            (b * 255.0).round() as u8,
        );
        let x0 = rect.left() + idx as f32 * band_w;
        let x1 = if idx == 3 { rect.right() } else { x0 + band_w };
        let sub =
            egui::Rect::from_min_max(egui::pos2(x0, rect.top()), egui::pos2(x1, rect.bottom()));
        let rounding = match idx {
            0 => Rounding {
                nw: 4.0,
                sw: 4.0,
                ne: 0.0,
                se: 0.0,
            },
            3 => Rounding {
                nw: 0.0,
                sw: 0.0,
                ne: 4.0,
                se: 4.0,
            },
            _ => Rounding::ZERO,
        };
        painter.rect_filled(sub, rounding, col);
    }
    painter.rect_stroke(
        rect,
        Rounding::same(4.0),
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 28)),
    );
}

fn draw_bc_preset_thumbnail(ui: &mut Ui, rect: egui::Rect, preset: BrightnessContrastPreset) {
    let painter = ui.painter();
    let (br, ct, gm) = preset.params();
    let steps = 5usize;
    let step_w = rect.width() / steps as f32;
    let inv_g = (1.0 / gm.max(0.1)) as f32;

    for i in 0..steps {
        let t = (i as f32 + 0.5) / steps as f32;
        let mut v = (ct as f32) * (t - 0.5) + 0.5 + (br as f32);
        v = v.clamp(0.0, 1.0).powf(inv_g).clamp(0.0, 1.0);
        let u = (v * 255.0).round() as u8;
        let col = Color32::from_rgb(u, u, (u as u16 * 105 / 100).min(255) as u8);
        let x0 = rect.left() + i as f32 * step_w;
        let x1 = if i + 1 == steps {
            rect.right()
        } else {
            x0 + step_w
        };
        let sub =
            egui::Rect::from_min_max(egui::pos2(x0, rect.top()), egui::pos2(x1, rect.bottom()));
        painter.rect_filled(sub, Rounding::ZERO, col);
    }
    painter.rect_stroke(
        rect,
        Rounding::same(3.0),
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 24)),
    );
}

fn render_color_card(
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
                    old_state,
                    new_state,
                });
            }
            ui.data_mut(|d| d.remove::<ClipColorGradingState>(cg_drag_id));
        }
    };

    let reset_colors = std::cell::Cell::new(false);
    inspector_card(
        ui,
        "🎨",
        "Renk Düzenleme, Presetler & 3D LUT",
        ACCENT_BLUE,
        |ui| {
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
            if reset_colors.get() {
                let old_state = ClipColorGradingState::from_clip(clip);
                clip.reset_color_grading();
                clip.color_wheels = None;
                let new_state = ClipColorGradingState::from_clip(clip);
                *action = Some(InspectorAction::SetColorGrading {
                    track_id: track_id.to_string(),
                    clip_id: clip.id.clone(),
                    old_state,
                    new_state,
                });
            }

            // =================================================================
            // 1. WHITE BALANCE (BEYAZ DENGESİ)
            // =================================================================
            Frame::none()
                .fill(SUBCARD_BG)
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x24, 0x30)))
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(8.0, 7.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("☀ Beyaz Dengesi (White Balance)")
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
                                            RichText::new("Sıfırla")
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
                                    old_state,
                                    new_state,
                                });
                            }
                            render_badge(ui, clip.wb_preset.short_label(), ACCENT_AMBER);
                        });
                    });

                    ui.add_space(4.0);

                    // Auto White Balance Button
                    let is_auto = clip.wb_preset == WhiteBalancePreset::Auto;
                    let auto_btn = Button::new(
                        RichText::new("✨ Otomatik Beyaz Dengesi (Auto WB - Kare Analizi)")
                            .size(11.0)
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
                    .rounding(Rounding::same(5.0))
                    .min_size(Vec2::new(ui.available_width(), 26.0));

                    if ui
                        .add(auto_btn)
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

                    ui.add_space(4.0);

                    // WB Presets Grid (7 illumination presets)
                    let wb_presets = [
                        WhiteBalancePreset::Daylight,
                        WhiteBalancePreset::Tungsten,
                        WhiteBalancePreset::Cloudy,
                        WhiteBalancePreset::Shade,
                        WhiteBalancePreset::Fluorescent,
                        WhiteBalancePreset::Warm,
                        WhiteBalancePreset::Cool,
                    ];

                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                        for preset in wb_presets {
                            let is_sel = clip.wb_preset == preset;
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

                            if ui.add(btn).on_hover_text(preset.label()).clicked() {
                                let old_state = ClipColorGradingState::from_clip(clip);
                                clip.apply_wb_preset(preset);
                                let new_state = ClipColorGradingState::from_clip(clip);
                                *action = Some(InspectorAction::SetColorGrading {
                                    track_id: track_id.to_string(),
                                    clip_id: clip.id.clone(),
                                    old_state,
                                    new_state,
                                });
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
                            RichText::new("🌗 Parlaklık & Kontrast Hazır Ayarları")
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
                                        old_state,
                                        new_state,
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
                            RichText::new("🎬 Yaratıcı Renk Görünümleri (Color Looks)")
                                .size(11.5)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            render_badge(ui, clip.lut_preset.short_label(), ACCENT_BLUE);
                        });
                    });

                    ui.add_space(4.0);

                    // Category filter tabs stored in egui temp data
                    let cat_tab_id = ui.make_persistent_id(format!("look_cat_tab_{}", clip.id));
                    let mut active_tab: u8 =
                        ui.data_mut(|d| d.get_temp::<u8>(cat_tab_id)).unwrap_or(0);

                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                        let tabs: [(u8, &str); 4] = [
                            (0, "Tümü"),
                            (1, "🎬 Sinematik"),
                            (2, "🎞 Film"),
                            (3, "🎨 Yaratıcı"),
                        ];
                        for (tab_id, title) in tabs {
                            let is_active = active_tab == tab_id;
                            let btn = Button::new(RichText::new(title).size(10.5).strong().color(
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
                            if ui.add(btn).clicked() {
                                active_tab = tab_id;
                                ui.data_mut(|d| d.insert_temp(cat_tab_id, active_tab));
                            }
                        }
                    });

                    ui.add_space(5.0);

                    let filtered_looks: Vec<LutPreset> = match active_tab {
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
                        _ => LutPreset::ALL.to_vec(),
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

                                if resp.on_hover_text(preset.label()).clicked() {
                                    let old_state = ClipColorGradingState::from_clip(clip);
                                    clip.lut_preset = *preset;
                                    let new_state = ClipColorGradingState::from_clip(clip);
                                    if old_state != new_state {
                                        *action = Some(InspectorAction::SetColorGrading {
                                            track_id: track_id.to_string(),
                                            clip_id: clip.id.clone(),
                                            old_state,
                                            new_state,
                                        });
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
                                    old_state,
                                    new_state,
                                });
                            }
                            ui.label(
                                RichText::new(format!("📁 {}", fname))
                                    .size(11.0)
                                    .color(Color32::from_rgb(0x58, 0xb0, 0xff)),
                            );
                        } else if ui
                            .add(
                                Button::new(
                                    RichText::new("📁 .cube Dosyası Seç...")
                                        .size(11.0)
                                        .color(Color32::from_rgb(0xc8, 0xd0, 0xe4)),
                                )
                                .fill(Color32::from_rgb(0x1f, 0x23, 0x30))
                                .rounding(Rounding::same(5.0)),
                            )
                            .clicked()
                        {
                            *action = Some(InspectorAction::SelectCustomLut(clip.id.clone()));
                        }
                    });
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
                            RichText::new("🟢 Yeşil / Mavi Perde (Chroma Key)")
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
        },
    );
}

fn render_transitions_card(ui: &mut Ui, clip: &mut Clip, action: &mut Option<InspectorAction>) {
    inspector_card(
        ui,
        "✨",
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

fn render_audio_card(
    ui: &mut Ui,
    track_id: &str,
    clip: &mut Clip,
    action: &mut Option<InspectorAction>,
) {
    let pct = (clip.volume * 100.0).round() as i32;
    let is_muted = clip.muted;
    inspector_card(
        ui,
        "🔊",
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
                            RichText::new("Görsel Ses Dalgası (Audio Visualizer)")
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

fn render_titler_card(ui: &mut Ui, clip: &mut Clip, action: &mut Option<InspectorAction>) {
    inspector_card(
        ui,
        "🔤",
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
                ui.label(RichText::new("Yazı İçeriği:").size(11.0).color(LABEL_COLOR));
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
                    egui::ComboBox::from_id_salt("inspector_font_family")
                        .selected_text(&txt.font_family)
                        .show_ui(ui, |ui| {
                            for font in ["Sans-Serif", "Serif", "Monospace", "Impact", "Montserrat"]
                            {
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
                    for (lbl, py, bg) in [
                        ("Üst", 0.15_f32, None),
                        ("Orta", 0.50_f32, None),
                        ("Alt Yazı", 0.85_f32, Some([0, 0, 0, 180])),
                    ] {
                        if ui
                            .add(
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

                prop_row(ui, "Stil & Kutu", |ui| {
                    if ui.checkbox(&mut txt.has_shadow, "Gölge").changed() {
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

                if txt.motion_style == MotionTitleStyle::LowerThird {
                    prop_row(ui, "Alt Başlık", |ui| {
                        let mut sub = txt.subtitle_text.clone().unwrap_or_default();
                        if ui
                            .add(TextEdit::singleline(&mut sub).hint_text("Ünvan / Açıklama"))
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
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        Button::new(RichText::new("📥 SRT İçe Aktar").size(10.5))
                            .fill(Color32::from_rgb(0x1f, 0x23, 0x30))
                            .rounding(Rounding::same(5.0)),
                    )
                    .clicked()
                {
                    *action = Some(InspectorAction::ImportSubtitles);
                }
                if ui
                    .add(
                        Button::new(RichText::new("📤 SRT Dışa Aktar").size(10.5))
                            .fill(Color32::from_rgb(0x1f, 0x23, 0x30))
                            .rounding(Rounding::same(5.0)),
                    )
                    .clicked()
                {
                    *action = Some(InspectorAction::ExportSubtitles);
                }
            });

            let btn_whisper = Button::new(
                RichText::new("🎙️  Lokal Whisper ile Otomatik Deşifre Et")
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

fn render_compositing_card(
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
        "Karışım Modu (Blend)"
    };
    let clip_id = clip.id.clone();
    let is_adj = clip.is_adjustment_layer();

    inspector_card(
        ui,
        "✨",
        title,
        accent,
        move |ui| {
            if is_adj {
                render_badge(ui, "AYAR KATMANI", Color32::from_rgb(0xb8, 0x58, 0xff));
            }
        },
        |ui| {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [LABEL_COL_W, 20.0],
                    egui::Label::new(RichText::new("Karışım Modu").size(11.5).color(LABEL_COLOR)),
                );
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

            if clip.is_adjustment_layer() {
                ui.add_space(4.0);
                let op_drag_id = ui.make_persistent_id(format!("adj_op_drag_{}", clip.id));
                let orig_op: Option<(f64, f64, f64, f64, f64)> =
                    ui.data_mut(|d| d.get_temp(op_drag_id));
                let cur_op = clip.get_property_at_time("opacity", clip_time);
                let mut op_val = cur_op;
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [LABEL_COL_W, 20.0],
                        egui::Label::new(RichText::new("Opaklık").size(11.5).color(LABEL_COLOR)),
                    );
                    let resp = ui.add(
                        egui::Slider::new(&mut op_val, 0.0..=1.0)
                            .fixed_decimals(2)
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

fn render_masks_card(
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
        "🎭",
        "Maskeler",
        ACCENT_TEAL,
        move |ui| {
            ui.label(RichText::new(count_txt).size(10.5).color(LABEL_COLOR));
        },
        |ui| {
            ui.horizontal_wrapped(|ui| {
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
                        .add(
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
                                            RichText::new("🗑").size(10.5).color(Color32::WHITE),
                                        )
                                        .fill(Color32::from_rgb(0x3a, 0x20, 0x28))
                                        .rounding(Rounding::same(4.0)),
                                    )
                                    .on_hover_text("Maskeyi Sil")
                                    .clicked()
                                {
                                    remove_idx = Some(idx);
                                }
                            });
                        });

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [62.0, 20.0],
                                egui::Label::new(
                                    RichText::new("Hedef").size(11.0).color(LABEL_COLOR),
                                ),
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
                                    ui.add_sized(
                                        [68.0, 18.0],
                                        egui::Label::new(
                                            RichText::new(label).size(10.5).color(LABEL_COLOR),
                                        ),
                                    );
                                    let resp = ui.add(
                                        egui::Slider::new(val, range)
                                            .fixed_decimals(2)
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

fn render_mask_kf_btn(
    ui: &mut Ui,
    mask: &mut trim_core::Mask,
    prop: &str,
    clip_time: f64,
    val: f64,
) -> bool {
    let has_kf = mask
        .keyframes
        .get(prop)
        .is_some_and(|list| list.iter().any(|k| (k.time - clip_time).abs() <= 0.05));
    let (btn_txt, btn_col, bg_col) = if has_kf {
        ("◆", Color32::WHITE, ACCENT_AMBER)
    } else {
        (
            "◇",
            Color32::from_rgb(0x88, 0x90, 0xa4),
            Color32::from_rgb(0x1d, 0x20, 0x2b),
        )
    };
    if ui
        .add(
            Button::new(RichText::new(btn_txt).size(10.0).strong().color(btn_col))
                .fill(bg_col)
                .rounding(Rounding::same(4.0))
                .min_size(Vec2::new(20.0, 18.0)),
        )
        .on_hover_text("Maske Anahtar Karesi (Keyframe)")
        .clicked()
    {
        if has_kf {
            if let Some(list) = mask.keyframes.get_mut(prop) {
                list.retain(|k| (k.time - clip_time).abs() > 0.05);
            }
        } else {
            let list = mask.keyframes.entry(prop.to_string()).or_default();
            list.push(trim_core::keyframe::Keyframe {
                time: clip_time,
                value: val,
                interpolation: Interpolation::Linear,
            });
            list.sort_by(|a, b| a.time.total_cmp(&b.time));
        }
        return true;
    }
    false
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

    let (btn_txt, btn_col, bg_col) = if has_kf {
        ("◆", Color32::WHITE, ACCENT_AMBER)
    } else {
        (
            "◇",
            Color32::from_rgb(0x88, 0x90, 0xa4),
            Color32::from_rgb(0x1d, 0x20, 0x2b),
        )
    };

    if ui
        .add(
            Button::new(RichText::new(btn_txt).size(10.5).strong().color(btn_col))
                .fill(bg_col)
                .rounding(Rounding::same(4.0))
                .min_size(Vec2::new(22.0, 20.0)),
        )
        .on_hover_text(if has_kf {
            "Bu karedeki anahtar kareyi (Keyframe) kaldır"
        } else {
            "Bu kareye anahtar kare (Keyframe) ekle"
        })
        .clicked()
    {
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
