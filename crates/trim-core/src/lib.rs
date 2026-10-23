//! triM. Core models, types, and timecode utilities.

pub mod clip;
pub mod error;
pub mod keyframe;
pub mod marker;
pub mod media;
pub mod paths;
pub mod project;
pub mod subtitles;
pub mod time;
pub mod timeline;
pub mod track;

pub use clip::{AudioEqPreset, Clip, LutPreset, TextOverlay};
pub use error::CoreError;
pub use keyframe::{interpolate_keyframes, Interpolation, Keyframe};
pub use marker::Marker;
pub use media::{MediaItem, MediaType};
pub use project::{Project, ProjectSettings};
pub use subtitles::{
    clips_to_subtitle_entries, export_srt, parse_srt, parse_vtt, subtitle_entries_to_clips, SubtitleEntry,
};
pub use time::{seconds_to_timecode, timecode_to_seconds};
pub use timeline::{AspectRatio, TimelineModel};
pub use track::{Track, TrackType};
