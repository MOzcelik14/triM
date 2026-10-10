//! Export video dialog modal.

use egui::{Button, Color32, ProgressBar, RichText, Window};
use trim_export::presets::{get_default_presets, ExportPreset};

#[derive(Debug, Clone)]
pub struct ExportDialog {
    pub is_open: bool,
    pub presets: Vec<ExportPreset>,
    pub selected_preset_idx: usize,
    pub output_path: String,
    pub is_exporting: bool,
    pub progress: f32, // 0.0 .. 100.0
    pub status_text: String,
}

impl Default for ExportDialog {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let default_out = format!("{}/Masaüstü/cikti.mp4", home);
        Self {
            is_open: false,
            presets: get_default_presets(),
            selected_preset_idx: 0,
            output_path: default_out,
            is_exporting: false,
            progress: 0.0,
            status_text: "Hazır".to_string(),
        }
    }
}

pub enum ExportDialogAction {
    StartExport(ExportPreset, String),
    CancelExport,
}

impl ExportDialog {
    pub fn show(&mut self, ctx: &egui::Context) -> Option<ExportDialogAction> {
        if !self.is_open {
            return None;
        }

        let mut action = None;

        Window::new("Videoyu Dışa Aktar")
            .collapsible(false)
            .resizable(false)
            .default_width(450.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);

                if !self.is_exporting {
                    ui.group(|ui| {
                        ui.strong("Hazır Dışa Aktarma Profili");
                        for (idx, p) in self.presets.iter().enumerate() {
                            if ui.selectable_label(self.selected_preset_idx == idx, &p.name).clicked() {
                                self.selected_preset_idx = idx;
                            }
                        }
                    });

                    ui.add_space(8.0);

                    ui.group(|ui| {
                        ui.strong("Çıktı Dosyası");
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut self.output_path);
                            if ui.button("Gözat...").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_file_name("cikti.mp4")
                                    .add_filter("MP4 Video", &["mp4"])
                                    .save_file()
                                {
                                    self.output_path = path.to_string_lossy().to_string();
                                }
                            }
                        });
                    });

                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        let btn_start = Button::new(RichText::new("Dışa Aktarmayı Başlat").color(Color32::WHITE).strong())
                            .fill(Color32::from_rgb(0xe0, 0x7a, 0x38));

                        if ui.add(btn_start).clicked() {
                            let preset = self.presets[self.selected_preset_idx].clone();
                            self.is_exporting = true;
                            self.progress = 0.0;
                            action = Some(ExportDialogAction::StartExport(preset, self.output_path.clone()));
                        }

                        if ui.button("Kapat").clicked() {
                            self.is_open = false;
                        }
                    });
                } else {
                    ui.group(|ui| {
                        ui.strong("Dışa Aktarma Durumu");
                        ui.add_space(6.0);
                        ui.add(ProgressBar::new(self.progress / 100.0).text(format!("{:.1}%", self.progress)));
                        ui.add_space(6.0);
                        ui.label(RichText::new(&self.status_text).size(11.0).color(Color32::from_rgb(0xa0, 0xa0, 0xb0)));
                    });

                    ui.add_space(12.0);

                    if ui.button(RichText::new("İptal Et").color(Color32::from_rgb(0xe0, 0x50, 0x50))).clicked() {
                        action = Some(ExportDialogAction::CancelExport);
                    }
                }
            });

        action
    }
}
