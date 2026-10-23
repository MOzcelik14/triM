use std::path::{Path, PathBuf};

/// Generates a standardized 720p H.264 proxy file for high-resolution 4K/UHD footage.
pub fn generate_proxy<P: AsRef<Path>, Q: AsRef<Path>>(
    source_path: P,
    output_path: Q,
) -> Result<(), String> {
    let src = source_path.as_ref();
    let out = output_path.as_ref();

    if let Some(parent) = out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let src_str = src.to_string_lossy().to_string();
    let out_str = out.to_string_lossy().to_string();

    let mut cmd = crate::ffmpeg::ffmpeg_cmd();
    cmd.args([
        "-y",
        "-v",
        "error",
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
        &out_str,
    ]);

    let output = cmd.output().map_err(|e| format!("FFmpeg proxy başlatılamadı: {}", e))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Proxy oluşturma hatası: {}", err.trim()));
    }

    if !out.exists() {
        return Err("Proxy dosyası oluşturulamadı.".to_string());
    }

    Ok(())
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

    // Add path hash suffix to prevent filename collisions across different directories
    let hash = format!("{:08x}", seahash_simple(p.to_string_lossy().as_bytes()));
    cache_dir.join(format!("{}_{}_720p.mp4", clean_stem, hash))
}

fn seahash_simple(data: &[u8]) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for &b in data {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    h
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
}
