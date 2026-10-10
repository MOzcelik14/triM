//! Clip model for timeline items.

use crate::keyframe::{interpolate_keyframes, Interpolation, Keyframe};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub id: String,
    pub media_id: String,
    #[serde(default)]
    pub name: String,
    pub timeline_in: f64,
    pub timeline_out: f64,
    #[serde(default)]
    pub source_in: f64,
    #[serde(default)]
    pub source_out: Option<f64>,
    #[serde(default = "default_speed")]
    pub speed: f64,
    #[serde(default)]
    pub reverse: bool,
    #[serde(default)]
    pub fade_in: f64,
    #[serde(default)]
    pub fade_out: f64,
    #[serde(default)]
    pub transition_in: Option<String>,
    #[serde(default)]
    pub transition_out: Option<String>,
    #[serde(default)]
    pub brightness: f64,
    #[serde(default = "default_one")]
    pub contrast: f64,
    #[serde(default = "default_one")]
    pub saturation: f64,
    #[serde(default = "default_one")]
    pub volume: f64,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub pos_x: f64,
    #[serde(default)]
    pub pos_y: f64,
    #[serde(default = "default_one")]
    pub scale: f64,
    #[serde(default)]
    pub rotation: f64,
    #[serde(default = "default_one")]
    pub opacity: f64,
    #[serde(default)]
    pub keyframes: HashMap<String, Vec<Keyframe>>,
}

fn default_speed() -> f64 {
    1.0
}
fn default_one() -> f64 {
    1.0
}

impl Default for Clip {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            media_id: String::new(),
            name: String::new(),
            timeline_in: 0.0,
            timeline_out: 5.0,
            source_in: 0.0,
            source_out: None,
            speed: 1.0,
            reverse: false,
            fade_in: 0.0,
            fade_out: 0.0,
            transition_in: None,
            transition_out: None,
            brightness: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            volume: 1.0,
            muted: false,
            pos_x: 0.0,
            pos_y: 0.0,
            scale: 1.0,
            rotation: 0.0,
            opacity: 1.0,
            keyframes: HashMap::new(),
        }
    }
}

impl Clip {
    pub fn new(media_id: impl Into<String>, timeline_in: f64, timeline_out: f64) -> Self {
        let dur = (timeline_out - timeline_in).max(0.04);
        Self {
            id: Uuid::new_v4().to_string(),
            media_id: media_id.into(),
            timeline_in,
            timeline_out: timeline_in + dur,
            source_in: 0.0,
            source_out: Some(dur),
            ..Default::default()
        }
    }

    pub fn duration(&self) -> f64 {
        (self.timeline_out - self.timeline_in).max(0.0)
    }

    pub fn effective_source_out(&self) -> f64 {
        match self.source_out {
            Some(so) => so,
            None => self.source_in + self.duration() * self.speed,
        }
    }

    pub fn map_timeline_to_source(&self, timeline_time: f64) -> f64 {
        let offset = timeline_time - self.timeline_in;
        if !self.reverse {
            self.source_in + offset * self.speed
        } else {
            let src_out = self.effective_source_out();
            src_out - offset * self.speed
        }
    }

    pub fn contains_timeline_time(&self, t: f64) -> bool {
        self.timeline_in <= t && t <= self.timeline_out + 0.001
    }

    pub fn move_to(&mut self, new_in: f64) {
        let dur = self.duration();
        self.timeline_in = new_in.max(0.0);
        self.timeline_out = self.timeline_in + dur;
    }

    pub fn trim_in(&mut self, new_in: f64) {
        let clamped_in = new_in.clamp(0.0, self.timeline_out - 0.04);
        let dt = clamped_in - self.timeline_in;
        self.timeline_in = clamped_in;
        self.source_in = (self.source_in + dt * self.speed).max(0.0);
    }

    pub fn trim_out(&mut self, new_out: f64) {
        let clamped_out = new_out.max(self.timeline_in + 0.04);
        self.timeline_out = clamped_out;
        self.source_out = Some(self.source_in + self.duration() * self.speed);
    }

    pub fn get_property_at_time(&self, prop: &str, clip_time: f64) -> f64 {
        let default_val = match prop {
            "pos_x" => self.pos_x,
            "pos_y" => self.pos_y,
            "scale" => self.scale,
            "opacity" => self.opacity,
            "rotation" => self.rotation,
            "volume" => self.volume,
            "brightness" => self.brightness,
            "contrast" => self.contrast,
            "saturation" => self.saturation,
            _ => 0.0,
        };

        if let Some(kfs) = self.keyframes.get(prop) {
            if !kfs.is_empty() {
                return interpolate_keyframes(kfs, clip_time, default_val);
            }
        }
        default_val
    }

    pub fn add_keyframe(&mut self, prop: &str, time: f64, value: f64, interpolation: Interpolation) {
        let list = self.keyframes.entry(prop.to_string()).or_default();
        // Remove existing keyframe if very close
        list.retain(|k| (k.time - time).abs() > 0.04);
        list.push(Keyframe::new(time, value, interpolation));
        list.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
    }

    pub fn remove_keyframe(&mut self, prop: &str, time: f64, tolerance: f64) -> bool {
        if let Some(list) = self.keyframes.get_mut(prop) {
            let orig_len = list.len();
            list.retain(|k| (k.time - time).abs() > tolerance);
            return list.len() < orig_len;
        }
        false
    }

    pub fn has_keyframes(&self, prop: Option<&str>) -> bool {
        match prop {
            Some(p) => self.keyframes.get(p).map_or(false, |list| !list.is_empty()),
            None => self.keyframes.values().any(|list| !list.is_empty()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clip_source_mapping_forward_and_reverse() {
        let mut c = Clip::new("m1", 2.0, 6.0);
        c.source_in = 1.0;
        c.source_out = Some(5.0);
        assert_eq!(c.map_timeline_to_source(2.0), 1.0);
        assert_eq!(c.map_timeline_to_source(4.0), 3.0);
        assert_eq!(c.map_timeline_to_source(6.0), 5.0);

        // Speed 2.0x
        c.speed = 2.0;
        c.timeline_in = 0.0;
        c.timeline_out = 2.0;
        c.source_in = 0.0;
        c.source_out = Some(4.0);
        assert_eq!(c.map_timeline_to_source(0.0), 0.0);
        assert_eq!(c.map_timeline_to_source(1.0), 2.0);
        assert_eq!(c.map_timeline_to_source(2.0), 4.0);

        // Reverse 1.0x
        c.speed = 1.0;
        c.reverse = true;
        c.timeline_in = 0.0;
        c.timeline_out = 3.0;
        c.source_in = 5.0;
        c.source_out = Some(8.0);
        assert_eq!(c.map_timeline_to_source(0.0), 8.0);
        assert_eq!(c.map_timeline_to_source(1.0), 7.0);
        assert_eq!(c.map_timeline_to_source(3.0), 5.0);
    }

    #[test]
    fn test_clip_keyframes() {
        let mut c = Clip::new("m1", 0.0, 4.0);
        assert!(!c.has_keyframes(None));
        c.add_keyframe("opacity", 0.0, 1.0, Interpolation::Linear);
        c.add_keyframe("opacity", 2.0, 0.2, Interpolation::Linear);
        c.add_keyframe("opacity", 4.0, 1.0, Interpolation::Linear);
        assert!(c.has_keyframes(Some("opacity")));
        assert!(!c.has_keyframes(Some("scale")));

        assert!((c.get_property_at_time("opacity", 1.0) - 0.6).abs() < 1e-4);
        assert!((c.get_property_at_time("opacity", 2.0) - 0.2).abs() < 1e-4);
    }
}
