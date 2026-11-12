# Changelog

All notable changes to **triM.** are documented in this file.

## [1.5.0] - 2026-11-12

### Added
- **Advanced Color Preset System**:
  - **White Balance (`WhiteBalancePreset`)**: Added `Temperature` (`-100..=100`) and `Tint` (`-100..=100`) parameters with presets (`Auto White Balance`, `Daylight 5600K`, `Tungsten 3200K`, `Cloudy`, `Shade`, `Fluorescent`, `Warm`, `Cool`) and post-preset manual slider fine-tuning.
  - **Real Image-Analysis Auto White Balance (`analyze_auto_white_balance_rgba`)**: Frame-based midtone-weighted Gray-World & chromatic log-ratio illuminant estimation that automatically neutralizes cool, warm, or green/magenta color casts on the active video/image frame.
  - **Brightness & Contrast Presets (`BrightnessContrastPreset`)**: Added `Neutral`, `Soft Contrast`, `High Contrast`, `Low Contrast`, `Bright`, `Dark / Moody`, `Cinematic Contrast`, and `Flat` presets driving `brightness`, `contrast`, and the new `gamma` (`0.2..=3.0`) curve parameter.
  - **Expanded Categorized Color Looks (`LutPreset`)**:
    - **Cinematic**: `Teal & Orange`, `Cinematic Warm`, `Cinematic Cool`, `Blockbuster`, `Filmic Neutral`.
    - **Film Emulation**: `Kodak 2383`, `Kodak Portra`, `Fuji Film`, `Vintage Film`, `Faded Film`, `Analog Film`.
    - **Creative**: `Monochrome` (`MonochromeNoir`), `Sepia`, `Bleach Bypass`, `Muted Colors`, `Vibrant`, `Pastel`, `Golden Hour`, `Cold Winter`, `Night Cinematic`, and `Cyberpunk Neon`.
  - **Redesigned Color Inspector & Live Preset Thumbnails**: Categorized color grading subcards with live-sampled miniature preset thumbnails, custom `.cube` 3D LUT support in both GPU shader and CPU software pipelines, full preview/export parity (`colorchannelmixer`, `eq` with `gamma`, and look filter chains), backward compatibility with existing `.trim` projects, and full Undo/Redo (`SetClipColorGradingCommand`).

---

## [1.4.0] - 2026-11-12

### Added
- **Sony XAVC S / XAVC HS Native MP4 Atom & Big-Endian PCM Decoder (`ipcm` / `lpcm` / `twos`)**:
  - Direct MP4/MOV `stsd`, `pcmC` (ISO/IEC 23003-5), and `stsz` atom inspection to determine exact channel count, bit depth (16-bit, 24-bit, 32-bit), and endianness for Sony camera recordings (ZV-E10, FX3, FX30, A7 IV, A6700, etc.).
  - Automatic `-c:a pcm_s16be` / `pcm_s24be` / `pcm_s32be` input codec injection and fallback chain before `-i` across timeline playback, waveform generation, proxy creation, silence detection, subtitle transcription, and video export — resolving missing audio on Pop!_OS and Ubuntu LTS distributions.
- **Source Monitor Real-Time Audio Preview**:
  - Real-time stereo audio playback and live VU meter feedback when previewing media in the Source Monitor (`Kaynak Monitörü`).
- **Linux Audio Output Backend Failover**:
  - Automatic runtime failover across `pw-cat` (PipeWire) -> `paplay` (PulseAudio) -> `aplay` (ALSA) -> `ffplay` when an audio daemon rejects or closes a stream pipe on Linux distributions such as Pop!_OS.

### Fixed
- **Zero-Channel / Unknown Codec Audio Probing**:
  - Fixed `channels: 0` and empty `codec_name` reported on unrecognized `ipcm` streams so Sony camera clips always create linked audio clips (`A1`) when dropped onto the timeline.
- **Unlinked Video Track Audio Fallback & Project Auto-Repair**:
  - Video clips on `V1..Vn` that have embedded audio but no linked `A1..An` clip now play their audio directly through the mixer.
  - Existing saved projects and autosaves with missing audio metadata are automatically re-probed on project load.
- **Concurrent Audio Preload Deduplication**:
  - Added in-flight synchronization to `AudioMixer` so background preloading and immediate playback never duplicate decoding or cache transient failures.

---

## [1.3.4] - 2026-11-07

### Fixed
- Prevented timeline clip titles and duration badges from overflowing clip boundaries or overlapping adjacent clips by applying strict header scissor clipping.

---

## [1.3.3] - 2026-11-07

### Changed
- Enlarged default UI typography and widget spacing (`1.08x` default scale) for improved readability and comfort.
- Stabilized the Preferences window layout with fixed dimensions and cached hardware encoder status to eliminate frame hitching.
- Redesigned the About dialog with official copyright and developer attribution (**Murat Özçelik**).

### Fixed
- Fixed Media Bin drag-and-drop overlay colliding with the empty state placeholder card.

---

## [1.3.2] - 2026-11-07

### Added
- Synchronized Razor Split (`C` / `S`) across linked video and audio clips with automatic re-linking.
- Multi-track Ripple Delete (`Shift+Delete` / `G`) to shift all unlocked tracks in sync.
- Cursor-centered smooth timeline zooming (`Ctrl + Scroll`, `+`, `-`, `=`).
- Full Undo/Redo support for clip fade handles and interactive monitor text editing.
- Relative track hierarchy preservation when copying and pasting clips across multiple tracks.
- Fast range export (`In/Out`) processing only the selected timeline interval.

---

## [1.3.1] - 2026-11-06

### Added
- Direct drag-and-drop file and folder import into the Media Bin.
- Fullscreen Program Monitor preview (`F11`, double-click, `ESC`).
- Double-click on canvas text to immediately enter inline text editing mode.
- Expanded font family selection (`Sans-Serif`, `Serif`, `Monospace`, `Impact`, `Montserrat`) with Bold and Italic toggles.
- Dedicated transparent title clips on the timeline.
