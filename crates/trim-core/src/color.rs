//! Unified color grading, White Balance (Temperature/Tint), Auto WB analysis,
//! Brightness/Contrast/Gamma presets, and categorized Creative Color Looks.

use serde::{Deserialize, Serialize};

/// Camera Log / Transfer input profiles converted to Rec.709 before primary color grading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraLogProfile {
    #[default]
    Rec709,
    SLog3,
    CLog,
    VLog,
    LogC,
}

impl CameraLogProfile {
    pub const ALL: &'static [Self] = &[
        Self::Rec709,
        Self::SLog3,
        Self::CLog,
        Self::VLog,
        Self::LogC,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Rec709 => "Rec.709 (Standart)",
            Self::SLog3 => "Sony S-Log3 → Rec.709",
            Self::CLog => "Canon C-Log → Rec.709",
            Self::VLog => "Panasonic V-Log → Rec.709",
            Self::LogC => "ARRI Log-C → Rec.709",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Rec709 => "Rec.709",
            Self::SLog3 => "S-Log3",
            Self::CLog => "C-Log",
            Self::VLog => "V-Log",
            Self::LogC => "Log-C",
        }
    }

    pub fn shader_id(&self) -> i32 {
        match self {
            Self::Rec709 => 0,
            Self::SLog3 => 1,
            Self::CLog => 2,
            Self::VLog => 3,
            Self::LogC => 4,
        }
    }

    /// Converts a single normalized Log channel value `[0.0, 1.0]` to linear scene reflectance
    /// and then encodes with the Rec.709 OETF.
    #[inline]
    pub fn apply_channel(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        let lin = match self {
            Self::Rec709 => return x,
            Self::SLog3 => {
                if x >= 0.100_537_78 {
                    (10.0_f32.powf((x - 0.790453) / 0.344676) - 0.009468) / 0.555556
                } else {
                    (x - 0.092864) / 8.735631
                }
            }
            Self::CLog => {
                if x >= 0.100_686_68 {
                    (10.0_f32.powf((x - 0.384316) / 0.245281) - 0.064829) / 5.555556
                } else {
                    (x - 0.092864) / 8.799461
                }
            }
            Self::VLog => {
                let cut = 171.210_3 / 1023.0;
                if x >= cut {
                    10.0_f32.powf((x * 1023.0 - 420.0) / 261.5) * 0.19 - 0.01
                } else {
                    ((x * 1023.0 - 95.0) * 0.01125) / (171.210_3 - 95.0)
                }
            }
            Self::LogC => {
                if x > 0.14 {
                    (10.0_f32.powf(3.89616 * x - 2.27752) - 0.0929) / 0.9892
                } else {
                    (x - 0.0929) / 6.025
                }
            }
        };
        let v = lin.clamp(0.0, 1.0);
        let oetf = if v < 0.018 {
            4.5 * v
        } else {
            1.099 * v.powf(0.45) - 0.099
        };
        oetf.clamp(0.0, 1.0)
    }

    #[inline]
    pub fn apply_rgb(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        if *self == Self::Rec709 {
            return [r, g, b];
        }
        [
            self.apply_channel(r),
            self.apply_channel(g),
            self.apply_channel(b),
        ]
    }
}

/// Analog film stock formats modeling toe compression, dye density, and highlight roll-off knee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilmStockFormat {
    #[default]
    None,
    Film35mm,
    Film16mm,
    Film8mm,
}

impl FilmStockFormat {
    pub const ALL: &'static [Self] = &[Self::None, Self::Film35mm, Self::Film16mm, Self::Film8mm];

    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "Kapalı (Dijital)",
            Self::Film35mm => "35mm Sinema Filmi (Dengeli Kontrast & İnce Doku)",
            Self::Film16mm => "16mm Bağımsız Film (Yumuşak Parlaklık & Organik Toe)",
            Self::Film8mm => "8mm Vintage Film (Yüksek Kontrast & Nostaljik Sıcaklık)",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Self::None => "Yok",
            Self::Film35mm => "35mm",
            Self::Film16mm => "16mm",
            Self::Film8mm => "8mm",
        }
    }

    pub fn shader_id(&self) -> i32 {
        match self {
            Self::None => 0,
            Self::Film35mm => 1,
            Self::Film16mm => 2,
            Self::Film8mm => 3,
        }
    }

    /// Applies analog film stock tone response and exponential highlight roll-off knee to `[0.0, 1.0]` RGB.
    #[inline]
    pub fn apply_rgb(self, mut r: f32, mut g: f32, mut b: f32, highlight_rolloff: f32) -> [f32; 3] {
        let roll_extra = highlight_rolloff.clamp(0.0, 1.0);
        if self == Self::None && roll_extra <= 1e-4 {
            return [r, g, b];
        }

        let (fmt_contrast, fmt_roll, warm_shift, shadow_lift, sat_scale) = match self {
            Self::None => (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32, 1.0_f32),
            Self::Film35mm => (0.06, 0.14, 0.015, 0.02, 0.96),
            Self::Film16mm => (0.11, 0.22, 0.028, 0.035, 0.91),
            Self::Film8mm => (0.17, 0.30, 0.045, 0.05, 0.84),
        };

        // Warm print bias & shadow toe lift
        r = (r + warm_shift + shadow_lift * (1.0 - r) * (1.0 - r)).clamp(0.0, 1.0);
        g = (g + warm_shift * 0.25 + shadow_lift * 0.85 * (1.0 - g) * (1.0 - g)).clamp(0.0, 1.0);
        b = (b - warm_shift * 0.75 + shadow_lift * 1.15 * (1.0 - b) * (1.0 - b)).clamp(0.0, 1.0);

        // Midtone contrast
        if fmt_contrast > 1e-4 {
            let k = 1.0 + fmt_contrast;
            r = ((r - 0.5) * k + 0.5).clamp(0.0, 1.0);
            g = ((g - 0.5) * k + 0.5).clamp(0.0, 1.0);
            b = ((b - 0.5) * k + 0.5).clamp(0.0, 1.0);
        }

        // Exponential highlight roll-off knee
        let roll = (fmt_roll + roll_extra * 0.65).clamp(0.0, 1.0);
        if roll > 1e-4 {
            let knee = 0.62 - roll * 0.18;
            let denom = (1.0 - knee).max(0.001);
            let k_exp = 1.0 + roll * 1.5;
            let e_denom = (1.0 - (-k_exp).exp()).max(1e-4);
            let roll_chan = |c: f32| -> f32 {
                if c <= knee {
                    c
                } else {
                    let t = (c - knee) / denom;
                    let comp = knee + denom * (1.0 - (-t * k_exp).exp()) / e_denom;
                    (c * (1.0 - roll) + comp * roll).clamp(0.0, 1.0)
                }
            };
            r = roll_chan(r);
            g = roll_chan(g);
            b = roll_chan(b);
        }

        // Dye density saturation response
        if (sat_scale - 1.0).abs() > 1e-4 {
            let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            r = (luma + (r - luma) * sat_scale).clamp(0.0, 1.0);
            g = (luma + (g - luma) * sat_scale).clamp(0.0, 1.0);
            b = (luma + (b - luma) * sat_scale).clamp(0.0, 1.0);
        }

        [r, g, b]
    }
}

/// White balance presets for quick color temperature & tint calibration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhiteBalancePreset {
    #[default]
    Custom,
    Auto,
    Daylight,
    Tungsten,
    Cloudy,
    Shade,
    Fluorescent,
    Warm,
    Cool,
}

impl WhiteBalancePreset {
    pub const ALL: &'static [Self] = &[
        Self::Auto,
        Self::Daylight,
        Self::Tungsten,
        Self::Cloudy,
        Self::Shade,
        Self::Fluorescent,
        Self::Warm,
        Self::Cool,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Custom => "Manuel (Özel)",
            Self::Auto => "Auto White Balance (Otomatik)",
            Self::Daylight => "Daylight (5600K)",
            Self::Tungsten => "Tungsten (3200K)",
            Self::Cloudy => "Cloudy (Bulutlu)",
            Self::Shade => "Shade (Gölge)",
            Self::Fluorescent => "Fluorescent (Floresan)",
            Self::Warm => "Warm (Sıcak)",
            Self::Cool => "Cool (Soğuk)",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Custom => "Özel",
            Self::Auto => "Auto WB",
            Self::Daylight => "Daylight",
            Self::Tungsten => "Tungsten",
            Self::Cloudy => "Cloudy",
            Self::Shade => "Shade",
            Self::Fluorescent => "Fluorescent",
            Self::Warm => "Warm",
            Self::Cool => "Cool",
        }
    }

    /// Returns static `(temperature, tint)` values in `[-100.0, 100.0]` for fixed presets.
    /// Returns `None` for `Auto` (computed dynamically from image analysis) and `Custom`.
    pub fn temperature_tint(&self) -> Option<(f64, f64)> {
        match self {
            Self::Custom | Self::Auto => None,
            Self::Daylight => Some((10.0, 2.0)),
            Self::Tungsten => Some((-45.0, -4.0)),
            Self::Cloudy => Some((22.0, 4.0)),
            Self::Shade => Some((35.0, 6.0)),
            Self::Fluorescent => Some((-18.0, 18.0)),
            Self::Warm => Some((30.0, 5.0)),
            Self::Cool => Some((-30.0, -5.0)),
        }
    }

    /// Representative RGB swatch color for UI badges/thumbnails.
    pub fn swatch_rgb(&self) -> [u8; 3] {
        match self {
            Self::Custom => [140, 148, 164],
            Self::Auto => [96, 195, 235],
            Self::Daylight => [255, 236, 196],
            Self::Tungsten => [130, 182, 255],
            Self::Cloudy => [255, 206, 142],
            Self::Shade => [255, 186, 112],
            Self::Fluorescent => [216, 178, 248],
            Self::Warm => [255, 174, 96],
            Self::Cool => [110, 190, 255],
        }
    }
}

/// Brightness, Contrast, and Gamma tone presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrightnessContrastPreset {
    #[default]
    Neutral,
    SoftContrast,
    HighContrast,
    LowContrast,
    Bright,
    DarkMoody,
    CinematicContrast,
    Flat,
}

impl BrightnessContrastPreset {
    pub const ALL: &'static [Self] = &[
        Self::Neutral,
        Self::SoftContrast,
        Self::HighContrast,
        Self::LowContrast,
        Self::Bright,
        Self::DarkMoody,
        Self::CinematicContrast,
        Self::Flat,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Neutral => "Neutral (Doğal)",
            Self::SoftContrast => "Soft Contrast (Yumuşak Kontrast)",
            Self::HighContrast => "High Contrast (Yüksek Kontrast)",
            Self::LowContrast => "Low Contrast (Düşük Kontrast)",
            Self::Bright => "Bright (Aydınlık)",
            Self::DarkMoody => "Dark / Moody (Karanlık / Dramatik)",
            Self::CinematicContrast => "Cinematic Contrast (Sinematik Kontrast)",
            Self::Flat => "Flat (Düz / Log Görünüm)",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Neutral => "Neutral",
            Self::SoftContrast => "Soft Contrast",
            Self::HighContrast => "High Contrast",
            Self::LowContrast => "Low Contrast",
            Self::Bright => "Bright",
            Self::DarkMoody => "Dark / Moody",
            Self::CinematicContrast => "Cinematic",
            Self::Flat => "Flat",
        }
    }

    /// Returns `(brightness, contrast, gamma)` for this preset.
    pub fn params(&self) -> (f64, f64, f64) {
        match self {
            Self::Neutral => (0.0, 1.0, 1.0),
            Self::SoftContrast => (0.02, 1.12, 1.06),
            Self::HighContrast => (-0.02, 1.45, 0.92),
            Self::LowContrast => (0.03, 0.78, 1.10),
            Self::Bright => (0.12, 1.08, 1.18),
            Self::DarkMoody => (-0.10, 1.25, 0.80),
            Self::CinematicContrast => (-0.03, 1.32, 0.90),
            Self::Flat => (0.05, 0.68, 1.22),
        }
    }
}

/// Categories for Creative Color Looks (`LutPreset`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LookCategory {
    Cinematic,
    FilmEmulation,
    Creative,
}

impl LookCategory {
    pub const ALL: &'static [Self] = &[Self::Cinematic, Self::FilmEmulation, Self::Creative];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Cinematic => "Cinematic (Sinematik)",
            Self::FilmEmulation => "Film Emulation (Film)",
            Self::Creative => "Creative (Yaratıcı)",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Cinematic => "Cinematic",
            Self::FilmEmulation => "Film Emulation",
            Self::Creative => "Creative",
        }
    }
}

/// Expanded Creative Color Looks & Film Emulation presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LutPreset {
    #[default]
    None,

    // --- Cinematic ---
    TealOrange,
    CinematicWarm,
    CinematicCool,
    Blockbuster,
    FilmicNeutral,

    // --- Film Emulation ---
    Kodak2383,
    KodakPortra,
    FujiFilm,
    VintageFilm,
    FadedFilm,
    AnalogFilm,

    // --- Creative ---
    #[serde(alias = "monochrome")]
    MonochromeNoir,
    Sepia,
    BleachBypass,
    MutedColors,
    Vibrant,
    Pastel,
    GoldenHour,
    ColdWinter,
    NightCinematic,
    CyberpunkNeon,
}

impl LutPreset {
    /// Alias matching `Monochrome` in the Creative category while preserving `MonochromeNoir` compatibility.
    #[allow(non_upper_case_globals)]
    pub const Monochrome: Self = Self::MonochromeNoir;

    pub const ALL: &'static [Self] = &[
        Self::None,
        // Cinematic
        Self::TealOrange,
        Self::CinematicWarm,
        Self::CinematicCool,
        Self::Blockbuster,
        Self::FilmicNeutral,
        // Film Emulation
        Self::Kodak2383,
        Self::KodakPortra,
        Self::FujiFilm,
        Self::VintageFilm,
        Self::FadedFilm,
        Self::AnalogFilm,
        // Creative
        Self::MonochromeNoir,
        Self::Sepia,
        Self::BleachBypass,
        Self::MutedColors,
        Self::Vibrant,
        Self::Pastel,
        Self::GoldenHour,
        Self::ColdWinter,
        Self::NightCinematic,
        Self::CyberpunkNeon,
    ];

    pub const CINEMATIC: &'static [Self] = &[
        Self::TealOrange,
        Self::CinematicWarm,
        Self::CinematicCool,
        Self::Blockbuster,
        Self::FilmicNeutral,
    ];

    pub const FILM_EMULATION: &'static [Self] = &[
        Self::Kodak2383,
        Self::KodakPortra,
        Self::FujiFilm,
        Self::VintageFilm,
        Self::FadedFilm,
        Self::AnalogFilm,
    ];

    pub const CREATIVE: &'static [Self] = &[
        Self::MonochromeNoir,
        Self::Sepia,
        Self::BleachBypass,
        Self::MutedColors,
        Self::Vibrant,
        Self::Pastel,
        Self::GoldenHour,
        Self::ColdWinter,
        Self::NightCinematic,
        Self::CyberpunkNeon,
    ];

    pub fn by_category(category: LookCategory) -> &'static [Self] {
        match category {
            LookCategory::Cinematic => Self::CINEMATIC,
            LookCategory::FilmEmulation => Self::FILM_EMULATION,
            LookCategory::Creative => Self::CREATIVE,
        }
    }

    pub fn category(&self) -> Option<LookCategory> {
        match self {
            Self::None => None,
            Self::TealOrange
            | Self::CinematicWarm
            | Self::CinematicCool
            | Self::Blockbuster
            | Self::FilmicNeutral => Some(LookCategory::Cinematic),
            Self::Kodak2383
            | Self::KodakPortra
            | Self::FujiFilm
            | Self::VintageFilm
            | Self::FadedFilm
            | Self::AnalogFilm => Some(LookCategory::FilmEmulation),
            Self::MonochromeNoir
            | Self::Sepia
            | Self::BleachBypass
            | Self::MutedColors
            | Self::Vibrant
            | Self::Pastel
            | Self::GoldenHour
            | Self::ColdWinter
            | Self::NightCinematic
            | Self::CyberpunkNeon => Some(LookCategory::Creative),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "Yok (Doğal)",
            // Cinematic
            Self::TealOrange => "Teal & Orange (Sinematik)",
            Self::CinematicWarm => "Cinematic Warm (Sıcak Sinema)",
            Self::CinematicCool => "Cinematic Cool (Soğuk Sinema)",
            Self::Blockbuster => "Blockbuster (Yüksek Kontrast Aksiyon)",
            Self::FilmicNeutral => "Filmic Neutral (Doğal Sinema)",
            // Film Emulation
            Self::Kodak2383 => "Kodak 2383 (Baskı Film)",
            Self::KodakPortra => "Kodak Portra (Doğal Cilt Tonu)",
            Self::FujiFilm => "Fuji Film (Zümrüt Yeşil & Yumuşak)",
            Self::VintageFilm => "Vintage Film (Retro Sıcak)",
            Self::FadedFilm => "Faded Film (Mat Gölgeli Film)",
            Self::AnalogFilm => "Analog Film (Nostaljik Sıcak)",
            // Creative
            Self::MonochromeNoir => "Monochrome (Siyah & Beyaz)",
            Self::Sepia => "Sepia (Nostaljik Kahve)",
            Self::BleachBypass => "Bleach Bypass (Gümüş Tutma)",
            Self::MutedColors => "Muted Colors (Soluk Tonlar)",
            Self::Vibrant => "Vibrant (Canlı Renkler)",
            Self::Pastel => "Pastel (Yumuşak Pastel)",
            Self::GoldenHour => "Golden Hour (Altın Saat)",
            Self::ColdWinter => "Cold Winter (Soğuk Kış)",
            Self::NightCinematic => "Night Cinematic (Gece Sineması)",
            Self::CyberpunkNeon => "Cyberpunk Neon (Canlı Mor/Mavi)",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Self::None => "Natural",
            Self::TealOrange => "Teal & Orange",
            Self::CinematicWarm => "Cinematic Warm",
            Self::CinematicCool => "Cinematic Cool",
            Self::Blockbuster => "Blockbuster",
            Self::FilmicNeutral => "Filmic Neutral",
            Self::Kodak2383 => "Kodak 2383",
            Self::KodakPortra => "Kodak Portra",
            Self::FujiFilm => "Fuji Film",
            Self::VintageFilm => "Vintage Film",
            Self::FadedFilm => "Faded Film",
            Self::AnalogFilm => "Analog Film",
            Self::MonochromeNoir => "Monochrome",
            Self::Sepia => "Sepia",
            Self::BleachBypass => "Bleach Bypass",
            Self::MutedColors => "Muted Colors",
            Self::Vibrant => "Vibrant",
            Self::Pastel => "Pastel",
            Self::GoldenHour => "Golden Hour",
            Self::ColdWinter => "Cold Winter",
            Self::NightCinematic => "Night Cinematic",
            Self::CyberpunkNeon => "Cyberpunk Neon",
        }
    }

    /// Integer identifier passed to the OpenGL fragment shader (`u_lut_preset`).
    pub fn shader_id(&self) -> i32 {
        match self {
            Self::None => 0,
            Self::TealOrange => 1,
            Self::VintageFilm => 2,
            Self::MonochromeNoir => 3,
            Self::CyberpunkNeon => 4,
            Self::CinematicWarm => 5,
            Self::CinematicCool => 6,
            Self::Blockbuster => 7,
            Self::FilmicNeutral => 8,
            Self::Kodak2383 => 9,
            Self::KodakPortra => 10,
            Self::FujiFilm => 11,
            Self::FadedFilm => 12,
            Self::AnalogFilm => 13,
            Self::Sepia => 14,
            Self::BleachBypass => 15,
            Self::MutedColors => 16,
            Self::Vibrant => 17,
            Self::Pastel => 18,
            Self::GoldenHour => 19,
            Self::ColdWinter => 20,
            Self::NightCinematic => 21,
        }
    }

    /// Applies the analytical look transformation to normalized `rgb` in `[0.0, 1.0]`.
    /// This exact math is mirrored in the OpenGL fragment shader (`gpu_compositor.rs`).
    #[inline]
    pub fn apply_rgb(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        let mut r = r.clamp(0.0, 1.0);
        let mut g = g.clamp(0.0, 1.0);
        let mut b = b.clamp(0.0, 1.0);
        let luma = 0.299 * r + 0.587 * g + 0.114 * b;
        let sw = (1.0 - luma * 2.0).clamp(0.0, 1.0);
        let hw = ((luma - 0.5) * 2.0).clamp(0.0, 1.0);
        let mw = (1.0 - sw - hw).clamp(0.0, 1.0);

        match self {
            Self::None => {}
            Self::TealOrange => {
                if luma < 0.5 {
                    r *= 1.0 - 0.22 * sw;
                    g *= 1.0 + 0.08 * sw;
                    b *= 1.0 + 0.32 * sw;
                } else {
                    r *= 1.0 + 0.25 * hw;
                    g *= 1.0 + 0.08 * hw;
                    b *= 1.0 - 0.22 * hw;
                }
            }
            Self::CinematicWarm => {
                // Warm golden highlights, rich amber midtones, gently contrasted shadows
                r = r * (1.08 + 0.10 * hw) + 0.02 * mw;
                g = g * (1.02 + 0.04 * hw) + 0.01 * mw;
                b = b * (0.88 - 0.06 * hw) - 0.02 * sw;
                r = s_curve(r, 0.18);
                g = s_curve(g, 0.18);
                b = s_curve(b, 0.18);
            }
            Self::CinematicCool => {
                // Crisp steel-blue shadows and cyan-tinted midtones with clean highlights
                r = r * (0.90 - 0.08 * sw) + 0.02 * hw;
                g *= 1.01 + 0.03 * mw;
                b *= 1.14 + 0.12 * sw + 0.05 * mw;
                r = s_curve(r, 0.20);
                g = s_curve(g, 0.20);
                b = s_curve(b, 0.20);
            }
            Self::Blockbuster => {
                // High-impact cyan-teal shadows, punchy orange skin/highlights, strong S-curve
                r = r * (1.0 - 0.26 * sw + 0.28 * hw) + 0.03 * mw;
                g *= 1.0 + 0.06 * sw + 0.05 * hw;
                b *= 1.0 + 0.36 * sw - 0.26 * hw;
                r = s_curve(r.clamp(0.0, 1.0), 0.32);
                g = s_curve(g.clamp(0.0, 1.0), 0.32);
                b = s_curve(b.clamp(0.0, 1.0), 0.32);
                let l2 = 0.299 * r + 0.587 * g + 0.114 * b;
                r = l2 + 1.16 * (r - l2);
                g = l2 + 1.16 * (g - l2);
                b = l2 + 1.16 * (b - l2);
            }
            Self::FilmicNeutral => {
                // Soft highlight roll-off, natural toe, balanced 95% saturation
                r = filmic_tone(r);
                g = filmic_tone(g);
                b = filmic_tone(b);
                let l2 = 0.299 * r + 0.587 * g + 0.114 * b;
                r = l2 + 0.94 * (r - l2);
                g = l2 + 0.94 * (g - l2);
                b = l2 + 0.94 * (b - l2);
            }
            Self::Kodak2383 => {
                // Classic motion picture print stock: dense contrast, deep blue-teal shadows, warm golden-amber highlights
                let cr = s_curve(r, 0.36);
                let cg = s_curve(g, 0.36);
                let cb = s_curve(b, 0.36);
                r = cr * (1.0 - 0.12 * sw + 0.14 * hw) + 0.02 * mw;
                g = cg * (1.0 + 0.02 * sw + 0.06 * hw) - 0.01 * sw;
                b = cb * (1.0 + 0.18 * sw - 0.14 * hw) + 0.02 * sw;
            }
            Self::KodakPortra => {
                // Portrait film stock: lifted warm shadows, creamy skin midtones, gentle highlight compression
                r = 0.03 + 0.94 * r.powf(0.92) + 0.04 * mw + 0.03 * hw;
                g = 0.02 + 0.94 * g.powf(0.95) + 0.02 * mw;
                b = 0.02 + 0.91 * b.powf(0.98) - 0.03 * hw;
                let l2 = 0.299 * r + 0.587 * g + 0.114 * b;
                r = l2 + 1.06 * (r - l2);
                g = l2 + 1.06 * (g - l2);
                b = l2 + 1.06 * (b - l2);
            }
            Self::FujiFilm => {
                // Fuji Eterna / Provia inspired: cool emerald greens, subtle cyan shadows, clean highlights
                r *= 0.96 - 0.06 * sw + 0.04 * hw;
                g *= 1.05 + 0.06 * mw + 0.03 * sw;
                b *= 1.06 + 0.10 * sw - 0.03 * hw;
                r = s_curve(r.clamp(0.0, 1.0), 0.16);
                g = s_curve(g.clamp(0.0, 1.0), 0.16);
                b = s_curve(b.clamp(0.0, 1.0), 0.16);
            }
            Self::VintageFilm => {
                r = r * 1.06 + (14.0 / 255.0);
                g = g * 1.02 + (10.0 / 255.0);
                b = b * 0.90 + (6.0 / 255.0);
            }
            Self::FadedFilm => {
                // Matte lifted blacks, compressed highlights, nostalgic desaturated warmth
                let l_desat = 0.299 * r + 0.587 * g + 0.114 * b;
                let dr = l_desat + 0.78 * (r - l_desat);
                let dg = l_desat + 0.78 * (g - l_desat);
                let db = l_desat + 0.78 * (b - l_desat);
                r = 0.09 + dr * 0.82 + 0.02 * hw;
                g = 0.08 + dg * 0.81;
                b = 0.10 + db * 0.78 + 0.02 * sw;
            }
            Self::AnalogFilm => {
                // Cross-processed warm halation highlights, olive-green shadows, organic film curve
                r = s_curve(r, 0.20) * 1.08 + 0.03 * hw + 0.015;
                g = s_curve(g, 0.20) * 1.02 + 0.03 * sw + 0.01;
                b = s_curve(b, 0.20) * 0.89 + 0.04 * sw;
            }
            Self::MonochromeNoir => {
                let curved = if luma < 0.5 {
                    2.0 * luma * luma
                } else {
                    1.0 - 2.0 * (1.0 - luma) * (1.0 - luma)
                };
                r = curved;
                g = curved;
                b = curved;
            }
            Self::Sepia => {
                // Classic silver-gelatin warm brown toning
                let sr = (0.393 * r + 0.769 * g + 0.189 * b).min(1.0);
                let sg = (0.349 * r + 0.686 * g + 0.168 * b).min(1.0);
                let sb = (0.272 * r + 0.534 * g + 0.131 * b).min(1.0);
                r = r * 0.15 + sr * 0.85;
                g = g * 0.15 + sg * 0.85;
                b = b * 0.15 + sb * 0.85;
            }
            Self::BleachBypass => {
                // Silver retention: blends high-contrast monochrome silver layer with 45% saturation base
                let desat_r = luma + 0.42 * (r - luma);
                let desat_g = luma + 0.42 * (g - luma);
                let desat_b = luma + 0.42 * (b - luma);
                let silver = s_curve(luma, 0.55);
                r = 0.55 * desat_r + 0.45 * silver;
                g = 0.55 * desat_g + 0.45 * silver;
                b = 0.55 * desat_b + 0.45 * silver;
            }
            Self::MutedColors => {
                // Low-key editorial desaturation with soft shadow lift
                let mr = luma + 0.52 * (r - luma);
                let mg = luma + 0.52 * (g - luma);
                let mb = luma + 0.52 * (b - luma);
                r = 0.03 + mr * 0.92;
                g = 0.03 + mg * 0.93;
                b = 0.04 + mb * 0.94;
            }
            Self::Vibrant => {
                // Smart vibrance: boosts muted colors stronger than already-saturated skin tones
                let max_c = r.max(g).max(b);
                let min_c = r.min(g).min(b);
                let sat = max_c - min_c;
                let boost = 1.0 + 0.48 * (1.0 - sat * 0.65);
                r = luma + boost * (r - luma);
                g = luma + boost * (g - luma);
                b = luma + boost * (b - luma);
                r = s_curve(r.clamp(0.0, 1.0), 0.12);
                g = s_curve(g.clamp(0.0, 1.0), 0.12);
                b = s_curve(b.clamp(0.0, 1.0), 0.12);
            }
            Self::Pastel => {
                // Airy lifted shadows, soft contrast, delicate warm-pink / sky tint
                let pr = r.powf(0.85);
                let pg = g.powf(0.86);
                let pb = b.powf(0.84);
                let pl = 0.299 * pr + 0.587 * pg + 0.114 * pb;
                r = 0.08 + (pl + 0.82 * (pr - pl)) * 0.88 + 0.02 * hw;
                g = 0.07 + (pl + 0.82 * (pg - pl)) * 0.87 + 0.01 * mw;
                b = 0.09 + (pl + 0.82 * (pb - pl)) * 0.89 + 0.02 * sw;
            }
            Self::GoldenHour => {
                // Sunset golden amber glow
                r = r * (1.16 + 0.10 * hw) + 0.03 * mw;
                g = g * (1.03 + 0.04 * hw) + 0.01 * mw;
                b = b * (0.80 - 0.08 * hw) + 0.02 * sw;
                let l2 = 0.299 * r + 0.587 * g + 0.114 * b;
                r = l2 + 1.14 * (r - l2);
                g = l2 + 1.14 * (g - l2);
                b = l2 + 1.14 * (b - l2);
            }
            Self::ColdWinter => {
                // Crisp arctic blue-cyan atmosphere with bright snow highlights
                let wr = luma + 0.82 * (r - luma);
                let wg = luma + 0.88 * (g - luma);
                let wb = luma + 1.08 * (b - luma);
                r = wr * 0.88 + 0.02 * hw;
                g = wg * 1.02 + 0.03 * hw;
                b = wb * 1.18 + 0.04 * sw + 0.03 * hw;
            }
            Self::NightCinematic => {
                // Day-for-Night / Moonlight blue grade: darkened exposure, deep indigo shadows, cool cyan highlights
                let nr = r.powf(1.25) * 0.62;
                let ng = g.powf(1.18) * 0.78 + 0.02 * mw;
                let nb = b.powf(1.05) * 1.08 + 0.05 * sw + 0.04 * hw;
                r = nr;
                g = ng;
                b = nb;
            }
            Self::CyberpunkNeon => {
                r = (r * 1.22 + 8.0 / 255.0).clamp(0.0, 1.0);
                g = (g * 0.88).clamp(0.0, 1.0);
                b = (b * 1.32 + 16.0 / 255.0).clamp(0.0, 1.0);
                let luma_c = 0.299 * r + 0.587 * g + 0.114 * b;
                r = luma_c + 1.25 * (r - luma_c);
                g = luma_c + 1.25 * (g - luma_c);
                b = luma_c + 1.25 * (b - luma_c);
            }
        }

        [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]
    }

    /// Returns the equivalent FFmpeg video filter chain for this `LutPreset`.
    pub fn ffmpeg_filter(&self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::TealOrange => Some(
                "colorbalance=rs=0.15:gs=-0.05:bs=-0.25:rh=-0.2:gh=0.05:bh=0.25,eq=contrast=1.15:saturation=1.2",
            ),
            Self::CinematicWarm => Some(
                "colorbalance=rm=0.08:gm=0.02:bm=-0.08:rh=0.12:gh=0.04:bh=-0.10,eq=contrast=1.12:saturation=1.08",
            ),
            Self::CinematicCool => Some(
                "colorbalance=rs=-0.10:gs=0.02:bs=0.16:rm=-0.06:gm=0.02:bm=0.10,eq=contrast=1.12:saturation=0.96",
            ),
            Self::Blockbuster => Some(
                "colorbalance=rs=-0.22:gs=0.05:bs=0.28:rh=0.22:gh=0.04:bh=-0.22,eq=contrast=1.26:saturation=1.18",
            ),
            Self::FilmicNeutral => Some(
                "eq=contrast=1.06:brightness=0.01:saturation=0.94:gamma=1.04",
            ),
            Self::Kodak2383 => Some(
                "colorbalance=rs=-0.12:gs=0.01:bs=0.16:rm=0.03:gm=0.01:bm=-0.04:rh=0.14:gh=0.06:bh=-0.14,eq=contrast=1.24:saturation=1.08",
            ),
            Self::KodakPortra => Some(
                "colorbalance=rs=0.03:gs=0.02:bs=0.01:rm=0.05:gm=0.02:bm=-0.02:rh=0.04:gh=0.01:bh=-0.04,eq=contrast=0.96:brightness=0.02:saturation=1.06:gamma=1.06",
            ),
            Self::FujiFilm => Some(
                "colorbalance=rs=-0.05:gs=0.04:bs=0.08:rm=-0.02:gm=0.06:bm=0.02:rh=0.03:gh=0.02:bh=-0.03,eq=contrast=1.10:saturation=1.05",
            ),
            Self::VintageFilm => Some("curves=vintage,eq=contrast=0.95:saturation=0.85"),
            Self::FadedFilm => Some(
                "colorbalance=rs=0.06:gs=0.05:bs=0.08,eq=contrast=0.82:brightness=0.06:saturation=0.78",
            ),
            Self::AnalogFilm => Some(
                "colorbalance=rs=-0.02:gs=0.04:bs=0.05:rm=0.06:gm=0.02:bm=-0.06:rh=0.08:gh=0.03:bh=-0.08,eq=contrast=1.10:saturation=0.96",
            ),
            Self::MonochromeNoir => Some("hue=s=0,eq=contrast=1.45:brightness=-0.05"),
            Self::Sepia => Some(
                "colorchannelmixer=.393:.769:.189:0:.349:.686:.168:0:.272:.534:.131",
            ),
            Self::BleachBypass => Some(
                "eq=contrast=1.38:saturation=0.45:gamma=0.92",
            ),
            Self::MutedColors => Some(
                "eq=contrast=0.92:brightness=0.02:saturation=0.52",
            ),
            Self::Vibrant => Some(
                "eq=contrast=1.08:saturation=1.38",
            ),
            Self::Pastel => Some(
                "colorbalance=rs=0.05:gs=0.04:bs=0.06:rh=0.04:gh=0.02:bh=0.02,eq=contrast=0.88:brightness=0.06:saturation=0.82:gamma=1.14",
            ),
            Self::GoldenHour => Some(
                "colorbalance=rm=0.12:gm=0.04:bm=-0.12:rh=0.16:gh=0.06:bh=-0.16,eq=contrast=1.08:saturation=1.16",
            ),
            Self::ColdWinter => Some(
                "colorbalance=rs=-0.08:gs=0.02:bs=0.16:rm=-0.08:gm=0.02:bm=0.14:rh=-0.04:gh=0.03:bh=0.12,eq=contrast=1.06:saturation=0.88",
            ),
            Self::NightCinematic => Some(
                "colorbalance=rs=-0.16:gs=-0.04:bs=0.22:rm=-0.18:gm=-0.04:bm=0.20:rh=-0.12:gh=0.02:bh=0.18,eq=contrast=1.15:brightness=-0.12:saturation=0.78:gamma=0.84",
            ),
            Self::CyberpunkNeon => Some(
                "colorbalance=rm=0.2:gm=-0.1:bm=0.3:rh=-0.1:gh=-0.05:bh=0.25,eq=contrast=1.25:saturation=1.4",
            ),
        }
    }
}

/// Smooth S-curve contrast helper on `[0.0, 1.0]`, where `amount` is in `[0.0, 1.0]`.
#[inline]
pub fn s_curve(x: f32, amount: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let smooth = x * x * (3.0 - 2.0 * x);
    x + amount * (smooth - x)
}

/// Soft filmic shoulder/toe curve on `[0.0, 1.0]`.
#[inline]
pub fn filmic_tone(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let toe = x * x * (3.0 - 2.0 * x);
    let shoulder = 1.0 - (1.0 - x) * (1.0 - x);
    0.02 + 0.96 * (0.65 * toe + 0.35 * shoulder)
}

/// Computes `(wb_r, wb_g, wb_b)` multipliers from `temperature` (`-100..=100`) and `tint` (`-100..=100`).
#[inline]
pub fn white_balance_gains(temperature: f32, tint: f32) -> [f32; 3] {
    let temp = (temperature / 100.0).clamp(-1.0, 1.0);
    let tnt = (tint / 100.0).clamp(-1.0, 1.0);
    let r = (1.0 + temp * 0.28 + tnt * 0.10).max(0.1);
    let g = (1.0 - temp * 0.04 - tnt * 0.18).max(0.1);
    let b = (1.0 - temp * 0.28 + tnt * 0.10).max(0.1);
    [r, g, b]
}

/// Complete per-clip color grading parameters evaluated for a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorGradeParams {
    pub temperature: f32,
    pub tint: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub gamma: f32,
    pub saturation: f32,
    pub lut_preset: LutPreset,
}

impl Default for ColorGradeParams {
    fn default() -> Self {
        Self {
            temperature: 0.0,
            tint: 0.0,
            brightness: 0.0,
            contrast: 1.0,
            gamma: 1.0,
            saturation: 1.0,
            lut_preset: LutPreset::None,
        }
    }
}

/// Applies White Balance (Temperature/Tint), Contrast, Brightness, Gamma, Saturation, and Look Preset
/// to a normalized `[0.0, 1.0]` RGB pixel.
#[inline]
pub fn apply_color_grade_normalized(
    mut r: f32,
    mut g: f32,
    mut b: f32,
    params: &ColorGradeParams,
) -> [f32; 3] {
    // 1. White Balance (Temperature & Tint)
    if params.temperature.abs() > 1e-4 || params.tint.abs() > 1e-4 {
        let [wbr, wbg, wbb] = white_balance_gains(params.temperature, params.tint);
        r = (r * wbr).clamp(0.0, 1.0);
        g = (g * wbg).clamp(0.0, 1.0);
        b = (b * wbb).clamp(0.0, 1.0);
    }

    // 2. Contrast & Brightness
    let mid = 128.0 / 255.0;
    r = (params.contrast * (r - mid) + mid + params.brightness).clamp(0.0, 1.0);
    g = (params.contrast * (g - mid) + mid + params.brightness).clamp(0.0, 1.0);
    b = (params.contrast * (b - mid) + mid + params.brightness).clamp(0.0, 1.0);

    // 3. Gamma (midtone power curve matching FFmpeg eq=gamma)
    if (params.gamma - 1.0).abs() > 1e-4 {
        let inv_gamma = 1.0 / params.gamma.max(0.1);
        r = r.max(0.0).powf(inv_gamma).clamp(0.0, 1.0);
        g = g.max(0.0).powf(inv_gamma).clamp(0.0, 1.0);
        b = b.max(0.0).powf(inv_gamma).clamp(0.0, 1.0);
    }

    // 4. Saturation
    if (params.saturation - 1.0).abs() > 1e-4 {
        let luma = 0.299 * r + 0.587 * g + 0.114 * b;
        r = (luma + params.saturation * (r - luma)).clamp(0.0, 1.0);
        g = (luma + params.saturation * (g - luma)).clamp(0.0, 1.0);
        b = (luma + params.saturation * (b - luma)).clamp(0.0, 1.0);
    }

    // 5. Creative Color Look Preset
    if params.lut_preset != LutPreset::None {
        [r, g, b] = params.lut_preset.apply_rgb(r, g, b);
    }

    [r, g, b]
}

/// Analyzes an RGBA8 image buffer to compute corrective `(temperature, tint)` values
/// in `[-100.0, 100.0]` using a midtone-weighted Gray-World + chromatic cast estimator.
///
/// - If the image has a cool/blue cast (`B > R`), returns a positive `temperature` (warms the image).
/// - If the image has a warm/orange cast (`R > B`), returns a negative `temperature` (cools the image).
/// - If the image has a green cast (`G > avg(R,B)`), returns a positive `tint` (shifts toward magenta).
/// - If the image has a magenta cast (`G < avg(R,B)`), returns a negative `tint` (shifts toward green).
pub fn analyze_auto_white_balance_rgba(rgba: &[u8], width: u32, height: u32) -> (f64, f64) {
    let expected_len = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4);
    if rgba.len() < expected_len || expected_len == 0 {
        return (0.0, 0.0);
    }

    // Subsample large frames for sub-millisecond performance
    let total_pixels = (width as usize) * (height as usize);
    let step_px = (total_pixels / 16384).max(1);

    let mut sum_r = 0.0_f64;
    let mut sum_g = 0.0_f64;
    let mut sum_b = 0.0_f64;
    let mut weight_sum = 0.0_f64;

    for px_idx in (0..total_pixels).step_by(step_px) {
        let i = px_idx * 4;
        let a = rgba[i + 3];
        if a < 32 {
            continue;
        }
        let r = rgba[i] as f64;
        let g = rgba[i + 1] as f64;
        let b = rgba[i + 2] as f64;

        let max_c = r.max(g).max(b);
        let min_c = r.min(g).min(b);
        // Exclude clipped highlights and deep shadow noise
        if max_c > 246.0 || max_c < 16.0 {
            continue;
        }

        let luma = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
        // Midtone bell weight (peaks at luma = 0.5)
        let luma_w = (4.0 * luma * (1.0 - luma)).max(0.05);
        // Favor near-neutral / moderately saturated pixels over extreme neon colors
        let chroma = (max_c - min_c) / 255.0;
        let neutrality_w = (1.0 - chroma * 0.75).max(0.15);
        let w = luma_w * neutrality_w;

        sum_r += r * w;
        sum_g += g * w;
        sum_b += b * w;
        weight_sum += w;
    }

    if weight_sum <= 1e-6 {
        return (0.0, 0.0);
    }

    let mean_r = (sum_r / weight_sum).max(1.0);
    let mean_g = (sum_g / weight_sum).max(1.0);
    let mean_b = (sum_b / weight_sum).max(1.0);

    // Log-ratio chromaticity imbalance:
    // If mean_b > mean_r (blue cast), ln(mean_b / mean_r) > 0 -> positive temperature needed
    let rb_log = (mean_b / mean_r).ln();
    // Target: r * (1 + 0.28 * t) / (b * (1 - 0.28 * t)) ≈ 1 => 0.56 * t ≈ ln(b / r)
    let raw_temp = (rb_log / 0.56) * 100.0;

    // Green vs Red-Blue geometric mean:
    // If mean_g > sqrt(mean_r * mean_b) (green cast), positive tint (magenta) needed
    let rb_geom = (mean_r * mean_b).sqrt().max(1.0);
    let gm_log = (mean_g / rb_geom).ln();
    let raw_tint = (gm_log / 0.28) * 100.0;

    // Apply conservative damping (0.85) so Auto WB feels natural and never over-corrects
    let temp = ((raw_temp * 0.85).clamp(-100.0, 100.0) * 2.0).round() / 2.0;
    let tint = ((raw_tint * 0.85).clamp(-100.0, 100.0) * 2.0).round() / 2.0;

    (temp, tint)
}

/// Converts normalized RGB `[0.0, 1.0]` to `(hue_deg [0..360], sat [0..1], luma [0..1])`.
#[inline]
pub fn rgb_to_hsl_luma(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let rf = r.clamp(0.0, 1.0);
    let gf = g.clamp(0.0, 1.0);
    let bf = b.clamp(0.0, 1.0);
    let max_c = rf.max(gf).max(bf);
    let min_c = rf.min(gf).min(bf);
    let delta = max_c - min_c;
    let luma = 0.2126 * rf + 0.7152 * gf + 0.0722 * bf;
    let hue = if delta <= 1e-5 {
        0.0
    } else if (max_c - rf).abs() < 1e-5 {
        60.0 * (((gf - bf) / delta).rem_euclid(6.0))
    } else if (max_c - gf).abs() < 1e-5 {
        60.0 * (((bf - rf) / delta) + 2.0)
    } else {
        60.0 * (((rf - gf) / delta) + 4.0)
    };
    let sat = if max_c <= 1e-5 { 0.0 } else { delta / max_c };
    (hue, sat, luma)
}

/// Rotates hue (in degrees) and scales saturation/luma on a normalized RGB triplet.
#[inline]
pub fn shift_rgb_hue_sat_luma(
    r: f32,
    g: f32,
    b: f32,
    hue_shift_deg: f32,
    sat_mult: f32,
    luma_add: f32,
) -> [f32; 3] {
    let mut rf = r;
    let mut gf = g;
    let mut bf = b;
    if hue_shift_deg.abs() > 1e-3 {
        let rad = hue_shift_deg.to_radians();
        let cos_a = rad.cos();
        let sin_a = rad.sin();
        let s1 = (1.0 - cos_a) / 3.0;
        let s2 = (1.0 / 3.0_f32).sqrt() * sin_a;
        let m00 = cos_a + s1;
        let m01 = s1 - s2;
        let m02 = s1 + s2;
        let m10 = s1 + s2;
        let m11 = cos_a + s1;
        let m12 = s1 - s2;
        let m20 = s1 - s2;
        let m21 = s1 + s2;
        let m22 = cos_a + s1;
        let nr = rf * m00 + gf * m01 + bf * m02;
        let ng = rf * m10 + gf * m11 + bf * m12;
        let nb = rf * m20 + gf * m21 + bf * m22;
        rf = nr;
        gf = ng;
        bf = nb;
    }
    let luma = 0.2126 * rf + 0.7152 * gf + 0.0722 * bf;
    rf = luma + (rf - luma) * sat_mult + luma_add;
    gf = luma + (gf - luma) * sat_mult + luma_add;
    bf = luma + (bf - luma) * sat_mult + luma_add;
    [rf.clamp(0.0, 1.0), gf.clamp(0.0, 1.0), bf.clamp(0.0, 1.0)]
}

fn default_curve_points() -> Vec<[f32; 2]> {
    vec![[0.0, 0.0], [1.0, 1.0]]
}

/// Custom Spline Curves for Master Luma, Red, Green, and Blue channels (DaVinci Custom Curves).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomCurvesConfig {
    #[serde(default = "default_curve_points")]
    pub master: Vec<[f32; 2]>,
    #[serde(default = "default_curve_points")]
    pub red: Vec<[f32; 2]>,
    #[serde(default = "default_curve_points")]
    pub green: Vec<[f32; 2]>,
    #[serde(default = "default_curve_points")]
    pub blue: Vec<[f32; 2]>,
}

impl Default for CustomCurvesConfig {
    fn default() -> Self {
        Self {
            master: default_curve_points(),
            red: default_curve_points(),
            green: default_curve_points(),
            blue: default_curve_points(),
        }
    }
}

impl CustomCurvesConfig {
    pub fn is_identity_channel(pts: &[[f32; 2]]) -> bool {
        pts.len() == 2
            && (pts[0][0] - 0.0).abs() < 1e-4
            && (pts[0][1] - 0.0).abs() < 1e-4
            && (pts[1][0] - 1.0).abs() < 1e-4
            && (pts[1][1] - 1.0).abs() < 1e-4
    }

    pub fn is_identity(&self) -> bool {
        Self::is_identity_channel(&self.master)
            && Self::is_identity_channel(&self.red)
            && Self::is_identity_channel(&self.green)
            && Self::is_identity_channel(&self.blue)
    }

    /// Evaluates a sorted control-point curve at `x` in `[0.0, 1.0]` using smooth Hermite interpolation.
    #[inline]
    pub fn eval_curve(pts: &[[f32; 2]], x: f32) -> f32 {
        if pts.is_empty() {
            return x.clamp(0.0, 1.0);
        }
        let x = x.clamp(0.0, 1.0);
        if pts.len() == 1 {
            return pts[0][1].clamp(0.0, 1.0);
        }
        if x <= pts[0][0] {
            return pts[0][1].clamp(0.0, 1.0);
        }
        let last = pts.len() - 1;
        if x >= pts[last][0] {
            return pts[last][1].clamp(0.0, 1.0);
        }

        // Find segment i where pts[i][0] <= x <= pts[i+1][0]
        let mut idx = 0;
        for i in 0..last {
            if x >= pts[i][0] && x <= pts[i + 1][0] {
                idx = i;
                break;
            }
        }
        let p1 = pts[idx];
        let p2 = pts[idx + 1];
        let dx = (p2[0] - p1[0]).max(1e-5);
        let t = ((x - p1[0]) / dx).clamp(0.0, 1.0);

        if pts.len() == 2 {
            return (p1[1] + t * (p2[1] - p1[1])).clamp(0.0, 1.0);
        }

        // Secant slopes with Fritsch-Carlson clamping to prevent overshoot
        let sec = (p2[1] - p1[1]) / dx;
        let m1 = if idx > 0 {
            let p0 = pts[idx - 1];
            let s0 = (p1[1] - p0[1]) / (p1[0] - p0[0]).max(1e-5);
            if s0 * sec <= 0.0 {
                0.0
            } else {
                0.5 * (s0 + sec)
            }
        } else {
            sec
        };
        let m2 = if idx + 2 <= last {
            let p3 = pts[idx + 2];
            let s2 = (p3[1] - p2[1]) / (p3[0] - p2[0]).max(1e-5);
            if sec * s2 <= 0.0 {
                0.0
            } else {
                0.5 * (sec + s2)
            }
        } else {
            sec
        };

        let t2 = t * t;
        let t3 = t2 * t;
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        let y = h00 * p1[1] + h10 * dx * m1 + h01 * p2[1] + h11 * dx * m2;
        y.clamp(0.0, 1.0)
    }

    #[inline]
    pub fn apply_rgb(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        let mr = Self::eval_curve(&self.master, r);
        let mg = Self::eval_curve(&self.master, g);
        let mb = Self::eval_curve(&self.master, b);
        [
            Self::eval_curve(&self.red, mr),
            Self::eval_curve(&self.green, mg),
            Self::eval_curve(&self.blue, mb),
        ]
    }

    /// Generates an FFmpeg `curves=...` filter string if any curve is non-identity.
    pub fn ffmpeg_filter(&self) -> Option<String> {
        if self.is_identity() {
            return None;
        }
        let fmt_pts = |pts: &[[f32; 2]]| -> String {
            let mut sorted = pts.to_vec();
            sorted.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal));
            sorted.dedup_by(|a, b| (a[0] - b[0]).abs() < 0.01);
            sorted
                .iter()
                .map(|p| format!("{:.3}/{:.3}", p[0].clamp(0.0, 1.0), p[1].clamp(0.0, 1.0)))
                .collect::<Vec<_>>()
                .join(" ")
        };
        let mut parts = Vec::new();
        if !Self::is_identity_channel(&self.master) {
            parts.push(format!("master='{}'", fmt_pts(&self.master)));
        }
        if !Self::is_identity_channel(&self.red) {
            parts.push(format!("r='{}'", fmt_pts(&self.red)));
        }
        if !Self::is_identity_channel(&self.green) {
            parts.push(format!("g='{}'", fmt_pts(&self.green)));
        }
        if !Self::is_identity_channel(&self.blue) {
            parts.push(format!("b='{}'", fmt_pts(&self.blue)));
        }
        if parts.is_empty() {
            None
        } else {
            Some(format!("curves={}", parts.join(":")))
        }
    }
}

/// DaVinci-style 6-vector HSL Curves (`Hue vs Hue`, `Hue vs Sat`, `Hue vs Luma`, `Luma vs Sat`).
/// Color vectors correspond to `[Red 0°, Yellow 60°, Green 120°, Cyan 180°, Blue 240°, Magenta 300°]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HslCurvesConfig {
    /// Hue shift in degrees `[-60.0..=60.0]` for `[R, Y, G, C, B, M]`.
    #[serde(default)]
    pub hue_vs_hue: [f32; 6],
    /// Saturation delta `[-1.0..=1.0]` for `[R, Y, G, C, B, M]`.
    #[serde(default)]
    pub hue_vs_sat: [f32; 6],
    /// Luminance delta `[-0.5..=0.5]` for `[R, Y, G, C, B, M]`.
    #[serde(default)]
    pub hue_vs_luma: [f32; 6],
    /// Saturation delta `[-1.0..=1.0]` across 5 luma zones `[Shadows, DarkMid, Mid, BrightMid, Highlights]`.
    #[serde(default)]
    pub luma_vs_sat: [f32; 5],
}

impl Default for HslCurvesConfig {
    fn default() -> Self {
        Self {
            hue_vs_hue: [0.0; 6],
            hue_vs_sat: [0.0; 6],
            hue_vs_luma: [0.0; 6],
            luma_vs_sat: [0.0; 5],
        }
    }
}

impl HslCurvesConfig {
    pub fn is_identity(&self) -> bool {
        self.hue_vs_hue.iter().all(|v| v.abs() < 1e-4)
            && self.hue_vs_sat.iter().all(|v| v.abs() < 1e-4)
            && self.hue_vs_luma.iter().all(|v| v.abs() < 1e-4)
            && self.luma_vs_sat.iter().all(|v| v.abs() < 1e-4)
    }

    #[inline]
    fn sample_hue_ring(values: &[f32; 6], hue_deg: f32) -> f32 {
        let h = hue_deg.rem_euclid(360.0) / 60.0;
        let idx0 = (h.floor() as usize) % 6;
        let idx1 = (idx0 + 1) % 6;
        let frac = h - h.floor();
        let smooth = frac * frac * (3.0 - 2.0 * frac);
        values[idx0] * (1.0 - smooth) + values[idx1] * smooth
    }

    #[inline]
    fn sample_luma_5(values: &[f32; 5], luma: f32) -> f32 {
        let pos = (luma.clamp(0.0, 1.0)) * 4.0;
        let idx0 = (pos.floor() as usize).min(4);
        let idx1 = (idx0 + 1).min(4);
        let frac = pos - pos.floor();
        let smooth = frac * frac * (3.0 - 2.0 * frac);
        values[idx0] * (1.0 - smooth) + values[idx1] * smooth
    }

    #[inline]
    pub fn apply_rgb(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        if self.is_identity() {
            return [r, g, b];
        }
        let (hue, sat, luma) = rgb_to_hsl_luma(r, g, b);
        // Weight hue-based adjustments by chroma saturation so neutral greys aren't tinted
        let chroma_weight = (sat * 3.0).clamp(0.0, 1.0);
        let d_hue = Self::sample_hue_ring(&self.hue_vs_hue, hue) * chroma_weight;
        let d_sat_hue = Self::sample_hue_ring(&self.hue_vs_sat, hue) * chroma_weight;
        let d_luma = Self::sample_hue_ring(&self.hue_vs_luma, hue) * chroma_weight * 0.45;
        let d_sat_luma = Self::sample_luma_5(&self.luma_vs_sat, luma);

        let sat_mult = (1.0 + d_sat_hue + d_sat_luma).max(0.0);
        shift_rgb_hue_sat_luma(r, g, b, d_hue, sat_mult, d_luma)
    }
}

/// 3D HSL Qualifier for secondary selective color grading and matte visualization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HslQualifierConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Center hue in degrees `[0.0, 360.0]`.
    #[serde(default = "default_qual_hue_center")]
    pub hue_center: f32,
    /// Half-width in degrees `[5.0, 180.0]`.
    #[serde(default = "default_qual_hue_width")]
    pub hue_width: f32,
    /// Feather softness in degrees `[1.0, 60.0]`.
    #[serde(default = "default_qual_hue_softness")]
    pub hue_softness: f32,
    #[serde(default = "default_qual_sat_min")]
    pub sat_min: f32,
    #[serde(default = "default_qual_one")]
    pub sat_max: f32,
    #[serde(default = "default_qual_luma_min")]
    pub luma_min: f32,
    #[serde(default = "default_qual_one")]
    pub luma_max: f32,
    #[serde(default)]
    pub invert: bool,
    /// When true, displays non-qualified pixels as desaturated monochrome matte in the monitor.
    #[serde(default)]
    pub show_matte: bool,
    /// Secondary hue shift in degrees `[-180..=180]`.
    #[serde(default)]
    pub hue_shift: f32,
    /// Secondary saturation multiplier `[0.0..=2.5]`.
    #[serde(default = "default_qual_one")]
    pub sat_scale: f32,
    /// Secondary luminance offset `[-0.5..=0.5]`.
    #[serde(default)]
    pub luma_offset: f32,
}

fn default_qual_hue_center() -> f32 {
    25.0 // Skin-tone / warm orange default
}
fn default_qual_hue_width() -> f32 {
    35.0
}
fn default_qual_hue_softness() -> f32 {
    15.0
}
fn default_qual_sat_min() -> f32 {
    0.12
}
fn default_qual_luma_min() -> f32 {
    0.05
}
fn default_qual_one() -> f32 {
    1.0
}

impl Default for HslQualifierConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            hue_center: default_qual_hue_center(),
            hue_width: default_qual_hue_width(),
            hue_softness: default_qual_hue_softness(),
            sat_min: default_qual_sat_min(),
            sat_max: 1.0,
            luma_min: default_qual_luma_min(),
            luma_max: 1.0,
            invert: false,
            show_matte: false,
            hue_shift: 0.0,
            sat_scale: 1.0,
            luma_offset: 0.0,
        }
    }
}

impl HslQualifierConfig {
    /// Creates an active HSL Qualifier centered on a sampled 8-bit RGB pixel (Eyedropper pick).
    pub fn from_rgb_sample(rgb: [u8; 3]) -> Self {
        let (hue, sat, luma) = rgb_to_hsl_luma(
            rgb[0] as f32 / 255.0,
            rgb[1] as f32 / 255.0,
            rgb[2] as f32 / 255.0,
        );
        Self {
            enabled: true,
            hue_center: hue,
            hue_width: 28.0,
            hue_softness: 14.0,
            sat_min: (sat - 0.25).max(0.05),
            sat_max: (sat + 0.35).min(1.0),
            luma_min: (luma - 0.30).max(0.0),
            luma_max: (luma + 0.35).min(1.0),
            ..Default::default()
        }
    }

    /// Computes the `[0.0, 1.0]` selection matte for a normalized RGB pixel.
    #[inline]
    pub fn compute_matte(&self, r: f32, g: f32, b: f32) -> f32 {
        let (hue, sat, luma) = rgb_to_hsl_luma(r, g, b);
        // Shortest angular distance on 360° circle
        let mut dh = (hue - self.hue_center).abs() % 360.0;
        if dh > 180.0 {
            dh = 360.0 - dh;
        }
        let hw = self.hue_width.max(1.0);
        let hs = self.hue_softness.max(1.0);
        let m_hue = if dh <= hw {
            1.0
        } else if dh >= hw + hs {
            0.0
        } else {
            1.0 - (dh - hw) / hs
        };

        let soft_sl = 0.08_f32;
        let m_sat_lo = ((sat - (self.sat_min - soft_sl)) / soft_sl).clamp(0.0, 1.0);
        let m_sat_hi = (((self.sat_max + soft_sl) - sat) / soft_sl).clamp(0.0, 1.0);
        let m_lum_lo = ((luma - (self.luma_min - soft_sl)) / soft_sl).clamp(0.0, 1.0);
        let m_lum_hi = (((self.luma_max + soft_sl) - luma) / soft_sl).clamp(0.0, 1.0);

        let raw = (m_hue * m_sat_lo * m_sat_hi * m_lum_lo * m_lum_hi).clamp(0.0, 1.0);
        if self.invert {
            1.0 - raw
        } else {
            raw
        }
    }

    #[inline]
    pub fn apply_rgb(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        if !self.enabled {
            return [r, g, b];
        }
        let matte = self.compute_matte(r, g, b);
        let graded =
            shift_rgb_hue_sat_luma(r, g, b, self.hue_shift, self.sat_scale, self.luma_offset);
        if self.show_matte {
            // Highlight Matte mode: qualified pixels show graded color, unselected show dimmed greyscale
            let grey = (0.2126 * r + 0.7152 * g + 0.0722 * b) * 0.35;
            [
                grey * (1.0 - matte) + graded[0] * matte,
                grey * (1.0 - matte) + graded[1] * matte,
                grey * (1.0 - matte) + graded[2] * matte,
            ]
        } else {
            [
                r * (1.0 - matte) + graded[0] * matte,
                g * (1.0 - matte) + graded[1] * matte,
                b * (1.0 - matte) + graded[2] * matte,
            ]
        }
    }
}

/// Computes corrective `(temperature, tint)` values in `[-100.0, 100.0]` from a single
/// 8-bit RGB pixel picked with the White Balance Eyedropper so that the sampled pixel becomes neutral gray.
pub fn compute_wb_from_neutral_rgb(r: u8, g: u8, b: u8) -> (f64, f64) {
    let rf = (r as f64).max(4.0);
    let gf = (g as f64).max(4.0);
    let bf = (b as f64).max(4.0);

    // Blue > Red => positive temperature (warms); Red > Blue => negative temperature (cools)
    let rb_log = (bf / rf).ln();
    let raw_temp = (rb_log / 0.56) * 100.0;

    // Green > sqrt(R*B) => positive tint (shifts toward magenta)
    let rb_geom = (rf * bf).sqrt().max(4.0);
    let gm_log = (gf / rb_geom).ln();
    let raw_tint = (gm_log / 0.28) * 100.0;

    let temp = (raw_temp.clamp(-100.0, 100.0) * 2.0).round() / 2.0;
    let tint = (raw_tint.clamp(-100.0, 100.0) * 2.0).round() / 2.0;
    (temp, tint)
}

/// Bakes a clip's complete color grading pipeline (Camera Log profile, 3-Point Color Wheels,
/// White Balance, Contrast/Brightness/Gamma/Saturation, Creative Look Preset, Custom RGB/Luma Curves,
/// 6-Vector HSL Curves, and 3D HSL Qualifier) into a standard Adobe/Resolve `.cube` 3D LUT string.
pub fn bake_clip_grade_to_cube(clip: &crate::clip::Clip, size: usize) -> String {
    use std::fmt::Write;
    let n = size.clamp(17, 65);
    let title = if clip.name.trim().is_empty() {
        "triM Color Grade".to_string()
    } else {
        clip.name
            .chars()
            .filter(|c| *c != '"' && *c != '\n' && *c != '\r')
            .collect::<String>()
    };
    let mut out = String::with_capacity(n * n * n * 28 + 160);
    let _ = writeln!(out, "# Generated by triM. Color Engine");
    let _ = writeln!(out, "TITLE \"{title}\"");
    let _ = writeln!(out, "LUT_3D_SIZE {n}");
    let _ = writeln!(out, "DOMAIN_MIN 0.0 0.0 0.0");
    let _ = writeln!(out, "DOMAIN_MAX 1.0 1.0 1.0");

    let denom = (n - 1) as f32;
    for b_idx in 0..n {
        let b_in = b_idx as f32 / denom;
        for g_idx in 0..n {
            let g_in = g_idx as f32 / denom;
            for r_idx in 0..n {
                let r_in = r_idx as f32 / denom;
                let [ro, go, bo] = clip.evaluate_color_pipeline_rgb(r_in, g_in, b_in);
                let _ = writeln!(
                    out,
                    "{:.6} {:.6} {:.6}",
                    ro.clamp(0.0, 1.0),
                    go.clamp(0.0, 1.0),
                    bo.clamp(0.0, 1.0)
                );
            }
        }
    }
    out
}

/// Computes automatic Shot Match grading parameters `(brightness, contrast, saturation, temperature, tint)`
/// to match `src_rgba` statistics to `ref_rgba` statistics.
pub fn analyze_shot_match_rgba(
    src_rgba: &[u8],
    src_w: u32,
    src_h: u32,
    ref_rgba: &[u8],
    ref_w: u32,
    ref_h: u32,
) -> (f64, f64, f64, f64, f64) {
    fn frame_stats(rgba: &[u8], w: u32, h: u32) -> (f64, f64, f64, f64, f64) {
        let total = (w as usize) * (h as usize);
        if total == 0 || rgba.len() < total * 4 {
            return (0.5, 0.2, 0.25, 128.0, 128.0);
        }
        let step = (total / 8192).max(1);
        let mut sum_l = 0.0;
        let mut sum_l2 = 0.0;
        let mut sum_sat = 0.0;
        let mut sum_r = 0.0;
        let mut sum_b = 0.0;
        let mut cnt = 0.0;
        for i in (0..total).step_by(step) {
            let idx = i * 4;
            let r = rgba[idx] as f64 / 255.0;
            let g = rgba[idx + 1] as f64 / 255.0;
            let b = rgba[idx + 2] as f64 / 255.0;
            let l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            let sat = r.max(g).max(b) - r.min(g).min(b);
            sum_l += l;
            sum_l2 += l * l;
            sum_sat += sat;
            sum_r += r;
            sum_b += b;
            cnt += 1.0;
        }
        if cnt <= 0.0 {
            return (0.5, 0.2, 0.25, 0.5, 0.5);
        }
        let mean_l = sum_l / cnt;
        let var_l = (sum_l2 / cnt - mean_l * mean_l).max(1e-4);
        (
            mean_l,
            var_l.sqrt(),
            (sum_sat / cnt).max(0.02),
            sum_r / cnt,
            sum_b / cnt,
        )
    }

    let (src_l, src_std, src_sat, src_r, src_b) = frame_stats(src_rgba, src_w, src_h);
    let (ref_l, ref_std, ref_sat, ref_r, ref_b) = frame_stats(ref_rgba, ref_w, ref_h);

    let brightness = ((ref_l - src_l) * 0.85).clamp(-0.4, 0.4);
    let contrast = ((ref_std / src_std.max(0.05)).clamp(0.6, 1.6) * 100.0).round() / 100.0;
    let saturation = ((ref_sat / src_sat.max(0.05)).clamp(0.5, 1.8) * 100.0).round() / 100.0;
    let d_rb = ((ref_r - ref_b) - (src_r - src_b)) * 140.0;
    let temperature = d_rb.clamp(-80.0, 80.0).round();
    let tint = 0.0;

    (brightness, contrast, saturation, temperature, tint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_white_balance_detects_blue_and_warm_casts() {
        // 1. Neutral gray image -> (0.0, 0.0)
        let neutral = [128u8, 128, 128, 255].repeat(64 * 64);
        let (t_n, tint_n) = analyze_auto_white_balance_rgba(&neutral, 64, 64);
        assert!(t_n.abs() < 1.0, "neutral temp should be ~0, got {t_n}");
        assert!(
            tint_n.abs() < 1.0,
            "neutral tint should be ~0, got {tint_n}"
        );

        // 2. Cool/Bluish image (R=95, G=125, B=175) -> positive temperature (warm correction)
        let cool_img = [95u8, 125, 175, 255].repeat(64 * 64);
        let (t_cool, _) = analyze_auto_white_balance_rgba(&cool_img, 64, 64);
        assert!(
            t_cool > 25.0,
            "cool image should yield positive warming temperature, got {t_cool}"
        );

        // 3. Warm/Orange tungsten cast (R=185, G=130, B=85) -> negative temperature (cool correction)
        let warm_img = [185u8, 130, 85, 255].repeat(64 * 64);
        let (t_warm, _) = analyze_auto_white_balance_rgba(&warm_img, 64, 64);
        assert!(
            t_warm < -25.0,
            "warm image should yield negative cooling temperature, got {t_warm}"
        );

        // 4. Green fluorescent cast (R=110, G=165, B=110) -> positive magenta tint
        let green_img = [110u8, 165, 110, 255].repeat(64 * 64);
        let (_, tint_green) = analyze_auto_white_balance_rgba(&green_img, 64, 64);
        assert!(
            tint_green > 20.0,
            "greenish image should yield positive magenta tint, got {tint_green}"
        );
    }

    #[test]
    fn test_all_lut_presets_produce_distinct_valid_colors() {
        let shadow_sample = [0.25_f32, 0.30, 0.35];
        let highlight_sample = [0.75_f32, 0.68, 0.58];
        for preset in LutPreset::ALL {
            let out_s = preset.apply_rgb(shadow_sample[0], shadow_sample[1], shadow_sample[2]);
            let out_h = preset.apply_rgb(
                highlight_sample[0],
                highlight_sample[1],
                highlight_sample[2],
            );
            for c in out_s.into_iter().chain(out_h) {
                assert!((0.0..=1.0).contains(&c));
            }
            if *preset != LutPreset::None {
                let diff_s = (out_s[0] - shadow_sample[0]).abs()
                    + (out_s[1] - shadow_sample[1]).abs()
                    + (out_s[2] - shadow_sample[2]).abs();
                let diff_h = (out_h[0] - highlight_sample[0]).abs()
                    + (out_h[1] - highlight_sample[1]).abs()
                    + (out_h[2] - highlight_sample[2]).abs();
                let total_diff = diff_s + diff_h;
                assert!(
                    total_diff > 0.02,
                    "Preset {:?} must meaningfully transform pixel values (diff={})",
                    preset,
                    total_diff
                );
                assert!(
                    preset.ffmpeg_filter().is_some(),
                    "Preset {:?} must have an FFmpeg export filter",
                    preset
                );
            }
        }
    }

    #[test]
    fn test_brightness_contrast_presets_and_wb_presets() {
        for wb in WhiteBalancePreset::ALL {
            if *wb != WhiteBalancePreset::Auto {
                let (temp, tint) = wb
                    .temperature_tint()
                    .expect("Static WB preset must have temp/tint");
                assert!((-100.0..=100.0).contains(&temp));
                assert!((-100.0..=100.0).contains(&tint));
            }
        }

        for bc in BrightnessContrastPreset::ALL {
            let (b, c, g) = bc.params();
            assert!((-1.0..=1.0).contains(&b));
            assert!((0.1..=3.0).contains(&c));
            assert!((0.2..=3.0).contains(&g));
        }
    }

    #[test]
    fn test_camera_log_profiles_and_wb_eyedropper_and_cube_baking() {
        for profile in CameraLogProfile::ALL {
            let [rs, gs, bs] = profile.apply_rgb(0.25, 0.25, 0.25);
            let [rh, gh, bh] = profile.apply_rgb(0.75, 0.75, 0.75);
            for v in [rs, gs, bs, rh, gh, bh] {
                assert!((0.0..=1.0).contains(&v));
            }
            if *profile != CameraLogProfile::Rec709 {
                let diff = (rs - 0.25).abs() + (rh - 0.75).abs();
                assert!(
                    diff > 0.05,
                    "Log profile {:?} should expand contrast between shadows and highlights (diff={})",
                    profile,
                    diff
                );
            }
        }

        let (temp_n, tint_n) = compute_wb_from_neutral_rgb(128, 128, 128);
        assert!(temp_n.abs() < 0.1 && tint_n.abs() < 0.1);
        let (temp_blue, _) = compute_wb_from_neutral_rgb(90, 125, 175);
        assert!(
            temp_blue > 25.0,
            "blue pixel should require positive warming temp"
        );
        let (temp_warm, _) = compute_wb_from_neutral_rgb(185, 125, 85);
        assert!(
            temp_warm < -25.0,
            "warm pixel should require negative cooling temp"
        );

        let mut clip = crate::clip::Clip::new("m1", 0.0, 5.0);
        clip.name = "Noir Scene 01".to_string();
        clip.camera_log_profile = CameraLogProfile::SLog3;
        clip.temperature = 18.0;
        clip.contrast = 1.2;
        clip.lut_preset = LutPreset::Kodak2383;
        assert!(clip.needs_baked_cube_lut());

        let cube = bake_clip_grade_to_cube(&clip, 17);
        assert!(cube.contains("TITLE \"Noir Scene 01\""));
        assert!(cube.contains("LUT_3D_SIZE 17"));
        let data_lines = cube
            .lines()
            .filter(|l| {
                !l.starts_with('#')
                    && !l.starts_with("TITLE")
                    && !l.starts_with("LUT_3D_SIZE")
                    && !l.starts_with("DOMAIN_")
                    && !l.trim().is_empty()
            })
            .count();
        assert_eq!(data_lines, 17 * 17 * 17);
    }
}
