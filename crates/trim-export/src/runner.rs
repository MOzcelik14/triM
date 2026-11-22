//! Background export runner monitoring FFmpeg progress.

use crate::filtergraph::TimelineExporter;
use crate::presets::ExportPreset;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use trim_core::project::Project;
use trim_media::ChildGuard;

/// Background watcher that kills the export child as soon as `cancel` is raised, even while the
/// export thread is blocked reading FFmpeg's progress pipe.
///
/// Its `Drop` always raises the `done` flag and joins the thread, so the watcher terminates on every
/// exit path of the export thread, including a panic (unwind). Once the watcher has stopped, the
/// last `Arc` to the `ChildGuard` is released and the guard's `Drop` kills and reaps FFmpeg.
pub(crate) struct CancelWatcher {
    done: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl CancelWatcher {
    pub(crate) fn spawn(
        child: Arc<std::sync::Mutex<Option<ChildGuard>>>,
        cancel: Arc<AtomicBool>,
    ) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let watcher_done = Arc::clone(&done);
        let handle = thread::spawn(move || {
            while !watcher_done.load(Ordering::Acquire) {
                if cancel.load(Ordering::SeqCst) {
                    if let Ok(mut guard) = child.lock() {
                        if let Some(ref mut c) = *guard {
                            let _ = c.kill();
                        }
                    }
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(50));
            }
        });
        Self {
            done,
            handle: Some(handle),
        }
    }
}

impl Drop for CancelWatcher {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Release);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExportMessage {
    Progress(f32), // 0.0 .. 100.0
    Status(String),
    Finished(Result<String, String>),
}

pub struct ExportRunner {
    pub is_cancelled: Arc<AtomicBool>,
    active_job_cancel: std::sync::Mutex<Arc<AtomicBool>>,
}

impl Drop for ExportRunner {
    fn drop(&mut self) {
        self.cancel();
    }
}

impl Default for ExportRunner {
    fn default() -> Self {
        Self {
            is_cancelled: Arc::new(AtomicBool::new(false)),
            active_job_cancel: std::sync::Mutex::new(Arc::new(AtomicBool::new(false))),
        }
    }
}

impl ExportRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
        if let Ok(guard) = self.active_job_cancel.lock() {
            guard.store(true, Ordering::SeqCst);
        }
    }

    pub fn start_export(
        &self,
        project: Project,
        preset: ExportPreset,
        output_path: String,
        range: Option<(f64, f64)>,
        tx: Sender<ExportMessage>,
    ) {
        self.is_cancelled.store(false, Ordering::SeqCst);
        let job_cancel = Arc::new(AtomicBool::new(false));
        if let Ok(mut guard) = self.active_job_cancel.lock() {
            guard.store(true, Ordering::SeqCst);
            *guard = Arc::clone(&job_cancel);
        }

        thread::spawn(move || {
            let prepared = match TimelineExporter::build_export_pipeline_prepared(
                &project,
                &preset,
                &output_path,
                range,
            ) {
                Ok(res) => res,
                Err(err) => {
                    let _ = tx.send(ExportMessage::Finished(Err(err)));
                    return;
                }
            };
            let mut cmd_args = prepared.cmd_args;
            let total_duration = prepared.total_duration;
            let _temp_dir = prepared.temp_dir;

            // Avoid Windows 32KB CreateProcess CLI limit by writing large filtergraphs to a temp script
            // inside our private export TempDir (cleaned up automatically when _temp_dir drops).
            let mut _fallback_temp_dir: Option<tempfile::TempDir> = None;
            if let Some(fc_idx) = cmd_args.iter().position(|a| a == "-filter_complex") {
                if let Some(fg) = cmd_args.get(fc_idx + 1) {
                    if fg.len() > 16_000 {
                        let td_ref = match _temp_dir.as_ref() {
                            Some(td) => Some(td),
                            None => {
                                _fallback_temp_dir = tempfile::tempdir().ok();
                                _fallback_temp_dir.as_ref()
                            }
                        };
                        if let Some(td) = td_ref {
                            let script_path = td.path().join("filter_complex.txt");
                            if std::fs::write(&script_path, fg).is_ok() {
                                cmd_args[fc_idx] = "-filter_complex_script".to_string();
                                cmd_args[fc_idx + 1] = script_path.to_string_lossy().to_string();
                            }
                        }
                    }
                }
            }

            let _ = tx.send(ExportMessage::Status("FFmpeg başlatılıyor...".to_string()));
            log::info!("Executing export pipeline: {}", cmd_args.join(" "));

            if cmd_args.is_empty() {
                let _ = tx.send(ExportMessage::Finished(Err("Boş komut".to_string())));
                return;
            }

            let mut cmd = trim_media::create_command(&cmd_args[0]);
            cmd.args(&cmd_args[1..])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            let mut child = match ChildGuard::spawn(&mut cmd) {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(ExportMessage::Finished(Err(format!(
                        "FFmpeg başlatılamadı: {}",
                        e
                    ))));
                    return;
                }
            };

            // Concurrently drain stderr to EOF in a 64 KiB ring buffer so FFmpeg never hits SIGPIPE
            // and the final error diagnostics are always preserved.
            let stderr_handle = child.take_stderr().map(|err_pipe| {
                thread::spawn(move || {
                    use std::collections::VecDeque;
                    use std::io::Read;
                    const MAX_STDERR_BYTES: usize = 64 * 1024;
                    let mut ring: VecDeque<u8> = VecDeque::with_capacity(MAX_STDERR_BYTES);
                    let mut reader = err_pipe;
                    let mut chunk = [0u8; 4096];
                    loop {
                        match reader.read(&mut chunk) {
                            Ok(0) | Err(_) => break,
                            Ok(n) => {
                                if n >= MAX_STDERR_BYTES {
                                    ring.clear();
                                    ring.extend(&chunk[n - MAX_STDERR_BYTES..n]);
                                } else {
                                    let excess = (ring.len() + n).saturating_sub(MAX_STDERR_BYTES);
                                    for _ in 0..excess {
                                        ring.pop_front();
                                    }
                                    ring.extend(&chunk[..n]);
                                }
                            }
                        }
                    }
                    let bytes: Vec<u8> = ring.into_iter().collect();
                    String::from_utf8_lossy(&bytes).to_string()
                })
            });

            let stdout_pipe = child.take_stdout();
            let child_arc = Arc::new(std::sync::Mutex::new(Some(child)));
            let cancel_watcher =
                CancelWatcher::spawn(Arc::clone(&child_arc), Arc::clone(&job_cancel));

            let safe_total_duration = if total_duration.is_finite() && total_duration > 0.001 {
                total_duration
            } else {
                0.001
            };

            if let Some(stdout) = stdout_pipe {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    if job_cancel.load(Ordering::SeqCst) {
                        if let Ok(mut guard) = child_arc.lock() {
                            if let Some(ref mut c) = *guard {
                                let _ = c.kill();
                            }
                        }
                        break;
                    }

                    let line_str = line.trim();
                    if let Some(val_str) = line_str.strip_prefix("out_time_us=") {
                        if let Ok(us) = val_str.parse::<f64>() {
                            let cur_sec = (us / 1_000_000.0).max(0.0);
                            let pct =
                                ((cur_sec / safe_total_duration) * 100.0).clamp(0.0, 100.0) as f32;
                            let _ = tx.send(ExportMessage::Progress(pct));
                            let _ = tx.send(ExportMessage::Status(format!(
                                "Dışa Aktarılıyor: %{:.1} ({:.1}s / {:.1}s)",
                                pct, cur_sec, total_duration
                            )));
                        }
                    }
                }
            }

            drop(cancel_watcher);

            let status = if let Ok(mut guard) = child_arc.lock() {
                if let Some(mut c) = guard.take() {
                    c.wait()
                } else {
                    Err(std::io::Error::other("Missing child"))
                }
            } else {
                Err(std::io::Error::other("Child mutex poisoned"))
            };
            drop(_fallback_temp_dir);
            drop(_temp_dir);
            let stderr_str = stderr_handle
                .and_then(|h| h.join().ok())
                .unwrap_or_default();

            if job_cancel.load(Ordering::SeqCst) {
                let p = Path::new(&output_path);
                if p.is_file() {
                    let _ = std::fs::remove_file(p);
                }
                let _ = tx.send(ExportMessage::Finished(Err(
                    "Dışa aktarma iptal edildi".to_string()
                )));
                return;
            }

            match status {
                Ok(s) if s.success() => {
                    let _ = tx.send(ExportMessage::Progress(100.0));
                    let _ = tx.send(ExportMessage::Status(
                        "Dışa aktarma başarıyla tamamlandı!".to_string(),
                    ));
                    let _ = tx.send(ExportMessage::Finished(Ok(output_path)));
                }
                Ok(s) => {
                    let _ = tx.send(ExportMessage::Finished(Err(format!(
                        "FFmpeg hatası (kod {:?}): {}",
                        s.code(),
                        stderr_str
                    ))));
                }
                Err(e) => {
                    let _ = tx.send(ExportMessage::Finished(Err(format!(
                        "FFmpeg bekleme hatası: {}",
                        e
                    ))));
                }
            }
        });
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::process::Command;

    fn proc_exists(pid: u32) -> bool {
        Path::new(&format!("/proc/{pid}")).exists()
    }

    fn spawn_sleeper() -> (ChildGuard, u32) {
        let mut cmd = Command::new("sleep");
        cmd.arg("60");
        let guard = ChildGuard::spawn(&mut cmd).expect("spawn sleep");
        let pid = guard.id().expect("pid");
        (guard, pid)
    }

    #[test]
    fn test_cancel_watcher_kills_child_when_cancel_raised() {
        let (guard, pid) = spawn_sleeper();
        let child = Arc::new(std::sync::Mutex::new(Some(guard)));
        let cancel = Arc::new(AtomicBool::new(false));
        let watcher = CancelWatcher::spawn(Arc::clone(&child), Arc::clone(&cancel));
        assert!(proc_exists(pid));
        cancel.store(true, Ordering::SeqCst);
        // The watcher (still alive) must kill the child on its own, while the guard is still held.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while proc_exists(pid) && std::time::Instant::now() < deadline {
            thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(!proc_exists(pid), "watcher must kill child after cancel");
        drop(watcher);
        drop(child);
    }

    #[test]
    fn test_cancel_watcher_terminates_and_child_dies_when_export_thread_panics() {
        let (guard, pid) = spawn_sleeper();
        let flag_done = Arc::new(AtomicBool::new(false));
        let flag_done_thread = Arc::clone(&flag_done);
        let handle = thread::spawn(move || {
            let child = Arc::new(std::sync::Mutex::new(Some(guard)));
            let cancel = Arc::new(AtomicBool::new(false));
            let _watcher = CancelWatcher::spawn(Arc::clone(&child), cancel);
            let _marker = scopeguard_set(flag_done_thread);
            panic!("simulated export thread panic");
        });
        let res = handle.join();
        assert!(res.is_err(), "export thread must have panicked");
        assert!(
            flag_done.load(Ordering::SeqCst),
            "unwind must have run destructors"
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while proc_exists(pid) && std::time::Instant::now() < deadline {
            thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(
            !proc_exists(pid),
            "child must die after export thread panic"
        );
    }

    struct SetFlag(Arc<AtomicBool>);
    impl Drop for SetFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    fn scopeguard_set(flag: Arc<AtomicBool>) -> SetFlag {
        SetFlag(flag)
    }
}
