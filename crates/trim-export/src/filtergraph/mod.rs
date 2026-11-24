//! FFmpeg complex filtergraph pipeline generator.
//!
//! The export graph mirrors the interactive preview compositor
//! (`trim_media::video::draw_clip_onto_canvas` and
//! `trim_media::audio::AudioMixer::render_audio_interval`):
//!
//! * A black base canvas spanning the whole timeline is generated.
//! * Every visible video track is composited bottom-to-top (V1, V2, ...).
//!   Each clip is decoded, retimed, scaled, colour graded, faded and then
//!   `overlay`-ed onto the canvas at its timeline position. Overlapping
//!   tracks therefore produce layers instead of being concatenated.
//! * Audio from every non-muted clip (on video *and* audio tracks, honouring
//!   solo/mute and track volume) is delayed to its timeline position and
//!   summed with `amix`, anchored to timeline time.

mod clip_filters;
mod helpers;
mod pipeline;
#[cfg(test)]
mod tests_a;
#[cfg(test)]
mod tests_b;

pub(super) const PREVIEW_W: f64 = 640.0;
pub(super) const PREVIEW_H: f64 = 360.0;

pub struct TimelineExporter;

/// Prepared FFmpeg export invocation along with a private RAII temporary directory
/// holding baked LUTs, mask PGMs, and any `-filter_complex_script` file.
pub struct PreparedExport {
    pub cmd_args: Vec<String>,
    pub total_duration: f64,
    pub temp_dir: Option<tempfile::TempDir>,
}
