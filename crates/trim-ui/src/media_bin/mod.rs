//! Premiere Pro style Media Bin panel with Grid/List views, search filtering,
//! folder/bin organization, thumbnail cards with duration badges, and Source Monitor double-click loading.

use egui::{TextureHandle, Ui, Vec2};
use std::collections::HashMap;
use trim_core::project::Project;

mod grid_view;
mod list_view;
mod toolbar;
mod types;
pub use types::*;

use grid_view::render_media_grid;
use list_view::render_media_list;
use toolbar::render_toolbar_and_filter;

pub(super) fn render_move_to_bin_menu(
    ui: &mut Ui,
    project: &Project,
    item: &trim_core::media::MediaItem,
    m_id: &str,
) -> Option<MediaBinAction> {
    let mut move_act = None;
    ui.menu_button("▸ Klasöre Taşı", |ui| {
        ui.set_min_width(190.0);
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        ui.spacing_mut().button_padding = Vec2::new(8.0, 4.0);

        let is_in_root = item.bin_id.is_none();
        let root_btn = ui.add_enabled(
            !is_in_root,
            egui::Button::new(if is_in_root {
                "✓ Ana Havuz (Kök Dizin)"
            } else {
                "▸ Ana Havuz (Kök Dizin)"
            }),
        );
        if root_btn.clicked() {
            move_act = Some(MediaBinAction::MoveToBin {
                media_id: m_id.to_string(),
                bin_id: None,
            });
            ui.close_menu();
        }

        if !project.bins.is_empty() {
            ui.separator();
            for bin in &project.bins {
                let is_in_this = item.bin_id.as_deref() == Some(&bin.id);
                let label = if is_in_this {
                    format!("✓ {}", bin.name)
                } else {
                    format!("▸ {}", bin.name)
                };
                let bin_btn = ui.add_enabled(!is_in_this, egui::Button::new(label));
                if bin_btn.clicked() {
                    move_act = Some(MediaBinAction::MoveToBin {
                        media_id: m_id.to_string(),
                        bin_id: Some(bin.id.clone()),
                    });
                    ui.close_menu();
                }
            }
        }
    });
    move_act
}

#[allow(clippy::too_many_arguments)]
pub fn render_media_bin(
    ui: &mut Ui,
    project: &Project,
    selected_media_id: &mut Option<String>,
    search_query: &mut String,
    is_grid_view: &mut bool,
    thumb_size: &mut f32,
    current_bin_id: &Option<String>,
    filter_type: &mut MediaFilterType,
    textures: &HashMap<String, TextureHandle>,
) -> Option<MediaBinAction> {
    let mut action = None;

    let Some(filtered) = render_toolbar_and_filter(
        ui,
        project,
        search_query,
        is_grid_view,
        thumb_size,
        current_bin_id,
        filter_type,
        &mut action,
    ) else {
        return action;
    };

    // Precompute how many times each media item is used in the active timeline
    let mut media_usage_counts: HashMap<&str, usize> = HashMap::new();
    for track in &project.timeline.tracks {
        for clip in &track.clips {
            if !clip.media_id.is_empty() {
                *media_usage_counts
                    .entry(clip.media_id.as_str())
                    .or_insert(0) += 1;
            }
        }
    }

    if *is_grid_view {
        render_media_grid(
            ui,
            project,
            selected_media_id,
            *thumb_size,
            textures,
            &filtered.matching_bins,
            &filtered.matching_sequences,
            &filtered.filtered_items,
            &media_usage_counts,
            &mut action,
        );
    } else {
        render_media_list(
            ui,
            project,
            selected_media_id,
            &filtered.matching_bins,
            &filtered.matching_sequences,
            &filtered.filtered_items,
            &media_usage_counts,
            &mut action,
        );
    }

    action
}
