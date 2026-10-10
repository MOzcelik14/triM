import os
from pathlib import Path
import subprocess
import tempfile
import time
import pytest

os.environ["QT_QPA_PLATFORM"] = "offscreen"

from trim.app.application import CutlineApplication
from trim.core.clip import Clip
from trim.core.project import Project
from trim.media.ffprobe import FFprobeAnalyzer
from trim.media.playback import PlaybackEngine


def test_audio_playback_engine():
    app = CutlineApplication.instance() or CutlineApplication([])

    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)
        vid = tmp_path / "sound_video.mp4"

        # Create 2-second video with 440Hz sine wave audio
        subprocess.run([
            "ffmpeg", "-y", "-v", "quiet",
            "-f", "lavfi", "-i", "testsrc=duration=2:size=320x240:rate=30",
            "-f", "lavfi", "-i", "sine=frequency=440:duration=2",
            "-c:v", "libx264", "-c:a", "aac",
            str(vid)
        ], check=True)

        proj = Project()
        media = FFprobeAnalyzer.analyze(vid)
        proj.add_media(media)

        v_track = proj.timeline.get_video_tracks()[0]
        clip = Clip(media_id=media.id, timeline_in=0.0, timeline_out=2.0)
        v_track.add_clip(clip)

        engine = PlaybackEngine(proj)

        # Seek to middle
        engine.seek(1.0)
        assert abs(engine.current_time - 1.0) < 1e-4

        # Start playback
        engine.play()
        assert engine.is_playing

        # Let it run through multiple ticks
        time.sleep(0.1)
        app.processEvents()

        assert engine.current_time > 1.0

        # Pause
        engine.pause()
        assert not engine.is_playing

        engine.close()


def test_audio_only_track_playback():
    app = CutlineApplication.instance() or CutlineApplication([])

    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)
        audio_file = tmp_path / "test_music.mp3"

        # Generate a 2-second MP3 sine wave at 440Hz
        subprocess.run([
            "ffmpeg", "-y", "-v", "quiet",
            "-f", "lavfi", "-i", "sine=frequency=440:duration=2",
            "-c:a", "libmp3lame", "-b:a", "192k",
            str(audio_file)
        ], check=True)

        proj = Project()
        media = FFprobeAnalyzer.analyze(audio_file)
        proj.add_media(media)

        # Place on audio track
        a_tracks = proj.timeline.get_audio_tracks()
        assert len(a_tracks) > 0
        a_track = a_tracks[0]

        clip = Clip(media_id=media.id, timeline_in=0.0, timeline_out=2.0)
        a_track.add_clip(clip)

        engine = PlaybackEngine(proj)

        # Capture audio levels
        levels_captured = []
        engine.audio_levels_ready.connect(lambda l, r: levels_captured.append((l, r)))

        # Test audio interval rendering directly
        pcm_bytes, db_l, db_r = engine._render_audio_interval(0.0, 0.5)
        assert len(pcm_bytes) == int(round(0.5 * 48000 * 4))
        assert db_l > -40.0
        assert db_r > -40.0

        # Audio signal output requires a device; hosted CI has none.
        # PCM generation above is still tested on headless runners.
        from PySide6.QtMultimedia import QMediaDevices
        if QMediaDevices.defaultAudioOutput().isNull():
            engine.close()
            pytest.skip("No audio output device in headless CI (PCM rendering passed)")

        # Start playback
        engine.play()
        assert engine.is_playing

        time.sleep(0.1)
        app.processEvents()

        assert len(levels_captured) > 0
        assert any(l > -40.0 for l, r in levels_captured)

        engine.pause()
        engine.close()


def test_reverse_audio_playback():
    app = CutlineApplication.instance() or CutlineApplication([])

    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)
        wav_file = tmp_path / "chirp.wav"

        # Generate 1-second audio with ascending chirp
        subprocess.run([
            "ffmpeg", "-y", "-v", "quiet",
            "-f", "lavfi", "-i", "sine=frequency=200:duration=1",
            str(wav_file)
        ], check=True)

        proj = Project()
        media = FFprobeAnalyzer.analyze(wav_file)
        proj.add_media(media)

        a_track = proj.timeline.get_audio_tracks()[0]
        clip = Clip(media_id=media.id, timeline_in=0.0, timeline_out=1.0)
        a_track.add_clip(clip)

        engine = PlaybackEngine(proj)

        # Forward PCM
        pcm_fwd, _, _ = engine._render_audio_interval(0.0, 0.2)

        # Reverse clip
        clip.reverse = True
        pcm_rev, _, _ = engine._render_audio_interval(0.0, 0.2)

        assert len(pcm_fwd) == len(pcm_rev)
        assert len(pcm_fwd) > 0
        engine.close()


def test_multi_track_audio_mixing_and_mute():
    app = CutlineApplication.instance() or CutlineApplication([])

    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)
        wav1 = tmp_path / "track1.wav"
        wav2 = tmp_path / "track2.wav"

        subprocess.run([
            "ffmpeg", "-y", "-v", "quiet",
            "-f", "lavfi", "-i", "sine=frequency=440:duration=1",
            str(wav1)
        ], check=True)
        subprocess.run([
            "ffmpeg", "-y", "-v", "quiet",
            "-f", "lavfi", "-i", "sine=frequency=880:duration=1",
            str(wav2)
        ], check=True)

        proj = Project()
        m1 = FFprobeAnalyzer.analyze(wav1)
        m2 = FFprobeAnalyzer.analyze(wav2)
        proj.add_media(m1)
        proj.add_media(m2)

        t1 = proj.timeline.tracks[0]  # Video 1
        t2 = proj.timeline.get_audio_tracks()[0]  # Audio 1

        c1 = Clip(media_id=m1.id, timeline_in=0.0, timeline_out=1.0)
        c2 = Clip(media_id=m2.id, timeline_in=0.0, timeline_out=1.0)
        t1.add_clip(c1)
        t2.add_clip(c2)

        engine = PlaybackEngine(proj)

        # Both unmuted
        pcm_both, db_l_both, _ = engine._render_audio_interval(0.0, 0.1)
        assert db_l_both > -40.0

        # Mute track 2
        t2.muted = True
        pcm_t1_only, db_l_t1, _ = engine._render_audio_interval(0.0, 0.1)

        # Mute track 1 as well -> should be silent
        t1.muted = True
        pcm_silent, db_l_silent, _ = engine._render_audio_interval(0.0, 0.1)
        assert db_l_silent == -60.0

        engine.close()
