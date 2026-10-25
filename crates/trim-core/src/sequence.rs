//! Multi-sequence and nested sequence models for triM.

use crate::timeline::TimelineModel;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sequence {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub timeline: TimelineModel,
}

impl Sequence {
    pub fn new(name: impl Into<String>, timeline: TimelineModel) -> Self {
        Self {
            id: format!("seq_{}", Uuid::new_v4()),
            name: name.into(),
            timeline,
        }
    }

    pub fn with_id(
        id: impl Into<String>,
        name: impl Into<String>,
        timeline: TimelineModel,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            timeline,
        }
    }
}

/// Flattens nested sequence clips recursively into a concrete TimelineModel for rendering or export.
pub fn flatten_timeline_sequences(
    timeline: &TimelineModel,
    sequences: &[Sequence],
    max_depth: usize,
) -> TimelineModel {
    if max_depth == 0 {
        return timeline.clone();
    }

    let mut flattened = timeline.clone();
    flattened.tracks.clear();

    for track in &timeline.tracks {
        let mut new_track = track.clone();
        new_track.clips.clear();

        for clip in &track.clips {
            if let Some(ref seq_id) = clip.nested_sequence_id {
                if let Some(seq) = sequences.iter().find(|s| &s.id == seq_id) {
                    let sub_flattened =
                        flatten_timeline_sequences(&seq.timeline, sequences, max_depth - 1);
                    for st in sub_flattened
                        .tracks
                        .iter()
                        .filter(|t| t.track_type == track.track_type)
                    {
                        let clip_src_in = clip.source_in;
                        let clip_src_out = clip.effective_source_out();
                        let spd = if clip.speed.abs() > 0.001 {
                            clip.speed
                        } else {
                            1.0
                        };

                        for sub_clip in &st.clips {
                            let s_in = sub_clip.timeline_in.max(clip_src_in);
                            let s_out = sub_clip.timeline_out.min(clip_src_out);
                            if s_out > s_in + 0.001 {
                                let offset_in = (s_in - clip_src_in) / spd;
                                let offset_dur = (s_out - s_in) / spd;

                                let mut mapped_clip = sub_clip.clone();
                                mapped_clip.timeline_in = clip.timeline_in + offset_in;
                                mapped_clip.timeline_out = mapped_clip.timeline_in + offset_dur;
                                mapped_clip.source_in =
                                    sub_clip.source_in + (s_in - sub_clip.timeline_in);
                                mapped_clip.source_out =
                                    Some(mapped_clip.source_in + (s_out - s_in));
                                mapped_clip.speed = sub_clip.speed * spd;
                                // Combine opacity and volume
                                mapped_clip.opacity *= clip.opacity;
                                mapped_clip.volume *= clip.volume;

                                new_track.clips.push(mapped_clip);
                            }
                        }
                    }
                    continue;
                }
            }

            new_track.clips.push(clip.clone());
        }

        new_track.clips.sort_by(|a, b| {
            a.timeline_in
                .partial_cmp(&b.timeline_in)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        flattened.tracks.push(new_track);
    }

    flattened
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::Clip;
    use crate::track::{Track, TrackType};

    #[test]
    fn test_sequence_creation_and_flattening() {
        // Child sequence: 1 track with 1 clip spanning 0.0 .. 10.0
        let mut child_tl = TimelineModel::default();
        child_tl.tracks.clear();
        let mut child_tr = Track::new("Video 1", TrackType::Video);
        let c1 = Clip::new("media_vid", 0.0, 10.0);
        child_tr.add_clip(c1);
        child_tl.tracks.push(child_tr);

        let seq_child = Sequence::with_id("seq_child_1", "Sahne 01", child_tl);

        // Parent sequence: 1 track with a nested clip referencing seq_child_1 spanning 5.0 .. 15.0
        let mut parent_tl = TimelineModel::default();
        parent_tl.tracks.clear();
        let mut parent_tr = Track::new("Video 1", TrackType::Video);
        let mut nested_clip = Clip::new("placeholder", 5.0, 15.0);
        nested_clip.nested_sequence_id = Some("seq_child_1".to_string());
        parent_tr.add_clip(nested_clip);
        parent_tl.tracks.push(parent_tr);

        let sequences = vec![seq_child];
        let flattened = flatten_timeline_sequences(&parent_tl, &sequences, 3);

        assert_eq!(flattened.tracks.len(), 1);
        assert_eq!(flattened.tracks[0].clips.len(), 1);
        let f_clip = &flattened.tracks[0].clips[0];
        assert_eq!(f_clip.media_id, "media_vid");
        assert_eq!(f_clip.timeline_in, 5.0);
        assert_eq!(f_clip.timeline_out, 15.0);
    }
}
