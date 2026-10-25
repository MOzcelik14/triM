//! Multi-job Export / Render Queue dialog modal.

use egui::{Button, Color32, Frame, Margin, ProgressBar, RichText, Rounding, ScrollArea, Stroke, Vec2, Window};
use trim_export::queue::{RenderJobStatus, RenderQueue};

const CARD_BG: Color32 = Color32::from_rgb(0x17, 0x19, 0x24);
const CARD_BORDER: Color32 = Color32::from_rgb(0x27, 0x2b, 0x3a);
const TEXT_MUTED: Color32 = Color32::from_rgb(0x8a, 0x92, 0xa6);
const ACCENT_AMBER: Color32 = Color32::from_rgb(0xe0, 0x7a, 0x38);
const ACCENT_BLUE: Color32 = Color32::from_rgb(0x38, 0x7a, 0xe0);
const ACCENT_GREEN: Color32 = Color32::from_rgb(0x2e, 0xbd, 0x85);
const ACCENT_RED: Color32 = Color32::from_rgb(0xef, 0x44, 0x44);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderQueueDialogAction {
    OpenOutputFolder(String),
    NewExportJob,
    Close,
}

#[derive(Debug, Clone, Default)]
pub struct RenderQueueDialog {
    pub is_open: bool,
}

impl RenderQueueDialog {
    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        queue: &mut RenderQueue,
    ) -> Option<RenderQueueDialogAction> {
        if !self.is_open {
            return None;
        }

        let mut action = None;
        let mut open = self.is_open;

        Window::new("📋 Dışa Aktarma Kuyruğu (Render Queue)")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(720.0)
            .default_height(480.0)
            .min_width(560.0)
            .min_height(320.0)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);

                // Top Toolbar & Status
                let (queued_cnt, in_prog_cnt, comp_cnt, failed_cnt) = queue.summary_counts();

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "Toplam {} İş:  ⚪ {} Sırada  🔵 {} İşleniyor  🟢 {} Bitti  🔴 {} Hata",
                            queue.jobs.len(),
                            queued_cnt,
                            in_prog_cnt,
                            comp_cnt,
                            failed_cnt
                        ))
                        .size(12.0)
                        .color(Color32::from_rgb(0xda, 0xe0, 0xf0)),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if queue.is_active {
                            let btn_pause = Button::new(
                                RichText::new("⏸ Kuyruğu Duraklat")
                                    .size(11.5)
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(0x35, 0x3b, 0x4e))
                            .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                            .rounding(Rounding::same(6.0));

                            if ui.add(btn_pause).clicked() {
                                queue.pause();
                            }
                        } else {
                            let btn_start = Button::new(
                                RichText::new("▶ Kuyruğu Başlat")
                                    .size(11.5)
                                    .color(Color32::WHITE)
                                    .strong(),
                            )
                            .fill(ACCENT_GREEN)
                            .rounding(Rounding::same(6.0));

                            if ui.add(btn_start).clicked() {
                                queue.start();
                            }
                        }

                        if comp_cnt > 0 || failed_cnt > 0 {
                            let btn_clear = Button::new(
                                RichText::new("🧹 Bitenleri Temizle")
                                    .size(11.5)
                                    .color(Color32::from_rgb(0xc0, 0xc6, 0xd8)),
                            )
                            .fill(Color32::from_rgb(0x22, 0x26, 0x34))
                            .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                            .rounding(Rounding::same(6.0));

                            if ui.add(btn_clear).clicked() {
                                queue.clear_finished();
                            }
                        }
                    });
                });

                ui.separator();

                if queue.jobs.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(60.0);
                        ui.label(
                            RichText::new("📥 Dışa Aktarma Kuyruğu Boş")
                                .size(16.0)
                                .color(TEXT_MUTED)
                                .strong(),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(
                                "Dışa Aktar penceresinden 'Kuyruğa Ekle' butonuna basarak\nbirden çok video çıktısını arka arkaya işlenmek üzere buraya gönderebilirsiniz.",
                            )
                            .size(12.0)
                            .color(Color32::from_rgb(0x70, 0x76, 0x8c)),
                        );
                        ui.add_space(20.0);
                        if ui.button("➕ Yeni Dışa Aktarma Başlat").clicked() {
                            action = Some(RenderQueueDialogAction::NewExportJob);
                        }
                        ui.add_space(40.0);
                    });
                    return;
                }

                // Jobs list
                let mut job_to_cancel = None;
                let mut job_to_remove = None;
                let mut folder_to_open = None;

                ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);

                    for job in &queue.jobs {
                        Frame::none()
                            .fill(CARD_BG)
                            .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(Margin::symmetric(14.0, 10.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // Status Badge
                                    match &job.status {
                                        RenderJobStatus::Queued => {
                                            ui.label(
                                                RichText::new("⚪ Sırada")
                                                    .size(11.0)
                                                    .color(Color32::from_rgb(0xb0, 0xb8, 0xcc))
                                                    .strong(),
                                            );
                                        }
                                        RenderJobStatus::InProgress { progress, .. } => {
                                            ui.label(
                                                RichText::new(format!("🔵 %{:.0} İşleniyor", progress))
                                                    .size(11.0)
                                                    .color(ACCENT_BLUE)
                                                    .strong(),
                                            );
                                        }
                                        RenderJobStatus::Completed { .. } => {
                                            ui.label(
                                                RichText::new("🟢 Tamamlandı")
                                                    .size(11.0)
                                                    .color(ACCENT_GREEN)
                                                    .strong(),
                                            );
                                        }
                                        RenderJobStatus::Failed { .. } => {
                                            ui.label(
                                                RichText::new("🔴 Başarısız")
                                                    .size(11.0)
                                                    .color(ACCENT_RED)
                                                    .strong(),
                                            );
                                        }
                                        RenderJobStatus::Cancelled => {
                                            ui.label(
                                                RichText::new("⚪ İptal Edildi")
                                                    .size(11.0)
                                                    .color(TEXT_MUTED),
                                            );
                                        }
                                    }

                                    ui.add_space(8.0);
                                    ui.label(
                                        RichText::new(&job.name)
                                            .size(12.5)
                                            .color(Color32::WHITE)
                                            .strong(),
                                    );

                                    ui.label(
                                        RichText::new(format!("|  {}", job.preset.name))
                                            .size(11.0)
                                            .color(ACCENT_AMBER),
                                    );

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        match &job.status {
                                            RenderJobStatus::InProgress { .. } => {
                                                if ui.small_button("❌ İptal").clicked() {
                                                    job_to_cancel = Some(job.id.clone());
                                                }
                                            }
                                            RenderJobStatus::Completed { output_path } => {
                                                if ui.small_button("📁 Klasörde Göster").clicked() {
                                                    folder_to_open = Some(output_path.clone());
                                                }
                                                if ui.small_button("🗑").clicked() {
                                                    job_to_remove = Some(job.id.clone());
                                                }
                                            }
                                            RenderJobStatus::Queued => {
                                                if ui.small_button("❌ İptal").clicked() {
                                                    job_to_cancel = Some(job.id.clone());
                                                }
                                                if ui.small_button("🗑").clicked() {
                                                    job_to_remove = Some(job.id.clone());
                                                }
                                            }
                                            _ => {
                                                if ui.small_button("🗑 Sil").clicked() {
                                                    job_to_remove = Some(job.id.clone());
                                                }
                                            }
                                        }
                                    });
                                });

                                // Output path & range
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&job.output_path)
                                            .size(10.5)
                                            .color(Color32::from_rgb(0xa0, 0xa8, 0xbe)),
                                    );
                                    if let Some((st, en)) = job.range {
                                        ui.label(
                                            RichText::new(format!("(Aralık: {:.2}s - {:.2}s)", st, en))
                                                .size(10.0)
                                                .color(TEXT_MUTED),
                                        );
                                    }
                                });

                                // Progress bar or error message
                                match &job.status {
                                    RenderJobStatus::InProgress { progress, status_text } => {
                                        ui.add_space(4.0);
                                        ui.horizontal(|ui| {
                                            let p_bar = ProgressBar::new(*progress / 100.0)
                                                .show_percentage()
                                                .fill(ACCENT_BLUE);
                                            ui.add_sized(Vec2::new(ui.available_width() - 180.0, 14.0), p_bar);
                                            ui.label(
                                                RichText::new(status_text)
                                                    .size(10.0)
                                                    .color(Color32::from_rgb(0xc0, 0xc6, 0xd8)),
                                            );
                                        });
                                    }
                                    RenderJobStatus::Failed { error } => {
                                        ui.add_space(2.0);
                                        ui.label(
                                            RichText::new(format!("Hata: {}", error))
                                                .size(10.5)
                                                .color(ACCENT_RED),
                                        );
                                    }
                                    _ => {}
                                }
                            });
                    }
                });

                if let Some(c_id) = job_to_cancel {
                    queue.cancel_job(&c_id);
                }
                if let Some(r_id) = job_to_remove {
                    queue.remove_job(&r_id);
                }
                if let Some(p) = folder_to_open {
                    action = Some(RenderQueueDialogAction::OpenOutputFolder(p));
                }
            });

        self.is_open = open;
        action
    }
}
