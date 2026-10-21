//! Cross-platform FFmpeg process spawning and environment handling.
//!
//! On Windows, GUI applications spawning child processes will flash or show an unwanted
//! black console (`cmd`/`conhost`) window unless the `CREATE_NO_WINDOW` flag is specified.
//! This module ensures all `ffmpeg`, `ffprobe`, and `ffplay` processes run silently without
//! stealing focus or flickering console windows.
//!
//! In addition, it detects portable FFmpeg executables located in the same directory as the
//! running `trim.exe` binary before falling back to `%PATH%`.

use std::path::PathBuf;
use std::process::Command;

/// Applies Windows-specific process creation flags to suppress console windows.
pub fn apply_no_window(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Creates a `Command` for the requested tool (`ffmpeg`, `ffprobe`, `ffplay`),
/// automatically detecting local/portable binaries next to `trim` and suppressing
/// console windows on Windows.
pub fn create_command(name: &str) -> Command {
    let resolved_path = resolve_binary_path(name);
    let mut cmd = Command::new(resolved_path);
    apply_no_window(&mut cmd);
    cmd
}

/// Resolves binary location, checking:
/// 1. Next to the running executable (`./ffmpeg.exe` or `./ffmpeg`)
/// 2. In system PATH as fallback
fn resolve_binary_path(name: &str) -> PathBuf {
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            if cfg!(windows) {
                let candidate = exe_dir.join(format!("{}.exe", name));
                if candidate.is_file() {
                    return candidate;
                }
            } else {
                let candidate = exe_dir.join(name);
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }
    PathBuf::from(name)
}

/// Creates a `Command` for `ffmpeg`.
pub fn ffmpeg_cmd() -> Command {
    create_command("ffmpeg")
}

/// Creates a `Command` for `ffprobe`.
pub fn ffprobe_cmd() -> Command {
    create_command("ffprobe")
}

/// Creates a `Command` for `ffplay`.
pub fn ffplay_cmd() -> Command {
    create_command("ffplay")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_command_basic() {
        let cmd = create_command("ffmpeg");
        assert!(cmd.get_program().to_string_lossy().contains("ffmpeg"));
    }

    #[test]
    fn test_resolve_binary_fallback() {
        let p = resolve_binary_path("custom_tool_xyz");
        assert_eq!(p, PathBuf::from("custom_tool_xyz"));
    }
}
