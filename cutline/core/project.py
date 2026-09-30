from __future__ import annotations

from dataclasses import dataclass, field
import json
import logging
from pathlib import Path
from typing import Any, Optional

from PySide6.QtCore import QObject, Signal

from .media import MediaItem
from .timeline import TimelineModel
from .track import Track, TrackType

logger = logging.getLogger(__name__)


@dataclass
class ProjectSettings:
    name: str = "Untitled Project"
    fps: float = 30.0
    width: int = 1920
    height: int = 1080
    sample_rate: int = 48000

    def to_dict(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "fps": self.fps,
            "width": self.width,
            "height": self.height,
            "sample_rate": self.sample_rate,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ProjectSettings:
        return cls(
            name=data.get("name", "Untitled Project"),
            fps=float(data.get("fps", 30.0)),
            width=int(data.get("width", 1920)),
            height=int(data.get("height", 1080)),
            sample_rate=int(data.get("sample_rate", 48000)),
        )


class Project(QObject):
    """Cutline project manager encapsulating media pool, timeline, and persistence."""

    project_saved = Signal(str)
    project_loaded = Signal(str)
    dirty_state_changed = Signal(bool)
    media_added = Signal(MediaItem)
    media_removed = Signal(str)

    def __init__(
        self,
        settings: Optional[ProjectSettings] = None,
        parent: Optional[QObject] = None,
    ) -> None:
        super().__init__(parent)
        self.settings = settings or ProjectSettings()
        self.file_path: Optional[str] = None
        self.media_pool: dict[str, MediaItem] = {}
        self.timeline = TimelineModel(
            fps=self.settings.fps,
            width=self.settings.width,
            height=self.settings.height,
            parent=self,
        )
        self._is_dirty: bool = False

        # Add initial default tracks if empty
        if not self.timeline.tracks:
            self.timeline.add_track(Track(name="Video 1", track_type=TrackType.VIDEO))
            self.timeline.add_track(Track(name="Audio 1", track_type=TrackType.AUDIO))

    @property
    def is_dirty(self) -> bool:
        return self._is_dirty

    @is_dirty.setter
    def is_dirty(self, val: bool) -> None:
        if self._is_dirty != val:
            self._is_dirty = val
            self.dirty_state_changed.emit(val)

    def mark_dirty(self) -> None:
        self.is_dirty = True

    def add_media(self, item: MediaItem) -> None:
        self.media_pool[item.id] = item
        self.mark_dirty()
        self.media_added.emit(item)

    def remove_media(self, media_id: str) -> Optional[MediaItem]:
        if media_id in self.media_pool:
            item = self.media_pool.pop(media_id)
            self.mark_dirty()
            self.media_removed.emit(media_id)
            return item
        return None

    def get_media(self, media_id: str) -> Optional[MediaItem]:
        return self.media_pool.get(media_id)

    def to_dict(self) -> dict[str, Any]:
        base_dir = Path(self.file_path).parent if self.file_path else None
        return {
            "format": "cutline",
            "version": 1,
            "settings": self.settings.to_dict(),
            "media_pool": [item.to_dict(base_dir=base_dir) for item in self.media_pool.values()],
            "timeline": self.timeline.to_dict(),
        }

    def save(self, target_path: Optional[str] = None) -> None:
        if target_path:
            self.file_path = str(Path(target_path).resolve())
        if not self.file_path:
            raise ValueError("No file path specified for saving project")

        dest = Path(self.file_path)
        dest.parent.mkdir(parents=True, exist_ok=True)

        payload = self.to_dict()
        tmp_path = dest.with_suffix(".tmp")
        with open(tmp_path, "w", encoding="utf-8") as f:
            json.dump(payload, f, indent=2, ensure_ascii=False)

        # Atomic rename
        tmp_path.replace(dest)
        self.is_dirty = False
        self.project_saved.emit(self.file_path)
        logger.info("Project successfully saved to %s", self.file_path)

    @classmethod
    def load(cls, file_path: str, parent: Optional[QObject] = None) -> Project:
        path = Path(file_path).resolve()
        if not path.is_file():
            raise FileNotFoundError(f"Project file not found: {file_path}")

        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)

        if data.get("format") != "cutline":
            raise ValueError("Invalid project file format: missing 'cutline' header")

        settings = ProjectSettings.from_dict(data.get("settings", {}))
        proj = cls(settings=settings, parent=parent)
        proj.file_path = str(path)

        base_dir = path.parent
        proj.media_pool.clear()
        for md in data.get("media_pool", []):
            item = MediaItem.from_dict(md, base_dir=base_dir)
            proj.media_pool[item.id] = item

        proj.timeline = TimelineModel.from_dict(data.get("timeline", {}), parent=proj)
        proj.is_dirty = False
        proj.project_loaded.emit(str(path))
        return proj
