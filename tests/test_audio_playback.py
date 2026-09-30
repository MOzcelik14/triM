import os
from pathlib import Path
import subprocess
import tempfile
import time
import pytest

os.environ["QT_QPA_PLATFORM"] = "offscreen"

from cutline.app.application import CutlineApplication
from cutline.core.clip import Clip
from cutline.core.project import Project
from cutline.media.ffprobe import FFprobeAnalyzer
from cutline.media.playback import PlaybackEngine


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
