use super::*;
use image::{ImageBuffer, Rgba, RgbaImage};
use std::sync::{Arc, Mutex};
use trim_core::media::MediaType;
use trim_core::project::Project;

#[test]
fn test_video_frame_decoder_smoke() {
    let mut decoder = VideoFrameDecoder::new();
    let res = decoder.decode_frame("/tmp/nonexistent_video_file.mp4", 0.0);
    assert!(res.is_none());

    let dummy = "/tmp/sample_trim_test.mp4";
    if std::path::Path::new(dummy).is_file() {
        let frame = decoder.decode_frame(dummy, 0.5);
        assert!(frame.is_some());
        let img = frame.unwrap();
        assert!(img.width() > 0 && img.height() > 0);
    }

    let user_file = "/home/mozcelik/Videolar/İsimsiz Proje.mp4";
    if std::path::Path::new(user_file).is_file() {
        let frame = decoder.decode_frame(user_file, 11.467);
        assert!(frame.is_some());
    }
}

#[test]
fn test_image_preview_clears_at_end_and_during_gaps() {
    use tempfile::tempdir;
    use trim_core::clip::Clip;
    use trim_core::media::MediaItem;
    use trim_core::project::Project;

    let dir = tempdir().unwrap();
    let image_path = dir.path().join("preview.png");
    RgbaImage::from_pixel(8, 8, Rgba([200, 30, 40, 255]))
        .save(&image_path)
        .unwrap();

    let mut project = Project::default();
    let mut media = MediaItem::new(&image_path);
    media.media_type = MediaType::Image;
    let media_id = media.id.clone();
    project.add_media(media);

    let track = project
        .timeline
        .get_video_tracks_mut()
        .into_iter()
        .next()
        .unwrap();
    track.add_clip(Clip::new(&media_id, 0.0, 2.0));
    track.add_clip(Clip::new(&media_id, 3.0, 4.0));

    let mut engine = VideoEngine::new();
    assert!(engine.render_timeline_frame(&project, 1.9, false).is_some());
    assert!(engine.last_rendered_frame.is_some());

    // The clip's out-point and the following gap must both be black.
    assert!(engine.render_timeline_frame(&project, 2.0, false).is_none());
    assert!(engine.last_rendered_frame.is_none());
    assert!(!engine.is_decoding_seek);
    assert!(engine.render_timeline_frame(&project, 2.5, false).is_none());

    // Seeking to another image must draw it, then clear at its out-point.
    assert!(engine.render_timeline_frame(&project, 3.0, false).is_some());
    assert!(engine.render_timeline_frame(&project, 4.0, false).is_none());
    assert!(engine.last_rendered_frame.is_none());
}

#[test]
fn test_video_engine_lifecycle() {
    let engine = VideoEngine::new();
    assert!(!engine.streamer.is_active());
    let dummy = "/tmp/sample_trim_test.mp4";
    if std::path::Path::new(dummy).is_file() {
        let frame = decode_frame_raw(dummy, 0.5);
        assert!(frame.is_some());
        let f = frame.unwrap();
        assert_eq!(f.width(), PREVIEW_WIDTH);
        assert_eq!(f.height(), PREVIEW_HEIGHT);
    }
}

#[test]
fn test_playback_streamer_and_async_seek() {
    let dummy = "/home/mozcelik/Videolar/İsimsiz Proje.mp4";
    if std::path::Path::new(dummy).is_file() {
        // Test PlaybackStreamer
        let mut streamer = PlaybackStreamer::new();
        streamer.start("test_clip", dummy, 1.0, 30.0, false, 0.0);
        assert!(streamer.is_active());
        let start = std::time::Instant::now();
        let mut frame = None;
        while start.elapsed().as_millis() < 1500 {
            if let Some(f) = streamer.get_frame_for_time(1.2) {
                frame = Some(f);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(frame.is_some());
        let f = frame.unwrap();
        assert_eq!(f.width(), PREVIEW_WIDTH);
        assert_eq!(f.height(), PREVIEW_HEIGHT);
        streamer.stop();
        assert!(!streamer.is_active());

        // Test AsyncFrameDecoder
        let mut async_dec = AsyncFrameDecoder::new();
        let req_id = async_dec.request_seek(dummy, 2.0);
        assert!(req_id > 0);
        let start = std::time::Instant::now();
        let mut got_result = None;
        while start.elapsed().as_millis() < 1200 {
            if let Some(res) = async_dec.poll_result() {
                got_result = Some(res);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        assert!(got_result.is_some());
        let res = got_result.unwrap();
        let frame = res.frame.expect("should decode test frame");
        assert_eq!(frame.width(), PREVIEW_WIDTH);
        assert_eq!(frame.height(), PREVIEW_HEIGHT);
    }
}

#[test]
fn test_render_timeline_real_file() {
    let path = "/home/mozcelik/Videolar/İsimsiz Proje.mp4";
    if std::path::Path::new(path).exists() {
        let mut project = Project::default();
        let media = trim_core::media::MediaItem::new(path);
        let media_id = media.id.clone();
        project.add_media(media);
        let mut track = trim_core::track::Track::new("V1", trim_core::track::TrackType::Video);
        let clip = trim_core::clip::Clip::new(&media_id, 0.0, 10.0);
        track.add_clip(clip);
        project.timeline.add_track(track);

        let mut engine = VideoEngine::new();
        let f1 = engine.render_timeline_frame(&project, 0.0, false);
        assert!(f1.is_some(), "f1 should be some for real file at time 0.0");

        // Test scrub to 2.0s
        let _ = engine.render_timeline_frame(&project, 2.0, false);
        let start = std::time::Instant::now();
        let mut seek_done = false;
        while start.elapsed().as_millis() < 1200 {
            let f = engine.render_timeline_frame(&project, 2.0, false);
            if !engine.is_decoding_seek {
                assert!(f.is_some());
                seek_done = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(seek_done, "Seek to 2.0s should complete asynchronously");

        // Test playback at 30fps
        let mut got_streamed = false;
        let play_start = std::time::Instant::now();
        let mut t = 2.0;
        while play_start.elapsed().as_millis() < 1500 {
            t += 0.033;
            let f = engine.render_timeline_frame(&project, t, true);
            if let Some(img) = f {
                assert_eq!(img.width(), PREVIEW_WIDTH);
                got_streamed = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        assert!(got_streamed, "Playback should produce frames");
    }
}

#[test]
fn test_render_timeline_with_fade_out() {
    let mut project = Project::default();
    let mut media = trim_core::media::MediaItem::new("/nonexistent_test.mp4");
    media.media_type = trim_core::media::MediaType::Video;
    let media_id = media.id.clone();
    project.add_media(media);

    let mut track = trim_core::track::Track::new("V1", trim_core::track::TrackType::Video);
    let mut clip = trim_core::clip::Clip::new(&media_id, 0.0, 5.0);
    clip.fade_out = 1.5;
    track.add_clip(clip);
    project.timeline.add_track(track);

    let mut engine = VideoEngine::new();
    // Visible at 4.0s (within fade-out period)
    let _ = engine.render_timeline_frame(&project, 4.0, false);
    // Beyond clip at 6.0s (empty, should return None and clean state)
    let beyond = engine.render_timeline_frame(&project, 6.0, false);
    assert!(beyond.is_none());
}

#[test]
fn test_reverse_playback_streamer_flags() {
    let mut streamer = PlaybackStreamer::new();
    assert!(!streamer.is_reversed());
    streamer.start("clip_rev", "/nonexistent.mp4", 5.0, 30.0, true, 1.0);
    assert!(streamer.is_reversed());
    assert_eq!(streamer.last_pts(), 5.0);
    assert!(!streamer.should_restart(5.0, true));
    assert!(streamer.should_restart(5.0, false));
    streamer.stop();
    assert!(!streamer.is_active());
}

#[test]
fn test_reverse_playback_streamer_real_frames() {
    let test_file = "/tmp/test_trim_rev.mp4";
    if std::path::Path::new(test_file).is_file() {
        let mut streamer = PlaybackStreamer::new();
        streamer.start("clip_rev_real", test_file, 4.0, 30.0, true, 0.0);
        assert!(streamer.is_active());
        assert!(streamer.is_reversed());

        let start = std::time::Instant::now();
        let mut got_frame = false;
        let mut t = 4.0;
        while start.elapsed().as_millis() < 2000 {
            t -= 0.033;
            if let Some(f) = streamer.get_frame_for_time(t) {
                assert_eq!(f.width(), PREVIEW_WIDTH);
                assert_eq!(f.height(), PREVIEW_HEIGHT);
                got_frame = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(
            got_frame,
            "Streamer should deliver reversed frames from real video"
        );
        assert!(streamer.frames_delivered() > 0);
        streamer.stop();
        assert!(!streamer.is_active());
    }
}

#[test]
fn test_chromakey_rendering_removes_key_color() {
    use trim_core::clip::{ChromaKeyConfig, Clip};

    let mut canvas = ImageBuffer::from_pixel(10, 10, Rgba([0, 0, 255, 255])); // Blue canvas background

    // Frame with pure green on left half (x < 5) and bright red on right half (x >= 5)
    let mut frame = ImageBuffer::from_pixel(10, 10, Rgba([0, 255, 0, 255]));
    for y in 0..10 {
        for x in 5..10 {
            frame.put_pixel(x, y, Rgba([255, 0, 0, 255]));
        }
    }

    let mut clip = Clip::new("m1", 0.0, 5.0);
    clip.chromakey = Some(ChromaKeyConfig {
        enabled: true,
        key_color: [0, 255, 0],
        similarity: 0.25,
        smoothness: 0.05,
    });

    draw_clip_onto_canvas(&mut canvas, &clip, &frame, 1.0);

    // Keyed out area (left half) should retain the background canvas color (Blue)
    let keyed_px = canvas.get_pixel(2, 2);
    assert_eq!(keyed_px[0], 0, "Red channel should be 0 on keyed area");
    assert_eq!(
        keyed_px[1], 0,
        "Green channel should be keyed out (not green!)"
    );
    assert_eq!(keyed_px[2], 255, "Blue background should show through");

    // Non-keyed area (right half) should show the red foreground from the clip
    let red_px = canvas.get_pixel(7, 2);
    assert_eq!(red_px[0], 255, "Red foreground should be rendered");
    assert_eq!(red_px[1], 0);
    assert_eq!(red_px[2], 0);
}

#[test]
fn test_color_wheels_rendering_modifies_pixels() {
    use trim_core::clip::{Clip, ColorWheelsConfig};

    let mut canvas = ImageBuffer::from_pixel(10, 10, Rgba([0, 0, 0, 255]));
    let frame = ImageBuffer::from_pixel(10, 10, Rgba([128, 128, 128, 255]));

    let mut clip = Clip::new("m1", 0.0, 5.0);
    clip.color_wheels = Some(ColorWheelsConfig {
        lift: [0.2, 0.0, -0.2],
        gamma: [1.2, 1.0, 0.8],
        gain: [1.2, 1.0, 0.8],
        luminance: [0.0, 0.1, 0.0],
    });

    draw_clip_onto_canvas(&mut canvas, &clip, &frame, 1.0);

    let modified_px = canvas.get_pixel(5, 5);
    assert!(
        modified_px[0] > modified_px[1],
        "Red should be boosted over Green"
    );
    assert!(
        modified_px[1] > modified_px[2],
        "Green should be higher than Blue"
    );
}

#[test]
fn test_image_chromakey_bypasses_fast_path() {
    use trim_core::clip::{ChromaKeyConfig, Clip};
    use trim_core::media::{MediaItem, MediaType};

    let temp_dir = std::env::temp_dir();
    let img_path = temp_dir.join("test_trim_ck_bypass.png");

    // Create 640x360 green image
    let green_img: RgbaImage = ImageBuffer::from_pixel(
        PREVIEW_WIDTH,
        PREVIEW_HEIGHT,
        Rgba([0u8, 255u8, 0u8, 255u8]),
    );
    green_img
        .save(&img_path)
        .expect("Failed to save temp test image");

    let mut project = Project::default();
    let mut media = MediaItem::new(img_path.to_str().unwrap());
    media.media_type = MediaType::Image;
    let m_id = media.id.clone();
    project.add_media(media);

    let mut track = trim_core::track::Track::new("V1", trim_core::track::TrackType::Video);
    let mut clip = Clip::new(&m_id, 0.0, 5.0);
    clip.chromakey = Some(ChromaKeyConfig {
        enabled: true,
        key_color: [0, 255, 0],
        similarity: 0.3,
        smoothness: 0.05,
    });
    track.add_clip(clip);
    project.timeline.add_track(track);

    let mut engine = VideoEngine::new();
    let rendered = engine.render_timeline_frame(&project, 1.0, false);
    assert!(rendered.is_some(), "Should render a frame");

    let frame = rendered.unwrap();
    // Since green was keyed out on single clip, canvas background is black [0, 0, 0, 255]
    let px = frame.get_pixel(100, 100);
    assert_eq!(
        px[1], 0,
        "Green should be removed by Chroma Key instead of returning raw green frame"
    );

    let _ = std::fs::remove_file(img_path);
}

#[test]
fn test_video_engine_frame_cache_lru_bounded() {
    let engine = VideoEngine::new();
    let dummy_frame = Arc::new(ImageBuffer::from_pixel(16, 16, Rgba([0, 0, 0, 255])));

    {
        let mut cache = engine.frame_cache.write().unwrap();
        for i in 0..400 {
            let key = ("test_media.mp4".to_string(), i);
            cache.insert(key, Arc::clone(&dummy_frame));
        }
        // Max capacity is 300, so size should be capped at 300
        assert_eq!(cache.len(), 300);
        // First 100 items (0..100) should have been evicted
        for i in 0..100 {
            let key = ("test_media.mp4".to_string(), i);
            assert!(
                cache.get(&key).is_none(),
                "Key {} should have been evicted",
                i
            );
        }
        // Items 100..400 should be present
        for i in 100..400 {
            let key = ("test_media.mp4".to_string(), i);
            assert!(cache.get(&key).is_some(), "Key {} should be in cache", i);
        }
    }
}

#[test]
fn test_parse_cube_lut_str_unwraps_3d_strip() {
    let cube_content = "\
# Test 2x2x2 3D LUT
TITLE \"UnitTestLUT\"
LUT_3D_SIZE 2
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
";
    let lut = parse_cube_lut_str(cube_content).expect("Should parse valid 2x2x2 .cube LUT");
    assert_eq!(lut.size, 2);
    // 2x2x2 = 8 RGBA texels = 32 bytes
    assert_eq!(lut.rgba_strip.len(), 32);
    // Entry 0: (0,0,0) -> [0, 0, 0, 255]
    assert_eq!(&lut.rgba_strip[0..4], &[0, 0, 0, 255]);
    // Entry 1: (1,0,0) -> [255, 0, 0, 255]
    assert_eq!(&lut.rgba_strip[4..8], &[255, 0, 0, 255]);
    // Entry 7: (1,1,1) -> [255, 255, 255, 255]
    assert_eq!(&lut.rgba_strip[28..32], &[255, 255, 255, 255]);
}

#[test]
fn test_collect_timeline_layers_multitrack_order() {
    use tempfile::tempdir;
    use trim_core::clip::Clip;
    use trim_core::media::MediaItem;
    use trim_core::project::Project;

    let dir = tempdir().unwrap();
    let img1_path = dir.path().join("layer1.png");
    let img2_path = dir.path().join("layer2.png");
    RgbaImage::from_pixel(16, 16, Rgba([255, 0, 0, 255]))
        .save(&img1_path)
        .unwrap();
    RgbaImage::from_pixel(16, 16, Rgba([0, 255, 0, 255]))
        .save(&img2_path)
        .unwrap();

    let mut project = Project::default();
    let mut m1 = MediaItem::new(&img1_path);
    m1.media_type = MediaType::Image;
    let m1_id = m1.id.clone();
    project.add_media(m1);

    let mut m2 = MediaItem::new(&img2_path);
    m2.media_type = MediaType::Image;
    let m2_id = m2.id.clone();
    project.add_media(m2);

    let v1_id = project
        .timeline
        .tracks
        .iter()
        .find(|t| t.name == "V1")
        .unwrap()
        .id
        .clone();
    let v2_id = project
        .timeline
        .tracks
        .iter()
        .find(|t| t.name == "V2")
        .unwrap()
        .id
        .clone();

    project
        .timeline
        .get_track_by_id_mut(&v1_id)
        .unwrap()
        .add_clip(Clip::new(&m1_id, 0.0, 5.0));
    project
        .timeline
        .get_track_by_id_mut(&v2_id)
        .unwrap()
        .add_clip(Clip::new(&m2_id, 0.0, 5.0));

    let mut engine = VideoEngine::new();
    let layers = engine.collect_timeline_layers(&project, 1.0, false);
    assert_eq!(layers.len(), 2);
    // Bottom track (V1) comes first, top track (V2) comes second for alpha blending
    assert_eq!(layers[0].clip.media_id, m1_id);
    assert_eq!(layers[1].clip.media_id, m2_id);
}

#[test]
fn test_advanced_color_presets_rendering() {
    use trim_core::clip::{BrightnessContrastPreset, Clip, LutPreset, WhiteBalancePreset};

    let frame = ImageBuffer::from_pixel(8, 8, Rgba([128, 128, 128, 255]));

    // 1. Warm White Balance boosts Red and reduces Blue
    let mut canvas_wb = ImageBuffer::from_pixel(8, 8, Rgba([0, 0, 0, 255]));
    let mut clip_wb = Clip::new("m1", 0.0, 5.0);
    clip_wb.apply_wb_preset(WhiteBalancePreset::Warm);
    draw_clip_onto_canvas(&mut canvas_wb, &clip_wb, &frame, 1.0);
    let px_wb = canvas_wb.get_pixel(4, 4);
    assert!(px_wb[0] > 128, "Warm WB should increase red channel");
    assert!(px_wb[2] < 128, "Warm WB should decrease blue channel");

    // 2. Dark / Moody BC preset darkens midtones via brightness + gamma
    let mut canvas_bc = ImageBuffer::from_pixel(8, 8, Rgba([0, 0, 0, 255]));
    let mut clip_bc = Clip::new("m1", 0.0, 5.0);
    clip_bc.apply_bc_preset(BrightnessContrastPreset::DarkMoody);
    draw_clip_onto_canvas(&mut canvas_bc, &clip_bc, &frame, 1.0);
    let px_bc = canvas_bc.get_pixel(4, 4);
    assert!(px_bc[0] < 128, "DarkMoody preset should darken midtones");

    // 3. Sepia look produces warm R > G > B ordering
    let mut canvas_sepia = ImageBuffer::from_pixel(8, 8, Rgba([0, 0, 0, 255]));
    let mut clip_sepia = Clip::new("m1", 0.0, 5.0);
    clip_sepia.lut_preset = LutPreset::Sepia;
    draw_clip_onto_canvas(&mut canvas_sepia, &clip_sepia, &frame, 1.0);
    let px_sepia = canvas_sepia.get_pixel(4, 4);
    assert!(px_sepia[0] > px_sepia[1] && px_sepia[1] > px_sepia[2]);
}

#[test]
fn test_analyze_clip_auto_wb_on_image_clip() {
    use tempfile::tempdir;
    use trim_core::clip::Clip;
    use trim_core::media::MediaItem;
    use trim_core::project::Project;

    let dir = tempdir().unwrap();
    let img_path = dir.path().join("blue_cast.png");
    // Image with strong blue cast
    RgbaImage::from_pixel(32, 32, Rgba([90, 125, 185, 255]))
        .save(&img_path)
        .unwrap();

    let mut project = Project::default();
    let mut media = MediaItem::new(&img_path);
    media.media_type = MediaType::Image;
    let media_id = media.id.clone();
    project.add_media(media);

    let clip = Clip::new(&media_id, 0.0, 5.0);
    let mut engine = VideoEngine::new();
    let (temp, _tint) = engine
        .analyze_clip_auto_wb(&project, &clip, 1.0)
        .expect("Should analyze auto WB for image clip");
    assert!(
        temp > 10.0,
        "Blue-cast image should produce warm positive temperature compensation, got {temp}"
    );
}

#[test]
fn test_cpu_blend_modes_masks_and_adjustment_layer() {
    use trim_core::{BlendMode, Clip, Mask, MaskShape, MaskTarget};

    // 1. BlendMode::Multiply over mid-gray (128) with red (255, 0, 0) => (128, 0, 0)
    let mut canvas = ImageBuffer::from_pixel(16, 16, Rgba([128, 128, 128, 255]));
    let top_frame = ImageBuffer::from_pixel(16, 16, Rgba([255, 0, 0, 255]));
    let mut top_clip = Clip::new("m_top", 0.0, 5.0);
    top_clip.blend_mode = BlendMode::Multiply;
    draw_clip_onto_canvas(&mut canvas, &top_clip, &top_frame, 1.0);
    let px = canvas.get_pixel(8, 8);
    assert!(
        px[0] > 120 && px[1] < 5 && px[2] < 5,
        "Multiply failed: {:?}",
        px
    );

    // 2. Rectangle ClipAlpha mask on top clip keeps corners untouched
    let mut canvas_mask = ImageBuffer::from_pixel(32, 32, Rgba([10, 20, 30, 255]));
    let white_frame = ImageBuffer::from_pixel(32, 32, Rgba([250, 250, 250, 255]));
    let mut masked_clip = Clip::new("m_mask", 0.0, 5.0);
    let mut rect_mask = Mask::new(MaskShape::Rectangle, "Center Box");
    rect_mask.scale_x = 0.4;
    rect_mask.scale_y = 0.4;
    rect_mask.feather = 0.0;
    rect_mask.target = MaskTarget::ClipAlpha;
    masked_clip.masks.push(rect_mask);
    draw_clip_onto_canvas(&mut canvas_mask, &masked_clip, &white_frame, 1.0);
    assert!(
        canvas_mask.get_pixel(16, 16)[0] > 240,
        "Center inside mask should be white"
    );
    assert_eq!(
        canvas_mask.get_pixel(1, 1)[0],
        10,
        "Corner outside mask should remain backdrop"
    );

    // 3. Adjustment Layer with brightness boost + ColorAndEffects mask affects only center
    let mut canvas_adj = ImageBuffer::from_pixel(32, 32, Rgba([100, 100, 100, 255]));
    let mut adj = Clip::new_adjustment_layer(0.0, 5.0);
    adj.brightness = 0.4;
    let mut adj_mask = Mask::new(MaskShape::Rectangle, "Grade Box");
    adj_mask.scale_x = 0.4;
    adj_mask.scale_y = 0.4;
    adj_mask.feather = 0.0;
    adj_mask.target = MaskTarget::ColorAndEffects;
    adj.masks.push(adj_mask);
    let sentinel = ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0]));
    draw_clip_onto_canvas(&mut canvas_adj, &adj, &sentinel, 1.0);
    assert!(
        canvas_adj.get_pixel(16, 16)[0] > 170,
        "Adjustment layer center should be brightened: {:?}",
        canvas_adj.get_pixel(16, 16)
    );
    assert_eq!(
        canvas_adj.get_pixel(1, 1)[0],
        100,
        "Adjustment layer outside ColorAndEffects mask should keep backdrop"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn test_playback_streamer_panic_cleans_up_ffmpeg_child() {
    if !crate::ffmpeg::is_ffmpeg_available() {
        return;
    }
    let tmp = tempfile::tempdir().expect("tempdir");
    let vid_path = tmp.path().join("test_panic.mp4");
    let gen = crate::ffmpeg::ffmpeg_cmd()
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=5:size=320x180:rate=30",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&vid_path)
        .output()
        .expect("spawn ffmpeg testsrc");
    assert!(gen.status.success(), "testsrc generation failed");

    let vid_str = vid_path.to_string_lossy().to_string();
    let pid_slot = Arc::new(Mutex::new(None::<u32>));
    let pid_capture = pid_slot.clone();

    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let mut streamer = PlaybackStreamer::new();
        streamer.start("clip_panic", &vid_str, 0.0, 30.0, false, 0.0);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while std::time::Instant::now() < deadline {
            if let Some(ref arc) = streamer.active_child {
                if let Ok(guard_opt) = arc.lock() {
                    if let Some(ref g) = *guard_opt {
                        if let Some(pid) = g.id() {
                            if let Ok(mut slot) = pid_capture.lock() {
                                *slot = Some(pid);
                            }
                            break;
                        }
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        panic!("intentional panic to test ChildGuard::drop");
    }));
    assert!(res.is_err(), "expected panic");

    if let Some(pid) = pid_slot.lock().ok().and_then(|g| *g) {
        let proc_dir = std::path::PathBuf::from(format!("/proc/{pid}"));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while proc_dir.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(
            !proc_dir.exists(),
            "FFmpeg child PID {pid} survived panic/unwind!"
        );
    }
}

#[test]
fn test_frame_tolerance_scales_with_fps_and_bounds_vfr() {
    let tol_24 = frame_tolerance_for_fps(24.0);
    let tol_30 = frame_tolerance_for_fps(30.0);
    let tol_60 = frame_tolerance_for_fps(60.0);
    assert!((tol_24 - 0.5 / 24.0).abs() < 1e-6);
    assert!((tol_30 - 0.5 / 30.0).abs() < 1e-6);
    assert!((tol_60 - 0.5 / 60.0).abs() < 1e-6);
    // Must always be strictly smaller than 1 frame period
    assert!(tol_24 < 1.0 / 24.0);
    assert!(tol_30 < 1.0 / 30.0);
    assert!(tol_60 < 1.0 / 60.0);
    // VFR / extreme bounds
    assert!((frame_tolerance_for_fps(5.0) - 0.030).abs() < 1e-6);
    assert!((frame_tolerance_for_fps(240.0) - 0.004).abs() < 1e-6);
    assert!((frame_tolerance_for_fps(f64::NAN) - 0.5 / 30.0).abs() < 1e-6);
}

#[test]
fn test_select_closest_frame_24_30_60_fps_no_adjacent_mismatch() {
    for fps in [24.0_f64, 30.0_f64, 60.0_f64] {
        let step = 1.0 / fps;
        let tol = frame_tolerance_for_fps(fps);
        let frames: Vec<(f64, usize)> = (0..10).map(|i| (i as f64 * step, i)).collect();

        // Exact and near-exact requests select the closest frame
        let target_3 = 3.0 * step + 0.2 * step;
        let (pts, idx) =
            select_closest_frame_within_tolerance(frames.iter(), target_3, tol).unwrap();
        assert_eq!(*idx, 3, "fps={fps} should pick frame 3");
        assert!((pts - 3.0 * step).abs() < 1e-9);

        // Requesting 1 full frame step away from frame 0 must NOT pick frame 0
        let only_frame_0 = [(0.0, 0usize)];
        assert!(
            select_closest_frame_within_tolerance(only_frame_0.iter(), step, tol).is_none(),
            "fps={fps}: adjacent frame at +1 frame period must not match frame 0"
        );
    }
}

#[test]
fn test_persistent_seek_decoder_no_stale_frame_on_forward_scrub() {
    if !crate::ffmpeg::is_ffmpeg_available() {
        return;
    }
    let tmp = tempfile::tempdir().expect("tempdir");
    let vid_path = tmp.path().join("scrub_distinct.mp4");
    let gen = crate::ffmpeg::ffmpeg_cmd()
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=3:size=160x90:rate=30",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&vid_path)
        .output()
        .expect("spawn ffmpeg testsrc");
    assert!(gen.status.success(), "testsrc generation failed");

    let vid_str = vid_path.to_string_lossy().to_string();
    let mut decoder = PersistentSeekDecoder::new();
    let f0 = decoder
        .seek_with_fps(&vid_str, 0.2, 30.0, 160, 90)
        .expect("frame at 0.2s");
    let f1 = decoder
        .seek_with_fps(&vid_str, 0.8, 30.0, 160, 90)
        .expect("frame at 0.8s");
    // Verify Arc sharing (zero-copy) on cached re-query at 0.8s
    let f1_cached = decoder
        .seek_with_fps(&vid_str, 0.8, 30.0, 160, 90)
        .expect("cached frame at 0.8s");
    assert!(Arc::ptr_eq(&f1, &f1_cached));
    // testsrc counter changes visually across 0.6s, so f0 and f1 must differ
    assert_ne!(
        f0.as_raw(),
        f1.as_raw(),
        "forward scrub returned stale frame!"
    );
}
