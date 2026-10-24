//! CMX 3600 Edit Decision List (EDL) generator for NLE interoperability.
//!
//! Generates industry-standard CMX 3600 EDL files that can be imported directly
//! into DaVinci Resolve, Adobe Premiere Pro, Final Cut Pro, and Avid Media Composer.

use trim_core::clip::Clip;
use trim_core::media::{MediaItem, MediaType};
use trim_core::project::Project;
use trim_core::track::{Track, TrackType};

/// Formats seconds into EDL timecode `HH:MM:SS:FF`.
pub fn seconds_to_timecode(seconds: f64, fps: f64) -> String {
    let effective_fps = if fps <= 0.0 { 30.0 } else { fps };
    let fps_round = effective_fps.round().max(1.0) as u64;
    let total_frames = (seconds.max(0.0) * effective_fps).round() as u64;
    let ff = total_frames % fps_round;
    let total_seconds = total_frames / fps_round;
    let ss = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let mm = total_minutes % 60;
    let hh = total_minutes / 60;
    format!("{:02}:{:02}:{:02}:{:02}", hh, mm, ss, ff)
}

/// Parses an EDL timecode `HH:MM:SS:FF` or `HH:MM:SS;FF` into seconds.
pub fn timecode_to_seconds(tc: &str, fps: f64) -> Result<f64, String> {
    let clean = tc.trim();
    let parts: Vec<&str> = clean.split([':', ';']).collect();
    if parts.len() != 4 {
        return Err(format!(
            "Geçersiz zaman kodu (HH:MM:SS:FF biçimi bekleniyor): {clean}"
        ));
    }
    let hh: f64 = parts[0]
        .parse()
        .map_err(|_| format!("Geçersiz saat bileşeni: {}", parts[0]))?;
    let mm: f64 = parts[1]
        .parse()
        .map_err(|_| format!("Geçersiz dakika bileşeni: {}", parts[1]))?;
    let ss: f64 = parts[2]
        .parse()
        .map_err(|_| format!("Geçersiz saniye bileşeni: {}", parts[2]))?;
    let ff: f64 = parts[3]
        .parse()
        .map_err(|_| format!("Geçersiz kare bileşeni: {}", parts[3]))?;

    let effective_fps = if fps <= 0.0 { 30.0 } else { fps };
    Ok(hh * 3600.0 + mm * 60.0 + ss + (ff / effective_fps))
}

/// Generates a CMX 3600 EDL string from a `Project`.
pub fn generate_edl(project: &Project) -> String {
    let fps = if project.settings.fps > 0.0 {
        project.settings.fps
    } else {
        30.0
    };

    let title = if project.settings.name.trim().is_empty() {
        "UNTITLED_PROJECT"
    } else {
        project.settings.name.trim()
    };

    // Sanitize title for CMX 3600 header (alphanumeric, underscore, dash)
    let sanitized_title: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let mut out = String::new();
    out.push_str(&format!("TITLE: {}\n", sanitized_title));
    out.push_str("FCM: NON-DROP FRAME\n\n");

    let mut event_idx = 1usize;

    for (track_idx, track) in project.timeline.tracks.iter().enumerate() {
        let is_video = track.track_type == TrackType::Video;
        let track_code = if is_video {
            "V".to_string()
        } else {
            let audio_num = track_idx + 1;
            if audio_num == 1 {
                "A".to_string()
            } else {
                format!("A{}", audio_num)
            }
        };

        let mut sorted_clips = track.clips.clone();
        sorted_clips.sort_by(|a, b| a.timeline_in.total_cmp(&b.timeline_in));

        for clip in sorted_clips {
            let media_opt = project.media_pool.get(&clip.media_id);
            let clip_name = if !clip.name.trim().is_empty() {
                clip.name.clone()
            } else if let Some(m) = media_opt {
                m.name.clone()
            } else {
                format!("CLIP_{}", &clip.id[..clip.id.len().min(8)])
            };

            let src_in = clip.source_in;
            let src_out = clip.effective_source_out();
            let rec_in = clip.timeline_in;
            let rec_out = clip.timeline_out;

            let tc_src_in = seconds_to_timecode(src_in, fps);
            let tc_src_out = seconds_to_timecode(src_out, fps);
            let tc_rec_in = seconds_to_timecode(rec_in, fps);
            let tc_rec_out = seconds_to_timecode(rec_out, fps);

            let edit_type = if clip.transition_in.is_some() {
                "D"
            } else {
                "C"
            };

            // CMX 3600 line format:
            // 001  AX       V     C        00:00:00:00 00:00:05:00 00:00:00:00 00:00:05:00
            out.push_str(&format!(
                "{:03}  AX       {:<5} {:<8} {} {} {} {}\n",
                event_idx % 1000,
                track_code,
                edit_type,
                tc_src_in,
                tc_src_out,
                tc_rec_in,
                tc_rec_out
            ));

            out.push_str(&format!("* FROM CLIP NAME: {}\n", clip_name));
            if let Some(m) = media_opt {
                if !m.file_path.is_empty() {
                    out.push_str(&format!("* SOURCE FILE: {}\n", m.file_path));
                }
            }
            out.push('\n');

            event_idx += 1;
        }
    }

    out
}

/// Parses a CMX 3600 EDL string into a triM `Project`.
pub fn parse_edl(edl_text: &str, default_fps: Option<f64>) -> Result<Project, String> {
    let mut title = "İçe Aktarılan EDL".to_string();
    let mut fps = default_fps.unwrap_or(30.0);

    #[derive(Debug, Default)]
    struct EdlEvent {
        track_type: TrackType,
        track_index: usize,
        src_in: f64,
        src_out: f64,
        rec_in: f64,
        rec_out: f64,
        clip_name: Option<String>,
        source_file: Option<String>,
        is_dissolve: bool,
    }

    let mut events: Vec<EdlEvent> = Vec::new();
    let mut current_event: Option<EdlEvent> = None;

    for line in edl_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(t) = trimmed.strip_prefix("TITLE:") {
            title = t.trim().to_string();
            continue;
        }

        if let Some(fcm) = trimmed.strip_prefix("FCM:") {
            if fcm.contains("DROP FRAME") && !fcm.contains("NON-DROP") && default_fps.is_none() {
                fps = 29.97;
            }
            continue;
        }

        // Comment lines for the current event:
        if trimmed.starts_with('*') {
            if let Some(ref mut ev) = current_event {
                let comment = trimmed.trim_start_matches('*').trim();
                if let Some(name) = comment.strip_prefix("FROM CLIP NAME:") {
                    ev.clip_name = Some(name.trim().to_string());
                } else if let Some(src) = comment.strip_prefix("SOURCE FILE:") {
                    ev.source_file = Some(src.trim().to_string());
                }
            }
            continue;
        }

        // Event lines start with an event number (e.g. 001, 1, 002)
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if !tokens.is_empty() && tokens[0].chars().all(|c| c.is_ascii_digit()) {
            if let Some(ev) = current_event.take() {
                events.push(ev);
            }

            if tokens.len() >= 6 {
                let track_str = tokens[2].to_uppercase();
                let edit_type = tokens[3].to_uppercase();
                let is_dissolve = edit_type.starts_with('D');

                let mut tc_tokens = Vec::new();
                for &tok in &tokens[4..] {
                    if tok.contains(':') || tok.contains(';') {
                        tc_tokens.push(tok);
                    }
                }

                if tc_tokens.len() >= 4 {
                    let s_in = timecode_to_seconds(tc_tokens[0], fps)?;
                    let s_out = timecode_to_seconds(tc_tokens[1], fps)?;
                    let r_in = timecode_to_seconds(tc_tokens[2], fps)?;
                    let r_out = timecode_to_seconds(tc_tokens[3], fps)?;

                    let (track_type, track_idx) = if track_str.starts_with('V') {
                        let idx = track_str
                            .strip_prefix('V')
                            .and_then(|s| s.parse::<usize>().ok())
                            .unwrap_or(1)
                            .saturating_sub(1);
                        (TrackType::Video, idx)
                    } else {
                        let idx = if track_str.starts_with('A') {
                            track_str
                                .strip_prefix('A')
                                .and_then(|s| s.parse::<usize>().ok())
                                .unwrap_or(1)
                                .saturating_sub(1)
                        } else {
                            0
                        };
                        (TrackType::Audio, idx)
                    };

                    current_event = Some(EdlEvent {
                        track_type,
                        track_index: track_idx,
                        src_in: s_in,
                        src_out: s_out,
                        rec_in: r_in,
                        rec_out: r_out,
                        clip_name: None,
                        source_file: None,
                        is_dissolve,
                    });
                }
            }
        }
    }

    if let Some(ev) = current_event {
        events.push(ev);
    }

    if events.is_empty() {
        return Err("EDL dosyasında geçerli kurgu olayı (event) bulunamadı".to_string());
    }

    let mut project = Project::new(&title);
    project.settings.fps = fps;
    project.timeline.tracks.clear();

    let max_v_idx = events
        .iter()
        .filter(|e| e.track_type == TrackType::Video)
        .map(|e| e.track_index)
        .max()
        .unwrap_or(0);
    let max_a_idx = events
        .iter()
        .filter(|e| e.track_type == TrackType::Audio)
        .map(|e| e.track_index)
        .max()
        .unwrap_or(0)
        .max(1);

    for i in 0..=max_v_idx {
        project
            .timeline
            .tracks
            .push(Track::new(format!("Video {}", i + 1), TrackType::Video));
    }
    for i in 0..=max_a_idx {
        project
            .timeline
            .tracks
            .push(Track::new(format!("Ses {}", i + 1), TrackType::Audio));
    }

    for (idx, ev) in events.into_iter().enumerate() {
        let file_path = ev.source_file.clone().unwrap_or_default();
        let clip_name = ev.clip_name.unwrap_or_else(|| {
            if !file_path.is_empty() {
                std::path::Path::new(&file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Klip")
                    .to_string()
            } else {
                format!("Klip {}", idx + 1)
            }
        });

        let media_id = if !file_path.is_empty() {
            if let Some((existing_id, _)) = project
                .media_pool
                .iter()
                .find(|(_, m)| m.file_path == file_path)
            {
                existing_id.clone()
            } else {
                let mut item = MediaItem::new(&file_path);
                item.name = clip_name.clone();
                item.media_type = if ev.track_type == TrackType::Video {
                    MediaType::Video
                } else {
                    MediaType::Audio
                };
                let id = item.id.clone();
                project.add_media(item);
                id
            }
        } else {
            let item = MediaItem {
                id: format!("media_{}", idx + 1),
                name: clip_name.clone(),
                file_path: format!("/placeholder/{}", clip_name),
                media_type: if ev.track_type == TrackType::Video {
                    MediaType::Video
                } else {
                    MediaType::Audio
                },
                duration: (ev.src_out - ev.src_in).max(1.0),
                ..Default::default()
            };
            let id = item.id.clone();
            project.add_media(item);
            id
        };

        let mut clip = Clip::new(&media_id, ev.rec_in, ev.rec_out);
        clip.name = clip_name;
        clip.source_in = ev.src_in;
        clip.source_out = Some(ev.src_out);
        if ev.is_dissolve {
            clip.transition_in = Some("crossfade".to_string());
        }

        if ev.track_type == TrackType::Video {
            if let Some(track) = project
                .timeline
                .tracks
                .iter_mut()
                .filter(|t| t.track_type == TrackType::Video)
                .nth(ev.track_index)
            {
                track.add_clip(clip);
            }
        } else if let Some(track) = project
            .timeline
            .tracks
            .iter_mut()
            .filter(|t| t.track_type == TrackType::Audio)
            .nth(ev.track_index)
        {
            track.add_clip(clip);
        }
    }

    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;
    use trim_core::clip::Clip;
    use trim_core::media::MediaItem;
    use trim_core::project::Project;
    use trim_core::track::{Track, TrackType};

    #[test]
    fn test_seconds_to_timecode() {
        assert_eq!(seconds_to_timecode(0.0, 30.0), "00:00:00:00");
        assert_eq!(seconds_to_timecode(1.0, 30.0), "00:00:01:00");
        assert_eq!(seconds_to_timecode(65.5, 30.0), "00:01:05:15");
        assert_eq!(seconds_to_timecode(3661.0, 25.0), "01:01:01:00");
    }

    #[test]
    fn test_generate_edl() {
        let mut proj = Project::default();
        proj.settings.name = "Test Movie".to_string();
        proj.settings.fps = 24.0;

        let media = MediaItem {
            id: "m1".to_string(),
            name: "interview.mp4".to_string(),
            file_path: "/path/to/interview.mp4".to_string(),
            ..Default::default()
        };
        proj.media_pool.insert("m1".to_string(), media);

        let clip = Clip {
            id: "c1".to_string(),
            media_id: "m1".to_string(),
            name: "Intro Scene".to_string(),
            source_in: 2.0,
            source_out: Some(8.0),
            timeline_in: 0.0,
            timeline_out: 6.0,
            ..Default::default()
        };

        let mut track = Track::new("Video 1", TrackType::Video);
        track.clips.push(clip);
        proj.timeline.tracks.push(track);

        let edl = generate_edl(&proj);
        assert!(edl.contains("TITLE: Test_Movie"));
        assert!(edl.contains("FCM: NON-DROP FRAME"));
        assert!(edl.contains("001  AX       V     C"));
        assert!(edl.contains("* FROM CLIP NAME: Intro Scene"));
        assert!(edl.contains("* SOURCE FILE: /path/to/interview.mp4"));
    }

    #[test]
    fn test_timecode_to_seconds_and_roundtrip() {
        let fps = 25.0;
        let s = 125.68;
        let tc = seconds_to_timecode(s, fps);
        let back = timecode_to_seconds(&tc, fps).expect("parse timecode");
        assert!((s - back).abs() < (1.0 / fps));
    }

    #[test]
    fn test_parse_edl_roundtrip() {
        let edl_content = r#"TITLE: Short Film
FCM: NON-DROP FRAME

001  AX       V     C        00:00:02:00 00:00:08:00 00:00:00:00 00:00:06:00
* FROM CLIP NAME: Scene 1
* SOURCE FILE: /media/scene1.mp4

002  AX       A     C        00:00:00:00 00:00:06:00 00:00:00:00 00:00:06:00
* FROM CLIP NAME: Ambience
* SOURCE FILE: /media/ambience.wav

003  AX       V     D    030 00:00:05:00 00:00:10:00 00:00:06:00 00:00:11:00
* FROM CLIP NAME: Scene 2
* SOURCE FILE: /media/scene2.mp4
"#;

        let proj = parse_edl(edl_content, Some(24.0)).expect("parse valid edl");
        assert_eq!(proj.settings.name, "Short Film");
        assert_eq!(proj.settings.fps, 24.0);
        assert_eq!(proj.timeline.tracks.len(), 3); // Video 1, Ses 1, Ses 2

        let v1 = &proj.timeline.tracks[0];
        assert_eq!(v1.clips.len(), 2);
        assert_eq!(v1.clips[0].name, "Scene 1");
        assert_eq!(v1.clips[0].timeline_in, 0.0);
        assert_eq!(v1.clips[0].timeline_out, 6.0);
        assert_eq!(v1.clips[1].name, "Scene 2");
        assert!(v1.clips[1].transition_in.is_some()); // Dissolve

        let a1 = &proj.timeline.tracks[1];
        assert_eq!(a1.clips.len(), 1);
        assert_eq!(a1.clips[0].name, "Ambience");
    }
}
