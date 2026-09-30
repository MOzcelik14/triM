from PySide6.QtGui import QUndoStack
from trim.core.clip import Clip
from trim.core.timeline import TimelineModel
from trim.core.track import Track, TrackType
from trim.commands.timeline_commands import (
    AddClipCommand,
    RemoveClipCommand,
    MoveClipCommand,
    TrimClipCommand,
    SplitClipCommand,
    RippleDeleteCommand,
)


def test_undo_redo_add_remove():
    timeline = TimelineModel()
    track = Track(name="V1", track_type=TrackType.VIDEO)
    timeline.add_track(track)
    undo_stack = QUndoStack()

    clip = Clip(media_id="m1", timeline_in=0.0, timeline_out=5.0)
    cmd = AddClipCommand(timeline, track.id, clip)
    undo_stack.push(cmd)

    assert len(track.clips) == 1
    assert track.clips[0].id == clip.id

    # Undo
    undo_stack.undo()
    assert len(track.clips) == 0

    # Redo
    undo_stack.redo()
    assert len(track.clips) == 1

    # Remove
    del_cmd = RemoveClipCommand(timeline, track.id, clip.id)
    undo_stack.push(del_cmd)
    assert len(track.clips) == 0

    # Undo remove
    undo_stack.undo()
    assert len(track.clips) == 1


def test_undo_redo_split_and_move():
    timeline = TimelineModel()
    track = Track(name="V1", track_type=TrackType.VIDEO)
    timeline.add_track(track)
    undo_stack = QUndoStack()

    clip = Clip(media_id="m1", timeline_in=0.0, timeline_out=10.0, source_in=0.0)
    track.add_clip(clip)

    # Split at 4.0
    split_cmd = SplitClipCommand(timeline, track.id, clip.id, 4.0)
    undo_stack.push(split_cmd)

    assert len(track.clips) == 2
    assert track.clips[0].timeline_out == 4.0
    assert track.clips[1].timeline_in == 4.0

    # Move right clip from 4.0 to 6.0
    right_clip_id = track.clips[1].id
    move_cmd = MoveClipCommand(timeline, track.id, right_clip_id, 4.0, 6.0)
    undo_stack.push(move_cmd)

    assert track.clips[1].timeline_in == 6.0

    # Undo move
    undo_stack.undo()
    assert track.clips[1].timeline_in == 4.0

    # Undo split
    undo_stack.undo()
    assert len(track.clips) == 1
    assert track.clips[0].timeline_out == 10.0
