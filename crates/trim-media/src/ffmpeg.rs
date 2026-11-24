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

static FFMPEG_AVAIL_CACHE: std::sync::OnceLock<std::sync::RwLock<Option<bool>>> =
    std::sync::OnceLock::new();
static FFPROBE_AVAIL_CACHE: std::sync::OnceLock<std::sync::RwLock<Option<bool>>> =
    std::sync::OnceLock::new();

/// Clears cached FFmpeg/FFprobe availability so the next check re-probes the system.
pub fn refresh_ffmpeg_availability() {
    if let Ok(mut w) = FFMPEG_AVAIL_CACHE
        .get_or_init(|| std::sync::RwLock::new(None))
        .write()
    {
        *w = None;
    }
    if let Ok(mut w) = FFPROBE_AVAIL_CACHE
        .get_or_init(|| std::sync::RwLock::new(None))
        .write()
    {
        *w = None;
    }
}

/// Checks whether `ffmpeg` is available and executable (cached across UI frames).
pub fn is_ffmpeg_available() -> bool {
    let lock = FFMPEG_AVAIL_CACHE.get_or_init(|| std::sync::RwLock::new(None));
    if let Ok(r) = lock.read() {
        if let Some(val) = *r {
            return val;
        }
    }
    let avail = ffmpeg_cmd()
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if let Ok(mut w) = lock.write() {
        *w = Some(avail);
    }
    avail
}

/// Checks whether `ffprobe` is available and executable (cached across UI frames).
pub fn is_ffprobe_available() -> bool {
    let lock = FFPROBE_AVAIL_CACHE.get_or_init(|| std::sync::RwLock::new(None));
    if let Ok(r) = lock.read() {
        if let Some(val) = *r {
            return val;
        }
    }
    let avail = ffprobe_cmd()
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if let Ok(mut w) = lock.write() {
        *w = Some(avail);
    }
    avail
}

/// Checks whether the `libx264` encoder is compiled into the active `ffmpeg` binary
/// (e.g. Fedora's `ffmpeg-free` omits `libx264`).
pub fn is_libx264_available() -> bool {
    static LIBX264_AVAIL: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *LIBX264_AVAIL.get_or_init(|| {
        ffmpeg_cmd()
            .args(["-v", "quiet", "-encoders"])
            .output()
            .is_ok_and(|o| {
                o.status.success() && String::from_utf8_lossy(&o.stdout).contains("libx264")
            })
    })
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

    pub fn stdin_mut(&mut self) -> Option<&mut std::process::ChildStdin> {
        self.child.as_mut().and_then(|c| c.stdin.as_mut())
    }

    pub fn as_child(&self) -> Option<&std::process::Child> {
        self.child.as_ref()
    }

    pub fn id(&self) -> Option<u32> {
        self.child.as_ref().map(|c| c.id())
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

/// Owns short-lived helper child processes (e.g. platform file manager launchers) via `ChildGuard`.
/// Reaps finished processes non-blockingly on `poll_reap()` and guarantees kill+wait on `Drop`
/// so no detached threads or zombie/orphan processes can survive application shutdown or cancellation.
#[derive(Default)]
pub struct TrackedChildPool {
    children: Vec<ChildGuard>,
}

impl std::fmt::Debug for TrackedChildPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackedChildPool")
            .field("active_count", &self.children.len())
            .finish()
    }
}

impl TrackedChildPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reaps any children that have already exited without blocking.
    pub fn poll_reap(&mut self) {
        self.children
            .retain_mut(|guard| matches!(guard.try_wait(), Ok(None)));
    }

    /// Spawns `cmd` wrapped in `ChildGuard` and tracks it until exit or pool drop.
    pub fn spawn(&mut self, cmd: &mut Command) -> std::io::Result<Option<u32>> {
        self.poll_reap();
        let guard = ChildGuard::spawn(cmd)?;
        let pid = guard.id();
        self.children.push(guard);
        Ok(pid)
    }

    /// Returns the number of currently tracked active child processes (after non-blocking reap).
    pub fn active_count(&mut self) -> usize {
        self.poll_reap();
        self.children.len()
    }

    /// Immediately terminates and reaps all tracked child processes.
    pub fn kill_all(&mut self) {
        self.children.clear();
    }
}

/// Hardware decode backend detected via functional runtime probing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwAccelBackend {
    None,
    Vaapi,
    Cuda,
    Vulkan,
}

impl HwAccelBackend {
    pub fn as_ffmpeg_flag(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Vaapi => Some("vaapi"),
            Self::Cuda => Some("cuda"),
            Self::Vulkan => Some("vulkan"),
        }
    }
}

/// Probes FFmpeg hardware decode backends (`vaapi`, `cuda`, `vulkan`) at runtime using a
/// short bounded functional decode test. Silently falls back to `HwAccelBackend::None` on
/// any probe failure and logs the fallback reason.
pub fn probe_working_hwaccel() -> HwAccelBackend {
    static CACHED: std::sync::OnceLock<HwAccelBackend> = std::sync::OnceLock::new();
    *CACHED.get_or_init(probe_working_hwaccel_uncached)
}

pub fn probe_working_hwaccel_uncached() -> HwAccelBackend {
    if !is_ffmpeg_available() {
        return HwAccelBackend::None;
    }
    let mut list_cmd = ffmpeg_cmd();
    list_cmd.args(["-v", "error", "-hwaccels"]);
    let Ok(list_out) =
        run_command_bounded(&mut list_cmd, 16 * 1024, std::time::Duration::from_secs(2))
    else {
        log::debug!("ffmpeg -hwaccels failed; using software decode");
        return HwAccelBackend::None;
    };
    let supported = String::from_utf8_lossy(&list_out.stdout).to_lowercase();

    for backend in [
        HwAccelBackend::Vaapi,
        HwAccelBackend::Cuda,
        HwAccelBackend::Vulkan,
    ] {
        let Some(flag) = backend.as_ffmpeg_flag() else {
            continue;
        };
        if !supported.lines().any(|l| l.trim() == flag) {
            continue;
        }
        let mut probe_cmd = ffmpeg_cmd();
        probe_cmd.args([
            "-v",
            "error",
            "-hwaccel",
            flag,
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=64x64:rate=25:duration=0.1",
            "-frames:v",
            "1",
            "-f",
            "null",
            "-",
        ]);
        match run_command_bounded(&mut probe_cmd, 8 * 1024, std::time::Duration::from_secs(3)) {
            Ok(out) if out.status.success() => {
                log::info!("Runtime hwaccel probe succeeded: {:?}", backend);
                return backend;
            }
            Ok(out) => {
                log::debug!(
                    "Runtime hwaccel probe failed for {:?} (falling back to software): {}",
                    backend,
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            Err(e) => {
                log::debug!("Runtime hwaccel probe error for {:?}: {}", backend, e);
            }
        }
    }

    log::info!("No working hwaccel backend found; using software decode");
    HwAccelBackend::None
}

/// Runs a subprocess with bounded stdout/stderr capture (`max_bytes` per stream) and a hard timeout.
/// Prevents OOM and UI/worker hangs on corrupt or adversarial media files.
pub fn run_command_bounded(
    cmd: &mut Command,
    max_bytes: usize,
    timeout: std::time::Duration,
) -> std::io::Result<std::process::Output> {
    use std::io::Read;
    use std::process::Stdio;
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut guard = ChildGuard::spawn(cmd)?;
    let stdout_pipe = guard.take_stdout();
    let stderr_pipe = guard.take_stderr();

    let out_handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(out) = stdout_pipe {
            let _ = out.take(max_bytes as u64).read_to_end(&mut buf);
        }
        buf
    });
    let err_handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(err) = stderr_pipe {
            let _ = err.take(max_bytes as u64).read_to_end(&mut buf);
        }
        buf
    });

    let start = std::time::Instant::now();
    let status = loop {
        if let Some(ref mut child) = guard.child {
            match child.try_wait() {
                Ok(Some(st)) => break st,
                Ok(None) => {
                    if start.elapsed() >= timeout {
                        let _ = guard.kill();
                        let st = guard.wait()?;
                        let _ = out_handle.join();
                        let _ = err_handle.join();
                        return Ok(std::process::Output {
                            status: st,
                            stdout: Vec::new(),
                            stderr: b"Process timed out".to_vec(),
                        });
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => return Err(e),
            }
        } else {
            return Err(std::io::Error::other("Child missing"));
        }
    };

    let stdout = out_handle.join().unwrap_or_default();
    let stderr = err_handle.join().unwrap_or_default();
    Ok(std::process::Output {
        status,
        stdout,
        stderr,
    })
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
    pub h264_videotoolbox: bool,
    pub hevc_videotoolbox: bool,
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

    pub fn has_videotoolbox(&self) -> bool {
        self.h264_videotoolbox || self.hevc_videotoolbox
    }

    pub fn any_hardware(&self) -> bool {
        self.has_nvidia_nvenc()
            || self.has_intel_qsv()
            || self.has_vaapi()
            || self.has_amd_amf()
            || self.has_videotoolbox()
    }
}

static ENCODERS_CACHE: std::sync::OnceLock<AvailableEncoders> = std::sync::OnceLock::new();
static VAAPI_DEVICE_CACHE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();

fn candidate_vaapi_devices() -> Vec<String> {
    let mut devs = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/dev/dri") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let s = name.to_string_lossy();
            if s.starts_with("renderD") {
                devs.push(entry.path().to_string_lossy().to_string());
            }
        }
    }
    devs.sort();
    if devs.is_empty() && std::path::Path::new("/dev/dri/renderD128").exists() {
        devs.push("/dev/dri/renderD128".to_string());
    }
    devs
}

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

fn is_vaapi_encoder_functional(codec: &str, device: &str) -> bool {
    let output = ffmpeg_cmd()
        .args([
            "-v",
            "quiet",
            "-vaapi_device",
            device,
            "-f",
            "lavfi",
            "-i",
            "color=s=256x256:d=0.04",
            "-vf",
            "format=nv12,hwupload",
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

/// Returns the first functional VAAPI render device node (e.g. `/dev/dri/renderD128`), if any.
pub fn detect_vaapi_device() -> Option<String> {
    VAAPI_DEVICE_CACHE
        .get_or_init(|| {
            if !cfg!(target_os = "linux") {
                return None;
            }
            candidate_vaapi_devices().into_iter().find(|dev| {
                is_vaapi_encoder_functional("h264_vaapi", dev)
                    || is_vaapi_encoder_functional("hevc_vaapi", dev)
            })
        })
        .clone()
}

/// Queries FFmpeg for supported hardware encoders, caching the result for subsequent calls.
pub fn detect_available_encoders() -> AvailableEncoders {
    *ENCODERS_CACHE.get_or_init(|| {
        let mut encoders = AvailableEncoders::default();
        let output = ffmpeg_cmd().args(["-v", "quiet", "-encoders"]).output();
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);

            encoders.h264_nvenc =
                text.contains("h264_nvenc") && is_encoder_functional("h264_nvenc");
            encoders.hevc_nvenc =
                text.contains("hevc_nvenc") && is_encoder_functional("hevc_nvenc");

            encoders.h264_qsv = text.contains("h264_qsv") && is_encoder_functional("h264_qsv");
            encoders.hevc_qsv = text.contains("hevc_qsv") && is_encoder_functional("hevc_qsv");

            if let Some(vaapi_dev) = detect_vaapi_device() {
                encoders.h264_vaapi = text.contains("h264_vaapi")
                    && is_vaapi_encoder_functional("h264_vaapi", &vaapi_dev);
                encoders.hevc_vaapi = text.contains("hevc_vaapi")
                    && is_vaapi_encoder_functional("hevc_vaapi", &vaapi_dev);
            }

            encoders.h264_amf = text.contains("h264_amf") && is_encoder_functional("h264_amf");
            encoders.hevc_amf = text.contains("hevc_amf") && is_encoder_functional("hevc_amf");

            encoders.h264_videotoolbox =
                text.contains("h264_videotoolbox") && is_encoder_functional("h264_videotoolbox");
            encoders.hevc_videotoolbox =
                text.contains("hevc_videotoolbox") && is_encoder_functional("hevc_videotoolbox");
        }
        encoders
    })
}

/// Parses `ffmpeg -list_devices true -f dshow -i dummy` output into `(video_devices, audio_devices)`.
pub fn parse_dshow_device_list(stderr: &str) -> (Vec<String>, Vec<String>) {
    let mut video = Vec::new();
    let mut audio = Vec::new();
    for line in stderr.lines() {
        let trimmed = line.trim();
        let Some(first_q) = trimmed.find('"') else {
            continue;
        };
        let rest = &trimmed[first_q + 1..];
        let Some(end_q) = rest.find('"') else {
            continue;
        };
        let name = rest[..end_q].to_string();
        if name.is_empty() || trimmed.contains("Alternative name") {
            continue;
        }
        let after = &rest[end_q + 1..];
        if after.contains("(video)") {
            video.push(name);
        } else if after.contains("(audio)") {
            audio.push(name);
        }
    }
    (video, audio)
}

/// Lists available Windows DirectShow `(video, audio)` device names via FFmpeg.
pub fn list_dshow_devices() -> (Vec<String>, Vec<String>) {
    let Ok(out) = ffmpeg_cmd()
        .args([
            "-hide_banner",
            "-list_devices",
            "true",
            "-f",
            "dshow",
            "-i",
            "dummy",
        ])
        .output()
    else {
        return (Vec::new(), Vec::new());
    };
    let stderr = String::from_utf8_lossy(&out.stderr);
    parse_dshow_device_list(&stderr)
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
            c.args([
                "/C",
                "for /L %i in (1,1,3000) do @echo 1234567890123456789012345678901234567890 1>&2",
            ]);
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
        }
        let _ = child.wait();
        assert!(finished, "Process deadlocked on full pipe buffer!");
    }

    #[test]
    fn test_parse_dshow_device_list() {
        let sample = "\
[dshow @ 000001] \"Integrated Camera\" (video)
[dshow @ 000001]   Alternative name \"@device_pnp_\\\\?\\usb#vid_1234\"
[dshow @ 000001] \"OBS Virtual Camera\" (video)
[dshow @ 000001] \"Microphone (Realtek(R) Audio)\" (audio)
[dshow @ 000001] \"Stereo Mix (Realtek(R) Audio)\" (audio)
";
        let (video, audio) = parse_dshow_device_list(sample);
        assert_eq!(video, vec!["Integrated Camera", "OBS Virtual Camera"]);
        assert_eq!(
            audio,
            vec![
                "Microphone (Realtek(R) Audio)",
                "Stereo Mix (Realtek(R) Audio)"
            ]
        );
    }

    #[test]
    fn test_tracked_child_pool_cancel_and_drop_kills_without_zombie() {
        let mut pool = TrackedChildPool::new();
        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("ping");
            c.args(["-n", "30", "127.0.0.1"]);
            c
        };
        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new("sleep");
            c.arg("30");
            c
        };

        let pid = pool
            .spawn(&mut cmd)
            .expect("spawn sleep")
            .expect("child pid");
        assert_eq!(pool.active_count(), 1);

        // Cancel/drop must kill and wait immediately so no process or zombie remains
        pool.kill_all();
        assert_eq!(pool.active_count(), 0);

        #[cfg(target_os = "linux")]
        {
            let proc_path = std::path::PathBuf::from(format!("/proc/{pid}"));
            assert!(
                !proc_path.exists(),
                "Tracked child PID {pid} or zombie still exists in /proc after kill_all!"
            );
        }
        #[cfg(not(target_os = "linux"))]
        let _ = pid;
    }

    #[test]
    fn test_probe_working_hwaccel_never_panics_and_caches() {
        let first = probe_working_hwaccel();
        let second = probe_working_hwaccel();
        assert_eq!(first, second);
    }
}
