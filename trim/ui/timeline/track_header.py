from __future__ import annotations

from typing import Optional

from PySide6.QtCore import Qt, Signal
from PySide6.QtWidgets import (
    QHBoxLayout,
    QLabel,
    QPushButton,
    QVBoxLayout,
    QWidget,
)

from trim.core.timeline import TimelineModel
from trim.core.track import Track, TrackType


class SingleTrackHeader(QWidget):
    """Header for an individual track with Turkish tooltips."""

    def __init__(self, track: Track, parent: Optional[QWidget] = None) -> None:
        super().__init__(parent)
        self.track = track
        self.setFixedHeight(60)
        self.setStyleSheet(
            "background-color: #24242c; border-bottom: 1px solid #32323c; border-right: 1px solid #32323c;"
        )

        layout = QHBoxLayout(self)
        layout.setContentsMargins(8, 4, 8, 4)
        layout.setSpacing(4)

        # Track name
        type_prefix = "V" if track.track_type == TrackType.VIDEO else "A"
        self.lbl_name = QLabel(f"{type_prefix}: {track.name}")
        self.lbl_name.setStyleSheet("font-weight: bold; color: #d0d0e0;")
        layout.addWidget(self.lbl_name)
        layout.addStretch()

        # Visibility / Mute button
        self.btn_vis = QPushButton("👁" if track.visible else "🚫")
        self.btn_vis.setFixedSize(26, 24)
        self.btn_vis.setToolTip("Görünürlüğü Aç / Kapat")
        self.btn_vis.clicked.connect(self._toggle_vis)
        layout.addWidget(self.btn_vis)

        # Lock button
        self.btn_lock = QPushButton("🔓" if not track.locked else "🔒")
        self.btn_lock.setFixedSize(26, 24)
        self.btn_lock.setToolTip("Kanalı Kilitle / Aç")
        self.btn_lock.clicked.connect(self._toggle_lock)
        layout.addWidget(self.btn_lock)

    def _toggle_vis(self) -> None:
        self.track.visible = not self.track.visible
        self.btn_vis.setText("👁" if self.track.visible else "🚫")

    def _toggle_lock(self) -> None:
        self.track.locked = not self.track.locked
        self.btn_lock.setText("🔓" if not self.track.locked else "🔒")


class TrackHeadersWidget(QWidget):
    """Container holding vertical track headers synchronized with canvas tracks."""

    def __init__(self, timeline: TimelineModel, parent: Optional[QWidget] = None) -> None:
        super().__init__(parent)
        self.timeline = timeline
        self.setFixedWidth(140)

        self._layout = QVBoxLayout(self)
        self._layout.setContentsMargins(0, 38, 0, 0)
        self._layout.setSpacing(6)
        self._layout.setAlignment(Qt.AlignmentFlag.AlignTop)

        self.timeline.tracks_changed.connect(self.refresh)
        self.refresh()

    def refresh(self) -> None:
        while self._layout.count():
            item = self._layout.takeAt(0)
            if item.widget():
                item.widget().deleteLater()

        for track in self.timeline.tracks:
            header = SingleTrackHeader(track)
            self._layout.addWidget(header)
