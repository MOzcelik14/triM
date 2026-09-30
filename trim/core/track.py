from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Optional
import uuid

from .clip import Clip


class TrackType(str, Enum):
    VIDEO = "video"
    AUDIO = "audio"


@dataclass
class Track:
    name: str
    track_type: TrackType = TrackType.VIDEO
    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    clips: list[Clip] = field(default_factory=list)
    muted: bool = False
    solo: bool = False
    locked: bool = False
    visible: bool = True
    volume: float = 1.0  # 0.0 to 1.5 gain multiplier

    def __post_init__(self) -> None:
        if isinstance(self.track_type, str) and not isinstance(self.track_type, TrackType):
            self.track_type = TrackType(self.track_type)
        self.sort_clips()

    def sort_clips(self) -> None:
        self.clips.sort(key=lambda c: c.timeline_in)

    @property
    def duration(self) -> float:
        if not self.clips:
            return 0.0
        return max(c.timeline_out for c in self.clips)

    def get_clip_by_id(self, clip_id: str) -> Optional[Clip]:
        for clip in self.clips:
            if clip.id == clip_id:
                return clip
        return None

    def find_clip_at(self, time: float) -> Optional[Clip]:
        for clip in self.clips:
            if clip.contains_timeline_time(time):
                return clip
        return None

    def check_collision(self, clip: Clip, ignore_clip_id: Optional[str] = None) -> bool:
        """Returns True if clip overlaps with any existing clip on this track."""
        for other in self.clips:
            if ignore_clip_id and other.id == ignore_clip_id:
                continue
            # Overlap exists if max(start1, start2) < min(end1, end2)
            if max(clip.timeline_in, other.timeline_in) < min(clip.timeline_out, other.timeline_out) - 1e-4:
                return True
        return False

    def add_clip(self, clip: Clip, allow_overlap: bool = False) -> bool:
        if self.locked:
            return False
        if not allow_overlap and self.check_collision(clip):
            return False
        self.clips.append(clip)
        self.sort_clips()
        return True

    def remove_clip(self, clip_id: str) -> Optional[Clip]:
        if self.locked:
            return None
        for i, clip in enumerate(self.clips):
            if clip.id == clip_id:
                removed = self.clips.pop(i)
                return removed
        return None

    def split_clip(self, clip_id: str, split_time: float) -> tuple[Clip, Clip]:
        if self.locked:
            raise RuntimeError(f"Track '{self.name}' is locked")
        clip = self.get_clip_by_id(clip_id)
        if not clip:
            raise ValueError(f"Clip '{clip_id}' not found on track")

        left, right = clip.split(split_time)
        # Replace original with left, and append right
        idx = self.clips.index(clip)
        self.clips[idx] = left
        self.clips.insert(idx + 1, right)
        self.sort_clips()
        return left, right

    def ripple_delete(self, clip_id: str) -> Optional[Clip]:
        """Removes the clip and shifts later clips to the left by its duration."""
        if self.locked:
            return None
        clip = self.get_clip_by_id(clip_id)
        if not clip:
            return None

        idx = self.clips.index(clip)
        deleted = self.clips.pop(idx)
        shift = deleted.duration

        for later in self.clips[idx:]:
            later.move_to(later.timeline_in - shift)

        self.sort_clips()
        return deleted

    def to_dict(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "name": self.name,
            "track_type": self.track_type.value,
            "muted": self.muted,
            "solo": self.solo,
            "locked": self.locked,
            "visible": self.visible,
            "volume": self.volume,
            "clips": [c.to_dict() for c in self.clips],
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> Track:
        track = cls(
            id=data.get("id", str(uuid.uuid4())),
            name=data.get("name", "Track"),
            track_type=TrackType(data.get("track_type", "video")),
            muted=bool(data.get("muted", False)),
            solo=bool(data.get("solo", False)),
            locked=bool(data.get("locked", False)),
            visible=bool(data.get("visible", True)),
            volume=float(data.get("volume", 1.0)),
        )
        track.clips = [Clip.from_dict(cd) for cd in data.get("clips", [])]
        track.sort_clips()
        return track
