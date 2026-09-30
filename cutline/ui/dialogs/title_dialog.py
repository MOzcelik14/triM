from __future__ import annotations

import logging
from pathlib import Path
from typing import Optional
import uuid

from PySide6.QtCore import QPointF, QRectF, Qt
from PySide6.QtGui import (
    QBrush,
    QColor,
    QFont,
    QImage,
    QPainter,
    QPixmap,
)
from PySide6.QtWidgets import (
    QCheckBox,
    QColorDialog,
    QComboBox,
    QDialog,
    QDialogButtonBox,
    QDoubleSpinBox,
    QFontComboBox,
    QFormLayout,
    QGroupBox,
    QHBoxLayout,
    QLabel,
    QPlainTextEdit,
    QPushButton,
    QSpinBox,
    QVBoxLayout,
    QWidget,
)

from cutline.commands.timeline_commands import AddClipCommand
from cutline.core.clip import Clip
from cutline.core.media import MediaItem, MediaType
from cutline.core.project import Project
from cutline.core.track import Track, TrackType

logger = logging.getLogger(__name__)


class TitleDialog(QDialog):
    """Modern text and title generator dialog for Cutline."""

    def __init__(self, project: Project, playhead_time: float = 0.0, parent: Optional[QWidget] = None) -> None:
        super().__init__(parent)
        self.project = project
        self.playhead_time = playhead_time
        self.setWindowTitle("Yeni Başlık / Metin Klibi Ekle")
        self.setMinimumSize(640, 520)

        self._text_color = QColor(255, 255, 255)
        self._bg_color = QColor(0, 0, 0, 0)  # Transparent default

        self._titles_dir = Path.home() / ".local" / "share" / "cutline" / "titles"
        self._titles_dir.mkdir(parents=True, exist_ok=True)

        self._setup_ui()
        self._update_preview()

    def _setup_ui(self) -> None:
        main_layout = QHBoxLayout(self)
        main_layout.setSpacing(16)

        # Left Column: Controls
        left_widget = QWidget()
        left_layout = QVBoxLayout(left_widget)
        left_layout.setContentsMargins(0, 0, 0, 0)
        left_layout.setSpacing(10)

        # Text Input
        grp_text = QGroupBox("Metin İçeriği")
        grp_text_layout = QVBoxLayout(grp_text)
        self.txt_content = QPlainTextEdit("Başlık Metni")
        self.txt_content.setMaximumHeight(80)
        self.txt_content.textChanged.connect(self._update_preview)
        grp_text_layout.addWidget(self.txt_content)
        left_layout.addWidget(grp_text)

        # Style & Font
        grp_style = QGroupBox("Yazı Tipi ve Biçimlendirme")
        style_layout = QFormLayout(grp_style)
        style_layout.setSpacing(6)

        self.cmb_font = QFontComboBox()
        self.cmb_font.currentFontChanged.connect(self._update_preview)
        style_layout.addRow("Yazı Tipi:", self.cmb_font)

        self.spn_font_size = QSpinBox()
        self.spn_font_size.setRange(16, 240)
        self.spn_font_size.setValue(64)
        self.spn_font_size.setSuffix(" pt")
        self.spn_font_size.valueChanged.connect(self._update_preview)
        style_layout.addRow("Yazı Boyutu:", self.spn_font_size)

        chk_layout = QHBoxLayout()
        self.chk_bold = QCheckBox("Kalın")
        self.chk_bold.setChecked(True)
        self.chk_bold.toggled.connect(self._update_preview)
        chk_layout.addWidget(self.chk_bold)

        self.chk_italic = QCheckBox("İtalik")
        self.chk_italic.toggled.connect(self._update_preview)
        chk_layout.addWidget(self.chk_italic)
        style_layout.addRow("Biçim:", chk_layout)

        # Color picker
        color_layout = QHBoxLayout()
        self.btn_color = QPushButton("Renk Seç...")
        self.btn_color.clicked.connect(self._pick_color)
        self.lbl_color_sample = QLabel()
        self.lbl_color_sample.setFixedSize(28, 24)
        self._update_color_sample()
        color_layout.addWidget(self.btn_color)
        color_layout.addWidget(self.lbl_color_sample)
        style_layout.addRow("Yazı Rengi:", color_layout)

        # Layout & Alignment
        self.cmb_position = QComboBox()
        self.cmb_position.addItem("Ekran Ortası", "center")
        self.cmb_position.addItem("Alt Üçte Bir (Lower Third)", "lower_third")
        self.cmb_position.addItem("Üst Başlık (Header)", "top")
        self.cmb_position.currentIndexChanged.connect(self._update_preview)
        style_layout.addRow("Konum:", self.cmb_position)

        self.cmb_bg = QComboBox()
        self.cmb_bg.addItem("Saydam (Arkaplan Yok)", "transparent")
        self.cmb_bg.addItem("Koyu Şerit (Bant)", "dark_banner")
        self.cmb_bg.addItem("Tam Karartma", "solid_black")
        self.cmb_bg.currentIndexChanged.connect(self._update_preview)
        style_layout.addRow("Arkaplan:", self.cmb_bg)

        left_layout.addWidget(grp_style)

        # Timing & Placement
        grp_timing = QGroupBox("Zaman Çizgisi")
        timing_layout = QFormLayout(grp_timing)
        timing_layout.setSpacing(6)

        self.spn_duration = QDoubleSpinBox()
        self.spn_duration.setRange(0.5, 60.0)
        self.spn_duration.setValue(5.0)
        self.spn_duration.setSingleStep(0.5)
        self.spn_duration.setSuffix(" s")
        timing_layout.addRow("Klip Süresi:", self.spn_duration)

        self.chk_add_to_timeline = QCheckBox("Zaman Çizgisine Ekle (Oynatma Konumuna)")
        self.chk_add_to_timeline.setChecked(True)
        timing_layout.addRow("", self.chk_add_to_timeline)

        left_layout.addWidget(grp_timing)
        left_layout.addStretch()

        # Dialog buttons
        self.button_box = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        self.button_box.button(QDialogButtonBox.StandardButton.Ok).setText("Oluştur ve Ekle")
        self.button_box.button(QDialogButtonBox.StandardButton.Cancel).setText("İptal")
        self.button_box.accepted.connect(self.accept)
        self.button_box.rejected.connect(self.reject)
        left_layout.addWidget(self.button_box)

        main_layout.addWidget(left_widget, stretch=1)

        # Right Column: Live Preview
        right_widget = QWidget()
        right_layout = QVBoxLayout(right_widget)
        right_layout.setContentsMargins(0, 0, 0, 0)
        right_layout.setSpacing(6)

        lbl_preview_title = QLabel("Canlı Önizleme:")
        lbl_preview_title.setStyleSheet("font-weight: bold; color: #a0a0b0;")
        right_layout.addWidget(lbl_preview_title)

        self.lbl_preview = QLabel()
        self.lbl_preview.setFixedSize(360, 202)  # 16:9 ratio preview
        self.lbl_preview.setStyleSheet("background-color: #1a1a20; border: 1px solid #333340; border-radius: 4px;")
        self.lbl_preview.setAlignment(Qt.AlignmentFlag.AlignCenter)
        right_layout.addWidget(self.lbl_preview)
        right_layout.addStretch()

        main_layout.addWidget(right_widget, stretch=1)

    def _update_color_sample(self) -> None:
        self.lbl_color_sample.setStyleSheet(
            f"background-color: {self._text_color.name()}; border: 1px solid #555; border-radius: 3px;"
        )

    def _pick_color(self) -> None:
        color = QColorDialog.getColor(self._text_color, self, "Yazı Rengi Seçin")
        if color.isValid():
            self._text_color = color
            self._update_color_sample()
            self._update_preview()

    def _render_title_image(self, target_w: int = 1920, target_h: int = 1080) -> QImage:
        img = QImage(target_w, target_h, QImage.Format.Format_ARGB32_Premultiplied)
        img.fill(Qt.GlobalColor.transparent)

        bg_mode = self.cmb_bg.currentData()
        painter = QPainter(img)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing, True)
        painter.setRenderHint(QPainter.RenderHint.TextAntialiasing, True)

        pos_mode = self.cmb_position.currentData()

        # Draw background if selected
        if bg_mode == "solid_black":
            painter.fillRect(img.rect(), QColor(0, 0, 0, 255))
        elif bg_mode == "dark_banner":
            if pos_mode == "lower_third":
                banner_rect = QRectF(0, target_h * 0.65, target_w, target_h * 0.25)
            elif pos_mode == "top":
                banner_rect = QRectF(0, target_h * 0.08, target_w, target_h * 0.22)
            else:
                banner_rect = QRectF(0, target_h * 0.38, target_w, target_h * 0.24)
            painter.fillRect(banner_rect, QColor(0, 0, 0, 175))

        # Font configuration
        font = self.cmb_font.currentFont()
        font.setPointSize(self.spn_font_size.value())
        font.setBold(self.chk_bold.isChecked())
        font.setItalic(self.chk_italic.isChecked())
        painter.setFont(font)

        # Text rect based on position
        pad_x = target_w * 0.08
        if pos_mode == "lower_third":
            text_rect = QRectF(pad_x, target_h * 0.66, target_w - (pad_x * 2), target_h * 0.23)
            flags = Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter | Qt.TextFlag.TextWordWrap
        elif pos_mode == "top":
            text_rect = QRectF(pad_x, target_h * 0.09, target_w - (pad_x * 2), target_h * 0.20)
            flags = Qt.AlignmentFlag.AlignCenter | Qt.AlignmentFlag.AlignVCenter | Qt.TextFlag.TextWordWrap
        else:
            text_rect = QRectF(pad_x, target_h * 0.2, target_w - (pad_x * 2), target_h * 0.6)
            flags = Qt.AlignmentFlag.AlignCenter | Qt.AlignmentFlag.AlignVCenter | Qt.TextFlag.TextWordWrap

        # Text shadow for enhanced readability on transparent overlays
        shadow_rect = QRectF(text_rect.x() + 3, text_rect.y() + 3, text_rect.width(), text_rect.height())
        painter.setPen(QColor(0, 0, 0, 180))
        painter.drawText(shadow_rect, flags, self.txt_content.toPlainText())

        # Main text
        painter.setPen(self._text_color)
        painter.drawText(text_rect, flags, self.txt_content.toPlainText())

        painter.end()
        return img

    def _update_preview(self) -> None:
        high_res = self._render_title_image(1920, 1080)
        scaled = high_res.scaled(
            self.lbl_preview.size(),
            Qt.AspectRatioMode.KeepAspectRatio,
            Qt.TransformationMode.SmoothTransformation,
        )
        self.lbl_preview.setPixmap(QPixmap.fromImage(scaled))

    def create_title_media(self) -> tuple[MediaItem, bool]:
        """Saves generated title image to disk and returns (MediaItem, add_to_timeline)."""
        w = self.project.timeline.width or 1920
        h = self.project.timeline.height or 1080
        img = self._render_title_image(w, h)

        title_id = str(uuid.uuid4())
        file_path = str(self._titles_dir / f"title_{title_id[:8]}.png")
        img.save(file_path, "PNG")

        raw_text = self.txt_content.toPlainText().strip().replace("\n", " ")
        short_title = (raw_text[:20] + "...") if len(raw_text) > 20 else raw_text
        if not short_title:
            short_title = "Metin"

        dur = self.spn_duration.value()
        media_item = MediaItem(
            id=title_id,
            file_path=file_path,
            name=f"Metin: {short_title}",
            media_type=MediaType.IMAGE,
            duration=dur,
            width=w,
            height=h,
        )
        add_to_timeline = self.chk_add_to_timeline.isChecked()
        return media_item, add_to_timeline
