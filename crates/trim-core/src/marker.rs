//! Marker model for timeline timestamps.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Marker {
    pub id: String,
    pub time: f64,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_marker_color")]
    pub color: String,
}

fn default_marker_color() -> String {
    "#E07A38".to_string()
}

impl Default for Marker {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            time: 0.0,
            name: String::new(),
            color: default_marker_color(),
        }
    }
}

impl Marker {
    pub fn new(time: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            time,
            name: String::new(),
            color: default_marker_color(),
        }
    }

    pub fn with_details(time: f64, name: impl Into<String>, color: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            time,
            name: name.into(),
            color: color.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marker_serialization() {
        let marker = Marker::with_details(3.5, "Verse 1", "#3498db");
        let json = serde_json::to_string(&marker).unwrap();
        let deserialized: Marker = serde_json::from_str(&json).unwrap();
        assert_eq!(marker.id, deserialized.id);
        assert_eq!(marker.time, deserialized.time);
        assert_eq!(marker.name, deserialized.name);
        assert_eq!(marker.color, deserialized.color);
    }
}
