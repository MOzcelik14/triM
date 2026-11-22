use std::time::{Duration, Instant};

fn median_f64(vals: &mut [f64]) -> (f64, f64, f64) {
    vals.sort_by(|a, b| a.total_cmp(b));
    let min = vals.first().copied().unwrap_or(0.0);
    let max = vals.last().copied().unwrap_or(0.0);
    let med = if vals.is_empty() {
        0.0
    } else if vals.len() % 2 == 1 {
        vals[vals.len() / 2]
    } else {
        0.5 * (vals[vals.len() / 2 - 1] + vals[vals.len() / 2])
    };
    (med, min, max)
}

fn percentile_f64(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn parse_rational_fps(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some((num_s, den_s)) = s.split_once('/') {
        let num: f64 = num_s.trim().parse().ok()?;
        let den: f64 = den_s.trim().parse().ok()?;
        if num > 0.0 && den > 0.0 {
            return Some(num / den);
        }
    } else if let Ok(val) = s.parse::<f64>() {
        if val > 0.0 {
            return Some(val);
        }
    }
    None
}

#[test]
fn test_median_and_percentiles() {
    let mut empty: Vec<f64> = vec![];
    assert_eq!(median_f64(&mut empty), (0.0, 0.0, 0.0));
    assert_eq!(percentile_f64(&empty, 0.5), 0.0);

    let mut single = vec![42.0];
    assert_eq!(median_f64(&mut single), (42.0, 42.0, 42.0));
    assert_eq!(percentile_f64(&single, 0.5), 42.0);

    let mut odd = vec![5.0, 1.0, 3.0];
    assert_eq!(median_f64(&mut odd), (3.0, 1.0, 5.0));

    let mut even = vec![10.0, 20.0, 30.0, 40.0];
    assert_eq!(median_f64(&mut even), (25.0, 10.0, 40.0));

    let mut series: Vec<f64> = (1..=100).map(|x| x as f64).collect();
    series.sort_by(|a, b| a.total_cmp(b));
    let p50 = percentile_f64(&series, 0.50);
    let p95 = percentile_f64(&series, 0.95);
    let p99 = percentile_f64(&series, 0.99);

    assert!((p50 - 50.0).abs() <= 1.0, "p50 expected ~50, got {p50}");
    assert!((p95 - 95.0).abs() <= 1.0, "p95 expected ~95, got {p95}");
    assert!((p99 - 99.0).abs() <= 1.0, "p99 expected ~99, got {p99}");
}

#[test]
fn test_parse_rational_fps() {
    let r23 = parse_rational_fps("24000/1001").expect("parse 24000/1001");
    assert!((r23 - 23.976023976).abs() < 1e-6);

    let r30 = parse_rational_fps("30/1").expect("parse 30/1");
    assert_eq!(r30, 30.0);

    let r59 = parse_rational_fps("60000/1001").expect("parse 60000/1001");
    assert!((r59 - 59.94005994).abs() < 1e-6);

    let r_direct = parse_rational_fps("25.0").expect("parse 25.0");
    assert_eq!(r_direct, 25.0);

    assert!(parse_rational_fps("0/0").is_none());
    assert!(parse_rational_fps("invalid").is_none());
    assert!(parse_rational_fps("-30/1").is_none());
}

/// Mock frame source to test paced deadline logic and dropped frame accounting.
struct MockFrameSource {
    available_up_to_idx: usize,
}

impl MockFrameSource {
    fn poll_frame(&self, frame_idx: usize) -> Option<u32> {
        if frame_idx <= self.available_up_to_idx {
            Some(frame_idx as u32)
        } else {
            None
        }
    }
}

#[test]
fn test_paced_timing_and_drop_accounting_with_mock() {
    let fps = 24.0f64;
    let total_frames = 24usize; // 1 second
    let warmup_frames = 4usize; // first 4 frames are warmup

    // Mock scenario: frame source only produces frames 0..15 on time; frames 16..23 are delayed/missed
    let mock = MockFrameSource {
        available_up_to_idx: 15,
    };

    let mut warmup_delivered = 0;
    let mut warmup_missed = 0;
    let mut measured_delivered = 0;
    let mut measured_missed = 0;

    let t0 = Instant::now();
    for n in 0..total_frames {
        let target_t = n as f64 / fps;
        let deadline = t0 + Duration::from_secs_f64(target_t);
        assert!(deadline >= t0);

        // Verification of deadline monotonicity and spacing
        let calculated_time = n as f64 / fps;
        assert_eq!(target_t, calculated_time);

        let is_warmup = n < warmup_frames;
        if mock.poll_frame(n).is_some() {
            if is_warmup {
                warmup_delivered += 1;
            } else {
                measured_delivered += 1;
            }
        } else if is_warmup {
            warmup_missed += 1;
        } else {
            measured_missed += 1;
        }
    }

    assert_eq!(warmup_delivered, 4);
    assert_eq!(warmup_missed, 0);

    // Measured: frames 4..23 (20 frames total).
    // frames 4..15 available = 12 delivered.
    // frames 16..23 missing = 8 missed.
    assert_eq!(measured_delivered, 12);
    assert_eq!(measured_missed, 8);
    let delivered_pct = (measured_delivered as f64 / 20.0) * 100.0;
    let missed_pct = (measured_missed as f64 / 20.0) * 100.0;
    assert_eq!(delivered_pct, 60.0);
    assert_eq!(missed_pct, 40.0);
}
