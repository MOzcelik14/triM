//! triM. User Interface components and main application.

pub mod app;
pub mod canvas;
pub mod dialogs;
pub mod inspector;
pub mod media_bin;
pub mod monitor;
pub mod ruler;
pub mod theme;
pub mod toolbar;
pub mod track_header;
pub mod transport;
pub mod vu_meter;

pub use app::TrimApp;
pub use canvas::TimelineCanvasState;
pub use media_bin::LeftPanelTab;
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
                            let (resp, _) = track_header::render_track_header(ui, track, canvas_state.track_height, false);
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
}


