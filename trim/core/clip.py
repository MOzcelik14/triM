from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Optional
import uuid

from .keyframe import Keyframe, interpolate_keyframes


@dataclass
class Clip:
    media_id: str
    timeline_in: float
    timeline_out: float
    source_in: float = 0.0
    source_out: Optional[float] = None
    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    name: str = ""
    # Speed and direction properties
    speed: float = 1.0  # 0.1x to 10.0x
    reverse: bool = False
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
    # Animated keyframes dictionary: property_name -> list[Keyframe]
    keyframes: dict[str, list[Keyframe]] = field(default_factory=dict)

    def __post_init__(self) -> None:
        duration = max(0.0, self.timeline_out - self.timeline_in)
        if self.source_out is None:
            self.source_out = self.source_in + duration * max(0.01, self.speed)
        else:
            # Ensure duration consistency with speed
            expected_source_duration = self.source_out - self.source_in
            expected_timeline_duration = max(0.0, expected_source_duration / max(0.01, self.speed))
            if abs(expected_timeline_duration - duration) > 1e-4:
                self.timeline_out = self.timeline_in + expected_timeline_duration

    @property
    def duration(self) -> float:
        return max(0.0, self.timeline_out - self.timeline_in)

    def contains_timeline_time(self, time: float) -> bool:
        """Returns True if time falls within [timeline_in, timeline_out)."""
        return self.timeline_in <= time < self.timeline_out

    def map_timeline_to_source(self, timeline_time: float) -> float:
        """Translates timeline time to media source time taking speed and reverse into account."""
        offset = max(0.0, timeline_time - self.timeline_in)
        if self.reverse:
            src_out = self.source_out if self.source_out is not None else (self.source_in + self.duration * self.speed)
            return max(self.source_in, src_out - offset * self.speed)
        return max(0.0, self.source_in + offset * self.speed)

    def add_keyframe(
        self,
        prop: str,
        time: float,
        value: float,
        interpolation: str = "linear",
    ) -> Keyframe:
        """Adds or updates a keyframe for the given property at clip-relative time."""
        if prop not in self.keyframes:
            self.keyframes[prop] = []
        self.keyframes[prop] = [k for k in self.keyframes[prop] if abs(k.time - time) >= 0.03]
        kf = Keyframe(time=round(time, 4), value=value, interpolation=interpolation)
        self.keyframes[prop].append(kf)
        self.keyframes[prop].sort(key=lambda k: k.time)
        return kf

    def remove_keyframe(self, prop: str, time: float, tolerance: float = 0.05) -> bool:
        """Removes keyframe near time. Returns True if a keyframe was removed."""
        if prop not in self.keyframes:
            return False
        orig_len = len(self.keyframes[prop])
        self.keyframes[prop] = [k for k in self.keyframes[prop] if abs(k.time - time) > tolerance]
        return len(self.keyframes[prop]) < orig_len

    def get_property_at_time(self, prop: str, clip_time: float) -> float:
        """Evaluates animated or static property at clip-relative time."""
        default_val = float(getattr(self, prop, 0.0))
        if prop not in self.keyframes or not self.keyframes[prop]:
            return default_val
        return interpolate_keyframes(self.keyframes[prop], clip_time, default_val)

    def has_keyframes(self, prop: Optional[str] = None) -> bool:
        """Returns True if clip has animated keyframes (for prop or any property)."""
        if prop is not None:
            return bool(self.keyframes.get(prop))
        return any(bool(kfs) for kfs in self.keyframes.values())

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
        cloned_kfs: dict[str, list[Keyframe]] = {}
        for prop, kfs in self.keyframes.items():
            cloned_kfs[prop] = [Keyframe(time=k.time, value=k.value, interpolation=k.interpolation) for k in kfs]

        return Clip(
            media_id=self.media_id,
            timeline_in=self.timeline_in,
            timeline_out=self.timeline_out,
            source_in=self.source_in,
            source_out=self.source_out,
            id=str(uuid.uuid4()) if new_id else self.id,
            name=self.name,
            speed=self.speed,
            reverse=self.reverse,
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
            keyframes=cloned_kfs,
        )

    def to_dict(self) -> dict[str, Any]:
        kfs_dict: dict[str, list[dict[str, Any]]] = {}
        for prop, kfs in self.keyframes.items():
            kfs_dict[prop] = [k.to_dict() for k in kfs]

        return {
            "id": self.id,
            "media_id": self.media_id,
            "name": self.name,
            "speed": self.speed,
            "reverse": self.reverse,
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
            "keyframes": kfs_dict,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> Clip:
        loaded_kfs: dict[str, list[Keyframe]] = {}
        for prop, kfs_data in data.get("keyframes", {}).items():
            loaded_kfs[prop] = [Keyframe.from_dict(kd) for kd in kfs_data]

        return cls(
            id=data.get("id", str(uuid.uuid4())),
            media_id=data["media_id"],
            name=data.get("name", ""),
            speed=float(data.get("speed", 1.0)),
            reverse=bool(data.get("reverse", False)),
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
            keyframes=loaded_kfs,
        )
