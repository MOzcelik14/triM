//! Independent, resizable Color Scopes panel (RGB Parade, Luma Waveform,
//! Vectorscope, Histogram, False Color, Zebra Exposure).

use egui::{
    Color32, ColorImage, FontId, Pos2, Rect, RichText, Rounding, ScrollArea, Sense, Stroke,
    TextureHandle, TextureOptions, Ui, Vec2,
};
use trim_core::scopes::{
    compute_false_color_rgba, compute_scopes_rgba, compute_zebra_rgba, vectorscope_targets_75,
    ScopeConfig, ScopeData, PARADE_CH_W, VECTORSCOPE_SIZE, WAVEFORM_H, WAVEFORM_W,
};

pub struct ScopesPanelState {
    pub is_open: bool,
    pub config: ScopeConfig,
    pub data: ScopeData,
    pub last_eval_time: f64,
    pub needs_refresh: bool,
    parade_tex: Option<TextureHandle>,
    waveform_tex: Option<TextureHandle>,
    vectorscope_tex: Option<TextureHandle>,
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
            parade_tex: None,
            waveform_tex: None,
            vectorscope_tex: None,
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
    }

    /// Recomputes active scopes from real RGBA pixel data and updates cached textures.
    pub fn update_from_rgba(&mut self, ctx: &egui::Context, rgba: &[u8], width: u32, height: u32) {
        if width == 0 || height == 0 || rgba.len() < (width as usize) * (height as usize) * 4 {
            return;
        }
        self.data = compute_scopes_rgba(rgba, width, height);
        self.needs_refresh = false;

        if self.config.rgb_parade {
            let img = build_parade_image(&self.data);
            set_or_load_tex(ctx, &mut self.parade_tex, "scope_parade", img);
        }
        if self.config.luma_waveform {
            let img = build_luma_waveform_image(&self.data);
            set_or_load_tex(ctx, &mut self.waveform_tex, "scope_waveform", img);
        }
        if self.config.vectorscope {
            let img = build_vectorscope_image(&self.data);
            set_or_load_tex(ctx, &mut self.vectorscope_tex, "scope_vectorscope", img);
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

                if self.config.rgb_parade {
                    draw_scope_section_header(ui, "RGB Parade (IRE)");
                    if let Some(tex) = &self.parade_tex {
                        draw_waveform_graticule(ui, tex, avail_w, 132.0, true);
                    }
                    ui.add_space(6.0);
                }

                if self.config.luma_waveform {
                    draw_scope_section_header(ui, "Luma Waveform (Rec.709 IRE)");
                    if let Some(tex) = &self.waveform_tex {
                        draw_waveform_graticule(ui, tex, avail_w, 132.0, false);
                    }
                    ui.add_space(6.0);
                }

                if self.config.vectorscope {
                    draw_scope_section_header(ui, "Vectorscope (Cb/Cr)");
                    if let Some(tex) = &self.vectorscope_tex {
                        draw_vectorscope_graticule(ui, tex, avail_w);
                    }
                    ui.add_space(6.0);
                }

                if self.config.histogram {
                    ui.horizontal(|ui| {
                        draw_scope_section_header(ui, "Histogram");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.checkbox(&mut self.config.histogram_luma, "Luma");
                        });
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

fn draw_waveform_graticule(
    ui: &mut Ui,
    tex: &TextureHandle,
    width: f32,
    height: f32,
    is_parade: bool,
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
    }

    painter.rect_stroke(
        inner,
        2.0,
        Stroke::new(1.0_f32, Color32::from_rgb(45, 48, 58)),
    );
}

fn draw_vectorscope_graticule(ui: &mut Ui, tex: &TextureHandle, avail_w: f32) {
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
