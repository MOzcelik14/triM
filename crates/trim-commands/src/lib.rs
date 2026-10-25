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

    pub fn push(
        &mut self,
        mut command: Box<dyn Command>,
        timeline: &mut TimelineModel,
    ) -> Result<(), CommandError> {
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

    pub fn len(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn is_empty(&self) -> bool {
        self.undo_stack.is_empty()
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

    #[test]
    fn test_add_and_remove_clip_command() {
        let mut tl = TimelineModel::default();
        let mut stack = UndoStack::new();

        let track_id = tl.tracks[0].id.clone();
        let clip = Clip::new("m1", 2.0, 6.0);
        let clip_id = clip.id.clone();

        // 1. Add Clip
        stack
            .push(Box::new(AddClipCommand::new(&track_id, clip)), &mut tl)
            .unwrap();
        assert_eq!(tl.tracks[0].clips.len(), 1);

        // Undo
        assert!(stack.undo(&mut tl).unwrap());
        assert_eq!(tl.tracks[0].clips.len(), 0);

        // Redo
        assert!(stack.redo(&mut tl).unwrap());
        assert_eq!(tl.tracks[0].clips.len(), 1);

        // 2. Remove Clip
        stack
            .push(
                Box::new(RemoveClipCommand::new(&track_id, &clip_id)),
                &mut tl,
            )
            .unwrap();
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
        let mut clip = Clip::new("m1", 0.0, 10.0);
        clip.add_keyframe(
            "opacity",
            2.0,
            0.5,
            trim_core::keyframe::Interpolation::Linear,
        );
        clip.add_keyframe(
            "opacity",
            7.0,
            0.9,
            trim_core::keyframe::Interpolation::Linear,
        );
        clip.fade_in = 1.0;
        clip.fade_out = 1.5;
        let clip_id = clip.id.clone();
        tl.tracks[0].add_clip(clip);

        // Split at 4.0s
        stack
            .push(
                Box::new(SplitClipCommand::new(&track_id, &clip_id, 4.0)),
                &mut tl,
            )
            .unwrap();
        assert_eq!(tl.tracks[0].clips.len(), 2);
        assert_eq!(tl.tracks[0].clips[0].timeline_out, 4.0);
        assert_eq!(tl.tracks[0].clips[1].timeline_in, 4.0);

        // Verify keyframes partitioning
        let left = &tl.tracks[0].clips[0];
        let right = &tl.tracks[0].clips[1];
        assert_eq!(left.keyframes.get("opacity").unwrap().len(), 1);
        assert_eq!(left.keyframes.get("opacity").unwrap()[0].time, 2.0);
        assert_eq!(left.fade_in, 1.0);
        assert_eq!(left.fade_out, 0.0); // inner cut reset

        assert_eq!(right.keyframes.get("opacity").unwrap().len(), 1);
        assert_eq!(right.keyframes.get("opacity").unwrap()[0].time, 3.0); // 7.0 - 4.0 = 3.0
        assert_eq!(right.fade_in, 0.0); // inner cut reset
        assert_eq!(right.fade_out, 1.5);

        // Undo Split
        assert!(stack.undo(&mut tl).unwrap());
        assert_eq!(tl.tracks[0].clips.len(), 1);
        assert_eq!(tl.tracks[0].clips[0].duration(), 10.0);
        assert_eq!(
            tl.tracks[0].clips[0]
                .keyframes
                .get("opacity")
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn test_split_clip_undo_redo_preserves_uuid_for_subsequent_commands() {
        let mut tl = TimelineModel::default();
        let mut stack = UndoStack::new();

        let track_id = tl.tracks[0].id.clone();
        let clip = Clip::new("m1", 0.0, 10.0);
        let clip_id = clip.id.clone();
        tl.tracks[0].add_clip(clip);

        // 1. Split at 4.0s
        stack
            .push(
                Box::new(SplitClipCommand::new(&track_id, &clip_id, 4.0)),
                &mut tl,
            )
            .unwrap();

        assert_eq!(tl.tracks[0].clips.len(), 2);
        // Find the right clip ID generated by the split
        let right_clip_id = tl.tracks[0].clips[1].id.clone();

        // 2. Subsequent command operating on the right clip (e.g. MoveClipCommand)
        stack
            .push(
                Box::new(MoveClipCommand::new(
                    &track_id,
                    &track_id,
                    &right_clip_id,
                    4.0,
                    5.0,
                )),
                &mut tl,
            )
            .unwrap();

        assert_eq!(tl.tracks[0].clips[1].timeline_in, 5.0);

        // 3. Undo both commands: undo move, undo split
        assert!(stack.undo(&mut tl).unwrap()); // Undo move
        assert_eq!(tl.tracks[0].clips[1].timeline_in, 4.0);
        assert!(stack.undo(&mut tl).unwrap()); // Undo split
        assert_eq!(tl.tracks[0].clips.len(), 1);

        // 4. Redo both commands: redo split, redo move
        assert!(stack.redo(&mut tl).unwrap(), "Redo split should succeed");
        assert_eq!(tl.tracks[0].clips.len(), 2);

        // Crucial: right clip MUST preserve its original UUID so MoveClipCommand can find it!
        assert_eq!(
            tl.tracks[0].clips[1].id, right_clip_id,
            "Split redo must preserve the exact right_clip UUID"
        );

        // Redo move command on that same right clip
        assert!(
            stack.redo(&mut tl).unwrap(),
            "Redo move should succeed on preserved UUID"
        );
        assert_eq!(tl.tracks[0].clips[1].timeline_in, 5.0);
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
        stack
            .push(
                Box::new(RippleDeleteCommand::new(&track_id, &c1_id)),
                &mut tl,
            )
            .unwrap();
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
        stack
            .push(
                Box::new(ChangeClipSpeedCommand::new(&track_id, &clip_id, 2.0, true)),
                &mut tl,
            )
            .unwrap();
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

        // Slow down to 0.5x -> duration should stretch to 8.0s
        stack
            .push(
                Box::new(ChangeClipSpeedCommand::new(&track_id, &clip_id, 0.5, false)),
                &mut tl,
            )
            .unwrap();
        let c = tl.tracks[0].get_clip_by_id(&clip_id).unwrap();
        assert_eq!(c.speed, 0.5);
        assert!(!c.reverse);
        assert!((c.duration() - 8.0).abs() < 1e-3);
        assert_eq!(c.timeline_in, 2.0);
        assert_eq!(c.timeline_out, 10.0);
    }

    #[test]
    fn test_move_clip_across_tracks() {
        let mut tl = TimelineModel::default();
        let mut stack = UndoStack::new();

        // Starts with V1 and A1
        let v2_id = tl.add_new_video_track(); // index 0 is V2, index 1 is V1
        let v1_id = tl.tracks[1].id.clone();

        let clip = Clip::new("m1", 2.0, 6.0);
        let clip_id = clip.id.clone();
        tl.tracks[1].add_clip(clip); // add to V1

        assert_eq!(tl.tracks[1].clips.len(), 1);
        assert_eq!(tl.tracks[0].clips.len(), 0);

        // Move clip from V1 to V2 at new_in = 5.0
        let cmd = Box::new(MoveClipCommand::new(&v1_id, &v2_id, &clip_id, 2.0, 5.0));
        stack.push(cmd, &mut tl).unwrap();

        // Now V1 has 0 clips, V2 has 1 clip at 5.0
        assert_eq!(tl.tracks[1].clips.len(), 0);
        assert_eq!(tl.tracks[0].clips.len(), 1);
        assert_eq!(tl.tracks[0].clips[0].timeline_in, 5.0);

        // Undo -> clip returns to V1 at 2.0
        assert!(stack.undo(&mut tl).unwrap());
        assert_eq!(tl.tracks[1].clips.len(), 1);
        assert_eq!(tl.tracks[0].clips.len(), 0);
        assert_eq!(tl.tracks[1].clips[0].timeline_in, 2.0);

        // Redo -> clip moves back to V2 at 5.0
        assert!(stack.redo(&mut tl).unwrap());
        assert_eq!(tl.tracks[1].clips.len(), 0);
        assert_eq!(tl.tracks[0].clips.len(), 1);
        assert_eq!(tl.tracks[0].clips[0].timeline_in, 5.0);
    }

    #[test]
    fn test_add_and_remove_track_command() {
        let mut tl = TimelineModel::default();
        let mut stack = UndoStack::new();

        assert_eq!(tl.tracks.len(), 4);
        let new_track = trim_core::track::Track::new("V3", trim_core::track::TrackType::Video);
        let tr_id = new_track.id.clone();

        // Add track at index 0
        stack
            .push(Box::new(AddTrackCommand::new(new_track, 0)), &mut tl)
            .unwrap();
        assert_eq!(tl.tracks.len(), 5);
        assert_eq!(tl.tracks[0].name, "V3");

        // Undo -> back to 4 tracks
        assert!(stack.undo(&mut tl).unwrap());
        assert_eq!(tl.tracks.len(), 4);

        // Redo -> 5 tracks
        assert!(stack.redo(&mut tl).unwrap());
        assert_eq!(tl.tracks.len(), 5);

        // Remove track
        stack
            .push(Box::new(RemoveTrackCommand::new(&tr_id)), &mut tl)
            .unwrap();
        assert_eq!(tl.tracks.len(), 4);

        // Undo Remove -> restored
        assert!(stack.undo(&mut tl).unwrap());
        assert_eq!(tl.tracks.len(), 5);
        assert_eq!(tl.tracks[0].name, "V3");
    }

    #[test]
    fn test_batch_split_clip_command() {
        let mut tl = TimelineModel::default();
        let mut stack = UndoStack::new();

        let track_id = tl.tracks[0].id.clone();
        let clip = Clip::new("media1", 0.0, 10.0);
        let clip_id = clip.id.clone();
        tl.tracks[0].add_clip(clip);

        assert_eq!(tl.tracks[0].clips.len(), 1);

        // Split at 2.5 and 7.0
        let cmd = Box::new(BatchSplitClipCommand::new(
            &track_id,
            &clip_id,
            vec![2.5, 7.0],
        ));
        stack.push(cmd, &mut tl).unwrap();

        // Should now have 3 clips: [0.0..2.5], [2.5..7.0], [7.0..10.0]
        assert_eq!(tl.tracks[0].clips.len(), 3);
        let mut sorted = tl.tracks[0].clips.clone();
        sorted.sort_by(|a, b| a.timeline_in.partial_cmp(&b.timeline_in).unwrap());
        assert_eq!(sorted[0].timeline_in, 0.0);
        assert_eq!(sorted[0].timeline_out, 2.5);
        assert_eq!(sorted[1].timeline_in, 2.5);
        assert_eq!(sorted[1].timeline_out, 7.0);
        assert_eq!(sorted[2].timeline_in, 7.0);
        assert_eq!(sorted[2].timeline_out, 10.0);

        // Undo -> back to 1 original clip
        assert!(stack.undo(&mut tl).unwrap());
        assert_eq!(tl.tracks[0].clips.len(), 1);
        assert_eq!(tl.tracks[0].clips[0].id, clip_id);
        assert_eq!(tl.tracks[0].clips[0].timeline_in, 0.0);
        assert_eq!(tl.tracks[0].clips[0].timeline_out, 10.0);
    }

    #[test]
    fn test_auto_silence_cut_command() {
        let mut tl = TimelineModel::default();
        let mut stack = UndoStack::new();

        let track_id = tl.tracks[0].id.clone();
        let clip = Clip::new("speech_media", 0.0, 10.0);
        let clip_id = clip.id.clone();
        tl.tracks[0].add_clip(clip);

        // Subsequent clip at 12.0
        let next_clip = Clip::new("other_media", 12.0, 15.0);
        let next_id = next_clip.id.clone();
        tl.tracks[0].add_clip(next_clip);

        // Suppose silence is between 3.0 and 6.0. Speech intervals are [0.0..3.0] and [6.0..10.0]
        let speech_intervals = vec![(0.0, 3.0), (6.0, 10.0)];
        let cmd = Box::new(AutoSilenceCutCommand::new(
            &track_id,
            &clip_id,
            speech_intervals,
        ));
        stack.push(cmd, &mut tl).unwrap();

        // Original clip of 10s replaced by 3s + 4s = 7s.
        // Old end was 10.0, new end is 7.0, silence eliminated = 3.0s.
        // Track now has 2 speech subclips + 1 shifted subsequent clip = 3 clips total.
        assert_eq!(tl.tracks[0].clips.len(), 3);
        let c0 = &tl.tracks[0].clips[0];
        let c1 = &tl.tracks[0].clips[1];
        let c2 = &tl.tracks[0].clips[2];

        assert_eq!(c0.timeline_in, 0.0);
        assert_eq!(c0.timeline_out, 3.0);
        assert_eq!(c1.timeline_in, 3.0);
        assert_eq!(c1.timeline_out, 7.0);

        // next_clip shifted left by 3.0s (from 12.0 to 9.0)
        assert_eq!(c2.id, next_id);
        assert_eq!(c2.timeline_in, 9.0);
        assert_eq!(c2.timeline_out, 12.0);

        // Undo
        assert!(stack.undo(&mut tl).unwrap());
        assert_eq!(tl.tracks[0].clips.len(), 2);
        assert_eq!(tl.tracks[0].clips[0].id, clip_id);
        assert_eq!(tl.tracks[0].clips[0].timeline_out, 10.0);
        assert_eq!(tl.tracks[0].clips[1].id, next_id);
        assert_eq!(tl.tracks[0].clips[1].timeline_in, 12.0);
    }

    #[test]
    fn test_link_unlink_clips_commands() {
        let mut tl = TimelineModel::default();
        let mut stack = UndoStack::new();

        let v1_id = tl.tracks[0].id.clone();
        let a1_id = tl.tracks[1].id.clone();

        let c_video = Clip::new("v_media", 0.0, 5.0);
        let c_audio = Clip::new("a_media", 0.0, 5.0);
        let cv_id = c_video.id.clone();
        let ca_id = c_audio.id.clone();

        tl.tracks[0].add_clip(c_video);
        tl.tracks[1].add_clip(c_audio);

        // 1. Link them together
        let pairs = vec![
            (v1_id.clone(), cv_id.clone()),
            (a1_id.clone(), ca_id.clone()),
        ];
        let link_cmd = Box::new(LinkClipsCommand::new(pairs.clone()));
        stack.push(link_cmd, &mut tl).unwrap();

        let gid_v = tl.tracks[0]
            .get_clip_by_id(&cv_id)
            .unwrap()
            .linked_group_id
            .clone();
        let gid_a = tl.tracks[1]
            .get_clip_by_id(&ca_id)
            .unwrap()
            .linked_group_id
            .clone();
        assert!(gid_v.is_some());
        assert_eq!(gid_v, gid_a);

        // 2. Unlink them
        let unlink_cmd = Box::new(UnlinkClipsCommand::new(pairs));
        stack.push(unlink_cmd, &mut tl).unwrap();

        assert!(tl.tracks[0]
            .get_clip_by_id(&cv_id)
            .unwrap()
            .linked_group_id
            .is_none());
        assert!(tl.tracks[1]
            .get_clip_by_id(&ca_id)
            .unwrap()
            .linked_group_id
            .is_none());

        // Undo Unlink
        assert!(stack.undo(&mut tl).unwrap());
        assert_eq!(
            tl.tracks[0].get_clip_by_id(&cv_id).unwrap().linked_group_id,
            tl.tracks[1].get_clip_by_id(&ca_id).unwrap().linked_group_id
        );

        // Undo Link
        assert!(stack.undo(&mut tl).unwrap());
        assert!(tl.tracks[0]
            .get_clip_by_id(&cv_id)
            .unwrap()
            .linked_group_id
            .is_none());
    }

    #[test]
    fn test_close_gap_command() {
        let mut tl = TimelineModel::default();
        let tr_id = tl.add_new_video_track();

        let c1 = Clip::new("m1", 0.0, 2.0);
        let c2 = Clip::new("m2", 5.0, 8.0); // 3.0s gap from 2.0 to 5.0
        let c2_id = c2.id.clone();
        tl.tracks[0].add_clip(c1);
        tl.tracks[0].add_clip(c2);

        let mut stack = UndoStack::new();
        let cmd = Box::new(CloseGapCommand::new(Some(tr_id.clone()), 2.0, 5.0));
        stack.push(cmd, &mut tl).unwrap();

        // c2 should now start at 2.0 and end at 5.0
        let moved_c2 = tl.tracks[0].get_clip_by_id(&c2_id).unwrap();
        assert_eq!(moved_c2.timeline_in, 2.0);
        assert_eq!(moved_c2.timeline_out, 5.0);

        // Undo
        assert!(stack.undo(&mut tl).unwrap());
        let undone_c2 = tl.tracks[0].get_clip_by_id(&c2_id).unwrap();
        assert_eq!(undone_c2.timeline_in, 5.0);
        assert_eq!(undone_c2.timeline_out, 8.0);
    }

    #[test]
    fn test_set_clip_color_command() {
        let mut tl = TimelineModel::default();
        let tr_id = tl.add_new_video_track();
        let c = Clip::new("m1", 0.0, 3.0);
        let c_id = c.id.clone();
        tl.tracks[0].add_clip(c);

        let mut stack = UndoStack::new();
        let cmd = Box::new(SetClipColorCommand::new(
            tr_id.clone(),
            c_id.clone(),
            trim_core::clip::ColorLabel::Cyan,
        ));
        stack.push(cmd, &mut tl).unwrap();

        assert_eq!(
            tl.tracks[0].get_clip_by_id(&c_id).unwrap().color_label,
            trim_core::clip::ColorLabel::Cyan
        );

        assert!(stack.undo(&mut tl).unwrap());
        assert_eq!(
            tl.tracks[0].get_clip_by_id(&c_id).unwrap().color_label,
            trim_core::clip::ColorLabel::None
        );
    }

    #[test]
    fn test_set_clip_volume_command() {
        let mut tl = TimelineModel::default();
        let tr_id = tl.add_new_audio_track();
        let c = Clip::new("m1", 0.0, 3.0);
        let c_id = c.id.clone();
        tl.get_track_by_id_mut(&tr_id).unwrap().add_clip(c);

        let mut stack = UndoStack::new();
        let cmd = Box::new(SetClipVolumeCommand::new(
            tr_id.clone(),
            c_id.clone(),
            1.0,
            1.5,
        ));
        stack.push(cmd, &mut tl).unwrap();

        assert_eq!(
            tl.get_track_by_id(&tr_id)
                .unwrap()
                .get_clip_by_id(&c_id)
                .unwrap()
                .volume,
            1.5
        );

        assert!(stack.undo(&mut tl).unwrap());
        assert_eq!(
            tl.get_track_by_id(&tr_id)
                .unwrap()
                .get_clip_by_id(&c_id)
                .unwrap()
                .volume,
            1.0
        );
    }

    #[test]
    fn test_detach_audio_pairs_with_corresponding_audio_track_v2_to_a2() {
        let mut tl = TimelineModel::default();
        // default TimelineModel has V2, V1, A1, A2
        let v2_id = tl
            .tracks
            .iter()
            .find(|t| t.name == "V2")
            .map(|t| t.id.clone())
            .expect("V2 track must exist");
        let a1_id = tl
            .tracks
            .iter()
            .find(|t| t.name == "A1")
            .map(|t| t.id.clone())
            .expect("A1 track must exist");
        let a2_id = tl
            .tracks
            .iter()
            .find(|t| t.name == "A2")
            .map(|t| t.id.clone())
            .expect("A2 track must exist");

        let clip = Clip::new("m_video", 1.0, 6.0);
        let clip_id = clip.id.clone();
        tl.get_track_by_id_mut(&v2_id).unwrap().add_clip(clip);

        let mut stack = UndoStack::new();
        let cmd = Box::new(DetachAudioCommand::new(&v2_id, &clip_id));
        stack.push(cmd, &mut tl).unwrap();

        // The video clip on V2 must now be muted
        let v_clip = tl
            .get_track_by_id(&v2_id)
            .unwrap()
            .get_clip_by_id(&clip_id)
            .unwrap();
        assert!(
            v_clip.muted,
            "Video clip should be muted after detaching audio"
        );

        // The audio clip must be placed on A2, NOT A1!
        let a1_clips = &tl.get_track_by_id(&a1_id).unwrap().clips;
        let a2_clips = &tl.get_track_by_id(&a2_id).unwrap().clips;
        assert_eq!(a1_clips.len(), 0, "A1 track must not receive audio from V2");
        assert_eq!(a2_clips.len(), 1, "A2 track must receive audio from V2");

        // Undo
        assert!(stack.undo(&mut tl).unwrap());
        let v_clip_unmuted = tl
            .get_track_by_id(&v2_id)
            .unwrap()
            .get_clip_by_id(&clip_id)
            .unwrap();
        assert!(
            !v_clip_unmuted.muted,
            "Video clip should be unmuted after undo"
        );
        assert_eq!(tl.get_track_by_id(&a2_id).unwrap().clips.len(), 0);

        // Redo
        assert!(stack.redo(&mut tl).unwrap());
        assert_eq!(tl.get_track_by_id(&a2_id).unwrap().clips.len(), 1);
    }

    #[test]
    fn test_macro_command_atomic_undo_redo() {
        let mut tl = TimelineModel::default();
        let v1_id = tl
            .tracks
            .iter()
            .find(|t| t.name == "V1")
            .unwrap()
            .id
            .clone();
        let a1_id = tl
            .tracks
            .iter()
            .find(|t| t.name == "A1")
            .unwrap()
            .id
            .clone();

        let v_clip = Clip::new("m_vid", 0.0, 5.0);
        let a_clip = Clip::new("m_vid", 0.0, 5.0);

        let cmd_v = Box::new(AddClipCommand::new(&v1_id, v_clip));
        let cmd_a = Box::new(AddClipCommand::new(&a1_id, a_clip));

        let macro_cmd = Box::new(MacroCommand::new("Add Linked Clip", vec![cmd_v, cmd_a]));

        let mut stack = UndoStack::new();
        stack.push(macro_cmd, &mut tl).unwrap();

        // Both clips should be in timeline
        assert_eq!(tl.get_track_by_id(&v1_id).unwrap().clips.len(), 1);
        assert_eq!(tl.get_track_by_id(&a1_id).unwrap().clips.len(), 1);

        // Single undo must remove BOTH clips atomically
        assert!(stack.undo(&mut tl).unwrap());
        assert_eq!(tl.get_track_by_id(&v1_id).unwrap().clips.len(), 0);
        assert_eq!(tl.get_track_by_id(&a1_id).unwrap().clips.len(), 0);

        // Single redo must restore BOTH clips atomically
        assert!(stack.redo(&mut tl).unwrap());
        assert_eq!(tl.get_track_by_id(&v1_id).unwrap().clips.len(), 1);
        assert_eq!(tl.get_track_by_id(&a1_id).unwrap().clips.len(), 1);
    }

    #[test]
    fn test_inspector_undo_commands() {
        use trim_core::clip::{AudioEqPreset, ColorWheelsConfig};

        let mut tl = TimelineModel::default();
        let v1_id = tl.tracks[0].id.clone();
        let clip = Clip::new("m1", 0.0, 5.0);
        let clip_id = clip.id.clone();
        tl.tracks[0].add_clip(clip);

        let mut stack = UndoStack::new();

        // 1. Transform Command
        let old_tf = (0.0, 0.0, 1.0, 0.0, 1.0);
        let new_tf = (50.0, -20.0, 1.5, 45.0, 0.8);
        let tf_cmd = Box::new(SetClipTransformCommand::new(
            &v1_id, &clip_id, old_tf, new_tf,
        ));
        stack.push(tf_cmd, &mut tl).unwrap();

        let c = tl.tracks[0].get_clip_by_id(&clip_id).unwrap();
        assert_eq!(c.pos_x, 50.0);
        assert_eq!(c.scale, 1.5);
        assert_eq!(c.opacity, 0.8);

        stack.undo(&mut tl).unwrap();
        let c = tl.tracks[0].get_clip_by_id(&clip_id).unwrap();
        assert_eq!(c.pos_x, 0.0);
        assert_eq!(c.scale, 1.0);

        stack.redo(&mut tl).unwrap();
        let c = tl.tracks[0].get_clip_by_id(&clip_id).unwrap();
        assert_eq!(c.pos_x, 50.0);

        // 2. Color Wheels Command
        let wheels = ColorWheelsConfig {
            lift: [0.1, 0.2, 0.3],
            ..Default::default()
        };
        let cw_cmd = Box::new(SetClipColorWheelsCommand::new(
            &v1_id,
            &clip_id,
            None,
            Some(wheels.clone()),
        ));
        stack.push(cw_cmd, &mut tl).unwrap();
        assert_eq!(
            tl.tracks[0].get_clip_by_id(&clip_id).unwrap().color_wheels,
            Some(wheels)
        );

        stack.undo(&mut tl).unwrap();
        assert_eq!(
            tl.tracks[0].get_clip_by_id(&clip_id).unwrap().color_wheels,
            None
        );

        // 3. EQ Preset Command
        let eq_cmd = Box::new(SetClipEqPresetCommand::new(
            &v1_id,
            &clip_id,
            AudioEqPreset::Flat,
            AudioEqPreset::VoiceClarity,
        ));
        stack.push(eq_cmd, &mut tl).unwrap();
        assert_eq!(
            tl.tracks[0].get_clip_by_id(&clip_id).unwrap().eq_preset,
            AudioEqPreset::VoiceClarity
        );

        stack.undo(&mut tl).unwrap();
        assert_eq!(
            tl.tracks[0].get_clip_by_id(&clip_id).unwrap().eq_preset,
            AudioEqPreset::Flat
        );

        // 4. Crop Command
        let old_crop = (0.0, 0.0, 0.0, 0.0);
        let new_crop = (0.1, 0.15, 0.05, 0.2);
        let crop_cmd = Box::new(SetClipCropCommand::new(
            &v1_id, &clip_id, old_crop, new_crop,
        ));
        stack.push(crop_cmd, &mut tl).unwrap();
        let c = tl.tracks[0].get_clip_by_id(&clip_id).unwrap();
        assert_eq!(c.crop_left, 0.1);
        assert_eq!(c.crop_right, 0.15);
        assert_eq!(c.crop_top, 0.05);
        assert_eq!(c.crop_bottom, 0.2);
        assert!(c.has_crop());

        stack.undo(&mut tl).unwrap();
        let c = tl.tracks[0].get_clip_by_id(&clip_id).unwrap();
        assert_eq!(c.crop_left, 0.0);
        assert_eq!(c.crop_right, 0.0);
        assert!(!c.has_crop());

        stack.redo(&mut tl).unwrap();
        let c = tl.tracks[0].get_clip_by_id(&clip_id).unwrap();
        assert_eq!(c.crop_left, 0.1);
        assert!(c.has_crop());

        // 5. Ripple Trim Edge Command
        let c2 = Clip::new("media_2", 5.0, 10.0);
        let c2_id = c2.id.clone();
        tl.tracks[0].add_clip(c2);

        let old_bounds = (0.0, 5.0, 0.0, 5.0);
        let new_bounds = (0.0, 3.0, 0.0, 3.0);
        let shifted = vec![(v1_id.clone(), c2_id.clone(), 5.0, 3.0)];
        let ripple_cmd = Box::new(RippleTrimEdgeCommand::new(
            &v1_id, &clip_id, old_bounds, new_bounds, shifted,
        ));
        stack.push(ripple_cmd, &mut tl).unwrap();

        let c1 = tl.tracks[0].get_clip_by_id(&clip_id).unwrap();
        assert_eq!(c1.timeline_out, 3.0);
        let c2 = tl.tracks[0].get_clip_by_id(&c2_id).unwrap();
        assert_eq!(c2.timeline_in, 3.0);

        stack.undo(&mut tl).unwrap();
        let c1 = tl.tracks[0].get_clip_by_id(&clip_id).unwrap();
        assert_eq!(c1.timeline_out, 5.0);
        let c2 = tl.tracks[0].get_clip_by_id(&c2_id).unwrap();
        assert_eq!(c2.timeline_in, 5.0);

        stack.redo(&mut tl).unwrap();
        let c1 = tl.tracks[0].get_clip_by_id(&clip_id).unwrap();
        assert_eq!(c1.timeline_out, 3.0);
        let c2 = tl.tracks[0].get_clip_by_id(&c2_id).unwrap();
        assert_eq!(c2.timeline_in, 3.0);

        // 6. Set Clip Keyframes Command
        use trim_core::keyframe::{Interpolation, Keyframe};
        let kfs = vec![
            Keyframe::new(0.0, 1.0, Interpolation::Linear),
            Keyframe::new(2.5, 0.3, Interpolation::Linear),
            Keyframe::new(5.0, 1.0, Interpolation::Linear),
        ];
        let kf_cmd = Box::new(SetClipKeyframesCommand::new(
            &v1_id,
            &clip_id,
            "volume",
            Vec::new(),
            kfs.clone(),
        ));
        stack.push(kf_cmd, &mut tl).unwrap();

        let c1 = tl.tracks[0].get_clip_by_id(&clip_id).unwrap();
        assert_eq!(c1.keyframes.get("volume"), Some(&kfs));
        assert!((c1.get_property_at_time("volume", 2.5) - 0.3).abs() < 1e-4);

        stack.undo(&mut tl).unwrap();
        let c1 = tl.tracks[0].get_clip_by_id(&clip_id).unwrap();
        assert!(!c1.keyframes.contains_key("volume"));

        stack.redo(&mut tl).unwrap();
        let c1 = tl.tracks[0].get_clip_by_id(&clip_id).unwrap();
        assert_eq!(c1.keyframes.get("volume"), Some(&kfs));
    }
}
