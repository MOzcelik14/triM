//! triM. User Interface components and main application.

pub mod app;
pub mod canvas;
pub mod dialogs;
pub mod inspector;
pub mod media_bin;
pub mod monitor;
pub mod ruler;
pub mod theme;
pub mod track_header;
pub mod transport;
pub mod vu_meter;

pub use app::TrimApp;
pub use canvas::TimelineCanvasState;
pub use theme::setup_theme;
pub use vu_meter::VuMeterState;
