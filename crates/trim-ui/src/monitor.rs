//! Program Monitor widget rendering the active video frame, viewport boundary, and playback status.

use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, TextureHandle, Ui, Vec2};

pub enum MonitorAction {
    NewProject,
    OpenProject,
    ImportMedia,
}

pub struct MonitorWidget<'a> {
    texture: Option<&'a TextureHandle>,
    has_clips: bool,
    aspect_ratio: f32,
    timecode: &'a str,
    duration_timecode: &'a str,
    is_playing: bool,
    playback_speed: f64,
    media_label: Option<&'a str>,
    is_decoding: bool,
}

impl<'a> MonitorWidget<'a> {
    pub fn new(
        texture: Option<&'a TextureHandle>,
        has_clips: bool,
        aspect_ratio: f32,
        timecode: &'a str,
        duration_timecode: &'a str,
        is_playing: bool,
        playback_speed: f64,
        media_label: Option<&'a str>,
        is_decoding: bool,
    ) -> Self {
        Self {
            texture,
            has_clips,
            aspect_ratio: if aspect_ratio > 0.1 { aspect_ratio } else { 16.0 / 9.0 },
            timecode,
            duration_timecode,
            is_playing,
            playback_speed,
            media_label,
            is_decoding,
        }
    }

    pub fn show(self, ui: &mut Ui) -> (Response, Option<MonitorAction>) {
        let avail_w = ui.available_width().max(160.0);
        let avail_h = ui.available_height().max(100.0);
        let (rect, response) = ui.allocate_exact_size(Vec2::new(avail_w, avail_h), Sense::hover());
        let painter = ui.painter();

        // Theater dark monitor surround
        painter.rect_filled(rect, 4.0, Color32::from_rgb(0x0a, 0x0a, 0x0d));
        painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(0x1e, 0x1e, 0x26)));

        let mut action = None;

        if !self.has_clips {
            // Empty state overlay
            let center = rect.center();

            painter.text(
                Pos2::new(center.x, center.y - 50.0),
                egui::Align2::CENTER_CENTER,
                "triM.",
                egui::FontId::proportional(32.0),
                Color32::from_rgb(0xe0, 0x7a, 0x38),
            );

            painter.text(
                Pos2::new(center.x, center.y - 15.0),
                egui::Align2::CENTER_CENTER,
                "Zaman çizgisinde henüz klip yok",
                egui::FontId::proportional(14.0),
                Color32::from_rgb(0x90, 0x90, 0xa0),
            );

            let btn_y = center.y + 30.0;
            let btn_w = 120.0;
            let btn_h = 30.0;

            let r_import = Rect::from_center_size(Pos2::new(center.x, btn_y), Vec2::new(btn_w + 40.0, btn_h));
            let r_new = Rect::from_center_size(Pos2::new(center.x - btn_w - 30.0, btn_y), Vec2::new(btn_w, btn_h));
            let r_open = Rect::from_center_size(Pos2::new(center.x + btn_w + 30.0, btn_y), Vec2::new(btn_w, btn_h));

            if let Some(mouse_pos) = response.hover_pos() {
                if ui.input(|i| i.pointer.primary_clicked()) {
                    if r_import.contains(mouse_pos) {
                        action = Some(MonitorAction::ImportMedia);
                    } else if r_new.contains(mouse_pos) {
                        action = Some(MonitorAction::NewProject);
                    } else if r_open.contains(mouse_pos) {
                        action = Some(MonitorAction::OpenProject);
                    }
                }
            }

            draw_button(&painter, r_new, "Yeni Proje", false);
            draw_button(&painter, r_import, "+ Medya İçe Aktar", true);
            draw_button(&painter, r_open, "Proje Aç", false);

            return (response, action);
        }

        // Active Monitor Viewport
        let bottom_bar_h = 26.0;
        let viewport_rect = Rect::from_min_max(
            Pos2::new(rect.left(), rect.top()),
            Pos2::new(rect.right(), rect.bottom() - bottom_bar_h),
        );

        let max_w = (viewport_rect.width() - 16.0).max(10.0);
        let max_h = (viewport_rect.height() - 16.0).max(10.0);
        let (target_w, target_h) = fit_aspect(max_w, max_h, self.aspect_ratio);

        let frame_rect = Rect::from_center_size(viewport_rect.center(), Vec2::new(target_w, target_h));

        // Letterbox viewport deep black background
        painter.rect_filled(frame_rect, 0.0, Color32::from_rgb(0x02, 0x02, 0x04));

        if let Some(tex) = self.texture {
            painter.image(
                tex.id(),
                frame_rect,
                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            painter.text(
                frame_rect.center(),
                egui::Align2::CENTER_CENTER,
                if self.is_decoding { "Kare hazırlanıyor..." } else { "🎬 Önizleme Yükleniyor..." },
                egui::FontId::proportional(14.0),
                Color32::from_rgb(0xa0, 0xa0, 0xb0),
            );
        }

        // High-contrast video edge stroke (ensures frame boundaries are clearly visible even for dark video frames)
        painter.rect_stroke(
            frame_rect,
            0.0,
            Stroke::new(1.0_f32, Color32::from_rgb(0x4a, 0x4a, 0x5e)),
        );

        // Subtle 16:9 safe zone corner cues
        draw_corner_guides(&painter, frame_rect);

        // -------------------------------------------------------------
        // Bottom Monitor Status Bar
        // -------------------------------------------------------------
        let bar_rect = Rect::from_min_max(
            Pos2::new(rect.left(), rect.bottom() - bottom_bar_h),
            Pos2::new(rect.right(), rect.bottom()),
        );

        painter.rect_filled(bar_rect, 4.0, Color32::from_rgb(0x11, 0x11, 0x16));
        painter.line_segment(
            [bar_rect.left_top(), bar_rect.right_top()],
            Stroke::new(1.0_f32, Color32::from_rgb(0x22, 0x22, 0x2c)),
        );

        // Left: Timecode / Duration
        let tc_str = format!("{} / {}", self.timecode, self.duration_timecode);
        painter.text(
            Pos2::new(bar_rect.left() + 10.0, bar_rect.center().y),
            egui::Align2::LEFT_CENTER,
            tc_str,
            egui::FontId::monospace(11.0),
            Color32::from_rgb(0xe0, 0x7a, 0x38),
        );

        // Center: Playback Status Badge
        let status_text = if self.is_playing {
            if (self.playback_speed - 1.0).abs() < 0.01 {
                "▶  Oynatılıyor (1.0x)".to_string()
            } else {
                format!("▶  Oynatılıyor ({:.1}x)", self.playback_speed)
            }
        } else {
            "⏸  Duraklatıldı".to_string()
        };

        let status_color = if self.is_playing {
            Color32::from_rgb(0x50, 0xd0, 0x70)
        } else {
            Color32::from_rgb(0x80, 0x80, 0x90)
        };

        painter.text(
            bar_rect.center(),
            egui::Align2::CENTER_CENTER,
            status_text,
            egui::FontId::proportional(11.0),
            status_color,
        );

        // Right: Format badge / Media label
        let label = self.media_label.unwrap_or("Önizleme • 640x360");
        painter.text(
            Pos2::new(bar_rect.right() - 10.0, bar_rect.center().y),
            egui::Align2::RIGHT_CENTER,
            label,
            egui::FontId::proportional(10.5),
            Color32::from_rgb(0x90, 0x90, 0xa0),
        );

        (response, action)
    }
}

fn fit_aspect(max_w: f32, max_h: f32, aspect: f32) -> (f32, f32) {
    if max_w <= 0.0 || max_h <= 0.0 {
        return (100.0, 100.0);
    }
    let h_from_w = max_w / aspect;
    if h_from_w <= max_h {
        (max_w, h_from_w)
    } else {
        (max_h * aspect, max_h)
    }
}

fn draw_corner_guides(painter: &egui::Painter, rect: Rect) {
    let guide_len = 10.0;
    let guide_stroke = Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 50));

    // Top-Left
    painter.line_segment([rect.left_top(), Pos2::new(rect.left() + guide_len, rect.top())], guide_stroke);
    painter.line_segment([rect.left_top(), Pos2::new(rect.left(), rect.top() + guide_len)], guide_stroke);

    // Top-Right
    painter.line_segment([rect.right_top(), Pos2::new(rect.right() - guide_len, rect.top())], guide_stroke);
    painter.line_segment([rect.right_top(), Pos2::new(rect.right(), rect.top() + guide_len)], guide_stroke);

    // Bottom-Left
    painter.line_segment([rect.left_bottom(), Pos2::new(rect.left() + guide_len, rect.bottom())], guide_stroke);
    painter.line_segment([rect.left_bottom(), Pos2::new(rect.left(), rect.bottom() - guide_len)], guide_stroke);

    // Bottom-Right
    painter.line_segment([rect.right_bottom(), Pos2::new(rect.right() - guide_len, rect.bottom())], guide_stroke);
    painter.line_segment([rect.right_bottom(), Pos2::new(rect.right(), rect.bottom() - guide_len)], guide_stroke);
}

fn draw_button(painter: &egui::Painter, rect: Rect, text: &str, is_primary: bool) {
    let bg = if is_primary {
        Color32::from_rgb(0xe0, 0x7a, 0x38)
    } else {
        Color32::from_rgb(0x24, 0x24, 0x2b)
    };
    let text_col = Color32::WHITE;

    painter.rect_filled(rect, 4.0, bg);
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0_f32, if is_primary { Color32::from_rgb(0xcb, 0x69, 0x29) } else { Color32::from_rgb(0x38, 0x38, 0x44) }),
    );

    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(12.0),
        text_col,
    );
}
