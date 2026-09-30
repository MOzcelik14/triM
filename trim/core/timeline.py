from __future__ import annotations

import math
from typing import Any, Optional
import uuid

from PySide6.QtCore import QObject, Signal

from .clip import Clip
from .track import Track, TrackType


class TimelineModel(QObject):
    """Core timeline data model maintaining tracks, timebase, and change signals."""

    clip_added = Signal(str, str)  # track_id, clip_id
    clip_removed = Signal(str, str)  # track_id, clip_id
    clip_modified = Signal(str, str)  # track_id, clip_id
    tracks_changed = Signal()
    duration_changed = Signal(float)

    def __init__(
        self,
        fps: float = 30.0,
        width: int = 1920,
        height: int = 1080,
        parent: Optional[QObject] = None,
    ) -> None:
        super().__init__(parent)
        self.fps = fps
        self.width = width
        self.height = height
        self.tracks: list[Track] = []

    @property
    def duration(self) -> float:
        if not self.tracks:
            return 0.0
        return max((t.duration for t in self.tracks), default=0.0)

    def add_track(self, track: Track) -> None:
        self.tracks.append(track)
        self.tracks_changed.emit()
        self.duration_changed.emit(self.duration)

    def remove_track(self, track_id: str) -> Optional[Track]:
        for i, t in enumerate(self.tracks):
            if t.id == track_id:
                removed = self.tracks.pop(i)
                self.tracks_changed.emit()
                self.duration_changed.emit(self.duration)
                return removed
        return None

    def get_track_by_id(self, track_id: str) -> Optional[Track]:
        for t in self.tracks:
            if t.id == track_id:
                return t
        return None

    def find_track_for_clip(self, clip_id: str) -> Optional[Track]:
        for t in self.tracks:
            if t.get_clip_by_id(clip_id) is not None:
                return t
        return None

    def get_clip_by_id(self, clip_id: str) -> Optional[Clip]:
        for t in self.tracks:
            c = t.get_clip_by_id(clip_id)
            if c is not None:
                return c
        return None

    def get_video_tracks(self) -> list[Track]:
        return [t for t in self.tracks if t.track_type == TrackType.VIDEO]

    def get_audio_tracks(self) -> list[Track]:
        return [t for t in self.tracks if t.track_type == TrackType.AUDIO]

    def get_top_visible_video_clip_at(self, time: float) -> Optional[tuple[Track, Clip]]:
        """Returns the highest-priority visible video track and clip at timestamp."""
        for track in self.get_video_tracks():
            if not track.visible:
                continue
            clip = track.find_clip_at(time)
            if clip:
                return track, clip
        return None

    def get_visible_video_clips_at(self, time: float) -> list[tuple[Track, Clip]]:
        """Returns all visible video clips at timestamp in bottom-to-top rendering order."""
        clips = []
        for track in reversed(self.get_video_tracks()):
            if not track.visible:
                continue
            clip = track.find_clip_at(time)
            if clip:
                clips.append((track, clip))
        return clips

    def get_active_audio_clips_at(self, time: float) -> list[tuple[Track, Clip]]:
        """Returns all unmuted audio/video clips with audio at timestamp."""
        clips = []
        for track in self.tracks:
            if track.muted:
                continue
            clip = track.find_clip_at(time)
            if clip and not clip.muted:
                clips.append((track, clip))
        return clips

    def snap_time(
        self,
        target_time: float,
        threshold: float = 0.15,
        ignore_clip_id: Optional[str] = None,
    ) -> float:
        """Finds closest snap point (clip edges, 0.0) within threshold."""
        snap_points = [0.0]
        for track in self.tracks:
            for clip in track.clips:
                if ignore_clip_id and clip.id == ignore_clip_id:
                    continue
                snap_points.append(clip.timeline_in)
                snap_points.append(clip.timeline_out)

        closest = target_time
        min_dist = threshold
        for sp in snap_points:
            dist = abs(target_time - sp)
            if dist < min_dist:
                min_dist = dist
                closest = sp

        return closest

    def time_to_frame(self, time: float) -> int:
        return max(0, int(round(time * self.fps)))

    def frame_to_time(self, frame: int) -> float:
        return max(0.0, frame / self.fps)

    def time_to_timecode(self, time: float) -> str:
        """Formats time into HH:MM:SS:FF."""
        total_frames = self.time_to_frame(time)
        fps_int = int(round(self.fps)) or 30

        frames = total_frames % fps_int
        total_seconds = total_frames // fps_int
        seconds = total_seconds % 60
        total_minutes = total_seconds // 60
        minutes = total_minutes % 60
        hours = total_minutes // 60

        return f"{hours:02d}:{minutes:02d}:{seconds:02d}:{frames:02d}"

    def timecode_to_time(self, timecode: str) -> float:
        """Parses HH:MM:SS:FF into seconds."""
        parts = timecode.strip().split(":")
        if len(parts) != 4:
            raise ValueError(f"Invalid timecode format: {timecode}. Expected HH:MM:SS:FF")
        h, m, s, f = map(int, parts)
        fps_val = self.fps if self.fps > 0 else 30.0
        total_seconds = (h * 3600) + (m * 60) + s + (f / fps_val)
        return max(0.0, total_seconds)

    def notify_clip_added(self, track_id: str, clip_id: str) -> None:
        self.clip_added.emit(track_id, clip_id)
        self.duration_changed.emit(self.duration)

    def notify_clip_removed(self, track_id: str, clip_id: str) -> None:
        self.clip_removed.emit(track_id, clip_id)
        self.duration_changed.emit(self.duration)

    def notify_clip_modified(self, track_id: str, clip_id: str) -> None:
        self.clip_modified.emit(track_id, clip_id)
        self.duration_changed.emit(self.duration)

    def to_dict(self) -> dict[str, Any]:
        return {
            "fps": self.fps,
            "width": self.width,
            "height": self.height,
            "tracks": [t.to_dict() for t in self.tracks],
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any], parent: Optional[QObject] = None) -> TimelineModel:
        model = cls(
            fps=float(data.get("fps", 30.0)),
            width=int(data.get("width", 1920)),
            height=int(data.get("height", 1080)),
            parent=parent,
        )
        for td in data.get("tracks", []):
            model.tracks.append(Track.from_dict(td))
        return model
