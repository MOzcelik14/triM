//! Clip playback speed and reverse dialog modal.

use egui::{Button, Color32, DragValue, RichText, Window};

#[derive(Debug, Clone)]
pub struct SpeedDialog {
    pub is_open: bool,
    pub track_id: String,
    pub clip_id: String,
    pub speed: f64,
    pub reverse: bool,
    pub original_duration: f64,
}

impl Default for SpeedDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            track_id: String::new(),
            clip_id: String::new(),
            speed: 1.0,
            reverse: false,
            original_duration: 1.0,
        }
    }
}

pub enum SpeedDialogAction {
    Apply {
        track_id: String,
        clip_id: String,
        speed: f64,
        reverse: bool,
    },
}

impl SpeedDialog {
    pub fn open(
        &mut self,
        track_id: impl Into<String>,
        clip_id: impl Into<String>,
        current_speed: f64,
        reverse: bool,
        duration: f64,
    ) {
        self.track_id = track_id.into();
        self.clip_id = clip_id.into();
        self.speed = current_speed;
        self.reverse = reverse;
        self.original_duration = duration;
        self.is_open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<SpeedDialogAction> {
        if !self.is_open {
            return None;
        }

        let mut action = None;

        Window::new("Hız ve Süre")
            .collapsible(false)
            .resizable(false)
            .default_width(320.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Hız Çarpanı:");
                    ui.add(
                        DragValue::new(&mut self.speed)
                            .speed(0.1)
                            .range(0.1..=10.0)
                            .suffix("x"),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Hazır:");
                    if ui.button("0.5x").clicked() {
                        self.speed = 0.5;
                    }
                    if ui.button("1.0x").clicked() {
                        self.speed = 1.0;
                    }
                    if ui.button("2.0x").clicked() {
                        self.speed = 2.0;
                    }
                    if ui.button("4.0x").clicked() {
                        self.speed = 4.0;
                    }
                });

                ui.checkbox(&mut self.reverse, "Tersine Çevir (Geriye Doğru Oynat)");

                ui.add_space(8.0);
                let new_dur = (self.original_duration / self.speed).max(0.04);
                ui.label(
                    RichText::new(format!(
                        "Yeni Süre: {:.2}s  (Eski: {:.2}s)",
                        new_dur, self.original_duration
                    ))
                    .color(Color32::from_rgb(0xa0, 0xa0, 0xb0))
                    .size(11.0),
                );

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    let btn_apply =
                        Button::new(RichText::new("Uygula").color(Color32::WHITE).strong())
                            .fill(Color32::from_rgb(0xe0, 0x7a, 0x38));

                    if ui.add(btn_apply).clicked() {
                        action = Some(SpeedDialogAction::Apply {
                            track_id: self.track_id.clone(),
                            clip_id: self.clip_id.clone(),
                            speed: self.speed,
                            reverse: self.reverse,
                        });
                        self.is_open = false;
                    }

                    if ui.button("İptal").clicked() {
                        self.is_open = false;
                    }
                });
            });

        action
    }
}
