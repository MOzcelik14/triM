//! Unified color grading, White Balance (Temperature/Tint), Auto WB analysis,
//! Brightness/Contrast/Gamma presets, and categorized Creative Color Looks.

use serde::{Deserialize, Serialize};

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
}
