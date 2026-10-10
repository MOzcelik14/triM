//! Synthetic title and text generator dialog modal.

use egui::{Button, Color32, DragValue, RichText, Slider, Window};

#[derive(Debug, Clone)]
pub struct TitleDialog {
    pub is_open: bool,
    pub text: String,
    pub font_size: u32,
    pub text_color: [u8; 3],
    pub bg_color: [u8; 4],
    pub duration: f64,
}

impl Default for TitleDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            text: "Başlık Metni".to_string(),
            font_size: 64,
            text_color: [255, 255, 255],
            bg_color: [0, 0, 0, 0], // Transparent default
            duration: 5.0,
        }
    }
}

pub enum TitleDialogAction {
    CreateTitle {
        text: String,
        font_size: u32,
        text_color: [u8; 3],
        bg_color: [u8; 4],
        duration: f64,
    },
}

impl TitleDialog {
    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<TitleDialogAction> {
        if !self.is_open {
            return None;
        }

        let mut action = None;

        Window::new("Yeni Başlık / Metin Klibi Ekle")
            .collapsible(false)
            .resizable(false)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);

                ui.group(|ui| {
                    ui.strong("Metin İçeriği");
                    ui.text_edit_multiline(&mut self.text);
                });

                ui.add_space(6.0);

                ui.group(|ui| {
                    ui.strong("Biçimlendirme & Boyut");
                    ui.horizontal(|ui| {
                        ui.label("Yazı Boyutu:");
                        ui.add(Slider::new(&mut self.font_size, 16..=160).suffix(" pt"));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Klip Süresi:");
                        ui.add(DragValue::new(&mut self.duration).speed(0.1).range(0.5..=60.0).suffix(" s"));
                    });
                });

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    let btn_create = Button::new(RichText::new("Zaman Çizgisine Ekle").color(Color32::WHITE).strong())
                        .fill(Color32::from_rgb(0xe0, 0x7a, 0x38));

                    if ui.add(btn_create).clicked() {
                        action = Some(TitleDialogAction::CreateTitle {
                            text: self.text.clone(),
                            font_size: self.font_size,
                            text_color: self.text_color,
                            bg_color: self.bg_color,
                            duration: self.duration,
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
