from __future__ import annotations

from typing import Optional

from PySide6.QtCore import Qt, Signal
from PySide6.QtWidgets import (
    QHBoxLayout,
    QLabel,
    QPushButton,
    QSlider,
    QVBoxLayout,
    QWidget,
)

from trim.core.timeline import TimelineModel
from trim.core.track import Track, TrackType


class SingleTrackHeader(QWidget):
    """Header for an individual track with Mute, Solo, Volume, Visibility, and Lock controls."""

    def __init__(
        self,
        track: Track,
        timeline: Optional[TimelineModel] = None,
        parent: Optional[QWidget] = None,
    ) -> None:
        super().__init__(parent)
        self.track = track
        self.timeline = timeline
        self.setFixedHeight(60)
        self.setStyleSheet(
            "background-color: #24242c; border-bottom: 1px solid #32323c; border-right: 1px solid #32323c;"
        )

        main_layout = QVBoxLayout(self)
        main_layout.setContentsMargins(8, 4, 8, 4)
        main_layout.setSpacing(2)

        # Row 1: Name and primary toggles
        row1 = QHBoxLayout()
        row1.setSpacing(4)
        row1.setContentsMargins(0, 0, 0, 0)

        type_prefix = "V" if track.track_type == TrackType.VIDEO else "A"
        self.lbl_name = QLabel(f"{type_prefix}: {track.name}")
        self.lbl_name.setStyleSheet("font-weight: bold; color: #d0d0e0; font-size: 11px;")
        row1.addWidget(self.lbl_name)
        row1.addStretch()

        if track.track_type == TrackType.VIDEO:
            self.btn_vis = QPushButton("👁" if track.visible else "🚫")
            self.btn_vis.setFixedSize(24, 22)
            self.btn_vis.setToolTip("Görünürlüğü Aç / Kapat")
            self.btn_vis.clicked.connect(self._toggle_vis)
            row1.addWidget(self.btn_vis)
        else:
            # Audio Mute button
            self.btn_mute = QPushButton("M")
            self.btn_mute.setFixedSize(22, 22)
            self.btn_mute.setToolTip("Sesi Kapat (Mute)")
            self.btn_mute.setCheckable(True)
            self.btn_mute.setChecked(track.muted)
            self._update_mute_style()
            self.btn_mute.clicked.connect(self._toggle_mute)
            row1.addWidget(self.btn_mute)

            # Audio Solo button
            self.btn_solo = QPushButton("S")
            self.btn_solo.setFixedSize(22, 22)
            self.btn_solo.setToolTip("Yalnız Dinle (Solo)")
            self.btn_solo.setCheckable(True)
            self.btn_solo.setChecked(track.solo)
            self._update_solo_style()
            self.btn_solo.clicked.connect(self._toggle_solo)
            row1.addWidget(self.btn_solo)

        # Lock button
        self.btn_lock = QPushButton("🔓" if not track.locked else "🔒")
        self.btn_lock.setFixedSize(24, 22)
        self.btn_lock.setToolTip("Kanalı Kilitle / Aç")
        self.btn_lock.clicked.connect(self._toggle_lock)
        row1.addWidget(self.btn_lock)
        main_layout.addLayout(row1)

        # Row 2: Secondary controls (Volume slider for audio)
        if track.track_type == TrackType.AUDIO:
            row2 = QHBoxLayout()
            row2.setSpacing(4)
            row2.setContentsMargins(0, 0, 0, 0)

            self.vol_slider = QSlider(Qt.Orientation.Horizontal)
            self.vol_slider.setRange(0, 150)
            self.vol_slider.setValue(int(track.volume * 100))
            self.vol_slider.setFixedHeight(18)
            self.vol_slider.setStyleSheet("""
                QSlider::groove:horizontal {
                    height: 4px;
                    background: #18181c;
                    border-radius: 2px;
                }
                QSlider::sub-page:horizontal {
                    background: #e07a38;
                    border-radius: 2px;
                }
                QSlider::handle:horizontal {
                    background: #ffffff;
                    width: 10px;
                    margin-top: -3px;
                    margin-bottom: -3px;
                    border-radius: 5px;
                }
            """)
            self.vol_slider.setToolTip(f"Kanal Ses Seviyesi: %{int(track.volume * 100)}")
            self.vol_slider.valueChanged.connect(self._on_vol_changed)
            row2.addWidget(self.vol_slider)

            self.lbl_vol = QLabel(f"{int(track.volume * 100)}%")
            self.lbl_vol.setStyleSheet("color: #8e8e99; font-size: 10px; min-width: 32px;")
            row2.addWidget(self.lbl_vol)

            main_layout.addLayout(row2)

    def _update_mute_style(self) -> None:
        if self.track.muted:
            self.btn_mute.setStyleSheet(
                "background-color: #e07a38; color: #ffffff; font-weight: bold; border-radius: 3px; border: none; font-size: 10px;"
            )
        else:
            self.btn_mute.setStyleSheet(
                "background-color: #2e2e38; color: #8e8e99; font-weight: bold; border-radius: 3px; border: 1px solid #3e3e4a; font-size: 10px;"
            )

    def _update_solo_style(self) -> None:
        if self.track.solo:
            self.btn_solo.setStyleSheet(
                "background-color: #38c172; color: #ffffff; font-weight: bold; border-radius: 3px; border: none; font-size: 10px;"
            )
        else:
            self.btn_solo.setStyleSheet(
                "background-color: #2e2e38; color: #8e8e99; font-weight: bold; border-radius: 3px; border: 1px solid #3e3e4a; font-size: 10px;"
            )

    def _toggle_vis(self) -> None:
        self.track.visible = not self.track.visible
        self.btn_vis.setText("👁" if self.track.visible else "🚫")
        if self.timeline:
            self.timeline.tracks_changed.emit()

    def _toggle_lock(self) -> None:
        self.track.locked = not self.track.locked
        self.btn_lock.setText("🔓" if not self.track.locked else "🔒")
        if self.timeline:
            self.timeline.tracks_changed.emit()

    def _toggle_mute(self) -> None:
        self.track.muted = self.btn_mute.isChecked()
        self._update_mute_style()
        if self.timeline:
            self.timeline.tracks_changed.emit()

    def _toggle_solo(self) -> None:
        self.track.solo = self.btn_solo.isChecked()
        self._update_solo_style()
        if self.timeline:
            self.timeline.tracks_changed.emit()

    def _on_vol_changed(self, value: int) -> None:
        self.track.volume = value / 100.0
        self.lbl_vol.setText(f"{value}%")
        self.vol_slider.setToolTip(f"Kanal Ses Seviyesi: %{value}")
        if self.timeline:
            self.timeline.tracks_changed.emit()


class TrackHeadersWidget(QWidget):
    """Container holding vertical track headers synchronized with canvas tracks."""

    def __init__(self, timeline: TimelineModel, parent: Optional[QWidget] = None) -> None:
        super().__init__(parent)
        self.timeline = timeline
        self.setFixedWidth(160)

        self._layout = QVBoxLayout(self)
        self._layout.setContentsMargins(0, 38, 0, 0)
        self._layout.setSpacing(6)
        self._layout.setAlignment(Qt.AlignmentFlag.AlignTop)

        self.timeline.tracks_changed.connect(self.refresh)
        self.refresh()

    def refresh(self) -> None:
        while self._layout.count():
            item = self._layout.takeAt(0)
            if item.widget():
                item.widget().deleteLater()

        for track in self.timeline.tracks:
            header = SingleTrackHeader(track, timeline=self.timeline)
            self._layout.addWidget(header)
