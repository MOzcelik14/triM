from __future__ import annotations

import logging
from pathlib import Path
import sys
from typing import Optional

from PySide6.QtCore import Qt
from PySide6.QtGui import QIcon
from PySide6.QtWidgets import QApplication, QMessageBox

from trim.core.project import Project
from trim.ui.main_window import MainWindow

logger = logging.getLogger(__name__)


class TrimApplication(QApplication):
    """Primary QApplication subclass configuring desktop settings and unhandled exception hooks."""

    def __init__(self, argv: list[str]) -> None:
        super().__init__(argv)
        self.setApplicationName("triM.")
        self.setApplicationDisplayName("triM.")
        self.setDesktopFileName("io.github.mozcelik14.triM.desktop")
        self.setOrganizationName("mozcelik14")
        self.setOrganizationDomain("github.com/MOzcelik14")

        # Set application icon
        icon_path = Path(__file__).resolve().parent.parent / "resources" / "icons" / "trim.png"
        if icon_path.is_file():
            self.setWindowIcon(QIcon(str(icon_path)))

        self.main_window: Optional[MainWindow] = None
        self._setup_exception_hook()

    def _setup_exception_hook(self) -> None:
        old_hook = sys.excepthook

        def custom_hook(exctype, value, tb):
            logger.critical("Uncaught exception: %s", value, exc_info=(exctype, value, tb))
            try:
                QMessageBox.critical(
                    self.main_window,
                    "triM. Hatası",
                    f"Beklenmeyen bir hata oluştu:\n{value}\n\nLütfen terminal kayıtlarını kontrol edin.",
                )
            except Exception:
                pass
            old_hook(exctype, value, tb)

        sys.excepthook = custom_hook

    def start(self, initial_project_path: Optional[str] = None) -> int:
        project = None
        if initial_project_path:
            try:
                project = Project.load(initial_project_path)
            except Exception as e:
                logger.error("Failed to load initial project %s: %s", initial_project_path, e)

        self.main_window = MainWindow(project=project)
        self.main_window.show()
        return self.exec()


# Backwards compatibility alias
CutlineApplication = TrimApplication
