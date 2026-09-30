from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any
import uuid


@dataclass
class Marker:
    time: float
    name: str = ""
    color: str = "#E07A38"  # Default Precision Amber
    id: str = field(default_factory=lambda: str(uuid.uuid4()))

    def to_dict(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "time": self.time,
            "name": self.name,
            "color": self.color,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> Marker:
        return cls(
            id=data.get("id", str(uuid.uuid4())),
            time=float(data["time"]),
            name=data.get("name", ""),
            color=data.get("color", "#E07A38"),
        )
