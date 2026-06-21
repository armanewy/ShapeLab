//! Asset panel entry points.

#![allow(dead_code)]

pub(crate) mod candidate_gallery;
pub(crate) mod history;
pub(crate) mod inspector;
pub(crate) mod part_tree;

use crate::asset::{AssetAppCommand, AssetUiState};

/// Render the novice-facing asset side panels into command DTOs.
pub(crate) fn show_asset_panels(ui: &mut egui::Ui, state: &AssetUiState) -> Vec<AssetAppCommand> {
    let mut commands = Vec::new();
    ui.columns(2, |columns| {
        egui::ScrollArea::vertical().show(&mut columns[0], |ui| {
            commands.extend(part_tree::show(ui, state));
            ui.separator();
            commands.extend(history::show(ui, state));
        });
        egui::ScrollArea::vertical().show(&mut columns[1], |ui| {
            commands.extend(inspector::show(ui, state));
        });
    });
    ui.separator();
    commands.extend(candidate_gallery::show(ui, state));
    commands
}
