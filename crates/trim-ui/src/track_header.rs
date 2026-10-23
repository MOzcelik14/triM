//! Track header controls: Track title, Icons, Mute/Visibility, Solo, and Volume.

use egui::{Button, Color32, Response, RichText, Sense, Stroke, Ui, Vec2};
use trim_core::track::{Track, TrackType};

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
    let width = 142.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, track_height), Sense::click());
    let painter = ui.painter_at(rect);

    // 1. Sleek Card Background & Border
    let bg = if track.locked {
        Color32::from_rgb(0x24, 0x18, 0x18)
    } else if track.muted {
        Color32::from_rgb(0x18, 0x18, 0x1e)
    } else if response.hovered() {
        Color32::from_rgb(0x22, 0x22, 0x2c)
    } else {
        Color32::from_rgb(0x1a, 0x1a, 0x22)
    };
    painter.rect_filled(rect, 3.0, bg);
    painter.rect_stroke(rect, 3.0, Stroke::new(1.0_f32, Color32::from_rgb(0x2c, 0x2c, 0x38)));

    // 2. Left Color Accent Bar (Video: Cyan, Audio: Emerald)
    let accent_color = match track.track_type {
        TrackType::Video => Color32::from_rgb(0x38, 0xa0, 0xf0),
        TrackType::Audio => Color32::from_rgb(0x3d, 0xdc, 0x84),
    };
    let accent_rect = egui::Rect::from_min_size(rect.left_top(), Vec2::new(3.5, track_height));
    painter.rect_filled(accent_rect, egui::Rounding { nw: 3.0, sw: 3.0, ne: 0.0, se: 0.0 }, accent_color);

    let mut action = None;

    // 3. Vertically Centered Controls Row
    let content_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 6.0, rect.top()),
        Vec2::new(width - 8.0, track_height),
    );
    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));
    let ui = &mut child_ui;

    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;

        // Unified Track Pill Badge (e.g. V1, A1) - click to toggle patching target
        let (badge_bg, badge_text_color, badge_stroke) = if is_patch_target {
            (
                Color32::from_rgb(0x28, 0x5a, 0x9e),
                Color32::WHITE,
                Stroke::new(1.2_f32, Color32::from_rgb(0x4a, 0x8a, 0xe8)),
            )
        } else {
            (
                Color32::from_rgb(0x24, 0x24, 0x30),
                accent_color,
                Stroke::new(1.0_f32, Color32::from_rgb(0x36, 0x36, 0x46)),
            )
        };

        let badge_btn = Button::new(
            RichText::new(&track.name)
                .size(11.0)
                .color(badge_text_color)
                .strong(),
        )
        .min_size(Vec2::new(26.0, 22.0))
        .fill(badge_bg)
        .stroke(badge_stroke);

        let patch_tip = if is_patch_target {
            format!("{} (Kaynak Hedefleme Aktif)", track.name)
        } else {
            format!("{} (Kaynak Hedef Seçmek İçin Tıklayın)", track.name)
        };

        if ui.add(badge_btn).on_hover_text(patch_tip).clicked() {
            action = Some(TrackHeaderAction::TogglePatchTarget);
        }

        match track.track_type {
            TrackType::Video => {
                // Video Visibility Toggle (Göz / V)
                let is_vis = !track.muted;
                let (vis_bg, vis_text, vis_stroke) = if is_vis {
                    (
                        Color32::from_rgb(0x1c, 0x35, 0x52),
                        Color32::from_rgb(0x5b, 0x9e, 0xe8),
                        Stroke::new(1.0_f32, Color32::from_rgb(0x2b, 0x58, 0x8c)),
                    )
                } else {
                    (
                        Color32::from_rgb(0x32, 0x1c, 0x1c),
                        Color32::from_rgb(0xb0, 0x50, 0x50),
                        Stroke::new(1.0_f32, Color32::from_rgb(0x58, 0x28, 0x28)),
                    )
                };

                let eye_btn = Button::new(
                    RichText::new(if is_vis { "👁" } else { "X" })
                        .size(11.0)
                        .color(vis_text)
                        .strong(),
                )
                .min_size(Vec2::new(22.0, 22.0))
                .fill(vis_bg)
                .stroke(vis_stroke);

                if ui.add(eye_btn).on_hover_text(if is_vis { "Görüntüyü Gizle (Mute)" } else { "Görüntüyü Göster" }).clicked() {
                    action = Some(TrackHeaderAction::ToggleMute);
                }

                // Lock Toggle (K)
                let (lock_bg, lock_text, lock_stroke) = if track.locked {
                    (
                        Color32::from_rgb(0x90, 0x24, 0x24),
                        Color32::WHITE,
                        Stroke::new(1.0_f32, Color32::from_rgb(0xd0, 0x40, 0x40)),
                    )
                } else {
                    (
                        Color32::from_rgb(0x22, 0x22, 0x2a),
                        Color32::from_rgb(0x75, 0x75, 0x85),
                        Stroke::new(1.0_f32, Color32::from_rgb(0x30, 0x30, 0x3c)),
                    )
                };

                let lock_btn = Button::new(
                    RichText::new("K")
                        .size(10.5)
                        .color(lock_text)
                        .strong(),
                )
                .min_size(Vec2::new(22.0, 22.0))
                .fill(lock_bg)
                .stroke(lock_stroke);

                if ui.add(lock_btn).on_hover_text(if track.locked { "Kanal Kilidini Kaldır" } else { "Kanalı Kilitle (Lock)" }).clicked() {
                    action = Some(TrackHeaderAction::ToggleLock);
                }
            }
            TrackType::Audio => {
                // Audio Mute Toggle (M)
                let (m_bg, m_text, m_stroke) = if track.muted {
                    (
                        Color32::from_rgb(0xaa, 0x28, 0x28),
                        Color32::WHITE,
                        Stroke::new(1.0_f32, Color32::from_rgb(0xe0, 0x48, 0x48)),
                    )
                } else {
                    (
                        Color32::from_rgb(0x22, 0x22, 0x2a),
                        Color32::from_rgb(0x88, 0x88, 0x98),
                        Stroke::new(1.0_f32, Color32::from_rgb(0x30, 0x30, 0x3c)),
                    )
                };

                let m_btn = Button::new(
                    RichText::new("M")
                        .size(10.0)
                        .color(m_text)
                        .strong(),
                )
                .min_size(Vec2::new(20.0, 22.0))
                .fill(m_bg)
                .stroke(m_stroke);

                if ui.add(m_btn).on_hover_text(if track.muted { "Sesi Aç" } else { "Sesi Kapat (Mute)" }).clicked() {
                    action = Some(TrackHeaderAction::ToggleMute);
                }

                // Audio Solo Toggle (S)
                let (s_bg, s_text, s_stroke) = if track.solo {
                    (
                        Color32::from_rgb(0xd4, 0x94, 0x18),
                        Color32::BLACK,
                        Stroke::new(1.0_f32, Color32::from_rgb(0xf4, 0xb4, 0x30)),
                    )
                } else {
                    (
                        Color32::from_rgb(0x22, 0x22, 0x2a),
                        Color32::from_rgb(0x88, 0x88, 0x98),
                        Stroke::new(1.0_f32, Color32::from_rgb(0x30, 0x30, 0x3c)),
                    )
                };

                let s_btn = Button::new(
                    RichText::new("S")
                        .size(10.0)
                        .color(s_text)
                        .strong(),
                )
                .min_size(Vec2::new(20.0, 22.0))
                .fill(s_bg)
                .stroke(s_stroke);

                if ui.add(s_btn).on_hover_text(if track.solo { "Solo Kapat" } else { "Tek Başına Dinle (Solo)" }).clicked() {
                    action = Some(TrackHeaderAction::ToggleSolo);
                }

                // Audio Lock Toggle (K)
                let (lock_bg, lock_text, lock_stroke) = if track.locked {
                    (
                        Color32::from_rgb(0x90, 0x24, 0x24),
                        Color32::WHITE,
                        Stroke::new(1.0_f32, Color32::from_rgb(0xd0, 0x40, 0x40)),
                    )
                } else {
                    (
                        Color32::from_rgb(0x22, 0x22, 0x2a),
                        Color32::from_rgb(0x75, 0x75, 0x85),
                        Stroke::new(1.0_f32, Color32::from_rgb(0x30, 0x30, 0x3c)),
                    )
                };

                let lock_btn = Button::new(
                    RichText::new("K")
                        .size(10.0)
                        .color(lock_text)
                        .strong(),
                )
                .min_size(Vec2::new(20.0, 22.0))
                .fill(lock_bg)
                .stroke(lock_stroke);

                if ui.add(lock_btn).on_hover_text(if track.locked { "Kanal Kilidini Kaldır" } else { "Kanalı Kilitle (Lock)" }).clicked() {
                    action = Some(TrackHeaderAction::ToggleLock);
                }

                // Audio Voiceover Record Button (🎤 / ⏹)
                let (vo_bg, vo_text, vo_stroke) = if is_recording_vo {
                    (
                        Color32::from_rgb(0xd0, 0x24, 0x24),
                        Color32::WHITE,
                        Stroke::new(1.2_f32, Color32::from_rgb(0xff, 0x48, 0x48)),
                    )
                } else {
                    (
                        Color32::from_rgb(0x22, 0x22, 0x2a),
                        Color32::from_rgb(0xaa, 0xaa, 0xba),
                        Stroke::new(1.0_f32, Color32::from_rgb(0x30, 0x30, 0x3c)),
                    )
                };

                let vo_icon = if is_recording_vo { "⏹" } else { "🎤" };
                let vo_btn = Button::new(
                    RichText::new(vo_icon)
                        .size(10.0)
                        .color(vo_text),
                )
                .min_size(Vec2::new(20.0, 22.0))
                .fill(vo_bg)
                .stroke(vo_stroke);

                let vo_tip = if is_recording_vo {
                    "Ses Kaydını Durdur (Voiceover)"
                } else {
                    "Bu Kanala Canlı Seslendirme / Dublaj Kaydet (Voiceover)"
                };

                if ui.add(vo_btn).on_hover_text(vo_tip).clicked() {
                    action = Some(TrackHeaderAction::RecordVoiceover);
                }

                // Compact Volume Percentage
                ui.label(
                    RichText::new(format!("{:.0}%", track.volume * 100.0))
                        .size(9.0)
                        .color(Color32::from_rgb(0x80, 0x80, 0x90)),
                );
            }
        }
    });

    response.context_menu(|ui| {
        ui.label(RichText::new(&track.name).strong());
        ui.separator();
        if ui.button("Kanalı Sil").clicked() {
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

