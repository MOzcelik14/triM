//! Local offline speech-to-text transcription engine using Whisper.
//!
//! Searches for local Whisper binaries (`whisper-cli`, `whisper.cpp`, `main`, `whisper`),
//! extracts 16kHz mono audio from media files via FFmpeg, runs local transcription,
//! and parses generated timestamps into timeline subtitle entries.

use std::path::{Path, PathBuf};
use std::process::Command;
use trim_core::subtitles::{parse_srt, SubtitleEntry};

/// Detects available local Whisper executables on the system or next to `trim`.
pub fn find_whisper_executable() -> Option<PathBuf> {
    // 1. Check directory next to the running trim executable
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let candidates = if cfg!(windows) {
                vec![
                    "whisper-cli.exe",
                    "whisper.exe",
                    "main.exe",
                    "whisper-cpp.exe",
                ]
            } else {
                vec!["whisper-cli", "whisper.cpp", "main", "whisper"]
            };

            for candidate in candidates {
                let candidate_path = exe_dir.join(candidate);
                if candidate_path.is_file() {
                    return Some(candidate_path);
                }
            }
        }
    }

    // 2. Check system PATH
    let path_names = if cfg!(windows) {
        &["whisper-cli.exe", "whisper.exe", "whisper-cpp.exe"][..]
    } else {
        &["whisper-cli", "whisper.cpp", "whisper"][..]
    };

    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            for name in path_names {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

/// Extracts a clean 16kHz mono WAV file optimized for Whisper speech-to-text inference.
pub fn extract_audio_for_whisper(media_path: &Path, output_wav: &Path) -> Result<(), String> {
    let mut cmd = crate::ffmpeg::ffmpeg_cmd();
    cmd.arg("-y")
        .arg("-v")
        .arg("error")
        .arg("-i")
        .arg(media_path)
        .arg("-vn")
        .arg("-ar")
        .arg("16000")
        .arg("-ac")
        .arg("1")
        .arg("-c:a")
        .arg("pcm_s16le")
        .arg(output_wav);

    let status = cmd
        .status()
        .map_err(|e| format!("FFmpeg ses ayıklama hatası: {}", e))?;
    if !status.success() {
        return Err("FFmpeg 16kHz mono ses ayıklama başarısız oldu".to_string());
    }

    Ok(())
}

/// Executes local Whisper inference and parses the generated SRT subtitles.
pub fn run_whisper_transcription(
    whisper_bin: &Path,
    audio_wav: &Path,
    model_path: &str,
    language: Option<&str>,
) -> Result<Vec<SubtitleEntry>, String> {
    let temp_dir = std::env::temp_dir().join("trim_transcribe");
    let _ = std::fs::create_dir_all(&temp_dir);

    let out_prefix = temp_dir.join("whisper_out");
    let expected_srt = temp_dir.join("whisper_out.srt");

    let bin_name = whisper_bin
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let mut cmd = Command::new(whisper_bin);
    crate::ffmpeg::apply_no_window(&mut cmd);

    if bin_name.contains("cli")
        || bin_name.contains("cpp")
        || bin_name == "main"
        || bin_name == "main.exe"
    {
        // whisper.cpp / whisper-cli syntax:
        // whisper-cli -m <model> -f <audio.wav> --output-srt -of <out_prefix> -l <lang>
        cmd.arg("-m").arg(model_path);
        cmd.arg("-f").arg(audio_wav);
        cmd.arg("--output-srt");
        cmd.arg("-of").arg(&out_prefix);
        if let Some(lang) = language {
            if !lang.is_empty() && lang != "auto" {
                cmd.arg("-l").arg(lang);
            }
        }
    } else {
        // Python openai-whisper syntax:
        // whisper <audio.wav> --model <model> --output_format srt --output_dir <dir>
        cmd.arg(audio_wav);
        cmd.arg("--model").arg(model_path);
        cmd.arg("--output_format").arg("srt");
        cmd.arg("--output_dir").arg(&temp_dir);
        if let Some(lang) = language {
            if !lang.is_empty() && lang != "auto" {
                cmd.arg("--language").arg(lang);
            }
        }
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Whisper çalıştırılamadı: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Whisper işlem hatası: {}", stderr));
    }

    // Try reading expected_srt or any .srt file in the temp output folder
    let srt_content = if expected_srt.is_file() {
        std::fs::read_to_string(&expected_srt).map_err(|e| format!("SRT okunamadı: {}", e))?
    } else {
        // Fallback: search directory for first generated .srt
        let mut found = None;
        if let Ok(entries) = std::fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().is_some_and(|ext| ext == "srt") {
                    found = Some(std::fs::read_to_string(entry.path()).map_err(|e| e.to_string())?);
                    break;
                }
            }
        }
        found.ok_or_else(|| "Whisper tarafından SRT dosyası üretilemedi".to_string())?
    };

    let entries =
        parse_srt(&srt_content).map_err(|e| format!("Altyazı ayrıştırma hatası: {}", e))?;
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_whisper_srt_output() {
        let fake_whisper_srt = r#"1
00:00:01,000 --> 00:00:03,500
Merhaba triM kullanıcıları, bugün video kurguluyoruz.

2
00:00:03,800 --> 00:00:06,200
Yerel whisper ile yapay zeka altyazıları hazır.
"#;

        let entries = parse_srt(fake_whisper_srt).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].index, 1);
        assert!((entries[0].start_time - 1.0).abs() < 1e-4);
        assert!((entries[0].end_time - 3.5).abs() < 1e-4);
        assert_eq!(
            entries[0].text,
            "Merhaba triM kullanıcıları, bugün video kurguluyoruz."
        );

        assert_eq!(entries[1].index, 2);
        assert!((entries[1].start_time - 3.8).abs() < 1e-4);
        assert_eq!(
            entries[1].text,
            "Yerel whisper ile yapay zeka altyazıları hazır."
        );
    }
}
