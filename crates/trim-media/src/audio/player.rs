use std::io::Write;
use std::process::Stdio;

/// Cross-platform audio stream player writing 48kHz 16-bit stereo PCM.
/// - Linux: PipeWire (`pw-cat`) -> PulseAudio (`paplay`) -> ALSA (`aplay`) -> `ffplay`
///   Automatically fails over to the next backend if a spawned player exits prematurely (`BrokenPipe`).
/// - Windows: `ffplay.exe` (`CREATE_NO_WINDOW`, low-delay WASAPI/DirectSound output) with silent fallback
pub struct AudioStreamPlayer {
    pub(super) child: Option<crate::ffmpeg::ChildGuard>,
    backend_index: usize,
    backend_unavailable: bool,
    last_recovery_attempt: Option<std::time::Instant>,
}

impl Default for AudioStreamPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioStreamPlayer {
    pub fn new() -> Self {
        Self {
            child: None,
            backend_index: 0,
            backend_unavailable: false,
            last_recovery_attempt: None,
        }
    }

    /// Manually resets the audio device backend (kills current sink and restarts from primary backend).
    pub fn reset_device(&mut self) {
        self.stop();
        self.last_recovery_attempt = None;
        self.prepare();
    }

    #[cfg(target_os = "windows")]
    fn spawn_backend(start_idx: usize) -> Option<(usize, crate::ffmpeg::ChildGuard)> {
        if start_idx > 0 {
            return None;
        }
        let mut ffplay = crate::ffmpeg::ffplay_cmd();
        ffplay
            .args([
                "-nodisp",
                "-autoexit",
                "-loglevel",
                "quiet",
                "-fflags",
                "nobuffer",
                "-flags",
                "low_delay",
                "-f",
                "s16le",
                "-ar",
                "48000",
                "-ac",
                "2",
                "-i",
                "pipe:0",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        crate::ffmpeg::ChildGuard::spawn(&mut ffplay)
            .ok()
            .map(|c| (0, c))
    }

    #[cfg(target_os = "macos")]
    fn spawn_backend(_start_idx: usize) -> Option<(usize, crate::ffmpeg::ChildGuard)> {
        let mut ffplay = crate::ffmpeg::ffplay_cmd();
        ffplay
            .args([
                "-nodisp",
                "-autoexit",
                "-loglevel",
                "quiet",
                "-f",
                "s16le",
                "-ar",
                "48000",
                "-ac",
                "2",
                "-i",
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        crate::ffmpeg::ChildGuard::spawn(&mut ffplay)
            .ok()
            .map(|c| (0, c))
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn spawn_backend(start_idx: usize) -> Option<(usize, crate::ffmpeg::ChildGuard)> {
        for idx in start_idx..4 {
            let res = match idx {
                0 => {
                    let mut pw_cmd = crate::ffmpeg::create_command("pw-cat");
                    pw_cmd
                        .args([
                            "-p",
                            "--format=s16",
                            "--rate=48000",
                            "--channels=2",
                            "--latency=20ms",
                            "-",
                        ])
                        .stdin(Stdio::piped())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null());
                    crate::ffmpeg::ChildGuard::spawn(&mut pw_cmd)
                }
                1 => {
                    let mut paplay_cmd = crate::ffmpeg::create_command("paplay");
                    paplay_cmd
                        .args([
                            "--raw",
                            "--format=s16le",
                            "--rate=48000",
                            "--channels=2",
                            "--latency-msec=25",
                        ])
                        .stdin(Stdio::piped())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null());
                    crate::ffmpeg::ChildGuard::spawn(&mut paplay_cmd)
                }
                2 => {
                    let mut aplay_cmd = crate::ffmpeg::create_command("aplay");
                    aplay_cmd
                        .args([
                            "-q",
                            "-t",
                            "raw",
                            "-c",
                            "2",
                            "-f",
                            "S16_LE",
                            "-r",
                            "48000",
                            "--buffer-time=25000",
                            "--period-time=5000",
                            "-",
                        ])
                        .stdin(Stdio::piped())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null());
                    crate::ffmpeg::ChildGuard::spawn(&mut aplay_cmd)
                }
                _ => {
                    let mut ffplay = crate::ffmpeg::ffplay_cmd();
                    ffplay
                        .args([
                            "-nodisp",
                            "-autoexit",
                            "-loglevel",
                            "quiet",
                            "-f",
                            "s16le",
                            "-ar",
                            "48000",
                            "-ac",
                            "2",
                            "-i",
                            "-",
                        ])
                        .stdin(Stdio::piped())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null());
                    crate::ffmpeg::ChildGuard::spawn(&mut ffplay)
                }
            };

            if let Ok(mut guard) = res {
                // Verify the child didn't immediately exit on launch
                if guard.try_wait().ok().flatten().is_none() {
                    return Some((idx, guard));
                }
            }
        }
        None
    }

    fn ensure_started(&mut self) -> Option<&mut std::process::ChildStdin> {
        if let Some(ref mut c) = self.child {
            if c.try_wait().ok().flatten().is_some() {
                self.child = None;
                self.backend_index += 1;
            }
        }

        if self.child.is_none() && !self.backend_unavailable {
            match Self::spawn_backend(self.backend_index) {
                Some((idx, c)) => {
                    self.backend_index = idx;
                    self.child = Some(c);
                }
                None => {
                    // Prevent repeated process-spawn attempts during the same playback session
                    self.backend_unavailable = true;
                }
            }
        }

        self.child.as_mut().and_then(|c| c.stdin_mut())
    }

    /// Pre-spawns audio output process to eliminate startup delay upon clicking play.
    pub fn prepare(&mut self) {
        self.backend_unavailable = false;
        let _ = self.ensure_started();
    }

    /// Writes raw 16-bit stereo PCM samples to the active audio device, failing over to the
    /// next platform backend (`pw-cat` -> `paplay` -> `aplay` -> `ffplay`) if a pipe breaks.
    pub fn write_samples(&mut self, samples: &[i16]) {
        let _ = self.write_samples_checked(samples);
    }

    /// Writes raw 16-bit stereo PCM samples and returns `true` if accepted by an active sink.
    pub fn write_samples_checked(&mut self, samples: &[i16]) -> bool {
        if self.backend_unavailable {
            let can_retry = self
                .last_recovery_attempt
                .is_none_or(|t| t.elapsed() >= std::time::Duration::from_secs(2));
            if can_retry {
                self.backend_unavailable = false;
                self.backend_index = 0;
                self.last_recovery_attempt = Some(std::time::Instant::now());
            }
        }
        if samples.is_empty() {
            return !self.backend_unavailable;
        }

        let mut byte_buf = Vec::with_capacity(samples.len() * 2);
        for &s in samples {
            byte_buf.extend_from_slice(&s.to_le_bytes());
        }

        while !self.backend_unavailable {
            let write_ok = if let Some(stdin) = self.ensure_started() {
                stdin.write_all(&byte_buf).is_ok()
            } else {
                false
            };

            if write_ok {
                return true;
            }

            if let Some(mut c) = self.child.take() {
                let _ = c.kill();
            }
            self.last_recovery_attempt = Some(std::time::Instant::now());
            if self.backend_unavailable {
                break;
            }
            self.backend_index += 1;
        }
        false
    }

    /// Stops audio playback immediately and kills the child audio process.
    pub fn stop(&mut self) {
        self.backend_unavailable = false;
        self.backend_index = 0;
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
        }
    }
}

impl Drop for AudioStreamPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}
