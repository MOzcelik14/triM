//! Crash recovery modal dialog with project preview metadata.

use egui::{
    Align2, Area, Button, Color32, Frame, Id, Margin, Order, RichText, Sense, Stroke, Vec2, Window,
};
use std::path::PathBuf;
use trim_core::project::Project;

#[derive(Debug, Clone, Default)]
pub struct RecoveryDialog {
    pub is_open: bool,
    pub recovery_path: Option<PathBuf>,
    pub preview_summary: Option<String>,
}

pub enum RecoveryAction {
    Restore(PathBuf),
    Discard(PathBuf),
}

impl RecoveryDialog {
    pub fn prompt(&mut self, recovery_file: PathBuf) {
        self.preview_summary = std::fs::read_to_string(&recovery_file)
            .ok()
            .and_then(|s| Project::from_json(&s).ok())
            .map(|proj| {
                let (mut w, mut h, mut fps) =
                    (proj.timeline.width, proj.timeline.height, proj.timeline.fps);
                if w == 1920 && h == 1080 {
                    if let Some((vw, vh, vfps)) = proj
                        .timeline
                        .tracks
                        .iter()
                        .flat_map(|t| t.clips.iter())
                        .filter_map(|c| proj.media_pool.get(&c.media_id))
                        .find(|m| {
                            m.media_type == trim_core::media::MediaType::Video
                                && m.width > 0
                                && m.height > 0
                        })
                        .map(|m| (m.width, m.height, m.fps))
                    {
                        w = vw;
                        h = vh;
                        if vfps > 1.0 {
                            fps = vfps;
                        }
                    }
                }
                let clip_count: usize = proj.timeline.tracks.iter().map(|t| t.clips.len()).sum();
                format!(
                    "{}  •  {}×{} @ {:.0}fps  •  {} klip",
                    proj.settings.name, w, h, fps, clip_count
                )
            });
        self.recovery_path = Some(recovery_file);
        self.is_open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<RecoveryAction> {
        if !self.is_open || self.recovery_path.is_none() {
            return None;
        }

        let mut action = None;
        let path = self.recovery_path.clone().unwrap();

        let screen_rect = ctx.screen_rect();
        Area::new(Id::new("recovery_modal_dimmer"))
            .order(Order::Foreground)
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(screen_rect.size(), Sense::click_and_drag());
                ui.painter()
                    .rect_filled(rect, 0.0, Color32::from_black_alpha(180));
            });

        let modal_frame = Frame::window(&ctx.style())
            .fill(Color32::from_rgb(0x11, 0x12, 0x17))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x32, 0x35, 0x44)))
            .rounding(egui::Rounding::same(12.0))
            .inner_margin(Margin::symmetric(22.0, 20.0));

        Window::new("Çökme Kurtarma")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .order(Order::Foreground)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(430.0, 210.0))
            .frame(modal_frame)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    Frame::none()
                        .fill(Color32::from_rgba_unmultiplied(0xe0, 0x7a, 0x38, 36))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0xe0, 0x7a, 0x38)))
                        .rounding(egui::Rounding::same(6.0))
                        .inner_margin(Margin::symmetric(8.0, 4.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("⚡ OTOMATİK KURTARMA")
                                    .size(10.5)
                                    .strong()
                                    .color(Color32::from_rgb(0xf5, 0x9e, 0x42)),
                            );
                        });
                });

                ui.add_space(10.0);
                ui.label(
                    RichText::new("Önceki oturumdan kaydedilmemiş bir proje bulundu")
                        .size(15.0)
                        .strong()
                        .color(Color32::WHITE),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(
                        "Çalışmanızı kaldığınız yerden (orijinal sekans çözünürlüğü ve zaman çizgisiyle) geri yüklemek ister misiniz?",
                    )
                    .size(11.5)
                    .color(Color32::from_rgb(0x9c, 0xa3, 0xb8)),
                );

                if let Some(summary) = &self.preview_summary {
                    ui.add_space(10.0);
                    Frame::none()
                        .fill(Color32::from_rgb(0x17, 0x19, 0x22))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x27, 0x2a, 0x38)))
                        .rounding(egui::Rounding::same(7.0))
                        .inner_margin(Margin::symmetric(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.label(
                                RichText::new(format!("🎞  {}", summary))
                                    .size(11.5)
                                    .strong()
                                    .color(Color32::from_rgb(0xe2, 0xe8, 0xf0)),
                            );
                        });
                }

                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    let btn_restore = Button::new(
                        RichText::new("✓  Projeyi Geri Yükle")
                            .size(12.0)
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(0xe0, 0x7a, 0x38))
                    .rounding(egui::Rounding::same(6.0))
                    .min_size(Vec2::new(175.0, 32.0));

                    if ui.add(btn_restore).clicked() {
                        action = Some(RecoveryAction::Restore(path.clone()));
                        self.is_open = false;
                    }

                    ui.add_space(6.0);
                    let btn_discard = Button::new(
                        RichText::new("Yok Say ve Sil")
                            .size(11.5)
                            .color(Color32::from_rgb(0xb0, 0xb6, 0xc8)),
                    )
                    .fill(Color32::from_rgb(0x1c, 0x1f, 0x29))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x2e, 0x32, 0x42)))
                    .rounding(egui::Rounding::same(6.0))
                    .min_size(Vec2::new(130.0, 32.0));

                    if ui.add(btn_discard).clicked() {
                        action = Some(RecoveryAction::Discard(path.clone()));
                        self.is_open = false;
                    }
                });
            });

        action
    }
}
