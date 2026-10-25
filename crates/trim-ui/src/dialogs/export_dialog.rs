//! Modern NLE Export Video Studio dialog modal with presets, custom resolution/codecs, and progress.

use crate::file_dialog::{FileDialogBridge, FileDialogConfig, FileDialogRequest};
use egui::{
    Align2, Area, Button, Color32, DragValue, Frame, Id, Layout, Margin, Order, Pos2, ProgressBar,
    RichText, Rounding, ScrollArea, Sense, Shadow, Slider, Stroke, TextEdit, Ui, Vec2, Window,
};
use trim_core::project::Project;
use trim_core::time::seconds_to_timecode;
use trim_export::presets::{get_default_presets, ExportPreset, HardwareAccel, LoudnessNorm};
use trim_media::detect_available_encoders;

const MODAL_BG: Color32 = Color32::from_rgb(0x13, 0x15, 0x1c);
const CARD_BG: Color32 = Color32::from_rgb(0x17, 0x19, 0x22);
const CARD_BORDER: Color32 = Color32::from_rgb(0x27, 0x2b, 0x38);
const ACCENT_AMBER: Color32 = Color32::from_rgb(0xe0, 0x7a, 0x38);
const ACCENT_BLUE: Color32 = Color32::from_rgb(0x38, 0x7a, 0xe0);
const ACCENT_TEAL: Color32 = Color32::from_rgb(0x2d, 0xb8, 0x9a);
const TEXT_MUTED: Color32 = Color32::from_rgb(0x8a, 0x92, 0xa6);

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
    pub file_dialog_bridge: FileDialogBridge,
    pub timeline_native_width: u32,
    pub timeline_native_height: u32,
    pub timeline_native_fps: f64,
    pub in_point: Option<f64>,
    pub out_point: Option<f64>,
    pub export_range_only: bool,
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
            file_dialog_bridge: FileDialogBridge::default(),
            timeline_native_width: 1920,
            timeline_native_height: 1080,
            timeline_native_fps: 30.0,
            in_point: None,
            out_point: None,
            export_range_only: false,
        }
    }
}

pub enum ExportDialogAction {
    StartExport(ExportPreset, String, Option<(f64, f64)>),
    AddToQueue(ExportPreset, String, Option<(f64, f64)>),
    CancelExport,
    OpenOutputFolder(String),
}

impl ExportDialog {
    pub fn open(&mut self, project: &Project) {
        self.timeline_native_width = project.timeline.width;
        self.timeline_native_height = project.timeline.height;
        self.timeline_native_fps = project.timeline.fps;

        self.custom_width = project.timeline.width;
        self.custom_height = project.timeline.height;
        self.custom_fps = project.timeline.fps;
        self.in_point = project.timeline.in_point;
        self.out_point = project.timeline.out_point;
        self.export_range_only =
            project.timeline.in_point.is_some() || project.timeline.out_point.is_some();
        self.open_with_duration(project.timeline.duration());
    }

    pub fn open_with_duration(&mut self, duration: f64) {
        self.is_open = true;
        self.timeline_duration = duration.max(0.1);
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<ExportDialogAction> {
        if let Some(resp) = self.file_dialog_bridge.poll() {
            if let Some(path) = resp.path {
                self.output_path = path.to_string_lossy().to_string();
            }
            ctx.request_repaint();
        }
        if self.file_dialog_bridge.is_pending() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        if !self.is_open {
            return None;
        }

        // Escape closes when not currently rendering
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !self.is_exporting {
            self.is_open = false;
            return None;
        }

        if self.is_exporting {
            ctx.request_repaint();
        }

        // Full-screen modal backdrop dimmer & input blocker
        let screen_rect = ctx.screen_rect();
        Area::new(Id::new("export_modal_backdrop"))
            .order(Order::Middle)
            .fixed_pos(Pos2::ZERO)
            .show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(screen_rect.size(), Sense::click_and_drag());
                ui.painter()
                    .rect_filled(rect, 0.0, Color32::from_black_alpha(175));
            });

        let mut action = None;

        let modal_frame = Frame::window(&ctx.style())
            .fill(MODAL_BG)
            .rounding(Rounding::same(12.0))
            .stroke(Stroke::new(1.0_f32, CARD_BORDER))
            .inner_margin(Margin::symmetric(20.0, 18.0))
            .shadow(Shadow {
                offset: Vec2::new(0.0, 16.0),
                blur: 40.0,
                spread: 4.0,
                color: Color32::from_black_alpha(190),
            });

        Window::new("triM. - Videoyu Dışa Aktar")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .order(Order::Foreground)
            .fixed_size(Vec2::new(760.0, 520.0))
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(modal_frame)
            .show(ctx, |ui| {
                // Header Bar
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🎬").size(18.0));
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new("Videoyu Dışa Aktar")
                            .size(16.0)
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.add_space(6.0);
                    Frame::none()
                        .fill(Color32::from_rgba_unmultiplied(0xe0, 0x7a, 0x38, 30))
                        .stroke(Stroke::new(1.0_f32, ACCENT_AMBER))
                        .rounding(Rounding::same(4.0))
                        .inner_margin(Margin::symmetric(6.0, 2.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("RENDER STUDIO")
                                    .size(9.5)
                                    .strong()
                                    .color(Color32::from_rgb(0xf2, 0x9b, 0x5e)),
                            );
                        });

                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.is_exporting {
                            let close_btn = Button::new(
                                RichText::new("✕")
                                    .size(13.0)
                                    .color(Color32::from_rgb(0x8a, 0x92, 0xa6)),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE);
                            if ui.add(close_btn).on_hover_text("Kapat (ESC)").clicked() {
                                self.is_open = false;
                            }
                        }
                    });
                });

                ui.add_space(10.0);

                if !self.is_exporting {
                    // Two-Column Layout: Left Presets (250px), Right Details (470px)
                    ui.horizontal(|ui| {
                        // =========================================================
                        // LEFT COLUMN: Presets Library
                        // =========================================================
                        let left_w = 260.0;
                        let content_h = 415.0;

                        ui.allocate_ui_with_layout(
                            Vec2::new(left_w, content_h),
                            Layout::top_down(egui::Align::Min),
                            |ui| {
                                Frame::none()
                                    .fill(CARD_BG)
                                    .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                                    .rounding(Rounding::same(8.0))
                                    .inner_margin(Margin::symmetric(10.0, 10.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new("Hazır Profiller")
                                                    .size(12.0)
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            );
                                            ui.with_layout(
                                                Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{} profil",
                                                            self.presets.len()
                                                        ))
                                                        .size(10.5)
                                                        .color(TEXT_MUTED),
                                                    );
                                                },
                                            );
                                        });

                                        ui.add_space(8.0);

                                        ScrollArea::vertical()
                                            .max_height(365.0)
                                            .auto_shrink([false, false])
                                            .show(ui, |ui| {
                                                ui.spacing_mut().item_spacing.y = 5.0;
                                                for (idx, p) in self.presets.iter().enumerate() {
                                                    let is_sel = self.selected_preset_idx == idx;
                                                    let (icon, tag, short_name) =
                                                        get_preset_metadata(&p.name);

                                                    let card_frame = Frame::none()
                                                        .fill(if is_sel {
                                                            Color32::from_rgb(0x22, 0x27, 0x36)
                                                        } else {
                                                            Color32::from_rgb(0x13, 0x15, 0x1d)
                                                        })
                                                        .stroke(Stroke::new(
                                                            1.0_f32,
                                                            if is_sel {
                                                                ACCENT_AMBER
                                                            } else {
                                                                Color32::from_rgb(0x1e, 0x21, 0x2c)
                                                            },
                                                        ))
                                                        .rounding(Rounding::same(6.0))
                                                        .inner_margin(Margin::symmetric(9.0, 7.0));

                                                    let resp = card_frame.show(ui, |ui| {
                                                        ui.horizontal(|ui| {
                                                            ui.label(RichText::new(icon).size(15.0));
                                                            ui.vertical(|ui| {
                                                                ui.label(
                                                                    RichText::new(short_name)
                                                                        .size(11.5)
                                                                        .strong()
                                                                        .color(if is_sel {
                                                                            Color32::WHITE
                                                                        } else {
                                                                            Color32::from_rgb(0xd8, 0xde, 0xec)
                                                                        }),
                                                                );
                                                                let dim_str = match (p.width, p.height) {
                                                                    (Some(w), Some(h)) => format!("{}×{}", w, h),
                                                                    _ => "Orijinal Sekans".to_string(),
                                                                };
                                                                ui.label(
                                                                    RichText::new(format!(
                                                                        "{} · {}",
                                                                        dim_str, p.video_codec
                                                                    ))
                                                                    .size(10.0)
                                                                    .color(TEXT_MUTED),
                                                                );
                                                            });
                                                            ui.with_layout(
                                                                Layout::right_to_left(egui::Align::Center),
                                                                |ui| {
                                                                    render_tag_pill(ui, tag, is_sel);
                                                                },
                                                            );
                                                        });
                                                    });

                                                    if resp.response.interact(Sense::click()).clicked() {
                                                        self.selected_preset_idx = idx;
                                                        if let Some(w) = p.width {
                                                            self.custom_width = w;
                                                        } else {
                                                            self.custom_width = self.timeline_native_width;
                                                        }
                                                        if let Some(h) = p.height {
                                                            self.custom_height = h;
                                                        } else {
                                                            self.custom_height = self.timeline_native_height;
                                                        }
                                                        if let Some(f) = p.fps {
                                                            self.custom_fps = f;
                                                        } else {
                                                            self.custom_fps = self.timeline_native_fps;
                                                        }
                                                        self.custom_crf = p.crf;
                                                        self.loudness_norm = p.loudness_norm;
                                                    }
                                                }
                                            });
                                    });
                            },
                        );

                        ui.add_space(6.0);

                        // =========================================================
                        // RIGHT COLUMN: Parameters & Settings (455px)
                        // =========================================================
                        let right_w = 450.0;
                        ui.allocate_ui_with_layout(
                            Vec2::new(right_w, content_h),
                            Layout::top_down(egui::Align::Min),
                            |ui| {
                                let curr_preset = &self.presets[self.selected_preset_idx];

                                // Card 1: Video Parameters
                                Frame::none()
                                    .fill(CARD_BG)
                                    .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                                    .rounding(Rounding::same(8.0))
                                    .inner_margin(Margin::symmetric(12.0, 10.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("🎞").size(12.0).color(ACCENT_AMBER));
                                            ui.label(
                                                RichText::new("Video Parametreleri")
                                                    .size(11.5)
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            );
                                            ui.with_layout(
                                                Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    render_chip(
                                                        ui,
                                                        &curr_preset.video_codec,
                                                        Color32::from_rgb(0xe0, 0x7a, 0x38),
                                                    );
                                                },
                                            );
                                        });

                                        ui.add_space(6.0);

                                        // Format Quick Switcher Pills
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("Hızlı Format:").size(11.0).color(TEXT_MUTED));
                                            for (lbl, target_w, target_h) in [
                                                ("16:9 Yatay", 1920, 1080),
                                                ("9:16 Dikey", 1080, 1920),
                                                ("1:1 Kare", 1080, 1080),
                                                ("21:9 Sinema", 2560, 1080),
                                            ] {
                                                let is_match = self.custom_width == target_w && self.custom_height == target_h;
                                                let pill = Button::new(
                                                    RichText::new(lbl)
                                                        .size(10.0)
                                                        .strong()
                                                        .color(if is_match { Color32::WHITE } else { TEXT_MUTED }),
                                                )
                                                .fill(if is_match { ACCENT_AMBER } else { Color32::from_rgb(0x1f, 0x22, 0x2e) })
                                                .rounding(Rounding::same(4.0));
                                                if ui.add(pill).clicked() {
                                                    self.custom_width = target_w;
                                                    self.custom_height = target_h;
                                                }
                                            }
                                        });

                                        ui.add_space(4.0);

                                        // Resolution & Native Reset
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("Çözünürlük:").size(11.0).color(TEXT_MUTED));
                                            ui.add(
                                                DragValue::new(&mut self.custom_width)
                                                    .range(320..=7680)
                                                    .speed(2.0)
                                                    .suffix(" px"),
                                            );
                                            ui.label(RichText::new("×").size(11.0).color(TEXT_MUTED));
                                            ui.add(
                                                DragValue::new(&mut self.custom_height)
                                                    .range(240..=4320)
                                                    .speed(2.0)
                                                    .suffix(" px"),
                                            );

                                            if (self.custom_width != self.timeline_native_width
                                                || self.custom_height != self.timeline_native_height)
                                                && ui
                                                    .add(
                                                        Button::new(
                                                            RichText::new("↺ Sekanstan Al")
                                                                .size(10.0)
                                                                .color(Color32::from_rgb(0xc0, 0xca, 0xe0)),
                                                        )
                                                        .fill(Color32::from_rgb(0x22, 0x26, 0x34))
                                                        .rounding(Rounding::same(4.0)),
                                                    )
                                                    .on_hover_text(format!(
                                                        "Zaman çizgisi çözünürlüğüne ({}) geri dön",
                                                        self.timeline_native_width
                                                    ))
                                                    .clicked()
                                            {
                                                self.custom_width = self.timeline_native_width;
                                                self.custom_height = self.timeline_native_height;
                                                self.custom_fps = self.timeline_native_fps;
                                            }
                                        });

                                        ui.add_space(4.0);

                                        // FPS Row + Quick Presets
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("Kare Hızı:").size(11.0).color(TEXT_MUTED));
                                            ui.add(
                                                DragValue::new(&mut self.custom_fps)
                                                    .range(12.0..=120.0)
                                                    .speed(0.5)
                                                    .suffix(" fps"),
                                            );
                                            for fps_val in [24.0, 30.0, 60.0] {
                                                let is_fps = (self.custom_fps - fps_val).abs() < 0.05;
                                                let pill = Button::new(
                                                    RichText::new(format!("{:.0}", fps_val))
                                                        .size(10.0)
                                                        .strong()
                                                        .color(if is_fps { Color32::WHITE } else { TEXT_MUTED }),
                                                )
                                                .fill(if is_fps { ACCENT_BLUE } else { Color32::from_rgb(0x1f, 0x22, 0x2e) })
                                                .rounding(Rounding::same(4.0));
                                                if ui.add(pill).clicked() {
                                                    self.custom_fps = fps_val;
                                                }
                                            }
                                        });

                                        ui.add_space(4.0);

                                        // Quality (CRF) Slider
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("Kalite (CRF):").size(11.0).color(TEXT_MUTED));
                                            ui.add(
                                                Slider::new(&mut self.custom_crf, 10..=35)
                                                    .show_value(true)
                                                    .trailing_fill(true),
                                            );
                                            let (qual_label, qual_col) = if self.custom_crf <= 15 {
                                                ("Kayıpsıza Yakın", Color32::from_rgb(0x2d, 0xb8, 0x9a))
                                            } else if self.custom_crf <= 20 {
                                                ("Yüksek Kalite", Color32::from_rgb(0x38, 0xa0, 0xf0))
                                            } else if self.custom_crf <= 25 {
                                                ("Dengeli", Color32::from_rgb(0xf2, 0x9b, 0x5e))
                                            } else {
                                                ("Kompakt", Color32::from_rgb(0xa0, 0xa0, 0xb0))
                                            };
                                            render_chip(ui, qual_label, qual_col);
                                        });

                                        ui.add_space(4.0);

                                        // Hardware Acceleration (GPU)
                                        let detected_enc = detect_available_encoders();
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("Donanım (GPU):").size(11.0).color(TEXT_MUTED));
                                            egui::ComboBox::from_id_salt("hw_accel_combo")
                                                .selected_text(self.hw_accel.short_label())
                                                .show_ui(ui, |ui| {
                                                    ui.selectable_value(
                                                        &mut self.hw_accel,
                                                        HardwareAccel::Auto,
                                                        "Otomatik (En Hızlı Donanım GPU)",
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
                                                        "Yazılım / İşlemci (CPU x264/x265)",
                                                    );
                                                });

                                            let (badge_text, badge_col) = if detected_enc.has_nvidia_nvenc() {
                                                ("⚡ NVENC Donanım Aktif", Color32::from_rgb(0x3d, 0xdc, 0x84))
                                            } else if detected_enc.has_intel_qsv() {
                                                ("⚡ QuickSync Aktif", Color32::from_rgb(0x38, 0xa0, 0xf0))
                                            } else if detected_enc.has_vaapi() {
                                                ("⚡ VAAPI Aktif", Color32::from_rgb(0x3d, 0xdc, 0x84))
                                            } else if detected_enc.has_amd_amf() {
                                                ("⚡ AMD AMF Aktif", Color32::from_rgb(0xe0, 0x50, 0x50))
                                            } else {
                                                ("CPU Yazılım", Color32::from_rgb(0x8a, 0x92, 0xa6))
                                            };
                                            render_chip(ui, badge_text, badge_col);
                                        });
                                    });

                                ui.add_space(6.0);

                                // Card 2: Audio & Smart Normalization
                                Frame::none()
                                    .fill(CARD_BG)
                                    .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                                    .rounding(Rounding::same(8.0))
                                    .inner_margin(Margin::symmetric(12.0, 9.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("🔊").size(12.0).color(ACCENT_TEAL));
                                            ui.label(
                                                RichText::new("Ses & Normalizasyon")
                                                    .size(11.5)
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            );
                                            ui.with_layout(
                                                Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    render_chip(
                                                        ui,
                                                        &format!("{} @ {}", curr_preset.audio_codec, curr_preset.audio_bitrate),
                                                        ACCENT_TEAL,
                                                    );
                                                },
                                            );
                                        });

                                        ui.add_space(5.0);

                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("Normalizasyon:").size(11.0).color(TEXT_MUTED));
                                            egui::ComboBox::from_id_salt("loudness_norm_select")
                                                .selected_text(self.loudness_norm.label())
                                                .width(260.0)
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
                                    });

                                ui.add_space(6.0);

                                // Card 3: Output Destination & Estimated Size
                                Frame::none()
                                    .fill(CARD_BG)
                                    .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                                    .rounding(Rounding::same(8.0))
                                    .inner_margin(Margin::symmetric(12.0, 9.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("📁").size(12.0).color(ACCENT_BLUE));
                                            ui.label(
                                                RichText::new("Çıktı Dosyası")
                                                    .size(11.5)
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            );

                                            // Size calculation
                                            let is_prores = curr_preset.video_codec.contains("prores");
                                            let approx_mbps = if is_prores {
                                                147.0
                                            } else {
                                                match self.custom_crf {
                                                    0..=15 => 28.0,
                                                    16..=19 => 14.0,
                                                    20..=24 => 7.0,
                                                    _ => 3.5,
                                                }
                                            };
                                            let est_mb = (approx_mbps
                                                * (self.custom_width as f64 * self.custom_height as f64
                                                    / (1920.0 * 1080.0))
                                                * (self.custom_fps / 30.0).clamp(0.5, 2.0)
                                                * self.timeline_duration
                                                / 8.0)
                                                .max(0.2);

                                            ui.with_layout(
                                                Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    render_chip(
                                                        ui,
                                                        &format!("~{:.1} MB", est_mb),
                                                        Color32::from_rgb(0x38, 0xa0, 0xf0),
                                                    );
                                                    let tc = seconds_to_timecode(self.timeline_duration, self.custom_fps);
                                                    render_chip(ui, &tc, TEXT_MUTED);
                                                },
                                            );
                                        });

                                        ui.add_space(5.0);

                                        ui.horizontal(|ui| {
                                            ui.add(
                                                TextEdit::singleline(&mut self.output_path)
                                                    .desired_width(ui.available_width() - 85.0),
                                            );
                                            let browse_enabled = !self.file_dialog_bridge.is_pending();
                                            if ui
                                                .add_enabled(
                                                    browse_enabled,
                                                    Button::new(
                                                        RichText::new("📁 Gözat...")
                                                            .size(11.0)
                                                            .color(Color32::WHITE),
                                                    )
                                                    .fill(Color32::from_rgb(0x23, 0x28, 0x38))
                                                    .rounding(Rounding::same(5.0)),
                                                )
                                                .clicked()
                                            {
                                                let config = FileDialogConfig::save_file(
                                                    "Çıktı Dosyası Seç",
                                                    Some("cikti.mp4".to_string()),
                                                )
                                                .add_filter("MP4 Video", &["mp4"])
                                                .add_filter("QuickTime MOV", &["mov"]);
                                                self.file_dialog_bridge.open(
                                                    FileDialogRequest::SelectExportOutput,
                                                    config,
                                                );
                                            }
                                        });
                                    });
                            },
                        );
                    });

                    if self.in_point.is_some() || self.out_point.is_some() {
                        let in_val = self.in_point.unwrap_or(0.0);
                        let out_val = self.out_point.unwrap_or(self.timeline_duration);
                        ui.horizontal(|ui| {
                            ui.checkbox(
                                &mut self.export_range_only,
                                RichText::new(format!(
                                    "Yalnızca Giriş/Çıkış Aralığını Aktar ({} - {})",
                                    seconds_to_timecode(in_val, self.timeline_native_fps),
                                    seconds_to_timecode(out_val, self.timeline_native_fps),
                                ))
                                .size(11.5)
                                .color(Color32::from_rgb(0x38, 0xa0, 0xf0)),
                            );
                        });
                    }

                    ui.add_space(12.0);

                    // =========================================================
                    // BOTTOM ACTION BAR
                    // =========================================================
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Tip: Donanım hızlandırma aktifken render süresi 5 kata kadar hızlanır.")
                                .size(10.5)
                                .color(TEXT_MUTED),
                        );

                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            let btn_start = Button::new(
                                RichText::new("🎬  Dışa Aktarmayı Başlat")
                                    .color(Color32::WHITE)
                                    .size(12.5)
                                    .strong(),
                            )
                            .fill(ACCENT_AMBER)
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(195.0, 32.0));

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

                                let range = if self.export_range_only && (self.in_point.is_some() || self.out_point.is_some()) {
                                    let in_val = self.in_point.unwrap_or(0.0);
                                    let out_val = self.out_point.unwrap_or(self.timeline_duration);
                                    Some((in_val, out_val))
                                } else {
                                    None
                                };

                                action = Some(ExportDialogAction::StartExport(
                                    preset,
                                    self.output_path.clone(),
                                    range,
                                ));
                            }

                            ui.add_space(6.0);

                            let btn_queue = Button::new(
                                RichText::new("📥 Kuyruğa Ekle")
                                    .size(11.5)
                                    .color(Color32::from_rgb(0xe0, 0xe6, 0xf8))
                                    .strong(),
                            )
                            .fill(Color32::from_rgb(0x22, 0x27, 0x38))
                            .stroke(Stroke::new(1.0_f32, ACCENT_BLUE))
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(115.0, 32.0));

                            if ui.add(btn_queue).clicked() {
                                let mut preset = self.presets[self.selected_preset_idx].clone();
                                preset.width = Some(self.custom_width);
                                preset.height = Some(self.custom_height);
                                preset.fps = Some(self.custom_fps);
                                preset.crf = self.custom_crf;
                                preset.hw_accel = self.hw_accel;
                                preset.loudness_norm = self.loudness_norm;

                                let range = if self.export_range_only && (self.in_point.is_some() || self.out_point.is_some()) {
                                    let in_val = self.in_point.unwrap_or(0.0);
                                    let out_val = self.out_point.unwrap_or(self.timeline_duration);
                                    Some((in_val, out_val))
                                } else {
                                    None
                                };

                                self.is_open = false;
                                action = Some(ExportDialogAction::AddToQueue(
                                    preset,
                                    self.output_path.clone(),
                                    range,
                                ));
                            }

                            ui.add_space(6.0);

                            let btn_close = Button::new(
                                RichText::new("Vazgeç")
                                    .size(11.5)
                                    .color(Color32::from_rgb(0xc0, 0xc6, 0xd8)),
                            )
                            .fill(Color32::from_rgb(0x1a, 0x1d, 0x27))
                            .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(85.0, 32.0));

                            if ui.add(btn_close).clicked() {
                                self.is_open = false;
                            }
                        });
                    });
                } else {
                    // =========================================================
                    // EXPORT IN PROGRESS VIEW (Modern Dashboard)
                    // =========================================================
                    ui.add_space(20.0);
                    Frame::none()
                        .fill(CARD_BG)
                        .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::symmetric(24.0, 24.0))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                let is_done = self.progress >= 99.9;

                                if is_done {
                                    ui.label(RichText::new("🎉").size(34.0));
                                    ui.add_space(4.0);
                                    ui.label(
                                        RichText::new("Dışa Aktarma Başarıyla Tamamlandı!")
                                            .size(16.0)
                                            .strong()
                                            .color(Color32::from_rgb(0x3d, 0xdc, 0x84)),
                                    );
                                } else {
                                    ui.label(RichText::new("⚡").size(28.0).color(ACCENT_AMBER));
                                    ui.add_space(4.0);
                                    ui.label(
                                        RichText::new("Video Dışa Aktarılıyor...")
                                            .size(15.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                    );
                                }

                                ui.add_space(12.0);

                                // Big percentage readout
                                ui.label(
                                    RichText::new(format!("{:.1}%", self.progress))
                                        .size(36.0)
                                        .strong()
                                        .color(if is_done {
                                            Color32::from_rgb(0x3d, 0xdc, 0x84)
                                        } else {
                                            ACCENT_AMBER
                                        }),
                                );

                                ui.add_space(10.0);

                                ui.add(
                                    ProgressBar::new(self.progress / 100.0)
                                        .desired_width(520.0)
                                        .desired_height(14.0)
                                        .show_percentage(),
                                );

                                ui.add_space(12.0);

                                ui.label(
                                    RichText::new(&self.status_text)
                                        .size(12.0)
                                        .color(Color32::from_rgb(0xa0, 0xa8, 0xbe)),
                                );

                                ui.add_space(8.0);

                                ui.label(
                                    RichText::new(format!("Hedef: {}", self.output_path))
                                        .size(10.5)
                                        .color(TEXT_MUTED),
                                );

                                ui.add_space(20.0);

                                if is_done {
                                    ui.horizontal(|ui| {
                                        let btn_folder = Button::new(
                                            RichText::new("📁  Dosyayı Klasörde Göster")
                                                .size(12.0)
                                                .color(Color32::WHITE)
                                                .strong(),
                                        )
                                        .fill(Color32::from_rgb(0x28, 0x30, 0x44))
                                        .stroke(Stroke::new(1.0_f32, ACCENT_BLUE))
                                        .rounding(Rounding::same(6.0))
                                        .min_size(Vec2::new(190.0, 34.0));

                                        if ui.add(btn_folder).clicked() {
                                            action = Some(ExportDialogAction::OpenOutputFolder(
                                                self.output_path.clone(),
                                            ));
                                        }

                                        ui.add_space(8.0);

                                        let btn_done = Button::new(
                                            RichText::new("Tamamlandı")
                                                .size(12.0)
                                                .color(Color32::WHITE)
                                                .strong(),
                                        )
                                        .fill(Color32::from_rgb(0x23, 0x7a, 0x48))
                                        .rounding(Rounding::same(6.0))
                                        .min_size(Vec2::new(130.0, 34.0));

                                        if ui.add(btn_done).clicked() {
                                            self.is_open = false;
                                            self.is_exporting = false;
                                        }
                                    });
                                } else {
                                    let btn_cancel = Button::new(
                                        RichText::new("İptal Et")
                                            .size(11.5)
                                            .color(Color32::from_rgb(0xf0, 0x70, 0x70)),
                                    )
                                    .fill(Color32::from_rgb(0x28, 0x18, 0x1e))
                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x60, 0x24, 0x2c)))
                                    .rounding(Rounding::same(6.0))
                                    .min_size(Vec2::new(110.0, 30.0));

                                    if ui.add(btn_cancel).clicked() {
                                        action = Some(ExportDialogAction::CancelExport);
                                    }
                                }
                            });
                        });
                }
            });

        action
    }
}

fn get_preset_metadata(name: &str) -> (&'static str, &'static str, &'static str) {
    if name.contains("YouTube") {
        ("🖥", "1080p", "YouTube 1080p")
    } else if name.contains("720p") {
        ("⚡", "Hızlı", "Hızlı Web 720p")
    } else if name.contains("Kaynak") {
        ("🎞", "Kaynak", "Kaynak ile Eşleşen")
    } else if name.contains("4K") {
        ("🎬", "4K UHD", "4K UHD Sinematik")
    } else if name.contains("TikTok") || name.contains("Reels") {
        ("📱", "9:16", "Shorts / Reels")
    } else if name.contains("HEVC") || name.contains("H.265") {
        ("📦", "H.265", "HEVC / H.265")
    } else if name.contains("ProRes") {
        ("💎", "Master", "Apple ProRes 422")
    } else {
        ("🎬", "Özel", "Özel Profil")
    }
}

fn render_tag_pill(ui: &mut Ui, tag: &str, is_selected: bool) {
    let (bg, fg) = if is_selected {
        (
            Color32::from_rgba_unmultiplied(0xe0, 0x7a, 0x38, 40),
            Color32::from_rgb(0xf2, 0x9b, 0x5e),
        )
    } else {
        (
            Color32::from_rgb(0x1a, 0x1d, 0x26),
            Color32::from_rgb(0x8a, 0x92, 0xa6),
        )
    };
    Frame::none()
        .fill(bg)
        .rounding(Rounding::same(4.0))
        .inner_margin(Margin::symmetric(6.0, 2.0))
        .show(ui, |ui| {
            ui.label(RichText::new(tag).size(9.5).strong().color(fg));
        });
}

fn render_chip(ui: &mut Ui, text: &str, color: Color32) {
    Frame::none()
        .fill(Color32::from_rgba_unmultiplied(
            color.r(),
            color.g(),
            color.b(),
            32,
        ))
        .stroke(Stroke::new(1.0_f32, color))
        .rounding(Rounding::same(4.0))
        .inner_margin(Margin::symmetric(6.0, 2.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(9.5).strong().color(color));
        });
}
