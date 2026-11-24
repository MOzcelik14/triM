use crate::theme::*;
use egui::{Color32, Rect, Response, RichText, Vec2};
use trim_core::clip::ColorLabel;
use trim_core::project::Project;
use trim_core::track::TrackType;

use super::geometry::*;
use super::state::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_release_and_context_menu(
    response: &Response,
    rect: Rect,
    project: &mut Project,
    state: &mut TimelineCanvasState,
    pointer: &egui::PointerState,
    action: &mut Option<CanvasAction>,
) {
    // Release mouse
    if pointer.primary_released() && state.drag_mode != DragMode::None {
        match state.drag_mode {
            DragMode::Rubberband => {
                state.rubberband_rect = None;
                state.rubberband_start = None;
            }
            DragMode::Move => {
                state.snap_guide_x = None;
                if state.selected_clip_pairs.len() > 1 {
                    let mut moves = Vec::new();
                    for (t_id, c_id, orig_in) in &state.batch_orig_ins {
                        if let Some(tr) = project.timeline.get_track_by_id(t_id) {
                            if let Some(cl) = tr.get_clip_by_id(c_id) {
                                if (cl.timeline_in - orig_in).abs() > 1e-4 {
                                    moves.push((
                                        t_id.clone(),
                                        c_id.clone(),
                                        *orig_in,
                                        cl.timeline_in,
                                    ));
                                }
                            }
                        }
                    }
                    if !moves.is_empty() {
                        *action = Some(CanvasAction::BatchClipsMoved(moves));
                    }
                } else if let (Some(tr_id), Some(cl_id)) =
                    (&state.drag_track_id, &state.drag_clip_id)
                {
                    let target_track_id = state
                        .drag_target_track_id
                        .clone()
                        .unwrap_or_else(|| tr_id.clone());
                    if let Some(tr) = project.timeline.get_track_by_id(tr_id) {
                        if let Some(cl) = tr.get_clip_by_id(cl_id) {
                            if (cl.timeline_in - state.drag_orig_in).abs() > 1e-4
                                || &target_track_id != tr_id
                            {
                                *action = Some(CanvasAction::ClipMoved {
                                    old_track_id: tr_id.clone(),
                                    new_track_id: target_track_id,
                                    clip_id: cl_id.clone(),
                                    old_in: state.drag_orig_in,
                                    new_in: cl.timeline_in,
                                });
                            }
                        }
                    }
                }
                state.drag_target_track_id = None;
            }
            DragMode::Roll => {
                if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let dt = ((pos.x - state.drag_start_x) / state.pixels_per_second) as f64;
                        if dt.abs() > 0.005 {
                            if let Some(tr) = project.timeline.get_track_by_id(tr_id) {
                                if let Some(idx) = tr.clips.iter().position(|c| &c.id == cl_id) {
                                    if idx + 1 < tr.clips.len() {
                                        *action = Some(CanvasAction::ClipRollTrimmed {
                                            track_id: tr_id.clone(),
                                            left_clip_id: cl_id.clone(),
                                            right_clip_id: tr.clips[idx + 1].id.clone(),
                                            delta_secs: dt,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            DragMode::Slide => {
                if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let dt = ((pos.x - state.drag_start_x) / state.pixels_per_second) as f64;
                        if dt.abs() > 0.005 {
                            if let Some(tr) = project.timeline.get_track_by_id(tr_id) {
                                if let Some(idx) = tr.clips.iter().position(|c| &c.id == cl_id) {
                                    let left_clip_id = if idx > 0 {
                                        Some(tr.clips[idx - 1].id.clone())
                                    } else {
                                        None
                                    };
                                    let right_clip_id = if idx + 1 < tr.clips.len() {
                                        Some(tr.clips[idx + 1].id.clone())
                                    } else {
                                        None
                                    };
                                    *action = Some(CanvasAction::ClipSlideTrimmed {
                                        track_id: tr_id.clone(),
                                        clip_id: cl_id.clone(),
                                        left_clip_id,
                                        right_clip_id,
                                        delta_secs: dt,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            DragMode::TrimIn | DragMode::TrimOut | DragMode::Slip => {
                if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id) {
                    if let Some(tr) = project.timeline.get_track_by_id(tr_id) {
                        if let Some(cl) = tr.get_clip_by_id(cl_id) {
                            let new_bounds = (
                                cl.timeline_in,
                                cl.timeline_out,
                                cl.source_in,
                                cl.effective_source_out(),
                            );
                            let old_bounds = (
                                state.drag_orig_in,
                                state.drag_orig_out,
                                state.drag_orig_sin,
                                state.drag_orig_sout,
                            );
                            let mut companions = Vec::new();
                            for (comp_tr_id, comp_cl_id, orig_in, orig_out, orig_sin, orig_sout) in
                                &state.drag_companions
                            {
                                if let Some(comp_tr) = project.timeline.get_track_by_id(comp_tr_id)
                                {
                                    if let Some(comp_cl) = comp_tr.get_clip_by_id(comp_cl_id) {
                                        let comp_old = (*orig_in, *orig_out, *orig_sin, *orig_sout);
                                        let comp_new = (
                                            comp_cl.timeline_in,
                                            comp_cl.timeline_out,
                                            comp_cl.source_in,
                                            comp_cl.effective_source_out(),
                                        );
                                        companions.push((
                                            comp_tr_id.clone(),
                                            comp_cl_id.clone(),
                                            comp_old,
                                            comp_new,
                                        ));
                                    }
                                }
                            }
                            if new_bounds != old_bounds || !companions.is_empty() {
                                *action = Some(CanvasAction::ClipTrimmed {
                                    track_id: tr_id.clone(),
                                    clip_id: cl_id.clone(),
                                    old_bounds,
                                    new_bounds,
                                    companions,
                                });
                            }
                        }
                    }
                }
            }
            DragMode::RippleIn | DragMode::RippleOut => {
                if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id) {
                    if let Some(tr) = project.timeline.get_track_by_id(tr_id) {
                        if let Some(cl) = tr.get_clip_by_id(cl_id) {
                            let new_bounds = (
                                cl.timeline_in,
                                cl.timeline_out,
                                cl.source_in,
                                cl.effective_source_out(),
                            );
                            let old_bounds = (
                                state.drag_orig_in,
                                state.drag_orig_out,
                                state.drag_orig_sin,
                                state.drag_orig_sout,
                            );
                            let mut companions = Vec::new();
                            for (comp_tr_id, comp_cl_id, orig_in, orig_out, orig_sin, orig_sout) in
                                &state.drag_companions
                            {
                                if let Some(comp_tr) = project.timeline.get_track_by_id(comp_tr_id)
                                {
                                    if let Some(comp_cl) = comp_tr.get_clip_by_id(comp_cl_id) {
                                        let comp_old = (*orig_in, *orig_out, *orig_sin, *orig_sout);
                                        let comp_new = (
                                            comp_cl.timeline_in,
                                            comp_cl.timeline_out,
                                            comp_cl.source_in,
                                            comp_cl.effective_source_out(),
                                        );
                                        companions.push((
                                            comp_tr_id.clone(),
                                            comp_cl_id.clone(),
                                            comp_old,
                                            comp_new,
                                        ));
                                    }
                                }
                            }
                            let mut shifted_clips = Vec::new();
                            for (t_id, c_id, orig_in) in &state.batch_orig_ins {
                                if let Some(t) = project.timeline.get_track_by_id(t_id) {
                                    if let Some(c) = t.get_clip_by_id(c_id) {
                                        if (c.timeline_in - *orig_in).abs() > 1e-4 {
                                            shifted_clips.push((
                                                t_id.clone(),
                                                c_id.clone(),
                                                *orig_in,
                                                c.timeline_in,
                                            ));
                                        }
                                    }
                                }
                            }
                            if new_bounds != old_bounds
                                || !shifted_clips.is_empty()
                                || !companions.is_empty()
                            {
                                *action = Some(CanvasAction::ClipRippleTrimmed {
                                    track_id: tr_id.clone(),
                                    clip_id: cl_id.clone(),
                                    old_bounds,
                                    new_bounds,
                                    shifted_clips,
                                    companions,
                                });
                            }
                        }
                    }
                }
            }
            DragMode::FadeIn => {
                if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id) {
                    if let Some(tr) = project.timeline.get_track_by_id(tr_id) {
                        if let Some(cl) = tr.get_clip_by_id(cl_id) {
                            if (cl.fade_in - state.drag_orig_fade).abs() > 1e-4 {
                                *action = Some(CanvasAction::SetClipFade {
                                    track_id: tr_id.clone(),
                                    clip_id: cl_id.clone(),
                                    is_fade_in: true,
                                    old_fade: state.drag_orig_fade,
                                    new_fade: cl.fade_in,
                                });
                            } else {
                                *action = Some(CanvasAction::ClipModified);
                            }
                        }
                    }
                }
            }
            DragMode::FadeOut => {
                if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id) {
                    if let Some(tr) = project.timeline.get_track_by_id(tr_id) {
                        if let Some(cl) = tr.get_clip_by_id(cl_id) {
                            if (cl.fade_out - state.drag_orig_fade).abs() > 1e-4 {
                                *action = Some(CanvasAction::SetClipFade {
                                    track_id: tr_id.clone(),
                                    clip_id: cl_id.clone(),
                                    is_fade_in: false,
                                    old_fade: state.drag_orig_fade,
                                    new_fade: cl.fade_out,
                                });
                            } else {
                                *action = Some(CanvasAction::ClipModified);
                            }
                        }
                    }
                }
            }
            DragMode::VolumeRubberband => {
                if let (Some(tr_id), Some(cl_id)) = (&state.drag_track_id, &state.drag_clip_id) {
                    if let Some(tr) = project.timeline.get_track_by_id_mut(tr_id) {
                        if let Some(cl) = tr.get_clip_by_id_mut(cl_id) {
                            if state.drag_kf_index.is_some()
                                || !state.drag_orig_keyframes.is_empty()
                            {
                                if let Some(kfs) = cl.keyframes.get_mut("volume") {
                                    kfs.sort_by(|a, b| {
                                        a.time
                                            .partial_cmp(&b.time)
                                            .unwrap_or(std::cmp::Ordering::Equal)
                                    });
                                }
                                let new_kfs =
                                    cl.keyframes.get("volume").cloned().unwrap_or_default();
                                if new_kfs != state.drag_orig_keyframes {
                                    *action = Some(CanvasAction::SetClipKeyframes {
                                        track_id: tr_id.clone(),
                                        clip_id: cl_id.clone(),
                                        property: "volume".to_string(),
                                        old_keyframes: state.drag_orig_keyframes.clone(),
                                        new_keyframes: new_kfs,
                                    });
                                }
                            } else if (cl.volume - state.drag_orig_vol).abs() > 1e-4 {
                                *action = Some(CanvasAction::SetClipVolume {
                                    track_id: tr_id.clone(),
                                    clip_id: cl_id.clone(),
                                    old_volume: state.drag_orig_vol,
                                    new_volume: cl.volume,
                                });
                            }
                        }
                    }
                }
                state.drag_kf_index = None;
                state.drag_orig_keyframes.clear();
            }
            _ => {}
        }

        state.drag_mode = DragMode::None;
        state.drag_clip_id = None;
        state.drag_track_id = None;
        state.snap_guide_x = None;
        state.drag_kf_index = None;
        state.drag_orig_keyframes.clear();
        state.drag_companions.clear();
    }

    // Context Menu on right click
    response.context_menu(|ui| {
        ui.set_min_width(260.0);
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        ui.spacing_mut().button_padding = Vec2::new(10.0, 5.0);
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);

        // 1. If a gap is selected (or hovered right click on a gap)
        if state.selected_gap.is_none() && state.selected_clip_pairs.is_empty() {
            if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                if let Some(gap) = find_gap_at_pos(project, state, rect, pos) {
                    state.selected_gap = Some(gap);
                }
            }
        }

        if let Some((tr_id, gap_start, gap_end)) = state.selected_gap.clone() {
            let gap_dur = (gap_end - gap_start).max(0.0);
            ui.label(
                RichText::new(format!("⏩ Zaman Çizgisi Boşluğu ({:.2}s)", gap_dur))
                    .color(COLOR_AMBER)
                    .strong(),
            );
            ui.label(
                RichText::new(format!("Aralık: {:.2}s - {:.2}s", gap_start, gap_end))
                    .small()
                    .color(Color32::from_rgb(0xa0, 0xa0, 0xb0)),
            );
            ui.separator();

            let is_locked = project
                .timeline
                .get_track_by_id(&tr_id)
                .map(|t| t.locked)
                .unwrap_or(false);

            ui.add_enabled_ui(!is_locked, |ui| {
                if ui
                    .button("⏩ Boşluğu Kapat (Ripple Delete Gap)  [Del / Shift+Del]")
                    .clicked()
                {
                    *action = Some(CanvasAction::CloseGap {
                        track_id: None,
                        gap_start,
                        gap_end,
                    });
                    ui.close_menu();
                }
            });

            if ui
                .button("⏩ Yalnızca Bu Kanalın Boşluğunu Kapat")
                .clicked()
            {
                *action = Some(CanvasAction::CloseGap {
                    track_id: Some(tr_id.clone()),
                    gap_start,
                    gap_end,
                });
                ui.close_menu();
            }

            ui.separator();
            if ui.button("Boşluk Seçimini Kaldır (Esc)").clicked() {
                state.selected_gap = None;
                ui.close_menu();
            }
            return;
        }

        // 2. If clip(s) are selected
        if let Some((tr_id, cl_id)) = state.selected_clip_pairs.first().cloned() {
            let is_locked = project
                .timeline
                .get_track_by_id(&tr_id)
                .map(|t| t.locked)
                .unwrap_or(false);

            if is_locked {
                ui.label(
                    RichText::new("[K] Kanal Kilitli (Düzenlenemez)")
                        .color(Color32::from_rgb(0xe0, 0x7a, 0x38))
                        .italics(),
                );
                ui.separator();
            }

            ui.add_enabled_ui(!is_locked, |ui| {
                if ui.button("Hız ve Süre (Speed & Duration)...").clicked() {
                    *action = Some(CanvasAction::PromptSpeed(tr_id.clone(), cl_id.clone()));
                    ui.close_menu();
                }

                if let Some(tr) = project.timeline.get_track_by_id(&tr_id) {
                    if tr.track_type == TrackType::Video
                        && ui.button("Sesi Ayır (Detach Audio)").clicked()
                    {
                        *action = Some(CanvasAction::DetachAudio(tr_id.clone(), cl_id.clone()));
                        ui.close_menu();
                    }
                }

                if ui.button("Klibi Böl (S)").clicked() {
                    *action = Some(CanvasAction::SplitAtPlayhead);
                    ui.close_menu();
                }

                ui.separator();

                // Color labels submenu
                ui.menu_button("■ Renk Etiketi (Color Label)", |ui| {
                    ui.set_min_width(200.0);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    ui.spacing_mut().button_padding = Vec2::new(8.0, 4.0);

                    for color in ColorLabel::ALL {
                        let mut btn_text = RichText::new(color.label());
                        if let Some([r, g, b]) = color.color_rgb() {
                            btn_text = btn_text.color(Color32::from_rgb(r, g, b));
                        }
                        if ui.button(btn_text).clicked() {
                            *action = Some(CanvasAction::SetClipColor {
                                track_id: tr_id.clone(),
                                clip_id: cl_id.clone(),
                                color: *color,
                            });
                            ui.close_menu();
                        }
                    }
                });

                // Link / Unlink toggle button
                let is_linked = project
                    .timeline
                    .get_track_by_id(&tr_id)
                    .and_then(|t| t.get_clip_by_id(&cl_id))
                    .and_then(|c| c.linked_group_id.as_ref())
                    .is_some();
                if is_linked {
                    if ui
                        .button("↔ Klip Bağını Kaldır (Unlink)  [Ctrl+L]")
                        .clicked()
                    {
                        *action = Some(CanvasAction::ToggleLinkSelected);
                        ui.close_menu();
                    }
                } else if state.selected_clip_pairs.len() > 1
                    && ui.button("↔ Klipleri Bağla (Link)  [Ctrl+L]").clicked()
                {
                    *action = Some(CanvasAction::ToggleLinkSelected);
                    ui.close_menu();
                }

                if state.selected_clip_pairs.len() >= 2
                    && ui
                        .button("∿ Ses Dalga Formuna Göre Hizala (Audio Sync)")
                        .clicked()
                {
                    *action = Some(CanvasAction::AlignSelectedByAudio);
                    ui.close_menu();
                }

                if state.selected_clip_pairs.len() >= 2
                    && ui
                        .button("⏱ Timecode ile Hizala (Multicam Timecode Sync)")
                        .clicked()
                {
                    *action = Some(CanvasAction::AlignSelectedByTimecode);
                    ui.close_menu();
                }

                if ui
                    .button("⊞ Bileşik Klip Oluştur (Compound Clip / Nest)")
                    .clicked()
                {
                    *action = Some(CanvasAction::CreateCompoundClip);
                    ui.close_menu();
                }

                if state.selected_clip_pairs.len() >= 2
                    && ui
                        .button("↔ Bitişik Kesikleri Birleştir (Join Through-Edits)")
                        .clicked()
                {
                    *action = Some(CanvasAction::JoinThroughEdits);
                    ui.close_menu();
                }

                if ui.button("+ Kopyala (Ctrl+C)").clicked() {
                    *action = Some(CanvasAction::CopySelected);
                    ui.close_menu();
                }

                if ui.button("+ Yapıştır (Ctrl+V)").clicked() {
                    *action = Some(CanvasAction::PasteClipboard);
                    ui.close_menu();
                }

                if ui
                    .button("≡ Özellikleri Seçerek Yapıştır... (Ctrl+Alt+V)")
                    .clicked()
                {
                    *action = Some(CanvasAction::OpenPasteAttributesDialog);
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Sil (Del)").clicked() {
                    *action = Some(CanvasAction::DeleteSelected);
                    ui.close_menu();
                }

                if ui.button("⏪ Boşluksuz Sil (Shift+Del)").clicked() {
                    *action = Some(CanvasAction::RippleDeleteSelected);
                    ui.close_menu();
                }
            });
        }
    });
}
