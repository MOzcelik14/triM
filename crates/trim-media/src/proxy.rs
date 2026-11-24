use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Generates a standardized 720p H.264 proxy file for high-resolution 4K/UHD footage.
pub fn generate_proxy<P: AsRef<Path>, Q: AsRef<Path>>(
    source_path: P,
    output_path: Q,
) -> Result<(), String> {
    let never_cancel = AtomicBool::new(false);
    generate_proxy_cancellable(source_path, output_path, &never_cancel)
}

/// Generates a standardized 720p H.264 proxy file with cooperative cancellation via `ChildGuard`.
pub fn generate_proxy_cancellable<P: AsRef<Path>, Q: AsRef<Path>>(
    source_path: P,
    output_path: Q,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let src = source_path.as_ref();
    let out = output_path.as_ref();

    if let Some(parent) = out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let src_str = src.to_string_lossy().to_string();
    let tmp_out = out.with_extension(format!("tmp.{}.mp4", std::process::id()));
    let tmp_str = tmp_out.to_string_lossy().to_string();

    let mut cmd = crate::ffmpeg::ffmpeg_cmd();
    cmd.args([
        "-y",
        "-v",
        "error",
        "-ignore_unknown",
        "-protocol_whitelist",
        "file",
    ]);
    if let Some(info) = crate::analyzer::detect_forced_pcm_input_codec(src) {
        cmd.args(["-c:a", info.codec]);
    }
    cmd.args([
        "-i",
        &src_str,
        "-vf",
        "scale=-2:720:force_original_aspect_ratio=decrease",
        "-c:v",
        "libx264",
        "-preset",
        "veryfast",
        "-crf",
        "24",
        "-c:a",
        "aac",
        "-b:a",
        "128k",
        &tmp_str,
    ]);
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());

    let mut guard = crate::ffmpeg::ChildGuard::spawn(&mut cmd)
        .map_err(|e| format!("FFmpeg proxy başlatılamadı (FFmpeg kurulu mu?): {}", e))?;
    let stderr_opt = guard.take_stderr();
    let stderr_handle = std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = Vec::new();
        if let Some(err) = stderr_opt {
            let _ = err.take(16 * 1024).read_to_end(&mut buf);
        }
        buf
    });

    let status = loop {
        if cancel.load(Ordering::Acquire) {
            let _ = guard.kill();
            let _ = std::fs::remove_file(&tmp_out);
            return Err("Proxy oluşturma iptal edildi.".to_string());
        }
        match guard.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
            Err(e) => {
                let _ = guard.kill();
                let _ = std::fs::remove_file(&tmp_out);
                return Err(format!("Proxy süreci izlenemedi: {}", e));
            }
        }
    };

    let stderr_bytes = stderr_handle.join().unwrap_or_default();
    if !status.success() {
        let _ = std::fs::remove_file(&tmp_out);
        let err = String::from_utf8_lossy(&stderr_bytes);
        return Err(format!(
            "Proxy oluşturma hatası (medya bozuk olabilir): {}",
            err.trim()
        ));
    }

    if !tmp_out.exists() {
        return Err("Proxy dosyası oluşturulamadı.".to_string());
    }
    std::fs::rename(&tmp_out, out).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_out);
        format!("Proxy dosyası taşınamadı (disk dolu veya izin yok?): {}", e)
    })?;

    Ok(())
}

/// Policy deciding whether preview should route through a 720p proxy file.
/// Uses proxy when `use_proxies` is enabled and a valid proxy exists, or automatically
/// recommends proxy for UHD (>= 2560x1440) sources when a proxy is available.
pub fn should_use_proxy(width: u32, height: u32, use_proxies: bool, has_proxy: bool) -> bool {
    if !has_proxy {
        return false;
    }
    use_proxies || width >= 2560 || height >= 1440
}

/// Computes the default proxy cache path for a given media file.
pub fn default_proxy_path_for<P: AsRef<Path>>(source_path: P) -> PathBuf {
    let p = source_path.as_ref();
    let cache_dir = trim_core::paths::cache_dir().join("proxies");
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "media".to_string());
    let clean_stem: String = stem
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();

    // Include mtime & file size so modified source media automatically invalidates stale proxies.
    let (mtime, size) = trim_core::paths::file_mtime_and_size(p);
    let key = format!("{}|{}|{}", p.to_string_lossy(), mtime, size);
    let hash = format!("{:016x}", trim_core::paths::stable_hash_64(key.as_bytes()));
    cache_dir.join(format!("{}_{}_720p.mp4", clean_stem, hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_proxy_path_for() {
        let p = Path::new("/videos/sample 4k footage.mp4");
        let proxy_path = default_proxy_path_for(p);
        let s = proxy_path.to_string_lossy();
        assert!(s.contains("sample_4k_footage"));
        assert!(s.ends_with("_720p.mp4"));
    }

    #[test]
    fn test_should_use_proxy_policy() {
        assert!(!should_use_proxy(3840, 2160, true, false));
        assert!(should_use_proxy(1920, 1080, true, true));
        assert!(!should_use_proxy(1920, 1080, false, true));
        assert!(should_use_proxy(3840, 2160, false, true));
    }
}
