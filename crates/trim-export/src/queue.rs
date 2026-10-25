//! Sequential background export queue management.

use crate::presets::ExportPreset;
use crate::runner::{ExportMessage, ExportRunner};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{channel, Receiver};
use std::time::SystemTime;
use trim_core::project::Project;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderJobStatus {
    Queued,
    InProgress { progress: f32, status_text: String },
    Completed { output_path: String },
    Failed { error: String },
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderJob {
    pub id: String,
    pub name: String,
    pub project: Project,
    pub preset: ExportPreset,
    pub output_path: String,
    pub range: Option<(f64, f64)>,
    pub status: RenderJobStatus,
    pub created_at_sec: u64,
}

impl RenderJob {
    pub fn new(
        name: impl Into<String>,
        project: Project,
        preset: ExportPreset,
        output_path: impl Into<String>,
        range: Option<(f64, f64)>,
    ) -> Self {
        let created_at_sec = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            project,
            preset,
            output_path: output_path.into(),
            range,
            status: RenderJobStatus::Queued,
            created_at_sec,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RenderQueueEvent {
    JobProgress {
        job_id: String,
        progress: f32,
        status_text: String,
    },
    JobCompleted {
        job_id: String,
        output_path: String,
    },
    JobFailed {
        job_id: String,
        error: String,
    },
    QueueFinished,
}

pub struct RenderQueue {
    pub jobs: Vec<RenderJob>,
    pub is_active: bool,
    pub current_job_id: Option<String>,
    active_runner: Option<ExportRunner>,
    rx: Option<Receiver<ExportMessage>>,
}

impl Default for RenderQueue {
    fn default() -> Self {
        Self {
            jobs: Vec::new(),
            is_active: true,
            current_job_id: None,
            active_runner: None,
            rx: None,
        }
    }
}

impl RenderQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a new job to the queue. If queue is active and idle, starts it.
    pub fn add_job(&mut self, job: RenderJob) -> String {
        let id = job.id.clone();
        self.jobs.push(job);
        if self.is_active && self.current_job_id.is_none() {
            self.start_next_job();
        }
        id
    }

    /// Removes a job by ID. If it's currently running, cancels it first.
    pub fn remove_job(&mut self, job_id: &str) {
        if self.current_job_id.as_deref() == Some(job_id) {
            self.cancel_current();
        }
        self.jobs.retain(|j| j.id != job_id);
    }

    /// Removes all completed, failed, or cancelled jobs.
    pub fn clear_finished(&mut self) {
        self.jobs.retain(|j| {
            matches!(
                j.status,
                RenderJobStatus::Queued | RenderJobStatus::InProgress { .. }
            )
        });
    }

    /// Starts or resumes sequential queue processing.
    pub fn start(&mut self) {
        self.is_active = true;
        if self.current_job_id.is_none() {
            self.start_next_job();
        }
    }

    /// Pauses the queue. Does not kill the active job, but prevents next jobs from starting.
    pub fn pause(&mut self) {
        self.is_active = false;
    }

    /// Cancels currently rendering job.
    pub fn cancel_current(&mut self) {
        if let Some(runner) = self.active_runner.take() {
            runner.cancel();
        }
        if let Some(cur_id) = self.current_job_id.take() {
            if let Some(job) = self.jobs.iter_mut().find(|j| j.id == cur_id) {
                job.status = RenderJobStatus::Cancelled;
            }
        }
        self.rx = None;
    }

    /// Cancels a specific job by ID.
    pub fn cancel_job(&mut self, job_id: &str) {
        if self.current_job_id.as_deref() == Some(job_id) {
            self.cancel_current();
        } else if let Some(job) = self.jobs.iter_mut().find(|j| j.id == job_id) {
            if matches!(job.status, RenderJobStatus::Queued) {
                job.status = RenderJobStatus::Cancelled;
            }
        }
    }

    /// Internal method to pick the next queued job and launch it in background.
    fn start_next_job(&mut self) -> bool {
        let next_idx = self
            .jobs
            .iter()
            .position(|j| matches!(j.status, RenderJobStatus::Queued));
        let Some(idx) = next_idx else {
            self.current_job_id = None;
            self.active_runner = None;
            self.rx = None;
            return false;
        };

        let job = &mut self.jobs[idx];
        let runner = ExportRunner::new();
        let (tx, rx) = channel();

        runner.start_export(
            job.project.clone(),
            job.preset.clone(),
            job.output_path.clone(),
            job.range,
            tx,
        );

        job.status = RenderJobStatus::InProgress {
            progress: 0.0,
            status_text: "Başlatılıyor...".to_string(),
        };

        self.current_job_id = Some(job.id.clone());
        self.active_runner = Some(runner);
        self.rx = Some(rx);
        true
    }

    /// Polls running export task messages and progresses the queue.
    pub fn poll(&mut self) -> Vec<RenderQueueEvent> {
        let mut events = Vec::new();
        let mut finished_result = None;

        if let Some(ref rx) = self.rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ExportMessage::Progress(pct) => {
                        if let Some(ref cur_id) = self.current_job_id {
                            if let Some(job) = self.jobs.iter_mut().find(|j| &j.id == cur_id) {
                                if let RenderJobStatus::InProgress {
                                    ref mut progress, ..
                                } = job.status
                                {
                                    *progress = pct;
                                }
                            }
                            events.push(RenderQueueEvent::JobProgress {
                                job_id: cur_id.clone(),
                                progress: pct,
                                status_text: String::new(),
                            });
                        }
                    }
                    ExportMessage::Status(st) => {
                        if let Some(ref cur_id) = self.current_job_id {
                            if let Some(job) = self.jobs.iter_mut().find(|j| &j.id == cur_id) {
                                if let RenderJobStatus::InProgress {
                                    ref mut status_text,
                                    ..
                                } = job.status
                                {
                                    *status_text = st.clone();
                                }
                            }
                            events.push(RenderQueueEvent::JobProgress {
                                job_id: cur_id.clone(),
                                progress: 0.0,
                                status_text: st,
                            });
                        }
                    }
                    ExportMessage::Finished(res) => {
                        finished_result = Some(res);
                        break;
                    }
                }
            }
        }

        if let Some(res) = finished_result {
            self.active_runner = None;
            self.rx = None;

            if let Some(cur_id) = self.current_job_id.take() {
                match res {
                    Ok(out_p) => {
                        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == cur_id) {
                            job.status = RenderJobStatus::Completed {
                                output_path: out_p.clone(),
                            };
                        }
                        events.push(RenderQueueEvent::JobCompleted {
                            job_id: cur_id,
                            output_path: out_p,
                        });
                    }
                    Err(err) => {
                        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == cur_id) {
                            if !matches!(job.status, RenderJobStatus::Cancelled) {
                                job.status = RenderJobStatus::Failed { error: err.clone() };
                            }
                        }
                        events.push(RenderQueueEvent::JobFailed {
                            job_id: cur_id,
                            error: err,
                        });
                    }
                }
            }

            if self.is_active && !self.start_next_job() {
                events.push(RenderQueueEvent::QueueFinished);
            }
        }

        events
    }

    /// Returns counts: (queued, in_progress, completed, failed)
    pub fn summary_counts(&self) -> (usize, usize, usize, usize) {
        let mut queued = 0;
        let mut in_prog = 0;
        let mut completed = 0;
        let mut failed = 0;

        for j in &self.jobs {
            match j.status {
                RenderJobStatus::Queued => queued += 1,
                RenderJobStatus::InProgress { .. } => in_prog += 1,
                RenderJobStatus::Completed { .. } => completed += 1,
                RenderJobStatus::Failed { .. } | RenderJobStatus::Cancelled => failed += 1,
            }
        }

        (queued, in_prog, completed, failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trim_core::project::Project;

    #[test]
    fn test_render_queue_lifecycle() {
        let mut queue = RenderQueue::new();
        assert_eq!(queue.jobs.len(), 0);
        assert!(queue.is_active);

        let p1 = Project::default();
        let preset = ExportPreset::default();

        let job1 = RenderJob::new("Job 1", p1.clone(), preset.clone(), "/tmp/out1.mp4", None);
        let id1 = job1.id.clone();
        queue.add_job(job1);

        assert_eq!(queue.jobs.len(), 1);
        assert_eq!(queue.current_job_id.as_deref(), Some(id1.as_str()));

        // Cancel job1
        queue.cancel_current();
        assert_eq!(queue.jobs[0].status, RenderJobStatus::Cancelled);
        assert!(queue.current_job_id.is_none());

        // Add second job
        let job2 = RenderJob::new("Job 2", p1, preset, "/tmp/out2.mp4", Some((0.0, 5.0)));
        let id2 = job2.id.clone();
        queue.add_job(job2);

        assert_eq!(queue.jobs.len(), 2);
        assert_eq!(queue.current_job_id.as_deref(), Some(id2.as_str()));

        // Remove job1
        queue.remove_job(&id1);
        assert_eq!(queue.jobs.len(), 1);
        assert_eq!(queue.jobs[0].id, id2);
    }
}
