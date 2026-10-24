//! Auto Silence Cut (Jump-Cut) Dialog.

use egui::{Button, Color32, DragValue, Key, RichText, Slider, Window};

#[derive(Debug, Clone)]
pub enum SilenceDialogAction {
    DetectRequested {
        track_id: String,
        clip_id: String,
        noise_db: f32,
        min_duration: f64,
    },
    ApplyCut {
        track_id: String,
        clip_id: String,
        speech_intervals: Vec<(f64, f64)>,
    },
}

#[derive(Debug, Clone)]
pub struct SilenceDialog {
    pub is_open: bool,
    pub track_id: String,
    pub clip_id: String,
    pub clip_name: String,
    pub clip_duration: f64,
    pub noise_db: f32,
    pub min_duration: f64,
    pub padding: f64,
    pub is_analyzing: bool,
    pub detected_silence_count: Option<usize>,
    pub total_silence_duration: Option<f64>,
    pub speech_intervals: Option<Vec<(f64, f64)>>,
    pub status_text: String,
}

impl Default for SilenceDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            track_id: String::new(),
            clip_id: String::new(),
            clip_name: String::new(),
            clip_duration: 0.0,
            noise_db: -35.0,
            min_duration: 0.40,
            padding: 0.08,
            is_analyzing: false,
            detected_silence_count: None,
            total_silence_duration: None,
            speech_intervals: None,
            status_text: "Sessizlikleri tespit etmek için 'Analiz Et' butonuna tıklayın."
                .to_string(),
        }
    }
}

impl SilenceDialog {
    pub fn open(
        &mut self,
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        clip_name: impl Into<String>,
        duration: f64,
    ) {
        self.track_id = track_id.into();
        self.clip_id = clip_id.into();
        self.clip_name = clip_name.into();
        self.clip_duration = duration;
        self.is_analyzing = false;
        self.detected_silence_count = None;
        self.total_silence_duration = None;
        self.speech_intervals = None;
        self.status_text = format!(
            "Klip: \"{}\" ({:.1}s) hazır.",
            self.clip_name, self.clip_duration
        );
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.is_analyzing = false;
    }

    pub fn set_analysis_results(&mut self, silences: Vec<(f64, f64)>, speech: Vec<(f64, f64)>) {
        self.is_analyzing = false;
        let total_sil_dur: f64 = silences.iter().map(|(s, e)| (e - s).max(0.0)).sum();
        self.detected_silence_count = Some(silences.len());
        self.total_silence_duration = Some(total_sil_dur);
        self.speech_intervals = Some(speech);
        self.status_text = format!(
            "Tespit: {} sessiz bölge ({:.2}s tasarruf).",
            silences.len(),
            total_sil_dur
        );
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<SilenceDialogAction> {
        if !self.is_open {
            return None;
        }

        let mut action = None;

        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.close();
            return None;
        }

        Window::new("⚡ Sessizlikleri Otomatik Kes (Jump-Cut)")
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚡").size(24.0).color(Color32::from_rgb(0xe0, 0x7a, 0x38)));
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("Klip: {}", self.clip_name)).strong().color(Color32::WHITE));
                        ui.label(RichText::new("Konuşma aralarındaki esleri ve nefesleri otomatik kaldırarak akıcı video üretir.").size(12.0).color(Color32::from_rgb(0xa0, 0xa0, 0xb0)));
                    });
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                egui::Grid::new("silence_params_grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                    ui.label("Gürültü Eşiği (dB):");
                    ui.horizontal(|ui| {
                        ui.add(Slider::new(&mut self.noise_db, -60.0..=-15.0).suffix(" dB").step_by(1.0));
                    });
                    ui.end_row();

                    ui.label("Minimum Boşluk Süresi:");
                    ui.horizontal(|ui| {
                        ui.add(DragValue::new(&mut self.min_duration).speed(0.05).range(0.1..=3.0).suffix(" s"));
                        ui.label(RichText::new("(Örn: 0.4s altı kesilmez)").size(11.0).color(Color32::GRAY));
                    });
                    ui.end_row();

                    ui.label("Konuşma Kenar Payı (Padding):");
                    ui.horizontal(|ui| {
                        ui.add(DragValue::new(&mut self.padding).speed(0.01).range(0.0..=0.3).suffix(" s"));
                        ui.label(RichText::new("(Harf yutmayı önler)").size(11.0).color(Color32::GRAY));
                    });
                    ui.end_row();
                });

                ui.add_space(10.0);

                // Status message & findings
                let status_color = if self.speech_intervals.is_some() {
                    Color32::from_rgb(0x50, 0xc8, 0x78)
                } else if self.is_analyzing {
                    Color32::from_rgb(0xe0, 0xa0, 0x30)
                } else {
                    Color32::from_rgb(0xa0, 0xa0, 0xb0)
                };

                ui.horizontal(|ui| {
                    if self.is_analyzing {
                        ui.spinner();
                    }
                    ui.label(RichText::new(&self.status_text).color(status_color).size(12.5));
                });

                ui.add_space(14.0);

                ui.horizontal(|ui| {
                    // Step 1: Analyze button
                    let analyze_btn = Button::new(RichText::new("🔍 Analiz Et").strong());
                    if ui.add_enabled(!self.is_analyzing, analyze_btn).clicked() {
                        self.is_analyzing = true;
                        self.status_text = "FFmpeg ile ses dalgası taranıyor...".to_string();
                        action = Some(SilenceDialogAction::DetectRequested {
                            track_id: self.track_id.clone(),
                            clip_id: self.clip_id.clone(),
                            noise_db: self.noise_db,
                            min_duration: self.min_duration,
                        });
                    }

                    // Step 2: Apply cut button
                    let can_apply = self.speech_intervals.as_ref().map(|s| !s.is_empty()).unwrap_or(false) && !self.is_analyzing;
                    let apply_btn = Button::new(RichText::new("✂️ Kes ve Boşlukları Kapat").color(Color32::WHITE).strong())
                        .fill(Color32::from_rgb(0xe0, 0x7a, 0x38));

                    if ui.add_enabled(can_apply, apply_btn).clicked() {
                        if let Some(intervals) = self.speech_intervals.take() {
                            action = Some(SilenceDialogAction::ApplyCut {
                                track_id: self.track_id.clone(),
                                clip_id: self.clip_id.clone(),
                                speech_intervals: intervals,
                            });
                            self.close();
                        }
                    }

                    if ui.button("Kapat").clicked() {
                        self.close();
                    }
                });
            });

        action
    }
}
