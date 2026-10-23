//! Media probing, thumbnail generation, waveform extraction, and audio/video mixing.

pub mod analyzer;
pub mod audio;
pub mod ffmpeg;
pub mod proxy;
pub mod scene_detect;
pub mod screen_record;
pub mod silence_detect;
pub mod thumbnail;
pub mod transcribe;
pub mod video;
pub mod voiceover;
pub mod waveform;

pub use analyzer::{FFprobeAnalyzer, ProbeError};
pub use audio::{AudioMixer, AudioStreamPlayer, SAMPLE_RATE};
pub use ffmpeg::{apply_no_window, create_command, detect_available_encoders, ffmpeg_cmd, ffplay_cmd, ffprobe_cmd, AvailableEncoders};
pub use proxy::{default_proxy_path_for, generate_proxy};
pub use scene_detect::{detect_scene_changes, parse_scene_cut_timestamps};
pub use screen_record::{CaptureSource, ScreenRecorder};
pub use silence_detect::{calculate_speech_intervals, detect_silence_regions, parse_silence_timestamps};
pub use thumbnail::ThumbnailGenerator;
pub use transcribe::{extract_audio_for_whisper, find_whisper_executable, run_whisper_transcription};
pub use video::{decode_frame_raw, generate_title_image, AsyncFrameDecoder, PlaybackStreamer, VideoEngine, VideoFrameDecoder};
pub use voiceover::VoiceoverRecorder;
pub use waveform::WaveformGenerator;

