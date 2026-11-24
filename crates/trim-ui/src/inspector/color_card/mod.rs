mod creative_looks;
mod curves_and_film;
mod presets;
mod primary;

use crate::inspector::{inspector_card, InspectorAction, ACCENT_BLUE};
use creative_looks::render_creative_looks_section;
use curves_and_film::render_curves_and_film_section;
use egui::{Button, Color32, RichText, Rounding, Ui};
use primary::render_primary_section;
use trim_commands::commands::ClipColorGradingState;
use trim_core::clip::{BrightnessContrastPreset, Clip, WhiteBalancePreset};

pub(super) fn record_cg_start(
    ui: &mut Ui,
    cg_drag_id: egui::Id,
    orig: &mut Option<ClipColorGradingState>,
    clip: &Clip,
) {
    if orig.is_none() {
        let snap = ClipColorGradingState::from_clip(clip);
        *orig = Some(snap.clone());
        ui.data_mut(|d| d.insert_temp(cg_drag_id, snap));
    }
}

pub(super) fn check_cg_stop(
    ui: &mut Ui,
    cg_drag_id: egui::Id,
    orig: &mut Option<ClipColorGradingState>,
    track_id: &str,
    clip: &Clip,
    action: &mut Option<InspectorAction>,
) {
    if let Some(old_state) = orig.take() {
        let new_state = ClipColorGradingState::from_clip(clip);
        if old_state != new_state {
            *action = Some(InspectorAction::SetColorGrading {
                track_id: track_id.to_string(),
                clip_id: clip.id.clone(),
                old_state: Box::new(old_state),
                new_state: Box::new(new_state),
            });
        }
        ui.data_mut(|d| d.remove::<ClipColorGradingState>(cg_drag_id));
    }
}

pub(super) fn render_color_card(
    ui: &mut Ui,
    track_id: &str,
    clip: &mut Clip,
    action: &mut Option<InspectorAction>,
) {
    let has_color_changes = clip.has_color_adjustments()
        || clip.wb_preset != WhiteBalancePreset::Custom
        || clip.bc_preset != BrightnessContrastPreset::Neutral;

    let cg_drag_id = ui.make_persistent_id(format!("cg_drag_{}", clip.id));
    let mut orig_cg: Option<ClipColorGradingState> = ui.data_mut(|d| d.get_temp(cg_drag_id));
    let reset_colors = std::cell::Cell::new(false);
    let trigger_shot_match = std::cell::Cell::new(false);
    inspector_card(
        ui,
        "◈",
        "Renk Düzenleme, Presetler & 3D LUT",
        ACCENT_BLUE,
        |ui| {
            if ui
                .add(
                    Button::new(
                        RichText::new("⌖ Eşle")
                            .size(10.0)
                            .color(Color32::from_rgb(0xe8, 0xa8, 0x38)),
                    )
                    .fill(Color32::from_rgb(0x28, 0x22, 0x18))
                    .rounding(Rounding::same(4.0)),
                )
                .on_hover_text(
                    "Shot Match: Seçili referans kareye (veya nötr ton dengesine) göre pozlama, kontrast ve sıcaklığı otomatik eşle",
                )
                .clicked()
            {
                trigger_shot_match.set(true);
            }
            if has_color_changes
                && ui
                    .add(
                        Button::new(
                            RichText::new("↺ Sıfırla")
                                .size(10.0)
                                .color(Color32::from_rgb(0xc8, 0xd0, 0xe4)),
                        )
                        .fill(Color32::from_rgb(0x22, 0x26, 0x34))
                        .rounding(Rounding::same(4.0)),
                    )
                    .on_hover_text(
                        "Tüm beyaz dengesi, kontrast, renk görünümü ve LUT ayarlarını sıfırla",
                    )
                    .clicked()
            {
                reset_colors.set(true);
            }
        },
        |ui| {
            if trigger_shot_match.get() {
                *action = Some(InspectorAction::ShotMatch {
                    track_id: track_id.to_string(),
                    clip_id: clip.id.clone(),
                });
            }
            if reset_colors.get() {
                let old_state = ClipColorGradingState::from_clip(clip);
                clip.reset_color_grading();
                clip.color_wheels = None;
                clip.custom_curves = None;
                clip.hsl_curves = None;
                clip.hsl_qualifier = None;
                clip.halation = 0.0;
                clip.bloom = 0.0;
                clip.chromatic_aberration = 0.0;
                let new_state = ClipColorGradingState::from_clip(clip);
                *action = Some(InspectorAction::SetColorGrading {
                    track_id: track_id.to_string(),
                    clip_id: clip.id.clone(),
                    old_state: Box::new(old_state),
                    new_state: Box::new(new_state),
                });
            }

            render_primary_section(ui, track_id, clip, cg_drag_id, &mut orig_cg, action);
            render_creative_looks_section(ui, track_id, clip, action);
            render_curves_and_film_section(ui, track_id, clip, action);
        },
    );
}
