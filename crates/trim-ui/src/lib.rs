//! triM. User Interface components and main application.

pub mod app;
pub mod canvas;
pub mod dialogs;
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

                println!("Canvas track_height = {}, track_spacing = {}", canvas_state.track_height, canvas_state.track_spacing);

                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let header_w = 130.0;

                ui.allocate_ui(Vec2::new(header_w, 600.0), |ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        let (corner_rect, _) = ui.allocate_exact_size(Vec2::new(header_w, 26.0), egui::Sense::hover());
                        println!("Corner rect: top = {}, bottom = {}, height = {}", corner_rect.top(), corner_rect.bottom(), corner_rect.height());

                        for (idx, track) in project.timeline.tracks.iter().enumerate() {
                            let (resp, _) = track_header::render_track_header(ui, track, canvas_state.track_height, false, false);
                            println!("Track {} ({}) header rect: top = {}, bottom = {}, height = {}", idx, track.name, resp.rect.top(), resp.rect.bottom(), resp.rect.height());
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
        let welcome = WelcomeDialog::default();
        assert!(welcome.is_open);
        assert_eq!(welcome.new_project_name, "Yeni Proje");

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
        dialog.prompt(PendingCloseAction::OpenPath(test_path.clone()), "Another Project");
        assert!(dialog.is_open);
        assert_eq!(dialog.pending_action, Some(PendingCloseAction::OpenPath(test_path)));
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
}




