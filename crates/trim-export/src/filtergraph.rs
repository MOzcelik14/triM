//! FFmpeg complex filtergraph pipeline generator.

use crate::presets::ExportPreset;
use trim_core::media::MediaType;
use trim_core::project::Project;

pub struct TimelineExporter;

impl TimelineExporter {
    pub fn get_atempo_filters(speed: f64) -> Vec<String> {
        let mut filters = Vec::new();
        let mut curr = speed;
        if curr > 1.0 {
            while curr > 2.0 {
                filters.push("atempo=2.0".to_string());
                curr /= 2.0;
            }
            if (curr - 1.0).abs() > 0.001 {
                filters.push(format!("atempo={:.4}", curr));
            }
        } else if curr < 1.0 && curr > 0.0 {
            while curr < 0.5 {
                filters.push("atempo=0.5".to_string());
                curr /= 0.5;
            }
            if (curr - 1.0).abs() > 0.001 {
                filters.push(format!("atempo={:.4}", curr));
            }
        }
        filters
    }

    pub fn build_export_pipeline(
        project: &Project,
        preset: &ExportPreset,
        output_path: &str,
    ) -> Result<(Vec<String>, f64), String> {
        let video_tracks = project.timeline.get_video_tracks();
        if video_tracks.is_empty() {
            return Err("Projede video kanalı bulunamadı".to_string());
        }

        let mut clips = Vec::new();
        for track in video_tracks {
            if track.visible {
                clips.extend(track.clips.clone());
            }
        }
        clips.sort_by(|a, b| a.timeline_in.partial_cmp(&b.timeline_in).unwrap_or(std::cmp::Ordering::Equal));

        if clips.is_empty() {
            return Err("Dışa aktarılacak klip bulunamadı".to_string());
        }

        let out_w = preset.width.unwrap_or(project.timeline.width);
        let out_h = preset.height.unwrap_or(project.timeline.height);
        let out_fps = preset.fps.unwrap_or(project.timeline.fps);

        let mut cmd_inputs: Vec<String> = Vec::new();
        let mut filter_parts: Vec<String> = Vec::new();
        let mut concat_v_inputs: Vec<String> = Vec::new();
        let mut concat_a_inputs: Vec<String> = Vec::new();

        let mut cursor = 0.0;
        let mut input_idx = 0;

        for clip in &clips {
            // Gap handling
            if clip.timeline_in > cursor + 0.01 {
                let gap_dur = clip.timeline_in - cursor;
                cmd_inputs.extend([
                    "-f".into(),
                    "lavfi".into(),
                    "-t".into(),
                    format!("{:.4}", gap_dur),
                    "-i".into(),
                    format!("color=c=black:s={}x{}:r={}", out_w, out_h, out_fps),
                    "-f".into(),
                    "lavfi".into(),
                    "-t".into(),
                    format!("{:.4}", gap_dur),
                    "-i".into(),
                    "anullsrc=r=48000:cl=stereo".into(),
                ]);
                let gap_v_in = input_idx;
                let gap_a_in = input_idx + 1;
                input_idx += 2;

                filter_parts.push(format!("[{}:v]setsar=1[gv{}];", gap_v_in, gap_v_in));
                filter_parts.push(format!("[{}:a]asetpts=PTS-STARTPTS[ga{}];", gap_a_in, gap_a_in));
                concat_v_inputs.push(format!("[gv{}]", gap_v_in));
                concat_a_inputs.push(format!("[ga{}]", gap_a_in));
                cursor = clip.timeline_in;
            }

            let media_item = match project.get_media(&clip.media_id) {
                Some(m) => m,
                None => continue,
            };

            let dur = clip.duration();
            let src_in = clip.source_in;

            if media_item.media_type == MediaType::Image {
                cmd_inputs.extend([
                    "-loop".into(),
                    "1".into(),
                    "-t".into(),
                    format!("{:.4}", dur),
                    "-i".into(),
                    media_item.file_path.clone(),
                ]);

                let mut v_filters = vec![
                    format!("scale={}:{}:force_original_aspect_ratio=decrease", out_w, out_h),
                    format!("pad={}:{}:(ow-iw)/2:(oh-ih)/2", out_w, out_h),
                    "setsar=1".into(),
                    format!("fps={}", out_fps),
                ];

                if clip.brightness != 0.0 || clip.contrast != 1.0 || clip.saturation != 1.0 {
                    v_filters.push(format!(
                        "eq=brightness={:.3}:contrast={:.3}:saturation={:.3}",
                        clip.brightness, clip.contrast, clip.saturation
                    ));
                }
                if clip.fade_in > 0.0 {
                    let color = if clip.transition_in.as_deref() == Some("dip_white") { "white" } else { "black" };
                    v_filters.push(format!("fade=t=in:st=0:d={:.4}:color={}", clip.fade_in, color));
                }
                if clip.fade_out > 0.0 {
                    let st_out = (dur - clip.fade_out).max(0.0);
                    let color = if clip.transition_out.as_deref() == Some("dip_white") { "white" } else { "black" };
                    v_filters.push(format!("fade=t=out:st={:.4}:d={:.4}:color={}", st_out, clip.fade_out, color));
                }

                filter_parts.push(format!("[{}:v]{}[v{}];", input_idx, v_filters.join(","), input_idx));

                // Silent audio for image
                cmd_inputs.extend([
                    "-f".into(),
                    "lavfi".into(),
                    "-t".into(),
                    format!("{:.4}", dur),
                    "-i".into(),
                    "anullsrc=r=48000:cl=stereo".into(),
                ]);
                filter_parts.push(format!("[{}:a]asetpts=PTS-STARTPTS[a{}];", input_idx + 1, input_idx));
                concat_v_inputs.push(format!("[v{}]", input_idx));
                concat_a_inputs.push(format!("[a{}]", input_idx));
                input_idx += 2;
            } else {
                // Keep FFmpeg video/audio labels tied to the original source input.
                // Adding a silence generator below advances input_idx.
                let video_input_idx = input_idx;
                let src_dur = dur * clip.speed;
                cmd_inputs.extend([
                    "-ss".into(),
                    format!("{:.4}", src_in),
                    "-t".into(),
                    format!("{:.4}", src_dur),
                    "-i".into(),
                    media_item.file_path.clone(),
                ]);

                let mut v_filters = Vec::new();
                if clip.reverse {
                    v_filters.push("reverse".to_string());
                }
                if (clip.speed - 1.0).abs() > 0.001 {
                    v_filters.push(format!("setpts=(1.0/{:.4})*(PTS-STARTPTS)", clip.speed));
                } else {
                    v_filters.push("setpts=PTS-STARTPTS".to_string());
                }

                v_filters.extend([
                    format!("scale={}:{}:force_original_aspect_ratio=decrease", out_w, out_h),
                    format!("pad={}:{}:(ow-iw)/2:(oh-ih)/2", out_w, out_h),
                    "setsar=1".into(),
                    format!("fps={}", out_fps),
                ]);

                if clip.brightness != 0.0 || clip.contrast != 1.0 || clip.saturation != 1.0 {
                    v_filters.push(format!(
                        "eq=brightness={:.3}:contrast={:.3}:saturation={:.3}",
                        clip.brightness, clip.contrast, clip.saturation
                    ));
                }
                if clip.fade_in > 0.0 {
                    let color = if clip.transition_in.as_deref() == Some("dip_white") { "white" } else { "black" };
                    v_filters.push(format!("fade=t=in:st=0:d={:.4}:color={}", clip.fade_in, color));
                }
                if clip.fade_out > 0.0 {
                    let st_out = (dur - clip.fade_out).max(0.0);
                    let color = if clip.transition_out.as_deref() == Some("dip_white") { "white" } else { "black" };
                    v_filters.push(format!("fade=t=out:st={:.4}:d={:.4}:color={}", st_out, clip.fade_out, color));
                }

                filter_parts.push(format!("[{}:v]{}[v{}];", input_idx, v_filters.join(","), input_idx));

                let track = project.timeline.find_track_for_clip(&clip.id);
                let track_vol = track.map(|t| t.volume).unwrap_or(1.0);
                let track_muted = track.map(|t| t.muted).unwrap_or(false);
                let any_solo = project.timeline.tracks.iter().any(|t| t.solo);
                let track_solo = track.map(|t| t.solo).unwrap_or(false);
                let is_muted = clip.muted || track_muted || (any_solo && !track_solo);
                let eff_vol = clip.volume * track_vol;

                if !media_item.audio_codec.is_empty() && !is_muted {
                    let mut a_filters = vec![
                        "aformat=sample_rates=48000:channel_layouts=stereo".to_string(),
                    ];
                    if clip.reverse {
                        a_filters.push("areverse".to_string());
                    }
                    a_filters.push("asetpts=PTS-STARTPTS".to_string());
                    if (clip.speed - 1.0).abs() > 0.001 {
                        a_filters.extend(Self::get_atempo_filters(clip.speed));
                    }
                    if (eff_vol - 1.0).abs() > 0.01 {
                        a_filters.push(format!("volume={:.4}", eff_vol));
                    }
                    if clip.fade_in > 0.0 {
                        a_filters.push(format!("afade=t=in:ss=0:d={:.4}", clip.fade_in));
                    }
                    if clip.fade_out > 0.0 {
                        let st_out = (dur - clip.fade_out).max(0.0);
                        a_filters.push(format!("afade=t=out:st={:.4}:d={:.4}", st_out, clip.fade_out));
                    }

                    filter_parts.push(format!("[{}:a]{}[a{}];", input_idx, a_filters.join(","), input_idx));
                } else {
                    cmd_inputs.extend([
                        "-f".into(),
                        "lavfi".into(),
                        "-t".into(),
                        format!("{:.4}", dur),
                        "-i".into(),
                        "anullsrc=r=48000:cl=stereo".into(),
                    ]);
                    filter_parts.push(format!("[{}:a]asetpts=PTS-STARTPTS[a{}];", input_idx + 1, input_idx));
                    input_idx += 1;
                }

                concat_v_inputs.push(format!("[v{}]", video_input_idx));
                concat_a_inputs.push(format!("[a{}]", video_input_idx));
                input_idx += 1;
            }

            cursor = clip.timeline_out;
        }

        let total_duration = cursor.max(0.1);
        let num_segments = concat_v_inputs.len();
        if num_segments == 0 {
            return Err("Dışa aktarılacak segment bulunamadı".to_string());
        }

        let mut concat_filter = String::new();
        for (v, a) in concat_v_inputs.iter().zip(&concat_a_inputs) {
            concat_filter.push_str(v);
            concat_filter.push_str(a);
        }
        concat_filter.push_str(&format!("concat=n={}:v=1:a=1[outv][outa]", num_segments));
        filter_parts.push(concat_filter);

        let full_filtergraph = filter_parts.concat();

        let mut cmd = vec!["ffmpeg".into(), "-y".into(), "-v".into(), "error".into()];
        cmd.extend(cmd_inputs);
        cmd.extend([
            "-filter_complex".into(),
            full_filtergraph,
            "-map".into(),
            "[outv]".into(),
            "-map".into(),
            "[outa]".into(),
            "-c:v".into(),
            preset.video_codec.clone(),
            "-preset".into(),
            preset.preset.clone(),
            "-crf".into(),
            preset.crf.to_string(),
            "-c:a".into(),
            preset.audio_codec.clone(),
            "-b:a".into(),
            preset.audio_bitrate.clone(),
            "-pix_fmt".into(),
            "yuv420p".into(),
            "-progress".into(),
            "pipe:1".into(),
            output_path.to_string(),
        ]);

        Ok((cmd, total_duration))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trim_core::clip::Clip;
    use trim_core::media::MediaItem;

    #[test]
    fn test_atempo_filter_chain() {
        // 1.0x -> no filters
        assert!(TimelineExporter::get_atempo_filters(1.0).is_empty());

        // 1.5x -> [atempo=1.5]
        let f15 = TimelineExporter::get_atempo_filters(1.5);
        assert_eq!(f15.len(), 1);
        assert_eq!(f15[0], "atempo=1.5000");

        // 3.0x -> [atempo=2.0, atempo=1.5]
        let f30 = TimelineExporter::get_atempo_filters(3.0);
        assert_eq!(f30.len(), 2);
        assert_eq!(f30[0], "atempo=2.0");
        assert_eq!(f30[1], "atempo=1.5000");

        // 0.25x -> [atempo=0.5, atempo=0.5]
        let f025 = TimelineExporter::get_atempo_filters(0.25);
        assert_eq!(f025.len(), 2);
        assert_eq!(f025[0], "atempo=0.5");
        assert_eq!(f025[1], "atempo=0.5000");
    }

    #[test]
    fn test_silent_video_uses_correct_filtergraph_labels() {
        let mut project = Project::new("Silent video");
        let mut media = MediaItem::new("/videos/no-audio.mp4");
        media.media_type = MediaType::Video;
        media.duration = 5.0;
        media.audio_codec.clear();
        let id = media.id.clone();
        project.add_media(media);
        let video_track = project.timeline.get_video_tracks_mut().into_iter().next().unwrap();
        video_track.add_clip(Clip::new(id, 0.0, 5.0));

        let (command, _) = TimelineExporter::build_export_pipeline(
            &project,
            &ExportPreset::default(),
            "/tmp/out.mp4",
        ).unwrap();
        let index = command.iter().position(|arg| arg == "-filter_complex").unwrap();
        let graph = &command[index + 1];
        assert!(graph.contains("[0:v]"), "video input missing: {graph}");
        assert!(graph.contains("[1:a]"), "silence input missing: {graph}");
        assert!(graph.contains("[v0][a0]concat=n=1"), "incorrect concat labels: {graph}");
        assert!(!graph.contains("[v1]"), "video incorrectly points at audio: {graph}");
    }

    #[test]
    fn test_export_pipeline_generation() {
        let mut proj = Project::new("Export Test");
        let mut m = MediaItem::new("/path/test.mp4");
        m.media_type = MediaType::Video;
        m.duration = 10.0;
        let m_id = m.id.clone();
        proj.add_media(m);

        let v_track = proj.timeline.get_video_tracks_mut().into_iter().next().unwrap();
        let mut c = Clip::new(&m_id, 2.0, 6.0);
        c.speed = 2.0;
        c.brightness = 0.1;
        v_track.add_clip(c);

        let preset = ExportPreset::default();
        let (cmd, dur) = TimelineExporter::build_export_pipeline(&proj, &preset, "/tmp/out.mp4").unwrap();

        assert!((dur - 6.0).abs() < 1e-3);
        assert!(cmd.contains(&"-filter_complex".to_string()));
        assert!(cmd.contains(&"/tmp/out.mp4".to_string()));
    }
}
