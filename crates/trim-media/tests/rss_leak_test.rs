//! Long-session RSS memory bound test for frame buffer pooling and bounded LRU caching.

use trim_core::lru::LruCache;
use trim_media::frame_pool::FrameBufferPool;

#[cfg(target_os = "linux")]
fn read_rss_kb() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some(pages * 4)
}

#[test]
fn test_long_session_frame_pool_and_lru_rss_bounded() {
    const FRAME_BYTES: usize = 1920 * 1080 * 4; // ~8.29 MB per 1080p RGBA frame
    let pool = FrameBufferPool::new(8);
    let mut cache: LruCache<u64, Vec<u8>> = LruCache::with_byte_limit(64 * 1024 * 1024);

    let run_frames = |start: u64, end: u64, cache: &mut LruCache<u64, Vec<u8>>| {
        for idx in start..end {
            let mut buf = pool.checkout(FRAME_BYTES);
            buf[0] = (idx & 0xff) as u8;
            buf[FRAME_BYTES - 1] = ((idx >> 8) & 0xff) as u8;
            // Half go into bounded LRU cache, half are returned directly to the pool
            if idx % 2 == 0 {
                cache.insert(idx, buf);
            } else {
                pool.checkin(buf);
            }
        }
    };

    // Warmup 100 frames (~829 MB total frame traffic)
    run_frames(0, 100, &mut cache);

    #[cfg(target_os = "linux")]
    let rss_after_warmup_kb = read_rss_kb().expect("Failed to read /proc/self/statm");

    // Simulate 900 more frames (~7.46 GB cumulative frame traffic)
    run_frames(100, 1000, &mut cache);

    #[cfg(target_os = "linux")]
    {
        let rss_end_kb = read_rss_kb().expect("Failed to read /proc/self/statm");
        let delta_kb = rss_end_kb.saturating_sub(rss_after_warmup_kb);
        eprintln!(
            "RSS after 100-frame warmup: {} KB, after 1000 frames: {} KB (delta: +{} KB)",
            rss_after_warmup_kb, rss_end_kb, delta_kb
        );
        assert!(
            delta_kb <= 32_768,
            "RSS grew unexpectedly over 900 1080p frames: warmup={} KB, end={} KB (+{} KB)",
            rss_after_warmup_kb,
            rss_end_kb,
            delta_kb
        );
    }
}
