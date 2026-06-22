//! Beginner-facing asset inspector.

#![allow(dead_code)]

use egui::{RichText, Slider};

use crate::asset::{
    AssetAppCommand, AssetCutControl, AssetCutOperation, AssetEdgeTreatment, AssetLockTarget,
    AssetParameter, AssetParameterGroup, AssetPart, AssetUiState, OperationId, ParameterId,
    PartDefinitionId, PartInstanceId,
};

/// One reflected beginner parameter group.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParameterGroupSection {
    pub group: AssetParameterGroup,
    pub parameters: Vec<AssetParameter>,
}

/// Render selected-part controls and beginner parameter groups.
pub(crate) fn show(ui: &mut egui::Ui, state: &AssetUiState) -> Vec<AssetAppCommand> {
    let mut commands = Vec::new();
    ui.heading("Inspector");

    let Some(selected) = state.selected_part() else {
        ui.weak(empty_inspector_message());
        return commands;
    };

    ui.label(RichText::new(&selected.name).strong());
    ui.small(format!("Definition: {}", selected.definition_name))
        .on_hover_text(format!("definition.{}", selected.definition.0));
    commands.extend(render_part_locks(ui, state, selected));
    commands.extend(render_cut_operations(ui, state, selected));

    for section in grouped_parameter_sections(&state.parameters) {
        let has_selected_parameters = section
            .parameters
            .iter()
            .any(|parameter| parameter.part == Some(selected.id));
        if !has_selected_parameters {
            continue;
        }

        ui.collapsing(section.group.label(), |ui| {
            ui.label(section.group.help());
            for parameter in section
                .parameters
                .iter()
                .filter(|parameter| parameter.part == Some(selected.id))
            {
                commands.extend(render_parameter_row(ui, parameter));
            }
        });
    }

    commands.extend(render_optional_presence(ui, state, selected));
    commands
}

/// Group reflected parameters in the required beginner-facing order.
#[must_use]
pub(crate) fn grouped_parameter_sections(
    parameters: &[AssetParameter],
) -> Vec<ParameterGroupSection> {
    AssetParameterGroup::all()
        .into_iter()
        .filter_map(|group| {
            let group_parameters = parameters
                .iter()
                .filter(|parameter| parameter.group == group)
                .cloned()
                .collect::<Vec<_>>();
            if group_parameters.is_empty() {
                None
            } else {
                Some(ParameterGroupSection {
                    group,
                    parameters: group_parameters,
                })
            }
        })
        .collect()
}

/// Emit a cut operation selection only when it changes.
#[must_use]
pub(crate) fn cut_operation_select_command(
    current: Option<OperationId>,
    selected: OperationId,
) -> Option<AssetAppCommand> {
    (current != Some(selected)).then_some(AssetAppCommand::SelectCutOperation(Some(selected)))
}

/// Emit a cut scalar command only for finite, changed values.
#[must_use]
pub(crate) fn cut_operation_scalar_command(
    operation: &AssetCutOperation,
    control: &AssetCutControl,
    proposed: f32,
    locked: bool,
) -> Option<AssetAppCommand> {
    if locked || !proposed.is_finite() {
        return None;
    }
    let clamped = proposed.clamp(control.minimum, control.maximum);
    cut_control_value_changed(control, clamped).then_some(AssetAppCommand::SetCutOperationScalar {
        definition: operation.definition,
        operation: operation.operation,
        field: control.field.clone(),
        value: clamped,
    })
}

/// Emit a cut removal command.
#[must_use]
pub(crate) fn cut_operation_remove_command(
    operation: &AssetCutOperation,
    locked: bool,
) -> Option<AssetAppCommand> {
    (!locked).then_some(AssetAppCommand::RemoveCutOperation {
        definition: operation.definition,
        operation: operation.operation,
    })
}

/// Emit an edge-treatment scalar command only for finite, changed values.
#[must_use]
pub(crate) fn edge_treatment_scalar_command(
    treatment: &AssetEdgeTreatment,
    control: &AssetCutControl,
    proposed: f32,
    locked: bool,
) -> Option<AssetAppCommand> {
    if locked || !proposed.is_finite() {
        return None;
    }
    let clamped = proposed.clamp(control.minimum, control.maximum);
    cut_control_value_changed(control, clamped).then_some(AssetAppCommand::SetCutOperationScalar {
        definition: treatment.definition,
        operation: treatment.operation,
        field: control.field.clone(),
        value: clamped,
    })
}

/// Emit an edge-treatment removal command.
#[must_use]
pub(crate) fn edge_treatment_remove_command(
    treatment: &AssetEdgeTreatment,
    locked: bool,
) -> Option<AssetAppCommand> {
    (!locked).then_some(AssetAppCommand::RemoveCutOperation {
        definition: treatment.definition,
        operation: treatment.operation,
    })
}

/// Emit a scalar command only for unlocked, finite, meaningfully changed values.
#[must_use]
pub(crate) fn parameter_command(
    parameter: &AssetParameter,
    proposed: f32,
) -> Option<AssetAppCommand> {
    if parameter.locked || !proposed.is_finite() {
        return None;
    }
    let clamped = proposed.clamp(parameter.minimum, parameter.maximum);
    parameter_value_changed(parameter, clamped).then_some(AssetAppCommand::SetParameter {
        parameter: parameter.id,
        value: clamped,
    })
}

/// Emit a parameter lock command only when the state changes.
#[must_use]
pub(crate) fn parameter_lock_command(
    parameter: &AssetParameter,
    locked: bool,
) -> Option<AssetAppCommand> {
    (parameter.locked != locked).then_some(AssetAppCommand::SetLock {
        target: AssetLockTarget::Parameter(parameter.id),
        locked,
    })
}

/// Emit an optional-part command only when the enabled state changes and the part is editable.
#[must_use]
pub(crate) fn part_presence_command(
    part: &AssetPart,
    proposed_enabled: bool,
    locked: bool,
) -> Option<AssetAppCommand> {
    (part.optional && !locked && part.enabled != proposed_enabled).then_some(
        AssetAppCommand::ToggleOptionalPart {
            instance: part.id,
            enabled: proposed_enabled,
        },
    )
}

/// Emit an instance lock command only when the state changes.
#[must_use]
pub(crate) fn part_lock_command(
    part: PartInstanceId,
    was_locked: bool,
    locked: bool,
) -> Option<AssetAppCommand> {
    (was_locked != locked).then_some(AssetAppCommand::SetLock {
        target: AssetLockTarget::Instance(part),
        locked,
    })
}

/// Emit a subtree lock command only when the state changes.
#[must_use]
pub(crate) fn subtree_lock_command(
    part: PartInstanceId,
    was_locked: bool,
    locked: bool,
) -> Option<AssetAppCommand> {
    (was_locked != locked).then_some(AssetAppCommand::SetLock {
        target: AssetLockTarget::Subtree(part),
        locked,
    })
}

/// Emit a topology lock command only when the state changes.
#[must_use]
pub(crate) fn topology_lock_command(
    definition: PartDefinitionId,
    was_locked: bool,
    locked: bool,
) -> Option<AssetAppCommand> {
    (was_locked != locked).then_some(AssetAppCommand::SetLock {
        target: AssetLockTarget::Topology(definition),
        locked,
    })
}

/// Empty-state copy for tests and the UI.
#[must_use]
pub(crate) fn empty_inspector_message() -> &'static str {
    "Select a part to edit its beginner controls."
}

fn render_parameter_row(ui: &mut egui::Ui, parameter: &AssetParameter) -> Vec<AssetAppCommand> {
    let mut commands = Vec::new();
    let mut value = parameter.value;
    let mut locked = parameter.locked;

    ui.horizontal(|ui| {
        ui.label(&parameter.label).on_hover_text(format!(
            "{}\n{}",
            parameter.technical_name, parameter.beginner_description
        ));
        ui.add_space(4.0);
        ui.monospace(format!("{:.3}", parameter.value));
        let response = ui.add_enabled(
            !parameter.locked,
            Slider::new(&mut value, parameter.minimum..=parameter.maximum)
                .step_by(f64::from(parameter.step.max(f32::EPSILON)))
                .show_value(false),
        );
        if response.changed() {
            commands.extend(parameter_command(parameter, value));
        }
        if parameter.topology_changing {
            ui.small("Topology")
                .on_hover_text("This control can change generated topology.");
        }
        if ui
            .checkbox(&mut locked, "Lock")
            .on_hover_text("Locked values are preserved by generated directions.")
            .changed()
        {
            commands.extend(parameter_lock_command(parameter, locked));
        }
    });

    commands
}

fn render_cut_operations(
    ui: &mut egui::Ui,
    state: &AssetUiState,
    selected: &AssetPart,
) -> Vec<AssetAppCommand> {
    let mut commands = Vec::new();
    let operations = state
        .cut_operations
        .iter()
        .filter(|operation| operation.part == selected.id)
        .collect::<Vec<_>>();
    if operations.is_empty() {
        return commands;
    }

    ui.collapsing("Cuts", |ui| {
        for operation in operations {
            ui.horizontal(|ui| {
                let response = ui
                    .selectable_label(operation.selected, &operation.label)
                    .on_hover_text(format!(
                        "{} | operation.{}",
                        operation.kind.label(),
                        operation.operation.0
                    ));
                if response.clicked() {
                    commands.extend(cut_operation_select_command(
                        state.selected_cut_operation,
                        operation.operation,
                    ));
                }
                let topology_locked = state.topology_locks.contains(&operation.definition);
                if operation.selected
                    && ui
                        .add_enabled(!topology_locked, egui::Button::new("Remove"))
                        .on_hover_text("Remove this cut and dependent edge treatments.")
                        .clicked()
                {
                    commands.extend(cut_operation_remove_command(operation, topology_locked));
                }
            });

            if operation.selected {
                for control in &operation.controls {
                    commands.extend(render_cut_control_row(ui, state, operation, control));
                }
                commands.extend(render_edge_treatments(ui, state, operation));
            }
        }
    });
    commands
}

fn render_cut_control_row(
    ui: &mut egui::Ui,
    state: &AssetUiState,
    operation: &AssetCutOperation,
    control: &AssetCutControl,
) -> Vec<AssetAppCommand> {
    let mut commands = Vec::new();
    let mut value = control.value;
    let locked = control.topology_changing && state.topology_locks.contains(&operation.definition);

    ui.horizontal(|ui| {
        ui.label(&control.label).on_hover_text(format!(
            "definition.{}.operation.{}.{}",
            operation.definition.0, operation.operation.0, control.field
        ));
        ui.add_space(4.0);
        ui.monospace(format!("{:.3}", control.value));
        let response = ui.add_enabled(
            !locked,
            Slider::new(&mut value, control.minimum..=control.maximum)
                .step_by(f64::from(control.step.max(f32::EPSILON)))
                .show_value(false),
        );
        if response.changed() {
            commands.extend(cut_operation_scalar_command(
                operation, control, value, locked,
            ));
        }
        if control.topology_changing {
            ui.small("Topology")
                .on_hover_text("This control can change generated topology.");
        }
    });

    commands
}

fn render_edge_treatments(
    ui: &mut egui::Ui,
    state: &AssetUiState,
    operation: &AssetCutOperation,
) -> Vec<AssetAppCommand> {
    let mut commands = Vec::new();
    if operation.edge_treatments.is_empty() {
        return commands;
    }

    ui.collapsing("Edge Treatments", |ui| {
        for treatment in &operation.edge_treatments {
            ui.horizontal(|ui| {
                ui.label(&treatment.label).on_hover_text(format!(
                    "definition.{}.operation.{} consumes loop.{} from operation.{}",
                    treatment.definition.0,
                    treatment.operation.0,
                    treatment.target_loop.0,
                    treatment.source_operation.0
                ));
                let topology_locked = state.topology_locks.contains(&treatment.definition);
                if ui
                    .add_enabled(!topology_locked, egui::Button::new("Remove"))
                    .on_hover_text("Remove this edge treatment.")
                    .clicked()
                {
                    commands.extend(edge_treatment_remove_command(treatment, topology_locked));
                }
            });
            for control in &treatment.controls {
                commands.extend(render_edge_treatment_control_row(
                    ui, state, treatment, control,
                ));
            }
        }
    });
    commands
}

fn render_edge_treatment_control_row(
    ui: &mut egui::Ui,
    state: &AssetUiState,
    treatment: &AssetEdgeTreatment,
    control: &AssetCutControl,
) -> Vec<AssetAppCommand> {
    let mut commands = Vec::new();
    let mut value = control.value;
    let locked = control.topology_changing && state.topology_locks.contains(&treatment.definition);

    ui.horizontal(|ui| {
        ui.label(&control.label).on_hover_text(format!(
            "definition.{}.operation.{}.{}",
            treatment.definition.0, treatment.operation.0, control.field
        ));
        ui.add_space(4.0);
        ui.monospace(format!("{:.3}", control.value));
        let response = ui.add_enabled(
            !locked,
            Slider::new(&mut value, control.minimum..=control.maximum)
                .step_by(f64::from(control.step.max(f32::EPSILON)))
                .show_value(false),
        );
        if response.changed() {
            commands.extend(edge_treatment_scalar_command(
                treatment, control, value, locked,
            ));
        }
        if control.topology_changing {
            ui.small("Topology")
                .on_hover_text("This control can change generated topology.");
        }
    });

    commands
}

fn render_optional_presence(
    ui: &mut egui::Ui,
    state: &AssetUiState,
    selected: &AssetPart,
) -> Vec<AssetAppCommand> {
    let mut commands = Vec::new();
    if !selected.optional {
        return commands;
    }

    ui.collapsing(AssetParameterGroup::PartPresence.label(), |ui| {
        ui.label(AssetParameterGroup::PartPresence.help());
        let mut enabled = selected.enabled;
        let locked = state.part_locks.contains(&selected.id)
            || state.subtree_locks.contains(&selected.id)
            || state.topology_locks.contains(&selected.definition);
        if ui
            .add_enabled(
                !locked,
                egui::Checkbox::new(&mut enabled, "Include this part"),
            )
            .on_hover_text("Optional authored part presence.")
            .changed()
        {
            commands.extend(part_presence_command(selected, enabled, locked));
        }
    });
    commands
}

fn render_part_locks(
    ui: &mut egui::Ui,
    state: &AssetUiState,
    selected: &AssetPart,
) -> Vec<AssetAppCommand> {
    let mut commands = Vec::new();
    ui.collapsing("Locks", |ui| {
        let mut part_locked = state.part_locks.contains(&selected.id);
        if ui
            .checkbox(&mut part_locked, "Lock part")
            .on_hover_text("Prevent direct edits to this part instance.")
            .changed()
        {
            commands.extend(part_lock_command(
                selected.id,
                state.part_locks.contains(&selected.id),
                part_locked,
            ));
        }

        let mut subtree_locked = state.subtree_locks.contains(&selected.id);
        if ui
            .checkbox(&mut subtree_locked, "Lock subtree")
            .on_hover_text("Prevent edits to this part and all descendants.")
            .changed()
        {
            commands.extend(subtree_lock_command(
                selected.id,
                state.subtree_locks.contains(&selected.id),
                subtree_locked,
            ));
        }

        let mut topology_locked = state.topology_locks.contains(&selected.definition);
        if ui
            .checkbox(&mut topology_locked, "Lock topology")
            .on_hover_text("Prevent topology-changing edits to this shared definition.")
            .changed()
        {
            commands.extend(topology_lock_command(
                selected.definition,
                state.topology_locks.contains(&selected.definition),
                topology_locked,
            ));
        }
    });
    commands
}

fn parameter_value_changed(parameter: &AssetParameter, proposed: f32) -> bool {
    let tolerance = parameter.step.max(1.0e-5) * 1.0e-3;
    (parameter.value - proposed).abs() > tolerance
}

fn cut_control_value_changed(control: &AssetCutControl, proposed: f32) -> bool {
    let tolerance = control.step.max(1.0e-5) * 1.0e-3;
    (control.value - proposed).abs() > tolerance
}

#[must_use]
pub(crate) fn parameter_ids(parameters: &[AssetParameter]) -> Vec<ParameterId> {
    parameters.iter().map(|parameter| parameter.id).collect()
}
