from __future__ import annotations

import json
import logging
from pathlib import Path
import time
from typing import Optional

from PySide6.QtCore import QObject, QTimer

from .project import Project

logger = logging.getLogger(__name__)


class AutosaveManager(QObject):
    """Manages automatic periodic snapshots of project state for crash recovery."""

    def __init__(
        self,
        project: Project,
        interval_ms: int = 120_000,  # 2 minutes
        recovery_dir: Optional[Path] = None,
        parent: Optional[QObject] = None,
    ) -> None:
        super().__init__(parent)
        self.project = project
        self.recovery_dir = recovery_dir or (Path.home() / ".config" / "trim" / "recovery")
        self.recovery_dir.mkdir(parents=True, exist_ok=True)

        self.timer = QTimer(self)
        self.timer.timeout.connect(self._on_autosave_timer)
        self.timer.start(interval_ms)

    def get_recovery_file_path(self) -> Path:
        if self.project.file_path:
            stem = Path(self.project.file_path).stem
            return self.recovery_dir / f"{stem}_recovery.trim"
        return self.recovery_dir / "unsaved_project_recovery.trim"

    def has_recovery_file(self) -> bool:
        rec_path = self.get_recovery_file_path()
        if rec_path.is_file() and rec_path.stat().st_size > 0:
            return True
        # Check legacy cutline path
        legacy_path = (Path.home() / ".config" / "cutline" / "recovery" / "unsaved_project_recovery.cutline")
        return legacy_path.is_file() and legacy_path.stat().st_size > 0

    def cleanup_recovery_file(self) -> None:
        try:
            rec_path = self.get_recovery_file_path()
            if rec_path.exists():
                rec_path.unlink()
                logger.info("Cleaned up recovery file: %s", rec_path)
        except OSError as e:
            logger.warning("Failed to remove recovery file: %s", e)

    def _on_autosave_timer(self) -> None:
        if not self.project.is_dirty:
            return

        try:
            rec_path = self.get_recovery_file_path()
            data = self.project.to_dict()
            data["_autosave_timestamp"] = time.time()

            tmp_path = rec_path.with_suffix(".tmp")
            with open(tmp_path, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=2, ensure_ascii=False)
            tmp_path.replace(rec_path)
            logger.debug("Autosave completed to %s", rec_path)
        except Exception as e:
            logger.error("Autosave error: %s", e)
