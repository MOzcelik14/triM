//! Dialog modals for triM.

pub mod audio_mixer_dialog;
pub mod close_confirm_dialog;
pub mod export_dialog;
pub mod recovery_dialog;
pub mod screen_record_dialog;
pub mod shortcuts_dialog;
pub mod silence_dialog;
pub mod speed_dialog;
pub mod title_dialog;
pub mod transcribe_dialog;
pub mod welcome_dialog;

pub use audio_mixer_dialog::{AudioMixerAction, AudioMixerDialog};
pub use close_confirm_dialog::{CloseConfirmAction, CloseConfirmDialog, PendingCloseAction};
pub use export_dialog::{ExportDialog, ExportDialogAction};
pub use recovery_dialog::{RecoveryAction, RecoveryDialog};
pub use screen_record_dialog::{ScreenRecordAction, ScreenRecordDialog};
pub use shortcuts_dialog::ShortcutsDialog;
pub use silence_dialog::{SilenceDialog, SilenceDialogAction};
pub use speed_dialog::{SpeedDialog, SpeedDialogAction};
pub use title_dialog::{TitleDialog, TitleDialogAction, TitlePosition};
pub use transcribe_dialog::{TranscribeDialog, TranscribeDialogAction};
pub use welcome_dialog::{WelcomeAction, WelcomeDialog};

