# Changelog

All notable changes to **triM.** are documented in this file.

## [Unreleased]

### Fixed & Verified (P0 Reliability)
- **RAII `ChildGuard` in `PlaybackStreamer`**: Wrapped both forward and reverse `PlaybackStreamer` FFmpeg subprocesses (`crates/trim-media/src/video.rs`) in `ChildGuard` so child processes are deterministically terminated and reaped on drop or thread unwind.
- **Release Panic Policy (`panic = "unwind"`)**: Switched `[profile.release]` in root `Cargo.toml` from `abort` to `unwind` so `ChildGuard::drop` executes on panic instead of leaving orphan FFmpeg/audio processes.
- **Subprocess & Roadmap Documentation Alignment**: Updated `ARCHITECTURE.md` §4 (`ChildGuard` coverage & `panic = "unwind"`) and added a verified `ISSUE` status table to `ROADMAP.md`.
- **Tests**: Added Linux-only panic/drop child cleanup test (`test_playback_streamer_panic_cleans_up_ffmpeg_child`) and headless FFmpeg E2E smoke test (`crates/trim-export/tests/e2e_smoke.rs`).

## [2.2.1] - 2026-11-21

### Changed & Fixed
- **Strict Inspector Alignment & Equal-Column Grids**: Unified label column widths (`104px`), aligned all slider bars and keyframe diamond indicators (`◆`/`◇`) across Transform, Compositing, Masks, Speed, Color, Audio, and Titler panels, and converted ragged wrapped buttons into uniform equal-width column grids.
- **Clean Vector Iconography Across UI**: Replaced emojis across the Inspector, Media Bin, Timeline, Monitor, Menus, and Modal Dialogs with crisp typographic/geometric vector symbols (`▸`, `▾`, `●`, `○`, `■`, `□`, `◆`, `◇`, `◈`, `⊞`, `◧`, `⬒`, `▥`, `★`, `☆`, `⚙`, `≡`, `↗`, `←`, `→`, `⌖`, `▶`, `⏸`, `⏹`, `∿`, `♪`, `✂`, `✓`, `✕`, `↺`).
- **News Ticker (`MotionTitleStyle::Ticker`) Badge Clipping Fix**: Fixed scrolling ticker text overlapping the left badge (`BİLGİ` / subtitle badge) by dynamically sizing the badge to its text width and clipping the scrolling text region cleanly at the right edge of the badge.

## [2.2.0] - 2026-11-21

### Added
- **3D `.cube` LUT Baking & Export (`bake_clip_grade_to_cube`)**: Bakes the entire clip color grading pipeline (Camera Log profile, Primary Color, 4-Way Color Wheels, RGB/Luma Curves, HSL Curves, Film Stock emulation, and `LutPreset`) into a `33×33×33` `.cube` 3D LUT for 1:1 OpenGL preview and FFmpeg `lut3d` export parity, plus user-facing `.cube` LUT file export from the Inspector.
- **Camera Log Input Profiles (`CameraLogProfile`)**: Built-in mathematical EOTF/OETF conversion to Rec.709 for Sony `S-Log3`, Canon `C-Log`, Panasonic `V-Log`, and ARRI `Log-C` across OpenGL shaders, CPU preview, and FFmpeg export.
- **On-Monitor White Balance Eyedropper (`compute_wb_from_neutral_rgb`)**: Pick any neutral gray/white pixel directly from the Program Monitor to calibrate clip color temperature and green/magenta tint in one click.
- **Analog Film Emulation Lab (`FilmStockFormat`)**: `35mm`, `16mm`, and `8mm` film stock response curves with `Highlight Roll-off`, `Diffusion`, `Mist`, and `35mm Gate Weave` controls in both real-time GPU preview and FFmpeg export.
- **Visual LUT Gallery Enhancements**: Added `★ Favoriler` (Favorites) tab, per-card star pinning, and live search filtering across built-in and custom LUTs.
- **Monitor Comparison Modes & `2.39:1` Cinemascope Safe-Frame**: Added Horizontal Wipe (`◧`), Vertical Wipe (`⬒`), and Side-by-Side 2x (`▥`) comparison modes (cycled via right-click on the Wipe toggle) along with `2.39:1` anamorphic Cinemascope guide lines.
- **Cinematic Title Styles, Custom `.ttf`/`.otf` Font Loader & TR → EN Converter**: Added `RollingCredits` and `ChapterCard` title animations, custom `.ttf`/`.otf` font file loading with live egui registration and FFmpeg `fontfile=` export, and a one-click `TR → EN` ASCII character converter (`Ğ→G, Ş→S, İ→I, Ç→C, Ö→O, Ü→U`).
- **Procedural Soundscape & Title Foley Synthesizer (`trim_media::soundscape`)**: Built-in procedural `.wav` DSP synthesizer in the Media Bin (`🎛+ SFX`) and Titler Inspector generating `Gerilim Riser`, `Sub Bass Drop`, `35mm Film Projektörü`, `Gece Yağmuru`, `Sinematik Whoosh`, and text-synchronized `Daktilo + 35mm Projektör Foley`.
- **5 Studio Themes, Welcome Hardware Diagnostics & Auto-Proxy**: Added 5 studio color themes (`Varsayılan İndigo & Amber`, `Sinema Lime`, `Saf Monokrom`, `Gece Mavisi`, `Sıcak Tungsten`), a hardware/encoder diagnostic summary card on the Welcome screen, automatic 720p proxy generation on video import when `auto_generate_proxies` is enabled, and in-app GitHub Releases update check.

## [2.1.0] - 2026-11-17

### Added
- **Audio Scrubbing & JKL Shuttle Audio Preview**: Real-time 30ms audio scrubbing on playhead drag and forward/reverse resampled audio playback across JKL shuttle speeds (`2x`, `4x`, `8x`, `-1x`).
- **Multi-Track Video Playback (`PlaybackStreamer` Pool)**: Per-clip `extra_streamers` and `clip_last_frames` in `VideoEngine` with unified `resolve_layer_source_frame`, eliminating cross-track frame bleed in PiP and multi-layer playback.
- **Export Parity & Regression Tests**: Added `rotation` (static + keyframed `rotate`), `speed`-keyframe average `atempo` sync, and `pitch_lock == false` (`asetrate`) support in `filtergraph.rs`, plus new tests for `ripple_trim_head/tail` and export parity.
- **Shortcut Consistency**: Bound `Ctrl+R` to the Clip Speed/Duration dialog and fixed DaVinci Resolve `A` tool status feedback.

### Changed & Fixed
- **Performance & Concurrency**:
  - Added mtime-aware 16-entry LRU cache (`CUBE_LUT_CACHE`) for `.cube` 3D LUT files.
  - Eliminated 601x `Vec` allocations/sorts in `keyframe::interpolate_keyframes` / `integrate_keyframes` for sorted keyframes.
  - Added persistent per-clip `ClipDspState` (EQ, compressor, de-esser, reverb/echo delay buffer) across audio chunks.
  - Replaced unbounded `AudioMixer::preload_audio_async` threads with a bounded 2-worker pool and separated lock acquisition order.
  - Moved Whisper transcription, Smart Scene Cut, and Silence Detection off the UI thread onto `BackgroundTaskMsg` workers (with single-step `MacroCommand` undo for Whisper).
  - Avoided redundant mask GPU uploads in `GpuCompositor` via `uploaded_mask_key` and bounded `mask_cache` to 32 entries.
- **Portability & Platform**:
  - Included `proxy_path` and `custom_lut_path` in `bundle_project`, sanitized bundled `.trim` filenames, and switched `AutosaveManager` to deterministic FNV-1a hashing.
  - Fixed `fcpxml` percent-encoding and Windows drive-letter `file://` decoding, OS-specific FFmpeg missing alerts (Linux vs Windows), and Turkish `Videolar` XDG export path fallback.

## [2.0.2] - 2026-11-16

### Added
- **Expanded Popular Font Catalog & System Font Loading**:
  - Added 31 widely-used font families (`POPULAR_FONT_FAMILIES` in `trim-core`: `Arial`, `Helvetica`, `Times New Roman`, `Georgia`, `Verdana`, `Tahoma`, `Trebuchet MS`, `Courier New`, `Impact`, `Montserrat`, `Bebas Neue`, `Oswald`, `Anton`, `Poppins`, `Roboto`, `Open Sans`, `Lato`, `Inter`, `Raleway`, `Playfair Display`, `Cinzel`, `Lobster`, `Pacifico`, `DejaVu Sans/Serif`, `Liberation Sans`, `Noto Sans`, `Ubuntu`, plus generic `Sans-Serif`/`Serif`/`Monospace`).
  - Added OS font directory discovery and registration in `theme.rs` + `monitor.rs` so locally installed `.ttf`/`.otf` fonts render directly in the Program Monitor preview with safe fallback.
  - Updated FFmpeg `drawtext` export in `filtergraph.rs` to pass any sanitized `font_family` name directly instead of restricting to 5 hardcoded names.

## [2.0.1] - 2026-11-16

### Fixed
- **Stability & Undo/Redo**:
  - Prevented infinite playback clock advance on empty timelines (`dur <= 0.0`).
  - Added centralized playback cache invalidation (`video_engine`, `audio_mixer`, monitor frame) on Undo/Redo, bounded `UndoStack` to 200 entries, and preserved commands if `undo()`/`execute()` fails.
  - Included `halation`, `bloom`, `chromatic_aberration`, `custom_curves`, `hsl_curves`, and `hsl_qualifier` in `SetClipColorGradingCommand`.
- **Timeline Editing & Crash Protection**:
  - Fixed `Clip::trim_in`/`trim_out` for keyframes, speed ramps, and reversed clips; partitioned mask keyframes and supported N-way `linked_group_id` in `SplitClipCommand`, `BatchSplitClipCommand`, and `RippleDeleteCommand`.
  - Guarded parametric EQ against >6 enabled bands, removed non-UTF-8 `unwrap()` panics in media/EDL paths, guarded fullscreen zero-height division, and stopped active screen recording when closing the dialog via `X`.
- **Performance & FFmpeg Export**:
  - Added horizontal viewport culling and 3s TTL-cached offline media checks in `canvas.rs`; fixed mask keyframe hashing in `GpuCompositor` and filmstrip texture cleanup on media deletion.
  - Fixed `ProRes 422` (`yuv422p10le`), `VAAPI` (`format=nv12,hwupload` + `-vaapi_device`), `reverse` range slicing, `ExportRunner` cancel race, stderr size cap, and `-filter_complex_script` fallback for large filtergraphs.

## [2.0.0] - 2026-11-15

### Added
- **Video Scopes & Reference Monitoring**:
  - **Quad Scope (2x2)**, **CIE 1931 Chromaticity Scope** (with Rec.709 primaries), and live **Program Monitor Pixel Readout + Scope Reticle** (`RGB` / `Luma` / `IRE`).
  - **Split-Screen Wipe** comparison line on the Program Monitor and **Reference Still Gallery** for grabbing/applying reference frames and grades.
- **Advanced Color Grading & Film FX**:
  - **Custom RGB / Luma Spline Curves** (`CustomCurvesConfig`) with interactive control points, CPU/GPU preview, and FFmpeg `curves` export.
  - **6-Vector HSL Curves** (`HslCurvesConfig`) and **3D HSL Qualifier** (`HslQualifierConfig`) with secondary matte preview (`show_matte`) and invert support.
  - **Analog Film & Lens FX**: `halation`, `bloom`, and `chromatic_aberration` across CPU preview, OpenGL shaders, and FFmpeg export.
  - **One-Click Shot Match** (`analyze_shot_match_rgba`) matching clip exposure, contrast, saturation, and white balance to a reference still.
- **Fairlight-Style Audio Processing**:
  - **6-Band Parametric EQ** and **Dynamics Processor (Compressor / Limiter / De-Esser)** integrated in real-time audio playback and FFmpeg export.
  - **Clip & Track Stereo Pan**, **Reverb / Echo**, and **Pitch Lock** for speed-ramped clips.
- **Editing Workflow, Compositing & Delivery**:
  - **Create Compound Clip**, **Paste Attributes (`Ctrl+Alt+V`)**, and **Join Through-Edits**.
  - **4-Corner Perspective Pin**, **Track Matte** (luma matte from track above), and **Rich Typography** (`stroke_width`, `stroke_color`, `letter_spacing`, `shadow_distance`).
  - **Multicam 2x2 Monitor View** with `1–4` live angle switching, **Media Bin Usage Count Badge**, **YouTube Chapters + Embedded MP4 Chapters**, and **CMX3600 EDL + FCPXML** export.

## [1.7.0] - 2026-11-15

### Added
- **Timeline & Editing Experience**:
  - **Filmstrip Thumbnails**: Asynchronous 8-frame horizontal sprite strips (`{hash}_strip8.jpg`) rendered time-mapped across video clips on the timeline.
  - **Advanced Keyframe Curves**: Added `EaseIn`, `EaseOut`, and `CubicBezier` (`bezier_handles`) interpolation modes with an interactive draggable Cubic Bezier curve editor popup in the Inspector.
  - **Speed Ramping (Time Remapping)**: Analytical speed-curve integration (`integrate_keyframes`), 6 built-in speed ramp presets, live speed curve graph in the Inspector, and piecewise-linear FFmpeg `setpts` export.
  - **Audio Waveform Auto-Sync**: One-click multi-clip alignment (`🎙️ Ses Dalga Formuna Göre Hizala`) using onset envelope cross-correlation (`compute_audio_sync_offset`) with full Undo/Redo support.
- **Visual Effects, Transitions & Motion**:
  - **Direct Visual Effect Sliders**: Added **Blur**, **Sharpen**, **Vignette**, and **Film Grain** sliders to the Color & Effects card with full parity across OpenGL shaders, CPU fallback preview, and FFmpeg export.
  - **Expanded Transition Library**: Added `blur_dissolve`, `luma_wipe`, `whip_pan`, `zoom_out`, `glitch`, and `light_leak` transitions.
  - **Video Stabilization & Mask Motion Tracking**: Added `deshake` video stabilization control and automated SAD block-matching mask motion tracking (`🎯 Takip`) generating `pos_x` / `pos_y` keyframes.
- **Audio & Subtitle Workflow**:
  - **Karaoke / Reels Word-Level Captions**: Character-weighted speech timing, per-word spring bounce, and rounded pill highlights for `WordHighlightYellow`, `WordHighlightNeon`, and `PopUpBouncy` caption styles.
  - **Voice Denoise & Auto-Ducking**: Real-time rumble filter + spectral noise gate (`afftdn` on export) and one-click dialogue **Auto-Ducking** (`🔉 Konuşmada Müziği Kıs`) generating smooth volume ducking keyframes.
- **Performance & Hardware Compatibility**:
  - **Timeline Smart Render Cache Bar**: Top-of-ruler emerald/amber cache indicator bar with idle background frame prewarming around the playhead.
  - **Hardware Encoder Probing**: Added runtime **Apple VideoToolbox** (`h264_videotoolbox` / `hevc_videotoolbox`) detection alongside NVENC, QSV, AMF, and VAAPI.

---

## [1.6.2] - 2026-11-15

### Improved
- **Master VU Meter Layout & Alignment**:
  - Redesigned the Master VU Meter (`VuMeterWidget`) on the right side of the timeline with a crisp vertical divider, wider `L`/`R` channel bars, dB scale tick marks, channel footer labels, and a `24 px` `MASTER` header aligned flush with the timeline ruler from top to bottom.
- **Documentation & GitHub Pages**:
  - Updated `README.md` and `README.tr.md` with a cleaner user-focused presentation, added **prisM. Suite** ecosystem links, and enabled automatic version synchronization on GitHub Pages.

---

## [1.6.1] - 2026-11-13

### Fixed
- **Windows FFmpeg & FFprobe Bundling / Detection (`install.ps1` & CI)**:
  - Fixed Windows packages and `install.ps1` potentially copying Chocolatey shims or DLL-dependent FFmpeg executables that fail to launch on end-user machines (`FFmpeg bulunamadı` modal).
  - Enforced standalone static binary validation (`>= 5 MB` and `ffmpeg -version` / `ffprobe -version` execution checks) in both GitHub Actions Windows packaging and `install.ps1`.

### Changed
- **Branding**: Removed the release codename across the UI, CLI (`--version` / `--help`), window titles, About/Welcome/Splash dialogs, and README badges — now displaying plain `triM.`.

---

## [1.6.0] - 2026-11-12

### Added
- **Professional Color Scopes (`ScopesPanel`)**:
  - Real-time RGBA frame analysis (`compute_scopes_rgba`) supporting **RGB Parade**, **Luma Waveform**, **Vectorscope** (with skin-tone line and 75% color targets), **RGB & Luma Histogram**, **False Color** (IRE exposure mapping), and **Zebra Exposure** overlays.
  - Dedicated toggleable Scopes panel updated dynamically from GPU composited readback or CPU fallback frames as playback or clip selection changes.
- **Mask System (`Rectangle`, `Ellipse`, `Bezier`)**:
  - Full mask model (`Mask`, `MaskShape`, `MaskTarget`, `MaskVertex`) with position, scale, rotation, feather, expansion, opacity, invert, multi-mask compositing, and per-property keyframe interpolation.
  - Interactive Program Monitor handles for dragging center, corner scale, rotation, and Bezier vertices/tangents, plus Inspector mask controls and keyframe diamonds (`SetClipMasksCommand`).
- **Adjustment Layers**:
  - Media-less `ClipKind::AdjustmentLayer` clips that apply color grading, masks, opacity, and blend modes to underlying composited video tracks (`AddAdjustmentLayerCommand`), with distinct purple/magenta timeline styling and `"✨ Ayar Katmanı Ekle"` in the `+ Kanal` menu.
- **Blend Modes & Full Pipeline Parity**:
  - Added 11 blend modes (`Normal`, `Multiply`, `Screen`, `Overlay`, `SoftLight`, `HardLight`, `Add`, `Subtract`, `Difference`, `Darken`, `Lighten`) with `SetClipBlendModeCommand` and full Undo/Redo support.
  - Unified parity across the OpenGL GPU compositor (`backdrop_fbo` blit + mask textures), CPU software fallback renderer, and FFmpeg export filtergraph (`maskedmerge`, `blend`, `overlay`), with `#[serde(default)]` backward compatibility for existing `.trim` projects.

---

## [1.5.1] - 2026-11-12

### Fixed
- **Fullscreen Preview GPU Composited Video Rendering**:
  - Fixed a black screen bug in Fullscreen Preview (`F11` / double-click) where only audio would play while displaying "Önizleme Yükleniyor..." or "Oynatılıyor...". The fullscreen monitor now resolves both hardware GPU composited textures (`monitor_gpu_texture_id`) and CPU fallback textures with identical performance and quality.
  - Added full overlay rendering support in fullscreen mode for active subtitles, captions, lower-thirds, and motion title graphics during playback and scrubbing.

---

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
