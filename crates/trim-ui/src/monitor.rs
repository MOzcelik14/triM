//! Dual Monitor widget rendering Source Monitor (raw clips) and Program Monitor (timeline sequence)
//! with Premiere Pro style transport bars, In/Out range marking, and frame extraction.

use egui::{
    Button, Color32, Pos2, Rect, Response, RichText, Sense, Stroke, TextureHandle, Ui, Vec2,
};

#[path = "monitor/overlays.rs"]
mod overlays;
pub use overlays::draw_text_overlays_on_rect;
use overlays::{
    draw_button, draw_corner_guides, draw_safe_zones, fit_aspect, handle_mask_overlay_interaction,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorMode {
    Source,
    Program,
}

pub enum MonitorAction {
    NewProject,
    OpenProject,
    ImportMedia,
    TogglePlay,
    StepFrame(i32),
    MarkIn,
    MarkOut,
    GoToIn,
    GoToOut,
    InsertToTimeline,
    OverwriteToTimeline,
    ExportFrame,
    Seek(f64),
    ToggleSafeZones,
    SetAspectRatio(trim_core::AspectRatio),
    TextToolClick {
        pos_x: f32,
        pos_y: f32,
    },
    TextPositionDrag {
        pos_x: f32,
        pos_y: f32,
    },
    /// Selection tool picked the text overlay whose anchor is at (`pos_x`, `pos_y`).
    TextSelectAt {
        pos_x: f32,
        pos_y: f32,
    },
    TextContentEdited(String),
    TextDoubleClick {
        pos_x: f32,
        pos_y: f32,
    },
    TextPositionDragEnded,
    TextEditingCompleted,
    ToggleFullscreen,
    ToggleSplitWipe,
    SplitWipePosChanged(f32),
    ToggleMulticamView,
    MulticamAngleSelected(usize),
    HoverPixelSample {
        x_norm: f32,
        y_norm: f32,
    },
    PickWhiteBalancePixel {
        x_norm: f32,
        y_norm: f32,
    },
    CancelWbEyedropper,
    MasksEdited {
        clip_id: String,
        old_masks: Vec<trim_core::Mask>,
        new_masks: Vec<trim_core::Mask>,
        commit_undo: bool,
    },
}

/// Approximate on-screen bounding box of a text overlay (used for Selection tool hit-testing).
fn overlay_hit_rect(painter: &egui::Painter, frame_rect: Rect, ov: &ActiveTextOverlay) -> Rect {
    let scaled_font_size = (ov.font_size * (frame_rect.height() / 1080.0)).clamp(10.0, 160.0);
    let family = crate::theme::resolve_egui_font_family(&ov.font_family);
    let galley = painter.layout_no_wrap(
        ov.text.clone(),
        egui::FontId::new(scaled_font_size, family),
        Color32::WHITE,
    );
    let size = Vec2::new(galley.size().x.max(40.0), galley.size().y.max(24.0));
    let center = Pos2::new(
        frame_rect.left() + frame_rect.width() * ov.pos_x.clamp(0.0, 1.0),
        frame_rect.top() + frame_rect.height() * ov.pos_y.clamp(0.0, 1.0),
    );
    Rect::from_center_size(center, size).expand(8.0)
}

#[derive(Debug, Clone)]
pub struct ActiveTextOverlay {
    pub text: String,
    pub font_size: f32,
    pub text_color: Color32,
    pub bg_color: Option<Color32>,
    pub pos_x: f32,
    pub pos_y: f32,
    pub has_shadow: bool,
    pub shadow_distance: f32,
    pub stroke_width: f32,
    pub stroke_color: Color32,
    pub letter_spacing: f32,
    pub alpha: f32,
    pub caption_style: trim_core::clip::CaptionStyle,
    pub highlight_color: Color32,
    pub progress: f32,
    pub motion_style: trim_core::clip::MotionTitleStyle,
    pub subtitle_text: Option<String>,
    pub font_family: String,
    pub bold: bool,
    pub italic: bool,
}

impl Default for ActiveTextOverlay {
    fn default() -> Self {
        Self {
            text: String::new(),
            font_size: 42.0,
            text_color: Color32::WHITE,
            bg_color: None,
            pos_x: 0.5,
            pos_y: 0.85,
            has_shadow: true,
            shadow_distance: 2.0,
            stroke_width: 0.0,
            stroke_color: Color32::BLACK,
            letter_spacing: 0.0,
            alpha: 1.0,
            caption_style: trim_core::clip::CaptionStyle::Standard,
            highlight_color: Color32::from_rgb(255, 230, 0),
            progress: 0.0,
            motion_style: trim_core::clip::MotionTitleStyle::None,
            subtitle_text: None,
            font_family: "Sans-Serif".to_string(),
            bold: false,
            italic: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActiveMaskOverlay {
    pub clip_id: String,
    pub masks: Vec<trim_core::Mask>,
    pub clip_time: f64,
}

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

    pub fn show(self, ui: &mut Ui) -> (Response, Option<MonitorAction>) {
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

        // Viewport video frame calculated with respect to project aspect ratio
        let max_w = (viewport_rect.width() - 8.0).max(10.0);
        let max_h = (viewport_rect.height() - 8.0).max(10.0);
        let (target_w, target_h) = fit_aspect(max_w, max_h, self.aspect_ratio);
        let frame_rect =
            Rect::from_center_size(viewport_rect.center(), Vec2::new(target_w, target_h));

        // Deep black canvas background
        painter.rect_filled(frame_rect, 0.0, Color32::from_rgb(0x02, 0x02, 0x04));

        if !self.has_content {
            let center = frame_rect.center();
            painter.text(
                Pos2::new(center.x, center.y - 30.0),
                egui::Align2::CENTER_CENTER,
                if self.mode == MonitorMode::Source {
                    "Klipsiz Kaynak"
                } else {
                    "Sekans Boş"
                },
                egui::FontId::proportional(16.0),
                Color32::from_rgb(0x80, 0x80, 0x95),
            );

            let msg = if self.mode == MonitorMode::Source {
                "Medya havuzundan bir klibe çift tıklayın"
            } else {
                "Zaman çizgisine klip ekleyin"
            };
            painter.text(
                Pos2::new(center.x, center.y),
                egui::Align2::CENTER_CENTER,
                msg,
                egui::FontId::proportional(11.5),
                Color32::from_rgb(0x60, 0x60, 0x70),
            );

            let btn_y = center.y + 36.0;
            let btn_w = 110.0;
            let btn_h = 26.0;
            let r_import =
                Rect::from_center_size(Pos2::new(center.x, btn_y), Vec2::new(btn_w + 30.0, btn_h));

            if let Some(mouse_pos) = response.hover_pos() {
                if ui.input(|i| i.pointer.primary_clicked()) && r_import.contains(mouse_pos) {
                    action = Some(MonitorAction::ImportMedia);
                }
            }
            draw_button(&painter, r_import, "+ Medya İçe Aktar", true);
        } else if let Some(tex_id) = self
            .custom_texture_id
            .or_else(|| self.texture.map(|t| t.id()))
        {
            if self.mode == MonitorMode::Program && self.split_wipe_enabled {
                let cmp_mode_id = egui::Id::new("program_monitor_compare_submode");
                let mut cmp_mode: u8 = ui.data_mut(|d| d.get_temp::<u8>(cmp_mode_id)).unwrap_or(0);

                match cmp_mode {
                    1 => {
                        // Vertical Wipe (Top vs Bottom)
                        let split_y = frame_rect.top() + frame_rect.height() * self.split_wipe_pos;
                        let top_rect = Rect::from_min_max(
                            frame_rect.left_top(),
                            Pos2::new(frame_rect.right(), split_y),
                        );
                        let bot_rect = Rect::from_min_max(
                            Pos2::new(frame_rect.left(), split_y),
                            frame_rect.right_bottom(),
                        );
                        let top_uv =
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, self.split_wipe_pos));
                        let bot_uv = Rect::from_min_max(
                            Pos2::new(0.0, self.split_wipe_pos),
                            Pos2::new(1.0, 1.0),
                        );

                        if let Some(ref_tex) = self.reference_texture {
                            painter.image(ref_tex.id(), top_rect, top_uv, Color32::WHITE);
                        } else {
                            painter.image(
                                tex_id,
                                top_rect,
                                top_uv,
                                Color32::from_rgb(205, 205, 210),
                            );
                        }
                        painter.image(tex_id, bot_rect, bot_uv, Color32::WHITE);

                        painter.line_segment(
                            [
                                Pos2::new(frame_rect.left(), split_y),
                                Pos2::new(frame_rect.right(), split_y),
                            ],
                            Stroke::new(2.0_f32, Color32::from_rgb(0x58, 0xb4, 0xff)),
                        );
                        let handle_center = Pos2::new(frame_rect.center().x, split_y);
                        painter.circle_filled(
                            handle_center,
                            9.0,
                            Color32::from_rgb(0x18, 0x20, 0x30),
                        );
                        painter.circle_stroke(
                            handle_center,
                            9.0,
                            Stroke::new(1.5_f32, Color32::from_rgb(0x58, 0xb4, 0xff)),
                        );
                        painter.text(
                            Pos2::new(frame_rect.left() + 8.0, frame_rect.top() + 14.0),
                            egui::Align2::LEFT_CENTER,
                            if self.reference_texture.is_some() {
                                "REFERANS STILL (ÜST)"
                            } else {
                                "KARŞILAŞTIRMA (ÜST)"
                            },
                            egui::FontId::monospace(9.5),
                            Color32::from_rgb(220, 230, 245),
                        );
                        painter.text(
                            Pos2::new(frame_rect.right() - 8.0, frame_rect.bottom() - 14.0),
                            egui::Align2::RIGHT_CENTER,
                            "MEVCUT GRADE (ALT)",
                            egui::FontId::monospace(9.5),
                            Color32::from_rgb(88, 180, 255),
                        );

                        if let Some(ptr) = response.interact_pointer_pos() {
                            if response.dragged() && frame_rect.contains(ptr) {
                                let new_pos = ((ptr.y - frame_rect.top()) / frame_rect.height())
                                    .clamp(0.05, 0.95);
                                action = Some(MonitorAction::SplitWipePosChanged(new_pos));
                            }
                        }
                    }
                    2 => {
                        // Side-by-Side 2x Comparison (Full Left vs Full Right)
                        let mid_x = frame_rect.center().x;
                        let left_rect = Rect::from_min_max(
                            frame_rect.left_top(),
                            Pos2::new(mid_x - 1.0, frame_rect.bottom()),
                        );
                        let right_rect = Rect::from_min_max(
                            Pos2::new(mid_x + 1.0, frame_rect.top()),
                            frame_rect.right_bottom(),
                        );
                        let full_uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));

                        if let Some(ref_tex) = self.reference_texture {
                            painter.image(ref_tex.id(), left_rect, full_uv, Color32::WHITE);
                        } else {
                            painter.image(
                                tex_id,
                                left_rect,
                                full_uv,
                                Color32::from_rgb(205, 205, 210),
                            );
                        }
                        painter.image(tex_id, right_rect, full_uv, Color32::WHITE);

                        painter.line_segment(
                            [
                                Pos2::new(mid_x, frame_rect.top()),
                                Pos2::new(mid_x, frame_rect.bottom()),
                            ],
                            Stroke::new(2.0_f32, Color32::from_rgb(0x58, 0xb4, 0xff)),
                        );
                        painter.text(
                            Pos2::new(left_rect.left() + 8.0, frame_rect.top() + 14.0),
                            egui::Align2::LEFT_CENTER,
                            if self.reference_texture.is_some() {
                                "REFERANS (2x YAN YANA)"
                            } else {
                                "ÖNCESİ (2x YAN YANA)"
                            },
                            egui::FontId::monospace(9.5),
                            Color32::from_rgb(220, 230, 245),
                        );
                        painter.text(
                            Pos2::new(right_rect.right() - 8.0, frame_rect.top() + 14.0),
                            egui::Align2::RIGHT_CENTER,
                            "MEVCUT GRADE (2x)",
                            egui::FontId::monospace(9.5),
                            Color32::from_rgb(88, 180, 255),
                        );
                    }
                    _ => {
                        // Horizontal Wipe (Left vs Right)
                        let split_x = frame_rect.left() + frame_rect.width() * self.split_wipe_pos;
                        let left_rect = Rect::from_min_max(
                            frame_rect.left_top(),
                            Pos2::new(split_x, frame_rect.bottom()),
                        );
                        let right_rect = Rect::from_min_max(
                            Pos2::new(split_x, frame_rect.top()),
                            frame_rect.right_bottom(),
                        );
                        let left_uv =
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(self.split_wipe_pos, 1.0));
                        let right_uv = Rect::from_min_max(
                            Pos2::new(self.split_wipe_pos, 0.0),
                            Pos2::new(1.0, 1.0),
                        );

                        if let Some(ref_tex) = self.reference_texture {
                            painter.image(ref_tex.id(), left_rect, left_uv, Color32::WHITE);
                        } else {
                            painter.image(
                                tex_id,
                                left_rect,
                                left_uv,
                                Color32::from_rgb(205, 205, 210),
                            );
                        }
                        painter.image(tex_id, right_rect, right_uv, Color32::WHITE);

                        painter.line_segment(
                            [
                                Pos2::new(split_x, frame_rect.top()),
                                Pos2::new(split_x, frame_rect.bottom()),
                            ],
                            Stroke::new(2.0_f32, Color32::from_rgb(0x58, 0xb4, 0xff)),
                        );
                        let handle_center = Pos2::new(split_x, frame_rect.center().y);
                        painter.circle_filled(
                            handle_center,
                            9.0,
                            Color32::from_rgb(0x18, 0x20, 0x30),
                        );
                        painter.circle_stroke(
                            handle_center,
                            9.0,
                            Stroke::new(1.5_f32, Color32::from_rgb(0x58, 0xb4, 0xff)),
                        );
                        painter.text(
                            Pos2::new(left_rect.left() + 8.0, frame_rect.top() + 14.0),
                            egui::Align2::LEFT_CENTER,
                            if self.reference_texture.is_some() {
                                "REFERANS STILL"
                            } else {
                                "KARŞILAŞTIRMA (SOL)"
                            },
                            egui::FontId::monospace(9.5),
                            Color32::from_rgb(220, 230, 245),
                        );
                        painter.text(
                            Pos2::new(right_rect.right() - 8.0, frame_rect.top() + 14.0),
                            egui::Align2::RIGHT_CENTER,
                            "MEVCUT GRADE (SAĞ)",
                            egui::FontId::monospace(9.5),
                            Color32::from_rgb(88, 180, 255),
                        );

                        if let Some(ptr) = response.interact_pointer_pos() {
                            if response.dragged() && frame_rect.contains(ptr) {
                                let new_pos = ((ptr.x - frame_rect.left()) / frame_rect.width())
                                    .clamp(0.05, 0.95);
                                action = Some(MonitorAction::SplitWipePosChanged(new_pos));
                            }
                        }
                    }
                }

                // Right-click on monitor while split wipe is active cycles mode (Horizontal -> Vertical -> Side-by-Side)
                if response.secondary_clicked() && !self.wb_eyedropper_active {
                    cmp_mode = (cmp_mode + 1) % 3;
                    ui.data_mut(|d| d.insert_temp(cmp_mode_id, cmp_mode));
                }
            } else {
                painter.image(
                    tex_id,
                    frame_rect,
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }

            if self.mode == MonitorMode::Program && self.multicam_mode {
                let mid = frame_rect.center();
                let quads = [
                    Rect::from_min_max(frame_rect.left_top(), mid),
                    Rect::from_min_max(
                        Pos2::new(mid.x, frame_rect.top()),
                        Pos2::new(frame_rect.right(), mid.y),
                    ),
                    Rect::from_min_max(
                        Pos2::new(frame_rect.left(), mid.y),
                        Pos2::new(mid.x, frame_rect.bottom()),
                    ),
                    Rect::from_min_max(mid, frame_rect.right_bottom()),
                ];
                for (qi, qrect) in quads.iter().enumerate() {
                    let is_active = self.multicam_active_angle == qi;
                    painter.rect_stroke(
                        *qrect,
                        0.0,
                        Stroke::new(
                            if is_active { 2.5_f32 } else { 1.0_f32 },
                            if is_active {
                                Color32::from_rgb(235, 65, 65)
                            } else {
                                Color32::from_rgba_unmultiplied(180, 190, 210, 110)
                            },
                        ),
                    );
                    let default_lbl = format!("KAMERA {} (Tuş {})", qi + 1, qi + 1);
                    let lbl = self
                        .multicam_labels
                        .get(qi)
                        .map(|s| s.as_str())
                        .unwrap_or(&default_lbl);
                    let badge_rect = Rect::from_min_size(
                        Pos2::new(qrect.left() + 6.0, qrect.top() + 6.0),
                        Vec2::new(136.0, 20.0),
                    );
                    painter.rect_filled(
                        badge_rect,
                        3.0,
                        if is_active {
                            Color32::from_rgba_unmultiplied(200, 35, 35, 220)
                        } else {
                            Color32::from_rgba_unmultiplied(15, 18, 26, 200)
                        },
                    );
                    painter.text(
                        badge_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        lbl,
                        egui::FontId::monospace(10.0),
                        Color32::WHITE,
                    );
                    if let Some(click_pos) = response.interact_pointer_pos() {
                        if response.clicked() && qrect.contains(click_pos) {
                            action = Some(MonitorAction::MulticamAngleSelected(qi));
                        }
                    }
                }
            }

            if self.mode == MonitorMode::Program && self.wb_eyedropper_active {
                let banner_rect = Rect::from_center_size(
                    Pos2::new(frame_rect.center().x, frame_rect.top() + 18.0),
                    Vec2::new((frame_rect.width() - 16.0).clamp(160.0, 380.0), 22.0),
                );
                painter.rect_filled(
                    banner_rect,
                    4.0,
                    Color32::from_rgba_unmultiplied(15, 22, 36, 225),
                );
                painter.rect_stroke(
                    banner_rect,
                    4.0,
                    Stroke::new(1.2_f32, Color32::from_rgb(0x58, 0xb4, 0xff)),
                );
                painter.text(
                    banner_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "⌖ WB Damlalığı: Nötr beyaz/gri bir noktaya tıklayın (İptal: Sağ Tık)",
                    egui::FontId::proportional(10.5),
                    Color32::WHITE,
                );

                if let Some(hover_pos) = response.hover_pos() {
                    if frame_rect.contains(hover_pos) {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                        painter.circle_stroke(
                            hover_pos,
                            7.0,
                            Stroke::new(1.5_f32, Color32::from_rgb(0xf5, 0xb9, 0x71)),
                        );
                        painter.circle_stroke(hover_pos, 8.5, Stroke::new(1.0_f32, Color32::BLACK));
                    }
                }
                if response.secondary_clicked() {
                    action = Some(MonitorAction::CancelWbEyedropper);
                } else if response.clicked() {
                    if let Some(click_pos) = response.interact_pointer_pos() {
                        if frame_rect.contains(click_pos) {
                            let x_norm = ((click_pos.x - frame_rect.left()) / frame_rect.width())
                                .clamp(0.0, 1.0);
                            let y_norm = ((click_pos.y - frame_rect.top()) / frame_rect.height())
                                .clamp(0.0, 1.0);
                            action = Some(MonitorAction::PickWhiteBalancePixel { x_norm, y_norm });
                        }
                    }
                }
            }

            if self.mode == MonitorMode::Program && action.is_none() {
                if let Some(hover_pos) = response.hover_pos() {
                    if frame_rect.contains(hover_pos) {
                        let x_norm = ((hover_pos.x - frame_rect.left()) / frame_rect.width())
                            .clamp(0.0, 1.0);
                        let y_norm = ((hover_pos.y - frame_rect.top()) / frame_rect.height())
                            .clamp(0.0, 1.0);
                        action = Some(MonitorAction::HoverPixelSample { x_norm, y_norm });
                    }
                }
            }
        } else {
            painter.text(
                frame_rect.center(),
                egui::Align2::CENTER_CENTER,
                if self.is_decoding {
                    "Kare hazırlanıyor..."
                } else {
                    "Önizleme Yükleniyor..."
                },
                egui::FontId::proportional(12.0),
                Color32::from_rgb(0x80, 0x80, 0x90),
            );
        }

        // Real-time Titler and Text Overlay rendering
        let is_type_tool = self.active_tool == Some(crate::toolbar::EditTool::Type);

        if self.mode == MonitorMode::Program && is_type_tool {
            if let Some(hover_pos) = response.hover_pos() {
                if frame_rect.contains(hover_pos) {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                }
            }

            // Click or drag on monitor frame with Type Tool to place or reposition text
            if let Some(interact_pos) = response.interact_pointer_pos() {
                if frame_rect.contains(interact_pos) {
                    let (target_x, target_y) = if let Some(first_ov) = self.text_overlays.first() {
                        (
                            frame_rect.left() + frame_rect.width() * first_ov.pos_x.clamp(0.0, 1.0),
                            frame_rect.top() + frame_rect.height() * first_ov.pos_y.clamp(0.0, 1.0),
                        )
                    } else {
                        (frame_rect.center().x, frame_rect.center().y)
                    };
                    let input_y = if target_y - 36.0 > frame_rect.top() + 10.0 {
                        target_y - 36.0
                    } else {
                        target_y + 36.0
                    };
                    let input_rect = Rect::from_center_size(
                        Pos2::new(target_x, input_y),
                        Vec2::new(260.0, 28.0),
                    );
                    let is_inside_editor = self.inline_text_edit.is_some()
                        && input_rect.expand(6.0).contains(interact_pos);

                    if !is_inside_editor {
                        let norm_x = ((interact_pos.x - frame_rect.left()) / frame_rect.width())
                            .clamp(0.05, 0.95);
                        let norm_y = ((interact_pos.y - frame_rect.top()) / frame_rect.height())
                            .clamp(0.05, 0.95);

                        if response.clicked() {
                            action = Some(MonitorAction::TextToolClick {
                                pos_x: norm_x,
                                pos_y: norm_y,
                            });
                        } else if response.dragged() {
                            action = Some(MonitorAction::TextPositionDrag {
                                pos_x: norm_x,
                                pos_y: norm_y,
                            });
                        } else if response.drag_stopped() {
                            action = Some(MonitorAction::TextPositionDragEnded);
                        }
                    }
                }
            }
        }

        // Selection tool: pick an on-screen text overlay and drag it directly in the viewport
        let is_select_tool = self.active_tool == Some(crate::toolbar::EditTool::Select);
        if self.mode == MonitorMode::Program && is_select_tool && self.has_content {
            let drag_id = egui::Id::new("program_monitor_text_drag_offset");
            let norm_of = |p: Pos2| -> (f32, f32) {
                (
                    (p.x - frame_rect.left()) / frame_rect.width().max(1.0),
                    (p.y - frame_rect.top()) / frame_rect.height().max(1.0),
                )
            };
            let hit_overlay = |p: Pos2| -> Option<&ActiveTextOverlay> {
                if !frame_rect.contains(p) {
                    return None;
                }
                self.text_overlays.iter().find(|ov| {
                    ov.alpha > 0.001
                        && !ov.text.is_empty()
                        && overlay_hit_rect(&painter, frame_rect, ov).contains(p)
                })
            };

            if let Some(hover) = response.hover_pos() {
                if hit_overlay(hover).is_some() {
                    ui.ctx().set_cursor_icon(if response.dragged() {
                        egui::CursorIcon::Grabbing
                    } else {
                        egui::CursorIcon::Grab
                    });
                }
            }

            if response.drag_started() {
                let origin = ui.input(|i| i.pointer.press_origin());
                let grabbed = origin.and_then(|p| hit_overlay(p).map(|ov| (p, ov)));
                if let Some((p, ov)) = grabbed {
                    let (nx, ny) = norm_of(p);
                    let offset = Vec2::new(nx - ov.pos_x, ny - ov.pos_y);
                    ui.memory_mut(|m| m.data.insert_temp(drag_id, Some(offset)));
                    action = Some(MonitorAction::TextSelectAt {
                        pos_x: ov.pos_x,
                        pos_y: ov.pos_y,
                    });
                } else {
                    ui.memory_mut(|m| m.data.insert_temp(drag_id, None::<Vec2>));
                }
            } else if response.dragged() {
                let offset = ui
                    .memory(|m| m.data.get_temp::<Option<Vec2>>(drag_id))
                    .flatten();
                if let (Some(offset), Some(p)) = (offset, response.interact_pointer_pos()) {
                    let (nx, ny) = norm_of(p);
                    action = Some(MonitorAction::TextPositionDrag {
                        pos_x: (nx - offset.x).clamp(0.02, 0.98),
                        pos_y: (ny - offset.y).clamp(0.02, 0.98),
                    });
                }
            } else if response.drag_stopped() {
                action = Some(MonitorAction::TextPositionDragEnded);
            } else if response.double_clicked() {
                if let Some(p) = response.interact_pointer_pos() {
                    if let Some(ov) = hit_overlay(p) {
                        action = Some(MonitorAction::TextDoubleClick {
                            pos_x: ov.pos_x,
                            pos_y: ov.pos_y,
                        });
                    } else if frame_rect.contains(p) {
                        action = Some(MonitorAction::ToggleFullscreen);
                    }
                }
            } else if response.clicked() {
                if let Some(p) = response.interact_pointer_pos() {
                    if let Some(ov) = hit_overlay(p) {
                        action = Some(MonitorAction::TextSelectAt {
                            pos_x: ov.pos_x,
                            pos_y: ov.pos_y,
                        });
                    }
                }
            }

            if response.drag_stopped() {
                ui.memory_mut(|m| m.data.insert_temp(drag_id, None::<Vec2>));
            }
        }

        let rendered_overlays =
            draw_text_overlays_on_rect(&painter, frame_rect, &self.text_overlays);

        if is_type_tool && self.mode == MonitorMode::Program {
            for (_idx, text_rect) in &rendered_overlays {
                let pad = 6.0;
                let bbox = text_rect.expand(pad);
                painter.rect_stroke(
                    bbox,
                    2.0,
                    Stroke::new(1.5_f32, Color32::from_rgb(0x38, 0xa0, 0xf0)),
                );
                let dot_r = 3.5;
                let dot_col = Color32::from_rgb(0x81, 0x8c, 0xf8);
                painter.circle_filled(bbox.left_top(), dot_r, dot_col);
                painter.circle_filled(bbox.right_top(), dot_r, dot_col);
                painter.circle_filled(bbox.left_bottom(), dot_r, dot_col);
                painter.circle_filled(bbox.right_bottom(), dot_r, dot_col);
            }
        }

        // Floating inline on-screen text editor
        if is_type_tool && self.mode == MonitorMode::Program {
            if let Some(mut edit_txt) = self.inline_text_edit {
                let (target_x, target_y) = if let Some(first_ov) = self.text_overlays.first() {
                    (
                        frame_rect.left() + frame_rect.width() * first_ov.pos_x.clamp(0.0, 1.0),
                        frame_rect.top() + frame_rect.height() * first_ov.pos_y.clamp(0.0, 1.0),
                    )
                } else {
                    (frame_rect.center().x, frame_rect.center().y)
                };

                let input_w = 260.0;
                let input_h = 28.0;
                let input_y = if target_y - 36.0 > frame_rect.top() + 10.0 {
                    target_y - 36.0
                } else {
                    target_y + 36.0
                };
                let input_rect = Rect::from_center_size(
                    Pos2::new(target_x, input_y),
                    Vec2::new(input_w, input_h),
                );

                painter.rect_filled(input_rect, 4.0, Color32::from_black_alpha(220));
                painter.rect_stroke(
                    input_rect,
                    4.0,
                    Stroke::new(1.5_f32, Color32::from_rgb(0x38, 0xa0, 0xf0)),
                );

                let mut child_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(input_rect.shrink(2.0))
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                let resp = child_ui.add(
                    egui::TextEdit::singleline(&mut edit_txt)
                        .desired_width(input_w - 12.0)
                        .hint_text("Metin girin...")
                        .text_color(Color32::WHITE),
                );
                if resp.changed() {
                    action = Some(MonitorAction::TextContentEdited(edit_txt));
                } else if resp.lost_focus() {
                    action = Some(MonitorAction::TextEditingCompleted);
                }
            }
        }

        if self.mode == MonitorMode::Program {
            if let Some(mut mask_ov) = self.active_masks {
                handle_mask_overlay_interaction(
                    ui,
                    &painter,
                    frame_rect,
                    &response,
                    &mut mask_ov,
                    &mut action,
                );
            }
        }

        // High contrast frame border
        painter.rect_stroke(
            frame_rect,
            0.0,
            Stroke::new(1.0_f32, Color32::from_rgb(0x38, 0x38, 0x46)),
        );
        draw_corner_guides(&painter, frame_rect);
        if self.safe_zones {
            draw_safe_zones(&painter, frame_rect, self.aspect_ratio < 1.0);
        }

        // 3. Scrubber Bar
        let scrub_rect = Rect::from_min_max(
            Pos2::new(
                outer_rect.left() + 6.0,
                outer_rect.bottom() - bottom_total_h,
            ),
            Pos2::new(
                outer_rect.right() - 6.0,
                outer_rect.bottom() - bottom_total_h + scrubber_h,
            ),
        );
        painter.rect_filled(scrub_rect, 2.0, Color32::from_rgb(0x18, 0x18, 0x20));
        painter.rect_stroke(
            scrub_rect,
            2.0,
            Stroke::new(1.0_f32, Color32::from_rgb(0x28, 0x28, 0x32)),
        );

        if self.duration > 0.0 {
            let dur = self.duration;
            let track_w = scrub_rect.width() - 8.0;
            let x_start = scrub_rect.left() + 4.0;

            // Mark In / Mark Out shaded range for Source Monitor
            if let (Some(m_in), Some(m_out)) = (self.mark_in, self.mark_out) {
                let in_ratio = (m_in / dur).clamp(0.0, 1.0) as f32;
                let out_ratio = (m_out / dur).clamp(0.0, 1.0) as f32;
                let r_in_x = x_start + in_ratio * track_w;
                let r_out_x = x_start + out_ratio * track_w;
                let in_out_rect = Rect::from_min_max(
                    Pos2::new(r_in_x.min(r_out_x), scrub_rect.top() + 2.0),
                    Pos2::new(r_in_x.max(r_out_x), scrub_rect.bottom() - 2.0),
                );
                painter.rect_filled(
                    in_out_rect,
                    1.0,
                    Color32::from_rgba_premultiplied(56, 122, 224, 70),
                );

                // Bracket handles { }
                painter.line_segment(
                    [
                        Pos2::new(r_in_x, scrub_rect.top()),
                        Pos2::new(r_in_x, scrub_rect.bottom()),
                    ],
                    Stroke::new(2.0_f32, Color32::from_rgb(0x4a, 0x9a, 0xff)),
                );
                painter.line_segment(
                    [
                        Pos2::new(r_out_x, scrub_rect.top()),
                        Pos2::new(r_out_x, scrub_rect.bottom()),
                    ],
                    Stroke::new(2.0_f32, Color32::from_rgb(0x4a, 0x9a, 0xff)),
                );
            }

            // Current playhead position
            let curr_ratio = (self.current_time / dur).clamp(0.0, 1.0) as f32;
            let playhead_x = x_start + curr_ratio * track_w;
            painter.line_segment(
                [
                    Pos2::new(playhead_x, scrub_rect.top()),
                    Pos2::new(playhead_x, scrub_rect.bottom()),
                ],
                Stroke::new(2.0_f32, Color32::from_rgb(0xe0, 0x7a, 0x38)),
            );

            // Scrubber interaction
            if let Some(mouse_pos) = response.hover_pos() {
                if scrub_rect.contains(mouse_pos) && ui.input(|i| i.pointer.primary_down()) {
                    let click_ratio = ((mouse_pos.x - x_start) / track_w).clamp(0.0, 1.0);
                    let target_t = (click_ratio as f64) * dur;
                    action = Some(MonitorAction::Seek(target_t));
                }
            }
        }

        // 4. Bottom Transport Toolbar (Premiere Pro buttons row)
        let tool_rect = Rect::from_min_max(
            Pos2::new(
                outer_rect.left(),
                outer_rect.bottom() - (bottom_total_h - scrubber_h),
            ),
            Pos2::new(outer_rect.right(), outer_rect.bottom()),
        );
        painter.rect_filled(tool_rect, 3.0, Color32::from_rgb(0x16, 0x16, 0x1b));
        painter.line_segment(
            [tool_rect.left_top(), tool_rect.right_top()],
            Stroke::new(1.0_f32, Color32::from_rgb(0x26, 0x26, 0x30)),
        );

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(tool_rect), |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add_space(8.0);

                // Timecode
                ui.label(
                    RichText::new(self.timecode)
                        .monospace()
                        .size(11.5)
                        .color(Color32::from_rgb(0x38, 0xa0, 0xe8))
                        .strong(),
                );
                ui.label(
                    RichText::new("/")
                        .size(10.0)
                        .color(Color32::from_rgb(0x60, 0x60, 0x70)),
                );
                ui.label(
                    RichText::new(self.duration_timecode)
                        .monospace()
                        .size(11.0)
                        .color(Color32::from_rgb(0x90, 0x90, 0xa0)),
                );

                if self.is_playing && (self.playback_speed - 1.0).abs() > 0.01 {
                    let speed_label = if self.playback_speed > 0.0 {
                        format!(">> {:.0}x", self.playback_speed)
                    } else {
                        format!("<< {:.0}x", self.playback_speed.abs())
                    };
                    let speed_color = if self.playback_speed > 0.0 {
                        Color32::from_rgb(0x50, 0xd0, 0x70)
                    } else {
                        Color32::from_rgb(0xe0, 0x9a, 0x38)
                    };
                    ui.label(
                        RichText::new(speed_label)
                            .monospace()
                            .size(11.0)
                            .strong()
                            .color(speed_color),
                    );
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                // Mark In button {
                let btn_in = Button::new(RichText::new("{").strong().size(13.0))
                    .min_size(Vec2::new(24.0, 24.0))
                    .fill(Color32::from_rgb(0x22, 0x22, 0x2a));
                if ui
                    .add(btn_in)
                    .on_hover_text("Giriş Noktası Koy (Mark In - I veya {)")
                    .clicked()
                {
                    action = Some(MonitorAction::MarkIn);
                }

                // Mark Out button }
                let btn_out = Button::new(RichText::new("}").strong().size(13.0))
                    .min_size(Vec2::new(24.0, 24.0))
                    .fill(Color32::from_rgb(0x22, 0x22, 0x2a));
                if ui
                    .add(btn_out)
                    .on_hover_text("Çıkış Noktası Koy (Mark Out - O veya })")
                    .clicked()
                {
                    action = Some(MonitorAction::MarkOut);
                }

                // Go to In
                let btn_go_in = Button::new(RichText::new("|<").size(11.0).strong())
                    .min_size(Vec2::new(24.0, 24.0))
                    .fill(Color32::from_rgb(0x22, 0x22, 0x2a));
                if ui
                    .add(btn_go_in)
                    .on_hover_text("Giriş Noktasına Git (Go to In)")
                    .clicked()
                {
                    action = Some(MonitorAction::GoToIn);
                }

                // Step -1 frame
                let btn_prev = Button::new(RichText::new("<").size(12.0).strong())
                    .min_size(Vec2::new(24.0, 24.0))
                    .fill(Color32::from_rgb(0x22, 0x22, 0x2a));
                if ui
                    .add(btn_prev)
                    .on_hover_text("Önceki Kare (Sol Ok)")
                    .clicked()
                {
                    action = Some(MonitorAction::StepFrame(-1));
                }

                // Play / Pause button (vector painted triangle / pause bars)
                let (play_rect, play_resp) =
                    ui.allocate_exact_size(Vec2::new(30.0, 24.0), Sense::click());
                let pp = ui.painter_at(play_rect);
                let bg_col = if play_resp.hovered() {
                    Color32::from_rgb(0x38, 0x38, 0x46)
                } else {
                    Color32::from_rgb(0x28, 0x28, 0x34)
                };
                pp.rect_filled(play_rect, 3.0, bg_col);
                pp.rect_stroke(
                    play_rect,
                    3.0,
                    Stroke::new(1.0_f32, Color32::from_rgb(0x3c, 0x3c, 0x4c)),
                );

                let p_center = play_rect.center();
                if self.is_playing {
                    let bar_w = 3.0;
                    let bar_h = 10.0;
                    let bar1 = Rect::from_center_size(
                        Pos2::new(p_center.x - 3.5, p_center.y),
                        Vec2::new(bar_w, bar_h),
                    );
                    let bar2 = Rect::from_center_size(
                        Pos2::new(p_center.x + 3.5, p_center.y),
                        Vec2::new(bar_w, bar_h),
                    );
                    pp.rect_filled(bar1, 1.0, Color32::from_rgb(0x50, 0xd0, 0x70));
                    pp.rect_filled(bar2, 1.0, Color32::from_rgb(0x50, 0xd0, 0x70));
                } else {
                    let tri = vec![
                        Pos2::new(p_center.x - 4.0, p_center.y - 6.0),
                        Pos2::new(p_center.x + 6.0, p_center.y),
                        Pos2::new(p_center.x - 4.0, p_center.y + 6.0),
                    ];
                    pp.add(egui::Shape::convex_polygon(
                        tri,
                        Color32::WHITE,
                        Stroke::NONE,
                    ));
                }
                if play_resp
                    .on_hover_text("Oynat / Duraklat (Boşluk)")
                    .clicked()
                {
                    action = Some(MonitorAction::TogglePlay);
                }

                // Step +1 frame
                let btn_next = Button::new(RichText::new(">").size(12.0).strong())
                    .min_size(Vec2::new(24.0, 24.0))
                    .fill(Color32::from_rgb(0x22, 0x22, 0x2a));
                if ui
                    .add(btn_next)
                    .on_hover_text("Sonraki Kare (Sağ Ok)")
                    .clicked()
                {
                    action = Some(MonitorAction::StepFrame(1));
                }

                // Go to Out
                let btn_go_out = Button::new(RichText::new(">|").size(11.0).strong())
                    .min_size(Vec2::new(24.0, 24.0))
                    .fill(Color32::from_rgb(0x22, 0x22, 0x2a));
                if ui
                    .add(btn_go_out)
                    .on_hover_text("Çıkış Noktasına Git (Go to Out)")
                    .clicked()
                {
                    action = Some(MonitorAction::GoToOut);
                }

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                match self.mode {
                    MonitorMode::Source => {
                        // Insert button (shortcut ,)
                        let btn_insert = Button::new(
                            RichText::new("+ Ekle")
                                .size(11.0)
                                .color(Color32::from_rgb(0x80, 0xc0, 0xff))
                                .strong(),
                        )
                        .min_size(Vec2::new(26.0, 24.0))
                        .fill(Color32::from_rgb(0x20, 0x2a, 0x38));
                        if ui
                            .add(btn_insert)
                            .on_hover_text("Zaman Çizgisine Ekle (Insert - Virgül ',')")
                            .clicked()
                        {
                            action = Some(MonitorAction::InsertToTimeline);
                        }

                        // Overwrite button (shortcut .)
                        let btn_overwrite = Button::new(
                            RichText::new("= Yaz")
                                .size(11.0)
                                .color(Color32::from_rgb(0x60, 0xd8, 0xa0))
                                .strong(),
                        )
                        .min_size(Vec2::new(26.0, 24.0))
                        .fill(Color32::from_rgb(0x1c, 0x30, 0x28));
                        if ui
                            .add(btn_overwrite)
                            .on_hover_text("Üzerine Yaz (Overwrite - Nokta '.')")
                            .clicked()
                        {
                            action = Some(MonitorAction::OverwriteToTimeline);
                        }
                    }
                    MonitorMode::Program => {
                        // Export Frame camera icon
                        let btn_frame = Button::new(
                            RichText::new("Kare Al")
                                .size(11.0)
                                .color(Color32::from_rgb(0xe0, 0xaa, 0x50))
                                .strong(),
                        )
                        .min_size(Vec2::new(26.0, 24.0))
                        .fill(Color32::from_rgb(0x2d, 0x28, 0x20));
                        if ui
                            .add(btn_frame)
                            .on_hover_text("Mevcut Kareyi Dışa Aktar (Export Frame)")
                            .clicked()
                        {
                            action = Some(MonitorAction::ExportFrame);
                        }

                        let safe_color = if self.safe_zones {
                            Color32::from_rgb(0x38, 0xa0, 0xe8)
                        } else {
                            Color32::from_rgb(0xa0, 0xa0, 0xb0)
                        };
                        let btn_safe = Button::new(
                            RichText::new("Rehber")
                                .size(11.0)
                                .color(safe_color)
                                .strong(),
                        )
                        .min_size(Vec2::new(26.0, 24.0))
                        .fill(if self.safe_zones {
                            Color32::from_rgb(0x18, 0x30, 0x48)
                        } else {
                            Color32::from_rgb(0x22, 0x22, 0x2a)
                        });
                        if ui
                            .add(btn_safe)
                            .on_hover_text("Güvenli Alan & Sosyal Medya Rehber Çizgileri")
                            .clicked()
                        {
                            action = Some(MonitorAction::ToggleSafeZones);
                        }

                        let cmp_mode_id = egui::Id::new("program_monitor_compare_submode");
                        let mut cmp_mode: u8 = ui
                            .data_mut(|d| d.get_temp::<u8>(cmp_mode_id))
                            .unwrap_or(0);
                        let wipe_lbl = match cmp_mode {
                            1 => "⬒ Wipe (Dikey)",
                            2 => "▥ 2x Yan Yana",
                            _ => "◧ Wipe",
                        };
                        let wipe_col = if self.split_wipe_enabled {
                            Color32::from_rgb(0x58, 0xb4, 0xff)
                        } else {
                            Color32::from_rgb(0xa0, 0xa0, 0xb0)
                        };
                        let btn_wipe = Button::new(
                            RichText::new(wipe_lbl)
                                .size(11.0)
                                .color(wipe_col)
                                .strong(),
                        )
                        .min_size(Vec2::new(26.0, 24.0))
                        .fill(if self.split_wipe_enabled {
                            Color32::from_rgb(0x18, 0x30, 0x48)
                        } else {
                            Color32::from_rgb(0x22, 0x22, 0x2a)
                        });
                        let wipe_resp = ui.add(btn_wipe).on_hover_text(
                            "Sol Tık: Bölünmüş Ekran Aç/Kapat • Sağ Tık: Mod Değiştir (Yatay Wipe / Dikey Wipe / 2x Yan Yana)",
                        );
                        if wipe_resp.clicked() {
                            action = Some(MonitorAction::ToggleSplitWipe);
                        } else if wipe_resp.secondary_clicked() {
                            cmp_mode = (cmp_mode + 1) % 3;
                            ui.data_mut(|d| d.insert_temp(cmp_mode_id, cmp_mode));
                            if !self.split_wipe_enabled {
                                action = Some(MonitorAction::ToggleSplitWipe);
                            }
                        }

                        let mc_col = if self.multicam_mode {
                            Color32::from_rgb(0xf8, 0x71, 0x71)
                        } else {
                            Color32::from_rgb(0xa0, 0xa0, 0xb0)
                        };
                        let btn_mc = Button::new(
                            RichText::new("⊞ Multicam")
                                .size(11.0)
                                .color(mc_col)
                                .strong(),
                        )
                        .min_size(Vec2::new(26.0, 24.0))
                        .fill(if self.multicam_mode {
                            Color32::from_rgb(0x3a, 0x1e, 0x24)
                        } else {
                            Color32::from_rgb(0x22, 0x22, 0x2a)
                        });
                        if ui
                            .add(btn_mc)
                            .on_hover_text("2x2 Çoklu Kamera (Multicam) İzleyici ve Canlı Açı Kesimi (1-4 Tuşları)")
                            .clicked()
                        {
                            action = Some(MonitorAction::ToggleMulticamView);
                        }

                        if let Some(curr_ar) = self.current_aspect_ratio {
                            ui.add_space(2.0);
                            egui::ComboBox::from_id_salt("monitor_aspect_combo")
                                .selected_text(curr_ar.short_label())
                                .show_ui(ui, |ui| {
                                    for r in [
                                        trim_core::AspectRatio::Wide16x9,
                                        trim_core::AspectRatio::Vertical9x16,
                                        trim_core::AspectRatio::Square1x1,
                                        trim_core::AspectRatio::Portrait4x5,
                                        trim_core::AspectRatio::Classic4x3,
                                        trim_core::AspectRatio::Cinema21x9,
                                    ] {
                                        if ui.selectable_label(curr_ar == r, r.label()).clicked() {
                                            action = Some(MonitorAction::SetAspectRatio(r));
                                        }
                                    }
                                });
                        }

                        ui.add_space(2.0);
                        let btn_fs = egui::Button::new(
                            RichText::new("⛶")
                                .size(13.0)
                                .color(Color32::from_rgb(0xc0, 0xc5, 0xd0)),
                        )
                        .min_size(Vec2::new(26.0, 24.0))
                        .fill(Color32::from_rgb(0x22, 0x22, 0x2a));
                        if ui
                            .add(btn_fs)
                            .on_hover_text("Tam Ekran Önizleme (F11)")
                            .clicked()
                        {
                            action = Some(MonitorAction::ToggleFullscreen);
                        }
                    }
                }
            });
        });

        (response, action)
    }
}
