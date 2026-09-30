from __future__ import annotations

import json
import logging
from pathlib import Path
import subprocess
from typing import Optional

from trim.core.media import MediaItem, MediaType

logger = logging.getLogger(__name__)

IMAGE_EXTENSIONS = {".png", ".jpg", ".jpeg", ".webp", ".bmp", ".tiff", ".gif"}
AUDIO_EXTENSIONS = {".mp3", ".wav", ".aac", ".flac", ".ogg", ".m4a"}
VIDEO_EXTENSIONS = {".mp4", ".mov", ".mkv", ".webm", ".avi", ".ts"}


class FFprobeAnalyzer:
    """Extracts media metadata using the system ffprobe CLI tool."""

    @staticmethod
    def analyze(file_path: str | Path) -> MediaItem:
        path = Path(file_path).resolve()
        if not path.is_file():
            raise FileNotFoundError(f"Media file not found: {path}")

        cmd = [
            "ffprobe",
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            str(path),
        ]

        try:
            res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=True)
            info = json.loads(res.stdout)
        except (subprocess.SubprocessError, json.JSONDecodeError) as e:
            logger.error("FFprobe failed on %s: %s", path, e)
            raise RuntimeError(f"Failed to probe media file: {e}")

        streams = info.get("streams", [])
        fmt = info.get("format", {})

        video_stream: Optional[dict] = None
        audio_stream: Optional[dict] = None

        for s in streams:
            codec_type = s.get("codec_type")
            if codec_type == "video" and not video_stream:
                # Exclude attached pictures in audio files (cover art)
                if s.get("disposition", {}).get("attached_pic", 0) != 1:
                    video_stream = s
            elif codec_type == "audio" and not audio_stream:
                audio_stream = s

        ext = path.suffix.lower()
        duration = 0.0
        try:
            duration = float(fmt.get("duration", 0.0))
        except (ValueError, TypeError):
            duration = 0.0

        # Determine media type
        if ext in IMAGE_EXTENSIONS:
            media_type = MediaType.IMAGE
            duration = 5.0  # Default duration for static images on timeline
        elif video_stream:
            media_type = MediaType.VIDEO
        elif audio_stream or ext in AUDIO_EXTENSIONS:
            media_type = MediaType.AUDIO
        else:
            media_type = MediaType.UNKNOWN

        # FPS calculation
        fps = 30.0
        width = 1920
        height = 1080
        video_codec = ""

        if video_stream:
            width = int(video_stream.get("width", 1920))
            height = int(video_stream.get("height", 1080))
            video_codec = video_stream.get("codec_name", "")

            # Parse fps string (e.g. "30/1" or "30000/1001")
            fps_str = video_stream.get("r_frame_rate", video_stream.get("avg_frame_rate", "30/1"))
            try:
                if "/" in fps_str:
                    num, den = map(float, fps_str.split("/"))
                    fps = num / den if den != 0 else 30.0
                else:
                    fps = float(fps_str)
                if fps <= 0.0 or fps > 240.0:
                    fps = 30.0
            except (ValueError, ZeroDivisionError):
                fps = 30.0

            # If format duration is 0 but stream has duration
            if duration <= 0.0 and "duration" in video_stream:
                try:
                    duration = float(video_stream["duration"])
                except (ValueError, TypeError):
                    pass

            if duration <= 0.0 and "nb_frames" in video_stream and fps > 0:
                try:
                    duration = float(video_stream["nb_frames"]) / fps
                except (ValueError, TypeError):
                    pass

        audio_codec = ""
        sample_rate = 48000
        channels = 2

        if audio_stream:
            audio_codec = audio_stream.get("codec_name", "")
            sample_rate = int(audio_stream.get("sample_rate", 48000))
            channels = int(audio_stream.get("channels", 2))
            if duration <= 0.0 and "duration" in audio_stream:
                try:
                    duration = float(audio_stream["duration"])
                except (ValueError, TypeError):
                    pass

        return MediaItem(
            file_path=str(path),
            name=path.name,
            media_type=media_type,
            duration=max(0.1, duration),
            fps=fps,
            width=width,
            height=height,
            video_codec=video_codec,
            audio_codec=audio_codec,
            sample_rate=sample_rate,
            channels=channels,
        )
