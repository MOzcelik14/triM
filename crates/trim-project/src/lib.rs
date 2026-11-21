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

const MAX_PROJECT_FILE_BYTES: u64 = 64 * 1024 * 1024;

fn write_atomic(target: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(target).map_err(|e| e.error)?;
    Ok(())
}

/// Saves project to disk using atomic temp-file + sync_all + persist.
pub fn save_project<P: AsRef<Path>>(
    project: &mut Project,
    target_path: P,
) -> Result<(), ProjectError> {
    let target = target_path.as_ref();
    let json_data = project.to_json()?;
    write_atomic(target, json_data.as_bytes())?;

    project.file_path = Some(target.to_string_lossy().to_string());
    project.is_dirty = false;
    log::info!("Project saved successfully to {:?}", target);
    Ok(())
}

fn resolve_safe_media_path(raw: &str, project_parent: &Path) -> String {
    let norm = trim_core::paths::normalize_path(raw);
    if !norm.is_relative() {
        return norm.to_string_lossy().to_string();
    }
    if norm
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        log::warn!("Rejected path traversal in project reference: {:?}", raw);
        return String::new();
    }
    let abs_path = project_parent.join(&norm);
    if let Ok(meta) = fs::symlink_metadata(&abs_path) {
        if !meta.file_type().is_symlink() && meta.is_file() {
            return abs_path.to_string_lossy().to_string();
        }
    }
    norm.to_string_lossy().to_string()
}

/// Loads project from disk (.trim or .cutline).
pub fn load_project<P: AsRef<Path>>(path: P) -> Result<Project, ProjectError> {
    use std::io::Read;

    let p = path.as_ref();
    let meta = fs::symlink_metadata(p).map_err(|_| {
        ProjectError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Project file not found: {:?}", p),
        ))
    })?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(ProjectError::InvalidFormat(format!(
            "Project path is not a regular file: {:?}",
            p
        )));
    }
    if meta.len() > MAX_PROJECT_FILE_BYTES {
        return Err(ProjectError::InvalidFormat(format!(
            "Project file exceeds {} MiB safety limit",
            MAX_PROJECT_FILE_BYTES / (1024 * 1024)
        )));
    }

    let file = fs::File::open(p)?;
    let mut contents = String::new();
    file.take(MAX_PROJECT_FILE_BYTES + 1)
        .read_to_string(&mut contents)?;
    if contents.len() as u64 > MAX_PROJECT_FILE_BYTES {
        return Err(ProjectError::InvalidFormat(format!(
            "Project file exceeds {} MiB safety limit",
            MAX_PROJECT_FILE_BYTES / (1024 * 1024)
        )));
    }

    let mut project = Project::from_json(&contents)?;
    if project.format != "trim" && project.format != "cutline" {
        return Err(ProjectError::InvalidFormat(format!(
            "Unsupported project format: {:?}",
            project.format
        )));
    }
    if project.version == 0 || project.version > Project::CURRENT_SCHEMA_VERSION {
        return Err(ProjectError::InvalidFormat(format!(
            "Unsupported project schema version: {} (max supported: {})",
            project.version,
            Project::CURRENT_SCHEMA_VERSION
        )));
    }

    project.file_path = Some(p.to_string_lossy().to_string());
    project.is_dirty = false;

    // Normalize media and LUT paths across Unix/Windows separators and resolve relative paths
    if let Some(parent) = p.parent().filter(|par| !par.as_os_str().is_empty()) {
        for item in project.media_pool.values_mut() {
            item.file_path = resolve_safe_media_path(&item.file_path, parent);
            if let Some(proxy) = &mut item.proxy_path {
                *proxy = resolve_safe_media_path(proxy, parent);
            }
        }
        for track in &mut project.timeline.tracks {
            for clip in &mut track.clips {
                if let Some(lut) = &mut clip.custom_lut_path {
                    *lut = resolve_safe_media_path(lut, parent);
                }
            }
        }
        for seq in &mut project.sequences {
            for track in &mut seq.timeline.tracks {
                for clip in &mut track.clips {
                    if let Some(lut) = &mut clip.custom_lut_path {
                        *lut = resolve_safe_media_path(lut, parent);
                    }
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
    bundled_project.sync_active_sequence_to_list();
    let mut count = 0;
    let mut total_bytes = 0u64;

    let mut copied_sources: std::collections::HashMap<PathBuf, String> =
        std::collections::HashMap::new();
    let mut used_dest_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    fn bundle_copy_file(
        src_path: &Path,
        fallback_stem: &str,
        media_dir: &Path,
        copied_sources: &mut std::collections::HashMap<PathBuf, String>,
        used_dest_names: &mut std::collections::HashSet<String>,
        count: &mut usize,
        total_bytes: &mut u64,
    ) -> std::io::Result<Option<String>> {
        let Ok(meta) = fs::symlink_metadata(src_path) else {
            return Ok(None);
        };
        if meta.file_type().is_symlink() || !meta.is_file() {
            log::warn!(
                "Skipping non-regular or symlinked media file during bundle: {:?}",
                src_path
            );
            return Ok(None);
        }
        let canon_path = fs::canonicalize(src_path).unwrap_or_else(|_| src_path.to_path_buf());
        if let Some(existing_rel) = copied_sources.get(&canon_path) {
            return Ok(Some(existing_rel.clone()));
        }

        let raw_name = src_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| fallback_stem.to_string());
        let orig_name: String = raw_name
            .chars()
            .map(|c| {
                if matches!(c, '/' | '\\' | '\0') {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        let orig_name = if orig_name.is_empty() || orig_name == "." || orig_name == ".." {
            fallback_stem.to_string()
        } else {
            orig_name
        };

        let chosen_name = if !used_dest_names.contains(&orig_name) {
            orig_name
        } else {
            let stem = src_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| fallback_stem.to_string());
            let ext = src_path
                .extension()
                .map(|s| format!(".{}", s.to_string_lossy()))
                .unwrap_or_default();

            let mut counter = 2usize;
            loop {
                let candidate = format!("{}_{}{}", stem, counter, ext);
                if !used_dest_names.contains(&candidate) {
                    break candidate;
                }
                counter += 1;
            }
        };

        let dest_file = media_dir.join(&chosen_name);
        let mut src_file = fs::File::open(src_path)?;
        let mut dst_file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&dest_file)?;
        let bytes = std::io::copy(&mut src_file, &mut dst_file)?;
        dst_file.sync_all()?;
        *total_bytes += bytes;
        *count += 1;

        let rel_path = format!("media/{}", chosen_name);
        used_dest_names.insert(chosen_name);
        copied_sources.insert(canon_path, rel_path.clone());
        Ok(Some(rel_path))
    }

    for (id, item) in &mut bundled_project.media_pool {
        if let Some(rel) = bundle_copy_file(
            Path::new(&item.file_path),
            &format!("media_{id}"),
            &media_dir,
            &mut copied_sources,
            &mut used_dest_names,
            &mut count,
            &mut total_bytes,
        )? {
            item.file_path = rel;
        }
        if let Some(proxy_str) = item.proxy_path.clone() {
            if let Some(rel) = bundle_copy_file(
                Path::new(&proxy_str),
                &format!("proxy_{id}"),
                &media_dir,
                &mut copied_sources,
                &mut used_dest_names,
                &mut count,
                &mut total_bytes,
            )? {
                item.proxy_path = Some(rel);
            }
        }
    }

    for tr in &mut bundled_project.timeline.tracks {
        for clip in &mut tr.clips {
            if let Some(lut_str) = clip.custom_lut_path.clone() {
                if let Some(rel) = bundle_copy_file(
                    Path::new(&lut_str),
                    &format!("lut_{}", clip.id),
                    &media_dir,
                    &mut copied_sources,
                    &mut used_dest_names,
                    &mut count,
                    &mut total_bytes,
                )? {
                    clip.custom_lut_path = Some(rel);
                }
            }
        }
    }
    for seq in &mut bundled_project.sequences {
        for tr in &mut seq.timeline.tracks {
            for clip in &mut tr.clips {
                if let Some(lut_str) = clip.custom_lut_path.clone() {
                    if let Some(rel) = bundle_copy_file(
                        Path::new(&lut_str),
                        &format!("lut_{}", clip.id),
                        &media_dir,
                        &mut copied_sources,
                        &mut used_dest_names,
                        &mut count,
                        &mut total_bytes,
                    )? {
                        clip.custom_lut_path = Some(rel);
                    }
                }
            }
        }
    }

    let safe_proj_name: String = project
        .settings
        .name
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let safe_proj_name = safe_proj_name.trim().trim_matches('.');
    let file_stem = if safe_proj_name.is_empty() {
        "project"
    } else {
        safe_proj_name
    };
    let proj_file = dest_dir.join(format!("{file_stem}.trim"));
    bundled_project.file_path = Some(proj_file.to_string_lossy().to_string());
    bundled_project.is_dirty = false;
    let json = bundled_project.to_json()?;
    write_atomic(&proj_file, json.as_bytes())?;

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

/// Deterministic 64-bit FNV-1a hash so recovery filenames remain stable across Rust compiler versions.
fn hash_path(path_str: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in path_str.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
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

        let rec_path = self.get_recovery_path(project);
        let json_data = project.to_json()?;
        write_atomic(&rec_path, json_data.as_bytes())?;
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
    fn test_bundle_project_same_filename_collision_and_deduplication() {
        use trim_core::media::MediaItem;

        let base_dir = tempdir().unwrap();
        let cam1_dir = base_dir.path().join("camera1");
        let cam2_dir = base_dir.path().join("camera2");
        fs::create_dir_all(&cam1_dir).unwrap();
        fs::create_dir_all(&cam2_dir).unwrap();

        let cam1_file = cam1_dir.join("clip.mp4");
        let cam2_file = cam2_dir.join("clip.mp4");
        fs::write(&cam1_file, b"content-of-cam1").unwrap();
        fs::write(&cam2_file, b"content-of-cam2").unwrap();

        let mut proj = Project::new("MultiCamCollisions");
        let item1 = MediaItem::new(&cam1_file);
        let id1 = item1.id.clone();
        proj.add_media(item1);

        let item2 = MediaItem::new(&cam2_file);
        let id2 = item2.id.clone();
        proj.add_media(item2);

        // Third item references the exact same file as item1 (deduplication check)
        let mut item3 = MediaItem::new(&cam1_file);
        item3.id = "m3_duplicate_cam1".into();
        let id3 = item3.id.clone();
        proj.add_media(item3);

        let bundle_target = base_dir.path().join("BundledProject");
        let (count, total_bytes) = bundle_project(&proj, &bundle_target).unwrap();

        // 2 unique files copied, not 3 (item3 is deduplicated)
        assert_eq!(count, 2, "Should copy exactly 2 unique media files");
        assert_eq!(
            total_bytes,
            (b"content-of-cam1".len() + b"content-of-cam2".len()) as u64
        );

        let loaded = load_project(bundle_target.join("MultiCamCollisions.trim")).unwrap();
        let loaded_m1 = loaded.get_media(&id1).unwrap();
        let loaded_m2 = loaded.get_media(&id2).unwrap();
        let loaded_m3 = loaded.get_media(&id3).unwrap();

        // m1 and m3 must share the same bundled path
        assert_eq!(loaded_m1.file_path, loaded_m3.file_path);
        // m1 and m2 must have DIFFERENT bundled paths
        assert_ne!(loaded_m1.file_path, loaded_m2.file_path);

        // Verify contents are NOT overwritten
        let path1 = Path::new(&loaded_m1.file_path);
        let path2 = Path::new(&loaded_m2.file_path);
        assert_eq!(fs::read(path1).unwrap(), b"content-of-cam1");
        assert_eq!(fs::read(path2).unwrap(), b"content-of-cam2");
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

    #[test]
    fn test_load_project_rejects_traversal_and_invalid_schema() {
        let dir = tempdir().unwrap();
        let proj_path = dir.path().join("evil.trim");

        // 1. Reject unsupported format
        let mut p = Project::new("Evil");
        p.format = "not_trim".into();
        std::fs::write(&proj_path, p.to_json().unwrap()).unwrap();
        assert!(load_project(&proj_path).is_err());

        // 2. Reject future schema version
        let mut p = Project::new("Evil");
        p.version = Project::CURRENT_SCHEMA_VERSION + 99;
        std::fs::write(&proj_path, p.to_json().unwrap()).unwrap();
        assert!(load_project(&proj_path).is_err());

        // 3. Reject `..` path traversal in relative media_pool paths
        let mut p = Project::new("Traversal");
        let mut item = trim_core::media::MediaItem::new("../../etc/passwd");
        item.media_type = trim_core::media::MediaType::Video;
        let id = item.id.clone();
        p.add_media(item);
        std::fs::write(&proj_path, p.to_json().unwrap()).unwrap();
        let loaded = load_project(&proj_path).unwrap();
        assert_eq!(loaded.media_pool.get(&id).unwrap().file_path, "");
    }

    #[cfg(unix)]
    #[test]
    fn test_load_project_rejects_symlink() {
        let dir = tempdir().unwrap();
        let real_path = dir.path().join("real.trim");
        let link_path = dir.path().join("link.trim");
        let mut p = Project::new("Real");
        save_project(&mut p, &real_path).unwrap();
        std::os::unix::fs::symlink(&real_path, &link_path).unwrap();
        assert!(load_project(&link_path).is_err());
    }
}
