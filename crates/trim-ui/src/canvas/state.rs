use crate::toolbar::EditTool;
use egui::{Pos2, Rect, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragMode {
    None,
    Move,
    TrimIn,
    TrimOut,
    FadeIn,
    FadeOut,
    VolumeRubberband,
    Scrub,
    Rubberband,
    HandPan,
    Slip,
    Slide,
    Roll,
    RippleIn,
    RippleOut,
}

pub const TRACK_TOP_PADDING: f32 = 0.0;
pub const SNAP_THRESHOLD_PX: f32 = 10.0;

pub type CompanionDragOrigin = (String, String, f64, f64, f64, f64);
pub type CompanionTrimBounds = (String, String, (f64, f64, f64, f64), (f64, f64, f64, f64));

#[derive(Debug, Clone)]
pub struct TimelineCanvasState {
    pub pixels_per_second: f32,
    pub track_height: f32,
    pub track_spacing: f32,
    pub trim_handle_width: f32,

    pub selected_clip_pairs: Vec<(String, String)>, // (track_id, clip_id)

    pub active_tool: EditTool,
    pub drag_mode: DragMode,
    pub drag_clip_id: Option<String>,
    pub drag_track_id: Option<String>,
    pub drag_start_x: f32,
    pub drag_orig_in: f64,
    pub drag_orig_out: f64,
    pub drag_orig_sin: f64,
    pub drag_orig_sout: f64,
    pub drag_orig_vol: f64,
    pub drag_orig_fade: f64,
    pub batch_orig_ins: Vec<(String, String, f64)>, // (track_id, clip_id, orig_in)
    pub drag_companions: Vec<CompanionDragOrigin>, // (track_id, clip_id, orig_in, orig_out, orig_sin, orig_sout)

    pub rubberband_start: Option<Pos2>,
    pub rubberband_rect: Option<Rect>,
    pub snap_guide_x: Option<f32>,
    pub drag_target_track_id: Option<String>,
    pub track_heights: std::collections::HashMap<String, f32>,
    pub selected_gap: Option<(String, f64, f64)>, // (track_id, gap_start, gap_end)
    pub scroll_offset: Vec2,
    pub pending_scroll_x: Option<f32>,
    pub pending_scroll_y: Option<f32>,
    pub drag_kf_index: Option<usize>,
    pub drag_orig_keyframes: Vec<trim_core::keyframe::Keyframe>,
    /// Cached `(file_path, exists, checked_at)` per `media_id` to avoid per-frame `stat()` syscalls.
    pub media_exists_cache: std::collections::HashMap<String, (String, bool, std::time::Instant)>,
    pub lang: crate::i18n::Language,
}

impl Default for TimelineCanvasState {
    fn default() -> Self {
        Self {
            pixels_per_second: 60.0,
            track_height: 56.0,
            track_spacing: 6.0,
            trim_handle_width: 8.0,
            selected_clip_pairs: Vec::new(),
            active_tool: EditTool::Select,
            drag_mode: DragMode::None,
            drag_clip_id: None,
            drag_track_id: None,
            drag_start_x: 0.0,
            drag_orig_in: 0.0,
            drag_orig_out: 0.0,
            drag_orig_sin: 0.0,
            drag_orig_sout: 0.0,
            drag_orig_vol: 1.0,
            drag_orig_fade: 0.0,
            batch_orig_ins: Vec::new(),
            drag_companions: Vec::new(),
            rubberband_start: None,
            rubberband_rect: None,
            snap_guide_x: None,
            drag_target_track_id: None,
            track_heights: std::collections::HashMap::new(),
            selected_gap: None,
            scroll_offset: Vec2::ZERO,
            pending_scroll_x: None,
            pending_scroll_y: None,
            drag_kf_index: None,
            drag_orig_keyframes: Vec::new(),
            media_exists_cache: std::collections::HashMap::new(),
            lang: crate::i18n::Language::default(),
        }
    }
}

impl TimelineCanvasState {
    pub fn is_media_online(&mut self, media_id: &str, file_path: &str) -> bool {
        let now = std::time::Instant::now();
        if let Some((cached_path, exists, checked_at)) = self.media_exists_cache.get(media_id) {
            if cached_path == file_path && now.duration_since(*checked_at).as_secs() < 3 {
                return *exists;
            }
        }
        let exists = std::path::Path::new(file_path).exists();
        self.media_exists_cache
            .insert(media_id.to_string(), (file_path.to_string(), exists, now));
        exists
    }

    pub fn get_track_height(&self, track_id: &str) -> f32 {
        self.track_heights
            .get(track_id)
            .copied()
            .unwrap_or(self.track_height)
    }

    pub fn set_track_height(&mut self, track_id: &str, height: f32) {
        self.track_heights
            .insert(track_id.to_string(), height.clamp(32.0, 160.0));
    }

    pub fn calculate_track_offsets(
        &self,
        tracks: &[trim_core::track::Track],
        top: f32,
    ) -> Vec<(f32, f32)> {
        let mut offsets = Vec::with_capacity(tracks.len());
        let mut current_y = top + TRACK_TOP_PADDING;
        for track in tracks {
            let h = self.get_track_height(&track.id);
            offsets.push((current_y, h));
            current_y += h + self.track_spacing;
        }
        offsets
    }
}

pub enum CanvasAction {
    Seek(f64),
    SelectClip(String, String),
    SelectClips(Vec<(String, String)>),
    ClearSelection,
    ClipMoved {
        old_track_id: String,
        new_track_id: String,
        clip_id: String,
        old_in: f64,
        new_in: f64,
    },
    BatchClipsMoved(Vec<(String, String, f64, f64)>),
    ClipTrimmed {
        track_id: String,
        clip_id: String,
        old_bounds: (f64, f64, f64, f64),
        new_bounds: (f64, f64, f64, f64),
        companions: Vec<CompanionTrimBounds>,
    },
    ClipRippleTrimmed {
        track_id: String,
        clip_id: String,
        old_bounds: (f64, f64, f64, f64),
        new_bounds: (f64, f64, f64, f64),
        shifted_clips: Vec<(String, String, f64, f64)>,
        companions: Vec<CompanionTrimBounds>,
    },
    ClipModified,
    SetClipFade {
        track_id: String,
        clip_id: String,
        is_fade_in: bool,
        old_fade: f64,
        new_fade: f64,
    },
    SetClipKeyframes {
        track_id: String,
        clip_id: String,
        property: String,
        old_keyframes: Vec<trim_core::keyframe::Keyframe>,
        new_keyframes: Vec<trim_core::keyframe::Keyframe>,
    },
    PromptSpeed(String, String),
    DetachAudio(String, String),
    SplitAtPlayhead,
    RazorCut {
        track_id: String,
        clip_id: String,
        cut_time: f64,
        split_linked: bool,
    },
    CreateTitleAtTime(f64),
    Pan(f32),
    CopySelected,
    PasteClipboard,
    DeleteSelected,
    RippleDeleteSelected,
    MediaDropped {
        media_id: String,
        track_id: String,
        time: f64,
    },
    CloseGap {
        track_id: Option<String>,
        gap_start: f64,
        gap_end: f64,
    },
    SetClipColor {
        track_id: String,
        clip_id: String,
        color: trim_core::clip::ColorLabel,
    },
    SetClipVolume {
        track_id: String,
        clip_id: String,
        old_volume: f64,
        new_volume: f64,
    },
    ToggleLinkSelected,
    AlignSelectedByAudio,
    AlignSelectedByTimecode,
    OpenSequence(String),
    CreateCompoundClip,
    JoinThroughEdits,
    OpenPasteAttributesDialog,
    ClipRollTrimmed {
        track_id: String,
        left_clip_id: String,
        right_clip_id: String,
        delta_secs: f64,
    },
    ClipSlideTrimmed {
        track_id: String,
        clip_id: String,
        left_clip_id: Option<String>,
        right_clip_id: Option<String>,
        delta_secs: f64,
    },
}
