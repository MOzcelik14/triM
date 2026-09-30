from __future__ import annotations

import logging
from pathlib import Path
from typing import Optional

from PySide6.QtCore import Qt
from PySide6.QtWidgets import (
    QComboBox,
    QDialog,
    QFileDialog,
    QFormLayout,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QMessageBox,
    QProgressBar,
    QPushButton,
    QVBoxLayout,
    QWidget,
)

from cutline.core.project import Project
from cutline.export.exporter import ExportWorker
from cutline.export.presets import DEFAULT_PRESETS, ExportPreset

logger = logging.getLogger(__name__)


class ExportDialog(QDialog):
    """Dialog for configuring and executing timeline video export."""

    def __init__(self, project: Project, parent: Optional[QWidget] = None) -> None:
        super().__init__(parent)
        self.project = project
        self.worker: Optional[ExportWorker] = None

        self.setWindowTitle("Export Video")
        self.setMinimumWidth(500)
        self.resize(540, 280)

        layout = QVBoxLayout(self)
        layout.setContentsMargins(16, 16, 16, 16)
        layout.setSpacing(12)

        form = QFormLayout()
        form.setSpacing(10)

        # Preset selector
        self.cbo_preset = QComboBox()
        for p in DEFAULT_PRESETS:
            self.cbo_preset.addItem(p.name, p)
        self.cbo_preset.currentIndexChanged.connect(self._on_preset_changed)
        form.addRow("Preset:", self.cbo_preset)

        # Preset description
        self.lbl_desc = QLabel(DEFAULT_PRESETS[0].description)
        self.lbl_desc.setWordWrap(True)
        self.lbl_desc.setStyleSheet("color: #9a9ab0; font-size: 11px;")
        form.addRow("", self.lbl_desc)

        # Output destination
        dest_layout = QHBoxLayout()
        self.txt_dest = QLineEdit()
        default_out = str(Path.home() / "Videos" / f"{self.project.settings.name or 'output'}.mp4")
        self.txt_dest.setText(default_out)
        dest_layout.addWidget(self.txt_dest)

        self.btn_browse = QPushButton("Browse...")
        self.btn_browse.clicked.connect(self._browse_destination)
        dest_layout.addWidget(self.btn_browse)
        form.addRow("Output File:", dest_layout)

        layout.addLayout(form)

        # Progress bar
        self.progress_bar = QProgressBar()
        self.progress_bar.setRange(0, 100)
        self.progress_bar.setValue(0)
        layout.addWidget(self.progress_bar)

        # Status text
        self.lbl_status = QLabel("Ready to export")
        self.lbl_status.setStyleSheet("color: #a0a0b0;")
        layout.addWidget(self.lbl_status)

        layout.addStretch()

        # Action buttons
        btn_layout = QHBoxLayout()
        btn_layout.addStretch()

        self.btn_cancel = QPushButton("Cancel")
        self.btn_cancel.clicked.connect(self._on_cancel_clicked)
        self.btn_cancel.setEnabled(False)
        btn_layout.addWidget(self.btn_cancel)

        self.btn_export = QPushButton("Export")
        self.btn_export.setObjectName("PrimaryButton")
        self.btn_export.clicked.connect(self._start_export)
        btn_layout.addWidget(self.btn_export)

        self.btn_close = QPushButton("Close")
        self.btn_close.clicked.connect(self.accept)
        btn_layout.addWidget(self.btn_close)

        layout.addLayout(btn_layout)

    def _on_preset_changed(self, idx: int) -> None:
        preset: ExportPreset = self.cbo_preset.currentData()
        if preset:
            self.lbl_desc.setText(preset.description)

    def _browse_destination(self) -> None:
        path, _ = QFileDialog.getSaveFileName(
            self,
            "Choose Export Destination",
            self.txt_dest.text(),
            "MP4 Video (*.mp4);;All Files (*)",
        )
        if path:
            self.txt_dest.setText(path)

    def _start_export(self) -> None:
        target_path = self.txt_dest.text().strip()
        if not target_path:
            QMessageBox.warning(self, "Invalid Path", "Please specify an output file path.")
            return

        preset: ExportPreset = self.cbo_preset.currentData()

        # UI state during export
        self.btn_export.setEnabled(False)
        self.btn_browse.setEnabled(False)
        self.cbo_preset.setEnabled(False)
        self.btn_close.setEnabled(False)
        self.btn_cancel.setEnabled(True)
        self.progress_bar.setValue(0)

        self.worker = ExportWorker(self.project, preset, target_path, parent=self)
        self.worker.progress_updated.connect(self.progress_bar.setValue)
        self.worker.status_updated.connect(self.lbl_status.setText)
        self.worker.export_finished.connect(self._on_export_finished)
        self.worker.start()

    def _on_cancel_clicked(self) -> None:
        if self.worker and self.worker.isRunning():
            self.lbl_status.setText("Cancelling export...")
            self.worker.cancel()

    def _on_export_finished(self, success: bool, msg: str) -> None:
        self.btn_cancel.setEnabled(False)
        self.btn_close.setEnabled(True)
        self.btn_export.setEnabled(True)
        self.btn_browse.setEnabled(True)
        self.cbo_preset.setEnabled(True)

        if success:
            self.lbl_status.setText(f"Export completed: {Path(msg).name}")
            QMessageBox.information(self, "Export Complete", f"Video successfully exported to:\n{msg}")
        else:
            self.lbl_status.setText(f"Export failed: {msg}")
            QMessageBox.critical(self, "Export Error", f"Export failed:\n{msg}")
