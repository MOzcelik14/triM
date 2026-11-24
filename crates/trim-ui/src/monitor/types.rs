use egui::{Color32, Pos2, Rect, Vec2};

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
    CreateSubclip,
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
pub(super) fn overlay_hit_rect(
    painter: &egui::Painter,
    frame_rect: Rect,
    ov: &ActiveTextOverlay,
) -> Rect {
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
