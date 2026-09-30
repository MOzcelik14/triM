from __future__ import annotations

import logging
from pathlib import Path
from typing import Optional

from PySide6.QtCore import Qt
from PySide6.QtGui import QAction, QCloseEvent, QIcon, QKeySequence, QShortcut, QUndoStack
from PySide6.QtWidgets import (
    QFileDialog,
    QMainWindow,
    QMessageBox,
    QSplitter,
    QStatusBar,
    QToolBar,
    QVBoxLayout,
    QWidget,
)

from cutline.commands.timeline_commands import AddClipCommand
from cutline.core.autosave import AutosaveManager
from cutline.core.clip import Clip
from cutline.core.project import Project, ProjectSettings
from cutline.media.playback import PlaybackEngine
from cutline.ui.dialogs.export_dialog import ExportDialog
from cutline.ui.inspector.inspector_widget import InspectorWidget
from cutline.ui.preview.monitor_widget import MonitorWidget
from cutline.ui.preview.transport_bar import TransportBar
from cutline.ui.project_bin.media_bin_widget import MediaBinWidget
from cutline.ui.theme import DARK_THEME_QSS
from cutline.ui.timeline.timeline_widget import TimelineWidget

logger = logging.getLogger(__name__)


class MainWindow(QMainWindow):
    """Cutline Primary Application Window."""

    def __init__(self, project: Optional[Project] = None) -> None:
        super().__init__()
        self.project = project or Project()
        self.undo_stack = QUndoStack(self)
        self.playback_engine = PlaybackEngine(self.project, parent=self)
        self.autosave_mgr = AutosaveManager(self.project, parent=self)

        self.setWindowTitle("Cutline - Video Editor")
        self.resize(1366, 850)
        self.setStyleSheet(DARK_THEME_QSS)

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

        # Center: Program Monitor + Transport Bar
        center_widget = QWidget()
        center_layout = QVBoxLayout(center_widget)
        center_layout.setContentsMargins(0, 0, 0, 0)
        center_layout.setSpacing(0)

        self.monitor = MonitorWidget()
        center_layout.addWidget(self.monitor, stretch=1)

        self.transport_bar = TransportBar(self.playback_engine, self.project.timeline)
        center_layout.addWidget(self.transport_bar)
        self.top_splitter.addWidget(center_widget)

        # Right: Inspector
        self.inspector = InspectorWidget(self.project, parent=self)
        self.top_splitter.addWidget(self.inspector)

        # Set default proportions for top panels (25% Bin, 50% Preview, 25% Inspector)
        self.top_splitter.setSizes([320, 700, 320])
        self.v_splitter.addWidget(self.top_splitter)

        # Bottom: Timeline
        self.timeline_widget = TimelineWidget(self.project, self.undo_stack, parent=self)
        self.v_splitter.addWidget(self.timeline_widget)

        # Vertical proportions (60% Top, 40% Timeline)
        self.v_splitter.setSizes([520, 330])
        self.setCentralWidget(self.v_splitter)

        # Status Bar
        self.status_bar = QStatusBar()
        self.setStatusBar(self.status_bar)
        self.status_bar.showMessage(
            f"Project: {self.project.settings.name} | {self.project.timeline.width}x{self.project.timeline.height} @ {self.project.timeline.fps}fps"
        )

    def _setup_actions(self) -> None:
        toolbar = QToolBar("Main Toolbar")
        toolbar.setMovable(False)
        self.addToolBar(toolbar)

        # New
        self.act_new = QAction("New Project", self)
        self.act_new.setShortcut(QKeySequence.StandardKey.New)
        self.act_new.triggered.connect(self.new_project)
        toolbar.addAction(self.act_new)

        # Open
        self.act_open = QAction("Open Project", self)
        self.act_open.setShortcut(QKeySequence.StandardKey.Open)
        self.act_open.triggered.connect(self.open_project)
        toolbar.addAction(self.act_open)

        # Save
        self.act_save = QAction("Save", self)
        self.act_save.setShortcut(QKeySequence.StandardKey.Save)
        self.act_save.triggered.connect(self.save_project)
        toolbar.addAction(self.act_save)

        toolbar.addSeparator()

        # Import Media
        self.act_import = QAction("Import Media", self)
        self.act_import.triggered.connect(self.media_bin.prompt_import_media)
        toolbar.addAction(self.act_import)

        toolbar.addSeparator()

        # Undo
        self.act_undo = self.undo_stack.createUndoAction(self, "Undo")
        self.act_undo.setShortcut(QKeySequence.StandardKey.Undo)
        toolbar.addAction(self.act_undo)

        # Redo
        self.act_redo = self.undo_stack.createRedoAction(self, "Redo")
        self.act_redo.setShortcut(QKeySequence.StandardKey.Redo)
        toolbar.addAction(self.act_redo)

        toolbar.addSeparator()

        # Export
        self.act_export = QAction("Export Video", self)
        self.act_export.setShortcut(QKeySequence("Ctrl+E"))
        self.act_export.triggered.connect(self.export_video)
        toolbar.addAction(self.act_export)

        # Setup Menu Bar
        menu_bar = self.menuBar()
        file_menu = menu_bar.addMenu("&File")
        file_menu.addAction(self.act_new)
        file_menu.addAction(self.act_open)
        file_menu.addAction(self.act_save)
        act_save_as = file_menu.addAction("Save As...")
        act_save_as.triggered.connect(self.save_project_as)
        file_menu.addSeparator()
        file_menu.addAction(self.act_import)
        file_menu.addAction(self.act_export)
        file_menu.addSeparator()
        act_exit = file_menu.addAction("Exit")
        act_exit.triggered.connect(self.close)

        edit_menu = menu_bar.addMenu("&Edit")
        edit_menu.addAction(self.act_undo)
        edit_menu.addAction(self.act_redo)
        edit_menu.addSeparator()
        act_split = edit_menu.addAction("Split at Playhead")
        act_split.setShortcut(QKeySequence("S"))
        act_split.triggered.connect(self.timeline_widget.canvas.split_at_playhead)
        act_del = edit_menu.addAction("Delete Selected")
        act_del.setShortcut(QKeySequence.StandardKey.Delete)
        act_del.triggered.connect(self.timeline_widget.canvas.delete_selected)
        act_ripple = edit_menu.addAction("Ripple Delete")
        act_ripple.setShortcut(QKeySequence("Shift+Delete"))
        act_ripple.triggered.connect(self.timeline_widget.canvas.ripple_delete_selected)

        help_menu = menu_bar.addMenu("&Help")
        act_about = help_menu.addAction("About Cutline")
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

        # S: Split
        sc_split = QShortcut(QKeySequence(Qt.Key.Key_S), self)
        sc_split.activated.connect(self.timeline_widget.canvas.split_at_playhead)

        # Delete / Backspace
        sc_del = QShortcut(QKeySequence(Qt.Key.Key_Delete), self)
        sc_del.activated.connect(self.timeline_widget.canvas.delete_selected)
        sc_back = QShortcut(QKeySequence(Qt.Key.Key_Backspace), self)
        sc_back.activated.connect(self.timeline_widget.canvas.delete_selected)

        # J/K/L navigation
        sc_j = QShortcut(QKeySequence(Qt.Key.Key_J), self)
        sc_j.activated.connect(lambda: self.playback_engine.step_frame(-5))
        sc_k = QShortcut(QKeySequence(Qt.Key.Key_K), self)
        sc_k.activated.connect(self.playback_engine.pause)
        sc_l = QShortcut(QKeySequence(Qt.Key.Key_L), self)
        sc_l.activated.connect(lambda: self.playback_engine.step_frame(5))

    def _connect_signals(self) -> None:
        # Playback to Monitor & Timeline
        self.playback_engine.frame_ready.connect(self.monitor.set_frame)
        self.playback_engine.position_changed.connect(self.timeline_widget.set_current_time)

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

        # Initial frame render
        self.playback_engine.refresh_current_frame()

    def _update_window_title(self, *args) -> None:
        dirty_flag = " *" if self.project.is_dirty else ""
        name = Path(self.project.file_path).name if self.project.file_path else (self.project.settings.name or "Untitled")
        self.setWindowTitle(f"Cutline - {name}{dirty_flag}")

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
                "Unsaved Changes",
                "Current project has unsaved changes. Do you want to save before creating a new project?",
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
                "Unsaved Changes",
                "Current project has unsaved changes. Do you want to save before opening another project?",
                QMessageBox.StandardButton.Save | QMessageBox.StandardButton.Discard | QMessageBox.StandardButton.Cancel,
            )
            if res == QMessageBox.StandardButton.Save:
                self.save_project()
            elif res == QMessageBox.StandardButton.Cancel:
                return

        path, _ = QFileDialog.getOpenFileName(
            self,
            "Open Cutline Project",
            "",
            "Cutline Projects (*.cutline);;JSON Files (*.json);;All Files (*)",
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
            self.status_bar.showMessage(f"Project loaded: {path}", 5000)
        except Exception as e:
            logger.error("Failed to load project %s: %s", path, e)
            QMessageBox.critical(self, "Load Error", f"Failed to load project:\n{e}")

    def save_project(self) -> bool:
        if not self.project.file_path:
            return self.save_project_as()
        try:
            self.project.save()
            self.autosave_mgr.cleanup_recovery_file()
            self._update_window_title()
            self.status_bar.showMessage(f"Project saved: {self.project.file_path}", 4000)
            return True
        except Exception as e:
            logger.error("Failed to save project: %s", e)
            QMessageBox.critical(self, "Save Error", f"Failed to save project:\n{e}")
            return False

    def save_project_as(self) -> bool:
        path, _ = QFileDialog.getSaveFileName(
            self,
            "Save Cutline Project As",
            self.project.file_path or "project.cutline",
            "Cutline Projects (*.cutline);;All Files (*)",
        )
        if not path:
            return False

        if not path.endswith(".cutline"):
            path += ".cutline"

        try:
            self.project.save(path)
            self.autosave_mgr.cleanup_recovery_file()
            self._update_window_title()
            self.status_bar.showMessage(f"Project saved: {path}", 4000)
            return True
        except Exception as e:
            logger.error("Failed to save project: %s", e)
            QMessageBox.critical(self, "Save Error", f"Failed to save project:\n{e}")
            return False

    def export_video(self) -> None:
        dlg = ExportDialog(self.project, parent=self)
        dlg.exec()

    def _reload_project_ui(self) -> None:
        # Recreate playback engine and reconnect
        self.playback_engine = PlaybackEngine(self.project, parent=self)
        self.autosave_mgr = AutosaveManager(self.project, parent=self)

        # Update sub-widgets
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
            "Crash Recovery Available",
            "An unsaved session from an unexpected shutdown was detected.\nWould you like to recover it?",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
        )
        if res == QMessageBox.StandardButton.Yes:
            try:
                self.project = Project.load(str(rec_path), parent=self)
                self._reload_project_ui()
                self.status_bar.showMessage("Project recovered successfully from autosave.", 5000)
            except Exception as e:
                logger.error("Failed to recover project: %s", e)
                QMessageBox.warning(self, "Recovery Error", f"Could not recover autosave: {e}")
        else:
            self.autosave_mgr.cleanup_recovery_file()

    def _show_about(self) -> None:
        QMessageBox.about(
            self,
            "About Cutline",
            "<b>Cutline 0.1.0</b><br><br>"
            "Modern, Fast, Open-Source Non-Linear Video Editor for Linux.<br>"
            "Powered by Python 3.12, PySide6, FFmpeg, and PyAV.<br><br>"
            "Milestone 1 Release.",
        )

    def closeEvent(self, event: QCloseEvent) -> None:
        if self.project.is_dirty:
            res = QMessageBox.question(
                self,
                "Save Changes?",
                "Do you want to save changes before exiting?",
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
