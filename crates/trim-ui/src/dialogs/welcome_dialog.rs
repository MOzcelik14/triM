//! Modern Welcome / Project Launcher modal with triM. branding, New Project creator, and Recent Projects.

use egui::{Button, Color32, RichText, Vec2, Window};

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

impl Default for WelcomeDialog {
    fn default() -> Self {
        // Load recent projects or provide standard defaults
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
            is_open: true, // Opened by default on cold launch
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

        let mut action = None;

        Window::new("triM. - Başlangıç")
            .collapsible(false)
            .resizable(false)
            .default_size(Vec2::new(640.0, 420.0))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left Column: Branding and Project Creation (Width: ~280px)
                    ui.allocate_ui(Vec2::new(280.0, 390.0), |ui| {
                        ui.vertical(|ui| {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("triM.")
                                        .size(28.0)
                                        .strong()
                                        .color(Color32::from_rgb(0xe0, 0x7a, 0x38)),
                                );
                                ui.label(
                                    RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                                        .size(11.0)
                                        .color(Color32::from_rgb(0x80, 0x80, 0x90)),
                                );
                            });
                            ui.label(
                                RichText::new("Modern ve Hızlı Video Düzenleyici")
                                    .size(11.0)
                                    .color(Color32::from_rgb(0xa0, 0xa0, 0xb4)),
                            );

                            ui.add_space(14.0);
                            ui.separator();
                            ui.add_space(8.0);

                            ui.strong("Yeni Proje Oluştur");
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label("Proje Adı:");
                                ui.text_edit_singleline(&mut self.new_project_name);
                            });

                            ui.add_space(4.0);
                            ui.label("Format & Çözünürlük:");
                            let formats = [
                                ("1080p Standart (1920x1080 @ 30fps)", 1920, 1080, 30.0),
                                ("4K Sinematik (3840x2160 @ 60fps)", 3840, 2160, 60.0),
                                ("9:16 Dikey Shorts/Reels (1080x1920)", 1080, 1920, 30.0),
                                ("720p Hızlı Web (1280x720 @ 30fps)", 1280, 720, 30.0),
                            ];

                            for (idx, (label, _, _, _)) in formats.iter().enumerate() {
                                let is_sel = self.selected_preset_idx == idx;
                                let text_color = if is_sel {
                                    Color32::WHITE
                                } else {
                                    Color32::from_rgb(0xc0, 0xc0, 0xd0)
                                };
                                let btn = egui::SelectableLabel::new(
                                    is_sel,
                                    RichText::new(*label).size(11.5).color(text_color),
                                );
                                if ui.add_sized([260.0, 24.0], btn).clicked() {
                                    self.selected_preset_idx = idx;
                                }
                                ui.add_space(2.0);
                            }

                            ui.add_space(12.0);

                            let (_, w, h, fps) = formats[self.selected_preset_idx];
                            let new_btn = Button::new(
                                RichText::new("+ Projeyi Başlat")
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .min_size(Vec2::new(260.0, 32.0))
                            .fill(Color32::from_rgb(0xe0, 0x7a, 0x38));

                            if ui.add(new_btn).clicked() {
                                action = Some(WelcomeAction::CreateNewProject {
                                    name: self.new_project_name.clone(),
                                    width: w,
                                    height: h,
                                    fps,
                                });
                                self.is_open = false;
                            }

                            ui.add_space(6.0);
                            let open_btn = Button::new(
                                RichText::new("Var Olan Projeyi Aç...")
                                    .size(12.0)
                                    .color(Color32::WHITE),
                            )
                            .min_size(Vec2::new(260.0, 28.0))
                            .fill(Color32::from_rgb(0x28, 0x5a, 0x9e));

                            if ui.add(open_btn).clicked() {
                                action = Some(WelcomeAction::OpenProjectRequested);
                                self.is_open = false;
                            }
                        });
                    });

                    ui.separator();

                    // Right Column: Recent Projects List (Width: ~320px)
                    ui.allocate_ui(Vec2::new(320.0, 390.0), |ui| {
                        ui.vertical(|ui| {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.strong("Son Kullanılan Projeler");
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .button("Doğrudan Başla")
                                            .on_hover_text(
                                                "Karşılama ekranını kapat ve boş projeye geç",
                                            )
                                            .clicked()
                                        {
                                            action = Some(WelcomeAction::Close);
                                            self.is_open = false;
                                        }
                                    },
                                );
                            });
                            ui.add_space(4.0);

                            if self.recent_projects.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(40.0);
                                    ui.label(
                                        RichText::new("Henüz son kullanılan proje yok")
                                            .color(Color32::from_rgb(0x70, 0x70, 0x80))
                                            .size(11.5),
                                    );
                                });
                            } else {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    let mut to_remove = None;
                                    for (idx, p) in self.recent_projects.iter().enumerate() {
                                        ui.group(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.vertical(|ui| {
                                                    let btn = Button::new(
                                                        RichText::new(&p.name)
                                                            .size(12.5)
                                                            .strong()
                                                            .color(Color32::WHITE),
                                                    )
                                                    .fill(Color32::TRANSPARENT);
                                                    if ui.add(btn).clicked() {
                                                        action =
                                                            Some(WelcomeAction::OpenRecentProject(
                                                                p.path.clone(),
                                                            ));
                                                        self.is_open = false;
                                                    }
                                                    ui.label(
                                                        RichText::new(&p.path).size(9.5).color(
                                                            Color32::from_rgb(0x80, 0x80, 0x95),
                                                        ),
                                                    );
                                                });
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        if ui
                                                            .button("×")
                                                            .on_hover_text("Listeden kaldır")
                                                            .clicked()
                                                        {
                                                            to_remove = Some(idx);
                                                        }
                                                    },
                                                );
                                            });
                                        });
                                        ui.add_space(2.0);
                                    }
                                    if let Some(idx) = to_remove {
                                        self.recent_projects.remove(idx);
                                    }
                                });
                            }
                        });
                    });
                });
            });

        action
    }
}
