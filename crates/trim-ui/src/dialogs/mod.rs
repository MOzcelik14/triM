//! Dialog modals for triM.

pub mod export_dialog;
pub mod recovery_dialog;
pub mod speed_dialog;
pub mod title_dialog;

pub use export_dialog::{ExportDialog, ExportDialogAction};
pub use recovery_dialog::{RecoveryAction, RecoveryDialog};
pub use speed_dialog::{SpeedDialog, SpeedDialogAction};
pub use title_dialog::{TitleDialog, TitleDialogAction};
