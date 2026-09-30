from __future__ import annotations

import hashlib
import logging
from pathlib import Path
import subprocess
from typing import Optional

from trim.core.media import MediaItem, MediaType

logger = logging.getLogger(__name__)


class ThumbnailGenerator:
    """Extracts and caches poster thumbnails for media items."""

    def __init__(self, cache_dir: Optional[Path] = None) -> None:
        self.cache_dir = cache_dir or (Path.home() / ".cache" / "trim" / "thumbnails")
        self.cache_dir.mkdir(parents=True, exist_ok=True)

    def get_thumbnail_path(self, item: MediaItem) -> Path:
        key = f"{item.file_path}_{item.duration}_{item.width}x{item.height}"
        h = hashlib.md5(key.encode("utf-8")).hexdigest()
        return self.cache_dir / f"{h}.jpg"

    def generate(self, item: MediaItem) -> Optional[str]:
        """Generates or retrieves cached thumbnail for item."""
        target = self.get_thumbnail_path(item)
        if target.is_file() and target.stat().st_size > 0:
            return str(target)

        source_path = Path(item.file_path)
        if not source_path.is_file():
            return None

        try:
            if item.media_type == MediaType.IMAGE:
                cmd = [
                    "ffmpeg",
                    "-v",
                    "quiet",
                    "-y",
                    "-i",
                    str(source_path),
                    "-vf",
                    "scale=240:-1",
                    "-q:v",
                    "3",
                    str(target),
                ]
                subprocess.run(cmd, check=True)
            elif item.media_type == MediaType.VIDEO:
                # Seek to 10% of duration or 1 sec
                seek_time = min(1.0, max(0.0, item.duration * 0.1))
                cmd = [
                    "ffmpeg",
                    "-v",
                    "quiet",
                    "-y",
                    "-ss",
                    f"{seek_time:.2f}",
                    "-i",
                    str(source_path),
                    "-vframes",
                    "1",
                    "-vf",
                    "scale=240:-1",
                    "-q:v",
                    "3",
                    str(target),
                ]
                subprocess.run(cmd, check=True)
            else:
                # For audio, we'll generate in milestone 2 waveform
                return None

            if target.is_file() and target.stat().st_size > 0:
                return str(target)
        except Exception as e:
            logger.warning("Failed to generate thumbnail for %s: %s", source_path, e)

        return None
