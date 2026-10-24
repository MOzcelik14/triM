# triM Architecture Specification

`triM` is a non-linear video editor (NLE) written in Rust (2021 edition). It renders its user interface directly via immediate-mode `egui`/`eframe` over an OpenGL (`glow`) backend on Wayland/X11, and offloads media decoding, audio mixing, waveform generation, thumbnail extraction, and rendering pipelines to isolated background worker threads and `ffmpeg` / `pw-cat` subprocesses.

---

## 1. Threading & Data Flow Topology

The UI render loop (`eframe::App::update`) executes strictly on the main thread and is prohibited from performing synchronous disk I/O, blocking IPC, or direct subprocess spawning. All heavy media operations are decoupled via non-blocking `std::sync::mpsc` channels and shared bounded caches.

```text
+-----------------------------------------------------------------------------------+
| MAIN THREAD (trim-ui / eframe::App::update)                                       |
|                                                                                   |
|  +------------------+   +-------------------+   +------------------------------+  |
|  | Timeline Canvas  |   | Source / Program  |   | Media Bin & Inspector        |  |
|  | (Waveform poll)  |   | Monitors (GL Tex) |   | (Thumbnail poll, Commands)   |  |
|  +--------+---------+   +---------+---------+   +--------------+---------------+  |
|           |                       |                            |                  |
|           | try_recv / RwLock     | try_recv / RwLock          | try_recv         |
+-----------+-----------------------+----------------------------+------------------+
            |                       |                            |
            v                       v                            v
+-----------------------------------------------------------------------------------+
| BOUNDED MEMORY LAYER (trim-core::lru::LruCache)                                   |
|                                                                                   |
|  * VideoEngine.frame_cache: Arc<RwLock<LruCache<(String, u64), Arc<RgbaImage>>>>  |
|    (Capacity: 300 decoded preview frames, incremental LRU eviction)               |
|  * VideoFrameDecoder.frame_cache: LruCache<(String, u64), RgbaImage>              |
|    (Capacity: 150 scrub frames)                                                   |
|  * AudioMixer.pcm_cache: LruCache<String, Vec<f32>>                               |
|    (Capacity: 256 entries / Hard ceiling: 512 MB raw f32 PCM samples)             |
|  * WaveformGenerator.cache: Arc<Mutex<HashMap<String, Arc<WaveformData>>>>        |
+-----------+-----------------------+----------------------------+------------------+
            ^                       ^                            ^
            | mpsc::Sender          | mpsc::Sender               | mpsc::Sender
+-----------+-----------------------+----------------------------+------------------+
| BACKGROUND WORKER THREADS (trim-media / trim-export)                              |
|                                                                                   |
|  +--------------------+  +------------------------+  +-------------------------+  |
|  | Waveform Worker    |  | PlaybackStreamer &     |  | Thumbnail Worker &      |  |
|  | Thread             |  | AsyncFrameDecoder      |  | Export / Proxy Workers  |  |
|  +---------+----------+  +-----------+------------+  +------------+------------+  |
|            |                         |                            |               |
|            v                         v                            v               |
|  +-----------------------------------------------------------------------------+  |
|  | RAII SUBPROCESS LAYER (trim-media::ffmpeg::ChildGuard)                      |  |
|  |                                                                             |  |
|  |  * ffmpeg (rawvideo RGBA pipe, f32le PCM pipe, filter_complex export)       |  |
|  |  * ffprobe (JSON stream/container metadata inspection)                      |  |
|  |  * pw-cat / aplay / paplay (low-latency PCM audio output stream)            |  |
|  |  * Drop implementation guarantees SIGKILL + wait() reaping on scope exit    |  |
|  +-----------------------------------------------------------------------------+  |
+-----------------------------------------------------------------------------------+
```

---

## 2. Workspace Crate Responsibilities

The workspace consists of 8 crates arranged in a strict directed acyclic dependency graph (DAG):

### `trim-core`
- **Role:** Foundational data structures, domain models, and zero-dependency utilities.
- **Contents:** `Project`, `Timeline`, `Track` (`Video` / `Audio`), `Clip`, `MediaItem`, `KeyframeTrack`, `Marker`, subtitle parsers (`SRT`/`VTT`), XDG path resolution, timecode arithmetic, and the arena-backed `LruCache<K, V>`.
- **Constraints:** Contains no UI, subprocess, or hardware I/O logic. All state types implement `serde::Serialize` and `serde::Deserialize`.

### `trim-commands`
- **Role:** Deterministic state mutation and Undo/Redo history management.
- **Contents:** `Command` trait (`execute`, `undo`, `description`), `CommandHistory` stack, and concrete command implementations (`AddClipCommand`, `MoveClipCommand`, `SplitClipCommand`, `RippleDeleteCommand`, `ChangeClipSpeedCommand`, etc.).
- **Constraints:** Operates exclusively on `&mut trim_core::project::Project`. Every user-initiated timeline or inspector edit is encapsulated as a reversible command.

### `trim-timeline`
- **Role:** Stateless timeline geometry and editing algorithms.
- **Contents:** Playhead splitting (`split_at_playhead`), magnetic snapping point discovery (`find_edit_points`), track lock validation, and multi-track synchronization helpers.
- **Constraints:** Depends only on `trim-core` and `trim-commands`.

### `trim-project`
- **Role:** Project persistence, crash recovery, and portable archiving.
- **Contents:** JSON project serialization (`.trim`), atomic autosave file management, crash recovery lockfiles, and portable project bundling (copying referenced media assets into a self-contained directory).
- **Constraints:** Depends on `trim-core`.

### `trim-media`
- **Role:** Subprocess orchestration, media decoding, audio synthesis, and background analysis workers.
- **Contents:**
  - `ffmpeg.rs`: Binary resolution (`ffmpeg`, `ffprobe`), environment sanitization (Snap/Flatpak `LD_LIBRARY_PATH` stripping), and `ChildGuard`.
  - `video.rs`: `VideoEngine`, `PlaybackStreamer`, `ReversePlaybackStreamer`, `AsyncFrameDecoder`, software compositing (Chroma Key, 3-Way Color Wheels, bilinear scaling, alpha blending).
  - `audio.rs`: `AudioMixer` (3-band biquad EQ, pitch-preserving tempo scaling, stereo panning, fade envelopes) and `AudioStreamPlayer`.
  - `thumbnail.rs` & `waveform.rs`: Asynchronous worker queues for visual asset generation.
  - `proxy.rs`, `scene_detect.rs`, `silence_detect.rs`, `transcribe.rs`, `voiceover.rs`: Background media processing pipelines.

### `trim-export`
- **Role:** Offline rendering and NLE interchange format generation.
- **Contents:**
  - `filtergraph.rs`: Compiles `trim-core::Project` timelines into `ffmpeg -filter_complex` graphs (overlay chains, `drawtext`, `colorkey`, `lut3d`, `loudnorm`, keyframe expressions) and parses real-time `ffmpeg -progress pipe:1` output.
  - `edl.rs` & `fcpxml.rs`: CMX 3600 EDL and Final Cut Pro X FCPXML v1.9 exporters.

### `trim-ui`
- **Role:** Immediate-mode GUI state machine and OpenGL texture presentation.
- **Contents:**
  - `app.rs`: Core `TrimApp` state struct, monitor presentation, and `eframe::App` implementation.
  - `app/playback.rs`: Transport control, shuttle speed state, non-blocking frame polling.
  - `app/project_io.rs`: File dialogs, project load/save/bundle handlers, export triggers.
  - `app/media_actions.rs`: Media import, proxy toggling, AI transcription/cut dispatchers.
  - `app/timeline_actions.rs`: Track routing (`resolve_default_target_tracks`), clip linking, ripple deletion, monitor text overlay manipulation.
  - Widget modules: `canvas.rs`, `inspector.rs`, `media_bin.rs`, `monitor.rs`, `ruler.rs`, `toolbar.rs`, `track_header.rs`, `transport.rs`, `vu_meter.rs`, `color_wheel.rs`, `dialogs.rs`.

### `trim-app`
- **Role:** Binary entrypoint (`trim`).
- **Contents:** CLI argument parsing (opening `.trim` files from command line), logger initialization (`env_logger`), window icon generation, and `eframe::run_native` bootstrap.

---

## 3. Memory Management Strategy

Unbounded caching of decoded video frames and raw PCM audio buffers rapidly exhausts system RAM in NLE workloads. `triM` enforces deterministic memory ceilings via `trim_core::lru::LruCache`:

1. **Arena-Backed Doubly Linked List (`LruCache<K, V>`):**
   - Implemented without `unsafe` pointers using a `Vec<Node<K, V>>` arena, free-list index recycling, and a `HashMap<K, usize>` lookup table.
   - Provides $O(1)$ insertion, lookup, promotion, and least-recently-used eviction.
   - Supports dual bounds: maximum entry count (`max_entries`) and maximum cumulative byte size (`max_bytes`).

2. **PCM Audio Cache Ceiling (512 MB):**
   - `AudioMixer.pcm_cache` decodes source audio streams into stereo 44.1 kHz `f32` buffers (`4 bytes * 2 channels * 44100 Hz ≈ 352.8 KB/sec`).
   - Each entry's byte footprint (`samples.len() * std::mem::size_of::<f32>()`) is tracked upon insertion.
   - When total cached PCM data exceeds `MAX_PCM_CACHE_BYTES` (`512 * 1024 * 1024` bytes), the least recently accessed audio buffers are automatically evicted before new allocations occur.

3. **Incremental Video Frame Eviction:**
   - `VideoEngine.frame_cache` stores `Arc<RgbaImage>` preview frames (640x360 RGBA = ~921.6 KB per frame) capped at 300 entries (~276 MB max).
   - `VideoFrameDecoder.frame_cache` is capped at 150 entries (~138 MB max).
   - Bulk `HashMap::clear()` calls are strictly avoided to prevent sudden frame drops during playback and allocator fragmentation churn. When full, inserting a new frame evicts only the single oldest frame.

---

## 4. Subprocess & RAII Safety (`ChildGuard`)

All external media processes (`ffmpeg`, `ffprobe`, `pw-cat`, `aplay`, `paplay`, `parecord`) are spawned through `trim_media::ffmpeg::create_command`, which strips conflicting library paths (`LD_LIBRARY_PATH`, `LIBGL_DRIVERS_PATH`) injected by sandboxed environments.

Every spawned `std::process::Child` that runs across multiple frames or inside a worker thread is wrapped in `ChildGuard`:

```rust
pub struct ChildGuard {
    child: Option<Child>,
    exit_status: Option<ExitStatus>,
}
```

- **Lifecycle Guarantee:** When `ChildGuard` goes out of scope (whether by normal completion, early `Result` return, channel disconnection, or thread panic), its `Drop` implementation invokes `child.kill()` followed immediately by `child.wait()`.
- **Zombie Prevention:** Reaping the process via `wait()` ensures the OS kernel process table never accumulates defunct `<zombie>` `ffmpeg` or `pw-cat` processes during rapid timeline scrubbing or repeated play/pause toggling.
- **Status Inspection:** `ChildGuard::wait()` and `ChildGuard::try_wait()` cache the resulting `ExitStatus` in `exit_status`, allowing callers to inspect process termination codes idempotently even after the underlying handle has been reaped.
