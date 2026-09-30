import os
from pathlib import Path
import subprocess
import pytest

os.environ["QT_QPA_PLATFORM"] = "offscreen"

from trim.app.application import CutlineApplication
from trim.core.clip import Clip
from trim.core.media import MediaItem, MediaType
from trim.core.project import Project
from trim.export.exporter import TimelineExporter
from trim.export.presets import DEFAULT_PRESETS
from trim.ui.dialogs.title_dialog import TitleDialog


@pytest.fixture
def sample_media(tmp_path: Path) -> str:
    path = str(tmp_path / "fade_source.mp4")
    cmd = [
        "ffmpeg", "-y", "-v", "error",
        "-f", "lavfi", "-i", "testsrc=duration=3:size=320x240:rate=30",
        "-f", "lavfi", "-i", "sine=frequency=440:duration=3",
        "-c:v", "libx264", "-c:a", "aac",
        path
    ]
    subprocess.run(cmd, check=True)
    return path


def test_export_with_fades_and_title(tmp_path: Path, sample_media: str):
    app = CutlineApplication.instance() or CutlineApplication([])
    project = Project()

    # 1. Base video clip with fade in and fade out
    media_vid = MediaItem(file_path=sample_media, name="Video", media_type=MediaType.VIDEO, duration=3.0, audio_codec="aac")
    project.add_media(media_vid)

    v1_track = project.timeline.get_video_tracks()[0]
    clip1 = Clip(
        media_id=media_vid.id,
        timeline_in=0.0,
        timeline_out=2.5,
        fade_in=0.5,
        fade_out=0.5,
        volume=0.8,
    )
    v1_track.add_clip(clip1)

    # 2. Title clip
    dlg = TitleDialog(project)
    dlg.txt_content.setPlainText("Son")
    dlg.spn_duration.setValue(1.5)
    title_media, _ = dlg.create_title_media()
    project.add_media(title_media)

    clip2 = Clip(
        media_id=title_media.id,
        timeline_in=2.5,
        timeline_out=4.0,
        fade_in=0.3,
        fade_out=0.3,
    )
    v1_track.add_clip(clip2)

    # 3. Export
    out_file = str(tmp_path / "exported_with_fades.mp4")
    preset = DEFAULT_PRESETS[0]  # 1080p Standard
    cmd, dur = TimelineExporter.build_export_pipeline(project, preset, out_file)

    assert "fade=t=in" in " ".join(cmd)
    assert "afade=t=in" in " ".join(cmd)

    proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    assert proc.returncode == 0, f"FFmpeg failed: {proc.stderr.decode()}"
    assert Path(out_file).is_file()
    assert Path(out_file).stat().st_size > 1000
