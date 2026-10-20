//! Dialog modals for triM.

pub mod audio_mixer_dialog;
pub mod export_dialog;
pub mod recovery_dialog;
pub mod shortcuts_dialog;
pub mod speed_dialog;
pub mod title_dialog;
pub mod welcome_dialog;

pub use audio_mixer_dialog::{AudioMixerAction, AudioMixerDialog};
pub use export_dialog::{ExportDialog, ExportDialogAction};
pub use recovery_dialog::{RecoveryAction, RecoveryDialog};
pub use shortcuts_dialog::ShortcutsDialog;
pub use speed_dialog::{SpeedDialog, SpeedDialogAction};
pub use title_dialog::{TitleDialog, TitleDialogAction, TitlePosition};
pub use welcome_dialog::{WelcomeAction, WelcomeDialog};
