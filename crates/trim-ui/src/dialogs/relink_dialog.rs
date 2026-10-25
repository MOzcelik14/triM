//! Offline Media Relink Dialog Modal.

use egui::{Color32, RichText, ScrollArea, Window};
use trim_core::project::Project;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflineMediaEntry {
    pub media_id: String,
    pub name: String,
    pub file_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelinkDialogAction {
    PickFolder,
    PickSingleMedia { media_id: String },
    Refresh,
    Close,
}

#[derive(Debug, Clone, Default)]
pub struct RelinkDialog {
    pub is_open: bool,
    pub offline_items: Vec<OfflineMediaEntry>,
    pub status_text: String,
}

impl RelinkDialog {
    pub fn open(&mut self, project: &Project) {
        self.refresh(project);
        self.is_open = true;
    }

    pub fn refresh(&mut self, project: &Project) {
        let offline_ids = project.get_offline_media_ids();
        self.offline_items = offline_ids
            .into_iter()
            .filter_map(|id| {
                project.get_media(&id).map(|m| OfflineMediaEntry {
                    media_id: id,
                    name: m.name.clone(),
                    file_path: m.file_path.clone(),
                })
            })
            .collect();

        if self.offline_items.is_empty() {
            self.status_text = "Tüm medyalar bağlı, çevrimdışı dosya bulunmuyor.".to_string();
        } else {
            self.status_text = format!(
                "{} adet çevrimdışı (kayıp) medya dosyası tespit edildi.",
                self.offline_items.len()
            );
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<RelinkDialogAction> {
        if !self.is_open {
            return None;
        }

        let mut action = None;
        let mut is_open = self.is_open;

        Window::new("🔗 Çevrimdışı Medyaları Yeniden Bağla (Relink)")
            .open(&mut is_open)
            .collapsible(false)
            .resizable(true)
            .default_width(620.0)
            .default_height(380.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);

                if self.offline_items.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(
                            RichText::new("✔ Tüm Medyalar Çevrimiçi")
                                .size(16.0)
                                .color(Color32::from_rgb(0x40, 0xc0, 0x57)),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("Projede eksik veya ulaşılamayan bir medya dosyası yok.")
                                .color(Color32::GRAY),
                        );
                        ui.add_space(20.0);
                        if ui.button("Kapat").clicked() {
                            action = Some(RelinkDialogAction::Close);
                        }
                    });
                    return;
                }

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("⚠")
                            .size(16.0)
                            .color(Color32::from_rgb(0xe0, 0x38, 0x38)),
                    );
                    ui.label(
                        RichText::new(&self.status_text)
                            .strong()
                            .color(Color32::from_rgb(0xe0, 0x38, 0x38)),
                    );
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new(
                        "Dosyaların taşındığı ana klasörü seçerek hepsini tek seferde arayabilir \
                         ya da her dosyayı tek tek elle seçebilirsiniz.",
                    )
                    .size(12.0)
                    .color(Color32::GRAY),
                );

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(
                            RichText::new("📁 Klasör Seçerek Otomatik Yeniden Bağla...").strong(),
                        )
                        .clicked()
                    {
                        action = Some(RelinkDialogAction::PickFolder);
                    }
                    if ui.button("🔄 Yenile").clicked() {
                        action = Some(RelinkDialogAction::Refresh);
                    }
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);

                ui.label(RichText::new("Kayıp Medya Listesi:").strong());
                ui.add_space(4.0);

                ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    for item in &self.offline_items {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(&item.name).strong());
                                    ui.label(
                                        RichText::new(&item.file_path)
                                            .size(11.0)
                                            .color(Color32::from_rgb(0xe0, 0x70, 0x70)),
                                    );
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("Gözat...").clicked() {
                                            action = Some(RelinkDialogAction::PickSingleMedia {
                                                media_id: item.media_id.clone(),
                                            });
                                        }
                                    },
                                );
                            });
                        });
                        ui.add_space(2.0);
                    }
                });

                ui.add_space(10.0);
                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Kapat").clicked() {
                        action = Some(RelinkDialogAction::Close);
                    }
                });
            });

        if !is_open {
            self.is_open = false;
        }

        if matches!(action, Some(RelinkDialogAction::Close)) {
            self.is_open = false;
        }

        action
    }
}
