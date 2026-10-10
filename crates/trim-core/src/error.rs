//! Core error definitions.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Track not found: {0}")]
    TrackNotFound(String),

    #[error("Clip not found: {0}")]
    ClipNotFound(String),

    #[error("Media not found: {0}")]
    MediaNotFound(String),

    #[error("Invalid timecode: {0}")]
    InvalidTimecode(String),
}
