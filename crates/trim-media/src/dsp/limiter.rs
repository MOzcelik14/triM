//! Stereo-linked lookahead brickwall peak limiter with zero hot-path allocations.

pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;
pub const DEFAULT_LOOKAHEAD_MS: f32 = 5.0;
pub const DEFAULT_CEILING_DBFS: f32 = -0.1;
pub const DEFAULT_ATTACK_MS: f32 = 1.0;
pub const DEFAULT_RELEASE_MS: f32 = 75.0;

/// Stereo-linked lookahead brickwall peak limiter.
///
/// Uses a pre-allocated circular delay line (`delay_buf`) and peak window (`peak_buf`)
/// to anticipate incoming transients `lookahead_ms` (default 5.0 ms = 240 frames at 48 kHz)
/// before they reach the output stage. Both stereo channels are attenuated by an identical
/// scalar gain factor to preserve stereo image and inter-channel phase relationships.
#[derive(Debug, Clone)]
pub struct LookaheadLimiter {
    sample_rate: u32,
    lookahead_samples: usize,
    ceiling: f32,
    attack_coeff: f32,
    release_coeff: f32,
    delay_buf: Vec<[f32; 2]>,
    peak_buf: Vec<f32>,
    cursor: usize,
    window_max_peak: f32,
    envelope_gain: f32,
    last_gain_reduction_db: f32,
}

impl Default for LookaheadLimiter {
    fn default() -> Self {
        Self::new(DEFAULT_SAMPLE_RATE)
    }
}

impl LookaheadLimiter {
    pub fn new(sample_rate: u32) -> Self {
        Self::with_config(
            sample_rate,
            DEFAULT_LOOKAHEAD_MS,
            DEFAULT_CEILING_DBFS,
            DEFAULT_ATTACK_MS,
            DEFAULT_RELEASE_MS,
        )
    }

    pub fn with_config(
        sample_rate: u32,
        lookahead_ms: f32,
        ceiling_dbfs: f32,
        attack_ms: f32,
        release_ms: f32,
    ) -> Self {
        let sr = sample_rate.max(1) as f32;
        let lookahead_samples = ((sr * lookahead_ms.max(0.1) * 0.001).round() as usize).max(1);
        let ceiling = Self::dbfs_to_linear(ceiling_dbfs);
        let attack_coeff = Self::time_constant_coeff(attack_ms.max(0.05), sr);
        let release_coeff = Self::time_constant_coeff(release_ms.max(1.0), sr);

        Self {
            sample_rate: sample_rate.max(1),
            lookahead_samples,
            ceiling,
            attack_coeff,
            release_coeff,
            delay_buf: vec![[0.0, 0.0]; lookahead_samples],
            peak_buf: vec![0.0; lookahead_samples],
            cursor: 0,
            window_max_peak: 0.0,
            envelope_gain: 1.0,
            last_gain_reduction_db: 0.0,
        }
    }

    #[inline]
    pub fn dbfs_to_linear(dbfs: f32) -> f32 {
        10.0_f32.powf(dbfs / 20.0)
    }

    #[inline]
    fn time_constant_coeff(time_ms: f32, sample_rate: f32) -> f32 {
        let samples = (time_ms * 0.001 * sample_rate).max(1.0);
        (-1.0 / samples).exp()
    }

    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[inline]
    pub fn lookahead_samples(&self) -> usize {
        self.lookahead_samples
    }

    #[inline]
    pub fn ceiling(&self) -> f32 {
        self.ceiling
    }

    pub fn set_ceiling_dbfs(&mut self, ceiling_dbfs: f32) {
        self.ceiling = Self::dbfs_to_linear(ceiling_dbfs);
    }

    #[inline]
    pub fn envelope_gain(&self) -> f32 {
        self.envelope_gain
    }

    #[inline]
    pub fn last_gain_reduction_db(&self) -> f32 {
        self.last_gain_reduction_db
    }

    /// Resets internal delay line and gain envelope state without reallocating memory.
    pub fn reset(&mut self) {
        self.delay_buf.fill([0.0, 0.0]);
        self.peak_buf.fill(0.0);
        self.cursor = 0;
        self.window_max_peak = 0.0;
        self.envelope_gain = 1.0;
        self.last_gain_reduction_db = 0.0;
    }

    /// Pre-fills the lookahead ring buffer with up to `lookahead_samples` stereo frames
    /// so subsequent `process_sample` / `process_buffer_in_place` calls emit time-aligned
    /// output starting at frame 0 of `lookahead_frames` (zero AV sync shift).
    pub fn prime(&mut self, lookahead_frames: &[[f32; 2]]) {
        self.reset();
        let count = lookahead_frames.len().min(self.lookahead_samples);
        for &frame in &lookahead_frames[..count] {
            let in_peak = frame[0].abs().max(frame[1].abs());
            self.delay_buf[self.cursor] = frame;
            self.peak_buf[self.cursor] = in_peak;
            self.cursor += 1;
            if self.cursor >= self.lookahead_samples {
                self.cursor = 0;
            }
            if in_peak > self.window_max_peak {
                self.window_max_peak = in_peak;
            }
            self.step_envelope();
        }
    }

    #[inline]
    fn step_envelope(&mut self) {
        let target_gain = if self.window_max_peak > self.ceiling {
            self.ceiling / self.window_max_peak
        } else {
            1.0
        };

        if target_gain < self.envelope_gain {
            self.envelope_gain =
                self.attack_coeff * self.envelope_gain + (1.0 - self.attack_coeff) * target_gain;
        } else {
            self.envelope_gain =
                self.release_coeff * self.envelope_gain + (1.0 - self.release_coeff) * target_gain;
        }
    }

    /// Processes a single normalized stereo frame `[left, right]` through the lookahead
    /// delay line and returns the delayed, brickwall-limited stereo frame.
    #[inline]
    pub fn process_sample(&mut self, input: [f32; 2]) -> [f32; 2] {
        let delayed = self.delay_buf[self.cursor];
        let evicted_peak = self.peak_buf[self.cursor];

        let in_peak = input[0].abs().max(input[1].abs());
        self.delay_buf[self.cursor] = input;
        self.peak_buf[self.cursor] = in_peak;

        self.cursor += 1;
        if self.cursor >= self.lookahead_samples {
            self.cursor = 0;
        }

        if in_peak >= self.window_max_peak {
            self.window_max_peak = in_peak;
        } else if evicted_peak >= self.window_max_peak {
            let mut max_p = 0.0_f32;
            for &p in &self.peak_buf {
                if p > max_p {
                    max_p = p;
                }
            }
            self.window_max_peak = max_p;
        }

        self.step_envelope();

        // Enforce strict brickwall ceiling on the outgoing delayed frame
        let delayed_peak = delayed[0].abs().max(delayed[1].abs());
        let brickwall_gain = if delayed_peak > self.ceiling {
            self.ceiling / delayed_peak
        } else {
            1.0
        };

        let applied_gain = self.envelope_gain.min(brickwall_gain);
        if applied_gain < self.envelope_gain {
            self.envelope_gain = applied_gain;
        }

        self.last_gain_reduction_db = if applied_gain < 0.999_99 {
            20.0 * applied_gain.max(1e-6).log10()
        } else {
            0.0
        };

        [delayed[0] * applied_gain, delayed[1] * applied_gain]
    }

    /// Processes a slice of normalized stereo frames in-place with zero heap allocations.
    pub fn process_buffer_in_place(&mut self, samples: &mut [[f32; 2]]) {
        for frame in samples.iter_mut() {
            *frame = self.process_sample(*frame);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    #[test]
    fn test_limiter_overdrive_6dbfs_sine_never_exceeds_ceiling() {
        let mut limiter = LookaheadLimiter::new(48_000);
        let ceiling = limiter.ceiling();
        assert!((ceiling - 0.988_553_1).abs() < 1e-5);
        assert_eq!(limiter.lookahead_samples(), 240);

        // +6.02 dBFS sine wave (amplitude = 2.0) at 440 Hz over 100 ms (4800 frames)
        let num_frames = 4_800;
        let mut buffer: Vec<[f32; 2]> = (0..num_frames)
            .map(|i| {
                let phase = TAU * 440.0 * (i as f32 / 48_000.0);
                let s = 2.0 * phase.sin();
                [s, -s]
            })
            .collect();

        limiter.process_buffer_in_place(&mut buffer);

        let mut max_observed = 0.0_f32;
        for &[l, r] in &buffer {
            let peak = l.abs().max(r.abs());
            assert!(
                peak <= ceiling + 1e-6,
                "Output peak {} exceeded ceiling {} (-0.1 dBFS)",
                peak,
                ceiling
            );
            if peak > max_observed {
                max_observed = peak;
            }
        }
        // Ensure signal actually reached near the ceiling and was not muted
        assert!(max_observed > 0.95);
        assert!(limiter.last_gain_reduction_db() < -5.0);
    }

    #[test]
    fn test_limiter_transparency_on_silence_and_low_level_signals() {
        let mut limiter = LookaheadLimiter::new(48_000);

        // 1. Silence test
        let mut silence = vec![[0.0_f32, 0.0_f32]; 1_000];
        limiter.process_buffer_in_place(&mut silence);
        assert!(silence.iter().all(|&[l, r]| l == 0.0 && r == 0.0));
        assert!((limiter.envelope_gain() - 1.0).abs() < 1e-6);
        assert_eq!(limiter.last_gain_reduction_db(), 0.0);

        // 2. Low-amplitude signal (-12 dBFS, amplitude = 0.25) with priming for zero-latency alignment
        let total_frames = 1_240;
        let source: Vec<[f32; 2]> = (0..total_frames)
            .map(|i| {
                let t = i as f32 / 48_000.0;
                [
                    0.25 * (TAU * 220.0 * t).sin(),
                    0.15 * (TAU * 330.0 * t).cos(),
                ]
            })
            .collect();

        limiter.prime(&source[..240]);
        let mut work = source[240..1_240].to_vec();
        limiter.process_buffer_in_place(&mut work);

        // Because prime() loaded [0..240], processing [240..1240] must output exact [0..1000] with 1.0x gain
        for (i, (&out_frame, &expected_frame)) in
            work.iter().zip(source[..1_000].iter()).enumerate()
        {
            assert!(
                (out_frame[0] - expected_frame[0]).abs() < 1e-6
                    && (out_frame[1] - expected_frame[1]).abs() < 1e-6,
                "Frame {} diverged: got {:?}, expected {:?}",
                i,
                out_frame,
                expected_frame
            );
        }
        assert!((limiter.envelope_gain() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_limiter_stereo_linking_preserves_balance_and_phase() {
        let mut limiter = LookaheadLimiter::new(48_000);

        // Left channel is +6 dBFS (2.0), Right channel is -6 dBFS (0.5) -> constant 4:1 ratio, opposite phase
        let total = 2_240;
        let source: Vec<[f32; 2]> = (0..total)
            .map(|i| {
                let s = (TAU * 500.0 * (i as f32 / 48_000.0)).sin();
                [2.0 * s, -0.5 * s]
            })
            .collect();

        limiter.prime(&source[..240]);
        let mut buf = source[240..].to_vec();
        limiter.process_buffer_in_place(&mut buf);

        for (i, &[l, r]) in buf.iter().enumerate() {
            if l.abs() > 1e-4 {
                let ratio = l / r;
                assert!(
                    (ratio - (-4.0)).abs() < 1e-4,
                    "Stereo balance / phase ratio corrupted at sample {}: l={}, r={}, ratio={}",
                    i,
                    l,
                    r,
                    ratio
                );
            }
            assert!(l.abs() <= limiter.ceiling() + 1e-6);
            assert!(r.abs() <= limiter.ceiling() + 1e-6);
        }
    }
}
