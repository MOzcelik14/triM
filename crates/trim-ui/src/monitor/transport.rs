use super::{MonitorAction, MonitorMode, MonitorWidget};
use egui::{Button, Color32, Pos2, Rect, Response, RichText, Sense, Stroke, Ui, Vec2};

impl<'a> MonitorWidget<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_transport(
        &self,
        ui: &mut Ui,
        painter: &egui::Painter,
        response: &Response,
        outer_rect: Rect,
        bottom_total_h: f32,
        scrubber_h: f32,
        action: &mut Option<MonitorAction>,
    ) {
        // 3. Scrubber Bar
        let scrub_rect = Rect::from_min_max(
            Pos2::new(
                outer_rect.left() + 6.0,
                outer_rect.bottom() - bottom_total_h,
            ),
            Pos2::new(
                outer_rect.right() - 6.0,
                outer_rect.bottom() - bottom_total_h + scrubber_h,
            ),
        );
        painter.rect_filled(scrub_rect, 2.0, Color32::from_rgb(0x18, 0x18, 0x20));
        painter.rect_stroke(
            scrub_rect,
            2.0,
            Stroke::new(1.0_f32, Color32::from_rgb(0x28, 0x28, 0x32)),
        );

        if self.duration > 0.0 {
            let dur = self.duration;
            let track_w = scrub_rect.width() - 8.0;
            let x_start = scrub_rect.left() + 4.0;

            // Mark In / Mark Out shaded range for Source Monitor
            if let (Some(m_in), Some(m_out)) = (self.mark_in, self.mark_out) {
                let in_ratio = (m_in / dur).clamp(0.0, 1.0) as f32;
                let out_ratio = (m_out / dur).clamp(0.0, 1.0) as f32;
                let r_in_x = x_start + in_ratio * track_w;
                let r_out_x = x_start + out_ratio * track_w;
                let in_out_rect = Rect::from_min_max(
                    Pos2::new(r_in_x.min(r_out_x), scrub_rect.top() + 2.0),
                    Pos2::new(r_in_x.max(r_out_x), scrub_rect.bottom() - 2.0),
                );
                painter.rect_filled(
                    in_out_rect,
                    1.0,
                    Color32::from_rgba_premultiplied(56, 122, 224, 70),
                );

                // Bracket handles { }
                painter.line_segment(
                    [
                        Pos2::new(r_in_x, scrub_rect.top()),
                        Pos2::new(r_in_x, scrub_rect.bottom()),
                    ],
                    Stroke::new(2.0_f32, Color32::from_rgb(0x4a, 0x9a, 0xff)),
                );
                painter.line_segment(
                    [
                        Pos2::new(r_out_x, scrub_rect.top()),
                        Pos2::new(r_out_x, scrub_rect.bottom()),
                    ],
                    Stroke::new(2.0_f32, Color32::from_rgb(0x4a, 0x9a, 0xff)),
                );
            }

            // Current playhead position
            let curr_ratio = (self.current_time / dur).clamp(0.0, 1.0) as f32;
            let playhead_x = x_start + curr_ratio * track_w;
            painter.line_segment(
                [
                    Pos2::new(playhead_x, scrub_rect.top()),
                    Pos2::new(playhead_x, scrub_rect.bottom()),
                ],
                Stroke::new(2.0_f32, Color32::from_rgb(0xe0, 0x7a, 0x38)),
            );

            // Scrubber interaction
            if let Some(mouse_pos) = response.hover_pos() {
                if scrub_rect.contains(mouse_pos) && ui.input(|i| i.pointer.primary_down()) {
                    let click_ratio = ((mouse_pos.x - x_start) / track_w).clamp(0.0, 1.0);
                    let target_t = (click_ratio as f64) * dur;
                    *action = Some(MonitorAction::Seek(target_t));
                }
            }
        }

        // 4. Bottom Transport Toolbar (Premiere Pro buttons row)
        let tool_rect = Rect::from_min_max(
            Pos2::new(
                outer_rect.left(),
                outer_rect.bottom() - (bottom_total_h - scrubber_h),
            ),
            Pos2::new(outer_rect.right(), outer_rect.bottom()),
        );
        painter.rect_filled(tool_rect, 3.0, Color32::from_rgb(0x16, 0x16, 0x1b));
        painter.line_segment(
            [tool_rect.left_top(), tool_rect.right_top()],
            Stroke::new(1.0_f32, Color32::from_rgb(0x26, 0x26, 0x30)),
        );

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(tool_rect), |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add_space(8.0);

                // Timecode
                ui.label(
                    RichText::new(self.timecode)
                        .monospace()
                        .size(11.5)
                        .color(Color32::from_rgb(0x38, 0xa0, 0xe8))
                        .strong(),
                );
                ui.label(
                    RichText::new("/")
                        .size(10.0)
                        .color(Color32::from_rgb(0x60, 0x60, 0x70)),
                );
                ui.label(
                    RichText::new(self.duration_timecode)
                        .monospace()
                        .size(11.0)
                        .color(Color32::from_rgb(0x90, 0x90, 0xa0)),
                );

                if self.is_playing && (self.playback_speed - 1.0).abs() > 0.01 {
                    let speed_label = if self.playback_speed > 0.0 {
                        format!(">> {:.0}x", self.playback_speed)
                    } else {
                        format!("<< {:.0}x", self.playback_speed.abs())
                    };
                    let speed_color = if self.playback_speed > 0.0 {
                        Color32::from_rgb(0x50, 0xd0, 0x70)
                    } else {
                        Color32::from_rgb(0xe0, 0x9a, 0x38)
                    };
                    ui.label(
                        RichText::new(speed_label)
                            .monospace()
                            .size(11.0)
                            .strong()
                            .color(speed_color),
                    );
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                // Mark In button {
                let btn_in = Button::new(RichText::new("{").strong().size(13.0))
                    .min_size(Vec2::new(24.0, 24.0))
                    .fill(Color32::from_rgb(0x22, 0x22, 0x2a));
                if ui
                    .add(btn_in)
                    .on_hover_text("Giriş Noktası Koy (Mark In - I veya {)")
                    .clicked()
                {
                    *action = Some(MonitorAction::MarkIn);
                }

                // Mark Out button }
                let btn_out = Button::new(RichText::new("}").strong().size(13.0))
                    .min_size(Vec2::new(24.0, 24.0))
                    .fill(Color32::from_rgb(0x22, 0x22, 0x2a));
                if ui
                    .add(btn_out)
                    .on_hover_text("Çıkış Noktası Koy (Mark Out - O veya })")
                    .clicked()
                {
                    *action = Some(MonitorAction::MarkOut);
                }

                // Go to In
                let btn_go_in = Button::new(RichText::new("|<").size(11.0).strong())
                    .min_size(Vec2::new(24.0, 24.0))
                    .fill(Color32::from_rgb(0x22, 0x22, 0x2a));
                if ui
                    .add(btn_go_in)
                    .on_hover_text("Giriş Noktasına Git (Go to In)")
                    .clicked()
                {
                    *action = Some(MonitorAction::GoToIn);
                }

                // Step -1 frame
                let btn_prev = Button::new(RichText::new("<").size(12.0).strong())
                    .min_size(Vec2::new(24.0, 24.0))
                    .fill(Color32::from_rgb(0x22, 0x22, 0x2a));
                if ui
                    .add(btn_prev)
                    .on_hover_text("Önceki Kare (Sol Ok)")
                    .clicked()
                {
                    *action = Some(MonitorAction::StepFrame(-1));
                }

                // Play / Pause button (vector painted triangle / pause bars)
                let (play_rect, play_resp) =
                    ui.allocate_exact_size(Vec2::new(30.0, 24.0), Sense::click());
                let pp = ui.painter_at(play_rect);
                let bg_col = if play_resp.hovered() {
                    Color32::from_rgb(0x38, 0x38, 0x46)
                } else {
                    Color32::from_rgb(0x28, 0x28, 0x34)
                };
                pp.rect_filled(play_rect, 3.0, bg_col);
                pp.rect_stroke(
                    play_rect,
                    3.0,
                    Stroke::new(1.0_f32, Color32::from_rgb(0x3c, 0x3c, 0x4c)),
                );

                let p_center = play_rect.center();
                if self.is_playing {
                    let bar_w = 3.0;
                    let bar_h = 10.0;
                    let bar1 = Rect::from_center_size(
                        Pos2::new(p_center.x - 3.5, p_center.y),
                        Vec2::new(bar_w, bar_h),
                    );
                    let bar2 = Rect::from_center_size(
                        Pos2::new(p_center.x + 3.5, p_center.y),
                        Vec2::new(bar_w, bar_h),
                    );
                    pp.rect_filled(bar1, 1.0, Color32::from_rgb(0x50, 0xd0, 0x70));
                    pp.rect_filled(bar2, 1.0, Color32::from_rgb(0x50, 0xd0, 0x70));
                } else {
                    let tri = vec![
                        Pos2::new(p_center.x - 4.0, p_center.y - 6.0),
                        Pos2::new(p_center.x + 6.0, p_center.y),
                        Pos2::new(p_center.x - 4.0, p_center.y + 6.0),
                    ];
                    pp.add(egui::Shape::convex_polygon(
                        tri,
                        Color32::WHITE,
                        Stroke::NONE,
                    ));
                }
                if play_resp
                    .on_hover_text("Oynat / Duraklat (Boşluk)")
                    .clicked()
                {
                    *action = Some(MonitorAction::TogglePlay);
                }

                // Step +1 frame
                let btn_next = Button::new(RichText::new(">").size(12.0).strong())
                    .min_size(Vec2::new(24.0, 24.0))
                    .fill(Color32::from_rgb(0x22, 0x22, 0x2a));
                if ui
                    .add(btn_next)
                    .on_hover_text("Sonraki Kare (Sağ Ok)")
                    .clicked()
                {
                    *action = Some(MonitorAction::StepFrame(1));
                }

                // Go to Out
                let btn_go_out = Button::new(RichText::new(">|").size(11.0).strong())
                    .min_size(Vec2::new(24.0, 24.0))
                    .fill(Color32::from_rgb(0x22, 0x22, 0x2a));
                if ui
                    .add(btn_go_out)
                    .on_hover_text("Çıkış Noktasına Git (Go to Out)")
                    .clicked()
                {
                    *action = Some(MonitorAction::GoToOut);
                }

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                match self.mode {
                    MonitorMode::Source => {
                        // Insert button (shortcut ,)
                        let btn_insert = Button::new(
                            RichText::new("+ Ekle")
                                .size(11.0)
                                .color(Color32::from_rgb(0x80, 0xc0, 0xff))
                                .strong(),
                        )
                        .min_size(Vec2::new(26.0, 24.0))
                        .fill(Color32::from_rgb(0x20, 0x2a, 0x38));
                        if ui
                            .add(btn_insert)
                            .on_hover_text("Zaman Çizgisine Ekle (Insert - Virgül ',')")
                            .clicked()
                        {
                            *action = Some(MonitorAction::InsertToTimeline);
                        }

                        // Overwrite button (shortcut .)
                        let btn_overwrite = Button::new(
                            RichText::new("= Yaz")
                                .size(11.0)
                                .color(Color32::from_rgb(0x60, 0xd8, 0xa0))
                                .strong(),
                        )
                        .min_size(Vec2::new(26.0, 24.0))
                        .fill(Color32::from_rgb(0x1c, 0x30, 0x28));
                        if ui
                            .add(btn_overwrite)
                            .on_hover_text("Üzerine Yaz (Overwrite - Nokta '.')")
                            .clicked()
                        {
                            *action = Some(MonitorAction::OverwriteToTimeline);
                        }

                        let btn_subclip = Button::new(
                            RichText::new("✂ Alt-Klip")
                                .size(11.0)
                                .color(Color32::from_rgb(0xe8, 0xb8, 0x58)),
                        )
                        .min_size(Vec2::new(28.0, 24.0))
                        .fill(Color32::from_rgb(0x2a, 0x24, 0x18));
                        if ui
                            .add(btn_subclip)
                            .on_hover_text("In/Out aralığından Medya Havuzuna Alt-Klip (Subclip) oluştur")
                            .clicked()
                        {
                            *action = Some(MonitorAction::CreateSubclip);
                        }
                    }
                    MonitorMode::Program => {
                        // Export Frame camera icon
                        let btn_frame = Button::new(
                            RichText::new("Kare Al")
                                .size(11.0)
                                .color(Color32::from_rgb(0xe0, 0xaa, 0x50))
                                .strong(),
                        )
                        .min_size(Vec2::new(26.0, 24.0))
                        .fill(Color32::from_rgb(0x2d, 0x28, 0x20));
                        if ui
                            .add(btn_frame)
                            .on_hover_text("Mevcut Kareyi Dışa Aktar (Export Frame)")
                            .clicked()
                        {
                            *action = Some(MonitorAction::ExportFrame);
                        }

                        let safe_color = if self.safe_zones {
                            Color32::from_rgb(0x38, 0xa0, 0xe8)
                        } else {
                            Color32::from_rgb(0xa0, 0xa0, 0xb0)
                        };
                        let btn_safe = Button::new(
                            RichText::new("Rehber")
                                .size(11.0)
                                .color(safe_color)
                                .strong(),
                        )
                        .min_size(Vec2::new(26.0, 24.0))
                        .fill(if self.safe_zones {
                            Color32::from_rgb(0x18, 0x30, 0x48)
                        } else {
                            Color32::from_rgb(0x22, 0x22, 0x2a)
                        });
                        if ui
                            .add(btn_safe)
                            .on_hover_text("Güvenli Alan & Sosyal Medya Rehber Çizgileri")
                            .clicked()
                        {
                            *action = Some(MonitorAction::ToggleSafeZones);
                        }

                        let cmp_mode_id = egui::Id::new("program_monitor_compare_submode");
                        let mut cmp_mode: u8 = ui
                            .data_mut(|d| d.get_temp::<u8>(cmp_mode_id))
                            .unwrap_or(0);
                        let wipe_lbl = match cmp_mode {
                            1 => "⬒ Wipe (Dikey)",
                            2 => "▥ 2x Yan Yana",
                            _ => "◧ Wipe",
                        };
                        let wipe_col = if self.split_wipe_enabled {
                            Color32::from_rgb(0x58, 0xb4, 0xff)
                        } else {
                            Color32::from_rgb(0xa0, 0xa0, 0xb0)
                        };
                        let btn_wipe = Button::new(
                            RichText::new(wipe_lbl)
                                .size(11.0)
                                .color(wipe_col)
                                .strong(),
                        )
                        .min_size(Vec2::new(26.0, 24.0))
                        .fill(if self.split_wipe_enabled {
                            Color32::from_rgb(0x18, 0x30, 0x48)
                        } else {
                            Color32::from_rgb(0x22, 0x22, 0x2a)
                        });
                        let wipe_resp = ui.add(btn_wipe).on_hover_text(
                            "Sol Tık: Bölünmüş Ekran Aç/Kapat • Sağ Tık: Mod Değiştir (Yatay Wipe / Dikey Wipe / 2x Yan Yana)",
                        );
                        if wipe_resp.clicked() {
                            *action = Some(MonitorAction::ToggleSplitWipe);
                        } else if wipe_resp.secondary_clicked() {
                            cmp_mode = (cmp_mode + 1) % 3;
                            ui.data_mut(|d| d.insert_temp(cmp_mode_id, cmp_mode));
                            if !self.split_wipe_enabled {
                                *action = Some(MonitorAction::ToggleSplitWipe);
                            }
                        }

                        let mc_col = if self.multicam_mode {
                            Color32::from_rgb(0xf8, 0x71, 0x71)
                        } else {
                            Color32::from_rgb(0xa0, 0xa0, 0xb0)
                        };
                        let btn_mc = Button::new(
                            RichText::new("⊞ Multicam")
                                .size(11.0)
                                .color(mc_col)
                                .strong(),
                        )
                        .min_size(Vec2::new(26.0, 24.0))
                        .fill(if self.multicam_mode {
                            Color32::from_rgb(0x3a, 0x1e, 0x24)
                        } else {
                            Color32::from_rgb(0x22, 0x22, 0x2a)
                        });
                        if ui
                            .add(btn_mc)
                            .on_hover_text("2x2 Çoklu Kamera (Multicam) İzleyici ve Canlı Açı Kesimi (1-4 Tuşları)")
                            .clicked()
                        {
                            *action = Some(MonitorAction::ToggleMulticamView);
                        }

                        if let Some(curr_ar) = self.current_aspect_ratio {
                            ui.add_space(2.0);
                            egui::ComboBox::from_id_salt("monitor_aspect_combo")
                                .selected_text(curr_ar.short_label())
                                .show_ui(ui, |ui| {
                                    for r in [
                                        trim_core::AspectRatio::Wide16x9,
                                        trim_core::AspectRatio::Vertical9x16,
                                        trim_core::AspectRatio::Square1x1,
                                        trim_core::AspectRatio::Portrait4x5,
                                        trim_core::AspectRatio::Classic4x3,
                                        trim_core::AspectRatio::Cinema21x9,
                                    ] {
                                        if ui.selectable_label(curr_ar == r, r.label()).clicked() {
                                            *action = Some(MonitorAction::SetAspectRatio(r));
                                        }
                                    }
                                });
                        }

                        ui.add_space(2.0);
                        let btn_fs = egui::Button::new(
                            RichText::new("⛶")
                                .size(13.0)
                                .color(Color32::from_rgb(0xc0, 0xc5, 0xd0)),
                        )
                        .min_size(Vec2::new(26.0, 24.0))
                        .fill(Color32::from_rgb(0x22, 0x22, 0x2a));
                        if ui
                            .add(btn_fs)
                            .on_hover_text("Tam Ekran Önizleme (F11)")
                            .clicked()
                        {
                            *action = Some(MonitorAction::ToggleFullscreen);
                        }
                    }
                }
            });
        });
    }
}
