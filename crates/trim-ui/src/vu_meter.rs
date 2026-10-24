//! Dual-channel (L/R) vertical audio VU meter widget with smooth decay ballistics.

use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};

#[derive(Debug, Clone)]
pub struct VuMeterState {
    pub current_l: f32,
    pub current_r: f32,
    pub target_l: f32,
    pub target_r: f32,
    pub peak_l: f32,
    pub peak_r: f32,
    pub peak_hold_l: u32,
    pub peak_hold_r: u32,
}

impl Default for VuMeterState {
    fn default() -> Self {
        Self {
            current_l: -60.0,
            current_r: -60.0,
            target_l: -60.0,
            target_r: -60.0,
            peak_l: -60.0,
            peak_r: -60.0,
            peak_hold_l: 0,
            peak_hold_r: 0,
        }
    }
}

impl VuMeterState {
    pub fn set_levels(&mut self, left_db: f32, right_db: f32) {
        self.target_l = left_db.clamp(-60.0, 0.0);
        self.target_r = right_db.clamp(-60.0, 0.0);

        if self.target_l > self.peak_l {
            self.peak_l = self.target_l;
            self.peak_hold_l = 30;
        }
        if self.target_r > self.peak_r {
            self.peak_r = self.target_r;
            self.peak_hold_r = 30;
        }
    }

    pub fn tick_decay(&mut self) {
        let decay = 1.2; // dB per tick

        if self.target_l > self.current_l {
            self.current_l = self.target_l;
        } else {
            self.current_l = (self.current_l - decay).max(-60.0);
        }

        if self.target_r > self.current_r {
            self.current_r = self.target_r;
        } else {
            self.current_r = (self.current_r - decay).max(-60.0);
        }

        if self.peak_hold_l > 0 {
            self.peak_hold_l -= 1;
        } else {
            self.peak_l = (self.peak_l - decay * 0.5).max(-60.0);
        }

        if self.peak_hold_r > 0 {
            self.peak_hold_r -= 1;
        } else {
            self.peak_r = (self.peak_r - decay * 0.5).max(-60.0);
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

pub struct VuMeterWidget<'a> {
    state: &'a mut VuMeterState,
}

impl<'a> VuMeterWidget<'a> {
    pub fn new(state: &'a mut VuMeterState) -> Self {
        Self { state }
    }
}

impl<'a> Widget for VuMeterWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::new(52.0, ui.available_height().max(160.0));
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

        let painter = ui.painter_at(rect);

        // Background
        painter.rect_filled(rect, 4.0, Color32::from_rgb(0x18, 0x18, 0x1d));
        painter.rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0_f32, Color32::from_rgb(0x28, 0x28, 0x30)),
        );

        let bar_width = 8.0;
        let spacing = 4.0;
        let left_x = rect.left() + 7.0;
        let right_x = left_x + bar_width + spacing;
        let top_y = rect.top() + 8.0;
        let bottom_y = rect.bottom() - 8.0;
        let bar_height = (bottom_y - top_y).max(10.0);

        // Draw left and right channels
        draw_bar(
            &painter,
            left_x,
            top_y,
            bar_width,
            bar_height,
            self.state.current_l,
            self.state.peak_l,
        );
        draw_bar(
            &painter,
            right_x,
            top_y,
            bar_width,
            bar_height,
            self.state.current_r,
            self.state.peak_r,
        );

        // Draw dB scale text with comfortable left/right margin
        let labels = [
            ("0", 0.0),
            ("-6", -6.0),
            ("-12", -12.0),
            ("-24", -24.0),
            ("-40", -40.0),
        ];
        let text_x = right_x + bar_width + 6.0;
        for (txt, db) in labels {
            let frac = (db - (-60.0)) / 60.0;
            let y = bottom_y - bar_height * frac;
            painter.text(
                Pos2::new(text_x, y),
                egui::Align2::LEFT_CENTER,
                txt,
                egui::FontId::monospace(8.5),
                Color32::from_rgb(0x82, 0x86, 0x96),
            );
        }

        response
    }
}

fn draw_bar(
    painter: &egui::Painter,
    x: f32,
    top_y: f32,
    width: f32,
    height: f32,
    db: f32,
    peak_db: f32,
) {
    let bar_rect = Rect::from_min_size(Pos2::new(x, top_y), Vec2::new(width, height));
    painter.rect_filled(bar_rect, 1.0, Color32::from_rgb(0x10, 0x10, 0x14));

    let frac = ((db - (-60.0)) / 60.0).clamp(0.0, 1.0);
    let fill_h = height * frac;
    let fill_top = (top_y + height) - fill_h;

    // Steps rendering (segments)
    let step = 3.0;
    let mut cur_y = top_y + height - step;
    while cur_y >= fill_top {
        let seg_frac = ((top_y + height) - cur_y) / height;
        let seg_db = -60.0 + seg_frac * 60.0;
        let color = if seg_db > -3.0 {
            Color32::from_rgb(0xe0, 0x40, 0x40) // Red
        } else if seg_db > -12.0 {
            Color32::from_rgb(0xe0, 0xa0, 0x30) // Yellow
        } else {
            Color32::from_rgb(0x30, 0xb0, 0x60) // Green
        };
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(x, cur_y), Vec2::new(width, 2.0)),
            0.5,
            color,
        );
        cur_y -= step;
    }

    // Peak hold line
    let peak_frac = ((peak_db - (-60.0)) / 60.0).clamp(0.0, 1.0);
    let peak_y = (top_y + height) - height * peak_frac;
    painter.line_segment(
        [Pos2::new(x, peak_y), Pos2::new(x + width, peak_y)],
        Stroke::new(1.5_f32, Color32::from_rgb(0xff, 0xff, 0xff)),
    );
}
