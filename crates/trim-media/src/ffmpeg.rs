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

/// Hardware video encoders detected on the system's FFmpeg installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct AvailableEncoders {
    pub h264_nvenc: bool,
    pub hevc_nvenc: bool,
    pub h264_qsv: bool,
    pub hevc_qsv: bool,
    pub h264_vaapi: bool,
    pub hevc_vaapi: bool,
    pub h264_amf: bool,
    pub hevc_amf: bool,
}

impl AvailableEncoders {
    pub fn has_nvidia_nvenc(&self) -> bool {
        self.h264_nvenc || self.hevc_nvenc
    }

    pub fn has_intel_qsv(&self) -> bool {
        self.h264_qsv || self.hevc_qsv
    }

    pub fn has_vaapi(&self) -> bool {
        self.h264_vaapi || self.hevc_vaapi
    }

    pub fn has_amd_amf(&self) -> bool {
        self.h264_amf || self.hevc_amf
    }

    pub fn any_hardware(&self) -> bool {
        self.has_nvidia_nvenc() || self.has_intel_qsv() || self.has_vaapi() || self.has_amd_amf()
    }
}

static ENCODERS_CACHE: std::sync::OnceLock<AvailableEncoders> = std::sync::OnceLock::new();

fn is_encoder_functional(codec: &str) -> bool {
    let output = ffmpeg_cmd()
        .args([
            "-v",
            "quiet",
            "-f",
            "lavfi",
            "-i",
            "color=s=256x256:d=0.04",
            "-frames:v",
            "1",
            "-c:v",
            codec,
            "-f",
            "null",
            "-",
        ])
        .output();
    matches!(output, Ok(o) if o.status.success())
}

/// Queries FFmpeg for supported hardware encoders, caching the result for subsequent calls.
pub fn detect_available_encoders() -> AvailableEncoders {
    *ENCODERS_CACHE.get_or_init(|| {
        let mut encoders = AvailableEncoders::default();
        let output = ffmpeg_cmd().args(["-v", "quiet", "-encoders"]).output();
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);

            if text.contains("h264_nvenc") && is_encoder_functional("h264_nvenc") {
                encoders.h264_nvenc = true;
                encoders.hevc_nvenc = text.contains("hevc_nvenc") && is_encoder_functional("hevc_nvenc");
            }
            if text.contains("h264_qsv") && is_encoder_functional("h264_qsv") {
                encoders.h264_qsv = true;
                encoders.hevc_qsv = text.contains("hevc_qsv") && is_encoder_functional("hevc_qsv");
            }
            if text.contains("h264_vaapi") && is_encoder_functional("h264_vaapi") {
                encoders.h264_vaapi = true;
                encoders.hevc_vaapi = text.contains("hevc_vaapi") && is_encoder_functional("hevc_vaapi");
            }
            if text.contains("h264_amf") && is_encoder_functional("h264_amf") {
                encoders.h264_amf = true;
                encoders.hevc_amf = text.contains("hevc_amf") && is_encoder_functional("hevc_amf");
            }
        }
        encoders
    })
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
