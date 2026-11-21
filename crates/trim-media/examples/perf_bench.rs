use std::path::Path;
use std::time::{Duration, Instant};
use trim_core::{Clip, MediaItem, MediaType, Project, Track, TrackType};
use trim_media::ffmpeg::ffmpeg_cmd;
use trim_media::video::{decode_frame_raw, PlaybackStreamer, VideoEngine};

fn read_rss_mb() -> Option<f64> {
    #[cfg(target_os = "linux")]
    {
        let s = std::fs::read_to_string("/proc/self/statm").ok()?;
        let resident_pages: u64 = s.split_whitespace().nth(1)?.parse().ok()?;
        Some((resident_pages * 4096) as f64 / (1024.0 * 1024.0))
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

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

fn generate_test_clip(path: &Path, width: u32, height: u32, codec: &str, dur_s: u32) -> bool {
    let input = format!("testsrc2=duration={dur_s}:size={width}x{height}:rate=30");
    let out = ffmpeg_cmd()
        .args([
            "-y",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            &input,
            "-c:v",
            codec,
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(path)
        .output();
    matches!(out, Ok(o) if o.status.success())
}

fn bench_forward_stream(path: &Path, duration_s: f64) -> (f64, usize) {
    let mut streamer = PlaybackStreamer::new();
    let p_str = path.to_string_lossy().to_string();
    let t0 = Instant::now();
    streamer.start("bench_clip", &p_str, 0.0, 30.0, false, 0.0);
    let mut delivered = 0usize;
    let mut last_pts = -1.0f64;
    let total_expected = (duration_s * 30.0).round() as usize;

    // Wait up to duration_s + 2s for all frames to arrive from pipe
    let deadline = Instant::now() + Duration::from_secs_f64(duration_s + 2.0);
    let mut target_t = 0.0f64;
    while Instant::now() < deadline && target_t < duration_s {
        if streamer.get_frame_for_time(target_t).is_some() {
            if (target_t - last_pts).abs() > 1e-4 {
                delivered += 1;
                last_pts = target_t;
            }
            target_t += 1.0 / 30.0;
        } else {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    let elapsed = t0.elapsed().as_secs_f64().max(0.001);
    streamer.stop();
    let dropped = total_expected.saturating_sub(delivered);
    (delivered as f64 / elapsed, dropped)
}

fn bench_random_seeks(path: &Path, duration_s: f64, count: usize) -> (f64, f64) {
    let mut latencies_ms = Vec::with_capacity(count);
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    for _ in 0..count {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let frac = (seed % 10_000) as f64 / 10_000.0;
        let ts = frac * (duration_s - 0.1).max(0.1);
        let t0 = Instant::now();
        let _ = decode_frame_raw(path, ts);
        latencies_ms.push(t0.elapsed().as_secs_f64() * 1000.0);
    }
    latencies_ms.sort_by(|a, b| a.total_cmp(b));
    (
        percentile_f64(&latencies_ms, 0.50),
        percentile_f64(&latencies_ms, 0.95),
    )
}

fn bench_timeline_tracks(path: &Path, track_count: usize) -> (f64, f64) {
    let mut project = Project::default();
    let mut media = MediaItem::new(path);
    media.media_type = MediaType::Video;
    media.duration = 5.0;
    media.width = 1920;
    media.height = 1080;
    media.fps = 30.0;
    let mid = media.id.clone();
    project.add_media(media);

    project.timeline.tracks[0].add_clip(Clip::new(&mid, 0.0, 5.0));
    for idx in 1..track_count {
        let mut tr = Track::new(format!("V{}", idx + 1), TrackType::Video);
        let mut c = Clip::new(&mid, 0.0, 5.0);
        c.opacity = 0.5;
        tr.add_clip(c);
        project.timeline.tracks.insert(0, tr);
    }

    let mut engine = VideoEngine::new();
    // Warm up 5 frames
    for i in 0..5 {
        let _ = engine.render_timeline_frame(&project, i as f64 * 0.033, false);
    }

    let frames = 60usize;
    let t0 = Instant::now();
    for i in 0..frames {
        let t = (i as f64 * 0.033) % 4.5;
        let _ = engine.render_timeline_frame(&project, t, true);
    }
    let ms_per_frame = (t0.elapsed().as_secs_f64() * 1000.0) / frames as f64;
    let rss = read_rss_mb().unwrap_or(0.0);
    (ms_per_frame, rss)
}

fn main() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let h264_1080 = tmp.path().join("1080p_h264.mp4");
    let h264_4k = tmp.path().join("4k_h264.mp4");
    let hevc_4k = tmp.path().join("4k_hevc.mp4");

    println!("# triM. Preview & Media Benchmark");
    println!("Generating synthetic test media...");
    let ok_1080 = generate_test_clip(&h264_1080, 1920, 1080, "libx264", 5);
    let ok_4k_264 = generate_test_clip(&h264_4k, 3840, 2160, "libx264", 4);
    let ok_4k_hevc = generate_test_clip(&hevc_4k, 3840, 2160, "libx265", 4);

    for (label, path, ok, dur) in [
        ("1080p H.264", &h264_1080, ok_1080, 5.0),
        ("4K H.264", &h264_4k, ok_4k_264, 4.0),
        ("4K HEVC", &hevc_4k, ok_4k_hevc, 4.0),
    ] {
        if !ok {
            println!("## {label}: generation failed / skipped");
            continue;
        }
        let mut fps_runs = Vec::new();
        let mut drop_runs = Vec::new();
        for _ in 0..5 {
            let (fps, dropped) = bench_forward_stream(path, dur);
            fps_runs.push(fps);
            drop_runs.push(dropped as f64);
        }
        let (fps_med, fps_min, fps_max) = median_f64(&mut fps_runs);
        let (drop_med, _, _) = median_f64(&mut drop_runs);

        let mut p50_runs = Vec::new();
        let mut p95_runs = Vec::new();
        for _ in 0..5 {
            let (p50, p95) = bench_random_seeks(path, dur, 20);
            p50_runs.push(p50);
            p95_runs.push(p95);
        }
        let (p50_med, p50_min, p50_max) = median_f64(&mut p50_runs);
        let (p95_med, p95_min, p95_max) = median_f64(&mut p95_runs);

        println!("## {label}");
        println!(
            "- Forward stream: median={fps_med:.1} FPS (range {fps_min:.1}..{fps_max:.1}), median dropped={drop_med:.0}"
        );
        println!(
            "- Random seek (5x20): p50={p50_med:.1} ms ({p50_min:.1}..{p50_max:.1}), p95={p95_med:.1} ms ({p95_min:.1}..{p95_max:.1})"
        );
    }

    if ok_1080 {
        for tracks in [1usize, 3usize] {
            let mut ms_runs = Vec::new();
            let mut rss_runs = Vec::new();
            for _ in 0..5 {
                let (ms, rss) = bench_timeline_tracks(&h264_1080, tracks);
                ms_runs.push(ms);
                rss_runs.push(rss);
            }
            let (ms_med, ms_min, ms_max) = median_f64(&mut ms_runs);
            let (rss_med, _, _) = median_f64(&mut rss_runs);
            println!(
                "## Timeline {tracks}-track (1080p H.264): median={ms_med:.2} ms/frame ({ms_min:.2}..{ms_max:.2}), RSS={rss_med:.1} MB"
            );
        }
    }
}
