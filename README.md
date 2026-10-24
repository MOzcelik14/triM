# triM

`triM` is a native, non-linear video editor (NLE) for Linux written in Rust. It uses `egui` and `eframe` over an OpenGL (`glow`) rendering backend for the user interface, `ffmpeg` and `ffprobe` for media decoding, filtering, and encoding, and PipeWire (`pw-cat`), PulseAudio (`paplay`), or ALSA (`aplay`) for PCM audio output. It contains no web runtime, Electron, or WebKit dependencies.

---

## Technical Capabilities

- **Timeline Editing:** Multi-track video and audio timeline, ripple delete, razor split, slip edit, magnetic snapping, clip linking/unlinking, track locking/solo/mute, and timeline markers.
- **Compositing & Color:** Software preview and hardware/software FFmpeg export for Chroma Key (`colorkey`), 3-Way Color Wheels (Lift, Gamma, Gain, Luminance), `.cube` 3D LUT application, opacity fades, and keyframed 2D transforms (`position`, `scale`, `rotation`, `opacity`).
- **Audio Processing:** Real-time software PCM mixer (`44.1 kHz` stereo `f32le`) with 3-band biquad parametric EQ (Low Shelf, Peaking Mid, High Shelf), stereo panning, pitch-preserving speed scaling, voiceover recording, and EBU R128 loudness normalization (`loudnorm`) on export.
- **Subtitles & Automated Cuts:** `.srt` and `.vtt` parser/exporter, on-monitor interactive text overlay editor, FFmpeg `scdet` scene cut detection, `silencedetect` speech interval extraction, and external Whisper CLI integration for speech-to-subtitle generation.
- **Interchange & Export:** Multi-codec rendering (`H.264`, `H.265/HEVC`, `VP9`, `AV1`, `ProRes`, `DNxHR`, `NVENC`, `VAAPI`, `QSV`), CMX 3600 `.edl` export, and Final Cut Pro `.fcpxml` (v1.9) export.

---

## System Requirements

### Supported Target Platform
- **OS:** Linux (`x86_64`)
- **Display Server:** Wayland or X11 with OpenGL 3.3+ (`libGL` / Mesa DRI)
- **Rust Toolchain:** `rustc` / `cargo` >= 1.80 (2021 edition)

### Runtime Dependencies
- `ffmpeg` and `ffprobe` (in `$PATH`)
- Audio output backend (resolved in order):
  1. PipeWire (`pw-cat`)
  2. PulseAudio (`paplay`, `parecord`)
  3. ALSA (`aplay`, `arecord`)
- Optional runtime binaries for auxiliary features:
  - `whisper` or `whisper-ctranslate2` (for local speech transcription)

### Build Dependencies (Debian / Ubuntu)
```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  pkg-config \
  libasound2-dev \
  libpipewire-0.3-dev \
  libx11-dev \
  libgl1-mesa-dev \
  libxkbcommon-dev \
  libwayland-dev \
  ffmpeg
```

---

## Build, Test, and Execution

Clone the repository and build the workspace in release mode:

```bash
git clone https://github.com/MOzcelik14/triM.git
cd triM
cargo build --workspace --release
```

Run the binary directly:

```bash
./target/release/trim
```

Open an existing `.trim` project file from the command line:

```bash
./target/release/trim /path/to/project.trim
```

Run the test suite and static analysis checks:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

---

## Architecture Overview

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for the complete threading model, data flow topology, memory ceilings (`LruCache`), and subprocess lifecycle management (`ChildGuard`).

Workspace structure:
- `crates/trim-core`: Domain models, serialization, timecode math, and bounded `LruCache`.
- `crates/trim-commands`: Reversible `Command` implementations and Undo/Redo stack.
- `crates/trim-timeline`: Stateless edit point discovery and splitting logic.
- `crates/trim-project`: `.trim` project persistence, autosave, crash recovery, and portable bundling.
- `crates/trim-media`: `ffmpeg`/`ffprobe` process wrappers, async frame decoders, `AudioMixer`, waveform and thumbnail workers.
- `crates/trim-export`: FFmpeg `filter_complex` compiler, EDL exporter, and FCPXML exporter.
- `crates/trim-ui`: Immediate-mode `egui` interface, timeline canvas, monitors, and inspector.
- `crates/trim-app`: Binary entrypoint.

---

## Known Limitations

1. **Software Preview Compositing Resolution:**
   - Real-time timeline preview decodes and composites video frames on the CPU at a fixed internal preview resolution of `640x360` RGBA before uploading textures to OpenGL. High-track-count compositions (e.g., 4+ simultaneous overlapping video layers with active Chroma Key and Color Wheels) remain CPU-bound during real-time preview.
2. **Subprocess-Driven Media Pipeline:**
   - `triM` invokes `ffmpeg`, `ffprobe`, and `pw-cat` as isolated child processes via standard pipes rather than linking dynamically against `libavcodec`/`libavformat` C libraries. While this isolates the UI process from codec memory corruption crashes, random-access scrubbing incurs process startup overhead (`~15-40 ms` per uncached seek) on high-GOP inter-frame codecs (such as long-GOP H.264/HEVC) unless proxy media is generated.
3. **Hardware Encoder Availability:**
   - Hardware-accelerated export options (`h264_nvenc`, `hevc_nvenc`, `h264_vaapi`, `hevc_vaapi`, `h264_qsv`) depend on the host system's installed `ffmpeg` build and kernel/driver DRM/VAAPI/NVIDIA runtime support. No automatic fallback to software `libx264` occurs if a user manually selects an unsupported hardware encoder in the export dialog.
4. **Platform Scope:**
   - Audio playback (`pw-cat`/`paplay`/`aplay`), voiceover recording (`parecord`/`arecord`), and screen capture (`x11grab`/PipeWire) are implemented specifically for Linux desktop environments. Windows and macOS targets compile for core logic verification but lack native audio/capture backend bindings.
5. **Color Space Management:**
   - Preview compositing operates in 8-bit sRGB RGBA (`u8` per channel). 10-bit HDR (`BT.2020` / PQ / HLG) source media is tone-mapped or truncated to 8-bit during preview and filtergraph processing.

---

## License

Licensed under the GNU General Public License v3.0 or later (`GPL-3.0-or-later`). See [`LICENSE`](LICENSE) for details.
