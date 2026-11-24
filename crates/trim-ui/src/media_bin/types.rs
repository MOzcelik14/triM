#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LeftPanelTab {
    #[default]
    Project,
    Effects,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaFilterType {
    #[default]
    All,
    Video,
    Audio,
    Image,
}

pub enum MediaBinAction {
    ImportRequested,
    AddToTimeline(String),       // media_id or sequence_id
    OpenInSourceMonitor(String), // media_id
    GenerateProxy(String),       // media_id
    DeleteMedia(String),         // media_id
    CreateBinPrompt,
    RenameBinPrompt(String),  // bin_id
    DeleteBin(String),        // bin_id
    EnterBin(Option<String>), // bin_id (None = root)
    MoveToBin {
        media_id: String,
        bin_id: Option<String>,
    },
    OpenSequence(String), // sequence_id
    CreateSequencePrompt,
    DuplicateSequence(String), // sequence_id
    DeleteSequence(String),    // sequence_id
    GenerateSoundscape(trim_media::SoundscapePreset),
}
