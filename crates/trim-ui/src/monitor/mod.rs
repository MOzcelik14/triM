//! Dual Monitor widget rendering Source Monitor (raw clips) and Program Monitor (timeline sequence)
//! with Premiere Pro style transport bars, In/Out range marking, and frame extraction.

use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, TextureHandle, Ui, Vec2};

mod overlays;
mod transport;
mod types;
mod viewport;

pub use overlays::draw_text_overlays_on_rect;
pub use types::*;

pub struct MonitorWidget<'a> {
    pub mode: MonitorMode,
    pub title: &'a str,
    pub texture: Option<&'a TextureHandle>,
    pub custom_texture_id: Option<egui::TextureId>,
    pub reference_texture: Option<&'a TextureHandle>,
    pub has_content: bool,
    pub aspect_ratio: f32,
    pub current_time: f64,
    pub duration: f64,
    pub mark_in: Option<f64>,
    pub mark_out: Option<f64>,
    pub timecode: &'a str,
    pub duration_timecode: &'a str,
    pub is_playing: bool,
    pub playback_speed: f64,
    pub media_label: Option<&'a str>,
    pub is_decoding: bool,
    pub safe_zones: bool,
    pub split_wipe_enabled: bool,
    pub split_wipe_pos: f32,
    pub multicam_mode: bool,
    pub multicam_active_angle: usize,
    pub multicam_labels: Vec<String>,
    pub current_aspect_ratio: Option<trim_core::AspectRatio>,
    pub text_overlays: Vec<ActiveTextOverlay>,
    pub active_tool: Option<crate::toolbar::EditTool>,
    pub inline_text_edit: Option<String>,
    pub active_masks: Option<ActiveMaskOverlay>,
    pub wb_eyedropper_active: bool,
}

impl<'a> MonitorWidget<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mode: MonitorMode,
        title: &'a str,
        texture: Option<&'a TextureHandle>,
        has_content: bool,
        aspect_ratio: f32,
        current_time: f64,
        duration: f64,
        mark_in: Option<f64>,
        mark_out: Option<f64>,
        timecode: &'a str,
        duration_timecode: &'a str,
        is_playing: bool,
        playback_speed: f64,
        media_label: Option<&'a str>,
        is_decoding: bool,
    ) -> Self {
        Self {
            mode,
            title,
            texture,
            custom_texture_id: None,
            reference_texture: None,
            has_content,
            aspect_ratio: if aspect_ratio > 0.1 {
                aspect_ratio
            } else {
                16.0 / 9.0
            },
            current_time,
            duration: duration.max(0.0),
            mark_in,
            mark_out,
            timecode,
            duration_timecode,
            is_playing,
            playback_speed,
            media_label,
            is_decoding,
            safe_zones: false,
            split_wipe_enabled: false,
            split_wipe_pos: 0.5,
            multicam_mode: false,
            multicam_active_angle: 0,
            multicam_labels: Vec::new(),
            current_aspect_ratio: None,
            text_overlays: Vec::new(),
            active_tool: None,
            inline_text_edit: None,
            active_masks: None,
            wb_eyedropper_active: false,
        }
    }

    pub fn wb_eyedropper(mut self, active: bool) -> Self {
        self.wb_eyedropper_active = active;
        self
    }

    pub fn custom_texture_id(mut self, id: Option<egui::TextureId>) -> Self {
        self.custom_texture_id = id;
        self
    }

    pub fn split_wipe(
        mut self,
        enabled: bool,
        pos: f32,
        ref_tex: Option<&'a TextureHandle>,
    ) -> Self {
        self.split_wipe_enabled = enabled;
        self.split_wipe_pos = pos.clamp(0.05, 0.95);
        self.reference_texture = ref_tex;
        self
    }

    pub fn multicam(mut self, enabled: bool, active_angle: usize, labels: Vec<String>) -> Self {
        self.multicam_mode = enabled;
        self.multicam_active_angle = active_angle;
        self.multicam_labels = labels;
        self
    }

    pub fn safe_zones(mut self, enabled: bool) -> Self {
        self.safe_zones = enabled;
        self
    }

    pub fn aspect_ratio_preset(mut self, ratio: Option<trim_core::AspectRatio>) -> Self {
        self.current_aspect_ratio = ratio;
        self
    }

    pub fn text_overlays(mut self, overlays: Vec<ActiveTextOverlay>) -> Self {
        self.text_overlays = overlays;
        self
    }

    pub fn active_tool(mut self, tool: crate::toolbar::EditTool) -> Self {
        self.active_tool = Some(tool);
        self
    }

    pub fn inline_text_edit(mut self, text: Option<String>) -> Self {
        self.inline_text_edit = text;
        self
    }

    pub fn mask_overlay(mut self, overlay: Option<ActiveMaskOverlay>) -> Self {
        self.active_masks = overlay;
        self
    }

    pub fn show(mut self, ui: &mut Ui) -> (Response, Option<MonitorAction>) {
        let avail_w = ui.available_width().max(180.0);
        let avail_h = ui.available_height().max(120.0);
        let (outer_rect, response) =
            ui.allocate_exact_size(Vec2::new(avail_w, avail_h), Sense::click_and_drag());
        let painter = ui.painter().clone();

        // Dark container background
        painter.rect_filled(outer_rect, 3.0, Color32::from_rgb(0x12, 0x12, 0x16));
        painter.rect_stroke(
            outer_rect,
            3.0,
            Stroke::new(1.0_f32, Color32::from_rgb(0x24, 0x24, 0x2c)),
        );

        let mut action = None;

        // 1. Header Tab Bar (Height: 28px)
        let tab_h = 28.0;
        let tab_rect = Rect::from_min_max(
            outer_rect.min,
            Pos2::new(outer_rect.max.x, outer_rect.min.y + tab_h),
        );
        painter.rect_filled(tab_rect, 3.0, Color32::from_rgb(0x18, 0x18, 0x1e));
        painter.line_segment(
            [tab_rect.left_bottom(), tab_rect.right_bottom()],
            Stroke::new(1.0_f32, Color32::from_rgb(0x28, 0x28, 0x32)),
        );

        // Tab badge on left
        let prefix = match self.mode {
            MonitorMode::Source => "Kaynak: ",
            MonitorMode::Program => "Program: ",
        };
        let tab_title = format!("{}{}", prefix, self.title);
        let tab_w = (tab_title.len() as f32 * 7.5 + 24.0).clamp(100.0, 260.0);
        let active_tab_rect = Rect::from_min_max(
            tab_rect.min,
            Pos2::new(tab_rect.min.x + tab_w, tab_rect.max.y),
        );
        painter.rect_filled(active_tab_rect, 2.0, Color32::from_rgb(0x22, 0x22, 0x2a));
        // Blue active top/bottom border
        painter.line_segment(
            [
                active_tab_rect.left_bottom(),
                active_tab_rect.right_bottom(),
            ],
            Stroke::new(2.0_f32, Color32::from_rgb(0x38, 0x7a, 0xe0)),
        );
        painter.text(
            Pos2::new(active_tab_rect.left() + 10.0, active_tab_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &tab_title,
            egui::FontId::proportional(11.5),
            Color32::WHITE,
        );

        // Right side info / zoom badge
        let label = self.media_label.unwrap_or(match self.mode {
            MonitorMode::Source => "Sığdır (Fit)",
            MonitorMode::Program => "Sığdır • 1/2",
        });
        painter.text(
            Pos2::new(tab_rect.right() - 10.0, tab_rect.center().y),
            egui::Align2::RIGHT_CENTER,
            label,
            egui::FontId::proportional(11.0),
            Color32::from_rgb(0x85, 0x85, 0x95),
        );

        // 2. Transport & Scrubber Bottom Area (Height: 52px total: 14px scrubber + 38px toolbar)
        let bottom_total_h = 56.0;
        let scrubber_h = 16.0;
        let viewport_rect = Rect::from_min_max(
            Pos2::new(outer_rect.left(), outer_rect.top() + tab_h),
            Pos2::new(outer_rect.right(), outer_rect.bottom() - bottom_total_h),
        );

        self.render_viewport(ui, &painter, &response, viewport_rect, &mut action);
        self.render_transport(
            ui,
            &painter,
            &response,
            outer_rect,
            bottom_total_h,
            scrubber_h,
            &mut action,
        );

        (response, action)
    }
}
