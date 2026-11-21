use trim_core::{Clip, MediaItem, MediaType, Project};
use trim_export::{ExportPreset, HardwareAccel, TimelineExporter};
use trim_media::ffmpeg::{ffmpeg_cmd, ffprobe_cmd, is_ffmpeg_available, is_ffprobe_available};

#[test]
fn test_headless_e2e_two_clips_transition_and_audio_smoke() {
    if !is_ffmpeg_available() || !is_ffprobe_available() {
        return;
    }

    let tmp = tempfile::tempdir().expect("tempdir");
    let v1_path = tmp.path().join("clip1.mp4");
    let v2_path = tmp.path().join("clip2.mp4");
    let a1_path = tmp.path().join("tone.wav");
    let out_path = tmp.path().join("e2e_smoke_out.mp4");

    assert!(
        ffmpeg_cmd()
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=red:s=320x180:r=25:d=1.2",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&v1_path)
            .output()
            .expect("gen clip1")
            .status
            .success(),
        "clip1 generation failed"
    );
    assert!(
        ffmpeg_cmd()
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=blue:s=320x180:r=25:d=1.2",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&v2_path)
            .output()
            .expect("gen clip2")
            .status
            .success(),
        "clip2 generation failed"
    );
    assert!(
        ffmpeg_cmd()
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=48000:duration=2.0",
            ])
            .arg(&a1_path)
            .output()
            .expect("gen audio")
            .status
            .success(),
        "audio generation failed"
    );

    let mut project = Project::default();
    project.settings.width = 320;
    project.settings.height = 180;
    project.settings.fps = 25.0;

    let mut m_v1 = MediaItem::new(&v1_path);
    m_v1.media_type = MediaType::Video;
    m_v1.duration = 1.2;
    m_v1.width = 320;
    m_v1.height = 180;
    m_v1.fps = 25.0;
    m_v1.video_codec = "h264".into();

    let mut m_v2 = MediaItem::new(&v2_path);
    m_v2.media_type = MediaType::Video;
    m_v2.duration = 1.2;
    m_v2.width = 320;
    m_v2.height = 180;
    m_v2.fps = 25.0;
    m_v2.video_codec = "h264".into();

    let mut m_a1 = MediaItem::new(&a1_path);
    m_a1.media_type = MediaType::Audio;
    m_a1.duration = 2.0;
    m_a1.audio_channels = 1;
    m_a1.audio_sample_rate = 48000;
    m_a1.audio_codec = "pcm_s16le".into();

    let (v1_id, v2_id, a1_id) = (m_v1.id.clone(), m_v2.id.clone(), m_a1.id.clone());
    project.add_media(m_v1);
    project.add_media(m_v2);
    project.add_media(m_a1);

    let mut c1 = Clip::new(&v1_id, 0.0, 1.0);
    c1.transition_out = Some("crossfade".to_string());
    c1.fade_out = 0.25;

    let mut c2 = Clip::new(&v2_id, 1.0, 2.0);
    c2.transition_in = Some("crossfade".to_string());
    c2.fade_in = 0.25;

    let a1 = Clip::new(&a1_id, 0.0, 2.0);

    project.timeline.tracks[0].add_clip(c1);
    project.timeline.tracks[0].add_clip(c2);
    project
        .timeline
        .get_audio_tracks_mut()
        .into_iter()
        .next()
        .expect("audio track")
        .add_clip(a1);

    let preset = ExportPreset {
        width: Some(320),
        height: Some(180),
        fps: Some(25.0),
        preset: "ultrafast".into(),
        hw_accel: HardwareAccel::Cpu,
        ..ExportPreset::default()
    };

    let (cmd, _dur) = TimelineExporter::build_export_pipeline(
        &project,
        &preset,
        out_path.to_str().expect("utf8 path"),
    )
    .expect("build export pipeline");
    let out = trim_media::create_command(&cmd[0])
        .args(&cmd[1..])
        .output()
        .expect("run ffmpeg export");
    assert!(
        out.status.success(),
        "FFmpeg E2E smoke export failed:\ncmd: {:?}\n{}",
        cmd,
        String::from_utf8_lossy(&out.stderr)
    );

    let probe = ffprobe_cmd()
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration:stream=codec_type,width,height",
            "-of",
            "default=noprint_wrappers=1",
        ])
        .arg(&out_path)
        .output()
        .expect("ffprobe execution");
    assert!(probe.status.success(), "ffprobe failed");

    let text = String::from_utf8_lossy(&probe.stdout);
    let video_count = text.lines().filter(|l| *l == "codec_type=video").count();
    let audio_count = text.lines().filter(|l| *l == "codec_type=audio").count();
    assert_eq!(video_count, 1, "expected 1 video stream, got:\n{text}");
    assert_eq!(audio_count, 1, "expected 1 audio stream, got:\n{text}");
    assert!(text.contains("width=320"), "expected width=320:\n{text}");
    assert!(text.contains("height=180"), "expected height=180:\n{text}");

    let dur: f64 = text
        .lines()
        .find_map(|l| l.strip_prefix("duration="))
        .expect("duration line")
        .trim()
        .parse()
        .expect("duration f64");
    let one_frame = 1.0 / 25.0;
    assert!(
        (dur - 2.0).abs() <= one_frame + 0.02,
        "expected ~2.0s duration (±1 frame), got {dur}s"
    );
}
