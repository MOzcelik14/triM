//! Media metadata extraction using system `ffprobe`.

use std::path::Path;
use thiserror::Error;
use trim_core::media::{MediaItem, MediaType};

#[derive(Error, Debug)]
pub enum ProbeError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("ffprobe execution failed: {0}")]
    ProcessFailed(String),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
}

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "tiff", "gif"];
const AUDIO_EXTENSIONS: &[&str] = &["mp3", "wav", "aac", "flac", "ogg", "m4a"];

pub struct FFprobeAnalyzer;

impl FFprobeAnalyzer {
    pub fn analyze<P: AsRef<Path>>(file_path: P) -> Result<MediaItem, ProbeError> {
        let path = file_path.as_ref();
        if !path.is_file() {
            return Err(ProbeError::FileNotFound(path.to_string_lossy().to_string()));
        }

        let output = crate::ffmpeg::ffprobe_cmd()
            .args([
                "-v",
                "error",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
            ])
            .arg(path)
            .output()
            .map_err(|e| ProbeError::ProcessFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let err_msg = if stderr.trim().is_empty() {
                format!("ffprobe exited with status {}", output.status)
            } else {
                stderr.to_string()
            };
            return Err(ProbeError::ProcessFailed(err_msg));
        }

        let json_val: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        Self::parse_probe_json(&json_val, path)
    }

    /// Internal parser separating JSON processing from process execution for thorough unit testing.
    pub fn parse_probe_json(
        json_val: &serde_json::Value,
        path: &Path,
    ) -> Result<MediaItem, ProbeError> {
        let streams = json_val
            .get("streams")
            .and_then(|s| s.as_array())
            .cloned()
            .unwrap_or_default();
        let format = json_val.get("format");

        let mut video_stream = None;
        let mut audio_stream = None;

        for s in &streams {
            let codec_type = s.get("codec_type").and_then(|c| c.as_str()).unwrap_or("");
            let tag_str = s
                .get("codec_tag_string")
                .and_then(|c| c.as_str())
                .unwrap_or("");
            let is_pcm_tag = matches!(
                tag_str,
                "ipcm" | "lpcm" | "fpcm" | "twos" | "sowt" | "in24" | "in32" | "fl32" | "fl64"
            );

            if codec_type == "video" && video_stream.is_none() {
                let is_attached_pic = s
                    .get("disposition")
                    .and_then(|d| d.get("attached_pic"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0)
                    == 1;
                // Exclude QuickTime timecode or data streams masquerading as video
                let codec_name = s.get("codec_name").and_then(|c| c.as_str()).unwrap_or("");
                let is_data = codec_name == "bin_data" || codec_name == "tmcd";
                if !is_attached_pic && !is_data {
                    video_stream = Some(s);
                }
            } else if (codec_type == "audio" || is_pcm_tag) && audio_stream.is_none() {
                audio_stream = Some(s);
            }
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let mut width = 1920;
        let mut height = 1080;
        let mut fps = 30.0;
        let mut video_codec = String::new();

        if let Some(vs) = video_stream {
            width = vs.get("width").and_then(|w| w.as_u64()).unwrap_or(1920) as u32;
            height = vs.get("height").and_then(|h| h.as_u64()).unwrap_or(1080) as u32;
            video_codec = vs
                .get("codec_name")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();

            let r_fps_str = vs.get("r_frame_rate").and_then(|r| r.as_str());
            let avg_fps_str = vs.get("avg_frame_rate").and_then(|r| r.as_str());

            fps = if let Some(r_str) = r_fps_str {
                let parsed = parse_fps(r_str);
                if r_str == "0/0" || parsed <= 0.0 {
                    avg_fps_str.map(parse_fps).unwrap_or(parsed)
                } else {
                    parsed
                }
            } else if let Some(avg_str) = avg_fps_str {
                parse_fps(avg_str)
            } else {
                30.0
            };
        }

        let duration = extract_duration(format, video_stream, audio_stream, fps, &ext);

        let file_size_bytes = format
            .and_then(|f| f.get("size"))
            .and_then(|s| s.as_str())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        let media_type = if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            MediaType::Image
        } else if video_stream.is_some()
            || ext == "mov"
            || ext == "mp4"
            || ext == "mkv"
            || ext == "m4v"
        {
            MediaType::Video
        } else if audio_stream.is_some() || AUDIO_EXTENSIONS.contains(&ext.as_str()) {
            MediaType::Audio
        } else {
            MediaType::Unknown
        };

        let mut audio_codec = String::new();
        let mut audio_channels = 0;
        let mut audio_sample_rate = 0;

        if let Some(as_) = audio_stream {
            let raw_codec = as_
                .get("codec_name")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .trim();
            let tag_str = as_
                .get("codec_tag_string")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .trim();

            audio_channels = as_
                .get("channels")
                .and_then(|c| c.as_u64())
                .filter(|&c| c > 0)
                .unwrap_or(2) as u32;
            audio_sample_rate = as_
                .get("sample_rate")
                .and_then(|s| s.as_str())
                .and_then(|s| s.parse::<u32>().ok())
                .filter(|&sr| sr > 0)
                .unwrap_or(48000);

            if !raw_codec.is_empty()
                && !raw_codec.eq_ignore_ascii_case("unknown")
                && !raw_codec.eq_ignore_ascii_case("none")
            {
                audio_codec = raw_codec.to_string();
            } else if let Some(pcm_info) = detect_forced_pcm_input_codec(path) {
                audio_codec = pcm_info.codec.to_string();
                if pcm_info.channels > 0 {
                    audio_channels = pcm_info.channels;
                }
                if pcm_info.sample_rate > 0 {
                    audio_sample_rate = pcm_info.sample_rate;
                }
            } else {
                let bits = as_
                    .get("bits_per_sample")
                    .and_then(|b| b.as_u64())
                    .filter(|&b| b > 0)
                    .or_else(|| {
                        as_.get("bits_per_raw_sample")
                            .and_then(|b| b.as_str())
                            .and_then(|s| s.parse::<u64>().ok())
                            .filter(|&b| b > 0)
                    })
                    .unwrap_or(16);
                audio_codec = match tag_str {
                    "in24" => "pcm_s24be".to_string(),
                    "in32" => "pcm_s32be".to_string(),
                    "sowt" => {
                        if bits == 24 {
                            "pcm_s24le".to_string()
                        } else {
                            "pcm_s16le".to_string()
                        }
                    }
                    "ipcm" | "lpcm" | "twos" => {
                        if bits == 24 {
                            "pcm_s24be".to_string()
                        } else if bits == 32 {
                            "pcm_s32be".to_string()
                        } else {
                            "pcm_s16be".to_string()
                        }
                    }
                    _ => "pcm_s16be".to_string(),
                };
            }
        } else if matches!(ext.as_str(), "mp4" | "mov" | "m4v") {
            if let Some(pcm_info) = detect_forced_pcm_input_codec(path) {
                audio_codec = pcm_info.codec.to_string();
                audio_channels = pcm_info.channels.max(2);
                audio_sample_rate = pcm_info.sample_rate.max(48000);
            }
        }

        let mut item = MediaItem::new(path);
        item.media_type = media_type;
        item.duration = duration;
        item.width = width;
        item.height = height;
        item.fps = fps;
        item.video_codec = video_codec;
        item.audio_codec = audio_codec;
        item.audio_channels = audio_channels;
        item.audio_sample_rate = audio_sample_rate;
        item.file_size_bytes = file_size_bytes;

        Ok(item)
    }
}

pub fn parse_time_string(val: &str) -> Option<f64> {
    let s = val.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("n/a") {
        return None;
    }
    if s.contains(':') {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 3 {
            let h = parts[0].parse::<f64>().ok()?;
            let m = parts[1].parse::<f64>().ok()?;
            let sec = parts[2].parse::<f64>().ok()?;
            let total = h * 3600.0 + m * 60.0 + sec;
            if total >= 0.0 {
                return Some(total);
            }
        } else if parts.len() == 2 {
            let m = parts[0].parse::<f64>().ok()?;
            let sec = parts[1].parse::<f64>().ok()?;
            let total = m * 60.0 + sec;
            if total >= 0.0 {
                return Some(total);
            }
        }
    }
    if let Ok(v) = s.parse::<f64>() {
        if v >= 0.0 {
            return Some(v);
        }
    }
    None
}

fn extract_duration(
    format: Option<&serde_json::Value>,
    video_stream: Option<&serde_json::Value>,
    audio_stream: Option<&serde_json::Value>,
    fps: f64,
    ext: &str,
) -> f64 {
    // 1. Format duration
    if let Some(dur) = format
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(parse_time_string)
    {
        if dur > 0.0 {
            return dur;
        }
    }

    // 2. Video stream duration
    if let Some(vs) = video_stream {
        if let Some(dur) = vs
            .get("duration")
            .and_then(|d| d.as_str())
            .and_then(parse_time_string)
        {
            if dur > 0.0 {
                return dur;
            }
        }
    }

    // 3. Audio stream duration
    if let Some(as_) = audio_stream {
        if let Some(dur) = as_
            .get("duration")
            .and_then(|d| d.as_str())
            .and_then(parse_time_string)
        {
            if dur > 0.0 {
                return dur;
            }
        }
    }

    // 4. Video stream tags (DURATION, DURATION-eng, etc.)
    if let Some(vs) = video_stream {
        if let Some(tags) = vs.get("tags") {
            for key in &["DURATION", "duration", "DURATION-eng", "duration-eng"] {
                if let Some(dur) = tags
                    .get(*key)
                    .and_then(|d| d.as_str())
                    .and_then(parse_time_string)
                {
                    if dur > 0.0 {
                        return dur;
                    }
                }
            }
        }
    }

    // 5. Format tags
    if let Some(f) = format {
        if let Some(tags) = f.get("tags") {
            for key in &["DURATION", "duration", "DURATION-eng", "duration-eng"] {
                if let Some(dur) = tags
                    .get(*key)
                    .and_then(|d| d.as_str())
                    .and_then(parse_time_string)
                {
                    if dur > 0.0 {
                        return dur;
                    }
                }
            }
        }
    }

    // 6. Fallback: nb_frames / fps (frequent in Canon camera QuickTime MOV)
    if let Some(vs) = video_stream {
        if let Some(nb_frames) = vs
            .get("nb_frames")
            .and_then(|n| n.as_str())
            .and_then(|s| s.parse::<f64>().ok())
        {
            if fps > 0.0 && nb_frames > 0.0 {
                return nb_frames / fps;
            }
        }
    }

    // Default duration for static images
    if IMAGE_EXTENSIONS.contains(&ext) {
        5.0
    } else {
        0.0
    }
}

pub fn parse_fps(fps_str: &str) -> f64 {
    let s = fps_str.trim();
    if s.contains('/') {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() == 2 {
            if let (Ok(num), Ok(den)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                if den > 0.0 && num > 0.0 {
                    let val = num / den;
                    if val > 0.0 && val <= 240.0 {
                        return val;
                    }
                }
            }
        }
    } else if let Ok(val) = s.parse::<f64>() {
        if val > 0.0 && val <= 240.0 {
            return val;
        }
    }
    30.0
}

/// Information about a raw/unrecognized PCM audio stream inside an MP4/MOV container
/// (such as Sony XAVC S / XAVC HS `ipcm`, `lpcm`, `twos`, `in24`, `sowt`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForcedPcmInputInfo {
    pub codec: &'static str,
    pub channels: u32,
    pub sample_rate: u32,
}

/// Inspects an MP4/MOV file's `stsd` / `pcmC` / `stsz` boxes directly from disk bytes
/// to identify uncompressed PCM parameters even when system FFmpeg (e.g. FFmpeg 4.4 on Pop!_OS)
/// fails to recognize the `ipcm` or `lpcm` FourCC tag.
pub fn inspect_mp4_pcm_atoms(path: &Path) -> Option<ForcedPcmInputInfo> {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = std::fs::File::open(path).ok()?;
    let file_len = file.metadata().ok()?.len();
    if file_len < 36 {
        return None;
    }

    const HEAD_READ: u64 = 2 * 1024 * 1024;
    const TAIL_READ: u64 = 4 * 1024 * 1024;

    let mut head_buf = vec![0u8; file_len.min(HEAD_READ) as usize];
    file.read_exact(&mut head_buf).ok()?;
    if let Some(info) = parse_pcm_atoms_from_slice(&head_buf) {
        return Some(info);
    }

    if file_len > HEAD_READ {
        let tail_start = file_len.saturating_sub(TAIL_READ);
        let tail_len = (file_len - tail_start) as usize;
        let mut tail_buf = vec![0u8; tail_len];
        file.seek(SeekFrom::Start(tail_start)).ok()?;
        file.read_exact(&mut tail_buf).ok()?;
        if let Some(info) = parse_pcm_atoms_from_slice(&tail_buf) {
            return Some(info);
        }
    }

    None
}

fn parse_pcm_atoms_from_slice(buf: &[u8]) -> Option<ForcedPcmInputInfo> {
    const TAGS: &[&[u8; 4]] = &[
        b"ipcm", b"lpcm", b"fpcm", b"twos", b"sowt", b"in24", b"in32", b"fl32", b"fl64",
    ];

    if buf.len() < 32 {
        return None;
    }

    for i in 0..=(buf.len() - 32) {
        let tag = &buf[i..i + 4];
        let Some(&matched_tag) = TAGS.iter().find(|&&t| t == tag) else {
            continue;
        };

        // An ISO/QuickTime SampleEntry has 6 reserved zero bytes immediately after the 4-byte tag
        if buf[i + 4..i + 10] != [0, 0, 0, 0, 0, 0] {
            continue;
        }
        let dref_idx = u16::from_be_bytes([buf[i + 10], buf[i + 11]]);
        if dref_idx == 0 || dref_idx > 64 {
            continue;
        }

        let version = u16::from_be_bytes([buf[i + 12], buf[i + 13]]);
        let raw_ch = u16::from_be_bytes([buf[i + 20], buf[i + 21]]) as u32;
        let raw_bits = u16::from_be_bytes([buf[i + 22], buf[i + 23]]) as u32;
        let raw_sr = u16::from_be_bytes([buf[i + 28], buf[i + 29]]) as u32;

        let channels = if (1..=16).contains(&raw_ch) {
            raw_ch
        } else {
            2
        };
        let sample_rate = if (8000..=192000).contains(&raw_sr) {
            raw_sr
        } else {
            48000
        };

        let mut is_le = matched_tag == b"sowt";
        let mut bits = match matched_tag {
            b"in24" => 24,
            b"in32" | b"fl32" | b"fpcm" => 32,
            b"fl64" => 64,
            _ if matches!(raw_bits, 16 | 24 | 32) => raw_bits,
            _ => 16,
        };

        // QuickTime Sound Description v1 stores exact bytes_per_packet at offset +36
        if version == 1 && i + 48 <= buf.len() {
            let bytes_per_pkt =
                u32::from_be_bytes([buf[i + 36], buf[i + 37], buf[i + 38], buf[i + 39]]);
            match bytes_per_pkt {
                2 => bits = 16,
                3 => bits = 24,
                4 => bits = 32,
                _ => {}
            }
        }

        // ISO/IEC 23003-5 `pcmC` child box stores explicit endianness and bit depth
        let pcmc_end = (i + 128).min(buf.len());
        if pcmc_end > i + 42 {
            let sub = &buf[i + 32..pcmc_end];
            if let Some(rel) = sub.windows(4).position(|w| w == b"pcmC") {
                if rel + 10 <= sub.len() {
                    let format_flags = sub[rel + 8];
                    let pcm_sample_size = sub[rel + 9] as u32;
                    is_le = (format_flags & 0x01) != 0;
                    if matches!(pcm_sample_size, 16 | 24 | 32) {
                        bits = pcm_sample_size;
                    }
                }
            }
        }

        // Also check sibling `stsz` box for fixed sample_size (channels * bytes_per_sample)
        let stsz_end = (i + 512).min(buf.len());
        if stsz_end > i + 48 {
            let sub = &buf[i + 32..stsz_end];
            if let Some(rel) = sub.windows(4).position(|w| w == b"stsz") {
                if rel + 12 <= sub.len() {
                    let sample_size = u32::from_be_bytes([
                        sub[rel + 8],
                        sub[rel + 9],
                        sub[rel + 10],
                        sub[rel + 11],
                    ]);
                    if sample_size > 0 && channels > 0 {
                        if sample_size == channels * 3 {
                            bits = 24;
                        } else if sample_size == channels * 2 {
                            bits = 16;
                        } else if sample_size == channels * 4 && bits != 32 {
                            bits = 32;
                        }
                    }
                }
            }
        }

        let codec = match matched_tag {
            b"fl32" | b"fpcm" => {
                if is_le {
                    "pcm_f32le"
                } else {
                    "pcm_f32be"
                }
            }
            b"fl64" => {
                if is_le {
                    "pcm_f64le"
                } else {
                    "pcm_f64be"
                }
            }
            _ => match (bits, is_le) {
                (24, true) => "pcm_s24le",
                (24, false) => "pcm_s24be",
                (32, true) => "pcm_s32le",
                (32, false) => "pcm_s32be",
                (_, true) => "pcm_s16le",
                (_, false) => "pcm_s16be",
            },
        };

        return Some(ForcedPcmInputInfo {
            codec,
            channels,
            sample_rate,
        });
    }

    None
}

/// Detects whether a media file contains a raw/unrecognized PCM audio track (such as Sony XAVC S `ipcm`)
/// and returns the exact FFmpeg PCM decoder (`pcm_s16be`, `pcm_s24be`, etc.) to pass before `-i`.
pub fn detect_forced_pcm_input_codec(path: &Path) -> Option<ForcedPcmInputInfo> {
    if !path.is_file() {
        return None;
    }

    let atom_info = inspect_mp4_pcm_atoms(path);

    // Also check first audio packet via ffprobe if available to refine bit depth (e.g. 24-bit vs 16-bit)
    if let Ok(output) = crate::ffmpeg::ffprobe_cmd()
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=codec_name,codec_tag_string,channels,sample_rate,bits_per_sample,bits_per_raw_sample:packet=size,duration",
            "-read_intervals",
            "%+#1",
            "-print_format",
            "json",
        ])
        .arg(path)
        .output()
    {
        if output.status.success() {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                if let Some(stream) = val
                    .get("streams")
                    .and_then(|s| s.as_array())
                    .and_then(|a| a.first())
                {
                    let codec_name = stream
                        .get("codec_name")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .trim();
                    let tag_str = stream
                        .get("codec_tag_string")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .trim();

                    let is_unknown_codec = codec_name.is_empty()
                        || codec_name.eq_ignore_ascii_case("unknown")
                        || codec_name.eq_ignore_ascii_case("none");
                    let is_pcm_tag = matches!(
                        tag_str,
                        "ipcm" | "lpcm" | "fpcm" | "twos" | "sowt" | "in24" | "in32" | "fl32" | "fl64"
                    );

                    // If FFmpeg already recognizes a standard compressed codec (e.g. aac, mp3, opus),
                    // no forced PCM decoder is needed.
                    if !is_unknown_codec && !is_pcm_tag && atom_info.is_none() {
                        return None;
                    }

                    let channels = stream
                        .get("channels")
                        .and_then(|c| c.as_u64())
                        .filter(|&c| c > 0)
                        .map(|c| c as u32)
                        .or_else(|| atom_info.map(|i| i.channels))
                        .unwrap_or(2);
                    let sample_rate = stream
                        .get("sample_rate")
                        .and_then(|s| s.as_str())
                        .and_then(|s| s.parse::<u32>().ok())
                        .filter(|&sr| sr > 0)
                        .or_else(|| atom_info.map(|i| i.sample_rate))
                        .unwrap_or(48000);

                    // Check first packet size / (duration * channels) for exact bytes_per_sample
                    if let Some(pkt) = val
                        .get("packets")
                        .and_then(|p| p.as_array())
                        .and_then(|a| a.first())
                    {
                        let pkt_size = pkt
                            .get("size")
                            .and_then(|s| s.as_str())
                            .and_then(|s| s.parse::<u64>().ok())
                            .or_else(|| pkt.get("size").and_then(|s| s.as_u64()))
                            .unwrap_or(0);
                        let pkt_dur = pkt
                            .get("duration")
                            .and_then(|d| d.as_u64())
                            .or_else(|| {
                                pkt.get("duration")
                                    .and_then(|d| d.as_str())
                                    .and_then(|s| s.parse::<u64>().ok())
                            })
                            .unwrap_or(0);

                        if pkt_size > 0 && pkt_dur > 0 && channels > 0 {
                            let denom = pkt_dur * (channels as u64);
                            if pkt_size % denom == 0 {
                                let bytes_per_sample = pkt_size / denom;
                                let is_le = tag_str == "sowt"
                                    || atom_info.is_some_and(|a| a.codec.ends_with("le"));
                                let codec = match (bytes_per_sample, is_le) {
                                    (3, true) => "pcm_s24le",
                                    (3, false) => "pcm_s24be",
                                    (4, true) => "pcm_s32le",
                                    (4, false) => "pcm_s32be",
                                    (2, true) => "pcm_s16le",
                                    (2, false) => "pcm_s16be",
                                    _ => atom_info.map(|i| i.codec).unwrap_or("pcm_s16be"),
                                };
                                return Some(ForcedPcmInputInfo {
                                    codec,
                                    channels,
                                    sample_rate,
                                });
                            }
                        }
                    }

                    if is_unknown_codec || is_pcm_tag {
                        if let Some(info) = atom_info {
                            return Some(info);
                        }
                        let codec = match tag_str {
                            "in24" => "pcm_s24be",
                            "in32" => "pcm_s32be",
                            "sowt" => "pcm_s16le",
                            _ => "pcm_s16be",
                        };
                        return Some(ForcedPcmInputInfo {
                            codec,
                            channels,
                            sample_rate,
                        });
                    }
                }
            }
        }
    }

    atom_info
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_time_string_various_formats() {
        assert_eq!(parse_time_string(""), None);
        assert_eq!(parse_time_string("N/A"), None);
        assert_eq!(parse_time_string("n/a"), None);
        assert_eq!(parse_time_string("12.5"), Some(12.5));
        assert_eq!(parse_time_string("00:01:30.000"), Some(90.0));
        assert_eq!(parse_time_string("01:00:00.000"), Some(3600.0));
        assert_eq!(parse_time_string("02:15.500"), Some(135.5));
    }

    #[test]
    fn test_canon_eos_mov_metadata_parsing() {
        let json_val = json!({
            "streams": [
                {
                    "codec_type": "video",
                    "codec_name": "h264",
                    "width": 3840,
                    "height": 2160,
                    "r_frame_rate": "30000/1001",
                    "avg_frame_rate": "30000/1001",
                    "tags": {
                        "DURATION": "00:00:45.500000000",
                        "creation_time": "2026-10-25T08:30:00.000000Z"
                    }
                },
                {
                    "codec_type": "audio",
                    "codec_name": "pcm_s16le",
                    "channels": 2,
                    "sample_rate": "48000"
                },
                {
                    "codec_type": "data",
                    "codec_name": "tmcd",
                    "tags": {
                        "timecode": "00:01:00:00"
                    }
                }
            ],
            "format": {
                "size": "150000000",
                "format_name": "mov,mp4,m4a,3gp,3g2,mj2"
            }
        });

        let path = Path::new("/path/to/MVI_0001.MOV");
        let item = FFprobeAnalyzer::parse_probe_json(&json_val, path).unwrap();

        assert_eq!(item.media_type, MediaType::Video);
        assert_eq!(item.width, 3840);
        assert_eq!(item.height, 2160);
        assert!((item.fps - 29.97).abs() < 0.01);
        assert_eq!(item.video_codec, "h264");
        assert_eq!(item.audio_codec, "pcm_s16le");
        assert_eq!(item.audio_channels, 2);
        assert_eq!(item.audio_sample_rate, 48000);
        assert!((item.duration - 45.5).abs() < 0.01);
    }

    #[test]
    fn test_sony_xavc_ipcm_missing_codec_name_and_zero_channels() {
        let json_val = json!({
            "streams": [
                {
                    "codec_type": "video",
                    "codec_name": "h264",
                    "width": 3840,
                    "height": 2160,
                    "r_frame_rate": "30000/1001",
                    "avg_frame_rate": "30000/1001",
                    "duration": "10.0"
                },
                {
                    "codec_type": "audio",
                    "codec_tag_string": "ipcm",
                    "channels": 0,
                    "sample_rate": "48000",
                    "bits_per_sample": 0
                },
                {
                    "codec_type": "data",
                    "codec_tag_string": "rtmd"
                }
            ],
            "format": {
                "duration": "10.0",
                "size": "125000000",
                "format_name": "mov,mp4,m4a,3gp,3g2,mj2"
            }
        });

        let path = Path::new("/path/to/C0001.MP4");
        let item = FFprobeAnalyzer::parse_probe_json(&json_val, path).unwrap();

        assert_eq!(item.media_type, MediaType::Video);
        assert_eq!(item.audio_codec, "pcm_s16be");
        assert_eq!(item.audio_channels, 2);
        assert_eq!(item.audio_sample_rate, 48000);
        assert!(item.has_audio());
    }

    #[test]
    fn test_canon_mov_fallback_nb_frames_and_avg_fps() {
        let json_val = json!({
            "streams": [
                {
                    "codec_type": "video",
                    "codec_name": "hevc",
                    "width": 1920,
                    "height": 1080,
                    "r_frame_rate": "0/0",
                    "avg_frame_rate": "60000/1001",
                    "nb_frames": "3000"
                }
            ],
            "format": {
                "duration": "N/A",
                "size": "50000000"
            }
        });

        let path = Path::new("/DCIM/100CANON/IMG_0123.MOV");
        let item = FFprobeAnalyzer::parse_probe_json(&json_val, path).unwrap();

        assert_eq!(item.media_type, MediaType::Video);
        assert!((item.fps - 59.94).abs() < 0.01);
        // 3000 frames at ~59.94006 fps = ~50.05 seconds
        assert!((item.duration - 50.05).abs() < 0.1);
    }
}
