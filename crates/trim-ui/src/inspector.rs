//! Clip Properties and Sequence Inspector panel in Turkish.

use egui::{
    Button, Color32, DragValue, Frame, Layout, Margin, Pos2, RichText, Rounding, ScrollArea,
    Stroke, TextEdit, Ui, Vec2,
};
use trim_commands::commands::ClipColorGradingState;
use trim_core::clip::{AudioEqPreset, Clip, ColorLabel, ColorWheelsConfig};
use trim_core::keyframe::Interpolation;
use trim_core::media::MediaType;
use trim_core::project::Project;
use trim_core::track::TrackType;

#[path = "inspector/color_card/mod.rs"]
mod color_card;
#[path = "inspector/media_cards/mod.rs"]
mod media_cards;
#[path = "inspector/transform_cards.rs"]
mod transform_cards;
use color_card::render_color_card;
use media_cards::{
    render_audio_card, render_compositing_card, render_masks_card, render_titler_card,
    render_transitions_card,
};
use transform_cards::{render_crop_card, render_speed_card, render_transform_card};
pub enum InspectorAction {
    ClipModified,
    ChangeSpeed(f64, bool),
    TriggerSceneDetect,
    TriggerSilenceDetect,
    TrackMaskMotion {
        track_id: String,
        clip_id: String,
        mask_index: usize,
    },
    AutoDuckAudio {
        track_id: String,
        clip_id: String,
    },
    TriggerTranscribe,
    ImportSubtitles,
    ExportSubtitles,
    SelectCustomLut(String),
    SelectCustomFont(String),
    GenerateTitleFoley {
        track_id: String,
        clip_id: String,
    },
    ExportCubeLut {
        track_id: String,
        clip_id: String,
    },
    AutoWhiteBalance {
        track_id: String,
        clip_id: String,
    },
    StartWbEyedropper {
        track_id: String,
        clip_id: String,
    },
    ShotMatch {
        track_id: String,
        clip_id: String,
    },
    SetColorGrading {
        track_id: String,
        clip_id: String,
        old_state: Box<ClipColorGradingState>,
        new_state: Box<ClipColorGradingState>,
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

pub(super) const CARD_BG: Color32 = Color32::from_rgb(0x13, 0x15, 0x1c);
pub(super) const CARD_HEADER_BG: Color32 = Color32::from_rgb(0x18, 0x1b, 0x24);
pub(super) const CARD_BORDER: Color32 = Color32::from_rgb(0x25, 0x29, 0x36);
pub(super) const SUBCARD_BG: Color32 = Color32::from_rgb(0x0f, 0x11, 0x17);
pub(super) const LABEL_COLOR: Color32 = Color32::from_rgb(0x9c, 0xa3, 0xb8);
pub(super) const ACCENT_AMBER: Color32 = Color32::from_rgb(0xe0, 0x7a, 0x38);
pub(super) const ACCENT_BLUE: Color32 = Color32::from_rgb(0x38, 0x7a, 0xe0);
pub(super) const ACCENT_TEAL: Color32 = Color32::from_rgb(0x2d, 0xb8, 0x9a);
pub(super) const LABEL_COL_W: f32 = 104.0;

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

    let Some((track_id, track_name, is_audio, clip)) = target_clip_and_track else {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                render_sequence_inspector(ui, project, &mut action);
            });
        return action;
    };
    let clip_time = (playhead_time - clip.timeline_in).max(0.0);

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // 1. Klip Bilgisi & Zamanlama
            render_info_and_timing_card(ui, clip, &track_name, is_audio, &mut action);
            ui.add_space(6.0);

            // If clip has text overlay enabled, surface Titler near the top
            if !is_audio && clip.text_overlay.is_some() {
                render_titler_card(ui, &track_id, clip, &mut action);
                ui.add_space(6.0);
            }

            if is_audio {
                render_audio_card(ui, &track_id, clip, &mut action);
                ui.add_space(6.0);

                render_speed_card(ui, clip, clip_time, &mut action);
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
                render_speed_card(ui, clip, clip_time, &mut action);
                ui.add_space(6.0);

                // 6. Giriş / Çıkış ve Geçişler
                render_transitions_card(ui, clip, &mut action);
            }
            ui.add_space(8.0);
        });

    action
}

pub(super) fn render_badge(ui: &mut Ui, text: &str, color: Color32) {
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

pub(super) fn inspector_card(
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
                        ui.label(RichText::new(icon).size(11.5).strong().color(accent));
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
                ui.add(
                    egui::Label::new(RichText::new(label).size(11.0).color(LABEL_COLOR)).truncate(),
                );
            },
        );
        ui.spacing_mut().slider_width = (ui.available_width() - 56.0).max(60.0);
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
        "▸",
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
                ui.spacing_mut().item_spacing.x = 4.0;
                let btn_w = ((ui.available_width() - 4.0 * 3.0) / 4.0).max(36.0);
                for (lbl, pw, ph) in [
                    ("4K", 3840_u32, 2160_u32),
                    ("1080p", 1920, 1080),
                    ("9:16", 1080, 1920),
                    ("1:1", 1080, 1080),
                ] {
                    let is_active = project.timeline.width == pw && project.timeline.height == ph;
                    let btn =
                        Button::new(RichText::new(lbl).size(10.5).strong().color(if is_active {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(0xb8, 0xc0, 0xd4)
                        }))
                        .fill(if is_active {
                            ACCENT_AMBER
                        } else {
                            Color32::from_rgb(0x1f, 0x23, 0x30)
                        })
                        .rounding(Rounding::same(4.0));
                    if ui
                        .add_sized([btn_w, 20.0], btn)
                        .on_hover_text(format!("{}×{} px", pw, ph))
                        .clicked()
                    {
                        project.set_resolution(pw, ph);
                        *action = Some(InspectorAction::ClipModified);
                    }
                }
            });

            if let Some((vw, vh, vfps, vname)) = first_video_media {
                if project.timeline.width != vw || project.timeline.height != vh {
                    ui.add_space(2.0);
                    let btn_sync = Button::new(
                        RichText::new(format!(
                            "⊞ Kaynak Videoya Eşitle ({}×{} @ {:.0}fps)",
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
                    if ui.add_sized([30.0, 20.0], pill).clicked() {
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
        "≡",
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
                    RichText::new("◈  Klip Denetçisi")
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
        "▸",
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
                    render_badge(ui, "● Bağlı (A/V Senkron)", Color32::from_rgb(59, 130, 246));
                } else {
                    render_badge(ui, "Bağımsız Klip", Color32::from_rgb(100, 116, 139));
                }
            });
        },
    );
}

pub(super) fn render_mask_kf_btn(
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
            list.push(trim_core::keyframe::Keyframe::new(
                clip_time,
                val,
                Interpolation::Linear,
            ));
            list.sort_by(|a, b| a.time.total_cmp(&b.time));
        }
        return true;
    }
    false
}

pub(super) fn render_kf_btn(
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

    if has_kf {
        if let Some(list) = clip.keyframes.get_mut(prop) {
            if let Some(kf) = list.iter_mut().find(|k| (k.time - clip_time).abs() <= 0.05) {
                let popup_id = ui.make_persistent_id(format!("kf_curve_{}_{}", clip.id, prop));
                let curve_btn = ui
                    .add(
                        Button::new(
                            RichText::new(kf.interpolation.short_icon())
                                .size(10.0)
                                .strong()
                                .color(ACCENT_BLUE),
                        )
                        .fill(Color32::from_rgb(0x18, 0x20, 0x2e))
                        .rounding(Rounding::same(4.0))
                        .min_size(Vec2::new(20.0, 20.0)),
                    )
                    .on_hover_text(format!("Eğri Tipi: {}", kf.interpolation.label()));

                if curve_btn.clicked() {
                    ui.memory_mut(|m| m.toggle_popup(popup_id));
                }

                egui::popup_below_widget(
                    ui,
                    popup_id,
                    &curve_btn,
                    egui::PopupCloseBehavior::CloseOnClickOutside,
                    |ui| {
                        ui.set_min_width(175.0);
                        ui.label(
                            RichText::new("Keyframe Eğrisi")
                                .size(11.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.add_space(2.0);
                        for interp in Interpolation::ALL {
                            if ui
                                .selectable_value(
                                    &mut kf.interpolation,
                                    *interp,
                                    format!("{}  {}", interp.short_icon(), interp.label()),
                                )
                                .changed()
                            {
                                *action = Some(InspectorAction::ClipModified);
                            }
                        }

                        ui.add_space(4.0);
                        let (g_rect, g_resp) = ui.allocate_exact_size(
                            Vec2::new(165.0, 74.0),
                            egui::Sense::click_and_drag(),
                        );
                        let painter = ui.painter_at(g_rect);
                        painter.rect_filled(g_rect, 4.0, Color32::from_rgb(0x11, 0x14, 0x1c));
                        painter.rect_stroke(
                            g_rect,
                            4.0,
                            Stroke::new(1.0_f32, Color32::from_rgb(0x28, 0x30, 0x42)),
                        );

                        let pad = 6.0;
                        let inner = g_rect.shrink(pad);
                        let mut pts = Vec::with_capacity(33);
                        for s in 0..=32 {
                            let t = s as f64 / 32.0;
                            let u = if kf.interpolation == Interpolation::Hold {
                                if s == 32 {
                                    1.0
                                } else {
                                    0.0
                                }
                            } else {
                                kf.interpolation
                                    .evaluate_curve(t, kf.bezier_handles)
                                    .clamp(0.0, 1.0)
                            };
                            let px = inner.left() + (t as f32) * inner.width();
                            let py = inner.bottom() - (u as f32) * inner.height();
                            pts.push(Pos2::new(px, py));
                        }
                        for w in pts.windows(2) {
                            painter.line_segment([w[0], w[1]], Stroke::new(1.8_f32, ACCENT_BLUE));
                        }

                        if kf.interpolation == Interpolation::CubicBezier {
                            let p1 = Pos2::new(
                                inner.left() + kf.bezier_handles[0] * inner.width(),
                                inner.bottom() - kf.bezier_handles[1] * inner.height(),
                            );
                            let p2 = Pos2::new(
                                inner.left() + kf.bezier_handles[2] * inner.width(),
                                inner.bottom() - kf.bezier_handles[3] * inner.height(),
                            );
                            painter.line_segment(
                                [Pos2::new(inner.left(), inner.bottom()), p1],
                                Stroke::new(1.0_f32, Color32::from_gray(110)),
                            );
                            painter.line_segment(
                                [Pos2::new(inner.right(), inner.top()), p2],
                                Stroke::new(1.0_f32, Color32::from_gray(110)),
                            );
                            painter.circle_filled(p1, 4.0, ACCENT_AMBER);
                            painter.circle_filled(p2, 4.0, ACCENT_TEAL);

                            if g_resp.dragged() {
                                if let Some(pos) = g_resp.interact_pointer_pos() {
                                    let nx =
                                        ((pos.x - inner.left()) / inner.width()).clamp(0.0, 1.0);
                                    let ny =
                                        ((inner.bottom() - pos.y) / inner.height()).clamp(0.0, 1.0);
                                    if pos.distance(p1) <= pos.distance(p2) {
                                        kf.bezier_handles[0] = nx;
                                        kf.bezier_handles[1] = ny;
                                    } else {
                                        kf.bezier_handles[2] = nx;
                                        kf.bezier_handles[3] = ny;
                                    }
                                    *action = Some(InspectorAction::ClipModified);
                                }
                            }
                        }
                    },
                );
            }
        }
    }
}

pub(super) fn sync_kf_value(clip: &mut Clip, prop: &str, clip_time: f64, val: f64) {
    if let Some(kfs) = clip.keyframes.get_mut(prop) {
        if let Some(kf) = kfs.iter_mut().find(|k| (k.time - clip_time).abs() <= 0.05) {
            kf.value = val;
        }
    }
}
