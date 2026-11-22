//! Live voiceover and timeline dubbing recorder using native audio capture via FFmpeg.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::time::Instant;

/// Active live voiceover recording session.
pub struct VoiceoverRecorder {
    child: Option<Child>,
    output_path: PathBuf,
    start_time: Instant,
}

impl VoiceoverRecorder {
    /// Starts recording audio from system default microphone into `output_path`.
    pub fn start<P: AsRef<Path>>(output_path: P) -> Result<Self, String> {
        let out = output_path.as_ref().to_path_buf();
        if let Some(parent) = out.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let out_str = out.to_string_lossy().to_string();

        // Cross-platform audio device argument resolution
        let mut child = if cfg!(target_os = "windows") {
            let (_, audio_devs) = crate::ffmpeg::list_dshow_devices();
            let first_audio = audio_devs
                .first()
                .ok_or_else(|| "Windows DirectShow mikrofon aygıtı bulunamadı.".to_string())?;
            let escaped_audio = first_audio.replace(':', "\\:");
            let input_spec = format!("audio={escaped_audio}");
            Self::spawn_ffmpeg(&[
                "-f",
                "dshow",
                "-i",
                &input_spec,
                "-ac",
                "2",
                "-ar",
                "44100",
                "-c:a",
                "pcm_s16le",
                &out_str,
            ])
            .map_err(|e| format!("Ses kaydı başlatılamadı: {e}"))?
        } else if cfg!(target_os = "macos") {
            Self::spawn_ffmpeg(&[
                "-f",
                "avfoundation",
                "-i",
                ":0",
                "-ac",
                "2",
                "-ar",
                "44100",
                "-c:a",
                "pcm_s16le",
                &out_str,
            ])
            .map_err(|e| format!("Ses kaydı başlatılamadı: {e}"))?
        } else {
            // Linux: Try PulseAudio/PipeWire default first, then fallback to ALSA default
            let pulse_res = Self::spawn_ffmpeg(&[
                "-f",
                "pulse",
                "-i",
                "default",
                "-ac",
                "2",
                "-ar",
                "44100",
                "-c:a",
                "pcm_s16le",
                &out_str,
            ]);
            let mut try_alsa = false;
            let chosen = match pulse_res {
                Ok(mut c) => {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    if matches!(c.try_wait(), Ok(Some(_))) {
                        try_alsa = true;
                        None
                    } else {
                        Some(c)
                    }
                }
                Err(_) => {
                    try_alsa = true;
                    None
                }
            };
            if let Some(c) = chosen {
                c
            } else if try_alsa {
                Self::spawn_ffmpeg(&[
                    "-f",
                    "alsa",
                    "-i",
                    "default",
                    "-ac",
                    "2",
                    "-ar",
                    "44100",
                    "-c:a",
                    "pcm_s16le",
                    &out_str,
                ])
                .map_err(|e| format!("Ses kaydı başlatılamadı: {e}"))?
            } else {
                return Err("Ses kaydı başlatılamadı".to_string());
            }
        };

        std::thread::sleep(std::time::Duration::from_millis(200));
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!("Ses kaydı başlatılırken sonlandı ({status})"));
        }
        crate::ffmpeg::drain_stderr(child.stderr.take(), "trim-voiceover-stderr");
        Ok(Self {
            child: Some(child),
            output_path: out,
            start_time: Instant::now(),
        })
    }

    fn spawn_ffmpeg(extra_args: &[&str]) -> Result<Child, std::io::Error> {
        let mut cmd = crate::ffmpeg::ffmpeg_cmd();
        cmd.arg("-y");
        cmd.arg("-v").arg("error");
        cmd.args(extra_args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());
        cmd.spawn()
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

    /// Stops recording gracefully and returns the final recorded duration in seconds.
    pub fn stop(&mut self) -> Result<f64, String> {
        let elapsed = self.elapsed_seconds();
        if let Some(mut c) = self.child.take() {
            // Send 'q' to FFmpeg stdin so it finalizes the WAV header properly
            if let Some(mut stdin) = c.stdin.take() {
                let _ = stdin.write_all(b"q\n");
                let _ = stdin.flush();
            }

            // Give ffmpeg up to 1 second to cleanly exit
            let start = Instant::now();
            let mut exited = false;
            while start.elapsed().as_millis() < 1000 {
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

        // Validate the output file
        if self.output_path.exists() {
            let metadata = std::fs::metadata(&self.output_path)
                .map_err(|e| format!("Kayıt dosyası doğrulanamadı: {}", e))?;
            if metadata.len() > 100 {
                return Ok(elapsed.max(0.1));
            }
        }

        Err("Kayıt dosyası oluşturulamadı veya boş.".to_string())
    }

    /// Cancels recording and deletes the incomplete file.
    pub fn cancel(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        if self.output_path.exists() {
            let _ = std::fs::remove_file(&self.output_path);
        }
    }
}

impl Drop for VoiceoverRecorder {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voiceover_recorder_lifecycle_mock() {
        let temp_dir = std::env::temp_dir();
        let target = temp_dir.join("trim_vo_test_mock.wav");
        if target.exists() {
            let _ = std::fs::remove_file(&target);
        }

        // Just test that output path is retained and elapsed_seconds increments
        let recorder = VoiceoverRecorder {
            child: None,
            output_path: target.clone(),
            start_time: Instant::now(),
        };
        assert_eq!(recorder.output_path(), target.as_path());
        assert!(recorder.elapsed_seconds() >= 0.0);
    }
}
