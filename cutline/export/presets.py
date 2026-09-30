from __future__ import annotations

from dataclasses import dataclass
from typing import Optional


@dataclass
class ExportPreset:
    name: str
    description: str
    container: str = "mp4"
    video_codec: str = "libx264"
    audio_codec: str = "aac"
    width: Optional[int] = 1920
    height: Optional[int] = 1080
    fps: Optional[float] = 30.0
    crf: int = 21
    preset: str = "medium"
    audio_bitrate: str = "192k"


DEFAULT_PRESETS = [
    ExportPreset(
        name="1080p H.264 (Standard MP4)",
        description="High quality 1080p MP4 suitable for YouTube, Web, and Archiving.",
        width=1920,
        height=1080,
        fps=30.0,
        crf=20,
        preset="medium",
        video_codec="libx264",
        audio_codec="aac",
    ),
    ExportPreset(
        name="720p H.264 (Fast MP4)",
        description="Faster render, lower file size 720p MP4 for quick sharing.",
        width=1280,
        height=720,
        fps=30.0,
        crf=22,
        preset="fast",
        video_codec="libx264",
        audio_codec="aac",
    ),
    ExportPreset(
        name="Source / Match Timeline",
        description="Exports at native project resolution and framerate.",
        width=None,
        height=None,
        fps=None,
        crf=20,
        preset="medium",
        video_codec="libx264",
        audio_codec="aac",
    ),
    ExportPreset(
        name="1080p H.265 / HEVC",
        description="Modern HEVC codec with smaller file sizes at high fidelity.",
        width=1920,
        height=1080,
        fps=30.0,
        crf=24,
        preset="medium",
        video_codec="libx265",
        audio_codec="aac",
    ),
]
