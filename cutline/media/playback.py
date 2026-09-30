from __future__ import annotations

import logging
from pathlib import Path
from typing import Optional

import av
from PySide6.QtCore import QObject, QTimer, Qt, Signal
from PySide6.QtGui import QColor, QImage, QPainter

from cutline.core.clip import Clip
from cutline.core.media import MediaType
from cutline.core.project import Project

logger = logging.getLogger(__name__)


class PlaybackEngine(QObject):
    """Engine responsible for timeline preview playback and frame-accurate seeking."""

    frame_ready = Signal(QImage, float)  # (rendered_frame, timeline_time)
    position_changed = Signal(float)  # current timeline time in seconds
    state_changed = Signal(bool)  # is_playing

    def __init__(self, project: Project, parent: Optional[QObject] = None) -> None:
        super().__init__(parent)
        self.project = project
        self._current_time: float = 0.0
        self._is_playing: bool = False

        # Container cache: file_path -> (container, video_stream)
        self._containers: dict[str, tuple[av.container.InputContainer, any]] = {}
        self._image_cache: dict[str, QImage] = {}

        # Last decoded clip state for fast sequential playback
        self._active_clip_id: Optional[str] = None
        self._generator = None

        # Playback timer
        self._timer = QTimer(self)
        self._timer.setTimerType(Qt.TimerType.PreciseTimer)
        self._timer.timeout.connect(self._on_playback_tick)

        # Connect project/timeline signals to refresh
        self.project.timeline.clip_added.connect(self._on_timeline_changed)
        self.project.timeline.clip_removed.connect(self._on_timeline_changed)
        self.project.timeline.clip_modified.connect(self._on_timeline_changed)

    @property
    def current_time(self) -> float:
        return self._current_time

    @property
    def is_playing(self) -> bool:
        return self._is_playing

    def _get_video_stream(self, file_path: str):
        if file_path not in self._containers:
            try:
                container = av.open(file_path)
                video_stream = None
                for s in container.streams.video:
                    if s.disposition.attached_pic != 1:
                        video_stream = s
                        break
                if not video_stream and container.streams.video:
                    video_stream = container.streams.video[0]
                self._containers[file_path] = (container, video_stream)
            except Exception as e:
                logger.error("Failed to open media container %s: %s", file_path, e)
                return None, None
        return self._containers[file_path]

    def _on_timeline_changed(self, *args) -> None:
        if not self._is_playing:
            self.refresh_current_frame()

    def refresh_current_frame(self) -> None:
        self.seek(self._current_time)

    def play(self) -> None:
        if self._is_playing:
            return
        if self._current_time >= self.project.timeline.duration and self.project.timeline.duration > 0.0:
            self._current_time = 0.0

        self._is_playing = True
        fps = self.project.timeline.fps or 30.0
        interval_ms = max(5, int(round(1000.0 / fps)))
        self._timer.start(interval_ms)
        self.state_changed.emit(True)

    def pause(self) -> None:
        if not self._is_playing:
            return
        self._is_playing = False
        self._timer.stop()
        self.state_changed.emit(False)

    def toggle_play(self) -> None:
        if self._is_playing:
            self.pause()
        else:
            self.play()

    def stop(self) -> None:
        self.pause()
        self.seek(0.0)

    def step_frame(self, delta_frames: int = 1) -> None:
        self.pause()
        fps = self.project.timeline.fps or 30.0
        new_time = max(0.0, self._current_time + (delta_frames / fps))
        self.seek(new_time)

    def seek(self, timeline_time: float) -> None:
        self._current_time = max(0.0, timeline_time)
        frame = self._render_frame_at(self._current_time)
        self.frame_ready.emit(frame, self._current_time)
        self.position_changed.emit(self._current_time)

    def _on_playback_tick(self) -> None:
        fps = self.project.timeline.fps or 30.0
        self._current_time += 1.0 / fps

        # If reached or exceeded timeline duration
        timeline_dur = self.project.timeline.duration
        if timeline_dur > 0 and self._current_time > timeline_dur:
            self._current_time = timeline_dur
            self.pause()
            frame = self._render_frame_at(self._current_time)
            self.frame_ready.emit(frame, self._current_time)
            self.position_changed.emit(self._current_time)
            return

        frame = self._render_frame_at(self._current_time)
        self.frame_ready.emit(frame, self._current_time)
        self.position_changed.emit(self._current_time)

    def _render_frame_at(self, timeline_time: float) -> QImage:
        res = self.project.timeline.get_top_visible_video_clip_at(timeline_time)
        target_w = self.project.timeline.width or 1920
        target_h = self.project.timeline.height or 1080

        if not res:
            # Blank black frame
            black = QImage(target_w, target_h, QImage.Format.Format_RGB32)
            black.fill(QColor(16, 16, 18))
            return black

        track, clip = res
        media_item = self.project.get_media(clip.media_id)
        if not media_item:
            black = QImage(target_w, target_h, QImage.Format.Format_RGB32)
            black.fill(QColor(30, 10, 10))
            return black

        # Static Image
        if media_item.media_type == MediaType.IMAGE:
            if media_item.file_path not in self._image_cache:
                img = QImage(media_item.file_path)
                if not img.isNull():
                    self._image_cache[media_item.file_path] = img
            return self._image_cache.get(media_item.file_path) or QImage()

        # Video Frame
        source_time = clip.map_timeline_to_source(timeline_time)
        container, stream = self._get_video_stream(media_item.file_path)
        if not container or not stream:
            black = QImage(target_w, target_h, QImage.Format.Format_RGB32)
            black.fill(QColor(25, 25, 28))
            return black

        try:
            time_base = stream.time_base
            target_pts = int(source_time / time_base)
            # Seek container near keyframe
            container.seek(target_pts, stream=stream, backward=True, any_frame=False)

            last_frame = None
            for f in container.decode(stream):
                last_frame = f
                if f.pts is not None and f.pts >= target_pts:
                    break

            if last_frame is not None:
                rgb = last_frame.to_rgb()
                arr = rgb.to_ndarray()
                qimg = QImage(
                    arr.data,
                    last_frame.width,
                    last_frame.height,
                    arr.strides[0],
                    QImage.Format.Format_RGB888,
                ).copy()
                return qimg

        except Exception as e:
            logger.debug("Playback decode exception at %s: %s", source_time, e)

        # Fallback frame
        fallback = QImage(target_w, target_h, QImage.Format.Format_RGB32)
        fallback.fill(QColor(20, 20, 24))
        return fallback

    def close(self) -> None:
        self.pause()
        for container, _ in self._containers.values():
            try:
                container.close()
            except Exception:
                pass
        self._containers.clear()
        self._image_cache.clear()
