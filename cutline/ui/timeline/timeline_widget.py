from __future__ import annotations

from typing import Optional

from PySide6.QtCore import Qt, Signal
from PySide6.QtGui import QUndoStack
from PySide6.QtWidgets import (
    QHBoxLayout,
    QLabel,
    QPushButton,
    QScrollArea,
    QSlider,
    QVBoxLayout,
    QWidget,
)

from cutline.core.project import Project
from cutline.core.track import Track, TrackType
from cutline.ui.timeline.time_ruler import TimeRuler
from cutline.ui.timeline.timeline_canvas import TimelineCanvas
from cutline.ui.timeline.track_header import TrackHeadersWidget


class TimelineWidget(QWidget):
    """Main timeline panel assembling ruler, canvas, headers, tools, and zoom controls."""

    clip_selected = Signal(str, str)  # (track_id, clip_id)
    seek_requested = Signal(float)  # (timeline_time)

    def __init__(
        self,
        project: Project,
        undo_stack: QUndoStack,
        parent: Optional[QWidget] = None,
    ) -> None:
        super().__init__(parent)
        self.project = project
        self.undo_stack = undo_stack
        self.pixels_per_second = 60.0

        main_layout = QVBoxLayout(self)
        main_layout.setContentsMargins(4, 4, 4, 4)
        main_layout.setSpacing(4)

        # 1. Timeline Tools Toolbar
        tools_layout = QHBoxLayout()
        tools_layout.setSpacing(6)

        self.btn_split = QPushButton("✂ Cut / Split (S)")
        self.btn_split.setToolTip("Split clip at playhead (S)")
        self.btn_split.clicked.connect(self._on_split_clicked)
        tools_layout.addWidget(self.btn_split)

        self.btn_delete = QPushButton("🗑 Delete (Del)")
        self.btn_delete.setToolTip("Delete selected clip (Delete/Backspace)")
        self.btn_delete.clicked.connect(self._on_delete_clicked)
        tools_layout.addWidget(self.btn_delete)

        self.btn_ripple = QPushButton("⏪ Ripple Delete")
        self.btn_ripple.setToolTip("Delete selected clip and shift subsequent clips left")
        self.btn_ripple.clicked.connect(self._on_ripple_clicked)
        tools_layout.addWidget(self.btn_ripple)

        tools_layout.addStretch()

        # Add Track Buttons
        self.btn_add_v = QPushButton("+ Video Track")
        self.btn_add_v.clicked.connect(self._add_video_track)
        tools_layout.addWidget(self.btn_add_v)

        self.btn_add_a = QPushButton("+ Audio Track")
        self.btn_add_a.clicked.connect(self._add_audio_track)
        tools_layout.addWidget(self.btn_add_a)

        tools_layout.addSpacing(16)

        # Zoom Controls
        tools_layout.addWidget(QLabel("Zoom:"))
        self.btn_zoom_out = QPushButton("−")
        self.btn_zoom_out.setFixedSize(26, 24)
        self.btn_zoom_out.clicked.connect(lambda: self.zoom_slider.setValue(self.zoom_slider.value() - 10))
        tools_layout.addWidget(self.btn_zoom_out)

        self.zoom_slider = QSlider(Qt.Orientation.Horizontal)
        self.zoom_slider.setRange(10, 300)
        self.zoom_slider.setValue(int(self.pixels_per_second))
        self.zoom_slider.setFixedWidth(120)
        self.zoom_slider.valueChanged.connect(self._on_zoom_changed)
        tools_layout.addWidget(self.zoom_slider)

        self.btn_zoom_in = QPushButton("+")
        self.btn_zoom_in.setFixedSize(26, 24)
        self.btn_zoom_in.clicked.connect(lambda: self.zoom_slider.setValue(self.zoom_slider.value() + 10))
        tools_layout.addWidget(self.btn_zoom_in)

        main_layout.addLayout(tools_layout)

        # 2. Scrollable Body
        body_layout = QHBoxLayout()
        body_layout.setSpacing(0)
        body_layout.setContentsMargins(0, 0, 0, 0)

        # Left: Track Headers
        self.track_headers = TrackHeadersWidget(self.project.timeline)
        body_layout.addWidget(self.track_headers)

        # Right: Scroll Area with Ruler and Canvas
        self.scroll_area = QScrollArea()
        self.scroll_area.setWidgetResizable(True)
        self.scroll_area.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAsNeeded)
        self.scroll_area.setVerticalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAsNeeded)

        container = QWidget()
        container_layout = QVBoxLayout(container)
        container_layout.setContentsMargins(0, 0, 0, 0)
        container_layout.setSpacing(0)

        self.time_ruler = TimeRuler(self.project.timeline, pixels_per_second=self.pixels_per_second)
        self.time_ruler.seek_requested.connect(self.seek_requested)
        container_layout.addWidget(self.time_ruler)

        self.canvas = TimelineCanvas(
            self.project,
            self.undo_stack,
            pixels_per_second=self.pixels_per_second,
        )
        self.canvas.clip_selected.connect(self.clip_selected)
        self.canvas.seek_requested.connect(self.seek_requested)
        container_layout.addWidget(self.canvas)

        self.scroll_area.setWidget(container)
        body_layout.addWidget(self.scroll_area)

        main_layout.addLayout(body_layout)

    def set_current_time(self, time: float) -> None:
        self.time_ruler.set_current_time(time)
        self.canvas.set_current_time(time)

    def _on_zoom_changed(self, val: int) -> None:
        self.pixels_per_second = float(val)
        self.time_ruler.set_pixels_per_second(self.pixels_per_second)
        self.canvas.set_pixels_per_second(self.pixels_per_second)

    def _on_split_clicked(self) -> None:
        self.canvas.split_at_playhead()

    def _on_delete_clicked(self) -> None:
        self.canvas.delete_selected()

    def _on_ripple_clicked(self) -> None:
        self.canvas.ripple_delete_selected()

    def _add_video_track(self) -> None:
        count = len(self.project.timeline.get_video_tracks()) + 1
        self.project.timeline.add_track(Track(name=f"Video {count}", track_type=TrackType.VIDEO))
        self.project.mark_dirty()

    def _add_audio_track(self) -> None:
        count = len(self.project.timeline.get_audio_tracks()) + 1
        self.project.timeline.add_track(Track(name=f"Audio {count}", track_type=TrackType.AUDIO))
        self.project.mark_dirty()
