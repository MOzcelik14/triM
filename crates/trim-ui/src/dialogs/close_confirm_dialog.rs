//! Unsaved changes confirmation dialog for safe exit and project transitions.

use std::path::PathBuf;
use egui::{Button, Color32, Key, RichText, Window};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingCloseAction {
    ExitApp,
    NewProject,
    OpenProject,
    OpenPath(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseConfirmAction {
    SaveAndProceed(PendingCloseAction),
    DiscardAndProceed(PendingCloseAction),
    Cancel,
}

#[derive(Debug, Clone, Default)]
pub struct CloseConfirmDialog {
    pub is_open: bool,
    pub pending_action: Option<PendingCloseAction>,
    pub project_name: String,
}

impl CloseConfirmDialog {
    pub fn prompt(&mut self, action: PendingCloseAction, project_name: impl Into<String>) {
        self.pending_action = Some(action);
        self.project_name = project_name.into();
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.pending_action = None;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<CloseConfirmAction> {
        if !self.is_open || self.pending_action.is_none() {
            return None;
        }

        let pending = self.pending_action.as_ref().unwrap().clone();
        let mut result = None;

        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.close();
            return Some(CloseConfirmAction::Cancel);
        }

        let title = match &pending {
            PendingCloseAction::ExitApp => "Çıkmadan Önce Kaydet",
            PendingCloseAction::NewProject => "Yeni Projeye Geçmeden Önce Kaydet",
            PendingCloseAction::OpenProject | PendingCloseAction::OpenPath(_) => "Farklı Proje Açmadan Önce Kaydet",
        };

        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new("close_confirm_backdrop")));
        painter.rect_filled(ctx.screen_rect(), 0.0, Color32::from_black_alpha(160));

        Window::new(title)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚠️").size(24.0).color(Color32::from_rgb(0xe0, 0x7a, 0x38)));
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!("\"{}\" projesinde kaydedilmemiş değişiklikler var.", self.project_name))
                                .strong()
                                .color(Color32::WHITE),
                        );
                        let subtext = match &pending {
                            PendingCloseAction::ExitApp => "Uygulamadan çıkmadan önce değişiklikleri kaydetmek istiyor musunuz?",
                            PendingCloseAction::NewProject => "Yeni projeye geçmeden önce mevcut değişiklikleri kaydetmek istiyor musunuz?",
                            PendingCloseAction::OpenProject | PendingCloseAction::OpenPath(_) => "Başka bir proje açmadan önce mevcut değişiklikleri kaydetmek istiyor musunuz?",
                        };
                        ui.label(RichText::new(subtext).size(12.0).color(Color32::from_rgb(0xa0, 0xa0, 0xb0)));
                    });
                });

                ui.add_space(14.0);

                ui.horizontal(|ui| {
                    let save_btn = Button::new(RichText::new("💾 Kaydet").color(Color32::WHITE).strong())
                        .fill(Color32::from_rgb(0xe0, 0x7a, 0x38));

                    if ui.add(save_btn).clicked() {
                        result = Some(CloseConfirmAction::SaveAndProceed(pending.clone()));
                        self.is_open = false;
                        self.pending_action = None;
                    }

                    if ui.button(RichText::new("Kaydetmeden Devam Et").color(Color32::from_rgb(0xdc, 0x5a, 0x5a))).clicked() {
                        result = Some(CloseConfirmAction::DiscardAndProceed(pending));
                        self.is_open = false;
                        self.pending_action = None;
                    }

                    if ui.button("İptal").clicked() {
                        result = Some(CloseConfirmAction::Cancel);
                        self.is_open = false;
                        self.pending_action = None;
                    }
                });
            });

        result
    }
}

