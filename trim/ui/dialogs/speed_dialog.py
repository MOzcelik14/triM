from __future__ import annotations

from typing import Optional

from PySide6.QtCore import Qt
from PySide6.QtWidgets import (
    QCheckBox,
    QDialog,
    QDialogButtonBox,
    QDoubleSpinBox,
    QFormLayout,
    QHBoxLayout,
    QLabel,
    QPushButton,
    QVBoxLayout,
    QWidget,
)

from trim.core.clip import Clip


class SpeedDialog(QDialog):
    """Dialog to adjust clip playback speed multiplier and reverse direction."""

    def __init__(
        self,
        clip: Optional[Clip] = None,
        current_speed: float = 1.0,
        reverse: bool = False,
        current_duration: float = 1.0,
        parent: Optional[QWidget] = None,
    ) -> None:
        super().__init__(parent)
        self.clip = clip
        self._current_duration = clip.duration if clip else current_duration
        init_speed = clip.speed if clip else current_speed
        init_reverse = clip.reverse if clip else reverse

        self.setWindowTitle("Klip Hızı ve Yönü")
        self.setMinimumWidth(320)
        self.setStyleSheet("""
            QDialog {
                background-color: #1c1c22;
                color: #e0e0e8;
            }
            QLabel {
                color: #d0d0dc;
            }
            QPushButton {
                background-color: #2a2a34;
                color: #e0e0e8;
                border: 1px solid #3c3c48;
                border-radius: 4px;
                padding: 4px 10px;
            }
            QPushButton:hover {
                background-color: #383846;
                border-color: #e07a38;
            }
            QDoubleSpinBox {
                background-color: #24242c;
                color: #ffffff;
                border: 1px solid #3c3c48;
                border-radius: 4px;
                padding: 4px;
            }
        """)

        layout = QVBoxLayout(self)
        layout.setSpacing(12)

        form = QFormLayout()
        form.setSpacing(8)

        # Speed spinbox
        self.spn_speed = QDoubleSpinBox()
        self.spn_speed.setRange(0.1, 10.0)
        self.spn_speed.setSingleStep(0.1)
        self.spn_speed.setValue(init_speed)
        self.spn_speed.setSuffix("x")
        self.spn_speed.valueChanged.connect(self._update_preview)
        form.addRow("Hız:", self.spn_speed)

        # Quick preset buttons
        preset_layout = QHBoxLayout()
        for preset in [0.25, 0.5, 1.0, 2.0, 4.0]:
            btn = QPushButton(f"{preset}x")
            btn.clicked.connect(lambda _, p=preset: self.spn_speed.setValue(p))
            preset_layout.addWidget(btn)
        form.addRow("Hazır:", preset_layout)

        # Reverse checkbox
        self.chk_reverse = QCheckBox("Geriye Doğru Oynat (Reverse)")
        self.chk_reverse.setChecked(init_reverse)
        form.addRow("", self.chk_reverse)

        # Duration preview label
        self.lbl_duration = QLabel()
        self.lbl_new_duration = self.lbl_duration
        form.addRow("Yeni Süre:", self.lbl_duration)

        layout.addLayout(form)

        # Dialog Buttons
        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        buttons.button(QDialogButtonBox.StandardButton.Ok).setText("Uygula")
        buttons.button(QDialogButtonBox.StandardButton.Cancel).setText("İptal")
        buttons.accepted.connect(self.accept)
        buttons.rejected.connect(self.reject)
        layout.addWidget(buttons)

        self._update_preview()

    def _update_preview(self) -> None:
        if self.clip:
            source_dur = (self.clip.source_out or (self.clip.source_in + self.clip.duration)) - self.clip.source_in
        else:
            source_dur = self._current_duration
        new_dur = source_dur / max(0.01, self.spn_speed.value())
        self.lbl_duration.setText(f"{new_dur:.2f} s  (Orijinal: {source_dur:.2f} s)")

    @property
    def speed(self) -> float:
        return self.spn_speed.value()

    @property
    def reverse(self) -> bool:
        return self.chk_reverse.isChecked()

    def get_values(self) -> tuple[float, bool]:
        return self.speed, self.reverse
