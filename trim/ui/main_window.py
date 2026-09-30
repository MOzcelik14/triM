from __future__ import annotations

import logging
from pathlib import Path
from typing import Optional

from PySide6.QtCore import Qt
from PySide6.QtGui import QAction, QCloseEvent, QIcon, QKeySequence, QShortcut, QUndoStack
from PySide6.QtWidgets import (
    QDialog,
    QFileDialog,
    QHBoxLayout,
    QMainWindow,
    QMessageBox,
    QSplitter,
    QStatusBar,
    QToolBar,
    QVBoxLayout,
    QWidget,
)

from trim.commands.timeline_commands import AddClipCommand
from trim.core.autosave import AutosaveManager
from trim.core.clip import Clip
from trim.core.project import Project, ProjectSettings
from trim.core.track import Track, TrackType
from trim.media.playback import PlaybackEngine
from trim.ui.dialogs.export_dialog import ExportDialog
from trim.ui.dialogs.title_dialog import TitleDialog
from trim.ui.inspector.inspector_widget import InspectorWidget
from trim.ui.preview.monitor_widget import MonitorWidget
from trim.ui.preview.transport_bar import TransportBar
from trim.ui.preview.vu_meter import AudioVUMeterWidget
from trim.ui.project_bin.media_bin_widget import MediaBinWidget
from trim.ui.theme import DARK_THEME_QSS
from trim.ui.timeline.timeline_widget import TimelineWidget

logger = logging.getLogger(__name__)


class MainWindow(QMainWindow):
    """triM. Primary Application Window with complete Turkish localization."""

    def __init__(self, project: Optional[Project] = None) -> None:
        super().__init__()
        self.project = project or Project()
        self.undo_stack = QUndoStack(self)
        self.playback_engine = PlaybackEngine(self.project, parent=self)
        self.autosave_mgr = AutosaveManager(self.project, parent=self)

        self.setWindowTitle("triM.")
        self.resize(1366, 850)
        self.setStyleSheet(DARK_THEME_QSS)

        # Set window icon
        icon_path = Path(__file__).resolve().parent.parent / "resources" / "icons" / "trim.png"
        if icon_path.is_file():
            self.setWindowIcon(QIcon(str(icon_path)))

        self._setup_ui()
        self._setup_actions()
        self._setup_shortcuts()
        self._connect_signals()
        self._update_window_title()

        # Check for crash recovery
        if self.autosave_mgr.has_recovery_file():
            self._prompt_recovery()

    def _setup_ui(self) -> None:
        # 1. Main vertical splitter (Top panels vs Bottom Timeline)
        self.v_splitter = QSplitter(Qt.Orientation.Vertical)

        # 2. Top horizontal splitter (Bin | Preview | Inspector)
        self.top_splitter = QSplitter(Qt.Orientation.Horizontal)

        # Left: Project Bin
        self.media_bin = MediaBinWidget(self.project, parent=self)
        self.top_splitter.addWidget(self.media_bin)

        # Center: Program Monitor + VU Meter + Transport Bar
        center_widget = QWidget()
        center_layout = QVBoxLayout(center_widget)
        center_layout.setContentsMargins(0, 0, 0, 0)
        center_layout.setSpacing(0)

        monitor_container = QWidget()
        monitor_layout = QHBoxLayout(monitor_container)
        monitor_layout.setContentsMargins(0, 0, 0, 0)
        monitor_layout.setSpacing(4)

        self.monitor = MonitorWidget()
        monitor_layout.addWidget(self.monitor, stretch=1)

        self.vu_meter = AudioVUMeterWidget()
        monitor_layout.addWidget(self.vu_meter)

        center_layout.addWidget(monitor_container, stretch=1)

        self.transport_bar = TransportBar(self.playback_engine, self.project.timeline)
        center_layout.addWidget(self.transport_bar)
        self.top_splitter.addWidget(center_widget)

        # Right: Inspector
        self.inspector = InspectorWidget(self.project, parent=self)
        self.top_splitter.addWidget(self.inspector)

        # Proportions: Bin 25%, Preview 50%, Inspector 25%
        self.top_splitter.setSizes([320, 700, 320])
        self.v_splitter.addWidget(self.top_splitter)

        # Bottom: Timeline
        self.timeline_widget = TimelineWidget(self.project, self.undo_stack, parent=self)
        self.v_splitter.addWidget(self.timeline_widget)

        # Vertical proportions: 60% Top, 40% Timeline
        self.v_splitter.setSizes([520, 330])
        self.setCentralWidget(self.v_splitter)

        # Status Bar
        self.status_bar = QStatusBar()
        self.setStatusBar(self.status_bar)
        self._update_status_bar()

    def _update_status_bar(self) -> None:
        self.status_bar.showMessage(
            f"Proje: {self.project.settings.name} | {self.project.timeline.width}x{self.project.timeline.height} @ {self.project.timeline.fps:.0f}fps"
        )

    def _setup_actions(self) -> None:
        toolbar = QToolBar("Ana Araç Çubuğu")
        toolbar.setMovable(False)
        self.addToolBar(toolbar)

        # New
        self.act_new = QAction("Yeni Proje", self)
        self.act_new.setShortcut(QKeySequence.StandardKey.New)
        self.act_new.triggered.connect(self.new_project)
        toolbar.addAction(self.act_new)

        # Open
        self.act_open = QAction("Proje Aç", self)
        self.act_open.setShortcut(QKeySequence.StandardKey.Open)
        self.act_open.triggered.connect(self.open_project)
        toolbar.addAction(self.act_open)

        # Save
        self.act_save = QAction("Kaydet", self)
        self.act_save.setShortcut(QKeySequence.StandardKey.Save)
        self.act_save.triggered.connect(self.save_project)
        toolbar.addAction(self.act_save)

        toolbar.addSeparator()

        # Import Media
        self.act_import = QAction("Medya İçe Aktar", self)
        self.act_import.triggered.connect(self.media_bin.prompt_import_media)
        toolbar.addAction(self.act_import)

        # Title Generator
        self.act_add_title = QAction("Başlık / Metin Ekle", self)
        self.act_add_title.setShortcut(QKeySequence("Ctrl+T"))
        self.act_add_title.triggered.connect(self.prompt_add_title)
        toolbar.addAction(self.act_add_title)

        toolbar.addSeparator()

        # Undo
        self.act_undo = self.undo_stack.createUndoAction(self, "Geri Al")
        self.act_undo.setShortcut(QKeySequence.StandardKey.Undo)
        toolbar.addAction(self.act_undo)

        # Redo
        self.act_redo = self.undo_stack.createRedoAction(self, "Yinele")
        self.act_redo.setShortcut(QKeySequence.StandardKey.Redo)
        toolbar.addAction(self.act_redo)

        toolbar.addSeparator()

        # Export
        self.act_export = QAction("Videoyu Dışa Aktar", self)
        self.act_export.setShortcut(QKeySequence("Ctrl+E"))
        self.act_export.triggered.connect(self.export_video)
        toolbar.addAction(self.act_export)

        # Setup Menu Bar
        menu_bar = self.menuBar()
        file_menu = menu_bar.addMenu("&Dosya")
        file_menu.addAction(self.act_new)
        file_menu.addAction(self.act_open)
        file_menu.addAction(self.act_save)
        act_save_as = file_menu.addAction("Farklı Kaydet...")
        act_save_as.triggered.connect(self.save_project_as)
        file_menu.addSeparator()
        file_menu.addAction(self.act_import)
        file_menu.addAction(self.act_add_title)
        file_menu.addAction(self.act_export)
        file_menu.addSeparator()
        act_exit = file_menu.addAction("Çıkış")
        act_exit.triggered.connect(self.close)

        edit_menu = menu_bar.addMenu("&Düzen")
        edit_menu.addAction(self.act_undo)
        edit_menu.addAction(self.act_redo)
        edit_menu.addSeparator()
        act_split = edit_menu.addAction("Oynatma Çizgisinden Kes / Böl")
        act_split.setShortcut(QKeySequence("S"))
        act_split.triggered.connect(self.timeline_widget.canvas.split_at_playhead)
        act_del = edit_menu.addAction("Seçileni Sil")
        act_del.setShortcut(QKeySequence.StandardKey.Delete)
        act_del.triggered.connect(self.timeline_widget.canvas.delete_selected)
        act_ripple = edit_menu.addAction("Boşluksuz Sil (Ripple Delete)")
        act_ripple.setShortcut(QKeySequence("Shift+Delete"))
        act_ripple.triggered.connect(self.timeline_widget.canvas.ripple_delete_selected)

        help_menu = menu_bar.addMenu("&Yardım")
        act_about = help_menu.addAction("triM. Hakkında")
        act_about.triggered.connect(self._show_about)

    def _setup_shortcuts(self) -> None:
        # Space: Play/Pause
        sc_play = QShortcut(QKeySequence(Qt.Key.Key_Space), self)
        sc_play.activated.connect(self.playback_engine.toggle_play)

        # Left: Prev frame
        sc_prev = QShortcut(QKeySequence(Qt.Key.Key_Left), self)
        sc_prev.activated.connect(lambda: self.playback_engine.step_frame(-1))

        # Right: Next frame
        sc_next = QShortcut(QKeySequence(Qt.Key.Key_Right), self)
        sc_next.activated.connect(lambda: self.playback_engine.step_frame(1))

        # Shift+Left: 1 second back
        sc_skip_back = QShortcut(QKeySequence("Shift+Left"), self)
        sc_skip_back.activated.connect(lambda: self.playback_engine.seek(max(0.0, self.playback_engine.current_time - 1.0)))

        # Shift+Right: 1 second forward
        sc_skip_fwd = QShortcut(QKeySequence("Shift+Right"), self)
        sc_skip_fwd.activated.connect(lambda: self.playback_engine.seek(min(self.project.timeline.duration, self.playback_engine.current_time + 1.0)))

        # Up / Down: Jump between edit cut points
        sc_cut_prev = QShortcut(QKeySequence(Qt.Key.Key_Up), self)
        sc_cut_prev.activated.connect(self._jump_to_prev_edit_point)
        sc_cut_next = QShortcut(QKeySequence(Qt.Key.Key_Down), self)
        sc_cut_next.activated.connect(self._jump_to_next_edit_point)

        # Home / End: Start / End of timeline
        sc_home = QShortcut(QKeySequence(Qt.Key.Key_Home), self)
        sc_home.activated.connect(lambda: self.playback_engine.seek(0.0))
        sc_end = QShortcut(QKeySequence(Qt.Key.Key_End), self)
        sc_end.activated.connect(lambda: self.playback_engine.seek(self.project.timeline.duration))

        # S: Split
        sc_split = QShortcut(QKeySequence(Qt.Key.Key_S), self)
        sc_split.activated.connect(self.timeline_widget.canvas.split_at_playhead)

        # Delete / Backspace
        sc_del = QShortcut(QKeySequence(Qt.Key.Key_Delete), self)
        sc_del.activated.connect(self.timeline_widget.canvas.delete_selected)
        sc_back = QShortcut(QKeySequence(Qt.Key.Key_Backspace), self)
        sc_back.activated.connect(self.timeline_widget.canvas.delete_selected)

        # J/K/L Shuttle navigation
        sc_j = QShortcut(QKeySequence(Qt.Key.Key_J), self)
        sc_j.activated.connect(self._on_shuttle_reverse)
        sc_k = QShortcut(QKeySequence(Qt.Key.Key_K), self)
        sc_k.activated.connect(self.playback_engine.pause)
        sc_l = QShortcut(QKeySequence(Qt.Key.Key_L), self)
        sc_l.activated.connect(self._on_shuttle_forward)

    def _connect_signals(self) -> None:
        # Playback to Monitor & Timeline
        self.playback_engine.frame_ready.connect(self.monitor.set_frame)
        self.playback_engine.position_changed.connect(self.timeline_widget.set_current_time)

        # Audio VU meter levels & Playback speed
        self.playback_engine.audio_levels_ready.connect(self.vu_meter.set_levels)
        self.playback_engine.speed_changed.connect(self._on_speed_changed)

        # Timeline seek to Playback
        self.timeline_widget.seek_requested.connect(self.playback_engine.seek)

        # Inspector selection
        self.timeline_widget.clip_selected.connect(self.inspector.inspect_clip)

        # Transport Bar requests
        self.transport_bar.prev_frame_requested.connect(lambda: self.playback_engine.step_frame(-1))
        self.transport_bar.next_frame_requested.connect(lambda: self.playback_engine.step_frame(1))
        self.transport_bar.stop_requested.connect(self.playback_engine.stop)

        # Media bin double click / add to timeline
        self.media_bin.add_to_timeline_requested.connect(self._add_media_to_timeline_at_playhead)

        # Dirty state
        self.project.dirty_state_changed.connect(self._update_window_title)

        # Empty state connections
        self.monitor.new_project_requested.connect(self.new_project)
        self.monitor.open_project_requested.connect(self.open_project)
        self.monitor.import_media_requested.connect(self.media_bin.prompt_import_media)
        self.project.timeline.clip_added.connect(self._update_empty_state)
        self.project.timeline.clip_removed.connect(self._update_empty_state)
        self.project.timeline.tracks_changed.connect(self._update_empty_state)
        self._update_empty_state()

        # Initial frame render
        self.playback_engine.refresh_current_frame()

    def _update_empty_state(self, *args) -> None:
        has_clips = any(len(t.clips) > 0 for t in self.project.timeline.tracks)
        self.monitor.set_empty_state(not has_clips)

    def _on_shuttle_forward(self) -> None:
        speeds = [1.0, 2.0, 4.0, 8.0]
        cur = self.playback_engine.speed
        if not self.playback_engine.is_playing or cur <= 0.0:
            self.playback_engine.set_speed(1.0)
        else:
            next_speeds = [s for s in speeds if s > cur + 0.1]
            new_speed = next_speeds[0] if next_speeds else speeds[-1]
            self.playback_engine.set_speed(new_speed)

    def _on_shuttle_reverse(self) -> None:
        speeds = [-1.0, -2.0, -4.0, -8.0]
        cur = self.playback_engine.speed
        if not self.playback_engine.is_playing or cur >= 0.0:
            self.playback_engine.set_speed(-1.0)
        else:
            next_speeds = [s for s in speeds if s < cur - 0.1]
            new_speed = next_speeds[0] if next_speeds else speeds[-1]
            self.playback_engine.set_speed(new_speed)

    def _on_speed_changed(self, speed: float) -> None:
        if abs(speed - 1.0) < 0.01:
            self._update_status_bar()
        elif speed > 1.0:
            self.status_bar.showMessage(f"İleri Sarma >> {speed:.0f}x | {self.project.settings.name}")
        elif speed < 0.0:
            self.status_bar.showMessage(f"Geri Sarma << {abs(speed):.0f}x | {self.project.settings.name}")

    def _jump_to_prev_edit_point(self) -> None:
        cur = self.playback_engine.current_time
        points = [0.0]
        for track in self.project.timeline.tracks:
            for clip in track.clips:
                points.append(clip.timeline_in)
                points.append(clip.timeline_out)
        prev_points = [p for p in sorted(set(points)) if p < cur - 0.05]
        target = prev_points[-1] if prev_points else 0.0
        self.playback_engine.seek(target)

    def _jump_to_next_edit_point(self) -> None:
        cur = self.playback_engine.current_time
        points = [self.project.timeline.duration]
        for track in self.project.timeline.tracks:
            for clip in track.clips:
                points.append(clip.timeline_in)
                points.append(clip.timeline_out)
        next_points = [p for p in sorted(set(points)) if p > cur + 0.05]
        target = next_points[0] if next_points else self.project.timeline.duration
        self.playback_engine.seek(target)

    def _update_window_title(self, *args) -> None:
        dirty_flag = " *" if self.project.is_dirty else ""
        name = Path(self.project.file_path).name if self.project.file_path else (self.project.settings.name or "İsimsiz Proje")
        self.setWindowTitle(f"triM. - {name}{dirty_flag}")
        self._update_status_bar()

    def _add_media_to_timeline_at_playhead(self, media_id: str) -> None:
        item = self.project.get_media(media_id)
        if not item:
            return
        v_tracks = self.project.timeline.get_video_tracks()
        if not v_tracks:
            return
        target_track = v_tracks[0]
        cur_t = self.playback_engine.current_time
        duration = min(item.duration, 30.0) if item.duration > 0 else 5.0
        clip = Clip(media_id=media_id, timeline_in=cur_t, timeline_out=cur_t + duration, name=item.name)
        cmd = AddClipCommand(self.project.timeline, target_track.id, clip)
        self.undo_stack.push(cmd)
        self.project.mark_dirty()

    def new_project(self) -> None:
        if self.project.is_dirty:
            res = QMessageBox.question(
                self,
                "Kaydedilmemiş Değişiklikler",
                "Mevcut projede kaydedilmemiş değişiklikler var. Yeni bir proje oluşturmadan önce kaydetmek istiyor musunuz?",
                QMessageBox.StandardButton.Save | QMessageBox.StandardButton.Discard | QMessageBox.StandardButton.Cancel,
            )
            if res == QMessageBox.StandardButton.Save:
                self.save_project()
            elif res == QMessageBox.StandardButton.Cancel:
                return

        self.playback_engine.stop()
        self.playback_engine.close()
        self.undo_stack.clear()

        self.project = Project()
        self._reload_project_ui()

    def open_project(self) -> None:
        if self.project.is_dirty:
            res = QMessageBox.question(
                self,
                "Kaydedilmemiş Değişiklikler",
                "Mevcut projede kaydedilmemiş değişiklikler var. Başka bir proje açmadan önce kaydetmek istiyor musunuz?",
                QMessageBox.StandardButton.Save | QMessageBox.StandardButton.Discard | QMessageBox.StandardButton.Cancel,
            )
            if res == QMessageBox.StandardButton.Save:
                self.save_project()
            elif res == QMessageBox.StandardButton.Cancel:
                return

        path, _ = QFileDialog.getOpenFileName(
            self,
            "triM. Projesi Aç",
            "",
            "triM. Projeleri (*.trim);;Cutline Projeleri (*.cutline);;JSON Dosyaları (*.json);;Tüm Dosyalar (*)",
        )
        if not path:
            return

        try:
            self.playback_engine.stop()
            self.playback_engine.close()
            self.undo_stack.clear()

            self.project = Project.load(path, parent=self)
            self._reload_project_ui()
            self.autosave_mgr.cleanup_recovery_file()
            self.status_bar.showMessage(f"Proje yüklendi: {path}", 5000)
        except Exception as e:
            logger.error("Failed to load project %s: %s", path, e)
            QMessageBox.critical(self, "Yükleme Hatası", f"Proje yüklenemedi:\n{e}")

    def save_project(self) -> bool:
        if not self.project.file_path:
            return self.save_project_as()
        try:
            self.project.save()
            self.autosave_mgr.cleanup_recovery_file()
            self._update_window_title()
            self.status_bar.showMessage(f"Proje kaydedildi: {self.project.file_path}", 4000)
            return True
        except Exception as e:
            logger.error("Failed to save project: %s", e)
            QMessageBox.critical(self, "Kaydetme Hatası", f"Proje kaydedilemedi:\n{e}")
            return False

    def save_project_as(self) -> bool:
        path, _ = QFileDialog.getSaveFileName(
            self,
            "triM. Projesini Farklı Kaydet",
            self.project.file_path or "proje.trim",
            "triM. Projeleri (*.trim);;Cutline Projeleri (*.cutline);;Tüm Dosyalar (*)",
        )
        if not path:
            return False

        if not path.endswith(".trim") and not path.endswith(".cutline"):
            path += ".trim"

        try:
            self.project.save(path)
            self.autosave_mgr.cleanup_recovery_file()
            self._update_window_title()
            self.status_bar.showMessage(f"Proje kaydedildi: {path}", 4000)
            return True
        except Exception as e:
            logger.error("Failed to save project: %s", e)
            QMessageBox.critical(self, "Kaydetme Hatası", f"Proje kaydedilemedi:\n{e}")
            return False

    def prompt_add_title(self) -> None:
        dlg = TitleDialog(self.project, playhead_time=self.playback_engine.current_time, parent=self)
        if dlg.exec() == QDialog.DialogCode.Accepted:
            media_item, add_to_timeline = dlg.create_title_media()
            self.project.add_media(media_item)

            if add_to_timeline:
                v_tracks = self.project.timeline.get_video_tracks()
                target_track = v_tracks[0] if v_tracks else None
                if target_track is None:
                    target_track = Track(name="Video 1", track_type=TrackType.VIDEO)
                    self.project.timeline.add_track(target_track)

                t_in = self.playback_engine.current_time
                new_clip = Clip(
                    media_id=media_item.id,
                    timeline_in=t_in,
                    timeline_out=t_in + media_item.duration,
                    name=media_item.name,
                )
                cmd = AddClipCommand(self.project.timeline, target_track.id, new_clip)
                self.undo_stack.push(cmd)
                self.project.mark_dirty()
                self.playback_engine.seek(t_in)
                self.status_bar.showMessage(f"Başlık klibi eklendi: {media_item.name}", 3000)

    def export_video(self) -> None:
        dlg = ExportDialog(self.project, parent=self)
        dlg.exec()

    def _reload_project_ui(self) -> None:
        self.playback_engine = PlaybackEngine(self.project, parent=self)
        self.autosave_mgr = AutosaveManager(self.project, parent=self)

        self.media_bin.project = self.project
        self.media_bin.refresh()

        self.timeline_widget.project = self.project
        self.timeline_widget.canvas.project = self.project
        self.timeline_widget.canvas.timeline = self.project.timeline
        self.timeline_widget.time_ruler.timeline = self.project.timeline
        self.timeline_widget.track_headers.timeline = self.project.timeline
        self.timeline_widget.track_headers.refresh()

        self.inspector.project = self.project
        self.transport_bar.playback_engine = self.playback_engine
        self.transport_bar.timeline = self.project.timeline

        self._connect_signals()
        self._update_window_title()
        self.playback_engine.refresh_current_frame()

    def _prompt_recovery(self) -> None:
        rec_path = self.autosave_mgr.get_recovery_file_path()
        res = QMessageBox.question(
            self,
            "Kurtarma Dosyası Bulundu",
            "Beklenmeyen bir kapanma sonrası kaydedilmemiş bir çalışma oturumu tespit edildi.\nKurtarmak istiyor musunuz?",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
        )
        if res == QMessageBox.StandardButton.Yes:
            try:
                self.project = Project.load(str(rec_path), parent=self)
                self._reload_project_ui()
                self.status_bar.showMessage("Proje otomatik kurtarmadan başarıyla geri yüklendi.", 5000)
            except Exception as e:
                logger.error("Failed to recover project: %s", e)
                QMessageBox.warning(self, "Kurtarma Hatası", f"Kurtarma dosyası yüklenemedi: {e}")
        else:
            self.autosave_mgr.cleanup_recovery_file()

    def _show_about(self) -> None:
        QMessageBox.about(
            self,
            "triM. Hakkında",
            "<h2 style='margin-bottom:2px;'><span style='color:#EDEDF2;'>tri</span><span style='color:#FFFFFF;'>M</span><span style='color:#E07A38;'>.</span></h2>"
            "<b>A lightweight non-linear video editor.</b><br><br>"
            "Sürüm 0.1.0<br><br>"
            "Python 3.12, PySide6, FFmpeg ve PyAV altyapısıyla geliştirilmiş<br>"
            "bağımsız, hafif ve modern video düzenleyici.",
        )

    def closeEvent(self, event: QCloseEvent) -> None:
        if self.project.is_dirty:
            res = QMessageBox.question(
                self,
                "Değişiklikler Kaydedilsin mi?",
                "Çıkmadan önce yaptığınız değişiklikleri kaydetmek istiyor musunuz?",
                QMessageBox.StandardButton.Save | QMessageBox.StandardButton.Discard | QMessageBox.StandardButton.Cancel,
            )
            if res == QMessageBox.StandardButton.Save:
                if self.save_project():
                    self.playback_engine.close()
                    event.accept()
                else:
                    event.ignore()
            elif res == QMessageBox.StandardButton.Discard:
                self.playback_engine.close()
                self.autosave_mgr.cleanup_recovery_file()
                event.accept()
            else:
                event.ignore()
        else:
            self.playback_engine.close()
            self.autosave_mgr.cleanup_recovery_file()
            event.accept()
