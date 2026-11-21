# Contributing to triM.

Thank you for your interest in contributing to **triM.**, part of the **prisM. Suite**. This guide outlines the workflow, coding standards, and verification steps expected for contributions.

## Before You Start

1. Check existing [GitHub Issues](https://github.com/MOzcelik14/triM/issues) and [ROADMAP.md](ROADMAP.md) to avoid duplicating ongoing work.
2. For larger features or architectural changes, open an issue first to discuss the design and crate boundaries.
3. Review [ARCHITECTURE.md](ARCHITECTURE.md) to understand the workspace structure.

## Workspace Boundaries

Keep changes scoped to the appropriate crate:

- `trim-core`: Pure domain models, time/frame math, color grading, scopes, masks, and subtitles. Must remain independent of `egui`, `eframe`, and FFmpeg subprocesses.
- `trim-commands`: Reversible editing commands and `UndoStack`. All user-facing timeline mutations should be undoable.
- `trim-timeline`: Playhead navigation, edit-point queries, and ripple/split helpers.
- `trim-project`: `.trim` project serialization, bundling, autosave, and recovery.
- `trim-media`: FFmpeg/FFprobe process management, video/audio decoding, DSP effects, waveforms, proxy generation, and voiceover capture.
- `trim-export`: FFmpeg filtergraph builder, `ExportRunner`, `RenderQueue`, and EDL/FCPXML interchange.
- `trim-ui`: `egui`/`eframe` interface, `GpuCompositor`, dialogs, shortcuts, and localization (`i18n.rs`).
- `trim-app`: Binary entry point and CLI flags.

## Development Workflow

1. Fork the repository and create a feature or bugfix branch from `master`.
2. Make focused, well-tested changes. Add or update unit tests for new behavior or bug fixes.
3. Follow descriptive or conventional commit prefixes (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`).
4. Run all local verification checks before opening a Pull Request:

```bash
# Check formatting
cargo fmt --all -- --check

# Run Clippy with warnings denied
cargo clippy --workspace --all-targets -- -D warnings

# Run all workspace tests
cargo test --workspace --locked

# Verify Linux packaging script syntax (when scripts/ is modified)
bash -n scripts/package-linux.sh
```

All pull requests are validated by GitHub Actions ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)) on both Linux (`ubuntu-24.04`) and Windows (`windows-latest`).

## Localization

UI strings are managed in `crates/trim-ui/src/i18n.rs` (currently supporting Turkish `tr` and English `en`).

- Avoid introducing new hardcoded UI strings when an `i18n` key is appropriate.
- When adding a new language, update `Language`, `Language::ALL`, `from_code`, `code`, `label`, and `translate(lang, key)`.
- Verify translation coverage with:

```bash
cargo test -p trim-ui i18n
```

## Reporting Bugs

When opening a bug report on [GitHub Issues](https://github.com/MOzcelik14/triM/issues), please include:

- **triM.** version and installation method (package, archive, or source build).
- Operating system, GPU driver, and FFmpeg version (`ffmpeg -version`).
- Steps to reproduce, expected behavior, and actual behavior.
- Relevant log output (run with `RUST_LOG=info` or `RUST_LOG=debug` when possible).

## License

By contributing to **triM.**, you agree that your contributions will be licensed under the **GNU General Public License v3.0 or later (`GPL-3.0-or-later`)**. See [LICENSE](LICENSE) for details.
