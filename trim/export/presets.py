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
        name="1080p H.264 (Standart MP4)",
        description="YouTube, web ve arşivleme için yüksek kaliteli 1080p MP4 çıktısı.",
        width=1920,
        height=1080,
        fps=30.0,
        crf=20,
        preset="medium",
        video_codec="libx264",
        audio_codec="aac",
    ),
    ExportPreset(
        name="720p H.264 (Hızlı MP4)",
        description="Daha hızlı render ve düşük dosya boyutu sağlayan 720p MP4 çıktısı.",
        width=1280,
        height=720,
        fps=30.0,
        crf=22,
        preset="fast",
        video_codec="libx264",
        audio_codec="aac",
    ),
    ExportPreset(
        name="Kaynak / Projeyle Eşleşen",
        description="Projenin doğal çözünürlük ve kare hızında doğrudan çıktı.",
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
        description="Yüksek kalitede daha küçük dosya boyutu sağlayan modern HEVC çıktısı.",
        width=1920,
        height=1080,
        fps=30.0,
        crf=24,
        preset="medium",
        video_codec="libx265",
        audio_codec="aac",
    ),
]
