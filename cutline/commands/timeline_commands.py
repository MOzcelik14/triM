from __future__ import annotations

from typing import Optional

from PySide6.QtGui import QUndoCommand

from cutline.core.clip import Clip
from cutline.core.timeline import TimelineModel


class AddClipCommand(QUndoCommand):
    """Command to add a clip to a track."""

    def __init__(
        self,
        timeline: TimelineModel,
        track_id: str,
        clip: Clip,
        description: str = "Klip Ekle",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.track_id = track_id
        self.clip = clip

    def redo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            track.add_clip(self.clip, allow_overlap=True)
            self.timeline.notify_clip_added(self.track_id, self.clip.id)

    def undo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            track.remove_clip(self.clip.id)
            self.timeline.notify_clip_removed(self.track_id, self.clip.id)


class RemoveClipCommand(QUndoCommand):
    """Command to remove a clip from a track."""

    def __init__(
        self,
        timeline: TimelineModel,
        track_id: str,
        clip_id: str,
        description: str = "Klip Sil",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.track_id = track_id
        self.clip_id = clip_id
        self._saved_clip: Optional[Clip] = None

    def redo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            clip = track.get_clip_by_id(self.clip_id)
            if clip:
                self._saved_clip = clip.clone(new_id=False)
            track.remove_clip(self.clip_id)
            self.timeline.notify_clip_removed(self.track_id, self.clip_id)

    def undo(self) -> None:
        if self._saved_clip:
            track = self.timeline.get_track_by_id(self.track_id)
            if track:
                track.add_clip(self._saved_clip, allow_overlap=True)
                self.timeline.notify_clip_added(self.track_id, self._saved_clip.id)


class MoveClipCommand(QUndoCommand):
    """Command to move a clip to a new start time."""

    def __init__(
        self,
        timeline: TimelineModel,
        track_id: str,
        clip_id: str,
        old_time: float,
        new_time: float,
        description: str = "Klip Taşı",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.track_id = track_id
        self.clip_id = clip_id
        self.old_time = old_time
        self.new_time = new_time

    def redo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            clip = track.get_clip_by_id(self.clip_id)
            if clip:
                clip.move_to(self.new_time)
                track.sort_clips()
                self.timeline.notify_clip_modified(self.track_id, self.clip_id)

    def undo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            clip = track.get_clip_by_id(self.clip_id)
            if clip:
                clip.move_to(self.old_time)
                track.sort_clips()
                self.timeline.notify_clip_modified(self.track_id, self.clip_id)


class TrimClipCommand(QUndoCommand):
    """Command to modify clip boundaries (in/out points)."""

    def __init__(
        self,
        timeline: TimelineModel,
        track_id: str,
        clip_id: str,
        old_bounds: tuple[float, float, float, float],  # (tl_in, tl_out, src_in, src_out)
        new_bounds: tuple[float, float, float, float],
        description: str = "Klip Kırp",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.track_id = track_id
        self.clip_id = clip_id
        self.old_bounds = old_bounds
        self.new_bounds = new_bounds

    def _apply(self, bounds: tuple[float, float, float, float]) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            clip = track.get_clip_by_id(self.clip_id)
            if clip:
                clip.timeline_in = bounds[0]
                clip.timeline_out = bounds[1]
                clip.source_in = bounds[2]
                clip.source_out = bounds[3]
                track.sort_clips()
                self.timeline.notify_clip_modified(self.track_id, self.clip_id)

    def redo(self) -> None:
        self._apply(self.new_bounds)

    def undo(self) -> None:
        self._apply(self.old_bounds)


class SplitClipCommand(QUndoCommand):
    """Command to cut a clip into two at a given timeline time."""

    def __init__(
        self,
        timeline: TimelineModel,
        track_id: str,
        clip_id: str,
        split_time: float,
        description: str = "Klip Böl",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.track_id = track_id
        self.clip_id = clip_id
        self.split_time = split_time
        self._original_clip: Optional[Clip] = None
        self._right_clip_id: Optional[str] = None

    def redo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            clip = track.get_clip_by_id(self.clip_id)
            if clip:
                self._original_clip = clip.clone(new_id=False)
                left, right = track.split_clip(self.clip_id, self.split_time)
                self._right_clip_id = right.id
                self.timeline.notify_clip_modified(self.track_id, left.id)
                self.timeline.notify_clip_added(self.track_id, right.id)

    def undo(self) -> None:
        if self._original_clip:
            track = self.timeline.get_track_by_id(self.track_id)
            if track:
                if self._right_clip_id:
                    track.remove_clip(self._right_clip_id)
                    self.timeline.notify_clip_removed(self.track_id, self._right_clip_id)

                # Restore original clip parameters
                clip = track.get_clip_by_id(self._original_clip.id)
                if clip:
                    clip.timeline_in = self._original_clip.timeline_in
                    clip.timeline_out = self._original_clip.timeline_out
                    clip.source_in = self._original_clip.source_in
                    clip.source_out = self._original_clip.source_out
                    track.sort_clips()
                    self.timeline.notify_clip_modified(self.track_id, clip.id)


class RippleDeleteCommand(QUndoCommand):
    """Command to delete a clip and shift following clips left."""

    def __init__(
        self,
        timeline: TimelineModel,
        track_id: str,
        clip_id: str,
        description: str = "Boşluksuz Sil",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.track_id = track_id
        self.clip_id = clip_id
        self._saved_clips_snapshot: list[Clip] = []

    def redo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            self._saved_clips_snapshot = [c.clone(new_id=False) for c in track.clips]
            track.ripple_delete(self.clip_id)
            self.timeline.notify_clip_removed(self.track_id, self.clip_id)

    def undo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            track.clips = [c.clone(new_id=False) for c in self._saved_clips_snapshot]
            track.sort_clips()
            self.timeline.tracks_changed.emit()
            self.timeline.duration_changed.emit(self.timeline.duration)
