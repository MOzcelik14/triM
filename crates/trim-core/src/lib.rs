//! triM. Core models, types, and timecode utilities.

pub mod clip;
pub mod color;
pub mod compositing;
pub mod error;
pub mod keyframe;
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

pub use clip::{AudioEqPreset, Clip, LutPreset, TextOverlay};
pub use color::{
    analyze_auto_white_balance_rgba, apply_color_grade_normalized, white_balance_gains,
    BrightnessContrastPreset, ColorGradeParams, LookCategory, WhiteBalancePreset,
};
pub use compositing::{BlendMode, ClipKind};
pub use error::CoreError;
pub use keyframe::{interpolate_keyframes, Interpolation, Keyframe};
pub use lru::LruCache;
pub use marker::Marker;
pub use mask::{Mask, MaskShape, MaskTarget, MaskVertex};
pub use media::{MediaItem, MediaType};
pub use preferences::UserPreferences;
pub use project::{Project, ProjectSettings};
pub use scopes::{
    compute_false_color_rgba, compute_scopes_rgba, compute_zebra_rgba, false_color_map_ire,
    rec709_cb_cr, rec709_luma_norm, vectorscope_targets_75, ScopeConfig, ScopeData,
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
pub const VERSION_CODENAME: &str = "Karadeniz";
