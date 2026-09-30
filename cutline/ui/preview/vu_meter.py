from __future__ import annotations

import math
from typing import Optional

from PySide6.QtCore import QPointF, QRectF, QTimer, Qt
from PySide6.QtGui import QBrush, QColor, QFont, QLinearGradient, QPainter, QPen
from PySide6.QtWidgets import QWidget


class AudioVUMeterWidget(QWidget):
    """Sleek vertical dual-channel (L/R) audio VU meter with smooth decay ballistics."""

    def __init__(self, parent: Optional[QWidget] = None) -> None:
        super().__init__(parent)
        self.setFixedWidth(46)
        self.setMinimumHeight(160)

        self._min_db = -60.0
        self._max_db = 0.0

        # Current display levels (in dB)
        self._current_l = self._min_db
        self._current_r = self._min_db

        # Target levels from playback engine
        self._target_l = self._min_db
        self._target_r = self._min_db

        # Peak hold indicators
        self._peak_l = self._min_db
        self._peak_r = self._min_db
        self._peak_hold_ticks_l = 0
        self._peak_hold_ticks_r = 0

        # Ballistics decay timer (60 FPS)
        self._decay_timer = QTimer(self)
        self._decay_timer.setInterval(16)
        self._decay_timer.timeout.connect(self._on_decay_tick)
        self._decay_timer.start()

    def set_levels(self, left_db: float, right_db: float) -> None:
        """Sets target audio levels in dBFS [-60.0, 0.0]."""
        self._target_l = max(self._min_db, min(self._max_db, left_db))
        self._target_r = max(self._min_db, min(self._max_db, right_db))

        # Update peaks immediately if higher
        if self._target_l > self._peak_l:
            self._peak_l = self._target_l
            self._peak_hold_ticks_l = 30
        if self._target_r > self._peak_r:
            self._peak_r = self._target_r
            self._peak_hold_ticks_r = 30

    def reset(self) -> None:
        self._target_l = self._min_db
        self._target_r = self._min_db
        self._current_l = self._min_db
        self._current_r = self._min_db
        self._peak_l = self._min_db
        self._peak_r = self._min_db
        self.update()

    def _on_decay_tick(self) -> None:
        decay_step = 1.2  # dB per tick decay

        # Attack fast, decay smooth
        if self._target_l > self._current_l:
            self._current_l = self._target_l
        else:
            self._current_l = max(self._min_db, self._current_l - decay_step)

        if self._target_r > self._current_r:
            self._current_r = self._target_r
        else:
            self._current_r = max(self._min_db, self._current_r - decay_step)

        # Peak hold ballistics
        if self._peak_hold_ticks_l > 0:
            self._peak_hold_ticks_l -= 1
        else:
            self._peak_l = max(self._min_db, self._peak_l - (decay_step * 0.5))

        if self._peak_hold_ticks_r > 0:
            self._peak_hold_ticks_r -= 1
        else:
            self._peak_r = max(self._min_db, self._peak_r - (decay_step * 0.5))

        self.update()

    def _db_to_y(self, db: float, top: float, height: float) -> float:
        # Logarithmic-like scaling between -60 dB and 0 dB
        # Map: -60 -> bottom, 0 -> top
        fraction = (db - self._min_db) / (self._max_db - self._min_db)
        fraction = max(0.0, min(1.0, fraction))
        return top + height * (1.0 - fraction)

    def paintEvent(self, event) -> None:
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing, False)

        # Background
        painter.fillRect(self.rect(), QColor(18, 18, 22))

        # Border
        painter.setPen(QColor(35, 35, 42))
        painter.drawRect(self.rect().adjusted(0, 0, -1, -1))

        margin_top = 18.0
        margin_bottom = 16.0
        meter_h = self.height() - margin_top - margin_bottom
        if meter_h <= 10:
            return

        bar_w = 7.0
        spacing = 3.0
        x_left = 6.0
        x_right = x_left + bar_w + spacing

        # Background troughs for L and R
        trough_rect_l = QRectF(x_left, margin_top, bar_w, meter_h)
        trough_rect_r = QRectF(x_right, margin_top, bar_w, meter_h)
        painter.fillRect(trough_rect_l, QColor(26, 26, 32))
        painter.fillRect(trough_rect_r, QColor(26, 26, 32))

        # Gradient setup for meters (Green -> Yellow -> Red)
        grad = QLinearGradient(0, margin_top + meter_h, 0, margin_top)
        grad.setColorAt(0.0, QColor(46, 204, 113))   # -60 dB: Green
        grad.setColorAt(0.75, QColor(46, 204, 113))  # -15 dB: Green
        grad.setColorAt(0.85, QColor(241, 196, 15))  # -9 dB: Yellow
        grad.setColorAt(0.95, QColor(230, 126, 34))  # -3 dB: Orange
        grad.setColorAt(1.0, QColor(231, 76, 60))    # 0 dB: Red
        meter_brush = QBrush(grad)

        # Draw Left Bar
        y_l = self._db_to_y(self._current_l, margin_top, meter_h)
        bar_rect_l = QRectF(x_left, y_l, bar_w, margin_top + meter_h - y_l)
        painter.fillRect(bar_rect_l, meter_brush)

        # Draw Right Bar
        y_r = self._db_to_y(self._current_r, margin_top, meter_h)
        bar_rect_r = QRectF(x_right, y_r, bar_w, margin_top + meter_h - y_r)
        painter.fillRect(bar_rect_r, meter_brush)

        # Draw Peak Hold lines
        peak_y_l = self._db_to_y(self._peak_l, margin_top, meter_h)
        painter.setPen(QPen(QColor(255, 255, 255, 220), 1.5))
        painter.drawLine(int(x_left), int(peak_y_l), int(x_left + bar_w), int(peak_y_l))

        peak_y_r = self._db_to_y(self._peak_r, margin_top, meter_h)
        painter.drawLine(int(x_right), int(peak_y_r), int(x_right + bar_w), int(peak_y_r))

        # Draw dB tick marks & labels
        ticks = [0, -6, -12, -24, -36, -48]
        painter.setPen(QColor(100, 100, 115))
        font = QFont()
        font.setPointSize(7)
        painter.setFont(font)

        tick_x_start = x_right + bar_w + 3.0
        for db in ticks:
            y = self._db_to_y(float(db), margin_top, meter_h)
            painter.drawLine(int(tick_x_start), int(y), int(tick_x_start + 3), int(y))
            label = f"{abs(db)}" if db != 0 else "0"
            painter.drawText(int(tick_x_start + 5), int(y + 3), label)

        # Top Channel Labels [L] [R]
        painter.setPen(QColor(150, 150, 165))
        font.setPointSize(7)
        font.setBold(True)
        painter.setFont(font)
        painter.drawText(QRectF(x_left - 1, 3, bar_w + 2, 12), Qt.AlignmentFlag.AlignCenter, "L")
        painter.drawText(QRectF(x_right - 1, 3, bar_w + 2, 12), Qt.AlignmentFlag.AlignCenter, "R")
