//! DaVinci Resolve / Kdenlive inspired modern cinematic startup splash screen for triM.

use egui::{
    Align, Align2, Area, Color32, FontId, Frame, Id, Key, Layout, Margin, Order, Pos2, Rect,
    RichText, Rounding, Shadow, Stroke, Vec2,
};
use std::time::Instant;

const ICON_BYTES: &[u8] = include_bytes!("../../../../resources/icons/trim_256x256.png");

/// Represents the startup splash screen state and animated lifecycle.
pub struct SplashScreen {
    pub is_active: bool,
    start_time: Option<Instant>,
    pub duration_secs: f32,
    logo_texture: Option<egui::TextureHandle>,
    steps: [(&'static str, f32); 5],
}

impl std::fmt::Debug for SplashScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SplashScreen")
            .field("is_active", &self.is_active)
            .field("duration_secs", &self.duration_secs)
            .finish()
    }
}

impl Default for SplashScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl SplashScreen {
    pub fn new() -> Self {
        Self {
            is_active: true,
            start_time: None,
            duration_secs: 1.8,
            logo_texture: None,
            steps: [
                (
                    "Grafik alt yapısı ve GPU gölgelendiricileri başlatılıyor...",
                    0.20,
                ),
                (
                    "FFmpeg medya motoru ve donanım kodlayıcıları taranıyor...",
                    0.45,
                ),
                (
                    "Düşük gecikmeli ses motoru ve master limiter yükleniyor...",
                    0.70,
                ),
                (
                    "Zaman çizelgesi ve çoklu sekans motoru hazırlanıyor...",
                    0.90,
                ),
                ("Çalışma alanı hazır.", 1.00),
            ],
        }
    }

    /// Dismiss the splash screen immediately.
    pub fn dismiss(&mut self) {
        self.is_active = false;
    }

    /// Render the splash screen modal over the entire window.
    /// Returns `true` while the splash screen is active and rendering.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        if !self.is_active {
            return false;
        }

        let start = *self.start_time.get_or_insert_with(Instant::now);
        let elapsed = start.elapsed().as_secs_f32();

        // Check if finished by timeout
        if elapsed >= self.duration_secs {
            self.dismiss();
            return false;
        }

        // Check user skip input: Esc, Space, Enter, or mouse click
        let input = ctx.input(|i| {
            i.key_pressed(Key::Escape)
                || i.key_pressed(Key::Space)
                || i.key_pressed(Key::Enter)
                || i.pointer.any_pressed()
        });
        if input {
            self.dismiss();
            return false;
        }

        // Keep 60fps animations smooth during splash
        ctx.request_repaint();

        let progress = (elapsed / self.duration_secs).clamp(0.0, 1.0);

        // Ensure logo texture is loaded
        if self.logo_texture.is_none() {
            if let Ok(img) = image::load_from_memory(ICON_BYTES) {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [w as usize, h as usize],
                    &rgba.into_raw(),
                );
                self.logo_texture = Some(ctx.load_texture(
                    "splash_logo",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }

        let screen_rect = ctx.screen_rect();

        // Dark full-screen backdrop overlay
        Area::new(Id::new("splash_backdrop"))
            .order(Order::Foreground)
            .fixed_pos(Pos2::ZERO)
            .interactable(true)
            .show(ctx, |ui| {
                ui.painter().rect_filled(
                    screen_rect,
                    Rounding::ZERO,
                    Color32::from_black_alpha(235),
                );
            });

        // Centered splash banner card
        Area::new(Id::new("splash_card"))
            .order(Order::Tooltip)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                Frame::none()
                    .fill(Color32::from_rgb(15, 17, 23))
                    .rounding(Rounding::same(14.0))
                    .stroke(Stroke::new(
                        1.0_f32,
                        Color32::from_rgba_unmultiplied(46, 170, 220, 85),
                    ))
                    .shadow(Shadow {
                        offset: Vec2::new(0.0, 16.0),
                        blur: 36.0,
                        spread: 4.0,
                        color: Color32::from_black_alpha(220),
                    })
                    .inner_margin(Margin::same(28.0))
                    .show(ui, |ui| {
                        ui.set_width(580.0);
                        ui.set_height(290.0);

                        ui.vertical(|ui| {
                            // Top branding header row
                            ui.horizontal(|ui| {
                                // App Icon
                                if let Some(tex) = &self.logo_texture {
                                    ui.image((tex.id(), Vec2::new(72.0, 72.0)));
                                } else {
                                    let (rect, _) = ui.allocate_exact_size(
                                        Vec2::new(72.0, 72.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(
                                        rect,
                                        Rounding::same(12.0),
                                        Color32::from_rgb(26, 30, 42),
                                    );
                                }

                                ui.add_space(18.0);

                                // Title and Taglines
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("triM")
                                                .font(FontId::proportional(34.0))
                                                .strong()
                                                .color(Color32::WHITE),
                                        );
                                        ui.label(
                                            RichText::new(".")
                                                .font(FontId::proportional(34.0))
                                                .strong()
                                                .color(Color32::from_rgb(46, 170, 220)),
                                        );
                                        ui.add_space(8.0);

                                        // Version Pill Badge
                                        Frame::none()
                                            .fill(Color32::from_rgba_unmultiplied(46, 170, 220, 25))
                                            .rounding(Rounding::same(12.0))
                                            .stroke(Stroke::new(
                                                1.0_f32,
                                                Color32::from_rgba_unmultiplied(46, 170, 220, 70),
                                            ))
                                            .inner_margin(Margin::symmetric(10.0, 4.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "v{}",
                                                        env!("CARGO_PKG_VERSION")
                                                    ))
                                                    .font(FontId::monospace(11.5))
                                                    .strong()
                                                    .color(Color32::from_rgb(70, 195, 245)),
                                                );
                                            });
                                    });

                                    ui.add_space(2.0);
                                    ui.label(
                                        RichText::new("HAFİF VİDEO DÜZENLEYİCİ")
                                            .font(FontId::proportional(11.0))
                                            .strong()
                                            .color(Color32::from_rgb(130, 150, 175)),
                                    );

                                    ui.label(
                                        RichText::new("NATIVE RUST & FFMPEG NLE ENGINE")
                                            .font(FontId::monospace(10.0))
                                            .color(Color32::from_rgb(85, 105, 130)),
                                    );
                                });
                            });

                            ui.add_space(22.0);

                            // Subtle separator
                            let (sep_rect, _) =
                                ui.allocate_exact_size(Vec2::new(580.0, 1.0), egui::Sense::hover());
                            ui.painter().rect_filled(
                                sep_rect,
                                Rounding::ZERO,
                                Color32::from_rgba_unmultiplied(255, 255, 255, 15),
                            );

                            ui.add_space(22.0);

                            // High-tech engine status info
                            let active_step_msg = self
                                .steps
                                .iter()
                                .find(|(_, threshold)| progress <= *threshold)
                                .map(|(msg, _)| *msg)
                                .unwrap_or("Çalışma alanı hazır.");

                            ui.horizontal(|ui| {
                                // Pulsing cyan status indicator dot
                                let pulse =
                                    ((elapsed * 6.0).sin().abs() * 0.5 + 0.5).clamp(0.2, 1.0);
                                let dot_color = Color32::from_rgba_unmultiplied(
                                    46,
                                    170,
                                    220,
                                    (pulse * 255.0) as u8,
                                );
                                let (dot_rect, _) = ui.allocate_exact_size(
                                    Vec2::new(10.0, 10.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter()
                                    .circle_filled(dot_rect.center(), 4.0, dot_color);

                                ui.add_space(6.0);

                                ui.label(
                                    RichText::new(active_step_msg)
                                        .font(FontId::proportional(13.0))
                                        .color(Color32::from_rgb(215, 225, 240)),
                                );

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("{:.0}%", progress * 100.0))
                                            .font(FontId::monospace(12.5))
                                            .strong()
                                            .color(Color32::from_rgb(46, 170, 220)),
                                    );
                                });
                            });

                            ui.add_space(10.0);

                            // Modern progress bar
                            let bar_width = 580.0;
                            let bar_height = 5.0;
                            let (bar_rect, _) = ui.allocate_exact_size(
                                Vec2::new(bar_width, bar_height),
                                egui::Sense::hover(),
                            );

                            // Track background
                            ui.painter().rect_filled(
                                bar_rect,
                                Rounding::same(2.5),
                                Color32::from_rgb(28, 32, 44),
                            );

                            // Filled portion
                            let fill_w = bar_width * progress;
                            if fill_w > 0.0 {
                                let fill_rect = Rect::from_min_size(
                                    bar_rect.min,
                                    Vec2::new(fill_w, bar_height),
                                );
                                ui.painter().rect_filled(
                                    fill_rect,
                                    Rounding::same(2.5),
                                    Color32::from_rgb(46, 170, 220),
                                );
                            }

                            ui.add_space(26.0);

                            // Footer info
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Geliştirici: Murat Özçelik")
                                        .font(FontId::proportional(11.0))
                                        .color(Color32::from_rgb(110, 125, 145)),
                                );

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new("Geçmek için tıklayın veya [Esc]")
                                            .font(FontId::proportional(11.0))
                                            .color(Color32::from_rgb(125, 140, 165)),
                                    );
                                });
                            });
                        });
                    });
            });

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_splash_screen_initial_state() {
        let splash = SplashScreen::new();
        assert!(splash.is_active);
        assert_eq!(splash.duration_secs, 1.8);
    }

    #[test]
    fn test_splash_screen_dismiss() {
        let mut splash = SplashScreen::new();
        assert!(splash.is_active);
        splash.dismiss();
        assert!(!splash.is_active);
    }
}
