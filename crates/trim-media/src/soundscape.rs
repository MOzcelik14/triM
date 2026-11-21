//! Procedural cinematic soundscape and title Foley WAV synthesizer (48 kHz 16-bit Stereo PCM).

use std::path::PathBuf;

/// Built-in procedural soundscape and Foley presets synthesized in pure Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundscapePreset {
    TensionRiser,
    SubBassDrop,
    FilmProjector35mm,
    NightRain,
    CinematicWhoosh,
    TypewriterFoley,
}

impl SoundscapePreset {
    pub const BIN_PRESETS: &'static [Self] = &[
        Self::TensionRiser,
        Self::SubBassDrop,
        Self::FilmProjector35mm,
        Self::NightRain,
        Self::CinematicWhoosh,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::TensionRiser => "∿ Gerilim Riser (Cinematic Tension Riser)",
            Self::SubBassDrop => "◈ Sub Bass Drop / Impact",
            Self::FilmProjector35mm => "▸ 35mm Film Projektörü & Makara",
            Self::NightRain => "≡ Gece Yağmuru & Rüzgar Ambiyansı",
            Self::CinematicWhoosh => "→ Sinematik Whoosh / Geçiş",
            Self::TypewriterFoley => "♪ Daktilo & Projektör Jenerik Foley",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::TensionRiser => "Gerilim Riser (SFX)",
            Self::SubBassDrop => "Sub Bass Drop (SFX)",
            Self::FilmProjector35mm => "35mm Projektör (Foley)",
            Self::NightRain => "Gece Yağmuru (Ambiyans)",
            Self::CinematicWhoosh => "Sinematik Whoosh (SFX)",
            Self::TypewriterFoley => "Daktilo & Projektör Foley",
        }
    }

    pub fn default_duration(&self) -> f64 {
        match self {
            Self::TensionRiser => 6.0,
            Self::SubBassDrop => 3.5,
            Self::FilmProjector35mm => 8.0,
            Self::NightRain => 10.0,
            Self::CinematicWhoosh => 2.0,
            Self::TypewriterFoley => 5.0,
        }
    }

    pub fn file_stem(&self) -> &'static str {
        match self {
            Self::TensionRiser => "trim_sfx_tension_riser",
            Self::SubBassDrop => "trim_sfx_sub_bass_drop",
            Self::FilmProjector35mm => "trim_sfx_35mm_projector",
            Self::NightRain => "trim_sfx_night_rain",
            Self::CinematicWhoosh => "trim_sfx_cinematic_whoosh",
            Self::TypewriterFoley => "trim_sfx_typewriter_foley",
        }
    }
}

/// Deterministic xorshift32 pseudo-random generator returning `[-1.0, 1.0]`.
#[inline]
fn next_noise(seed: &mut u32) -> f32 {
    let mut x = *seed;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *seed = x;
    ((x as f32) / (u32::MAX as f32)) * 2.0 - 1.0
}

/// Synthesizes a 48 kHz stereo 16-bit PCM WAV file for `preset` and writes it to the
/// user cache directory (`~/.cache/trim/soundscapes/`), returning the file path.
pub fn synthesize_soundscape_wav(
    preset: SoundscapePreset,
    duration_secs: f64,
    char_count: usize,
) -> Result<PathBuf, String> {
    let dur = duration_secs.clamp(0.5, 30.0);
    let sr = 48_000_u32;
    let total_frames = ((dur * sr as f64).round() as usize).max(1);
    let mut pcm_bytes = Vec::with_capacity(44 + total_frames * 4);

    let data_bytes = (total_frames * 4) as u32;
    let riff_size = 36 + data_bytes;
    pcm_bytes.extend_from_slice(b"RIFF");
    pcm_bytes.extend_from_slice(&riff_size.to_le_bytes());
    pcm_bytes.extend_from_slice(b"WAVE");
    pcm_bytes.extend_from_slice(b"fmt ");
    pcm_bytes.extend_from_slice(&16_u32.to_le_bytes());
    pcm_bytes.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    pcm_bytes.extend_from_slice(&2_u16.to_le_bytes()); // Stereo
    pcm_bytes.extend_from_slice(&sr.to_le_bytes());
    pcm_bytes.extend_from_slice(&(sr * 4).to_le_bytes());
    pcm_bytes.extend_from_slice(&4_u16.to_le_bytes());
    pcm_bytes.extend_from_slice(&16_u16.to_le_bytes());
    pcm_bytes.extend_from_slice(b"data");
    pcm_bytes.extend_from_slice(&data_bytes.to_le_bytes());

    let mut seed: u32 = 0x9E37_79B9;
    let mut phase1 = 0.0_f32;
    let mut phase2 = 0.0_f32;
    let mut lp_l = 0.0_f32;
    let mut lp_r = 0.0_f32;
    let tau = std::f32::consts::TAU;

    let type_chars = char_count.clamp(1, 200);
    let type_dur = (dur as f32 * 0.75).max(0.2);
    let step_sec = type_dur / type_chars as f32;

    for i in 0..total_frames {
        let t = i as f32 / sr as f32;
        let norm = (t / dur as f32).clamp(0.0, 1.0);
        let n_l = next_noise(&mut seed);
        let n_r = next_noise(&mut seed);

        let (mut l, mut r) = match preset {
            SoundscapePreset::TensionRiser => {
                let env = norm.powf(2.1) * (1.0 - ((norm - 0.94).max(0.0) / 0.06));
                let freq = 55.0 + 420.0 * norm.powf(2.4);
                phase1 = (phase1 + tau * freq / sr as f32) % tau;
                phase2 = (phase2 + tau * (freq * 1.498) / sr as f32) % tau;
                lp_l = lp_l * 0.92 + n_l * 0.08;
                lp_r = lp_r * 0.92 + n_r * 0.08;
                let shimmer = (t * (8.0 + 18.0 * norm)).sin() * 0.5 + 0.5;
                let s_l = (phase1.sin() * 0.42 + phase2.sin() * 0.25 + lp_l * shimmer * 0.55) * env;
                let s_r =
                    (phase1.sin() * 0.38 + (phase2 + 0.4).sin() * 0.28 + lp_r * shimmer * 0.55)
                        * env;
                (s_l, s_r)
            }
            SoundscapePreset::SubBassDrop => {
                let env = (-t * 1.45).exp() * (t / 0.015).min(1.0);
                let freq = 34.0 + 92.0 * (-t * 5.2).exp();
                phase1 = (phase1 + tau * freq / sr as f32) % tau;
                let transient = (-t * 28.0).exp() * n_l * 0.45;
                let body = (phase1.sin() * 1.35).tanh() * env * 0.78;
                (body + transient, body + transient * 0.9)
            }
            SoundscapePreset::FilmProjector35mm => {
                // 24 fps shutter sprocket click + warm motor hum + optical hiss
                let frame_phase = (t * 24.0).fract();
                let click_env = (-frame_phase * 42.0).exp();
                let click = click_env * (n_l * 0.55 + (t * 1450.0 * tau).sin() * 0.35);
                let motor = (t * 48.0 * tau).sin() * 0.08 + (t * 96.0 * tau).sin() * 0.05;
                lp_l = lp_l * 0.85 + n_l * 0.15;
                lp_r = lp_r * 0.85 + n_r * 0.15;
                let s_l = (click * 0.48 + motor + lp_l * 0.08) * 0.75;
                let s_r = (click * 0.44 + motor + lp_r * 0.08) * 0.75;
                (s_l, s_r)
            }
            SoundscapePreset::NightRain => {
                // Filtered pink-ish rain texture + slow wind swell
                lp_l = lp_l * 0.88 + n_l * 0.12;
                lp_r = lp_r * 0.88 + n_r * 0.12;
                let wind = 0.65 + 0.35 * (t * 0.45 * tau).sin();
                let drop = if n_l.abs() > 0.985 { n_l * 0.28 } else { 0.0 };
                let fade = (t / 0.4).min(1.0) * (((dur as f32 - t) / 0.5).clamp(0.0, 1.0));
                (
                    (lp_l * 0.48 * wind + drop) * fade,
                    (lp_r * 0.48 * wind + drop * 0.7) * fade,
                )
            }
            SoundscapePreset::CinematicWhoosh => {
                let bell = (norm * std::f32::consts::PI).sin().powf(1.8);
                let cutoff = 0.05 + 0.55 * bell;
                lp_l += cutoff * (n_l - lp_l);
                lp_r += cutoff * (n_r - lp_r);
                let pan = norm;
                let sub = (t * (60.0 + 90.0 * bell) * tau).sin() * 0.25 * bell;
                (
                    (lp_l * 0.75 * (1.0 - pan * 0.6) + sub) * bell,
                    (lp_r * 0.75 * (0.4 + pan * 0.6) + sub) * bell,
                )
            }
            SoundscapePreset::TypewriterFoley => {
                let mut strike = 0.0_f32;
                if t <= type_dur && step_sec > 0.005 {
                    let local = t % step_sec;
                    if local < 0.045 {
                        let env = (-local * 115.0).exp();
                        let clack = (local * 2150.0 * tau).sin() * 0.45
                            + (local * 3400.0 * tau).sin() * 0.30
                            + n_l * 0.65;
                        strike = clack * env * 0.72;
                    }
                }
                // Subtle background 35mm projector hum
                let frame_phase = (t * 24.0).fract();
                let proj_click = (-frame_phase * 50.0).exp() * n_r * 0.09;
                (strike + proj_click, strike * 0.92 + proj_click)
            }
        };

        l = l.clamp(-0.98, 0.98);
        r = r.clamp(-0.98, 0.98);
        let li = (l * 32767.0) as i16;
        let ri = (r * 32767.0) as i16;
        pcm_bytes.extend_from_slice(&li.to_le_bytes());
        pcm_bytes.extend_from_slice(&ri.to_le_bytes());
    }

    let dir = trim_core::paths::cache_dir().join("soundscapes");
    std::fs::create_dir_all(&dir).map_err(|e| format!("SFX klasörü oluşturulamadı: {e}"))?;
    let file_name = if preset == SoundscapePreset::TypewriterFoley {
        format!("{}_{:.1}s_{}c.wav", preset.file_stem(), dur, type_chars)
    } else {
        format!("{}.wav", preset.file_stem())
    };
    let out_path = dir.join(file_name);
    std::fs::write(&out_path, pcm_bytes).map_err(|e| format!("WAV dosyası yazılamadı: {e}"))?;
    Ok(out_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthesize_all_soundscape_presets() {
        for preset in SoundscapePreset::BIN_PRESETS {
            let path = synthesize_soundscape_wav(*preset, 0.6, 12)
                .expect("should synthesize soundscape wav");
            assert!(path.exists());
            let bytes = std::fs::read(&path).unwrap();
            assert!(bytes.starts_with(b"RIFF"));
            assert!(bytes.len() > 44);
        }
        let foley = synthesize_soundscape_wav(SoundscapePreset::TypewriterFoley, 1.0, 24)
            .expect("should synthesize typewriter foley wav");
        assert!(foley.exists());
    }
}
