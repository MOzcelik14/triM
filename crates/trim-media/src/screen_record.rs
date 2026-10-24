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
            Self::ScreenAndMic => "🖥️ Ekran + Mikrofon",
            Self::ScreenOnly => "🖥️ Sadece Ekran (Sessiz)",
            Self::WebcamOnly => "📷 Sadece Web Kamerası",
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
                    cmd.args(["-f", "gdigrab", "-framerate", "30", "-i", "desktop"]);
                    cmd.args(["-f", "dshow", "-i", "audio=default"]);
                }
                CaptureSource::WebcamOnly => {
                    cmd.args(["-f", "dshow", "-i", "video=default:audio=default"]);
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
            // Linux: x11grab + pulse / v4l2
            let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0.0".to_string());
            match source {
                CaptureSource::ScreenOnly => {
                    cmd.args(["-f", "x11grab", "-framerate", "30", "-i", &display]);
                }
                CaptureSource::ScreenAndMic => {
                    cmd.args(["-f", "x11grab", "-framerate", "30", "-i", &display]);
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
            Ok(child) => Ok(Self {
                child: Some(child),
                output_path: out,
                start_time: Instant::now(),
                source,
            }),
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
            }
        }

        Ok(elapsed)
    }
}
