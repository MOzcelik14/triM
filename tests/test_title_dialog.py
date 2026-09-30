import os
from pathlib import Path
import pytest

os.environ["QT_QPA_PLATFORM"] = "offscreen"

from trim.app.application import CutlineApplication
from trim.core.project import Project
from trim.ui.dialogs.title_dialog import TitleDialog


def test_title_dialog_creation_and_rendering(tmp_path: Path):
    app = CutlineApplication.instance() or CutlineApplication([])
    project = Project()

    dlg = TitleDialog(project, playhead_time=2.5)
    dlg.txt_content.setPlainText("Merhaba Cutline!")
    dlg.spn_duration.setValue(3.5)

    media_item, add_to_timeline = dlg.create_title_media()

    assert media_item is not None
    assert media_item.duration == 3.5
    assert "Merhaba Cutline!" in media_item.name
    assert Path(media_item.file_path).is_file()
    assert Path(media_item.file_path).stat().st_size > 0
    assert add_to_timeline is True
    assert media_item.has_audio is False
    assert media_item.has_video is True


def test_title_clip_timeline_canvas_paint():
    from trim.core.clip import Clip
    from trim.ui.timeline.timeline_canvas import TimelineCanvas
    from PySide6.QtGui import QImage, QUndoStack

    app = CutlineApplication.instance() or CutlineApplication([])
    project = Project()
    undo_stack = QUndoStack()

    dlg = TitleDialog(project)
    dlg.txt_content.setPlainText("Test Başlık")
    media_item, _ = dlg.create_title_media()
    project.add_media(media_item)

    track = project.timeline.get_video_tracks()[0]
    clip = Clip(media_id=media_item.id, timeline_in=0.0, timeline_out=4.0, name=media_item.name)
    track.add_clip(clip)

    canvas = TimelineCanvas(project, undo_stack)
    canvas.resize(800, 300)
    img = QImage(800, 300, QImage.Format.Format_RGB32)
    canvas.render(img)
    assert not img.isNull()
