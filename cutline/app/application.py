from __future__ import annotations

import logging
import sys
from typing import Optional

from PySide6.QtCore import Qt
from PySide6.QtWidgets import QApplication, QMessageBox

from cutline.core.project import Project
from cutline.ui.main_window import MainWindow

logger = logging.getLogger(__name__)


class CutlineApplication(QApplication):
    """Primary QApplication subclass configuring desktop settings and unhandled exception hooks."""

    def __init__(self, argv: list[str]) -> None:
        super().__init__(argv)
        self.setApplicationName("Cutline")
        self.setApplicationDisplayName("Cutline Video Editor")
        self.setOrganizationName("Cutline")
        self.setOrganizationDomain("cutline.org")

        self.main_window: Optional[MainWindow] = None
        self._setup_exception_hook()

    def _setup_exception_hook(self) -> None:
        old_hook = sys.excepthook

        def custom_hook(exctype, value, tb):
            logger.critical("Uncaught exception: %s", value, exc_info=(exctype, value, tb))
            try:
                QMessageBox.critical(
                    self.main_window,
                    "Cutline Error",
                    f"An unexpected error occurred:\n{value}\n\nPlease check the terminal logs.",
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
