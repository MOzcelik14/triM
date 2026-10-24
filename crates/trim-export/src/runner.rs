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
}

impl Default for ExportRunner {
    fn default() -> Self {
        Self {
            is_cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl ExportRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
    }

    pub fn start_export(
        &self,
        project: Project,
        preset: ExportPreset,
        output_path: String,
        tx: Sender<ExportMessage>,
    ) {
        let is_cancelled = Arc::clone(&self.is_cancelled);
        is_cancelled.store(false, Ordering::SeqCst);

        thread::spawn(move || {
            let (cmd_args, total_duration) =
                match TimelineExporter::build_export_pipeline(&project, &preset, &output_path) {
                    Ok(res) => res,
                    Err(err) => {
                        let _ = tx.send(ExportMessage::Finished(Err(err)));
                        return;
                    }
                };

            let _ = tx.send(ExportMessage::Status("FFmpeg başlatılıyor...".to_string()));
            log::info!("Executing export pipeline: {}", cmd_args.join(" "));

            if cmd_args.is_empty() {
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
                    let _ = tx.send(ExportMessage::Finished(Err(format!(
                        "FFmpeg başlatılamadı: {}",
                        e
                    ))));
                    return;
                }
            };

            // Concurrently drain stderr to prevent pipe buffer deadlocks if FFmpeg writes excessive logs
            let stderr_handle = child.stderr.take().map(|mut err_pipe| {
                thread::spawn(move || {
                    let mut s = String::new();
                    use std::io::Read;
                    let _ = err_pipe.read_to_string(&mut s);
                    s
                })
            });

            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    if is_cancelled.load(Ordering::SeqCst) {
                        let _ = child.kill();
                        break;
                    }

                    let line_str = line.trim();
                    if let Some(val_str) = line_str.strip_prefix("out_time_us=") {
                        if let Ok(us) = val_str.parse::<f64>() {
                            let cur_sec = us / 1_000_000.0;
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
            let stderr_str = stderr_handle
                .and_then(|h| h.join().ok())
                .unwrap_or_default();

            if is_cancelled.load(Ordering::SeqCst) {
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
