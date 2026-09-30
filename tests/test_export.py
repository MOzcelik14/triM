from pathlib import Path
import subprocess
import tempfile
import pytest

from trim.core.clip import Clip
from trim.core.project import Project
from trim.export.presets import DEFAULT_PRESETS
from trim.export.exporter import TimelineExporter
from trim.media.ffprobe import FFprobeAnalyzer


def create_synthetic_video(path: Path, duration: float = 1.0, color: str = "blue") -> None:
    cmd = [
        "ffmpeg", "-y", "-v", "quiet",
        "-f", "lavfi", "-i", f"color=c={color}:s=320x240:r=30:d={duration}",
        "-f", "lavfi", "-i", f"sine=frequency=440:duration={duration}",
        "-c:v", "libx264", "-pix_fmt", "yuv420p",
        "-c:a", "aac",
        str(path),
    ]
    subprocess.run(cmd, check=True)


def test_real_ffmpeg_export():
    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)
        clip1_path = tmp_path / "clip1.mp4"
        clip2_path = tmp_path / "clip2.mp4"
        out_path = tmp_path / "final_export.mp4"

        create_synthetic_video(clip1_path, duration=1.0, color="red")
        create_synthetic_video(clip2_path, duration=1.5, color="blue")

        proj = Project()
        m1 = FFprobeAnalyzer.analyze(clip1_path)
        m2 = FFprobeAnalyzer.analyze(clip2_path)
        proj.add_media(m1)
        proj.add_media(m2)

        track = proj.timeline.get_video_tracks()[0]

        # Clip 1: trimmed to 0.5s
        c1 = Clip(media_id=m1.id, timeline_in=0.0, timeline_out=0.5, source_in=0.2, source_out=0.7)
        # Clip 2: 1.0s, starting after clip 1
        c2 = Clip(media_id=m2.id, timeline_in=0.5, timeline_out=1.5, source_in=0.0, source_out=1.0)

        track.add_clip(c1)
        track.add_clip(c2)

        preset = DEFAULT_PRESETS[1]  # 720p fast preset
        cmd, total_duration = TimelineExporter.build_export_pipeline(proj, preset, str(out_path))

        assert total_duration == 1.5
        # Execute ffmpeg command
        res = subprocess.run(cmd, capture_output=True, text=True)
        assert res.returncode == 0, f"FFmpeg failed: {res.stderr}"

        # Verify exported file
        assert out_path.is_file()
        exported_meta = FFprobeAnalyzer.analyze(out_path)
        assert exported_meta.width == 1280
        assert exported_meta.height == 720
        assert abs(exported_meta.duration - 1.5) < 0.1
        assert exported_meta.video_codec == "h264"
        assert exported_meta.audio_codec == "aac"
