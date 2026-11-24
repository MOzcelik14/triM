use super::guides_and_masks::{
    draw_button, draw_corner_guides, draw_safe_zones, fit_aspect, handle_mask_overlay_interaction,
};
use super::overlays::draw_text_overlays_on_rect;
use super::{overlay_hit_rect, ActiveTextOverlay, MonitorAction, MonitorMode, MonitorWidget};
use egui::{Color32, Pos2, Rect, Response, Stroke, Ui, Vec2};

impl<'a> MonitorWidget<'a> {
    pub(super) fn render_viewport(
        &mut self,
        ui: &mut Ui,
        painter: &egui::Painter,
        response: &Response,
        viewport_rect: Rect,
        action: &mut Option<MonitorAction>,
    ) {
        // Viewport video frame calculated with respect to project aspect ratio
        let max_w = (viewport_rect.width() - 8.0).max(10.0);
        let max_h = (viewport_rect.height() - 8.0).max(10.0);
        let (target_w, target_h) = fit_aspect(max_w, max_h, self.aspect_ratio);
        let frame_rect =
            Rect::from_center_size(viewport_rect.center(), Vec2::new(target_w, target_h));

        // Deep black canvas background
        painter.rect_filled(frame_rect, 0.0, Color32::from_rgb(0x02, 0x02, 0x04));

        if !self.has_content {
            let center = frame_rect.center();
            painter.text(
                Pos2::new(center.x, center.y - 30.0),
                egui::Align2::CENTER_CENTER,
                if self.mode == MonitorMode::Source {
                    "Klipsiz Kaynak"
                } else {
                    "Sekans Boş"
                },
                egui::FontId::proportional(16.0),
                Color32::from_rgb(0x80, 0x80, 0x95),
            );

            let msg = if self.mode == MonitorMode::Source {
                "Medya havuzundan bir klibe çift tıklayın"
            } else {
                "Zaman çizgisine klip ekleyin"
            };
            painter.text(
                Pos2::new(center.x, center.y),
                egui::Align2::CENTER_CENTER,
                msg,
                egui::FontId::proportional(11.5),
                Color32::from_rgb(0x60, 0x60, 0x70),
            );

            let btn_y = center.y + 36.0;
            let btn_w = 110.0;
            let btn_h = 26.0;
            let r_import =
                Rect::from_center_size(Pos2::new(center.x, btn_y), Vec2::new(btn_w + 30.0, btn_h));

            if let Some(mouse_pos) = response.hover_pos() {
                if ui.input(|i| i.pointer.primary_clicked()) && r_import.contains(mouse_pos) {
                    *action = Some(MonitorAction::ImportMedia);
                }
            }
            draw_button(painter, r_import, "+ Medya İçe Aktar", true);
        } else if let Some(tex_id) = self
            .custom_texture_id
            .or_else(|| self.texture.map(|t| t.id()))
        {
            if self.mode == MonitorMode::Program && self.split_wipe_enabled {
                let cmp_mode_id = egui::Id::new("program_monitor_compare_submode");
                let mut cmp_mode: u8 = ui.data_mut(|d| d.get_temp::<u8>(cmp_mode_id)).unwrap_or(0);

                match cmp_mode {
                    1 => {
                        // Vertical Wipe (Top vs Bottom)
                        let split_y = frame_rect.top() + frame_rect.height() * self.split_wipe_pos;
                        let top_rect = Rect::from_min_max(
                            frame_rect.left_top(),
                            Pos2::new(frame_rect.right(), split_y),
                        );
                        let bot_rect = Rect::from_min_max(
                            Pos2::new(frame_rect.left(), split_y),
                            frame_rect.right_bottom(),
                        );
                        let top_uv =
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, self.split_wipe_pos));
                        let bot_uv = Rect::from_min_max(
                            Pos2::new(0.0, self.split_wipe_pos),
                            Pos2::new(1.0, 1.0),
                        );

                        if let Some(ref_tex) = self.reference_texture {
                            painter.image(ref_tex.id(), top_rect, top_uv, Color32::WHITE);
                        } else {
                            painter.image(
                                tex_id,
                                top_rect,
                                top_uv,
                                Color32::from_rgb(205, 205, 210),
                            );
                        }
                        painter.image(tex_id, bot_rect, bot_uv, Color32::WHITE);

                        painter.line_segment(
                            [
                                Pos2::new(frame_rect.left(), split_y),
                                Pos2::new(frame_rect.right(), split_y),
                            ],
                            Stroke::new(2.0_f32, Color32::from_rgb(0x58, 0xb4, 0xff)),
                        );
                        let handle_center = Pos2::new(frame_rect.center().x, split_y);
                        painter.circle_filled(
                            handle_center,
                            9.0,
                            Color32::from_rgb(0x18, 0x20, 0x30),
                        );
                        painter.circle_stroke(
                            handle_center,
                            9.0,
                            Stroke::new(1.5_f32, Color32::from_rgb(0x58, 0xb4, 0xff)),
                        );
                        painter.text(
                            Pos2::new(frame_rect.left() + 8.0, frame_rect.top() + 14.0),
                            egui::Align2::LEFT_CENTER,
                            if self.reference_texture.is_some() {
                                "REFERANS STILL (ÜST)"
                            } else {
                                "KARŞILAŞTIRMA (ÜST)"
                            },
                            egui::FontId::monospace(9.5),
                            Color32::from_rgb(220, 230, 245),
                        );
                        painter.text(
                            Pos2::new(frame_rect.right() - 8.0, frame_rect.bottom() - 14.0),
                            egui::Align2::RIGHT_CENTER,
                            "MEVCUT GRADE (ALT)",
                            egui::FontId::monospace(9.5),
                            Color32::from_rgb(88, 180, 255),
                        );

                        if let Some(ptr) = response.interact_pointer_pos() {
                            if response.dragged() && frame_rect.contains(ptr) {
                                let new_pos = ((ptr.y - frame_rect.top()) / frame_rect.height())
                                    .clamp(0.05, 0.95);
                                *action = Some(MonitorAction::SplitWipePosChanged(new_pos));
                            }
                        }
                    }
                    2 => {
                        // Side-by-Side 2x Comparison (Full Left vs Full Right)
                        let mid_x = frame_rect.center().x;
                        let left_rect = Rect::from_min_max(
                            frame_rect.left_top(),
                            Pos2::new(mid_x - 1.0, frame_rect.bottom()),
                        );
                        let right_rect = Rect::from_min_max(
                            Pos2::new(mid_x + 1.0, frame_rect.top()),
                            frame_rect.right_bottom(),
                        );
                        let full_uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));

                        if let Some(ref_tex) = self.reference_texture {
                            painter.image(ref_tex.id(), left_rect, full_uv, Color32::WHITE);
                        } else {
                            painter.image(
                                tex_id,
                                left_rect,
                                full_uv,
                                Color32::from_rgb(205, 205, 210),
                            );
                        }
                        painter.image(tex_id, right_rect, full_uv, Color32::WHITE);

                        painter.line_segment(
                            [
                                Pos2::new(mid_x, frame_rect.top()),
                                Pos2::new(mid_x, frame_rect.bottom()),
                            ],
                            Stroke::new(2.0_f32, Color32::from_rgb(0x58, 0xb4, 0xff)),
                        );
                        painter.text(
                            Pos2::new(left_rect.left() + 8.0, frame_rect.top() + 14.0),
                            egui::Align2::LEFT_CENTER,
                            if self.reference_texture.is_some() {
                                "REFERANS (2x YAN YANA)"
                            } else {
                                "ÖNCESİ (2x YAN YANA)"
                            },
                            egui::FontId::monospace(9.5),
                            Color32::from_rgb(220, 230, 245),
                        );
                        painter.text(
                            Pos2::new(right_rect.right() - 8.0, frame_rect.top() + 14.0),
                            egui::Align2::RIGHT_CENTER,
                            "MEVCUT GRADE (2x)",
                            egui::FontId::monospace(9.5),
                            Color32::from_rgb(88, 180, 255),
                        );
                    }
                    _ => {
                        // Horizontal Wipe (Left vs Right)
                        let split_x = frame_rect.left() + frame_rect.width() * self.split_wipe_pos;
                        let left_rect = Rect::from_min_max(
                            frame_rect.left_top(),
                            Pos2::new(split_x, frame_rect.bottom()),
                        );
                        let right_rect = Rect::from_min_max(
                            Pos2::new(split_x, frame_rect.top()),
                            frame_rect.right_bottom(),
                        );
                        let left_uv =
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(self.split_wipe_pos, 1.0));
                        let right_uv = Rect::from_min_max(
                            Pos2::new(self.split_wipe_pos, 0.0),
                            Pos2::new(1.0, 1.0),
                        );

                        if let Some(ref_tex) = self.reference_texture {
                            painter.image(ref_tex.id(), left_rect, left_uv, Color32::WHITE);
                        } else {
                            painter.image(
                                tex_id,
                                left_rect,
                                left_uv,
                                Color32::from_rgb(205, 205, 210),
                            );
                        }
                        painter.image(tex_id, right_rect, right_uv, Color32::WHITE);

                        painter.line_segment(
                            [
                                Pos2::new(split_x, frame_rect.top()),
                                Pos2::new(split_x, frame_rect.bottom()),
                            ],
                            Stroke::new(2.0_f32, Color32::from_rgb(0x58, 0xb4, 0xff)),
                        );
                        let handle_center = Pos2::new(split_x, frame_rect.center().y);
                        painter.circle_filled(
                            handle_center,
                            9.0,
                            Color32::from_rgb(0x18, 0x20, 0x30),
                        );
                        painter.circle_stroke(
                            handle_center,
                            9.0,
                            Stroke::new(1.5_f32, Color32::from_rgb(0x58, 0xb4, 0xff)),
                        );
                        painter.text(
                            Pos2::new(left_rect.left() + 8.0, frame_rect.top() + 14.0),
                            egui::Align2::LEFT_CENTER,
                            if self.reference_texture.is_some() {
                                "REFERANS STILL"
                            } else {
                                "KARŞILAŞTIRMA (SOL)"
                            },
                            egui::FontId::monospace(9.5),
                            Color32::from_rgb(220, 230, 245),
                        );
                        painter.text(
                            Pos2::new(right_rect.right() - 8.0, frame_rect.top() + 14.0),
                            egui::Align2::RIGHT_CENTER,
                            "MEVCUT GRADE (SAĞ)",
                            egui::FontId::monospace(9.5),
                            Color32::from_rgb(88, 180, 255),
                        );

                        if let Some(ptr) = response.interact_pointer_pos() {
                            if response.dragged() && frame_rect.contains(ptr) {
                                let new_pos = ((ptr.x - frame_rect.left()) / frame_rect.width())
                                    .clamp(0.05, 0.95);
                                *action = Some(MonitorAction::SplitWipePosChanged(new_pos));
                            }
                        }
                    }
                }

                // Right-click on monitor while split wipe is active cycles mode (Horizontal -> Vertical -> Side-by-Side)
                if response.secondary_clicked() && !self.wb_eyedropper_active {
                    cmp_mode = (cmp_mode + 1) % 3;
                    ui.data_mut(|d| d.insert_temp(cmp_mode_id, cmp_mode));
                }
            } else {
                painter.image(
                    tex_id,
                    frame_rect,
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }

            if self.mode == MonitorMode::Program && self.multicam_mode {
                let mid = frame_rect.center();
                let quads = [
                    Rect::from_min_max(frame_rect.left_top(), mid),
                    Rect::from_min_max(
                        Pos2::new(mid.x, frame_rect.top()),
                        Pos2::new(frame_rect.right(), mid.y),
                    ),
                    Rect::from_min_max(
                        Pos2::new(frame_rect.left(), mid.y),
                        Pos2::new(mid.x, frame_rect.bottom()),
                    ),
                    Rect::from_min_max(mid, frame_rect.right_bottom()),
                ];
                for (qi, qrect) in quads.iter().enumerate() {
                    let is_active = self.multicam_active_angle == qi;
                    painter.rect_stroke(
                        *qrect,
                        0.0,
                        Stroke::new(
                            if is_active { 2.5_f32 } else { 1.0_f32 },
                            if is_active {
                                Color32::from_rgb(235, 65, 65)
                            } else {
                                Color32::from_rgba_unmultiplied(180, 190, 210, 110)
                            },
                        ),
                    );
                    let default_lbl = format!("KAMERA {} (Tuş {})", qi + 1, qi + 1);
                    let lbl = self
                        .multicam_labels
                        .get(qi)
                        .map(|s| s.as_str())
                        .unwrap_or(&default_lbl);
                    let badge_rect = Rect::from_min_size(
                        Pos2::new(qrect.left() + 6.0, qrect.top() + 6.0),
                        Vec2::new(136.0, 20.0),
                    );
                    painter.rect_filled(
                        badge_rect,
                        3.0,
                        if is_active {
                            Color32::from_rgba_unmultiplied(200, 35, 35, 220)
                        } else {
                            Color32::from_rgba_unmultiplied(15, 18, 26, 200)
                        },
                    );
                    painter.text(
                        badge_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        lbl,
                        egui::FontId::monospace(10.0),
                        Color32::WHITE,
                    );
                    if let Some(click_pos) = response.interact_pointer_pos() {
                        if response.clicked() && qrect.contains(click_pos) {
                            *action = Some(MonitorAction::MulticamAngleSelected(qi));
                        }
                    }
                }
            }

            if self.mode == MonitorMode::Program && self.wb_eyedropper_active {
                let banner_rect = Rect::from_center_size(
                    Pos2::new(frame_rect.center().x, frame_rect.top() + 18.0),
                    Vec2::new((frame_rect.width() - 16.0).clamp(160.0, 380.0), 22.0),
                );
                painter.rect_filled(
                    banner_rect,
                    4.0,
                    Color32::from_rgba_unmultiplied(15, 22, 36, 225),
                );
                painter.rect_stroke(
                    banner_rect,
                    4.0,
                    Stroke::new(1.2_f32, Color32::from_rgb(0x58, 0xb4, 0xff)),
                );
                painter.text(
                    banner_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "⌖ WB Damlalığı: Nötr beyaz/gri bir noktaya tıklayın (İptal: Sağ Tık)",
                    egui::FontId::proportional(10.5),
                    Color32::WHITE,
                );

                if let Some(hover_pos) = response.hover_pos() {
                    if frame_rect.contains(hover_pos) {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                        painter.circle_stroke(
                            hover_pos,
                            7.0,
                            Stroke::new(1.5_f32, Color32::from_rgb(0xf5, 0xb9, 0x71)),
                        );
                        painter.circle_stroke(hover_pos, 8.5, Stroke::new(1.0_f32, Color32::BLACK));
                    }
                }
                if response.secondary_clicked() {
                    *action = Some(MonitorAction::CancelWbEyedropper);
                } else if response.clicked() {
                    if let Some(click_pos) = response.interact_pointer_pos() {
                        if frame_rect.contains(click_pos) {
                            let x_norm = ((click_pos.x - frame_rect.left()) / frame_rect.width())
                                .clamp(0.0, 1.0);
                            let y_norm = ((click_pos.y - frame_rect.top()) / frame_rect.height())
                                .clamp(0.0, 1.0);
                            *action = Some(MonitorAction::PickWhiteBalancePixel { x_norm, y_norm });
                        }
                    }
                }
            }

            if self.mode == MonitorMode::Program && action.is_none() {
                if let Some(hover_pos) = response.hover_pos() {
                    if frame_rect.contains(hover_pos) {
                        let x_norm = ((hover_pos.x - frame_rect.left()) / frame_rect.width())
                            .clamp(0.0, 1.0);
                        let y_norm = ((hover_pos.y - frame_rect.top()) / frame_rect.height())
                            .clamp(0.0, 1.0);
                        *action = Some(MonitorAction::HoverPixelSample { x_norm, y_norm });
                    }
                }
            }
        } else {
            painter.text(
                frame_rect.center(),
                egui::Align2::CENTER_CENTER,
                if self.is_decoding {
                    "Kare hazırlanıyor..."
                } else {
                    "Önizleme Yükleniyor..."
                },
                egui::FontId::proportional(12.0),
                Color32::from_rgb(0x80, 0x80, 0x90),
            );
        }

        // Real-time Titler and Text Overlay rendering
        let is_type_tool = self.active_tool == Some(crate::toolbar::EditTool::Type);

        if self.mode == MonitorMode::Program && is_type_tool {
            if let Some(hover_pos) = response.hover_pos() {
                if frame_rect.contains(hover_pos) {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                }
            }

            // Click or drag on monitor frame with Type Tool to place or reposition text
            if let Some(interact_pos) = response.interact_pointer_pos() {
                if frame_rect.contains(interact_pos) {
                    let (target_x, target_y) = if let Some(first_ov) = self.text_overlays.first() {
                        (
                            frame_rect.left() + frame_rect.width() * first_ov.pos_x.clamp(0.0, 1.0),
                            frame_rect.top() + frame_rect.height() * first_ov.pos_y.clamp(0.0, 1.0),
                        )
                    } else {
                        (frame_rect.center().x, frame_rect.center().y)
                    };
                    let input_y = if target_y - 36.0 > frame_rect.top() + 10.0 {
                        target_y - 36.0
                    } else {
                        target_y + 36.0
                    };
                    let input_rect = Rect::from_center_size(
                        Pos2::new(target_x, input_y),
                        Vec2::new(260.0, 28.0),
                    );
                    let is_inside_editor = self.inline_text_edit.is_some()
                        && input_rect.expand(6.0).contains(interact_pos);

                    if !is_inside_editor {
                        let norm_x = ((interact_pos.x - frame_rect.left()) / frame_rect.width())
                            .clamp(0.05, 0.95);
                        let norm_y = ((interact_pos.y - frame_rect.top()) / frame_rect.height())
                            .clamp(0.05, 0.95);

                        if response.clicked() {
                            *action = Some(MonitorAction::TextToolClick {
                                pos_x: norm_x,
                                pos_y: norm_y,
                            });
                        } else if response.dragged() {
                            *action = Some(MonitorAction::TextPositionDrag {
                                pos_x: norm_x,
                                pos_y: norm_y,
                            });
                        } else if response.drag_stopped() {
                            *action = Some(MonitorAction::TextPositionDragEnded);
                        }
                    }
                }
            }
        }

        // Selection tool: pick an on-screen text overlay and drag it directly in the viewport
        let is_select_tool = self.active_tool == Some(crate::toolbar::EditTool::Select);
        if self.mode == MonitorMode::Program && is_select_tool && self.has_content {
            let drag_id = egui::Id::new("program_monitor_text_drag_offset");
            let norm_of = |p: Pos2| -> (f32, f32) {
                (
                    (p.x - frame_rect.left()) / frame_rect.width().max(1.0),
                    (p.y - frame_rect.top()) / frame_rect.height().max(1.0),
                )
            };
            let hit_overlay = |p: Pos2| -> Option<&ActiveTextOverlay> {
                if !frame_rect.contains(p) {
                    return None;
                }
                self.text_overlays.iter().find(|ov| {
                    ov.alpha > 0.001
                        && !ov.text.is_empty()
                        && overlay_hit_rect(painter, frame_rect, ov).contains(p)
                })
            };

            if let Some(hover) = response.hover_pos() {
                if hit_overlay(hover).is_some() {
                    ui.ctx().set_cursor_icon(if response.dragged() {
                        egui::CursorIcon::Grabbing
                    } else {
                        egui::CursorIcon::Grab
                    });
                }
            }

            if response.drag_started() {
                let origin = ui.input(|i| i.pointer.press_origin());
                let grabbed = origin.and_then(|p| hit_overlay(p).map(|ov| (p, ov)));
                if let Some((p, ov)) = grabbed {
                    let (nx, ny) = norm_of(p);
                    let offset = Vec2::new(nx - ov.pos_x, ny - ov.pos_y);
                    ui.memory_mut(|m| m.data.insert_temp(drag_id, Some(offset)));
                    *action = Some(MonitorAction::TextSelectAt {
                        pos_x: ov.pos_x,
                        pos_y: ov.pos_y,
                    });
                } else {
                    ui.memory_mut(|m| m.data.insert_temp(drag_id, None::<Vec2>));
                }
            } else if response.dragged() {
                let offset = ui
                    .memory(|m| m.data.get_temp::<Option<Vec2>>(drag_id))
                    .flatten();
                if let (Some(offset), Some(p)) = (offset, response.interact_pointer_pos()) {
                    let (nx, ny) = norm_of(p);
                    *action = Some(MonitorAction::TextPositionDrag {
                        pos_x: (nx - offset.x).clamp(0.02, 0.98),
                        pos_y: (ny - offset.y).clamp(0.02, 0.98),
                    });
                }
            } else if response.drag_stopped() {
                *action = Some(MonitorAction::TextPositionDragEnded);
            } else if response.double_clicked() {
                if let Some(p) = response.interact_pointer_pos() {
                    if let Some(ov) = hit_overlay(p) {
                        *action = Some(MonitorAction::TextDoubleClick {
                            pos_x: ov.pos_x,
                            pos_y: ov.pos_y,
                        });
                    } else if frame_rect.contains(p) {
                        *action = Some(MonitorAction::ToggleFullscreen);
                    }
                }
            } else if response.clicked() {
                if let Some(p) = response.interact_pointer_pos() {
                    if let Some(ov) = hit_overlay(p) {
                        *action = Some(MonitorAction::TextSelectAt {
                            pos_x: ov.pos_x,
                            pos_y: ov.pos_y,
                        });
                    }
                }
            }

            if response.drag_stopped() {
                ui.memory_mut(|m| m.data.insert_temp(drag_id, None::<Vec2>));
            }
        }

        let rendered_overlays =
            draw_text_overlays_on_rect(painter, frame_rect, &self.text_overlays);

        if is_type_tool && self.mode == MonitorMode::Program {
            for (_idx, text_rect) in &rendered_overlays {
                let pad = 6.0;
                let bbox = text_rect.expand(pad);
                painter.rect_stroke(
                    bbox,
                    2.0,
                    Stroke::new(1.5_f32, Color32::from_rgb(0x38, 0xa0, 0xf0)),
                );
                let dot_r = 3.5;
                let dot_col = Color32::from_rgb(0x81, 0x8c, 0xf8);
                painter.circle_filled(bbox.left_top(), dot_r, dot_col);
                painter.circle_filled(bbox.right_top(), dot_r, dot_col);
                painter.circle_filled(bbox.left_bottom(), dot_r, dot_col);
                painter.circle_filled(bbox.right_bottom(), dot_r, dot_col);
            }
        }

        // Floating inline on-screen text editor
        if is_type_tool && self.mode == MonitorMode::Program {
            if let Some(mut edit_txt) = self.inline_text_edit.take() {
                let (target_x, target_y) = if let Some(first_ov) = self.text_overlays.first() {
                    (
                        frame_rect.left() + frame_rect.width() * first_ov.pos_x.clamp(0.0, 1.0),
                        frame_rect.top() + frame_rect.height() * first_ov.pos_y.clamp(0.0, 1.0),
                    )
                } else {
                    (frame_rect.center().x, frame_rect.center().y)
                };

                let input_w = 260.0;
                let input_h = 28.0;
                let input_y = if target_y - 36.0 > frame_rect.top() + 10.0 {
                    target_y - 36.0
                } else {
                    target_y + 36.0
                };
                let input_rect = Rect::from_center_size(
                    Pos2::new(target_x, input_y),
                    Vec2::new(input_w, input_h),
                );

                painter.rect_filled(input_rect, 4.0, Color32::from_black_alpha(220));
                painter.rect_stroke(
                    input_rect,
                    4.0,
                    Stroke::new(1.5_f32, Color32::from_rgb(0x38, 0xa0, 0xf0)),
                );

                let mut child_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(input_rect.shrink(2.0))
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                let resp = child_ui.add(
                    egui::TextEdit::singleline(&mut edit_txt)
                        .desired_width(input_w - 12.0)
                        .hint_text("Metin girin...")
                        .text_color(Color32::WHITE),
                );
                if resp.changed() {
                    *action = Some(MonitorAction::TextContentEdited(edit_txt));
                } else if resp.lost_focus() {
                    *action = Some(MonitorAction::TextEditingCompleted);
                }
            }
        }

        if self.mode == MonitorMode::Program {
            if let Some(mut mask_ov) = self.active_masks.take() {
                handle_mask_overlay_interaction(
                    ui,
                    painter,
                    frame_rect,
                    response,
                    &mut mask_ov,
                    action,
                );
            }
        }

        // High contrast frame border
        painter.rect_stroke(
            frame_rect,
            0.0,
            Stroke::new(1.0_f32, Color32::from_rgb(0x38, 0x38, 0x46)),
        );
        draw_corner_guides(painter, frame_rect);
        if self.safe_zones {
            draw_safe_zones(painter, frame_rect, self.aspect_ratio < 1.0);
        }
    }
}
