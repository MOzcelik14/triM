//! Keyboard Shortcuts visual cheat sheet dialog modal (F1).

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

    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.is_open {
            return;
        }

        Window::new("Klavye Kısayolları (triM.)")
            .collapsible(true)
            .resizable(false)
            .default_size(Vec2::new(560.0, 420.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Hızlı Kısayol Rehberi").size(14.0).strong().color(Color32::from_rgb(0xe0, 0x7a, 0x38)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Kapat (Esc)").clicked() {
                            self.is_open = false;
                        }
                    });
                });
                ui.separator();
                ui.add_space(4.0);

                let sections = [
                    (
                        "Dikey Araç Çubuğu (Tools)",
                        vec![
                            ("V", "Seçim Aracı (Selection Tool)"),
                            ("A", "İleri İz Seçme Aracı (Track Select Forward)"),
                            ("B", "Dalgalanma Düzenleme (Ripple Edit)"),
                            ("C", "Jilet / Kesme Aracı (Razor Tool)"),
                            ("Y", "Kaydırma Aracı (Slip Tool)"),
                            ("P", "Kalem / Anahtar Kare (Pen Tool)"),
                            ("H", "El / Gezinme Aracı (Hand Pan)"),
                            ("T", "Metin / Yazı Aracı (Type Tool)"),
                        ],
                    ),
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
                    (
                        "Düzenleme & Komutlar (Editing & Timeline)",
                        vec![
                            ("Ctrl + Z", "Son İşlemi Geri Al (Undo)"),
                            ("Ctrl + Y", "Geri Alınan İşlemi Yinele (Redo)"),
                            ("Ctrl + K", "İmlecin Bulunduğu Klibi Böl (Split / Cut)"),
                            ("Ctrl + R", "Klip Hızı / Yönünü Değiştir (Speed / Duration)"),
                            ("Shift + Del", "Boşluğu Kapatarak Sil (Ripple Delete)"),
                            ("Ctrl + A", "Zaman Çizgisindeki Tüm Klipleri Seç"),
                            ("Ctrl + D", "Seçimi Kaldır (Deselect All)"),
                        ],
                    ),
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
                            ui.set_width(520.0);
                            ui.strong(RichText::new(cat).color(Color32::from_rgb(0x38, 0xa0, 0xf0)).size(12.0));
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
                                        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(0x3c, 0x3c, 0x4c)))
                                        .rounding(3.0);
                                    ui.add_sized([130.0, 18.0], key_btn);

                                    ui.add_space(6.0);
                                    ui.label(RichText::new(desc).size(11.0).color(Color32::from_rgb(0xb0, 0xb0, 0xc0)));
                                });
                            }
                        });
                        ui.add_space(4.0);
                    }
                });
            });
    }
}
