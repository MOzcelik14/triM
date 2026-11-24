//! triM. Core models, types, and timecode utilities.

pub mod clip;
pub mod color;
pub mod compositing;
pub mod error;
pub mod interchange;
pub mod keyframe;
pub mod keymap;
pub mod looks;
pub mod lru;
pub mod marker;
pub mod mask;
pub mod media;
pub mod paths;
pub mod preferences;
pub mod project;
pub mod scopes;
pub mod sequence;
pub mod subtitles;
pub mod time;
pub mod timeline;
pub mod track;

pub use clip::{
    AudioCompressorConfig, AudioEqPreset, Clip, CornerPinConfig, LutPreset, ParametricEqBand,
    ParametricEqConfig, TextOverlay,
};
pub use color::{
    analyze_auto_white_balance_rgba, analyze_shot_match_rgba, apply_color_grade_normalized,
    bake_clip_grade_to_cube, compute_wb_from_neutral_rgb, white_balance_gains,
    BrightnessContrastPreset, CameraLogProfile, ColorGradeParams, CustomCurvesConfig,
    FilmStockFormat, HslCurvesConfig, HslQualifierConfig, LookCategory, WhiteBalancePreset,
};
pub use compositing::{BlendMode, ClipKind};
pub use error::CoreError;
pub use interchange::{
    export_cmx3600_edl, export_fcpxml, export_youtube_chapters, generate_ffmetadata_chapters,
};
pub use keyframe::{interpolate_keyframes, Interpolation, Keyframe};
pub use lru::LruCache;
pub use marker::Marker;
pub use mask::{Mask, MaskShape, MaskTarget, MaskVertex};
pub use media::{MediaItem, MediaType, Subclip};
pub use preferences::UserPreferences;
pub use project::{Project, ProjectSettings};
pub use scopes::{
    compute_false_color_rgba, compute_scopes_rgba, compute_zebra_rgba, false_color_map_ire,
    rec709_cb_cr, rec709_cie1931_primaries, rec709_luma_norm, rgb_to_cie1931_xy, sample_pixel_rgba,
    vectorscope_targets_75, PixelSampleReadout, ScopeConfig, ScopeData,
};
pub use sequence::{flatten_timeline_sequences, Sequence};
pub use subtitles::{
    clips_to_subtitle_entries, export_srt, parse_srt, parse_vtt, subtitle_entries_to_clips,
    SubtitleEntry,
};
pub use time::{seconds_to_timecode, timecode_to_seconds};
pub use timeline::{AspectRatio, TimelineModel};
pub use track::{Track, TrackType};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
