from pathlib import Path
import subprocess
import tempfile
import pytest

from PySide6.QtGui import QUndoStack
from PySide6.QtWidgets import QApplication

from trim.commands.timeline_commands import (
    AddMarkerCommand,
    ChangeClipSpeedCommand,
    RemoveMarkerCommand,
)
from trim.core.clip import Clip
from trim.core.keyframe import Keyframe, interpolate_keyframes
from trim.core.marker import Marker
from trim.core.project import Project
from trim.core.timeline import TimelineModel
from trim.core.track import Track, TrackType
from trim.export.exporter import TimelineExporter
from trim.export.presets import DEFAULT_PRESETS
from trim.media.ffprobe import FFprobeAnalyzer
from trim.ui.dialogs.speed_dialog import SpeedDialog
from trim.ui.inspector.inspector_widget import InspectorWidget
from trim.ui.main_window import MainWindow


def create_synthetic_av_clip(path: Path, duration: float = 2.0) -> None:
    cmd = [
        "ffmpeg", "-y", "-v", "quiet",
        "-f", "lavfi", "-i", f"color=c=cyan:s=320x240:r=30:d={duration}",
        "-f", "lavfi", "-i", f"sine=frequency=500:duration={duration}",
        "-c:v", "libx264", "-pix_fmt", "yuv420p",
        "-c:a", "aac",
        str(path),
    ]
    subprocess.run(cmd, check=True)


# =====================================================================
# 1. Timeline Markers & Snapping Tests
# =====================================================================

def test_marker_creation_and_serialization():
    m = Marker(time=2.5, name="Verse 1", color="#3498db")
    d = m.to_dict()
    assert d["time"] == 2.5
    assert d["name"] == "Verse 1"
    assert d["color"] == "#3498db"

    restored = Marker.from_dict(d)
    assert restored.id == m.id
    assert restored.time == 2.5
    assert restored.name == "Verse 1"
    assert restored.color == "#3498db"


def test_timeline_marker_operations():
    tl = TimelineModel()
    m1 = tl.add_marker(1.0, "Intro")
    m2 = tl.add_marker(5.5, "Outro", color="#2ecc71")
    assert len(tl.markers) == 2
    assert tl.markers[0].time == 1.0
    assert tl.markers[1].time == 5.5

    # Find marker
    found = tl.get_marker_at(5.51, tolerance=0.05)
    assert found is not None
    assert found.name == "Outro"

    # Remove marker
    assert tl.remove_marker(m1.id) is not None
    assert len(tl.markers) == 1
    assert tl.remove_marker("nonexistent") is None


def test_magnetic_snapping_to_markers():
    tl = TimelineModel()
    tl.add_marker(4.0, "Beat")
    tl.snap_enabled = True

    # Near marker (threshold default 0.15s)
    snapped = tl.snap_time(4.05)
    assert snapped == 4.0

    # Far from marker
    snapped_far = tl.snap_time(4.5)
    assert snapped_far == 4.5


def test_marker_undo_redo_commands():
    tl = TimelineModel()
    stack = QUndoStack()

    cmd_add = AddMarkerCommand(tl, 3.2, "Chorus", color="#e07a38")
    stack.push(cmd_add)
    assert len(tl.markers) == 1
    marker_id = cmd_add.marker_id

    stack.undo()
    assert len(tl.markers) == 0

    stack.redo()
    assert len(tl.markers) == 1
    assert tl.markers[0].id == marker_id

    cmd_rem = RemoveMarkerCommand(tl, marker_id)
    stack.push(cmd_rem)
    assert len(tl.markers) == 0

    stack.undo()
    assert len(tl.markers) == 1
    assert tl.markers[0].id == marker_id


# =====================================================================
# 2. Keyframe Animation Tests
# =====================================================================

def test_keyframe_serialization():
    kf = Keyframe(time=1.5, value=0.75, interpolation="ease_in_out")
    d = kf.to_dict()
    assert d == {"time": 1.5, "value": 0.75, "interpolation": "ease_in_out"}

    restored = Keyframe.from_dict(d)
    assert restored.time == 1.5
    assert restored.value == 0.75
    assert restored.interpolation == "ease_in_out"


def test_keyframe_interpolation_curves():
    # Empty
    assert interpolate_keyframes([], 2.0, default_value=1.0) == 1.0

    # Single
    kfs = [Keyframe(time=1.0, value=50.0)]
    assert interpolate_keyframes(kfs, 0.5, 0.0) == 50.0
    assert interpolate_keyframes(kfs, 2.0, 0.0) == 50.0

    # Two linear keyframes: 0.0s -> 0.0, 2.0s -> 100.0
    kfs_lin = [
        Keyframe(time=0.0, value=0.0, interpolation="linear"),
        Keyframe(time=2.0, value=100.0, interpolation="linear"),
    ]
    assert interpolate_keyframes(kfs_lin, -0.5, 0.0) == 0.0
    assert interpolate_keyframes(kfs_lin, 2.5, 0.0) == 100.0
    assert abs(interpolate_keyframes(kfs_lin, 1.0, 0.0) - 50.0) < 1e-4
    assert abs(interpolate_keyframes(kfs_lin, 0.5, 0.0) - 25.0) < 1e-4

    # Hold interpolation
    kfs_hold = [
        Keyframe(time=0.0, value=10.0, interpolation="hold"),
        Keyframe(time=2.0, value=20.0, interpolation="hold"),
    ]
    assert interpolate_keyframes(kfs_hold, 1.0, 0.0) == 10.0
    assert interpolate_keyframes(kfs_hold, 1.99, 0.0) == 10.0
    assert interpolate_keyframes(kfs_hold, 2.0, 0.0) == 20.0

    # Ease-in-out interpolation (smoothstep: 3t^2 - 2t^3)
    kfs_ease = [
        Keyframe(time=0.0, value=0.0, interpolation="ease_in_out"),
        Keyframe(time=1.0, value=100.0, interpolation="ease_in_out"),
    ]
    # At t=0.5, smoothstep is 3*(0.25) - 2*(0.125) = 0.75 - 0.25 = 0.5 -> value 50.0
    assert abs(interpolate_keyframes(kfs_ease, 0.5, 0.0) - 50.0) < 1e-4
    # At t=0.2, linear would be 20.0, ease-in should be lower (slower start)
    # smoothstep(0.2) = 3*(0.04) - 2*(0.008) = 0.12 - 0.016 = 0.104 -> value 10.4
    val_at_02 = interpolate_keyframes(kfs_ease, 0.2, 0.0)
    assert abs(val_at_02 - 10.4) < 1e-3


def test_clip_keyframe_management():
    clip = Clip(media_id="m1", timeline_in=0.0, timeline_out=4.0, opacity=1.0, scale=1.0)

    assert not clip.has_keyframes()
    assert clip.get_property_at_time("opacity", 1.0) == 1.0

    # Add keyframes for opacity: fade down then up
    clip.add_keyframe("opacity", 0.0, 1.0)
    clip.add_keyframe("opacity", 2.0, 0.2)
    clip.add_keyframe("opacity", 4.0, 1.0)

    assert clip.has_keyframes("opacity")
    assert not clip.has_keyframes("scale")
    assert clip.has_keyframes()

    # Interpolated values
    assert abs(clip.get_property_at_time("opacity", 1.0) - 0.6) < 1e-3
    assert abs(clip.get_property_at_time("opacity", 2.0) - 0.2) < 1e-3

    # Serialization and cloning
    cloned = clip.clone()
    assert cloned.has_keyframes("opacity")
    assert len(cloned.keyframes["opacity"]) == 3

    # Independent copy
    clip.remove_keyframe("opacity", 2.0)
    assert len(clip.keyframes["opacity"]) == 2
    assert len(cloned.keyframes["opacity"]) == 3

    d = clip.to_dict()
    restored = Clip.from_dict(d)
    assert len(restored.keyframes["opacity"]) == 2


# =====================================================================
# 3. Clip Playback Speed & Reverse Tests
# =====================================================================

def test_clip_speed_and_reverse_source_mapping():
    # Clip: timeline [2.0, 6.0] (dur = 4.0), source_in = 1.0
    c_fwd = Clip(media_id="m1", timeline_in=2.0, timeline_out=6.0, source_in=1.0, speed=1.0, reverse=False)
    # Normal forward
    assert c_fwd.map_timeline_to_source(2.0) == 1.0
    assert c_fwd.map_timeline_to_source(4.0) == 3.0
    assert c_fwd.map_timeline_to_source(6.0) == 5.0

    # Fast forward (2.0x speed)
    c_fast = Clip(media_id="m1", timeline_in=0.0, timeline_out=2.0, source_in=0.0, speed=2.0, reverse=False)
    # Timeline 0s -> source 0s, timeline 1s -> source 2s, timeline 2s -> source 4s
    assert c_fast.map_timeline_to_source(0.0) == 0.0
    assert c_fast.map_timeline_to_source(1.0) == 2.0
    assert c_fast.map_timeline_to_source(2.0) == 4.0

    # Slow motion (0.5x speed)
    c_slow = Clip(media_id="m1", timeline_in=0.0, timeline_out=4.0, source_in=2.0, speed=0.5, reverse=False)
    # Timeline 0s -> source 2s, timeline 2s -> source 3s, timeline 4s -> source 4s
    assert c_slow.map_timeline_to_source(0.0) == 2.0
    assert c_slow.map_timeline_to_source(2.0) == 3.0
    assert c_slow.map_timeline_to_source(4.0) == 4.0

    # Reverse normal (1.0x)
    # timeline [0.0, 3.0], source_in = 5.0, source_out = 8.0
    c_rev = Clip(media_id="m1", timeline_in=0.0, timeline_out=3.0, source_in=5.0, source_out=8.0, speed=1.0, reverse=True)
    assert c_rev.map_timeline_to_source(0.0) == 8.0
    assert c_rev.map_timeline_to_source(1.0) == 7.0
    assert c_rev.map_timeline_to_source(3.0) == 5.0

    # Reverse fast (2.0x)
    # timeline [0.0, 2.0], source_in = 0.0, source_out = 4.0
    c_rev_fast = Clip(media_id="m1", timeline_in=0.0, timeline_out=2.0, source_in=0.0, source_out=4.0, speed=2.0, reverse=True)
    assert c_rev_fast.map_timeline_to_source(0.0) == 4.0
    assert c_rev_fast.map_timeline_to_source(1.0) == 2.0
    assert c_rev_fast.map_timeline_to_source(2.0) == 0.0


def test_change_clip_speed_command():
    tl = TimelineModel()
    tr = Track(name="V1", track_type=TrackType.VIDEO)
    c = Clip(media_id="m1", timeline_in=2.0, timeline_out=6.0, source_in=0.0, source_out=4.0, speed=1.0)
    tr.add_clip(c)
    tl.add_track(tr)

    stack = QUndoStack()
    # Change speed to 2.0x (duration cuts from 4.0 to 2.0)
    cmd = ChangeClipSpeedCommand(tl, tr.id, c.id, new_speed=2.0, new_reverse=True)
    stack.push(cmd)

    assert c.speed == 2.0
    assert c.reverse is True
    assert abs(c.duration - 2.0) < 1e-3
    assert c.timeline_out == 4.0

    # Undo
    stack.undo()
    assert c.speed == 1.0
    assert c.reverse is False
    assert abs(c.duration - 4.0) < 1e-3
    assert c.timeline_out == 6.0

    # Redo
    stack.redo()
    assert c.speed == 2.0
    assert c.reverse is True
    assert abs(c.duration - 2.0) < 1e-3


def test_atempo_chained_filters():
    assert TimelineExporter.get_atempo_filters(1.0) == []
    assert TimelineExporter.get_atempo_filters(2.0) == ["atempo=2.0000"]
    assert TimelineExporter.get_atempo_filters(4.0) == ["atempo=2.0", "atempo=2.0000"]
    assert TimelineExporter.get_atempo_filters(0.5) == ["atempo=0.5000"]
    assert TimelineExporter.get_atempo_filters(0.25) == ["atempo=0.5", "atempo=0.5000"]


def test_export_pipeline_with_speed_and_reverse():
    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)
        clip_path = tmp_path / "test_av.mp4"
        out_path = tmp_path / "out_fast_reverse.mp4"

        create_synthetic_av_clip(clip_path, duration=2.0)

        proj = Project()
        m = FFprobeAnalyzer.analyze(clip_path)
        proj.add_media(m)

        tr = proj.timeline.get_video_tracks()[0]
        # 1.0s clip on timeline played at 2.0x speed (takes 2.0s of source) and reversed
        c = Clip(
            media_id=m.id,
            timeline_in=0.0,
            timeline_out=1.0,
            source_in=0.0,
            source_out=2.0,
            speed=2.0,
            reverse=True,
        )
        tr.add_clip(c)

        preset = DEFAULT_PRESETS[1]
        cmd, total_dur = TimelineExporter.build_export_pipeline(proj, preset, str(out_path))

        assert total_dur == 1.0
        cmd_str = " ".join(cmd)
        assert "reverse" in cmd_str
        assert "setpts=" in cmd_str
        assert "areverse" in cmd_str
        assert "atempo=2.0" in cmd_str

        # Run FFmpeg command and check result
        res = subprocess.run(cmd, capture_output=True, text=True)
        assert res.returncode == 0, f"FFmpeg export failed: {res.stderr}"

        assert out_path.is_file()
        exported_meta = FFprobeAnalyzer.analyze(out_path)
        assert abs(exported_meta.duration - 1.0) < 0.15
        assert exported_meta.video_codec == "h264"
        assert exported_meta.audio_codec == "aac"


# =====================================================================
# 4. UI Component Integration Tests
# =====================================================================

def test_speed_dialog_ui(qapp):
    dlg = SpeedDialog(current_speed=1.0, reverse=False, current_duration=4.0)
    assert dlg.spn_speed.value() == 1.0
    assert not dlg.chk_reverse.isChecked()
    assert "4.00 s" in dlg.lbl_new_duration.text()

    # Change to 2.0x
    dlg.spn_speed.setValue(2.0)
    assert "2.00 s" in dlg.lbl_new_duration.text()

    dlg.chk_reverse.setChecked(True)
    speed, rev = dlg.get_values()
    assert speed == 2.0
    assert rev is True


def test_main_window_marker_shortcut(qapp):
    proj = Project()
    win = MainWindow(proj)

    # Initial timeline has no markers
    assert len(proj.timeline.markers) == 0

    # Trigger marker shortcut action
    win.sc_m.activated.emit()

    assert len(proj.timeline.markers) == 1
    assert proj.timeline.markers[0].time == 0.0

    # Move playhead to 3.5s and trigger again
    win.playback_engine.seek(3.5)
    win.sc_m.activated.emit()
    assert len(proj.timeline.markers) == 2
    assert abs(proj.timeline.markers[1].time - 3.5) < 0.01
