//! Command pattern and Undo/Redo stack for triM.

pub mod commands;

pub use commands::*;
use thiserror::Error;
use trim_core::timeline::TimelineModel;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Track not found: {0}")]
    TrackNotFound(String),

    #[error("Clip not found: {0}")]
    ClipNotFound(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// Command trait representing reversible editing actions.
pub trait Command: std::fmt::Debug + Send + Sync {
    fn execute(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError>;
    fn undo(&mut self, timeline: &mut TimelineModel) -> Result<(), CommandError>;
    fn name(&self) -> &str;
}

/// Stack of reversible commands.
#[derive(Default, Debug)]
pub struct UndoStack {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
}

impl UndoStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, mut command: Box<dyn Command>, timeline: &mut TimelineModel) -> Result<(), CommandError> {
        command.execute(timeline)?;
        self.undo_stack.push(command);
        self.redo_stack.clear();
        Ok(())
    }

    pub fn undo(&mut self, timeline: &mut TimelineModel) -> Result<bool, CommandError> {
        if let Some(mut command) = self.undo_stack.pop() {
            command.undo(timeline)?;
            self.redo_stack.push(command);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn redo(&mut self, timeline: &mut TimelineModel) -> Result<bool, CommandError> {
        if let Some(mut command) = self.redo_stack.pop() {
            command.execute(timeline)?;
            self.undo_stack.push(command);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_text(&self) -> Option<&str> {
        self.undo_stack.last().map(|c| c.name())
    }

    pub fn redo_text(&self) -> Option<&str> {
        self.redo_stack.last().map(|c| c.name())
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trim_core::clip::Clip;
    use trim_core::track::{Track, TrackType};

    #[test]
    fn test_add_and_remove_clip_command() {
        let mut tl = TimelineModel::default();
        let mut stack = UndoStack::new();

        let track_id = tl.tracks[0].id.clone();
        let clip = Clip::new("m1", 2.0, 6.0);
        let clip_id = clip.id.clone();

        // 1. Add Clip
        stack.push(Box::new(AddClipCommand::new(&track_id, clip)), &mut tl).unwrap();
        assert_eq!(tl.tracks[0].clips.len(), 1);

        // Undo
        assert!(stack.undo(&mut tl).unwrap());
        assert_eq!(tl.tracks[0].clips.len(), 0);

        // Redo
        assert!(stack.redo(&mut tl).unwrap());
        assert_eq!(tl.tracks[0].clips.len(), 1);

        // 2. Remove Clip
        stack.push(Box::new(RemoveClipCommand::new(&track_id, &clip_id)), &mut tl).unwrap();
        assert_eq!(tl.tracks[0].clips.len(), 0);

        // Undo Remove
        assert!(stack.undo(&mut tl).unwrap());
        assert_eq!(tl.tracks[0].clips.len(), 1);
    }

    #[test]
    fn test_split_clip_command() {
        let mut tl = TimelineModel::default();
        let mut stack = UndoStack::new();

        let track_id = tl.tracks[0].id.clone();
        let clip = Clip::new("m1", 0.0, 10.0);
        let clip_id = clip.id.clone();
        tl.tracks[0].add_clip(clip);

        // Split at 4.0s
        stack.push(Box::new(SplitClipCommand::new(&track_id, &clip_id, 4.0)), &mut tl).unwrap();
        assert_eq!(tl.tracks[0].clips.len(), 2);
        assert_eq!(tl.tracks[0].clips[0].timeline_out, 4.0);
        assert_eq!(tl.tracks[0].clips[1].timeline_in, 4.0);

        // Undo Split
        assert!(stack.undo(&mut tl).unwrap());
        assert_eq!(tl.tracks[0].clips.len(), 1);
        assert_eq!(tl.tracks[0].clips[0].duration(), 10.0);
    }

    #[test]
    fn test_ripple_delete_command() {
        let mut tl = TimelineModel::default();
        let mut stack = UndoStack::new();

        let track_id = tl.tracks[0].id.clone();
        let c1 = Clip::new("m1", 0.0, 3.0);
        let c2 = Clip::new("m2", 5.0, 10.0);
        let c1_id = c1.id.clone();
        tl.tracks[0].add_clip(c1);
        tl.tracks[0].add_clip(c2);

        // Ripple delete c1 (dur = 3.0s)
        stack.push(Box::new(RippleDeleteCommand::new(&track_id, &c1_id)), &mut tl).unwrap();
        assert_eq!(tl.tracks[0].clips.len(), 1);
        // c2 should be shifted left from 5.0 to 2.0
        assert_eq!(tl.tracks[0].clips[0].timeline_in, 2.0);

        // Undo
        assert!(stack.undo(&mut tl).unwrap());
        assert_eq!(tl.tracks[0].clips.len(), 2);
        assert_eq!(tl.tracks[0].clips[1].timeline_in, 5.0);
    }

    #[test]
    fn test_change_clip_speed_command() {
        let mut tl = TimelineModel::default();
        let mut stack = UndoStack::new();

        let track_id = tl.tracks[0].id.clone();
        let mut clip = Clip::new("m1", 2.0, 6.0); // dur = 4.0
        clip.source_in = 0.0;
        clip.source_out = Some(4.0);
        let clip_id = clip.id.clone();
        tl.tracks[0].add_clip(clip);

        // Change speed to 2.0x
        stack.push(Box::new(ChangeClipSpeedCommand::new(&track_id, &clip_id, 2.0, true)), &mut tl).unwrap();
        let c = tl.tracks[0].get_clip_by_id(&clip_id).unwrap();
        assert_eq!(c.speed, 2.0);
        assert!(c.reverse);
        assert!((c.duration() - 2.0).abs() < 1e-3);

        // Undo
        assert!(stack.undo(&mut tl).unwrap());
        let c = tl.tracks[0].get_clip_by_id(&clip_id).unwrap();
        assert_eq!(c.speed, 1.0);
        assert!(!c.reverse);
        assert!((c.duration() - 4.0).abs() < 1e-3);
    }
}
