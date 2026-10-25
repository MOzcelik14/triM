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
pub fn save_project<P: AsRef<Path>>(
    project: &mut Project,
    target_path: P,
) -> Result<(), ProjectError> {
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

    // Normalize media and LUT paths across Unix/Windows separators and resolve relative paths
    if let Some(parent) = p.parent() {
        for item in project.media_pool.values_mut() {
            let norm_item = trim_core::paths::normalize_path(&item.file_path);
            if norm_item.is_relative() {
                let abs_path = parent.join(&norm_item);
                if abs_path.exists() {
                    item.file_path = abs_path.to_string_lossy().to_string();
                } else {
                    item.file_path = norm_item.to_string_lossy().to_string();
                }
            } else {
                item.file_path = norm_item.to_string_lossy().to_string();
            }
            if let Some(proxy) = &mut item.proxy_path {
                let norm_proxy = trim_core::paths::normalize_path(&*proxy);
                *proxy = norm_proxy.to_string_lossy().to_string();
            }
        }
        for track in &mut project.timeline.tracks {
            for clip in &mut track.clips {
                if let Some(lut) = &mut clip.custom_lut_path {
                    let norm_lut = trim_core::paths::normalize_path(&*lut);
                    if norm_lut.is_relative() {
                        let abs_lut = parent.join(&norm_lut);
                        if abs_lut.exists() {
                            *lut = abs_lut.to_string_lossy().to_string();
                            continue;
                        }
                    }
                    *lut = norm_lut.to_string_lossy().to_string();
                }
            }
        }
    }

    log::info!("Loaded project: {:?} (format: {})", p, project.format);
    Ok(project)
}

/// Bundles a project and all its referenced media files into a self-contained portable directory.
/// Copies media files into `<target_dir>/media/` and rewrites paths in `<target_dir>/<name>.trim` to relative paths.
/// Returns `(media_count, total_bytes)`.
pub fn bundle_project<P: AsRef<Path>>(
    project: &Project,
    target_dir: P,
) -> Result<(usize, u64), ProjectError> {
    let dest_dir = target_dir.as_ref();
    fs::create_dir_all(dest_dir)?;
    let media_dir = dest_dir.join("media");
    fs::create_dir_all(&media_dir)?;

    let mut bundled_project = project.clone();
    let mut count = 0;
    let mut total_bytes = 0u64;

    for (id, item) in &mut bundled_project.media_pool {
        let src_path = Path::new(&item.file_path);
        if src_path.exists() {
            let file_name = src_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("media_{}", id));
            let dest_file = media_dir.join(&file_name);
            let bytes = fs::copy(src_path, &dest_file)?;
            total_bytes += bytes;
            count += 1;
            // Relative path for standalone portability
            item.file_path = format!("media/{}", file_name);
        }
    }

    let proj_file = dest_dir.join(format!("{}.trim", project.settings.name));
    bundled_project.file_path = Some(proj_file.to_string_lossy().to_string());
    bundled_project.is_dirty = false;
    let json = bundled_project.to_json()?;
    fs::write(&proj_file, json)?;

    log::info!(
        "Project successfully bundled to {:?}: {} media files, {} bytes",
        dest_dir,
        count,
        total_bytes
    );
    Ok((count, total_bytes))
}

/// Autosave and recovery snapshot manager.
#[derive(Debug, Clone)]
pub struct AutosaveManager {
    pub recovery_dir: PathBuf,
}

impl Default for AutosaveManager {
    fn default() -> Self {
        let rec_dir = trim_core::paths::config_dir().join("recovery");
        Self {
            recovery_dir: rec_dir,
        }
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
            self.recovery_dir
                .join(format!("{}_{}_recovery.trim", stem, hash))
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
            if legacy_path.is_file() && legacy_path.metadata().map(|m| m.len() > 0).unwrap_or(false)
            {
                return Some(legacy_path);
            }
        }

        // Also check legacy .config/cutline/recovery
        let leg_path = trim_core::paths::home_dir()
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
        assert_eq!(
            proj.file_path,
            Some(file_path.to_string_lossy().to_string())
        );

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

        assert_ne!(
            path1, path2,
            "Recovery paths must not collide for identical stems in different dirs"
        );

        // Test legacy fallback
        let legacy_file = mgr.get_legacy_recovery_path(&proj1).unwrap();
        fs::write(&legacy_file, "legacy recovery data").unwrap();
        assert_eq!(mgr.check_recovery(&proj1), Some(legacy_file.clone()));

        mgr.cleanup(&proj1);
        assert!(!legacy_file.exists());
    }

    #[test]
    fn test_bundle_project_portable_roundtrip() {
        use trim_core::media::MediaItem;

        let src_dir = tempdir().unwrap();
        let fake_video = src_dir.path().join("sample_clip.mp4");
        fs::write(&fake_video, b"fake video bytes").unwrap();

        let mut proj = Project::new("MyAwesomeMovie");
        let item = MediaItem::new(&fake_video);
        let m_id = item.id.clone();
        proj.add_media(item);
        proj.timeline.tracks[0].add_clip(Clip::new(&m_id, 0.0, 5.0));

        let dest_bundle_dir = tempdir().unwrap();
        let target_bundle = dest_bundle_dir.path().join("BundleOutput");

        let (count, bytes) = bundle_project(&proj, &target_bundle).unwrap();
        assert_eq!(count, 1);
        assert_eq!(bytes, 16);

        let bundled_trim_file = target_bundle.join("MyAwesomeMovie.trim");
        assert!(bundled_trim_file.is_file());
        assert!(target_bundle
            .join("media")
            .join("sample_clip.mp4")
            .is_file());

        // Now move bundle to a different directory to test portability
        let new_location = tempdir().unwrap();
        let final_dest = new_location.path().join("MovedBundle");
        fs::create_dir_all(&final_dest).unwrap();
        fs::rename(&target_bundle, &final_dest).unwrap();

        let moved_trim_file = final_dest.join("MyAwesomeMovie.trim");
        let loaded = load_project(&moved_trim_file).unwrap();

        let loaded_media = loaded.get_media(&m_id).unwrap();
        let loaded_media_path = Path::new(&loaded_media.file_path);
        assert!(
            loaded_media_path.is_file(),
            "Bundled media should exist at resolved path: {:?}",
            loaded_media_path
        );
        assert_eq!(fs::read(loaded_media_path).unwrap(), b"fake video bytes");
    }

    #[test]
    fn test_generate_synthetic_showcase_files() {
        use trim_core::clip::{ColorLabel, ColorWheelsConfig, LutPreset};
        use trim_core::marker::Marker;
        use trim_core::media::{MediaItem, MediaType};
        use trim_core::timeline::AspectRatio;

        let mut proj = Project::new("triM Sinematik Kurgu");
        let m1 = MediaItem {
            id: "m_cam_a".into(),
            file_path: "/tmp/trim_test_media/camera_a.mp4".into(),
            name: "Kamera_A_Roportaj.mp4".into(),
            media_type: MediaType::Video,
            duration: 12.0,
            width: 1920,
            height: 1080,
            fps: 30.0,
            video_codec: "h264".into(),
            audio_codec: "aac".into(),
            audio_channels: 2,
            audio_sample_rate: 48000,
            file_size_bytes: 500_000,
            date_added: 0.0,
            proxy_path: None,
            bin_id: None,
        };
        let m2 = MediaItem {
            id: "m_cam_b".into(),
            file_path: "/tmp/trim_test_media/camera_b.mp4".into(),
            name: "B-Roll_Aksiyon.mp4".into(),
            media_type: MediaType::Video,
            duration: 10.0,
            width: 1920,
            height: 1080,
            fps: 30.0,
            video_codec: "h264".into(),
            audio_codec: "aac".into(),
            audio_channels: 2,
            audio_sample_rate: 48000,
            file_size_bytes: 400_000,
            date_added: 0.0,
            proxy_path: None,
            bin_id: None,
        };
        let m3 = MediaItem {
            id: "m_audio".into(),
            file_path: "/tmp/trim_test_media/soundtrack.wav".into(),
            name: "Film_Muzigi.wav".into(),
            media_type: MediaType::Audio,
            duration: 20.0,
            width: 0,
            height: 0,
            fps: 0.0,
            video_codec: "".into(),
            audio_codec: "pcm_s16le".into(),
            audio_channels: 2,
            audio_sample_rate: 48000,
            file_size_bytes: 1_000_000,
            date_added: 0.0,
            proxy_path: None,
            bin_id: None,
        };

        proj.media_pool.insert(m1.id.clone(), m1);
        proj.media_pool.insert(m2.id.clone(), m2);
        proj.media_pool.insert(m3.id.clone(), m3);

        // V1 clip
        let mut clip_v1 = Clip::new("m_cam_a", 0.0, 7.5);
        clip_v1.name = "Kamera_A_Roportaj.mp4".into();
        clip_v1.lut_preset = LutPreset::TealOrange;
        clip_v1.color_wheels = Some(ColorWheelsConfig {
            lift: [0.03, -0.01, -0.05],
            gamma: [1.06, 1.0, 0.94],
            gain: [1.0, 1.03, 0.96],
            luminance: [0.0, 0.0, 0.0],
        });
        clip_v1.color_label = ColorLabel::Green;
        proj.timeline.tracks[1].add_clip(clip_v1);

        // V2 clip (PIP)
        let mut clip_v2 = Clip::new("m_cam_b", 3.0, 6.8);
        clip_v2.name = "B-Roll_Aksiyon.mp4".into();
        clip_v2.scale = 0.42;
        clip_v2.pos_x = 0.72;
        clip_v2.pos_y = 0.28;
        clip_v2.fade_in = 0.4;
        clip_v2.fade_out = 0.4;
        clip_v2.color_label = ColorLabel::Blue;
        proj.timeline.tracks[0].add_clip(clip_v2);

        // A1 clip (dialogue)
        let mut clip_a1 = Clip::new("m_cam_a", 0.0, 7.5);
        clip_a1.name = "Vokal_Roportaj.wav".into();
        clip_a1.fade_in = 0.2;
        clip_a1.fade_out = 0.3;
        clip_a1.color_label = ColorLabel::Yellow;
        proj.timeline.tracks[2].add_clip(clip_a1);

        // A2 clip (music)
        let mut clip_a2 = Clip::new("m_audio", 0.0, 10.0);
        clip_a2.name = "Film_Muzigi.wav".into();
        clip_a2.volume = 0.4;
        clip_a2.fade_in = 1.0;
        clip_a2.fade_out = 1.2;
        clip_a2.color_label = ColorLabel::Purple;
        proj.timeline.tracks[3].add_clip(clip_a2);

        proj.timeline.markers.push(Marker {
            id: "m1".into(),
            time: 3.0,
            name: "B-Roll Giris".into(),
            color: "#E07A38".into(),
        });

        save_project(&mut proj, "/tmp/trim_showcase.trim").unwrap();

        // Vertical project
        let mut proj_vert = Project::new("triM Dikey Video");
        proj_vert
            .timeline
            .set_aspect_ratio(AspectRatio::Vertical9x16);
        proj_vert.settings.width = 1080;
        proj_vert.settings.height = 1920;
        let m_v = MediaItem {
            id: "m_reel".into(),
            file_path: "/tmp/trim_test_media/reel_clip.mp4".into(),
            name: "Reels_Story.mp4".into(),
            media_type: MediaType::Video,
            duration: 10.0,
            width: 1080,
            height: 1920,
            fps: 30.0,
            video_codec: "h264".into(),
            audio_codec: "aac".into(),
            audio_channels: 2,
            audio_sample_rate: 48000,
            file_size_bytes: 400_000,
            date_added: 0.0,
            proxy_path: None,
            bin_id: None,
        };
        proj_vert.media_pool.insert(m_v.id.clone(), m_v);
        let mut clip_reel = Clip::new("m_reel", 0.0, 8.0);
        clip_reel.name = "Reels_Story.mp4".into();
        clip_reel.color_label = ColorLabel::Magenta;
        proj_vert.timeline.tracks[1].add_clip(clip_reel);

        save_project(&mut proj_vert, "/tmp/trim_vertical.trim").unwrap();
    }
}
