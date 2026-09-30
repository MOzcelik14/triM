from __future__ import annotations

from typing import Optional

from PySide6.QtCore import QRect, QRectF, QSize, Qt
from PySide6.QtGui import QColor, QFont, QImage, QPainter
from PySide6.QtWidgets import QWidget


class MonitorWidget(QWidget):
    """Program Monitor widget rendering frame-accurate preview with letterboxing."""

    def __init__(self, parent: Optional[QWidget] = None) -> None:
        super().__init__(parent)
        self._current_frame: Optional[QImage] = None
        self._current_time: float = 0.0
        self.setMinimumSize(320, 180)
        self.setStyleSheet("background-color: #121215;")

    def set_frame(self, frame: QImage, time: float) -> None:
        self._current_frame = frame
        self._current_time = time
        self.update()

    def paintEvent(self, event) -> None:
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.SmoothPixmapTransform, True)

        # Clear background
        painter.fillRect(self.rect(), QColor(14, 14, 16))

        if self._current_frame is None or self._current_frame.isNull():
            # Draw placeholder when empty
            painter.setPen(QColor(70, 70, 80))
            font = QFont()
            font.setPointSize(12)
            painter.setFont(font)
            painter.drawText(
                self.rect(),
                Qt.AlignmentFlag.AlignCenter,
                "No Media Active\nImport media or move playhead over clips",
            )
            return

        img_w = self._current_frame.width()
        img_h = self._current_frame.height()
        if img_w <= 0 or img_h <= 0:
            return

        widget_w = self.width()
        widget_h = self.height()

        # Compute aspect ratio fit
        scale = min(widget_w / img_w, widget_h / img_h)
        target_w = int(img_w * scale)
        target_h = int(img_h * scale)

        target_x = (widget_w - target_w) // 2
        target_y = (widget_h - target_h) // 2
        dest_rect = QRect(target_x, target_y, target_w, target_h)

        painter.drawImage(dest_rect, self._current_frame)
