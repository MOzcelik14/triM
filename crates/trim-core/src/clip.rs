//! Clip model for timeline items.

use crate::keyframe::{interpolate_keyframes, Interpolation, Keyframe};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioEqPreset {
    #[default]
    Flat,
    VoiceClarity,
    BassBoost,
    Podcast,
    DenoiseGate,
}

impl AudioEqPreset {
    pub const ALL: &'static [Self] = &[
        Self::Flat,
        Self::VoiceClarity,
        Self::BassBoost,
        Self::Podcast,
        Self::DenoiseGate,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Flat => "Düz (Doğal)",
            Self::VoiceClarity => "Konuşma / Vokal Netliği",
            Self::BassBoost => "Derin Bas (Bass Boost)",
            Self::Podcast => "Podcast Yayını",
            Self::DenoiseGate => "Dip Ses / Gürültü Filtresi",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LutPreset {
    #[default]
    None,
    TealOrange,
    VintageFilm,
    MonochromeNoir,
    CyberpunkNeon,
}

impl LutPreset {
    pub const ALL: &'static [Self] = &[
        Self::None,
        Self::TealOrange,
        Self::VintageFilm,
        Self::MonochromeNoir,
        Self::CyberpunkNeon,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "Yok (Doğal)",
            Self::TealOrange => "Teal & Orange (Sinematik)",
            Self::VintageFilm => "Vintage Film (Retro Sıcak)",
            Self::MonochromeNoir => "Monochrome Noir (Siyah & Beyaz)",
            Self::CyberpunkNeon => "Cyberpunk Neon (Canlı Mor/Mavi)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptionStyle {
    #[default]
    Standard,
    PopUpBouncy,
    WordHighlightYellow,
    WordHighlightNeon,
    BoxBackground,
}

impl CaptionStyle {
    pub const ALL: &'static [Self] = &[
        Self::Standard,
        Self::PopUpBouncy,
        Self::WordHighlightYellow,
        Self::WordHighlightNeon,
        Self::BoxBackground,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Standard => "Klasik Standart Altyazı",
            Self::PopUpBouncy => "💥 Pop-Up / Zıplayan Kelime (TikTok / Reels)",
            Self::WordHighlightYellow => "⚡ Sarı Vurgulu Kelime (YouTube Shorts)",
            Self::WordHighlightNeon => "🟢 Neon Yeşil Vurgu (Dinamik)",
            Self::BoxBackground => "📦 Arka Plan Kutulu (Modern NLE)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorLabel {
    #[default]
    None,
    Blue,
    Green,
    Orange,
    Purple,
    Yellow,
    Red,
    Cyan,
    Magenta,
}

impl ColorLabel {
    pub const ALL: &'static [Self] = &[
        Self::None,
        Self::Blue,
        Self::Green,
        Self::Orange,
        Self::Purple,
        Self::Yellow,
        Self::Red,
        Self::Cyan,
        Self::Magenta,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "Varsayılan (Gri/Kanal Rengi)",
            Self::Blue => "Mavi (B-Roll / Sahne)",
            Self::Green => "Yeşil (Röportaj / A-Roll)",
            Self::Orange => "Turuncu (Aksiyon / Vurgu)",
            Self::Purple => "Mor (Müzik / SFX)",
            Self::Yellow => "Sarı (Diyalog / Vokal)",
            Self::Red => "Kırmızı (Önemli / Düzeltme)",
            Self::Cyan => "Camgöbeği (VFX / Grafik)",
            Self::Magenta => "Pembe / Magenta (Sosyal Medya)",
        }
    }

    pub fn color_rgb(&self) -> Option<[u8; 3]> {
        match self {
            Self::None => None,
            Self::Blue => Some([59, 130, 246]),
            Self::Green => Some([34, 197, 94]),
            Self::Orange => Some([249, 115, 22]),
            Self::Purple => Some([168, 85, 247]),
            Self::Yellow => Some([234, 179, 8]),
            Self::Red => Some([239, 68, 68]),
            Self::Cyan => Some([6, 182, 212]),
            Self::Magenta => Some([236, 72, 153]),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanZoomPreset {
    #[default]
    None,
    ZoomIn,
    ZoomOut,
    PanLeft,
    PanRight,
    DramaticZoom,
}

impl PanZoomPreset {
    pub const ALL: &'static [Self] = &[
        Self::None,
        Self::ZoomIn,
        Self::ZoomOut,
        Self::PanLeft,
        Self::PanRight,
        Self::DramaticZoom,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "Yok (Sabit)",
            Self::ZoomIn => "🔍 Yavaşça Yaklaş (Zoom In)",
            Self::ZoomOut => "🔍 Yavaşça Uzaklaş (Zoom Out)",
            Self::PanLeft => "⬅️ Sola Kay (Pan Left)",
            Self::PanRight => "➡️ Sağa Kay (Pan Right)",
            Self::DramaticZoom => "⚡ Hızlı Dramatik Yaklaş",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChromaKeyConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_chromakey_green")]
    pub key_color: [u8; 3],
    #[serde(default = "default_chromakey_similarity")]
    pub similarity: f32,
    #[serde(default = "default_chromakey_smoothness")]
    pub smoothness: f32,
}

fn default_chromakey_green() -> [u8; 3] {
    [0, 255, 0]
}
fn default_chromakey_similarity() -> f32 {
    0.25
}
fn default_chromakey_smoothness() -> f32 {
    0.10
}

impl Default for ChromaKeyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            key_color: default_chromakey_green(),
            similarity: default_chromakey_similarity(),
            smoothness: default_chromakey_smoothness(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorWheelsConfig {
    #[serde(default = "default_zero_vec3")]
    pub lift: [f32; 3], // RGB shift for Shadows: [-0.5 .. 0.5]
    #[serde(default = "default_one_vec3")]
    pub gamma: [f32; 3], // RGB shift for Midtones: [0.2 .. 2.0]
    #[serde(default = "default_one_vec3")]
    pub gain: [f32; 3], // RGB multiplier for Highlights: [0.0 .. 2.0]
    #[serde(default = "default_zero_vec3")]
    pub luminance: [f32; 3], // Lift, Gamma, Gain Y/brightness sliders
}

fn default_zero_vec3() -> [f32; 3] {
    [0.0, 0.0, 0.0]
}
fn default_one_vec3() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

impl Default for ColorWheelsConfig {
    fn default() -> Self {
        Self {
            lift: default_zero_vec3(),
            gamma: default_one_vec3(),
            gain: default_one_vec3(),
            luminance: default_zero_vec3(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualizerStyle {
    #[default]
    WaveformBars,
    SpectrumLine,
    CircularPulsing,
}

impl VisualizerStyle {
    pub const ALL: &'static [Self] = &[
        Self::WaveformBars,
        Self::SpectrumLine,
        Self::CircularPulsing,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::WaveformBars => "📊 Dalga Boyu Çubukları (Bars)",
            Self::SpectrumLine => "📈 Frekans Spektrumu Çizgisi (Spectrum)",
            Self::CircularPulsing => "🟣 Dairesel / Nabız Görselleştirici",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioVisualizerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub style: VisualizerStyle,
    #[serde(default = "default_visualizer_color")]
    pub color: [u8; 4],
    #[serde(default = "default_visualizer_pos_y")]
    pub pos_y: f32,
    #[serde(default = "default_one_f32")]
    pub scale: f32,
}

fn default_visualizer_color() -> [u8; 4] {
    [99, 102, 241, 255]
}
fn default_visualizer_pos_y() -> f32 {
    0.75
}
fn default_one_f32() -> f32 {
    1.0
}

impl Default for AudioVisualizerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            style: VisualizerStyle::WaveformBars,
            color: default_visualizer_color(),
            pos_y: default_visualizer_pos_y(),
            scale: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextOverlay {
    pub content: String,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_white_color")]
    pub text_color: [u8; 4],
    #[serde(default)]
    pub bg_color: Option<[u8; 4]>,
    #[serde(default = "default_half_f32")]
    pub pos_x: f32,
    #[serde(default = "default_subtitle_pos_y")]
    pub pos_y: f32,
    #[serde(default = "default_true")]
    pub has_shadow: bool,
    #[serde(default)]
    pub caption_style: CaptionStyle,
    #[serde(default = "default_highlight_color")]
    pub highlight_color: [u8; 4],
}

fn default_font_size() -> f32 {
    42.0
}
fn default_white_color() -> [u8; 4] {
    [255, 255, 255, 255]
}
fn default_highlight_color() -> [u8; 4] {
    [255, 230, 0, 255] // Yellow pop
}
fn default_half_f32() -> f32 {
    0.5
}
fn default_subtitle_pos_y() -> f32 {
    0.82
}
fn default_true() -> bool {
    true
}

impl Default for TextOverlay {
    fn default() -> Self {
        Self {
            content: "Yeni Başlık".to_string(),
            font_size: 42.0,
            text_color: [255, 255, 255, 255],
            bg_color: None,
            pos_x: 0.5,
            pos_y: 0.82,
            has_shadow: true,
            caption_style: CaptionStyle::Standard,
            highlight_color: default_highlight_color(),
        }
    }
}

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
    #[serde(default)]
    pub lut_preset: LutPreset,
    #[serde(default)]
    pub custom_lut_path: Option<String>,
    #[serde(default = "default_one")]
    pub volume: f64,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub eq_preset: AudioEqPreset,
    #[serde(default)]
    pub noise_gate: bool,
    #[serde(default)]
    pub text_overlay: Option<TextOverlay>,
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
    #[serde(default)]
    pub linked_group_id: Option<String>,
    #[serde(default)]
    pub chromakey: Option<ChromaKeyConfig>,
    #[serde(default)]
    pub color_wheels: Option<ColorWheelsConfig>,
    #[serde(default)]
    pub audio_visualizer: Option<AudioVisualizerConfig>,
    #[serde(default)]
    pub color_label: ColorLabel,
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
            lut_preset: LutPreset::None,
            custom_lut_path: None,
            volume: 1.0,
            muted: false,
            eq_preset: AudioEqPreset::default(),
            noise_gate: false,
            text_overlay: None,
            pos_x: 0.0,
            pos_y: 0.0,
            scale: 1.0,
            rotation: 0.0,
            opacity: 1.0,
            keyframes: HashMap::new(),
            linked_group_id: None,
            chromakey: None,
            color_wheels: None,
            audio_visualizer: None,
            color_label: ColorLabel::None,
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

    pub fn apply_pan_zoom_preset(&mut self, preset: PanZoomPreset) {
        let dur = self.duration().max(0.1);
        match preset {
            PanZoomPreset::None => {
                self.keyframes.remove("scale");
                self.keyframes.remove("pos_x");
                self.keyframes.remove("pos_y");
                self.scale = 1.0;
                self.pos_x = 0.0;
                self.pos_y = 0.0;
            }
            PanZoomPreset::ZoomIn => {
                self.pos_x = 0.0;
                self.pos_y = 0.0;
                self.keyframes.insert(
                    "scale".to_string(),
                    vec![
                        Keyframe::new(0.0, 1.0, Interpolation::EaseInOut),
                        Keyframe::new(dur, 1.25, Interpolation::EaseInOut),
                    ],
                );
            }
            PanZoomPreset::ZoomOut => {
                self.pos_x = 0.0;
                self.pos_y = 0.0;
                self.keyframes.insert(
                    "scale".to_string(),
                    vec![
                        Keyframe::new(0.0, 1.25, Interpolation::EaseInOut),
                        Keyframe::new(dur, 1.0, Interpolation::EaseInOut),
                    ],
                );
            }
            PanZoomPreset::PanLeft => {
                self.keyframes.insert(
                    "scale".to_string(),
                    vec![Keyframe::new(0.0, 1.15, Interpolation::Linear)],
                );
                self.keyframes.insert(
                    "pos_x".to_string(),
                    vec![
                        Keyframe::new(0.0, 60.0, Interpolation::EaseInOut),
                        Keyframe::new(dur, -60.0, Interpolation::EaseInOut),
                    ],
                );
            }
            PanZoomPreset::PanRight => {
                self.keyframes.insert(
                    "scale".to_string(),
                    vec![Keyframe::new(0.0, 1.15, Interpolation::Linear)],
                );
                self.keyframes.insert(
                    "pos_x".to_string(),
                    vec![
                        Keyframe::new(0.0, -60.0, Interpolation::EaseInOut),
                        Keyframe::new(dur, 60.0, Interpolation::EaseInOut),
                    ],
                );
            }
            PanZoomPreset::DramaticZoom => {
                self.pos_x = 0.0;
                self.pos_y = 0.0;
                self.keyframes.insert(
                    "scale".to_string(),
                    vec![
                        Keyframe::new(0.0, 1.0, Interpolation::EaseInOut),
                        Keyframe::new(dur, 1.5, Interpolation::EaseInOut),
                    ],
                );
            }
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

    pub const SUPPORTED_TRANSITIONS: &'static [(&'static str, &'static str)] = &[
        ("dip_black", "Siyaha Geçiş (Dip to Black)"),
        ("dip_white", "Beyaza Geçiş (Dip to White)"),
        ("cross_dissolve", "Çapraz Erime (Cross Dissolve)"),
        ("wipe_left", "Sola Süpürme (Wipe Left)"),
        ("wipe_right", "Sağa Süpürme (Wipe Right)"),
        ("slide_left", "Sola Kayma (Slide Left)"),
        ("slide_right", "Sağa Kayma (Slide Right)"),
        ("zoom_in", "Yakınlaşma (Zoom In)"),
    ];

    pub fn transition_label(trans: Option<&str>) -> &'static str {
        match trans {
            Some(key) => Self::SUPPORTED_TRANSITIONS
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, l)| *l)
                .unwrap_or("Özel"),
            None => "Yok (Düz)",
        }
    }

    pub fn contains_timeline_time(&self, t: f64) -> bool {
        // Timeline intervals are half-open: [in, out). The out point is the
        // first instant after this clip, not another visible frame.
        self.timeline_in <= t && t < self.timeline_out
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

    /// Changes the playback speed of this clip and proportionally stretches or
    /// shrinks its duration on the timeline, maintaining the same range of source media.
    pub fn change_speed(&mut self, new_speed: f64) {
        let clamped_speed = new_speed.clamp(0.1, 10.0);
        if (self.speed - clamped_speed).abs() < 1e-6 {
            return;
        }

        // Determine source span covered by this clip BEFORE updating speed
        let old_src_out = self.effective_source_out();
        let src_span = (old_src_out - self.source_in).max(0.04);

        // Adjust timeline_out based on the new speed
        let new_dur = (src_span / clamped_speed).max(0.04);
        self.timeline_out = self.timeline_in + new_dur;
        self.source_out = Some(self.source_in + src_span);
        self.speed = clamped_speed;
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

    pub fn add_keyframe(
        &mut self,
        prop: &str,
        time: f64,
        value: f64,
        interpolation: Interpolation,
    ) {
        let list = self.keyframes.entry(prop.to_string()).or_default();
        // Remove existing keyframe if very close
        list.retain(|k| (k.time - time).abs() > 0.04);
        list.push(Keyframe::new(time, value, interpolation));
        list.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
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
            Some(p) => self.keyframes.get(p).is_some_and(|list| !list.is_empty()),
            None => self.keyframes.values().any(|list| !list.is_empty()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clip_visibility_excludes_out_point() {
        let clip = Clip::new("frame", 1.0, 3.0);
        assert!(!clip.contains_timeline_time(0.999));
        assert!(clip.contains_timeline_time(1.0));
        assert!(clip.contains_timeline_time(2.999));
        assert!(!clip.contains_timeline_time(3.0));
        assert!(!clip.contains_timeline_time(3.001));
    }

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

    #[test]
    fn test_clip_change_speed_scales_timeline_duration() {
        let mut c = Clip::new("m1", 2.0, 6.0); // duration = 4.0, speed = 1.0
        assert_eq!(c.duration(), 4.0);

        // Slow down to 0.5x -> duration should double to 8.0s
        c.change_speed(0.5);
        assert_eq!(c.speed, 0.5);
        assert_eq!(c.timeline_in, 2.0);
        assert_eq!(c.timeline_out, 10.0);
        assert_eq!(c.duration(), 8.0);
        assert_eq!(c.source_out, Some(4.0));

        // Speed up to 2.0x -> duration should become 4.0 / 2.0 = 2.0s
        c.change_speed(2.0);
        assert_eq!(c.speed, 2.0);
        assert_eq!(c.timeline_in, 2.0);
        assert_eq!(c.timeline_out, 4.0);
        assert_eq!(c.duration(), 2.0);
        assert_eq!(c.source_out, Some(4.0));

        // Back to 1.0x -> duration back to 4.0s
        c.change_speed(1.0);
        assert_eq!(c.speed, 1.0);
        assert_eq!(c.timeline_in, 2.0);
        assert_eq!(c.timeline_out, 6.0);
        assert_eq!(c.duration(), 4.0);
    }

    #[test]
    fn test_pan_zoom_presets() {
        let mut c = Clip::new("m1", 0.0, 4.0);
        c.apply_pan_zoom_preset(PanZoomPreset::ZoomIn);
        assert!(c.has_keyframes(Some("scale")));
        let kfs = c.keyframes.get("scale").unwrap();
        assert_eq!(kfs.len(), 2);
        assert_eq!(kfs[0].value, 1.0);
        assert_eq!(kfs[1].value, 1.25);

        c.apply_pan_zoom_preset(PanZoomPreset::PanLeft);
        assert!(c.has_keyframes(Some("pos_x")));
        let x_kfs = c.keyframes.get("pos_x").unwrap();
        assert_eq!(x_kfs[0].value, 60.0);
        assert_eq!(x_kfs[1].value, -60.0);

        c.apply_pan_zoom_preset(PanZoomPreset::None);
        assert!(!c.has_keyframes(Some("scale")));
        assert!(!c.has_keyframes(Some("pos_x")));
        assert_eq!(c.scale, 1.0);
        assert_eq!(c.pos_x, 0.0);
    }

    #[test]
    fn test_chromakey_and_color_wheels_defaults() {
        let c = Clip::default();
        assert!(c.chromakey.is_none());
        assert!(c.color_wheels.is_none());
        assert!(c.audio_visualizer.is_none());

        let ck = ChromaKeyConfig::default();
        assert!(ck.enabled);
        assert_eq!(ck.key_color, [0, 255, 0]);
        assert_eq!(ck.similarity, 0.25);

        let cw = ColorWheelsConfig::default();
        assert_eq!(cw.lift, [0.0, 0.0, 0.0]);
        assert_eq!(cw.gamma, [1.0, 1.0, 1.0]);
        assert_eq!(cw.gain, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_color_label_roundtrip() {
        let mut c = Clip::default();
        assert_eq!(c.color_label, ColorLabel::None);
        c.color_label = ColorLabel::Orange;
        assert_eq!(c.color_label.label(), "Turuncu (Aksiyon / Vurgu)");
        assert_eq!(c.color_label.color_rgb(), Some([249, 115, 22]));

        let json = serde_json::to_string(&c).expect("serialize clip");
        let decoded: Clip = serde_json::from_str(&json).expect("deserialize clip");
        assert_eq!(decoded.color_label, ColorLabel::Orange);
    }
}
