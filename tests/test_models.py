import pytest
from cutline.core.clip import Clip
from cutline.core.track import Track, TrackType
from cutline.core.timeline import TimelineModel
from cutline.core.media import MediaItem, MediaType


def test_clip_duration_and_mapping():
    clip = Clip(media_id="m1", timeline_in=5.0, timeline_out=15.0, source_in=2.0)
    assert clip.duration == 10.0
    assert clip.source_out == 12.0
    assert clip.contains_timeline_time(5.0)
    assert clip.contains_timeline_time(10.0)
    assert not clip.contains_timeline_time(15.0)
    assert clip.map_timeline_to_source(5.0) == 2.0
    assert clip.map_timeline_to_source(10.0) == 7.0


def test_clip_move_and_trim():
    clip = Clip(media_id="m1", timeline_in=10.0, timeline_out=20.0, source_in=0.0)
    clip.move_to(25.0)
    assert clip.timeline_in == 25.0
    assert clip.timeline_out == 35.0
    assert clip.source_in == 0.0
    assert clip.source_out == 10.0

    # Trim in (move start from 25 to 27)
    clip.trim_in(27.0)
    assert clip.timeline_in == 27.0
    assert clip.source_in == 2.0
    assert clip.duration == 8.0

    # Trim out (move end from 35 to 32)
    clip.trim_out(32.0)
    assert clip.timeline_out == 32.0
    assert clip.duration == 5.0
    assert clip.source_out == 7.0


def test_clip_split():
    clip = Clip(media_id="m1", timeline_in=10.0, timeline_out=20.0, source_in=5.0)
    left, right = clip.split(14.0)

    assert left.timeline_in == 10.0
    assert left.timeline_out == 14.0
    assert left.source_in == 5.0
    assert left.source_out == 9.0

    assert right.timeline_in == 14.0
    assert right.timeline_out == 20.0
    assert right.source_in == 9.0
    assert right.source_out == 15.0


def test_track_operations():
    track = Track(name="Video 1", track_type=TrackType.VIDEO)
    c1 = Clip(media_id="m1", timeline_in=0.0, timeline_out=5.0)
    c2 = Clip(media_id="m2", timeline_in=10.0, timeline_out=15.0)

    assert track.add_clip(c1)
    assert track.add_clip(c2)
    assert track.duration == 15.0

    # Collision test
    c_colliding = Clip(media_id="m3", timeline_in=3.0, timeline_out=7.0)
    assert track.check_collision(c_colliding)
    assert not track.add_clip(c_colliding, allow_overlap=False)

    # Split clip on track
    left, right = track.split_clip(c1.id, 2.5)
    assert len(track.clips) == 3
    assert track.clips[0].timeline_out == 2.5
    assert track.clips[1].timeline_in == 2.5

    # Ripple delete left clip
    track.ripple_delete(left.id)
    assert len(track.clips) == 2
    # The right clip was at 2.5-5.0 (duration 2.5), c2 was at 10.0-15.0
    # Left clip had duration 2.5, so subsequent clips shift left by 2.5
    assert track.clips[0].timeline_in == 0.0
    assert track.clips[1].timeline_in == 7.5


def test_timeline_timecode_and_snapping():
    timeline = TimelineModel(fps=30.0)
    track = Track(name="V1", track_type=TrackType.VIDEO)
    c1 = Clip(media_id="m1", timeline_in=0.0, timeline_out=4.0)
    c2 = Clip(media_id="m2", timeline_in=5.0, timeline_out=10.0)
    track.add_clip(c1)
    track.add_clip(c2)
    timeline.add_track(track)

    assert timeline.time_to_timecode(0.0) == "00:00:00:00"
    assert timeline.time_to_timecode(1.5) == "00:00:01:15"
    assert timeline.time_to_timecode(65.0) == "00:01:05:00"

    assert abs(timeline.timecode_to_time("00:01:05:00") - 65.0) < 1e-4

    # Snapping
    # Near 4.0 (clip 1 end)
    assert timeline.snap_time(3.95, threshold=0.1) == 4.0
    # Far from any edge
    assert timeline.snap_time(2.5, threshold=0.1) == 2.5
