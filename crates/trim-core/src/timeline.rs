//! TimelineModel managing tracks, markers, and editing queries.

use crate::clip::Clip;
use crate::marker::Marker;
use crate::track::{Track, TrackType};
use serde::{Deserialize, Serialize};

/// Common video aspect ratios for YouTube, Reels/TikTok, Cinema and social media.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AspectRatio {
    #[default]
    Wide16x9,
    Vertical9x16,
    Square1x1,
    Portrait4x5,
    Classic4x3,
    Cinema21x9,
    Custom,
}

impl AspectRatio {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Wide16x9 => "16:9 Yatay (YouTube / TV)",
            Self::Vertical9x16 => "9:16 Dikey (Shorts / Reels / TikTok)",
            Self::Square1x1 => "1:1 Kare (Instagram)",
            Self::Portrait4x5 => "4:5 Portre (Sosyal Medya)",
            Self::Classic4x3 => "4:3 Klasik",
            Self::Cinema21x9 => "21:9 Sinematik",
            Self::Custom => "Özel Oran",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Wide16x9 => "16:9",
            Self::Vertical9x16 => "9:16",
            Self::Square1x1 => "1:1",
            Self::Portrait4x5 => "4:5",
            Self::Classic4x3 => "4:3",
            Self::Cinema21x9 => "21:9",
            Self::Custom => "Özel",
        }
    }

    pub fn default_resolution(&self) -> (u32, u32) {
        match self {
            Self::Wide16x9 => (1920, 1080),
            Self::Vertical9x16 => (1080, 1920),
            Self::Square1x1 => (1080, 1080),
            Self::Portrait4x5 => (1080, 1350),
            Self::Classic4x3 => (1440, 1080),
            Self::Cinema21x9 => (2560, 1080),
            Self::Custom => (1920, 1080),
        }
    }

    pub fn from_dimensions(width: u32, height: u32) -> Self {
        if width == 0 || height == 0 {
            return Self::Wide16x9;
        }
        let ratio = width as f64 / height as f64;
        if (ratio - 16.0 / 9.0).abs() < 0.05 {
            Self::Wide16x9
        } else if (ratio - 9.0 / 16.0).abs() < 0.05 {
            Self::Vertical9x16
        } else if (ratio - 1.0).abs() < 0.05 {
            Self::Square1x1
        } else if (ratio - 4.0 / 5.0).abs() < 0.05 {
            Self::Portrait4x5
        } else if (ratio - 4.0 / 3.0).abs() < 0.05 {
            Self::Classic4x3
        } else if (ratio - 21.0 / 9.0).abs() < 0.08 {
            Self::Cinema21x9
        } else {
            Self::Custom
        }
    }
}

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
    pub fn aspect_ratio(&self) -> AspectRatio {
        AspectRatio::from_dimensions(self.width, self.height)
    }

    pub fn set_aspect_ratio(&mut self, ratio: AspectRatio) {
        if ratio != AspectRatio::Custom {
            let (w, h) = ratio.default_resolution();
            self.width = w;
            self.height = h;
        }
    }

    pub fn duration(&self) -> f64 {
        self.tracks.iter().map(|t| t.duration()).fold(0.0, f64::max)
    }

    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn add_new_video_track(&mut self) -> String {
        let max_idx = self
            .get_video_tracks()
            .iter()
            .filter_map(|t| {
                t.name
                    .strip_prefix('V')
                    .and_then(|s| s.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(0);
        let track = Track::new(format!("V{}", max_idx + 1), TrackType::Video);
        let id = track.id.clone();
        // Video tracks stack upwards in timeline display: index 0 is topmost
        self.tracks.insert(0, track);
        id
    }

    pub fn add_new_audio_track(&mut self) -> String {
        let max_idx = self
            .get_audio_tracks()
            .iter()
            .filter_map(|t| {
                t.name
                    .strip_prefix('A')
                    .and_then(|s| s.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(0);
        let track = Track::new(format!("A{}", max_idx + 1), TrackType::Audio);
        let id = track.id.clone();
        // Audio tracks expand downwards
        self.tracks.push(track);
        id
    }

    pub fn remove_track_safe(&mut self, track_id: &str) -> Result<Track, &'static str> {
        let track_type = self
            .get_track_by_id(track_id)
            .map(|t| t.track_type)
            .ok_or("Kanal bulunamadı")?;
        let count = self
            .tracks
            .iter()
            .filter(|t| t.track_type == track_type)
            .count();
        if count <= 1 {
            return Err("Son kalan kanal silinemez");
        }
        self.remove_track(track_id).ok_or("Kanal bulunamadı")
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
        self.tracks
            .iter()
            .filter(|t| t.track_type == TrackType::Video)
            .collect()
    }

    pub fn get_video_tracks_mut(&mut self) -> Vec<&mut Track> {
        self.tracks
            .iter_mut()
            .filter(|t| t.track_type == TrackType::Video)
            .collect()
    }

    pub fn get_audio_tracks(&self) -> Vec<&Track> {
        self.tracks
            .iter()
            .filter(|t| t.track_type == TrackType::Audio)
            .collect()
    }

    pub fn get_audio_tracks_mut(&mut self) -> Vec<&mut Track> {
        self.tracks
            .iter_mut()
            .filter(|t| t.track_type == TrackType::Audio)
            .collect()
    }

    pub fn find_track_for_clip(&self, clip_id: &str) -> Option<&Track> {
        self.tracks
            .iter()
            .find(|t| t.get_clip_by_id(clip_id).is_some())
    }

    pub fn find_track_for_clip_mut(&mut self, clip_id: &str) -> Option<&mut Track> {
        self.tracks
            .iter_mut()
            .find(|t| t.get_clip_by_id(clip_id).is_some())
    }

    pub fn find_clip_by_id(&self, clip_id: &str) -> Option<(&Track, &Clip)> {
        for t in &self.tracks {
            if let Some(c) = t.get_clip_by_id(clip_id) {
                return Some((t, c));
            }
        }
        None
    }

    pub fn add_marker(
        &mut self,
        time: f64,
        name: impl Into<String>,
        color: impl Into<String>,
    ) -> Marker {
        let marker = Marker::with_details(time, name, color);
        self.markers.push(marker.clone());
        self.markers.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
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
    pub fn snap_time(
        &self,
        target_time: f64,
        threshold: Option<f64>,
        ignore_clip_id: Option<&str>,
    ) -> f64 {
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

    #[test]
    fn test_dynamic_track_addition_and_removal() {
        let mut tl = TimelineModel::default();
        // Starts with V1 and A1
        assert_eq!(tl.tracks.len(), 2);
        assert_eq!(tl.tracks[0].name, "V1");
        assert_eq!(tl.tracks[1].name, "A1");

        // Add V2 -> should become index 0
        let v2_id = tl.add_new_video_track();
        assert_eq!(tl.tracks[0].name, "V2");
        assert_eq!(tl.tracks[0].id, v2_id);
        assert_eq!(tl.tracks[1].name, "V1");
        assert_eq!(tl.tracks[2].name, "A1");

        // Add A2 -> should become index 3
        let a2_id = tl.add_new_audio_track();
        assert_eq!(tl.tracks.len(), 4);
        assert_eq!(tl.tracks[3].name, "A2");
        assert_eq!(tl.tracks[3].id, a2_id);

        // Safe removal of V2
        assert!(tl.remove_track_safe(&v2_id).is_ok());
        assert_eq!(tl.get_video_tracks().len(), 1);

        // Trying to remove the last video track (V1) should fail safely
        let v1_id = tl.tracks[0].id.clone();
        assert!(tl.remove_track_safe(&v1_id).is_err());
        assert_eq!(tl.get_video_tracks().len(), 1);
    }

    #[test]
    fn test_aspect_ratio_detection_and_switch() {
        let mut tl = TimelineModel::default();
        assert_eq!(tl.aspect_ratio(), AspectRatio::Wide16x9);

        tl.set_aspect_ratio(AspectRatio::Vertical9x16);
        assert_eq!(tl.width, 1080);
        assert_eq!(tl.height, 1920);
        assert_eq!(tl.aspect_ratio(), AspectRatio::Vertical9x16);

        tl.set_aspect_ratio(AspectRatio::Square1x1);
        assert_eq!(tl.width, 1080);
        assert_eq!(tl.height, 1080);
        assert_eq!(tl.aspect_ratio(), AspectRatio::Square1x1);
    }
}
