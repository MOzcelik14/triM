//! Keyboard Shortcuts visual cheat sheet dialog modal (F1).

use crate::shortcuts::{KeyBinding, KeymapConfig, ShortcutAction, ShortcutProfile};
use egui::{Color32, Key, RichText, Vec2, Window};

#[derive(Debug, Clone, Default)]
pub struct ShortcutsDialog {
    pub is_open: bool,
    pub capturing_action: Option<ShortcutAction>,
    pub status_hint: Option<String>,
    pub lang: crate::i18n::Language,
}

fn save_keymap(keymap: &KeymapConfig) {
    if let Err(e) = keymap.save_to_path(&KeymapConfig::config_path()) {
        log::warn!("Failed to save keybindings: {}", e);
    }
}

fn egui_key_to_name(key: Key) -> Option<&'static str> {
    match key {
        Key::Space => Some("Space"),
        Key::Delete => Some("Delete"),
        Key::Backspace => Some("Backspace"),
        Key::OpenBracket => Some("OpenBracket"),
        Key::CloseBracket => Some("CloseBracket"),
        Key::Backslash => Some("Backslash"),
        Key::Comma => Some("Comma"),
        Key::Period => Some("Period"),
        Key::Minus => Some("Minus"),
        Key::Equals => Some("Equals"),
        Key::Plus => Some("Plus"),
        Key::Enter => Some("Enter"),
        Key::Tab => Some("Tab"),
        Key::Home => Some("Home"),
        Key::End => Some("End"),
        Key::ArrowLeft => Some("ArrowLeft"),
        Key::ArrowRight => Some("ArrowRight"),
        Key::ArrowUp => Some("ArrowUp"),
        Key::ArrowDown => Some("ArrowDown"),
        Key::A => Some("A"),
        Key::B => Some("B"),
        Key::C => Some("C"),
        Key::D => Some("D"),
        Key::E => Some("E"),
        Key::F => Some("F"),
        Key::G => Some("G"),
        Key::H => Some("H"),
        Key::I => Some("I"),
        Key::J => Some("J"),
        Key::K => Some("K"),
        Key::L => Some("L"),
        Key::M => Some("M"),
        Key::N => Some("N"),
        Key::O => Some("O"),
        Key::P => Some("P"),
        Key::Q => Some("Q"),
        Key::R => Some("R"),
        Key::S => Some("S"),
        Key::T => Some("T"),
        Key::U => Some("U"),
        Key::V => Some("V"),
        Key::W => Some("W"),
        Key::X => Some("X"),
        Key::Y => Some("Y"),
        Key::Z => Some("Z"),
        Key::Num0 => Some("0"),
        Key::Num1 => Some("1"),
        Key::Num2 => Some("2"),
        Key::Num3 => Some("3"),
        Key::Num4 => Some("4"),
        Key::Num5 => Some("5"),
        Key::Num6 => Some("6"),
        Key::Num7 => Some("7"),
        Key::Num8 => Some("8"),
        Key::Num9 => Some("9"),
        _ => None,
    }
}

impl ShortcutsDialog {
    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
        if !self.is_open {
            self.capturing_action = None;
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        profile: &mut ShortcutProfile,
        keymap: &mut KeymapConfig,
    ) {
        if !self.is_open {
            self.capturing_action = None;
            return;
        }

        if let Some(action) = self.capturing_action {
            let captured = ctx.input(|i| {
                if i.key_pressed(Key::Escape) {
                    return Some(Err(()));
                }
                for key in &i.keys_down {
                    if i.key_pressed(*key) {
                        if let Some(name) = egui_key_to_name(*key) {
                            return Some(Ok(KeyBinding {
                                ctrl: i.modifiers.ctrl || i.modifiers.command,
                                shift: i.modifiers.shift,
                                alt: i.modifiers.alt,
                                key: name.to_string(),
                            }));
                        }
                    }
                }
                None
            });
            match captured {
                Some(Err(())) => {
                    self.capturing_action = None;
                    self.status_hint = Some("Tuş atama iptal edildi.".to_string());
                }
                Some(Ok(binding)) => {
                    let display = binding.display();
                    let action_label = self.lang.t(action.i18n_key());
                    let conflicts = keymap.rebind(action, binding, true);
                    save_keymap(keymap);
                    self.capturing_action = None;
                    self.status_hint = Some(if conflicts.is_empty() {
                        match self.lang {
                            crate::i18n::Language::Turkish => {
                                format!("{} -> {} olarak kaydedildi.", action_label, display)
                            }
                            crate::i18n::Language::English => {
                                format!("{} -> {} saved.", action_label, display)
                            }
                        }
                    } else {
                        match self.lang {
                            crate::i18n::Language::Turkish => format!(
                                "{} -> {} atandı (çakışan eski atama kaldırıldı).",
                                action_label, display
                            ),
                            crate::i18n::Language::English => format!(
                                "{} -> {} assigned (previous conflicting binding removed).",
                                action_label, display
                            ),
                        }
                    });
                }
                None => {}
            }
        }

        Window::new("Klavye Kısayolları (triM.)")
            .collapsible(true)
            .resizable(false)
            .default_size(Vec2::new(600.0, 500.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Hızlı Kısayol Rehberi ve Editörü")
                            .size(14.0)
                            .strong()
                            .color(Color32::from_rgb(0xe0, 0x7a, 0x38)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Kapat (Esc)").clicked() {
                            self.is_open = false;
                            self.capturing_action = None;
                        }
                    });
                });
                ui.separator();

                let prev_profile = *profile;
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Aktif Kısayol Profili:").strong());
                    egui::ComboBox::from_id_salt("shortcut_dialog_profile_combo")
                        .selected_text(profile.label())
                        .show_ui(ui, |ui| {
                            for p in ShortcutProfile::ALL {
                                ui.selectable_value(profile, *p, p.label());
                            }
                        });
                    if ui.button("Varsayılana Sıfırla").clicked() {
                        *keymap = KeymapConfig::for_profile(*profile);
                        save_keymap(keymap);
                        self.capturing_action = None;
                        self.status_hint = Some(format!(
                            "{} profilinin varsayılanları geri yüklendi.",
                            profile.label()
                        ));
                    }
                });
                if *profile != prev_profile {
                    *keymap = KeymapConfig::for_profile(*profile);
                    save_keymap(keymap);
                    self.capturing_action = None;
                    self.status_hint =
                        Some(format!("Profil {} olarak güncellendi.", profile.label()));
                }
                ui.label(
                    RichText::new(profile.description())
                        .size(10.5)
                        .color(Color32::from_rgb(0x90, 0x90, 0xa5)),
                );
                if let Some(hint) = &self.status_hint {
                    ui.label(
                        RichText::new(hint)
                            .size(11.0)
                            .color(Color32::from_rgb(0x74, 0xc6, 0x9d)),
                    );
                }
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
                    ShortcutProfile::FinalCutPro => vec![
                        ("A", "Seçim Aracı (Select)"),
                        ("B", "Bıçak Aracı (Blade)"),
                        ("T", "Kırpma Aracı (Trim)"),
                        ("P", "Konum Aracı (Position)"),
                        ("H", "El Aracı (Hand)"),
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
                    ShortcutProfile::FinalCutPro => vec![
                        ("Ctrl + B", "İmleç Konumunda Klibi Böl (Blade)"),
                        ("[ / ]", "Baş / Son Kırpma (Trim Start / End)"),
                        ("Ctrl + Z", "Son İşlemi Geri Al (Undo)"),
                        ("Ctrl + Shift + Z", "Yinele (Redo)"),
                        ("Backspace", "Standart Silme (Delete)"),
                        ("Shift + Del", "Boşluğu Kapatıp Sil (Ripple Delete)"),
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
                    ui.group(|ui| {
                        ui.set_width(560.0);
                        ui.strong(
                            RichText::new("Özelleştirilebilir Kısayollar (Tıklayıp Tuş Atayın)")
                                .color(Color32::from_rgb(0xe0, 0x7a, 0x38))
                                .size(12.0),
                        );
                        ui.add_space(3.0);
                        for &action in ShortcutAction::ALL {
                            ui.horizontal(|ui| {
                                ui.set_min_height(20.0);
                                let is_capturing = self.capturing_action == Some(action);
                                let btn_text = if is_capturing {
                                    "Tuşa basın... (Esc)".to_string()
                                } else {
                                    keymap
                                        .bindings
                                        .get(&action)
                                        .map(|b| b.display())
                                        .unwrap_or_else(|| "Atanmamış".to_string())
                                };
                                let fill = if is_capturing {
                                    Color32::from_rgb(0x7a, 0x3e, 0x18)
                                } else {
                                    Color32::from_rgb(0x28, 0x28, 0x36)
                                };
                                let key_btn = egui::Button::new(
                                    RichText::new(btn_text)
                                        .size(11.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(fill)
                                .stroke(egui::Stroke::new(
                                    1.0_f32,
                                    Color32::from_rgb(0x3c, 0x3c, 0x4c),
                                ))
                                .rounding(3.0);
                                if ui.add_sized([150.0, 20.0], key_btn).clicked() {
                                    self.capturing_action =
                                        if is_capturing { None } else { Some(action) };
                                }
                                ui.label(
                                    RichText::new(self.lang.t(action.i18n_key()))
                                        .size(11.5)
                                        .color(Color32::from_rgb(0xd0, 0xd0, 0xdc)),
                                );
                            });
                        }
                    });
                    ui.add_space(4.0);

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
