import json
from pathlib import Path
import pytest

import trim
import cutline
from trim.app.application import TrimApplication, CutlineApplication
from trim.core.clip import Clip
from trim.core.project import Project
from trim.core.track import Track, TrackType
from trim.ui.main_window import MainWindow


def test_import_compatibility():
    assert cutline.core.project.Project is trim.core.project.Project
    assert CutlineApplication is TrimApplication


def test_application_metadata(qapp):
    app = qapp
    assert app.applicationName() == "triM."
    assert app.applicationDisplayName() == "triM."
    assert app.desktopFileName() in ("io.github.mozcelik14.triM", "io.github.mozcelik14.triM.desktop")


def test_window_title_and_branding(qapp):
    project = Project()
    win = MainWindow(project)
    assert "triM." in win.windowTitle()
    # Check empty state overlay is visible (not hidden) when no clips on timeline
    assert not win.monitor._empty_overlay.isHidden()

    # Adding a clip should hide the empty state
    track = project.timeline.get_video_tracks()[0]
    clip = Clip(media_id="test", timeline_in=0.0, timeline_out=3.0)
    track.add_clip(clip)
    win._update_empty_state()
    assert win.monitor._empty_overlay.isHidden()

    win.close()


def test_project_format_and_backwards_compatibility(tmp_path):
    project = Project()
    track = project.timeline.get_video_tracks()[0]
    clip = Clip(media_id="m1", timeline_in=0.0, timeline_out=5.0)
    track.add_clip(clip)

    # 1. Save as new .trim file
    trim_file = tmp_path / "new_project.trim"
    project.save(str(trim_file))
    assert trim_file.is_file()

    with open(trim_file, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["format"] == "trim"

    # Reload .trim
    loaded_trim = Project.load(str(trim_file))
    v_tracks = loaded_trim.timeline.get_video_tracks()
    assert len(v_tracks) >= 1
    assert len(v_tracks[0].clips) == 1
    assert v_tracks[0].clips[0].duration == 5.0

    # 2. Simulate legacy .cutline project file
    legacy_file = tmp_path / "legacy_project.cutline"
    data["format"] = "cutline"
    with open(legacy_file, "w", encoding="utf-8") as f:
        json.dump(data, f)

    # Reload legacy .cutline
    loaded_legacy = Project.load(str(legacy_file))
    v_leg_tracks = loaded_legacy.timeline.get_video_tracks()
    assert len(v_leg_tracks) >= 1
    assert len(v_leg_tracks[0].clips) == 1
    assert v_leg_tracks[0].clips[0].duration == 5.0


def test_brand_assets_exist():
    base_dir = Path(__file__).resolve().parent.parent
    branding_dir = base_dir / "trim" / "resources" / "branding"
    icons_dir = base_dir / "trim" / "resources" / "icons"

    assert (branding_dir / "logo_wordmark.svg").is_file()
    assert (branding_dir / "app_icon.svg").is_file()
    assert (icons_dir / "trim.png").is_file()
    assert (icons_dir / "trim_16x16.png").is_file()
    assert (icons_dir / "trim_32x32.png").is_file()
    assert (icons_dir / "trim_48x48.png").is_file()
    assert (icons_dir / "trim_64x64.png").is_file()
    assert (icons_dir / "trim_128x128.png").is_file()
    assert (icons_dir / "trim_256x256.png").is_file()
    assert (icons_dir / "trim_512x512.png").is_file()

    assert (base_dir / "io.github.mozcelik14.triM.desktop").is_file()
    assert (base_dir / "io.github.mozcelik14.triM.metainfo.xml").is_file()
    assert (base_dir / "io.github.mozcelik14.triM.svg").is_file()
    assert (base_dir / "io.github.mozcelik14.triM.png").is_file()
