from __future__ import annotations

import hashlib
import logging
from pathlib import Path
import subprocess
from typing import Optional

import numpy as np
from PySide6.QtCore import QObject, QThread, Signal

logger = logging.getLogger(__name__)


class WaveformGenerator:
    """Extracts and caches audio waveform min/max peaks for timeline rendering."""

    def __init__(self, cache_dir: Optional[Path] = None, points_per_second: int = 100) -> None:
        self.cache_dir = cache_dir or (Path.home() / ".cache" / "trim" / "waveforms")
        self.cache_dir.mkdir(parents=True, exist_ok=True)
        self.points_per_second = points_per_second

    def _get_cache_path(self, file_path: str) -> Path:
        p = Path(file_path).resolve()
        stat = p.stat() if p.is_file() else None
        size = stat.st_size if stat else 0
        mtime = stat.st_mtime if stat else 0
        key = f"{p}_{size}_{mtime}_{self.points_per_second}"
        h = hashlib.md5(key.encode("utf-8")).hexdigest()
        return self.cache_dir / f"{h}.dat"

    def get_waveform(self, file_path: str) -> Optional[np.ndarray]:
        """Returns (N, 2) float32 array where columns are (min_peak, max_peak) in [-1.0, 1.0]."""
        cache_path = self._get_cache_path(file_path)
        if cache_path.is_file() and cache_path.stat().st_size > 0:
            try:
                arr = np.fromfile(cache_path, dtype=np.float32).reshape(-1, 2)
                return arr
            except Exception as e:
                logger.debug("Failed to load cached waveform %s: %s", cache_path, e)

        # Generate waveform using downsampled mono PCM
        source_path = Path(file_path).resolve()
        if not source_path.is_file():
            return None

        # Sample rate = points_per_second * 40
        sample_rate = self.points_per_second * 40
        cmd = [
            "ffmpeg",
            "-v",
            "quiet",
            "-y",
            "-i",
            str(source_path),
            "-vn",
            "-ac",
            "1",
            "-ar",
            str(sample_rate),
            "-f",
            "s16le",
            "-",
        ]

        try:
            proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
            raw, _ = proc.communicate(timeout=30.0)
            if proc.returncode != 0 or not raw:
                return None

            samples = np.frombuffer(raw, dtype=np.int16)
            if len(samples) == 0:
                return None

            # Reshape into buckets of 40
            buckets = len(samples) // 40
            if buckets == 0:
                return None

            truncated = samples[: buckets * 40].reshape(-1, 40)
            peaks_max = truncated.max(axis=1).astype(np.float32) / 32768.0
            peaks_min = truncated.min(axis=1).astype(np.float32) / 32768.0
            peaks = np.column_stack((peaks_min, peaks_max)).astype(np.float32)

            # Save to cache
            peaks.tofile(cache_path)
            return peaks

        except Exception as e:
            logger.warning("Failed to generate waveform for %s: %s", file_path, e)
            return None


class WaveformWorker(QThread):
    """Background worker for non-blocking waveform generation."""

    waveform_ready = Signal(str, object)  # (media_id, np.ndarray)

    def __init__(
        self,
        media_id: str,
        file_path: str,
        generator: Optional[WaveformGenerator] = None,
        parent: Optional[QObject] = None,
    ) -> None:
        super().__init__(parent)
        self.media_id = media_id
        self.file_path = file_path
        self.generator = generator or WaveformGenerator()

    def run(self) -> None:
        try:
            peaks = self.generator.get_waveform(self.file_path)
            if peaks is not None:
                self.waveform_ready.emit(self.media_id, peaks)
        except Exception as e:
            logger.error("Waveform worker failed for %s: %s", self.file_path, e)
