//! Clip Properties and Sequence Inspector panel in Turkish.

use crate::color_wheel::render_color_wheels_panel;
use egui::{
    Button, Color32, DragValue, Frame, Layout, Margin, RichText, Rounding, ScrollArea, Slider,
    Stroke, TextEdit, Ui, Vec2,
};
use trim_core::clip::{
    AudioEqPreset, AudioVisualizerConfig, CaptionStyle, ChromaKeyConfig, Clip, ColorLabel,
    ColorWheelsConfig, LutPreset, MotionTitleStyle, PanZoomPreset, TextOverlay, VisualizerStyle,
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
            .map(|t| (t.name.clone(), t.track_type == TrackType::Audio))
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
                    if let Some((ref t_name, is_audio)) = track_meta {
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
        if let Some((t_name, is_audio)) = track_meta {
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
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                render_sequence_inspector(ui, project, &mut action);
            });
        return action;
    }

    let (track_name, is_audio, clip) = target_clip_and_track.unwrap();
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
                render_audio_card(ui, clip, &mut action);
                ui.add_space(6.0);

                render_speed_card(ui, clip, &mut action);
                ui.add_space(6.0);

                render_transitions_card(ui, clip, &mut action);
            } else {
                // 2. Video Dönüşümü & Anahtar Kareler
                render_transform_card(ui, clip, clip_time, &mut action);
                ui.add_space(6.0);

                // 3. Renk, LUT & Chroma Key
                render_color_card(ui, clip, &mut action);
                ui.add_space(6.0);

                // 4. Ses & EQ Profili
                render_audio_card(ui, clip, &mut action);
                ui.add_space(6.0);

                // 5. Hız, Yön & Akıllı Kurgu
                render_speed_card(ui, clip, &mut action);
                ui.add_space(6.0);

                // 6. Giriş / Çıkış ve Geçişler
                render_transitions_card(ui, clip, &mut action);

                if clip.text_overlay.is_none() {
                    ui.add_space(6.0);
                    render_titler_card(ui, clip, &mut action);
                }
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
    clip: &mut Clip,
    clip_time: f64,
    action: &mut Option<InspectorAction>,
) {
    let is_modified = clip.pos_x.abs() > 0.01
        || clip.pos_y.abs() > 0.01
        || (clip.scale - 1.0).abs() > 0.005
        || (clip.opacity - 1.0).abs() > 0.005;

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
                clip.pos_x = 0.0;
                clip.pos_y = 0.0;
                clip.scale = 1.0;
                clip.opacity = 1.0;
                *action = Some(InspectorAction::ClipModified);
            }

            prop_row(ui, "Konum X", |ui| {
                render_kf_btn(ui, clip, "pos_x", clip_time, clip.pos_x, action);
                if ui
                    .add(DragValue::new(&mut clip.pos_x).speed(1.0).suffix(" px"))
                    .changed()
                {
                    sync_kf_value(clip, "pos_x", clip_time, clip.pos_x);
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Konum Y", |ui| {
                render_kf_btn(ui, clip, "pos_y", clip_time, clip.pos_y, action);
                if ui
                    .add(DragValue::new(&mut clip.pos_y).speed(1.0).suffix(" px"))
                    .changed()
                {
                    sync_kf_value(clip, "pos_y", clip_time, clip.pos_y);
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Ölçek", |ui| {
                render_kf_btn(ui, clip, "scale", clip_time, clip.scale, action);
                if ui
                    .add(
                        Slider::new(&mut clip.scale, 0.1..=4.0)
                            .show_value(false)
                            .trailing_fill(true),
                    )
                    .changed()
                {
                    sync_kf_value(clip, "scale", clip_time, clip.scale);
                    *action = Some(InspectorAction::ClipModified);
                }
                if ui
                    .add(
                        DragValue::new(&mut clip.scale)
                            .speed(0.01)
                            .range(0.05..=10.0)
                            .fixed_decimals(2),
                    )
                    .changed()
                {
                    sync_kf_value(clip, "scale", clip_time, clip.scale);
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Opaklık", |ui| {
                render_kf_btn(ui, clip, "opacity", clip_time, clip.opacity, action);
                if ui
                    .add(
                        Slider::new(&mut clip.opacity, 0.0..=1.0)
                            .show_value(false)
                            .trailing_fill(true),
                    )
                    .changed()
                {
                    sync_kf_value(clip, "opacity", clip_time, clip.opacity);
                    *action = Some(InspectorAction::ClipModified);
                }
                if ui
                    .add(
                        DragValue::new(&mut clip.opacity)
                            .speed(0.01)
                            .range(0.0..=1.0)
                            .fixed_decimals(2),
                    )
                    .changed()
                {
                    sync_kf_value(clip, "opacity", clip_time, clip.opacity);
                    *action = Some(InspectorAction::ClipModified);
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

fn render_color_card(ui: &mut Ui, clip: &mut Clip, action: &mut Option<InspectorAction>) {
    let has_color_changes = clip.brightness.abs() > 0.005
        || (clip.contrast - 1.0).abs() > 0.005
        || (clip.saturation - 1.0).abs() > 0.005
        || clip.lut_preset != LutPreset::None
        || clip.custom_lut_path.is_some()
        || clip.color_wheels.is_some();

    let reset_colors = std::cell::Cell::new(false);
    inspector_card(
        ui,
        "🎨",
        "Renk, 3D LUT & Chroma Key",
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
                    .on_hover_text("Tüm renk ve LUT ayarlarını varsayılana döndür")
                    .clicked()
            {
                reset_colors.set(true);
            }
        },
        |ui| {
            if reset_colors.get() {
                clip.brightness = 0.0;
                clip.contrast = 1.0;
                clip.saturation = 1.0;
                clip.lut_preset = LutPreset::None;
                clip.custom_lut_path = None;
                clip.color_wheels = None;
                *action = Some(InspectorAction::ClipModified);
            }

            prop_row(ui, "Sinematik LUT", |ui| {
                let mut curr_lut = clip.lut_preset;
                egui::ComboBox::from_id_salt("lut_preset_combo")
                    .selected_text(curr_lut.label())
                    .width(ui.available_width().max(110.0))
                    .show_ui(ui, |ui| {
                        for preset in LutPreset::ALL {
                            if ui
                                .selectable_value(&mut curr_lut, *preset, preset.label())
                                .clicked()
                            {
                                clip.lut_preset = curr_lut;
                                *action = Some(InspectorAction::ClipModified);
                            }
                        }
                    });
            });

            prop_row(ui, "Özel .cube LUT", |ui| {
                if let Some(p) = &clip.custom_lut_path {
                    let fname = trim_core::paths::normalize_path(p)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| p.clone());
                    if ui
                        .add(
                            Button::new(RichText::new("✕").size(10.0).color(Color32::WHITE))
                                .fill(Color32::from_rgb(0x3a, 0x20, 0x28))
                                .rounding(Rounding::same(4.0)),
                        )
                        .on_hover_text("Özel LUT'u Kaldır")
                        .clicked()
                    {
                        clip.custom_lut_path = None;
                        *action = Some(InspectorAction::ClipModified);
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

            prop_row(ui, "Parlaklık", |ui| {
                if ui
                    .add(
                        Slider::new(&mut clip.brightness, -1.0..=1.0)
                            .show_value(false)
                            .trailing_fill(true),
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
                if ui
                    .add(
                        DragValue::new(&mut clip.brightness)
                            .speed(0.02)
                            .range(-1.0..=1.0)
                            .fixed_decimals(2),
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Kontrast", |ui| {
                if ui
                    .add(
                        Slider::new(&mut clip.contrast, 0.0..=3.0)
                            .show_value(false)
                            .trailing_fill(true),
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
                if ui
                    .add(
                        DragValue::new(&mut clip.contrast)
                            .speed(0.02)
                            .range(0.0..=3.0)
                            .fixed_decimals(2),
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Doygunluk", |ui| {
                if ui
                    .add(
                        Slider::new(&mut clip.saturation, 0.0..=3.0)
                            .show_value(false)
                            .trailing_fill(true),
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
                if ui
                    .add(
                        DragValue::new(&mut clip.saturation)
                            .speed(0.02)
                            .range(0.0..=3.0)
                            .fixed_decimals(2),
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            // Subcard: 3-Point Color Wheels
            ui.add_space(4.0);
            Frame::none()
                .fill(SUBCARD_BG)
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x24, 0x30)))
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
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
                        clip.color_wheels = if has_cw {
                            Some(ColorWheelsConfig::default())
                        } else {
                            None
                        };
                        *action = Some(InspectorAction::ClipModified);
                    }

                    if let Some(cw) = &mut clip.color_wheels {
                        ui.add_space(4.0);
                        if render_color_wheels_panel(ui, cw) {
                            *action = Some(InspectorAction::ClipModified);
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

fn render_audio_card(ui: &mut Ui, clip: &mut Clip, action: &mut Option<InspectorAction>) {
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
                            {
                                clip.eq_preset = *p;
                                *action = Some(InspectorAction::ClipModified);
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
