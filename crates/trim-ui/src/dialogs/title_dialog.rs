//! Synthetic title and text generator dialog modal with motion graphics & lower thirds support.

use egui::{Button, Color32, DragValue, RichText, Slider, Window};
use trim_core::clip::MotionTitleStyle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitlePosition {
    Center,
    LowerThird,
    Subtitle,
    TopHeader,
}

impl TitlePosition {
    pub fn as_str(&self) -> &'static str {
        match self {
            TitlePosition::Center => "center",
            TitlePosition::LowerThird => "lower_third",
            TitlePosition::Subtitle => "subtitle",
            TitlePosition::TopHeader => "top",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TitleDialog {
    pub is_open: bool,
    pub text: String,
    pub subtitle_text: String,
    pub font_size: u32,
    pub text_color: [u8; 3],
    pub bg_color: [u8; 4],
    pub position: TitlePosition,
    pub has_shadow: bool,
    pub has_box: bool,
    pub duration: f64,
    pub motion_style: MotionTitleStyle,
}

impl Default for TitleDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            text: "Başlık Metni".to_string(),
            subtitle_text: "Açıklama / Ünvan".to_string(),
            font_size: 54,
            text_color: [255, 255, 255],
            bg_color: [0, 0, 0, 0], // Transparent default
            position: TitlePosition::Center,
            has_shadow: true,
            has_box: false,
            duration: 5.0,
            motion_style: MotionTitleStyle::None,
        }
    }
}

pub enum TitleDialogAction {
    CreateTitle {
        text: String,
        font_size: u32,
        text_color: [u8; 3],
        bg_color: [u8; 4],
        position: String,
        has_shadow: bool,
        duration: f64,
        motion_style: MotionTitleStyle,
        subtitle_text: Option<String>,
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
            .default_width(450.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);

                // 1. Motion Style / Animation Presets
                ui.group(|ui| {
                    ui.strong("Animasyon Şablonu (Motion Preset)");
                    ui.horizontal_wrapped(|ui| {
                        for style in MotionTitleStyle::ALL {
                            let is_selected = self.motion_style == *style;
                            if ui.selectable_label(is_selected, style.label()).clicked() {
                                self.motion_style = *style;
                                match style {
                                    MotionTitleStyle::LowerThird => {
                                        self.position = TitlePosition::LowerThird;
                                        self.has_box = true;
                                        self.font_size = 44;
                                    }
                                    MotionTitleStyle::Ticker => {
                                        self.position = TitlePosition::Subtitle;
                                        self.has_box = true;
                                        self.font_size = 38;
                                    }
                                    MotionTitleStyle::CinematicZoom => {
                                        self.position = TitlePosition::Center;
                                        self.has_shadow = true;
                                    }
                                    MotionTitleStyle::Typewriter => {
                                        self.has_shadow = true;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    });
                });

                ui.add_space(6.0);

                // 2. Text Content (Main Title & Subtitle for Lower Thirds)
                ui.group(|ui| {
                    if self.motion_style == MotionTitleStyle::LowerThird {
                        ui.strong("Ana İsim / Başlık:");
                        ui.text_edit_singleline(&mut self.text);
                        ui.add_space(4.0);
                        ui.strong("Alt Başlık / Ünvan / Açıklama:");
                        ui.text_edit_singleline(&mut self.subtitle_text);
                    } else {
                        ui.strong("Metin İçeriği");
                        ui.text_edit_multiline(&mut self.text);
                    }
                });

                ui.add_space(6.0);

                // 3. Positioning
                ui.group(|ui| {
                    ui.strong("Konumlandırma");
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .selectable_label(self.position == TitlePosition::Center, "Orta Başlık")
                            .clicked()
                        {
                            self.position = TitlePosition::Center;
                            self.has_box = false;
                        }
                        if ui
                            .selectable_label(
                                self.position == TitlePosition::LowerThird,
                                "Alt Üçte Bir (Lower Third)",
                            )
                            .clicked()
                        {
                            self.position = TitlePosition::LowerThird;
                            self.has_box = true;
                            self.font_size = 42;
                        }
                        if ui
                            .selectable_label(
                                self.position == TitlePosition::Subtitle,
                                "Altyazı (Subtitle)",
                            )
                            .clicked()
                        {
                            self.position = TitlePosition::Subtitle;
                            self.has_box = true;
                            self.font_size = 36;
                        }
                        if ui
                            .selectable_label(
                                self.position == TitlePosition::TopHeader,
                                "Üst Başlık",
                            )
                            .clicked()
                        {
                            self.position = TitlePosition::TopHeader;
                            self.has_box = false;
                        }
                    });
                });

                ui.add_space(6.0);

                // 4. Formatting & Styling
                ui.group(|ui| {
                    ui.strong("Biçimlendirme & Stil");
                    ui.horizontal(|ui| {
                        ui.label("Yazı Boyutu:");
                        ui.add(Slider::new(&mut self.font_size, 16..=140).suffix(" pt"));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Klip Süresi:");
                        ui.add(
                            DragValue::new(&mut self.duration)
                                .speed(0.1)
                                .range(0.5..=60.0)
                                .suffix(" s"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut self.has_shadow,
                            "Kontur / Gölge (Her zemin için okunabilirlik)",
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.has_box, "Yarı Saydam Kutu Arka Planı");
                    });
                });

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    let btn_create = Button::new(
                        RichText::new("Zaman Çizgisine Ekle (Şeffaf Katman)")
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(0xe0, 0x7a, 0x38));

                    if ui.add(btn_create).clicked() {
                        let eff_bg = if self.has_box {
                            [0, 0, 0, 160] // Semi-transparent black box
                        } else {
                            [0, 0, 0, 0] // 100% transparent
                        };

                        let sub = if self.motion_style == MotionTitleStyle::LowerThird
                            && !self.subtitle_text.trim().is_empty()
                        {
                            Some(self.subtitle_text.trim().to_string())
                        } else {
                            None
                        };

                        action = Some(TitleDialogAction::CreateTitle {
                            text: self.text.clone(),
                            font_size: self.font_size,
                            text_color: self.text_color,
                            bg_color: eff_bg,
                            position: self.position.as_str().to_string(),
                            has_shadow: self.has_shadow,
                            duration: self.duration,
                            motion_style: self.motion_style,
                            subtitle_text: sub,
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
