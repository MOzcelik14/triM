//! Local Speech-to-Text Whisper Transcription Dialog.

use egui::{Button, Color32, Key, RichText, Window};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum TranscribeDialogAction {
    StartTranscribe {
        media_path: String,
        model_name: String,
        language: String,
    },
}

#[derive(Debug, Clone)]
pub struct TranscribeDialog {
    pub is_open: bool,
    pub media_path: String,
    pub clip_name: String,
    pub language: String,
    pub model_name: String,
    pub whisper_bin: Option<PathBuf>,
    pub is_transcribing: bool,
    pub status_message: String,
}

impl Default for TranscribeDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            media_path: String::new(),
            clip_name: String::new(),
            language: "auto".to_string(),
            model_name: "base".to_string(),
            whisper_bin: trim_media::find_whisper_executable(),
            is_transcribing: false,
            status_message: "Seçili klibi yerel yapay zeka ile deşifre edip altyazı oluşturun.".to_string(),
        }
    }
}

impl TranscribeDialog {
    pub fn open(&mut self, media_path: impl Into<String>, clip_name: impl Into<String>) {
        self.media_path = media_path.into();
        self.clip_name = clip_name.into();
        self.whisper_bin = trim_media::find_whisper_executable();
        self.is_transcribing = false;
        self.status_message = "Deşifre başlatmaya hazır.".to_string();
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> Option<TranscribeDialogAction> {
        if !self.is_open {
            return None;
        }

        let mut action = None;
        let mut open = self.is_open;

        Window::new(RichText::new("🎙️ Yerel Whisper ile Deşifre Et (Speech-to-Text)").size(15.0).strong())
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(460.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);

                // Source clip info
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Kaynak Medya:").strong());
                        ui.label(RichText::new(&self.clip_name).color(Color32::from_rgb(0x60, 0xb0, 0xf0)));
                    });
                });
                ui.add_space(6.0);

                // Binary status badge
                if let Some(bin) = &self.whisper_bin {
                    ui.label(
                        RichText::new(format!("✓ Whisper Bulundu: {}", bin.file_name().unwrap_or_default().to_string_lossy()))
                            .color(Color32::from_rgb(0x50, 0xc8, 0x78))
                            .size(11.5),
                    );
                } else {
                    ui.group(|ui| {
                        ui.label(
                            RichText::new("⚠️ Yerel Whisper CLI veya whisper.cpp bulunamadı")
                                .color(Color32::from_rgb(0xe0, 0x90, 0x30))
                                .strong(),
                        );
                        ui.label(
                            RichText::new(
                                "Çevrimdışı deşifre için lütfen 'whisper-cli' veya 'openai-whisper' kurun ya da ikili dosyayı triM yanına yerleştirin.",
                            )
                            .size(11.0)
                            .color(Color32::from_rgb(0xa0, 0xa0, 0xb0)),
                        );
                    });
                }
                ui.add_space(6.0);

                // Settings
                ui.group(|ui| {
                    egui::Grid::new("whisper_settings_grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                        ui.label("Dil:");
                        egui::ComboBox::from_id_salt("whisper_lang_combo")
                            .selected_text(match self.language.as_str() {
                                "tr" => "Türkçe (tr)",
                                "en" => "İngilizce (en)",
                                "de" => "Almanca (de)",
                                "fr" => "Fransızca (fr)",
                                "es" => "İspanyolca (es)",
                                _ => "Otomatik Algıla (auto)",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.language, "auto".to_string(), "Otomatik Algıla (auto)");
                                ui.selectable_value(&mut self.language, "tr".to_string(), "Türkçe (tr)");
                                ui.selectable_value(&mut self.language, "en".to_string(), "İngilizce (en)");
                                ui.selectable_value(&mut self.language, "de".to_string(), "Almanca (de)");
                                ui.selectable_value(&mut self.language, "fr".to_string(), "Fransızca (fr)");
                                ui.selectable_value(&mut self.language, "es".to_string(), "İspanyolca (es)");
                            });
                        ui.end_row();

                        ui.label("Model Boyutu:");
                        egui::ComboBox::from_id_salt("whisper_model_combo")
                            .selected_text(match self.model_name.as_str() {
                                "tiny" => "Tiny (~75 MB - En Hızlı)",
                                "small" => "Small (~460 MB - Dengeli)",
                                "medium" => "Medium (~1.5 GB - Yüksek Kalite)",
                                _ => "Base (~140 MB - Önerilen)",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.model_name, "tiny".to_string(), "Tiny (~75 MB - En Hızlı)");
                                ui.selectable_value(&mut self.model_name, "base".to_string(), "Base (~140 MB - Önerilen)");
                                ui.selectable_value(&mut self.model_name, "small".to_string(), "Small (~460 MB - Dengeli)");
                                ui.selectable_value(&mut self.model_name, "medium".to_string(), "Medium (~1.5 GB - Yüksek Kalite)");
                            });
                        ui.end_row();
                    });
                });
                ui.add_space(6.0);

                // Status message
                ui.label(RichText::new(&self.status_message).size(11.5).color(Color32::from_rgb(0xc0, 0xc0, 0xd0)));
                ui.add_space(8.0);

                // Action buttons
                ui.horizontal(|ui| {
                    let can_run = !self.is_transcribing && !self.media_path.is_empty();
                    let start_btn = Button::new(
                        RichText::new(if self.is_transcribing { "⏳ Deşifre Ediliyor..." } else { "⚡ Deşifre Et ve Altyazı Ekle" })
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(0x40, 0x80, 0xd0));

                    if ui.add_enabled(can_run, start_btn).clicked() {
                        self.is_transcribing = true;
                        self.status_message = "Ses 16kHz mono olarak ayıklanıyor ve analiz ediliyor...".to_string();
                        action = Some(TranscribeDialogAction::StartTranscribe {
                            media_path: self.media_path.clone(),
                            model_name: self.model_name.clone(),
                            language: self.language.clone(),
                        });
                    }

                    if ui.button("Kapat").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                        self.close();
                    }
                });
            });

        self.is_open = open;
        action
    }
}
