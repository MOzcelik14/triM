//! TimelineModel managing tracks, markers, and editing queries.

use crate::clip::Clip;
use crate::marker::Marker;
use crate::track::{Track, TrackType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineModel {
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub markers: Vec<Marker>,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    #[serde(default = "default_fps")]
    pub fps: f64,
    #[serde(default = "default_true")]
    pub snap_enabled: bool,
    #[serde(default = "default_snap_threshold")]
    pub snap_threshold: f64,
}

fn default_width() -> u32 {
    1920
}
fn default_height() -> u32 {
    1080
}
fn default_fps() -> f64 {
    30.0
}
fn default_true() -> bool {
    true
}
fn default_snap_threshold() -> f64 {
    0.15
}

impl Default for TimelineModel {
    fn default() -> Self {
        Self {
            tracks: vec![
                Track::new("V1", TrackType::Video),
                Track::new("A1", TrackType::Audio),
            ],
            markers: Vec::new(),
            width: 1920,
            height: 1080,
            fps: 30.0,
            snap_enabled: true,
            snap_threshold: 0.15,
        }
    }
}

impl TimelineModel {
    pub fn duration(&self) -> f64 {
        self.tracks
            .iter()
            .map(|t| t.duration())
            .fold(0.0, f64::max)
    }

    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn remove_track(&mut self, track_id: &str) -> Option<Track> {
        if let Some(pos) = self.tracks.iter().position(|t| t.id == track_id) {
            Some(self.tracks.remove(pos))
        } else {
            None
        }
    }

    pub fn get_track_by_id(&self, track_id: &str) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == track_id)
    }

    pub fn get_track_by_id_mut(&mut self, track_id: &str) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| t.id == track_id)
    }

    pub fn get_video_tracks(&self) -> Vec<&Track> {
        self.tracks.iter().filter(|t| t.track_type == TrackType::Video).collect()
    }

    pub fn get_video_tracks_mut(&mut self) -> Vec<&mut Track> {
        self.tracks.iter_mut().filter(|t| t.track_type == TrackType::Video).collect()
    }

    pub fn get_audio_tracks(&self) -> Vec<&Track> {
        self.tracks.iter().filter(|t| t.track_type == TrackType::Audio).collect()
    }

    pub fn get_audio_tracks_mut(&mut self) -> Vec<&mut Track> {
        self.tracks.iter_mut().filter(|t| t.track_type == TrackType::Audio).collect()
    }

    pub fn find_track_for_clip(&self, clip_id: &str) -> Option<&Track> {
        self.tracks.iter().find(|t| t.get_clip_by_id(clip_id).is_some())
    }

    pub fn find_track_for_clip_mut(&mut self, clip_id: &str) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| t.get_clip_by_id(clip_id).is_some())
    }

    pub fn find_clip_by_id(&self, clip_id: &str) -> Option<(&Track, &Clip)> {
        for t in &self.tracks {
            if let Some(c) = t.get_clip_by_id(clip_id) {
                return Some((t, c));
            }
        }
        None
    }

    pub fn add_marker(&mut self, time: f64, name: impl Into<String>, color: impl Into<String>) -> Marker {
        let marker = Marker::with_details(time, name, color);
        self.markers.push(marker.clone());
        self.markers
            .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
        marker
    }

    pub fn remove_marker(&mut self, marker_id: &str) -> Option<Marker> {
        if let Some(pos) = self.markers.iter().position(|m| m.id == marker_id) {
            Some(self.markers.remove(pos))
        } else {
            None
        }
    }

    pub fn get_marker_at(&self, time: f64, threshold: f64) -> Option<&Marker> {
        self.markers
            .iter()
            .find(|m| (m.time - time).abs() <= threshold)
    }

    /// Snaps a timestamp magnetically to 0.0, clip boundaries, or markers.
    pub fn snap_time(&self, target_time: f64, threshold: Option<f64>, ignore_clip_id: Option<&str>) -> f64 {
        if !self.snap_enabled {
            return target_time;
        }

        let thresh = threshold.unwrap_or(self.snap_threshold);
        let mut snap_points = vec![0.0];

        // Add clip boundaries
        for track in &self.tracks {
            for clip in &track.clips {
                if let Some(ignore_id) = ignore_clip_id {
                    if clip.id == ignore_id {
                        continue;
                    }
                }
                snap_points.push(clip.timeline_in);
                snap_points.push(clip.timeline_out);
            }
        }

        // Add markers
        for marker in &self.markers {
            snap_points.push(marker.time);
        }

        let mut closest_point = target_time;
        let mut min_dist = thresh;

        for pt in snap_points {
            let dist = (pt - target_time).abs();
            if dist < min_dist {
                min_dist = dist;
                closest_point = pt;
            }
        }

        closest_point
    }

    /// Returns visible video clips at `time` in bottom-to-top order (V1, V2, ...).
    pub fn get_visible_video_clips_at(&self, time: f64) -> Vec<(&Track, &Clip)> {
        let mut result = Vec::new();
        for track in self.get_video_tracks() {
            if !track.visible {
                continue;
            }
            if let Some(clip) = track.find_clip_at(time) {
                result.push((track, clip));
            }
        }
        result
    }

    /// Returns the topmost visible video clip at `time`.
    pub fn get_top_visible_video_clip_at(&self, time: f64) -> Option<(&Track, &Clip)> {
        // Iterate backwards from highest layer to lowest
        for track in self.get_video_tracks().into_iter().rev() {
            if !track.visible {
                continue;
            }
            if let Some(clip) = track.find_clip_at(time) {
                return Some((track, clip));
            }
        }
        None
    }

    /// Returns active audio clips at `time`, prioritizing solo tracks.
    pub fn get_active_audio_clips_at(&self, time: f64) -> Vec<(&Track, &Clip)> {
        let has_solo = self.tracks.iter().any(|t| t.solo);
        let mut result = Vec::new();

        for track in &self.tracks {
            if track.muted {
                continue;
            }
            if has_solo && !track.solo {
                continue;
            }
            for clip in &track.clips {
                if clip.muted {
                    continue;
                }
                if clip.contains_timeline_time(time) {
                    result.push((track, clip));
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magnetic_snapping() {
        let mut tl = TimelineModel::default();
        tl.tracks[0].add_clip(Clip::new("m1", 2.0, 5.0));
        tl.add_marker(8.0, "Chorus", "#E07A38");

        // Snap to 0.0
        assert_eq!(tl.snap_time(0.08, None, None), 0.0);
        // Snap to clip in (2.0)
        assert_eq!(tl.snap_time(2.05, None, None), 2.0);
        // Snap to clip out (5.0)
        assert_eq!(tl.snap_time(4.95, None, None), 5.0);
        // Snap to marker (8.0)
        assert_eq!(tl.snap_time(8.07, None, None), 8.0);
        // Far away from snap points
        assert_eq!(tl.snap_time(3.5, None, None), 3.5);
    }

    #[test]
    fn test_solo_audio_selection() {
        let mut tl = TimelineModel::default();
        tl.tracks.clear();
        let mut t1 = Track::new("A1", TrackType::Audio);
        let mut t2 = Track::new("A2", TrackType::Audio);
        t2.solo = true;

        t1.add_clip(Clip::new("m1", 0.0, 10.0));
        t2.add_clip(Clip::new("m2", 0.0, 10.0));
        tl.add_track(t1);
        tl.add_track(t2);

        let active = tl.get_active_audio_clips_at(5.0);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].0.name, "A2");

        // Turn solo off
        tl.tracks[1].solo = false;
        let active2 = tl.get_active_audio_clips_at(5.0);
        assert_eq!(active2.len(), 2);
    }
}
