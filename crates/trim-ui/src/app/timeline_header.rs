use super::TrimApp;
use egui::{Button, Color32, RichText, Stroke};
use trim_core::time::seconds_to_timecode;

impl TrimApp {
    pub(super) fn render_timeline_header(&mut self, ui: &mut egui::Ui) {
        // Sequence Timeline Header Bar
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;

            self.project.ensure_sequences();

            let mut seq_to_switch = None;
            let mut seq_to_create = false;
            let mut seq_to_remove = None;
            let mut seq_to_duplicate = None;

            for seq in &self.project.sequences {
                let is_active = self.project.active_sequence_id.as_deref() == Some(&seq.id);

                let fill_col = if is_active {
                    Color32::from_rgb(0x28, 0x2c, 0x3c)
                } else {
                    Color32::from_rgb(0x18, 0x1a, 0x22)
                };

                let border_stroke = if is_active {
                    Stroke::new(1.5_f32, Color32::from_rgb(0x38, 0x7a, 0xe0))
                } else {
                    Stroke::new(1.0_f32, Color32::from_rgb(0x28, 0x2c, 0x38))
                };

                let text_col = if is_active {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(0x9a, 0xa2, 0xb8)
                };

                let tab_btn =
                    Button::new(RichText::new(&seq.name).size(11.5).strong().color(text_col))
                        .fill(fill_col)
                        .stroke(border_stroke);

                let resp = ui.add(tab_btn);
                if resp.clicked() && !is_active {
                    seq_to_switch = Some(seq.id.clone());
                }

                resp.context_menu(|ui| {
                    if ui.button("+ Sekansı Çoğalt (Duplicate)").clicked() {
                        seq_to_duplicate = Some(seq.id.clone());
                        ui.close_menu();
                    }
                    if self.project.sequences.len() > 1 && ui.button("✕ Bu Sekansı Kapat").clicked()
                    {
                        seq_to_remove = Some(seq.id.clone());
                        ui.close_menu();
                    }
                });
            }

            // Button to add new sequence
            let add_seq_btn = Button::new(
                RichText::new("+")
                    .size(12.0)
                    .strong()
                    .color(Color32::from_rgb(0xa0, 0xa8, 0xc0)),
            )
            .fill(Color32::from_rgb(0x18, 0x1a, 0x22))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x28, 0x2c, 0x38)));

            if ui
                .add(add_seq_btn)
                .on_hover_text("Yeni Sekans Oluştur")
                .clicked()
            {
                seq_to_create = true;
            }

            if let Some(target_id) = seq_to_switch {
                self.project.switch_sequence(&target_id);
                self.media.video_engine.clear_cache();
                self.playback.audio_mixer.clear_cache();
                self.canvas_state.selected_clip_pairs.clear();
                self.playback.current_time = 0.0;
            }

            if seq_to_create {
                let next_num = self.project.sequences.len() + 1;
                let new_name = format!("Sekans {:02}", next_num);
                self.project.create_sequence(new_name);
                self.media.video_engine.clear_cache();
                self.playback.audio_mixer.clear_cache();
                self.canvas_state.selected_clip_pairs.clear();
                self.playback.current_time = 0.0;
            }

            if let Some(rem_id) = seq_to_remove {
                self.project.remove_sequence(&rem_id);
                self.media.video_engine.clear_cache();
                self.playback.audio_mixer.clear_cache();
                self.canvas_state.selected_clip_pairs.clear();
                self.playback.current_time = 0.0;
            }

            if let Some(dup_id) = seq_to_duplicate {
                self.project.duplicate_sequence(&dup_id);
            }

            ui.add_space(8.0);
            let current_tc =
                seconds_to_timecode(self.playback.current_time, self.project.timeline.fps);
            ui.label(
                RichText::new(&current_tc)
                    .monospace()
                    .size(15.0)
                    .color(Color32::from_rgb(0x38, 0xa0, 0xf0))
                    .strong(),
            );

            ui.add_space(10.0);
            let snap_btn = Button::new(RichText::new("Mıknatıs").size(11.0).strong().color(
                if self.tools.snap_enabled {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(0x90, 0x94, 0xa6)
                },
            ))
            .fill(if self.tools.snap_enabled {
                Color32::from_rgb(0x28, 0x5a, 0x9e)
            } else {
                Color32::from_rgb(0x20, 0x20, 0x26)
            })
            .stroke(Stroke::new(
                1.0_f32,
                if self.tools.snap_enabled {
                    Color32::from_rgb(0x4a, 0x8a, 0xe8)
                } else {
                    Color32::from_rgb(0x30, 0x32, 0x3e)
                },
            ));
            if ui
                .add(snap_btn)
                .on_hover_text("Mıknatıs / Hizalama (Snapping - N)")
                .clicked()
            {
                self.toggle_snap();
            }

            if ui
                .button(RichText::new("İşaretçi").size(11.0))
                .on_hover_text("İşaretçi Ekle / Sil (M)")
                .clicked()
            {
                self.add_or_toggle_marker();
            }

            if ui
                .button(RichText::new("+ Video").size(11.0))
                .on_hover_text("Yeni Video Kanalı Ekle (Üste)")
                .clicked()
            {
                self.add_video_track();
            }

            if ui
                .button(RichText::new("+ Ses").size(11.0))
                .on_hover_text("Yeni Ses Kanalı Ekle (Alta)")
                .clicked()
            {
                self.add_audio_track();
            }

            if ui
                .button(
                    RichText::new("+ Başlık")
                        .size(11.0)
                        .color(Color32::from_rgb(0x81, 0x8c, 0xf8)),
                )
                .on_hover_text("Zaman Çizgisine Yeni Başlık Klibi Ekle (Title Clip Modal)")
                .clicked()
            {
                self.dialogs.title_dialog.open();
            }

            ui.add_space(6.0);
            let proxy_btn = Button::new(
                RichText::new(if self.media.use_proxies {
                    "◈ Proxy: Açık"
                } else {
                    "◈ Proxy: Kapalı"
                })
                .size(11.0)
                .color(if self.media.use_proxies {
                    Color32::from_rgb(0x38, 0xa0, 0xf0)
                } else {
                    Color32::from_rgb(0x70, 0x70, 0x80)
                }),
            )
            .fill(if self.media.use_proxies {
                Color32::from_rgb(0x18, 0x28, 0x3e)
            } else {
                Color32::from_rgb(0x20, 0x20, 0x26)
            });
            if ui
                .add(proxy_btn)
                .on_hover_text("Düşük Donanımlar İçin Akıllı 720p Proxy Önizleme")
                .clicked()
            {
                self.toggle_proxy();
            }

            ui.add_space(4.0);
            let rec_btn = Button::new(
                RichText::new("● Ekran Kaydet")
                    .size(11.0)
                    .color(Color32::from_rgb(0xe0, 0x80, 0x40)),
            )
            .fill(Color32::from_rgb(0x28, 0x20, 0x1e));
            if ui
                .add(rec_btn)
                .on_hover_text("Dahili Ekran ve Web Kamerası Kaydedici")
                .clicked()
            {
                self.open_screen_record_dialog();
            }

            if let Some(ref vo) = self.media.active_voiceover {
                let vo_dur = vo.recorder.elapsed_seconds();
                let vo_rec_btn = Button::new(
                    RichText::new(format!("● SES KAYDI: {:.1}s [⏹ Durdur]", vo_dur))
                        .size(11.0)
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(0xd0, 0x24, 0x24));
                if ui.add(vo_rec_btn).clicked() {
                    self.finish_voiceover();
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                ui.add_sized(
                    [70.0, 18.0],
                    egui::Slider::new(&mut self.canvas_state.pixels_per_second, 15.0..=350.0)
                        .show_value(false),
                );
                ui.label(
                    RichText::new("Zoom:")
                        .size(11.0)
                        .color(Color32::from_rgb(0x80, 0x80, 0x90)),
                );
            });
        });
    }
}
