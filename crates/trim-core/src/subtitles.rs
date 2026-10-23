use crate::clip::{Clip, TextOverlay};

#[derive(Debug, Clone, PartialEq)]
pub struct SubtitleEntry {
    pub index: usize,
    pub start_time: f64,
    pub end_time: f64,
    pub text: String,
}

impl SubtitleEntry {
    pub fn new(index: usize, start_time: f64, end_time: f64, text: impl Into<String>) -> Self {
        Self {
            index,
            start_time,
            end_time,
            text: text.into(),
        }
    }
}

/// Parses a timestamp string (`HH:MM:SS,mmm` or `HH:MM:SS.mmm` or `MM:SS.mmm`) into seconds.
pub fn parse_timestamp_to_seconds(ts: &str) -> Option<f64> {
    let clean = ts.trim().replace(',', ".");
    let parts: Vec<&str> = clean.split(':').collect();
    match parts.len() {
        2 => {
            let m: f64 = parts[0].trim().parse().ok()?;
            let s: f64 = parts[1].trim().parse().ok()?;
            Some(m * 60.0 + s)
        }
        3 => {
            let h: f64 = parts[0].trim().parse().ok()?;
            let m: f64 = parts[1].trim().parse().ok()?;
            let s: f64 = parts[2].trim().parse().ok()?;
            Some(h * 3600.0 + m * 60.0 + s)
        }
        _ => None,
    }
}

/// Formats seconds into SRT timestamp (`HH:MM:SS,mmm`).
pub fn format_seconds_to_srt_timestamp(seconds: f64) -> String {
    let total_ms = (seconds.max(0.0) * 1000.0).round() as u64;
    let ms = total_ms % 1000;
    let total_s = total_ms / 1000;
    let s = total_s % 60;
    let total_m = total_s / 60;
    let m = total_m % 60;
    let h = total_m / 60;
    format!("{:02}:{:02}:{:02},{:03}", h, m, s, ms)
}

/// Strips simple HTML styling tags (`<i>`, `<b>`, `<u>`, `<font...>`, `</...>`).
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(c);
        }
    }
    out
}

/// Parses standard SRT subtitle format content.
pub fn parse_srt(content: &str) -> Result<Vec<SubtitleEntry>, String> {
    let mut entries = Vec::new();
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let blocks = normalized.split("\n\n");

    for block in blocks {
        let lines: Vec<&str> = block.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        if lines.is_empty() {
            continue;
        }

        // Line 0 is usually the number index (e.g. "1"), or the arrow time line if index is omitted
        let (index, time_line_idx) = if lines[0].contains("-->") {
            (entries.len() + 1, 0)
        } else {
            let idx = lines[0].parse::<usize>().unwrap_or(entries.len() + 1);
            if lines.len() < 2 {
                continue;
            }
            (idx, 1)
        };

        let time_line = lines[time_line_idx];
        if !time_line.contains("-->") {
            continue;
        }

        let time_parts: Vec<&str> = time_line.split("-->").collect();
        if time_parts.len() != 2 {
            continue;
        }

        // Some VTT or SRT lines contain position coordinates after the end timestamp (e.g. "00:00:05.000 line:80%")
        let start_str = time_parts[0].trim();
        let end_str = time_parts[1].split_whitespace().next().unwrap_or("");

        let start_time = match parse_timestamp_to_seconds(start_str) {
            Some(t) => t,
            None => continue,
        };
        let end_time = match parse_timestamp_to_seconds(end_str) {
            Some(t) => t,
            None => continue,
        };

        if end_time <= start_time {
            continue;
        }

        let text_lines: Vec<&str> = lines.iter().skip(time_line_idx + 1).copied().collect();
        let raw_text = text_lines.join("\n");
        let clean_text = strip_tags(&raw_text).trim().to_string();

        if !clean_text.is_empty() {
            entries.push(SubtitleEntry {
                index,
                start_time,
                end_time,
                text: clean_text,
            });
        }
    }

    Ok(entries)
}

/// Parses WebVTT (.vtt) subtitle format content.
pub fn parse_vtt(content: &str) -> Result<Vec<SubtitleEntry>, String> {
    // WebVTT has "WEBVTT" header at the beginning; strip it and parse remainder as standard subtitle blocks
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let without_header = if let Some(stripped) = normalized.strip_prefix("WEBVTT") {
        stripped
    } else {
        &normalized
    };

    parse_srt(without_header)
}

/// Exports a list of subtitle entries to standard SRT string format.
pub fn export_srt(entries: &[SubtitleEntry]) -> String {
    let mut out = String::new();
    let mut sorted = entries.to_vec();
    sorted.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap_or(std::cmp::Ordering::Equal));

    for (i, entry) in sorted.iter().enumerate() {
        let idx = i + 1;
        let start_str = format_seconds_to_srt_timestamp(entry.start_time);
        let end_str = format_seconds_to_srt_timestamp(entry.end_time);
        out.push_str(&format!("{}\n{} --> {}\n{}\n\n", idx, start_str, end_str, entry.text));
    }

    out
}

/// Converts a slice of `SubtitleEntry` into timeline `Clip`s equipped with `TextOverlay`.
pub fn subtitle_entries_to_clips(entries: &[SubtitleEntry], default_font_size: f32) -> Vec<Clip> {
    entries
        .iter()
        .map(|entry| {
            let mut clip = Clip::new("", entry.start_time, entry.end_time);
            clip.name = format!("Altyazı #{}: {}", entry.index, entry.text.chars().take(20).collect::<String>());
            clip.fade_in = 0.05;
            clip.fade_out = 0.05;
            clip.text_overlay = Some(TextOverlay {
                content: entry.text.clone(),
                font_size: default_font_size,
                text_color: [255, 255, 255, 255],
                bg_color: Some([0, 0, 0, 180]),
                pos_x: 0.5,
                pos_y: 0.85,
                has_shadow: true,
            });
            clip
        })
        .collect()
}

/// Extracts `SubtitleEntry` from clips that have a non-empty `TextOverlay`.
pub fn clips_to_subtitle_entries(clips: &[Clip]) -> Vec<SubtitleEntry> {
    let mut entries = Vec::new();
    let mut sorted: Vec<&Clip> = clips
        .iter()
        .filter(|c| c.text_overlay.as_ref().is_some_and(|t| !t.content.trim().is_empty()))
        .collect();

    sorted.sort_by(|a, b| a.timeline_in.partial_cmp(&b.timeline_in).unwrap_or(std::cmp::Ordering::Equal));

    for (i, c) in sorted.iter().enumerate() {
        if let Some(txt) = &c.text_overlay {
            entries.push(SubtitleEntry {
                index: i + 1,
                start_time: c.timeline_in,
                end_time: c.timeline_out,
                text: txt.content.clone(),
            });
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp_variations() {
        assert_eq!(parse_timestamp_to_seconds("00:01:23,456"), Some(83.456));
        assert_eq!(parse_timestamp_to_seconds("00:01:23.456"), Some(83.456));
        assert_eq!(parse_timestamp_to_seconds("01:23.456"), Some(83.456));
        assert_eq!(parse_timestamp_to_seconds("01:00:00,000"), Some(3600.0));
    }

    #[test]
    fn test_format_seconds_to_srt_timestamp() {
        assert_eq!(format_seconds_to_srt_timestamp(83.456), "00:01:23,456");
        assert_eq!(format_seconds_to_srt_timestamp(3661.050), "01:01:01,050");
    }

    #[test]
    fn test_parse_and_export_srt_roundtrip() {
        let srt_content = "1\n00:00:01,200 --> 00:00:04,500\nMerhaba arkadaslar!\n\n2\n00:00:05,000 --> 00:00:08,250\ntriM. ile video kurguluyoruz.\n";
        let parsed = parse_srt(srt_content).expect("Valid SRT");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].text, "Merhaba arkadaslar!");
        assert_eq!(parsed[0].start_time, 1.2);
        assert_eq!(parsed[0].end_time, 4.5);

        assert_eq!(parsed[1].text, "triM. ile video kurguluyoruz.");
        assert_eq!(parsed[1].start_time, 5.0);
        assert_eq!(parsed[1].end_time, 8.25);

        let exported = export_srt(&parsed);
        assert!(exported.contains("00:00:01,200 --> 00:00:04,500"));
        assert!(exported.contains("Merhaba arkadaslar!"));
        assert!(exported.contains("00:00:05,000 --> 00:00:08,250"));
        assert!(exported.contains("triM. ile video kurguluyoruz."));
    }

    #[test]
    fn test_parse_vtt_with_header() {
        let vtt = "WEBVTT\n\n1\n00:00:02.000 --> 00:00:05.000\n<b>Web VTT</b> subtitle\n";
        let parsed = parse_vtt(vtt).expect("Valid VTT");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].text, "Web VTT subtitle"); // tags stripped
    }

    #[test]
    fn test_subtitle_entries_to_clips_and_back() {
        let entries = vec![
            SubtitleEntry::new(1, 0.0, 3.0, "Birinci baslik"),
            SubtitleEntry::new(2, 3.5, 7.0, "Ikinci baslik"),
        ];

        let clips = subtitle_entries_to_clips(&entries, 48.0);
        assert_eq!(clips.len(), 2);
        assert_eq!(clips[0].timeline_in, 0.0);
        assert_eq!(clips[0].duration(), 3.0);
        assert_eq!(clips[0].text_overlay.as_ref().unwrap().content, "Birinci baslik");

        let roundtrip_entries = clips_to_subtitle_entries(&clips);
        assert_eq!(roundtrip_entries.len(), 2);
        assert_eq!(roundtrip_entries[0].text, "Birinci baslik");
        assert_eq!(roundtrip_entries[1].text, "Ikinci baslik");
    }
}
