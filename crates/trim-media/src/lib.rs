//! Media probing, thumbnail generation, waveform extraction, and audio/video mixing.

pub mod analyzer;
pub mod audio;
pub mod ffmpeg;
pub mod thumbnail;
pub mod video;
pub mod waveform;

pub use analyzer::{FFprobeAnalyzer, ProbeError};
pub use audio::{AudioMixer, AudioStreamPlayer, SAMPLE_RATE};
pub use ffmpeg::{apply_no_window, create_command, ffmpeg_cmd, ffplay_cmd, ffprobe_cmd};
pub use thumbnail::ThumbnailGenerator;
pub use video::{decode_frame_raw, generate_title_image, AsyncFrameDecoder, PlaybackStreamer, VideoEngine, VideoFrameDecoder};
pub use waveform::WaveformGenerator;
