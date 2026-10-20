//! Multitrack Audio Mixer console modal with vertical channel faders, pan, and VU meters.

use egui::{Button, Color32, ProgressBar, RichText, Slider, Vec2, Window};
use trim_core::project::Project;

#[derive(Debug, Clone)]
pub struct AudioMixerDialog {
    pub is_open: bool,
    pub master_volume: f32, // 0.0 .. 2.0 (1.0 = 0dB)
}

impl Default for AudioMixerDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            master_volume: 1.0,
        }
    }
}

pub enum AudioMixerAction {
    TrackVolumeChanged(String, f64),
    TrackMuteToggled(String),
    TrackSoloToggled(String),
    MasterVolumeChanged(f32),
}

impl AudioMixerDialog {
    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        project: &Project,
        vu_level_l: f32,
        vu_level_r: f32,
    ) -> Vec<AudioMixerAction> {
        if !self.is_open {
            return Vec::new();
        }

        let mut actions = Vec::new();

        Window::new("Çok Kanallı Ses Mikseri")
            .collapsible(true)
            .resizable(true)
            .default_size(Vec2::new(520.0, 360.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Ses Kanalları").size(13.0).strong().color(Color32::from_rgb(0x3d, 0xdc, 0x84)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Kapat").clicked() {
                            self.is_open = false;
                        }
                    });
                });
                ui.separator();
                ui.add_space(4.0);

                let audio_tracks = project.timeline.get_audio_tracks();

                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;

                        // Render each audio track channel strip
                        for track in &audio_tracks {
                            ui.group(|ui| {
                                ui.set_width(76.0);
                                ui.vertical_centered(|ui| {
                                    // Track Name
                                    ui.label(
                                        RichText::new(&track.name)
                                            .size(12.0)
                                            .strong()
                                            .color(Color32::from_rgb(0x3d, 0xdc, 0x84)),
                                    );
                                    ui.add_space(2.0);

                                    // Mute and Solo buttons
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 2.0;

                                        let mute_btn = Button::new(
                                            RichText::new("M")
                                                .size(10.0)
                                                .strong()
                                                .color(if track.muted { Color32::WHITE } else { Color32::from_rgb(0x90, 0x90, 0xa0) }),
                                        )
                                        .min_size(Vec2::new(28.0, 20.0))
                                        .fill(if track.muted { Color32::from_rgb(0xdc, 0x35, 0x45) } else { Color32::from_rgb(0x22, 0x22, 0x2a) });

                                        if ui.add(mute_btn).on_hover_text("Sessize Al (Mute)").clicked() {
                                            actions.push(AudioMixerAction::TrackMuteToggled(track.id.clone()));
                                        }

                                        let solo_btn = Button::new(
                                            RichText::new("S")
                                                .size(10.0)
                                                .strong()
                                                .color(if track.solo { Color32::WHITE } else { Color32::from_rgb(0x90, 0x90, 0xa0) }),
                                        )
                                        .min_size(Vec2::new(28.0, 20.0))
                                        .fill(if track.solo { Color32::from_rgb(0xe0, 0x9a, 0x24) } else { Color32::from_rgb(0x22, 0x22, 0x2a) });

                                        if ui.add(solo_btn).on_hover_text("Solo").clicked() {
                                            actions.push(AudioMixerAction::TrackSoloToggled(track.id.clone()));
                                        }
                                    });

                                    ui.add_space(6.0);

                                    // Level display in dB
                                    let db_val = if track.volume <= 0.0001 {
                                        "-inf dB".to_string()
                                    } else {
                                        format!("{:.1} dB", 20.0 * track.volume.log10())
                                    };
                                    ui.label(RichText::new(db_val).size(10.0).color(Color32::from_rgb(0xa0, 0xa0, 0xb4)));

                                    ui.add_space(4.0);

                                    // Fader and VU Meter side-by-side
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;

                                        // Vertical volume slider
                                        let mut vol = track.volume as f32;
                                        let fader = Slider::new(&mut vol, 0.0..=2.0)
                                            .vertical()
                                            .show_value(false);

                                        if ui.add_sized([26.0, 160.0], fader).changed() {
                                            actions.push(AudioMixerAction::TrackVolumeChanged(track.id.clone(), vol as f64));
                                        }

                                        // Mini track VU meter bar
                                        let trk_vu = if track.muted { 0.0 } else { ((vu_level_l + vu_level_r) / 2.0).clamp(0.0, 1.0) };
                                        let vu_bar = ProgressBar::new(trk_vu)
                                            .show_percentage()
                                            .fill(if trk_vu > 0.85 { Color32::from_rgb(0xfa, 0x4d, 0x56) } else { Color32::from_rgb(0x24, 0xa1, 0x48) });
                                        ui.add_sized([10.0, 160.0], vu_bar);
                                    });

                                    ui.add_space(4.0);
                                    ui.label(RichText::new(format!("{:.0}%", track.volume * 100.0)).size(10.0).color(Color32::from_rgb(0xc0, 0xc0, 0xd0)));
                                });
                            });
                        }

                        ui.separator();

                        // Master Bus Channel Strip
                        ui.group(|ui| {
                            ui.set_width(84.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new("MASTER")
                                        .size(12.0)
                                        .strong()
                                        .color(Color32::from_rgb(0x38, 0xa0, 0xf0)),
                                );
                                ui.add_space(2.0);

                                let master_db = if self.master_volume <= 0.0001 {
                                    "-inf dB".to_string()
                                } else {
                                    format!("{:.1} dB", 20.0 * (self.master_volume as f64).log10())
                                };
                                ui.label(RichText::new(master_db).size(10.0).color(Color32::from_rgb(0xa0, 0xa0, 0xb4)));

                                ui.add_space(26.0);

                                // Master Fader and Dual Stereo VU meters
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;

                                    let fader = Slider::new(&mut self.master_volume, 0.0..=2.0)
                                        .vertical()
                                        .show_value(false);

                                    if ui.add_sized([28.0, 160.0], fader).changed() {
                                        actions.push(AudioMixerAction::MasterVolumeChanged(self.master_volume));
                                    }

                                    // L Channel VU
                                    let vu_l = ProgressBar::new(vu_level_l.clamp(0.0, 1.0))
                                        .fill(if vu_level_l > 0.85 { Color32::from_rgb(0xfa, 0x4d, 0x56) } else { Color32::from_rgb(0x24, 0xa1, 0x48) });
                                    ui.add_sized([8.0, 160.0], vu_l);

                                    // R Channel VU
                                    let vu_r = ProgressBar::new(vu_level_r.clamp(0.0, 1.0))
                                        .fill(if vu_level_r > 0.85 { Color32::from_rgb(0xfa, 0x4d, 0x56) } else { Color32::from_rgb(0x24, 0xa1, 0x48) });
                                    ui.add_sized([8.0, 160.0], vu_r);
                                });

                                ui.add_space(4.0);
                                ui.label(RichText::new(format!("{:.0}%", self.master_volume * 100.0)).size(10.0).color(Color32::from_rgb(0x38, 0xa0, 0xf0)).strong());
                            });
                        });
                    });
                });
            });

        actions
    }
}
