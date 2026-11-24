//! Linux-only: cancelling a real FFmpeg export must kill the child process and delete the partial output.

#![cfg(target_os = "linux")]

use std::sync::mpsc::channel;
use std::time::{Duration, Instant};
use trim_core::{Clip, MediaItem, MediaType, Project};
use trim_export::{ExportMessage, ExportPreset, ExportRunner, HardwareAccel};
use trim_media::ffmpeg::{ffmpeg_cmd, is_ffmpeg_available};

/// PIDs of processes whose command line mentions `needle` (scans `/proc`).
fn processes_mentioning(needle: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir("/proc") else {
        return out;
    };
    for e in rd.flatten() {
        let Some(pid) = e.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else {
            continue;
        };
        if let Ok(raw) = std::fs::read(format!("/proc/{pid}/cmdline")) {
            if String::from_utf8_lossy(&raw).contains(needle) {
                out.push(pid);
            }
        }
    }
    out
}

#[test]
fn test_export_cancel_kills_ffmpeg_and_removes_partial_output() {
    if !is_ffmpeg_available() || !trim_media::ffmpeg::is_libx264_available() {
        return;
    }
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = tmp.path().join("src.mp4");
    let out = tmp.path().join("trim_cancel_test_out.mp4");

    let gen = ffmpeg_cmd()
        .args([
            "-y",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=1920x1080:rate=25:duration=30",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&src)
        .output()
        .expect("gen src");
    assert!(gen.status.success(), "source generation failed");

    let mut project = Project::default();
    project.settings.width = 1920;
    project.settings.height = 1080;
    project.settings.fps = 25.0;
    let mut m = MediaItem::new(&src);
    m.media_type = MediaType::Video;
    m.duration = 30.0;
    m.width = 1920;
    m.height = 1080;
    m.fps = 25.0;
    m.video_codec = "h264".into();
    let mid = m.id.clone();
    project.add_media(m);
    project.timeline.tracks[0].add_clip(Clip::new(&mid, 0.0, 30.0));

    // Deliberately slow encode so the export is still running when we cancel.
    let preset = ExportPreset {
        width: Some(1920),
        height: Some(1080),
        fps: Some(25.0),
        crf: 12,
        preset: "veryslow".into(),
        hw_accel: HardwareAccel::Cpu,
        ..ExportPreset::default()
    };

    let out_str = out.to_str().expect("utf8").to_string();
    let runner = ExportRunner::new();
    let (tx, rx) = channel();
    runner.start_export(project, preset, out_str.clone(), None, tx);

    // Wait until ffmpeg is demonstrably running for this export.
    let deadline = Instant::now() + Duration::from_secs(15);
    while processes_mentioning(&out_str).is_empty() {
        assert!(
            Instant::now() < deadline,
            "export ffmpeg never started for {out_str}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }

    runner.cancel();

    let mut finished = None;
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Ok(ExportMessage::Finished(r)) = rx.recv_timeout(Duration::from_millis(200)) {
            finished = Some(r);
            break;
        }
    }
    let result = finished.expect("export thread must report Finished after cancel");
    assert!(result.is_err(), "cancelled export must report Err");
    assert!(
        processes_mentioning(&out_str).is_empty(),
        "ffmpeg must be gone after cancel"
    );
    assert!(!out.exists(), "partial output must be deleted on cancel");
}
