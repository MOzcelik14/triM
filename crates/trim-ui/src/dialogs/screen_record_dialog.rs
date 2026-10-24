//! Screen and Webcam Recording Modal Dialog.

use egui::{Button, Color32, Key, RichText, Window};
use std::path::PathBuf;
use trim_media::{CaptureSource, ScreenRecorder};

#[derive(Debug, Clone)]
pub enum ScreenRecordAction {
    RecordingSaved {
        file_path: PathBuf,
        auto_add_to_timeline: bool,
    },
}

pub struct ScreenRecordDialog {
    pub is_open: bool,
    pub source: CaptureSource,
    pub recorder: Option<ScreenRecorder>,
    pub auto_add_to_timeline: bool,
    pub status_message: String,
}

impl Default for ScreenRecordDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            source: CaptureSource::ScreenAndMic,
            recorder: None,
            auto_add_to_timeline: true,
            status_message: "Kayıt kaynağını seçip 'Kaydı Başlat' butonuna tıklayın.".to_string(),
        }
    }
}

impl ScreenRecordDialog {
    pub fn open(&mut self) {
        self.is_open = true;
        self.recorder = None;
        self.status_message = "Kayıt başlatmaya hazır.".to_string();
    }

    pub fn close(&mut self) {
        if let Some(mut rec) = self.recorder.take() {
            let _ = rec.stop();
        }
        self.is_open = false;
    }

    pub fn is_recording(&mut self) -> bool {
        self.recorder.as_mut().is_some_and(|r| r.is_recording())
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> Option<ScreenRecordAction> {
        if !self.is_open {
            return None;
        }

        let mut action = None;
        let mut open = self.is_open;
        let is_rec = self.is_recording();

        Window::new(
            RichText::new("📹 Ekran & Web Kamerası Kaydedici")
                .size(15.0)
                .strong(),
        )
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .min_width(440.0)
        .show(ctx, |ui| {
            ui.add_space(4.0);

            if is_rec {
                let elapsed = self
                    .recorder
                    .as_ref()
                    .map(|r| r.elapsed_seconds())
                    .unwrap_or(0.0);
                let mins = (elapsed / 60.0).floor() as u64;
                let secs = (elapsed % 60.0).floor() as u64;

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let blink = ctx.input(|i| i.time * 2.0).sin() > 0.0;
                        let rec_color = if blink {
                            Color32::from_rgb(240, 50, 50)
                        } else {
                            Color32::from_rgb(180, 40, 40)
                        };
                        ui.label(RichText::new("● REC").color(rec_color).size(16.0).strong());
                        ui.label(
                            RichText::new(format!("{:02}:{:02}", mins, secs))
                                .size(16.0)
                                .monospace(),
                        );
                        ui.label(format!("({})", self.source.label()));
                    });
                });
            } else {
                ui.group(|ui| {
                    egui::Grid::new("screen_rec_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Kayıt Kaynağı:");
                            egui::ComboBox::from_id_salt("rec_src_combo")
                                .selected_text(self.source.label())
                                .show_ui(ui, |ui| {
                                    for s in CaptureSource::ALL {
                                        ui.selectable_value(&mut self.source, *s, s.label());
                                    }
                                });
                            ui.end_row();

                            ui.label("Otomatik İçe Aktar:");
                            ui.checkbox(
                                &mut self.auto_add_to_timeline,
                                "Zaman çizgisine otomatik ekle",
                            );
                            ui.end_row();
                        });
                });
            }

            ui.add_space(6.0);
            ui.label(
                RichText::new(&self.status_message)
                    .size(11.5)
                    .color(Color32::from_rgb(0xc0, 0xc0, 0xd0)),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if is_rec {
                    let stop_btn = Button::new(
                        RichText::new("⏹️ Kaydı Bitir ve İçe Aktar")
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(0x38, 0xa0, 0x50));

                    if ui.add(stop_btn).clicked() {
                        if let Some(mut rec) = self.recorder.take() {
                            match rec.stop() {
                                Ok(_) => {
                                    let file_path = rec.output_path().to_path_buf();
                                    self.status_message =
                                        format!("Kayıt tamamlandı: {:?}", file_path.file_name());
                                    action = Some(ScreenRecordAction::RecordingSaved {
                                        file_path,
                                        auto_add_to_timeline: self.auto_add_to_timeline,
                                    });
                                    self.is_open = false;
                                }
                                Err(e) => {
                                    self.status_message =
                                        format!("Kayıt sonlandırma hatası: {}", e);
                                }
                            }
                        }
                    }
                } else {
                    let start_btn = Button::new(
                        RichText::new("🔴 Kaydı Başlat")
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(0xd0, 0x40, 0x40));

                    if ui.add(start_btn).clicked() {
                        let temp_dir = std::env::temp_dir().join("trim_captures");
                        let _ = std::fs::create_dir_all(&temp_dir);
                        let filename = format!("kayit_{}.mp4", chrono_timestamp());
                        let out_path = temp_dir.join(filename);

                        match ScreenRecorder::start(&out_path, self.source) {
                            Ok(rec) => {
                                self.recorder = Some(rec);
                                self.status_message = "Kayıt devam ediyor...".to_string();
                            }
                            Err(e) => {
                                self.status_message = format!("Başlatma hatası: {}", e);
                            }
                        }
                    }

                    if ui.button("Kapat").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                        self.close();
                    }
                }
            });
        });

        if is_rec {
            ctx.request_repaint();
        }

        self.is_open = open;
        action
    }
}

fn chrono_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
