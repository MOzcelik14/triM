//! Audio decoding, multi-track PCM mixing, lookahead brickwall limiting, and dual VU meter RMS level analysis.

mod mixer;
mod player;
mod worker;

pub use mixer::*;
pub use player::*;
pub use worker::*;

#[cfg(test)]
mod tests {
    use super::mixer::decode_audio_to_pcm;
    use super::*;
    use std::f32::consts::TAU;
    use std::sync::Arc;
    use trim_core::clip::Clip;
    use trim_core::media::{MediaItem, MediaType};
    use trim_core::project::Project;
    use trim_core::track::{Track, TrackType};

    #[test]
    fn test_audio_mixer_empty_timeline() {
        let mut mixer = AudioMixer::new();
        let project = Project::new("Empty");
        let (samples, db_l, db_r) = mixer.render_audio_interval(&project, 0.0, 1.0);
        assert_eq!(samples.len(), SAMPLE_RATE as usize * 2);
        // All zero samples
        assert!(samples.iter().all(|&s| s == 0));
        assert_eq!(db_l, -60.0);
        assert_eq!(db_r, -60.0);
    }

    #[test]
    fn test_audio_stream_player_lifecycle() {
        let mut player = AudioStreamPlayer::new();
        player.write_samples(&[]);
        assert!(player.child.is_none());
        player.stop();
        assert!(player.child.is_none());
    }

    #[test]
    fn test_audio_mixer_pcm_cache_bounded() {
        let mixer = AudioMixer::new();
        // Insert a 256MB synthetic buffer
        let samples_count = 64 * 1024 * 1024; // 64M stereo pairs = 64M * 4 bytes = 256MB
        let buf = Arc::new(vec![[0i16, 0i16]; samples_count]);
        {
            let mut cache = mixer.pcm_cache.write().unwrap();
            cache.insert_with_size(
                "file1.wav".to_string(),
                Some(buf.clone()),
                samples_count * 4,
            );
            assert_eq!(cache.len(), 1);
            assert_eq!(cache.total_bytes(), 256 * 1024 * 1024);
        }

        // Insert another 256MB synthetic buffer -> 512MB total
        {
            let mut cache = mixer.pcm_cache.write().unwrap();
            cache.insert_with_size(
                "file2.wav".to_string(),
                Some(buf.clone()),
                samples_count * 4,
            );
            assert_eq!(cache.len(), 2);
            assert_eq!(cache.total_bytes(), 512 * 1024 * 1024);
        }

        // Insert a third buffer (64MB) -> exceeds 512MB limit, evicts least recently used ("file1.wav")
        let small_count = 16 * 1024 * 1024; // 64MB
        let small_buf = Arc::new(vec![[0i16, 0i16]; small_count]);
        {
            let mut cache = mixer.pcm_cache.write().unwrap();
            cache.insert_with_size("file3.wav".to_string(), Some(small_buf), small_count * 4);
            assert_eq!(cache.len(), 2);
            assert!(!cache.contains_key(&"file1.wav".to_string()));
            assert!(cache.contains_key(&"file2.wav".to_string()));
            assert!(cache.contains_key(&"file3.wav".to_string()));
            assert_eq!(cache.total_bytes(), 320 * 1024 * 1024);
        }
    }

    #[test]
    fn test_audio_mixer_multitrack_overdrive_brickwall_limited() {
        let mut mixer = AudioMixer::new();
        let ceiling = mixer.limiter.ceiling();

        // Create a synthetic 0 dBFS stereo sine wave (L = 1.0, R = 0.5) of 0.2s (9600 frames)
        let pcm_frames: Vec<[i16; 2]> = (0..9600)
            .map(|i| {
                let s = (TAU * 440.0 * (i as f32 / SAMPLE_RATE as f32)).sin();
                [(s * 32767.0) as i16, (s * 16383.5) as i16]
            })
            .collect();

        let fake_path = "/tmp/synthetic_overdrive_test.wav";
        mixer.insert_cached_pcm(fake_path, pcm_frames);

        let mut project = Project::new("OverdriveTest");
        let mut media = MediaItem::new(fake_path);
        media.media_type = MediaType::Audio;
        media.duration = 0.2;
        let media_id = media.id.clone();
        project.add_media(media);

        // Add 2 full-scale audio tracks (summing to +6 dBFS = 2.0x overdrive on Left, 1.0x on Right)
        project.timeline.tracks.clear();
        for idx in 0..2 {
            let mut tr = Track::new(format!("A{}", idx + 1), TrackType::Audio);
            tr.add_clip(Clip::new(&media_id, 0.0, 0.2));
            project.timeline.add_track(tr);
        }

        // Mix in two contiguous 50ms chunks to verify seamless lookahead continuity and AV sync
        let chunk1 = mixer.mix_timeline_chunk(&project, 0.0, 0.05).to_vec();
        let chunk2 = mixer.mix_timeline_chunk(&project, 0.05, 0.10).to_vec();

        for frame in chunk1.iter().chain(chunk2.iter()) {
            let peak = frame[0].abs().max(frame[1].abs());
            assert!(
                peak <= ceiling + 1e-5,
                "Mixed peak {} exceeded -0.1 dBFS ceiling {}",
                peak,
                ceiling
            );
            // Verify 2:1 Left/Right stereo balance is preserved under gain reduction
            assert!(
                (frame[0] - 2.0 * frame[1]).abs() < 1e-3,
                "Stereo balance distorted: L={}, R={}",
                frame[0],
                frame[1],
            );
            if frame[0].abs() > 0.05 {
                let ratio = frame[0] / frame[1];
                assert!(
                    (ratio - 2.0).abs() < 5e-3,
                    "Stereo balance distorted: L={}, R={}, ratio={}",
                    frame[0],
                    frame[1],
                    ratio
                );
            }
        }
    }

    #[test]
    fn test_decode_sony_ipcm_unrecognized_mp4_audio() {
        if !crate::is_ffmpeg_available() {
            return;
        }
        let tmp_mov = std::env::temp_dir().join("trim_test_sony_twos.mov");
        let tmp_mp4 = std::env::temp_dir().join("trim_test_sony_ipcm.mp4");

        // Generate a 0.2s stereo 48kHz pcm_s16be MOV and patch its sample entry tag to `ipcm`
        // (without a pcmC box), reproducing how Pop!_OS 22.04 (FFmpeg 4.4) sees Sony XAVC S MP4s.
        let status = crate::ffmpeg::ffmpeg_cmd()
            .args([
                "-y",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=48000:duration=0.2",
                "-ac",
                "2",
                "-c:a",
                "pcm_s16be",
            ])
            .arg(&tmp_mov)
            .status();

        if status.is_ok_and(|s| s.success()) {
            if let Ok(mut bytes) = std::fs::read(&tmp_mov) {
                if let Some(pos) = bytes.windows(4).position(|w| w == b"twos") {
                    bytes[pos..pos + 4].copy_from_slice(b"ipcm");
                    let _ = std::fs::write(&tmp_mp4, &bytes);

                    let info = crate::analyzer::detect_forced_pcm_input_codec(&tmp_mp4);
                    assert!(info.is_some(), "Should detect ipcm PCM info");
                    assert_eq!(info.unwrap().codec, "pcm_s16be");

                    let pcm = decode_audio_to_pcm(&tmp_mp4);
                    assert!(
                        pcm.is_some(),
                        "Should decode Sony ipcm MP4 audio via forced pcm_s16be"
                    );
                    let frames = pcm.unwrap();
                    assert!(!frames.is_empty(), "Decoded PCM frames should not be empty");
                    let max_amp = frames
                        .iter()
                        .map(|f| f[0].unsigned_abs())
                        .max()
                        .unwrap_or(0);
                    assert!(
                        max_amp > 1000,
                        "Decoded sine wave should be audible, got max_amp={}",
                        max_amp
                    );
                }
            }
        }
        let _ = std::fs::remove_file(&tmp_mov);
        let _ = std::fs::remove_file(&tmp_mp4);
    }

    #[test]
    fn test_plan_audio_chunk_forward_and_reverse() {
        assert_eq!(
            plan_audio_chunk(1.0, 1.0, 0.02, 5.0),
            Some((1.0, 1.02, false))
        );
        assert_eq!(
            plan_audio_chunk(4.99, 1.0, 0.02, 5.0),
            Some((4.99, 5.0, false))
        );
        assert_eq!(plan_audio_chunk(5.0, 1.0, 0.02, 5.0), None);
        assert_eq!(
            plan_audio_chunk(2.0, -2.0, 0.02, 5.0),
            Some((1.96, 2.0, true))
        );
        assert_eq!(plan_audio_chunk(0.0, -1.0, 0.02, 5.0), None);
    }

    #[test]
    fn test_compute_audio_clock_time_with_latency() {
        // 4800 frames at 48kHz = 0.100s; minus 0.025s sink latency = 0.075s
        let t = compute_audio_clock_time(1.0, 1.0, 4800, 48000, 0.025);
        assert!((t - 1.075).abs() < 1e-6);

        // Reverse 2x: 4800 frames -> 0.075s * -2.0 = -0.150s from base 2.0 -> 1.850s
        let rev_t = compute_audio_clock_time(2.0, -2.0, 4800, 48000, 0.025);
        assert!((rev_t - 1.85).abs() < 1e-6);
    }

    #[test]
    fn test_process_playback_chunk_reverse_and_speed() {
        let samples = vec![100, -100, 200, -200, 300, -300, 400, -400];
        let (out, _, _) = process_playback_chunk(samples, -6.0, -6.0, true, 2.0, 1.0);
        // Reversed: [400, -400, 300, -300, 200, -200, 100, -100], 2x speed -> 2 stereo frames
        assert_eq!(out, vec![400, -400, 200, -200]);
    }

    #[test]
    fn test_windowed_audio_chunk_cache_and_device_reset() {
        let mixer = AudioMixer::new();
        mixer.set_max_cache_bytes(32 * 1024 * 1024);
        // Simulate two 10s chunks stored under the windowed chunk keys
        mixer.insert_cached_pcm(
            "/tmp/fake_2h.wav#chunk:0",
            vec![[1000, -1000]; SAMPLE_RATE as usize * 10],
        );
        mixer.insert_cached_pcm(
            "/tmp/fake_2h.wav#chunk:1",
            vec![[2000, -2000]; SAMPLE_RATE as usize * 10],
        );
        // Crossing the 10.0s boundary (9.9s..10.1s) should stitch both chunks cleanly
        let (stitched, base_offset) = mixer
            .get_audio_pcm_window("/tmp/fake_2h.wav", 9.9, 10.1)
            .expect("stitched window");
        assert_eq!(base_offset, 0);
        assert_eq!(stitched.len(), SAMPLE_RATE as usize * 20);
        assert!(mixer.current_cache_bytes() < 32 * 1024 * 1024);

        let mut player = AudioStreamPlayer::new();
        player.reset_device();
        player.stop();
    }
}
