//! Track model and TrackType enum.

use crate::clip::Clip;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TrackType {
    #[default]
    #[serde(alias = "Video")]
    Video,
    #[serde(alias = "Audio")]
    Audio,
}

impl std::fmt::Display for TrackType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Video => write!(f, "video"),
            Self::Audio => write!(f, "audio"),
        }
    }
}

/// Submix audio bus routing target for a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioBus {
    #[default]
    Master,
    Dialogue,
    Music,
    Sfx,
}

impl AudioBus {
    pub const ALL: &'static [Self] = &[Self::Master, Self::Dialogue, Self::Music, Self::Sfx];

    pub fn label(self) -> &'static str {
        match self {
            Self::Master => "Master",
            Self::Dialogue => "Diyalog",
            Self::Music => "Müzik",
            Self::Sfx => "SFX",
        }
    }
}

/// Per-bus submix linear gains (`1.0` = 0 dB unity).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioBusGains {
    pub dialogue: f64,
    pub music: f64,
    pub sfx: f64,
}

impl Default for AudioBusGains {
    fn default() -> Self {
        Self {
            dialogue: 1.0,
            music: 1.0,
            sfx: 1.0,
        }
    }
}

impl AudioBusGains {
    pub fn gain_for(self, bus: AudioBus) -> f64 {
        match bus {
            AudioBus::Master => 1.0,
            AudioBus::Dialogue => self.dialogue,
            AudioBus::Music => self.music,
            AudioBus::Sfx => self.sfx,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub name: String,
    #[serde(default, alias = "kind")]
    pub track_type: TrackType,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub solo: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_volume")]
    pub volume: f64,
    #[serde(default)]
    pub pan: f64,
    #[serde(default)]
    pub bus: AudioBus,
    #[serde(default)]
    pub clips: Vec<Clip>,
}

fn default_true() -> bool {
    true
}
fn default_volume() -> f64 {
    1.0
}

impl Default for Track {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: "Track".to_string(),
            track_type: TrackType::Video,
            muted: false,
            solo: false,
            locked: false,
            visible: true,
            volume: 1.0,
            pan: 0.0,
            bus: AudioBus::Master,
            clips: Vec::new(),
        }
    }
}

impl Track {
    pub fn new(name: impl Into<String>, track_type: TrackType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            track_type,
            ..Default::default()
        }
    }

    pub fn duration(&self) -> f64 {
        self.clips
            .iter()
            .map(|c| c.timeline_out)
            .fold(0.0, f64::max)
    }

    pub fn add_clip(&mut self, clip: Clip) {
        self.clips.push(clip);
        self.sort_clips();
    }

    pub fn remove_clip(&mut self, clip_id: &str) -> Option<Clip> {
        if let Some(pos) = self.clips.iter().position(|c| c.id == clip_id) {
            Some(self.clips.remove(pos))
        } else {
            None
        }
    }

    pub fn get_clip_by_id(&self, clip_id: &str) -> Option<&Clip> {
        self.clips.iter().find(|c| c.id == clip_id)
    }

    pub fn get_clip_by_id_mut(&mut self, clip_id: &str) -> Option<&mut Clip> {
        self.clips.iter_mut().find(|c| c.id == clip_id)
    }

    pub fn find_clip_at(&self, time: f64) -> Option<&Clip> {
        self.clips.iter().find(|c| c.contains_timeline_time(time))
    }

    pub fn find_clip_at_mut(&mut self, time: f64) -> Option<&mut Clip> {
        self.clips
            .iter_mut()
            .find(|c| c.contains_timeline_time(time))
    }

    pub fn sort_clips(&mut self) {
        self.clips.sort_by(|a, b| {
            a.timeline_in
                .partial_cmp(&b.timeline_in)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_operations() {
        let mut track = Track::new("V1", TrackType::Video);
        let c1 = Clip::new("m1", 2.0, 5.0);
        let c2 = Clip::new("m2", 0.0, 2.0);
        track.add_clip(c1);
        track.add_clip(c2);

        // Should be sorted by timeline_in
        assert_eq!(track.clips[0].timeline_in, 0.0);
        assert_eq!(track.clips[1].timeline_in, 2.0);
        assert_eq!(track.duration(), 5.0);

        assert!(track.find_clip_at(1.0).is_some());
        assert!(track.find_clip_at(6.0).is_none());
    }
}
