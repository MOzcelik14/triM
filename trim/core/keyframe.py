from __future__ import annotations

from dataclasses import dataclass
import math
from typing import Any


@dataclass
class Keyframe:
    """A parameter keyframe at a specific timestamp relative to clip start."""

    time: float  # seconds relative to clip in-point (0.0 <= time <= clip.duration)
    value: float
    interpolation: str = "linear"  # "linear", "hold", "ease_in_out"

    def to_dict(self) -> dict[str, Any]:
        return {
            "time": self.time,
            "value": self.value,
            "interpolation": self.interpolation,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> Keyframe:
        return cls(
            time=float(data["time"]),
            value=float(data["value"]),
            interpolation=str(data.get("interpolation", "linear")),
        )


def interpolate_keyframes(
    keyframes: list[Keyframe],
    clip_time: float,
    default_value: float = 0.0,
) -> float:
    """Evaluates the animated parameter value at clip_time given a list of keyframes."""
    if not keyframes:
        return default_value

    sorted_kfs = sorted(keyframes, key=lambda k: k.time)

    # Boundary conditions
    if clip_time <= sorted_kfs[0].time:
        return sorted_kfs[0].value
    if clip_time >= sorted_kfs[-1].time:
        return sorted_kfs[-1].value

    # Find interval [k1, k2]
    k1 = sorted_kfs[0]
    k2 = sorted_kfs[-1]
    for i in range(len(sorted_kfs) - 1):
        if sorted_kfs[i].time <= clip_time <= sorted_kfs[i + 1].time:
            k1 = sorted_kfs[i]
            k2 = sorted_kfs[i + 1]
            break

    dt = k2.time - k1.time
    if abs(dt) < 1e-6:
        return k1.value

    u = max(0.0, min(1.0, (clip_time - k1.time) / dt))

    if k1.interpolation == "hold":
        return k1.value
    elif k1.interpolation == "ease_in_out":
        # Smoothstep curve: 3u^2 - 2u^3
        u_smooth = u * u * (3.0 - 2.0 * u)
        return k1.value + u_smooth * (k2.value - k1.value)
    else:
        # Standard linear interpolation
        return k1.value + u * (k2.value - k1.value)
