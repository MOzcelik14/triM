import subprocess
import numpy as np
import pytest
from PySide6.QtGui import QImage, QColor

from trim.core.clip import Clip
from trim.core.media import MediaItem, MediaType
from trim.core.project import Project
from trim.core.track import Track, TrackType
from trim.media.playback import PlaybackEngine
from trim.export.exporter import TimelineExporter
from trim.export.presets import DEFAULT_PRESETS


def test_clip_color_and_transition_serialization():
    clip = Clip(
        media_id="test_media",
        timeline_in=1.0,
        timeline_out=5.0,
        brightness=0.2,
        contrast=1.5,
        saturation=0.8,
        transition_in="dip_black",
        transition_out="dip_white",
    )
    d = clip.to_dict()
    assert d["brightness"] == 0.2
    assert d["contrast"] == 1.5
    assert d["saturation"] == 0.8
    assert d["transition_in"] == "dip_black"
    assert d["transition_out"] == "dip_white"

    cloned = clip.clone()
    assert cloned.brightness == 0.2
    assert cloned.contrast == 1.5
    assert cloned.saturation == 0.8
    assert cloned.transition_in == "dip_black"
    assert cloned.transition_out == "dip_white"

    restored = Clip.from_dict(d)
    assert restored.brightness == 0.2
    assert restored.contrast == 1.5
    assert restored.saturation == 0.8
    assert restored.transition_in == "dip_black"
    assert restored.transition_out == "dip_white"


def test_apply_color_adjustments(qapp):
    project = Project()
    engine = PlaybackEngine(project)

    # Create a 100x100 solid colored image (R=200, G=100, B=50)
    w, h = 100, 100
    img = QImage(w, h, QImage.Format.Format_RGB888)
    img.fill(QColor(200, 100, 50))

    # Test Saturation = 0 (Black & White conversion)
    bw_img = engine._apply_color_adjustments(img, brightness=0.0, contrast=1.0, saturation=0.0)
    assert not bw_img.isNull()
    pixel = bw_img.pixelColor(50, 50)
    # In grayscale, R == G == B (approximate due to integer rounding)
    assert abs(pixel.red() - pixel.green()) <= 2
    assert abs(pixel.green() - pixel.blue()) <= 2

    # Test Brightness increase
    bright_img = engine._apply_color_adjustments(img, brightness=0.2, contrast=1.0, saturation=1.0)
    b_pixel = bright_img.pixelColor(50, 50)
    assert b_pixel.red() > 200

    # Test Contrast increase
    contrast_img = engine._apply_color_adjustments(img, brightness=0.0, contrast=1.5, saturation=1.0)
    c_pixel = contrast_img.pixelColor(50, 50)
    assert c_pixel.red() != 200


def test_export_with_color_adjustments_and_transitions(tmp_path, qapp):
    media_path = str(tmp_path / "src_color.mp4")
    cmd = [
        "ffmpeg", "-y", "-v", "error",
        "-f", "lavfi", "-i", "testsrc=size=320x240:rate=30",
        "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000",
        "-t", "2.0",
        "-c:v", "libx264", "-pix_fmt", "yuv420p",
        "-c:a", "aac",
        media_path
    ]
    subprocess.run(cmd, check=True)

    project = Project()
    item = MediaItem(file_path=media_path, name="TestVid", media_type=MediaType.VIDEO, duration=2.0, audio_codec="aac")
    project.add_media(item)

    v1_track = project.timeline.get_video_tracks()[0]
    clip = Clip(
        media_id=item.id,
        timeline_in=0.0,
        timeline_out=2.0,
        source_in=0.0,
        source_out=2.0,
        brightness=0.1,
        contrast=1.2,
        saturation=0.5,
        fade_in=0.5,
        fade_out=0.5,
        transition_in="dip_black",
        transition_out="dip_white",
    )
    v1_track.add_clip(clip)

    out_file = str(tmp_path / "out_color.mp4")
    preset = DEFAULT_PRESETS[0]
    export_cmd, dur = TimelineExporter.build_export_pipeline(project, preset, out_file)

    # Verify eq filter and fade colors in command
    cmd_str = " ".join(export_cmd)
    assert "eq=brightness=0.100:contrast=1.200:saturation=0.500" in cmd_str
    assert "fade=t=in:st=0:d=0.5000:color=black" in cmd_str
    assert "color=white" in cmd_str

    proc = subprocess.run(export_cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    assert proc.returncode == 0, f"FFmpeg export failed: {proc.stderr.decode()}"
    assert (tmp_path / "out_color.mp4").is_file()
    assert (tmp_path / "out_color.mp4").stat().st_size > 1000

    # Check that output video was generated and has duration
    import av
    container = av.open(out_file)
    assert len(container.streams.video) > 0
    v = container.streams.video[0]
    assert v.width == 1920
    assert v.height == 1080
    container.close()
