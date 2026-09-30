import os
from pathlib import Path
import subprocess
import tempfile
import pytest

os.environ["QT_QPA_PLATFORM"] = "offscreen"

from trim.app.application import CutlineApplication
from trim.core.clip import Clip
from trim.core.project import Project
from trim.export.presets import DEFAULT_PRESETS
from trim.media.ffprobe import FFprobeAnalyzer
from trim.ui.main_window import MainWindow


def create_synthetic_media(dir_path: Path):
    vid1 = dir_path / "intro.mp4"
    vid2 = dir_path / "action.mp4"
    img1 = dir_path / "photo.png"

    # 1.5s video 1 (red) with 440Hz tone
    subprocess.run([
        "ffmpeg", "-y", "-v", "quiet",
        "-f", "lavfi", "-i", "color=c=red:s=640x360:r=30:d=1.5",
        "-f", "lavfi", "-i", "sine=frequency=440:duration=1.5",
        "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac",
        str(vid1)
    ], check=True)

    # 2.0s video 2 (blue) with 880Hz tone
    subprocess.run([
        "ffmpeg", "-y", "-v", "quiet",
        "-f", "lavfi", "-i", "color=c=blue:s=640x360:r=30:d=2.0",
        "-f", "lavfi", "-i", "sine=frequency=880:duration=2.0",
        "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac",
        str(vid2)
    ], check=True)

    # Static PNG image
    subprocess.run([
        "ffmpeg", "-y", "-v", "quiet",
        "-f", "lavfi", "-i", "color=c=green:s=640x360:r=1:d=1",
        "-vframes", "1",
        str(img1)
    ], check=True)

    return vid1, vid2, img1


def test_full_nle_workflow():
    app = CutlineApplication.instance() or CutlineApplication([])

    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)
        vid1, vid2, img1 = create_synthetic_media(tmp_path)

        win = MainWindow()
        win.show()

        # 1. Import media into Bin
        m1 = win.media_bin.import_file(str(vid1))
        m2 = win.media_bin.import_file(str(vid2))
        m3 = win.media_bin.import_file(str(img1))

        assert m1 is not None and m1.exists()
        assert m2 is not None and m2.exists()
        assert m3 is not None and m3.exists()
        assert len(win.project.media_pool) == 3

        # 2. Add clips to Timeline
        v_track = win.project.timeline.get_video_tracks()[0]

        # Clip 1: 0.0 to 1.5s
        c1 = Clip(media_id=m1.id, timeline_in=0.0, timeline_out=1.5, name="Intro")
        # Clip 2: 1.5s to 3.5s
        c2 = Clip(media_id=m2.id, timeline_in=1.5, timeline_out=3.5, name="Action")
        # Clip 3: 3.5s to 5.0s (Image)
        c3 = Clip(media_id=m3.id, timeline_in=3.5, timeline_out=5.0, name="Photo")

        v_track.add_clip(c1)
        v_track.add_clip(c2)
        v_track.add_clip(c3)
        win.project.timeline.notify_clip_added(v_track.id, c1.id)
        win.project.timeline.notify_clip_added(v_track.id, c2.id)
        win.project.timeline.notify_clip_added(v_track.id, c3.id)

        assert abs(win.project.timeline.duration - 5.0) < 1e-4

        # 3. Test Playback seek and frame decoding
        frames_received = []
        win.playback_engine.frame_ready.connect(lambda frame, t: frames_received.append((frame, t)))

        # Seek within clip 1 (red)
        win.playback_engine.seek(0.5)
        assert len(frames_received) >= 1
        f1, t1 = frames_received[-1]
        assert not f1.isNull()
        assert abs(t1 - 0.5) < 1e-4

        # Seek within clip 2 (blue)
        win.playback_engine.seek(2.0)
        f2, t2 = frames_received[-1]
        assert not f2.isNull()
        assert abs(t2 - 2.0) < 1e-4

        # Seek within clip 3 (image)
        win.playback_engine.seek(4.0)
        f3, t3 = frames_received[-1]
        assert not f3.isNull()
        assert abs(t3 - 4.0) < 1e-4

        # 4. Test Split at Playhead
        win.playback_engine.seek(0.75)
        win.timeline_widget.canvas.split_at_playhead()
        assert len(v_track.clips) == 4
        assert v_track.clips[0].timeline_out == 0.75
        assert v_track.clips[1].timeline_in == 0.75

        # 5. Test Undo Split
        assert win.undo_stack.canUndo()
        win.undo_stack.undo()
        assert len(v_track.clips) == 3
        assert v_track.clips[0].timeline_out == 1.5

        # 6. Test Save Project
        project_file = tmp_path / "test_session.trim"
        win.save_project_as = lambda: win.project.save(str(project_file))
        win.save_project_as()
        assert project_file.is_file()

        # 7. Test Reloading Project
        reloaded = Project.load(str(project_file))
        assert len(reloaded.media_pool) == 3
        reloaded_v = reloaded.timeline.get_video_tracks()[0]
        assert len(reloaded_v.clips) == 3
        assert abs(reloaded.timeline.duration - 5.0) < 1e-4

        # 8. Test Export to MP4
        out_mp4 = tmp_path / "export_final.mp4"
        preset = DEFAULT_PRESETS[1]  # 720p fast
        from trim.export.exporter import TimelineExporter
        cmd, dur = TimelineExporter.build_export_pipeline(win.project, preset, str(out_mp4))
        res = subprocess.run(cmd, capture_output=True, text=True)
        assert res.returncode == 0, f"Export failed: {res.stderr}"

        assert out_mp4.is_file()
        meta = FFprobeAnalyzer.analyze(out_mp4)
        assert meta.width == 1280
        assert meta.height == 720
        assert abs(meta.duration - 5.0) < 0.2
        assert meta.video_codec == "h264"
        assert meta.audio_codec == "aac"

        win.playback_engine.close()
        win.close()
