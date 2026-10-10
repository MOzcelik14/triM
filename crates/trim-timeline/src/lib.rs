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

    let prev = points.into_iter().filter(|&p| p < current_time - 0.04).last();
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
            if let Some(clip) = track.get_clip_by_id(cl_id) {
                if clip.timeline_in + 0.04 < playhead_time && playhead_time < clip.timeline_out - 0.04 {
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
    };

    let target = target.or_else(|| {
        for track in &timeline.tracks {
            if let Some(clip) = track.find_clip_at(playhead_time) {
                if clip.timeline_in + 0.04 < playhead_time && playhead_time < clip.timeline_out - 0.04 {
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
            if let Some(clip) = track.get_clip_by_id(cl_id) {
                if clip.timeline_in + 0.04 < playhead_time && playhead_time <= clip.timeline_out {
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
    };

    let target = target.or_else(|| {
        for track in &timeline.tracks {
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
            if let Some(clip) = track.get_clip_by_id(cl_id) {
                if clip.timeline_in <= playhead_time && playhead_time < clip.timeline_out - 0.04 {
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
    };

    let target = target.or_else(|| {
        for track in &timeline.tracks {
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
}
