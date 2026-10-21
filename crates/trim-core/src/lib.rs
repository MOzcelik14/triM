//! triM. Core models, types, and timecode utilities.

pub mod clip;
pub mod error;
pub mod keyframe;
pub mod marker;
pub mod media;
pub mod paths;
pub mod project;
pub mod time;
pub mod timeline;
pub mod track;

pub use clip::Clip;
pub use error::CoreError;
pub use keyframe::{interpolate_keyframes, Interpolation, Keyframe};
pub use marker::Marker;
pub use media::{MediaItem, MediaType};
pub use project::{Project, ProjectSettings};
pub use time::{seconds_to_timecode, timecode_to_seconds};
pub use timeline::TimelineModel;
pub use track::{Track, TrackType};
