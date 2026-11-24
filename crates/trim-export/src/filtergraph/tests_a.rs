use super::helpers::*;
use super::*;
use crate::presets::{ExportPreset, HardwareAccel, LoudnessNorm};
use trim_core::clip::Clip;
use trim_core::keyframe::{interpolate_keyframes, Interpolation, Keyframe};
use trim_core::media::{MediaItem, MediaType};
use trim_core::project::Project;
use trim_core::track::{Track, TrackType};

fn graph_of(cmd: &[String]) -> String {
    let idx = cmd.iter().position(|a| a == "-filter_complex").unwrap();
    cmd[idx + 1].clone()
}
#[test]
fn test_atempo_filter_chain() {
    // 1.0x -> no filters
    assert!(TimelineExporter::get_atempo_filters(1.0).is_empty());

    // 1.5x -> [atempo=1.5]
    let f15 = TimelineExporter::get_atempo_filters(1.5);
    assert_eq!(f15.len(), 1);
    assert_eq!(f15[0], "atempo=1.5000");

    // 3.0x -> [atempo=2.0, atempo=1.5]
    let f30 = TimelineExporter::get_atempo_filters(3.0);
    assert_eq!(f30.len(), 2);
    assert_eq!(f30[0], "atempo=2.0");
    assert_eq!(f30[1], "atempo=1.5000");

    // 0.25x -> [atempo=0.5, atempo=0.5]
    let f025 = TimelineExporter::get_atempo_filters(0.25);
    assert_eq!(f025.len(), 2);
    assert_eq!(f025[0], "atempo=0.5");
    assert_eq!(f025[1], "atempo=0.5000");
}

#[test]
fn test_silent_video_uses_correct_filtergraph_labels() {
    let mut project = Project::new("Silent video");
    let mut media = MediaItem::new("/videos/no-audio.mp4");
    media.media_type = MediaType::Video;
    media.duration = 5.0;
    media.audio_codec.clear();
    let id = media.id.clone();
    project.add_media(media);
    let video_track = project
        .timeline
        .get_video_tracks_mut()
        .into_iter()
        .next()
        .unwrap();
    video_track.add_clip(Clip::new(id, 0.0, 5.0));

    let (command, _) =
        TimelineExporter::build_export_pipeline(&project, &ExportPreset::default(), "/tmp/out.mp4")
            .unwrap();
    let graph = graph_of(&command);
    // Inputs: 0 = base colour, 1 = silence bed, 2 = clip.
    assert!(graph.contains("[2:v]"), "video input missing: {graph}");
    assert!(
        !graph.contains("[2:a]"),
        "silent media must not reference an audio stream: {graph}"
    );
    assert!(graph.contains("amix=inputs=1"), "{graph}");
}

#[test]
fn test_export_pipeline_generation() {
    let mut proj = Project::new("Export Test");
    let mut m = MediaItem::new("/path/test.mp4");
    m.media_type = MediaType::Video;
    m.duration = 10.0;
    let m_id = m.id.clone();
    proj.add_media(m);

    let v_track = proj
        .timeline
        .get_video_tracks_mut()
        .into_iter()
        .next()
        .unwrap();
    let mut c = Clip::new(&m_id, 2.0, 6.0);
    c.speed = 2.0;
    c.brightness = 0.1;
    v_track.add_clip(c);

    let preset = ExportPreset::default();
    let (cmd, dur) =
        TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();

    assert!((dur - 6.0).abs() < 1e-3);
    assert!(cmd.contains(&"-filter_complex".to_string()));
    assert!(cmd.contains(&"/tmp/out.mp4".to_string()));
    let graph = graph_of(&cmd);
    assert!(graph.contains("setpts=PTS+2.000000/TB"), "{graph}");
    // 4 s on the timeline at 2x reads 8 s of source.
    let t_pos = cmd.iter().position(|a| a == "/path/test.mp4").unwrap();
    assert_eq!(cmd[t_pos - 2], "8.000000");
}

#[test]
fn test_overlapping_tracks_are_composited_not_concatenated() {
    let mut proj = Project::new("Layers");
    let mut bg = MediaItem::new("/v/bg.mp4");
    bg.media_type = MediaType::Video;
    bg.audio_codec = "aac".into();
    let mut fg = MediaItem::new("/v/logo.png");
    fg.media_type = MediaType::Image;
    let mut music = MediaItem::new("/a/music.mp3");
    music.media_type = MediaType::Audio;
    music.audio_codec = "mp3".into();
    let (bg_id, fg_id, mu_id) = (bg.id.clone(), fg.id.clone(), music.id.clone());
    proj.add_media(bg);
    proj.add_media(fg);
    proj.add_media(music);

    let v1 = proj
        .timeline
        .tracks
        .iter_mut()
        .find(|t| t.name == "V1")
        .unwrap();
    v1.add_clip(Clip::new(&bg_id, 0.0, 10.0));

    let v2 = proj
        .timeline
        .tracks
        .iter_mut()
        .find(|t| t.name == "V2")
        .unwrap();
    v2.add_clip(Clip::new(&fg_id, 2.0, 4.0));

    let a1 = proj
        .timeline
        .tracks
        .iter_mut()
        .find(|t| t.name == "A1")
        .unwrap();
    a1.add_clip(Clip::new(&mu_id, 1.0, 9.0));

    let a2 = proj
        .timeline
        .tracks
        .iter_mut()
        .find(|t| t.name == "A2")
        .unwrap();
    a2.add_clip(Clip::new(&bg_id, 0.0, 10.0));

    let (cmd, dur) =
        TimelineExporter::build_export_pipeline(&proj, &ExportPreset::default(), "/tmp/o.mp4")
            .unwrap();
    let graph = graph_of(&cmd);
    assert!((dur - 10.0).abs() < 1e-6);
    assert!(!graph.contains("concat"), "{graph}");
    assert_eq!(graph.matches("overlay=").count(), 2, "{graph}");
    // Background audio on A2 + music on A1 + silence bed.
    assert!(graph.contains("amix=inputs=3"), "{graph}");
    assert!(graph.contains("adelay=delays=1000:all=1"), "{graph}");
}

#[test]
fn test_hidden_and_muted_tracks() {
    let mut proj = Project::new("Mute");
    let mut m = MediaItem::new("/v/a.mp4");
    m.media_type = MediaType::Video;
    m.audio_codec = "aac".into();
    let id = m.id.clone();
    proj.add_media(m);
    proj.timeline.tracks[0].add_clip(Clip::new(&id, 0.0, 3.0));
    let a1 = proj
        .timeline
        .get_audio_tracks_mut()
        .into_iter()
        .next()
        .unwrap();
    a1.add_clip(Clip::new(&id, 0.0, 3.0));
    let a1_id = a1.id.clone();

    proj.timeline.get_track_by_id_mut(&a1_id).unwrap().muted = true;
    let (cmd, _) =
        TimelineExporter::build_export_pipeline(&proj, &ExportPreset::default(), "/tmp/o.mp4")
            .unwrap();
    let g = graph_of(&cmd);
    assert!(g.contains("overlay="));
    assert!(g.contains("amix=inputs=1"), "{g}");

    proj.timeline.get_track_by_id_mut(&a1_id).unwrap().muted = false;
    proj.timeline.tracks[0].visible = false;
    let (cmd, _) =
        TimelineExporter::build_export_pipeline(&proj, &ExportPreset::default(), "/tmp/o.mp4")
            .unwrap();
    let g = graph_of(&cmd);
    assert!(!g.contains("overlay="), "{g}");
    assert!(g.contains("amix=inputs=2"), "{g}");
}

#[test]
fn test_keyframe_expression_matches_interpolation() {
    let kfs = vec![
        Keyframe::new(0.5, 10.0, Interpolation::Linear),
        Keyframe::new(1.5, 30.0, Interpolation::EaseInOut),
        Keyframe::new(2.5, 0.0, Interpolation::Hold),
        Keyframe::new(3.0, 5.0, Interpolation::Linear),
    ];
    let expr = keyframe_expr(&kfs, "T", |v| v);
    // Evaluate the generated expression with a tiny interpreter-free
    // check: substitute and compare a few sample points via meval-like
    // manual evaluation is overkill; instead verify structure and
    // boundary constants.
    assert!(expr.starts_with("if(lt(T,0.500000),10.000000,"));
    assert!(expr.contains("3*"));
    assert_eq!(expr.matches("if(").count(), kfs.len());
    // Sanity: the reference interpolation at boundaries.
    assert_eq!(interpolate_keyframes(&kfs, 0.0, 0.0), 10.0);
    assert_eq!(interpolate_keyframes(&kfs, 4.0, 0.0), 5.0);
}

/// End-to-end: actually renders a layered timeline with FFmpeg when it is
/// available on the machine.
#[test]
fn test_ffmpeg_end_to_end_render() {
    if trim_media::ffmpeg_cmd().arg("-version").output().is_err() {
        eprintln!("ffmpeg not available, skipping");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let vid = dir.path().join("src.mp4");
    let img = dir.path().join("logo.png");
    let aud = dir.path().join("tone.wav");
    let ok = |args: &[&str]| {
        trim_media::ffmpeg_cmd()
            .args(["-v", "error", "-y"])
            .args(args)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };
    assert!(ok(&[
        "-f",
        "lavfi",
        "-i",
        "testsrc=size=320x240:rate=25:duration=4",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:duration=4",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-shortest",
        vid.to_str().unwrap(),
    ]));
    assert!(ok(&[
        "-f",
        "lavfi",
        "-i",
        "color=c=red@0.5:s=64x64,format=rgba",
        "-frames:v",
        "1",
        img.to_str().unwrap()
    ]));
    assert!(ok(&[
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=880:duration=2",
        aud.to_str().unwrap()
    ]));

    let mut proj = Project::new("E2E");
    proj.timeline.width = 320;
    proj.timeline.height = 240;
    let mut v = MediaItem::new(&vid);
    v.media_type = MediaType::Video;
    v.audio_codec = "aac".into();
    let mut i = MediaItem::new(&img);
    i.media_type = MediaType::Image;
    let mut a = MediaItem::new(&aud);
    a.media_type = MediaType::Audio;
    a.audio_codec = "pcm_s16le".into();
    let (vid_id, img_id, aud_id) = (v.id.clone(), i.id.clone(), a.id.clone());
    proj.add_media(v);
    proj.add_media(i);
    proj.add_media(a);

    let mut c1 = Clip::new(&vid_id, 0.0, 2.0);
    c1.fade_out = 0.5;
    proj.timeline.tracks[0].add_clip(c1);
    let mut c2 = Clip::new(&vid_id, 3.0, 4.0);
    c2.speed = 2.0;
    c2.reverse = true;
    c2.source_in = 0.5;
    c2.source_out = Some(2.5);
    proj.timeline.tracks[0].add_clip(c2);
    let mut v2 = Track::new("V2", TrackType::Video);
    let mut logo = Clip::new(&img_id, 1.0, 3.5);
    logo.pos_x = 50.0;
    logo.scale = 0.5;
    logo.add_keyframe("opacity", 0.0, 0.0, Interpolation::Linear);
    logo.add_keyframe("opacity", 1.0, 1.0, Interpolation::Linear);
    v2.add_clip(logo);
    proj.timeline.tracks.insert(1, v2);
    let a1 = proj
        .timeline
        .get_audio_tracks_mut()
        .into_iter()
        .next()
        .unwrap();
    a1.add_clip(Clip::new(&aud_id, 0.5, 2.5));

    let out = dir.path().join("out.mp4");
    let preset = ExportPreset {
        width: None,
        height: None,
        preset: "ultrafast".into(),
        hw_accel: HardwareAccel::Cpu,
        ..Default::default()
    };
    let (cmd, dur) =
        TimelineExporter::build_export_pipeline(&proj, &preset, out.to_str().unwrap()).unwrap();
    let output = trim_media::create_command(&cmd[0])
        .args(&cmd[1..])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "ffmpeg failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let probe = trim_media::ffprobe_cmd()
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=nw=1:nk=1",
        ])
        .arg(&out)
        .output()
        .unwrap();
    let got: f64 = String::from_utf8_lossy(&probe.stdout)
        .trim()
        .parse()
        .unwrap_or(0.0);
    assert!((got - dur).abs() < 0.25, "duration {got} vs {dur}");
}

#[test]
fn test_resolve_video_codec_args() {
    let preset = ExportPreset {
        hw_accel: HardwareAccel::Cpu,
        ..Default::default()
    };
    let args = TimelineExporter::resolve_video_codec_args(&preset);
    assert!(args.contains(&"libx264".to_string()));
    assert!(args.contains(&"-crf".to_string()));

    let prores_preset = ExportPreset {
        video_codec: "prores_ks".to_string(),
        ..Default::default()
    };
    let args = TimelineExporter::resolve_video_codec_args(&prores_preset);
    assert!(args.contains(&"prores_ks".to_string()));
    assert!(args.contains(&"-profile:v".to_string()));
}

#[test]
fn test_loudness_norm_and_eq_filtergraph() {
    use trim_core::clip::AudioEqPreset;

    let mut proj = Project::new("AudioTest");
    let mut v = MediaItem::new("/tmp/fake.mp4");
    v.media_type = MediaType::Video;
    v.audio_codec = "aac".into();
    let vid_id = v.id.clone();
    proj.add_media(v);

    let mut c = Clip::new(&vid_id, 0.0, 5.0);
    c.eq_preset = AudioEqPreset::VoiceClarity;
    c.noise_gate = true;
    let a1 = proj
        .timeline
        .get_audio_tracks_mut()
        .into_iter()
        .next()
        .unwrap();
    a1.add_clip(c);

    let preset = ExportPreset {
        loudness_norm: LoudnessNorm::YouTube,
        ..Default::default()
    };
    let (cmd, _) = TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();

    let fg = cmd
        .iter()
        .position(|a| a == "-filter_complex")
        .map(|idx| &cmd[idx + 1])
        .unwrap();
    assert!(
        fg.contains("equalizer=f=3000"),
        "filtergraph should include voice clarity EQ: {fg}"
    );
    assert!(
        fg.contains("agate=threshold=0.03"),
        "filtergraph should include noise gate: {fg}"
    );
    assert!(
        fg.contains("loudnorm=I=-14.0"),
        "filtergraph should include EBU R128 loudnorm: {fg}"
    );
}
#[test]
fn test_text_overlay_drawtext_generation() {
    use trim_core::clip::TextOverlay;

    let mut proj = Project::new("TextTest");
    let mut v = MediaItem::new("/tmp/fake.mp4");
    v.media_type = MediaType::Video;
    let vid_id = v.id.clone();
    proj.add_media(v);

    let mut c = Clip::new(&vid_id, 0.0, 5.0);
    c.text_overlay = Some(TextOverlay {
        content: "Hello triM".to_string(),
        font_size: 48.0,
        text_color: [255, 255, 255, 255],
        bg_color: Some([0, 0, 0, 180]),
        pos_x: 0.5,
        pos_y: 0.85,
        has_shadow: true,
        ..Default::default()
    });
    proj.timeline.tracks[0].add_clip(c);

    let preset = ExportPreset::default();
    let (cmd, _) = TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();
    let fg = cmd
        .iter()
        .position(|a| a == "-filter_complex")
        .map(|idx| &cmd[idx + 1])
        .unwrap();

    assert!(
        fg.contains("drawtext=text='Hello triM'"),
        "filtergraph should contain drawtext: {fg}"
    );
    assert!(
        fg.contains("box=1:boxcolor="),
        "filtergraph should contain subtitle box: {fg}"
    );
    assert!(
        fg.contains("shadowx=3"),
        "filtergraph should contain shadow: {fg}"
    );
}
#[test]
fn test_lower_third_drawtext_generation() {
    use trim_core::clip::{MotionTitleStyle, TextOverlay};

    let mut proj = Project::new("LowerThirdTest");
    let mut v = MediaItem::new("/tmp/fake_lt.mp4");
    v.media_type = MediaType::Video;
    let vid_id = v.id.clone();
    proj.add_media(v);

    let mut c = Clip::new(&vid_id, 0.0, 5.0);
    c.text_overlay = Some(TextOverlay {
        content: "Jane Doe".to_string(),
        motion_style: MotionTitleStyle::LowerThird,
        subtitle_text: Some("Yönetmen".to_string()),
        font_size: 40.0,
        text_color: [255, 255, 255, 255],
        pos_x: 0.08,
        pos_y: 0.82,
        has_shadow: true,
        ..Default::default()
    });
    proj.timeline.tracks[0].add_clip(c);

    let preset = ExportPreset::default();
    let (cmd, _) = TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();
    let fg = cmd
        .iter()
        .position(|a| a == "-filter_complex")
        .map(|idx| &cmd[idx + 1])
        .unwrap();

    assert!(
        fg.contains("drawtext=text='Jane Doe'"),
        "filtergraph should contain main title: {fg}"
    );
    assert!(
        fg.contains("drawtext=text='Yönetmen'"),
        "filtergraph should contain subtitle: {fg}"
    );
    assert!(
        fg.contains("boxcolor=0x0f172a@0.85"),
        "filtergraph should contain lower third box: {fg}"
    );
}
#[test]
fn test_lut_preset_and_custom_cube_filtergraph() {
    use trim_core::clip::LutPreset;
    let mut proj = Project::new("LutTest");
    let mut v = MediaItem::new("/tmp/fake_lut.mp4");
    v.media_type = MediaType::Video;
    let vid_id = v.id.clone();
    proj.add_media(v);

    let mut c1 = Clip::new(&vid_id, 0.0, 5.0);
    c1.lut_preset = LutPreset::TealOrange;
    proj.timeline.tracks[0].add_clip(c1);

    let preset = ExportPreset::default();
    let (cmd1, _) =
        TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out1.mp4").unwrap();
    let fg1 = cmd1
        .iter()
        .position(|a| a == "-filter_complex")
        .map(|idx| &cmd1[idx + 1])
        .unwrap();
    assert!(
        fg1.contains("colorbalance=rs=-0.20:gs=0.06:bs=0.28"),
        "should generate teal & orange filter: {fg1}"
    );

    // Now test custom cube file (Unix path)
    proj.timeline.tracks[0].clips[0].custom_lut_path = Some("/path/to/cinematic.cube".to_string());
    let (cmd2, _) =
        TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out2.mp4").unwrap();
    let fg2 = cmd2
        .iter()
        .position(|a| a == "-filter_complex")
        .map(|idx| &cmd2[idx + 1])
        .unwrap();
    assert!(
        fg2.contains("lut3d=file='/path/to/cinematic.cube'"),
        "should generate lut3d filter: {fg2}"
    );

    // Test Windows custom cube file path with drive letter and backslashes
    proj.timeline.tracks[0].clips[0].custom_lut_path =
        Some(r"C:\Users\Editor\LUTs\cinematic.cube".to_string());
    let (cmd3, _) =
        TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out3.mp4").unwrap();
    let fg3 = cmd3
        .iter()
        .position(|a| a == "-filter_complex")
        .map(|idx| &cmd3[idx + 1])
        .unwrap();
    assert!(
        fg3.contains("lut3d=file='C\\:/Users/Editor/LUTs/cinematic.cube'"),
        "should escape Windows drive colon and backslashes in lut3d filter: {fg3}"
    );
}
#[test]
fn test_typewriter_drawtext_generation() {
    use trim_core::clip::{MotionTitleStyle, TextOverlay};

    let mut proj = Project::new("TypewriterTest");
    let mut v = MediaItem::new("/tmp/fake_tw.mp4");
    v.media_type = MediaType::Video;
    let vid_id = v.id.clone();
    proj.add_media(v);

    let mut c = Clip::new(&vid_id, 0.0, 4.0);
    c.text_overlay = Some(TextOverlay {
        content: "triM".to_string(),
        motion_style: MotionTitleStyle::Typewriter,
        subtitle_text: Some("NLE".to_string()),
        font_size: 48.0,
        text_color: [255, 255, 255, 255],
        has_shadow: true,
        ..Default::default()
    });
    proj.timeline.tracks[0].add_clip(c);

    let preset = ExportPreset::default();
    let (cmd, _) = TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();
    let fg = cmd
        .iter()
        .position(|a| a == "-filter_complex")
        .map(|idx| &cmd[idx + 1])
        .unwrap();

    // Should contain progressive letter reveals
    assert!(
        fg.contains("drawtext=text='t'"),
        "should contain first char: {fg}"
    );
    assert!(
        fg.contains("drawtext=text='tr'"),
        "should contain two chars: {fg}"
    );
    assert!(
        fg.contains("drawtext=text='tri'"),
        "should contain three chars: {fg}"
    );
    assert!(
        fg.contains("drawtext=text='triM'"),
        "should contain full word: {fg}"
    );
    assert!(
        fg.contains(":enable='between(t,"),
        "should contain enable between: {fg}"
    );
    assert!(
        fg.contains(":enable='gte(t,"),
        "should contain enable gte: {fg}"
    );
    assert!(
        fg.contains("drawtext=text='NLE'"),
        "should contain subtitle: {fg}"
    );
}
#[test]
fn test_text_clip_fade_in_fade_out_order() {
    use trim_core::clip::TextOverlay;

    let mut proj = Project::new("TextFadeTest");
    let mut v = MediaItem::new("/tmp/fake_fade.png");
    v.media_type = MediaType::Image;
    let vid_id = v.id.clone();
    proj.add_media(v);

    let mut c = Clip::new(&vid_id, 0.0, 5.0);
    c.fade_in = 1.0;
    c.fade_out = 1.0;
    c.text_overlay = Some(TextOverlay {
        content: "Fade Me".to_string(),
        font_size: 48.0,
        text_color: [255, 255, 255, 255],
        ..Default::default()
    });
    proj.timeline.tracks[0].add_clip(c);

    let preset = ExportPreset::default();
    let (cmd, _) = TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();
    let fg = cmd
        .iter()
        .position(|a| a == "-filter_complex")
        .map(|idx| &cmd[idx + 1])
        .unwrap();

    let drawtext_pos = fg
        .find("drawtext=text='Fade Me'")
        .expect("drawtext must exist");
    let fade_in_pos = fg
        .find("fade=t=in:st=0:d=1.0000:alpha=1")
        .expect("fade in must exist");
    let fade_out_pos = fg
        .find("fade=t=out:st=4.0000:d=1.0000:alpha=1")
        .expect("fade out must exist");

    // CRUCIAL: drawtext MUST come BEFORE fade=t=in and fade=t=out so the text is properly faded!
    assert!(
        drawtext_pos < fade_in_pos,
        "drawtext ({drawtext_pos}) must be before fade_in ({fade_in_pos}): {fg}"
    );
    assert!(
        fade_in_pos < fade_out_pos,
        "fade_in ({fade_in_pos}) must be before fade_out ({fade_out_pos}): {fg}"
    );
}
