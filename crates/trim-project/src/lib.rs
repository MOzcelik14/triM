//! Project serialization, file I/O, autosave and crash recovery.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;
use trim_core::project::Project;

#[derive(Error, Debug)]
pub enum ProjectError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid project file: {0}")]
    InvalidFormat(String),
}

/// Saves project to disk using atomic rename (.tmp -> dest).
pub fn save_project<P: AsRef<Path>>(project: &mut Project, target_path: P) -> Result<(), ProjectError> {
    let target = target_path.as_ref();
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = target.with_extension("tmp");
    let json_data = project.to_json()?;

    {
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(json_data.as_bytes())?;
        file.sync_all()?;
    }

    fs::rename(&tmp_path, target)?;

    project.file_path = Some(target.to_string_lossy().to_string());
    project.is_dirty = false;
    log::info!("Project saved successfully to {:?}", target);
    Ok(())
}

/// Loads project from disk (.trim or .cutline).
pub fn load_project<P: AsRef<Path>>(path: P) -> Result<Project, ProjectError> {
    let p = path.as_ref();
    if !p.is_file() {
        return Err(ProjectError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Project file not found: {:?}", p),
        )));
    }

    let contents = fs::read_to_string(p)?;
    let mut project = Project::from_json(&contents)?;
    project.file_path = Some(p.to_string_lossy().to_string());
    project.is_dirty = false;
    log::info!("Loaded project: {:?} (format: {})", p, project.format);
    Ok(project)
}

/// Autosave and recovery snapshot manager.
#[derive(Debug, Clone)]
pub struct AutosaveManager {
    pub recovery_dir: PathBuf,
}

impl Default for AutosaveManager {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let rec_dir = PathBuf::from(home).join(".config").join("trim").join("recovery");
        Self { recovery_dir: rec_dir }
    }
}

fn hash_path(path_str: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    path_str.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

impl AutosaveManager {
    pub fn new<P: Into<PathBuf>>(recovery_dir: P) -> Self {
        Self {
            recovery_dir: recovery_dir.into(),
        }
    }

    pub fn get_recovery_path(&self, project: &Project) -> PathBuf {
        if let Some(fp) = &project.file_path {
            let stem = Path::new(fp)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("project");
            let hash = hash_path(fp);
            self.recovery_dir.join(format!("{}_{}_recovery.trim", stem, hash))
        } else {
            self.recovery_dir.join("unsaved_project_recovery.trim")
        }
    }

    pub fn get_legacy_recovery_path(&self, project: &Project) -> Option<PathBuf> {
        project.file_path.as_ref().map(|fp| {
            let stem = Path::new(fp)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("project");
            self.recovery_dir.join(format!("{}_recovery.trim", stem))
        })
    }

    pub fn check_recovery(&self, project: &Project) -> Option<PathBuf> {
        let rec_path = self.get_recovery_path(project);
        if rec_path.is_file() && rec_path.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            return Some(rec_path);
        }

        // Check legacy stem-only recovery path
        if let Some(legacy_path) = self.get_legacy_recovery_path(project) {
            if legacy_path.is_file() && legacy_path.metadata().map(|m| m.len() > 0).unwrap_or(false) {
                return Some(legacy_path);
            }
        }

        // Also check legacy .config/cutline/recovery
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let leg_path = PathBuf::from(home)
            .join(".config")
            .join("cutline")
            .join("recovery")
            .join("unsaved_project_recovery.cutline");
        if leg_path.is_file() && leg_path.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            return Some(leg_path);
        }

        None
    }

    pub fn save_snapshot(&self, project: &Project) -> Result<(), ProjectError> {
        if !project.is_dirty {
            return Ok(());
        }

        fs::create_dir_all(&self.recovery_dir)?;
        let rec_path = self.get_recovery_path(project);
        let tmp_path = rec_path.with_extension("tmp");

        let json_data = project.to_json()?;
        {
            let mut file = fs::File::create(&tmp_path)?;
            file.write_all(json_data.as_bytes())?;
            file.sync_all()?;
        }
        fs::rename(&tmp_path, &rec_path)?;
        log::debug!("Autosave snapshot written to {:?}", rec_path);
        Ok(())
    }

    pub fn cleanup(&self, project: &Project) {
        let rec_path = self.get_recovery_path(project);
        if rec_path.is_file() {
            let _ = fs::remove_file(rec_path);
        }
        if let Some(legacy_path) = self.get_legacy_recovery_path(project) {
            if legacy_path.is_file() {
                let _ = fs::remove_file(legacy_path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use trim_core::clip::Clip;

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("my_project.trim");

        let mut proj = Project::new("Test Save");
        proj.timeline.tracks[0].add_clip(Clip::new("m1", 0.0, 5.0));

        save_project(&mut proj, &file_path).unwrap();
        assert!(file_path.is_file());
        assert!(!proj.is_dirty);
        assert_eq!(proj.file_path, Some(file_path.to_string_lossy().to_string()));

        let loaded = load_project(&file_path).unwrap();
        assert_eq!(loaded.settings.name, "Test Save");
        assert_eq!(loaded.timeline.tracks[0].clips.len(), 1);
        assert_eq!(loaded.timeline.tracks[0].clips[0].duration(), 5.0);
    }

    #[test]
    fn test_autosave_lifecycle() {
        let dir = tempdir().unwrap();
        let mgr = AutosaveManager::new(dir.path().to_path_buf());

        let mut proj = Project::new("Autosave Test");
        assert!(mgr.check_recovery(&proj).is_none());

        proj.mark_dirty();
        mgr.save_snapshot(&proj).unwrap();
        assert!(mgr.check_recovery(&proj).is_some());

        mgr.cleanup(&proj);
        assert!(mgr.check_recovery(&proj).is_none());
    }

    #[test]
    fn test_recovery_path_collision_prevention() {
        let dir = tempdir().unwrap();
        let mgr = AutosaveManager::new(dir.path().to_path_buf());

        let mut proj1 = Project::new("Same Name");
        proj1.file_path = Some("/home/user/folder1/project.trim".into());

        let mut proj2 = Project::new("Same Name");
        proj2.file_path = Some("/home/user/folder2/project.trim".into());

        let path1 = mgr.get_recovery_path(&proj1);
        let path2 = mgr.get_recovery_path(&proj2);

        assert_ne!(path1, path2, "Recovery paths must not collide for identical stems in different dirs");

        // Test legacy fallback
        let legacy_file = mgr.get_legacy_recovery_path(&proj1).unwrap();
        fs::write(&legacy_file, "legacy recovery data").unwrap();
        assert_eq!(mgr.check_recovery(&proj1), Some(legacy_file.clone()));

        mgr.cleanup(&proj1);
        assert!(!legacy_file.exists());
    }
}
