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

/// Windows `CREATE_NO_WINDOW` process creation flag (`0x08000000`).
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Applies Windows-specific process creation flags (`CREATE_NO_WINDOW`) to suppress console windows.
pub fn apply_no_window(cmd: &mut Command) -> &mut Command {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Returns the platform-appropriate executable filename (e.g., `ffmpeg.exe` on Windows, `ffmpeg` on Unix).
pub fn executable_filename(name: &str) -> String {
    let trimmed = name.trim();
    if cfg!(target_os = "windows")
        && !trimmed.to_ascii_lowercase().ends_with(".exe")
        && !trimmed.contains('\\')
        && !trimmed.contains('/')
    {
        format!("{}.exe", trimmed)
    } else {
        trimmed.to_string()
    }
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
/// 1. Environment overrides (`TRIM_FFMPEG_PATH`, `TRIM_FFPROBE_PATH`, `TRIM_FFPLAY_PATH`)
/// 2. Next to the running executable (`./ffmpeg.exe`, `./bin/ffmpeg.exe` or `./ffmpeg`)
/// 3. Windows `%LOCALAPPDATA%\triM\bin\<name>.exe`
/// 4. In system PATH (`ffmpeg.exe` on Windows, `ffmpeg` on Unix)
pub fn resolve_binary_path(name: &str) -> PathBuf {
    let env_key = match name.trim_end_matches(".exe") {
        "ffmpeg" => Some("TRIM_FFMPEG_PATH"),
        "ffprobe" => Some("TRIM_FFPROBE_PATH"),
        "ffplay" => Some("TRIM_FFPLAY_PATH"),
        _ => None,
    };
    if let Some(key) = env_key {
        if let Ok(val) = std::env::var(key) {
            let p = PathBuf::from(val.trim());
            if p.is_file() {
                return p;
            }
        }
    }

    let exe_name = executable_filename(name);

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let direct = exe_dir.join(&exe_name);
            if direct.is_file() {
                return direct;
            }
            let sub_bin = exe_dir.join("bin").join(&exe_name);
            if sub_bin.is_file() {
                return sub_bin;
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            if !localappdata.is_empty() {
                let candidate = PathBuf::from(localappdata)
                    .join("triM")
                    .join("bin")
                    .join(&exe_name);
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }

    PathBuf::from(exe_name)
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

/// Continuously drains a child process's stderr in a dedicated background worker
/// to prevent OS pipe buffer exhaustion (e.g. 64KB Linux default) and avoid process deadlocks.
pub fn drain_stderr(stderr: Option<std::process::ChildStderr>, thread_name: &'static str) {
    if let Some(mut err_reader) = stderr {
        let _ = std::thread::Builder::new()
            .name(thread_name.to_string())
            .spawn(move || {
                let mut sink = std::io::sink();
                let _ = std::io::copy(&mut err_reader, &mut sink);
            });
    }
}

/// RAII process guard that guarantees spawned child processes are cleanly killed
/// and reaped on drop via `std::process::Child::kill()` + `.wait()`, preventing
/// zombie or orphan background processes across Linux, Windows, and macOS.
#[derive(Debug)]
pub struct ChildGuard {
    child: Option<std::process::Child>,
    exit_status: Option<std::process::ExitStatus>,
}

impl ChildGuard {
    pub fn new(child: std::process::Child) -> Self {
        Self {
            child: Some(child),
            exit_status: None,
        }
    }

    pub fn spawn(cmd: &mut Command) -> std::io::Result<Self> {
        apply_no_window(cmd);
        let child = cmd.spawn()?;
        Ok(Self::new(child))
    }

    pub fn take_stdin(&mut self) -> Option<std::process::ChildStdin> {
        self.child.as_mut().and_then(|c| c.stdin.take())
    }

    pub fn take_stdout(&mut self) -> Option<std::process::ChildStdout> {
        self.child.as_mut().and_then(|c| c.stdout.take())
    }

    pub fn take_stderr(&mut self) -> Option<std::process::ChildStderr> {
        self.child.as_mut().and_then(|c| c.stderr.take())
    }

    pub fn kill(&mut self) -> std::io::Result<()> {
        if let Some(mut c) = self.child.take() {
            // Drop stdin first so pipe-reading processes (like ffmpeg/ffplay) can terminate
            let _ = c.stdin.take();
            if let Ok(Some(st)) = c.try_wait() {
                self.exit_status = Some(st);
                return Ok(());
            }
            let _ = c.kill();
            if let Ok(st) = c.wait() {
                self.exit_status = Some(st);
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    pub fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        if let Some(st) = self.exit_status {
            return Ok(st);
        }
        if let Some(mut c) = self.child.take() {
            let st = c.wait()?;
            self.exit_status = Some(st);
            Ok(st)
        } else {
            Err(std::io::Error::other(
                "Process has already exited or been reaped",
            ))
        }
    }

    pub fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        if let Some(st) = self.exit_status {
            return Ok(Some(st));
        }
        if let Some(ref mut c) = self.child {
            let st_opt = c.try_wait()?;
            if let Some(st) = st_opt {
                self.exit_status = Some(st);
                self.child = None;
            }
            Ok(st_opt)
        } else {
            Ok(None)
        }
    }

    pub fn into_inner(mut self) -> Option<std::process::Child> {
        self.child.take()
    }
}

impl std::ops::Deref for ChildGuard {
    type Target = std::process::Child;
    fn deref(&self) -> &Self::Target {
        self.child
            .as_ref()
            .expect("Child process was already consumed")
    }
}

impl std::ops::DerefMut for ChildGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.child
            .as_mut()
            .expect("Child process was already consumed")
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.stdin.take();
            let _ = c.kill();
            let _ = c.wait();
        }
    }
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
                encoders.hevc_nvenc =
                    text.contains("hevc_nvenc") && is_encoder_functional("hevc_nvenc");
            }
            if text.contains("h264_qsv") && is_encoder_functional("h264_qsv") {
                encoders.h264_qsv = true;
                encoders.hevc_qsv = text.contains("hevc_qsv") && is_encoder_functional("hevc_qsv");
            }
            if text.contains("h264_vaapi") && is_encoder_functional("h264_vaapi") {
                encoders.h264_vaapi = true;
                encoders.hevc_vaapi =
                    text.contains("hevc_vaapi") && is_encoder_functional("hevc_vaapi");
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
        let prog = cmd.get_program().to_string_lossy().to_lowercase();
        assert!(prog.contains("ffmpeg"));
        #[cfg(target_os = "windows")]
        assert!(prog.ends_with(".exe"));
    }

    #[test]
    fn test_resolve_binary_fallback() {
        let p = resolve_binary_path("custom_tool_xyz");
        if cfg!(target_os = "windows") {
            assert_eq!(p, PathBuf::from("custom_tool_xyz.exe"));
        } else {
            assert_eq!(p, PathBuf::from("custom_tool_xyz"));
        }
    }

    #[test]
    fn test_child_guard_lifecycle() {
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("ping");
            c.args(["-n", "6", "127.0.0.1"]);
            c
        } else {
            let mut c = Command::new("sleep");
            c.arg("5");
            c
        };
        if let Ok(mut guard) = ChildGuard::spawn(&mut cmd) {
            assert!(guard.try_wait().unwrap().is_none());
            assert!(guard.kill().is_ok());
            assert!(guard.try_wait().unwrap().is_some());
        }
    }

    #[test]
    fn test_drain_stderr_prevents_pipe_deadlock() {
        use std::process::Stdio;
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", "for /L %i in (1,1,3000) do @echo 1234567890123456789012345678901234567890 1>&2"]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", "for i in $(seq 1 4000); do echo '12345678901234567890123456789012345678901234567890' >&2; done"]);
            c
        };
        cmd.stderr(Stdio::piped()).stdout(Stdio::null());
        let mut child = cmd.spawn().expect("Failed to spawn test process");

        // Drain stderr in background thread
        drain_stderr(child.stderr.take(), "test-drain-stderr");

        // The process writing >150KB to stderr MUST complete without deadlocking
        let start = std::time::Instant::now();
        let mut finished = false;
        while start.elapsed().as_secs() < 3 {
            if let Ok(Some(_)) = child.try_wait() {
                finished = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        if !finished {
            let _ = child.kill();
            panic!("Process deadlocked on full pipe buffer!");
        }
    }
}
