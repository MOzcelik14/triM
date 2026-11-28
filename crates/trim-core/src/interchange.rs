//! NLE Timeline Interchange (CMX3600 EDL, Final Cut XML `.fcpxml`) and
//! Chapter Metadata Export (YouTube Timestamps & FFmpeg `;FFMETADATA1` embedded chapters).

use crate::marker::Marker;
use crate::project::Project;
use crate::time::seconds_to_timecode;
use crate::track::TrackType;

/// Formats timeline markers into YouTube description chapter timestamps (`MM:SS Title` or `HH:MM:SS Title`).
/// Automatically prepends `00:00 Giriş` if no marker exists within the first 0.5 seconds, as required by YouTube.
pub fn export_youtube_chapters(markers: &[Marker], _total_duration: f64) -> String {
    let mut sorted = markers.to_vec();
    sorted.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let use_hours = sorted.last().is_some_and(|m| m.time >= 3600.0);
    let fmt_ts = |secs: f64| -> String {
        let total = secs.max(0.0).floor() as u64;
        let h = total / 3600;
        let m = (total % 3600) / 60;
        let s = total % 60;
        if use_hours || h > 0 {
            format!("{h:02}:{m:02}:{s:02}")
        } else {
            format!("{m:02}:{s:02}")
        }
    };

    let mut lines = Vec::new();
    let has_zero = sorted.first().is_some_and(|m| m.time < 0.5);
    if !has_zero {
        lines.push(format!("{} Giriş", fmt_ts(0.0)));
    }

    for (idx, m) in sorted.iter().enumerate() {
        let title = if m.name.trim().is_empty() {
            format!("Bölüm {}", idx + 1)
        } else {
            m.name.trim().to_string()
        };
        let ts = if idx == 0 && m.time < 0.5 {
            fmt_ts(0.0)
        } else {
            fmt_ts(m.time)
        };
        lines.push(format!("{ts} {title}"));
    }

    lines.join("\n")
}

/// Generates an FFmpeg `;FFMETADATA1` string containing `[CHAPTER]` blocks for embedding into MP4/MKV containers.
pub fn generate_ffmetadata_chapters(
    markers: &[Marker],
    total_duration: f64,
    project_title: &str,
) -> String {
    let mut sorted = markers.to_vec();
    sorted.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let escape = |s: &str| -> String {
        s.replace('\\', "\\\\")
            .replace('=', "\\=")
            .replace(';', "\\;")
            .replace('#', "\\#")
            .replace(['\r', '\n'], " ")
    };

    let mut out = String::from(";FFMETADATA1\n");
    if !project_title.trim().is_empty() {
        out.push_str(&format!("title={}\n", escape(project_title.trim())));
    }
    out.push_str("encoder=triM. Video Editor\n");

    if sorted.is_empty() {
        return out;
    }

    let safe_total = if total_duration.is_finite() {
        total_duration.max(1.0)
    } else {
        1.0
    };
    let end_ms_total = (safe_total * 1000.0).round() as u64;
    for (idx, m) in sorted.iter().enumerate() {
        let m_time = if m.time.is_finite() {
            m.time.max(0.0)
        } else {
            0.0
        };
        let start_ms = (m_time * 1000.0).round() as u64;
        let next_ms = sorted
            .get(idx + 1)
            .map(|nm| {
                let nm_t = if nm.time.is_finite() { nm.time } else { 0.0 };
                ((nm_t.max(m_time + 0.1)) * 1000.0).round() as u64
            })
            .unwrap_or_else(|| end_ms_total.max(start_ms + 1000));
        let title = if m.name.trim().is_empty() {
            format!("Bölüm {}", idx + 1)
        } else {
            m.name.trim().to_string()
        };
        out.push_str("\n[CHAPTER]\n");
        out.push_str("TIMEBASE=1/1000\n");
        out.push_str(&format!("START={start_ms}\n"));
        out.push_str(&format!("END={next_ms}\n"));
        out.push_str(&format!("title={}\n", escape(&title)));
    }

    out
}

/// Exports the active timeline to a standard CMX 3600 EDL string compatible with DaVinci Resolve and Premiere Pro.
pub fn export_cmx3600_edl(project: &Project) -> String {
    let fps = if project.settings.fps.is_finite() {
        project.settings.fps.max(1.0)
    } else {
        30.0
    };
    let sanitize_edl_line = |s: &str| -> String {
        s.chars()
            .map(|c| {
                if matches!(c, '\r' | '\n' | '\0') {
                    ' '
                } else {
                    c
                }
            })
            .collect()
    };
    let raw_title = if project.settings.name.trim().is_empty() {
        "TRIM_TIMELINE"
    } else {
        project.settings.name.trim()
    };
    let title = sanitize_edl_line(raw_title);

    let rate = crate::time::FrameRate::from_fps(fps);
    let fcm = if rate.is_drop_frame_capable() {
        "DROP FRAME"
    } else {
        "NON-DROP FRAME"
    };
    let mut edl = format!("TITLE: {title}\nFCM: {fcm}\n\n");
    let mut event_idx = 1usize;

    for track in &project.timeline.tracks {
        let track_code = match track.track_type {
            TrackType::Video => "V",
            TrackType::Audio => "A",
        };
        for clip in &track.clips {
            if clip.is_adjustment_layer() {
                continue;
            }
            let reel = format!("AX{:06}", event_idx.min(999999));
            let src_in = if clip.source_in.is_finite() {
                clip.source_in.max(0.0)
            } else {
                0.0
            };
            let dur = clip.duration();
            let src_out = clip
                .source_out
                .filter(|v| v.is_finite())
                .unwrap_or(src_in + dur);
            let rec_in = if clip.timeline_in.is_finite() {
                clip.timeline_in.max(0.0)
            } else {
                0.0
            };
            let rec_out = if clip.timeline_out.is_finite() {
                clip.timeline_out.max(rec_in)
            } else {
                rec_in
            };

            let fmt_tc = if rate.is_drop_frame_capable() {
                crate::time::seconds_to_drop_frame_timecode
            } else {
                seconds_to_timecode
            };
            let tc_src_in = fmt_tc(src_in, fps);
            let tc_src_out = fmt_tc(src_out, fps);
            let tc_rec_in = fmt_tc(rec_in, fps);
            let tc_rec_out = fmt_tc(rec_out, fps);

            edl.push_str(&format!(
                "{event_idx:03}  {reel:<8} {track_code:<5} C        {tc_src_in} {tc_src_out} {tc_rec_in} {tc_rec_out}\n"
            ));
            let raw_clip_name = if !clip.name.trim().is_empty() {
                clip.name.trim().to_string()
            } else if let Some(media) = project.media_pool.get(&clip.media_id) {
                media.name.clone()
            } else {
                clip.id.clone()
            };
            let clip_name = sanitize_edl_line(&raw_clip_name);
            edl.push_str(&format!("* FROM CLIP NAME: {clip_name}\n\n"));
            event_idx += 1;
        }
    }

    edl
}

fn encode_file_uri(path: &str) -> String {
    let forward = path.replace('\\', "/");
    let mut uri = String::from("file://");
    if !forward.starts_with('/') {
        uri.push('/');
    }
    for &b in forward.as_bytes() {
        let keep =
            b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.' | b'~' | b':');
        if keep {
            uri.push(b as char);
        } else {
            uri.push_str(&format!("%{:02X}", b));
        }
    }
    uri
}

/// Exports the active project timeline to Final Cut Pro XML v1.9 (`.fcpxml`) compatible with DaVinci Resolve, Premiere Pro, and Final Cut Pro.
pub fn export_fcpxml(project: &Project) -> String {
    let xml_escape = |s: &str| -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    };

    let fps_safe = if project.settings.fps.is_finite() {
        project.settings.fps
    } else {
        30.0
    };
    let rate = crate::time::FrameRate::from_fps(fps_safe);
    let exact_fps = rate.to_fps();
    let fps_label = rate.nominal_fps();
    let frame_dur_attr = rate.fcpxml_frame_duration();
    let tc_format = if rate.is_drop_frame_capable() {
        "DF"
    } else {
        "NDF"
    };
    let width = project.settings.width.max(1);
    let height = project.settings.height.max(1);
    let total_dur_secs = project.timeline.duration().max(1.0);
    let total_frames = (total_dur_secs * exact_fps).round().max(1.0) as u64;
    let total_dur_attr = rate.fcpxml_time_for_frames(total_frames);

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<!DOCTYPE fcpxml>\n");
    xml.push_str("<fcpxml version=\"1.9\">\n");
    xml.push_str("  <resources>\n");
    xml.push_str(&format!(
        "    <format id=\"r1\" name=\"FFVideoFormat{height}p{fps_label}\" frameDuration=\"{frame_dur_attr}\" width=\"{width}\" height=\"{height}\"/>\n"
    ));

    let mut media_keys: Vec<_> = project.media_pool.keys().cloned().collect();
    media_keys.sort();
    let mut asset_map = std::collections::HashMap::new();
    for (idx, media_id) in media_keys.iter().enumerate() {
        if let Some(item) = project.media_pool.get(media_id) {
            let res_id = format!("r{}", idx + 2);
            asset_map.insert(media_id.clone(), res_id.clone());
            let dur_safe = if item.duration.is_finite() {
                item.duration.max(1.0)
            } else {
                1.0
            };
            let dur_frames = (dur_safe * exact_fps).round().max(1.0) as u64;
            let dur_attr = rate.fcpxml_time_for_frames(dur_frames);
            let name = xml_escape(&item.name);
            let uri_escaped = xml_escape(&encode_file_uri(&item.file_path));
            xml.push_str(&format!(
                "    <asset id=\"{res_id}\" name=\"{name}\" start=\"0s\" duration=\"{dur_attr}\" hasVideo=\"1\" hasAudio=\"1\">\n"
            ));
            xml.push_str(&format!(
                "      <media-rep kind=\"original-media\" src=\"{uri_escaped}\"/>\n"
            ));
            xml.push_str("    </asset>\n");
        }
    }
    xml.push_str("  </resources>\n");
    xml.push_str("  <library>\n");
    xml.push_str(&format!(
        "    <event name=\"{}\">\n",
        xml_escape(&project.settings.name)
    ));
    xml.push_str(&format!(
        "      <project name=\"{}\">\n",
        xml_escape(&project.settings.name)
    ));
    xml.push_str(&format!(
        "        <sequence format=\"r1\" duration=\"{total_dur_attr}\" tcStart=\"0s\" tcFormat=\"{tc_format}\">\n"
    ));
    xml.push_str("          <spine>\n");

    for (t_idx, track) in project.timeline.tracks.iter().enumerate() {
        for clip in &track.clips {
            if clip.is_adjustment_layer() {
                continue;
            }
            let Some(ref_id) = asset_map.get(&clip.media_id) else {
                continue;
            };
            let offset_frames = (clip.timeline_in.max(0.0) * exact_fps).round() as u64;
            let end_frames = (clip.timeline_out.max(clip.timeline_in) * exact_fps).round() as u64;
            let start_frames = (clip.source_in.max(0.0) * exact_fps).round() as u64;
            let dur_frames = end_frames.saturating_sub(offset_frames).max(1);
            let offset_attr = rate.fcpxml_time_for_frames(offset_frames);
            let start_attr = rate.fcpxml_time_for_frames(start_frames);
            let dur_attr = rate.fcpxml_time_for_frames(dur_frames);
            let name = xml_escape(if clip.name.is_empty() {
                &clip.id
            } else {
                &clip.name
            });
            let lane_attr = if t_idx == 0 {
                String::new()
            } else {
                format!(" lane=\"{t_idx}\"")
            };
            xml.push_str(&format!(
                "            <asset-clip name=\"{name}\" ref=\"{ref_id}\"{lane_attr} offset=\"{offset_attr}\" start=\"{start_attr}\" duration=\"{dur_attr}\"/>\n"
            ));
        }
    }

    xml.push_str("          </spine>\n");
    xml.push_str("        </sequence>\n");
    xml.push_str("      </project>\n");
    xml.push_str("    </event>\n");
    xml.push_str("  </library>\n");
    xml.push_str("</fcpxml>\n");
    xml
}

/// Exports the project timeline to OpenTimelineIO (`.otio`) JSON using `RationalTime`.
pub fn export_otio(project: &Project) -> String {
    let fps = project.timeline.fps.max(1.0);
    let fr = crate::time::FrameRate::from_fps(fps);
    let rate = fr.to_fps();

    let mut track_children = Vec::new();
    for track in &project.timeline.tracks {
        let kind = match track.track_type {
            crate::track::TrackType::Audio => "Audio",
            _ => "Video",
        };
        let mut clips_sorted = track.clips.clone();
        clips_sorted.sort_by(|a, b| a.timeline_in.total_cmp(&b.timeline_in));

        let mut items = Vec::new();
        let mut cursor_frames = 0i64;
        for clip in &clips_sorted {
            let tl_in_frames = crate::time::RationalTime::from_seconds(clip.timeline_in, fr).frames;
            if tl_in_frames > cursor_frames {
                let gap_frames = tl_in_frames - cursor_frames;
                items.push(serde_json::json!({
                    "OTIO_SCHEMA": "Gap.1",
                    "name": "",
                    "source_range": {
                        "OTIO_SCHEMA": "TimeRange.1",
                        "start_time": { "OTIO_SCHEMA": "RationalTime.1", "value": 0.0, "rate": rate },
                        "duration": { "OTIO_SCHEMA": "RationalTime.1", "value": gap_frames as f64, "rate": rate }
                    }
                }));
            }
            let start_frames = crate::time::RationalTime::from_seconds(clip.source_in, fr).frames;
            let dur_frames = crate::time::RationalTime::from_seconds(clip.duration().max(0.01), fr)
                .frames
                .max(1);
            let target_url = project
                .media_pool
                .get(&clip.media_id)
                .map(|m| m.file_path.clone())
                .unwrap_or_default();
            items.push(serde_json::json!({
                "OTIO_SCHEMA": "Clip.1",
                "name": clip.name,
                "source_range": {
                    "OTIO_SCHEMA": "TimeRange.1",
                    "start_time": { "OTIO_SCHEMA": "RationalTime.1", "value": start_frames as f64, "rate": rate },
                    "duration": { "OTIO_SCHEMA": "RationalTime.1", "value": dur_frames as f64, "rate": rate }
                },
                "media_reference": {
                    "OTIO_SCHEMA": "ExternalReference.1",
                    "target_url": target_url
                }
            }));
            cursor_frames = tl_in_frames + dur_frames;
        }

        track_children.push(serde_json::json!({
            "OTIO_SCHEMA": "Track.1",
            "name": track.name,
            "kind": kind,
            "children": items
        }));
    }

    let root = serde_json::json!({
        "OTIO_SCHEMA": "Timeline.1",
        "name": project.settings.name,
        "global_start_time": {
            "OTIO_SCHEMA": "RationalTime.1",
            "value": 0.0,
            "rate": rate
        },
        "tracks": {
            "OTIO_SCHEMA": "Stack.1",
            "name": "tracks",
            "children": track_children
        }
    });
    serde_json::to_string_pretty(&root).unwrap_or_else(|_| "{}".to_string())
}

/// Imports an OpenTimelineIO (`.otio`) JSON timeline into a triM `Project`.
pub fn import_otio(json_str: &str) -> Result<Project, String> {
    let val: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("OTIO JSON ayrıştırma hatası: {e}"))?;
    let mut project = Project::default();
    if let Some(name) = val.get("name").and_then(|v| v.as_str()) {
        if !name.trim().is_empty() {
            project.settings.name = name.to_string();
        }
    }
    if let Some(rate) = val
        .get("global_start_time")
        .and_then(|g| g.get("rate"))
        .and_then(|r| r.as_f64())
    {
        if rate.is_finite() && rate > 1.0 {
            project.set_fps(rate);
        }
    }

    let parse_rt = |rt: &serde_json::Value, fallback_rate: f64| -> f64 {
        let v = rt.get("value").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let r = rt
            .get("rate")
            .and_then(|x| x.as_f64())
            .filter(|x| *x > 0.0)
            .unwrap_or(fallback_rate);
        (v / r).max(0.0)
    };

    let Some(tracks_arr) = val
        .get("tracks")
        .and_then(|t| t.get("children"))
        .and_then(|c| c.as_array())
    else {
        return Err("OTIO dosyasında 'tracks.children' dizisi bulunamadı".to_string());
    };

    project.timeline.tracks.clear();
    let default_rate = project.timeline.fps.max(1.0);
    let mut media_by_url: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for (t_idx, trk_val) in tracks_arr.iter().enumerate() {
        let kind_str = trk_val
            .get("kind")
            .and_then(|k| k.as_str())
            .unwrap_or("Video");
        let track_type = if kind_str.eq_ignore_ascii_case("Audio") {
            crate::track::TrackType::Audio
        } else {
            crate::track::TrackType::Video
        };
        let trk_name = trk_val
            .get("name")
            .and_then(|n| n.as_str())
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("Track {}", t_idx + 1));
        let mut track = crate::track::Track::new(trk_name, track_type);

        let mut cursor_secs = 0.0_f64;
        if let Some(children) = trk_val.get("children").and_then(|c| c.as_array()) {
            for item in children {
                let schema = item
                    .get("OTIO_SCHEMA")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                let sr = item.get("source_range");
                let dur = sr
                    .and_then(|s| s.get("duration"))
                    .map(|d| parse_rt(d, default_rate))
                    .unwrap_or(0.0);

                if schema.starts_with("Gap") {
                    cursor_secs += dur;
                    continue;
                }
                if !schema.starts_with("Clip") || dur <= 0.0 {
                    continue;
                }

                let src_in = sr
                    .and_then(|s| s.get("start_time"))
                    .map(|d| parse_rt(d, default_rate))
                    .unwrap_or(0.0);
                let target_url = item
                    .get("media_reference")
                    .and_then(|m| m.get("target_url"))
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();

                let media_id = if let Some(existing_id) = media_by_url.get(&target_url) {
                    existing_id.clone()
                } else {
                    let mut m = crate::media::MediaItem::new(&target_url);
                    m.media_type = if track_type == crate::track::TrackType::Audio {
                        crate::media::MediaType::Audio
                    } else {
                        crate::media::MediaType::Video
                    };
                    m.duration = (src_in + dur).max(1.0);
                    let id = m.id.clone();
                    media_by_url.insert(target_url.clone(), id.clone());
                    project.media_pool.insert(id.clone(), m);
                    id
                };

                let mut clip = crate::clip::Clip::new(media_id, cursor_secs, cursor_secs + dur);
                clip.source_in = src_in;
                clip.source_out = Some(src_in + dur);
                if let Some(cname) = item.get("name").and_then(|n| n.as_str()) {
                    if !cname.is_empty() {
                        clip.name = cname.to_string();
                    }
                }
                track.add_clip(clip);
                cursor_secs += dur;
            }
        }
        project.timeline.tracks.push(track);
    }

    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::Clip;
    use crate::media::{MediaItem, MediaType};

    #[test]
    fn test_youtube_chapters_prepends_zero_and_sorts() {
        let markers = vec![
            Marker::with_details(125.0, "Kurgu Araçları", "#ff0000"),
            Marker::with_details(45.0, "Renk Düzeltme", "#00ff00"),
        ];
        let yt = export_youtube_chapters(&markers, 300.0);
        let lines: Vec<&str> = yt.lines().collect();
        assert_eq!(lines[0], "00:00 Giriş");
        assert_eq!(lines[1], "00:45 Renk Düzeltme");
        assert_eq!(lines[2], "02:05 Kurgu Araçları");
    }

    #[test]
    fn test_ffmetadata_chapters_format() {
        let markers = vec![
            Marker::with_details(0.0, "Başlangıç", "#ff0000"),
            Marker::with_details(10.5, "Bölüm İki", "#00ff00"),
        ];
        let meta = generate_ffmetadata_chapters(&markers, 30.0, "Demo Video");
        assert!(meta.starts_with(";FFMETADATA1"));
        assert!(meta.contains("title=Demo Video"));
        assert!(meta.contains("START=0\nEND=10500\ntitle=Başlangıç"));
        assert!(meta.contains("START=10500\nEND=30000\ntitle=Bölüm İki"));
    }

    #[test]
    fn test_edl_and_fcpxml_export() {
        let mut proj = Project::default();
        proj.settings.name = "Test Projesi".to_string();
        let mut media = MediaItem::new("/tmp/sample.mp4");
        media.media_type = MediaType::Video;
        media.duration = 20.0;
        let media_id = media.id.clone();
        proj.media_pool.insert(media_id.clone(), media);

        let mut clip = Clip::new(&media_id, 0.0, 5.0);
        clip.name = "Sahne 1".to_string();
        proj.timeline.tracks[0].clips.push(clip);

        let edl = export_cmx3600_edl(&proj);
        assert!(edl.contains("TITLE: Test Projesi"));
        assert!(edl.contains("00:00:00:00 00:00:05:00"));
        assert!(edl.contains("* FROM CLIP NAME: Sahne 1"));

        let xml = export_fcpxml(&proj);
        assert!(xml.contains("<fcpxml version=\"1.9\">"));
        assert!(xml.contains("file:///tmp/sample.mp4"));
        assert!(xml.contains("name=\"Sahne 1\""));
    }

    #[test]
    fn test_otio_roundtrip_preserves_tracks_and_rational_timing() {
        let mut proj = Project::default();
        proj.settings.name = "OTIO Test".to_string();
        proj.set_fps(24.0);
        let media = MediaItem::new("/tmp/otio_clip.mp4");
        let mid = media.id.clone();
        proj.media_pool.insert(mid.clone(), media);

        let mut clip = Clip::new(&mid, 1.0, 4.0);
        clip.name = "Hero Shot".to_string();
        clip.source_in = 2.0;
        clip.source_out = Some(5.0);
        proj.timeline.tracks[0].add_clip(clip);

        let otio_json = export_otio(&proj);
        assert!(otio_json.contains("\"OTIO_SCHEMA\": \"Timeline.1\""));
        assert!(otio_json.contains("\"OTIO_SCHEMA\": \"RationalTime.1\""));

        let reimported = import_otio(&otio_json).expect("OTIO import should succeed");
        assert_eq!(reimported.settings.name, "OTIO Test");
        assert!((reimported.timeline.fps - 24.0).abs() < 0.01);
        let c = &reimported.timeline.tracks[0].clips[0];
        assert_eq!(c.name, "Hero Shot");
        assert!((c.timeline_in - 1.0).abs() < 0.05);
        assert!((c.duration() - 3.0).abs() < 0.05);
        assert!((c.source_in - 2.0).abs() < 0.05);
    }
}
