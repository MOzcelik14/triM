from __future__ import annotations

from typing import Optional

from PySide6.QtCore import QPointF, QRect, Qt, Signal
from PySide6.QtGui import QBrush, QColor, QFont, QMouseEvent, QPainter, QPolygonF
from PySide6.QtWidgets import QWidget

from trim.core.timeline import TimelineModel


class TimeRuler(QWidget):
    """Timeline time ruler with tick marks, timecode headers, and scrubbable playhead handle."""

    seek_requested = Signal(float)

    def __init__(
        self,
        timeline: TimelineModel,
        pixels_per_second: float = 60.0,
        parent: Optional[QWidget] = None,
    ) -> None:
        super().__init__(parent)
        self.timeline = timeline
        self.pixels_per_second = pixels_per_second
        self._current_time: float = 0.0
        self._is_scrubbing: bool = False
        self.setFixedHeight(28)
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setMouseTracking(True)
        self.timeline.markers_changed.connect(self.update)

    def set_pixels_per_second(self, pps: float) -> None:
        self.pixels_per_second = max(5.0, min(1000.0, pps))
        self.update()

    def set_current_time(self, time: float) -> None:
        self._current_time = max(0.0, time)
        self.update()

    def mousePressEvent(self, event: QMouseEvent) -> None:
        if event.button() == Qt.MouseButton.LeftButton:
            self._is_scrubbing = True
            self._seek_from_x(event.position().x())

    def mouseMoveEvent(self, event: QMouseEvent) -> None:
        if self._is_scrubbing:
            self._seek_from_x(event.position().x())
        else:
            x = event.position().x()
            hovered_marker = None
            for m in self.timeline.markers:
                if abs(m.time * self.pixels_per_second - x) <= 6:
                    hovered_marker = m
                    break
            if hovered_marker:
                tc = self.timeline.time_to_timecode(hovered_marker.time)
                name = hovered_marker.name or "İşaretçi"
                self.setToolTip(f"{name} [{tc}]")
            else:
                self.setToolTip("")

    def mouseReleaseEvent(self, event: QMouseEvent) -> None:
        if event.button() == Qt.MouseButton.LeftButton:
            self._is_scrubbing = False

    def _seek_from_x(self, x: float) -> None:
        t = max(0.0, x / self.pixels_per_second)
        # Snap time
        snapped = self.timeline.snap_time(t, threshold=0.1)
        self.seek_requested.emit(snapped)

    def paintEvent(self, event) -> None:
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing, True)

        # Background
        painter.fillRect(self.rect(), QColor(26, 26, 32))
        painter.setPen(QColor(42, 42, 50))
        painter.drawLine(0, self.height() - 1, self.width(), self.height() - 1)

        # Determine interval step based on zoom
        pps = self.pixels_per_second
        if pps >= 200:
            step_sec = 0.5
            minor_step = 0.1
        elif pps >= 80:
            step_sec = 1.0
            minor_step = 0.2
        elif pps >= 30:
            step_sec = 5.0
            minor_step = 1.0
        elif pps >= 10:
            step_sec = 10.0
            minor_step = 2.0
        else:
            step_sec = 30.0
            minor_step = 5.0

        max_sec = max(self.timeline.duration + 10.0, self.width() / pps)
        font = QFont()
        font.setPointSize(9)
        painter.setFont(font)

        # Draw minor ticks
        painter.setPen(QColor(60, 60, 70))
        cur_t = 0.0
        while cur_t <= max_sec:
            x = int(cur_t * pps)
            painter.drawLine(x, self.height() - 6, x, self.height() - 1)
            cur_t += minor_step

        # Draw major ticks and labels
        cur_t = 0.0
        while cur_t <= max_sec:
            x = int(cur_t * pps)
            painter.setPen(QColor(100, 100, 115))
            painter.drawLine(x, self.height() - 12, x, self.height() - 1)

            # Draw time label
            tc_str = self.timeline.time_to_timecode(cur_t)
            # Short format MM:SS:FF
            parts = tc_str.split(":")
            short_tc = f"{parts[1]}:{parts[2]}"
            painter.setPen(QColor(160, 160, 175))
            painter.drawText(x + 4, 14, short_tc)

            cur_t += step_sec

        # Draw Markers
        for marker in self.timeline.markers:
            mx = marker.time * pps
            marker_color = QColor(marker.color)
            m_poly = QPolygonF([
                QPointF(mx, 4),
                QPointF(mx + 5, 11),
                QPointF(mx, 18),
                QPointF(mx - 5, 11),
            ])
            painter.setBrush(QBrush(marker_color))
            painter.setPen(QColor(20, 20, 24))
            painter.drawPolygon(m_poly)

        # Draw Playhead triangle
        ph_x = self._current_time * pps
        poly = QPolygonF([
            QPointF(ph_x - 6, 0),
            QPointF(ph_x + 6, 0),
            QPointF(ph_x + 6, 12),
            QPointF(ph_x, 20),
            QPointF(ph_x - 6, 12),
        ])
        painter.setBrush(QBrush(QColor(224, 122, 56)))  # Precision Amber
        painter.setPen(Qt.PenStyle.NoPen)
        painter.drawPolygon(poly)
