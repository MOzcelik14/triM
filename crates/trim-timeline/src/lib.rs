//! High-level timeline editing operations and edit-point navigation.

use trim_commands::commands::*;
use trim_commands::UndoStack;
use trim_core::timeline::TimelineModel;

/// Finds previous cut / edit point before `current_time`.
pub fn find_prev_edit_point(timeline: &TimelineModel, current_time: f64) -> f64 {
    let mut points = vec![0.0];
    for track in &timeline.tracks {
        for clip in &track.clips {
            points.push(clip.timeline_in);
            points.push(clip.timeline_out);
        }
    }
    for marker in &timeline.markers {
        points.push(marker.time);
    }

    points.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    points.dedup_by(|a, b| (*a - *b).abs() < 1e-4);

    let prev = points.into_iter().rfind(|&p| p < current_time - 0.04);
    prev.unwrap_or(0.0)
}

/// Finds next cut / edit point after `current_time`.
pub fn find_next_edit_point(timeline: &TimelineModel, current_time: f64) -> f64 {
    let mut points = vec![timeline.duration()];
    for track in &timeline.tracks {
        for clip in &track.clips {
            points.push(clip.timeline_in);
            points.push(clip.timeline_out);
        }
    }
    for marker in &timeline.markers {
        points.push(marker.time);
    }

    points.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    points.dedup_by(|a, b| (*a - *b).abs() < 1e-4);

    let next = points.into_iter().find(|&p| p > current_time + 0.04);
    next.unwrap_or_else(|| timeline.duration())
}

/// Splits the active clip or top clip under playhead.
pub fn split_at_playhead(
    timeline: &mut TimelineModel,
    undo_stack: &mut UndoStack,
    preferred_track_id: Option<&str>,
    preferred_clip_id: Option<&str>,
    playhead_time: f64,
) -> bool {
    let target = if let (Some(tr_id), Some(cl_id)) = (preferred_track_id, preferred_clip_id) {
        if let Some(track) = timeline.get_track_by_id(tr_id) {
            if !track.locked {
                if let Some(clip) = track.get_clip_by_id(cl_id) {
                    if clip.timeline_in + 0.04 < playhead_time
                        && playhead_time < clip.timeline_out - 0.04
                    {
                        Some((tr_id.to_string(), cl_id.to_string()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let target = target.or_else(|| {
        for track in &timeline.tracks {
            if track.locked {
                continue;
            }
            if let Some(clip) = track.find_clip_at(playhead_time) {
                if clip.timeline_in + 0.04 < playhead_time
                    && playhead_time < clip.timeline_out - 0.04
                {
                    return Some((track.id.clone(), clip.id.clone()));
                }
            }
        }
        None
    });

    if let Some((tr_id, cl_id)) = target {
        let cmd = Box::new(SplitClipCommand::new(tr_id, cl_id, playhead_time));
        undo_stack.push(cmd, timeline).is_ok()
    } else {
        false
    }
}

/// Ripple trims head of the active clip to playhead.
pub fn ripple_trim_head(
    timeline: &mut TimelineModel,
    undo_stack: &mut UndoStack,
    preferred_track_id: Option<&str>,
    preferred_clip_id: Option<&str>,
    playhead_time: f64,
) -> bool {
    let target = if let (Some(tr_id), Some(cl_id)) = (preferred_track_id, preferred_clip_id) {
        if let Some(track) = timeline.get_track_by_id(tr_id) {
            if !track.locked {
                if let Some(clip) = track.get_clip_by_id(cl_id) {
                    if clip.timeline_in + 0.04 < playhead_time && playhead_time <= clip.timeline_out
                    {
                        Some((tr_id.to_string(), cl_id.to_string()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let target = target.or_else(|| {
        for track in &timeline.tracks {
            if track.locked {
                continue;
            }
            if let Some(clip) = track.find_clip_at(playhead_time) {
                if clip.timeline_in + 0.04 < playhead_time && playhead_time <= clip.timeline_out {
                    return Some((track.id.clone(), clip.id.clone()));
                }
            }
        }
        None
    });

    if let Some((tr_id, cl_id)) = target {
        let cmd = Box::new(RippleTrimHeadCommand::new(tr_id, cl_id, playhead_time));
        undo_stack.push(cmd, timeline).is_ok()
    } else {
        false
    }
}

/// Ripple trims tail of the active clip to playhead.
pub fn ripple_trim_tail(
    timeline: &mut TimelineModel,
    undo_stack: &mut UndoStack,
    preferred_track_id: Option<&str>,
    preferred_clip_id: Option<&str>,
    playhead_time: f64,
) -> bool {
    let target = if let (Some(tr_id), Some(cl_id)) = (preferred_track_id, preferred_clip_id) {
        if let Some(track) = timeline.get_track_by_id(tr_id) {
            if !track.locked {
                if let Some(clip) = track.get_clip_by_id(cl_id) {
                    if clip.timeline_in <= playhead_time && playhead_time < clip.timeline_out - 0.04
                    {
                        Some((tr_id.to_string(), cl_id.to_string()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let target = target.or_else(|| {
        for track in &timeline.tracks {
            if track.locked {
                continue;
            }
            if let Some(clip) = track.find_clip_at(playhead_time) {
                if clip.timeline_in <= playhead_time && playhead_time < clip.timeline_out - 0.04 {
                    return Some((track.id.clone(), clip.id.clone()));
                }
            }
        }
        None
    });

    if let Some((tr_id, cl_id)) = target {
        let cmd = Box::new(RippleTrimTailCommand::new(tr_id, cl_id, playhead_time));
        undo_stack.push(cmd, timeline).is_ok()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trim_core::clip::Clip;

    #[test]
    fn test_find_edit_points() {
        let mut tl = TimelineModel::default();
        tl.tracks[0].add_clip(Clip::new("m1", 2.0, 5.0));
        tl.tracks[0].add_clip(Clip::new("m2", 8.0, 12.0));

        assert_eq!(find_prev_edit_point(&tl, 3.0), 2.0);
        assert_eq!(find_prev_edit_point(&tl, 6.0), 5.0);
        assert_eq!(find_next_edit_point(&tl, 3.0), 5.0);
        assert_eq!(find_next_edit_point(&tl, 5.0), 8.0);
    }

    #[test]
    fn test_split_respects_locked_track() {
        let mut tl = TimelineModel::default();
        let mut stack = UndoStack::new();
        tl.tracks[0].add_clip(Clip::new("m1", 0.0, 10.0));
        tl.tracks[0].locked = true;

        let res = split_at_playhead(&mut tl, &mut stack, None, None, 5.0);
        assert!(!res, "Must not split clips on a locked track");
        assert_eq!(tl.tracks[0].clips.len(), 1);

        tl.tracks[0].locked = false;
        let res2 = split_at_playhead(&mut tl, &mut stack, None, None, 5.0);
        assert!(res2);
        assert_eq!(tl.tracks[0].clips.len(), 2);
    }

    #[test]
    fn test_ripple_trim_head_and_tail() {
        let mut tl = TimelineModel::default();
        let mut stack = UndoStack::new();
        tl.tracks[0].add_clip(Clip::new("m1", 0.0, 10.0));
        tl.tracks[0].add_clip(Clip::new("m2", 10.0, 15.0));

        // Ripple trim tail of m1 to 7.0 -> m2 should shift left by 3.0s to [7.0, 12.0]
        assert!(ripple_trim_tail(&mut tl, &mut stack, None, None, 7.0));
        assert!((tl.tracks[0].clips[0].timeline_out - 7.0).abs() < 1e-6);
        assert!((tl.tracks[0].clips[1].timeline_in - 7.0).abs() < 1e-6);

        // Ripple trim head of m1 at 2.0 -> m1 becomes [0.0, 5.0], m2 shifts to [5.0, 10.0]
        assert!(ripple_trim_head(&mut tl, &mut stack, None, None, 2.0));
        assert!((tl.tracks[0].clips[0].duration() - 5.0).abs() < 1e-6);
        assert!((tl.tracks[0].clips[1].timeline_in - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_find_edit_points_with_markers_and_empty_timeline() {
        let mut tl = TimelineModel::default();
        assert_eq!(find_prev_edit_point(&tl, 5.0), 0.0);
        assert_eq!(find_next_edit_point(&tl, 5.0), 0.0);

        tl.tracks[0].add_clip(Clip::new("m1", 2.0, 6.0));
        tl.markers.push(trim_core::marker::Marker::new(4.0));
        assert_eq!(find_prev_edit_point(&tl, 5.0), 4.0);
        assert_eq!(find_next_edit_point(&tl, 2.5), 4.0);
    }

    #[test]
    fn test_split_at_playhead_preferred_track_and_boundary_004s() {
        let mut tl = TimelineModel::default();
        let mut stack = UndoStack::new();
        let c1 = Clip::new("m1", 0.0, 10.0);
        let c1_id = c1.id.clone();
        let t0_id = tl.tracks[0].id.clone();
        tl.tracks[0].add_clip(c1);

        // Within 0.04s of clip edge -> must not split
        assert!(!split_at_playhead(
            &mut tl,
            &mut stack,
            Some(&t0_id),
            Some(&c1_id),
            0.02
        ));
        assert!(!split_at_playhead(
            &mut tl,
            &mut stack,
            Some(&t0_id),
            Some(&c1_id),
            9.98
        ));

        // Preferred track/clip in middle -> splits cleanly
        assert!(split_at_playhead(
            &mut tl,
            &mut stack,
            Some(&t0_id),
            Some(&c1_id),
            5.0
        ));
        assert_eq!(tl.tracks[0].clips.len(), 2);

        // Locked preferred track -> must not split even if IDs provided
        tl.tracks[0].locked = true;
        assert!(!split_at_playhead(
            &mut tl,
            &mut stack,
            Some(&t0_id),
            None,
            2.5
        ));
    }

    #[test]
    fn test_ripple_trim_preferred_clip_and_004s_guards() {
        let mut tl = TimelineModel::default();
        let mut stack = UndoStack::new();
        let c1 = Clip::new("m1", 0.0, 10.0);
        let c1_id = c1.id.clone();
        let t0_id = tl.tracks[0].id.clone();
        tl.tracks[0].add_clip(c1);

        assert!(!ripple_trim_head(
            &mut tl,
            &mut stack,
            Some(&t0_id),
            Some(&c1_id),
            0.02
        ));
        assert!(!ripple_trim_tail(
            &mut tl,
            &mut stack,
            Some(&t0_id),
            Some(&c1_id),
            9.98
        ));
        assert!(ripple_trim_tail(
            &mut tl,
            &mut stack,
            Some(&t0_id),
            Some(&c1_id),
            8.0
        ));
        assert!((tl.tracks[0].clips[0].timeline_out - 8.0).abs() < 1e-6);
    }
}
