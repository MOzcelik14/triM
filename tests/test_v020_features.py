from __future__ import annotations

import pytest
from PySide6.QtCore import QPointF, QRectF, Qt
from PySide6.QtGui import QUndoStack

from trim.commands.timeline_commands import (
    DetachAudioCommand,
    MoveMultipleClipsCommand,
    RemoveMultipleClipsCommand,
    RippleTrimHeadCommand,
    RippleTrimTailCommand,
)
from trim.core.clip import Clip
from trim.core.project import Project
from trim.core.track import Track, TrackType
from trim.ui.main_window import MainWindow
from trim.ui.timeline.timeline_canvas import DragMode, TimelineCanvas
from trim.ui.timeline.track_header import SingleTrackHeader


def test_track_mixer_fields_and_serialization():
    """Verify track solo and volume fields and json serialization."""
    track = Track(name="Voiceover", track_type=TrackType.AUDIO, muted=True, solo=True, volume=1.25)
    data = track.to_dict()
    assert data["solo"] is True
    assert data["muted"] is True
    assert data["volume"] == 1.25

    restored = Track.from_dict(data)
    assert restored.solo is True
    assert restored.muted is True
    assert restored.volume == 1.25


def test_timeline_solo_audio_selection():
    """Verify get_active_audio_clips_at prioritizes soloed tracks."""
    project = Project()
    t1 = Track(name="Track 1", track_type=TrackType.AUDIO, solo=False)
    t2 = Track(name="Track 2", track_type=TrackType.AUDIO, solo=True)
    c1 = Clip(media_id="m1", timeline_in=0.0, timeline_out=10.0, name="Clip 1")
    c2 = Clip(media_id="m2", timeline_in=0.0, timeline_out=10.0, name="Clip 2")
    t1.add_clip(c1)
    t2.add_clip(c2)
    project.timeline.add_track(t1)
    project.timeline.add_track(t2)

    # Because t2 is solo, only t2's clip should be returned
    active = project.timeline.get_active_audio_clips_at(5.0)
    assert len(active) == 1
    assert active[0][1].id == c2.id

    # If t2 solo is turned off, both are returned
    t2.solo = False
    active_both = project.timeline.get_active_audio_clips_at(5.0)
    assert len(active_both) == 2


def test_single_track_header_controls(qapp):
    """Verify SingleTrackHeader UI elements for audio tracks."""
    track = Track(name="Music", track_type=TrackType.AUDIO, volume=1.0)
    header = SingleTrackHeader(track)

    assert hasattr(header, "btn_mute")
    assert hasattr(header, "btn_solo")
    assert hasattr(header, "vol_slider")

    # Change volume slider
    header.vol_slider.setValue(120)
    assert track.volume == 1.2
    assert "120%" in header.lbl_vol.text()

    # Toggle Mute
    header.btn_mute.click()
    assert track.muted is True

    # Toggle Solo
    header.btn_solo.click()
    assert track.solo is True


def test_move_multiple_clips_command():
    """Verify atomic batch move of multiple clips with Undo/Redo."""
    project = Project()
    t1 = Track(name="V1", track_type=TrackType.VIDEO)
    t2 = Track(name="V2", track_type=TrackType.VIDEO)
    c1 = Clip(media_id="m1", timeline_in=2.0, timeline_out=5.0)
    c2 = Clip(media_id="m2", timeline_in=4.0, timeline_out=8.0)
    t1.add_clip(c1)
    t2.add_clip(c2)
    project.timeline.add_track(t1)
    project.timeline.add_track(t2)

    undo_stack = QUndoStack()
    moves = [
        (t1.id, c1.id, 2.0, 5.0),
        (t2.id, c2.id, 4.0, 7.0),
    ]
    cmd = MoveMultipleClipsCommand(project.timeline, moves)
    undo_stack.push(cmd)

    assert c1.timeline_in == 5.0
    assert c1.timeline_out == 8.0
    assert c2.timeline_in == 7.0
    assert c2.timeline_out == 11.0

    undo_stack.undo()
    assert c1.timeline_in == 2.0
    assert c2.timeline_in == 4.0

    undo_stack.redo()
    assert c1.timeline_in == 5.0
    assert c2.timeline_in == 7.0


def test_remove_multiple_clips_command():
    """Verify batch deletion with Undo/Redo."""
    project = Project()
    t1 = Track(name="V1", track_type=TrackType.VIDEO)
    c1 = Clip(media_id="m1", timeline_in=0.0, timeline_out=3.0)
    c2 = Clip(media_id="m2", timeline_in=4.0, timeline_out=7.0)
    t1.add_clip(c1)
    t1.add_clip(c2)
    project.timeline.add_track(t1)

    undo_stack = QUndoStack()
    cmd = RemoveMultipleClipsCommand(project.timeline, [(t1.id, c1.id), (t1.id, c2.id)])
    undo_stack.push(cmd)

    assert len(t1.clips) == 0

    undo_stack.undo()
    assert len(t1.clips) == 2
    assert t1.get_clip_by_id(c1.id) is not None
    assert t1.get_clip_by_id(c2.id) is not None


def test_detach_audio_command():
    """Verify detaching video audio to audio track and undoing cleanly."""
    project = Project()
    project.timeline.tracks.clear()
    v_track = Track(name="Video 1", track_type=TrackType.VIDEO)
    v_clip = Clip(media_id="m1", timeline_in=2.0, timeline_out=8.0, source_in=1.0, source_out=7.0, name="MyVideo")
    v_track.add_clip(v_clip)
    project.timeline.add_track(v_track)

    undo_stack = QUndoStack()
    cmd = DetachAudioCommand(project.timeline, v_track.id, v_clip.id)
    undo_stack.push(cmd)

    # Video clip should be muted
    assert v_clip.muted is True

    # Audio track should have been created
    audio_tracks = project.timeline.get_audio_tracks()
    assert len(audio_tracks) == 1
    a_track = audio_tracks[0]
    assert len(a_track.clips) == 1
    a_clip = a_track.clips[0]
    assert a_clip.timeline_in == 2.0
    assert a_clip.timeline_out == 8.0
    assert a_clip.source_in == 1.0
    assert a_clip.source_out == 7.0
    assert "Ses" in a_clip.name

    # Undo
    undo_stack.undo()
    assert v_clip.muted is False
    assert len(project.timeline.get_audio_tracks()) == 0


def test_ripple_trim_head_command():
    """Verify RippleTrimHeadCommand trims clip start to playhead and shifts subsequent clips left."""
    project = Project()
    t = Track(name="V1", track_type=TrackType.VIDEO)
    c1 = Clip(media_id="m1", timeline_in=2.0, timeline_out=8.0, source_in=0.0, source_out=6.0)
    c2 = Clip(media_id="m2", timeline_in=10.0, timeline_out=15.0, source_in=0.0, source_out=5.0)
    t.add_clip(c1)
    t.add_clip(c2)
    project.timeline.add_track(t)

    undo_stack = QUndoStack()
    # Playhead at 4.0: cuts 2.0s from head of c1
    cmd = RippleTrimHeadCommand(project.timeline, t.id, c1.id, playhead_time=4.0)
    undo_stack.push(cmd)

    assert c1.timeline_in == 2.0
    assert c1.timeline_out == 6.0  # was 8.0 - 2.0 cut
    assert c1.source_in == 2.0    # shifted by 2.0
    assert c1.duration == 4.0
    assert c2.timeline_in == 8.0  # shifted left by 2.0 (10.0 - 2.0)

    undo_stack.undo()
    assert c1.timeline_in == 2.0
    assert c1.timeline_out == 8.0
    assert c1.source_in == 0.0
    assert c2.timeline_in == 10.0


def test_ripple_trim_tail_command():
    """Verify RippleTrimTailCommand trims clip tail to playhead and shifts subsequent clips left."""
    project = Project()
    t = Track(name="V1", track_type=TrackType.VIDEO)
    c1 = Clip(media_id="m1", timeline_in=2.0, timeline_out=8.0, source_in=0.0, source_out=6.0)
    c2 = Clip(media_id="m2", timeline_in=8.0, timeline_out=14.0, source_in=0.0, source_out=6.0)
    t.add_clip(c1)
    t.add_clip(c2)
    project.timeline.add_track(t)

    undo_stack = QUndoStack()
    # Playhead at 5.0: trims end of c1 to 5.0 (3.0s cut)
    cmd = RippleTrimTailCommand(project.timeline, t.id, c1.id, playhead_time=5.0)
    undo_stack.push(cmd)

    assert c1.timeline_in == 2.0
    assert c1.timeline_out == 5.0
    assert c1.duration == 3.0
    assert c2.timeline_in == 5.0  # shifted left from 8.0 to 5.0

    undo_stack.undo()
    assert c1.timeline_in == 2.0
    assert c1.timeline_out == 8.0
    assert c2.timeline_in == 8.0


def test_timeline_canvas_multi_selection_and_properties(qapp):
    """Verify TimelineCanvas multi-selection properties, compatibility, and clearing."""
    project = Project()
    t1 = Track(name="V1", track_type=TrackType.VIDEO)
    c1 = Clip(media_id="m1", timeline_in=0.0, timeline_out=5.0)
    c2 = Clip(media_id="m2", timeline_in=6.0, timeline_out=10.0)
    t1.add_clip(c1)
    t1.add_clip(c2)
    project.timeline.add_track(t1)

    canvas = TimelineCanvas(project, QUndoStack())
    assert canvas.selected_clip is None

    # Select single clip via setter
    canvas.selected_clip = c1
    assert canvas.selected_clip == c1
    assert len(canvas.selected_clips) == 1
    assert canvas.selected_track == t1

    # Multi-select both clips
    canvas.selected_clips = [(t1, c1), (t1, c2)]
    assert len(canvas.selected_clips) == 2
    assert canvas.selected_clip == c1

    # Clear selection
    canvas.clear_selection()
    assert canvas.selected_clip is None
    assert len(canvas.selected_clips) == 0


def test_main_window_qw_ripple_shortcuts(qapp):
    """Verify Q and W shortcuts on MainWindow."""
    project = Project()
    t = Track(name="V1", track_type=TrackType.VIDEO)
    c1 = Clip(media_id="m1", timeline_in=0.0, timeline_out=10.0)
    t.add_clip(c1)
    project.timeline.add_track(t)

    win = MainWindow(project)
    win.playback_engine.seek(3.0)

    # Q shortcut: Ripple trim head
    win._on_ripple_trim_head()
    assert c1.duration == 7.0
    assert c1.source_in == 3.0

    # W shortcut: Ripple trim tail at 4.0
    win.playback_engine.seek(4.0)
    win._on_ripple_trim_tail()
    assert c1.timeline_out == 4.0
    assert c1.duration == 4.0
