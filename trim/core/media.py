from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Optional
import uuid


class MediaType(str, Enum):
    VIDEO = "video"
    AUDIO = "audio"
    IMAGE = "image"
    UNKNOWN = "unknown"


@dataclass
class MediaItem:
    file_path: str
    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    name: str = ""
    media_type: MediaType = MediaType.UNKNOWN
    duration: float = 0.0  # In seconds
    fps: float = 30.0
    width: int = 1920
    height: int = 1080
    video_codec: str = ""
    audio_codec: str = ""
    sample_rate: int = 48000
    channels: int = 2
    thumbnail_path: Optional[str] = None

    def __post_init__(self) -> None:
        if not self.name:
            self.name = Path(self.file_path).name
        if isinstance(self.media_type, str) and not isinstance(self.media_type, MediaType):
            try:
                self.media_type = MediaType(self.media_type)
            except ValueError:
                self.media_type = MediaType.UNKNOWN

    @property
    def path(self) -> Path:
        return Path(self.file_path)

    def exists(self) -> bool:
        return self.path.is_file()

    @property
    def has_audio(self) -> bool:
        return bool(self.audio_codec) or self.media_type == MediaType.AUDIO

    @property
    def has_video(self) -> bool:
        return bool(self.video_codec) or self.media_type in (MediaType.VIDEO, MediaType.IMAGE)

    def to_dict(self, base_dir: Optional[Path] = None) -> dict[str, Any]:
        """Serializes media item. If base_dir is given, stores relative path."""
        file_path_str = self.file_path
        if base_dir:
            try:
                rel = Path(self.file_path).resolve().relative_to(base_dir.resolve())
                file_path_str = str(rel)
            except (ValueError, RuntimeError):
                file_path_str = str(Path(self.file_path).resolve())

        return {
            "id": self.id,
            "file_path": file_path_str,
            "name": self.name,
            "media_type": self.media_type.value,
            "duration": self.duration,
            "fps": self.fps,
            "width": self.width,
            "height": self.height,
            "video_codec": self.video_codec,
            "audio_codec": self.audio_codec,
            "sample_rate": self.sample_rate,
            "channels": self.channels,
            "thumbnail_path": self.thumbnail_path,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any], base_dir: Optional[Path] = None) -> MediaItem:
        file_path_str = data["file_path"]
        if base_dir and not Path(file_path_str).is_absolute():
            resolved = (base_dir / file_path_str).resolve()
            if resolved.exists():
                file_path_str = str(resolved)

        return cls(
            id=data.get("id", str(uuid.uuid4())),
            file_path=file_path_str,
            name=data.get("name", Path(file_path_str).name),
            media_type=MediaType(data.get("media_type", "unknown")),
            duration=float(data.get("duration", 0.0)),
            fps=float(data.get("fps", 30.0)),
            width=int(data.get("width", 1920)),
            height=int(data.get("height", 1080)),
            video_codec=data.get("video_codec", ""),
            audio_codec=data.get("audio_codec", ""),
            sample_rate=int(data.get("sample_rate", 48000)),
            channels=int(data.get("channels", 2)),
            thumbnail_path=data.get("thumbnail_path"),
        )
