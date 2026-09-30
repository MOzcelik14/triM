from __future__ import annotations

from typing import Optional

from PySide6.QtGui import QUndoCommand

from trim.core.clip import Clip
from trim.core.marker import Marker
from trim.core.timeline import TimelineModel
from trim.core.track import Track, TrackType


class AddClipCommand(QUndoCommand):
    """Command to add a clip to a track."""

    def __init__(
        self,
        timeline: TimelineModel,
        track_id: str,
        clip: Clip,
        description: str = "Klip Ekle",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.track_id = track_id
        self.clip = clip

    def redo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            track.add_clip(self.clip, allow_overlap=True)
            self.timeline.notify_clip_added(self.track_id, self.clip.id)

    def undo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            track.remove_clip(self.clip.id)
            self.timeline.notify_clip_removed(self.track_id, self.clip.id)


class RemoveClipCommand(QUndoCommand):
    """Command to remove a clip from a track."""

    def __init__(
        self,
        timeline: TimelineModel,
        track_id: str,
        clip_id: str,
        description: str = "Klip Sil",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.track_id = track_id
        self.clip_id = clip_id
        self._saved_clip: Optional[Clip] = None

    def redo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            clip = track.get_clip_by_id(self.clip_id)
            if clip:
                self._saved_clip = clip.clone(new_id=False)
            track.remove_clip(self.clip_id)
            self.timeline.notify_clip_removed(self.track_id, self.clip_id)

    def undo(self) -> None:
        if self._saved_clip:
            track = self.timeline.get_track_by_id(self.track_id)
            if track:
                track.add_clip(self._saved_clip, allow_overlap=True)
                self.timeline.notify_clip_added(self.track_id, self._saved_clip.id)


class MoveClipCommand(QUndoCommand):
    """Command to move a clip to a new start time."""

    def __init__(
        self,
        timeline: TimelineModel,
        track_id: str,
        clip_id: str,
        old_time: float,
        new_time: float,
        description: str = "Klip Taşı",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.track_id = track_id
        self.clip_id = clip_id
        self.old_time = old_time
        self.new_time = new_time

    def redo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            clip = track.get_clip_by_id(self.clip_id)
            if clip:
                clip.move_to(self.new_time)
                track.sort_clips()
                self.timeline.notify_clip_modified(self.track_id, self.clip_id)

    def undo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            clip = track.get_clip_by_id(self.clip_id)
            if clip:
                clip.move_to(self.old_time)
                track.sort_clips()
                self.timeline.notify_clip_modified(self.track_id, self.clip_id)


class TrimClipCommand(QUndoCommand):
    """Command to modify clip boundaries (in/out points)."""

    def __init__(
        self,
        timeline: TimelineModel,
        track_id: str,
        clip_id: str,
        old_bounds: tuple[float, float, float, float],  # (tl_in, tl_out, src_in, src_out)
        new_bounds: tuple[float, float, float, float],
        description: str = "Klip Kırp",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.track_id = track_id
        self.clip_id = clip_id
        self.old_bounds = old_bounds
        self.new_bounds = new_bounds

    def _apply(self, bounds: tuple[float, float, float, float]) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            clip = track.get_clip_by_id(self.clip_id)
            if clip:
                clip.timeline_in = bounds[0]
                clip.timeline_out = bounds[1]
                clip.source_in = bounds[2]
                clip.source_out = bounds[3]
                track.sort_clips()
                self.timeline.notify_clip_modified(self.track_id, self.clip_id)

    def redo(self) -> None:
        self._apply(self.new_bounds)

    def undo(self) -> None:
        self._apply(self.old_bounds)


class SplitClipCommand(QUndoCommand):
    """Command to cut a clip into two at a given timeline time."""

    def __init__(
        self,
        timeline: TimelineModel,
        track_id: str,
        clip_id: str,
        split_time: float,
        description: str = "Klip Böl",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.track_id = track_id
        self.clip_id = clip_id
        self.split_time = split_time
        self._original_clip: Optional[Clip] = None
        self._right_clip_id: Optional[str] = None

    def redo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            clip = track.get_clip_by_id(self.clip_id)
            if clip:
                self._original_clip = clip.clone(new_id=False)
                left, right = track.split_clip(self.clip_id, self.split_time)
                self._right_clip_id = right.id
                self.timeline.notify_clip_modified(self.track_id, left.id)
                self.timeline.notify_clip_added(self.track_id, right.id)

    def undo(self) -> None:
        if self._original_clip:
            track = self.timeline.get_track_by_id(self.track_id)
            if track:
                if self._right_clip_id:
                    track.remove_clip(self._right_clip_id)
                    self.timeline.notify_clip_removed(self.track_id, self._right_clip_id)

                # Restore original clip parameters
                clip = track.get_clip_by_id(self._original_clip.id)
                if clip:
                    clip.timeline_in = self._original_clip.timeline_in
                    clip.timeline_out = self._original_clip.timeline_out
                    clip.source_in = self._original_clip.source_in
                    clip.source_out = self._original_clip.source_out
                    track.sort_clips()
                    self.timeline.notify_clip_modified(self.track_id, clip.id)


class RippleDeleteCommand(QUndoCommand):
    """Command to delete a clip and shift following clips left."""

    def __init__(
        self,
        timeline: TimelineModel,
        track_id: str,
        clip_id: str,
        description: str = "Boşluksuz Sil",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.track_id = track_id
        self.clip_id = clip_id
        self._saved_clips_snapshot: list[Clip] = []

    def redo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            self._saved_clips_snapshot = [c.clone(new_id=False) for c in track.clips]
            track.ripple_delete(self.clip_id)
            self.timeline.notify_clip_removed(self.track_id, self.clip_id)

    def undo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            track.clips = [c.clone(new_id=False) for c in self._saved_clips_snapshot]
            track.sort_clips()
            self.timeline.tracks_changed.emit()
            self.timeline.duration_changed.emit(self.timeline.duration)


class MoveMultipleClipsCommand(QUndoCommand):
    """Command to move multiple clips across tracks simultaneously."""

    def __init__(
        self,
        timeline: TimelineModel,
        moves: list[tuple[str, str, float, float]],  # (track_id, clip_id, old_time, new_time)
        description: str = "Çoklu Klip Taşı",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.moves = moves

    def redo(self) -> None:
        affected_track_ids = set()
        for track_id, clip_id, _old_time, new_time in self.moves:
            track = self.timeline.get_track_by_id(track_id)
            if track:
                clip = track.get_clip_by_id(clip_id)
                if clip:
                    clip.move_to(new_time)
                    affected_track_ids.add(track_id)
                    self.timeline.notify_clip_modified(track_id, clip_id)

        for track_id in affected_track_ids:
            track = self.timeline.get_track_by_id(track_id)
            if track:
                track.sort_clips()
        if affected_track_ids:
            self.timeline.tracks_changed.emit()
            self.timeline.duration_changed.emit(self.timeline.duration)

    def undo(self) -> None:
        affected_track_ids = set()
        for track_id, clip_id, old_time, _new_time in self.moves:
            track = self.timeline.get_track_by_id(track_id)
            if track:
                clip = track.get_clip_by_id(clip_id)
                if clip:
                    clip.move_to(old_time)
                    affected_track_ids.add(track_id)
                    self.timeline.notify_clip_modified(track_id, clip_id)

        for track_id in affected_track_ids:
            track = self.timeline.get_track_by_id(track_id)
            if track:
                track.sort_clips()
        if affected_track_ids:
            self.timeline.tracks_changed.emit()
            self.timeline.duration_changed.emit(self.timeline.duration)


class RemoveMultipleClipsCommand(QUndoCommand):
    """Command to remove multiple clips across tracks simultaneously."""

    def __init__(
        self,
        timeline: TimelineModel,
        removals: list[tuple[str, str]],  # (track_id, clip_id)
        description: str = "Çoklu Klip Sil",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.removals = removals
        self._saved_clips: list[tuple[str, Clip]] = []

    def redo(self) -> None:
        self._saved_clips = []
        for track_id, clip_id in self.removals:
            track = self.timeline.get_track_by_id(track_id)
            if track:
                clip = track.get_clip_by_id(clip_id)
                if clip:
                    self._saved_clips.append((track_id, clip.clone(new_id=False)))
                track.remove_clip(clip_id)
                self.timeline.notify_clip_removed(track_id, clip_id)
        self.timeline.tracks_changed.emit()
        self.timeline.duration_changed.emit(self.timeline.duration)

    def undo(self) -> None:
        for track_id, saved_clip in self._saved_clips:
            track = self.timeline.get_track_by_id(track_id)
            if track:
                track.add_clip(saved_clip, allow_overlap=True)
                self.timeline.notify_clip_added(track_id, saved_clip.id)
        self.timeline.tracks_changed.emit()
        self.timeline.duration_changed.emit(self.timeline.duration)


class DetachAudioCommand(QUndoCommand):
    """Command to detach audio from a video clip into a separate audio track."""

    def __init__(
        self,
        timeline: TimelineModel,
        video_track_id: str,
        video_clip_id: str,
        description: str = "Sesi Ayır",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.video_track_id = video_track_id
        self.video_clip_id = video_clip_id
        self._audio_track_id: Optional[str] = None
        self._audio_clip_id: Optional[str] = None
        self._created_audio_track: bool = False
        self._prev_muted: bool = False
        self._prev_volume: float = 1.0

    def redo(self) -> None:
        video_track = self.timeline.get_track_by_id(self.video_track_id)
        if not video_track:
            return
        video_clip = video_track.get_clip_by_id(self.video_clip_id)
        if not video_clip:
            return

        self._prev_muted = video_clip.muted
        self._prev_volume = video_clip.volume
        video_clip.muted = True

        # Find or create an audio track
        audio_tracks = self.timeline.get_audio_tracks()
        if audio_tracks:
            audio_track = audio_tracks[0]
            self._created_audio_track = False
        else:
            audio_track = Track(
                name=f"Audio {len(self.timeline.tracks) + 1}",
                track_type=TrackType.AUDIO,
            )
            self.timeline.add_track(audio_track)
            self._created_audio_track = True

        self._audio_track_id = audio_track.id

        # Create audio clip
        audio_clip = Clip(
            media_id=video_clip.media_id,
            timeline_in=video_clip.timeline_in,
            timeline_out=video_clip.timeline_out,
            source_in=video_clip.source_in,
            source_out=video_clip.source_out,
            name=f"{video_clip.name or 'Klip'} (Ses)",
            volume=self._prev_volume,
            muted=False,
            fade_in=video_clip.fade_in,
            fade_out=video_clip.fade_out,
        )
        self._audio_clip_id = audio_clip.id
        audio_track.add_clip(audio_clip, allow_overlap=True)

        self.timeline.notify_clip_modified(self.video_track_id, self.video_clip_id)
        self.timeline.notify_clip_added(audio_track.id, audio_clip.id)
        self.timeline.tracks_changed.emit()

    def undo(self) -> None:
        video_track = self.timeline.get_track_by_id(self.video_track_id)
        if video_track:
            video_clip = video_track.get_clip_by_id(self.video_clip_id)
            if video_clip:
                video_clip.muted = self._prev_muted
                video_clip.volume = self._prev_volume
                self.timeline.notify_clip_modified(self.video_track_id, self.video_clip_id)

        if self._audio_track_id and self._audio_clip_id:
            audio_track = self.timeline.get_track_by_id(self._audio_track_id)
            if audio_track:
                audio_track.remove_clip(self._audio_clip_id)
                self.timeline.notify_clip_removed(self._audio_track_id, self._audio_clip_id)
                if self._created_audio_track and len(audio_track.clips) == 0:
                    self.timeline.remove_track(self._audio_track_id)
                else:
                    self.timeline.tracks_changed.emit()


class RippleTrimHeadCommand(QUndoCommand):
    """Command to ripple trim from clip start to playhead position."""

    def __init__(
        self,
        timeline: TimelineModel,
        track_id: str,
        clip_id: str,
        playhead_time: float,
        description: str = "Başı Çizgiye Kırp ve Kaydır",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.track_id = track_id
        self.clip_id = clip_id
        self.playhead_time = playhead_time
        self._saved_clips_snapshot: list[Clip] = []

    def redo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if not track:
            return
        clip = track.get_clip_by_id(self.clip_id)
        if not clip or not (clip.timeline_in + 0.04 < self.playhead_time <= clip.timeline_out):
            return

        self._saved_clips_snapshot = [c.clone(new_id=False) for c in track.clips]
        cut_amount = self.playhead_time - clip.timeline_in

        # Update target clip: shift source_in forward, decrease duration
        clip.source_in += cut_amount
        clip.timeline_out -= cut_amount

        # Shift all subsequent clips on this track left by cut_amount
        for other in track.clips:
            if other.id != clip.id and other.timeline_in >= self.playhead_time - 0.01:
                other.move_to(max(0.0, other.timeline_in - cut_amount))

        track.sort_clips()
        self.timeline.tracks_changed.emit()
        self.timeline.duration_changed.emit(self.timeline.duration)

    def undo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            saved_map = {c.id: c for c in self._saved_clips_snapshot}
            for clip in track.clips:
                if clip.id in saved_map:
                    saved = saved_map[clip.id]
                    clip.timeline_in = saved.timeline_in
                    clip.timeline_out = saved.timeline_out
                    clip.source_in = saved.source_in
                    clip.source_out = saved.source_out
            track.sort_clips()
            self.timeline.tracks_changed.emit()
            self.timeline.duration_changed.emit(self.timeline.duration)


class RippleTrimTailCommand(QUndoCommand):
    """Command to ripple trim from playhead position to clip end."""

    def __init__(
        self,
        timeline: TimelineModel,
        track_id: str,
        clip_id: str,
        playhead_time: float,
        description: str = "Sonu Çizgiye Kırp ve Kaydır",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.track_id = track_id
        self.clip_id = clip_id
        self.playhead_time = playhead_time
        self._saved_clips_snapshot: list[Clip] = []

    def redo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if not track:
            return
        clip = track.get_clip_by_id(self.clip_id)
        if not clip or not (clip.timeline_in <= self.playhead_time < clip.timeline_out - 0.04):
            return

        self._saved_clips_snapshot = [c.clone(new_id=False) for c in track.clips]
        cut_amount = clip.timeline_out - self.playhead_time

        # Update target clip: trim end to playhead
        clip.trim_out(self.playhead_time)

        # Shift all subsequent clips on this track left by cut_amount
        for other in track.clips:
            if other.id != clip.id and other.timeline_in >= self.playhead_time - 0.01:
                other.move_to(max(0.0, other.timeline_in - cut_amount))

        track.sort_clips()
        self.timeline.tracks_changed.emit()
        self.timeline.duration_changed.emit(self.timeline.duration)

    def undo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if track:
            saved_map = {c.id: c for c in self._saved_clips_snapshot}
            for clip in track.clips:
                if clip.id in saved_map:
                    saved = saved_map[clip.id]
                    clip.timeline_in = saved.timeline_in
                    clip.timeline_out = saved.timeline_out
                    clip.source_in = saved.source_in
                    clip.source_out = saved.source_out
            track.sort_clips()
            self.timeline.tracks_changed.emit()
            self.timeline.duration_changed.emit(self.timeline.duration)


class AddMarkerCommand(QUndoCommand):
    """Command to add a timeline marker."""

    def __init__(
        self,
        timeline: TimelineModel,
        marker_or_time: Marker | float,
        name: str = "",
        color: str = "#E07A38",
        description: str = "İşaretçi Ekle",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        if isinstance(marker_or_time, Marker):
            self.marker = marker_or_time
        else:
            self.marker = Marker(time=float(marker_or_time), name=name, color=color)

    @property
    def marker_id(self) -> str:
        return self.marker.id

    def redo(self) -> None:
        self.timeline.add_marker(self.marker)

    def undo(self) -> None:
        self.timeline.remove_marker(self.marker.id)


class RemoveMarkerCommand(QUndoCommand):
    """Command to remove a timeline marker."""

    def __init__(
        self,
        timeline: TimelineModel,
        marker_id: str,
        description: str = "İşaretçiyi Sil",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.marker_id = marker_id
        self._saved_marker: Optional[Marker] = None

    def redo(self) -> None:
        self._saved_marker = self.timeline.remove_marker(self.marker_id)

    def undo(self) -> None:
        if self._saved_marker:
            self.timeline.add_marker(self._saved_marker)


class ChangeClipSpeedCommand(QUndoCommand):
    """Command to change clip playback speed and reverse direction."""

    def __init__(
        self,
        timeline: TimelineModel,
        track_id: str,
        clip_id: str,
        new_speed: float,
        new_reverse: bool = False,
        description: str = "Klip Hızını Değiştir",
    ) -> None:
        super().__init__(description)
        self.timeline = timeline
        self.track_id = track_id
        self.clip_id = clip_id
        self.new_speed = max(0.1, min(10.0, new_speed))
        self.new_reverse = new_reverse

        self.old_speed: float = 1.0
        self.old_reverse: bool = False
        self.old_timeline_out: float = 0.0
        self.new_timeline_out: float = 0.0

    def redo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if not track:
            return
        clip = track.get_clip_by_id(self.clip_id)
        if not clip:
            return

        self.old_speed = getattr(clip, "speed", 1.0)
        self.old_reverse = getattr(clip, "reverse", False)
        self.old_timeline_out = clip.timeline_out

        source_dur = (clip.source_out or (clip.source_in + clip.duration)) - clip.source_in
        new_dur = max(0.04, source_dur / self.new_speed)
        self.new_timeline_out = clip.timeline_in + new_dur

        clip.speed = self.new_speed
        clip.reverse = self.new_reverse
        clip.timeline_out = self.new_timeline_out

        track.sort_clips()
        self.timeline.notify_clip_modified(self.track_id, self.clip_id)
        self.timeline.tracks_changed.emit()
        self.timeline.duration_changed.emit(self.timeline.duration)

    def undo(self) -> None:
        track = self.timeline.get_track_by_id(self.track_id)
        if not track:
            return
        clip = track.get_clip_by_id(self.clip_id)
        if not clip:
            return

        clip.speed = self.old_speed
        clip.reverse = self.old_reverse
        clip.timeline_out = self.old_timeline_out

        track.sort_clips()
        self.timeline.notify_clip_modified(self.track_id, self.clip_id)
        self.timeline.tracks_changed.emit()
        self.timeline.duration_changed.emit(self.timeline.duration)

