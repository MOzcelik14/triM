import os
from pathlib import Path
import subprocess
import pytest

os.environ["QT_QPA_PLATFORM"] = "offscreen"

from trim.app.application import CutlineApplication
from trim.core.clip import Clip
from trim.core.media import MediaItem, MediaType
from trim.core.project import Project
from trim.core.track import Track, TrackType
from trim.media.playback import PlaybackEngine


@pytest.fixture
def sample_video(tmp_path: Path) -> str:
    path = str(tmp_path / "comp_source.mp4")
    cmd = [
        "ffmpeg", "-y", "-v", "error",
        "-f", "lavfi", "-i", "testsrc=duration=4:size=320x240:rate=30",
        "-f", "lavfi", "-i", "sine=frequency=440:duration=4",
        "-c:v", "libx264", "-pix_fmt", "yuv420p",
        "-c:a", "aac",
        path
    ]
    subprocess.run(cmd, check=True)
    return path


def test_multitrack_and_transform_composition(sample_video: str):
    app = CutlineApplication.instance() or CutlineApplication([])
    project = Project()
    media_v1 = MediaItem(file_path=sample_video, name="Track1 Vid", media_type=MediaType.VIDEO, duration=4.0)
    media_v2 = MediaItem(file_path=sample_video, name="Track2 Overlay", media_type=MediaType.VIDEO, duration=4.0)
    project.add_media(media_v1)
    project.add_media(media_v2)

    # Track 1 (V1)
    v1_track = project.timeline.get_video_tracks()[0]
    clip1 = Clip(media_id=media_v1.id, timeline_in=0.0, timeline_out=3.0, name="Base")
    v1_track.add_clip(clip1)

    # Track 2 (V2 - on top)
    v2_track = Track(name="Video 2", track_type=TrackType.VIDEO)
    project.timeline.add_track(v2_track)
    clip2 = Clip(
        media_id=media_v2.id,
        timeline_in=0.5,
        timeline_out=2.5,
        name="Overlay",
        scale=0.5,
        pos_x=50.0,
        pos_y=30.0,
        opacity=0.75,
        fade_in=0.5,
        fade_out=0.5,
    )
    v2_track.add_clip(clip2)

    engine = PlaybackEngine(project)

    # Test seek at 0.2s (only V1 is active)
    frame_02 = engine._seek_and_render_frame(0.2)
    assert not frame_02.isNull()
    assert frame_02.width() == 1920
    assert frame_02.height() == 1080

    # Test seek at 1.0s (both V1 and V2 active, V2 overlaid with PIP transform)
    frame_10 = engine._seek_and_render_frame(1.0)
    assert not frame_10.isNull()
    assert frame_10.width() == 1920
    assert frame_10.height() == 1080

    engine.close()
