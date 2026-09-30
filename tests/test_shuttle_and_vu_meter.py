import pytest
from trim.core.clip import Clip
from trim.core.project import Project
from trim.core.track import Track, TrackType
from trim.media.playback import PlaybackEngine
from trim.ui.main_window import MainWindow
from trim.ui.preview.vu_meter import AudioVUMeterWidget


def test_vu_meter_levels_and_ballistics(qapp):
    meter = AudioVUMeterWidget()
    assert meter._current_l == -60.0
    assert meter._current_r == -60.0

    # Set new levels
    meter.set_levels(-12.0, -6.0)
    assert meter._target_l == -12.0
    assert meter._target_r == -6.0
    assert meter._peak_l == -12.0
    assert meter._peak_r == -6.0

    # Decay ticks
    for _ in range(5):
        meter._on_decay_tick()

    # Current level should have risen towards target
    assert meter._current_l > -60.0
    assert meter._current_r > -60.0

    # Reset
    meter.reset()
    assert meter._target_l == -60.0
    assert meter._current_l == -60.0


def test_shuttle_playback_speed_and_signals(qapp):
    project = Project()
    engine = PlaybackEngine(project)

    speeds = []
    engine.speed_changed.connect(lambda s: speeds.append(s))

    # Test speed change
    engine.set_speed(2.0)
    assert engine.speed == 2.0
    assert 2.0 in speeds

    # Test pause resets speed to 1.0
    engine.pause()
    assert engine.speed == 1.0
    assert 1.0 in speeds

    # Test reverse shuttle
    engine.set_speed(-2.0)
    assert engine.speed == -2.0
    assert -2.0 in speeds
    engine.pause()


def test_main_window_shuttle_and_edit_points(qapp):
    project = Project()
    track = Track(name="Video 1", track_type=TrackType.VIDEO)
    c1 = Clip(media_id="m1", timeline_in=0.0, timeline_out=5.0)
    c2 = Clip(media_id="m2", timeline_in=8.0, timeline_out=12.0)
    track.add_clip(c1)
    track.add_clip(c2)
    project.timeline.add_track(track)

    win = MainWindow(project)

    # Test shuttle cycling forward: 1x -> 2x -> 4x -> 8x
    win._on_shuttle_forward()
    assert win.playback_engine.speed == 1.0
    win._on_shuttle_forward()
    assert win.playback_engine.speed == 2.0
    win._on_shuttle_forward()
    assert win.playback_engine.speed == 4.0
    win._on_shuttle_forward()
    assert win.playback_engine.speed == 8.0
    win.playback_engine.pause()

    # Test shuttle cycling reverse: -1x -> -2x -> -4x -> -8x
    win._on_shuttle_reverse()
    assert win.playback_engine.speed == -1.0
    win._on_shuttle_reverse()
    assert win.playback_engine.speed == -2.0
    win._on_shuttle_reverse()
    assert win.playback_engine.speed == -4.0
    win._on_shuttle_reverse()
    assert win.playback_engine.speed == -8.0
    win.playback_engine.pause()

    # Test edit point navigation
    # Points: [0.0, 5.0, 8.0, 12.0]
    win.playback_engine.seek(0.0)
    win._jump_to_next_edit_point()
    assert abs(win.playback_engine.current_time - 5.0) < 0.05

    win._jump_to_next_edit_point()
    assert abs(win.playback_engine.current_time - 8.0) < 0.05

    win._jump_to_next_edit_point()
    assert abs(win.playback_engine.current_time - 12.0) < 0.05

    win._jump_to_prev_edit_point()
    assert abs(win.playback_engine.current_time - 8.0) < 0.05

    win._jump_to_prev_edit_point()
    assert abs(win.playback_engine.current_time - 5.0) < 0.05

    win._jump_to_prev_edit_point()
    assert abs(win.playback_engine.current_time - 0.0) < 0.05

    win.close()
