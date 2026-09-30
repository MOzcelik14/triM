from __future__ import annotations

from typing import Optional

from PySide6.QtCore import QRect, QSize, Qt, Signal
from PySide6.QtGui import QColor, QFont, QImage, QPainter, QResizeEvent
from PySide6.QtWidgets import QHBoxLayout, QLabel, QPushButton, QVBoxLayout, QWidget


class MonitorWidget(QWidget):
    """Program Monitor widget rendering frame-accurate preview with letterboxing and minimal triM. empty state."""

    new_project_requested = Signal()
    open_project_requested = Signal()
    import_media_requested = Signal()

    def __init__(self, parent: Optional[QWidget] = None) -> None:
        super().__init__(parent)
        self._current_frame: Optional[QImage] = None
        self._current_time: float = 0.0
        self.setMinimumSize(320, 180)
        self.setStyleSheet("background-color: #121215;")

        # Minimal empty state overlay
        self._empty_overlay = QWidget(self)
        self._empty_overlay.setStyleSheet("background-color: #141417;")
        overlay_layout = QVBoxLayout(self._empty_overlay)
        overlay_layout.setAlignment(Qt.AlignmentFlag.AlignCenter)
        overlay_layout.setSpacing(10)

        # Wordmark: triM.
        lbl_brand = QLabel(
            '<span style="color:#EDEDF2; font-size:38px; font-weight:600; letter-spacing:-1.5px;">tri</span>'
            '<span style="color:#FFFFFF; font-size:38px; font-weight:800; letter-spacing:-1px;">M</span>'
            '<span style="color:#E07A38; font-size:38px; font-weight:900;">.</span>'
        )
        lbl_brand.setAlignment(Qt.AlignmentFlag.AlignCenter)
        overlay_layout.addWidget(lbl_brand)

        lbl_desc = QLabel("A lightweight non-linear video editor.")
        lbl_desc.setStyleSheet("color: #7E7E8C; font-size: 11px; margin-bottom: 6px;")
        lbl_desc.setAlignment(Qt.AlignmentFlag.AlignCenter)
        overlay_layout.addWidget(lbl_desc)

        btn_row = QHBoxLayout()
        btn_row.setSpacing(10)
        btn_row.setAlignment(Qt.AlignmentFlag.AlignCenter)

        btn_new = QPushButton("Yeni Proje")
        btn_new.setStyleSheet(
            "padding: 6px 14px; font-size: 11px; background-color: #202026; "
            "border: 1px solid #2E2E38; border-radius: 3px; color: #D6D6DE;"
        )
        btn_new.setCursor(Qt.CursorShape.PointingHandCursor)
        btn_new.clicked.connect(self.new_project_requested.emit)
        btn_row.addWidget(btn_new)

        btn_open = QPushButton("Proje Aç")
        btn_open.setStyleSheet(
            "padding: 6px 14px; font-size: 11px; background-color: #202026; "
            "border: 1px solid #2E2E38; border-radius: 3px; color: #D6D6DE;"
        )
        btn_open.setCursor(Qt.CursorShape.PointingHandCursor)
        btn_open.clicked.connect(self.open_project_requested.emit)
        btn_row.addWidget(btn_open)

        btn_import = QPushButton("Medya İçe Aktar")
        btn_import.setStyleSheet(
            "padding: 6px 14px; font-size: 11px; background-color: #202026; "
            "border: 1px solid #2E2E38; border-radius: 3px; color: #D6D6DE;"
        )
        btn_import.setCursor(Qt.CursorShape.PointingHandCursor)
        btn_import.clicked.connect(self.import_media_requested.emit)
        btn_row.addWidget(btn_import)

        overlay_layout.addLayout(btn_row)
        self.set_empty_state(True)

    def resizeEvent(self, event: QResizeEvent) -> None:
        super().resizeEvent(event)
        self._empty_overlay.setGeometry(self.rect())

    def set_empty_state(self, empty: bool) -> None:
        self._empty_overlay.setVisible(empty)
        if empty:
            self._empty_overlay.raise_()
        self.update()

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
