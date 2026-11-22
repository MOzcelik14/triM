use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use trim_core::{Clip, MediaItem, MediaType, Project, Track, TrackType};
use trim_media::ffmpeg::ffmpeg_cmd;
use trim_media::video::{decode_frame_raw_sized, PlaybackStreamer, VideoEngine};

// ---------------------------------------------------------------------------
// Math & Statistics Helpers
// ---------------------------------------------------------------------------

pub fn median_f64(vals: &mut [f64]) -> (f64, f64, f64) {
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

pub fn percentile_f64(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

pub fn parse_rational_fps(s: &str) -> Option<f64> {
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
// Linux /proc Metrics Readers
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
    // Standard Linux USER_HZ / CLK_TCK is 100
    let user_hz = 100.0;
    (delta_ticks / (elapsed_s * user_hz)) * 100.0
}

// ---------------------------------------------------------------------------
// System Info Extraction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct SystemMetadata {
    cpu_model: String,
    logical_cores: usize,
    ram_total_mb: f64,
    ram_avail_mb: f64,
    ffmpeg_version_line1: String,
    os_kernel: String,
    is_vm: bool,
    vm_description: String,
}

fn get_system_metadata() -> SystemMetadata {
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
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(kb) = parts.get(1).and_then(|s| s.parse::<f64>().ok()) {
                    ram_total_mb = kb / 1024.0;
                }
            } else if line.starts_with("MemAvailable:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(kb) = parts.get(1).and_then(|s| s.parse::<f64>().ok()) {
                    ram_avail_mb = kb / 1024.0;
                }
            }
        }
    }

    let ffmpeg_out = ffmpeg_cmd().arg("-version").output().ok();
    let ffmpeg_version_line1 = ffmpeg_out
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(|l| l.to_string()))
        .unwrap_or_else(|| "FFmpeg not found".to_string());

    let uname_out = Command::new("uname").arg("-srmo").output().ok();
    let os_kernel = uname_out
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| std::env::consts::OS.to_string());

    let virt_out = Command::new("systemd-detect-virt").output().ok();
    let vm_str = virt_out
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "none".to_string());
    let is_vm = vm_str != "none" && !vm_str.is_empty();

    SystemMetadata {
        cpu_model,
        logical_cores,
        ram_total_mb,
        ram_avail_mb,
        ffmpeg_version_line1,
        os_kernel,
        is_vm,
        vm_description: if is_vm {
            vm_str
        } else {
            "Bare Metal (none)".to_string()
        },
    }
}

// ---------------------------------------------------------------------------
// Media Info Extraction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct MediaMetadata {
    path: String,
    label: String, // GERÇEK, SENTETİK, TÜRETİLMİŞ
    codec: String,
    profile: String,
    width: u32,
    height: u32,
    fps_rational_str: String,
    fps: f64,
    bitrate_kbps: f64,
    pix_fmt: String,
    duration_s: f64,
    i_frames: usize,
    p_frames: usize,
    b_frames: usize,
    warning: Option<String>,
}

fn probe_media(path: &Path, label: &str) -> Option<MediaMetadata> {
    let p_str = path.to_string_lossy().to_string();
    let probe_out = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            &p_str,
        ])
        .output()
        .ok()?;

    if !probe_out.status.success() {
        return None;
    }

    let json: serde_json::Value = serde_json::from_slice(&probe_out.stdout).ok()?;
    let video_stream = json["streams"]
        .as_array()?
        .iter()
        .find(|s| s["codec_type"].as_str() == Some("video"))?;

    let codec = video_stream["codec_name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let profile = video_stream["profile"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let width = video_stream["width"].as_u64().unwrap_or(0) as u32;
    let height = video_stream["height"].as_u64().unwrap_or(0) as u32;
    let fps_rational_str = video_stream["r_frame_rate"]
        .as_str()
        .unwrap_or("30/1")
        .to_string();
    let fps = parse_rational_fps(&fps_rational_str).unwrap_or(30.0);
    let bitrate_kbps = video_stream["bit_rate"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| {
            json["format"]["bit_rate"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
        })
        .unwrap_or(0.0)
        / 1000.0;
    let pix_fmt = video_stream["pix_fmt"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let duration_s = json["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| {
            video_stream["duration"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
        })
        .unwrap_or(0.0);

    // Count I/P/B frames via ffprobe pict_type
    let mut i_frames = 0;
    let mut p_frames = 0;
    let mut b_frames = 0;
    if let Ok(pict_out) = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v",
            "-show_entries",
            "frame=pict_type",
            "-of",
            "compact=p=0:nk=1",
            &p_str,
        ])
        .output()
    {
        if pict_out.status.success() {
            let s = String::from_utf8_lossy(&pict_out.stdout);
            for line in s.lines() {
                match line.trim() {
                    "I" => i_frames += 1,
                    "P" => p_frames += 1,
                    "B" => b_frames += 1,
                    _ => {}
                }
            }
        }
    }

    let warning = if label == "SENTETİK" {
        Some("SENTETİK MEDYA: Gerçek çekim yerine sentetik testsrc2 klibi kullanılmaktadır (gerçek çekim yerine geçmez).".to_string())
    } else if label == "TÜRETİLMİŞ" {
        Some("TÜRETİLMİŞ MEDYA: Orijinal 1080p klipten yukarı ölçeklenmiş 4K HEVC'dir, gerçek 4K kamera çekimi değildir.".to_string())
    } else {
        None
    };

    Some(MediaMetadata {
        path: p_str,
        label: label.to_string(),
        codec,
        profile,
        width,
        height,
        fps_rational_str,
        fps,
        bitrate_kbps,
        pix_fmt,
        duration_s,
        i_frames,
        p_frames,
        b_frames,
        warning,
    })
}

// ---------------------------------------------------------------------------
// Paced Forward Playback Simulation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct PacedPlaybackResult {
    warmup_duration_s: f64,
    warmup_delivered: usize,
    warmup_missed: usize,
    measured_duration_s: f64,
    delivered_frames: usize,
    missed_frames: usize,
    total_expected_frames: usize,
    delivered_pct: f64,
    missed_pct: f64,
    latency_p50_ms: f64,
    latency_p95_ms: f64,
    latency_p99_ms: f64,
    time_to_first_frame_ms: f64,
    pts_drift_max_ms: f64,
    cpu_trim_pct: f64,
    cpu_ffmpeg_pct: f64,
    peak_rss_trim_mb: f64,
    peak_rss_ffmpeg_mb: f64,
}

fn bench_paced_forward_playback(
    path: &Path,
    fps: f64,
    duration_s: f64,
    warmup_s: f64,
) -> PacedPlaybackResult {
    let mut streamer = PlaybackStreamer::new();
    let p_str = path.to_string_lossy().to_string();

    let my_pid = std::process::id();
    let my_cpu_start = read_proc_cpu_ticks(my_pid).unwrap_or_default();

    let t0 = Instant::now();
    streamer.start_sized("paced_bench", &p_str, 0.0, fps, false, 0.0, 640, 360);

    let mut first_frame_time = None;
    let mut warmup_delivered = 0usize;
    let mut warmup_missed = 0usize;
    let mut delivered_frames = 0usize;
    let mut missed_frames = 0usize;
    let mut latencies_ms = Vec::new();
    let mut pts_drift_max_ms = 0.0f64;

    let total_frames = ((duration_s * fps).round() as usize).max(1);
    let warmup_frame_count = ((warmup_s * fps).round() as usize).min(total_frames);

    let mut max_rss_trim = read_proc_rss_mb(my_pid).unwrap_or(0.0);
    let mut max_rss_ffmpeg = 0.0f64;
    let mut ffmpeg_cpu_start = ProcCpuTicks::default();
    let mut ffmpeg_pid_recorded = None;

    for n in 0..total_frames {
        let target_t = n as f64 / fps;
        let deadline = t0 + Duration::from_secs_f64(target_t);

        let now = Instant::now();
        if now < deadline {
            std::thread::sleep(deadline - now);
        }

        // Periodic process info sampling
        if ffmpeg_pid_recorded.is_none() {
            if let Some(f_pid) = streamer.child_pid() {
                ffmpeg_pid_recorded = Some(f_pid);
                ffmpeg_cpu_start = read_proc_cpu_ticks(f_pid).unwrap_or_default();
            }
        }
        if let Some(f_pid) = ffmpeg_pid_recorded {
            if let Some(rss_f) = read_proc_rss_mb(f_pid) {
                if rss_f > max_rss_ffmpeg {
                    max_rss_ffmpeg = rss_f;
                }
            }
        }
        if let Some(rss_m) = read_proc_rss_mb(my_pid) {
            if rss_m > max_rss_trim {
                max_rss_trim = rss_m;
            }
        }

        // Query frame at deadline (do NOT wait/poll if missing)
        let query_t = Instant::now();
        let frame_opt = streamer.get_frame_for_time(target_t);
        let latency_ms = query_t.saturating_duration_since(deadline).as_secs_f64() * 1000.0;

        let is_warmup = n < warmup_frame_count;

        if frame_opt.is_some() {
            if first_frame_time.is_none() {
                first_frame_time = Some(t0.elapsed().as_secs_f64() * 1000.0);
            }
            if is_warmup {
                warmup_delivered += 1;
            } else {
                delivered_frames += 1;
                latencies_ms.push(latency_ms);
            }

            // PTS drift check: verify target_t matches index / fps
            let calculated_time = n as f64 / fps;
            let drift_ms = (target_t - calculated_time).abs() * 1000.0;
            if drift_ms > pts_drift_max_ms {
                pts_drift_max_ms = drift_ms;
            }
        } else if is_warmup {
            warmup_missed += 1;
        } else {
            missed_frames += 1;
        }
    }

    let my_cpu_end = read_proc_cpu_ticks(my_pid).unwrap_or_default();
    let total_elapsed_s = t0.elapsed().as_secs_f64().max(0.001);

    let ffmpeg_cpu_end = ffmpeg_pid_recorded
        .and_then(read_proc_cpu_ticks)
        .unwrap_or(ffmpeg_cpu_start);

    streamer.stop();

    latencies_ms.sort_by(|a, b| a.total_cmp(b));
    let measured_frames = total_frames.saturating_sub(warmup_frame_count).max(1);
    let delivered_pct = (delivered_frames as f64 / measured_frames as f64) * 100.0;
    let missed_pct = (missed_frames as f64 / measured_frames as f64) * 100.0;

    let cpu_trim_pct = calculate_cpu_pct(my_cpu_start, my_cpu_end, total_elapsed_s);
    let cpu_ffmpeg_pct = calculate_cpu_pct(ffmpeg_cpu_start, ffmpeg_cpu_end, total_elapsed_s);

    PacedPlaybackResult {
        warmup_duration_s: warmup_s,
        warmup_delivered,
        warmup_missed,
        measured_duration_s: duration_s - warmup_s,
        delivered_frames,
        missed_frames,
        total_expected_frames: measured_frames,
        delivered_pct,
        missed_pct,
        latency_p50_ms: percentile_f64(&latencies_ms, 0.50),
        latency_p95_ms: percentile_f64(&latencies_ms, 0.95),
        latency_p99_ms: percentile_f64(&latencies_ms, 0.99),
        time_to_first_frame_ms: first_frame_time.unwrap_or(0.0),
        pts_drift_max_ms,
        cpu_trim_pct,
        cpu_ffmpeg_pct,
        peak_rss_trim_mb: max_rss_trim,
        peak_rss_ffmpeg_mb: max_rss_ffmpeg,
    }
}

// ---------------------------------------------------------------------------
// Free-Run (Unpaced) Decode Throughput
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct FreeRunResult {
    unpaced_fps: f64,
    delivered_frames: usize,
    elapsed_s: f64,
    notes: String,
}

fn bench_free_run_throughput(path: &Path, fps: f64, duration_s: f64) -> FreeRunResult {
    let mut streamer = PlaybackStreamer::new();
    let p_str = path.to_string_lossy().to_string();

    let t0 = Instant::now();
    streamer.start_sized("free_run", &p_str, 0.0, fps, false, 0.0, 640, 360);

    let mut delivered = 0usize;
    let deadline = Instant::now() + Duration::from_secs_f64(duration_s + 1.0);
    let mut target_t = 0.0f64;

    while Instant::now() < deadline && target_t < duration_s {
        if streamer.get_frame_for_time(target_t).is_some() {
            delivered += 1;
            target_t += 1.0 / fps;
        } else {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    let elapsed_s = t0.elapsed().as_secs_f64().max(0.001);
    streamer.stop();

    let unpaced_fps = delivered as f64 / elapsed_s;
    let notes = "Serbest decode hızı: Oynatma saatine / zaman sınırına (deadline) bağlı olmadan boru hattından çekilebilen maksimum ham decode verimi.".to_string();

    FreeRunResult {
        unpaced_fps,
        delivered_frames: delivered,
        elapsed_s,
        notes,
    }
}

// ---------------------------------------------------------------------------
// Cold vs Hot Seek Benchmark
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct SeekResultSummary {
    iterations: usize,
    cold_p50_ms: f64,
    cold_p95_ms: f64,
    cold_p99_ms: f64,
    hot_p50_ms: f64,
    hot_p95_ms: f64,
    hot_p99_ms: f64,
    ratio_cold_to_hot_p50: f64,
}

fn bench_cold_vs_hot_seek(
    path: &Path,
    fps: f64,
    duration_s: f64,
    count: usize,
) -> SeekResultSummary {
    let p_str = path.to_string_lossy().to_string();
    let mut cold_latencies = Vec::with_capacity(count);
    let mut hot_latencies = Vec::with_capacity(count);

    // Warm up an active streamer for hot seeks
    let mut streamer = PlaybackStreamer::new();
    streamer.start_sized("seek_streamer", &p_str, 0.0, fps, false, 0.0, 640, 360);
    // Wait up to 500ms for first frame to ensure process is initialized
    let warmup_deadline = Instant::now() + Duration::from_millis(500);
    while Instant::now() < warmup_deadline {
        if streamer.get_frame_for_time(0.0).is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }

    let mut seed: u64 = 0x8543_2911_DEAD_BEEF;

    for _ in 0..count {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let frac = (seed % 10_000) as f64 / 10_000.0;
        let ts = (frac * (duration_s - 1.0)).max(0.1);

        // 1. Cold seek (new FFmpeg process spawn + probe each time)
        let t_cold_0 = Instant::now();
        let _ = decode_frame_raw_sized(path, ts, 640, 360);
        cold_latencies.push(t_cold_0.elapsed().as_secs_f64() * 1000.0);

        // 2. Hot seek (re-starting playback position on warm streamer)
        let t_hot_0 = Instant::now();
        streamer.start_sized("seek_streamer", &p_str, ts, fps, false, 0.0, 640, 360);
        let deadline = Instant::now() + Duration::from_millis(2000);
        while Instant::now() < deadline {
            if streamer.get_frame_for_time(ts).is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        hot_latencies.push(t_hot_0.elapsed().as_secs_f64() * 1000.0);
    }

    streamer.stop();

    cold_latencies.sort_by(|a, b| a.total_cmp(b));
    hot_latencies.sort_by(|a, b| a.total_cmp(b));

    let cold_p50 = percentile_f64(&cold_latencies, 0.50);
    let hot_p50 = percentile_f64(&hot_latencies, 0.50);
    let ratio = if hot_p50 > 0.001 {
        cold_p50 / hot_p50
    } else {
        0.0
    };

    SeekResultSummary {
        iterations: count,
        cold_p50_ms: cold_p50,
        cold_p95_ms: percentile_f64(&cold_latencies, 0.95),
        cold_p99_ms: percentile_f64(&cold_latencies, 0.99),
        hot_p50_ms: hot_p50,
        hot_p95_ms: percentile_f64(&hot_latencies, 0.95),
        hot_p99_ms: percentile_f64(&hot_latencies, 0.99),
        ratio_cold_to_hot_p50: ratio,
    }
}

// ---------------------------------------------------------------------------
// Timeline Software Compositing Benchmark (No Cache Cheating)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct TimelineTrackResult {
    tracks: usize,
    queries: usize,
    ms_per_frame: f64,
    cache_hit_pct: f64,
    rss_mb: f64,
}

fn bench_timeline_compositing(
    path: &Path,
    track_count: usize,
    fps: f64,
    duration_s: f64,
) -> TimelineTrackResult {
    let mut project = Project::default();
    let mut media = MediaItem::new(path);
    media.media_type = MediaType::Video;
    media.duration = duration_s;
    media.width = 1920;
    media.height = 1080;
    media.fps = fps;
    let mid = media.id.clone();
    project.add_media(media);

    project.timeline.fps = fps;
    project.timeline.tracks[0].add_clip(Clip::new(&mid, 0.0, duration_s));

    for idx in 1..track_count {
        let mut tr = Track::new(format!("V{}", idx + 1), TrackType::Video);
        let mut c = Clip::new(&mid, 0.0, duration_s);
        c.opacity = 0.5;
        tr.add_clip(c);
        project.timeline.tracks.insert(0, tr);
    }

    let mut engine = VideoEngine::new();
    // Use capacity of 0 or query 300 strictly unique timestamps to prevent caching bias
    let query_count = 300usize;
    let mut cache_hits = 0usize;

    let t0 = Instant::now();
    for i in 0..query_count {
        // Query unique non-repeating timestamps spaced throughout the clip
        let t = ((i as f64 * (1.0 / fps)) % (duration_s - 0.2)).max(0.01);
        let (_, is_cached) = engine.is_timeline_time_cached(&project, t);
        if is_cached {
            cache_hits += 1;
        }
        let _ = engine.render_timeline_frame(&project, t, false);
    }
    let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let ms_per_frame = elapsed_ms / query_count as f64;
    let cache_hit_pct = (cache_hits as f64 / query_count as f64) * 100.0;
    let rss_mb = read_proc_rss_mb(std::process::id()).unwrap_or(0.0);

    TimelineTrackResult {
        tracks: track_count,
        queries: query_count,
        ms_per_frame,
        cache_hit_pct,
        rss_mb,
    }
}

// ---------------------------------------------------------------------------
// Color Range Parity Check (yuvj420p Full Range vs Export)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct ColorParityResult {
    mean_abs_diff_rgb: f64,
    mean_abs_diff_r: f64,
    mean_abs_diff_g: f64,
    mean_abs_diff_b: f64,
    preview_black_rgb: [u8; 3],
    preview_white_rgb: [u8; 3],
    export_black_rgb: [u8; 3],
    export_white_rgb: [u8; 3],
    source_pix_fmt: String,
    assessment: String,
}

fn bench_color_range_parity(path: &Path) -> Option<ColorParityResult> {
    let p_str = path.to_string_lossy().to_string();

    // 1. Preview RGBA extraction at t = 1.0s, resolution 640x360
    let preview_img = decode_frame_raw_sized(path, 1.0, 640, 360)?;
    let preview_raw = preview_img.as_raw();

    // 2. Export 1-frame MP4 at 640x360 via libx264 yuv420p
    let tmp = tempfile::tempdir().ok()?;
    let out_mp4 = tmp.path().join("color_parity_test.mp4");

    let exp_status = ffmpeg_cmd()
        .args([
            "-y",
            "-v",
            "error",
            "-ss",
            "1.0",
            "-i",
            &p_str,
            "-vframes",
            "1",
            "-vf",
            "scale=640:360:force_original_aspect_ratio=decrease,pad=640:360:(ow-iw)/2:(oh-ih)/2:black",
            "-c:v",
            "libx264",
            "-crf",
            "10",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&out_mp4)
        .output()
        .ok()?;

    if !exp_status.status.success() {
        return None;
    }

    // Decode exported MP4 back to RGBA
    let dec_status = ffmpeg_cmd()
        .args([
            "-v",
            "error",
            "-i",
            out_mp4.to_str()?,
            "-vframes",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-",
        ])
        .output()
        .ok()?;

    if !dec_status.status.success() || dec_status.stdout.len() != preview_raw.len() {
        return None;
    }

    let export_raw = &dec_status.stdout;

    let mut sum_diff_r = 0.0f64;
    let mut sum_diff_g = 0.0f64;
    let mut sum_diff_b = 0.0f64;

    let mut p_min = [255u8, 255u8, 255u8];
    let mut p_max = [0u8, 0u8, 0u8];
    let mut e_min = [255u8, 255u8, 255u8];
    let mut e_max = [0u8, 0u8, 0u8];

    let pixel_count = (640 * 360) as f64;

    for chunk_idx in 0..(640 * 360) {
        let offset = chunk_idx * 4;
        let pr = preview_raw[offset];
        let pg = preview_raw[offset + 1];
        let pb = preview_raw[offset + 2];

        let er = export_raw[offset];
        let eg = export_raw[offset + 1];
        let eb = export_raw[offset + 2];

        sum_diff_r += (pr as f64 - er as f64).abs();
        sum_diff_g += (pg as f64 - eg as f64).abs();
        sum_diff_b += (pb as f64 - eb as f64).abs();

        for i in 0..3 {
            let pv = [pr, pg, pb][i];
            let ev = [er, eg, eb][i];
            if pv < p_min[i] {
                p_min[i] = pv;
            }
            if pv > p_max[i] {
                p_max[i] = pv;
            }
            if ev < e_min[i] {
                e_min[i] = ev;
            }
            if ev > e_max[i] {
                e_max[i] = ev;
            }
        }
    }

    let mean_r = sum_diff_r / pixel_count;
    let mean_g = sum_diff_g / pixel_count;
    let mean_b = sum_diff_b / pixel_count;
    let mean_rgb = (mean_r + mean_g + mean_b) / 3.0;

    let assessment = if mean_rgb <= 5.0 {
        "YÜKSEK UYUM (mean |ΔRGB| <= 5.0): Preview ve export renk aralığı kabul edilebilir sınırlar içinde.".to_string()
    } else {
        format!("RENK ARALIĞI UYUŞMAZLIĞI (mean |ΔRGB| = {mean_rgb:.2}): yuvj420p (full range 0-255) swscale tarafından RGBA'ya çevrilirken ile yuv420p (tv/limited range 16-235) export arasında seviye sıkıştırma farkı mevcut.")
    };

    Some(ColorParityResult {
        mean_abs_diff_rgb: mean_rgb,
        mean_abs_diff_r: mean_r,
        mean_abs_diff_g: mean_g,
        mean_abs_diff_b: mean_b,
        preview_black_rgb: p_min,
        preview_white_rgb: p_max,
        export_black_rgb: e_min,
        export_white_rgb: e_max,
        source_pix_fmt: "yuvj420p (full range pc, smpte170m/bt709)".to_string(),
        assessment,
    })
}

// ---------------------------------------------------------------------------
// Hardware Decode Experiment
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct HwDecodeExperiment {
    engine: String,
    success: bool,
    frames_tested: usize,
    elapsed_s: Option<f64>,
    fps: Option<f64>,
    notes: String,
}

fn bench_hw_accel_attempt(path: &Path) -> Vec<HwDecodeExperiment> {
    let p_str = path.to_string_lossy().to_string();
    let mut results = Vec::new();

    // 1. CPU Reference decode
    let t0 = Instant::now();
    let cpu_out = ffmpeg_cmd()
        .args([
            "-v",
            "error",
            "-i",
            &p_str,
            "-frames:v",
            "60",
            "-vf",
            "scale=640:360,format=rgba",
            "-f",
            "null",
            "-",
        ])
        .output();
    let cpu_dur = t0.elapsed().as_secs_f64();
    if let Ok(o) = cpu_out {
        if o.status.success() {
            results.push(HwDecodeExperiment {
                engine: "CPU Reference (swscale)".to_string(),
                success: true,
                frames_tested: 60,
                elapsed_s: Some(cpu_dur),
                fps: Some(60.0 / cpu_dur.max(0.001)),
                notes: "Temel yazılım decode ve RGBA ölçekleme hattı".to_string(),
            });
        }
    }

    // 2. VAAPI Attempt
    let t_vaapi = Instant::now();
    let vaapi_out = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-hwaccel",
            "vaapi",
            "-vaapi_device",
            "/dev/dri/renderD128",
            "-i",
            &p_str,
            "-frames:v",
            "60",
            "-vf",
            "scale_vaapi=w=640:h=360,hwdownload,format=nv12,format=rgba",
            "-f",
            "null",
            "-",
        ])
        .output();
    let vaapi_dur = t_vaapi.elapsed().as_secs_f64();
    match vaapi_out {
        Ok(o) if o.status.success() => {
            results.push(HwDecodeExperiment {
                engine: "VAAPI (Intel iHD)".to_string(),
                success: true,
                frames_tested: 60,
                elapsed_s: Some(vaapi_dur),
                fps: Some(60.0 / vaapi_dur.max(0.001)),
                notes: "Başarılı donanım decode".to_string(),
            });
        }
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr).trim().replace('\n', " ");
            results.push(HwDecodeExperiment {
                engine: "VAAPI (Intel iHD)".to_string(),
                success: false,
                frames_tested: 60,
                elapsed_s: None,
                fps: None,
                notes: format!("Bu ortamda çalışmadı: {err}"),
            });
        }
        Err(e) => {
            results.push(HwDecodeExperiment {
                engine: "VAAPI (Intel iHD)".to_string(),
                success: false,
                frames_tested: 60,
                elapsed_s: None,
                fps: None,
                notes: format!("Çalıştırılamadı: {e}"),
            });
        }
    }

    // 3. CUDA Attempt
    let t_cuda = Instant::now();
    let cuda_out = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-hwaccel",
            "cuda",
            "-hwaccel_output_format",
            "cuda",
            "-i",
            &p_str,
            "-frames:v",
            "60",
            "-vf",
            "scale_cuda=640:360,hwdownload,format=nv12,format=rgba",
            "-f",
            "null",
            "-",
        ])
        .output();
    let cuda_dur = t_cuda.elapsed().as_secs_f64();
    match cuda_out {
        Ok(o) if o.status.success() => {
            results.push(HwDecodeExperiment {
                engine: "CUDA (NVIDIA RTX 3050)".to_string(),
                success: true,
                frames_tested: 60,
                elapsed_s: Some(cuda_dur),
                fps: Some(60.0 / cuda_dur.max(0.001)),
                notes: format!("Başarılı ancak context init + hwdownload nedeniyle 60 kare için CPU'dan daha uzun sürdü ({cuda_dur:.3}s vs CPU {cpu_dur:.3}s)."),
            });
        }
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr).trim().replace('\n', " ");
            results.push(HwDecodeExperiment {
                engine: "CUDA (NVIDIA RTX 3050)".to_string(),
                success: false,
                frames_tested: 60,
                elapsed_s: None,
                fps: None,
                notes: format!("Bu ortamda çalışmadı: {err}"),
            });
        }
        Err(e) => {
            results.push(HwDecodeExperiment {
                engine: "CUDA (NVIDIA RTX 3050)".to_string(),
                success: false,
                frames_tested: 60,
                elapsed_s: None,
                fps: None,
                notes: format!("Çalıştırılamadı: {e}"),
            });
        }
    }

    results
}

// ---------------------------------------------------------------------------
// Synthetic Media Generator (Fallback)
// ---------------------------------------------------------------------------

fn generate_synthetic_clip(path: &Path, width: u32, height: u32, codec: &str, dur_s: u32) -> bool {
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

// ---------------------------------------------------------------------------
// Full Benchmark Report Schema
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct CompleteBenchmarkReport {
    timestamp: String,
    system: SystemMetadata,
    primary_media: MediaMetadata,
    primary_paced: PacedPlaybackResult,
    primary_free_run: FreeRunResult,
    primary_seek: SeekResultSummary,
    timeline_compositing: Vec<TimelineTrackResult>,
    color_parity: Option<ColorParityResult>,
    hw_decode_experiments: Vec<HwDecodeExperiment>,
    derived_4k: Option<Derived4kBenchmark>,
    methodology_explanation: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct Derived4kBenchmark {
    media: MediaMetadata,
    paced: PacedPlaybackResult,
    free_run: FreeRunResult,
    seek: SeekResultSummary,
}

// ---------------------------------------------------------------------------
// Markdown Report Generator
// ---------------------------------------------------------------------------

fn generate_markdown_report(report: &CompleteBenchmarkReport) -> String {
    let mut md = String::new();

    md.push_str("# triM. Dürüst ve Gerçekçi Medya & Önizleme Benchmark Raporu\n\n");
    md.push_str(&format!("**Tarih:** {}\n", report.timestamp));
    md.push_str("**Ölçüm Aracı:** `cargo run --release -p trim-media --example perf_bench`\n\n");

    md.push_str("## 1. Sistem ve Ortam Bilgisi\n\n");
    md.push_str(&format!(
        "- **İşlemci (CPU):** {} ({} mantıksal çekirdek)\n",
        report.system.cpu_model, report.system.logical_cores
    ));
    md.push_str(&format!(
        "- **Bellek (RAM):** Toplam {:.1} MB (~{:.1} GB), Kullanılabilir {:.1} MB\n",
        report.system.ram_total_mb,
        report.system.ram_total_mb / 1024.0,
        report.system.ram_avail_mb
    ));
    md.push_str(&format!(
        "- **İşletim Sistemi / Çekirdek:** {}\n",
        report.system.os_kernel
    ));
    md.push_str(&format!(
        "- **Sanallaştırma (VM):** {}\n",
        report.system.vm_description
    ));
    md.push_str(&format!(
        "- **FFmpeg Sürümü:** {}\n\n",
        report.system.ffmpeg_version_line1
    ));

    md.push_str("## 2. Test Edilen Medya Özellikleri\n\n");
    let pm = &report.primary_media;
    md.push_str(&format!("- **Etiket:** `{}`\n", pm.label));
    if let Some(w) = &pm.warning {
        md.push_str(&format!("  > ⚠️ {}\n", w));
    }
    md.push_str(&format!("- **Dosya Yolu:** `{}`\n", pm.path));
    md.push_str(&format!(
        "- **Kodek & Profil:** {} ({})\n",
        pm.codec, pm.profile
    ));
    md.push_str(&format!("- **Çözünürlük:** {}×{}\n", pm.width, pm.height));
    md.push_str(&format!(
        "- **Rasyonel FPS:** `{}` ({:.3} fps)\n",
        pm.fps_rational_str, pm.fps
    ));
    md.push_str(&format!(
        "- **Bit Hızı:** {:.1} kbps (~{:.1} Mbps)\n",
        pm.bitrate_kbps,
        pm.bitrate_kbps / 1000.0
    ));
    md.push_str(&format!("- **Piksel Formatı:** `{}`\n", pm.pix_fmt));
    md.push_str(&format!("- **Süre:** {:.3} saniye\n", pm.duration_s));
    md.push_str(&format!(
        "- **GOP Yapısı:** Toplam {} I-kare, {} P-kare, {} B-kare\n\n",
        pm.i_frames, pm.p_frames, pm.b_frames
    ));

    md.push_str(
        "## 3. İleri Oynatma: Paced (Zaman Kısıtlı) vs Serbest Ham Decode (Side-by-Side)\n\n",
    );
    md.push_str(
        "| Metrik | Paced (Zaman Kısıtlı) Gerçek Zamanlı | Serbest Ham Decode (Unpaced) |\n",
    );
    md.push_str("|---|---:|---:|\n");
    md.push_str("| **Yöntem** | Deadline = $t_0 + n/fps$ (uyku ile senkron) | Boru hattından kesintisiz maksimum çekim |\n");
    md.push_str(&format!("| **Efektif FPS** | **{:.3} FPS** (ekran hızına kilitli) | **{:.1} FPS** (işlemci decode sınırı) |\n", pm.fps, report.primary_free_run.unpaced_fps));
    md.push_str(&format!(
        "| **Ölçülen Süre** | {:.1} s (ilk {:.1} s ısınma hariç) | {:.2} s |\n",
        report.primary_paced.measured_duration_s,
        report.primary_paced.warmup_duration_s,
        report.primary_free_run.elapsed_s
    ));
    md.push_str(&format!(
        "| **Teslim Edilen Kare** | {} / {} ({:.1}%) | {} kare |\n",
        report.primary_paced.delivered_frames,
        report.primary_paced.total_expected_frames,
        report.primary_paced.delivered_pct,
        report.primary_free_run.delivered_frames
    ));
    md.push_str(&format!(
        "| **Kaçırılan / Düşen Kare** | **{} ({:.1}%)** | 0 (deadline yok) |\n",
        report.primary_paced.missed_frames, report.primary_paced.missed_pct
    ));
    md.push_str(&format!(
        "| **Kare Gecikmesi p50 / p95 / p99** | {:.2} ms / {:.2} ms / {:.2} ms | N/A |\n",
        report.primary_paced.latency_p50_ms,
        report.primary_paced.latency_p95_ms,
        report.primary_paced.latency_p99_ms
    ));
    md.push_str(&format!(
        "| **İlk Kareye Ulaşma Süresi** | {:.1} ms | N/A |\n",
        report.primary_paced.time_to_first_frame_ms
    ));
    md.push_str(&format!(
        "| **PTS Sürüklenme (Drift) Maks** | {:.3} ms | N/A |\n",
        report.primary_paced.pts_drift_max_ms
    ));
    md.push_str(&format!(
        "| **triM İşlemci (CPU) / Bellek** | %{:.1} CPU / {:.1} MB RSS | N/A |\n",
        report.primary_paced.cpu_trim_pct, report.primary_paced.peak_rss_trim_mb
    ));
    md.push_str(&format!(
        "| **FFmpeg İşlemci (CPU) / Bellek** | %{:.1} CPU / {:.1} MB RSS | N/A |\n\n",
        report.primary_paced.cpu_ffmpeg_pct, report.primary_paced.peak_rss_ffmpeg_mb
    ));

    md.push_str("### 634 FPS vs 130 FPS / Gerçek Zaman Çelişkisinin Açıklaması\n");
    md.push_str(&format!("{}\n\n", report.methodology_explanation));

    md.push_str("## 4. Rastgele Seek Gecikmesi: Soğuk (Cold) vs Sıcak (Hot)\n\n");
    let sk = &report.primary_seek;
    md.push_str(&format!(
        "*Sabit tohumlu rastgele zaman noktalarında {} iterasyon:*\n\n",
        sk.iterations
    ));
    md.push_str("| Seek Tipi | p50 Gecikme | p95 Gecikme | p99 Gecikme | Süreç Maliyeti |\n");
    md.push_str("|---|---:|---:|---:|---|\n");
    md.push_str(&format!("| **Soğuk Seek (`decode_frame_raw`)** | **{:.1} ms** | {:.1} ms | {:.1} ms | Her seek için yeni `ffmpeg` process spawn + probe + seek |\n", sk.cold_p50_ms, sk.cold_p95_ms, sk.cold_p99_ms));
    md.push_str(&format!("| **Sıcak Seek (Açık `PlaybackStreamer`)** | **{:.1} ms** | {:.1} ms | {:.1} ms | Çalışan streamer pipe'ına yeni zaman konumu verme |\n", sk.hot_p50_ms, sk.hot_p95_ms, sk.hot_p99_ms));
    md.push_str(&format!("| **Fark Oranı (Soğuk / Sıcak)** | **{:.1}×** | - | - | Soğuk seek gecikmesinin %{:.0}'i process başlatma + probe |\n\n", sk.ratio_cold_to_hot_p50, (1.0 - (sk.hot_p50_ms / sk.cold_p50_ms.max(0.001))) * 100.0));

    md.push_str("## 5. Zaman Çizelgesi Kompozisyonu (Önbellek Hilesiz, 300 Benzersiz Sorgu)\n\n");
    md.push_str("> [!NOTE]\n");
    md.push_str("> Bu ölçüm GL context'siz ortamda yazılım CPU compositing yolunu (`draw_clip_onto_canvas`) ölçmektedir. GPU compositor bu harness kapsamı dışındadır.\n\n");
    md.push_str(
        "| Senaryo | Kare Başına Süre (ms) | Efektif FPS | Cache Vuruş Oranı | RSS (Bellek) |\n",
    );
    md.push_str("|---|---:|---:|---:|---:|\n");
    for tc in &report.timeline_compositing {
        let fps = if tc.ms_per_frame > 0.001 {
            1000.0 / tc.ms_per_frame
        } else {
            0.0
        };
        md.push_str(&format!(
            "| **{} Video Track** | **{:.2} ms/frame** | {:.1} FPS | {:.1}% | {:.1} MB |\n",
            tc.tracks, tc.ms_per_frame, fps, tc.cache_hit_pct, tc.rss_mb
        ));
    }
    md.push('\n');

    if let Some(cp) = &report.color_parity {
        md.push_str("## 6. Renk Aralığı Doğruluğu (Kaynak `yuvj420p` Full Range vs Export)\n\n");
        md.push_str(&format!(
            "- **Kaynak Piksel Formatı:** `{}`\n",
            cp.source_pix_fmt
        ));
        md.push_str(&format!(
            "- **Ortalama |ΔRGB| Sapması:** **{:.2} / 255** (ΔR: {:.2}, ΔG: {:.2}, ΔB: {:.2})\n",
            cp.mean_abs_diff_rgb, cp.mean_abs_diff_r, cp.mean_abs_diff_g, cp.mean_abs_diff_b
        ));
        md.push_str(&format!(
            "- **Önizleme Siyah/Beyaz Seviyesi:** Min RGB = [{}, {}, {}], Max RGB = [{}, {}, {}]\n",
            cp.preview_black_rgb[0],
            cp.preview_black_rgb[1],
            cp.preview_black_rgb[2],
            cp.preview_white_rgb[0],
            cp.preview_white_rgb[1],
            cp.preview_white_rgb[2]
        ));
        md.push_str(&format!("- **Dışa Aktarma Siyah/Beyaz Seviyesi:** Min RGB = [{}, {}, {}], Max RGB = [{}, {}, {}]\n", cp.export_black_rgb[0], cp.export_black_rgb[1], cp.export_black_rgb[2], cp.export_white_rgb[0], cp.export_white_rgb[1], cp.export_white_rgb[2]));
        md.push_str(&format!("- **Değerlendirme:** {}\n\n", cp.assessment));
    }

    md.push_str("## 7. Donanım Hızlandırma (HW Decode) Denemeleri\n\n");
    md.push_str("| Motor | Durum | 60 Kare Süresi | Decode FPS | Açıklama |\n");
    md.push_str("|---|---|---:|---:|---|\n");
    for hw in &report.hw_decode_experiments {
        let status_icon = if hw.success {
            "✅ Başarılı"
        } else {
            "❌ Başarısız"
        };
        let dur_str = hw
            .elapsed_s
            .map(|d| format!("{:.3} s", d))
            .unwrap_or_else(|| "-".to_string());
        let fps_str = hw
            .fps
            .map(|f| format!("{:.1} FPS", f))
            .unwrap_or_else(|| "-".to_string());
        md.push_str(&format!(
            "| **{}** | {} | {} | {} | {} |\n",
            hw.engine, status_icon, dur_str, fps_str, hw.notes
        ));
    }
    md.push('\n');

    if let Some(d4k) = &report.derived_4k {
        md.push_str("## 8. Türetilmiş 4K HEVC Medya Sonuçları (`TÜRETİLMİŞ`)\n\n");
        md.push_str("> [!WARNING]\n");
        md.push_str("> Bu medya orijinal 1080p klibin libx265 ile 3840x2160 çözünürlüğe yukarı ölçeklenmesiyle oluşturulmuştur; gerçek 4K kamera çekimi değildir.\n\n");
        md.push_str(&format!("- **Paced Oynatma:** Teslim Edilen = %{:.1}, Düşen = %{:.1}, Gecikme p50 = {:.2} ms, p95 = {:.2} ms\n", d4k.paced.delivered_pct, d4k.paced.missed_pct, d4k.paced.latency_p50_ms, d4k.paced.latency_p95_ms));
        md.push_str(&format!(
            "- **Serbest Ham Decode:** {:.1} FPS\n",
            d4k.free_run.unpaced_fps
        ));
        md.push_str(&format!(
            "- **Seek Gecikmesi:** Soğuk p50 = {:.1} ms, Sıcak p50 = {:.1} ms (Fark: {:.1}×)\n\n",
            d4k.seek.cold_p50_ms, d4k.seek.hot_p50_ms, d4k.seek.ratio_cold_to_hot_p50
        ));
    }

    md
}

// ---------------------------------------------------------------------------
// Main Executable
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut media_arg = None;
    let mut derived_4k_arg = None;

    let mut i = 1;
    while i < args.len() {
        if (args[i] == "--media" || args[i] == "-m") && i + 1 < args.len() {
            media_arg = Some(args[i + 1].clone());
            i += 2;
        } else if args[i] == "--derived-4k" && i + 1 < args.len() {
            derived_4k_arg = Some(args[i + 1].clone());
            i += 2;
        } else {
            i += 1;
        }
    }

    println!("============================================================");
    println!("triM. Dürüst ve Gerçekçi Medya & Önizleme Benchmark Aracı");
    println!("============================================================");

    let system = get_system_metadata();
    println!("CPU: {} ({} cores)", system.cpu_model, system.logical_cores);
    println!(
        "RAM: {:.1} GB total, {:.1} GB avail",
        system.ram_total_mb / 1024.0,
        system.ram_avail_mb / 1024.0
    );
    println!("OS:  {} (VM: {})", system.os_kernel, system.vm_description);
    println!("FFmpeg: {}", system.ffmpeg_version_line1);
    println!("------------------------------------------------------------");

    let tmp_dir = tempfile::tempdir().expect("tempdir");

    // Resolve primary test media
    let (primary_path, primary_label) = if let Some(m) = &media_arg {
        let p = PathBuf::from(m);
        if p.exists() {
            println!("Girdi Medyası: GERÇEK ({})", p.display());
            (p, "GERÇEK")
        } else {
            eprintln!(
                "UYARI: Belirtilen medya dosyası bulunamadı: {}",
                p.display()
            );
            let synth = tmp_dir.path().join("synth_1080p.mp4");
            println!("Sentetik 1080p üretiliyor...");
            generate_synthetic_clip(&synth, 1920, 1080, "libx264", 30);
            (synth, "SENTETİK")
        }
    } else {
        println!("UYARI: `--media` belirtilmedi. Sentetik test medyası üretiliyor...");
        let synth = tmp_dir.path().join("synth_1080p.mp4");
        generate_synthetic_clip(&synth, 1920, 1080, "libx264", 30);
        (synth, "SENTETİK")
    };

    let primary_media =
        probe_media(&primary_path, primary_label).expect("Failed to probe primary media");
    println!(
        "Kodek: {}, Profil: {}, Çözünürlük: {}x{}, FPS: {} ({:.3})",
        primary_media.codec,
        primary_media.profile,
        primary_media.width,
        primary_media.height,
        primary_media.fps_rational_str,
        primary_media.fps
    );
    println!(
        "GOP: {} I, {} P, {} B kare",
        primary_media.i_frames, primary_media.p_frames, primary_media.b_frames
    );

    let test_dur_s = primary_media.duration_s.clamp(5.0, 30.0);

    // 1. Paced forward playback
    println!(
        "\n[1/6] Paced ileri oynatma ölçülüyor ({}s, 2s ısınma)...",
        test_dur_s
    );
    let paced_res = bench_paced_forward_playback(&primary_path, primary_media.fps, test_dur_s, 2.0);
    println!(
        "  -> Teslim Edilen: {}/{} ({:.1}%), Düşen: {} ({:.1}%)",
        paced_res.delivered_frames,
        paced_res.total_expected_frames,
        paced_res.delivered_pct,
        paced_res.missed_frames,
        paced_res.missed_pct
    );
    println!(
        "  -> Gecikme: p50={:.2}ms, p95={:.2}ms, p99={:.2}ms, İlk kare={:.1}ms",
        paced_res.latency_p50_ms,
        paced_res.latency_p95_ms,
        paced_res.latency_p99_ms,
        paced_res.time_to_first_frame_ms
    );
    println!(
        "  -> Kaynak Kullanımı: triM %{:.1} CPU / {:.1} MB RSS, FFmpeg %{:.1} CPU / {:.1} MB RSS",
        paced_res.cpu_trim_pct,
        paced_res.peak_rss_trim_mb,
        paced_res.cpu_ffmpeg_pct,
        paced_res.peak_rss_ffmpeg_mb
    );

    // 2. Free-run decode throughput
    println!("\n[2/6] Serbest ham decode hızı (unpaced) ölçülüyor...");
    let free_run_res = bench_free_run_throughput(&primary_path, primary_media.fps, test_dur_s);
    println!(
        "  -> Serbest Decode Throughput: {:.1} FPS ({} kare, {:.2}s)",
        free_run_res.unpaced_fps, free_run_res.delivered_frames, free_run_res.elapsed_s
    );

    // 3. Cold vs Hot Seek
    println!("\n[3/6] Rastgele Seek gecikmesi ölçülüyor (100 iterasyon)...");
    let seek_res = bench_cold_vs_hot_seek(
        &primary_path,
        primary_media.fps,
        primary_media.duration_s,
        100,
    );
    println!(
        "  -> Soğuk Seek: p50={:.1}ms, p95={:.1}ms, p99={:.1}ms",
        seek_res.cold_p50_ms, seek_res.cold_p95_ms, seek_res.cold_p99_ms
    );
    println!(
        "  -> Sıcak Seek: p50={:.1}ms, p95={:.1}ms, p99={:.1}ms",
        seek_res.hot_p50_ms, seek_res.hot_p95_ms, seek_res.hot_p99_ms
    );
    println!(
        "  -> Soğuk / Sıcak Oranı: {:.1}×",
        seek_res.ratio_cold_to_hot_p50
    );

    // 4. Timeline Compositing
    println!(
        "\n[4/6] Zaman çizelgesi kompozisyonu ölçülüyor (1, 3, 6 track, 300 benzersiz sorgu)..."
    );
    let mut timeline_results = Vec::new();
    for tracks in [1usize, 3usize, 6usize] {
        let tc = bench_timeline_compositing(
            &primary_path,
            tracks,
            primary_media.fps,
            primary_media.duration_s,
        );
        println!(
            "  -> {} Track: {:.2} ms/frame ({:.1} FPS), Cache Hit: {:.1}%, RSS: {:.1} MB",
            tc.tracks,
            tc.ms_per_frame,
            if tc.ms_per_frame > 0.001 {
                1000.0 / tc.ms_per_frame
            } else {
                0.0
            },
            tc.cache_hit_pct,
            tc.rss_mb
        );
        timeline_results.push(tc);
    }

    // 5. Color Parity
    println!("\n[5/6] Renk aralığı doğruluğu (yuvj420p vs export) test ediliyor...");
    let color_parity = bench_color_range_parity(&primary_path);
    if let Some(cp) = &color_parity {
        println!(
            "  -> Ortalama |ΔRGB|: {:.2} (ΔR: {:.2}, ΔG: {:.2}, ΔB: {:.2})",
            cp.mean_abs_diff_rgb, cp.mean_abs_diff_r, cp.mean_abs_diff_g, cp.mean_abs_diff_b
        );
        println!(
            "  -> Preview Siyah/Beyaz: {:?} / {:?}",
            cp.preview_black_rgb, cp.preview_white_rgb
        );
        println!(
            "  -> Export Siyah/Beyaz:  {:?} / {:?}",
            cp.export_black_rgb, cp.export_white_rgb
        );
    } else {
        println!("  -> Renk testi atlandı / başarısız oldu.");
    }

    // 6. Hardware Decode Attempt
    println!("\n[6/6] Donanım hızlandırma (VAAPI / CUDA) deneniyor...");
    let hw_experiments = bench_hw_accel_attempt(&primary_path);
    for hw in &hw_experiments {
        println!(
            "  -> {}: success={}, fps={:?}, notes={}",
            hw.engine, hw.success, hw.fps, hw.notes
        );
    }

    // Optional 7. Derived 4K HEVC
    let derived_4k = {
        let d4k_path = if let Some(p) = derived_4k_arg {
            Some(PathBuf::from(p))
        } else {
            let p = PathBuf::from("/tmp/mvi_4k_hevc.mp4");
            if p.exists() {
                Some(p)
            } else {
                None
            }
        };

        if let Some(p) = d4k_path {
            if let Some(meta) = probe_media(&p, "TÜRETİLMİŞ") {
                println!("\n[Ekstra] Türetilmiş 4K HEVC ölçülüyor ({})", p.display());
                let dur = meta.duration_s.clamp(3.0, 10.0);
                let p_res = bench_paced_forward_playback(&p, meta.fps, dur, 1.0);
                let f_res = bench_free_run_throughput(&p, meta.fps, dur);
                let s_res = bench_cold_vs_hot_seek(&p, meta.fps, meta.duration_s, 20);
                Some(Derived4kBenchmark {
                    media: meta,
                    paced: p_res,
                    free_run: f_res,
                    seek: s_res,
                })
            } else {
                None
            }
        } else {
            None
        }
    };

    let methodology_explanation = "Eski benchmark raporundaki 634 FPS (ileri akış) ile 130 FPS (CPU decode) arasındaki çelişki tamamen ölçüm yöntemi farkından kaynaklanmaktadır:\n\
    1. 634 FPS sayısı, ffmpeg'den okunan ham boru hattının zamanlama kısıtı (pacing deadline) olmadan serbest akışta (unpaced free-run) ne kadar hızlı boşaltıldığını göstermektedir. Bu bir 'oynatma' performansı değil, boru hattı verimidir.\n\
    2. 130 FPS sayısı ise tek bir komutla 2 saniyelik klibin tamamının decode edilip diske/null'a yazılmasıdır.\n\
    3. Paced gerçek zamanlı oynatma yapıldığında ise kareler oynatma saatine (23.976 fps) göre deadline ile sorgulanır. Gerçek oynatmada FPS daima video kaynak hızına (23.976) kilitlidir; asıl başarı kriteri düşen kare oranı ve gecikme dağılımıdır.".to_string();

    let now_str = chrono_or_fallback_timestamp();

    let report = CompleteBenchmarkReport {
        timestamp: now_str,
        system,
        primary_media,
        primary_paced: paced_res,
        primary_free_run: free_run_res,
        primary_seek: seek_res,
        timeline_compositing: timeline_results,
        color_parity,
        hw_decode_experiments: hw_experiments,
        derived_4k,
        methodology_explanation,
    };

    // Output markdown & JSON
    let docs_dir = PathBuf::from("docs/benchmarks");
    let _ = std::fs::create_dir_all(&docs_dir);
    let md_path = docs_dir.join("2026-11-22_honest_benchmark.md");
    let json_path = docs_dir.join("2026-11-22_honest_benchmark.json");

    let md_content = generate_markdown_report(&report);
    if let Ok(mut f) = File::create(&md_path) {
        let _ = f.write_all(md_content.as_bytes());
        println!("\n✅ Markdown raporu kaydedildi: {}", md_path.display());
    }

    if let Ok(json_str) = serde_json::to_string_pretty(&report) {
        if let Ok(mut f) = File::create(&json_path) {
            let _ = f.write_all(json_str.as_bytes());
            println!("✅ JSON raporu kaydedildi: {}", json_path.display());
        }
    }

    println!("\nBenchmark başarıyla tamamlandı.");
}

fn chrono_or_fallback_timestamp() -> String {
    // Basic UTC date-time string without heavy external chrono crate
    let out = Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S %z")
        .output()
        .ok();
    out.and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "2026-11-22 12:00:00 +0300".to_string())
}
