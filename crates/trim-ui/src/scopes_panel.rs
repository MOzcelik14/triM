//! Independent, resizable Color Scopes panel (RGB Parade, Luma Waveform,
//! Vectorscope, Histogram, CIE 1931 Chromaticity, False Color, Zebra Exposure,
//! Quad 2x2 View, Live Pixel Readout & Reticle, Reference Still Gallery).

use egui::{
    Color32, ColorImage, FontId, Pos2, Rect, RichText, Rounding, ScrollArea, Sense, Stroke,
    TextureHandle, TextureOptions, Ui, Vec2,
};
use trim_core::clip::Clip;
use trim_core::scopes::{
    compute_false_color_rgba, compute_scopes_rgba, compute_zebra_rgba, rec709_cie1931_primaries,
    sample_pixel_rgba, vectorscope_targets_75, PixelSampleReadout, ScopeConfig, ScopeData,
    CIE_SIZE, PARADE_CH_W, VECTORSCOPE_SIZE, WAVEFORM_H, WAVEFORM_W,
};

pub struct ReferenceStill {
    pub label: String,
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub grade_snapshot: Option<Clip>,
    pub tex: Option<TextureHandle>,
}

pub struct ScopesPanelState {
    pub is_open: bool,
    pub config: ScopeConfig,
    pub data: ScopeData,
    pub last_eval_time: f64,
    pub needs_refresh: bool,
    pub active_pixel_sample: Option<PixelSampleReadout>,
    pub last_frame_rgba: Option<(Vec<u8>, u32, u32)>,
    pub reference_stills: Vec<ReferenceStill>,
    pub active_reference_still: Option<usize>,
    pub request_grab_still: bool,
    pub pending_apply_still_idx: Option<usize>,
    parade_tex: Option<TextureHandle>,
    waveform_tex: Option<TextureHandle>,
    vectorscope_tex: Option<TextureHandle>,
    cie_tex: Option<TextureHandle>,
    false_color_tex: Option<TextureHandle>,
    zebra_tex: Option<TextureHandle>,
}

impl Default for ScopesPanelState {
    fn default() -> Self {
        Self {
            is_open: false,
            config: ScopeConfig::default(),
            data: ScopeData::default(),
            last_eval_time: -1.0,
            needs_refresh: true,
            active_pixel_sample: None,
            last_frame_rgba: None,
            reference_stills: Vec::new(),
            active_reference_still: None,
            request_grab_still: false,
            pending_apply_still_idx: None,
            parade_tex: None,
            waveform_tex: None,
            vectorscope_tex: None,
            cie_tex: None,
            false_color_tex: None,
            zebra_tex: None,
        }
    }
}

impl ScopesPanelState {
    pub fn mark_dirty(&mut self) {
        self.needs_refresh = true;
    }

    pub fn clear(&mut self) {
        self.data = ScopeData::default();
        self.last_eval_time = -1.0;
        self.active_pixel_sample = None;
    }

    /// Updates the live pixel readout when the user hovers over `(x_norm, y_norm)` on the monitor.
    pub fn sample_hover_pixel(&mut self, x_norm: f32, y_norm: f32) {
        if let Some((ref rgba, w, h)) = self.last_frame_rgba {
            self.active_pixel_sample = sample_pixel_rgba(rgba, w, h, x_norm, y_norm);
        }
    }

    /// Captures the current monitor frame and active clip grade into the Reference Still Gallery.
    pub fn grab_still(&mut self, ctx: &egui::Context, active_clip: Option<&Clip>, time_sec: f64) {
        let Some((ref rgba, w, h)) = self.last_frame_rgba else {
            return;
        };
        let (tw, th, thumb) = downsample_rgba(rgba, w, h, 320, 180);
        let idx = self.reference_stills.len() + 1;
        let label = format!("Still #{idx} ({time_sec:.1}s)");
        let img = ColorImage::from_rgba_unmultiplied([tw as usize, th as usize], &thumb);
        let tex = ctx.load_texture(format!("ref_still_{idx}"), img, TextureOptions::LINEAR);
        self.reference_stills.push(ReferenceStill {
            label,
            rgba: thumb,
            width: tw,
            height: th,
            grade_snapshot: active_clip.cloned(),
            tex: Some(tex),
        });
        self.active_reference_still = Some(self.reference_stills.len() - 1);
    }

    /// Recomputes active scopes from real RGBA pixel data and updates cached textures.
    pub fn update_from_rgba(&mut self, ctx: &egui::Context, rgba: &[u8], width: u32, height: u32) {
        if width == 0 || height == 0 || rgba.len() < (width as usize) * (height as usize) * 4 {
            return;
        }
        self.last_frame_rgba = Some((rgba.to_vec(), width, height));
        self.data = compute_scopes_rgba(rgba, width, height);
        self.needs_refresh = false;

        if self.config.rgb_parade || self.config.quad_layout {
            let img = build_parade_image(&self.data);
            set_or_load_tex(ctx, &mut self.parade_tex, "scope_parade", img);
        }
        if self.config.luma_waveform || self.config.quad_layout {
            let img = build_luma_waveform_image(&self.data);
            set_or_load_tex(ctx, &mut self.waveform_tex, "scope_waveform", img);
        }
        if self.config.vectorscope || self.config.quad_layout {
            let img = build_vectorscope_image(&self.data);
            set_or_load_tex(ctx, &mut self.vectorscope_tex, "scope_vectorscope", img);
        }
        if self.config.cie_1931 || self.config.quad_layout {
            let img = build_cie_image(&self.data);
            set_or_load_tex(ctx, &mut self.cie_tex, "scope_cie1931", img);
        }

        if self.config.false_color || self.config.zebra_exposure {
            let (dw, dh, small) = downsample_rgba(rgba, width, height, 320, 180);
            if self.config.false_color {
                let fc = compute_false_color_rgba(&small, dw, dh);
                let img = ColorImage::from_rgba_unmultiplied([dw as usize, dh as usize], &fc);
                set_or_load_tex(ctx, &mut self.false_color_tex, "scope_false_color", img);
            }
            if self.config.zebra_exposure {
                let zb = compute_zebra_rgba(&small, dw, dh, self.config.zebra_threshold);
                let img = ColorImage::from_rgba_unmultiplied([dw as usize, dh as usize], &zb);
                set_or_load_tex(ctx, &mut self.zebra_tex, "scope_zebra", img);
            }
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Renk Kapsamları (Scopes)")
                    .strong()
                    .size(13.0)
                    .color(Color32::from_rgb(0xe8, 0xea, 0xf0)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("✕")
                    .on_hover_text("Renk Kapsamları Panelini Kapat")
                    .clicked()
                {
                    self.is_open = false;
                }
                if scope_chip(ui, "⊞ Quad 2x2", self.config.quad_layout)
                    .on_hover_text("DaVinci Resolve tarzı 2x2 Dörtlü Eşzamanlı Scope Görünümü")
                    .clicked()
                {
                    self.config.quad_layout = !self.config.quad_layout;
                    self.needs_refresh = true;
                }
            });
        });
        ui.separator();

        // Scope toggles
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
            if scope_chip(ui, "RGB Parade", self.config.rgb_parade).clicked() {
                self.config.rgb_parade = !self.config.rgb_parade;
                self.needs_refresh = true;
            }
            if scope_chip(ui, "Waveform", self.config.luma_waveform).clicked() {
                self.config.luma_waveform = !self.config.luma_waveform;
                self.needs_refresh = true;
            }
            if scope_chip(ui, "Vectorscope", self.config.vectorscope).clicked() {
                self.config.vectorscope = !self.config.vectorscope;
                self.needs_refresh = true;
            }
            if scope_chip(ui, "CIE 1931", self.config.cie_1931).clicked() {
                self.config.cie_1931 = !self.config.cie_1931;
                self.needs_refresh = true;
            }
            if scope_chip(ui, "Histogram", self.config.histogram).clicked() {
                self.config.histogram = !self.config.histogram;
                self.needs_refresh = true;
            }
            if scope_chip(ui, "False Color", self.config.false_color).clicked() {
                self.config.false_color = !self.config.false_color;
                self.needs_refresh = true;
            }
            if scope_chip(ui, "Zebra", self.config.zebra_exposure).clicked() {
                self.config.zebra_exposure = !self.config.zebra_exposure;
                self.needs_refresh = true;
            }
        });

        ui.add_space(4.0);

        // Live Pixel Sample Readout Pill (when hovering over the monitor)
        if let Some(sample) = &self.active_pixel_sample {
            egui::Frame::none()
                .fill(Color32::from_rgb(0x18, 0x1b, 0x24))
                .rounding(Rounding::same(4.0))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x32, 0x38, 0x4a)))
                .inner_margin(egui::Margin::symmetric(6.0, 3.0))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let (swatch, _) = ui.allocate_exact_size(Vec2::splat(11.0), Sense::hover());
                        ui.painter().rect_filled(
                            swatch,
                            2.0,
                            Color32::from_rgb(sample.rgb[0], sample.rgb[1], sample.rgb[2]),
                        );
                        ui.label(
                            RichText::new(format!(
                                "RGB({},{},{}) • {:.0} IRE • H:{:.0}° S:{:.0}% • xy({:.2},{:.2})",
                                sample.rgb[0],
                                sample.rgb[1],
                                sample.rgb[2],
                                sample.ire,
                                sample.hue_deg,
                                sample.sat * 100.0,
                                sample.cie_xy[0],
                                sample.cie_xy[1]
                            ))
                            .monospace()
                            .size(10.0)
                            .color(Color32::from_rgb(0xd8, 0xe0, 0xf0)),
                        );
                    });
                });
            ui.add_space(4.0);
        }

        if self.data.sample_count == 0 {
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new("Aktif görüntü karesi bekleniyor...")
                        .size(12.0)
                        .color(Color32::from_rgb(0x78, 0x7c, 0x8a)),
                );
            });
            return;
        }

        // Live exposure summary metrics
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!(
                    "Ort: {:.0} IRE  •  Tepe: {:.0} IRE",
                    self.data.avg_ire, self.data.peak_ire
                ))
                .size(11.0)
                .color(Color32::from_rgb(0xa8, 0xb2, 0xc4)),
            );
            if self.data.clipped_high_pct > 0.1 {
                ui.label(
                    RichText::new(format!("▲ Kırpılan: %{:.1}", self.data.clipped_high_pct))
                        .size(11.0)
                        .color(Color32::from_rgb(0xf8, 0x71, 0x71)),
                );
            }
            if self.data.crushed_low_pct > 0.1 {
                ui.label(
                    RichText::new(format!("▼ Ezilen: %{:.1}", self.data.crushed_low_pct))
                        .size(11.0)
                        .color(Color32::from_rgb(0x81, 0x8c, 0xf8)),
                );
            }
        });
        ui.add_space(4.0);

        ScrollArea::vertical()
            .id_salt("scopes_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let avail_w = ui.available_width().max(160.0);
                let sample_opt = self.active_pixel_sample.as_ref();

                if self.config.quad_layout {
                    let cell_w = ((avail_w - 8.0) * 0.5).max(80.0);
                    let cell_h = 126.0;
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            draw_scope_section_header(ui, "RGB Parade");
                            if let Some(tex) = &self.parade_tex {
                                draw_waveform_graticule(ui, tex, cell_w, cell_h, true, sample_opt);
                            }
                        });
                        ui.vertical(|ui| {
                            draw_scope_section_header(ui, "Luma Waveform");
                            if let Some(tex) = &self.waveform_tex {
                                draw_waveform_graticule(
                                    ui, tex, cell_w, cell_h, false, sample_opt,
                                );
                            }
                        });
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            draw_scope_section_header(ui, "Vectorscope");
                            if let Some(tex) = &self.vectorscope_tex {
                                draw_vectorscope_graticule(ui, tex, cell_w, sample_opt);
                            }
                        });
                        ui.vertical(|ui| {
                            if self.config.cie_1931 {
                                draw_scope_section_header(ui, "CIE 1931 xy");
                                if let Some(tex) = &self.cie_tex {
                                    draw_cie_graticule(ui, tex, cell_w, sample_opt);
                                }
                            } else {
                                draw_scope_section_header(ui, "Histogram");
                                draw_histogram_widget(
                                    ui,
                                    &self.data,
                                    self.config.histogram_luma,
                                    cell_w,
                                    cell_w.min(140.0),
                                );
                            }
                        });
                    });
                    ui.add_space(6.0);
                } else {
                    if self.config.rgb_parade {
                        draw_scope_section_header(ui, "RGB Parade (IRE)");
                        if let Some(tex) = &self.parade_tex {
                            draw_waveform_graticule(ui, tex, avail_w, 132.0, true, sample_opt);
                        }
                        ui.add_space(6.0);
                    }

                    if self.config.luma_waveform {
                        draw_scope_section_header(ui, "Luma Waveform (Rec.709 IRE)");
                        if let Some(tex) = &self.waveform_tex {
                            draw_waveform_graticule(ui, tex, avail_w, 132.0, false, sample_opt);
                        }
                        ui.add_space(6.0);
                    }

                    if self.config.vectorscope {
                        draw_scope_section_header(ui, "Vectorscope (Cb/Cr)");
                        if let Some(tex) = &self.vectorscope_tex {
                            draw_vectorscope_graticule(ui, tex, avail_w, sample_opt);
                        }
                        ui.add_space(6.0);
                    }

                    if self.config.cie_1931 {
                        draw_scope_section_header(ui, "CIE 1931 Chromaticity (Rec.709 Gamut)");
                        if let Some(tex) = &self.cie_tex {
                            draw_cie_graticule(ui, tex, avail_w, sample_opt);
                        }
                        ui.add_space(6.0);
                    }

                    if self.config.histogram {
                        ui.horizontal(|ui| {
                            draw_scope_section_header(ui, "Histogram");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.checkbox(&mut self.config.histogram_luma, "Luma");
                                },
                            );
                        });
                        draw_histogram_widget(
                            ui,
                            &self.data,
                            self.config.histogram_luma,
                            avail_w,
                            100.0,
                        );
                        ui.add_space(6.0);
                    }
                }

                if self.config.false_color {
                    draw_scope_section_header(ui, "False Color (Pozlama Haritası)");
                    if let Some(tex) = &self.false_color_tex {
                        let aspect = tex.size()[0] as f32 / tex.size()[1].max(1) as f32;
                        let h = (avail_w / aspect).clamp(90.0, 200.0);
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(avail_w, h), Sense::hover());
                        ui.painter().image(
                            tex.id(),
                            rect,
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                        draw_false_color_legend(ui);
                    }
                    ui.add_space(6.0);
                }

                if self.config.zebra_exposure {
                    ui.horizontal(|ui| {
                        draw_scope_section_header(ui, "Zebra Exposure");
                        let mut pct = self.config.zebra_threshold * 100.0;
                        if ui
                            .add(
                                egui::Slider::new(&mut pct, 50.0..=100.0)
                                    .suffix(" IRE")
                                    .max_decimals(0),
                            )
                            .changed()
                        {
                            self.config.zebra_threshold = (pct / 100.0).clamp(0.5, 1.0);
                            self.needs_refresh = true;
                        }
                    });
                    if let Some(tex) = &self.zebra_tex {
                        let aspect = tex.size()[0] as f32 / tex.size()[1].max(1) as f32;
                        let h = (avail_w / aspect).clamp(90.0, 200.0);
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(avail_w, h), Sense::hover());
                        ui.painter().image(
                            tex.id(),
                            rect,
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    ui.add_space(6.0);
                }

                // Reference Still Gallery (DaVinci Gallery Stills)
                ui.separator();
                ui.horizontal(|ui| {
                    draw_scope_section_header(ui, "Referans Kare Galerisi (Stills)");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(RichText::new("📸 Kare Yakala").size(10.5))
                            .on_hover_text(
                                "Mevcut monitör karesini ve renk ayarlarını referans galerisine kaydet",
                            )
                            .clicked()
                        {
                            self.request_grab_still = true;
                        }
                    });
                });
                if self.reference_stills.is_empty() {
                    ui.label(
                        RichText::new(
                            "Henüz referans kare yok. Monitör bölünmüş ekran (Wipe) karşılaştırması veya Shot Match için '📸 Kare Yakala' butonuna tıklayın.",
                        )
                        .size(10.0)
                        .color(Color32::from_rgb(0x78, 0x80, 0x92)),
                    );
                } else {
                    let mut delete_idx = None;
                    for (idx, still) in self.reference_stills.iter().enumerate() {
                        let is_selected = self.active_reference_still == Some(idx);
                        egui::Frame::none()
                            .fill(if is_selected {
                                Color32::from_rgb(0x20, 0x2a, 0x3c)
                            } else {
                                Color32::from_rgb(0x16, 0x18, 0x20)
                            })
                            .rounding(Rounding::same(5.0))
                            .stroke(Stroke::new(
                                1.0_f32,
                                if is_selected {
                                    Color32::from_rgb(0x4a, 0x90, 0xe2)
                                } else {
                                    Color32::from_rgb(0x28, 0x2c, 0x38)
                                },
                            ))
                            .inner_margin(egui::Margin::symmetric(6.0, 4.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if let Some(tex) = &still.tex {
                                        let (r, resp) = ui.allocate_exact_size(
                                            Vec2::new(56.0, 32.0),
                                            Sense::click(),
                                        );
                                        ui.painter().image(
                                            tex.id(),
                                            r,
                                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                            Color32::WHITE,
                                        );
                                        if resp.clicked() {
                                            self.active_reference_still = Some(idx);
                                        }
                                    }
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new(&still.label)
                                                .size(11.0)
                                                .strong()
                                                .color(Color32::WHITE),
                                        );
                                        ui.horizontal(|ui| {
                                            if ui
                                                .small_button(if is_selected {
                                                    "✓ Aktif Referans"
                                                } else {
                                                    "Referans Seç"
                                                })
                                                .clicked()
                                            {
                                                self.active_reference_still = Some(idx);
                                            }
                                            if still.grade_snapshot.is_some()
                                                && ui
                                                    .small_button("🎨 Rengi Uygula")
                                                    .on_hover_text(
                                                        "Bu referans karenin tüm renk ayarlarını seçili klibe kopyala",
                                                    )
                                                    .clicked()
                                            {
                                                self.pending_apply_still_idx = Some(idx);
                                            }
                                            if ui.small_button("🗑").clicked() {
                                                delete_idx = Some(idx);
                                            }
                                        });
                                    });
                                });
                            });
                        ui.add_space(3.0);
                    }
                    if let Some(del) = delete_idx {
                        self.reference_stills.remove(del);
                        if self.active_reference_still == Some(del) {
                            self.active_reference_still = None;
                        }
                    }
                }
            });
    }
}

fn scope_chip(ui: &mut Ui, label: &str, active: bool) -> egui::Response {
    let fill = if active {
        Color32::from_rgb(0x28, 0x48, 0x70)
    } else {
        Color32::from_rgb(0x22, 0x24, 0x2e)
    };
    let text_col = if active {
        Color32::WHITE
    } else {
        Color32::from_rgb(0x98, 0x9c, 0xa8)
    };
    ui.add(
        egui::Button::new(RichText::new(label).size(11.0).color(text_col))
            .fill(fill)
            .rounding(Rounding::same(4.0)),
    )
}

fn draw_scope_section_header(ui: &mut Ui, title: &str) {
    ui.label(
        RichText::new(title)
            .size(11.5)
            .strong()
            .color(Color32::from_rgb(0xb8, 0xc0, 0xd0)),
    );
}

fn set_or_load_tex(
    ctx: &egui::Context,
    slot: &mut Option<TextureHandle>,
    name: &str,
    img: ColorImage,
) {
    if let Some(tex) = slot {
        tex.set(img, TextureOptions::LINEAR);
    } else {
        *slot = Some(ctx.load_texture(name, img, TextureOptions::LINEAR));
    }
}

fn downsample_rgba(
    rgba: &[u8],
    src_w: u32,
    src_h: u32,
    max_w: u32,
    max_h: u32,
) -> (u32, u32, Vec<u8>) {
    if src_w <= max_w && src_h <= max_h {
        return (src_w, src_h, rgba.to_vec());
    }
    let scale = (max_w as f32 / src_w as f32).min(max_h as f32 / src_h as f32);
    let dst_w = ((src_w as f32 * scale).round() as u32).max(2);
    let dst_h = ((src_h as f32 * scale).round() as u32).max(2);
    let mut out = vec![0u8; (dst_w * dst_h * 4) as usize];
    for y in 0..dst_h {
        let sy = ((y as u64 * src_h as u64) / dst_h as u64).min((src_h - 1) as u64) as usize;
        for x in 0..dst_w {
            let sx = ((x as u64 * src_w as u64) / dst_w as u64).min((src_w - 1) as u64) as usize;
            let si = (sy * src_w as usize + sx) * 4;
            let di = ((y * dst_w + x) * 4) as usize;
            out[di..di + 4].copy_from_slice(&rgba[si..si + 4]);
        }
    }
    (dst_w, dst_h, out)
}

#[inline]
fn intensity_gain(hits: u16, peak: u16) -> f32 {
    if hits == 0 {
        return 0.0;
    }
    let norm = (hits as f32 / (peak.max(1) as f32 * 0.25).max(1.0)).clamp(0.0, 1.0);
    norm.powf(0.45).clamp(0.18, 1.0)
}

fn build_parade_image(data: &ScopeData) -> ColorImage {
    let total_w = PARADE_CH_W * 3;
    let h = WAVEFORM_H;
    let mut pixels = vec![0u8; total_w * h * 4];
    for y_img in 0..h {
        let y_ire = h - 1 - y_img;
        for x in 0..PARADE_CH_W {
            let src_idx = y_ire * PARADE_CH_W + x;

            let gr = intensity_gain(data.parade_r[src_idx], data.parade_peak);
            let ir = (y_img * total_w + x) * 4;
            pixels[ir] = (gr * 255.0) as u8;
            pixels[ir + 1] = (gr * 45.0) as u8;
            pixels[ir + 2] = (gr * 45.0) as u8;
            pixels[ir + 3] = 255;

            let gg = intensity_gain(data.parade_g[src_idx], data.parade_peak);
            let ig = (y_img * total_w + PARADE_CH_W + x) * 4;
            pixels[ig] = (gg * 45.0) as u8;
            pixels[ig + 1] = (gg * 255.0) as u8;
            pixels[ig + 2] = (gg * 65.0) as u8;
            pixels[ig + 3] = 255;

            let gb = intensity_gain(data.parade_b[src_idx], data.parade_peak);
            let ib = (y_img * total_w + PARADE_CH_W * 2 + x) * 4;
            pixels[ib] = (gb * 60.0) as u8;
            pixels[ib + 1] = (gb * 125.0) as u8;
            pixels[ib + 2] = (gb * 255.0) as u8;
            pixels[ib + 3] = 255;
        }
    }
    ColorImage::from_rgba_unmultiplied([total_w, h], &pixels)
}

fn build_luma_waveform_image(data: &ScopeData) -> ColorImage {
    let w = WAVEFORM_W;
    let h = WAVEFORM_H;
    let mut pixels = vec![0u8; w * h * 4];
    for y_img in 0..h {
        let y_ire = h - 1 - y_img;
        for x in 0..w {
            let hits = data.luma_waveform[y_ire * w + x];
            let g = intensity_gain(hits, data.luma_peak);
            let dst = (y_img * w + x) * 4;
            pixels[dst] = (g * 195.0) as u8;
            pixels[dst + 1] = (g * 245.0) as u8;
            pixels[dst + 2] = (g * 220.0) as u8;
            pixels[dst + 3] = 255;
        }
    }
    ColorImage::from_rgba_unmultiplied([w, h], &pixels)
}

fn build_vectorscope_image(data: &ScopeData) -> ColorImage {
    let sz = VECTORSCOPE_SIZE;
    let mut pixels = vec![0u8; sz * sz * 4];
    for i in 0..(sz * sz) {
        let hits = data.vectorscope_hits[i];
        let dst = i * 4;
        if hits == 0 {
            pixels[dst] = 10;
            pixels[dst + 1] = 11;
            pixels[dst + 2] = 15;
            pixels[dst + 3] = 255;
        } else {
            let gain = intensity_gain(hits, data.vectorscope_peak);
            let sum_rgb = data.vectorscope_rgb[i];
            let avg_r = (sum_rgb[0] as f32 / hits as f32).clamp(40.0, 255.0);
            let avg_g = (sum_rgb[1] as f32 / hits as f32).clamp(40.0, 255.0);
            let avg_b = (sum_rgb[2] as f32 / hits as f32).clamp(40.0, 255.0);
            pixels[dst] = (avg_r * gain).clamp(0.0, 255.0) as u8;
            pixels[dst + 1] = (avg_g * gain).clamp(0.0, 255.0) as u8;
            pixels[dst + 2] = (avg_b * gain).clamp(0.0, 255.0) as u8;
            pixels[dst + 3] = 255;
        }
    }
    ColorImage::from_rgba_unmultiplied([sz, sz], &pixels)
}

fn build_cie_image(data: &ScopeData) -> ColorImage {
    let sz = CIE_SIZE;
    let mut pixels = vec![0u8; sz * sz * 4];
    for i in 0..(sz * sz) {
        let hits = data.cie_hits[i];
        let dst = i * 4;
        if hits == 0 {
            pixels[dst] = 10;
            pixels[dst + 1] = 11;
            pixels[dst + 2] = 15;
            pixels[dst + 3] = 255;
        } else {
            let gain = intensity_gain(hits, data.cie_peak);
            let sum_rgb = data.cie_rgb[i];
            let avg_r = (sum_rgb[0] as f32 / hits as f32).clamp(55.0, 255.0);
            let avg_g = (sum_rgb[1] as f32 / hits as f32).clamp(55.0, 255.0);
            let avg_b = (sum_rgb[2] as f32 / hits as f32).clamp(55.0, 255.0);
            pixels[dst] = (avg_r * gain).clamp(0.0, 255.0) as u8;
            pixels[dst + 1] = (avg_g * gain).clamp(0.0, 255.0) as u8;
            pixels[dst + 2] = (avg_b * gain).clamp(0.0, 255.0) as u8;
            pixels[dst + 3] = 255;
        }
    }
    ColorImage::from_rgba_unmultiplied([sz, sz], &pixels)
}

fn draw_waveform_graticule(
    ui: &mut Ui,
    tex: &TextureHandle,
    width: f32,
    height: f32,
    is_parade: bool,
    sample: Option<&PixelSampleReadout>,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(rect, 4.0, Color32::from_rgb(10, 11, 15));
    let inner = rect.shrink2(Vec2::new(26.0, 6.0));
    painter.image(
        tex.id(),
        inner,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );

    let line_col = Color32::from_rgba_unmultiplied(180, 190, 210, 42);
    let text_col = Color32::from_rgb(130, 140, 160);
    for (ire, label) in [
        (1.0_f32, "100"),
        (0.75, "75"),
        (0.50, "50"),
        (0.25, "25"),
        (0.0, "0"),
    ] {
        let y = inner.bottom() - ire * inner.height();
        painter.line_segment(
            [Pos2::new(inner.left(), y), Pos2::new(inner.right(), y)],
            Stroke::new(1.0_f32, line_col),
        );
        painter.text(
            Pos2::new(rect.left() + 22.0, y),
            egui::Align2::RIGHT_CENTER,
            label,
            FontId::monospace(9.5),
            text_col,
        );
    }

    if is_parade {
        let third = inner.width() / 3.0;
        for c in 1..3 {
            let x = inner.left() + third * (c as f32);
            painter.line_segment(
                [Pos2::new(x, inner.top()), Pos2::new(x, inner.bottom())],
                Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(180, 190, 210, 55)),
            );
        }
        if let Some(s) = sample {
            for (ch, val, col) in [
                (0, s.rgb[0], Color32::from_rgb(255, 110, 110)),
                (1, s.rgb[1], Color32::from_rgb(110, 255, 130)),
                (2, s.rgb[2], Color32::from_rgb(110, 175, 255)),
            ] {
                let x = inner.left() + third * (ch as f32 + s.x_norm.clamp(0.0, 1.0));
                let y = inner.bottom() - (val as f32 / 255.0) * inner.height();
                painter.circle_stroke(Pos2::new(x, y), 4.5, Stroke::new(1.5_f32, col));
            }
        }
    } else if let Some(s) = sample {
        let x = inner.left() + s.x_norm.clamp(0.0, 1.0) * inner.width();
        let y = inner.bottom() - (s.ire / 100.0).clamp(0.0, 1.0) * inner.height();
        painter.circle_stroke(
            Pos2::new(x, y),
            4.5,
            Stroke::new(1.5_f32, Color32::from_rgb(255, 235, 100)),
        );
    }

    painter.rect_stroke(
        inner,
        2.0,
        Stroke::new(1.0_f32, Color32::from_rgb(45, 48, 58)),
    );
}

fn draw_vectorscope_graticule(
    ui: &mut Ui,
    tex: &TextureHandle,
    avail_w: f32,
    sample: Option<&PixelSampleReadout>,
) {
    let size = avail_w.min(200.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, size), Sense::hover());
    let scope_rect = Rect::from_center_size(rect.center(), Vec2::splat(size));
    let painter = ui.painter();

    painter.rect_filled(rect, 4.0, Color32::from_rgb(10, 11, 15));
    painter.image(
        tex.id(),
        scope_rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );

    let center = scope_rect.center();
    let radius = size * 0.46;
    let ring_col = Color32::from_rgba_unmultiplied(170, 185, 205, 55);
    painter.circle_stroke(center, radius, Stroke::new(1.0_f32, ring_col));
    painter.circle_stroke(center, radius * 0.75, Stroke::new(1.0_f32, ring_col));

    // Crosshairs
    painter.line_segment(
        [
            Pos2::new(center.x - radius, center.y),
            Pos2::new(center.x + radius, center.y),
        ],
        Stroke::new(1.0_f32, ring_col),
    );
    painter.line_segment(
        [
            Pos2::new(center.x, center.y - radius),
            Pos2::new(center.x, center.y + radius),
        ],
        Stroke::new(1.0_f32, ring_col),
    );

    // Skin-tone line (I-line ~123 deg in Cb/Cr plane)
    let angle = (123.0_f32).to_radians();
    let skin_end = Pos2::new(
        center.x + angle.cos() * radius,
        center.y - angle.sin() * radius,
    );
    painter.line_segment(
        [center, skin_end],
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(245, 165, 120, 90)),
    );

    // 75% Rec.709 color target boxes
    for (label, cb, cr) in vectorscope_targets_75() {
        let tx = center.x + cb * (size * 0.92);
        let ty = center.y - cr * (size * 0.92);
        let box_r = Rect::from_center_size(Pos2::new(tx, ty), Vec2::splat(7.0));
        painter.rect_stroke(
            box_r,
            1.0,
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(210, 220, 235, 140)),
        );
        painter.text(
            Pos2::new(tx + 6.0, ty - 6.0),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::monospace(9.0),
            Color32::from_rgb(180, 190, 210),
        );
    }

    if let Some(s) = sample {
        let sx = center.x + s.cb * (size * 0.92);
        let sy = center.y - s.cr * (size * 0.92);
        painter.circle_stroke(
            Pos2::new(sx, sy),
            5.0,
            Stroke::new(1.6_f32, Color32::from_rgb(255, 240, 110)),
        );
    }
}

fn draw_cie_graticule(
    ui: &mut Ui,
    tex: &TextureHandle,
    avail_w: f32,
    sample: Option<&PixelSampleReadout>,
) {
    let size = avail_w.min(200.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, size), Sense::hover());
    let scope_rect = Rect::from_center_size(rect.center(), Vec2::splat(size));
    let painter = ui.painter();

    painter.rect_filled(rect, 4.0, Color32::from_rgb(10, 11, 15));
    painter.image(
        tex.id(),
        scope_rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );

    let map_xy = |xy: [f32; 2]| -> Pos2 {
        let u = (xy[0] / 0.80).clamp(0.0, 1.0);
        let v = (xy[1] / 0.90).clamp(0.0, 1.0);
        Pos2::new(
            scope_rect.left() + u * scope_rect.width(),
            scope_rect.bottom() - v * scope_rect.height(),
        )
    };

    let [r_xy, g_xy, b_xy] = rec709_cie1931_primaries();
    let pr = map_xy([r_xy.0, r_xy.1]);
    let pg = map_xy([g_xy.0, g_xy.1]);
    let pb = map_xy([b_xy.0, b_xy.1]);
    let pw = map_xy([0.3127, 0.3290]);

    let tri_col = Color32::from_rgba_unmultiplied(210, 225, 245, 115);
    painter.line_segment([pr, pg], Stroke::new(1.2_f32, tri_col));
    painter.line_segment([pg, pb], Stroke::new(1.2_f32, tri_col));
    painter.line_segment([pb, pr], Stroke::new(1.2_f32, tri_col));
    painter.circle_stroke(pw, 3.5, Stroke::new(1.2_f32, Color32::WHITE));

    if let Some(s) = sample {
        let pt = map_xy(s.cie_xy);
        painter.circle_stroke(
            pt,
            5.0,
            Stroke::new(1.6_f32, Color32::from_rgb(255, 235, 90)),
        );
    }

    painter.rect_stroke(
        scope_rect,
        2.0,
        Stroke::new(1.0_f32, Color32::from_rgb(45, 48, 58)),
    );
}

fn draw_histogram_widget(ui: &mut Ui, data: &ScopeData, show_luma: bool, width: f32, height: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, Color32::from_rgb(10, 11, 15));
    let inner = rect.shrink2(Vec2::new(4.0, 4.0));

    let peak = (data.hist_peak as f32).max(1.0);
    let draw_curve = |bins: &[u32; 256], stroke_col: Color32| {
        let mut pts = Vec::with_capacity(256);
        for (i, &val) in bins.iter().enumerate() {
            let x = inner.left() + (i as f32 / 255.0) * inner.width();
            let norm = (val as f32 / peak).powf(0.5).clamp(0.0, 1.0);
            let y = inner.bottom() - norm * inner.height();
            pts.push(Pos2::new(x, y));
        }
        painter.add(egui::Shape::line(pts, Stroke::new(1.3_f32, stroke_col)));
    };

    draw_curve(
        &data.hist_r,
        Color32::from_rgba_unmultiplied(245, 70, 70, 200),
    );
    draw_curve(
        &data.hist_g,
        Color32::from_rgba_unmultiplied(70, 235, 95, 200),
    );
    draw_curve(
        &data.hist_b,
        Color32::from_rgba_unmultiplied(80, 145, 255, 200),
    );
    if show_luma {
        draw_curve(
            &data.hist_luma,
            Color32::from_rgba_unmultiplied(235, 235, 240, 190),
        );
    }

    painter.rect_stroke(
        inner,
        2.0,
        Stroke::new(1.0_f32, Color32::from_rgb(45, 48, 58)),
    );
}

fn draw_false_color_legend(ui: &mut Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 2.0);
        for (col, lbl) in [
            (Color32::from_rgb(110, 0, 190), "<4%"),
            (Color32::from_rgb(25, 65, 225), "10%"),
            (Color32::from_rgb(0, 175, 215), "25%"),
            (Color32::from_rgb(40, 205, 65), "45% Gri"),
            (Color32::from_rgb(248, 135, 175), "65% Ten"),
            (Color32::from_rgb(250, 220, 25), "90%"),
            (Color32::from_rgb(245, 30, 30), "97%+"),
        ] {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(9.0), Sense::hover());
                ui.painter().rect_filled(r, 2.0, col);
                ui.label(
                    RichText::new(lbl)
                        .size(9.5)
                        .color(Color32::from_rgb(175, 180, 195)),
                );
            });
        }
    });
}
