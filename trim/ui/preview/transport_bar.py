from __future__ import annotations

from typing import Optional

from PySide6.QtCore import Qt, Signal
from PySide6.QtWidgets import (
    QHBoxLayout,
    QLabel,
    QPushButton,
    QWidget,
)

from trim.core.timeline import TimelineModel
from trim.media.playback import PlaybackEngine


class TransportBar(QWidget):
    """Transport control bar for play/pause, seek, step frame, and timecode."""

    play_requested = Signal()
    pause_requested = Signal()
    stop_requested = Signal()
    prev_frame_requested = Signal()
    next_frame_requested = Signal()

    def __init__(
        self,
        playback_engine: PlaybackEngine,
        timeline: TimelineModel,
        parent: Optional[QWidget] = None,
    ) -> None:
        super().__init__(parent)
        self.playback_engine = playback_engine
        self.timeline = timeline

        layout = QHBoxLayout(self)
        layout.setContentsMargins(6, 4, 6, 4)
        layout.setSpacing(6)

        # Timecode display
        self.lbl_timecode = QLabel("00:00:00:00")
        self.lbl_timecode.setStyleSheet(
            "font-family: monospace; font-size: 14px; font-weight: bold; "
            "color: #00d2ff; background-color: #121216; padding: 3px 8px; border-radius: 3px;"
        )
        layout.addWidget(self.lbl_timecode)

        self.lbl_duration = QLabel("/ 00:00:00:00")
        self.lbl_duration.setStyleSheet("font-family: monospace; color: #888892;")
        layout.addWidget(self.lbl_duration)

        layout.addStretch()

        # Step back
        self.btn_prev = QPushButton("⏮")
        self.btn_prev.setToolTip("Önceki Kare (Sol Ok)")
        self.btn_prev.setFixedWidth(36)
        self.btn_prev.clicked.connect(self.prev_frame_requested.emit)
        layout.addWidget(self.btn_prev)

        # Stop
        self.btn_stop = QPushButton("⏹")
        self.btn_stop.setToolTip("Durdur")
        self.btn_stop.setFixedWidth(36)
        self.btn_stop.clicked.connect(self.stop_requested.emit)
        layout.addWidget(self.btn_stop)

        # Play / Pause toggle
        self.btn_play = QPushButton("▶")
        self.btn_play.setObjectName("PrimaryButton")
        self.btn_play.setToolTip("Oynat / Duraklat (Boşluk)")
        self.btn_play.setFixedWidth(44)
        self.btn_play.clicked.connect(self._toggle_play)
        layout.addWidget(self.btn_play)

        # Step forward
        self.btn_next = QPushButton("⏭")
        self.btn_next.setToolTip("Sonraki Kare (Sağ Ok)")
        self.btn_next.setFixedWidth(36)
        self.btn_next.clicked.connect(self.next_frame_requested.emit)
        layout.addWidget(self.btn_next)

        layout.addStretch()

        # Connect playback signals
        self.playback_engine.position_changed.connect(self._on_position_changed)
        self.playback_engine.state_changed.connect(self._on_state_changed)
        self.timeline.duration_changed.connect(self._on_duration_changed)

        self._on_duration_changed(self.timeline.duration)

    def _toggle_play(self) -> None:
        self.playback_engine.toggle_play()

    def _on_position_changed(self, time_sec: float) -> None:
        self.lbl_timecode.setText(self.timeline.time_to_timecode(time_sec))

    def _on_state_changed(self, is_playing: bool) -> None:
        if is_playing:
            self.btn_play.setText("⏸")
        else:
            self.btn_play.setText("▶")

    def _on_duration_changed(self, dur_sec: float) -> None:
        self.lbl_duration.setText(f"/ {self.timeline.time_to_timecode(dur_sec)}")
