import os
from pathlib import Path
import tempfile
import pytest

from trim.core.project import Project, ProjectSettings
from trim.core.media import MediaItem, MediaType
from trim.core.track import Track, TrackType
from trim.core.clip import Clip


def test_project_save_and_load():
    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)
        proj_file = tmp_path / "my_project.cutline"
        sample_media = tmp_path / "test.mp4"
        sample_media.write_text("fake video data")

        proj = Project(settings=ProjectSettings(name="Test Project", fps=60.0))
        item = MediaItem(
            file_path=str(sample_media),
            duration=12.5,
            fps=60.0,
            width=1920,
            height=1080,
            media_type=MediaType.VIDEO,
        )
        proj.add_media(item)

        track = proj.timeline.get_video_tracks()[0]
        clip = Clip(media_id=item.id, timeline_in=0.0, timeline_out=5.0)
        track.add_clip(clip)

        proj.save(str(proj_file))
        assert proj_file.is_file()
        assert not proj.is_dirty

        # Reload
        loaded = Project.load(str(proj_file))
        assert loaded.settings.name == "Test Project"
        assert loaded.settings.fps == 60.0
        assert len(loaded.media_pool) == 1
        loaded_item = list(loaded.media_pool.values())[0]
        assert Path(loaded_item.file_path).name == "test.mp4"

        v_tracks = loaded.timeline.get_video_tracks()
        assert len(v_tracks) >= 1
        assert len(v_tracks[0].clips) == 1
        assert v_tracks[0].clips[0].duration == 5.0
