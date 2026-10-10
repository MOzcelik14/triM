//! Track header controls: Track title, Icons, Mute/Visibility, Solo, and Volume.

use egui::{Button, Color32, Response, RichText, Sense, Stroke, Ui, Vec2};
use trim_core::track::{Track, TrackType};

pub enum TrackHeaderAction {
    ToggleMute,
    ToggleSolo,
    VolumeChanged(f64),
}

pub fn render_track_header(
    ui: &mut Ui,
    track: &Track,
    track_height: f32,
) -> (Response, Option<TrackHeaderAction>) {
    let width = 120.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, track_height), Sense::hover());
    let painter = ui.painter_at(rect);

    // Header background
    let bg = if track.muted {
        Color32::from_rgb(0x18, 0x18, 0x1c)
    } else {
        Color32::from_rgb(0x1e, 0x1e, 0x24)
    };
    painter.rect_filled(rect, 2.0, bg);
    painter.rect_stroke(rect, 2.0, Stroke::new(1.0_f32, Color32::from_rgb(0x28, 0x28, 0x30)));

    let mut action = None;

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.add_space(3.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let (icon, badge_color) = match track.track_type {
                TrackType::Video => ("🎬", Color32::from_rgb(0x5b, 0x9e, 0xe8)),
                TrackType::Audio => ("🔊", Color32::from_rgb(0x42, 0xb8, 0x83)),
            };

            ui.label(RichText::new(icon).size(13.0));
            ui.label(RichText::new(&track.name).strong().color(badge_color));

            if track.track_type == TrackType::Video {
                // Video visibility toggle (Eye icon)
                let eye_icon = if track.muted { "🚫" } else { "👁" };
                let eye_btn = Button::new(RichText::new(eye_icon).size(11.0))
                    .fill(if track.muted { Color32::from_rgb(0x50, 0x25, 0x25) } else { Color32::from_rgb(0x2a, 0x2a, 0x34) });
                if ui.add(eye_btn).on_hover_text("Kanalı Gizle / Göster").clicked() {
                    action = Some(TrackHeaderAction::ToggleMute);
                }
            }
        });

        if track.track_type == TrackType::Audio {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;

                // Mute button
                let m_btn = Button::new(
                    RichText::new("M")
                        .size(10.0)
                        .color(if track.muted { Color32::WHITE } else { Color32::from_rgb(0xa0, 0xa0, 0xb0) })
                )
                .fill(if track.muted { Color32::from_rgb(0xd0, 0x40, 0x40) } else { Color32::from_rgb(0x28, 0x28, 0x30) });
                if ui.add(m_btn).on_hover_text("Sesi Kapat (Mute)").clicked() {
                    action = Some(TrackHeaderAction::ToggleMute);
                }

                // Solo button
                let s_btn = Button::new(
                    RichText::new("S")
                        .size(10.0)
                        .color(if track.solo { Color32::BLACK } else { Color32::from_rgb(0xa0, 0xa0, 0xb0) })
                )
                .fill(if track.solo { Color32::from_rgb(0xe0, 0xa0, 0x30) } else { Color32::from_rgb(0x28, 0x28, 0x30) });
                if ui.add(s_btn).on_hover_text("Tek Başına Dinle (Solo)").clicked() {
                    action = Some(TrackHeaderAction::ToggleSolo);
                }

                // Volume label
                ui.label(RichText::new(format!("{:.0}%", track.volume * 100.0)).size(10.0));
            });
        }
    });

    (response, action)
}
