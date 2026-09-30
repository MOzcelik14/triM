from __future__ import annotations

from typing import Optional

from PySide6.QtCore import Qt
from PySide6.QtWidgets import (
    QCheckBox,
    QDoubleSpinBox,
    QFormLayout,
    QGroupBox,
    QLabel,
    QLineEdit,
    QScrollArea,
    QVBoxLayout,
    QWidget,
)

from cutline.core.clip import Clip
from cutline.core.project import Project
from cutline.core.track import Track


class InspectorWidget(QWidget):
    """Properties and transform inspector panel in Turkish."""

    def __init__(self, project: Project, parent: Optional[QWidget] = None) -> None:
        super().__init__(parent)
        self.project = project
        self._current_track: Optional[Track] = None
        self._current_clip: Optional[Clip] = None
        self._updating_ui: bool = False

        self.setMinimumWidth(240)

        main_layout = QVBoxLayout(self)
        main_layout.setContentsMargins(8, 8, 8, 8)
        main_layout.setSpacing(8)

        title = QLabel("Özellikler / Denetçi")
        title.setStyleSheet("font-weight: bold; font-size: 13px; color: #d0d0dc;")
        main_layout.addWidget(title)

        self.scroll = QScrollArea()
        self.scroll.setWidgetResizable(True)
        self.scroll_content = QWidget()
        self.form_layout = QVBoxLayout(self.scroll_content)
        self.form_layout.setContentsMargins(0, 0, 0, 0)
        self.form_layout.setSpacing(10)

        # 1. Info Group
        self.grp_info = QGroupBox("Klip Bilgisi")
        info_layout = QFormLayout(self.grp_info)
        info_layout.setSpacing(6)

        self.txt_name = QLineEdit()
        self.txt_name.editingFinished.connect(self._on_name_changed)
        info_layout.addRow("İsim:", self.txt_name)

        self.lbl_file = QLabel("-")
        self.lbl_file.setWordWrap(True)
        self.lbl_file.setStyleSheet("color: #8e8ea0; font-size: 11px;")
        info_layout.addRow("Kaynak:", self.lbl_file)

        self.lbl_track = QLabel("-")
        info_layout.addRow("Kanal:", self.lbl_track)

        self.lbl_duration = QLabel("0.00s")
        info_layout.addRow("Süre:", self.lbl_duration)

        self.form_layout.addWidget(self.grp_info)

        # 2. Timing Group
        self.grp_timing = QGroupBox("Zamanlama")
        timing_layout = QFormLayout(self.grp_timing)
        timing_layout.setSpacing(6)

        self.spn_timeline_in = QDoubleSpinBox()
        self.spn_timeline_in.setRange(0.0, 99999.0)
        self.spn_timeline_in.setSingleStep(0.1)
        self.spn_timeline_in.setSuffix(" s")
        self.spn_timeline_in.valueChanged.connect(self._on_timing_changed)
        timing_layout.addRow("Başlangıç:", self.spn_timeline_in)

        self.spn_timeline_out = QDoubleSpinBox()
        self.spn_timeline_out.setRange(0.0, 99999.0)
        self.spn_timeline_out.setSingleStep(0.1)
        self.spn_timeline_out.setSuffix(" s")
        self.spn_timeline_out.valueChanged.connect(self._on_timing_changed)
        timing_layout.addRow("Bitiş:", self.spn_timeline_out)

        self.form_layout.addWidget(self.grp_timing)

        # 3. Video Transform Group
        self.grp_transform = QGroupBox("Video Dönüşümü")
        trans_layout = QFormLayout(self.grp_transform)
        trans_layout.setSpacing(6)

        self.spn_opacity = QDoubleSpinBox()
        self.spn_opacity.setRange(0.0, 1.0)
        self.spn_opacity.setSingleStep(0.05)
        self.spn_opacity.setValue(1.0)
        self.spn_opacity.valueChanged.connect(self._on_transform_changed)
        trans_layout.addRow("Opaklık:", self.spn_opacity)

        self.spn_scale = QDoubleSpinBox()
        self.spn_scale.setRange(0.1, 5.0)
        self.spn_scale.setSingleStep(0.1)
        self.spn_scale.setValue(1.0)
        self.spn_scale.valueChanged.connect(self._on_transform_changed)
        trans_layout.addRow("Ölçek:", self.spn_scale)

        self.form_layout.addWidget(self.grp_transform)

        # 4. Audio Group
        self.grp_audio = QGroupBox("Ses")
        audio_layout = QFormLayout(self.grp_audio)
        audio_layout.setSpacing(6)

        self.spn_volume = QDoubleSpinBox()
        self.spn_volume.setRange(0.0, 3.0)
        self.spn_volume.setSingleStep(0.05)
        self.spn_volume.setValue(1.0)
        self.spn_volume.valueChanged.connect(self._on_audio_changed)
        audio_layout.addRow("Ses Düzeyi:", self.spn_volume)

        self.chk_muted = QCheckBox("Klibi Sessize Al")
        self.chk_muted.toggled.connect(self._on_audio_changed)
        audio_layout.addRow("", self.chk_muted)

        self.form_layout.addWidget(self.grp_audio)

        self.form_layout.addStretch()
        self.scroll.setWidget(self.scroll_content)
        main_layout.addWidget(self.scroll)

        # Placeholder label
        self.lbl_placeholder = QLabel("Özelliklerini görüntülemek ve düzenlemek için\nzaman çizgisinden bir klip seçin.")
        self.lbl_placeholder.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.lbl_placeholder.setStyleSheet("color: #707080; font-size: 12px;")
        main_layout.addWidget(self.lbl_placeholder)

        self._show_placeholder(True)

    def _show_placeholder(self, show: bool) -> None:
        self.scroll.setVisible(not show)
        self.lbl_placeholder.setVisible(show)

    def inspect_clip(self, track_id: str, clip_id: str) -> None:
        track = self.project.timeline.get_track_by_id(track_id)
        if not track:
            self._show_placeholder(True)
            return

        clip = track.get_clip_by_id(clip_id)
        if not clip:
            self._show_placeholder(True)
            return

        self._current_track = track
        self._current_clip = clip
        self._updating_ui = True

        self._show_placeholder(False)
        self.txt_name.setText(clip.name)
        type_str = "Video" if track.track_type.value == "video" else "Ses"
        self.lbl_track.setText(f"{track.name} ({type_str})")

        media = self.project.get_media(clip.media_id)
        self.lbl_file.setText(media.file_path if media else "Bilinmiyor")
        self.lbl_duration.setText(f"{clip.duration:.2f}s")

        self.spn_timeline_in.setValue(clip.timeline_in)
        self.spn_timeline_out.setValue(clip.timeline_out)
        self.spn_opacity.setValue(clip.opacity)
        self.spn_scale.setValue(clip.scale)
        self.spn_volume.setValue(clip.volume)
        self.chk_muted.setChecked(clip.muted)

        self._updating_ui = False

    def _on_name_changed(self) -> None:
        if self._updating_ui or not self._current_clip:
            return
        self._current_clip.name = self.txt_name.text()
        self.project.mark_dirty()
        self.project.timeline.tracks_changed.emit()

    def _on_timing_changed(self) -> None:
        if self._updating_ui or not self._current_clip:
            return
        t_in = self.spn_timeline_in.value()
        t_out = self.spn_timeline_out.value()
        if t_out > t_in + 0.04:
            self._current_clip.timeline_in = t_in
            self._current_clip.timeline_out = t_out
            self._current_clip.source_out = self._current_clip.source_in + (t_out - t_in)
            self.lbl_duration.setText(f"{self._current_clip.duration:.2f}s")
            self.project.mark_dirty()
            assert self._current_track is not None
            self.project.timeline.notify_clip_modified(self._current_track.id, self._current_clip.id)

    def _on_transform_changed(self) -> None:
        if self._updating_ui or not self._current_clip:
            return
        self._current_clip.opacity = self.spn_opacity.value()
        self._current_clip.scale = self.spn_scale.value()
        self.project.mark_dirty()
        assert self._current_track is not None
        self.project.timeline.notify_clip_modified(self._current_track.id, self._current_clip.id)

    def _on_audio_changed(self) -> None:
        if self._updating_ui or not self._current_clip:
            return
        self._current_clip.volume = self.spn_volume.value()
        self._current_clip.muted = self.chk_muted.isChecked()
        self.project.mark_dirty()
        assert self._current_track is not None
        self.project.timeline.notify_clip_modified(self._current_track.id, self._current_clip.id)
