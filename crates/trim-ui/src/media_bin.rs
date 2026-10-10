//! Media Bin asset pool panel with import and drag/drop support.

use egui::{Color32, RichText, ScrollArea, Sense, Stroke, Ui, Vec2};
use trim_core::media::MediaType;
use trim_core::project::Project;

pub enum MediaBinAction {
    ImportRequested,
    AddToTimeline(String), // media_id
}

pub fn render_media_bin(
    ui: &mut Ui,
    project: &Project,
    selected_media_id: &mut Option<String>,
) -> Option<MediaBinAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        ui.heading(RichText::new("Medya Havuzu").size(14.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new("+ İçe Aktar").color(Color32::WHITE)).clicked() {
                action = Some(MediaBinAction::ImportRequested);
            }
        });
    });

    ui.add_space(4.0);
    ui.separator();

    if project.media_pool.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                RichText::new("Henüz medya eklenmedi.\nDosyaları buraya sürükleyin\nveya '+ İçe Aktar' butonuna basın.")
                    .color(Color32::from_rgb(0x80, 0x80, 0x90))
                    .size(11.0),
            );
        });
        return action;
    }

    ScrollArea::vertical().show(ui, |ui| {
        for (m_id, item) in &project.media_pool {
            let is_selected = selected_media_id.as_deref() == Some(m_id);
            let item_height = 42.0;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), item_height), Sense::click_and_drag());
            let painter = ui.painter_at(rect);

            let bg = if is_selected {
                Color32::from_rgb(0x35, 0x28, 0x1d)
            } else if resp.hovered() {
                Color32::from_rgb(0x28, 0x28, 0x30)
            } else {
                Color32::from_rgb(0x1c, 0x1c, 0x20)
            };

            painter.rect_filled(rect, 3.0, bg);
            if is_selected {
                painter.rect_stroke(rect, 3.0, Stroke::new(1.0_f32, Color32::from_rgb(0xe0, 0x7a, 0x38)));
            }

            // Icon by media type
            let icon = match item.media_type {
                MediaType::Video => "🎬",
                MediaType::Audio => "🎵",
                MediaType::Image => "🖼",
                MediaType::Unknown => "📄",
            };

            painter.text(
                egui::pos2(rect.left() + 8.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                icon,
                egui::FontId::proportional(16.0),
                Color32::WHITE,
            );

            // Name & duration
            let dur_str = format!("{:.1}s", item.duration);
            let res_str = if item.width > 0 {
                format!("{}x{}", item.width, item.height)
            } else {
                String::new()
            };

            painter.text(
                egui::pos2(rect.left() + 32.0, rect.top() + 8.0),
                egui::Align2::LEFT_TOP,
                &item.name,
                egui::FontId::proportional(12.0),
                Color32::from_rgb(0xe0, 0xe0, 0xea),
            );

            painter.text(
                egui::pos2(rect.left() + 32.0, rect.bottom() - 6.0),
                egui::Align2::LEFT_BOTTOM,
                format!("{}  {}", dur_str, res_str),
                egui::FontId::monospace(10.0),
                Color32::from_rgb(0x80, 0x80, 0x90),
            );

            if resp.clicked() {
                *selected_media_id = Some(m_id.clone());
            }

            if resp.dragged() {
                resp.dnd_set_drag_payload(m_id.clone());
            }

            if resp.double_clicked() {
                action = Some(MediaBinAction::AddToTimeline(m_id.clone()));
            }

            resp.context_menu(|ui| {
                if ui.button("Zaman Çizgisine Ekle").clicked() {
                    action = Some(MediaBinAction::AddToTimeline(m_id.clone()));
                    ui.close_menu();
                }
            });
        }
    });

    action
}
