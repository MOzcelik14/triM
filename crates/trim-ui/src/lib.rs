//! triM. User Interface components and main application.

pub mod app;
pub mod canvas;
pub mod color_wheel;
pub mod dialogs;
pub mod file_dialog;
pub use trim_render as gpu_compositor;
pub mod i18n;
pub mod inspector;
pub mod media_bin;
pub mod monitor;
pub mod ruler;
pub mod scopes_panel;
pub mod shortcuts;
pub mod theme;
pub mod toolbar;
pub mod track_header;
pub mod transport;
pub mod vu_meter;

pub use app::TrimApp;
pub use canvas::TimelineCanvasState;
pub use dialogs::{PreferencesAction, PreferencesDialog, SplashScreen};
pub use file_dialog::{
    FileDialogBridge, FileDialogConfig, FileDialogMode, FileDialogRequest, FileDialogResponse,
};
pub use gpu_compositor::{GpuCompositor, LayerUniforms, GPU_PREVIEW_SCALE};
pub use i18n::{tr, Language};
pub use media_bin::LeftPanelTab;
pub use shortcuts::ShortcutProfile;
pub use theme::setup_theme;
pub use vu_meter::VuMeterState;

#[cfg(test)]
mod tests;
