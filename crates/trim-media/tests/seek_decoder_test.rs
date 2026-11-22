use std::path::PathBuf;
use std::time::{Duration, Instant};
use trim_media::ffmpeg::is_ffmpeg_available;
use trim_media::video::{AsyncFrameDecoder, PersistentSeekDecoder};

#[test]
fn test_persistent_seek_decoder_lifecycle_and_scrubbing() {
    if !is_ffmpeg_available() {
        return;
    }

    let clip_path = PathBuf::from("/home/mozcelik/İndirilenler/MVI_8603.MOV");
    if !clip_path.exists() {
        // Fallback for environments without the test clip
        return;
    }
    let p_str = clip_path.to_string_lossy().to_string();

    let mut decoder = PersistentSeekDecoder::new();

    // 1. Initial seek (starts persistent streamer)
    let t0 = Instant::now();
    let frame1 = decoder.seek(&p_str, 2.0, 320, 180);
    let dur1 = t0.elapsed();
    assert!(frame1.is_some(), "Initial seek must return a valid frame");

    // 2. Sequential forward seek (+0.05s, within temporal locality)
    let t1 = Instant::now();
    let frame2 = decoder.seek(&p_str, 2.05, 320, 180);
    let dur2 = t1.elapsed();
    assert!(
        frame2.is_some(),
        "Sequential forward seek must return a valid frame"
    );

    // Sequential seek should be fast (retrieved from pipe without full process spawn)
    println!("Initial seek: {dur1:?}, Sequential forward scrub seek: {dur2:?}");

    // 3. Backward seek to previous timestamp (served from ring buffer)
    let t2 = Instant::now();
    let frame_back = decoder.seek(&p_str, 2.0, 320, 180);
    let dur_back = t2.elapsed();
    assert!(
        frame_back.is_some(),
        "Backward seek within ring buffer must return a frame"
    );
    assert!(
        dur_back < Duration::from_millis(15),
        "Ring buffer backward seek must be near-instant (<15ms), took {dur_back:?}"
    );

    // 4. Drop decoder - ChildGuard must cleanly reap FFmpeg process
    drop(decoder);
}

#[test]
fn test_async_frame_decoder_with_persistent_worker() {
    if !is_ffmpeg_available() {
        return;
    }

    let clip_path = PathBuf::from("/home/mozcelik/İndirilenler/MVI_8603.MOV");
    if !clip_path.exists() {
        return;
    }
    let p_str = clip_path.to_string_lossy().to_string();

    let mut async_decoder = AsyncFrameDecoder::new();

    let req_id1 = async_decoder.request_seek_sized(&p_str, 5.0, 320, 180);
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut got_res1 = None;
    while Instant::now() < deadline {
        if let Some(res) = async_decoder.poll_result() {
            if res.id == req_id1 {
                got_res1 = Some(res);
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        got_res1.is_some(),
        "AsyncFrameDecoder must deliver first seek result"
    );

    // Subsequent sequential scrub request
    let req_id2 = async_decoder.request_seek_sized(&p_str, 5.08, 320, 180);
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut got_res2 = None;
    while Instant::now() < deadline {
        if let Some(res) = async_decoder.poll_result() {
            if res.id == req_id2 {
                got_res2 = Some(res);
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        got_res2.is_some(),
        "AsyncFrameDecoder must deliver sequential scrub result"
    );

    drop(async_decoder);
}
