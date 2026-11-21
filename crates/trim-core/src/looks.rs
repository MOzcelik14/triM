//! Look/Preset library (`.trimlook`) and Title Template definitions.

use crate::clip::{Clip, LutPreset, MotionTitleStyle, TextOverlay};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const CURRENT_LOOK_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LookPresetFile {
    #[serde(default = "default_look_version")]
    pub version: u32,
    pub name: String,
    pub category: String,
    pub brightness: f64,
    pub contrast: f64,
    pub saturation: f64,
    pub gamma: f64,
    pub temperature: f64,
    pub tint: f64,
    pub lut_preset: LutPreset,
    pub film_grain: f32,
    pub vignette: f32,
    pub halation: f32,
    pub bloom: f32,
    pub chromatic_aberration: f32,
}

fn default_look_version() -> u32 {
    CURRENT_LOOK_VERSION
}

impl LookPresetFile {
    pub fn from_clip(clip: &Clip, name: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            version: CURRENT_LOOK_VERSION,
            name: name.into(),
            category: category.into(),
            brightness: clip.brightness,
            contrast: clip.contrast,
            saturation: clip.saturation,
            gamma: clip.gamma,
            temperature: clip.temperature,
            tint: clip.tint,
            lut_preset: clip.lut_preset,
            film_grain: clip.film_grain,
            vignette: clip.vignette,
            halation: clip.halation,
            bloom: clip.bloom,
            chromatic_aberration: clip.chromatic_aberration,
        }
    }

    pub fn apply_to_clip(&self, clip: &mut Clip) {
        clip.brightness = self.brightness;
        clip.contrast = self.contrast;
        clip.saturation = self.saturation;
        clip.gamma = self.gamma;
        clip.temperature = self.temperature;
        clip.tint = self.tint;
        clip.lut_preset = self.lut_preset;
        clip.film_grain = self.film_grain;
        clip.vignette = self.vignette;
        clip.halation = self.halation;
        clip.bloom = self.bloom;
        clip.chromatic_aberration = self.chromatic_aberration;
    }

    pub fn looks_dir() -> PathBuf {
        crate::paths::data_dir().join("looks")
    }

    pub fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    pub fn load_from_file(path: &Path) -> std::io::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        serde_json::from_str(&raw)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn list_saved_looks(dir: &Path) -> Vec<(PathBuf, Self)> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("trimlook") {
                if let Ok(look) = Self::load_from_file(&p) {
                    out.push((p, look));
                }
            }
        }
        out.sort_by(|a, b| a.1.name.cmp(&b.1.name));
        out
    }
}

/// Returns 12 built-in cinematic `.trimlook` presets.
pub fn builtin_looks() -> Vec<LookPresetFile> {
    let base = |name: &str, cat: &str| LookPresetFile {
        version: CURRENT_LOOK_VERSION,
        name: name.to_string(),
        category: cat.to_string(),
        brightness: 0.0,
        contrast: 1.0,
        saturation: 1.0,
        gamma: 1.0,
        temperature: 0.0,
        tint: 0.0,
        lut_preset: LutPreset::None,
        film_grain: 0.0,
        vignette: 0.0,
        halation: 0.0,
        bloom: 0.0,
        chromatic_aberration: 0.0,
    };

    vec![
        LookPresetFile {
            lut_preset: LutPreset::TealOrange,
            contrast: 1.12,
            vignette: 0.18,
            ..base("Blockbuster Teal & Orange", "Sinema")
        },
        LookPresetFile {
            lut_preset: LutPreset::Kodak2383,
            contrast: 1.15,
            film_grain: 0.12,
            halation: 0.15,
            ..base("Kodak 2383 Print", "Film")
        },
        LookPresetFile {
            lut_preset: LutPreset::FujiFilm,
            contrast: 1.08,
            saturation: 1.12,
            film_grain: 0.10,
            ..base("Fuji Film Vivid", "Film")
        },
        LookPresetFile {
            lut_preset: LutPreset::MonochromeNoir,
            contrast: 1.28,
            saturation: 0.0,
            film_grain: 0.22,
            vignette: 0.30,
            ..base("Classic Noir B&W", "Monokrom")
        },
        LookPresetFile {
            lut_preset: LutPreset::BleachBypass,
            contrast: 1.24,
            saturation: 0.65,
            vignette: 0.20,
            ..base("Silver Bleach Bypass", "Sinema")
        },
        LookPresetFile {
            lut_preset: LutPreset::Vibrant,
            contrast: 1.16,
            saturation: 1.25,
            bloom: 0.22,
            chromatic_aberration: 0.12,
            ..base("Neon Vibrant", "Stilize")
        },
        LookPresetFile {
            lut_preset: LutPreset::CinematicWarm,
            temperature: 0.22,
            saturation: 1.14,
            bloom: 0.18,
            gamma: 1.06,
            ..base("Golden Hour Warmth", "Doğa & Vlog")
        },
        LookPresetFile {
            lut_preset: LutPreset::CinematicCool,
            temperature: -0.20,
            contrast: 1.10,
            saturation: 0.88,
            vignette: 0.15,
            ..base("Nordic Cool Mist", "Belgesel")
        },
        LookPresetFile {
            lut_preset: LutPreset::VintageFilm,
            contrast: 0.94,
            saturation: 0.85,
            film_grain: 0.25,
            halation: 0.20,
            ..base("1970s Super 8", "Retro")
        },
        LookPresetFile {
            contrast: 1.06,
            saturation: 1.10,
            brightness: 0.03,
            ..base("Clean Commercial Pop", "Ticari")
        },
        LookPresetFile {
            contrast: 1.18,
            brightness: -0.04,
            saturation: 0.90,
            vignette: 0.28,
            ..base("Moody Thriller", "Sinema")
        },
        LookPresetFile {
            lut_preset: LutPreset::FadedFilm,
            contrast: 0.92,
            saturation: 0.95,
            bloom: 0.28,
            ..base("Soft Faded Film", "Stilize")
        },
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleTemplate {
    LowerThird,
    OpeningTitle,
    ChapterCard,
    RollingCredits,
    TickerBanner,
}

impl TitleTemplate {
    pub const ALL: &'static [Self] = &[
        Self::LowerThird,
        Self::OpeningTitle,
        Self::ChapterCard,
        Self::RollingCredits,
        Self::TickerBanner,
    ];

    pub fn i18n_key(&self) -> &'static str {
        match self {
            Self::LowerThird => "title_tpl_lower_third",
            Self::OpeningTitle => "title_tpl_opening",
            Self::ChapterCard => "title_tpl_chapter",
            Self::RollingCredits => "title_tpl_credits",
            Self::TickerBanner => "title_tpl_ticker",
        }
    }

    /// Maps the template onto existing `TextOverlay` + clip duration parameters.
    pub fn build_overlay(&self, title: &str, subtitle: &str) -> (TextOverlay, f64) {
        let mut ov = TextOverlay {
            content: title.to_string(),
            subtitle_text: if subtitle.is_empty() {
                None
            } else {
                Some(subtitle.to_string())
            },
            ..Default::default()
        };
        let dur = match self {
            Self::LowerThird => {
                ov.motion_style = MotionTitleStyle::LowerThird;
                ov.font_size = 38.0;
                ov.pos_x = 0.08;
                ov.pos_y = 0.82;
                4.5
            }
            Self::OpeningTitle => {
                ov.motion_style = MotionTitleStyle::CinematicZoom;
                ov.font_size = 68.0;
                ov.pos_x = 0.5;
                ov.pos_y = 0.5;
                4.0
            }
            Self::ChapterCard => {
                ov.motion_style = MotionTitleStyle::ChapterCard;
                ov.font_size = 56.0;
                ov.pos_x = 0.5;
                ov.pos_y = 0.5;
                3.5
            }
            Self::RollingCredits => {
                ov.motion_style = MotionTitleStyle::RollingCredits;
                ov.font_size = 34.0;
                ov.pos_x = 0.5;
                ov.pos_y = 0.5;
                8.0
            }
            Self::TickerBanner => {
                ov.motion_style = MotionTitleStyle::Ticker;
                ov.font_size = 28.0;
                ov.pos_x = 0.5;
                ov.pos_y = 0.90;
                6.0
            }
        };
        (ov, dur)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_looks_and_trimlook_roundtrip() {
        let looks = builtin_looks();
        assert!(looks.len() >= 10, "expected at least 10 built-in looks");

        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("custom.trimlook");
        looks[0].save_to_file(&path).expect("save .trimlook");

        let loaded = LookPresetFile::load_from_file(&path).expect("load .trimlook");
        assert_eq!(loaded, looks[0]);

        let listed = LookPresetFile::list_saved_looks(tmp.path());
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].1.name, looks[0].name);

        let mut clip = Clip::new("m1", 0.0, 3.0);
        loaded.apply_to_clip(&mut clip);
        assert_eq!(clip.lut_preset, looks[0].lut_preset);
        assert!((clip.contrast - looks[0].contrast).abs() < 1e-6);
    }

    #[test]
    fn test_title_templates_map_to_text_overlay() {
        assert_eq!(TitleTemplate::ALL.len(), 5);
        for tpl in TitleTemplate::ALL {
            let (ov, dur) = tpl.build_overlay("Başlık", "Alt Başlık");
            assert_eq!(ov.content, "Başlık");
            assert_eq!(ov.subtitle_text.as_deref(), Some("Alt Başlık"));
            assert!(dur >= 3.0);
            assert!(!tpl.i18n_key().is_empty());
        }
    }
}
