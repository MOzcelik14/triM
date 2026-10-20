# triM. — Repository audit (Updated 2026-10-20)

The repository has completed its full transition to a **100% pure native Rust desktop editor** (8 Cargo workspace crates). All legacy Python/PySide6 prototype code (`trim/`, `cutline/`, legacy tests, scripts, and requirements) has been permanently purged from the repository. The audit inspected repository structure, build metadata, Linux packaging, Pages, Actions history and critical Rust editing, export, project and audio paths.

## Fixes implemented

| Severity | Finding | Resolution |
|---|---|---|
| High | Multi-track compositing in Rust export (`crates/trim-export/src/filtergraph.rs`) previously concatenated clips rather than compositing layers. | Rewrote export pipeline using timed canvas overlays and timeline-anchored `amix` audio mixing matching the preview monitor. Added comprehensive tests including end-to-end FFmpeg rendering. |
| High | Legacy Python dual-codebase confusion and GitHub Linguist misattribution. | Completely deleted all legacy Python code (`cutline/`, `trim/`, pytest tests, `requirements.txt`, `pyproject.toml`). Repository is now 100% Rust. |
| High | Missing unicode glyphs and tofu boxes (`▯`) on Linux desktops across menus, toolbar, and dialogs. | Replaced system emojis with clean desktop typography, system fallback fonts, and native `ui.painter()` subpixel vector graphics for the 8 toolbar tools. |
| High | Export Studio layout collapse and preset horizontal crowding bug. | Enclosed preset list in vertical layout (`allocate_ui_with_layout` + `ui.vertical`), providing clean single-column selectable profiles with high-contrast text. |
| Medium | Timeline squashed / disappearing due to unconstrained monitor panel. | Restructured workspace layout: permanently anchored `TopBottomPanel::bottom("timeline_panel")` at bottom, `SidePanel::left` for Media Bin, and `CentralPanel` for video monitors. |
| Medium | Media Bin search bar horizontal overflow causing an unpainted 60px black strip. | Refactored `media_bin.rs` sub-toolbar using `Layout::right_to_left`, eliminating horizontal overflow and restoring seamless panel alignment. |
| Medium | Export error handling in `crates/trim-export/src/runner.rs` drained stderr only after stdout closed, risking pipe buffer deadlocks. | Concurrently drain and buffer stderr in a dedicated background thread while reading stdout progress. |
| Medium | Audio playback and mixing memory scalability in `crates/trim-media/src/audio.rs` cloned full PCM buffers on every slice request. | Switched audio cache to shared `Arc<Vec<[i16; 2]>>`, eliminating full buffer duplications and massive RAM usage. |
| Medium | Recovery filename collisions in `crates/trim-project/src/lib.rs` when multiple projects in different directories shared the same file stem. | Incorporated stable 64-bit path hash into recovery file names (`{stem}_{hash}_recovery.trim`) with legacy fallback support. |
| Medium | Native Rust app icon was read relative to the current working directory and could disappear when started from desktop menus. | Embedded the PNG icon in the executable using `include_bytes!`. |
| Medium | Text selection contrast issue where selected items had amber text on amber background. | Updated selection theme to high-contrast white text on slate blue background. |

## CI and release policy

- **Rust CI:** `.github/workflows/rust-ci.yml` checks, tests, and lints the Cargo workspace on every push and pull request.
- **Linux releases:** `.github/workflows/release-linux.yml` runs Rust checks, builds `.deb` and AppImage on Ubuntu, builds `.rpm` natively on Fedora, and publishes artifacts plus `SHA256SUMS.txt` for version tags matching `Cargo.toml`.
- All Linux artifacts are **x86_64 only**, self-contained native Rust executables.

## Validation Status

- `cargo test --workspace`: **52 tests passing (100% OK)**
- `cargo clippy --workspace --all-targets -- -D warnings`: Clean with zero warnings.
- Native release binary: `./target/release/trim` (19 MB, self-contained).
