//! Modern NLE Export Video Studio dialog modal with presets, custom resolution/codecs, and progress.

use egui::{Button, Color32, ProgressBar, RichText, Slider, Vec2, Window};
use trim_core::project::Project;
use trim_export::presets::{get_default_presets, ExportPreset, HardwareAccel, LoudnessNorm};
use trim_media::detect_available_encoders;

#[derive(Debug, Clone)]
pub struct ExportDialog {
    pub is_open: bool,
    pub presets: Vec<ExportPreset>,
    pub selected_preset_idx: usize,
    pub output_path: String,
    pub is_exporting: bool,
    pub progress: f32, // 0.0 .. 100.0
    pub status_text: String,
    pub custom_width: u32,
    pub custom_height: u32,
    pub custom_fps: f64,
    pub custom_crf: u32,
    pub hw_accel: HardwareAccel,
    pub loudness_norm: LoudnessNorm,
    pub timeline_duration: f64,
}

impl Default for ExportDialog {
    fn default() -> Self {
        let home = trim_core::paths::home_dir();
        let default_out = if home.join("Videos").is_dir() {
            home.join("Videos")
                .join("cikti.mp4")
                .to_string_lossy()
                .to_string()
        } else if home.join("Masaüstü").is_dir() {
            home.join("Masaüstü")
                .join("cikti.mp4")
                .to_string_lossy()
                .to_string()
        } else if home.join("Desktop").is_dir() {
            home.join("Desktop")
                .join("cikti.mp4")
                .to_string_lossy()
                .to_string()
        } else {
            home.join("cikti.mp4").to_string_lossy().to_string()
        };

        let presets = get_default_presets();
        let first_w = presets[0].width.unwrap_or(1920);
        let first_h = presets[0].height.unwrap_or(1080);
        let first_fps = presets[0].fps.unwrap_or(30.0);
        let first_crf = presets[0].crf;
        let first_norm = presets[0].loudness_norm;

        Self {
            is_open: false,
            presets,
            selected_preset_idx: 0,
            output_path: default_out,
            is_exporting: false,
            progress: 0.0,
            status_text: "Hazır".to_string(),
            custom_width: first_w,
            custom_height: first_h,
            custom_fps: first_fps,
            custom_crf: first_crf,
            hw_accel: HardwareAccel::Auto,
            loudness_norm: first_norm,
            timeline_duration: 10.0,
        }
    }
}

pub enum ExportDialogAction {
    StartExport(ExportPreset, String),
    CancelExport,
    OpenOutputFolder(String),
}

impl ExportDialog {
    pub fn open(&mut self, project: &Project) {
        self.custom_width = project.timeline.width;
        self.custom_height = project.timeline.height;
        self.custom_fps = project.timeline.fps;
        self.open_with_duration(project.timeline.duration());
    }

    pub fn open_with_duration(&mut self, duration: f64) {
        self.is_open = true;
        self.timeline_duration = duration.max(0.1);
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<ExportDialogAction> {
        if !self.is_open {
            return None;
        }

        let mut action = None;

        Window::new("Videoyu Dışa Aktar")
            .collapsible(false)
            .resizable(false)
            .default_size(Vec2::new(620.0, 460.0))
            .show(ctx, |ui| {
                if self.is_exporting {
                    ctx.request_repaint();
                }
                ui.add_space(4.0);

                if !self.is_exporting {
                    ui.horizontal(|ui| {
                        // Left: Preset Selector Column (Width: 230px, strictly vertical layout)
                        ui.allocate_ui_with_layout(
                            Vec2::new(230.0, 370.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.group(|ui| {
                                    ui.vertical(|ui| {
                                        ui.set_width(216.0);
                                        ui.strong("Hazır Profiller");
                                        ui.add_space(6.0);

                                        for (idx, p) in self.presets.iter().enumerate() {
                                            let is_sel = self.selected_preset_idx == idx;
                                            let text_color = if is_sel {
                                                Color32::WHITE
                                            } else {
                                                Color32::from_rgb(0xd0, 0xd0, 0xdc)
                                            };
                                            let btn = egui::SelectableLabel::new(
                                                is_sel,
                                                RichText::new(&p.name).size(12.0).color(text_color),
                                            );
                                            if ui.add_sized([210.0, 26.0], btn).clicked() {
                                                self.selected_preset_idx = idx;
                                                if let Some(w) = p.width {
                                                    self.custom_width = w;
                                                }
                                                if let Some(h) = p.height {
                                                    self.custom_height = h;
                                                }
                                                if let Some(f) = p.fps {
                                                    self.custom_fps = f;
                                                }
                                                self.custom_crf = p.crf;
                                                self.loudness_norm = p.loudness_norm;
                                            }
                                            ui.add_space(2.0);
                                        }
                                    });
                                });
                            },
                        );

                        ui.separator();

                        // Right: Detailed Settings and Parameters (Width: 320px)
                        ui.allocate_ui(Vec2::new(320.0, 360.0), |ui| {
                            ui.vertical(|ui| {
                                let curr_preset = &self.presets[self.selected_preset_idx];

                                ui.group(|ui| {
                                    ui.set_width(310.0);
                                    ui.strong("Video Parametreleri");
                                    ui.add_space(4.0);

                                    ui.horizontal(|ui| {
                                        ui.label("Format:");
                                        if ui
                                            .selectable_label(
                                                self.custom_width == 1920
                                                    && self.custom_height == 1080,
                                                "16:9",
                                            )
                                            .clicked()
                                        {
                                            self.custom_width = 1920;
                                            self.custom_height = 1080;
                                        }
                                        if ui
                                            .selectable_label(
                                                self.custom_width == 1080
                                                    && self.custom_height == 1920,
                                                "9:16 Dikey",
                                            )
                                            .clicked()
                                        {
                                            self.custom_width = 1080;
                                            self.custom_height = 1920;
                                        }
                                        if ui
                                            .selectable_label(
                                                self.custom_width == 1080
                                                    && self.custom_height == 1080,
                                                "1:1 Kare",
                                            )
                                            .clicked()
                                        {
                                            self.custom_width = 1080;
                                            self.custom_height = 1080;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Çözünürlük:");
                                        ui.add(
                                            egui::DragValue::new(&mut self.custom_width)
                                                .range(320..=7680),
                                        );
                                        ui.label("x");
                                        ui.add(
                                            egui::DragValue::new(&mut self.custom_height)
                                                .range(240..=4320),
                                        );
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Kare Hızı (FPS):");
                                        ui.add(
                                            egui::DragValue::new(&mut self.custom_fps)
                                                .range(12.0..=120.0)
                                                .suffix(" fps"),
                                        );
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Kalite (CRF):");
                                        ui.add(Slider::new(&mut self.custom_crf, 10..=35).text(""));
                                        let qual_label = if self.custom_crf <= 16 {
                                            "Ultra"
                                        } else if self.custom_crf <= 22 {
                                            "Yüksek"
                                        } else {
                                            "Kompakt"
                                        };
                                        ui.label(
                                            RichText::new(qual_label)
                                                .size(10.5)
                                                .color(Color32::from_rgb(0x38, 0xa0, 0xf0)),
                                        );
                                    });

                                    let detected_enc = detect_available_encoders();
                                    ui.horizontal(|ui| {
                                        ui.label("Hızlandırıcı (GPU):");
                                        egui::ComboBox::from_id_salt("hw_accel_combo")
                                            .selected_text(self.hw_accel.short_label())
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(
                                                    &mut self.hw_accel,
                                                    HardwareAccel::Auto,
                                                    "Otomatik (En Hızlı GPU)",
                                                );
                                                if detected_enc.has_nvidia_nvenc() {
                                                    ui.selectable_value(
                                                        &mut self.hw_accel,
                                                        HardwareAccel::Nvenc,
                                                        "NVIDIA NVENC (GPU)",
                                                    );
                                                }
                                                if detected_enc.has_intel_qsv() {
                                                    ui.selectable_value(
                                                        &mut self.hw_accel,
                                                        HardwareAccel::Qsv,
                                                        "Intel QuickSync (QSV)",
                                                    );
                                                }
                                                if detected_enc.has_vaapi() {
                                                    ui.selectable_value(
                                                        &mut self.hw_accel,
                                                        HardwareAccel::Vaapi,
                                                        "Linux VAAPI",
                                                    );
                                                }
                                                if detected_enc.has_amd_amf() {
                                                    ui.selectable_value(
                                                        &mut self.hw_accel,
                                                        HardwareAccel::Amf,
                                                        "AMD AMF (GPU)",
                                                    );
                                                }
                                                ui.selectable_value(
                                                    &mut self.hw_accel,
                                                    HardwareAccel::Cpu,
                                                    "İşlemci (CPU x264/x265)",
                                                );
                                            });
                                    });

                                    let badge_text = if detected_enc.has_nvidia_nvenc() {
                                        "⚡ NVIDIA NVENC Donanım Hızlandırma Algılandı"
                                    } else if detected_enc.has_intel_qsv() {
                                        "⚡ Intel QuickSync Donanım Hızlandırma Algılandı"
                                    } else if detected_enc.has_vaapi() {
                                        "⚡ Linux VAAPI Donanım Hızlandırma Algılandı"
                                    } else if detected_enc.has_amd_amf() {
                                        "⚡ AMD AMF Donanım Hızlandırma Algılandı"
                                    } else {
                                        "ℹ️ Donanım hızlandırma bulunamadı, CPU kullanılacak"
                                    };
                                    ui.label(RichText::new(badge_text).size(10.0).color(
                                        if detected_enc.any_hardware() {
                                            Color32::from_rgb(0x3d, 0xdc, 0x84)
                                        } else {
                                            Color32::from_rgb(0x90, 0x90, 0xa0)
                                        },
                                    ));

                                    ui.horizontal(|ui| {
                                        ui.label("Video Kodeki:");
                                        ui.label(
                                            RichText::new(&curr_preset.video_codec)
                                                .strong()
                                                .color(Color32::from_rgb(0xe0, 0x7a, 0x38)),
                                        );
                                    });
                                });

                                ui.add_space(4.0);

                                ui.group(|ui| {
                                    ui.set_width(310.0);
                                    ui.strong("Ses & Tahmini Boyut");
                                    ui.add_space(2.0);

                                    ui.horizontal(|ui| {
                                        ui.label("Ses Kodeki:");
                                        ui.label(
                                            RichText::new(format!(
                                                "{} @ {}",
                                                curr_preset.audio_codec, curr_preset.audio_bitrate
                                            ))
                                            .color(Color32::from_rgb(0x3d, 0xdc, 0x84)),
                                        );
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Normalizasyon:");
                                        egui::ComboBox::from_id_salt("loudness_norm_select")
                                            .selected_text(self.loudness_norm.label())
                                            .show_ui(ui, |ui| {
                                                for norm in LoudnessNorm::ALL {
                                                    ui.selectable_value(
                                                        &mut self.loudness_norm,
                                                        *norm,
                                                        norm.label(),
                                                    );
                                                }
                                            });
                                    });

                                    // Approximate size calculation: (crf-dependent bitrate + audio bitrate) * duration
                                    let approx_mbps = match self.custom_crf {
                                        0..=15 => 25.0,
                                        16..=19 => 14.0,
                                        20..=24 => 7.0,
                                        _ => 3.5,
                                    };
                                    let est_mb = (approx_mbps
                                        * (self.custom_width as f64 * self.custom_height as f64
                                            / (1920.0 * 1080.0))
                                        * self.timeline_duration
                                        / 8.0)
                                        .max(0.5);
                                    ui.horizontal(|ui| {
                                        ui.label("Tahmini Boyut:");
                                        ui.label(
                                            RichText::new(format!("~{:.1} MB", est_mb))
                                                .strong()
                                                .color(Color32::from_rgb(0x38, 0xa0, 0xf0)),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "({:.1} sn)",
                                                self.timeline_duration
                                            ))
                                            .size(10.0)
                                            .color(Color32::from_rgb(0x80, 0x80, 0x90)),
                                        );
                                    });
                                });

                                ui.add_space(4.0);

                                ui.group(|ui| {
                                    ui.set_width(310.0);
                                    ui.strong("Çıktı Dosyası Yolu");
                                    ui.horizontal(|ui| {
                                        ui.text_edit_singleline(&mut self.output_path);
                                        if ui.button("Gözat...").clicked() {
                                            if let Some(path) = rfd::FileDialog::new()
                                                .set_file_name("cikti.mp4")
                                                .add_filter("MP4 Video", &["mp4"])
                                                .add_filter("QuickTime MOV", &["mov"])
                                                .save_file()
                                            {
                                                self.output_path =
                                                    path.to_string_lossy().to_string();
                                            }
                                        }
                                    });
                                });
                            });
                        });
                    });

                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        let btn_start = Button::new(
                            RichText::new("Dışa Aktarmayı Başlat")
                                .color(Color32::WHITE)
                                .size(13.0)
                                .strong(),
                        )
                        .fill(Color32::from_rgb(0xe0, 0x7a, 0x38))
                        .min_size(Vec2::new(180.0, 30.0));

                        if ui.add(btn_start).clicked() {
                            let mut preset = self.presets[self.selected_preset_idx].clone();
                            preset.width = Some(self.custom_width);
                            preset.height = Some(self.custom_height);
                            preset.fps = Some(self.custom_fps);
                            preset.crf = self.custom_crf;
                            preset.hw_accel = self.hw_accel;
                            preset.loudness_norm = self.loudness_norm;

                            self.is_exporting = true;
                            self.progress = 0.0;
                            action = Some(ExportDialogAction::StartExport(
                                preset,
                                self.output_path.clone(),
                            ));
                        }

                        if ui.button("Kapat").clicked() {
                            self.is_open = false;
                        }
                    });
                } else {
                    // Export In Progress View
                    ui.group(|ui| {
                        ui.set_width(550.0);
                        ui.strong("Dışa Aktarma Devam Ediyor");
                        ui.add_space(8.0);
                        ui.add(
                            ProgressBar::new(self.progress / 100.0)
                                .text(format!("{:.1}%", self.progress)),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(&self.status_text)
                                .size(12.0)
                                .color(Color32::from_rgb(0xa0, 0xa0, 0xb8)),
                        );

                        if self.progress >= 99.9 {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("✅ Dışa aktarma başarıyla tamamlandı!")
                                    .color(Color32::from_rgb(0x3d, 0xdc, 0x84))
                                    .strong(),
                            );
                            ui.add_space(6.0);
                            if ui.button("📁 Dosyayı Klasörde Göster").clicked() {
                                action = Some(ExportDialogAction::OpenOutputFolder(
                                    self.output_path.clone(),
                                ));
                            }
                        }
                    });

                    ui.add_space(14.0);

                    if self.progress < 99.9 {
                        if ui
                            .button(
                                RichText::new("İptal Et")
                                    .color(Color32::from_rgb(0xe0, 0x50, 0x50)),
                            )
                            .clicked()
                        {
                            action = Some(ExportDialogAction::CancelExport);
                        }
                    } else if ui.button("Kapat").clicked() {
                        self.is_open = false;
                        self.is_exporting = false;
                    }
                }
            });

        action
    }
}
