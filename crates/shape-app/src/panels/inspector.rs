//! Parameter inspector and search controls.

#![allow(dead_code)]

use std::collections::BTreeSet;

use egui::{RichText, Slider};
use shape_core::{
    NodeId, NodeKind, ParamDescriptor, ParamGroup, ParamPath, ShapeDocument, ShapeNode,
    enumerate_parameters, get_scalar,
};
use shape_search::{ExplorationMode, TargetScope};

use crate::commands::AppCommand;
use crate::state::AppState;

const MIN_PROPOSAL_COUNT: usize = 1;
const MAX_PROPOSAL_COUNT: usize = 512;
const MIN_RESULT_COUNT: usize = 1;
const MAX_RESULT_COUNT: usize = 12;
const DEFAULT_PROPOSAL_COUNT: usize = 64;
const DEFAULT_RESULT_COUNT: usize = 6;

/// Stable order used by the inspector and search group toggles.
pub(crate) const PARAMETER_GROUP_ORDER: [ParamGroup; 5] = [
    ParamGroup::Form,
    ParamGroup::Placement,
    ParamGroup::Rotation,
    ParamGroup::Scale,
    ParamGroup::Blend,
];

/// UI-local controls that do not yet have an AppState contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectorPanelState {
    pub proposal_count: usize,
    pub result_count: usize,
}

impl Default for InspectorPanelState {
    fn default() -> Self {
        Self {
            proposal_count: DEFAULT_PROPOSAL_COUNT,
            result_count: DEFAULT_RESULT_COUNT,
        }
    }
}

/// A deterministic group of parameters for display and tests.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParameterGroupSection {
    pub group: ParamGroup,
    pub descriptors: Vec<ParamDescriptor>,
}

/// Render the selected-node inspector and search controls.
pub(crate) fn show(
    ui: &mut egui::Ui,
    state: &AppState,
    panel_state: &mut InspectorPanelState,
) -> Vec<AppCommand> {
    let mut commands = Vec::new();

    let Ok(document) = state.project.current_document() else {
        ui.heading("Inspector");
        ui.weak("The current project revision is unavailable.");
        return commands;
    };

    ui.heading("Inspector");
    commands.extend(render_selected_node(ui, document, state.selected_node));
    ui.separator();
    commands.extend(render_search_controls(ui, state, panel_state));
    commands
}

/// Build grouped parameter descriptors for one node in stable display order.
#[must_use]
pub(crate) fn grouped_parameter_sections(
    document: &ShapeDocument,
    node_id: NodeId,
) -> Vec<ParameterGroupSection> {
    let descriptors = enumerate_parameters(document)
        .into_iter()
        .filter(|descriptor| descriptor.path.node == node_id)
        .collect::<Vec<_>>();

    PARAMETER_GROUP_ORDER
        .into_iter()
        .filter_map(|group| {
            let group_descriptors = descriptors
                .iter()
                .filter(|descriptor| descriptor.group == group)
                .cloned()
                .collect::<Vec<_>>();
            if group_descriptors.is_empty() {
                None
            } else {
                Some(ParameterGroupSection {
                    group,
                    descriptors: group_descriptors,
                })
            }
        })
        .collect()
}

/// Create a scalar-edit command when a visible value really changed.
#[must_use]
pub(crate) fn scalar_command(
    descriptor: &ParamDescriptor,
    current: f32,
    proposed: f32,
    locked: bool,
) -> Option<AppCommand> {
    if locked || !proposed.is_finite() || !scalar_value_changed(descriptor, current, proposed) {
        return None;
    }
    Some(AppCommand::SetScalar {
        path: descriptor.path.clone(),
        value: proposed.clamp(descriptor.minimum, descriptor.maximum),
    })
}

/// Create a lock command only when the lock state changed.
#[must_use]
pub(crate) fn lock_command(path: &ParamPath, was_locked: bool, locked: bool) -> Option<AppCommand> {
    (was_locked != locked).then(|| AppCommand::ToggleLock {
        path: path.clone(),
        locked,
    })
}

/// Create a target-scope command only when the scope changed.
#[must_use]
pub(crate) fn target_scope_command(
    current: TargetScope,
    proposed: TargetScope,
) -> Option<AppCommand> {
    (current != proposed).then_some(AppCommand::SetTargetScope(proposed))
}

/// Create a parameter-group command only when the checkbox changed.
#[must_use]
pub(crate) fn parameter_group_command(
    group: ParamGroup,
    enabled_groups: &BTreeSet<ParamGroup>,
    proposed_enabled: bool,
) -> Option<AppCommand> {
    (enabled_groups.contains(&group) != proposed_enabled).then_some(AppCommand::SetParameterGroup {
        group,
        enabled: proposed_enabled,
    })
}

/// Create an exploration-mode command only when the mode changed.
#[must_use]
pub(crate) fn exploration_mode_command(
    current: ExplorationMode,
    proposed: ExplorationMode,
) -> Option<AppCommand> {
    (current != proposed).then_some(AppCommand::SetExplorationMode(proposed))
}

/// Create a seed command only when the seed changed.
#[must_use]
pub(crate) fn seed_command(current: u64, proposed: u64) -> Option<AppCommand> {
    (current != proposed).then_some(AppCommand::SetSeed(proposed))
}

/// Keep local search counts within safe display bounds.
pub(crate) fn sanitize_search_counts(panel_state: &mut InspectorPanelState) {
    panel_state.proposal_count = panel_state
        .proposal_count
        .clamp(MIN_PROPOSAL_COUNT, MAX_PROPOSAL_COUNT);
    panel_state.result_count = panel_state
        .result_count
        .clamp(MIN_RESULT_COUNT, MAX_RESULT_COUNT)
        .min(panel_state.proposal_count)
        .min(MAX_RESULT_COUNT);
}

fn render_selected_node(
    ui: &mut egui::Ui,
    document: &ShapeDocument,
    selected_node: Option<NodeId>,
) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    let Some(node_id) = selected_node else {
        ui.label(RichText::new("Whole Model").strong());
        ui.label("Choose a part in the shape list to edit its values.");
        ui.small(format!("{} parts in this model", document.nodes.len()));
        return commands;
    };

    let Some(node) = document.nodes.get(&node_id) else {
        ui.colored_label(
            ui.visuals().error_fg_color,
            format!("Selected part {} is missing.", node_id.0),
        );
        return commands;
    };

    render_node_summary(ui, node);
    let sections = grouped_parameter_sections(document, node_id);
    if sections.is_empty() {
        ui.weak("This part has no editable values.");
        return commands;
    }

    for section in sections {
        ui.collapsing(group_label(section.group), |ui| {
            ui.label(group_help(section.group));
            for descriptor in &section.descriptors {
                commands.extend(render_parameter_row(ui, document, descriptor));
            }
        });
    }
    commands
}

fn render_node_summary(ui: &mut egui::Ui, node: &ShapeNode) {
    ui.label(RichText::new(&node.name).strong());
    ui.small(format!(
        "ID {} | {}",
        node.id.0,
        node_kind_label(&node.kind)
    ));
    if node.tags.is_empty() {
        ui.weak("No tags");
    } else {
        ui.horizontal_wrapped(|ui| {
            ui.small("Tags");
            for tag in &node.tags {
                ui.label(RichText::new(format!("#{tag}")).small());
            }
        });
    }
}

fn render_parameter_row(
    ui: &mut egui::Ui,
    document: &ShapeDocument,
    descriptor: &ParamDescriptor,
) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    let Ok(current) = get_scalar(document, &descriptor.path) else {
        return commands;
    };

    let was_locked = document.locks.contains(&descriptor.path);
    let mut locked = was_locked;
    let mut proposed = current;
    let mut value_changed = false;

    ui.horizontal(|ui| {
        ui.label(&descriptor.label)
            .on_hover_text(parameter_help(descriptor));
        ui.add_space(4.0);
        ui.monospace(format!("{current:.3}"));
        ui.add_space(4.0);

        ui.add_enabled_ui(!was_locked, |ui| {
            let response = if should_use_slider(descriptor) {
                ui.add(
                    Slider::new(&mut proposed, descriptor.minimum..=descriptor.maximum)
                        .step_by(f64::from(descriptor.step))
                        .show_value(false),
                )
            } else {
                ui.add(
                    egui::DragValue::new(&mut proposed)
                        .range(descriptor.minimum..=descriptor.maximum)
                        .speed(f64::from(descriptor.step)),
                )
            };
            value_changed = response.changed();
        });

        let lock_response = ui
            .checkbox(&mut locked, "Lock")
            .on_hover_text("Keep this value fixed while new directions are generated.");
        if lock_response.changed() {
            commands.extend(lock_command(&descriptor.path, was_locked, locked));
        }
    });

    if value_changed {
        commands.extend(scalar_command(descriptor, current, proposed, was_locked));
    }

    commands
}

fn render_search_controls(
    ui: &mut egui::Ui,
    state: &AppState,
    panel_state: &mut InspectorPanelState,
) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    sanitize_search_counts(panel_state);

    ui.heading("Directions");
    ui.label("Choose what may change, then generate options.");

    commands.extend(render_target_scope_controls(
        ui,
        state.selected_target_scope,
    ));
    commands.extend(render_parameter_group_controls(
        ui,
        &state.enabled_param_groups,
    ));
    commands.extend(render_mode_controls(ui, state.exploration_mode));
    commands.extend(render_seed_control(ui, state.seed));
    render_count_controls(ui, panel_state);
    commands.extend(render_generation_buttons(
        ui,
        state.active_generation.is_some(),
    ));

    commands
}

fn render_target_scope_controls(ui: &mut egui::Ui, current_scope: TargetScope) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    ui.label("Change target");
    ui.horizontal(|ui| {
        let mut scope = current_scope;
        ui.radio_value(&mut scope, TargetScope::Selected, "Selected")
            .on_hover_text("Only the selected part may change.");
        ui.radio_value(&mut scope, TargetScope::Subtree, "Selected group")
            .on_hover_text("The selected part and its child parts may change.");
        ui.radio_value(&mut scope, TargetScope::WholeModel, "Whole Model")
            .on_hover_text("Any unlocked value in the model may change.");
        commands.extend(target_scope_command(current_scope, scope));
    });
    commands
}

fn render_parameter_group_controls(
    ui: &mut egui::Ui,
    enabled_groups: &BTreeSet<ParamGroup>,
) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    ui.label("Values allowed to change");
    ui.horizontal_wrapped(|ui| {
        for group in PARAMETER_GROUP_ORDER {
            let mut enabled = enabled_groups.contains(&group);
            let response = ui
                .checkbox(&mut enabled, group_label(group))
                .on_hover_text(group_help(group));
            if response.changed() {
                commands.extend(parameter_group_command(group, enabled_groups, enabled));
            }
        }
    });
    commands
}

fn render_mode_controls(ui: &mut egui::Ui, current_mode: ExplorationMode) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    ui.label("Style");
    ui.horizontal(|ui| {
        let mut mode = current_mode;
        ui.radio_value(&mut mode, ExplorationMode::Refine, "Refine")
            .on_hover_text("Small, careful changes near the current model.");
        ui.radio_value(&mut mode, ExplorationMode::Explore, "Explore")
            .on_hover_text("Broader changes that look for different directions.");
        commands.extend(exploration_mode_command(current_mode, mode));
    });
    commands
}

fn render_seed_control(ui: &mut egui::Ui, current_seed: u64) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    let mut seed = current_seed;
    ui.horizontal(|ui| {
        ui.label("Seed")
            .on_hover_text("Use the same seed to repeat the same generated directions.");
        let response = ui.add(egui::DragValue::new(&mut seed).speed(1.0));
        if response.changed() {
            commands.extend(seed_command(current_seed, seed));
        }
    });
    commands
}

fn render_count_controls(ui: &mut egui::Ui, panel_state: &mut InspectorPanelState) {
    ui.horizontal(|ui| {
        ui.label("Proposals")
            .on_hover_text("More proposals can find broader options but take longer.");
        let mut proposals = panel_state.proposal_count;
        if ui
            .add(
                egui::DragValue::new(&mut proposals).range(MIN_PROPOSAL_COUNT..=MAX_PROPOSAL_COUNT),
            )
            .changed()
        {
            panel_state.proposal_count = proposals;
            sanitize_search_counts(panel_state);
        }

        ui.label("Results")
            .on_hover_text("How many final direction cards should be shown.");
        let mut results = panel_state.result_count;
        if ui
            .add(egui::DragValue::new(&mut results).range(MIN_RESULT_COUNT..=MAX_RESULT_COUNT))
            .changed()
        {
            panel_state.result_count = results;
            sanitize_search_counts(panel_state);
        }
    });
}

fn render_generation_buttons(ui: &mut egui::Ui, generation_active: bool) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!generation_active, egui::Button::new("Generate Directions"))
            .on_hover_text("Create new model directions using the enabled values.")
            .clicked()
        {
            commands.push(AppCommand::GenerateDirections);
        }

        if generation_active && ui.button("Cancel").clicked() {
            commands.push(AppCommand::CancelActiveGeneration);
        }
    });
    commands
}

fn scalar_value_changed(descriptor: &ParamDescriptor, current: f32, proposed: f32) -> bool {
    let tolerance = descriptor.step.max(1.0e-5) * 1.0e-3;
    (current - proposed).abs() > tolerance
}

fn should_use_slider(descriptor: &ParamDescriptor) -> bool {
    let range = descriptor.maximum - descriptor.minimum;
    range.is_finite() && range <= 20.0
}

fn group_label(group: ParamGroup) -> &'static str {
    match group {
        ParamGroup::Form => "Shape",
        ParamGroup::Placement => "Position",
        ParamGroup::Rotation => "Rotation",
        ParamGroup::Scale => "Size",
        ParamGroup::Blend => "Soft joining",
    }
}

fn group_help(group: ParamGroup) -> &'static str {
    match group {
        ParamGroup::Form => "Dimensions that change the part's outline.",
        ParamGroup::Placement => "Where the part sits in the model.",
        ParamGroup::Rotation => "How the part is turned.",
        ParamGroup::Scale => "Stretching applied to the part.",
        ParamGroup::Blend => "How softly grouped parts merge together.",
    }
}

fn node_kind_label(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Primitive(primitive) => match primitive {
            shape_core::PrimitiveKind::Sphere { .. } => "Sphere",
            shape_core::PrimitiveKind::RoundedBox { .. } => "Rounded box",
            shape_core::PrimitiveKind::Capsule { .. } => "Capsule",
            shape_core::PrimitiveKind::Cylinder { .. } => "Cylinder",
            shape_core::PrimitiveKind::Torus { .. } => "Ring",
        },
        NodeKind::Union { .. } => "Group",
        NodeKind::SmoothUnion { .. } => "Soft group",
        NodeKind::Difference { .. } => "Cut group",
        NodeKind::Intersection { .. } => "Overlap group",
    }
}

fn parameter_help(descriptor: &ParamDescriptor) -> &'static str {
    match descriptor.path.key.as_str() {
        "transform.translation.x" | "transform.translation.y" | "transform.translation.z" => {
            "Move this part along one axis."
        }
        "transform.rotation_degrees.x"
        | "transform.rotation_degrees.y"
        | "transform.rotation_degrees.z" => "Turn this part around one axis.",
        "transform.scale.x" | "transform.scale.y" | "transform.scale.z" => {
            "Stretch or shrink this part along one axis."
        }
        "primitive.radius" => "Change the part's radius.",
        "primitive.half_extents.x" | "primitive.half_extents.y" | "primitive.half_extents.z" => {
            "Change half of this part's size along one axis."
        }
        "primitive.roundness" => "Round or sharpen this part's edges.",
        "primitive.half_length" | "primitive.half_height" => "Change half of this part's length.",
        "primitive.major_radius" | "primitive.minor_radius" => {
            "Change one radius of the ring shape."
        }
        "csg.smoothness" => "Change how softly grouped parts blend.",
        _ => "Editable model value.",
    }
}
