# triM. — Repository audit (2026-10-10)

The repository contains **two implementations**: the actively presented Rust/egui native editor (8 Cargo workspace crates) and the legacy Python/PySide6 `trim/` + `cutline/` implementations. The audit inspected repository structure, build metadata, Linux packaging, Pages, Actions history and critical Rust editing, export, project and audio paths. This is a static/CI audit, **not** an exhaustive manual GUI acceptance test.

## Fixes implemented

| Severity | Finding | Resolution |
|---|---|---|
| High | The old release workflow packaged Python sources in an architecture-independent `.deb` and created its Python virtualenv on first run. It was not a self-contained native Rust release. | Added `scripts/package-linux.sh` and `.github/workflows/release-linux.yml`; now uses a compiled Rust `trim` binary, distro-appropriate architecture, icons, desktop entry and AppStream metadata. The old `build_deb.sh` routes to the native packager. |
| High | Python CI failed whenever hosted runners had no audio output device (53/54 passing). | The audio PCM/rendering assertions remain active; device-dependent playback signal assertions skip only when `QMediaDevices` reports no output. Legacy Python CI remains separate. |
| High | `trim-export/src/filtergraph.rs` advanced its FFmpeg input index when inserting silent audio and then incorrectly referenced this index for the video stream. | Fixed the reference and added a regression test for a video clip without an audio stream. |
| Medium | Native Rust app icon was read relative to the current working directory and could disappear when started from desktop menus. | Embedded the PNG icon in the executable using `include_bytes!`. |
| Medium | The app startup log hard-coded `v0.3.0`. | Uses `CARGO_PKG_VERSION`. |
| Medium | Fixed test counts and Python-specific descriptions persisted in release docs and AppStream metadata. | Updated README/metadata; package builder injects matching release version into AppStream data. |

## Open findings (not silently claimed as fixed)

### High — Multi-track compositing in Rust export

`crates/trim-export/src/filtergraph.rs` currently collects clips from all visible video tracks, sorts them by timeline start and feeds them into an FFmpeg **concat** filter. Overlapping video tracks are thus not composited as layers. Independent audio tracks are not incorporated by a separate timed audio mix; audio is assembled primarily from the flattened video clips. This differs from the layered interactive timeline and needs a dedicated export pipeline rewrite, using overlay/compositing and audio mixing anchored to timeline time, with end-to-end FFmpeg tests.

### Medium — Memory and playback scalability

`crates/trim-media/src/audio.rs` decodes entire media audio streams into 48 kHz stereo PCM and clones cached PCM data while rendering intervals. Long clips and repeated timeline segments can consume substantial RAM/copy bandwidth. Consider a shared immutable audio cache (`Arc`) and bounded/chunked decoding.

### Medium — Recovery filename collisions

`crates/trim-project/src/lib.rs` derives recovery file names from just the project file stem. Two projects named alike in different directories could share one recovery snapshot. Include a stable hash of the full project path or a project UUID, with legacy recovery lookup compatibility.

### Medium — Legacy launcher ambiguity

`install.sh` and `run.sh` still launch the older Python implementation. `cargo build --release -p trim-app` and the GitHub Release downloads target the current Rust application. Explicitly label or relocate the legacy entry points before further onboarding work.

### Medium — Export error handling

`crates/trim-export/src/runner.rs` pipes both FFmpeg stdout and stderr but drains stderr only after stdout closes. A very noisy FFmpeg process can block when the stderr pipe fills. Concurrently drain stderr or use a bounded log reader.

## CI and release policy

- **Rust CI:** `.github/workflows/rust-ci.yml` checks/tests the Cargo workspace on relevant pushes and pull requests.
- **Legacy Python tests:** `.github/workflows/build-deb.yml` is now test-only, without Python-package publishing.
- **Linux releases:** `.github/workflows/release-linux.yml` runs Rust checks, builds `.deb` and AppImage on Ubuntu, builds `.rpm` natively on Fedora, and publishes artifacts plus `SHA256SUMS.txt` only for version tags matching `Cargo.toml`.
- New Linux artifacts are **x86_64 only**. The AppImage expects a system FFmpeg binary.
- Packaging workflows can run on package/workflow changes to `master` for pre-release validation, without publishing a release.
- The current historical `v0.3.0` release predates the new workflow; future version tags use the new pipeline. The workflow does not retroactively rebuild old tags.

## Remaining validation before declaring a release production-ready

1. Confirm Ubuntu and Fedora package jobs run successfully, extract/install each artifact, and test launching in graphical sessions (Wayland and X11).
2. Test a representative project with overlapping video tracks, separate audio tracks, cuts, speed changes and FFmpeg export.
3. Profile memory usage on long source files and repeated timeline scrubs.
4. Run dependency/security advisory checks and verify behavior with missing `ffmpeg`, `ffprobe`, `pw-cat` and `aplay`.

Issues above are findings and work items, not statements that all code paths were executed or that no other bugs exist.
