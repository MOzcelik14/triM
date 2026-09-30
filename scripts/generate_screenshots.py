#!/usr/bin/env python3
"""
Generate high-resolution native screenshots of triM. for documentation and GitHub Pages.
"""
import os
import sys
import uuid
import traceback
from pathlib import Path

# Ensure project root is in sys.path
PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

os.environ["QT_QPA_PLATFORM"] = "offscreen"

from PySide6.QtWidgets import QApplication
from PySide6.QtCore import Qt, QRectF
from PySide6.QtGui import QImage, QPainter, QColor, QFont

from trim.ui.main_window import MainWindow
from trim.core.media import MediaItem, MediaType
from trim.core.clip import Clip
from trim.media.waveform import WaveformGenerator
import numpy as np


def create_sample_title_image(text: str, file_path: str, subtitle: str = "") -> None:
    img = QImage(1920, 1080, QImage.Format.Format_ARGB32_Premultiplied)
    img.fill(QColor(24, 24, 30, 255))
    painter = QPainter(img)
    painter.setRenderHint(QPainter.RenderHint.Antialiasing, True)
    painter.setRenderHint(QPainter.RenderHint.TextAntialiasing, True)

    # Accent bar
    painter.fillRect(QRectF(160, 480, 8, 120), QColor("#E07A38"))

    font = QFont("Sans-Serif", 42, QFont.Weight.Bold)
    painter.setFont(font)
    painter.setPen(QColor(255, 255, 255))
    painter.drawText(QRectF(190, 480, 1500, 60), Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter, text)

    if subtitle:
        font2 = QFont("Sans-Serif", 22, QFont.Weight.Normal)
        painter.setFont(font2)
        painter.setPen(QColor(180, 180, 190))
        painter.drawText(QRectF(190, 545, 1500, 45), Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter, subtitle)

    painter.end()
    img.save(file_path, "PNG")


def main():
    try:
        from trim.core.autosave import AutosaveManager
        AutosaveManager.has_recovery_file = lambda self: False

        app = QApplication.instance() or QApplication([])

        output_dir = PROJECT_ROOT / "docs" / "assets" / "screenshots"
        output_dir.mkdir(parents=True, exist_ok=True)

        # 1. Empty State Screenshot
        win = MainWindow()
        win.resize(1366, 768)
        win.show()
        app.processEvents()

        empty_path = output_dir / "trim_empty_state.png"
        pix_empty = win.grab()
        pix_empty.save(str(empty_path))
        print(f"Captured empty state: {empty_path}")

        # 2. Populated Timeline Screenshot
        titles_dir = Path.home() / ".local" / "share" / "trim" / "titles"
        titles_dir.mkdir(parents=True, exist_ok=True)

        img1_path = str(titles_dir / "demo_intro.png")
        create_sample_title_image("triM. Sequence 01", img1_path, "Cinematic Cut / Linux Non-Linear Video Editor")
        media1 = MediaItem(
            id=str(uuid.uuid4()),
            file_path=img1_path,
            name="Metin: triM. Sequence 01",
            media_type=MediaType.IMAGE,
            duration=5.0,
            width=1920,
            height=1080,
        )
        win.project.add_media(media1)

        img2_path = str(titles_dir / "demo_broll.png")
        create_sample_title_image("B-Roll: Night Horizon", img2_path, "ISO 800 - 24fps - Rec.709")
        media2 = MediaItem(
            id=str(uuid.uuid4()),
            file_path=img2_path,
            name="Metin: B-Roll Night Horizon",
            media_type=MediaType.IMAGE,
            duration=8.0,
            width=1920,
            height=1080,
        )
        win.project.add_media(media2)

        audio_path = str(titles_dir / "Soundtrack_Ambient.wav")
        media_audio = MediaItem(
            id=str(uuid.uuid4()),
            file_path=audio_path,
            name="Soundtrack_Ambient.wav",
            media_type=MediaType.AUDIO,
            duration=12.0,
            sample_rate=48000,
            channels=2,
        )
        win.project.add_media(media_audio)

        # Add clips to timeline tracks
        v_tracks = win.project.timeline.get_video_tracks()
        if v_tracks:
            clip1 = Clip(
                media_id=media1.id,
                name=media1.name,
                timeline_in=0.0,
                timeline_out=4.5,
                source_in=0.0,
                fade_in=0.5,
                fade_out=0.5,
                brightness=0.05,
                contrast=1.15,
                saturation=1.1,
            )
            v_tracks[0].add_clip(clip1)

            clip2 = Clip(
                media_id=media2.id,
                name=media2.name,
                timeline_in=4.5,
                timeline_out=11.5,
                source_in=0.0,
                fade_in=0.5,
                fade_out=0.8,
                transition_in="dip_black",
                contrast=1.1,
            )
            v_tracks[0].add_clip(clip2)

        a_tracks = win.project.timeline.get_audio_tracks()
        if a_tracks:
            clip_audio = Clip(
                media_id=media_audio.id,
                name=media_audio.name,
                timeline_in=0.0,
                timeline_out=11.5,
                source_in=0.0,
                fade_in=1.0,
                fade_out=1.5,
                volume=0.9,
            )
            a_tracks[0].add_clip(clip_audio)

            # Pre-cache synthetic waveform
            wf_gen = WaveformGenerator()
            cache_file = wf_gen._get_cache_path(media_audio.file_path)
            cache_file.parent.mkdir(parents=True, exist_ok=True)
            num_points = int(12.0 * 100)
            t = np.linspace(0, 10, num_points)
            max_peaks = np.abs(np.sin(t * 3.5) * np.cos(t * 1.8)) * 0.72 + np.random.uniform(0.04, 0.12, num_points)
            min_peaks = -max_peaks
            wf_data = np.column_stack([min_peaks, max_peaks]).astype(np.float32)
            wf_data.tofile(cache_file)

        # Set canvas and ruler playhead
        win.timeline_widget.canvas.playhead_time = 2.5
        win.timeline_widget.time_ruler.playhead_time = 2.5
        win.timeline_widget.canvas.selected_clip = clip1
        win.inspector.inspect_clip(v_tracks[0].id, clip1.id)

        win.transport_bar.lbl_timecode.setText("00:00:02:15")
        win.transport_bar.lbl_duration.setText("/ 00:00:11:15")
        win.vu_meter.set_levels(-14.2, -11.8)

        # Show rendered image on monitor preview
        preview_img = QImage(img1_path)
        win.monitor.set_frame(preview_img, 2.5)

        win.timeline_widget.canvas.update()
        win.timeline_widget.time_ruler.update()
        app.processEvents()

        timeline_path = output_dir / "trim_timeline.png"
        pix_timeline = win.grab()
        pix_timeline.save(str(timeline_path))
        print(f"Captured timeline state: {timeline_path}")

    except Exception:
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
