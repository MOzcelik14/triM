from __future__ import annotations

from enum import Enum
import logging
from typing import Optional

from PySide6.QtCore import QPointF, QRectF, QSize, Qt, Signal
from PySide6.QtGui import (
    QBrush,
    QColor,
    QCursor,
    QDragEnterEvent,
    QDragMoveEvent,
    QDropEvent,
    QFont,
    QMouseEvent,
    QPainter,
    QPainterPath,
    QPen,
    QUndoStack,
)
from PySide6.QtWidgets import QMenu, QWidget

from trim.commands.timeline_commands import (
    AddClipCommand,
    ChangeClipSpeedCommand,
    DetachAudioCommand,
    MoveClipCommand,
    MoveMultipleClipsCommand,
    RemoveClipCommand,
    RemoveMultipleClipsCommand,
    RippleDeleteCommand,
    SplitClipCommand,
    TrimClipCommand,
)
from trim.core.clip import Clip
from trim.core.media import MediaType
from trim.core.project import Project
from trim.core.timeline import TimelineModel
from trim.core.track import Track, TrackType
from trim.media.waveform import WaveformGenerator, WaveformWorker
from trim.ui.dialogs.speed_dialog import SpeedDialog
from trim.ui.project_bin.media_bin_widget import MIME_MEDIA_ID
import numpy as np

logger = logging.getLogger(__name__)


class DragMode(Enum):
    NONE = 0
    MOVE = 1
    TRIM_IN = 2
    TRIM_OUT = 3
    SCRUB = 4
    FADE_IN = 5
    FADE_OUT = 6
    RUBBERBAND = 7


class TimelineCanvas(QWidget):
    """Interactive timeline canvas rendering tracks, clips, playhead, and handling editing gestures."""

    clip_selected = Signal(str, str)  # (track_id, clip_id)
    seek_requested = Signal(float)  # (timeline_time)

    def __init__(
        self,
        project: Project,
        undo_stack: QUndoStack,
        pixels_per_second: float = 60.0,
        parent: Optional[QWidget] = None,
    ) -> None:
        super().__init__(parent)
        self.project = project
        self.timeline = project.timeline
        self.undo_stack = undo_stack
        self.pixels_per_second = pixels_per_second

        self.track_height = 60
        self.track_spacing = 6
        self.trim_handle_width = 8

        self.waveform_generator = WaveformGenerator()
        self._waveform_cache: dict[str, Optional[np.ndarray]] = {}
        self._waveform_workers: list[WaveformWorker] = []

        self._current_time: float = 0.0
        self._selected_track_id: Optional[str] = None
        self._selected_clip_id: Optional[str] = None
        self._selected_clip_pairs: list[tuple[str, str]] = []

        self._drag_mode = DragMode.NONE
        self._drag_clip: Optional[Clip] = None
        self._drag_track: Optional[Track] = None
        self._drag_start_x: float = 0.0
        self._drag_orig_bounds: tuple[float, float, float, float] = (0.0, 0.0, 0.0, 0.0)
        self._batch_orig_bounds: dict[str, tuple[float, float, float, float]] = {}

        self._rubberband_start: Optional[QPointF] = None
        self._rubberband_rect: Optional[QRectF] = None
        self._snap_guide_x: Optional[float] = None

        self.setAcceptDrops(True)
        self.setMouseTracking(True)
        self.setFocusPolicy(Qt.FocusPolicy.StrongFocus)

        # Connect model signals
        self.timeline.clip_added.connect(self._on_model_changed)
        self.timeline.clip_removed.connect(self._on_model_changed)
        self.timeline.clip_modified.connect(self._on_model_changed)
        self.timeline.tracks_changed.connect(self._on_model_changed)
        self.timeline.markers_changed.connect(self._on_model_changed)

        self._update_geometry()

    @property
    def selected_clips(self) -> list[tuple[Track, Clip]]:
        """Returns list of selected (track, clip) pairs."""
        result = []
        for tr_id, cl_id in self._selected_clip_pairs:
            tr = self.timeline.get_track_by_id(tr_id)
            if tr:
                cl = tr.get_clip_by_id(cl_id)
                if cl:
                    result.append((tr, cl))
        return result

    @selected_clips.setter
    def selected_clips(self, pairs: list[tuple[Track, Clip]]) -> None:
        self._selected_clip_pairs = [(tr.id, cl.id) for tr, cl in pairs]
        if pairs:
            self._selected_track_id = pairs[0][0].id
            self._selected_clip_id = pairs[0][1].id
            self.clip_selected.emit(self._selected_track_id, self._selected_clip_id)
        else:
            self._selected_track_id = None
            self._selected_clip_id = None
        self.update()

    @property
    def selected_clip(self) -> Optional[Clip]:
        """Returns the primary selected clip for backwards compatibility."""
        clips = self.selected_clips
        return clips[0][1] if clips else None

    @selected_clip.setter
    def selected_clip(self, clip: Optional[Clip]) -> None:
        if clip:
            tr = self.timeline.find_track_for_clip(clip.id)
            if tr:
                self.selected_clips = [(tr, clip)]
            else:
                self.clear_selection()
        else:
            self.clear_selection()

    @property
    def selected_track(self) -> Optional[Track]:
        """Returns the primary selected track for backwards compatibility."""
        clips = self.selected_clips
        if clips:
            return clips[0][0]
        if self._selected_track_id:
            return self.timeline.get_track_by_id(self._selected_track_id)
        return None

    @selected_track.setter
    def selected_track(self, track: Optional[Track]) -> None:
        self._selected_track_id = track.id if track else None

    def clear_selection(self) -> None:
        self._selected_clip_pairs.clear()
        self._selected_track_id = None
        self._selected_clip_id = None
        self.update()

    def _request_waveform(self, media_id: str, file_path: str) -> None:
        if media_id in self._waveform_cache:
            return
        peaks = self.waveform_generator.get_waveform(file_path)
        if peaks is not None:
            self._waveform_cache[media_id] = peaks
            return

        worker = WaveformWorker(media_id, file_path, self.waveform_generator, self)

        def _on_ready(m_id: str, data: np.ndarray) -> None:
            self._waveform_cache[m_id] = data
            if worker in self._waveform_workers:
                self._waveform_workers.remove(worker)
            self.update()

        worker.waveform_ready.connect(_on_ready)
        self._waveform_workers.append(worker)
        worker.start()

    def set_pixels_per_second(self, pps: float) -> None:
        self.pixels_per_second = max(5.0, min(1000.0, pps))
        self._update_geometry()
        self.update()

    def set_current_time(self, time: float) -> None:
        self._current_time = max(0.0, time)
        self.update()

    def _on_model_changed(self, *args) -> None:
        self._update_geometry()
        self.update()

    def _update_geometry(self) -> None:
        min_sec = max(self.timeline.duration + 30.0, 60.0)
        total_w = int(min_sec * self.pixels_per_second)
        total_h = max(200, len(self.timeline.tracks) * (self.track_height + self.track_spacing) + 40)
        self.setMinimumSize(total_w, total_h)
        self.resize(max(self.width(), total_w), total_h)

    def _get_track_rect(self, index: int) -> QRectF:
        y = index * (self.track_height + self.track_spacing) + 10
        return QRectF(0, y, self.width(), self.track_height)

    def _get_clip_rect(self, track_idx: int, clip: Clip) -> QRectF:
        x = clip.timeline_in * self.pixels_per_second
        w = max(4.0, clip.duration * self.pixels_per_second)
        y = track_idx * (self.track_height + self.track_spacing) + 10
        return QRectF(x, y, w, self.track_height)

    def _find_clip_at_pos(self, pos: QPointF) -> tuple[Optional[Track], Optional[Clip], DragMode]:
        for idx, track in enumerate(self.timeline.tracks):
            track_rect = self._get_track_rect(idx)
            if not track_rect.contains(pos):
                continue

            for clip in track.clips:
                clip_rect = self._get_clip_rect(idx, clip)
                if clip_rect.contains(pos):
                    # Check fade handles first
                    fi_x = clip_rect.left() + (clip.fade_in * self.pixels_per_second)
                    fo_x = clip_rect.right() - (clip.fade_out * self.pixels_per_second)
                    fade_y = clip_rect.top() + 6.0

                    dist_fi = ((pos.x() - fi_x) ** 2 + (pos.y() - fade_y) ** 2) ** 0.5
                    dist_fo = ((pos.x() - fo_x) ** 2 + (pos.y() - fade_y) ** 2) ** 0.5

                    if dist_fi <= 10.0 or (clip.fade_in == 0.0 and pos.y() <= clip_rect.top() + 14 and clip_rect.left() <= pos.x() <= clip_rect.left() + 14):
                        return track, clip, DragMode.FADE_IN

                    if dist_fo <= 10.0 or (clip.fade_out == 0.0 and pos.y() <= clip_rect.top() + 14 and clip_rect.right() - 14 <= pos.x() <= clip_rect.right()):
                        return track, clip, DragMode.FADE_OUT

                    # Check trim handles
                    left_handle = QRectF(clip_rect.left(), clip_rect.top(), self.trim_handle_width, clip_rect.height())
                    right_handle = QRectF(clip_rect.right() - self.trim_handle_width, clip_rect.top(), self.trim_handle_width, clip_rect.height())

                    if left_handle.contains(pos):
                        return track, clip, DragMode.TRIM_IN
                    elif right_handle.contains(pos):
                        return track, clip, DragMode.TRIM_OUT
                    else:
                        return track, clip, DragMode.MOVE

            # Clicked empty track area
            return track, None, DragMode.SCRUB

        return None, None, DragMode.NONE

    def mousePressEvent(self, event: QMouseEvent) -> None:
        if event.button() != Qt.MouseButton.LeftButton:
            return super().mousePressEvent(event)

        pos = event.position()
        track, clip, mode = self._find_clip_at_pos(pos)
        is_ctrl_or_shift = bool(
            event.modifiers() & (Qt.KeyboardModifier.ShiftModifier | Qt.KeyboardModifier.ControlModifier)
        )

        if clip and track:
            pair = (track.id, clip.id)
            if is_ctrl_or_shift:
                if pair in self._selected_clip_pairs:
                    self._selected_clip_pairs.remove(pair)
                else:
                    self._selected_clip_pairs.append(pair)

                if self._selected_clip_pairs:
                    self._selected_track_id, self._selected_clip_id = self._selected_clip_pairs[0]
                    self.clip_selected.emit(self._selected_track_id, self._selected_clip_id)
                else:
                    self._selected_track_id = None
                    self._selected_clip_id = None
            else:
                if pair not in self._selected_clip_pairs:
                    self._selected_clip_pairs = [pair]
                    self._selected_track_id = track.id
                    self._selected_clip_id = clip.id
                    self.clip_selected.emit(track.id, clip.id)
                else:
                    self._selected_track_id = track.id
                    self._selected_clip_id = clip.id

            self._drag_mode = mode
            self._drag_clip = clip
            self._drag_track = track
            self._drag_start_x = pos.x()
            assert clip.source_out is not None
            self._drag_orig_bounds = (clip.timeline_in, clip.timeline_out, clip.source_in, clip.source_out)

            self._batch_orig_bounds = {}
            for tr_id, cl_id in self._selected_clip_pairs:
                tr = self.timeline.get_track_by_id(tr_id)
                if tr:
                    cl = tr.get_clip_by_id(cl_id)
                    if cl and cl.source_out is not None:
                        self._batch_orig_bounds[cl.id] = (
                            cl.timeline_in,
                            cl.timeline_out,
                            cl.source_in,
                            cl.source_out,
                        )
        else:
            if not is_ctrl_or_shift:
                self._selected_clip_pairs = []
                self._selected_track_id = None
                self._selected_clip_id = None

            self._drag_mode = DragMode.RUBBERBAND
            self._rubberband_start = pos
            self._rubberband_rect = QRectF(pos, pos)

        self._snap_guide_x = None
        self.update()

    def mouseMoveEvent(self, event: QMouseEvent) -> None:
        pos = event.position()

        # Update cursor when hovering
        if self._drag_mode == DragMode.NONE:
            _, clip, mode = self._find_clip_at_pos(pos)
            if mode in (DragMode.TRIM_IN, DragMode.TRIM_OUT):
                self.setCursor(Qt.CursorShape.SizeHorCursor)
            elif mode in (DragMode.FADE_IN, DragMode.FADE_OUT):
                self.setCursor(Qt.CursorShape.PointingHandCursor)
            elif mode == DragMode.MOVE:
                self.setCursor(Qt.CursorShape.SizeAllCursor)
            else:
                self.setCursor(Qt.CursorShape.ArrowCursor)
            return

        # Handle active drag
        dx = pos.x() - self._drag_start_x
        dt = dx / self.pixels_per_second

        if self._drag_mode == DragMode.RUBBERBAND and self._rubberband_start:
            self._rubberband_rect = QRectF(self._rubberband_start, pos).normalized()
            intersected_pairs = []
            for idx, track in enumerate(self.timeline.tracks):
                for clip in track.clips:
                    clip_rect = self._get_clip_rect(idx, clip)
                    if self._rubberband_rect.intersects(clip_rect):
                        intersected_pairs.append((track.id, clip.id))
            self._selected_clip_pairs = intersected_pairs
            if intersected_pairs:
                self._selected_track_id, self._selected_clip_id = intersected_pairs[0]
                self.clip_selected.emit(self._selected_track_id, self._selected_clip_id)
            else:
                self._selected_track_id = None
                self._selected_clip_id = None
            self.update()
            return

        elif self._drag_mode == DragMode.SCRUB:
            t = max(0.0, pos.x() / self.pixels_per_second)
            snapped = self.timeline.snap_time(t, threshold=0.1)
            self._snap_guide_x = snapped * self.pixels_per_second if abs(snapped - t) > 1e-4 else None
            self.seek_requested.emit(snapped)
            self.update()
            return

        elif self._drag_mode == DragMode.MOVE and self._drag_clip and self._drag_track:
            is_batch = len(self._selected_clip_pairs) > 1 and self._drag_clip.id in self._batch_orig_bounds
            if is_batch:
                min_in = min(b[0] for b in self._batch_orig_bounds.values())
                dt = max(-min_in, dt)
                orig_in = self._drag_orig_bounds[0]
                target_in = max(0.0, orig_in + dt)
                snapped_in = self.timeline.snap_time(target_in, threshold=0.1, ignore_clip_id=self._drag_clip.id)
                if abs(snapped_in - target_in) > 1e-4:
                    self._snap_guide_x = snapped_in * self.pixels_per_second
                    dt = snapped_in - orig_in
                else:
                    self._snap_guide_x = None

                for tr_id, cl_id in self._selected_clip_pairs:
                    tr = self.timeline.get_track_by_id(tr_id)
                    if tr and cl_id in self._batch_orig_bounds:
                        cl = tr.get_clip_by_id(cl_id)
                        if cl:
                            cl.move_to(max(0.0, self._batch_orig_bounds[cl_id][0] + dt))
            else:
                orig_in = self._drag_orig_bounds[0]
                target_in = max(0.0, orig_in + dt)
                snapped_in = self.timeline.snap_time(target_in, threshold=0.1, ignore_clip_id=self._drag_clip.id)
                if abs(snapped_in - target_in) > 1e-4:
                    self._snap_guide_x = snapped_in * self.pixels_per_second
                else:
                    self._snap_guide_x = None
                self._drag_clip.move_to(snapped_in)

            self.update()

        elif self._drag_mode == DragMode.TRIM_IN and self._drag_clip:
            orig_in = self._drag_orig_bounds[0]
            target_in = max(0.0, orig_in + dt)
            snapped_in = self.timeline.snap_time(target_in, threshold=0.1, ignore_clip_id=self._drag_clip.id)
            if abs(snapped_in - target_in) > 1e-4:
                self._snap_guide_x = snapped_in * self.pixels_per_second
            else:
                self._snap_guide_x = None
            self._drag_clip.trim_in(snapped_in)
            self.update()

        elif self._drag_mode == DragMode.TRIM_OUT and self._drag_clip:
            orig_out = self._drag_orig_bounds[1]
            target_out = max(self._drag_clip.timeline_in + 0.04, orig_out + dt)
            snapped_out = self.timeline.snap_time(target_out, threshold=0.1, ignore_clip_id=self._drag_clip.id)
            if abs(snapped_out - target_out) > 1e-4:
                self._snap_guide_x = snapped_out * self.pixels_per_second
            else:
                self._snap_guide_x = None
            self._drag_clip.trim_out(snapped_out)
            self.update()

        elif self._drag_mode == DragMode.FADE_IN and self._drag_clip and self._drag_track:
            idx = self.timeline.tracks.index(self._drag_track)
            clip_rect = self._get_clip_rect(idx, self._drag_clip)
            raw_dt = (pos.x() - clip_rect.left()) / self.pixels_per_second
            self._drag_clip.fade_in = round(max(0.0, min(self._drag_clip.duration / 2.0, raw_dt)), 2)
            self.update()

        elif self._drag_mode == DragMode.FADE_OUT and self._drag_clip and self._drag_track:
            idx = self.timeline.tracks.index(self._drag_track)
            clip_rect = self._get_clip_rect(idx, self._drag_clip)
            raw_dt = (clip_rect.right() - pos.x()) / self.pixels_per_second
            self._drag_clip.fade_out = round(max(0.0, min(self._drag_clip.duration / 2.0, raw_dt)), 2)
            self.update()

    def mouseReleaseEvent(self, event: QMouseEvent) -> None:
        if event.button() != Qt.MouseButton.LeftButton:
            return super().mouseReleaseEvent(event)

        if self._drag_mode == DragMode.RUBBERBAND:
            if self._rubberband_rect and self._rubberband_start:
                drag_dist = (event.position() - self._rubberband_start).manhattanLength()
                if drag_dist < 5:
                    t = max(0.0, event.position().x() / self.pixels_per_second)
                    snapped = self.timeline.snap_time(t, threshold=0.1)
                    self.seek_requested.emit(snapped)
            self._rubberband_rect = None
            self._rubberband_start = None

        elif self._drag_clip and self._drag_track:
            if self._drag_mode == DragMode.MOVE:
                is_batch = len(self._selected_clip_pairs) > 1 and self._drag_clip.id in self._batch_orig_bounds
                if is_batch:
                    moves = []
                    for tr_id, cl_id in self._selected_clip_pairs:
                        tr = self.timeline.get_track_by_id(tr_id)
                        if tr and cl_id in self._batch_orig_bounds:
                            cl = tr.get_clip_by_id(cl_id)
                            if cl:
                                orig_in = self._batch_orig_bounds[cl_id][0]
                                if abs(cl.timeline_in - orig_in) > 1e-4:
                                    moves.append((tr_id, cl_id, orig_in, cl.timeline_in))
                    if moves:
                        cmd = MoveMultipleClipsCommand(self.timeline, moves)
                        self.undo_stack.push(cmd)
                        self.project.mark_dirty()
                else:
                    if abs(self._drag_clip.timeline_in - self._drag_orig_bounds[0]) > 1e-4:
                        cmd = MoveClipCommand(
                            self.timeline,
                            self._drag_track.id,
                            self._drag_clip.id,
                            self._drag_orig_bounds[0],
                            self._drag_clip.timeline_in,
                        )
                        self.undo_stack.push(cmd)
                        self.project.mark_dirty()

            elif self._drag_mode in (DragMode.TRIM_IN, DragMode.TRIM_OUT):
                assert self._drag_clip.source_out is not None
                new_bounds = (
                    self._drag_clip.timeline_in,
                    self._drag_clip.timeline_out,
                    self._drag_clip.source_in,
                    self._drag_clip.source_out,
                )
                if new_bounds != self._drag_orig_bounds:
                    cmd = TrimClipCommand(
                        self.timeline,
                        self._drag_track.id,
                        self._drag_clip.id,
                        self._drag_orig_bounds,
                        new_bounds,
                    )
                    self.undo_stack.push(cmd)
                    self.project.mark_dirty()

            elif self._drag_mode in (DragMode.FADE_IN, DragMode.FADE_OUT):
                self.project.mark_dirty()
                self.timeline.notify_clip_modified(self._drag_track.id, self._drag_clip.id)

        self._drag_mode = DragMode.NONE
        self._drag_clip = None
        self._drag_track = None
        self._snap_guide_x = None
        self.setCursor(Qt.CursorShape.ArrowCursor)
        self.update()

    def contextMenuEvent(self, event) -> None:
        pos = event.position() if hasattr(event, "position") else event.pos()
        track, clip, _ = self._find_clip_at_pos(pos)
        if not clip or not track:
            return

        pair = (track.id, clip.id)
        if pair not in self._selected_clip_pairs:
            self._selected_clip_pairs = [pair]
            self._selected_track_id = track.id
            self._selected_clip_id = clip.id
            self.clip_selected.emit(track.id, clip.id)
            self.update()

        menu = QMenu(self)
        menu.setStyleSheet("""
            QMenu {
                background-color: #24242c;
                color: #e0e0e8;
                border: 1px solid #383842;
                padding: 4px;
            }
            QMenu::item {
                padding: 6px 20px;
                border-radius: 3px;
            }
            QMenu::item:selected {
                background-color: #e07a38;
                color: #ffffff;
            }
        """)

        # Speed and Duration
        act_speed = menu.addAction("⚡ Hız ve Süre (Speed & Duration)...")
        act_speed.triggered.connect(lambda: self.prompt_speed_for_clip(track.id, clip.id))

        # Detach audio (for video tracks)
        if track.track_type == TrackType.VIDEO:
            act_detach = menu.addAction("🔊 Sesi Ayır (Detach Audio)")
            act_detach.triggered.connect(lambda: self.detach_audio_for_clip(track.id, clip.id))

        act_split = menu.addAction("✂ Klibi Böl (S)")
        act_split.triggered.connect(self.split_at_playhead)

        menu.addSeparator()

        act_delete = menu.addAction("🗑 Sil (Delete)")
        act_delete.triggered.connect(self.delete_selected)

        act_ripple = menu.addAction("⏪ Boşluksuz Sil (Ripple Delete)")
        act_ripple.triggered.connect(self.ripple_delete_selected)

        menu.exec(event.globalPos())

    def prompt_speed_for_clip(self, track_id: str, clip_id: str) -> None:
        track = self.timeline.get_track(track_id)
        if not track:
            return
        clip = track.get_clip(clip_id)
        if not clip:
            return
        dlg = SpeedDialog(
            current_speed=clip.speed,
            reverse=clip.reverse,
            current_duration=clip.duration,
            parent=self,
        )
        if dlg.exec():
            new_speed, new_reverse = dlg.get_values()
            if abs(new_speed - clip.speed) > 1e-4 or new_reverse != clip.reverse:
                cmd = ChangeClipSpeedCommand(self.timeline, track_id, clip_id, new_speed, new_reverse)
                self.undo_stack.push(cmd)
                self.project.mark_dirty()
                self.update()

    def detach_audio_for_clip(self, track_id: str, clip_id: str) -> None:
        cmd = DetachAudioCommand(self.timeline, track_id, clip_id)
        self.undo_stack.push(cmd)
        self.project.mark_dirty()
        self.update()

    # Drag and Drop from Media Bin
    def dragEnterEvent(self, event: QDragEnterEvent) -> None:
        if event.mimeData().hasFormat(MIME_MEDIA_ID):
            event.acceptProposedAction()

    def dragMoveEvent(self, event: QDragMoveEvent) -> None:
        if event.mimeData().hasFormat(MIME_MEDIA_ID):
            event.acceptProposedAction()

    def dropEvent(self, event: QDropEvent) -> None:
        if not event.mimeData().hasFormat(MIME_MEDIA_ID):
            return

        media_id = bytes(event.mimeData().data(MIME_MEDIA_ID)).decode("utf-8")
        media_item = self.project.get_media(media_id)
        if not media_item:
            return

        pos = event.position()
        target_track = None
        for idx, track in enumerate(self.timeline.tracks):
            track_rect = self._get_track_rect(idx)
            if track_rect.contains(pos):
                target_track = track
                break

        # If not dropped on a specific track, choose first compatible track
        if not target_track:
            v_tracks = self.timeline.get_video_tracks()
            target_track = v_tracks[0] if v_tracks else None

        if not target_track:
            return

        t_drop = max(0.0, pos.x() / self.pixels_per_second)
        snapped_in = self.timeline.snap_time(t_drop, threshold=0.15)
        duration = min(media_item.duration, 30.0) if media_item.duration > 0 else 5.0

        new_clip = Clip(
            media_id=media_id,
            timeline_in=snapped_in,
            timeline_out=snapped_in + duration,
            name=media_item.name,
        )

        cmd = AddClipCommand(self.timeline, target_track.id, new_clip)
        self.undo_stack.push(cmd)
        self.project.mark_dirty()

        self._selected_track_id = target_track.id
        self._selected_clip_id = new_clip.id
        self._selected_clip_pairs = [(target_track.id, new_clip.id)]
        self.clip_selected.emit(target_track.id, new_clip.id)

        event.acceptProposedAction()
        self.update()

    # Split and Delete actions
    def split_at_playhead(self) -> None:
        target_clip = None
        target_track = None

        if self._selected_track_id and self._selected_clip_id:
            track = self.timeline.get_track_by_id(self._selected_track_id)
            if track:
                clip = track.get_clip_by_id(self._selected_clip_id)
                if clip and clip.contains_timeline_time(self._current_time):
                    target_clip = clip
                    target_track = track

        if not target_clip:
            for track in self.timeline.tracks:
                clip = track.find_clip_at(self._current_time)
                if clip:
                    target_clip = clip
                    target_track = track
                    break

        if target_clip and target_track:
            # Check strictly within bounds
            if target_clip.timeline_in + 0.05 < self._current_time < target_clip.timeline_out - 0.05:
                cmd = SplitClipCommand(
                    self.timeline, target_track.id, target_clip.id, self._current_time
                )
                self.undo_stack.push(cmd)
                self.project.mark_dirty()
                self.update()

    def delete_selected(self) -> None:
        if len(self._selected_clip_pairs) > 1:
            cmd = RemoveMultipleClipsCommand(self.timeline, list(self._selected_clip_pairs))
            self.undo_stack.push(cmd)
            self.project.mark_dirty()
            self._selected_clip_pairs.clear()
            self._selected_track_id = None
            self._selected_clip_id = None
            self.update()
            return

        if not self._selected_track_id or not self._selected_clip_id:
            return

        cmd = RemoveClipCommand(self.timeline, self._selected_track_id, self._selected_clip_id)
        self.undo_stack.push(cmd)
        self.project.mark_dirty()
        self._selected_clip_pairs.clear()
        self._selected_track_id = None
        self._selected_clip_id = None
        self.update()

    def ripple_delete_selected(self) -> None:
        if not self._selected_track_id or not self._selected_clip_id:
            return

        cmd = RippleDeleteCommand(self.timeline, self._selected_track_id, self._selected_clip_id)
        self.undo_stack.push(cmd)
        self.project.mark_dirty()
        self._selected_clip_pairs.clear()
        self._selected_track_id = None
        self._selected_clip_id = None
        self.update()

    def paintEvent(self, event) -> None:
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing, True)

        # Background
        painter.fillRect(self.rect(), QColor(22, 22, 26))

        # Track multi-selection clip IDs for quick lookup
        selected_clip_ids = {cl_id for _, cl_id in self._selected_clip_pairs}
        if self._selected_clip_id:
            selected_clip_ids.add(self._selected_clip_id)

        # Draw track lanes
        for idx, track in enumerate(self.timeline.tracks):
            track_rect = self._get_track_rect(idx)
            lane_color = QColor(28, 28, 34) if idx % 2 == 0 else QColor(32, 32, 38)
            painter.fillRect(track_rect, lane_color)
            painter.setPen(QColor(42, 42, 50))
            painter.drawLine(
                int(track_rect.left()),
                int(track_rect.bottom()),
                int(track_rect.right()),
                int(track_rect.bottom()),
            )

            # Draw clips on track
            for clip in track.clips:
                clip_rect = self._get_clip_rect(idx, clip)
                is_selected = clip.id in selected_clip_ids

                path = QPainterPath()
                path.addRoundedRect(clip_rect, 4, 4)

                # Base clip fill
                if track.track_type == TrackType.VIDEO:
                    base_color = QColor(42, 85, 128) if not is_selected else QColor(50, 105, 160)
                else:
                    base_color = QColor(36, 100, 70) if not is_selected else QColor(45, 125, 88)

                painter.fillPath(path, QBrush(base_color))

                # Audio waveform rendering
                media_item = self.project.get_media(clip.media_id)
                if media_item and media_item.media_type != MediaType.IMAGE and (track.track_type == TrackType.AUDIO or getattr(media_item, "has_audio", False)):
                    self._request_waveform(clip.media_id, media_item.file_path)
                    wf = self._waveform_cache.get(clip.media_id)
                    if wf is not None and len(wf) > 0:
                        src_in = clip.source_in
                        src_out = clip.source_out or (src_in + clip.duration)
                        pps = self.waveform_generator.points_per_second
                        idx_start = max(0, int(src_in * pps))
                        idx_end = min(len(wf), int(src_out * pps))

                        if idx_end > idx_start:
                            slice_wf = wf[idx_start:idx_end]
                            painter.save()
                            painter.setClipPath(path)
                            wf_pen = QPen(
                                QColor(100, 230, 160, 180)
                                if track.track_type == TrackType.AUDIO
                                else QColor(140, 205, 255, 140),
                                1.5,
                            )
                            painter.setPen(wf_pen)

                            c_w = int(clip_rect.width())
                            cy = clip_rect.center().y()
                            max_amp = (clip_rect.height() - 16) / 2.0

                            for px in range(0, c_w, 2):
                                t_norm = px / max(1, c_w)
                                s_idx = int(t_norm * len(slice_wf))
                                if s_idx < len(slice_wf):
                                    v_min, v_max = slice_wf[s_idx]
                                    x_pos = clip_rect.left() + px
                                    y1 = cy + float(v_min) * max_amp
                                    y2 = cy + float(v_max) * max_amp
                                    painter.drawLine(int(x_pos), int(y1), int(x_pos), int(y2))

                            painter.restore()

                # Fade In overlay & handle
                fi_w = clip.fade_in * self.pixels_per_second
                if clip.fade_in > 0:
                    painter.save()
                    painter.setClipPath(path)
                    fi_path = QPainterPath()
                    fi_path.moveTo(clip_rect.left(), clip_rect.top())
                    fi_path.lineTo(clip_rect.left() + fi_w, clip_rect.top())
                    fi_path.lineTo(clip_rect.left(), clip_rect.bottom())
                    fi_path.closeSubpath()
                    painter.fillPath(fi_path, QBrush(QColor(0, 0, 0, 110)))
                    painter.setPen(QPen(QColor(255, 255, 255, 180), 1.5))
                    painter.drawLine(
                        int(clip_rect.left()),
                        int(clip_rect.bottom()),
                        int(clip_rect.left() + fi_w),
                        int(clip_rect.top()),
                    )
                    painter.restore()

                    # Handle dot
                    painter.setBrush(QBrush(QColor(255, 255, 255, 230)))
                    painter.setPen(QPen(QColor(20, 20, 25), 1.0))
                    painter.drawEllipse(QPointF(clip_rect.left() + fi_w, clip_rect.top() + 6), 4.0, 4.0)
                else:
                    painter.setBrush(QBrush(QColor(255, 255, 255, 120)))
                    painter.setPen(Qt.PenStyle.NoPen)
                    painter.drawEllipse(QPointF(clip_rect.left() + 5, clip_rect.top() + 5), 3.0, 3.0)

                # Fade Out overlay & handle
                fo_w = clip.fade_out * self.pixels_per_second
                if clip.fade_out > 0:
                    painter.save()
                    painter.setClipPath(path)
                    fo_path = QPainterPath()
                    fo_path.moveTo(clip_rect.right() - fo_w, clip_rect.top())
                    fo_path.lineTo(clip_rect.right(), clip_rect.top())
                    fo_path.lineTo(clip_rect.right(), clip_rect.bottom())
                    fo_path.closeSubpath()
                    painter.fillPath(fo_path, QBrush(QColor(0, 0, 0, 110)))
                    painter.setPen(QPen(QColor(255, 255, 255, 180), 1.5))
                    painter.drawLine(
                        int(clip_rect.right() - fo_w),
                        int(clip_rect.top()),
                        int(clip_rect.right()),
                        int(clip_rect.bottom()),
                    )
                    painter.restore()

                    # Handle dot
                    painter.setBrush(QBrush(QColor(255, 255, 255, 230)))
                    painter.setPen(QPen(QColor(20, 20, 25), 1.0))
                    painter.drawEllipse(QPointF(clip_rect.right() - fo_w, clip_rect.top() + 6), 4.0, 4.0)
                else:
                    painter.setBrush(QBrush(QColor(255, 255, 255, 120)))
                    painter.setPen(Qt.PenStyle.NoPen)
                    painter.drawEllipse(QPointF(clip_rect.right() - 5, clip_rect.top() + 5), 3.0, 3.0)

                # Clip outline
                if is_selected:
                    painter.setPen(QPen(QColor(224, 122, 56), 2.0))
                else:
                    painter.setPen(QPen(QColor(60, 60, 75), 1.0))
                painter.drawPath(path)

                # Draw trim handle visual cues
                handle_color = QColor(255, 255, 255, 40)
                painter.fillRect(
                    QRectF(clip_rect.left(), clip_rect.top(), self.trim_handle_width, clip_rect.height()),
                    handle_color,
                )
                painter.fillRect(
                    QRectF(clip_rect.right() - self.trim_handle_width, clip_rect.top(), self.trim_handle_width, clip_rect.height()),
                    handle_color,
                )

                # Clip label text
                painter.setPen(QColor(240, 240, 250))
                font = QFont()
                font.setPointSize(9)
                font.setBold(True)
                painter.setFont(font)

                dur_str = f"{clip.duration:.2f}s"
                text_rect = QRectF(
                    clip_rect.left() + 10,
                    clip_rect.top() + 4,
                    clip_rect.width() - 20,
                    clip_rect.height() - 8,
                )
                display_name = clip.name or "Clip"
                speed_str = ""
                if abs(clip.speed - 1.0) > 0.01 or clip.reverse:
                    parts = []
                    if abs(clip.speed - 1.0) > 0.01:
                        parts.append(f"{clip.speed:g}x")
                    if clip.reverse:
                        parts.append("REV")
                    speed_str = f" [{' '.join(parts)}]"

                kf_str = " ◆" if clip.has_keyframes() else ""
                painter.drawText(
                    text_rect,
                    Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignTop,
                    f"{display_name}  [{dur_str}]{speed_str}{kf_str}",
                )

                # Draw keyframe markers on the clip
                if clip.has_keyframes():
                    painter.save()
                    painter.setClipPath(path)
                    drawn_times: set[float] = set()
                    for kfs in clip.keyframes.values():
                        for kf in kfs:
                            t_rounded = round(kf.time, 3)
                            if t_rounded in drawn_times:
                                continue
                            drawn_times.add(t_rounded)
                            kx = clip_rect.left() + kf.time * self.pixels_per_second
                            ky = clip_rect.bottom() - 7
                            diamond = QPainterPath()
                            diamond.moveTo(kx, ky - 4)
                            diamond.lineTo(kx + 4, ky)
                            diamond.lineTo(kx, ky + 4)
                            diamond.lineTo(kx - 4, ky)
                            diamond.closeSubpath()
                            painter.fillPath(diamond, QBrush(QColor(224, 122, 56)))
                            painter.setPen(QPen(QColor(20, 20, 25), 0.8))
                            painter.drawPath(diamond)
                    painter.restore()

        # Draw vertical marker guidelines across canvas
        for marker in self.timeline.markers:
            mx = int(marker.time * self.pixels_per_second)
            if 0 <= mx <= self.width():
                m_color = QColor(marker.color)
                m_color.setAlpha(120)
                painter.setPen(QPen(m_color, 1.0, Qt.PenStyle.DashLine))
                painter.drawLine(mx, 0, mx, self.height())

        # Draw vertical Playhead line across the entire canvas height
        ph_x = self._current_time * self.pixels_per_second
        painter.setPen(QPen(QColor(224, 122, 56), 2.0))
        painter.drawLine(int(ph_x), 0, int(ph_x), self.height())

        # Draw magnetic snapping visual guide line
        if self._snap_guide_x is not None:
            painter.setPen(QPen(QColor(224, 122, 56), 1.5, Qt.PenStyle.DashLine))
            painter.drawLine(int(self._snap_guide_x), 0, int(self._snap_guide_x), self.height())

        # Draw rubberband rectangle
        if self._rubberband_rect and not self._rubberband_rect.isEmpty():
            painter.fillRect(self._rubberband_rect, QColor(224, 122, 56, 45))
            painter.setPen(QPen(QColor(224, 122, 56, 200), 1.0, Qt.PenStyle.DashLine))
            painter.drawRect(self._rubberband_rect)
