from __future__ import annotations

from collections import deque
import logging
import math
from pathlib import Path
from typing import Generator, Optional

import av
import numpy as np
from PySide6.QtCore import QElapsedTimer, QObject, QRectF, QTimer, Qt, Signal
from PySide6.QtGui import QColor, QImage, QPainter
from PySide6.QtMultimedia import QAudioFormat, QAudioSink

from trim.core.clip import Clip
from trim.core.media import MediaType
from trim.core.project import Project

logger = logging.getLogger(__name__)


class PlaybackEngine(QObject):
    """High-performance frame-accurate video & audio playback engine.

    Features:
    - Pristine PCM audio playback via QAudioSink and PyAV AudioResampler
    - Pure sample byte extraction (zero memory padding / stride artifacts)
    - Aligned 4-byte stereo buffer feeding preventing sample phase distortion
    - Queue-buffered A/V synchronization maintaining exact 1.000x real-time speed
    - Instant frame-accurate seeking for timeline scrubbing and editing
    """

    frame_ready = Signal(QImage, float)  # (rendered_frame, timeline_time)
    position_changed = Signal(float)  # current timeline time in seconds
    state_changed = Signal(bool)  # is_playing
    audio_levels_ready = Signal(float, float)  # (left_db, right_db) in dBFS
    speed_changed = Signal(float)  # current playback speed multiplier

    def __init__(self, project: Project, parent: Optional[QObject] = None) -> None:
        super().__init__(parent)
        self.project = project
        self._current_time: float = 0.0
        self._is_playing: bool = False
        self._playback_speed: float = 1.0

        # Container cache: file_path -> (container, video_stream, audio_stream)
        self._containers: dict[str, tuple[av.container.InputContainer, any, any]] = {}
        self._image_cache: dict[str, QImage] = {}

        # Audio Output System (48kHz, 16-bit Stereo PCM)
        self._audio_format = QAudioFormat()
        self._audio_format.setSampleRate(48000)
        self._audio_format.setChannelCount(2)
        self._audio_format.setSampleFormat(QAudioFormat.SampleFormat.Int16)

        try:
            self._audio_sink: Optional[QAudioSink] = QAudioSink(self._audio_format, self)
            self._audio_sink.setBufferSize(96000)  # ~0.5 second buffer
        except Exception as e:
            logger.warning("Could not initialize QAudioSink: %s", e)
            self._audio_sink = None

        self._audio_io = None
        self._resampler = av.AudioResampler(format="s16", layout="stereo", rate=48000)

        # High-precision playback timing
        self._playback_clock = QElapsedTimer()
        self._playback_start_time = 0.0

        # Queue-buffered state
        self._video_queue: deque[tuple[QImage, float]] = deque()
        self._audio_queue = bytearray()

        self._seq_clip_id: Optional[str] = None
        self._seq_generator: Optional[Generator] = None
        self._seq_v_stream = None
        self._seq_a_stream = None
        self._seq_source_start: float = 0.0
        self._last_rendered_frame: Optional[QImage] = None

        # Playback timer (runs at 10ms / 100Hz for responsive queue updates)
        self._timer = QTimer(self)
        self._timer.setTimerType(Qt.TimerType.PreciseTimer)
        self._timer.timeout.connect(self._on_playback_tick)

        # Connect model signals
        self.project.timeline.clip_added.connect(self._on_timeline_changed)
        self.project.timeline.clip_removed.connect(self._on_timeline_changed)
        self.project.timeline.clip_modified.connect(self._on_timeline_changed)

    @property
    def current_time(self) -> float:
        return self._current_time

    @property
    def is_playing(self) -> bool:
        return self._is_playing

    def _get_media_streams(self, file_path: str):
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

                audio_stream = container.streams.audio[0] if container.streams.audio else None
                self._containers[file_path] = (container, video_stream, audio_stream)
            except Exception as e:
                logger.error("Failed to open media container %s: %s", file_path, e)
                return None, None, None
        return self._containers[file_path]

    def _on_timeline_changed(self, *args) -> None:
        if not self._is_playing:
            self.refresh_current_frame()

    def refresh_current_frame(self) -> None:
        self.seek(self._current_time)

    @property
    def speed(self) -> float:
        return self._playback_speed

    def set_speed(self, speed: float) -> None:
        if speed == 0.0:
            self.pause()
            return

        self._playback_speed = speed
        self.speed_changed.emit(speed)

        if not self._is_playing:
            self.play()
        else:
            self._playback_start_time = self._current_time
            self._playback_clock.restart()
            self._video_queue.clear()
            self._audio_queue.clear()
            if abs(speed - 1.0) < 0.01:
                self._setup_sequential_stream(self._current_time)
            else:
                self._seq_generator = None

    def play(self) -> None:
        if self._is_playing:
            return

        timeline_dur = self.project.timeline.duration
        if timeline_dur > 0.0 and self._current_time >= timeline_dur:
            self._current_time = 0.0

        self._is_playing = True
        self._playback_start_time = self._current_time
        self._playback_clock.start()

        self._video_queue.clear()
        self._audio_queue.clear()
        self._resampler = av.AudioResampler(format="s16", layout="stereo", rate=48000)

        # Start audio output device
        if self._audio_sink:
            try:
                self._audio_sink.reset()
                self._audio_io = self._audio_sink.start()
            except Exception as e:
                logger.warning("Failed to start audio sink: %s", e)
                self._audio_io = None

        if abs(self._playback_speed - 1.0) < 0.01:
            self._setup_sequential_stream(self._current_time)
        self._timer.start(10)  # 100 Hz timer
        self.state_changed.emit(True)

    def pause(self) -> None:
        if not self._is_playing:
            return
        self._is_playing = False
        self._playback_speed = 1.0
        self.speed_changed.emit(1.0)
        self._timer.stop()

        if self._audio_sink:
            try:
                self._audio_sink.stop()
            except Exception:
                pass
        self._audio_io = None
        self._seq_generator = None
        self._video_queue.clear()
        self._audio_queue.clear()

        self.audio_levels_ready.emit(-60.0, -60.0)
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
        was_playing = self._is_playing
        if was_playing:
            self.pause()

        self._current_time = max(0.0, timeline_time)
        self._video_queue.clear()
        self._audio_queue.clear()
        self._seq_generator = None

        frame = self._seek_and_render_frame(self._current_time)
        self.frame_ready.emit(frame, self._current_time)
        self.position_changed.emit(self._current_time)

        if was_playing:
            self.play()

    def _setup_sequential_stream(self, timeline_time: float) -> None:
        self._video_queue.clear()
        self._audio_queue.clear()

        res = self.project.timeline.get_top_visible_video_clip_at(timeline_time)
        if not res:
            self._seq_clip_id = None
            self._seq_generator = None
            self._seq_v_stream = None
            self._seq_a_stream = None
            return

        track, clip = res
        media_item = self.project.get_media(clip.media_id)
        if not media_item:
            self._seq_clip_id = None
            self._seq_generator = None
            self._seq_v_stream = None
            self._seq_a_stream = None
            return

        self._seq_clip_id = clip.id
        if self._audio_sink:
            vol = 0.0 if clip.muted or track.muted else clip.volume
            self._audio_sink.setVolume(max(0.0, min(1.0, vol)))

        if media_item.media_type == MediaType.IMAGE:
            self._seq_generator = None
            self._seq_v_stream = None
            self._seq_a_stream = None
            return

        container, v_stream, a_stream = self._get_media_streams(media_item.file_path)
        if not container or not v_stream:
            self._seq_generator = None
            self._seq_v_stream = None
            self._seq_a_stream = None
            return

        self._seq_v_stream = v_stream
        self._seq_a_stream = a_stream
        source_time = clip.map_timeline_to_source(timeline_time)
        self._seq_source_start = source_time

        try:
            target_pts = int(source_time / v_stream.time_base)
            container.seek(target_pts, stream=v_stream, backward=True, any_frame=False)

            streams = [v_stream]
            if a_stream and not clip.muted and not track.muted:
                streams.append(a_stream)

            self._seq_generator = container.demux(*streams)
        except Exception as e:
            logger.debug("Failed to set up stream generator at %s: %s", source_time, e)
            self._seq_generator = None
            self._seq_v_stream = None
            self._seq_a_stream = None

    def _on_playback_tick(self) -> None:
        if not self._is_playing:
            return

        elapsed_sec = (self._playback_clock.elapsed() / 1000.0) * self._playback_speed
        cur_time = self._playback_start_time + elapsed_sec

        timeline_dur = self.project.timeline.duration
        if timeline_dur > 0 and (cur_time >= timeline_dur or cur_time < 0.0):
            self._current_time = max(0.0, min(timeline_dur, cur_time))
            self.pause()
            self.seek(self._current_time)
            return

        self._current_time = cur_time

        # Fast forward or reverse playback mode
        if abs(self._playback_speed - 1.0) > 0.01:
            frame = self._seek_and_render_frame(self._current_time)
            self.frame_ready.emit(frame, self._current_time)
            self.position_changed.emit(self._current_time)
            self.audio_levels_ready.emit(-60.0, -60.0)
            return

        res = self.project.timeline.get_top_visible_video_clip_at(self._current_time)

        # Handle clip transitions or gaps
        current_clip_id = res[1].id if res else None
        if current_clip_id != self._seq_clip_id:
            self._setup_sequential_stream(self._current_time)

        if not res:
            # Empty gap
            target_w = self.project.timeline.width or 1920
            target_h = self.project.timeline.height or 1080
            blank = QImage(target_w, target_h, QImage.Format.Format_RGB32)
            blank.fill(QColor(14, 14, 16))
            self._last_rendered_frame = blank
            self.frame_ready.emit(blank, self._current_time)
            self.position_changed.emit(self._current_time)
            self.audio_levels_ready.emit(-60.0, -60.0)
            return

        track, clip = res
        media_item = self.project.get_media(clip.media_id)
        if not media_item:
            return

        # Static image handling
        if media_item.media_type == MediaType.IMAGE:
            if media_item.file_path not in self._image_cache:
                img = QImage(media_item.file_path)
                if not img.isNull():
                    self._image_cache[media_item.file_path] = img
            img = self._image_cache.get(media_item.file_path)
            if img:
                self._last_rendered_frame = img
                self.frame_ready.emit(img, self._current_time)
            self.position_changed.emit(self._current_time)
            self.audio_levels_ready.emit(-60.0, -60.0)
            return

        # Video & Audio queue processing
        if self._seq_generator and self._seq_v_stream:
            src_target = clip.map_timeline_to_source(self._current_time)

            # 1. Replenish queues (maintain up to 10 video frames and ~0.5s audio)
            try:
                while len(self._video_queue) < 10 or len(self._audio_queue) < 48000:
                    packet = next(self._seq_generator)
                    for f in packet.decode():
                        if isinstance(f, av.AudioFrame):
                            any_solo = any(t.solo for t in self.project.timeline.tracks)
                            track_active = (not any_solo or track.solo) and not track.muted
                            if not clip.muted and track_active and self._seq_a_stream:
                                if f.pts is not None:
                                    a_pts_sec = float(f.pts * self._seq_a_stream.time_base)
                                    # Discard past audio frames prior to seek start point
                                    if a_pts_sec >= self._seq_source_start - 0.05:
                                        resampled_list = self._resampler.resample(f)
                                        if resampled_list:
                                            for rf in resampled_list:
                                                # Use to_ndarray().tobytes() to ensure NO uninitialized padding!
                                                arr = rf.to_ndarray()
                                                # Apply volume & fade in / fade out
                                                dt = a_pts_sec - clip.source_in
                                                rem = clip.duration - dt
                                                fade = 1.0
                                                if clip.fade_in > 0.0 and dt < clip.fade_in:
                                                    fade *= max(0.0, dt / clip.fade_in)
                                                if clip.fade_out > 0.0 and rem < clip.fade_out:
                                                    fade *= max(0.0, rem / clip.fade_out)
                                                eff_vol = max(0.0, min(3.0, clip.volume * getattr(track, 'volume', 1.0) * fade))
                                                if abs(eff_vol - 1.0) > 0.01:
                                                    arr = np.clip(arr.astype(np.float32) * eff_vol, -32768, 32767).astype(np.int16)
                                                self._audio_queue.extend(arr.tobytes())

                        elif isinstance(f, av.VideoFrame):
                            if f.pts is not None:
                                v_pts_sec = float(f.pts * self._seq_v_stream.time_base)
                                if v_pts_sec >= self._seq_source_start - 0.05:
                                    rgb = f.to_rgb()
                                    arr = rgb.to_ndarray()
                                    qimg = QImage(
                                        arr.data,
                                        f.width,
                                        f.height,
                                        arr.strides[0],
                                        QImage.Format.Format_RGB888,
                                    ).copy()
                                    self._video_queue.append((qimg, v_pts_sec))
            except StopIteration:
                pass
            except Exception as e:
                logger.debug("Sequential demux error: %s", e)

            # 2. Feed audio sink (aligned strictly to 4-byte stereo 16-bit boundaries)
            if self._audio_sink and self._audio_io:
                free_bytes = self._audio_sink.bytesFree()
                if free_bytes > 0 and self._audio_queue:
                    to_write = min(free_bytes, len(self._audio_queue))
                    to_write = (to_write // 4) * 4  # Ensure complete stereo samples
                    if to_write > 0:
                        written = self._audio_io.write(self._audio_queue[:to_write])
                        if written > 0:
                            # Calculate live audio levels for VU meter
                            if written >= 32:
                                chunk = np.frombuffer(self._audio_queue[:written], dtype=np.int16)
                                if len(chunk) >= 2:
                                    left_samples = chunk[0::2].astype(np.float32) / 32768.0
                                    right_samples = chunk[1::2].astype(np.float32) / 32768.0
                                    rms_l = float(np.sqrt(np.mean(left_samples ** 2)))
                                    rms_r = float(np.sqrt(np.mean(right_samples ** 2)))
                                    db_l = 20.0 * math.log10(max(1e-4, rms_l))
                                    db_r = 20.0 * math.log10(max(1e-4, rms_r))
                                    self.audio_levels_ready.emit(db_l, db_r)
                            del self._audio_queue[:written]
                elif not self._audio_queue:
                    self.audio_levels_ready.emit(-60.0, -60.0)

            # 3. Deliver video frame whose PTS matches current source target
            frame_to_show = None
            while self._video_queue and src_target >= self._video_queue[0][1]:
                frame_to_show, _ = self._video_queue.popleft()

            if frame_to_show is not None:
                visible_clips = self.project.timeline.get_visible_video_clips_at(self._current_time)
                has_transform = (
                    clip.scale != 1.0
                    or clip.pos_x != 0.0
                    or clip.pos_y != 0.0
                    or clip.opacity < 0.999
                    or clip.fade_in > 0.0
                    or clip.fade_out > 0.0
                    or clip.brightness != 0.0
                    or clip.contrast != 1.0
                    or clip.saturation != 1.0
                )
                if len(visible_clips) > 1 or has_transform:
                    target_w = self.project.timeline.width or 1920
                    target_h = self.project.timeline.height or 1080
                    canvas = QImage(target_w, target_h, QImage.Format.Format_ARGB32_Premultiplied)
                    canvas.fill(QColor(14, 14, 16))
                    painter = QPainter(canvas)
                    painter.setRenderHint(QPainter.RenderHint.SmoothPixmapTransform, True)

                    for trk, clp in visible_clips:
                        if clp.id == clip.id:
                            self._draw_clip_frame(painter, clp, frame_to_show, target_w, target_h, self._current_time)
                        else:
                            m_item = self.project.get_media(clp.media_id)
                            if m_item:
                                if m_item.media_type == MediaType.IMAGE:
                                    img = self._image_cache.get(m_item.file_path)
                                    if not img:
                                        img = QImage(m_item.file_path)
                                        if not img.isNull():
                                            self._image_cache[m_item.file_path] = img
                                    if img:
                                        self._draw_clip_frame(painter, clp, img, target_w, target_h, self._current_time)
                                else:
                                    v_img = self._decode_single_video_frame(m_item.file_path, clp.map_timeline_to_source(self._current_time))
                                    if v_img:
                                        self._draw_clip_frame(painter, clp, v_img, target_w, target_h, self._current_time)
                    painter.end()
                    self._last_rendered_frame = canvas
                    self.frame_ready.emit(canvas, self._current_time)
                else:
                    self._last_rendered_frame = frame_to_show
                    self.frame_ready.emit(frame_to_show, self._current_time)
            elif self._last_rendered_frame is not None:
                self.frame_ready.emit(self._last_rendered_frame, self._current_time)

        self.position_changed.emit(self._current_time)

    def _apply_color_adjustments(self, img: QImage, brightness: float, contrast: float, saturation: float) -> QImage:
        w, h = img.width(), img.height()
        if w <= 0 or h <= 0:
            return img

        img_rgb = img.convertToFormat(QImage.Format.Format_RGB888)
        arr = np.frombuffer(img_rgb.bits(), dtype=np.uint8).reshape((h, w, 3)).astype(np.float32)

        # 1. Contrast & Brightness: contrast * (pixel - 128) + 128 + brightness * 255
        if contrast != 1.0 or brightness != 0.0:
            arr = contrast * (arr - 128.0) + 128.0 + (brightness * 255.0)

        # 2. Saturation: luma + saturation * (pixel - luma)
        if saturation != 1.0:
            gray = 0.299 * arr[..., 0] + 0.587 * arr[..., 1] + 0.114 * arr[..., 2]
            gray = np.expand_dims(gray, axis=-1)
            arr = gray + saturation * (arr - gray)

        arr = np.clip(arr, 0.0, 255.0).astype(np.uint8)
        return QImage(arr.data, w, h, arr.strides[0], QImage.Format.Format_RGB888).copy()

    def _draw_clip_frame(
        self,
        painter: QPainter,
        clip: Clip,
        frame_img: QImage,
        canvas_w: int,
        canvas_h: int,
        current_time: float,
    ) -> None:
        clip_time = max(0.0, current_time - clip.timeline_in)

        # Dynamic / animated properties (using keyframes if available)
        animated_opacity = clip.get_property_at_time("opacity", clip_time)
        animated_scale = clip.get_property_at_time("scale", clip_time)
        animated_pos_x = clip.get_property_at_time("pos_x", clip_time)
        animated_pos_y = clip.get_property_at_time("pos_y", clip_time)
        animated_rot = clip.get_property_at_time("rotation", clip_time)

        eff_opacity = max(0.0, min(1.0, animated_opacity))
        dt = current_time - clip.timeline_in
        rem = clip.timeline_out - current_time

        fade_in_factor = 1.0
        if clip.fade_in > 0.0 and dt < clip.fade_in:
            fade_in_factor = max(0.0, min(1.0, dt / clip.fade_in))
            eff_opacity *= fade_in_factor

        fade_out_factor = 1.0
        if clip.fade_out > 0.0 and rem < clip.fade_out:
            fade_out_factor = max(0.0, min(1.0, rem / clip.fade_out))
            eff_opacity *= fade_out_factor

        if eff_opacity <= 0.001:
            return

        fw = frame_img.width()
        fh = frame_img.height()
        if fw <= 0 or fh <= 0:
            return

        # Apply color adjustments if modified
        if clip.brightness != 0.0 or clip.contrast != 1.0 or clip.saturation != 1.0:
            frame_img = self._apply_color_adjustments(frame_img, clip.brightness, clip.contrast, clip.saturation)

        scale_fit = min(canvas_w / fw, canvas_h / fh)
        base_w = fw * scale_fit
        base_h = fh * scale_fit

        final_w = base_w * max(0.01, animated_scale)
        final_h = base_h * max(0.01, animated_scale)

        final_x = (canvas_w - final_w) / 2.0 + animated_pos_x
        final_y = (canvas_h - final_h) / 2.0 + animated_pos_y

        painter.save()
        painter.setOpacity(eff_opacity)
        dest_rect = QRectF(final_x, final_y, final_w, final_h)

        if abs(animated_rot) > 0.01:
            painter.translate(dest_rect.center())
            painter.rotate(animated_rot)
            painter.drawImage(QRectF(-final_w / 2.0, -final_h / 2.0, final_w, final_h), frame_img)
        else:
            painter.drawImage(dest_rect, frame_img)

        # Dip to White overlay transition
        if clip.transition_in == "dip_white" and fade_in_factor < 1.0:
            white_opacity = (1.0 - fade_in_factor) * eff_opacity
            painter.fillRect(dest_rect, QColor(255, 255, 255, int(white_opacity * 255)))
        elif clip.transition_out == "dip_white" and fade_out_factor < 1.0:
            white_opacity = (1.0 - fade_out_factor) * eff_opacity
            painter.fillRect(dest_rect, QColor(255, 255, 255, int(white_opacity * 255)))

        painter.restore()

    def _decode_single_video_frame(self, file_path: str, source_time: float) -> Optional[QImage]:
        container, stream, _ = self._get_media_streams(file_path)
        if not container or not stream:
            return None

        try:
            target_pts = int(source_time / stream.time_base)
            container.seek(target_pts, stream=stream, backward=True, any_frame=False)

            last_frame = None
            for f in container.decode(stream):
                last_frame = f
                if f.pts is not None and f.pts >= target_pts:
                    break

            if last_frame is not None:
                rgb = last_frame.to_rgb()
                arr = rgb.to_ndarray()
                return QImage(
                    arr.data,
                    last_frame.width,
                    last_frame.height,
                    arr.strides[0],
                    QImage.Format.Format_RGB888,
                ).copy()
        except Exception as e:
            logger.debug("Seek decode exception at %s: %s", source_time, e)
        return None

    def _seek_and_render_frame(self, timeline_time: float) -> QImage:
        visible_clips = self.project.timeline.get_visible_video_clips_at(timeline_time)
        target_w = self.project.timeline.width or 1920
        target_h = self.project.timeline.height or 1080

        if not visible_clips:
            blank = QImage(target_w, target_h, QImage.Format.Format_RGB32)
            blank.fill(QColor(14, 14, 16))
            self._last_rendered_frame = blank
            return blank

        canvas = QImage(target_w, target_h, QImage.Format.Format_ARGB32_Premultiplied)
        canvas.fill(QColor(14, 14, 16))
        painter = QPainter(canvas)
        painter.setRenderHint(QPainter.RenderHint.SmoothPixmapTransform, True)

        for track, clip in visible_clips:
            media_item = self.project.get_media(clip.media_id)
            if not media_item:
                continue

            frame_img = None
            if media_item.media_type == MediaType.IMAGE:
                if media_item.file_path not in self._image_cache:
                    img = QImage(media_item.file_path)
                    if not img.isNull():
                        self._image_cache[media_item.file_path] = img
                frame_img = self._image_cache.get(media_item.file_path)
            else:
                source_time = clip.map_timeline_to_source(timeline_time)
                frame_img = self._decode_single_video_frame(media_item.file_path, source_time)

            if frame_img and not frame_img.isNull():
                self._draw_clip_frame(painter, clip, frame_img, target_w, target_h, timeline_time)

        painter.end()
        self._last_rendered_frame = canvas
        return canvas

    def close(self) -> None:
        self.pause()
        for container, _, _ in self._containers.values():
            try:
                container.close()
            except Exception:
                pass
        self._containers.clear()
        self._image_cache.clear()
        if self._audio_sink:
            try:
                self._audio_sink.stop()
            except Exception:
                pass
        self._audio_io = None
