//! Parameter inspector and search controls.

#![allow(dead_code)]

use std::collections::BTreeSet;

use egui::{Color32, RichText, Slider};
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
        ui.weak("The current history step is unavailable.");
        return commands;
    };

    ui.heading("Inspector");
    commands.extend(render_selected_node(ui, document, state.selected_node));
    ui.separator();
    commands.extend(render_search_controls(ui, state, document, panel_state));
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

/// Build the reset command when there is an active built-in preset.
#[must_use]
pub(crate) fn reset_current_preset_command(active_preset: bool) -> Option<AppCommand> {
    active_preset.then_some(AppCommand::ResetCurrentPreset)
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
        ui.label("No single part is selected. Options can still change the whole model.");
        ui.small(format!("{} parts in this model", document.nodes.len()));
        return commands;
    };

    let Some(node) = document.nodes.get(&node_id) else {
        ui.colored_label(
            ui.visuals().error_fg_color,
            format!(
                "The selected part {} is no longer in this version.",
                node_id.0
            ),
        );
        return commands;
    };

    render_node_summary(ui, node);
    let sections = grouped_parameter_sections(document, node_id);
    if sections.is_empty() {
        ui.weak("This part is a container, so it has no direct values to edit.");
        return commands;
    }

    ui.small("Use Keep beside a value to lock it so generated options leave it unchanged.");
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
        "Part {} | {}",
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
        ui.label(display_parameter_label(descriptor))
            .on_hover_text(parameter_help(descriptor));
        ui.add_space(4.0);
        ui.monospace(format!("{current:.3}"))
            .on_hover_text("Current value before any new option is chosen.");
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
            .checkbox(&mut locked, "Keep")
            .on_hover_text("Checked values stay exactly as they are during generation.");
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
    document: &ShapeDocument,
    panel_state: &mut InspectorPanelState,
) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    sanitize_search_counts(panel_state);

    ui.heading("Options");
    ui.label("Choose what can change, then ask for new options.");

    commands.extend(render_target_scope_controls(
        ui,
        state.selected_target_scope,
    ));
    ui.small(change_target_summary(
        document,
        state.selected_node,
        state.selected_target_scope,
    ));
    commands.extend(render_parameter_group_controls(
        ui,
        &state.enabled_param_groups,
    ));
    commands.extend(render_mode_controls(ui, state.exploration_mode));
    commands.extend(render_seed_control(ui, state.seed));
    commands.extend(render_count_controls(
        ui,
        panel_state,
        state.proposal_count,
        state.result_count,
    ));
    commands.extend(render_generation_buttons(
        ui,
        state.active_generation.is_some(),
    ));
    commands.extend(render_preset_reset(
        ui,
        state.active_preset.is_some(),
        state.active_generation.is_some(),
    ));

    commands
}

fn render_target_scope_controls(ui: &mut egui::Ui, current_scope: TargetScope) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    ui.label("What can change next");
    ui.horizontal(|ui| {
        let mut scope = current_scope;
        ui.radio_value(&mut scope, TargetScope::Selected, "This part")
            .on_hover_text("Only the selected part may change.");
        ui.radio_value(&mut scope, TargetScope::Subtree, "This part and contents")
            .on_hover_text("The selected part and the parts inside it may change.");
        ui.radio_value(&mut scope, TargetScope::WholeModel, "Everything")
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
    ui.label("Kinds of values allowed to change");
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
    ui.label("Change style");
    ui.horizontal(|ui| {
        let mut mode = current_mode;
        let refine_text = mode_label(
            "Refine",
            current_mode == ExplorationMode::Refine,
            Color32::from_rgb(73, 132, 211),
        );
        let explore_text = mode_label(
            "Explore",
            current_mode == ExplorationMode::Explore,
            Color32::from_rgb(176, 117, 41),
        );
        ui.radio_value(&mut mode, ExplorationMode::Refine, refine_text)
            .on_hover_text("Small nudges that stay close to the current model.");
        ui.radio_value(&mut mode, ExplorationMode::Explore, explore_text)
            .on_hover_text("Bigger jumps that look for a noticeably different option.");
        commands.extend(exploration_mode_command(current_mode, mode));
    });
    ui.small("Refine is for careful nudges. Explore is for bolder alternatives.");
    commands
}

fn render_seed_control(ui: &mut egui::Ui, current_seed: u64) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    let mut seed = current_seed;
    ui.horizontal(|ui| {
        ui.label("Seed")
            .on_hover_text("Use the same seed to repeat the same generated options.");
        let response = ui.add(egui::DragValue::new(&mut seed).speed(1.0));
        if response.changed() {
            commands.extend(seed_command(current_seed, seed));
        }
    });
    commands
}

fn render_count_controls(
    ui: &mut egui::Ui,
    panel_state: &mut InspectorPanelState,
    current_proposal_count: usize,
    current_result_count: usize,
) -> Vec<AppCommand> {
    if panel_state.proposal_count != current_proposal_count
        || panel_state.result_count != current_result_count
    {
        panel_state.proposal_count = current_proposal_count;
        panel_state.result_count = current_result_count;
        sanitize_search_counts(panel_state);
    }

    let before = panel_state.clone();
    ui.horizontal(|ui| {
        ui.label("Options tried")
            .on_hover_text("Trying more options can find broader results but takes longer.");
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

        ui.label("Cards shown")
            .on_hover_text("How many final option cards should appear in the gallery.");
        let mut results = panel_state.result_count;
        if ui
            .add(egui::DragValue::new(&mut results).range(MIN_RESULT_COUNT..=MAX_RESULT_COUNT))
            .changed()
        {
            panel_state.result_count = results;
            sanitize_search_counts(panel_state);
        }
    });

    if before != *panel_state {
        vec![AppCommand::SetSearchBudget {
            proposal_count: panel_state.proposal_count,
            result_count: panel_state.result_count,
        }]
    } else {
        Vec::new()
    }
}

fn render_generation_buttons(ui: &mut egui::Ui, generation_active: bool) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!generation_active, egui::Button::new("Generate Options"))
            .on_hover_text("Create option cards using only the allowed values.")
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

fn render_preset_reset(
    ui: &mut egui::Ui,
    active_preset: bool,
    generation_active: bool,
) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    let enabled = active_preset && !generation_active;
    if ui
        .add_enabled(enabled, egui::Button::new("Reset to Preset"))
        .on_hover_text(
            "Start over from the original built-in preset. This clears generated options.",
        )
        .clicked()
    {
        commands.extend(reset_current_preset_command(active_preset));
    }
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
        ParamGroup::Form => "Outline",
        ParamGroup::Placement => "Position",
        ParamGroup::Rotation => "Turning",
        ParamGroup::Scale => "Stretch",
        ParamGroup::Blend => "Blending",
    }
}

fn group_help(group: ParamGroup) -> &'static str {
    match group {
        ParamGroup::Form => "Values that change the part's outline.",
        ParamGroup::Placement => "Where the part sits in the model.",
        ParamGroup::Rotation => "How the part is tilted or turned.",
        ParamGroup::Scale => "How much the part is stretched.",
        ParamGroup::Blend => "How softly nearby parts flow into each other.",
    }
}

fn node_kind_label(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Primitive(primitive) => match primitive {
            shape_core::PrimitiveKind::Sphere { .. } => "Round part",
            shape_core::PrimitiveKind::RoundedBox { .. } => "Rounded block",
            shape_core::PrimitiveKind::Capsule { .. } => "Long rounded part",
            shape_core::PrimitiveKind::Cylinder { .. } => "Round column",
            shape_core::PrimitiveKind::Torus { .. } => "Ring",
        },
        NodeKind::Union { .. } => "Part group",
        NodeKind::SmoothUnion { .. } => "Blended group",
        NodeKind::Difference { .. } => "Hollowed group",
        NodeKind::Intersection { .. } => "Overlap group",
    }
}

fn parameter_help(descriptor: &ParamDescriptor) -> &'static str {
    match descriptor.path.key.as_str() {
        "transform.translation.x" => "Move this part left or right.",
        "transform.translation.y" => "Move this part up or down.",
        "transform.translation.z" => "Move this part forward or back.",
        "transform.rotation_degrees.x" => "Tilt this part forward or back.",
        "transform.rotation_degrees.y" => "Turn this part left or right.",
        "transform.rotation_degrees.z" => "Spin this part clockwise or counterclockwise.",
        "transform.scale.x" => "Stretch or shrink this part's width.",
        "transform.scale.y" => "Stretch or shrink this part's height.",
        "transform.scale.z" => "Stretch or shrink this part's depth.",
        "primitive.radius" => "Make this rounded part larger or smaller.",
        "primitive.half_extents.x" | "primitive.half_extents.y" | "primitive.half_extents.z" => {
            "Change this block's width, height, or depth."
        }
        "primitive.roundness" => "Round or sharpen this part's edges.",
        "primitive.half_length" => "Make this long part longer or shorter.",
        "primitive.half_height" => "Make this column taller or shorter.",
        "primitive.major_radius" | "primitive.minor_radius" => {
            "Change the size or thickness of the ring."
        }
        "csg.smoothness" => "Change how softly grouped parts blend together.",
        _ => "Editable model value.",
    }
}

fn mode_label(text: &'static str, selected: bool, color: Color32) -> RichText {
    let label = RichText::new(text).color(color);
    if selected { label.strong() } else { label }
}

fn change_target_summary(
    document: &ShapeDocument,
    selected_node: Option<NodeId>,
    scope: TargetScope,
) -> String {
    let selected_name = selected_node
        .and_then(|node| document.nodes.get(&node))
        .map(|node| node.name.as_str())
        .unwrap_or("the whole model");

    match scope {
        TargetScope::Selected => format!("Next options may change only {selected_name}."),
        TargetScope::Subtree => {
            format!("Next options may change {selected_name} and the parts inside it.")
        }
        TargetScope::WholeModel => {
            "Next options may change any unlocked value in the model.".to_owned()
        }
    }
}

fn display_parameter_label(descriptor: &ParamDescriptor) -> &str {
    match descriptor.path.key.as_str() {
        "transform.translation.x" => "Left / Right",
        "transform.translation.y" => "Up / Down",
        "transform.translation.z" => "Forward / Back",
        "transform.rotation_degrees.x" => "Tilt",
        "transform.rotation_degrees.y" => "Turn",
        "transform.rotation_degrees.z" => "Spin",
        "transform.scale.x" => "Width stretch",
        "transform.scale.y" => "Height stretch",
        "transform.scale.z" => "Depth stretch",
        "primitive.radius" => "Overall size",
        "primitive.half_extents.x" => "Width",
        "primitive.half_extents.y" => "Height",
        "primitive.half_extents.z" => "Depth",
        "primitive.roundness" => "Edge softness",
        "primitive.half_length" => "Length",
        "primitive.half_height" => "Height",
        "primitive.major_radius" => "Ring size",
        "primitive.minor_radius" => "Ring thickness",
        "csg.smoothness" => "Blend amount",
        _ => descriptor.label.as_str(),
    }
}
