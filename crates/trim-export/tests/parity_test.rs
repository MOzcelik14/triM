use trim_core::{Clip, LutPreset, MediaItem, MediaType, Project};
use trim_export::{ExportPreset, HardwareAccel, LoudnessNorm, TimelineExporter};
use trim_media::ffmpeg::{ffmpeg_cmd, is_ffmpeg_available};

#[test]
fn test_preview_export_color_parity_within_measured_threshold() {
    if !is_ffmpeg_available() || !trim_media::ffmpeg::is_libx264_available() {
        return;
    }

    let tmp = tempfile::tempdir().expect("tempdir");
    let src_png = tmp.path().join("ramp.png");
    let out_mp4 = tmp.path().join("graded_out.mp4");

    // Generate a 64x64 test ramp image via FFmpeg
    let gen = ffmpeg_cmd()
        .args([
            "-y",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "nullsrc=s=64x64:r=25:d=0.2,format=rgb24,geq=r='X*4':g='Y*4':b='(X+Y)*2'",
            "-frames:v",
            "1",
        ])
        .arg(&src_png)
        .output()
        .expect("generate ramp.png");
    assert!(gen.status.success(), "ramp.png generation failed");

    let src_dec = ffmpeg_cmd()
        .args(["-v", "error", "-i"])
        .arg(&src_png)
        .args(["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
        .output()
        .expect("decode ramp.png");
    assert!(src_dec.status.success());
    assert_eq!(src_dec.stdout.len(), 64 * 64 * 3);

    let mut proj = Project::default();
    proj.settings.width = 64;
    proj.settings.height = 64;
    proj.settings.fps = 25.0;

    let mut media = MediaItem::new(&src_png);
    media.media_type = MediaType::Image;
    media.duration = 1.0;
    media.width = 64;
    media.height = 64;
    let mid = media.id.clone();
    proj.add_media(media);

    let mut clip = Clip::new(&mid, 0.0, 0.5);
    clip.brightness = 0.06;
    clip.contrast = 1.15;
    clip.saturation = 1.18;
    clip.gamma = 1.05;
    clip.highlight_rolloff = 0.2;
    clip.lut_preset = LutPreset::TealOrange;

    // (a) CPU preview reference output
    let mut cpu_ref = Vec::with_capacity(64 * 64);
    for px in src_dec.stdout.as_chunks::<3>().0 {
        let [r, g, b] = clip.evaluate_color_pipeline_rgb(
            px[0] as f32 / 255.0,
            px[1] as f32 / 255.0,
            px[2] as f32 / 255.0,
        );
        cpu_ref.push([
            (r * 255.0).round().clamp(0.0, 255.0) as f64,
            (g * 255.0).round().clamp(0.0, 255.0) as f64,
            (b * 255.0).round().clamp(0.0, 255.0) as f64,
        ]);
    }

    proj.timeline.tracks[0].add_clip(clip);

    // (b) FFmpeg export output (using baked 33^3 .cube LUT)
    let preset = ExportPreset {
        width: Some(64),
        height: Some(64),
        fps: Some(25.0),
        crf: 10,
        preset: "ultrafast".into(),
        hw_accel: HardwareAccel::Cpu,
        loudness_norm: LoudnessNorm::Off,
        ..Default::default()
    };
    let (cmd, _) =
        TimelineExporter::build_export_pipeline(&proj, &preset, out_mp4.to_str().expect("utf8"))
            .expect("build export pipeline");
    let exp = trim_media::create_command(&cmd[0])
        .args(&cmd[1..])
        .output()
        .expect("run ffmpeg export");
    assert!(
        exp.status.success(),
        "export failed: {}",
        String::from_utf8_lossy(&exp.stderr)
    );

    // Decode exported frame back to RGB24
    let dec = ffmpeg_cmd()
        .args(["-v", "error", "-i"])
        .arg(&out_mp4)
        .args(["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
        .output()
        .expect("decode exported frame");
    assert!(dec.status.success(), "decode failed");
    assert_eq!(dec.stdout.len(), 64 * 64 * 3);

    let mut total_abs_diff = 0.0f64;
    for (idx, chunk) in dec.stdout.as_chunks::<3>().0.iter().enumerate() {
        let [rr, rg, rb] = cpu_ref[idx];
        total_abs_diff += (chunk[0] as f64 - rr).abs();
        total_abs_diff += (chunk[1] as f64 - rg).abs();
        total_abs_diff += (chunk[2] as f64 - rb).abs();
    }
    let mean_abs_diff = total_abs_diff / (64.0 * 64.0 * 3.0);
    // Measured CPU reference vs FFmpeg lut3d + YUV420p export parity threshold: <= 8.0 / 255
    assert!(
        mean_abs_diff <= 8.0,
        "Preview/Export color parity drift too high: mean |ΔRGB| = {mean_abs_diff:.2}"
    );
}

#[test]
fn test_roll_and_slide_trim_export_parity() {
    if !is_ffmpeg_available() || !trim_media::ffmpeg::is_libx264_available() {
        return;
    }

    let tmp = tempfile::tempdir().expect("tempdir");
    let red_mp4 = tmp.path().join("red.mp4");
    let blue_mp4 = tmp.path().join("blue.mp4");
    let out_mp4 = tmp.path().join("rolled_out.mp4");

    for (color, dst) in [("red", &red_mp4), ("blue", &blue_mp4)] {
        let gen = ffmpeg_cmd()
            .args([
                "-y",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                &format!("color=c={color}:s=64x64:r=25:d=3"),
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(dst)
            .output()
            .expect("generate color clip");
        assert!(gen.status.success());
    }

    let mut proj = Project::default();
    proj.settings.width = 64;
    proj.settings.height = 64;
    proj.settings.fps = 25.0;
    proj.timeline.width = 64;
    proj.timeline.height = 64;
    proj.timeline.fps = 25.0;

    let mut m_red = MediaItem::new(&red_mp4);
    m_red.media_type = trim_core::MediaType::Video;
    m_red.duration = 3.0;
    m_red.width = 64;
    m_red.height = 64;
    let id_red = m_red.id.clone();

    let mut m_blue = MediaItem::new(&blue_mp4);
    m_blue.media_type = trim_core::MediaType::Video;
    m_blue.duration = 3.0;
    m_blue.width = 64;
    m_blue.height = 64;
    let id_blue = m_blue.id.clone();

    proj.add_media(m_red);
    proj.add_media(m_blue);

    let mut c1 = Clip::new(&id_red, 0.0, 1.0);
    c1.timeline_in = 0.0;
    c1.timeline_out = 1.0;
    let c1_id = c1.id.clone();

    let mut c2 = Clip::new(&id_blue, 1.0, 2.0);
    c2.source_in = 0.5;
    c2.source_out = Some(1.5);
    c2.timeline_in = 1.0;
    c2.timeline_out = 2.0;
    let c2_id = c2.id.clone();

    let tr_id = proj.timeline.tracks[0].id.clone();
    proj.timeline.tracks[0].add_clip(c1);
    proj.timeline.tracks[0].add_clip(c2);

    // Roll edit point from 1.0s to 1.5s (+0.5s) via RollTrimCommand
    let mut stack = trim_commands::UndoStack::new();
    stack
        .push(
            Box::new(trim_commands::RollTrimCommand::new(
                &tr_id, &c1_id, &c2_id, 0.5,
            )),
            &mut proj.timeline,
        )
        .expect("roll trim");
    assert!((proj.timeline.duration() - 2.0).abs() < 1e-6);

    let preset = ExportPreset {
        width: Some(64),
        height: Some(64),
        fps: Some(25.0),
        crf: 18,
        preset: "ultrafast".into(),
        hw_accel: HardwareAccel::Cpu,
        loudness_norm: LoudnessNorm::Off,
        ..Default::default()
    };
    let (cmd, _) =
        TimelineExporter::build_export_pipeline(&proj, &preset, out_mp4.to_str().expect("utf8"))
            .expect("build export pipeline");
    let exp = trim_media::create_command(&cmd[0])
        .args(&cmd[1..])
        .output()
        .expect("run ffmpeg export");
    assert!(
        exp.status.success(),
        "export failed: {}",
        String::from_utf8_lossy(&exp.stderr)
    );

    // Sample frame at t=1.2s (after old 1.0s cut, before rolled 1.5s cut) -> must still be RED!
    let sample_red = ffmpeg_cmd()
        .args(["-v", "error", "-ss", "1.2", "-i"])
        .arg(&out_mp4)
        .args(["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
        .output()
        .expect("decode t=1.2");
    assert!(sample_red.status.success());
    let px = &sample_red.stdout[0..3];
    assert!(
        px[0] > 180 && px[2] < 80,
        "Expected RED frame at t=1.2s after +0.5s roll trim, got RGB={px:?}"
    );
}
