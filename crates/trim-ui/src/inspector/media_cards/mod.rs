mod audio;
mod compositing_masks;
mod titler;

pub(in crate::inspector) use audio::render_audio_card;
pub(in crate::inspector) use compositing_masks::{render_compositing_card, render_masks_card};
pub(in crate::inspector) use titler::render_titler_card;

use super::{inspector_card, prop_row, InspectorAction, ACCENT_BLUE};
use egui::{DragValue, Ui};
use trim_core::clip::Clip;

pub(in crate::inspector) fn render_transitions_card(
    ui: &mut Ui,
    clip: &mut Clip,
    action: &mut Option<InspectorAction>,
) {
    inspector_card(
        ui,
        "◈",
        "Giriş / Çıkış ve Geçişler",
        ACCENT_BLUE,
        |_| {},
        |ui| {
            prop_row(ui, "Giriş Kararma", |ui| {
                if ui
                    .add(
                        DragValue::new(&mut clip.fade_in)
                            .speed(0.1)
                            .range(0.0..=60.0)
                            .suffix(" sn"),
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Giriş Geçişi", |ui| {
                egui::ComboBox::from_id_salt("trans_in")
                    .selected_text(Clip::transition_label(clip.transition_in.as_deref()))
                    .width(ui.available_width().max(110.0))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(clip.transition_in.is_none(), "Yok (Düz)")
                            .clicked()
                        {
                            clip.transition_in = None;
                            *action = Some(InspectorAction::ClipModified);
                        }
                        for (key, label) in Clip::SUPPORTED_TRANSITIONS {
                            let is_sel = clip.transition_in.as_deref() == Some(*key);
                            if ui.selectable_label(is_sel, *label).clicked() {
                                clip.transition_in = Some(key.to_string());
                                *action = Some(InspectorAction::ClipModified);
                            }
                        }
                    });
            });

            prop_row(ui, "Çıkış Kararma", |ui| {
                if ui
                    .add(
                        DragValue::new(&mut clip.fade_out)
                            .speed(0.1)
                            .range(0.0..=60.0)
                            .suffix(" sn"),
                    )
                    .changed()
                {
                    *action = Some(InspectorAction::ClipModified);
                }
            });

            prop_row(ui, "Çıkış Geçişi", |ui| {
                egui::ComboBox::from_id_salt("trans_out")
                    .selected_text(Clip::transition_label(clip.transition_out.as_deref()))
                    .width(ui.available_width().max(110.0))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(clip.transition_out.is_none(), "Yok (Düz)")
                            .clicked()
                        {
                            clip.transition_out = None;
                            *action = Some(InspectorAction::ClipModified);
                        }
                        for (key, label) in Clip::SUPPORTED_TRANSITIONS {
                            let is_sel = clip.transition_out.as_deref() == Some(*key);
                            if ui.selectable_label(is_sel, *label).clicked() {
                                clip.transition_out = Some(key.to_string());
                                *action = Some(InspectorAction::ClipModified);
                            }
                        }
                    });
            });
        },
    );
}
