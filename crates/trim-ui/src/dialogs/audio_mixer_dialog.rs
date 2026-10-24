//! Studio-grade Multitrack Audio Mixer console modal with vertical channel faders,
//! segmented LED VU meters, dB readouts, Mute/Solo toggles, and Master Bus brickwall limiter indicator.

use egui::{
    Align, Align2, Button, Color32, FontId, Frame, Layout, Margin, Pos2, Rect, RichText, Rounding,
    ScrollArea, Sense, Shadow, Slider, Stroke, Ui, Vec2, Window,
};
use trim_core::project::Project;

#[derive(Debug, Clone)]
pub struct AudioMixerDialog {
    pub is_open: bool,
    pub master_volume: f32, // 0.0 .. 2.0 (1.0 = 0 dB)
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
        let audio_tracks = project.timeline.get_audio_tracks();
        let any_solo = audio_tracks.iter().any(|t| t.solo);

        let modal_frame = Frame::window(&ctx.style())
            .fill(Color32::from_rgb(0x14, 0x15, 0x1b))
            .rounding(Rounding::same(10.0))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x2c, 0x2f, 0x3d)))
            .inner_margin(Margin::symmetric(16.0, 14.0))
            .shadow(Shadow {
                offset: Vec2::new(0.0, 10.0),
                blur: 28.0,
                spread: 2.0,
                color: Color32::from_black_alpha(160),
            });

        Window::new("Çok Kanallı Ses Mikseri")
            .collapsible(false)
            .resizable(true)
            .default_size(Vec2::new(560.0, 410.0))
            .min_width(340.0)
            .frame(modal_frame)
            .show(ctx, |ui| {
                // Header Bar
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("🎛 Ses Kanalları & Master Bus")
                            .size(13.0)
                            .strong()
                            .color(Color32::from_rgb(0x3d, 0xdc, 0x84)),
                    );
                    ui.add_space(6.0);
                    Frame::none()
                        .fill(Color32::from_rgb(0x1d, 0x26, 0x22))
                        .rounding(Rounding::same(4.0))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x2d, 0x54, 0x3e)))
                        .inner_margin(Margin::symmetric(6.0, 2.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("48 kHz · Brickwall Limiter (-0.1 dBFS)")
                                    .size(9.5)
                                    .color(Color32::from_rgb(0x7e, 0xd9, 0xa6)),
                            );
                        });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let close_btn = Button::new(
                            RichText::new("✕ Kapat")
                                .size(11.0)
                                .color(Color32::from_rgb(0xd0, 0xd4, 0xe4)),
                        )
                        .rounding(Rounding::same(5.0))
                        .fill(Color32::from_rgb(0x23, 0x25, 0x30));
                        if ui.add(close_btn).clicked() {
                            self.is_open = false;
                        }
                    });
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(8.0);

                let strip_h = 324.0;
                let fader_h = 156.0;

                ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;

                        // =====================================================
                        // INDIVIDUAL AUDIO TRACK CHANNEL STRIPS
                        // =====================================================
                        for track in &audio_tracks {
                            let is_audible = !track.muted && (!any_solo || track.solo);
                            let strip_w = 106.0;

                            let strip_fill = if is_audible {
                                Color32::from_rgb(0x1a, 0x1c, 0x24)
                            } else {
                                Color32::from_rgb(0x15, 0x16, 0x1c)
                            };
                            let strip_border = if track.solo {
                                Color32::from_rgb(0xe0, 0x9a, 0x24)
                            } else if track.muted {
                                Color32::from_rgb(0x7a, 0x28, 0x30)
                            } else {
                                Color32::from_rgb(0x2b, 0x2e, 0x3c)
                            };

                            Frame::none()
                                .fill(strip_fill)
                                .rounding(Rounding::same(8.0))
                                .stroke(Stroke::new(1.0_f32, strip_border))
                                .inner_margin(Margin::symmetric(10.0, 10.0))
                                .show(ui, |ui| {
                                    ui.set_min_size(Vec2::new(strip_w, strip_h));
                                    ui.set_max_size(Vec2::new(strip_w, strip_h));

                                    ui.vertical_centered(|ui| {
                                        // Track Header Badge
                                        Frame::none()
                                            .fill(Color32::from_rgb(0x1e, 0x2b, 0x25))
                                            .rounding(Rounding::same(5.0))
                                            .inner_margin(Margin::symmetric(10.0, 3.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(&track.name)
                                                        .size(12.5)
                                                        .strong()
                                                        .color(Color32::from_rgb(0x3d, 0xdc, 0x84)),
                                                );
                                            });

                                        ui.add_space(6.0);

                                        // Mute (M) & Solo (S) Row
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 6.0;
                                            ui.add_space(10.0);

                                            let mute_btn = Button::new(
                                                RichText::new("M").size(10.5).strong().color(
                                                    if track.muted {
                                                        Color32::WHITE
                                                    } else {
                                                        Color32::from_rgb(0x98, 0x9c, 0xb0)
                                                    },
                                                ),
                                            )
                                            .min_size(Vec2::new(36.0, 22.0))
                                            .rounding(Rounding::same(4.0))
                                            .fill(if track.muted {
                                                Color32::from_rgb(0xdc, 0x35, 0x45)
                                            } else {
                                                Color32::from_rgb(0x23, 0x25, 0x30)
                                            });

                                            if ui
                                                .add(mute_btn)
                                                .on_hover_text("Sessize Al (Mute)")
                                                .clicked()
                                            {
                                                actions.push(AudioMixerAction::TrackMuteToggled(
                                                    track.id.clone(),
                                                ));
                                            }

                                            let solo_btn = Button::new(
                                                RichText::new("S").size(10.5).strong().color(
                                                    if track.solo {
                                                        Color32::WHITE
                                                    } else {
                                                        Color32::from_rgb(0x98, 0x9c, 0xb0)
                                                    },
                                                ),
                                            )
                                            .min_size(Vec2::new(36.0, 22.0))
                                            .rounding(Rounding::same(4.0))
                                            .fill(if track.solo {
                                                Color32::from_rgb(0xe0, 0x9a, 0x24)
                                            } else {
                                                Color32::from_rgb(0x23, 0x25, 0x30)
                                            });

                                            if ui
                                                .add(solo_btn)
                                                .on_hover_text("Yalnızca Bu Kanalı Dinle (Solo)")
                                                .clicked()
                                            {
                                                actions.push(AudioMixerAction::TrackSoloToggled(
                                                    track.id.clone(),
                                                ));
                                            }
                                        });

                                        ui.add_space(6.0);

                                        // Digital dB Readout Pill
                                        let db_str = format_gain_db(track.volume);
                                        render_db_readout_pill(
                                            ui,
                                            &db_str,
                                            Color32::from_rgb(0xd4, 0xd8, 0xe8),
                                        );

                                        ui.add_space(8.0);

                                        // Vertical Fader + Custom Segmented Vertical VU Meter
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 6.0;
                                            ui.add_space(12.0);

                                            let mut vol = track.volume as f32;
                                            ui.scope(|ui| {
                                                ui.spacing_mut().slider_width = fader_h;
                                                ui.spacing_mut().slider_rail_height = 6.0;
                                                let fader = Slider::new(&mut vol, 0.0..=2.0)
                                                    .vertical()
                                                    .show_value(false);
                                                let resp = ui.add(fader).on_hover_text(
                                                    "Ses seviyesi (Çift tıkla: 0.0 dB sıfırla)",
                                                );
                                                if resp.double_clicked() {
                                                    vol = 1.0;
                                                    actions.push(
                                                        AudioMixerAction::TrackVolumeChanged(
                                                            track.id.clone(),
                                                            1.0,
                                                        ),
                                                    );
                                                } else if resp.changed() {
                                                    actions.push(
                                                        AudioMixerAction::TrackVolumeChanged(
                                                            track.id.clone(),
                                                            vol as f64,
                                                        ),
                                                    );
                                                }
                                            });

                                            // Estimate track meter dB from master L/R & track gain
                                            let trk_db = if !is_audible || track.volume <= 0.0001 {
                                                -60.0
                                            } else {
                                                let avg_master = 0.5 * (vu_level_l + vu_level_r);
                                                if avg_master <= -59.5 {
                                                    -60.0
                                                } else {
                                                    let gain_db =
                                                        20.0 * (track.volume as f32).log10();
                                                    (avg_master + gain_db).clamp(-60.0, 0.0)
                                                }
                                            };

                                            draw_vertical_vu_strip(ui, &[trk_db], fader_h, true);
                                        });

                                        ui.add_space(8.0);

                                        // Gain Percentage + Unity Reset Button
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 6.0;
                                            ui.add_space(10.0);
                                            ui.label(
                                                RichText::new(format!(
                                                    "{:.0}%",
                                                    track.volume * 100.0
                                                ))
                                                .monospace()
                                                .size(10.5)
                                                .color(Color32::from_rgb(0xc4, 0xc8, 0xd8)),
                                            );
                                            if ui
                                                .add(
                                                    Button::new(
                                                        RichText::new("0 dB").size(9.5).color(
                                                            Color32::from_rgb(0x90, 0x96, 0xaa),
                                                        ),
                                                    )
                                                    .min_size(Vec2::new(34.0, 18.0))
                                                    .rounding(Rounding::same(3.0))
                                                    .fill(Color32::from_rgb(0x22, 0x24, 0x2e)),
                                                )
                                                .on_hover_text(
                                                    "Kazancı 0.0 dB (100%) değerine sıfırla",
                                                )
                                                .clicked()
                                            {
                                                actions.push(AudioMixerAction::TrackVolumeChanged(
                                                    track.id.clone(),
                                                    1.0,
                                                ));
                                            }
                                        });
                                    });
                                });
                        }

                        // =====================================================
                        // MASTER BUS CHANNEL STRIP
                        // =====================================================
                        let master_w = 132.0;
                        Frame::none()
                            .fill(Color32::from_rgb(0x18, 0x1d, 0x28))
                            .rounding(Rounding::same(8.0))
                            .stroke(Stroke::new(1.2_f32, Color32::from_rgb(0x38, 0x7a, 0xe0)))
                            .inner_margin(Margin::symmetric(12.0, 10.0))
                            .show(ui, |ui| {
                                ui.set_min_size(Vec2::new(master_w, strip_h));
                                ui.set_max_size(Vec2::new(master_w, strip_h));

                                ui.vertical_centered(|ui| {
                                    // Master Header Badge
                                    Frame::none()
                                        .fill(Color32::from_rgb(0x1d, 0x2e, 0x48))
                                        .rounding(Rounding::same(5.0))
                                        .inner_margin(Margin::symmetric(12.0, 3.0))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new("MASTER BUS")
                                                    .size(12.0)
                                                    .strong()
                                                    .color(Color32::from_rgb(0x58, 0xb4, 0xff)),
                                            );
                                        });

                                    ui.add_space(6.0);

                                    // Stereo L/R Status Pill
                                    Frame::none()
                                        .fill(Color32::from_rgb(0x1f, 0x24, 0x32))
                                        .rounding(Rounding::same(4.0))
                                        .inner_margin(Margin::symmetric(8.0, 3.0))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new("STEREO L / R")
                                                    .size(9.5)
                                                    .strong()
                                                    .color(Color32::from_rgb(0x8c, 0x9e, 0xbe)),
                                            );
                                        });

                                    ui.add_space(6.0);

                                    // Master Digital dB Readout Pill
                                    let master_db_str = format_gain_db(self.master_volume as f64);
                                    render_db_readout_pill(
                                        ui,
                                        &master_db_str,
                                        Color32::from_rgb(0x58, 0xb4, 0xff),
                                    );

                                    ui.add_space(8.0);

                                    // Master Fader + Dual Stereo (L/R) Vertical VU Meter
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 8.0;
                                        ui.add_space(16.0);

                                        ui.scope(|ui| {
                                            ui.spacing_mut().slider_width = fader_h;
                                            ui.spacing_mut().slider_rail_height = 6.0;
                                            let fader =
                                                Slider::new(&mut self.master_volume, 0.0..=2.0)
                                                    .vertical()
                                                    .show_value(false);
                                            let resp = ui.add(fader).on_hover_text(
                                                "Ana çıkış seviyesi (Çift tıkla: 0.0 dB sıfırla)",
                                            );
                                            if resp.double_clicked() {
                                                self.master_volume = 1.0;
                                                actions.push(
                                                    AudioMixerAction::MasterVolumeChanged(1.0),
                                                );
                                            } else if resp.changed() {
                                                actions.push(
                                                    AudioMixerAction::MasterVolumeChanged(
                                                        self.master_volume,
                                                    ),
                                                );
                                            }
                                        });

                                        let eff_l = if self.master_volume <= 0.0001 {
                                            -60.0
                                        } else {
                                            vu_level_l.clamp(-60.0, 0.0)
                                        };
                                        let eff_r = if self.master_volume <= 0.0001 {
                                            -60.0
                                        } else {
                                            vu_level_r.clamp(-60.0, 0.0)
                                        };

                                        draw_vertical_vu_strip(ui, &[eff_l, eff_r], fader_h, true);
                                    });

                                    ui.add_space(8.0);

                                    // Master Gain Percentage + 0 dB Reset
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 6.0;
                                        ui.add_space(18.0);
                                        ui.label(
                                            RichText::new(format!(
                                                "{:.0}%",
                                                self.master_volume * 100.0
                                            ))
                                            .monospace()
                                            .size(10.5)
                                            .strong()
                                            .color(Color32::from_rgb(0x58, 0xb4, 0xff)),
                                        );
                                        if ui
                                            .add(
                                                Button::new(
                                                    RichText::new("0 dB")
                                                        .size(9.5)
                                                        .color(Color32::from_rgb(0x98, 0xa8, 0xc8)),
                                                )
                                                .min_size(Vec2::new(34.0, 18.0))
                                                .rounding(Rounding::same(3.0))
                                                .fill(Color32::from_rgb(0x22, 0x2a, 0x3a)),
                                            )
                                            .on_hover_text(
                                                "Master kazancını 0.0 dB (100%) değerine sıfırla",
                                            )
                                            .clicked()
                                        {
                                            self.master_volume = 1.0;
                                            actions
                                                .push(AudioMixerAction::MasterVolumeChanged(1.0));
                                        }
                                    });
                                });
                            });
                    });
                });
            });

        actions
    }
}

fn format_gain_db(gain: f64) -> String {
    if gain <= 0.0001 {
        "-inf dB".to_string()
    } else {
        let db = 20.0 * gain.log10();
        if db > 0.05 {
            format!("+{:.1} dB", db)
        } else if db.abs() <= 0.05 {
            "0.0 dB".to_string()
        } else {
            format!("{:.1} dB", db)
        }
    }
}

fn render_db_readout_pill(ui: &mut Ui, text: &str, text_color: Color32) {
    Frame::none()
        .fill(Color32::from_rgb(0x10, 0x11, 0x16))
        .rounding(Rounding::same(4.0))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x25, 0x28, 0x34)))
        .inner_margin(Margin::symmetric(8.0, 3.0))
        .show(ui, |ui| {
            ui.set_min_width(68.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(text).monospace().size(10.5).color(text_color));
            });
        });
}

/// Renders 1 or 2 vertical segmented LED VU bars (`db_channels` in `-60.0..=0.0` dB)
/// with optional dB scale labels (`0`, `-6`, `-12`, `-24`, `-48`).
fn draw_vertical_vu_strip(ui: &mut Ui, db_channels: &[f32], height: f32, show_scale: bool) {
    let num_ch = db_channels.len().max(1);
    let bar_w = 8.0;
    let gap = 3.0;
    let bars_total_w = (num_ch as f32) * bar_w + ((num_ch.saturating_sub(1)) as f32) * gap;
    let scale_w = if show_scale { 22.0 } else { 0.0 };
    let total_w = bars_total_w + scale_w + 6.0;

    let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, height), Sense::hover());
    let painter = ui.painter_at(rect);

    let top_y = rect.top() + 2.0;
    let bottom_y = rect.bottom() - 2.0;
    let bar_h = (bottom_y - top_y).max(16.0);

    for (ch_idx, &db_raw) in db_channels.iter().enumerate() {
        let x = rect.left() + 2.0 + (ch_idx as f32) * (bar_w + gap);
        let bg_rect = Rect::from_min_size(Pos2::new(x, top_y), Vec2::new(bar_w, bar_h));
        painter.rect_filled(bg_rect, 2.0, Color32::from_rgb(0x0e, 0x0f, 0x14));
        painter.rect_stroke(
            bg_rect,
            2.0,
            Stroke::new(1.0_f32, Color32::from_rgb(0x22, 0x24, 0x2e)),
        );

        let db = db_raw.clamp(-60.0, 0.0);
        let frac = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
        let fill_h = bar_h * frac;
        let fill_top = bottom_y - fill_h;

        let step = 3.5;
        let mut cur_y = bottom_y - step;
        while cur_y >= top_y {
            let seg_frac = (bottom_y - cur_y) / bar_h;
            let seg_db = -60.0 + seg_frac * 60.0;
            let is_lit = cur_y >= fill_top && frac > 0.005;

            let color = if is_lit {
                if seg_db > -3.0 {
                    Color32::from_rgb(0xf2, 0x44, 0x44)
                } else if seg_db > -12.0 {
                    Color32::from_rgb(0xf0, 0xad, 0x32)
                } else {
                    Color32::from_rgb(0x34, 0xc7, 0x69)
                }
            } else if seg_db > -3.0 {
                Color32::from_rgb(0x2a, 0x16, 0x18)
            } else if seg_db > -12.0 {
                Color32::from_rgb(0x28, 0x22, 0x16)
            } else {
                Color32::from_rgb(0x14, 0x24, 0x1b)
            };

            painter.rect_filled(
                Rect::from_min_size(Pos2::new(x + 1.0, cur_y), Vec2::new(bar_w - 2.0, 2.2)),
                0.5,
                color,
            );
            cur_y -= step;
        }
    }

    if show_scale {
        let text_x = rect.left() + 2.0 + bars_total_w + 5.0;
        for (label, db_mark) in [
            ("0", 0.0_f32),
            ("-6", -6.0),
            ("-12", -12.0),
            ("-24", -24.0),
            ("-48", -48.0),
        ] {
            let frac = (db_mark + 60.0) / 60.0;
            let y = bottom_y - bar_h * frac;
            painter.text(
                Pos2::new(text_x, y),
                Align2::LEFT_CENTER,
                label,
                FontId::monospace(8.0),
                Color32::from_rgb(0x74, 0x78, 0x8c),
            );
        }
    }
}
