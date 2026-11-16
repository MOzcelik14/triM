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
            let (mut cmd_args, total_duration) = match TimelineExporter::build_export_pipeline_range(
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

            // Avoid Windows 32KB CreateProcess CLI limit by writing large filtergraphs to a temp script
            let mut temp_script_path: Option<std::path::PathBuf> = None;
            if let Some(fc_idx) = cmd_args.iter().position(|a| a == "-filter_complex") {
                if let Some(fg) = cmd_args.get(fc_idx + 1) {
                    if fg.len() > 16_000 {
                        let script_path = std::env::temp_dir()
                            .join(format!("trim_fg_{}.txt", uuid::Uuid::new_v4()));
                        if std::fs::write(&script_path, fg).is_ok() {
                            cmd_args[fc_idx] = "-filter_complex_script".to_string();
                            cmd_args[fc_idx + 1] = script_path.to_string_lossy().to_string();
                            temp_script_path = Some(script_path);
                        }
                    }
                }
            }

            let _ = tx.send(ExportMessage::Status("FFmpeg başlatılıyor...".to_string()));
            log::info!("Executing export pipeline: {}", cmd_args.join(" "));

            if cmd_args.is_empty() {
                if let Some(p) = temp_script_path {
                    let _ = std::fs::remove_file(p);
                }
                let _ = tx.send(ExportMessage::Finished(Err("Boş komut".to_string())));
                return;
            }

            let mut cmd = trim_media::create_command(&cmd_args[0]);
            let mut child = match cmd
                .args(&cmd_args[1..])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    if let Some(p) = temp_script_path {
                        let _ = std::fs::remove_file(p);
                    }
                    let _ = tx.send(ExportMessage::Finished(Err(format!(
                        "FFmpeg başlatılamadı: {}",
                        e
                    ))));
                    return;
                }
            };

            // Concurrently drain stderr (capped at 64KB) to prevent pipe deadlocks and unbounded RAM use
            let stderr_handle = child.stderr.take().map(|err_pipe| {
                thread::spawn(move || {
                    use std::io::Read;
                    let mut buf = Vec::new();
                    let _ = err_pipe.take(64 * 1024).read_to_end(&mut buf);
                    String::from_utf8_lossy(&buf).to_string()
                })
            });

            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    if job_cancel.load(Ordering::SeqCst) {
                        let _ = child.kill();
                        break;
                    }

                    let line_str = line.trim();
                    if let Some(val_str) = line_str.strip_prefix("out_time_us=") {
                        if let Ok(us) = val_str.parse::<f64>() {
                            let cur_sec = (us / 1_000_000.0).max(0.0);
                            let pct = ((cur_sec / total_duration) * 100.0).clamp(0.0, 100.0) as f32;
                            let _ = tx.send(ExportMessage::Progress(pct));
                            let _ = tx.send(ExportMessage::Status(format!(
                                "Dışa Aktarılıyor: %{:.1} ({:.1}s / {:.1}s)",
                                pct, cur_sec, total_duration
                            )));
                        }
                    }
                }
            }

            let status = child.wait();
            if let Some(p) = temp_script_path {
                let _ = std::fs::remove_file(p);
            }
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
