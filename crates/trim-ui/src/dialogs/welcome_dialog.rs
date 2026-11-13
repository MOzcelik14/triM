//! Modern Welcome / Project Hub modal with triM. branding, 2x2 template cards, and Recent Projects.

use egui::{
    Align, Align2, Area, Button, Color32, FontId, Frame, Id, Layout, Margin, Order, Pos2, RichText,
    Rounding, ScrollArea, Sense, Shadow, Stroke, TextEdit, Vec2, Window,
};

#[derive(Debug, Clone)]
pub struct RecentProjectEntry {
    pub name: String,
    pub path: String,
    pub last_modified: String,
}

#[derive(Debug, Clone)]
pub struct WelcomeDialog {
    pub is_open: bool,
    pub new_project_name: String,
    pub selected_preset_idx: usize,
    pub recent_projects: Vec<RecentProjectEntry>,
}

pub enum WelcomeAction {
    CreateNewProject {
        name: String,
        width: u32,
        height: u32,
        fps: f64,
    },
    OpenProjectRequested,
    OpenRecentProject(String),
    Close,
}

struct FormatPreset {
    title: &'static str,
    subtitle: &'static str,
    badge: &'static str,
    icon: &'static str,
    width: u32,
    height: u32,
    fps: f64,
}

const FORMAT_PRESETS: [FormatPreset; 4] = [
    FormatPreset {
        title: "1080p Standart",
        subtitle: "1920×1080 · 30 FPS",
        badge: "16:9 Yatay",
        icon: "🖥",
        width: 1920,
        height: 1080,
        fps: 30.0,
    },
    FormatPreset {
        title: "4K Sinematik",
        subtitle: "3840×2160 · 60 FPS",
        badge: "UHD Master",
        icon: "🎬",
        width: 3840,
        height: 2160,
        fps: 60.0,
    },
    FormatPreset {
        title: "9:16 Shorts/Reels",
        subtitle: "1080×1920 · 30 FPS",
        badge: "Dikey Mobil",
        icon: "📱",
        width: 1080,
        height: 1920,
        fps: 30.0,
    },
    FormatPreset {
        title: "720p Hızlı Web",
        subtitle: "1280×720 · 30 FPS",
        badge: "Hafif Kurgu",
        icon: "⚡",
        width: 1280,
        height: 720,
        fps: 30.0,
    },
];

impl Default for WelcomeDialog {
    fn default() -> Self {
        let home = trim_core::paths::home_dir();
        let mut recents = Vec::new();
        let default_proj = home.join("Videos").join("Proje 1.trim");
        if default_proj.exists() {
            recents.push(RecentProjectEntry {
                name: "Proje 1".to_string(),
                path: default_proj.to_string_lossy().to_string(),
                last_modified: "Son kullanılan".to_string(),
            });
        }

        Self {
            is_open: true,
            new_project_name: "Yeni Proje".to_string(),
            selected_preset_idx: 0,
            recent_projects: recents,
        }
    }
}

impl WelcomeDialog {
    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn add_recent(&mut self, name: &str, path: &str) {
        self.recent_projects.retain(|p| p.path != path);
        self.recent_projects.insert(
            0,
            RecentProjectEntry {
                name: name.to_string(),
                path: path.to_string(),
                last_modified: "Şimdi".to_string(),
            },
        );
        if self.recent_projects.len() > 10 {
            self.recent_projects.truncate(10);
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<WelcomeAction> {
        if !self.is_open {
            return None;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.is_open = false;
            return Some(WelcomeAction::Close);
        }

        // Full-screen modal backdrop dimmer & input blocker
        let screen_rect = ctx.screen_rect();
        Area::new(Id::new("welcome_modal_backdrop"))
            .order(Order::Middle)
            .fixed_pos(Pos2::ZERO)
            .show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(screen_rect.size(), Sense::click_and_drag());
                ui.painter()
                    .rect_filled(rect, 0.0, Color32::from_black_alpha(175));
            });

        let mut action = None;

        let modal_frame = Frame::window(&ctx.style())
            .fill(Color32::from_rgb(0x16, 0x17, 0x1e))
            .rounding(Rounding::same(10.0))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x2e, 0x31, 0x3e)))
            .inner_margin(Margin::symmetric(22.0, 20.0))
            .shadow(Shadow {
                offset: Vec2::new(0.0, 14.0),
                blur: 36.0,
                spread: 4.0,
                color: Color32::from_black_alpha(180),
            });

        Window::new("triM. - Başlangıç")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .order(Order::Foreground)
            .fixed_size(Vec2::new(740.0, 480.0))
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(modal_frame)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // =========================================================
                    // LEFT PANEL: Branding, Project Name & 2x2 Format Cards
                    // =========================================================
                    let left_w = 336.0;
                    let panel_h = 440.0;

                    ui.allocate_ui(Vec2::new(left_w, panel_h), |ui| {
                        ui.vertical(|ui| {
                            // Top Branding Row: triM. Logo + Version Pill Badge
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("triM.")
                                        .size(30.0)
                                        .strong()
                                        .color(Color32::from_rgb(0xe0, 0x7a, 0x38)),
                                );
                                ui.add_space(6.0);
                                Frame::none()
                                    .fill(Color32::from_rgb(0x28, 0x1e, 0x18))
                                    .rounding(Rounding::same(6.0))
                                    .stroke(Stroke::new(
                                        1.0_f32,
                                        Color32::from_rgb(0x8a, 0x4b, 0x24),
                                    ))
                                    .inner_margin(Margin::symmetric(8.0, 3.0))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "v{}",
                                                env!("CARGO_PKG_VERSION")
                                            ))
                                            .size(11.0)
                                            .strong()
                                            .color(Color32::from_rgb(0xf2, 0x9b, 0x5e)),
                                        );
                                    });
                            });

                            ui.add_space(2.0);
                            ui.label(
                                RichText::new("Doğrusal Olmayan Masaüstü Video Kurgu Stüdyosu")
                                    .size(11.5)
                                    .color(Color32::from_rgb(0x8e, 0x92, 0xa4)),
                            );

                            ui.add_space(14.0);
                            ui.separator();
                            ui.add_space(12.0);

                            // Project Name Field
                            ui.label(
                                RichText::new("Yeni Proje Adı")
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::from_rgb(0xd8, 0xdc, 0xe8)),
                            );
                            ui.add_space(4.0);
                            ui.add_sized(
                                [left_w, 30.0],
                                TextEdit::singleline(&mut self.new_project_name)
                                    .hint_text("İsimsiz Proje")
                                    .margin(Margin::symmetric(10.0, 6.0)),
                            );

                            ui.add_space(14.0);

                            // 2x2 Format / Resolution Mini Cards
                            ui.label(
                                RichText::new("Sekans Formatı ve Çözünürlük")
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::from_rgb(0xd8, 0xdc, 0xe8)),
                            );
                            ui.add_space(6.0);

                            let card_w = (left_w - 10.0) * 0.5;
                            let card_h = 74.0;

                            for row in 0..2 {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 10.0;
                                    for col in 0..2 {
                                        let idx = row * 2 + col;
                                        let preset = &FORMAT_PRESETS[idx];
                                        let is_sel = self.selected_preset_idx == idx;

                                        let (rect, resp) = ui.allocate_exact_size(
                                            Vec2::new(card_w, card_h),
                                            Sense::click(),
                                        );
                                        if resp.clicked() {
                                            self.selected_preset_idx = idx;
                                        }

                                        let is_hov = resp.hovered();
                                        let (fill, stroke, title_col) = if is_sel {
                                            (
                                                Color32::from_rgb(0x2a, 0x20, 0x19),
                                                Stroke::new(
                                                    1.8_f32,
                                                    Color32::from_rgb(0xe0, 0x7a, 0x38),
                                                ),
                                                Color32::WHITE,
                                            )
                                        } else if is_hov {
                                            (
                                                Color32::from_rgb(0x22, 0x24, 0x2f),
                                                Stroke::new(
                                                    1.2_f32,
                                                    Color32::from_rgb(0x48, 0x4c, 0x62),
                                                ),
                                                Color32::from_rgb(0xf0, 0xf2, 0xf8),
                                            )
                                        } else {
                                            (
                                                Color32::from_rgb(0x1c, 0x1e, 0x26),
                                                Stroke::new(
                                                    1.0_f32,
                                                    Color32::from_rgb(0x2c, 0x2f, 0x3c),
                                                ),
                                                Color32::from_rgb(0xd0, 0xd4, 0xe2),
                                            )
                                        };

                                        let painter = ui.painter();
                                        painter.rect(rect, Rounding::same(8.0), fill, stroke);

                                        // Icon + Badge on top row of card
                                        painter.text(
                                            Pos2::new(rect.left() + 10.0, rect.top() + 15.0),
                                            Align2::LEFT_CENTER,
                                            preset.icon,
                                            FontId::proportional(14.0),
                                            if is_sel {
                                                Color32::from_rgb(0xe0, 0x7a, 0x38)
                                            } else {
                                                Color32::from_rgb(0x90, 0x96, 0xaa)
                                            },
                                        );
                                        painter.text(
                                            Pos2::new(rect.right() - 10.0, rect.top() + 15.0),
                                            Align2::RIGHT_CENTER,
                                            preset.badge,
                                            FontId::proportional(10.0),
                                            if is_sel {
                                                Color32::from_rgb(0xf2, 0x9b, 0x5e)
                                            } else {
                                                Color32::from_rgb(0x72, 0x76, 0x8a)
                                            },
                                        );

                                        // Card Title
                                        painter.text(
                                            Pos2::new(rect.left() + 10.0, rect.top() + 39.0),
                                            Align2::LEFT_CENTER,
                                            preset.title,
                                            FontId::proportional(12.5),
                                            title_col,
                                        );

                                        // Card Subtitle (Resolution & FPS)
                                        painter.text(
                                            Pos2::new(rect.left() + 10.0, rect.top() + 57.0),
                                            Align2::LEFT_CENTER,
                                            preset.subtitle,
                                            FontId::monospace(10.0),
                                            if is_sel {
                                                Color32::from_rgb(0xd8, 0xb8, 0xa0)
                                            } else {
                                                Color32::from_rgb(0x82, 0x86, 0x98)
                                            },
                                        );
                                    }
                                });
                                if row == 0 {
                                    ui.add_space(10.0);
                                }
                            }

                            ui.add_space(18.0);

                            // Primary CTA: Projeyi Başlat
                            let chosen = &FORMAT_PRESETS[self.selected_preset_idx.min(3)];
                            let cta_btn = Button::new(
                                RichText::new("Projeyi Başlat")
                                    .size(14.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .min_size(Vec2::new(left_w, 40.0))
                            .rounding(Rounding::same(7.0))
                            .fill(Color32::from_rgb(0xe0, 0x7a, 0x38));

                            if ui.add(cta_btn).clicked() {
                                let final_name = if self.new_project_name.trim().is_empty() {
                                    "İsimsiz Proje".to_string()
                                } else {
                                    self.new_project_name.trim().to_string()
                                };
                                action = Some(WelcomeAction::CreateNewProject {
                                    name: final_name,
                                    width: chosen.width,
                                    height: chosen.height,
                                    fps: chosen.fps,
                                });
                                self.is_open = false;
                            }
                        });
                    });

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(12.0);

                    // =========================================================
                    // RIGHT PANEL: Recent Projects & Quick Access
                    // =========================================================
                    let right_w = 336.0;
                    ui.allocate_ui(Vec2::new(right_w, panel_h), |ui| {
                        ui.vertical(|ui| {
                            // Header Row: "Son Projeler" + "Mevcut Projeyi Aç..."
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Son Projeler")
                                        .size(15.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let open_btn = Button::new(
                                        RichText::new("📂 Mevcut Projeyi Aç...")
                                            .size(11.5)
                                            .strong()
                                            .color(Color32::from_rgb(0xe4, 0xe8, 0xf4)),
                                    )
                                    .rounding(Rounding::same(6.0))
                                    .fill(Color32::from_rgb(0x25, 0x38, 0x56))
                                    .stroke(Stroke::new(
                                        1.0_f32,
                                        Color32::from_rgb(0x3a, 0x5a, 0x8c),
                                    ));

                                    if ui
                                        .add(open_btn)
                                        .on_hover_text("Diskteki bir .trim proje dosyasını aç")
                                        .clicked()
                                    {
                                        action = Some(WelcomeAction::OpenProjectRequested);
                                        self.is_open = false;
                                    }
                                });
                            });

                            ui.add_space(10.0);

                            // Recent Projects List Area (Height ~346px)
                            let list_h = 346.0;
                            Frame::none()
                                .fill(Color32::from_rgb(0x12, 0x13, 0x19))
                                .rounding(Rounding::same(8.0))
                                .stroke(Stroke::new(
                                    1.0_f32,
                                    Color32::from_rgb(0x25, 0x27, 0x33),
                                ))
                                .inner_margin(Margin::same(10.0))
                                .show(ui, |ui| {
                                    ui.set_min_size(Vec2::new(right_w - 22.0, list_h));

                                    if self.recent_projects.is_empty() {
                                        ui.vertical_centered(|ui| {
                                            ui.add_space(105.0);
                                            ui.label(
                                                RichText::new("📂")
                                                    .size(38.0)
                                                    .color(Color32::from_rgb(0x4c, 0x50, 0x62)),
                                            );
                                            ui.add_space(8.0);
                                            ui.label(
                                                RichText::new("Yakın zamanda açılan bir proje yok")
                                                    .size(13.0)
                                                    .strong()
                                                    .color(Color32::from_rgb(0x94, 0x98, 0xac)),
                                            );
                                            ui.add_space(4.0);
                                            ui.label(
                                                RichText::new(
                                                    "Kaydettiğiniz .trim projeleri hızlı erişim için burada listelenir.",
                                                )
                                                .size(11.0)
                                                .color(Color32::from_rgb(0x66, 0x6a, 0x7c)),
                                            );
                                        });
                                    } else {
                                        ScrollArea::vertical().max_height(list_h).show(ui, |ui| {
                                            let mut to_remove = None;
                                            for (idx, p) in
                                                self.recent_projects.iter().enumerate()
                                            {
                                                Frame::none()
                                                    .fill(Color32::from_rgb(0x1b, 0x1d, 0x26))
                                                    .rounding(Rounding::same(6.0))
                                                    .stroke(Stroke::new(
                                                        1.0_f32,
                                                        Color32::from_rgb(0x2c, 0x2f, 0x3d),
                                                    ))
                                                    .inner_margin(Margin::symmetric(10.0, 8.0))
                                                    .show(ui, |ui| {
                                                        ui.horizontal(|ui| {
                                                            ui.vertical(|ui| {
                                                                let btn = Button::new(
                                                                    RichText::new(&p.name)
                                                                        .size(12.5)
                                                                        .strong()
                                                                        .color(Color32::WHITE),
                                                                )
                                                                .frame(false);
                                                                if ui.add(btn).clicked() {
                                                                    action = Some(
                                                                        WelcomeAction::OpenRecentProject(
                                                                            p.path.clone(),
                                                                        ),
                                                                    );
                                                                    self.is_open = false;
                                                                }
                                                                ui.label(
                                                                    RichText::new(&p.path)
                                                                        .size(10.0)
                                                                        .color(Color32::from_rgb(
                                                                            0x7c, 0x80, 0x94,
                                                                        )),
                                                                );
                                                            });
                                                            ui.with_layout(
                                                                Layout::right_to_left(
                                                                    Align::Center,
                                                                ),
                                                                |ui| {
                                                                    if ui
                                                                        .button(
                                                                            RichText::new("✕")
                                                                                .size(11.0),
                                                                        )
                                                                        .on_hover_text(
                                                                            "Listeden kaldır",
                                                                        )
                                                                        .clicked()
                                                                    {
                                                                        to_remove = Some(idx);
                                                                    }
                                                                },
                                                            );
                                                        });
                                                    });
                                                ui.add_space(6.0);
                                            }
                                            if let Some(idx) = to_remove {
                                                self.recent_projects.remove(idx);
                                            }
                                        });
                                    }
                                });

                            ui.add_space(10.0);

                            // Bottom-Right Skip Action: "Doğrudan Boş Arayüze Geç"
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let skip_btn = Button::new(
                                    RichText::new("Doğrudan Boş Arayüze Geç →")
                                        .size(11.5)
                                        .color(Color32::from_rgb(0xb0, 0xb4, 0xc8)),
                                )
                                .rounding(Rounding::same(6.0))
                                .fill(Color32::from_rgb(0x1f, 0x21, 0x2b))
                                .stroke(Stroke::new(
                                    1.0_f32,
                                    Color32::from_rgb(0x32, 0x35, 0x44),
                                ));

                                if ui
                                    .add(skip_btn)
                                    .on_hover_text(
                                        "Başlangıç penceresini kapat ve doğrudan kurgu ekranına geç (Esc)",
                                    )
                                    .clicked()
                                {
                                    action = Some(WelcomeAction::Close);
                                    self.is_open = false;
                                }
                            });
                        });
                    });
                });
            });

        action
    }
}
