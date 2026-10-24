//! Asynchronous native file dialog threading bridge using `std::thread` and `mpsc::sync_channel(1)`.

use crate::dialogs::PendingCloseAction;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileDialogMode {
    PickFile,
    SaveFile,
    PickFolder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDialogFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDialogConfig {
    pub title: String,
    pub default_file_name: Option<String>,
    pub filters: Vec<FileDialogFilter>,
    pub mode: FileDialogMode,
}

impl FileDialogConfig {
    pub fn pick_file(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            default_file_name: None,
            filters: Vec::new(),
            mode: FileDialogMode::PickFile,
        }
    }

    pub fn save_file(title: impl Into<String>, default_file_name: Option<String>) -> Self {
        Self {
            title: title.into(),
            default_file_name,
            filters: Vec::new(),
            mode: FileDialogMode::SaveFile,
        }
    }

    pub fn pick_folder(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            default_file_name: None,
            filters: Vec::new(),
            mode: FileDialogMode::PickFolder,
        }
    }

    pub fn add_filter(mut self, name: impl Into<String>, extensions: &[&str]) -> Self {
        self.filters.push(FileDialogFilter {
            name: name.into(),
            extensions: extensions.iter().map(|s| (*s).to_string()).collect(),
        });
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileDialogRequest {
    OpenProject,
    SaveProject {
        after_save: Option<PendingCloseAction>,
    },
    SaveProjectAs,
    BundleProject {
        default_folder_name: String,
    },
    ImportSubtitles,
    ExportSubtitles {
        srt_text: String,
        entry_count: usize,
    },
    ExportEdl {
        edl_text: String,
    },
    ExportFcpxml {
        xml_text: String,
    },
    ImportMedia,
    SelectExportOutput,
    SelectCustomLut {
        clip_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDialogResponse {
    pub request: FileDialogRequest,
    pub path: Option<PathBuf>,
}

#[derive(Default)]
pub struct FileDialogBridge {
    pending_rx: Option<Receiver<FileDialogResponse>>,
}

impl std::fmt::Debug for FileDialogBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileDialogBridge")
            .field("is_pending", &self.pending_rx.is_some())
            .finish()
    }
}

impl Clone for FileDialogBridge {
    fn clone(&self) -> Self {
        Self { pending_rx: None }
    }
}

impl FileDialogBridge {
    pub fn new() -> Self {
        Self { pending_rx: None }
    }

    pub fn is_pending(&self) -> bool {
        self.pending_rx.is_some()
    }

    pub fn open(&mut self, request: FileDialogRequest, config: FileDialogConfig) -> bool {
        self.open_with_runner(request, move || {
            let mut dlg = rfd::FileDialog::new().set_title(&config.title);
            if let Some(ref fname) = config.default_file_name {
                dlg = dlg.set_file_name(fname);
            }
            for filter in &config.filters {
                let ext_refs: Vec<&str> = filter.extensions.iter().map(String::as_str).collect();
                dlg = dlg.add_filter(&filter.name, &ext_refs);
            }
            match config.mode {
                FileDialogMode::PickFile => dlg.pick_file(),
                FileDialogMode::SaveFile => dlg.save_file(),
                FileDialogMode::PickFolder => dlg.pick_folder(),
            }
        })
    }

    pub fn open_with_runner<F>(&mut self, request: FileDialogRequest, runner: F) -> bool
    where
        F: FnOnce() -> Option<PathBuf> + Send + 'static,
    {
        if self.pending_rx.is_some() {
            return false;
        }
        let (tx, rx) = mpsc::sync_channel(1);
        self.pending_rx = Some(rx);

        std::thread::spawn(move || {
            let path = runner();
            let _ = tx.send(FileDialogResponse { request, path });
        });

        true
    }

    pub fn poll(&mut self) -> Option<FileDialogResponse> {
        let rx = self.pending_rx.as_ref()?;
        match rx.try_recv() {
            Ok(resp) => {
                self.pending_rx = None;
                Some(resp)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.pending_rx = None;
                None
            }
        }
    }
}
