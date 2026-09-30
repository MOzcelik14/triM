from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Optional
import uuid


@dataclass
class Clip:
    media_id: str
    timeline_in: float
    timeline_out: float
    source_in: float = 0.0
    source_out: Optional[float] = None
    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    name: str = ""
    # Audio properties
    volume: float = 1.0
    muted: bool = False
    fade_in: float = 0.0
    fade_out: float = 0.0
    transition_in: str = "dip_black"  # "dip_black", "dip_white", "cross_dissolve"
    transition_out: str = "dip_black"
    # Visual transform properties
    opacity: float = 1.0
    scale: float = 1.0
    pos_x: float = 0.0
    pos_y: float = 0.0
    rotation: float = 0.0
    # Color adjustment properties
    brightness: float = 0.0  # -1.0 to 1.0
    contrast: float = 1.0    # 0.0 to 3.0
    saturation: float = 1.0  # 0.0 to 3.0 (0.0 = B&W)

    def __post_init__(self) -> None:
        duration = max(0.0, self.timeline_out - self.timeline_in)
        if self.source_out is None:
            self.source_out = self.source_in + duration
        else:
            # Ensure duration consistency
            expected_source_duration = self.source_out - self.source_in
            if abs(expected_source_duration - duration) > 1e-5:
                self.timeline_out = self.timeline_in + max(0.0, expected_source_duration)

    @property
    def duration(self) -> float:
        return max(0.0, self.timeline_out - self.timeline_in)

    def contains_timeline_time(self, time: float) -> bool:
        """Returns True if time falls within [timeline_in, timeline_out)."""
        return self.timeline_in <= time < self.timeline_out

    def map_timeline_to_source(self, timeline_time: float) -> float:
        """Translates timeline time to media source time."""
        offset = timeline_time - self.timeline_in
        return max(0.0, self.source_in + offset)

    def move_to(self, new_timeline_in: float) -> None:
        """Moves clip on timeline preserving duration and source boundaries."""
        cur_dur = self.duration
        self.timeline_in = max(0.0, new_timeline_in)
        self.timeline_out = self.timeline_in + cur_dur

    def trim_in(self, new_timeline_in: float, min_duration: float = 0.04) -> None:
        """Trims clip from the left edge."""
        new_timeline_in = max(0.0, new_timeline_in)
        # Cannot trim past timeline_out - min_duration
        if new_timeline_in > self.timeline_out - min_duration:
            new_timeline_in = self.timeline_out - min_duration

        delta = new_timeline_in - self.timeline_in
        new_source_in = self.source_in + delta
        if new_source_in < 0.0:
            delta = -self.source_in
            new_timeline_in = self.timeline_in + delta
            new_source_in = 0.0

        self.timeline_in = new_timeline_in
        self.source_in = new_source_in

    def trim_out(self, new_timeline_out: float, max_source_duration: Optional[float] = None, min_duration: float = 0.04) -> None:
        """Trims clip from the right edge."""
        if new_timeline_out < self.timeline_in + min_duration:
            new_timeline_out = self.timeline_in + min_duration

        new_duration = new_timeline_out - self.timeline_in
        if max_source_duration is not None:
            max_dur = max_source_duration - self.source_in
            if new_duration > max_dur:
                new_duration = max(min_duration, max_dur)
                new_timeline_out = self.timeline_in + new_duration

        self.timeline_out = new_timeline_out
        assert self.source_out is not None
        self.source_out = self.source_in + new_duration

    def split(self, split_time: float) -> tuple[Clip, Clip]:
        """Splits this clip into two clips at split_time."""
        if not (self.timeline_in < split_time < self.timeline_out):
            raise ValueError(f"Split time {split_time} must be strictly within [{self.timeline_in}, {self.timeline_out}]")

        split_offset = split_time - self.timeline_in
        split_source_time = self.source_in + split_offset

        left_clip = self.clone(new_id=False)
        left_clip.timeline_out = split_time
        left_clip.source_out = split_source_time

        right_clip = self.clone(new_id=True)
        right_clip.timeline_in = split_time
        right_clip.timeline_out = self.timeline_out
        right_clip.source_in = split_source_time
        right_clip.source_out = self.source_out

        return left_clip, right_clip

    def clone(self, new_id: bool = True) -> Clip:
        return Clip(
            media_id=self.media_id,
            timeline_in=self.timeline_in,
            timeline_out=self.timeline_out,
            source_in=self.source_in,
            source_out=self.source_out,
            id=str(uuid.uuid4()) if new_id else self.id,
            name=self.name,
            volume=self.volume,
            muted=self.muted,
            fade_in=self.fade_in,
            fade_out=self.fade_out,
            transition_in=self.transition_in,
            transition_out=self.transition_out,
            opacity=self.opacity,
            scale=self.scale,
            pos_x=self.pos_x,
            pos_y=self.pos_y,
            rotation=self.rotation,
            brightness=self.brightness,
            contrast=self.contrast,
            saturation=self.saturation,
        )

    def to_dict(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "media_id": self.media_id,
            "name": self.name,
            "timeline_in": self.timeline_in,
            "timeline_out": self.timeline_out,
            "source_in": self.source_in,
            "source_out": self.source_out,
            "volume": self.volume,
            "muted": self.muted,
            "fade_in": self.fade_in,
            "fade_out": self.fade_out,
            "transition_in": self.transition_in,
            "transition_out": self.transition_out,
            "opacity": self.opacity,
            "scale": self.scale,
            "pos_x": self.pos_x,
            "pos_y": self.pos_y,
            "rotation": self.rotation,
            "brightness": self.brightness,
            "contrast": self.contrast,
            "saturation": self.saturation,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> Clip:
        return cls(
            id=data.get("id", str(uuid.uuid4())),
            media_id=data["media_id"],
            name=data.get("name", ""),
            timeline_in=float(data["timeline_in"]),
            timeline_out=float(data["timeline_out"]),
            source_in=float(data.get("source_in", 0.0)),
            source_out=float(data["source_out"]) if data.get("source_out") is not None else None,
            volume=float(data.get("volume", 1.0)),
            muted=bool(data.get("muted", False)),
            fade_in=float(data.get("fade_in", 0.0)),
            fade_out=float(data.get("fade_out", 0.0)),
            transition_in=str(data.get("transition_in", "dip_black")),
            transition_out=str(data.get("transition_out", "dip_black")),
            opacity=float(data.get("opacity", 1.0)),
            scale=float(data.get("scale", 1.0)),
            pos_x=float(data.get("pos_x", 0.0)),
            pos_y=float(data.get("pos_y", 0.0)),
            rotation=float(data.get("rotation", 0.0)),
            brightness=float(data.get("brightness", 0.0)),
            contrast=float(data.get("contrast", 1.0)),
            saturation=float(data.get("saturation", 1.0)),
        )
