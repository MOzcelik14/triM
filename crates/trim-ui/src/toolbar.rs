//! Professional NLE Vertical Toolbar (Selection, Razor, Ripple, Slip, Pen, Hand, Type).

use egui::{Color32, Stroke, Ui, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditTool {
    #[default]
    Select, // V
    TrackSelect, // A
    Ripple,      // B
    Razor,       // C
    Slip,        // Y
    Pen,         // P
    Hand,        // H
    Type,        // T
}

impl EditTool {
    pub fn symbol(&self) -> &'static str {
        match self {
            EditTool::Select => "↖",
            EditTool::TrackSelect => "⇥",
            EditTool::Ripple => "◀▶",
            EditTool::Razor => "✂",
            EditTool::Slip => "↔",
            EditTool::Pen => "✎",
            EditTool::Hand => "✋",
            EditTool::Type => "T",
        }
    }

    pub fn tooltip(&self) -> &'static str {
        match self {
            EditTool::Select => "Seçim Aracı (V)\nKlipleri seçin, taşıyın ve kırpın",
            EditTool::TrackSelect => "İleri İz Seçme Aracı (A)\nTıklanan klip ve sonrasındaki tüm klipleri seçin",
            EditTool::Ripple => "Dalgalanma Düzenleme (B)\nKlipleri diğer kliplerin yerini kaydırarak düzenleyin",
            EditTool::Razor => "Jilet / Kesme Aracı (C)\nKlipleri tıkladığınız noktadan ikiye bölün",
            EditTool::Slip => "Kaydırma Aracı (Y)\nKlibin zaman çizgisindeki yeri sabitken kaynak içeriğini kaydırın",
            EditTool::Pen => "Kalem / Anahtar Kare Aracı (P)\nKlip özelliklerine anahtar kare noktaları ekleyin",
            EditTool::Hand => "El Aracı (H)\nZaman çizgisinde tutup sürükleyerek gezinin",
            EditTool::Type => "Metin Aracı (T)\nZaman çizgisine doğrudan başlık/yazı ekleyin",
        }
    }
}

pub fn render_vertical_toolbar(ui: &mut Ui, current_tool: &mut EditTool) {
    ui.vertical_centered(|ui| {
        ui.add_space(4.0);

        let tools = [
            EditTool::Select,
            EditTool::TrackSelect,
            EditTool::Ripple,
            EditTool::Razor,
            EditTool::Slip,
            EditTool::Pen,
            EditTool::Hand,
            EditTool::Type,
        ];

        for tool in tools {
            let tooltip = tool.tooltip();
            let is_selected = *current_tool == tool;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(28.0, 28.0), egui::Sense::click());
            if resp.clicked() {
                *current_tool = tool;
            }
            let is_hovered = resp.hovered();
            let _ = resp.on_hover_text(tooltip);

            let (bg_color, fg_color, border_stroke) = if is_selected {
                (
                    Color32::from_rgb(0x28, 0x5a, 0x9e),
                    Color32::WHITE,
                    Stroke::new(1.5_f32, Color32::from_rgb(0x4a, 0x8a, 0xe8)),
                )
            } else if is_hovered {
                (
                    Color32::from_rgb(0x28, 0x28, 0x34),
                    Color32::WHITE,
                    Stroke::new(1.0_f32, Color32::from_rgb(0x40, 0x40, 0x52)),
                )
            } else {
                (
                    Color32::from_rgb(0x1a, 0x1a, 0x22),
                    Color32::from_rgb(0xd0, 0xd0, 0xdc),
                    Stroke::new(1.0_f32, Color32::from_rgb(0x2d, 0x2d, 0x38)),
                )
            };

            let painter = ui.painter();
            painter.rect(rect, 4.0, bg_color, border_stroke);

            let center = rect.center();
            let stroke = Stroke::new(1.5_f32, fg_color);

            match tool {
                EditTool::Select => {
                    // Arrow cursor
                    let pts = vec![
                        center + Vec2::new(-5.0, -7.0),
                        center + Vec2::new(-5.0, 6.0),
                        center + Vec2::new(-1.0, 2.0),
                        center + Vec2::new(2.0, 7.0),
                        center + Vec2::new(4.5, 5.5),
                        center + Vec2::new(1.5, 1.0),
                        center + Vec2::new(6.0, 1.0),
                    ];
                    painter.add(egui::Shape::convex_polygon(pts, fg_color, Stroke::NONE));
                }
                EditTool::TrackSelect => {
                    // Vertical bar + double arrow
                    painter.line_segment(
                        [
                            center + Vec2::new(-6.0, -6.0),
                            center + Vec2::new(-6.0, 6.0),
                        ],
                        stroke,
                    );
                    painter.line_segment(
                        [center + Vec2::new(-3.0, -5.0), center + Vec2::new(1.0, 0.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [center + Vec2::new(1.0, 0.0), center + Vec2::new(-3.0, 5.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [center + Vec2::new(2.0, -5.0), center + Vec2::new(6.0, 0.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [center + Vec2::new(6.0, 0.0), center + Vec2::new(2.0, 5.0)],
                        stroke,
                    );
                }
                EditTool::Ripple => {
                    // Opposing brackets / edit point
                    painter.line_segment(
                        [
                            center + Vec2::new(-6.0, -6.0),
                            center + Vec2::new(-6.0, 6.0),
                        ],
                        stroke,
                    );
                    painter.line_segment(
                        [
                            center + Vec2::new(-6.0, 0.0),
                            center + Vec2::new(-1.0, -4.0),
                        ],
                        stroke,
                    );
                    painter.line_segment(
                        [center + Vec2::new(-6.0, 0.0), center + Vec2::new(-1.0, 4.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [center + Vec2::new(6.0, -6.0), center + Vec2::new(6.0, 6.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [center + Vec2::new(6.0, 0.0), center + Vec2::new(1.0, -4.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [center + Vec2::new(6.0, 0.0), center + Vec2::new(1.0, 4.0)],
                        stroke,
                    );
                }
                EditTool::Razor => {
                    // Scissors / razor cut
                    painter.line_segment(
                        [center + Vec2::new(-5.0, -6.0), center + Vec2::new(5.0, 6.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [center + Vec2::new(5.0, -6.0), center + Vec2::new(-5.0, 6.0)],
                        stroke,
                    );
                    painter.circle_stroke(center + Vec2::new(-4.0, 5.0), 2.2, stroke);
                    painter.circle_stroke(center + Vec2::new(4.0, 5.0), 2.2, stroke);
                }
                EditTool::Slip => {
                    // Two-way horizontal slip arrows
                    painter.line_segment(
                        [center + Vec2::new(-6.0, 0.0), center + Vec2::new(6.0, 0.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [
                            center + Vec2::new(-6.0, 0.0),
                            center + Vec2::new(-3.0, -3.0),
                        ],
                        stroke,
                    );
                    painter.line_segment(
                        [center + Vec2::new(-6.0, 0.0), center + Vec2::new(-3.0, 3.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [center + Vec2::new(6.0, 0.0), center + Vec2::new(3.0, -3.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [center + Vec2::new(6.0, 0.0), center + Vec2::new(3.0, 3.0)],
                        stroke,
                    );
                }
                EditTool::Pen => {
                    // Pen nib / keyframe diamond
                    painter.line_segment(
                        [center + Vec2::new(-4.0, 5.0), center + Vec2::new(4.0, -5.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [center + Vec2::new(2.0, -6.0), center + Vec2::new(6.0, -2.0)],
                        stroke,
                    );
                    painter.circle_filled(center + Vec2::new(-4.0, 5.0), 2.0, fg_color);
                }
                EditTool::Hand => {
                    painter.text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        "H",
                        egui::FontId::monospace(13.0),
                        fg_color,
                    );
                }
                EditTool::Type => {
                    painter.text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        "T",
                        egui::FontId::proportional(14.0),
                        fg_color,
                    );
                }
            }

            ui.add_space(3.0);
        }
    });
}
