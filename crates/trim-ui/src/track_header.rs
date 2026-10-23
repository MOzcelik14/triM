//! Track header controls: Track title, Icons, Mute/Visibility, Solo, and Volume.

use egui::{Button, Color32, Response, RichText, Sense, Stroke, Ui, Vec2};
use trim_core::track::{Track, TrackType};

/// Standard comfortable width for track headers in the timeline.
pub const TRACK_HEADER_WIDTH: f32 = 216.0;

pub enum TrackHeaderAction {
    ToggleMute,
    ToggleSolo,
    ToggleLock,
    TogglePatchTarget,
    RecordVoiceover,
    VolumeChanged(f64),
    DeleteTrack,
    ResizeTrack(f32),
}

pub fn render_track_header(
    ui: &mut Ui,
    track: &Track,
    track_height: f32,
    is_patch_target: bool,
    is_recording_vo: bool,
) -> (Response, Option<TrackHeaderAction>) {
    let width = TRACK_HEADER_WIDTH;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, track_height), Sense::click());
    let painter = ui.painter_at(rect);

    // 1. Sleek Card Background & Border
    let bg = if track.locked {
        Color32::from_rgb(0x23, 0x18, 0x1a)
    } else if track.muted {
        Color32::from_rgb(0x17, 0x17, 0x1e)
    } else if response.hovered() {
        Color32::from_rgb(0x21, 0x23, 0x2f)
    } else {
        Color32::from_rgb(0x1a, 0x1b, 0x24)
    };
    painter.rect_filled(rect, 2.0, bg);
    painter.rect_stroke(rect, 2.0, Stroke::new(1.0_f32, Color32::from_rgb(0x2d, 0x2f, 0x3d)));

    // Right Divider separating track headers from the timeline canvas
    painter.line_segment(
        [rect.right_top(), rect.right_bottom()],
        Stroke::new(1.0_f32, Color32::from_rgb(0x2d, 0x2f, 0x3d)),
    );

    // 2. Left Color Accent Bar (Video: Cyan/Blue, Audio: Emerald)
    let accent_color = match track.track_type {
        TrackType::Video => Color32::from_rgb(0x38, 0xa0, 0xf0),
        TrackType::Audio => Color32::from_rgb(0x10, 0xb9, 0x81),
    };
    let accent_rect = egui::Rect::from_min_size(rect.left_top(), Vec2::new(4.0, track_height));
    painter.rect_filled(accent_rect, egui::Rounding { nw: 2.0, sw: 2.0, ne: 0.0, se: 0.0 }, accent_color);

    let mut action = None;

    // 3. Vertically Centered Controls Row with comfortable breathing room
    let content_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 8.0, rect.top()),
        Vec2::new(width - 12.0, track_height),
    );
    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));
    let ui = &mut child_ui;

    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;

        // Unified Track Pill Badge (e.g. V1, A1) - click to toggle patching target
        let (badge_bg, badge_text_color, badge_stroke) = if is_patch_target {
            match track.track_type {
                TrackType::Video => (
                    Color32::from_rgb(0x1d, 0x4e, 0xd8),
                    Color32::WHITE,
                    Stroke::new(1.2_f32, Color32::from_rgb(0x60, 0xa5, 0xfa)),
                ),
                TrackType::Audio => (
                    Color32::from_rgb(0x04, 0x78, 0x57),
                    Color32::WHITE,
                    Stroke::new(1.2_f32, Color32::from_rgb(0x34, 0xd3, 0x99)),
                ),
            }
        } else {
            (
                Color32::from_rgb(0x22, 0x24, 0x30),
                accent_color,
                Stroke::new(1.0_f32, Color32::from_rgb(0x34, 0x37, 0x48)),
            )
        };

        let badge_btn = Button::new(
            RichText::new(&track.name)
                .size(11.0)
                .color(badge_text_color)
                .strong(),
        )
        .min_size(Vec2::new(32.0, 24.0))
        .fill(badge_bg)
        .stroke(badge_stroke)
        .rounding(egui::Rounding::same(4.0));

        let patch_tip = if is_patch_target {
            format!("{} (Kaynak Hedefleme Aktif - Klipler buraya eklenir)", track.name)
        } else {
            format!("{} (Kaynak Hedef Seçmek İçin Tıklayın)", track.name)
        };

        if ui.add(badge_btn).on_hover_text(patch_tip).clicked() {
            action = Some(TrackHeaderAction::TogglePatchTarget);
        }

        match track.track_type {
            TrackType::Video => {
                // Video Visibility Toggle (Göz)
                let is_vis = !track.muted;
                let (vis_bg, vis_text, vis_stroke, vis_icon) = if is_vis {
                    (
                        Color32::from_rgb(0x1e, 0x29, 0x3b),
                        Color32::from_rgb(0x60, 0xa5, 0xfa),
                        Stroke::new(1.0_f32, Color32::from_rgb(0x33, 0x41, 0x55)),
                        "👁",
                    )
                } else {
                    (
                        Color32::from_rgb(0x3f, 0x1d, 0x1d),
                        Color32::from_rgb(0xef, 0x44, 0x44),
                        Stroke::new(1.0_f32, Color32::from_rgb(0x7f, 0x1d, 0x1d)),
                        "🚫",
                    )
                };

                let eye_btn = Button::new(
                    RichText::new(vis_icon)
                        .size(11.5)
                        .color(vis_text)
                        .strong(),
                )
                .min_size(Vec2::new(26.0, 24.0))
                .fill(vis_bg)
                .stroke(vis_stroke)
                .rounding(egui::Rounding::same(4.0));

                if ui.add(eye_btn).on_hover_text(if is_vis { "Görüntüyü Gizle (Mute)" } else { "Görüntüyü Göster" }).clicked() {
                    action = Some(TrackHeaderAction::ToggleMute);
                }

                // Video Lock Toggle (🔒)
                let (lock_bg, lock_text, lock_stroke) = if track.locked {
                    (
                        Color32::from_rgb(0xb9, 0x1c, 0x1c),
                        Color32::WHITE,
                        Stroke::new(1.0_f32, Color32::from_rgb(0xef, 0x44, 0x44)),
                    )
                } else {
                    (
                        Color32::from_rgb(0x22, 0x24, 0x30),
                        Color32::from_rgb(0x64, 0x74, 0x8b),
                        Stroke::new(1.0_f32, Color32::from_rgb(0x33, 0x37, 0x48)),
                    )
                };

                let lock_btn = Button::new(
                    RichText::new("🔒")
                        .size(11.0)
                        .color(lock_text),
                )
                .min_size(Vec2::new(26.0, 24.0))
                .fill(lock_bg)
                .stroke(lock_stroke)
                .rounding(egui::Rounding::same(4.0));

                if ui.add(lock_btn).on_hover_text(if track.locked { "Kanal Kilidini Kaldır" } else { "Kanalı Kilitle (Lock)" }).clicked() {
                    action = Some(TrackHeaderAction::ToggleLock);
                }

                // Subtle balanced track type label
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Video")
                        .size(10.5)
                        .color(Color32::from_rgb(0x64, 0x74, 0x8b)),
                );
            }
            TrackType::Audio => {
                // Audio Mute Toggle (M)
                let (m_bg, m_text, m_stroke) = if track.muted {
                    (
                        Color32::from_rgb(0xdc, 0x26, 0x26),
                        Color32::WHITE,
                        Stroke::new(1.0_f32, Color32::from_rgb(0xef, 0x44, 0x44)),
                    )
                } else {
                    (
                        Color32::from_rgb(0x22, 0x24, 0x30),
                        Color32::from_rgb(0x94, 0xa3, 0xb8),
                        Stroke::new(1.0_f32, Color32::from_rgb(0x33, 0x37, 0x48)),
                    )
                };

                let m_btn = Button::new(
                    RichText::new("M")
                        .size(10.5)
                        .color(m_text)
                        .strong(),
                )
                .min_size(Vec2::new(24.0, 24.0))
                .fill(m_bg)
                .stroke(m_stroke)
                .rounding(egui::Rounding::same(4.0));

                if ui.add(m_btn).on_hover_text(if track.muted { "Sesi Aç" } else { "Sesi Kapat (Mute)" }).clicked() {
                    action = Some(TrackHeaderAction::ToggleMute);
                }

                // Audio Solo Toggle (S)
                let (s_bg, s_text, s_stroke) = if track.solo {
                    (
                        Color32::from_rgb(0xf5, 0x9e, 0x0b),
                        Color32::from_rgb(0x1a, 0x14, 0x05),
                        Stroke::new(1.0_f32, Color32::from_rgb(0xfb, 0xbf, 0x24)),
                    )
                } else {
                    (
                        Color32::from_rgb(0x22, 0x24, 0x30),
                        Color32::from_rgb(0x94, 0xa3, 0xb8),
                        Stroke::new(1.0_f32, Color32::from_rgb(0x33, 0x37, 0x48)),
                    )
                };

                let s_btn = Button::new(
                    RichText::new("S")
                        .size(10.5)
                        .color(s_text)
                        .strong(),
                )
                .min_size(Vec2::new(24.0, 24.0))
                .fill(s_bg)
                .stroke(s_stroke)
                .rounding(egui::Rounding::same(4.0));

                if ui.add(s_btn).on_hover_text(if track.solo { "Solo Kapat" } else { "Tek Başına Dinle (Solo)" }).clicked() {
                    action = Some(TrackHeaderAction::ToggleSolo);
                }

                // Audio Lock Toggle (🔒)
                let (lock_bg, lock_text, lock_stroke) = if track.locked {
                    (
                        Color32::from_rgb(0xb9, 0x1c, 0x1c),
                        Color32::WHITE,
                        Stroke::new(1.0_f32, Color32::from_rgb(0xef, 0x44, 0x44)),
                    )
                } else {
                    (
                        Color32::from_rgb(0x22, 0x24, 0x30),
                        Color32::from_rgb(0x64, 0x74, 0x8b),
                        Stroke::new(1.0_f32, Color32::from_rgb(0x33, 0x37, 0x48)),
                    )
                };

                let lock_btn = Button::new(
                    RichText::new("🔒")
                        .size(10.5)
                        .color(lock_text),
                )
                .min_size(Vec2::new(24.0, 24.0))
                .fill(lock_bg)
                .stroke(lock_stroke)
                .rounding(egui::Rounding::same(4.0));

                if ui.add(lock_btn).on_hover_text(if track.locked { "Kanal Kilidini Kaldır" } else { "Kanalı Kilitle (Lock)" }).clicked() {
                    action = Some(TrackHeaderAction::ToggleLock);
                }

                // Audio Voiceover Record Button (🎤 / ⏹)
                let (vo_bg, vo_text, vo_stroke) = if is_recording_vo {
                    (
                        Color32::from_rgb(0xdc, 0x26, 0x26),
                        Color32::WHITE,
                        Stroke::new(1.2_f32, Color32::from_rgb(0xf8, 0x71, 0x71)),
                    )
                } else {
                    (
                        Color32::from_rgb(0x22, 0x24, 0x30),
                        Color32::from_rgb(0xa7, 0x8b, 0xfa),
                        Stroke::new(1.0_f32, Color32::from_rgb(0x33, 0x37, 0x48)),
                    )
                };

                let vo_icon = if is_recording_vo { "⏹" } else { "🎤" };
                let vo_btn = Button::new(
                    RichText::new(vo_icon)
                        .size(10.5)
                        .color(vo_text),
                )
                .min_size(Vec2::new(24.0, 24.0))
                .fill(vo_bg)
                .stroke(vo_stroke)
                .rounding(egui::Rounding::same(4.0));

                let vo_tip = if is_recording_vo {
                    "Ses Kaydını Durdur (Voiceover)"
                } else {
                    "Bu Kanala Canlı Seslendirme / Dublaj Kaydet (Voiceover)"
                };

                if ui.add(vo_btn).on_hover_text(vo_tip).clicked() {
                    action = Some(TrackHeaderAction::RecordVoiceover);
                }

                // Interactive, Sleek Volume DragValue (% readout)
                ui.add_space(2.0);
                let mut vol_pct = (track.volume * 100.0).round();
                let vol_resp = ui.add_sized(
                    Vec2::new(48.0, 22.0),
                    egui::DragValue::new(&mut vol_pct)
                        .speed(1.0)
                        .range(0.0..=200.0)
                        .suffix("%"),
                );
                if vol_resp.changed() {
                    action = Some(TrackHeaderAction::VolumeChanged((vol_pct / 100.0).max(0.0)));
                }
                if vol_resp.double_clicked() {
                    action = Some(TrackHeaderAction::VolumeChanged(1.0));
                }
                vol_resp.on_hover_text("Kanal Ses Seviyesi (%)\nSürükleyerek ayarlayın, çift tık: %100");
            }
        }
    });

    response.context_menu(|ui| {
        ui.label(RichText::new(&track.name).strong());
        ui.separator();
        let mute_label = if track.muted { "Sesi/Görüntüyü Aç" } else { "Sesi/Görüntüyü Kapat" };
        if ui.button(mute_label).clicked() {
            action = Some(TrackHeaderAction::ToggleMute);
            ui.close_menu();
        }
        let lock_label = if track.locked { "Kilidi Kaldır" } else { "Kanalı Kilitle" };
        if ui.button(lock_label).clicked() {
            action = Some(TrackHeaderAction::ToggleLock);
            ui.close_menu();
        }
        if track.track_type == TrackType::Audio {
            let solo_label = if track.solo { "Solo Kapat" } else { "Solo Aç" };
            if ui.button(solo_label).clicked() {
                action = Some(TrackHeaderAction::ToggleSolo);
                ui.close_menu();
            }
        }
        ui.separator();
        if ui.button("↕ Yüksekliği Sıfırla (56px)").clicked() {
            action = Some(TrackHeaderAction::ResizeTrack(56.0));
            ui.close_menu();
        }
        ui.separator();
        if ui.button(RichText::new("Kanalı Sil").color(Color32::from_rgb(0xef, 0x44, 0x44))).clicked() {
            action = Some(TrackHeaderAction::DeleteTrack);
            ui.close_menu();
        }
    });

    // 4. Bottom Draggable Resize Handle
    let resize_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left(), rect.bottom() - 3.0),
        egui::pos2(rect.right(), rect.bottom() + 3.0),
    );
    let resize_resp = ui.interact(
        resize_rect,
        ui.id().with(("resize_track", &track.id)),
        Sense::click_and_drag(),
    );
    if resize_resp.hovered() || resize_resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        painter.line_segment(
            [resize_rect.left_center(), resize_rect.right_center()],
            Stroke::new(2.0_f32, Color32::from_rgb(0x38, 0xa0, 0xf0)),
        );
    }
    if resize_resp.double_clicked() {
        action = Some(TrackHeaderAction::ResizeTrack(56.0));
    } else if resize_resp.dragged() {
        let dy = resize_resp.drag_delta().y;
        let new_h = (track_height + dy).clamp(32.0, 160.0);
        action = Some(TrackHeaderAction::ResizeTrack(new_h));
    }

    (response, action)
}
