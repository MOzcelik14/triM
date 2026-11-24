//! Interactive timeline canvas rendering tracks, clips, waveforms, fades, keyframes, playhead and editing gestures.

mod gaps;
mod geometry;
mod input_drag;
mod input_press;
mod input_release;
mod overlays;
mod render;
mod state;
#[cfg(test)]
mod tests;
mod tracks;

pub use geometry::*;
pub use render::*;
pub use state::*;
