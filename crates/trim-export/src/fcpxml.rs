//! Final Cut Pro XML (FCP7 XML / xmeml) generator for NLE interoperability.
//!
//! Generates standard XML interchange files compatible with Adobe Premiere Pro,
//! DaVinci Resolve, Final Cut Pro, and other professional video editing software.

use trim_core::project::Project;
use trim_core::track::TrackType;

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn to_file_url(path: &str) -> String {
    let clean = path.replace('\\', "/");
    if clean.starts_with('/') {
        format!("file://localhost{}", clean)
    } else {
        format!("file://localhost/{}", clean)
    }
}

/// Generates Final Cut Pro XML (`xmeml` version 5) content from a `Project`.
pub fn generate_fcpxml(project: &Project) -> String {
    let fps = if project.settings.fps > 0.0 {
        project.settings.fps
    } else {
        30.0
    };
    let timebase = fps.round().max(1.0) as u64;

    let width = if project.settings.width > 0 {
        project.settings.width
    } else {
        1920
    };
    let height = if project.settings.height > 0 {
        project.settings.height
    } else {
        1080
    };

    let total_duration_seconds = project.timeline.duration();
    let total_duration_frames = (total_duration_seconds * fps).round().max(1.0) as u64;

    let proj_name = if project.settings.name.trim().is_empty() {
        "triM Proje"
    } else {
        project.settings.name.trim()
    };

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<!DOCTYPE xmeml>\n");
    xml.push_str("<xmeml version=\"5\">\n");
    xml.push_str("  <sequence id=\"sequence-1\">\n");
    xml.push_str(&format!("    <name>{}</name>\n", escape_xml(proj_name)));
    xml.push_str(&format!(
        "    <duration>{}</duration>\n",
        total_duration_frames
    ));
    xml.push_str("    <rate>\n");
    xml.push_str(&format!("      <timebase>{}</timebase>\n", timebase));
    xml.push_str("      <ntsc>FALSE</ntsc>\n");
    xml.push_str("    </rate>\n");
    xml.push_str("    <media>\n");

    // Video tracks
    xml.push_str("      <video>\n");
    xml.push_str("        <format>\n");
    xml.push_str("          <samplecharacteristics>\n");
    xml.push_str(&format!("            <width>{}</width>\n", width));
    xml.push_str(&format!("            <height>{}</height>\n", height));
    xml.push_str("            <pixelaspectratio>square</pixelaspectratio>\n");
    xml.push_str("            <rate>\n");
    xml.push_str(&format!(
        "              <timebase>{}</timebase>\n",
        timebase
    ));
    xml.push_str("            </rate>\n");
    xml.push_str("          </samplecharacteristics>\n");
    xml.push_str("        </format>\n");

    let mut clip_counter = 1usize;
    for track in project
        .timeline
        .tracks
        .iter()
        .filter(|t| t.track_type == TrackType::Video)
    {
        xml.push_str("        <track>\n");
        let mut sorted_clips = track.clips.clone();
        sorted_clips.sort_by(|a, b| a.timeline_in.total_cmp(&b.timeline_in));

        for clip in sorted_clips {
            let media_opt = project.media_pool.get(&clip.media_id);
            let clip_name = if !clip.name.trim().is_empty() {
                clip.name.clone()
            } else if let Some(m) = media_opt {
                m.name.clone()
            } else {
                format!("Klip_{}", clip_counter)
            };

            let in_frame = (clip.source_in * fps).round() as u64;
            let out_frame = (clip.effective_source_out() * fps).round() as u64;
            let start_frame = (clip.timeline_in * fps).round() as u64;
            let end_frame = (clip.timeline_out * fps).round() as u64;
            let clip_dur_frame = end_frame.saturating_sub(start_frame).max(1);

            let clip_id = format!("clipitem-{}", clip_counter);
            xml.push_str(&format!("          <clipitem id=\"{}\">\n", clip_id));
            xml.push_str(&format!(
                "            <name>{}</name>\n",
                escape_xml(&clip_name)
            ));
            xml.push_str(&format!(
                "            <duration>{}</duration>\n",
                clip_dur_frame
            ));
            xml.push_str("            <rate>\n");
            xml.push_str(&format!(
                "              <timebase>{}</timebase>\n",
                timebase
            ));
            xml.push_str("            </rate>\n");
            xml.push_str(&format!("            <start>{}</start>\n", start_frame));
            xml.push_str(&format!("            <end>{}</end>\n", end_frame));
            xml.push_str(&format!("            <in>{}</in>\n", in_frame));
            xml.push_str(&format!("            <out>{}</out>\n", out_frame));

            if let Some(m) = media_opt {
                let file_id = format!("file-{}", escape_xml(&m.id));
                let media_frames = (m.duration * fps).round().max(1.0) as u64;
                let file_url = to_file_url(&m.file_path);

                xml.push_str(&format!("            <file id=\"{}\">\n", file_id));
                xml.push_str(&format!(
                    "              <name>{}</name>\n",
                    escape_xml(&m.name)
                ));
                xml.push_str(&format!(
                    "              <pathurl>{}</pathurl>\n",
                    escape_xml(&file_url)
                ));
                xml.push_str("              <rate>\n");
                xml.push_str(&format!(
                    "                <timebase>{}</timebase>\n",
                    timebase
                ));
                xml.push_str("              </rate>\n");
                xml.push_str(&format!(
                    "              <duration>{}</duration>\n",
                    media_frames
                ));
                xml.push_str("              <media>\n");
                xml.push_str("                <video>\n");
                xml.push_str("                  <samplecharacteristics>\n");
                xml.push_str(&format!(
                    "                    <width>{}</width>\n",
                    if m.width > 0 { m.width } else { width }
                ));
                xml.push_str(&format!(
                    "                    <height>{}</height>\n",
                    if m.height > 0 { m.height } else { height }
                ));
                xml.push_str("                  </samplecharacteristics>\n");
                xml.push_str("                </video>\n");
                xml.push_str("              </media>\n");
                xml.push_str("            </file>\n");
            }

            xml.push_str("          </clipitem>\n");
            clip_counter += 1;
        }
        xml.push_str("        </track>\n");
    }
    xml.push_str("      </video>\n");

    // Audio tracks
    xml.push_str("      <audio>\n");
    for track in project
        .timeline
        .tracks
        .iter()
        .filter(|t| t.track_type == TrackType::Audio)
    {
        xml.push_str("        <track>\n");
        let mut sorted_clips = track.clips.clone();
        sorted_clips.sort_by(|a, b| a.timeline_in.total_cmp(&b.timeline_in));

        for clip in sorted_clips {
            let media_opt = project.media_pool.get(&clip.media_id);
            let clip_name = if !clip.name.trim().is_empty() {
                clip.name.clone()
            } else if let Some(m) = media_opt {
                m.name.clone()
            } else {
                format!("Ses_{}", clip_counter)
            };

            let in_frame = (clip.source_in * fps).round() as u64;
            let out_frame = (clip.effective_source_out() * fps).round() as u64;
            let start_frame = (clip.timeline_in * fps).round() as u64;
            let end_frame = (clip.timeline_out * fps).round() as u64;
            let clip_dur_frame = end_frame.saturating_sub(start_frame).max(1);

            let clip_id = format!("clipitem-{}", clip_counter);
            xml.push_str(&format!("          <clipitem id=\"{}\">\n", clip_id));
            xml.push_str(&format!(
                "            <name>{}</name>\n",
                escape_xml(&clip_name)
            ));
            xml.push_str(&format!(
                "            <duration>{}</duration>\n",
                clip_dur_frame
            ));
            xml.push_str("            <rate>\n");
            xml.push_str(&format!(
                "              <timebase>{}</timebase>\n",
                timebase
            ));
            xml.push_str("            </rate>\n");
            xml.push_str(&format!("            <start>{}</start>\n", start_frame));
            xml.push_str(&format!("            <end>{}</end>\n", end_frame));
            xml.push_str(&format!("            <in>{}</in>\n", in_frame));
            xml.push_str(&format!("            <out>{}</out>\n", out_frame));

            if let Some(m) = media_opt {
                let file_id = format!("file-{}", escape_xml(&m.id));
                let media_frames = (m.duration * fps).round().max(1.0) as u64;
                let file_url = to_file_url(&m.file_path);

                xml.push_str(&format!("            <file id=\"{}\">\n", file_id));
                xml.push_str(&format!(
                    "              <name>{}</name>\n",
                    escape_xml(&m.name)
                ));
                xml.push_str(&format!(
                    "              <pathurl>{}</pathurl>\n",
                    escape_xml(&file_url)
                ));
                xml.push_str("              <rate>\n");
                xml.push_str(&format!(
                    "                <timebase>{}</timebase>\n",
                    timebase
                ));
                xml.push_str("              </rate>\n");
                xml.push_str(&format!(
                    "              <duration>{}</duration>\n",
                    media_frames
                ));
                xml.push_str("            </file>\n");
            }

            xml.push_str("          </clipitem>\n");
            clip_counter += 1;
        }
        xml.push_str("        </track>\n");
    }
    xml.push_str("      </audio>\n");

    xml.push_str("    </media>\n");
    xml.push_str("  </sequence>\n");
    xml.push_str("</xmeml>\n");

    xml
}

#[cfg(test)]
mod tests {
    use super::*;
    use trim_core::clip::Clip;
    use trim_core::media::MediaItem;
    use trim_core::project::Project;
    use trim_core::track::{Track, TrackType};

    #[test]
    fn test_generate_fcpxml() {
        let mut proj = Project::default();
        proj.settings.name = "Short Film <Cut>".to_string();
        proj.settings.fps = 30.0;
        proj.settings.width = 1920;
        proj.settings.height = 1080;

        let media = MediaItem {
            id: "m1".to_string(),
            name: "scene1.mov".to_string(),
            file_path: "/media/scene1.mov".to_string(),
            duration: 10.0,
            ..Default::default()
        };
        proj.media_pool.insert("m1".to_string(), media);

        let clip = Clip {
            id: "c1".to_string(),
            media_id: "m1".to_string(),
            name: "Scene 1 Action".to_string(),
            source_in: 1.0,
            source_out: Some(5.0),
            timeline_in: 0.0,
            timeline_out: 4.0,
            ..Default::default()
        };

        let mut track = Track::new("Video 1", TrackType::Video);
        track.clips.push(clip);
        proj.timeline.tracks.push(track);

        let xml = generate_fcpxml(&proj);
        assert!(xml.contains("<!DOCTYPE xmeml>"));
        assert!(xml.contains("<name>Short Film &lt;Cut&gt;</name>"));
        assert!(xml.contains("<timebase>30</timebase>"));
        assert!(xml.contains("<width>1920</width>"));
        assert!(xml.contains("<height>1080</height>"));
        assert!(xml.contains("<clipitem id=\"clipitem-1\">"));
        assert!(xml.contains("<name>Scene 1 Action</name>"));
        assert!(xml.contains("<pathurl>file://localhost/media/scene1.mov</pathurl>"));
    }
}
