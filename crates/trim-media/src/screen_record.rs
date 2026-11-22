//! Live screen and webcam recorder using native FFmpeg capture devices.
//!
//! Captures screen, webcam, and audio without needing external software like OBS.
//! Automatically resolves platform capture devices:
//! - Linux: `x11grab` / `pulse` / `v4l2`
//! - Windows: `gdigrab` / `dshow`
//! - macOS: `avfoundation`

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaptureSource {
    #[default]
    ScreenAndMic,
    ScreenOnly,
    WebcamOnly,
}

impl CaptureSource {
    pub const ALL: &'static [Self] = &[Self::ScreenAndMic, Self::ScreenOnly, Self::WebcamOnly];

    pub fn label(&self) -> &'static str {
        match self {
            Self::ScreenAndMic => "⊞ Ekran + Mikrofon",
            Self::ScreenOnly => "⊞ Sadece Ekran (Sessiz)",
            Self::WebcamOnly => "● Sadece Web Kamerası",
        }
    }
}

/// Active live screen / webcam recording session.
pub struct ScreenRecorder {
    child: Option<Child>,
    output_path: PathBuf,
    start_time: Instant,
    source: CaptureSource,
}

impl ScreenRecorder {
    /// Starts recording from selected source into `output_path`.
    pub fn start<P: AsRef<Path>>(output_path: P, source: CaptureSource) -> Result<Self, String> {
        let out = output_path.as_ref().to_path_buf();
        if let Some(parent) = out.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let out_str = out.to_string_lossy().to_string();

        let mut cmd = crate::ffmpeg::ffmpeg_cmd();
        cmd.arg("-y");
        cmd.arg("-v").arg("error");

        // Configure input capture devices per platform and source
        if cfg!(target_os = "windows") {
            match source {
                CaptureSource::ScreenOnly => {
                    cmd.args(["-f", "gdigrab", "-framerate", "30", "-i", "desktop"]);
                }
                CaptureSource::ScreenAndMic => {
                    let (_, audio_devs) = crate::ffmpeg::list_dshow_devices();
                    let first_audio = audio_devs.first().ok_or_else(|| {
                        "Windows DirectShow mikrofon aygıtı bulunamadı.".to_string()
                    })?;
                    let spec = format!("audio={}", first_audio.replace(':', "\\:"));
                    cmd.args(["-f", "gdigrab", "-framerate", "30", "-i", "desktop"]);
                    cmd.args(["-f", "dshow", "-i", &spec]);
                }
                CaptureSource::WebcamOnly => {
                    let (video_devs, audio_devs) = crate::ffmpeg::list_dshow_devices();
                    let v = video_devs.first().ok_or_else(|| {
                        "Windows DirectShow kamera aygıtı bulunamadı.".to_string()
                    })?;
                    let v_esc = v.replace(':', "\\:");
                    let spec = if let Some(a) = audio_devs.first() {
                        format!("video={v_esc}:audio={}", a.replace(':', "\\:"))
                    } else {
                        format!("video={v_esc}")
                    };
                    cmd.args(["-f", "dshow", "-i", &spec]);
                }
            }
        } else if cfg!(target_os = "macos") {
            match source {
                CaptureSource::ScreenOnly => {
                    cmd.args(["-f", "avfoundation", "-framerate", "30", "-i", "1:none"]);
                }
                CaptureSource::ScreenAndMic => {
                    cmd.args(["-f", "avfoundation", "-framerate", "30", "-i", "1:0"]);
                }
                CaptureSource::WebcamOnly => {
                    cmd.args(["-f", "avfoundation", "-framerate", "30", "-i", "0:0"]);
                }
            }
        } else {
            // Linux: x11grab + pulse / v4l2 (warn clearly on pure Wayland sessions)
            let is_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some()
                || std::env::var("XDG_SESSION_TYPE")
                    .is_ok_and(|v| v.eq_ignore_ascii_case("wayland"));
            let display = std::env::var("DISPLAY").unwrap_or_default();
            if is_wayland && display.is_empty() && source != CaptureSource::WebcamOnly {
                return Err(
                    "Saf Wayland oturumunda x11grab desteklenmiyor. Lütfen X11/XWayland oturumu kullanın."
                        .to_string(),
                );
            }
            let eff_display = if display.is_empty() {
                ":0.0".to_string()
            } else {
                display
            };
            match source {
                CaptureSource::ScreenOnly => {
                    cmd.args(["-f", "x11grab", "-framerate", "30", "-i", &eff_display]);
                }
                CaptureSource::ScreenAndMic => {
                    cmd.args(["-f", "x11grab", "-framerate", "30", "-i", &eff_display]);
                    cmd.args(["-f", "pulse", "-i", "default"]);
                }
                CaptureSource::WebcamOnly => {
                    cmd.args(["-f", "v4l2", "-framerate", "30", "-i", "/dev/video0"]);
                    cmd.args(["-f", "pulse", "-i", "default"]);
                }
            }
        }

        // Fast video encoding settings for low latency desktop recording
        cmd.args([
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "23",
            "-pix_fmt",
            "yuv420p",
        ]);

        if source != CaptureSource::ScreenOnly {
            cmd.args(["-c:a", "aac", "-b:a", "192k"]);
        }

        cmd.arg(&out_str);

        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());

        match cmd.spawn() {
            Ok(mut child) => {
                std::thread::sleep(std::time::Duration::from_millis(200));
                if let Ok(Some(status)) = child.try_wait() {
                    return Err(format!("Ekran kaydı başlatılırken sonlandı ({status})"));
                }
                crate::ffmpeg::drain_stderr(child.stderr.take(), "trim-screenrecord-stderr");
                Ok(Self {
                    child: Some(child),
                    output_path: out,
                    start_time: Instant::now(),
                    source,
                })
            }
            Err(e) => Err(format!("Ekran kaydı başlatılamadı: {}", e)),
        }
    }

    pub fn is_recording(&mut self) -> bool {
        if let Some(ref mut c) = self.child {
            matches!(c.try_wait(), Ok(None))
        } else {
            false
        }
    }

    pub fn elapsed_seconds(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    pub fn output_path(&self) -> &Path {
        &self.output_path
    }

    pub fn source(&self) -> CaptureSource {
        self.source
    }

    /// Stops recording gracefully via stdin signal and finalizes the MP4 file.
    pub fn stop(&mut self) -> Result<f64, String> {
        let elapsed = self.elapsed_seconds();
        if let Some(mut c) = self.child.take() {
            if let Some(mut stdin) = c.stdin.take() {
                let _ = stdin.write_all(b"q\n");
                let _ = stdin.flush();
            }

            let start = Instant::now();
            let mut exited = false;
            while start.elapsed().as_millis() < 2000 {
                if let Ok(Some(_)) = c.try_wait() {
                    exited = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }

            if !exited {
                let _ = c.kill();
                let _ = c.wait();
            }
        }

        Ok(elapsed)
    }
}

impl Drop for ScreenRecorder {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.take() {
            if let Some(mut stdin) = c.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(b"q\n");
                let _ = stdin.flush();
            }
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
            let mut exited = false;
            while std::time::Instant::now() < deadline {
                if let Ok(Some(_)) = c.try_wait() {
                    exited = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            if !exited {
                let _ = c.kill();
                let _ = c.wait();
            }
        }
    }
}
