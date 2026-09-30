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
from PySide6.QtWidgets import QWidget

from cutline.commands.timeline_commands import (
    AddClipCommand,
    MoveClipCommand,
    RemoveClipCommand,
    RippleDeleteCommand,
    SplitClipCommand,
    TrimClipCommand,
)
from cutline.core.clip import Clip
from cutline.core.project import Project
from cutline.core.timeline import TimelineModel
from cutline.core.track import Track, TrackType
from cutline.ui.project_bin.media_bin_widget import MIME_MEDIA_ID

logger = logging.getLogger(__name__)


class DragMode(Enum):
    NONE = 0
    MOVE = 1
    TRIM_IN = 2
    TRIM_OUT = 3
    SCRUB = 4


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

        self._current_time: float = 0.0
        self._selected_track_id: Optional[str] = None
        self._selected_clip_id: Optional[str] = None

        self._drag_mode = DragMode.NONE
        self._drag_clip: Optional[Clip] = None
        self._drag_track: Optional[Track] = None
        self._drag_start_x: float = 0.0
        self._drag_orig_bounds: tuple[float, float, float, float] = (0.0, 0.0, 0.0, 0.0)

        self.setAcceptDrops(True)
        self.setMouseTracking(True)
        self.setFocusPolicy(Qt.FocusPolicy.StrongFocus)

        # Connect model signals
        self.timeline.clip_added.connect(self._on_model_changed)
        self.timeline.clip_removed.connect(self._on_model_changed)
        self.timeline.clip_modified.connect(self._on_model_changed)
        self.timeline.tracks_changed.connect(self._on_model_changed)

        self._update_geometry()

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

        if clip and track:
            self._selected_track_id = track.id
            self._selected_clip_id = clip.id
            self.clip_selected.emit(track.id, clip.id)

            self._drag_mode = mode
            self._drag_clip = clip
            self._drag_track = track
            self._drag_start_x = pos.x()
            assert clip.source_out is not None
            self._drag_orig_bounds = (clip.timeline_in, clip.timeline_out, clip.source_in, clip.source_out)
        else:
            self._selected_clip_id = None
            self._drag_mode = DragMode.SCRUB
            t = max(0.0, pos.x() / self.pixels_per_second)
            snapped = self.timeline.snap_time(t, threshold=0.1)
            self.seek_requested.emit(snapped)

        self.update()

    def mouseMoveEvent(self, event: QMouseEvent) -> None:
        pos = event.position()

        # Update cursor when hovering
        if self._drag_mode == DragMode.NONE:
            _, clip, mode = self._find_clip_at_pos(pos)
            if mode in (DragMode.TRIM_IN, DragMode.TRIM_OUT):
                self.setCursor(Qt.CursorShape.SizeHorCursor)
            elif mode == DragMode.MOVE:
                self.setCursor(Qt.CursorShape.SizeAllCursor)
            else:
                self.setCursor(Qt.CursorShape.ArrowCursor)
            return

        # Handle active drag
        dx = pos.x() - self._drag_start_x
        dt = dx / self.pixels_per_second

        if self._drag_mode == DragMode.SCRUB:
            t = max(0.0, pos.x() / self.pixels_per_second)
            snapped = self.timeline.snap_time(t, threshold=0.1)
            self.seek_requested.emit(snapped)

        elif self._drag_mode == DragMode.MOVE and self._drag_clip and self._drag_track:
            orig_in = self._drag_orig_bounds[0]
            target_in = max(0.0, orig_in + dt)
            snapped_in = self.timeline.snap_time(target_in, threshold=0.1, ignore_clip_id=self._drag_clip.id)
            self._drag_clip.move_to(snapped_in)
            self.update()

        elif self._drag_mode == DragMode.TRIM_IN and self._drag_clip:
            orig_in = self._drag_orig_bounds[0]
            target_in = max(0.0, orig_in + dt)
            snapped_in = self.timeline.snap_time(target_in, threshold=0.1, ignore_clip_id=self._drag_clip.id)
            self._drag_clip.trim_in(snapped_in)
            self.update()

        elif self._drag_mode == DragMode.TRIM_OUT and self._drag_clip:
            orig_out = self._drag_orig_bounds[1]
            target_out = max(self._drag_clip.timeline_in + 0.04, orig_out + dt)
            snapped_out = self.timeline.snap_time(target_out, threshold=0.1, ignore_clip_id=self._drag_clip.id)
            self._drag_clip.trim_out(snapped_out)
            self.update()

    def mouseReleaseEvent(self, event: QMouseEvent) -> None:
        if event.button() != Qt.MouseButton.LeftButton:
            return super().mouseReleaseEvent(event)

        if self._drag_clip and self._drag_track:
            assert self._drag_clip.source_out is not None
            new_bounds = (
                self._drag_clip.timeline_in,
                self._drag_clip.timeline_out,
                self._drag_clip.source_in,
                self._drag_clip.source_out,
            )

            if self._drag_mode == DragMode.MOVE:
                if abs(new_bounds[0] - self._drag_orig_bounds[0]) > 1e-4:
                    cmd = MoveClipCommand(
                        self.timeline,
                        self._drag_track.id,
                        self._drag_clip.id,
                        self._drag_orig_bounds[0],
                        new_bounds[0],
                    )
                    self.undo_stack.push(cmd)
                    self.project.mark_dirty()

            elif self._drag_mode in (DragMode.TRIM_IN, DragMode.TRIM_OUT):
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

        self._drag_mode = DragMode.NONE
        self._drag_clip = None
        self._drag_track = None
        self.setCursor(Qt.CursorShape.ArrowCursor)
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
        if not self._selected_track_id or not self._selected_clip_id:
            return

        cmd = RemoveClipCommand(self.timeline, self._selected_track_id, self._selected_clip_id)
        self.undo_stack.push(cmd)
        self.project.mark_dirty()
        self._selected_clip_id = None
        self.update()

    def ripple_delete_selected(self) -> None:
        if not self._selected_track_id or not self._selected_clip_id:
            return

        cmd = RippleDeleteCommand(self.timeline, self._selected_track_id, self._selected_clip_id)
        self.undo_stack.push(cmd)
        self.project.mark_dirty()
        self._selected_clip_id = None
        self.update()

    def paintEvent(self, event) -> None:
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing, True)

        # Background
        painter.fillRect(self.rect(), QColor(22, 22, 26))

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
                is_selected = clip.id == self._selected_clip_id

                path = QPainterPath()
                path.addRoundedRect(clip_rect, 4, 4)

                # Base clip fill
                if track.track_type == TrackType.VIDEO:
                    base_color = QColor(42, 85, 128) if not is_selected else QColor(50, 105, 160)
                else:
                    base_color = QColor(36, 100, 70) if not is_selected else QColor(45, 125, 88)

                painter.fillPath(path, QBrush(base_color))

                # Clip outline
                if is_selected:
                    painter.setPen(QPen(QColor(0, 210, 255), 2.0))
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
                painter.drawText(
                    text_rect,
                    Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignTop,
                    f"{display_name}  [{dur_str}]",
                )

        # Draw vertical Playhead line across the entire canvas height
        ph_x = self._current_time * self.pixels_per_second
        painter.setPen(QPen(QColor(230, 57, 70), 2.0))
        painter.drawLine(int(ph_x), 0, int(ph_x), self.height())
