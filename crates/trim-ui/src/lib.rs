//! triM. User Interface components and main application.

pub mod app;
pub mod canvas;
pub mod color_wheel;
pub mod dialogs;
pub mod file_dialog;
pub mod gpu_compositor;
pub mod inspector;
pub mod media_bin;
pub mod monitor;
pub mod ruler;
pub mod shortcuts;
pub mod theme;
pub mod toolbar;
pub mod track_header;
pub mod transport;
pub mod vu_meter;

pub use app::TrimApp;
pub use canvas::TimelineCanvasState;
pub use file_dialog::{
    FileDialogBridge, FileDialogConfig, FileDialogMode, FileDialogRequest, FileDialogResponse,
};
pub use gpu_compositor::{GpuCompositor, LayerUniforms, GPU_PREVIEW_SCALE};
pub use media_bin::LeftPanelTab;
pub use shortcuts::ShortcutProfile;
pub use theme::setup_theme;
pub use vu_meter::VuMeterState;

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, Vec2};
    use trim_core::project::Project;

    #[test]
    fn test_track_header_alignment() {
        let ctx = Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let project = Project::default();
                let canvas_state = TimelineCanvasState::default();

                println!(
                    "Canvas track_height = {}, track_spacing = {}",
                    canvas_state.track_height, canvas_state.track_spacing
                );

                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let header_w = track_header::TRACK_HEADER_WIDTH;

                ui.allocate_ui(Vec2::new(header_w, 600.0), |ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        let (corner_rect, _) =
                            ui.allocate_exact_size(Vec2::new(header_w, 26.0), egui::Sense::hover());
                        assert_eq!(corner_rect.width(), track_header::TRACK_HEADER_WIDTH);

                        for track in &project.timeline.tracks {
                            let (resp, _) = track_header::render_track_header(
                                ui,
                                track,
                                canvas_state.track_height,
                                false,
                                false,
                            );
                            assert_eq!(resp.rect.width(), track_header::TRACK_HEADER_WIDTH);
                            ui.add_space(canvas_state.track_spacing);
                        }
                    });
                });
            });
        });
    }

    #[test]
    fn test_canvas_state_initialization_and_target_track() {
        let mut state = TimelineCanvasState::default();
        assert_eq!(state.drag_mode, canvas::DragMode::None);
        assert_eq!(state.active_tool, toolbar::EditTool::Select);
        assert!(state.drag_target_track_id.is_none());

        state.active_tool = toolbar::EditTool::Razor;
        assert_eq!(state.active_tool, toolbar::EditTool::Razor);

        state.drag_target_track_id = Some("v2".to_string());
        assert_eq!(state.drag_target_track_id.as_deref(), Some("v2"));
    }

    #[test]
    fn test_track_height_resizing_and_offsets() {
        let mut state = TimelineCanvasState::default();
        let project = Project::default();
        let t0 = &project.timeline.tracks[0];
        let t1 = &project.timeline.tracks[1];

        // Default height is 56.0
        assert_eq!(state.get_track_height(&t0.id), 56.0);

        // Resize t0 to 80.0
        state.set_track_height(&t0.id, 80.0);
        assert_eq!(state.get_track_height(&t0.id), 80.0);
        assert_eq!(state.get_track_height(&t1.id), 56.0);

        // Clamping check (clamp between 32.0 and 160.0)
        state.set_track_height(&t0.id, 10.0);
        assert_eq!(state.get_track_height(&t0.id), 32.0);
        state.set_track_height(&t0.id, 250.0);
        assert_eq!(state.get_track_height(&t0.id), 160.0);

        // Offset calculations
        state.set_track_height(&t0.id, 80.0);
        state.set_track_height(&t1.id, 60.0);
        let offsets = state.calculate_track_offsets(&project.timeline.tracks, 100.0);
        assert_eq!(offsets.len(), project.timeline.tracks.len());
        assert_eq!(offsets[0], (100.0, 80.0));
        assert_eq!(offsets[1], (100.0 + 80.0 + state.track_spacing, 60.0));
    }

    #[test]
    fn test_toolbar_symbols_and_tooltips() {
        use toolbar::EditTool;
        assert_eq!(EditTool::Select.symbol(), "↖");
        assert_eq!(EditTool::TrackSelect.symbol(), "⇥");
        assert_eq!(EditTool::Ripple.symbol(), "◀▶");
        assert_eq!(EditTool::Razor.symbol(), "✂");
        assert_eq!(EditTool::Slip.symbol(), "↔");
        assert_eq!(EditTool::Pen.symbol(), "✎");
        assert_eq!(EditTool::Hand.symbol(), "✋");
        assert_eq!(EditTool::Type.symbol(), "T");

        for tool in [
            EditTool::Select,
            EditTool::TrackSelect,
            EditTool::Ripple,
            EditTool::Razor,
            EditTool::Slip,
            EditTool::Pen,
            EditTool::Hand,
            EditTool::Type,
        ] {
            assert!(!tool.tooltip().is_empty());
        }
    }

    #[test]
    fn test_welcome_dialog_and_audio_mixer_defaults() {
        use dialogs::{AudioMixerDialog, ShortcutsDialog, WelcomeDialog};
        let mut welcome = WelcomeDialog::default();
        assert!(welcome.is_open);
        assert_eq!(welcome.new_project_name, "Yeni Proje");

        let ctx = Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            let act = welcome.show(ctx);
            assert!(act.is_none());
            assert!(welcome.is_open);
        });

        let mixer = AudioMixerDialog::default();
        assert!(!mixer.is_open);
        assert_eq!(mixer.master_volume, 1.0);

        let shortcuts = ShortcutsDialog::default();
        assert!(!shortcuts.is_open);
    }

    #[test]
    fn test_left_panel_tab_states() {
        use media_bin::LeftPanelTab;
        let mut tab = LeftPanelTab::default();
        assert_eq!(tab, LeftPanelTab::Project);

        tab = LeftPanelTab::Effects;
        assert_eq!(tab, LeftPanelTab::Effects);
    }

    #[test]
    fn test_inspector_change_speed_scales_timeline_clip() {
        use trim_core::clip::Clip;
        let mut project = Project::default();
        let track_id = project.timeline.tracks[0].id.clone();
        let clip = Clip::new("media1", 0.0, 4.0); // 4.0s at 1.0x
        let clip_id = clip.id.clone();
        project.timeline.tracks[0].add_clip(clip);

        let tr = project.timeline.get_track_by_id_mut(&track_id).unwrap();
        let cl = tr.get_clip_by_id_mut(&clip_id).unwrap();

        // 0.5x slowdown
        cl.change_speed(0.5);
        assert_eq!(cl.speed, 0.5);
        assert_eq!(cl.duration(), 8.0);
        assert_eq!(cl.timeline_out, 8.0);

        // 2.0x speedup
        cl.change_speed(2.0);
        assert_eq!(cl.speed, 2.0);
        assert_eq!(cl.duration(), 2.0);
        assert_eq!(cl.timeline_out, 2.0);
    }

    #[test]
    fn test_close_confirm_dialog_lifecycle() {
        use dialogs::{CloseConfirmDialog, PendingCloseAction};
        use std::path::PathBuf;

        let mut dialog = CloseConfirmDialog::default();
        assert!(!dialog.is_open);
        assert!(dialog.pending_action.is_none());

        dialog.prompt(PendingCloseAction::ExitApp, "My Video Project");
        assert!(dialog.is_open);
        assert_eq!(dialog.pending_action, Some(PendingCloseAction::ExitApp));
        assert_eq!(dialog.project_name, "My Video Project");

        dialog.close();
        assert!(!dialog.is_open);
        assert!(dialog.pending_action.is_none());

        // Test with OpenPath
        let test_path = PathBuf::from("/tmp/test_project.trim");
        dialog.prompt(
            PendingCloseAction::OpenPath(test_path.clone()),
            "Another Project",
        );
        assert!(dialog.is_open);
        assert_eq!(
            dialog.pending_action,
            Some(PendingCloseAction::OpenPath(test_path))
        );
    }

    #[test]
    fn test_close_confirm_dialog_egui_render() {
        use dialogs::{CloseConfirmDialog, PendingCloseAction};
        let ctx = Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            let mut dialog = CloseConfirmDialog::default();
            dialog.prompt(PendingCloseAction::ExitApp, "Test Unsaved");
            let action = dialog.show(ctx);
            // In initial render without click, action is None
            assert!(action.is_none());
            assert!(dialog.is_open);
        });
    }

    #[test]
    fn test_project_dirty_flag_tracking() {
        let mut proj = Project::default();
        assert!(!proj.is_dirty);

        proj.mark_dirty();
        assert!(proj.is_dirty);
    }

    #[test]
    fn test_has_unsaved_changes_lifecycle() {
        use trim_commands::AddClipCommand;
        use trim_core::clip::Clip;

        let mut app = TrimApp::default();
        // Initially empty, clean project
        assert!(!app.has_unsaved_changes());

        // 1. Mark dirty directly
        app.project.mark_dirty();
        assert!(app.has_unsaved_changes());
        app.project.is_dirty = false;
        assert!(!app.has_unsaved_changes());

        // 2. Add clip to timeline via command
        let track_id = app.project.timeline.tracks[0].id.clone();
        let clip = Clip::new("m1", 0.0, 5.0);
        let cmd = Box::new(AddClipCommand::new(&track_id, clip));
        let _ = app.undo_stack.push(cmd, &mut app.project.timeline);
        // Since undo_stack len changed and project has clips without saved file_path
        assert!(app.has_unsaved_changes());

        // 3. Simulate save
        app.project.is_dirty = false;
        app.project.file_path = Some("/tmp/saved_proj.trim".to_string());
        app.last_saved_undo_len = app.undo_stack.len();
        assert!(!app.has_unsaved_changes());

        // 4. Slight change (e.g., track volume / mute / split)
        app.project.mark_dirty();
        assert!(app.has_unsaved_changes());
    }

    #[test]
    fn test_silence_dialog_lifecycle() {
        use dialogs::SilenceDialog;
        let mut dlg = SilenceDialog::default();
        assert!(!dlg.is_open);

        dlg.open("v1", "c1", "My Voice Clip", 14.5);
        assert!(dlg.is_open);
        assert_eq!(dlg.track_id, "v1");
        assert_eq!(dlg.clip_id, "c1");
        assert_eq!(dlg.clip_name, "My Voice Clip");
        assert_eq!(dlg.clip_duration, 14.5);

        // Simulate analysis output
        let silences = vec![(2.0, 4.5), (8.0, 10.0)];
        let speech = vec![(0.0, 2.0), (4.5, 8.0), (10.0, 14.5)];
        dlg.set_analysis_results(silences, speech.clone());

        assert_eq!(dlg.detected_silence_count, Some(2));
        assert!((dlg.total_silence_duration.unwrap() - 4.5).abs() < 1e-3);
        assert_eq!(dlg.speech_intervals, Some(speech));

        dlg.close();
        assert!(!dlg.is_open);
    }

    #[test]
    fn test_clip_linking_and_transitions_defaults() {
        use trim_core::clip::Clip;
        let mut c = Clip::new("m1", 0.0, 6.0);
        assert!(c.linked_group_id.is_none());
        assert!(c.transition_in.is_none());
        assert!(c.transition_out.is_none());

        c.linked_group_id = Some("grp-123".to_string());
        c.transition_in = Some("cross_dissolve".to_string());
        c.transition_out = Some("dip_black".to_string());

        assert_eq!(c.linked_group_id.as_deref(), Some("grp-123"));
        assert_eq!(c.transition_in.as_deref(), Some("cross_dissolve"));
        assert_eq!(c.transition_out.as_deref(), Some("dip_black"));
    }

    #[test]
    fn test_transcribe_and_screen_record_dialogs() {
        let mut td = crate::dialogs::TranscribeDialog::default();
        assert!(!td.is_open);
        td.open("/tmp/test.mp4", "Scene 1");
        assert!(td.is_open);
        assert_eq!(td.media_path, "/tmp/test.mp4");
        assert_eq!(td.clip_name, "Scene 1");
        td.close();
        assert!(!td.is_open);

        let mut srd = crate::dialogs::ScreenRecordDialog::default();
        assert!(!srd.is_open);
        assert_eq!(srd.source, trim_media::CaptureSource::ScreenAndMic);
        assert!(srd.auto_add_to_timeline);
        srd.open();
        assert!(srd.is_open);
        srd.close();
        assert!(!srd.is_open);
    }

    #[test]
    fn test_monitor_text_tool_click_drag_content() {
        use trim_core::clip::Clip;
        use trim_core::track::{Track, TrackType};

        let mut app = TrimApp::default();
        let mut track = Track::new("V1", TrackType::Video);
        let clip = Clip::new("m_test", 0.0, 5.0);
        let orig_cl_id = clip.id.clone();
        let orig_tr_id = track.id.clone();
        track.add_clip(clip);
        app.project.timeline.add_track(track);
        app.target_video_track = orig_tr_id.clone();
        app.current_time = 2.0;

        // Test click: creates a dedicated text clip on an upper track (V2)
        app.handle_monitor_text_click(0.35, 0.45);

        // Ensure original video clip is untouched
        let orig_tr = app.project.timeline.get_track_by_id(&orig_tr_id).unwrap();
        let orig_cl = orig_tr.get_clip_by_id(&orig_cl_id).unwrap();
        assert!(
            orig_cl.text_overlay.is_none(),
            "Original video clip should not be modified"
        );

        // Ensure newly created text clip is on the timeline
        let (selected_tr_id, selected_cl_id) = app
            .canvas_state
            .selected_clip_pairs
            .first()
            .cloned()
            .unwrap();
        let text_tr = app
            .project
            .timeline
            .get_track_by_id(&selected_tr_id)
            .unwrap();
        let text_cl = text_tr.get_clip_by_id(&selected_cl_id).unwrap();

        assert_eq!(text_cl.timeline_in, 2.0);
        assert_eq!(text_cl.timeline_out, 7.0);
        assert_eq!(text_cl.duration(), 5.0);
        let ov = text_cl
            .text_overlay
            .as_ref()
            .expect("text_overlay should be present on text clip");
        assert_eq!(ov.pos_x, 0.35);
        assert_eq!(ov.pos_y, 0.45);
        assert!(app.project.is_dirty);

        // Test drag: updates pos
        app.handle_monitor_text_drag(0.55, 0.75);
        let text_tr = app
            .project
            .timeline
            .get_track_by_id(&selected_tr_id)
            .unwrap();
        let text_cl = text_tr.get_clip_by_id(&selected_cl_id).unwrap();
        let ov = text_cl.text_overlay.as_ref().unwrap();
        assert_eq!(ov.pos_x, 0.55);
        assert_eq!(ov.pos_y, 0.75);

        // Test content change: updates text inline and updates clip name on timeline
        app.handle_monitor_text_content_changed("Alt Başlık Testi".to_string());
        let text_tr = app
            .project
            .timeline
            .get_track_by_id(&selected_tr_id)
            .unwrap();
        let text_cl = text_tr.get_clip_by_id(&selected_cl_id).unwrap();
        let ov = text_cl.text_overlay.as_ref().unwrap();
        assert_eq!(ov.content, "Alt Başlık Testi");
        assert_eq!(text_cl.name, "Metin: Alt Başlık Testi");

        // Test clicking near existing text overlay selects it instead of creating duplicates
        app.canvas_state.selected_clip_pairs.clear();
        app.handle_monitor_text_click(0.55, 0.75);
        assert_eq!(
            app.canvas_state.selected_clip_pairs,
            vec![(selected_tr_id, selected_cl_id)]
        );
    }

    #[test]
    fn test_edit_tools_and_shortcuts_mapping() {
        use crate::toolbar::EditTool;
        let mut app = TrimApp::default();
        assert_eq!(app.current_edit_tool, EditTool::Select);

        app.current_edit_tool = EditTool::Type;
        app.canvas_state.active_tool = EditTool::Type;
        assert_eq!(app.current_edit_tool, EditTool::Type);
        assert_eq!(app.canvas_state.active_tool, EditTool::Type);

        app.current_edit_tool = EditTool::Slip;
        app.canvas_state.active_tool = EditTool::Slip;
        assert_eq!(app.current_edit_tool, EditTool::Slip);

        app.current_edit_tool = EditTool::Pen;
        app.canvas_state.active_tool = EditTool::Pen;
        assert_eq!(app.current_edit_tool, EditTool::Pen);

        app.current_edit_tool = EditTool::Hand;
        app.canvas_state.active_tool = EditTool::Hand;
        assert_eq!(app.current_edit_tool, EditTool::Hand);
    }

    #[test]
    fn test_resolve_default_target_tracks_behavior() {
        let mut app = TrimApp::default();
        // Default project has V1 and A1
        app.resolve_default_target_tracks();
        assert_eq!(
            app.project
                .timeline
                .get_track_by_id(&app.target_video_track)
                .unwrap()
                .name,
            "V1"
        );
        assert_eq!(
            app.project
                .timeline
                .get_track_by_id(&app.target_audio_track)
                .unwrap()
                .name,
            "A1"
        );

        // Rename tracks
        for t in app.project.timeline.get_video_tracks_mut() {
            t.name = "CustomVideo".to_string();
        }
        for t in app.project.timeline.get_audio_tracks_mut() {
            t.name = "CustomAudio".to_string();
        }
        // Now resolve should fallback to the first video and audio tracks
        app.resolve_default_target_tracks();
        assert_eq!(
            app.project
                .timeline
                .get_track_by_id(&app.target_video_track)
                .unwrap()
                .name,
            "CustomVideo"
        );
        assert_eq!(
            app.project
                .timeline
                .get_track_by_id(&app.target_audio_track)
                .unwrap()
                .name,
            "CustomAudio"
        );

        // If tracks are empty
        app.project.timeline.tracks.clear();
        app.resolve_default_target_tracks();
        assert_eq!(app.target_video_track, "");
        assert_eq!(app.target_audio_track, "");
    }

    #[test]
    fn test_file_dialog_bridge_async_lifecycle_and_duplicate_guard() {
        use std::path::PathBuf;
        use std::sync::{Arc, Barrier};

        let mut bridge = FileDialogBridge::new();
        assert!(!bridge.is_pending());
        assert!(bridge.poll().is_none());

        let barrier = Arc::new(Barrier::new(2));
        let b_clone = Arc::clone(&barrier);

        let started = bridge.open_with_runner(FileDialogRequest::OpenProject, move || {
            b_clone.wait();
            Some(PathBuf::from("/tmp/test_project.trim"))
        });
        assert!(started);
        assert!(bridge.is_pending());

        // Duplicate invocation while pending must be rejected
        let duplicate = bridge.open_with_runner(FileDialogRequest::ImportMedia, || {
            Some(PathBuf::from("/tmp/other.mp4"))
        });
        assert!(!duplicate);

        // Release worker thread
        barrier.wait();

        let mut received = None;
        for _ in 0..100 {
            if let Some(resp) = bridge.poll() {
                received = Some(resp);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let resp = received.expect("Should receive FileDialogResponse from background thread");
        assert_eq!(resp.request, FileDialogRequest::OpenProject);
        assert_eq!(resp.path, Some(PathBuf::from("/tmp/test_project.trim")));
        assert!(!bridge.is_pending());
    }

    #[test]
    fn test_jkl_shuttle_and_snap_toggle() {
        let mut app = TrimApp::default();
        assert!(!app.is_playing);
        assert_eq!(app.playback_speed, 1.0);

        // Consecutive L presses: 1x -> 2x -> 4x -> 8x -> 8x
        app.shuttle_forward();
        assert!(app.is_playing);
        assert_eq!(app.playback_speed, 1.0);

        app.shuttle_forward();
        assert!(app.is_playing);
        assert_eq!(app.playback_speed, 2.0);

        app.shuttle_forward();
        assert_eq!(app.playback_speed, 4.0);

        app.shuttle_forward();
        assert_eq!(app.playback_speed, 8.0);

        app.shuttle_forward();
        assert_eq!(app.playback_speed, 8.0);

        // Consecutive J presses: -1x -> -2x -> -4x -> -8x -> -8x
        app.shuttle_reverse();
        assert!(app.is_playing);
        assert_eq!(app.playback_speed, -1.0);

        app.shuttle_reverse();
        assert_eq!(app.playback_speed, -2.0);

        app.shuttle_reverse();
        assert_eq!(app.playback_speed, -4.0);

        app.shuttle_reverse();
        assert_eq!(app.playback_speed, -8.0);

        app.shuttle_reverse();
        assert_eq!(app.playback_speed, -8.0);

        // K (shuttle_stop) pauses immediately and resets speed to 1.0
        app.shuttle_stop();
        assert!(!app.is_playing);
        assert_eq!(app.playback_speed, 1.0);

        // Space (toggle_play) plays at 1.0x and pauses back
        app.playback_speed = 4.0;
        app.toggle_play();
        assert!(app.is_playing);
        assert_eq!(app.playback_speed, 1.0);
        app.toggle_play();
        assert!(!app.is_playing);
        assert_eq!(app.playback_speed, 1.0);

        // Magnetic snapping toggle synchronizes TrimApp and TimelineModel
        assert!(app.snap_enabled);
        assert!(app.project.timeline.snap_enabled);
        app.toggle_snap();
        assert!(!app.snap_enabled);
        assert!(!app.project.timeline.snap_enabled);
        app.toggle_snap();
        assert!(app.snap_enabled);
        assert!(app.project.timeline.snap_enabled);
    }

    #[test]
    fn test_media_bin_bins_and_filters_navigation() {
        use crate::media_bin::MediaFilterType;
        use trim_core::media::{MediaItem, MediaType};

        let mut app = TrimApp::default();
        assert!(app.media_bin_current_bin.is_none());
        assert_eq!(app.media_bin_filter, MediaFilterType::All);

        // Add bins
        let bin1 = app.project.add_bin("Röportajlar");
        let bin2 = app.project.add_bin("Müzikler");
        assert_eq!(app.project.bins.len(), 2);

        // Add media items
        let video_item = MediaItem {
            id: "v1".to_string(),
            name: "interview.mp4".to_string(),
            media_type: MediaType::Video,
            bin_id: Some(bin1.clone()),
            ..Default::default()
        };
        app.project.add_media(video_item);

        let audio_item = MediaItem {
            id: "a1".to_string(),
            name: "bgm.mp3".to_string(),
            media_type: MediaType::Audio,
            bin_id: Some(bin2.clone()),
            ..Default::default()
        };
        app.project.add_media(audio_item);

        let root_img = MediaItem {
            id: "img1".to_string(),
            name: "logo.png".to_string(),
            media_type: MediaType::Image,
            bin_id: None,
            ..Default::default()
        };
        app.project.add_media(root_img);

        // Navigation into bin1
        app.media_bin_current_bin = Some(bin1.clone());
        assert_eq!(app.media_bin_current_bin.as_deref(), Some(bin1.as_str()));

        // Move root_img into bin1
        if let Some(item) = app.project.get_media_mut("img1") {
            item.bin_id = Some(bin1.clone());
        }
        assert_eq!(
            app.project.get_media("img1").unwrap().bin_id.as_deref(),
            Some(bin1.as_str())
        );

        // Filter chips switch
        app.media_bin_filter = MediaFilterType::Video;
        assert_eq!(app.media_bin_filter, MediaFilterType::Video);

        // Removing bin1 moves contained items (v1, img1) back to root
        app.project.remove_bin(&bin1);
        if app.media_bin_current_bin.as_deref() == Some(&bin1) {
            app.media_bin_current_bin = None;
        }
        assert!(app.media_bin_current_bin.is_none());
        assert_eq!(app.project.bins.len(), 1);
        assert_eq!(app.project.get_media("v1").unwrap().bin_id, None);
        assert_eq!(app.project.get_media("img1").unwrap().bin_id, None);
    }

    #[test]
    fn test_drop_media_v1_and_v2_corresponding_audio_tracks() {
        let mut app = TrimApp::default();
        // Add a mock video with audio channels
        let mut video_media = trim_core::media::MediaItem::new("/mock/video.mp4");
        video_media.id = "vid_1".to_string();
        video_media.name = "My Video".to_string();
        video_media.media_type = trim_core::media::MediaType::Video;
        video_media.audio_channels = 2;
        video_media.duration = 10.0;
        app.project.add_media(video_media);

        let v1_id = app
            .project
            .timeline
            .tracks
            .iter()
            .find(|t| t.name == "V1")
            .unwrap()
            .id
            .clone();
        let v2_id = app
            .project
            .timeline
            .tracks
            .iter()
            .find(|t| t.name == "V2")
            .unwrap()
            .id
            .clone();

        // 1. Drop onto V1: audio clip should end up in A1
        let a1_id_expected = app
            .project
            .timeline
            .tracks
            .iter()
            .find(|t| t.name == "A1")
            .unwrap()
            .id
            .clone();
        let a1_id = app
            .project
            .timeline
            .find_or_create_corresponding_audio_track(&v1_id);
        assert_eq!(a1_id, a1_id_expected);

        // 2. Drop onto V2: audio clip should end up in A2
        let a2_id_expected = app
            .project
            .timeline
            .tracks
            .iter()
            .find(|t| t.name == "A2")
            .unwrap()
            .id
            .clone();
        let a2_id = app
            .project
            .timeline
            .find_or_create_corresponding_audio_track(&v2_id);
        assert_eq!(a2_id, a2_id_expected);

        // 3. Drop onto V3 (non-existent yet): creates A3
        let v3_id = app.project.timeline.add_new_video_track();
        let a3_id = app
            .project
            .timeline
            .find_or_create_corresponding_audio_track(&v3_id);
        assert_eq!(
            app.project.timeline.get_track_by_id(&a3_id).unwrap().name,
            "A3"
        );
    }

    #[test]
    fn test_drop_media_atomic_undo() {
        let mut app = TrimApp::default();
        let mut video_media = trim_core::media::MediaItem::new("/mock/video.mp4");
        video_media.id = "vid_1".to_string();
        video_media.name = "My Video".to_string();
        video_media.media_type = trim_core::media::MediaType::Video;
        video_media.audio_channels = 2;
        video_media.duration = 10.0;
        app.project.add_media(video_media);

        let v1_id = app
            .project
            .timeline
            .tracks
            .iter()
            .find(|t| t.name == "V1")
            .unwrap()
            .id
            .clone();
        let a1_id = app
            .project
            .timeline
            .tracks
            .iter()
            .find(|t| t.name == "A1")
            .unwrap()
            .id
            .clone();

        app.target_video_track = v1_id.clone();
        app.target_audio_track = a1_id.clone();

        // Insert clip onto timeline from source monitor
        app.source_state.media_id = Some("vid_1".to_string());
        app.insert_source_to_timeline(false);

        assert_eq!(
            app.project
                .timeline
                .get_track_by_id(&v1_id)
                .unwrap()
                .clips
                .len(),
            1
        );
        assert_eq!(
            app.project
                .timeline
                .get_track_by_id(&a1_id)
                .unwrap()
                .clips
                .len(),
            1
        );

        // A single undo must atomically remove BOTH video and audio clips!
        assert!(app.undo_stack.undo(&mut app.project.timeline).unwrap());
        assert_eq!(
            app.project
                .timeline
                .get_track_by_id(&v1_id)
                .unwrap()
                .clips
                .len(),
            0
        );
        assert_eq!(
            app.project
                .timeline
                .get_track_by_id(&a1_id)
                .unwrap()
                .clips
                .len(),
            0
        );

        // A single redo must atomically restore BOTH video and audio clips!
        assert!(app.undo_stack.redo(&mut app.project.timeline).unwrap());
        assert_eq!(
            app.project
                .timeline
                .get_track_by_id(&v1_id)
                .unwrap()
                .clips
                .len(),
            1
        );
        assert_eq!(
            app.project
                .timeline
                .get_track_by_id(&a1_id)
                .unwrap()
                .clips
                .len(),
            1
        );
    }

    #[test]
    fn test_synchronized_follower_scroll() {
        let ctx = egui::Context::default();
        let mut canvas_offset = Vec2::ZERO;
        let canvas_id = egui::Id::new("canvas_scroll");
        let mut state = egui::scroll_area::State::load(&ctx, canvas_id).unwrap_or_default();
        state.offset.y = 50.0;
        state.store(&ctx, canvas_id);

        // Frame 1: Initial render
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                // Ruler (follower X)
                let ruler_out = egui::ScrollArea::horizontal()
                    .id_salt("ruler_scroll")
                    .horizontal_scroll_offset(canvas_offset.x)
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        ui.allocate_exact_size(Vec2::new(1000.0, 26.0), egui::Sense::hover());
                    });
                assert_eq!(ruler_out.state.offset.x, canvas_offset.x);

                // Track Headers (follower Y)
                let headers_out = egui::ScrollArea::vertical()
                    .id_salt("headers_scroll")
                    .vertical_scroll_offset(canvas_offset.y)
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        ui.allocate_exact_size(Vec2::new(216.0, 1000.0), egui::Sense::hover());
                    });
                assert_eq!(headers_out.state.offset.y, canvas_offset.y);

                // Canvas (driver X & Y)
                let canvas_out = egui::ScrollArea::both()
                    .id_salt("canvas_scroll")
                    .max_width(400.0)
                    .max_height(300.0)
                    .show(ui, |ui| {
                        ui.allocate_exact_size(Vec2::new(1000.0, 1000.0), egui::Sense::hover());
                    });
                canvas_offset = canvas_out.state.offset;
            });
        });

        // Verify initial state
        assert_eq!(canvas_offset, Vec2::ZERO);

        // Frame 2: Suppose canvas scrolled to (120.0, 75.0)
        canvas_offset = Vec2::new(120.0, 75.0);

        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                // Ruler receives canvas X offset
                let ruler_out = egui::ScrollArea::horizontal()
                    .id_salt("ruler_scroll")
                    .max_width(400.0)
                    .horizontal_scroll_offset(canvas_offset.x)
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        ui.allocate_exact_size(Vec2::new(1000.0, 26.0), egui::Sense::hover());
                    });
                assert_eq!(ruler_out.state.offset.x, 120.0);

                // Track Headers receives canvas Y offset
                let headers_out = egui::ScrollArea::vertical()
                    .id_salt("headers_scroll")
                    .max_height(300.0)
                    .vertical_scroll_offset(canvas_offset.y)
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        ui.allocate_exact_size(Vec2::new(216.0, 1000.0), egui::Sense::hover());
                    });
                assert_eq!(headers_out.state.offset.y, 75.0);
            });
        });

        // Frame 3: Mouse wheel over track headers (pos = 50, 50)
        let mut raw_input = egui::RawInput::default();
        raw_input
            .events
            .push(egui::Event::PointerMoved(egui::Pos2::new(50.0, 50.0)));
        raw_input.events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(0.0, -30.0), // scroll down 30px
            modifiers: Default::default(),
        });

        let _ = ctx.run(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _headers_resp = ui.scope(|ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("headers_scroll")
                        .max_height(300.0)
                        .vertical_scroll_offset(canvas_offset.y)
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .show(ui, |ui| {
                            ui.allocate_exact_size(Vec2::new(216.0, 1000.0), egui::Sense::hover());
                        })
                });

                // Check if user scrolled over headers
                if ui
                    .input(|i| i.pointer.hover_pos())
                    .map(|p| p.x <= 216.0)
                    .unwrap_or(false)
                {
                    let scroll_delta = ui.input(|i| i.raw_scroll_delta);
                    if scroll_delta.y != 0.0 {
                        canvas_offset.y = (canvas_offset.y - scroll_delta.y).clamp(0.0, 700.0);
                    }
                }
            });
        });

        assert_eq!(canvas_offset.y, 105.0);
    }

    #[test]
    fn test_copy_and_paste_clips_with_undo() {
        let mut app = app::TrimApp::default();
        let v1_id = app.target_video_track.clone();

        // Add a sample clip to V1
        let mut clip = trim_core::clip::Clip::new("m1", 0.0, 5.0);
        clip.id = "clip_original".to_string();
        clip.source_in = 1.0;
        app.project.timeline.get_track_by_id_mut(&v1_id).unwrap().add_clip(clip);

        assert_eq!(app.project.timeline.get_track_by_id(&v1_id).unwrap().clips.len(), 1);

        // 1. Copy when nothing is selected
        app.copy_selected_clips();
        assert!(app.clipboard_clips.is_empty());

        // 2. Select and copy clip
        app.canvas_state.selected_clip_pairs = vec![(v1_id.clone(), "clip_original".to_string())];
        app.copy_selected_clips();
        assert_eq!(app.clipboard_clips.len(), 1);

        // 3. Paste at time = 10.0
        app.current_time = 10.0;
        app.paste_clipboard_clips();

        let track = app.project.timeline.get_track_by_id(&v1_id).unwrap();
        assert_eq!(track.clips.len(), 2);
        let pasted = track.clips.iter().find(|c| c.id != "clip_original").unwrap();
        assert_eq!(pasted.timeline_in, 10.0);
        assert_eq!(pasted.timeline_out, 15.0);
        assert_eq!(pasted.source_in, 1.0);
        assert_eq!(app.canvas_state.selected_clip_pairs.len(), 1);
        assert_eq!(app.canvas_state.selected_clip_pairs[0].1, pasted.id);
        assert_eq!(app.current_time, 15.0);

        // 4. Undo paste
        assert!(app.undo_stack.can_undo());
        let _ = app.undo_stack.undo(&mut app.project.timeline);
        assert_eq!(app.project.timeline.get_track_by_id(&v1_id).unwrap().clips.len(), 1);

        // 5. Redo paste
        assert!(app.undo_stack.can_redo());
        let _ = app.undo_stack.redo(&mut app.project.timeline);
        assert_eq!(app.project.timeline.get_track_by_id(&v1_id).unwrap().clips.len(), 2);
    }

    #[test]
    fn test_slip_edit_tool() {
        use crate::canvas::{find_clip_at_pos, DragMode};
        use crate::toolbar::EditTool;
        use trim_commands::commands::TrimClipCommand;
        use trim_core::clip::Clip;

        let mut app = TrimApp::default();
        let v1_id = app.project.timeline.get_video_tracks()[0].id.clone();
        let mut clip = Clip::new("media_slip", 2.0, 7.0); // 5s duration
        clip.source_in = 3.0;
        clip.source_out = Some(8.0);
        let clip_id = clip.id.clone();
        app.project.timeline.get_track_by_id_mut(&v1_id).unwrap().add_clip(clip);

        // 1. Hit test with Select tool vs Slip tool
        app.canvas_state.active_tool = EditTool::Select;
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(1000.0, 500.0));
        let pos = egui::Pos2::new(
            (2.0 + 2.5) * app.canvas_state.pixels_per_second, // middle of clip
            20.0,
        );
        let res_select = find_clip_at_pos(&app.project, &app.canvas_state, rect, pos);
        assert_eq!(res_select.unwrap().2, DragMode::Move);

        app.canvas_state.active_tool = EditTool::Slip;
        let res_slip = find_clip_at_pos(&app.project, &app.canvas_state, rect, pos);
        assert_eq!(res_slip.unwrap().2, DragMode::Slip);

        // 2. Slip operation: timeline_in and timeline_out stay fixed (2.0 to 7.0)
        // Shifting source window from 3.0..8.0 to 5.0..10.0
        let old_bounds = (2.0, 7.0, 3.0, 8.0);
        let new_bounds = (2.0, 7.0, 5.0, 10.0);
        let cmd = Box::new(TrimClipCommand::new(&v1_id, &clip_id, old_bounds, new_bounds));
        app.undo_stack.push(cmd, &mut app.project.timeline).unwrap();

        let slipped_clip = app.project.timeline.get_track_by_id(&v1_id).unwrap().get_clip_by_id(&clip_id).unwrap();
        assert_eq!(slipped_clip.timeline_in, 2.0);
        assert_eq!(slipped_clip.timeline_out, 7.0);
        assert_eq!(slipped_clip.source_in, 5.0);
        assert_eq!(slipped_clip.source_out, Some(10.0));

        // 3. Undo Slip
        app.undo_stack.undo(&mut app.project.timeline).unwrap();
        let undone_clip = app.project.timeline.get_track_by_id(&v1_id).unwrap().get_clip_by_id(&clip_id).unwrap();
        assert_eq!(undone_clip.timeline_in, 2.0);
        assert_eq!(undone_clip.timeline_out, 7.0);
        assert_eq!(undone_clip.source_in, 3.0);
        assert_eq!(undone_clip.source_out, Some(8.0));

        // 4. Redo Slip
        app.undo_stack.redo(&mut app.project.timeline).unwrap();
        let redone_clip = app.project.timeline.get_track_by_id(&v1_id).unwrap().get_clip_by_id(&clip_id).unwrap();
        assert_eq!(redone_clip.source_in, 5.0);
        assert_eq!(redone_clip.source_out, Some(10.0));
    }

    #[test]
    fn test_relink_dialog_and_relink_actions() {
        use crate::dialogs::relink_dialog::RelinkDialogAction;
        use trim_core::media::MediaItem;

        let temp_dir = tempfile::tempdir().unwrap();
        let intro_file = temp_dir.path().join("camera_intro.mp4");
        std::fs::write(&intro_file, b"sample raw footage").unwrap();

        let mut app = TrimApp::default();
        let missing_item = MediaItem {
            id: "m_offline_1".to_string(),
            name: "camera_intro.mp4".to_string(),
            file_path: "/missing/folder/camera_intro.mp4".to_string(),
            ..Default::default()
        };
        app.project.add_media(missing_item);

        // Open relink dialog
        app.relink_dialog.open(&app.project);
        assert!(app.relink_dialog.is_open);
        assert_eq!(app.relink_dialog.offline_items.len(), 1);
        assert_eq!(app.relink_dialog.offline_items[0].media_id, "m_offline_1");

        // Relink from folder
        let relinked_count = app.project.relink_media_from_folder(temp_dir.path());
        assert_eq!(relinked_count, 1);

        // Refresh dialog
        app.relink_dialog.refresh(&app.project);
        assert!(app.relink_dialog.offline_items.is_empty());

        // Test rendering UI with egui context
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            let action = app.relink_dialog.show(ctx);
            assert!(action.is_none() || matches!(action, Some(RelinkDialogAction::Close)));
        });
    }
}

