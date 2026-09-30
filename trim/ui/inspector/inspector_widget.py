from __future__ import annotations

from typing import Optional

from PySide6.QtCore import Qt
from PySide6.QtWidgets import (
    QCheckBox,
    QComboBox,
    QDoubleSpinBox,
    QFormLayout,
    QGroupBox,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QPushButton,
    QScrollArea,
    QVBoxLayout,
    QWidget,
)

from trim.core.clip import Clip
from trim.core.project import Project
from trim.core.track import Track


class InspectorWidget(QWidget):
    """Properties and transform inspector panel in Turkish."""

    def __init__(self, project: Project, parent: Optional[QWidget] = None) -> None:
        super().__init__(parent)
        self.project = project
        self._current_track: Optional[Track] = None
        self._current_clip: Optional[Clip] = None
        self._current_time: float = 0.0
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
        self.grp_transform = QGroupBox("Video Dönüşümü & Keyframe")
        trans_layout = QFormLayout(self.grp_transform)
        trans_layout.setSpacing(6)

        def _make_kf_field(spinbox: QDoubleSpinBox, prop: str) -> tuple[QHBoxLayout, QPushButton]:
            box = QHBoxLayout()
            box.setSpacing(4)
            box.setContentsMargins(0, 0, 0, 0)
            box.addWidget(spinbox)
            btn = QPushButton("◆")
            btn.setFixedSize(22, 22)
            btn.setToolTip(f"{prop} için Keyframe Ekle/Sil")
            btn.setStyleSheet("QPushButton { padding: 0; font-size: 11px; color: #707080; }")
            btn.clicked.connect(lambda: self._on_toggle_keyframe(prop, spinbox.value(), btn))
            box.addWidget(btn)
            return box, btn

        self.spn_pos_x = QDoubleSpinBox()
        self.spn_pos_x.setRange(-3840.0, 3840.0)
        self.spn_pos_x.setSingleStep(10.0)
        self.spn_pos_x.setSuffix(" px")
        self.spn_pos_x.valueChanged.connect(self._on_transform_changed)
        box_pos_x, self.btn_kf_pos_x = _make_kf_field(self.spn_pos_x, "pos_x")
        trans_layout.addRow("Konum X:", box_pos_x)

        self.spn_pos_y = QDoubleSpinBox()
        self.spn_pos_y.setRange(-2160.0, 2160.0)
        self.spn_pos_y.setSingleStep(10.0)
        self.spn_pos_y.setSuffix(" px")
        self.spn_pos_y.valueChanged.connect(self._on_transform_changed)
        box_pos_y, self.btn_kf_pos_y = _make_kf_field(self.spn_pos_y, "pos_y")
        trans_layout.addRow("Konum Y:", box_pos_y)

        self.spn_scale = QDoubleSpinBox()
        self.spn_scale.setRange(0.05, 10.0)
        self.spn_scale.setSingleStep(0.05)
        self.spn_scale.setValue(1.0)
        self.spn_scale.setSuffix("x")
        self.spn_scale.valueChanged.connect(self._on_transform_changed)
        box_scale, self.btn_kf_scale = _make_kf_field(self.spn_scale, "scale")
        trans_layout.addRow("Ölçek:", box_scale)

        self.spn_opacity = QDoubleSpinBox()
        self.spn_opacity.setRange(0.0, 1.0)
        self.spn_opacity.setSingleStep(0.05)
        self.spn_opacity.setValue(1.0)
        self.spn_opacity.valueChanged.connect(self._on_transform_changed)
        box_opacity, self.btn_kf_opacity = _make_kf_field(self.spn_opacity, "opacity")
        trans_layout.addRow("Opaklık:", box_opacity)

        self.form_layout.addWidget(self.grp_transform)

        # 3.5 Speed & Direction Group
        self.grp_speed = QGroupBox("Hız ve Yön")
        speed_layout = QFormLayout(self.grp_speed)
        speed_layout.setSpacing(6)

        self.spn_speed = QDoubleSpinBox()
        self.spn_speed.setRange(0.1, 10.0)
        self.spn_speed.setSingleStep(0.1)
        self.spn_speed.setValue(1.0)
        self.spn_speed.setSuffix("x")
        self.spn_speed.valueChanged.connect(self._on_speed_changed)
        speed_layout.addRow("Hız:", self.spn_speed)

        preset_box = QHBoxLayout()
        preset_box.setSpacing(4)
        for p in [0.5, 1.0, 2.0]:
            p_btn = QPushButton(f"{p}x")
            p_btn.setStyleSheet("padding: 2px 6px; font-size: 11px;")
            p_btn.clicked.connect(lambda _, val=p: self.spn_speed.setValue(val))
            preset_box.addWidget(p_btn)
        speed_layout.addRow("Hazır:", preset_box)

        self.chk_reverse = QCheckBox("Geriye Oynat (Reverse)")
        self.chk_reverse.toggled.connect(self._on_reverse_changed)
        speed_layout.addRow("", self.chk_reverse)

        self.form_layout.addWidget(self.grp_speed)

        # 4. Color & Filters Group
        self.grp_color = QGroupBox("Renk ve Filtreler")
        color_layout = QFormLayout(self.grp_color)
        color_layout.setSpacing(6)

        self.spn_brightness = QDoubleSpinBox()
        self.spn_brightness.setRange(-1.0, 1.0)
        self.spn_brightness.setSingleStep(0.05)
        self.spn_brightness.setValue(0.0)
        self.spn_brightness.valueChanged.connect(self._on_color_changed)
        color_layout.addRow("Parlaklık:", self.spn_brightness)

        self.spn_contrast = QDoubleSpinBox()
        self.spn_contrast.setRange(0.0, 3.0)
        self.spn_contrast.setSingleStep(0.05)
        self.spn_contrast.setValue(1.0)
        self.spn_contrast.valueChanged.connect(self._on_color_changed)
        color_layout.addRow("Kontrast:", self.spn_contrast)

        self.spn_saturation = QDoubleSpinBox()
        self.spn_saturation.setRange(0.0, 3.0)
        self.spn_saturation.setSingleStep(0.05)
        self.spn_saturation.setValue(1.0)
        self.spn_saturation.valueChanged.connect(self._on_color_changed)
        color_layout.addRow("Doygunluk:", self.spn_saturation)

        btn_reset_color = QPushButton("Renkleri Sıfırla")
        btn_reset_color.setStyleSheet("padding: 4px; font-size: 11px;")
        btn_reset_color.clicked.connect(self._on_reset_color)
        color_layout.addRow("", btn_reset_color)

        self.form_layout.addWidget(self.grp_color)

        # 5. Fade & Transitions Group
        self.grp_fade = QGroupBox("Giriş / Çıkış ve Geçişler")
        fade_layout = QFormLayout(self.grp_fade)
        fade_layout.setSpacing(6)

        self.spn_fade_in = QDoubleSpinBox()
        self.spn_fade_in.setRange(0.0, 60.0)
        self.spn_fade_in.setSingleStep(0.1)
        self.spn_fade_in.setSuffix(" s")
        self.spn_fade_in.valueChanged.connect(self._on_fade_changed)
        fade_layout.addRow("Giriş (Fade In):", self.spn_fade_in)

        self.cmb_transition_in = QComboBox()
        self.cmb_transition_in.addItem("Yok (Düz Kararma)", None)
        self.cmb_transition_in.addItem("Siyaha Geçiş (Dip to Black)", "dip_black")
        self.cmb_transition_in.addItem("Beyaza Geçiş (Dip to White)", "dip_white")
        self.cmb_transition_in.currentIndexChanged.connect(self._on_transition_changed)
        fade_layout.addRow("Giriş Geçişi:", self.cmb_transition_in)

        self.spn_fade_out = QDoubleSpinBox()
        self.spn_fade_out.setRange(0.0, 60.0)
        self.spn_fade_out.setSingleStep(0.1)
        self.spn_fade_out.setSuffix(" s")
        self.spn_fade_out.valueChanged.connect(self._on_fade_changed)
        fade_layout.addRow("Çıkış (Fade Out):", self.spn_fade_out)

        self.cmb_transition_out = QComboBox()
        self.cmb_transition_out.addItem("Yok (Düz Kararma)", None)
        self.cmb_transition_out.addItem("Siyaha Geçiş (Dip to Black)", "dip_black")
        self.cmb_transition_out.addItem("Beyaza Geçiş (Dip to White)", "dip_white")
        self.cmb_transition_out.currentIndexChanged.connect(self._on_transition_changed)
        fade_layout.addRow("Çıkış Geçişi:", self.cmb_transition_out)

        self.form_layout.addWidget(self.grp_fade)

        # 6. Audio Group
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
        self.spn_pos_x.setValue(clip.pos_x)
        self.spn_pos_y.setValue(clip.pos_y)
        self.spn_opacity.setValue(clip.opacity)
        self.spn_scale.setValue(clip.scale)

        is_audio = track.track_type.value == "audio"
        self.grp_transform.setVisible(not is_audio)
        self.grp_color.setVisible(not is_audio)

        self.spn_brightness.setValue(clip.brightness)
        self.spn_contrast.setValue(clip.contrast)
        self.spn_saturation.setValue(clip.saturation)

        idx_in = self.cmb_transition_in.findData(clip.transition_in)
        self.cmb_transition_in.setCurrentIndex(idx_in if idx_in >= 0 else 0)

        idx_out = self.cmb_transition_out.findData(clip.transition_out)
        self.cmb_transition_out.setCurrentIndex(idx_out if idx_out >= 0 else 0)

        self.spn_fade_in.setValue(clip.fade_in)
        self.spn_fade_out.setValue(clip.fade_out)
        self.spn_volume.setValue(clip.volume)
        self.chk_muted.setChecked(clip.muted)

        self.spn_speed.setValue(getattr(clip, "speed", 1.0))
        self.chk_reverse.setChecked(getattr(clip, "reverse", False))
        self._update_kf_buttons()

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
        self._current_clip.pos_x = self.spn_pos_x.value()
        self._current_clip.pos_y = self.spn_pos_y.value()
        self._current_clip.opacity = self.spn_opacity.value()
        self._current_clip.scale = self.spn_scale.value()
        self.project.mark_dirty()
        assert self._current_track is not None
        self.project.timeline.notify_clip_modified(self._current_track.id, self._current_clip.id)

    def _on_color_changed(self) -> None:
        if self._updating_ui or not self._current_clip:
            return
        self._current_clip.brightness = self.spn_brightness.value()
        self._current_clip.contrast = self.spn_contrast.value()
        self._current_clip.saturation = self.spn_saturation.value()
        self.project.mark_dirty()
        assert self._current_track is not None
        self.project.timeline.notify_clip_modified(self._current_track.id, self._current_clip.id)

    def _on_reset_color(self) -> None:
        if not self._current_clip:
            return
        self.spn_brightness.setValue(0.0)
        self.spn_contrast.setValue(1.0)
        self.spn_saturation.setValue(1.0)

    def _on_transition_changed(self) -> None:
        if self._updating_ui or not self._current_clip:
            return
        self._current_clip.transition_in = self.cmb_transition_in.currentData()
        self._current_clip.transition_out = self.cmb_transition_out.currentData()
        self.project.mark_dirty()
        assert self._current_track is not None
        self.project.timeline.notify_clip_modified(self._current_track.id, self._current_clip.id)

    def _on_fade_changed(self) -> None:
        if self._updating_ui or not self._current_clip:
            return
        self._current_clip.fade_in = self.spn_fade_in.value()
        self._current_clip.fade_out = self.spn_fade_out.value()
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

    def set_current_time(self, time: float) -> None:
        """Updates inspector current playhead time reference."""
        self._current_time = max(0.0, time)

    def _update_kf_buttons(self) -> None:
        if not self._current_clip:
            return
        for prop, btn in [
            ("pos_x", self.btn_kf_pos_x),
            ("pos_y", self.btn_kf_pos_y),
            ("scale", self.btn_kf_scale),
            ("opacity", self.btn_kf_opacity),
        ]:
            if self._current_clip.has_keyframes(prop):
                btn.setStyleSheet("QPushButton { padding: 0; font-size: 11px; color: #e07a38; font-weight: bold; }")
            else:
                btn.setStyleSheet("QPushButton { padding: 0; font-size: 11px; color: #707080; }")

    def _on_toggle_keyframe(self, prop: str, value: float, btn: QPushButton) -> None:
        if not self._current_clip or not self._current_track:
            return
        # Calculate clip-relative time from current timeline time
        clip_rel_time = max(0.0, min(self._current_clip.duration, self._current_time - self._current_clip.timeline_in))
        # Toggle: if keyframe exists around clip_rel_time, remove it; else add it
        removed = self._current_clip.remove_keyframe(prop, clip_rel_time, tolerance=0.04)
        if not removed:
            self._current_clip.add_keyframe(prop, clip_rel_time, value)
        self.project.mark_dirty()
        self.project.timeline.notify_clip_modified(self._current_track.id, self._current_clip.id)
        self._update_kf_buttons()

    def _on_speed_changed(self) -> None:
        if self._updating_ui or not self._current_clip or not self._current_track:
            return
        new_speed = self.spn_speed.value()
        self._current_clip.speed = new_speed
        source_dur = (self._current_clip.source_out or (self._current_clip.source_in + self._current_clip.duration)) - self._current_clip.source_in
        new_dur = max(0.04, source_dur / new_speed)
        self._current_clip.timeline_out = self._current_clip.timeline_in + new_dur
        self.lbl_duration.setText(f"{self._current_clip.duration:.2f}s")
        self.project.mark_dirty()
        self.project.timeline.notify_clip_modified(self._current_track.id, self._current_clip.id)
        self.project.timeline.tracks_changed.emit()
        self.project.timeline.duration_changed.emit(self.project.timeline.duration)

    def _on_reverse_changed(self) -> None:
        if self._updating_ui or not self._current_clip or not self._current_track:
            return
        self._current_clip.reverse = self.chk_reverse.isChecked()
        self.project.mark_dirty()
        self.project.timeline.notify_clip_modified(self._current_track.id, self._current_clip.id)

