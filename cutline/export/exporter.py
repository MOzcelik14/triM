from __future__ import annotations

import logging
from pathlib import Path
import subprocess
import time
from typing import Optional

from PySide6.QtCore import QObject, QThread, Signal

from cutline.core.media import MediaType
from cutline.core.project import Project
from cutline.export.presets import ExportPreset

logger = logging.getLogger(__name__)


class TimelineExporter:
    """Builds FFmpeg commands for rendering a timeline."""

    @staticmethod
    def build_export_pipeline(
        project: Project,
        preset: ExportPreset,
        output_path: str,
    ) -> tuple[list[str], float]:
        video_tracks = project.timeline.get_video_tracks()
        if not video_tracks:
            raise ValueError("Project has no video tracks")

        # Find all clips on active video tracks
        clips = []
        for track in video_tracks:
            if track.visible:
                clips.extend(track.clips)
        clips.sort(key=lambda c: c.timeline_in)

        if not clips:
            raise ValueError("Timeline has no clips to export")

        out_w = preset.width or project.timeline.width or 1920
        out_h = preset.height or project.timeline.height or 1080
        out_fps = preset.fps or project.timeline.fps or 30.0

        # Construct segments taking gaps into account
        cmd_inputs: list[str] = []
        filter_parts: list[str] = []
        concat_v_inputs: list[str] = []
        concat_a_inputs: list[str] = []

        cursor = 0.0
        input_idx = 0

        for clip in clips:
            # Check for gap before this clip
            if clip.timeline_in > cursor + 0.01:
                gap_dur = clip.timeline_in - cursor
                cmd_inputs.extend([
                    "-f", "lavfi", "-t", f"{gap_dur:.4f}", "-i", f"color=c=black:s={out_w}x{out_h}:r={out_fps}",
                    "-f", "lavfi", "-t", f"{gap_dur:.4f}", "-i", "anullsrc=r=48000:cl=stereo",
                ])
                gap_v_in = input_idx
                gap_a_in = input_idx + 1
                input_idx += 2

                filter_parts.append(f"[{gap_v_in}:v]setsar=1[gv{gap_v_in}];")
                filter_parts.append(f"[{gap_a_in}:a]asetpts=PTS-STARTPTS[ga{gap_a_in}];")
                concat_v_inputs.append(f"[gv{gap_v_in}]")
                concat_a_inputs.append(f"[ga{gap_a_in}]")
                cursor = clip.timeline_in

            media_item = project.get_media(clip.media_id)
            if not media_item:
                continue

            dur = clip.duration
            src_in = clip.source_in

            if media_item.media_type == MediaType.IMAGE:
                # Loop image for duration
                cmd_inputs.extend([
                    "-loop", "1", "-t", f"{dur:.4f}", "-i", media_item.file_path,
                ])
                filter_parts.append(
                    f"[{input_idx}:v]scale={out_w}:{out_h}:force_original_aspect_ratio=decrease,"
                    f"pad={out_w}:{out_h}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={out_fps}[v{input_idx}];"
                )
                # Silent audio
                cmd_inputs.extend([
                    "-f", "lavfi", "-t", f"{dur:.4f}", "-i", "anullsrc=r=48000:cl=stereo",
                ])
                filter_parts.append(f"[{input_idx + 1}:a]asetpts=PTS-STARTPTS[a{input_idx}];")
                concat_v_inputs.append(f"[v{input_idx}]")
                concat_a_inputs.append(f"[a{input_idx}]")
                input_idx += 2
            else:
                # Video file with in-point and duration
                cmd_inputs.extend([
                    "-ss", f"{src_in:.4f}", "-t", f"{dur:.4f}", "-i", media_item.file_path,
                ])
                filter_parts.append(
                    f"[{input_idx}:v]scale={out_w}:{out_h}:force_original_aspect_ratio=decrease,"
                    f"pad={out_w}:{out_h}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={out_fps},setpts=PTS-STARTPTS[v{input_idx}];"
                )

                if media_item.audio_codec and not clip.muted:
                    filter_parts.append(
                        f"[{input_idx}:a]aformat=sample_rates=48000:channel_layouts=stereo,asetpts=PTS-STARTPTS[a{input_idx}];"
                    )
                else:
                    # Provide silent audio if file lacks audio or is muted
                    cmd_inputs.extend([
                        "-f", "lavfi", "-t", f"{dur:.4f}", "-i", "anullsrc=r=48000:cl=stereo",
                    ])
                    filter_parts.append(f"[{input_idx + 1}:a]asetpts=PTS-STARTPTS[a{input_idx}];")
                    input_idx += 1

                concat_v_inputs.append(f"[v{input_idx}]")
                concat_a_inputs.append(f"[a{input_idx}]")
                input_idx += 1

            cursor = clip.timeline_out

        total_duration = max(0.1, cursor)
        num_segments = len(concat_v_inputs)
        if num_segments == 0:
            raise ValueError("No valid media segments found to export")

        # Concat filter
        concat_filter = (
            "".join(f"{v}{a}" for v, a in zip(concat_v_inputs, concat_a_inputs))
            + f"concat=n={num_segments}:v=1:a=1[outv][outa]"
        )
        filter_parts.append(concat_filter)
        full_filtergraph = "".join(filter_parts)

        # Assemble full ffmpeg command
        cmd = ["ffmpeg", "-y", "-v", "error"]
        cmd.extend(cmd_inputs)
        cmd.extend([
            "-filter_complex", full_filtergraph,
            "-map", "[outv]",
            "-map", "[outa]",
            "-c:v", preset.video_codec,
            "-preset", preset.preset,
            "-crf", str(preset.crf),
            "-c:a", preset.audio_codec,
            "-b:a", preset.audio_bitrate,
            "-pix_fmt", "yuv420p",
            "-progress", "pipe:1",
            str(output_path),
        ])

        return cmd, total_duration


class ExportWorker(QThread):
    """Background worker thread executing FFmpeg export and parsing progress."""

    progress_updated = Signal(float)  # 0.0 to 100.0
    status_updated = Signal(str)
    export_finished = Signal(bool, str)  # (success, message_or_path)

    def __init__(
        self,
        project: Project,
        preset: ExportPreset,
        output_path: str,
        parent: Optional[QObject] = None,
    ) -> None:
        super().__init__(parent)
        self.project = project
        self.preset = preset
        self.output_path = output_path
        self._is_cancelled = False
        self._process: Optional[subprocess.Popen] = None

    def cancel(self) -> None:
        self._is_cancelled = True
        if self._process and self._process.poll() is None:
            logger.info("Cancelling export process...")
            self._process.terminate()
            try:
                self._process.wait(timeout=1.0)
            except subprocess.TimeoutExpired:
                self._process.kill()

    def run(self) -> None:
        try:
            cmd, total_duration = TimelineExporter.build_export_pipeline(
                self.project, self.preset, self.output_path
            )
            self.status_updated.emit("Starting FFmpeg encoder...")
            logger.info("Executing export command: %s", " ".join(cmd))

            self._process = subprocess.Popen(
                cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1,
            )

            assert self._process.stdout is not None
            for line in self._process.stdout:
                if self._is_cancelled:
                    break
                line = line.strip()
                if line.startswith("out_time_us="):
                    try:
                        out_us = int(line.split("=")[1])
                        current_sec = out_us / 1_000_000.0
                        pct = min(100.0, max(0.0, (current_sec / total_duration) * 100.0))
                        self.progress_updated.emit(pct)
                        self.status_updated.emit(
                            f"Exporting: {pct:.1f}% ({current_sec:.1f}s / {total_duration:.1f}s)"
                        )
                    except (ValueError, IndexError):
                        pass

            self._process.wait()

            if self._is_cancelled:
                # Remove partially exported file
                p = Path(self.output_path)
                if p.is_file():
                    p.unlink(missing_ok=True)
                self.export_finished.emit(False, "Export cancelled by user")
                return

            if self._process.returncode == 0:
                self.progress_updated.emit(100.0)
                self.status_updated.emit("Export completed successfully!")
                self.export_finished.emit(True, self.output_path)
            else:
                stderr_output = self._process.stderr.read() if self._process.stderr else ""
                logger.error("FFmpeg export failed with code %d: %s", self._process.returncode, stderr_output)
                self.export_finished.emit(False, f"FFmpeg error: {stderr_output.strip() or 'Unknown error'}")

        except Exception as e:
            logger.exception("Export worker failure: %s", e)
            self.export_finished.emit(False, str(e))
