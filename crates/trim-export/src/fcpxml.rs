//! Final Cut Pro XML (FCP7 XML / xmeml) generator for NLE interoperability.
//!
//! Generates standard XML interchange files compatible with Adobe Premiere Pro,
//! DaVinci Resolve, Final Cut Pro, and other professional video editing software.

use std::collections::HashMap;
use trim_core::clip::Clip;
use trim_core::media::{MediaItem, MediaType};
use trim_core::project::Project;
use trim_core::track::{Track, TrackType};

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

fn unescape_xml(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

/// Decodes standard `file://` URL into a local filesystem path.
pub fn decode_file_url(url: &str) -> String {
    let clean = url.trim();
    let stripped = if let Some(rest) = clean.strip_prefix("file://localhost") {
        rest
    } else if let Some(rest) = clean.strip_prefix("file://") {
        rest
    } else {
        clean
    };

    let mut bytes = Vec::new();
    let mut chars = stripped.as_bytes().iter().copied();
    while let Some(b) = chars.next() {
        if b == b'%' {
            if let (Some(h1), Some(h2)) = (chars.next(), chars.next()) {
                if let Ok(val) =
                    u8::from_str_radix(std::str::from_utf8(&[h1, h2]).unwrap_or(""), 16)
                {
                    bytes.push(val);
                    continue;
                }
            }
        }
        bytes.push(b);
    }
    String::from_utf8_lossy(&bytes).to_string()
}

/// Parses FCPX time format (e.g., "5s", "100/25s", "1200/24000s") into seconds.
pub fn parse_fcpx_time(s: &str) -> f64 {
    let clean = s.trim().trim_end_matches('s').trim();
    if let Some((num, den)) = clean.split_once('/') {
        let n: f64 = num.parse().unwrap_or(0.0);
        let d: f64 = den.parse().unwrap_or(1.0);
        if d != 0.0 {
            n / d
        } else {
            0.0
        }
    } else {
        clean.parse().unwrap_or(0.0)
    }
}

/// Lightweight XML node representation for NLE interchange files.
#[derive(Debug, Clone, Default)]
pub struct XmlNode {
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<XmlNode>,
    pub text: String,
}

impl XmlNode {
    pub fn child(&self, tag: &str) -> Option<&XmlNode> {
        self.children
            .iter()
            .find(|c| c.tag.eq_ignore_ascii_case(tag))
    }

    pub fn children_named<'a>(&'a self, tag: &'a str) -> impl Iterator<Item = &'a XmlNode> {
        self.children
            .iter()
            .filter(move |c| c.tag.eq_ignore_ascii_case(tag))
    }

    pub fn text_of(&self, tag: &str) -> Option<&str> {
        self.child(tag).map(|c| c.text.trim())
    }

    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    pub fn find_descendant(&self, tag: &str) -> Option<&XmlNode> {
        if self.tag.eq_ignore_ascii_case(tag) {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_descendant(tag) {
                return Some(found);
            }
        }
        None
    }

    pub fn parse(xml: &str) -> Result<Self, String> {
        let root = XmlNode {
            tag: "__root__".to_string(),
            ..Default::default()
        };
        let mut stack: Vec<XmlNode> = vec![root];

        let mut idx = 0;
        let bytes = xml.as_bytes();
        let len = bytes.len();

        while idx < len {
            if bytes[idx] == b'<' {
                if xml[idx..].starts_with("<!--") {
                    if let Some(end) = xml[idx..].find("-->") {
                        idx += end + 3;
                        continue;
                    } else {
                        break;
                    }
                }
                if xml[idx..].starts_with("<![CDATA[") {
                    if let Some(end) = xml[idx..].find("]]>") {
                        let cdata = &xml[idx + 9..idx + end];
                        if let Some(top) = stack.last_mut() {
                            top.text.push_str(cdata);
                        }
                        idx += end + 3;
                        continue;
                    }
                }
                if xml[idx..].starts_with("<!") || xml[idx..].starts_with("<?") {
                    if let Some(end) = xml[idx..].find('>') {
                        idx += end + 1;
                        continue;
                    } else {
                        break;
                    }
                }
                if xml[idx..].starts_with("</") {
                    if let Some(end) = xml[idx..].find('>') {
                        idx += end + 1;
                        if stack.len() > 1 {
                            let finished = stack.pop().unwrap();
                            if let Some(parent) = stack.last_mut() {
                                parent.children.push(finished);
                            }
                        }
                        continue;
                    } else {
                        break;
                    }
                }

                if let Some(end) = xml[idx..].find('>') {
                    let tag_content = xml[idx + 1..idx + end].trim();
                    let is_self_closing = tag_content.ends_with('/');
                    let content_no_slash = if is_self_closing {
                        tag_content.trim_end_matches('/').trim()
                    } else {
                        tag_content
                    };

                    let mut parts = content_no_slash.split_whitespace();
                    let tag_name = parts.next().unwrap_or("").to_string();

                    let mut node = XmlNode {
                        tag: tag_name,
                        ..Default::default()
                    };

                    let rest = content_no_slash
                        .strip_prefix(&node.tag)
                        .unwrap_or("")
                        .trim();
                    parse_attributes(rest, &mut node.attributes);

                    idx += end + 1;

                    if is_self_closing {
                        if let Some(top) = stack.last_mut() {
                            top.children.push(node);
                        }
                    } else {
                        stack.push(node);
                    }
                    continue;
                } else {
                    break;
                }
            } else {
                let next_tag = xml[idx..].find('<').map(|pos| idx + pos).unwrap_or(len);
                let text_chunk = &xml[idx..next_tag];
                let unescaped = unescape_xml(text_chunk);
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&unescaped);
                }
                idx = next_tag;
            }
        }

        while stack.len() > 1 {
            let finished = stack.pop().unwrap();
            if let Some(parent) = stack.last_mut() {
                parent.children.push(finished);
            }
        }

        let mut root = stack.pop().unwrap_or_default();
        if root.children.len() == 1 {
            Ok(root.children.pop().unwrap())
        } else {
            Ok(root)
        }
    }
}

fn parse_attributes(mut s: &str, attrs: &mut HashMap<String, String>) {
    while !s.is_empty() {
        s = s.trim();
        if s.is_empty() {
            break;
        }
        if let Some((key, rest)) = s.split_once('=') {
            let key = key.trim();
            let rest = rest.trim();
            if let Some(quote) = rest.chars().next() {
                if quote == '"' || quote == '\'' {
                    if let Some(end_quote) = rest[1..].find(quote) {
                        let val = &rest[1..1 + end_quote];
                        attrs.insert(key.to_string(), unescape_xml(val));
                        s = &rest[end_quote + 2..];
                        continue;
                    }
                }
            }
            let val = rest.split_whitespace().next().unwrap_or("");
            attrs.insert(key.to_string(), val.to_string());
            s = &rest[val.len()..];
        } else {
            break;
        }
    }
}

/// Parses Final Cut Pro XML (FCP7 `xmeml` or modern `fcpxml`) into a triM `Project`.
pub fn parse_fcpxml(xml_content: &str) -> Result<Project, String> {
    let root = XmlNode::parse(xml_content)?;

    if root.tag.eq_ignore_ascii_case("fcpxml")
        || root.find_descendant("fcpxml").is_some()
        || root.find_descendant("spine").is_some()
    {
        parse_fcpx_modern(&root)
    } else {
        parse_xmeml(&root)
    }
}

fn parse_xmeml(root: &XmlNode) -> Result<Project, String> {
    let seq = root
        .find_descendant("sequence")
        .ok_or_else(|| "FCPXML içinde <sequence> düğümü bulunamadı".to_string())?;

    let name = seq.text_of("name").unwrap_or("FCPXML Projesi");
    let fps = seq
        .find_descendant("rate")
        .and_then(|r| r.text_of("timebase"))
        .and_then(|t| t.parse::<f64>().ok())
        .unwrap_or(30.0);

    let width = seq
        .find_descendant("width")
        .and_then(|w| w.text.trim().parse::<u32>().ok())
        .unwrap_or(1920);
    let height = seq
        .find_descendant("height")
        .and_then(|h| h.text.trim().parse::<u32>().ok())
        .unwrap_or(1080);

    let mut project = Project::new(name);
    project.settings.fps = fps;
    project.settings.width = width;
    project.settings.height = height;
    project.timeline.tracks.clear();

    let media = seq.child("media");

    // Video tracks
    let mut v_tracks = Vec::new();
    if let Some(m) = media {
        if let Some(video) = m.child("video") {
            for track_node in video.children_named("track") {
                v_tracks.push(track_node);
            }
        }
    }

    if v_tracks.is_empty() {
        project
            .timeline
            .tracks
            .push(Track::new("Video 1", TrackType::Video));
    } else {
        for (idx, track_node) in v_tracks.into_iter().enumerate() {
            let mut track = Track::new(format!("Video {}", idx + 1), TrackType::Video);
            for clipitem in track_node.children_named("clipitem") {
                if let Some(clip) = parse_clipitem(clipitem, fps, &mut project, TrackType::Video) {
                    track.add_clip(clip);
                }
            }
            project.timeline.tracks.push(track);
        }
    }

    // Audio tracks
    let mut a_tracks = Vec::new();
    if let Some(m) = media {
        if let Some(audio) = m.child("audio") {
            for track_node in audio.children_named("track") {
                a_tracks.push(track_node);
            }
        }
    }

    if a_tracks.is_empty() {
        project
            .timeline
            .tracks
            .push(Track::new("Ses 1", TrackType::Audio));
        project
            .timeline
            .tracks
            .push(Track::new("Ses 2", TrackType::Audio));
    } else {
        for (idx, track_node) in a_tracks.into_iter().enumerate() {
            let mut track = Track::new(format!("Ses {}", idx + 1), TrackType::Audio);
            for clipitem in track_node.children_named("clipitem") {
                if let Some(clip) = parse_clipitem(clipitem, fps, &mut project, TrackType::Audio) {
                    track.add_clip(clip);
                }
            }
            project.timeline.tracks.push(track);
        }
    }

    Ok(project)
}

fn parse_clipitem(
    clipitem: &XmlNode,
    fps: f64,
    project: &mut Project,
    track_type: TrackType,
) -> Option<Clip> {
    let name = clipitem.text_of("name").unwrap_or("Klip");
    let start_f: f64 = clipitem.text_of("start")?.parse().ok()?;
    let end_f: f64 = clipitem.text_of("end")?.parse().ok()?;

    if start_f < 0.0 || end_f <= start_f {
        return None;
    }

    let in_f: f64 = clipitem
        .text_of("in")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0_f64)
        .max(0.0);
    let out_f: f64 = clipitem
        .text_of("out")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(in_f + (end_f - start_f));

    let timeline_in = start_f / fps;
    let timeline_out = end_f / fps;
    let source_in = in_f / fps;
    let source_out = out_f / fps;

    let file_node = clipitem.child("file");
    let pathurl = file_node.and_then(|f| f.text_of("pathurl"));
    let file_path = pathurl.map(decode_file_url).unwrap_or_default();

    let media_id = if !file_path.is_empty() {
        if let Some((existing_id, _)) = project
            .media_pool
            .iter()
            .find(|(_, m)| m.file_path == file_path)
        {
            existing_id.clone()
        } else {
            let mut item = MediaItem::new(&file_path);
            item.name = file_node
                .and_then(|f| f.text_of("name"))
                .unwrap_or(name)
                .to_string();
            item.media_type = if track_type == TrackType::Video {
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
            id: format!("media_{}", uuid::Uuid::new_v4()),
            name: name.to_string(),
            file_path: format!("/placeholder/{}", name),
            media_type: if track_type == TrackType::Video {
                MediaType::Video
            } else {
                MediaType::Audio
            },
            duration: (source_out - source_in).max(1.0),
            ..Default::default()
        };
        let id = item.id.clone();
        project.add_media(item);
        id
    };

    let mut clip = Clip::new(&media_id, timeline_in, timeline_out);
    clip.name = name.to_string();
    clip.source_in = source_in;
    clip.source_out = Some(source_out);

    Some(clip)
}

fn parse_fcpx_modern(root: &XmlNode) -> Result<Project, String> {
    let mut assets: HashMap<String, (String, String, f64)> = HashMap::new(); // id -> (name, path, duration)

    if let Some(resources) = root.find_descendant("resources") {
        for asset in resources.children_named("asset") {
            if let Some(id) = asset.attr("id") {
                let name = asset.attr("name").unwrap_or("Asset").to_string();
                let src = asset
                    .attr("src")
                    .map(decode_file_url)
                    .unwrap_or_else(|| format!("/placeholder/{}", name));
                let dur = asset.attr("duration").map(parse_fcpx_time).unwrap_or(10.0);
                assets.insert(id.to_string(), (name, src, dur));
            }
        }
    }

    let spine = root.find_descendant("spine");
    let mut project = Project::new("FCPX Projesi");
    project.timeline.tracks.clear();

    let mut v_track = Track::new("Video 1", TrackType::Video);
    let mut a_track = Track::new("Ses 1", TrackType::Audio);

    if let Some(sp) = spine {
        for item in &sp.children {
            let tag = item.tag.to_lowercase();
            let is_video = tag.contains("asset-clip") || tag.contains("video");
            let is_audio = tag.contains("audio");

            if !is_video && !is_audio {
                continue;
            }

            let asset_ref = item.attr("ref").unwrap_or("");
            let (asset_name, asset_path, _asset_dur) = assets.get(asset_ref).cloned().unwrap_or((
                item.attr("name").unwrap_or("Klip").to_string(),
                format!("/placeholder/{}", item.attr("name").unwrap_or("clip")),
                10.0,
            ));

            let offset = item.attr("offset").map(parse_fcpx_time).unwrap_or(0.0);
            let start = item.attr("start").map(parse_fcpx_time).unwrap_or(0.0);
            let dur = item.attr("duration").map(parse_fcpx_time).unwrap_or(5.0);

            let media_id = if let Some((existing_id, _)) = project
                .media_pool
                .iter()
                .find(|(_, m)| m.file_path == asset_path)
            {
                existing_id.clone()
            } else {
                let mut m = MediaItem::new(&asset_path);
                m.name = asset_name.clone();
                m.media_type = if is_video {
                    MediaType::Video
                } else {
                    MediaType::Audio
                };
                let id = m.id.clone();
                project.add_media(m);
                id
            };

            let mut clip = Clip::new(&media_id, offset, offset + dur);
            clip.name = asset_name;
            clip.source_in = start;
            clip.source_out = Some(start + dur);

            if is_video {
                v_track.add_clip(clip);
            } else {
                a_track.add_clip(clip);
            }
        }
    }

    project.timeline.tracks.push(v_track);
    project.timeline.tracks.push(a_track);

    Ok(project)
}

/// Automatically detects format (EDL or FCPXML) and imports into a triM `Project`.
pub fn import_interchange(content: &str, file_name: &str) -> Result<Project, String> {
    let lower = file_name.to_lowercase();
    if lower.ends_with(".edl") || content.starts_with("TITLE:") || content.contains("FCM:") {
        crate::edl::parse_edl(content, None)
    } else {
        parse_fcpxml(content)
    }
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

    #[test]
    fn test_parse_fcpxml_roundtrip() {
        let mut proj = Project::new("My Movie");
        proj.settings.fps = 25.0;
        proj.settings.width = 1920;
        proj.settings.height = 1080;

        let media = MediaItem {
            id: "m1".to_string(),
            name: "interview.mp4".to_string(),
            file_path: "/storage/interview.mp4".to_string(),
            duration: 60.0,
            ..Default::default()
        };
        proj.media_pool.insert("m1".to_string(), media);

        let clip = Clip {
            id: "c1".to_string(),
            media_id: "m1".to_string(),
            name: "Talking Head".to_string(),
            source_in: 5.0,
            source_out: Some(15.0),
            timeline_in: 2.0,
            timeline_out: 12.0,
            ..Default::default()
        };

        proj.timeline.tracks[0].clips.push(clip);

        let xml_str = generate_fcpxml(&proj);
        let imported = parse_fcpxml(&xml_str).expect("parse generated fcpxml");

        assert_eq!(imported.settings.name, "My Movie");
        assert_eq!(imported.settings.fps, 25.0);
        assert_eq!(imported.settings.width, 1920);
        assert_eq!(imported.settings.height, 1080);

        let v1 = &imported.timeline.tracks[0];
        assert_eq!(v1.clips.len(), 1);
        let c = &v1.clips[0];
        assert_eq!(c.name, "Talking Head");
        assert!((c.timeline_in - 2.0).abs() < 0.05);
        assert!((c.timeline_out - 12.0).abs() < 0.05);
        assert!((c.source_in - 5.0).abs() < 0.05);

        let resolved_media = imported.media_pool.values().next().expect("media item");
        assert_eq!(resolved_media.file_path, "/storage/interview.mp4");
    }

    #[test]
    fn test_parse_modern_fcpxml() {
        let modern_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<fcpxml version="1.9">
    <resources>
        <asset id="r1" name="action_shot" src="file:///media/action.mov" duration="20s" />
    </resources>
    <library>
        <event name="Event">
            <project name="Action Cut">
                <sequence>
                    <spine>
                        <asset-clip ref="r1" name="action_shot" offset="1s" start="2s" duration="6s" />
                    </spine>
                </sequence>
            </project>
        </event>
    </library>
</fcpxml>"#;

        let proj = parse_fcpxml(modern_xml).expect("parse modern fcpxml");
        assert_eq!(proj.timeline.tracks.len(), 2);
        let v1 = &proj.timeline.tracks[0];
        assert_eq!(v1.clips.len(), 1);
        assert_eq!(v1.clips[0].name, "action_shot");
        assert_eq!(v1.clips[0].timeline_in, 1.0);
        assert_eq!(v1.clips[0].timeline_out, 7.0);
        assert_eq!(v1.clips[0].source_in, 2.0);
    }
}
