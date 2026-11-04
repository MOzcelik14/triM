//! Preferences / Settings dialog modal for triM.
//! Allows configuring language (TR/EN), editing defaults, GPU acceleration, audio limiter, and autosave.

use crate::i18n::Language;
use egui::{
    Align, Align2, Button, Color32, ComboBox, FontId, Frame, Grid, Id, Layout, Margin, RichText,
    Rounding, ScrollArea, Shadow, Slider, Stroke, Vec2, Window,
};
use trim_core::preferences::UserPreferences;

/// Active category tab inside preferences window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferencesCategory {
    General,
    Editing,
    Performance,
    Audio,
    Autosave,
}

impl PreferencesCategory {
    pub fn label(&self, lang: Language) -> &'static str {
        match (self, lang) {
            (Self::General, Language::Turkish) => "🌐 Genel & Dil",
            (Self::General, Language::English) => "🌐 General & Language",
            (Self::Editing, Language::Turkish) => "⏱️ Kurgu & Zaman Çizgisi",
            (Self::Editing, Language::English) => "⏱️ Editing & Timeline",
            (Self::Performance, Language::Turkish) => "⚡ Performans & GPU",
            (Self::Performance, Language::English) => "⚡ Performance & GPU",
            (Self::Audio, Language::Turkish) => "🔊 Ses & Monitör",
            (Self::Audio, Language::English) => "🔊 Audio & Monitoring",
            (Self::Autosave, Language::Turkish) => "💾 Otomatik Kayıt & Önbellek",
            (Self::Autosave, Language::English) => "💾 Autosave & Cache",
        }
    }
}

/// Action emitted when user confirms or closes the Preferences modal.
#[derive(Debug, Clone)]
pub enum PreferencesAction {
    Save(UserPreferences),
    Close,
}

/// State of the Preferences / Settings modal.
#[derive(Debug, Clone)]
pub struct PreferencesDialog {
    pub is_open: bool,
    pub active_category: PreferencesCategory,
    pub draft: UserPreferences,
    pub cached_cache_size: Option<u64>,
    pub status_feedback: Option<String>,
}

impl Default for PreferencesDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            active_category: PreferencesCategory::General,
            draft: UserPreferences::default(),
            cached_cache_size: None,
            status_feedback: None,
        }
    }
}

impl PreferencesDialog {
    /// Open the preferences modal with current preferences loaded into draft.
    pub fn open(&mut self, current: &UserPreferences) {
        self.draft = current.clone();
        self.is_open = true;
        self.active_category = PreferencesCategory::General;
        self.cached_cache_size = Some(UserPreferences::cache_size_bytes());
        self.status_feedback = None;
    }

    /// Close the modal.
    pub fn close(&mut self) {
        self.is_open = false;
        self.status_feedback = None;
    }

    /// Toggle visibility.
    pub fn toggle(&mut self, current: &UserPreferences) {
        if self.is_open {
            self.close();
        } else {
            self.open(current);
        }
    }

    /// Render the modal dialog. Returns `Some(PreferencesAction)` if an action was confirmed.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        active_lang: Language,
    ) -> Option<PreferencesAction> {
        if !self.is_open {
            return None;
        }

        let mut action = None;
        let mut is_open = self.is_open;

        let title = match active_lang {
            Language::Turkish => "triM. — Tercihler / Ayarlar",
            Language::English => "triM. — Preferences / Settings",
        };

        let window_frame = Frame::none()
            .fill(Color32::from_rgb(18, 20, 28))
            .rounding(Rounding::same(12.0))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(46, 52, 68)))
            .inner_margin(Margin::same(18.0))
            .shadow(Shadow {
                offset: Vec2::new(0.0, 16.0),
                blur: 36.0,
                spread: 4.0,
                color: Color32::from_black_alpha(200),
            });

        Window::new(title)
            .id(Id::new("preferences_modal"))
            .open(&mut is_open)
            .frame(window_frame)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(760.0, 500.0))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(12.0, 12.0);

                // Two-column layout: Left navigation tabs, Right settings panel
                ui.horizontal_top(|ui| {
                    // Left Categories Sidebar (width ~200px)
                    ui.allocate_ui(Vec2::new(210.0, 420.0), |ui| {
                        Frame::none()
                            .fill(Color32::from_rgb(24, 27, 37))
                            .rounding(Rounding::same(8.0))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(38, 44, 58)))
                            .inner_margin(Margin::same(8.0))
                            .show(ui, |ui| {
                                ui.set_width(210.0);
                                ui.set_height(420.0);
                                ui.vertical(|ui| {
                                    for cat in [
                                        PreferencesCategory::General,
                                        PreferencesCategory::Editing,
                                        PreferencesCategory::Performance,
                                        PreferencesCategory::Audio,
                                        PreferencesCategory::Autosave,
                                    ] {
                                        let is_sel = self.active_category == cat;
                                        let btn = Button::new(
                                            RichText::new(cat.label(active_lang))
                                                .font(FontId::proportional(13.0))
                                                .color(if is_sel {
                                                    Color32::from_rgb(70, 195, 245)
                                                } else {
                                                    Color32::from_rgb(180, 190, 205)
                                                }),
                                        )
                                        .fill(if is_sel {
                                            Color32::from_rgb(34, 42, 60)
                                        } else {
                                            Color32::TRANSPARENT
                                        })
                                        .rounding(Rounding::same(6.0))
                                        .min_size(Vec2::new(194.0, 36.0));

                                        if ui.add(btn).clicked() {
                                            self.active_category = cat;
                                        }
                                        ui.add_space(4.0);
                                    }
                                });
                            });
                    });

                    // Right Settings Body (width ~500px)
                    ui.allocate_ui(Vec2::new(510.0, 420.0), |ui| {
                        Frame::none()
                            .fill(Color32::from_rgb(22, 25, 34))
                            .rounding(Rounding::same(8.0))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(38, 44, 58)))
                            .inner_margin(Margin::same(18.0))
                            .show(ui, |ui| {
                                ui.set_width(510.0);
                                ui.set_height(420.0);

                                ScrollArea::vertical().show(ui, |ui| {
                                    ui.set_width(474.0);
                                    match self.active_category {
                                        PreferencesCategory::General => {
                                            self.render_general_tab(ui, active_lang);
                                        }
                                        PreferencesCategory::Editing => {
                                            self.render_editing_tab(ui, active_lang);
                                        }
                                        PreferencesCategory::Performance => {
                                            self.render_performance_tab(ui, active_lang);
                                        }
                                        PreferencesCategory::Audio => {
                                            self.render_audio_tab(ui, active_lang);
                                        }
                                        PreferencesCategory::Autosave => {
                                            self.render_autosave_tab(ui, active_lang);
                                        }
                                    }
                                });
                            });
                    });
                });

                ui.add_space(6.0);
                ui.separator();

                // Bottom Action Footer Bar
                ui.horizontal(|ui| {
                    if let Some(msg) = &self.status_feedback {
                        ui.label(
                            RichText::new(msg)
                                .font(FontId::proportional(11.5))
                                .color(Color32::from_rgb(100, 210, 140)),
                        );
                    }

                    // Reset button on the left
                    let reset_label = match active_lang {
                        Language::Turkish => "Varsayılanlara Sıfırla",
                        Language::English => "Reset to Defaults",
                    };
                    if ui
                        .button(RichText::new(reset_label).color(Color32::from_rgb(220, 120, 120)))
                        .clicked()
                    {
                        self.draft = UserPreferences::default();
                        self.status_feedback = Some(match active_lang {
                            Language::Turkish => "Varsayılan ayarlar yüklendi.".to_string(),
                            Language::English => "Defaults restored.".to_string(),
                        });
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let save_label = match active_lang {
                            Language::Turkish => "Kaydet ve Uygula",
                            Language::English => "Save & Apply",
                        };
                        let save_btn = Button::new(
                            RichText::new(save_label)
                                .font(FontId::proportional(12.5))
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(34, 110, 180))
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(130.0, 30.0));

                        if ui.add(save_btn).clicked() {
                            action = Some(PreferencesAction::Save(self.draft.clone()));
                            self.close();
                        }

                        let cancel_label = match active_lang {
                            Language::Turkish => "İptal",
                            Language::English => "Cancel",
                        };
                        if ui.button(cancel_label).clicked() {
                            action = Some(PreferencesAction::Close);
                            self.close();
                        }
                    });
                });
            });

        if !is_open {
            self.close();
            if action.is_none() {
                action = Some(PreferencesAction::Close);
            }
        }

        action
    }

    fn render_general_tab(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.label(
            RichText::new(match lang {
                Language::Turkish => "Genel Tercihler ve Dil",
                Language::English => "General Preferences & Language",
            })
            .font(FontId::proportional(16.0))
            .strong()
            .color(Color32::WHITE),
        );
        ui.add_space(8.0);

        Grid::new("pref_general_grid")
            .num_columns(2)
            .spacing([24.0, 14.0])
            .show(ui, |ui| {
                // Language
                ui.label(match lang {
                    Language::Turkish => "Arayüz Dili:",
                    Language::English => "Interface Language:",
                });
                let mut current_lang = Language::from_code(&self.draft.language);
                ComboBox::from_id_salt("pref_lang_combo")
                    .selected_text(current_lang.label())
                    .show_ui(ui, |ui| {
                        for l in Language::ALL {
                            if ui
                                .selectable_value(&mut current_lang, l, l.label())
                                .clicked()
                            {
                                self.draft.language = current_lang.code().to_string();
                            }
                        }
                    });
                ui.end_row();

                // UI Scale
                ui.label(match lang {
                    Language::Turkish => "Arayüz Ölçeği:",
                    Language::English => "UI Scale Factor:",
                });
                let scale_label = format!("{:.0}%", self.draft.ui_scale * 100.0);
                ComboBox::from_id_salt("pref_scale_combo")
                    .selected_text(scale_label)
                    .show_ui(ui, |ui| {
                        for (factor, name) in [
                            (0.85, "85%"),
                            (1.0, "100% (Standart)"),
                            (1.10, "110%"),
                            (1.25, "125% (Yüksek DPI)"),
                            (1.50, "150%"),
                        ] {
                            if ui
                                .selectable_label((self.draft.ui_scale - factor).abs() < 0.01, name)
                                .clicked()
                            {
                                self.draft.ui_scale = factor;
                            }
                        }
                    });
                ui.end_row();
            });

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(10.0);

        ui.label(
            RichText::new(match lang {
                Language::Turkish => "Başlangıç Davranışı",
                Language::English => "Startup Behavior",
            })
            .font(FontId::proportional(14.0))
            .strong()
            .color(Color32::from_rgb(200, 210, 225)),
        );
        ui.add_space(6.0);

        ui.checkbox(
            &mut self.draft.show_splash_screen,
            match lang {
                Language::Turkish => "Açılışta sinematik splash ekranını göster",
                Language::English => "Show cinematic startup splash screen on launch",
            },
        );

        ui.checkbox(
            &mut self.draft.show_welcome_on_startup,
            match lang {
                Language::Turkish => "Açılışta Karşılama Ekranını (Proje Merkezi) aç",
                Language::English => "Open Project Hub / Welcome dialog on startup",
            },
        );
    }

    fn render_editing_tab(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.label(
            RichText::new(match lang {
                Language::Turkish => "Kurgu ve Zaman Çizgisi Ayarları",
                Language::English => "Editing & Timeline Settings",
            })
            .font(FontId::proportional(16.0))
            .strong()
            .color(Color32::WHITE),
        );
        ui.add_space(8.0);

        Grid::new("pref_editing_grid")
            .num_columns(2)
            .spacing([24.0, 14.0])
            .show(ui, |ui| {
                // Shortcut Profile
                ui.label(match lang {
                    Language::Turkish => "Kısayol Profili:",
                    Language::English => "Shortcut Profile:",
                });
                ComboBox::from_id_salt("pref_profile_combo")
                    .selected_text(&self.draft.shortcut_profile)
                    .show_ui(ui, |ui| {
                        for p in ["triM", "Adobe Premiere Pro", "DaVinci Resolve"] {
                            ui.selectable_value(&mut self.draft.shortcut_profile, p.to_string(), p);
                        }
                    });
                ui.end_row();

                // Snapping distance
                ui.label(match lang {
                    Language::Turkish => "Manyetik Yakalama Hassasiyeti:",
                    Language::English => "Snapping Distance:",
                });
                ui.add(Slider::new(&mut self.draft.snap_distance_px, 4.0..=32.0).suffix(" px"));
                ui.end_row();

                // Default still duration
                ui.label(match lang {
                    Language::Turkish => "Varsayılan Resim/Başlık Süresi:",
                    Language::English => "Default Still/Title Duration:",
                });
                ui.add(
                    Slider::new(&mut self.draft.default_still_duration_secs, 1.0..=20.0)
                        .suffix(" sn"),
                );
                ui.end_row();

                // Default track height
                ui.label(match lang {
                    Language::Turkish => "Varsayılan Kanal Yüksekliği:",
                    Language::English => "Default Track Height:",
                });
                ui.add(
                    Slider::new(&mut self.draft.default_track_height, 40.0..=90.0).suffix(" px"),
                );
                ui.end_row();
            });

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(10.0);

        ui.checkbox(
            &mut self.draft.snap_enabled_by_default,
            match lang {
                Language::Turkish => "Manyetik yapışma (Snapping) varsayılan olarak açık olsun",
                Language::English => "Enable magnetic snapping by default",
            },
        );

        ui.checkbox(
            &mut self.draft.return_playhead_on_stop,
            match lang {
                Language::Turkish => "Oynatma durdurulduğunda oynatma kafası durduğu yerde kalsın",
                Language::English => "Keep playhead at stop position when playback halts",
            },
        );
    }

    fn render_performance_tab(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.label(
            RichText::new(match lang {
                Language::Turkish => "Performans, GPU ve Donanım Hızlandırma",
                Language::English => "Performance, GPU & Hardware Acceleration",
            })
            .font(FontId::proportional(16.0))
            .strong()
            .color(Color32::WHITE),
        );
        ui.add_space(8.0);

        let detected = trim_media::detect_available_encoders();
        let auto_desc = if detected.has_nvidia_nvenc() {
            "Otomatik (NVIDIA NVENC Aktif)"
        } else if detected.has_intel_qsv() {
            "Otomatik (Intel QuickSync Aktif)"
        } else if detected.has_vaapi() {
            "Otomatik (Linux VA-API Aktif)"
        } else if detected.has_amd_amf() {
            "Otomatik (AMD AMF Aktif)"
        } else {
            "Otomatik (Yazılımsal CPU x264/x265)"
        };

        Grid::new("pref_perf_grid")
            .num_columns(2)
            .spacing([24.0, 14.0])
            .show(ui, |ui| {
                // Hardware encoder
                ui.label(match lang {
                    Language::Turkish => "Dışa Aktarma Kodlayıcı Tercihi:",
                    Language::English => "Preferred Export Hardware Encoder:",
                });
                let cur_selected = match self.draft.preferred_hw_accel.as_str() {
                    "nvenc" => "NVIDIA NVENC",
                    "qsv" => "Intel QuickSync (QSV)",
                    "vaapi" => "Linux VA-API",
                    "amf" => "AMD AMF",
                    "cpu" => "Saf CPU (libx264 / libx265)",
                    _ => auto_desc,
                };
                ComboBox::from_id_salt("pref_hw_combo")
                    .selected_text(cur_selected)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.draft.preferred_hw_accel,
                            "auto".to_string(),
                            format!("{} (Önerilen)", auto_desc),
                        );
                        let nvenc_lbl = if detected.has_nvidia_nvenc() {
                            "NVIDIA NVENC (Destekleniyor ✅)"
                        } else {
                            "NVIDIA NVENC (Algılanmadı ❌)"
                        };
                        ui.selectable_value(
                            &mut self.draft.preferred_hw_accel,
                            "nvenc".to_string(),
                            nvenc_lbl,
                        );
                        let qsv_lbl = if detected.has_intel_qsv() {
                            "Intel QuickSync QSV (Destekleniyor ✅)"
                        } else {
                            "Intel QuickSync QSV (Algılanmadı ❌)"
                        };
                        ui.selectable_value(
                            &mut self.draft.preferred_hw_accel,
                            "qsv".to_string(),
                            qsv_lbl,
                        );
                        let vaapi_lbl = if detected.has_vaapi() {
                            "Linux VA-API (Destekleniyor ✅)"
                        } else {
                            "Linux VA-API (Algılanmadı ❌)"
                        };
                        ui.selectable_value(
                            &mut self.draft.preferred_hw_accel,
                            "vaapi".to_string(),
                            vaapi_lbl,
                        );
                        let amf_lbl = if detected.has_amd_amf() {
                            "AMD AMF (Destekleniyor ✅)"
                        } else {
                            "AMD AMF (Algılanmadı ❌)"
                        };
                        ui.selectable_value(
                            &mut self.draft.preferred_hw_accel,
                            "amf".to_string(),
                            amf_lbl,
                        );
                        ui.selectable_value(
                            &mut self.draft.preferred_hw_accel,
                            "cpu".to_string(),
                            "Saf CPU - libx264/libx265 (Her Zaman Kullanılabilir ✅)",
                        );
                    });
                ui.end_row();
            });

        ui.add_space(8.0);

        // Hardware Status Card
        Frame::none()
            .fill(Color32::from_rgb(0x18, 0x18, 0x22))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x2d, 0x2d, 0x3d)))
            .rounding(Rounding::same(6.0))
            .inner_margin(Margin::same(10.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(match lang {
                        Language::Turkish => "🔍 Sistem Donanım Hızlandırma Durumu",
                        Language::English => "🔍 System Hardware Acceleration Status",
                    })
                    .size(12.0)
                    .strong()
                    .color(Color32::from_rgb(0x90, 0xb0, 0xe0)),
                );
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    let render_badge = |ui: &mut egui::Ui, name: &str, active: bool| {
                        let (icon, color) = if active {
                            ("✅", Color32::from_rgb(0x60, 0xd0, 0x80))
                        } else {
                            ("❌", Color32::from_rgb(0xa0, 0x80, 0x80))
                        };
                        ui.colored_label(
                            color,
                            RichText::new(format!("{} {}", icon, name)).size(11.0),
                        );
                        ui.add_space(8.0);
                    };
                    render_badge(ui, "NVIDIA NVENC", detected.has_nvidia_nvenc());
                    render_badge(ui, "Intel QuickSync", detected.has_intel_qsv());
                    render_badge(ui, "Linux VA-API", detected.has_vaapi());
                    render_badge(ui, "AMD AMF", detected.has_amd_amf());
                    render_badge(ui, "CPU Fallback (x264)", true);
                });
            });

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(10.0);

        ui.label(
            RichText::new(match lang {
                Language::Turkish => "Akıllı Proxy (Smart Proxy)",
                Language::English => "Smart Proxy Workflow",
            })
            .font(FontId::proportional(14.0))
            .strong()
            .color(Color32::from_rgb(200, 210, 225)),
        );
        ui.add_space(6.0);

        ui.checkbox(
            &mut self.draft.use_proxies_by_default,
            match lang {
                Language::Turkish => {
                    "720p H.264 Akıllı Proxy önizlemeyi varsayılan olarak açık başlat"
                }
                Language::English => "Enable 720p H.264 Smart Proxy preview by default",
            },
        );

        ui.checkbox(
            &mut self.draft.auto_generate_proxies,
            match lang {
                Language::Turkish => "4K ve üzeri yüksek çözünürlüklü medyalar içe aktarıldığında otomatik proxy oluştur",
                Language::English => "Automatically generate proxies when importing 4K+ high-res media",
            },
        );
    }

    fn render_audio_tab(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.label(
            RichText::new(match lang {
                Language::Turkish => "Ses Alt Yapısı ve Master Bus",
                Language::English => "Audio Engine & Master Bus",
            })
            .font(FontId::proportional(16.0))
            .strong()
            .color(Color32::WHITE),
        );
        ui.add_space(8.0);

        Grid::new("pref_audio_grid")
            .num_columns(2)
            .spacing([24.0, 14.0])
            .show(ui, |ui| {
                ui.label(match lang {
                    Language::Turkish => "Sınırlayıcı Tavanı (Ceiling):",
                    Language::English => "Limiter Ceiling Level:",
                });
                ui.add(Slider::new(&mut self.draft.limiter_ceiling_db, -3.0..=0.0).suffix(" dBFS"));
                ui.end_row();
            });

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(10.0);

        ui.checkbox(
            &mut self.draft.master_limiter_enabled,
            match lang {
                Language::Turkish => {
                    "Master Bus Lookahead Brickwall Peak Limiter (Kırpılma Koruması)"
                }
                Language::English => {
                    "Master Bus Lookahead Brickwall Peak Limiter (Clipping Protection)"
                }
            },
        );

        ui.checkbox(
            &mut self.draft.show_waveforms,
            match lang {
                Language::Turkish => {
                    "Zaman çizelgesi ses kliplerinde gerçek zamanlı dalga formlarını göster"
                }
                Language::English => "Display real-time audio waveforms on timeline clips",
            },
        );
    }

    fn render_autosave_tab(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.label(
            RichText::new(match lang {
                Language::Turkish => "Otomatik Kayıt ve Önbellek Yönetimi",
                Language::English => "Autosave & Cache Management",
            })
            .font(FontId::proportional(16.0))
            .strong()
            .color(Color32::WHITE),
        );
        ui.add_space(8.0);

        Grid::new("pref_autosave_grid")
            .num_columns(2)
            .spacing([24.0, 14.0])
            .show(ui, |ui| {
                ui.label(match lang {
                    Language::Turkish => "Otomatik Kayıt Aralığı:",
                    Language::English => "Autosave Interval:",
                });
                let cur_label = match self.draft.autosave_interval_secs {
                    0 => match lang {
                        Language::Turkish => "Devre Dışı",
                        Language::English => "Disabled",
                    },
                    30 => "30 sn / 30 sec",
                    60 => "1 dk / 1 min (Varsayılan)",
                    120 => "2 dk / 2 min",
                    300 => "5 dk / 5 min",
                    _other => "Özel / Custom",
                };

                ComboBox::from_id_salt("pref_autosave_combo")
                    .selected_text(cur_label)
                    .show_ui(ui, |ui| {
                        for (sec, txt) in [
                            (
                                0,
                                if lang == Language::Turkish {
                                    "Devre Dışı"
                                } else {
                                    "Disabled"
                                },
                            ),
                            (30, "30 sn"),
                            (60, "1 dk (Önerilen)"),
                            (120, "2 dk"),
                            (300, "5 dk"),
                        ] {
                            ui.selectable_value(&mut self.draft.autosave_interval_secs, sec, txt);
                        }
                    });
                ui.end_row();

                // Memory / RAM Cache Budget
                ui.label(match lang {
                    Language::Turkish => "Maksimum RAM Önbelleği (Bütçe):",
                    Language::English => "Maximum RAM Cache Budget:",
                });
                let mem_cur = match self.draft.max_memory_budget_mb {
                    256 => "256 MB (Düşük RAM / Low)",
                    1024 => "1024 MB (Yüksek / High)",
                    2048 => "2048 MB (Maksimum / Ultra)",
                    _ => "512 MB (Dengeli / Balanced)",
                };
                ComboBox::from_id_salt("pref_memory_combo")
                    .selected_text(mem_cur)
                    .show_ui(ui, |ui| {
                        for (mb, label) in [
                            (
                                256,
                                if lang == Language::Turkish {
                                    "256 MB (Düşük RAM / 4-8 GB sistemler)"
                                } else {
                                    "256 MB (Low RAM / 4-8 GB systems)"
                                },
                            ),
                            (
                                512,
                                if lang == Language::Turkish {
                                    "512 MB (Dengeli / Varsayılan Önerilen)"
                                } else {
                                    "512 MB (Balanced / Default Recommended)"
                                },
                            ),
                            (
                                1024,
                                if lang == Language::Turkish {
                                    "1024 MB (Yüksek / Akıcı 4K Kurgu)"
                                } else {
                                    "1024 MB (High / Smooth 4K Editing)"
                                },
                            ),
                            (
                                2048,
                                if lang == Language::Turkish {
                                    "2048 MB (Maksimum / 32+ GB İş İstasyonu)"
                                } else {
                                    "2048 MB (Maximum / 32+ GB Workstations)"
                                },
                            ),
                        ] {
                            ui.selectable_value(&mut self.draft.max_memory_budget_mb, mb, label);
                        }
                    });
                ui.end_row();

                // Cache size display
                ui.label(match lang {
                    Language::Turkish => "Mevcut Disk Önbelleği:",
                    Language::English => "Current Disk Cache Size:",
                });
                let bytes = self.cached_cache_size.unwrap_or(0);
                let mb = bytes as f64 / (1024.0 * 1024.0);
                ui.label(format!("{:.2} MB", mb));
                ui.end_row();
            });

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(10.0);

        let clear_label = match lang {
            Language::Turkish => "🗑️ Küçük Resim ve Dalgaboyu Önbelleğini Temizle",
            Language::English => "🗑️ Clear Thumbnail & Waveform Disk Cache",
        };
        if ui.button(clear_label).clicked() {
            if let Ok(()) = UserPreferences::clear_cache() {
                self.cached_cache_size = Some(UserPreferences::cache_size_bytes());
                self.status_feedback = Some(match lang {
                    Language::Turkish => "Önbellek başarıyla temizlendi.".to_string(),
                    Language::English => "Disk cache cleared successfully.".to_string(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preferences_dialog_open_close() {
        let mut dialog = PreferencesDialog::default();
        let prefs = UserPreferences::default();
        assert!(!dialog.is_open);
        dialog.open(&prefs);
        assert!(dialog.is_open);
        assert_eq!(dialog.draft.language, "tr");
        dialog.close();
        assert!(!dialog.is_open);
    }
}
