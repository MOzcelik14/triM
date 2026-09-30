from __future__ import annotations

import logging
from pathlib import Path
from typing import Optional

from PySide6.QtCore import QByteArray, QMimeData, QPoint, QSize, Qt, Signal
from PySide6.QtGui import QDrag, QIcon, QPixmap
from PySide6.QtWidgets import (
    QFileDialog,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QListWidget,
    QListWidgetItem,
    QMenu,
    QPushButton,
    QVBoxLayout,
    QWidget,
)

from cutline.core.media import MediaItem, MediaType
from cutline.core.project import Project
from cutline.media.ffprobe import FFprobeAnalyzer
from cutline.media.thumbnails import ThumbnailGenerator

logger = logging.getLogger(__name__)

MIME_MEDIA_ID = "application/x-cutline-media-id"


class MediaListWidget(QListWidget):
    """Custom QListWidget initiating QDrag for timeline placement."""

    def __init__(self, parent: Optional[QWidget] = None) -> None:
        super().__init__(parent)
        self.setDragEnabled(True)
        self.setViewMode(QListWidget.ViewMode.ListMode)
        self.setIconSize(QSize(64, 48))
        self.setSpacing(4)
        self._drag_start_pos: Optional[QPoint] = None

    def mousePressEvent(self, event) -> None:
        if event.button() == Qt.MouseButton.LeftButton:
            self._drag_start_pos = event.pos()
        super().mousePressEvent(event)

    def mouseMoveEvent(self, event) -> None:
        if not (event.buttons() & Qt.MouseButton.LeftButton):
            return super().mouseMoveEvent(event)
        if not self._drag_start_pos:
            return super().mouseMoveEvent(event)

        if (event.pos() - self._drag_start_pos).manhattanLength() < 5:
            return super().mouseMoveEvent(event)

        item = self.currentItem()
        if not item:
            return super().mouseMoveEvent(event)

        media_id = item.data(Qt.ItemDataRole.UserRole)
        if not media_id:
            return super().mouseMoveEvent(event)

        mime_data = QMimeData()
        mime_data.setData(MIME_MEDIA_ID, QByteArray(media_id.encode("utf-8")))

        drag = QDrag(self)
        drag.setMimeData(mime_data)
        drag.setPixmap(item.icon().pixmap(48, 36))
        drag.setHotSpot(QPoint(24, 18))
        drag.exec(Qt.DropAction.CopyAction)


class MediaBinWidget(QWidget):
    """Media Library / Project Bin widget in Turkish."""

    media_selected = Signal(str)  # media_id
    add_to_timeline_requested = Signal(str)  # media_id

    def __init__(self, project: Project, parent: Optional[QWidget] = None) -> None:
        super().__init__(parent)
        self.project = project
        self.thumbnail_gen = ThumbnailGenerator()

        layout = QVBoxLayout(self)
        layout.setContentsMargins(6, 6, 6, 6)
        layout.setSpacing(6)

        # Header / Toolbar
        header_layout = QHBoxLayout()
        title_lbl = QLabel("Medya Havuzu")
        title_lbl.setStyleSheet("font-weight: bold; font-size: 13px; color: #d0d0dc;")
        header_layout.addWidget(title_lbl)
        header_layout.addStretch()

        self.btn_import = QPushButton("+ Medya İçe Aktar")
        self.btn_import.setObjectName("PrimaryButton")
        self.btn_import.clicked.connect(self.prompt_import_media)
        header_layout.addWidget(self.btn_import)
        layout.addLayout(header_layout)

        # Filter search box
        self.search_box = QLineEdit()
        self.search_box.setPlaceholderText("Medyalarda filtrele...")
        self.search_box.textChanged.connect(self._filter_items)
        layout.addWidget(self.search_box)

        # Media List
        self.list_widget = MediaListWidget()
        self.list_widget.itemClicked.connect(self._on_item_clicked)
        self.list_widget.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)
        self.list_widget.customContextMenuRequested.connect(self._show_context_menu)
        layout.addWidget(self.list_widget)

        # Connect project signals
        self.project.media_added.connect(self._on_media_added)
        self.project.media_removed.connect(self._on_media_removed)

        self.refresh()

    def prompt_import_media(self) -> None:
        files, _ = QFileDialog.getOpenFileNames(
            self,
            "Medya Dosyalarını İçe Aktar",
            "",
            "Desteklenen Tüm Medyalar (*.mp4 *.mov *.mkv *.webm *.avi *.mp3 *.wav *.aac *.flac *.png *.jpg *.jpeg *.webp);;"
            "Video Dosyaları (*.mp4 *.mov *.mkv *.webm *.avi);;"
            "Ses Dosyaları (*.mp3 *.wav *.aac *.flac);;"
            "Görsel Dosyaları (*.png *.jpg *.jpeg *.webp);;"
            "Tüm Dosyalar (*)",
        )
        if files:
            for f in files:
                self.import_file(f)

    def import_file(self, file_path: str) -> Optional[MediaItem]:
        try:
            item = FFprobeAnalyzer.analyze(file_path)
            thumb = self.thumbnail_gen.generate(item)
            item.thumbnail_path = thumb
            self.project.add_media(item)
            return item
        except Exception as e:
            logger.error("Failed to import file %s: %s", file_path, e)
            return None

    def refresh(self) -> None:
        self.list_widget.clear()
        for item in self.project.media_pool.values():
            self._add_media_to_list(item)

    def _on_media_added(self, item: MediaItem) -> None:
        self._add_media_to_list(item)

    def _on_media_removed(self, media_id: str) -> None:
        for i in range(self.list_widget.count()):
            it = self.list_widget.item(i)
            if it.data(Qt.ItemDataRole.UserRole) == media_id:
                self.list_widget.takeItem(i)
                break

    def _add_media_to_list(self, item: MediaItem) -> None:
        # Check if already exists
        for i in range(self.list_widget.count()):
            if self.list_widget.item(i).data(Qt.ItemDataRole.UserRole) == item.id:
                return

        # Prepare icon
        icon = QIcon()
        if item.thumbnail_path and Path(item.thumbnail_path).is_file():
            pixmap = QPixmap(item.thumbnail_path)
            icon = QIcon(pixmap)
        else:
            # Fallback icon
            pixmap = QPixmap(64, 48)
            pixmap.fill(Qt.GlobalColor.darkGray)
            icon = QIcon(pixmap)

        # Description text
        dur_str = f"{int(item.duration // 60):02d}:{int(item.duration % 60):02d}"
        if item.media_type == MediaType.VIDEO:
            detail = f"{item.width}x{item.height} | {item.fps:.0f}fps | {dur_str}"
        elif item.media_type == MediaType.AUDIO:
            detail = f"Ses | {item.sample_rate}Hz | {dur_str}"
        elif item.media_type == MediaType.IMAGE:
            detail = f"Görsel | {item.width}x{item.height}"
        else:
            detail = dur_str

        text = f"{item.name}\n{detail}"

        list_item = QListWidgetItem(icon, text)
        list_item.setData(Qt.ItemDataRole.UserRole, item.id)
        list_item.setToolTip(f"{item.file_path}\nCodec: {item.video_codec or item.audio_codec}")
        self.list_widget.addItem(list_item)

    def _on_item_clicked(self, item: QListWidgetItem) -> None:
        media_id = item.data(Qt.ItemDataRole.UserRole)
        if media_id:
            self.media_selected.emit(media_id)

    def _filter_items(self, query: str) -> None:
        query = query.lower()
        for i in range(self.list_widget.count()):
            it = self.list_widget.item(i)
            it.setHidden(query not in it.text().lower())

    def _show_context_menu(self, pos: QPoint) -> None:
        item = self.list_widget.itemAt(pos)
        if not item:
            return

        media_id = item.data(Qt.ItemDataRole.UserRole)
        menu = QMenu(self)

        act_add = menu.addAction("Zaman Çizgisine Ekle")
        act_remove = menu.addAction("Projeden Kaldır")

        action = menu.exec(self.list_widget.mapToGlobal(pos))
        if action == act_add:
            self.add_to_timeline_requested.emit(media_id)
        elif action == act_remove:
            self.project.remove_media(media_id)
