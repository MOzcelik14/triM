//! CMX 3600 Edit Decision List (EDL) generator for NLE interoperability.
//!
//! Generates industry-standard CMX 3600 EDL files that can be imported directly
//! into DaVinci Resolve, Adobe Premiere Pro, Final Cut Pro, and Avid Media Composer.

use trim_core::project::Project;
use trim_core::track::TrackType;

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
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
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

            let edit_type = if clip.transition_in.is_some() { "D" } else { "C" };

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

        let mut media = MediaItem::default();
        media.id = "m1".to_string();
        media.name = "interview.mp4".to_string();
        media.file_path = "/path/to/interview.mp4".to_string();
        proj.media_pool.insert("m1".to_string(), media);

        let mut clip = Clip::default();
        clip.id = "c1".to_string();
        clip.media_id = "m1".to_string();
        clip.name = "Intro Scene".to_string();
        clip.source_in = 2.0;
        clip.source_out = Some(8.0);
        clip.timeline_in = 0.0;
        clip.timeline_out = 6.0;

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
}
