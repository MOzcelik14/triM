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


def test_playback_realtime_speed():
    app = CutlineApplication.instance() or CutlineApplication([])

    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)
        vid = tmp_path / "ten_seconds.mp4"

        # 10s video with audio
        subprocess.run([
            "ffmpeg", "-y", "-v", "quiet",
            "-f", "lavfi", "-i", "testsrc=duration=10:size=320x240:rate=30",
            "-f", "lavfi", "-i", "sine=frequency=440:duration=10",
            "-c:v", "libx264", "-c:a", "aac",
            str(vid)
        ], check=True)

        proj = Project()
        item = FFprobeAnalyzer.analyze(vid)
        assert abs(item.duration - 10.0) < 0.2
        proj.add_media(item)

        clip = Clip(media_id=item.id, timeline_in=0.0, timeline_out=item.duration)
        proj.timeline.get_video_tracks()[0].add_clip(clip)
        assert abs(proj.timeline.duration - 10.0) < 0.2

        engine = PlaybackEngine(proj)
        engine.seek(0.0)
        assert engine.current_time == 0.0

        engine.play()
        assert engine.is_playing

        # Let play for exactly 1.0 second
        t_start = time.time()
        while time.time() - t_start < 1.0:
            app.processEvents()
            time.sleep(0.01)

        engine.pause()
        assert not engine.is_playing

        # After 1.0 second, playback time should be very close to 1.0 second (not 3s, not 10s)
        print("Engine current time after 1s:", engine.current_time)
        assert 0.85 <= engine.current_time <= 1.25

        engine.close()
