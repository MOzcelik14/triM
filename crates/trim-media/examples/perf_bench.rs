use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use trim_core::{Clip, MediaItem, MediaType, Project, Track, TrackType};
use trim_media::ffmpeg::ffmpeg_cmd;
use trim_media::video::{
    decode_frame_raw_sized, frame_tolerance_for_fps, select_closest_frame_within_tolerance,
    PersistentSeekDecoder, PlaybackStreamer, VideoEngine,
};

// ---------------------------------------------------------------------------
// Statistics Helpers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct StatsSummary {
    samples: usize,
    median: f64,
    p95: f64,
    min: f64,
    max: f64,
}

fn summarize(vals: &[f64]) -> StatsSummary {
    if vals.is_empty() {
        return StatsSummary {
            samples: 0,
            median: 0.0,
            p95: 0.0,
            min: 0.0,
            max: 0.0,
        };
    }
    let mut sorted = vals.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let n = sorted.len();
    let median = if n % 2 == 1 {
        sorted[n / 2]
    } else {
        0.5 * (sorted[n / 2 - 1] + sorted[n / 2])
    };
    let p95_idx = ((n - 1) as f64 * 0.95).round() as usize;
    StatsSummary {
        samples: n,
        median,
        p95: sorted[p95_idx.min(n - 1)],
        min: sorted[0],
        max: sorted[n - 1],
    }
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

// ---------------------------------------------------------------------------
// Linux /proc Process Metrics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default)]
struct ProcCpuTicks {
    utime: u64,
    stime: u64,
}

fn read_proc_cpu_ticks(pid: u32) -> Option<ProcCpuTicks> {
    #[cfg(target_os = "linux")]
    {
        let path = format!("/proc/{pid}/stat");
        let content = std::fs::read_to_string(path).ok()?;
        let after_paren = content[content.rfind(')')? + 2..].split_whitespace();
        let fields: Vec<&str> = after_paren.collect();
        // fields[11] is utime, fields[12] is stime
        let utime: u64 = fields.get(11)?.parse().ok()?;
        let stime: u64 = fields.get(12)?.parse().ok()?;
        Some(ProcCpuTicks { utime, stime })
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
    }
}

fn read_proc_rss_mb(pid: u32) -> Option<f64> {
    #[cfg(target_os = "linux")]
    {
        let path = format!("/proc/{pid}/status");
        let content = std::fs::read_to_string(path).ok()?;
        for line in content.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let kb: f64 = parts.get(1)?.parse().ok()?;
                return Some(kb / 1024.0);
            }
        }
        None
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
    }
}

fn calculate_cpu_pct(start: ProcCpuTicks, end: ProcCpuTicks, elapsed_s: f64) -> f64 {
    if elapsed_s <= 0.001 {
        return 0.0;
    }
    let delta_ticks =
        (end.utime.saturating_sub(start.utime) + end.stime.saturating_sub(start.stime)) as f64;
    let user_hz = 100.0;
    (delta_ticks / (elapsed_s * user_hz)) * 100.0
}

// ---------------------------------------------------------------------------
// System & Media Probing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct SystemMetadata {
    cpu_model: String,
    logical_cores: usize,
    ram_total_mb: f64,
    ram_avail_mb: f64,
    ffmpeg_version: String,
    os_kernel: String,
    command_line: String,
}

fn get_system_metadata(cmd_line: &str) -> SystemMetadata {
    let logical_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let mut cpu_model = "Unknown CPU".to_string();
    let mut ram_total_mb = 0.0;
    let mut ram_avail_mb = 0.0;

    if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in cpuinfo.lines() {
            if line.starts_with("model name") {
                if let Some((_, model)) = line.split_once(':') {
                    cpu_model = model.trim().to_string();
                    break;
                }
            }
        }
    }

    if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(kb) = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<f64>().ok())
                {
                    ram_total_mb = kb / 1024.0;
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(kb) = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<f64>().ok())
                {
                    ram_avail_mb = kb / 1024.0;
                }
            }
        }
    }

    let ffmpeg_version = ffmpeg_cmd()
        .arg("-version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(str::to_string))
        .unwrap_or_else(|| "Unknown FFmpeg".to_string());

    let os_kernel = Command::new("uname")
        .arg("-srmo")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Linux".to_string());

    SystemMetadata {
        cpu_model,
        logical_cores,
        ram_total_mb,
        ram_avail_mb,
        ffmpeg_version,
        os_kernel,
        command_line: cmd_line.to_string(),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct MediaMetadata {
    label: String,
    path: String,
    codec: String,
    width: u32,
    height: u32,
    fps_rational: String,
    fps: f64,
    bitrate_kbps: f64,
    pix_fmt: String,
    duration_s: f64,
}

fn probe_media(path: &Path, label: &str) -> Option<MediaMetadata> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name,width,height,r_frame_rate,pix_fmt,bit_rate:format=duration,bit_rate",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .ok()?;
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let stream = json.get("streams")?.as_array()?.first()?;
    let fmt = json.get("format")?;

    let codec = stream
        .get("codec_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let width = stream.get("width").and_then(|v| v.as_u64()).unwrap_or(1920) as u32;
    let height = stream
        .get("height")
        .and_then(|v| v.as_u64())
        .unwrap_or(1080) as u32;
    let fps_rational = stream
        .get("r_frame_rate")
        .and_then(|v| v.as_str())
        .unwrap_or("30/1")
        .to_string();
    let fps = parse_rational_fps(&fps_rational).unwrap_or(30.0);
    let pix_fmt = stream
        .get("pix_fmt")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let duration_s: f64 = fmt
        .get("duration")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(10.0);
    let bit_rate_bps: f64 = stream
        .get("bit_rate")
        .or_else(|| fmt.get("bit_rate"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    Some(MediaMetadata {
        label: label.to_string(),
        path: path.to_string_lossy().to_string(),
        codec,
        width,
        height,
        fps_rational,
        fps,
        bitrate_kbps: bit_rate_bps / 1000.0,
        pix_fmt,
        duration_s,
    })
}

fn ensure_synthetic_fallback(tmp_dir: &Path) -> PathBuf {
    let out = tmp_dir.join("synthetic_1080p_24fps.mp4");
    if !out.exists() {
        let _ = ffmpeg_cmd()
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=duration=12:size=1920x1080:rate=24000/1001",
                "-c:v",
                "libx264",
                "-preset",
                "fast",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&out)
            .output();
    }
    out
}

// ---------------------------------------------------------------------------
// 1. Spawn Overhead Benchmark (Separated from Steady-State)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct SpawnOverheadResult {
    process_spawn_ms: StatsSummary,
    first_frame_ms: StatsSummary,
}

fn bench_spawn_overhead(path: &Path, fps: f64, repeats: usize) -> SpawnOverheadResult {
    let p_str = path.to_string_lossy().to_string();
    let mut spawn_times = Vec::with_capacity(repeats);
    let mut first_frame_times = Vec::with_capacity(repeats);

    // Warmup 1 run (not recorded)
    {
        let mut s = PlaybackStreamer::new();
        s.start_sized("warmup", &p_str, 0.0, fps, false, 0.0, 640, 360);
        let dl = Instant::now() + Duration::from_secs(2);
        while Instant::now() < dl {
            if s.get_frame_for_time_with_pts(0.0).is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        s.stop();
    }

    for i in 0..repeats {
        let t0 = Instant::now();
        let mut s = PlaybackStreamer::new();
        s.start_sized(
            &format!("spawn_{i}"),
            &p_str,
            0.0,
            fps,
            false,
            0.0,
            640,
            360,
        );
        let mut pid_seen = false;
        let mut first_seen = false;
        let dl = Instant::now() + Duration::from_secs(3);
        while Instant::now() < dl {
            if !pid_seen && s.child_pid().is_some() {
                spawn_times.push(t0.elapsed().as_secs_f64() * 1000.0);
                pid_seen = true;
            }
            if s.get_frame_for_time_with_pts(0.0).is_some() {
                first_frame_times.push(t0.elapsed().as_secs_f64() * 1000.0);
                first_seen = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        if !pid_seen {
            spawn_times.push(0.0);
        }
        if !first_seen {
            first_frame_times.push(3000.0);
        }
        s.stop();
    }

    SpawnOverheadResult {
        process_spawn_ms: summarize(&spawn_times),
        first_frame_ms: summarize(&first_frame_times),
    }
}

// ---------------------------------------------------------------------------
// 2. Paced Playback (PTS-Verified Delivery) vs Unpaced Decode Throughput
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct PacedRunResult {
    delivered_pct: f64,
    dropped_pct: f64,
    delivered_frames: usize,
    dropped_frames: usize,
    total_expected: usize,
    latency_p50_ms: f64,
    latency_p95_ms: f64,
    pts_drift_p95_ms: f64,
    pts_drift_max_ms: f64,
    cpu_trim_pct: f64,
    cpu_ffmpeg_pct: f64,
    peak_rss_trim_mb: f64,
    peak_rss_ffmpeg_mb: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
struct PacedSummary {
    repeats: usize,
    fps_locked: f64,
    tolerance_ms: f64,
    delivered_pct: StatsSummary,
    dropped_pct: StatsSummary,
    latency_p95_ms: StatsSummary,
    pts_drift_max_ms: StatsSummary,
    cpu_trim_pct: StatsSummary,
    cpu_ffmpeg_pct: StatsSummary,
    peak_rss_trim_mb: StatsSummary,
    peak_rss_ffmpeg_mb: StatsSummary,
}

fn run_paced_once(path: &Path, fps: f64, measure_s: f64, warmup_s: f64) -> PacedRunResult {
    let mut streamer = PlaybackStreamer::new();
    let p_str = path.to_string_lossy().to_string();
    let tol = frame_tolerance_for_fps(fps);
    let my_pid = std::process::id();

    streamer.start_sized("paced", &p_str, 0.0, fps, false, 0.0, 640, 360);

    // Wait for first frame before starting the paced playback clock (excludes spawn overhead)
    let mut ffmpeg_pid = None;
    let mut ffmpeg_cpu_start = ProcCpuTicks::default();
    let mut ffmpeg_cpu_last = ProcCpuTicks::default();
    let init_dl = Instant::now() + Duration::from_secs(3);
    while Instant::now() < init_dl {
        if ffmpeg_pid.is_none() {
            if let Some(pid) = streamer.child_pid() {
                ffmpeg_pid = Some(pid);
                ffmpeg_cpu_start = read_proc_cpu_ticks(pid).unwrap_or_default();
                ffmpeg_cpu_last = ffmpeg_cpu_start;
            }
        }
        if streamer.get_frame_for_time_with_pts(0.0).is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }

    let my_cpu_start = read_proc_cpu_ticks(my_pid).unwrap_or_default();
    let t0 = Instant::now();
    let total_dur = warmup_s + measure_s;
    let total_frames = ((total_dur * fps).round() as usize).max(1);
    let warmup_frames = ((warmup_s * fps).round() as usize).min(total_frames);

    let mut delivered_frames = 0usize;
    let mut dropped_frames = 0usize;
    let mut last_counted_pts = -1.0_f64;
    let mut latencies_ms = Vec::new();
    let mut drifts_ms = Vec::new();
    let mut max_rss_trim = read_proc_rss_mb(my_pid).unwrap_or(0.0);
    let mut max_rss_ffmpeg = 0.0_f64;

    for n in 0..total_frames {
        let target_t = n as f64 / fps;
        let deadline = t0 + Duration::from_secs_f64(target_t);
        let now = Instant::now();
        if now < deadline {
            std::thread::sleep(deadline - now);
        }

        if let Some(f_pid) = ffmpeg_pid {
            if let Some(ticks) = read_proc_cpu_ticks(f_pid) {
                ffmpeg_cpu_last = ticks;
            }
            if let Some(rss) = read_proc_rss_mb(f_pid) {
                max_rss_ffmpeg = max_rss_ffmpeg.max(rss);
            }
        } else if let Some(pid) = streamer.child_pid() {
            ffmpeg_pid = Some(pid);
            ffmpeg_cpu_start = read_proc_cpu_ticks(pid).unwrap_or_default();
            ffmpeg_cpu_last = ffmpeg_cpu_start;
        }
        if let Some(rss) = read_proc_rss_mb(my_pid) {
            max_rss_trim = max_rss_trim.max(rss);
        }

        let query_t = Instant::now();
        let res = streamer.get_frame_for_time_with_pts(target_t);
        let latency_ms = query_t.saturating_duration_since(deadline).as_secs_f64() * 1000.0;

        if n < warmup_frames {
            if let Some((pts, _)) = res {
                last_counted_pts = pts;
            }
            continue;
        }

        // Strict verification: frame counts as delivered ONLY if PTS is within tolerance
        // AND is not a duplicate/stale repeat of the previous counted frame.
        match res {
            Some((pts, _))
                if (pts - target_t).abs() <= tol && (pts - last_counted_pts).abs() > 1e-6 =>
            {
                delivered_frames += 1;
                last_counted_pts = pts;
                latencies_ms.push(latency_ms);
                drifts_ms.push((pts - target_t).abs() * 1000.0);
            }
            _ => {
                dropped_frames += 1;
            }
        }
    }

    let elapsed_s = t0.elapsed().as_secs_f64().max(0.001);
    let my_cpu_end = read_proc_cpu_ticks(my_pid).unwrap_or(my_cpu_start);
    if let Some(f_pid) = ffmpeg_pid {
        if let Some(ticks) = read_proc_cpu_ticks(f_pid) {
            ffmpeg_cpu_last = ticks;
        }
    }
    streamer.stop();

    let total_expected = total_frames.saturating_sub(warmup_frames).max(1);
    let lat_stats = summarize(&latencies_ms);
    let drift_stats = summarize(&drifts_ms);

    PacedRunResult {
        delivered_pct: (delivered_frames as f64 / total_expected as f64) * 100.0,
        dropped_pct: (dropped_frames as f64 / total_expected as f64) * 100.0,
        delivered_frames,
        dropped_frames,
        total_expected,
        latency_p50_ms: lat_stats.median,
        latency_p95_ms: lat_stats.p95,
        pts_drift_p95_ms: drift_stats.p95,
        pts_drift_max_ms: drift_stats.max,
        cpu_trim_pct: calculate_cpu_pct(my_cpu_start, my_cpu_end, elapsed_s),
        cpu_ffmpeg_pct: calculate_cpu_pct(ffmpeg_cpu_start, ffmpeg_cpu_last, elapsed_s),
        peak_rss_trim_mb: max_rss_trim,
        peak_rss_ffmpeg_mb: max_rss_ffmpeg,
    }
}

fn bench_paced_repeated(path: &Path, fps: f64, repeats: usize) -> PacedSummary {
    let mut runs = Vec::with_capacity(repeats);
    for _ in 0..repeats {
        runs.push(run_paced_once(path, fps, 2.5, 0.5));
    }
    PacedSummary {
        repeats,
        fps_locked: fps,
        tolerance_ms: frame_tolerance_for_fps(fps) * 1000.0,
        delivered_pct: summarize(&runs.iter().map(|r| r.delivered_pct).collect::<Vec<_>>()),
        dropped_pct: summarize(&runs.iter().map(|r| r.dropped_pct).collect::<Vec<_>>()),
        latency_p95_ms: summarize(&runs.iter().map(|r| r.latency_p95_ms).collect::<Vec<_>>()),
        pts_drift_max_ms: summarize(&runs.iter().map(|r| r.pts_drift_max_ms).collect::<Vec<_>>()),
        cpu_trim_pct: summarize(&runs.iter().map(|r| r.cpu_trim_pct).collect::<Vec<_>>()),
        cpu_ffmpeg_pct: summarize(&runs.iter().map(|r| r.cpu_ffmpeg_pct).collect::<Vec<_>>()),
        peak_rss_trim_mb: summarize(&runs.iter().map(|r| r.peak_rss_trim_mb).collect::<Vec<_>>()),
        peak_rss_ffmpeg_mb: summarize(
            &runs
                .iter()
                .map(|r| r.peak_rss_ffmpeg_mb)
                .collect::<Vec<_>>(),
        ),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct UnpacedSummary {
    repeats: usize,
    fps: StatsSummary,
    unique_frames_per_run: usize,
}

fn bench_unpaced_repeated(
    path: &Path,
    fps: f64,
    duration_s: f64,
    repeats: usize,
) -> UnpacedSummary {
    let p_str = path.to_string_lossy().to_string();
    let tol = frame_tolerance_for_fps(fps);
    let mut fps_vals = Vec::with_capacity(repeats);
    let mut last_unique = 0usize;

    // Warmup
    {
        let mut s = PlaybackStreamer::new();
        s.start_sized("unpaced_warmup", &p_str, 0.0, fps, false, 0.0, 640, 360);
        let dl = Instant::now() + Duration::from_secs(2);
        while Instant::now() < dl {
            if s.get_frame_for_time_with_pts(0.0).is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        s.stop();
    }

    for _ in 0..repeats {
        let mut s = PlaybackStreamer::new();
        s.start_sized("unpaced", &p_str, 0.0, fps, false, 0.0, 640, 360);
        // Wait for first frame so spawn is excluded from steady-state decode rate
        let init_dl = Instant::now() + Duration::from_secs(3);
        let mut last_pts = -1.0_f64;
        while Instant::now() < init_dl {
            if let Some((pts, _)) = s.get_frame_for_time_with_pts(0.0) {
                last_pts = pts;
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }

        let t0 = Instant::now();
        let mut unique_frames = 0usize;
        let mut target_t = 1.0 / fps;
        let hard_dl = Instant::now() + Duration::from_secs(5);
        while target_t < duration_s && Instant::now() < hard_dl {
            if let Some((pts, _)) = s.get_frame_for_time_with_pts(target_t) {
                if (pts - last_pts).abs() > 1e-6 && pts >= target_t - tol {
                    unique_frames += 1;
                    last_pts = pts;
                    target_t += 1.0 / fps;
                    continue;
                }
            }
            std::thread::sleep(Duration::from_micros(250));
        }
        let elapsed = t0.elapsed().as_secs_f64().max(0.001);
        s.stop();
        last_unique = unique_frames;
        fps_vals.push(unique_frames as f64 / elapsed);
    }

    UnpacedSummary {
        repeats,
        fps: summarize(&fps_vals),
        unique_frames_per_run: last_unique,
    }
}

// ---------------------------------------------------------------------------
// 3. Seek Benchmarks: Cache-Miss vs Cache-Hit Separated
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct SeekComparisonSummary {
    cold_seek_miss_ms: StatsSummary,
    hot_seek_miss_ms: StatsSummary,
    scrub_forward_miss_ms: StatsSummary,
    scrub_cache_hit_ms: StatsSummary,
    p02_false_hit_comparison: P02ToleranceComparison,
}

#[derive(Debug, Clone, serde::Serialize)]
struct P02ToleranceComparison {
    old_fixed_45ms_false_hits_at_24fps: usize,
    old_fixed_45ms_false_hits_at_30fps: usize,
    old_fixed_45ms_false_hits_at_60fps: usize,
    new_scaled_false_hits_at_24fps: usize,
    new_scaled_false_hits_at_30fps: usize,
    new_scaled_false_hits_at_60fps: usize,
}

fn measure_p02_tolerance_comparison() -> P02ToleranceComparison {
    let mut old_24 = 0;
    let mut old_30 = 0;
    let mut old_60 = 0;
    let mut new_24 = 0;
    let mut new_30 = 0;
    let mut new_60 = 0;

    for (fps, old_cnt, new_cnt) in [
        (23.976, &mut old_24, &mut new_24),
        (30.0, &mut old_30, &mut new_30),
        (60.0, &mut old_60, &mut new_60),
    ] {
        let step = 1.0 / fps;
        let new_tol = frame_tolerance_for_fps(fps);
        // Simulate 100 forward steps where ring buffer only holds the previous frame at `t - step`
        for i in 1..=100 {
            let prev = [((i - 1) as f64 * step, i - 1)];
            let target = i as f64 * step;
            if select_closest_frame_within_tolerance(prev.iter(), target, 0.045).is_some() {
                *old_cnt += 1;
            }
            if select_closest_frame_within_tolerance(prev.iter(), target, new_tol).is_some() {
                *new_cnt += 1;
            }
        }
    }

    P02ToleranceComparison {
        old_fixed_45ms_false_hits_at_24fps: old_24,
        old_fixed_45ms_false_hits_at_30fps: old_30,
        old_fixed_45ms_false_hits_at_60fps: old_60,
        new_scaled_false_hits_at_24fps: new_24,
        new_scaled_false_hits_at_30fps: new_30,
        new_scaled_false_hits_at_60fps: new_60,
    }
}

fn bench_seeks(path: &Path, fps: f64, duration_s: f64, iterations: usize) -> SeekComparisonSummary {
    let p_str = path.to_string_lossy().to_string();
    let tol = frame_tolerance_for_fps(fps);

    // Warmup 1 cold decode
    let _ = decode_frame_raw_sized(path, 1.0, 640, 360);

    let mut cold_ms = Vec::with_capacity(iterations);
    let mut hot_ms = Vec::with_capacity(iterations);
    let mut scrub_miss_ms = Vec::with_capacity(iterations);
    let mut scrub_hit_ms = Vec::with_capacity(iterations);

    let mut streamer = PlaybackStreamer::new();
    let mut seed: u64 = 0x8543_2911_DEAD_BEEF;

    for _ in 0..iterations {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let frac = (seed % 10_000) as f64 / 10_000.0;
        let ts = (frac * (duration_s - 1.5)).max(0.2);

        // Cold seek (cache miss + new ffmpeg spawn)
        let t0 = Instant::now();
        let _ = decode_frame_raw_sized(path, ts, 640, 360);
        cold_ms.push(t0.elapsed().as_secs_f64() * 1000.0);

        // Hot seek (restart streamer at new timestamp, wait until PTS actually matches `ts`)
        let t1 = Instant::now();
        streamer.start_sized("hot_seek", &p_str, ts, fps, false, 0.0, 640, 360);
        let dl = Instant::now() + Duration::from_secs(2);
        while Instant::now() < dl {
            if let Some((pts, _)) = streamer.get_frame_for_time_with_pts(ts) {
                if (pts - ts).abs() <= tol || pts >= ts - tol {
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        hot_ms.push(t1.elapsed().as_secs_f64() * 1000.0);
    }
    streamer.stop();

    // Scrub forward cache-miss vs ring-buffer cache-hit
    let mut scrub = PersistentSeekDecoder::new();
    let _ = scrub.seek_with_fps(&p_str, 2.0, fps, 640, 360); // warmup
    let step = 1.0 / fps;
    let mut cur_t = 2.0 + step;

    for _ in 0..iterations {
        if cur_t >= duration_s - 1.0 {
            cur_t = 1.0;
        }
        // Cache-miss: advance by 1 frame period (guaranteed outside tol = 0.5/fps)
        let t_miss = Instant::now();
        let _ = scrub.seek_with_fps(&p_str, cur_t, fps, 640, 360);
        scrub_miss_ms.push(t_miss.elapsed().as_secs_f64() * 1000.0);

        // Cache-hit: immediate re-query at the exact same timestamp
        let t_hit = Instant::now();
        let _ = scrub.seek_with_fps(&p_str, cur_t, fps, 640, 360);
        scrub_hit_ms.push(t_hit.elapsed().as_secs_f64() * 1000.0);

        cur_t += step;
    }

    SeekComparisonSummary {
        cold_seek_miss_ms: summarize(&cold_ms),
        hot_seek_miss_ms: summarize(&hot_ms),
        scrub_forward_miss_ms: summarize(&scrub_miss_ms),
        scrub_cache_hit_ms: summarize(&scrub_hit_ms),
        p02_false_hit_comparison: measure_p02_tolerance_comparison(),
    }
}

// ---------------------------------------------------------------------------
// 4. Timeline Software Compositing (Cache-Miss Only, 5 Repeats)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct CompositingSummary {
    tracks: usize,
    ms_per_frame: StatsSummary,
    rss_mb: f64,
}

fn bench_compositing(
    path: &Path,
    tracks_count: usize,
    fps: f64,
    duration_s: f64,
    repeats: usize,
) -> CompositingSummary {
    let mut project = Project::new("Bench Comp");
    project.timeline.width = 640;
    project.timeline.height = 360;
    project.timeline.fps = fps;
    project.timeline.tracks.clear();

    let mut media_item = MediaItem::new(path);
    media_item.id = "m_bench".to_string();
    media_item.name = "bench_clip".to_string();
    media_item.media_type = MediaType::Video;
    media_item.duration = duration_s;
    media_item.width = 1920;
    media_item.height = 1080;
    project.media_pool.insert("m_bench".to_string(), media_item);

    for i in 0..tracks_count {
        let mut tr = Track::new(format!("V{}", i + 1), TrackType::Video);
        let mut clip = Clip::new("m_bench", 0.0, duration_s.min(10.0));
        clip.opacity = if i == 0 { 1.0 } else { 0.5 };
        tr.add_clip(clip);
        project.timeline.tracks.push(tr);
    }

    let mut engine = VideoEngine::new();
    let queries = 60usize;
    let mut ms_runs = Vec::with_capacity(repeats);

    // Warmup
    let _ = engine.render_timeline_frame(&project, 0.5, false);

    for r in 0..repeats {
        engine.clear_cache();
        let t0 = Instant::now();
        for q in 0..queries {
            let ts = 0.5 + ((r * queries + q) as f64 * 0.03) % (duration_s - 1.5).max(1.0);
            let _ = engine.render_timeline_frame(&project, ts, false);
        }
        ms_runs.push((t0.elapsed().as_secs_f64() * 1000.0) / queries as f64);
    }

    let rss_mb = read_proc_rss_mb(std::process::id()).unwrap_or(0.0);
    CompositingSummary {
        tracks: tracks_count,
        ms_per_frame: summarize(&ms_runs),
        rss_mb,
    }
}

// ---------------------------------------------------------------------------
// Report Struct & Markdown Generation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct HonestBenchmarkReport {
    timestamp: String,
    system: SystemMetadata,
    media: MediaMetadata,
    spawn_overhead: SpawnOverheadResult,
    paced_playback: PacedSummary,
    unpaced_decode: UnpacedSummary,
    seek_comparison: SeekComparisonSummary,
    compositing: Vec<CompositingSummary>,
}

fn generate_markdown(r: &HonestBenchmarkReport) -> String {
    let mut md = String::new();
    md.push_str("# triM. Dürüst ve Tekrarlanabilir Medya Benchmark Raporu\n\n");
    md.push_str(&format!("**Tarih:** {}\n", r.timestamp));
    md.push_str(&format!(
        "**Komut Satırı:** `{}`\n\n",
        r.system.command_line
    ));

    md.push_str("> [!IMPORTANT]\n");
    md.push_str("> **Önceki Sürüm Düzeltme Notu:** Bu dosyanın önceki sürümündeki sayılar hatalıydı ve silinmiştir:\n");
    md.push_str("> - `1612470.7×` hızlanma oranı hatalıydı: 45 ms'lik sabit halka önbellek toleransı, 23.976 fps'te (kare süresi 41.7 ms) ardışık her scrub adımını yanlışlıkla `cache-hit` sayarak `0.000 ms` ölçüyordu.\n");
    md.push_str("> - `7133.8 FPS` unpaced decode sayısı hatalıydı: `get_frame_for_time` son kareyi (`current_frame.clone()`) döndürdüğü için tüketici döngüsü yeni kare beklemeden aynı kareyi sayıyordu.\n");
    md.push_str("> - `671/671 (%100)` teslim ve `0.000 ms PTS drift` hatalıydı: dönen karenin gerçek PTS'i kontrol edilmiyor ve `target_t - n/fps` (her zaman 0) hesaplanıyordu.\n");
    md.push_str("> - `FFmpeg CPU %0.0` hatalıydı: `/proc/<pid>/stat` çocuk süreç kapandıktan sonra okunduğu için `0` dönüyordu.\n\n");

    md.push_str("## 1. Sistem ve Ortam Bilgisi\n\n");
    md.push_str(&format!(
        "- **İşlemci (CPU):** {} ({} mantıksal çekirdek)\n",
        r.system.cpu_model, r.system.logical_cores
    ));
    md.push_str(&format!(
        "- **Bellek (RAM):** {:.1} MB toplam, {:.1} MB kullanılabilir\n",
        r.system.ram_total_mb, r.system.ram_avail_mb
    ));
    md.push_str(&format!(
        "- **İşletim Sistemi / Çekirdek:** {}\n",
        r.system.os_kernel
    ));
    md.push_str(&format!(
        "- **FFmpeg Sürümü:** {}\n\n",
        r.system.ffmpeg_version
    ));

    md.push_str("## 2. Test Edilen Medya\n\n");
    md.push_str(&format!("- **Etiket:** `{}`\n", r.media.label));
    md.push_str(&format!("- **Dosya:** `{}`\n", r.media.path));
    md.push_str(&format!(
        "- **Kodek / Çözünürlük / FPS:** {} · {}×{} · `{}` ({:.3} fps)\n",
        r.media.codec, r.media.width, r.media.height, r.media.fps_rational, r.media.fps
    ));
    md.push_str(&format!(
        "- **Bit Hızı / Piksel Formatı / Süre:** {:.1} kbps · `{}` · {:.2} s\n\n",
        r.media.bitrate_kbps, r.media.pix_fmt, r.media.duration_s
    ));

    md.push_str("## 3. Süreç Başlatma (Spawn) Maliyeti (Ayrı Ölçüm, 5 Tekrar)\n\n");
    md.push_str("| Aşama | Medyan (ms) | p95 (ms) | Min (ms) | Max (ms) |\n");
    md.push_str("|---|---:|---:|---:|---:|\n");
    md.push_str(&format!(
        "| **FFmpeg Process Spawn** | {:.2} | {:.2} | {:.2} | {:.2} |\n",
        r.spawn_overhead.process_spawn_ms.median,
        r.spawn_overhead.process_spawn_ms.p95,
        r.spawn_overhead.process_spawn_ms.min,
        r.spawn_overhead.process_spawn_ms.max
    ));
    md.push_str(&format!(
        "| **İlk Kareye Ulaşma (Spawn + Decode)** | {:.2} | {:.2} | {:.2} | {:.2} |\n\n",
        r.spawn_overhead.first_frame_ms.median,
        r.spawn_overhead.first_frame_ms.p95,
        r.spawn_overhead.first_frame_ms.min,
        r.spawn_overhead.first_frame_ms.max
    ));

    md.push_str("## 4. Oynatma: Paced (Kaynak FPS'e Kilitli) vs Unpaced (Yeni Kare Çekme Hızı) — 5 Tekrar\n\n");
    md.push_str(&format!(
        "*PTS toleransı: ±{:.2} ms (`0.5 / fps`). Yinelenen/bayat kareler düşen kare sayılır.*\n\n",
        r.paced_playback.tolerance_ms
    ));
    md.push_str("| Metrik | Medyan | p95 | Min | Max |\n");
    md.push_str("|---|---:|---:|---:|---:|\n");
    md.push_str(&format!(
        "| **Paced Teslim Edilen (%)** | {:.1}% | {:.1}% | {:.1}% | {:.1}% |\n",
        r.paced_playback.delivered_pct.median,
        r.paced_playback.delivered_pct.p95,
        r.paced_playback.delivered_pct.min,
        r.paced_playback.delivered_pct.max
    ));
    md.push_str(&format!(
        "| **Paced Düşen / Bayat (%)** | {:.1}% | {:.1}% | {:.1}% | {:.1}% |\n",
        r.paced_playback.dropped_pct.median,
        r.paced_playback.dropped_pct.p95,
        r.paced_playback.dropped_pct.min,
        r.paced_playback.dropped_pct.max
    ));
    md.push_str(&format!(
        "| **Paced Kare Gecikmesi p95 (ms)** | {:.2} | {:.2} | {:.2} | {:.2} |\n",
        r.paced_playback.latency_p95_ms.median,
        r.paced_playback.latency_p95_ms.p95,
        r.paced_playback.latency_p95_ms.min,
        r.paced_playback.latency_p95_ms.max
    ));
    md.push_str(&format!(
        "| **Paced PTS Sapma Maks (ms)** | {:.2} | {:.2} | {:.2} | {:.2} |\n",
        r.paced_playback.pts_drift_max_ms.median,
        r.paced_playback.pts_drift_max_ms.p95,
        r.paced_playback.pts_drift_max_ms.min,
        r.paced_playback.pts_drift_max_ms.max
    ));
    md.push_str(&format!(
        "| **triM CPU (%)** | {:.1}% | {:.1}% | {:.1}% | {:.1}% |\n",
        r.paced_playback.cpu_trim_pct.median,
        r.paced_playback.cpu_trim_pct.p95,
        r.paced_playback.cpu_trim_pct.min,
        r.paced_playback.cpu_trim_pct.max
    ));
    md.push_str(&format!("| **FFmpeg Alt Süreç CPU (%) (`/proc/<pid>/stat`)** | {:.1}% | {:.1}% | {:.1}% | {:.1}% |\n", r.paced_playback.cpu_ffmpeg_pct.median, r.paced_playback.cpu_ffmpeg_pct.p95, r.paced_playback.cpu_ffmpeg_pct.min, r.paced_playback.cpu_ffmpeg_pct.max));
    md.push_str(&format!(
        "| **FFmpeg Tepe RSS (MB)** | {:.1} | {:.1} | {:.1} | {:.1} |\n",
        r.paced_playback.peak_rss_ffmpeg_mb.median,
        r.paced_playback.peak_rss_ffmpeg_mb.p95,
        r.paced_playback.peak_rss_ffmpeg_mb.min,
        r.paced_playback.peak_rss_ffmpeg_mb.max
    ));
    md.push_str(&format!(
        "| **Unpaced Benzersiz Decode FPS** | {:.1} FPS | {:.1} FPS | {:.1} FPS | {:.1} FPS |\n\n",
        r.unpaced_decode.fps.median,
        r.unpaced_decode.fps.p95,
        r.unpaced_decode.fps.min,
        r.unpaced_decode.fps.max
    ));

    md.push_str("## 5. Seek Gecikmesi: Cache-Miss vs Cache-Hit (Ayrıştırılmış)\n\n");
    md.push_str("| Seek Tipi | Durum | Medyan (ms) | p95 (ms) | Min (ms) | Max (ms) |\n");
    md.push_str("|---|---|---:|---:|---:|---:|\n");
    md.push_str(&format!("| **Cold Seek (`decode_frame_raw_sized`)** | Cache-Miss (yeni süreç) | {:.2} | {:.2} | {:.2} | {:.2} |\n", r.seek_comparison.cold_seek_miss_ms.median, r.seek_comparison.cold_seek_miss_ms.p95, r.seek_comparison.cold_seek_miss_ms.min, r.seek_comparison.cold_seek_miss_ms.max));
    md.push_str(&format!("| **Hot Seek (`PlaybackStreamer` restart)** | Cache-Miss (konum atlama) | {:.2} | {:.2} | {:.2} | {:.2} |\n", r.seek_comparison.hot_seek_miss_ms.median, r.seek_comparison.hot_seek_miss_ms.p95, r.seek_comparison.hot_seek_miss_ms.min, r.seek_comparison.hot_seek_miss_ms.max));
    md.push_str(&format!("| **Scrub İleri Adım (`PersistentSeekDecoder`)** | Cache-Miss (yeni kare) | {:.2} | {:.2} | {:.2} | {:.2} |\n", r.seek_comparison.scrub_forward_miss_ms.median, r.seek_comparison.scrub_forward_miss_ms.p95, r.seek_comparison.scrub_forward_miss_ms.min, r.seek_comparison.scrub_forward_miss_ms.max));
    md.push_str(&format!("| **Scrub Tekrar Sorgu (`recent_frames`)** | Cache-Hit (Arc paylaşımı) | {:.4} | {:.4} | {:.4} | {:.4} |\n\n", r.seek_comparison.scrub_cache_hit_ms.median, r.seek_comparison.scrub_cache_hit_ms.p95, r.seek_comparison.scrub_cache_hit_ms.min, r.seek_comparison.scrub_cache_hit_ms.max));

    let p02 = &r.seek_comparison.p02_false_hit_comparison;
    md.push_str(
        "### P0-2 Önce / Sonra Karşılaştırması: Sabit 45 ms Tolerans vs FPS'e Bağlı Tolerans\n\n",
    );
    md.push_str("| Senaryo (100 ileri kare adımı, önbellekte yalnızca önceki kare var) | Eski Sabit `45 ms` | Yeni `0.5 / fps` |\n");
    md.push_str("|---|---:|---:|\n");
    md.push_str(&format!(
        "| **23.976 fps yanlış komşu kare eşleşmesi** | {} / 100 | {} / 100 |\n",
        p02.old_fixed_45ms_false_hits_at_24fps, p02.new_scaled_false_hits_at_24fps
    ));
    md.push_str(&format!(
        "| **30.0 fps yanlış komşu kare eşleşmesi** | {} / 100 | {} / 100 |\n",
        p02.old_fixed_45ms_false_hits_at_30fps, p02.new_scaled_false_hits_at_30fps
    ));
    md.push_str(&format!(
        "| **60.0 fps yanlış komşu kare eşleşmesi** | {} / 100 | {} / 100 |\n\n",
        p02.old_fixed_45ms_false_hits_at_60fps, p02.new_scaled_false_hits_at_60fps
    ));

    md.push_str("## 6. Zaman Çizelgesi Yazılım Kompozisyonu (Cache-Miss, 5 Tekrar)\n\n");
    md.push_str("| Track Sayısı | Medyan (ms/kare) | p95 (ms/kare) | Min (ms/kare) | Max (ms/kare) | RSS (MB) |\n");
    md.push_str("|---:|---:|---:|---:|---:|---:|\n");
    for c in &r.compositing {
        md.push_str(&format!(
            "| {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.1} |\n",
            c.tracks,
            c.ms_per_frame.median,
            c.ms_per_frame.p95,
            c.ms_per_frame.min,
            c.ms_per_frame.max,
            c.rss_mb
        ));
    }

    md
}

fn timestamp_now() -> String {
    Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S %z")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let clip_arg = args.get(1).cloned();
    let default_real = PathBuf::from("/home/mozcelik/İndirilenler/MVI_8603.MOV");

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let (primary_path, label) = if let Some(ref arg) = clip_arg {
        (PathBuf::from(arg), "KULLANICI KLİBİ".to_string())
    } else if default_real.exists() {
        (default_real, "GERÇEK".to_string())
    } else {
        println!("Uyarı: Gerçek test klibi bulunamadı, sentetik medya üretiliyor...");
        (
            ensure_synthetic_fallback(temp_dir.path()),
            "SENTETİK".to_string(),
        )
    };

    let cmd_line = if let Some(ref arg) = clip_arg {
        format!("cargo run --release -p trim-media --example perf_bench -- {arg}")
    } else {
        format!(
            "cargo run --release -p trim-media --example perf_bench -- {}",
            primary_path.display()
        )
    };

    let system = get_system_metadata(&cmd_line);
    let media = probe_media(&primary_path, &label).expect("Medya ffprobe ile okunamadı");
    let repeats = 5usize;

    println!("=== triM. Dürüst Benchmark ({}) ===", media.label);
    println!(
        "Medya: {} ({}x{} @ {:.3} fps)",
        media.path, media.width, media.height, media.fps
    );

    println!("[1/5] Süreç başlatma (spawn) maliyeti ölçülüyor ({repeats} tekrar)...");
    let spawn_overhead = bench_spawn_overhead(&primary_path, media.fps, repeats);
    println!(
        "  -> Spawn medyan: {:.2} ms | İlk kare medyan: {:.2} ms",
        spawn_overhead.process_spawn_ms.median, spawn_overhead.first_frame_ms.median
    );

    println!("[2/5] Paced oynatma ölçülüyor ({repeats} tekrar, PTS doğrulamalı)...");
    let paced_playback = bench_paced_repeated(&primary_path, media.fps, repeats);
    println!(
        "  -> Teslim medyan: {:.1}%, Düşen: {:.1}%, FFmpeg CPU medyan: {:.1}%",
        paced_playback.delivered_pct.median,
        paced_playback.dropped_pct.median,
        paced_playback.cpu_ffmpeg_pct.median
    );

    println!("[3/5] Unpaced benzersiz kare decode hızı ölçülüyor ({repeats} tekrar)...");
    let unpaced_decode = bench_unpaced_repeated(&primary_path, media.fps, 2.0, repeats);
    println!(
        "  -> Unpaced FPS medyan: {:.1} FPS (min {:.1}, max {:.1})",
        unpaced_decode.fps.median, unpaced_decode.fps.min, unpaced_decode.fps.max
    );

    println!("[4/5] Seek gecikmesi ölçülüyor (cache-miss vs cache-hit, 15 iterasyon)...");
    let seek_comparison = bench_seeks(&primary_path, media.fps, media.duration_s, 15);
    println!(
        "  -> Cold miss medyan: {:.2} ms | Hot miss: {:.2} ms | Scrub miss: {:.2} ms | Scrub hit: {:.4} ms",
        seek_comparison.cold_seek_miss_ms.median,
        seek_comparison.hot_seek_miss_ms.median,
        seek_comparison.scrub_forward_miss_ms.median,
        seek_comparison.scrub_cache_hit_ms.median
    );

    println!("[5/5] Zaman çizelgesi kompozisyonu ölçülüyor (1, 3, 6 track, {repeats} tekrar)...");
    let compositing = [1usize, 3, 6]
        .into_iter()
        .map(|t| bench_compositing(&primary_path, t, media.fps, media.duration_s, repeats))
        .collect::<Vec<_>>();
    for c in &compositing {
        println!(
            "  -> {} track: medyan {:.2} ms/kare (p95 {:.2} ms)",
            c.tracks, c.ms_per_frame.median, c.ms_per_frame.p95
        );
    }

    let report = HonestBenchmarkReport {
        timestamp: timestamp_now(),
        system,
        media,
        spawn_overhead,
        paced_playback,
        unpaced_decode,
        seek_comparison,
        compositing,
    };

    let docs_dir = PathBuf::from("docs/benchmarks");
    let _ = std::fs::create_dir_all(&docs_dir);
    let md_path = docs_dir.join("2026-11-22_honest_benchmark.md");
    let json_path = docs_dir.join("2026-11-22_honest_benchmark.json");

    if let Ok(mut f) = File::create(&md_path) {
        let _ = f.write_all(generate_markdown(&report).as_bytes());
        println!("Kaydedildi: {}", md_path.display());
    }
    if let Ok(json_str) = serde_json::to_string_pretty(&report) {
        if let Ok(mut f) = File::create(&json_path) {
            let _ = f.write_all(json_str.as_bytes());
            println!("Kaydedildi: {}", json_path.display());
        }
    }
}
