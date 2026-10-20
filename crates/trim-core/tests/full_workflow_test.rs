//! Integration test verifying full workflow parity with Python implementation.

use trim_core::clip::Clip;
use trim_core::media::{MediaItem, MediaType};
use trim_core::project::Project;
use trim_core::time::{seconds_to_timecode, timecode_to_seconds};

#[test]
fn test_full_nle_workflow_core() {
    let mut project = Project::new("Full NLE Test");

    // 1. Add synthetic media items
    let mut m1 = MediaItem::new("/media/intro.mp4");
    m1.media_type = MediaType::Video;
    m1.duration = 1.5;
    m1.width = 640;
    m1.height = 360;
    m1.fps = 30.0;
    m1.video_codec = "h264".into();
    m1.audio_codec = "aac".into();

    let mut m2 = MediaItem::new("/media/action.mp4");
    m2.media_type = MediaType::Video;
    m2.duration = 2.0;
    m2.width = 640;
    m2.height = 360;
    m2.fps = 30.0;
    m2.video_codec = "h264".into();
    m2.audio_codec = "aac".into();

    let mut m3 = MediaItem::new("/media/photo.png");
    m3.media_type = MediaType::Image;
    m3.duration = 1.5;
    m3.width = 640;
    m3.height = 360;

    let id1 = m1.id.clone();
    let id2 = m2.id.clone();
    let id3 = m3.id.clone();

    project.add_media(m1);
    project.add_media(m2);
    project.add_media(m3);

    assert_eq!(project.media_pool.len(), 3);

    // 2. Add clips to Video Track
    let v_track = project.timeline.get_video_tracks_mut().into_iter().next().unwrap();
    let c1 = Clip::new(&id1, 0.0, 1.5);
    let c2 = Clip::new(&id2, 1.5, 3.5);
    let c3 = Clip::new(&id3, 3.5, 5.0);

    v_track.add_clip(c1);
    v_track.add_clip(c2);
    v_track.add_clip(c3);

    assert!((project.timeline.duration() - 5.0).abs() < 1e-4);

    // 3. Test Timecode
    assert_eq!(seconds_to_timecode(5.0, 30.0), "00:00:05:00");
    assert_eq!(timecode_to_seconds("00:00:05:00", 30.0), Some(5.0));

    // 4. Test Source Mapping
    let track = project.timeline.get_video_tracks().into_iter().next().unwrap();
    let clip2 = track.get_clip_by_id(&track.clips[1].id).unwrap();
    assert_eq!(clip2.map_timeline_to_source(2.0), 0.5);

    // 5. Test Serialization roundtrip
    let json = project.to_json().unwrap();
    assert!(json.contains("\"format\": \"trim\""));
    let reloaded = Project::from_json(&json).unwrap();
    assert_eq!(reloaded.media_pool.len(), 3);
    assert!((reloaded.timeline.duration() - 5.0).abs() < 1e-4);
}
