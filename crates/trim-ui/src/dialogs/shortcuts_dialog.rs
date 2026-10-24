//! Keyboard Shortcuts visual cheat sheet dialog modal (F1).

use crate::shortcuts::ShortcutProfile;
use egui::{Color32, RichText, Vec2, Window};

#[derive(Debug, Clone, Default)]
pub struct ShortcutsDialog {
    pub is_open: bool,
}

impl ShortcutsDialog {
    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }

    pub fn show(&mut self, ctx: &egui::Context, profile: &mut ShortcutProfile) {
        if !self.is_open {
            return;
        }

        Window::new("Klavye Kısayolları (triM.)")
            .collapsible(true)
            .resizable(false)
            .default_size(Vec2::new(580.0, 460.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Hızlı Kısayol Rehberi")
                            .size(14.0)
                            .strong()
                            .color(Color32::from_rgb(0xe0, 0x7a, 0x38)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Kapat (Esc)").clicked() {
                            self.is_open = false;
                        }
                    });
                });
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Aktif Kısayol Profili:").strong());
                    egui::ComboBox::from_id_salt("shortcut_dialog_profile_combo")
                        .selected_text(profile.label())
                        .show_ui(ui, |ui| {
                            for p in ShortcutProfile::ALL {
                                ui.selectable_value(profile, *p, p.label());
                            }
                        });
                    ui.label(
                        RichText::new(profile.description())
                            .size(10.5)
                            .color(Color32::from_rgb(0x90, 0x90, 0xa5)),
                    );
                });
                ui.separator();
                ui.add_space(4.0);

                let tools_shortcuts = match profile {
                    ShortcutProfile::TrimDefault => vec![
                        ("V", "Seçim Aracı (Selection Tool)"),
                        ("C", "Jilet / Kesme Aracı (Razor Tool)"),
                        ("B", "Dalgalanma Düzenleme (Ripple Edit)"),
                        ("A", "İleri İz Seçme Aracı (Track Select Forward)"),
                        ("Y", "Kaydırma Aracı (Slip Tool)"),
                        ("P", "Kalem / Anahtar Kare (Pen Tool)"),
                        ("H", "El / Gezinme Aracı (Hand Pan)"),
                        ("T", "Metin / Yazı Aracı (Type Tool)"),
                    ],
                    ShortcutProfile::PremierePro => vec![
                        ("V", "Seçim Aracı (Selection Tool)"),
                        ("C", "Jilet Aracı (Razor Tool)"),
                        ("A", "İleri İz Seçme Aracı (Track Select Forward)"),
                        ("B", "Dalgalanma Düzenleme (Ripple Edit)"),
                        ("Y", "Kaydırma Aracı (Slip Tool)"),
                        ("P", "Kalem Aracı (Pen Tool)"),
                        ("H", "El Aracı (Hand Tool)"),
                        ("T", "Metin Aracı (Type Tool)"),
                    ],
                    ShortcutProfile::DaVinciResolve => vec![
                        ("A", "Normal Düzenleme Modu (Selection Mode)"),
                        ("B", "Bıçak Modu (Blade Edit Mode)"),
                        ("T", "Kırpma Düzenleme Modu (Trim Mode)"),
                        ("P", "Kalem / Eğri Aracı (Pen Tool)"),
                        ("H", "El / Kaydırma Aracı (Hand Tool)"),
                    ],
                };

                let edit_shortcuts = match profile {
                    ShortcutProfile::TrimDefault => vec![
                        ("S", "İmleç Konumunda Klibi Böl (Split / Cut)"),
                        ("Q / W", "Dalgalanma Kırpma (Ripple Trim Head / Tail)"),
                        ("Ctrl + Z", "Son İşlemi Geri Al (Undo)"),
                        ("Ctrl + Y", "Geri Alınan İşlemi Yinele (Redo)"),
                        ("Ctrl + R", "Klip Hızı / Yönünü Değiştir (Speed)"),
                        ("Delete", "Seçili Klibi Sil"),
                        ("Shift + Del", "Boşluğu Kapatarak Sil (Ripple Delete)"),
                        ("Ctrl + A", "Tüm Klipleri Seç"),
                    ],
                    ShortcutProfile::PremierePro => vec![
                        ("Ctrl + K", "İmleç Konumunda Klibi Böl (Add Edit)"),
                        ("Q / W", "Dalgalanma Kırpma (Ripple Trim Head / Tail)"),
                        ("Ctrl + Z", "Son İşlemi Geri Al (Undo)"),
                        ("Ctrl + Shift + Z", "Yinele (Redo)"),
                        ("Ctrl + R", "Hız / Süre İletişim Kutusu (Speed / Duration)"),
                        ("Delete / Backspace", "Seçili Klibi Sil"),
                        ("Shift + Del", "Boşluğu Kapatarak Sil (Ripple Delete)"),
                        ("Ctrl + A", "Tüm Klipleri Seç"),
                    ],
                    ShortcutProfile::DaVinciResolve => vec![
                        ("Ctrl + \\", "İmleç Konumunda Klibi Böl (Split Clip)"),
                        ("Ctrl + Z", "Son İşlemi Geri Al (Undo)"),
                        ("Ctrl + Shift + Z", "Yinele (Redo)"),
                        ("Ctrl + R", "Hız Değiştirme (Change Clip Speed)"),
                        ("Backspace", "Standart Silme (Delete)"),
                        ("Shift + Del", "Boşluğu Kapatıp Sil (Ripple Delete)"),
                        ("Ctrl + A", "Tüm Klipleri Seç"),
                    ],
                };

                let sections = [
                    ("Dikey Araç Çubuğu (Tools)", tools_shortcuts),
                    (
                        "Oynatma ve Gezinme (Transport & Playback)",
                        vec![
                            ("Boşluk (Space)", "Oynat / Duraklat (Play / Pause)"),
                            ("J / K / L", "Geri Sar / Duraklat / İleri Oynat"),
                            ("Home / End", "Zaman Çizgisinin Başına / Sonuna Git"),
                            ("Sol / Sağ Ok", "1 Kare Geri / İleri Git"),
                            ("Shift + Sol / Sağ Ok", "1 Saniye Geri / İleri Git"),
                            ("I / O", "Giriş (In) / Çıkış (Out) Noktası Koy"),
                            ("M", "Zaman Çizgisine İşaretçi (Marker) Ekle"),
                        ],
                    ),
                    ("Düzenleme & Komutlar (Editing & Timeline)", edit_shortcuts),
                    (
                        "Dosya, Paneller & Çalışma Alanı",
                        vec![
                            ("Ctrl + N", "Yeni Proje Oluştur"),
                            ("Ctrl + O", "Var Olan Projeyi Aç"),
                            ("Ctrl + S", "Projeyi Kaydet"),
                            ("Ctrl + I", "Medya Dosyası İçe Aktar (Import)"),
                            ("Ctrl + M", "Videoyu Dışa Aktar (Export)"),
                            ("Ctrl + 6", "Çok Kanallı Ses Mikseri Konsolu"),
                            ("F1", "Klavye Kısayolları Haritası"),
                        ],
                    ),
                ];

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (cat, items) in sections {
                        ui.group(|ui| {
                            ui.set_width(540.0);
                            ui.strong(
                                RichText::new(cat)
                                    .color(Color32::from_rgb(0x38, 0xa0, 0xf0))
                                    .size(12.0),
                            );
                            ui.add_space(3.0);

                            for (key, desc) in items {
                                ui.horizontal(|ui| {
                                    ui.set_min_height(18.0);
                                    let key_label = RichText::new(key)
                                        .size(11.0)
                                        .strong()
                                        .color(Color32::WHITE);
                                    let key_btn = egui::Button::new(key_label)
                                        .fill(Color32::from_rgb(0x28, 0x28, 0x36))
                                        .stroke(egui::Stroke::new(
                                            1.0_f32,
                                            Color32::from_rgb(0x3c, 0x3c, 0x4c),
                                        ))
                                        .rounding(3.0);
                                    ui.add_sized([130.0, 18.0], key_btn);
                                    ui.label(
                                        RichText::new(desc)
                                            .size(11.5)
                                            .color(Color32::from_rgb(0xd0, 0xd0, 0xdc)),
                                    );
                                });
                            }
                        });
                        ui.add_space(4.0);
                    }
                });
            });
    }
}
