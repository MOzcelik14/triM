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
            .replace('\n', " ")
    };

    let mut out = String::from(";FFMETADATA1\n");
    if !project_title.trim().is_empty() {
        out.push_str(&format!("title={}\n", escape(project_title.trim())));
    }
    out.push_str("encoder=triM. Video Editor\n");

    if sorted.is_empty() {
        return out;
    }

    let end_ms_total = ((total_duration.max(1.0)) * 1000.0).round() as u64;
    for (idx, m) in sorted.iter().enumerate() {
        let start_ms = ((m.time.max(0.0)) * 1000.0).round() as u64;
        let next_ms = sorted
            .get(idx + 1)
            .map(|nm| ((nm.time.max(m.time + 0.1)) * 1000.0).round() as u64)
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
    let fps = project.settings.fps.max(1.0);
    let title = if project.settings.name.trim().is_empty() {
        "TRIM_TIMELINE"
    } else {
        project.settings.name.trim()
    };

    let mut edl = format!("TITLE: {title}\nFCM: NON-DROP FRAME\n\n");
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
            let src_in = clip.source_in.max(0.0);
            let dur = clip.duration();
            let src_out = clip.source_out.unwrap_or(src_in + dur);
            let rec_in = clip.timeline_in.max(0.0);
            let rec_out = clip.timeline_out.max(rec_in);

            let tc_src_in = seconds_to_timecode(src_in, fps);
            let tc_src_out = seconds_to_timecode(src_out, fps);
            let tc_rec_in = seconds_to_timecode(rec_in, fps);
            let tc_rec_out = seconds_to_timecode(rec_out, fps);

            edl.push_str(&format!(
                "{event_idx:03}  {reel:<8} {track_code:<5} C        {tc_src_in} {tc_src_out} {tc_rec_in} {tc_rec_out}\n"
            ));
            let clip_name = if !clip.name.trim().is_empty() {
                clip.name.trim().to_string()
            } else if let Some(media) = project.media_pool.get(&clip.media_id) {
                media.name.clone()
            } else {
                clip.id.clone()
            };
            edl.push_str(&format!("* FROM CLIP NAME: {clip_name}\n\n"));
            event_idx += 1;
        }
    }

    edl
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

    let fps_num = (project.settings.fps.round() as u32).max(1);
    let width = project.settings.width.max(1);
    let height = project.settings.height.max(1);
    let total_dur_secs = project.timeline.duration().max(1.0);
    let total_frames = (total_dur_secs * fps_num as f64).round() as u64;

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<!DOCTYPE fcpxml>\n");
    xml.push_str("<fcpxml version=\"1.9\">\n");
    xml.push_str("  <resources>\n");
    xml.push_str(&format!(
        "    <format id=\"r1\" name=\"FFVideoFormat{height}p{fps_num}\" frameDuration=\"1/{fps_num}s\" width=\"{width}\" height=\"{height}\"/>\n"
    ));

    let mut media_keys: Vec<_> = project.media_pool.keys().cloned().collect();
    media_keys.sort();
    let mut asset_map = std::collections::HashMap::new();
    for (idx, media_id) in media_keys.iter().enumerate() {
        if let Some(item) = project.media_pool.get(media_id) {
            let res_id = format!("r{}", idx + 2);
            asset_map.insert(media_id.clone(), res_id.clone());
            let dur_frames = ((item.duration.max(1.0)) * fps_num as f64).round() as u64;
            let name = xml_escape(&item.name);
            let path_escaped = xml_escape(&item.file_path);
            xml.push_str(&format!(
                "    <asset id=\"{res_id}\" name=\"{name}\" start=\"0s\" duration=\"{dur_frames}/{fps_num}s\" hasVideo=\"1\" hasAudio=\"1\">\n"
            ));
            xml.push_str(&format!(
                "      <media-rep kind=\"original-media\" src=\"file://{path_escaped}\"/>\n"
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
        "        <sequence format=\"r1\" duration=\"{total_frames}/{fps_num}s\" tcStart=\"0s\" tcFormat=\"NDF\">\n"
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
            let offset_frames = (clip.timeline_in.max(0.0) * fps_num as f64).round() as u64;
            let start_frames = (clip.source_in.max(0.0) * fps_num as f64).round() as u64;
            let dur_frames = (clip.duration().max(0.04) * fps_num as f64).round() as u64;
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
                "            <asset-clip name=\"{name}\" ref=\"{ref_id}\"{lane_attr} offset=\"{offset_frames}/{fps_num}s\" start=\"{start_frames}/{fps_num}s\" duration=\"{dur_frames}/{fps_num}s\"/>\n"
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
}
