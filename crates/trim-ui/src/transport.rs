//! Transport controls bar: Play/Pause, Frame Step, Shuttle indicator and Timecode.

use egui::{Button, Color32, RichText, Ui};
use trim_core::time::seconds_to_timecode;

pub enum TransportAction {
    TogglePlay,
    Stop,
    StepFrame(i32),
}

pub fn render_transport_bar(
    ui: &mut Ui,
    current_time: f64,
    total_duration: f64,
    fps: f64,
    is_playing: bool,
    playback_speed: f64,
) -> Option<TransportAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        // Stop button
        if ui.button(RichText::new("⏹").size(14.0)).clicked() {
            action = Some(TransportAction::Stop);
        }

        // Previous frame
        if ui.button(RichText::new("⏴").size(14.0)).clicked() {
            action = Some(TransportAction::StepFrame(-1));
        }

        // Play / Pause button
        let play_icon = if is_playing { "⏸" } else { "▶" };
        let play_btn = Button::new(RichText::new(play_icon).size(16.0).color(Color32::WHITE))
            .fill(Color32::from_rgb(0xe0, 0x7a, 0x38));

        if ui.add(play_btn).clicked() {
            action = Some(TransportAction::TogglePlay);
        }

        // Next frame
        if ui.button(RichText::new("⏵").size(14.0)).clicked() {
            action = Some(TransportAction::StepFrame(1));
        }

        ui.separator();

        // Timecode display
        let tc_cur = seconds_to_timecode(current_time, fps);
        let tc_dur = seconds_to_timecode(total_duration, fps);

        ui.label(
            RichText::new(format!("{} / {}", tc_cur, tc_dur))
                .monospace()
                .size(13.0)
                .color(Color32::from_rgb(0xe0, 0xe0, 0xeb)),
        );

        // Shuttle speed indicator
        if (playback_speed - 1.0).abs() > 0.01 {
            let speed_txt = if playback_speed > 1.0 {
                format!(">> {:.0}x", playback_speed)
            } else if playback_speed < 0.0 {
                format!("<< {:.0}x", playback_speed.abs())
            } else {
                format!("{:.1}x", playback_speed)
            };

            ui.label(
                RichText::new(speed_txt)
                    .color(Color32::from_rgb(0xe0, 0x7a, 0x38))
                    .strong(),
            );
        }
    });

    action
}
