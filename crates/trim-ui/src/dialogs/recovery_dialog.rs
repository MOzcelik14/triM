//! Crash recovery modal dialog.

use egui::{Button, Color32, RichText, Window};
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct RecoveryDialog {
    pub is_open: bool,
    pub recovery_path: Option<PathBuf>,
}

pub enum RecoveryAction {
    Restore(PathBuf),
    Discard(PathBuf),
}

impl RecoveryDialog {
    pub fn prompt(&mut self, recovery_file: PathBuf) {
        self.recovery_path = Some(recovery_file);
        self.is_open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<RecoveryAction> {
        if !self.is_open || self.recovery_path.is_none() {
            return None;
        }

        let mut action = None;
        let path = self.recovery_path.clone().unwrap();

        Window::new("Çökme Kurtarma")
            .collapsible(false)
            .resizable(false)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);

                ui.label(
                    RichText::new("Önceki oturumdan kaydedilmemiş otomatik bir kurtarma dosyası bulundu.")
                        .color(Color32::from_rgb(0xe0, 0x7a, 0x38))
                        .strong(),
                );

                ui.add_space(4.0);
                ui.label("Projenizi son kaydedilen durumuna geri yüklemek istiyor musunuz?");

                ui.add_space(14.0);

                ui.horizontal(|ui| {
                    let btn_restore = Button::new(RichText::new("Geri Yükle").color(Color32::WHITE).strong())
                        .fill(Color32::from_rgb(0xe0, 0x7a, 0x38));

                    if ui.add(btn_restore).clicked() {
                        action = Some(RecoveryAction::Restore(path.clone()));
                        self.is_open = false;
                    }

                    if ui.button("Yok Say ve Sil").clicked() {
                        action = Some(RecoveryAction::Discard(path.clone()));
                        self.is_open = false;
                    }
                });
            });

        action
    }
}
