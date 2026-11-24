use super::*;
use crate::presets::{ExportPreset, LoudnessNorm};
use trim_core::clip::{Clip, MotionTitleStyle};
use trim_core::media::{MediaItem, MediaType};
use trim_core::project::Project;
use trim_core::track::TrackType;

#[test]
fn test_cinematic_zoom_drawtext_generation() {
    use trim_core::clip::{MotionTitleStyle, TextOverlay};

    let mut proj = Project::new("ZoomTest");
    let mut v = MediaItem::new("/tmp/fake_zoom.mp4");
    v.media_type = MediaType::Video;
    let vid_id = v.id.clone();
    proj.add_media(v);

    let mut c = Clip::new(&vid_id, 0.0, 5.0);
    c.text_overlay = Some(TextOverlay {
        content: "Cinematic Zoom".to_string(),
        motion_style: MotionTitleStyle::CinematicZoom,
        font_size: 50.0,
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
        fg.contains("fontsize='50*(0.94+0.10*t/5.0000)'"),
        "must contain animated fontsize: {fg}"
    );
}

#[test]
fn test_build_export_pipeline_with_in_out_range() {
    let mut proj = Project::new("RangeTest");
    let mut v = MediaItem::new("/tmp/fake_range.mp4");
    v.media_type = MediaType::Video;
    let vid_id = v.id.clone();
    proj.add_media(v);

    let mut v_outside = MediaItem::new("/tmp/outside_ignored.mp4");
    v_outside.media_type = MediaType::Video;
    let outside_id = v_outside.id.clone();
    proj.add_media(v_outside);

    proj.timeline.tracks[0].add_clip(Clip::new(&vid_id, 0.0, 10.0));
    // Clip completely after range (20.0 to 25.0)
    proj.timeline.tracks[0].add_clip(Clip::new(&outside_id, 20.0, 25.0));

    let preset = ExportPreset::default();
    let (cmd, dur) = TimelineExporter::build_export_pipeline_range(
        &proj,
        &preset,
        "/tmp/out_range.mp4",
        Some((2.5, 7.5)),
    )
    .unwrap();

    assert!(
        (dur - 5.0).abs() < 1e-4,
        "Effective duration should be 5.0s"
    );

    // Outside clip must be completely excluded!
    assert!(
        !cmd.iter().any(|arg| arg.contains("outside_ignored.mp4")),
        "Clips outside the in/out range must not be loaded into ffmpeg"
    );

    let ss_idx = cmd.iter().rposition(|a| a == "-ss");
    assert!(ss_idx.is_some(), "-ss argument must be present");
    assert_eq!(cmd[ss_idx.unwrap() + 1], "2.500000");

    let t_idx = cmd.iter().rposition(|a| a == "-t");
    assert!(t_idx.is_some(), "-t argument must be present");
    assert_eq!(cmd[t_idx.unwrap() + 1], "5.000000");
}

#[test]
fn test_build_export_pipeline_with_crop() {
    let mut proj = Project::new("CropTest");
    let mut v = MediaItem::new("/tmp/fake_crop.mp4");
    v.media_type = MediaType::Video;
    let vid_id = v.id.clone();
    proj.add_media(v);
    let mut clip = Clip::new(&vid_id, 0.0, 5.0);
    clip.crop_left = 0.1;
    clip.crop_right = 0.15;
    clip.crop_top = 0.05;
    clip.crop_bottom = 0.2;
    proj.timeline.tracks[0].add_clip(clip);

    let preset = ExportPreset::default();
    let (cmd, _) =
        TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out_crop.mp4").unwrap();
    let fg = cmd
        .iter()
        .position(|a| a == "-filter_complex")
        .map(|idx| &cmd[idx + 1])
        .unwrap();

    assert!(
        fg.contains("between(X,0.1000*W,0.8500*W)*between(Y,0.0500*H,0.8000*H)"),
        "Filtergraph must contain crop alpha mask: {fg}"
    );
}

#[test]
fn test_build_export_pipeline_with_volume_keyframes() {
    let mut proj = Project::new("VolKfTest");
    let mut a = MediaItem::new("/tmp/fake_audio.mp3");
    a.media_type = MediaType::Audio;
    a.audio_codec = "mp3".into();
    let a_id = a.id.clone();
    proj.add_media(a);

    let mut clip = Clip::new(&a_id, 0.0, 5.0);
    clip.add_keyframe(
        "volume",
        0.0,
        1.0,
        trim_core::keyframe::Interpolation::Linear,
    );
    clip.add_keyframe(
        "volume",
        2.5,
        0.3,
        trim_core::keyframe::Interpolation::Linear,
    );
    clip.add_keyframe(
        "volume",
        5.0,
        1.0,
        trim_core::keyframe::Interpolation::Linear,
    );
    let a1 = proj
        .timeline
        .tracks
        .iter_mut()
        .find(|t| t.name == "A1")
        .unwrap();
    a1.add_clip(clip);

    let preset = ExportPreset::default();
    let (cmd, _) =
        TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out_vol.mp4").unwrap();
    let fg = cmd
        .iter()
        .position(|a| a == "-filter_complex")
        .map(|idx| &cmd[idx + 1])
        .unwrap();

    assert!(
        fg.contains("volume=eval=frame:volume="),
        "Filtergraph must contain volume evaluation expression: {fg}"
    );
}

#[test]
fn test_white_balance_gamma_and_expanded_looks_filtergraph() {
    use trim_core::clip::{BrightnessContrastPreset, LutPreset, WhiteBalancePreset};

    let mut proj = Project::new("ColorExportTest");
    let mut v = MediaItem::new("/tmp/fake_color.mp4");
    v.media_type = MediaType::Video;
    let vid_id = v.id.clone();
    proj.add_media(v);

    let mut c1 = Clip::new(&vid_id, 0.0, 5.0);
    c1.apply_wb_preset(WhiteBalancePreset::Tungsten);
    c1.apply_bc_preset(BrightnessContrastPreset::CinematicContrast);
    c1.lut_preset = LutPreset::Kodak2383;
    proj.timeline.tracks[0].add_clip(c1);

    let preset = ExportPreset::default();
    let (cmd, _) =
        TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out_color.mp4").unwrap();
    let fg = cmd
        .iter()
        .position(|a| a == "-filter_complex")
        .map(|idx| &cmd[idx + 1])
        .unwrap();

    assert!(
        fg.contains("colorchannelmixer=rr="),
        "should include white balance colorchannelmixer filter: {fg}"
    );
    assert!(
        fg.contains(":gamma=0.900"),
        "should include gamma in eq filter for CinematicContrast: {fg}"
    );
    assert!(
        fg.contains("colorbalance=rs=-0.12"),
        "should include Kodak 2383 look filter: {fg}"
    );
}

#[test]
fn test_compositing_blend_mask_adjustment_layer_filtergraph() {
    let mut proj = Project::new("CompTest");
    let mut m = MediaItem::new("/tmp/fake_video.mp4");
    m.media_type = MediaType::Video;
    m.duration = 5.0;
    let m_id = m.id.clone();
    proj.add_media(m);

    let mut base_clip = Clip::new(&m_id, 0.0, 5.0);
    base_clip.blend_mode = trim_core::BlendMode::Screen;
    let mut alpha_mask = trim_core::Mask::new(trim_core::MaskShape::Rectangle, "Box");
    alpha_mask.target = trim_core::MaskTarget::ClipAlpha;
    base_clip.masks.push(alpha_mask);
    proj.timeline.tracks[0].add_clip(base_clip);

    let mut adj = Clip::new_adjustment_layer(1.0, 4.0);
    adj.brightness = 0.15;
    adj.blend_mode = trim_core::BlendMode::Overlay;
    let mut fx_mask = trim_core::Mask::new(trim_core::MaskShape::Ellipse, "Spot");
    fx_mask.target = trim_core::MaskTarget::ColorAndEffects;
    adj.masks.push(fx_mask);
    proj.timeline.add_new_video_track();
    proj.timeline
        .tracks
        .iter_mut()
        .rfind(|t| t.track_type == TrackType::Video)
        .unwrap()
        .add_clip(adj);

    let preset = ExportPreset::default();
    let (cmd, _) =
        TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out_comp.mp4").unwrap();
    let fg = cmd
        .iter()
        .position(|a| a == "-filter_complex")
        .map(|idx| &cmd[idx + 1])
        .unwrap();

    assert!(
        fg.contains("blend=all_mode=screen"),
        "Missing Screen blend: {fg}"
    );
    assert!(
        fg.contains("blend=all_mode=overlay"),
        "Missing Overlay blend on AdjLayer: {fg}"
    );
    assert!(
        fg.contains("maskedmerge"),
        "Missing maskedmerge for masks: {fg}"
    );
    assert!(
        fg.contains("eq=brightness=0.150"),
        "Missing AdjLayer color filter: {fg}"
    );
}

#[test]
fn test_rotation_prores_and_audio_speed_ramp_export() {
    let mut proj = Project::new("ParityTest");
    let mut m = MediaItem::new("/tmp/fake_parity.mp4");
    m.media_type = MediaType::Video;
    m.duration = 10.0;
    m.audio_codec = "aac".to_string();
    let m_id = m.id.clone();
    proj.add_media(m);

    let mut clip = Clip::new(&m_id, 0.0, 4.0);
    clip.rotation = 45.0;
    clip.add_keyframe(
        "speed",
        0.0,
        1.0,
        trim_core::keyframe::Interpolation::Linear,
    );
    clip.add_keyframe(
        "speed",
        4.0,
        2.0,
        trim_core::keyframe::Interpolation::Linear,
    );
    proj.timeline.tracks[0].add_clip(clip.clone());

    let mut a_clip = Clip::new(&m_id, 0.0, 4.0);
    a_clip.add_keyframe(
        "speed",
        0.0,
        1.0,
        trim_core::keyframe::Interpolation::Linear,
    );
    a_clip.add_keyframe(
        "speed",
        4.0,
        2.0,
        trim_core::keyframe::Interpolation::Linear,
    );
    let audio_tr_id = proj.timeline.get_audio_tracks()[0].id.clone();
    proj.timeline
        .get_track_by_id_mut(&audio_tr_id)
        .unwrap()
        .add_clip(a_clip);

    let prores_preset = ExportPreset {
        video_codec: "prores_ks".to_string(),
        ..Default::default()
    };
    let (cmd, _) =
        TimelineExporter::build_export_pipeline(&proj, &prores_preset, "/tmp/out.mov").unwrap();

    assert!(cmd.contains(&"yuv422p10le".to_string()));
    let fg = cmd
        .iter()
        .position(|a| a == "-filter_complex")
        .map(|idx| &cmd[idx + 1])
        .unwrap();
    assert!(fg.contains("rotate="), "Missing rotate filter: {fg}");
    assert!(
        fg.contains("atempo=1.5000"),
        "Missing average speed-ramp atempo on audio: {fg}"
    );
}

#[test]
fn test_baked_cube_lut_filtergraph_for_camera_log_and_hsl_curves() {
    let mut proj = Project::new("BakedCubeTest");
    let mut v = MediaItem::new("/tmp/slog_footage.mp4");
    v.media_type = MediaType::Video;
    v.duration = 6.0;
    let vid_id = v.id.clone();
    proj.add_media(v);

    let mut c1 = Clip::new(&vid_id, 0.0, 4.0);
    c1.camera_log_profile = trim_core::CameraLogProfile::SLog3;
    c1.temperature = 14.0;
    c1.lut_preset = trim_core::LutPreset::Kodak2383;
    proj.timeline.tracks[0].add_clip(c1);

    let (cmd, _) =
        TimelineExporter::build_export_pipeline(&proj, &ExportPreset::default(), "/tmp/out.mp4")
            .unwrap();
    let fg = cmd
        .iter()
        .position(|a| a == "-filter_complex")
        .map(|idx| &cmd[idx + 1])
        .unwrap();
    assert!(
        fg.contains("lut3d=file='") && fg.contains(":interp=trilinear"),
        "Camera Log clip should bake 3D .cube LUT and emit trilinear lut3d filter: {fg}"
    );
}

#[test]
fn test_filtergraph_escapes_injection_and_uses_protocol_whitelist_and_tempdir() {
    let mut proj = Project::new("SecurityTest");
    let mut v = MediaItem::new("-evil_leading_dash.mp4");
    v.media_type = MediaType::Video;
    v.duration = 5.0;
    let vid_id = v.id.clone();
    proj.add_media(v);

    let mut c1 = Clip::new(&vid_id, 0.0, 4.0);
    let txt = trim_core::clip::TextOverlay {
        content: "O'Brien %{localtime:%X}\n;copy[outv]".into(),
        motion_style: MotionTitleStyle::None,
        ..Default::default()
    };
    c1.text_overlay = Some(txt);
    proj.timeline.tracks[0].add_clip(c1);

    let prepared = TimelineExporter::build_export_pipeline_prepared(
        &proj,
        &ExportPreset::default(),
        "-out.mp4",
        None,
    )
    .unwrap();
    let temp_path = prepared.temp_dir.as_ref().unwrap().path().to_path_buf();
    assert!(temp_path.exists());

    let cmd = &prepared.cmd_args;
    assert!(cmd.windows(2).any(|w| w == ["-protocol_whitelist", "file"]));
    assert!(cmd.contains(&"./-evil_leading_dash.mp4".to_string()));
    assert_eq!(cmd.last().map(String::as_str), Some("./-out.mp4"));

    let fg = cmd
        .iter()
        .position(|a| a == "-filter_complex")
        .map(|idx| &cmd[idx + 1])
        .unwrap();
    assert!(fg.contains(":expansion=none"));
    assert!(fg.contains("O'\\''Brien"));
    assert!(!fg.contains("\n;copy[outv]"));

    drop(prepared);
    assert!(!temp_path.exists());
}

#[test]
fn test_k11_mask_plus_blend_and_adjustment_layer_split_valid_with_ffmpeg() {
    let mut proj = Project::new("K11Parity");
    proj.settings.width = 320;
    proj.settings.height = 180;
    proj.timeline.width = 320;
    proj.timeline.height = 180;
    proj.timeline.fps = 24.0;
    proj.timeline.tracks[1].pan = -0.5;

    let temp_dir = tempfile::tempdir().unwrap();
    let src_video = temp_dir.path().join("src.mp4");
    let out_video = temp_dir.path().join("out.mp4");

    // Create a real tiny 1-second test clip if FFmpeg is installed
    let has_ffmpeg = trim_media::is_ffmpeg_available();
    if has_ffmpeg {
        let gen = trim_media::ffmpeg_cmd()
            .args([
                "-y",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=blue:s=320x180:r=24:d=1",
                "-f",
                "lavfi",
                "-i",
                "sine=f=440:r=48000",
                "-ac",
                "2",
                "-t",
                "1",
                "-c:v",
                "libx264",
                "-c:a",
                "aac",
                src_video.to_str().unwrap(),
            ])
            .output();
        assert!(
            matches!(gen, Ok(o) if o.status.success()),
            "failed to generate synthetic clip"
        );
    } else {
        std::fs::write(&src_video, b"dummy").unwrap();
    }

    let mut v = MediaItem::new(src_video.to_str().unwrap());
    v.media_type = MediaType::Video;
    v.duration = 1.0;
    v.audio_channels = 2;
    v.audio_sample_rate = 48000;
    let vid_id = v.id.clone();
    proj.add_media(v);

    let mut c1 = Clip::new(&vid_id, 0.0, 1.0);
    c1.blend_mode = trim_core::BlendMode::Screen;
    c1.masks.push(trim_core::mask::Mask::new(
        trim_core::mask::MaskShape::Rectangle,
        "m_alpha",
    ));
    proj.timeline.tracks[0].add_clip(c1);

    let mut adj = Clip::new_adjustment_layer(0.0, 1.0);
    adj.brightness = 0.1;
    adj.masks.push(trim_core::mask::Mask::new(
        trim_core::mask::MaskShape::Rectangle,
        "adj_a",
    ));
    let mut adj_fx_mask =
        trim_core::mask::Mask::new(trim_core::mask::MaskShape::Rectangle, "adj_fx");
    adj_fx_mask.target = trim_core::mask::MaskTarget::ColorAndEffects;
    adj.masks.push(adj_fx_mask);
    proj.timeline.tracks[0].add_clip(adj);

    let a1 = Clip::new(&vid_id, 0.0, 1.0);
    let audio_track = proj
        .timeline
        .tracks
        .iter_mut()
        .find(|t| t.track_type == TrackType::Audio)
        .unwrap();
    audio_track.pan = -0.5;
    audio_track.add_clip(a1);

    let preset = ExportPreset {
        loudness_norm: LoudnessNorm::YouTube,
        ..Default::default()
    };
    let prepared = TimelineExporter::build_export_pipeline_prepared(
        &proj,
        &preset,
        out_video.to_str().unwrap(),
        None,
    )
    .unwrap();

    let fg = prepared
        .cmd_args
        .iter()
        .position(|a| a == "-filter_complex")
        .map(|idx| &prepared.cmd_args[idx + 1])
        .unwrap();
    assert!(
        fg.contains("split=3"),
        "expected split=3 for blend+mask/adj: {fg}"
    );
    assert!(
        fg.contains("aresample=48000"),
        "expected 48kHz resample after loudnorm: {fg}"
    );
    assert!(
        fg.contains("pan=stereo|"),
        "expected track.pan in filtergraph: {fg}"
    );
    assert!(prepared
        .cmd_args
        .windows(2)
        .any(|w| w == ["-colorspace", "bt709"]));
    assert!(prepared
        .cmd_args
        .windows(2)
        .any(|w| w == ["-movflags", "+faststart"]));

    if has_ffmpeg {
        let status = trim_media::create_command(&prepared.cmd_args[0])
            .args(&prepared.cmd_args[1..])
            .output()
            .expect("run ffmpeg");
        assert!(
            status.status.success(),
            "FFmpeg failed on K-11 mask+blend+adj export: {}",
            String::from_utf8_lossy(&status.stderr)
        );
        assert!(out_video.is_file() && out_video.metadata().unwrap().len() > 0);
    }
}

#[test]
fn test_adversarial_drawtext_and_path_escaping_with_ffmpeg() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let vid_path = tmp.path().join("adv_base.mp4");
    if trim_media::is_ffmpeg_available() {
        let gen = trim_media::ffmpeg_cmd()
            .args([
                "-y",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=320x180:r=25:d=1.0",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&vid_path)
            .output()
            .expect("gen adv base");
        assert!(gen.status.success());
    } else {
        let _ = std::fs::write(&vid_path, b"dummy");
    }

    let mut proj = Project::default();
    proj.settings.width = 320;
    proj.settings.height = 180;
    proj.settings.fps = 25.0;
    let mut m = MediaItem::new(&vid_path);
    m.media_type = MediaType::Video;
    m.duration = 1.0;
    m.width = 320;
    m.height = 180;
    m.fps = 25.0;
    let mid = m.id.clone();
    proj.add_media(m);

    let adversarial = "Merhaba 'Dünya': [1,2]; %100 \\ kaçış\nTürkçe: ğüşiöçİĞÜŞÖÇ ".repeat(4);
    let mut txt_clip = Clip::new(&mid, 0.0, 1.0);
    txt_clip.text_overlay = Some(trim_core::TextOverlay {
        content: adversarial,
        font_size: 24.0,
        ..Default::default()
    });
    proj.timeline.tracks[0].add_clip(txt_clip);

    let out_mp4 = tmp.path().join("adv_out.mp4");
    let (cmd_args, _) = TimelineExporter::build_export_pipeline(
        &proj,
        &ExportPreset {
            width: Some(320),
            height: Some(180),
            fps: Some(25.0),
            ..Default::default()
        },
        out_mp4.to_str().expect("utf8 path"),
    )
    .expect("build adversarial filtergraph");

    if trim_media::is_ffmpeg_available() {
        let fg_idx = cmd_args
            .iter()
            .position(|a| a == "-filter_complex")
            .expect("filter_complex flag");
        let fg_str = &cmd_args[fg_idx + 1];
        let out = trim_media::ffmpeg_cmd()
            .args([
                "-y",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=320x180:r=25:d=1.0",
                "-f",
                "lavfi",
                "-t",
                "1.0",
                "-i",
                "anullsrc=r=48000:cl=stereo",
                "-protocol_whitelist",
                "file",
                "-i",
                vid_path.to_str().expect("utf8"),
                "-filter_complex",
                fg_str,
                "-map",
                "[outv]",
                "-map",
                "[outa]",
                "-frames:v",
                "1",
                "-f",
                "null",
                "-",
            ])
            .output()
            .expect("run ffmpeg -filter_complex");
        assert!(
            out.status.success(),
            "FFmpeg rejected escaped adversarial drawtext filtergraph:\nfg={fg_str}\nerr={}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn test_golden_filtergraph_snapshot() {
    let mut proj = Project::default();
    proj.settings.width = 640;
    proj.settings.height = 360;
    proj.settings.fps = 30.0;
    let mut m = MediaItem::new("/tmp/golden_base.mp4");
    m.media_type = MediaType::Video;
    m.duration = 2.0;
    m.width = 640;
    m.height = 360;
    m.fps = 30.0;
    let mid = m.id.clone();
    proj.add_media(m);

    let mut txt = Clip::new(&mid, 0.0, 2.0);
    txt.text_overlay = Some(trim_core::TextOverlay {
        content: "Golden Snapshot".to_string(),
        font_size: 32.0,
        ..Default::default()
    });
    proj.timeline.tracks[0].add_clip(txt);

    let (cmd_args, _) = TimelineExporter::build_export_pipeline(
        &proj,
        &ExportPreset {
            width: Some(640),
            height: Some(360),
            fps: Some(30.0),
            loudness_norm: LoudnessNorm::Off,
            ..Default::default()
        },
        "/tmp/golden.mp4",
    )
    .expect("golden pipeline");
    let fg_idx = cmd_args
        .iter()
        .position(|a| a == "-filter_complex")
        .expect("filter_complex flag");
    let fg = &cmd_args[fg_idx + 1];
    assert!(fg.contains("[0:v]format=yuv420p,setsar=1[base0];"));
    assert!(fg.contains("drawtext=text='Golden Snapshot'"));
    assert!(fg.contains("[base1]format=yuv420p[outv];"));
    assert!(fg.ends_with("alimiter=limit=0.988553:attack=1:release=75:level=disabled[outa]"));
}
