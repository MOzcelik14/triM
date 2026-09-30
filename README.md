# triM.

A lightweight non-linear video editor for Linux.

Built with Python 3.12, PySide6 (Qt6), FFmpeg, and PyAV. Designed with a minimal, dark native desktop interface focusing on speed, precision, and low visual clutter.

---

## Features

- **Multi-Track Timeline:** Independent video and audio tracks with split (cut), ripple delete, frame-snapping, and drag-and-drop sequencing.
- **Audio Waveforms:** Multi-threaded audio peak extraction with cached waveform visualizations directly on timeline clips.
- **Interactive Fade Handles:** Visual fade in/out drag handles with live audio gain ramps and video opacity curves.
- **Program Monitor & VU Meter:** Frame-accurate preview with letterboxing, timecode display, and dual-channel real-time RMS dBFS audio VU meters with peak hold indicators.
- **Precision Shuttle Navigation:** Industry-standard J-K-L shuttle playback (`1x`, `2x`, `4x`, `8x`, `-1x`, `-2x`, `-4x`, `-8x`), single-frame stepping (`←` / `→`), and edit-point jumping (`↑` / `↓`).
- **Color & Transition Controls:** Live brightness, contrast, and saturation adjustments (including instant black-and-white conversion) and smooth transitions (Dip to Black, Dip to White).
- **Typography & Titles:** Built-in title generator with system font selection, lower-third presets, alignment, and alpha overlays.
- **Native FFmpeg Export:** Background rendering pipeline with resolution/framerate presets, progress tracking, and filter-graph compilation (`fade`, `afade`, `eq`, `scale`, `pad`).
- **Crash Recovery & Autosave:** Periodic background project state snapshots and recovery detection.

---

## Installation

### Prerequisites

Ensure the following system dependencies are installed:

- Python 3.12 or newer
- FFmpeg and FFprobe (`ffmpeg`, `ffprobe` in `$PATH`)

On Ubuntu / Debian:
```bash
sudo apt update
sudo apt install python3 python3-venv ffmpeg
```

On Fedora:
```bash
sudo dnf install python3 ffmpeg
```

On Arch Linux:
```bash
sudo pacman -S python ffmpeg
```

### Setup

Clone the repository and run the launcher script:

```bash
git clone https://github.com/MOzcelik14/VideoEditor.git
cd VideoEditor
chmod +x run.sh
./run.sh
```

`run.sh` automatically creates a virtual environment, installs required dependencies, and launches the editor.

Manual installation:
```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -e .
trim
```

---

## Development

Run tests using `pytest`:

```bash
.venv/bin/pytest tests/ -v
```

Generate brand assets and icons:

```bash
.venv/bin/python3 scripts/generate_branding_and_icons.py
```

Code formatting and syntax validation:

```bash
.venv/bin/python3 -m py_compile trim/**/*.py tests/*.py
```

---

## Roadmap

- [x] Core NLE architecture (Timeline, Media Bin, Monitor, Inspector)
- [x] Frame-accurate audio/video synchronization and playback
- [x] Audio waveforms and interactive fade handles
- [x] Multi-track composition, transforms, and title generator
- [x] Stereo audio VU metering and J-K-L shuttle playback
- [x] Color adjustments and video transitions
- [x] triM. brand identity, icon system, and FreeDesktop integration
- [ ] Flatpak package distribution (`io.github.mozcelik14.triM`)
- [ ] Multi-clip group selection and batch moving
- [ ] Keyframe animation for position, scale, and opacity
- [ ] Proxy media workflow for 4K editing on low-spec hardware

---

## License

MIT License. See [LICENSE](LICENSE) for details.
