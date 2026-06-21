//! Adapter from reducer state to asset panel DTOs.

use std::collections::BTreeMap;

use shape_asset::{
    AssetRecipe, ModelingOperationSpec, ParameterDescriptor, enumerate_parameters, get_scalar,
};

use crate::asset::{
    AssetAppState, AssetCandidate, AssetCandidateEdit, AssetCutControl, AssetCutOperation,
    AssetCutOperationKind, AssetHistoryRevision, AssetJobProgress, AssetParameter,
    AssetParameterGroup, AssetPart, AssetUiJobKind, AssetUiState, AssetValidationMessage,
    AssetValidationState, GeneratedPartKind, OperationId, ParameterId, PartDefinitionId,
    PartInstanceId,
};
use shape_search::asset::AssetCandidateEditKind;

use super::jobs::AssetJobSlot;
use super::state::{AssetAppIssue, AssetCandidateSlot};

/// Build the complete UI DTO consumed by asset panels.
pub(crate) fn build_asset_ui_state(state: &AssetAppState, wireframe: bool) -> AssetUiState {
    let mut ui_state = AssetUiState::empty(state.recipe.title.clone());
    ui_state.selected_part = state.selected_part_instance;
    ui_state.selected_cut_operation = state.selected_cut_operation;
    ui_state.parts = parts_for_recipe(&state.recipe, &state.validation_issues);
    ui_state.parameters = parameters_for_recipe(&state.recipe);
    ui_state.cut_operations = cut_operations_for_recipe(
        &state.recipe,
        state.selected_part_instance,
        state.selected_cut_operation,
    );
    ui_state.candidates = candidates_for_state(state);
    ui_state.history = history_for_state(state);
    ui_state.active_job = active_job_for_state(state);
    ui_state.validation = validation_for_state(&state.validation_issues);
    ui_state.parameter_locks = state.locks.parameters.clone();
    ui_state.part_locks = state.locks.instances.clone();
    ui_state.subtree_locks = state.locks.subtrees.clone();
    ui_state.topology_locks = state.locks.topology.clone();
    ui_state.wireframe = wireframe;
    ui_state
}

fn parts_for_recipe(recipe: &AssetRecipe, issues: &[AssetAppIssue]) -> Vec<AssetPart> {
    recipe
        .instances
        .values()
        .map(|instance| {
            let definition = recipe.definitions.get(&instance.definition);
            AssetPart {
                id: instance.id,
                parent: instance.parent,
                definition: instance.definition,
                name: instance.name.clone(),
                definition_name: definition
                    .map(|definition| definition.name.clone())
                    .unwrap_or_else(|| format!("definition {}", instance.definition.0)),
                enabled: instance.enabled,
                optional: recipe.variation.optional_instances.contains(&instance.id),
                generated: instance
                    .generated_by
                    .map(|_| GeneratedPartKind::LinearArray { index: 1, count: 1 })
                    .unwrap_or(GeneratedPartKind::Authored),
                socket_count: definition
                    .map(|definition| definition.sockets.len())
                    .unwrap_or(0),
                region_count: definition
                    .map(|definition| definition.regions.len())
                    .unwrap_or(0),
                warning_count: issues_for_part(issues, instance.id),
            }
        })
        .collect()
}

fn parameters_for_recipe(recipe: &AssetRecipe) -> Vec<AssetParameter> {
    enumerate_parameters(recipe)
        .into_iter()
        .filter_map(|parameter| parameter_for_recipe(recipe, parameter))
        .collect()
}

fn cut_operations_for_recipe(
    recipe: &AssetRecipe,
    selected_part: Option<PartInstanceId>,
    selected_operation: Option<OperationId>,
) -> Vec<AssetCutOperation> {
    let Some(part_id) = selected_part else {
        return Vec::new();
    };
    let Some(part) = recipe.instances.get(&part_id) else {
        return Vec::new();
    };
    let Some(definition) = recipe.definitions.get(&part.definition) else {
        return Vec::new();
    };

    definition
        .geometry
        .operations
        .iter()
        .filter_map(|operation| {
            cut_operation_for_spec(definition.id, part_id, operation, selected_operation)
        })
        .collect()
}

fn cut_operation_for_spec(
    definition: PartDefinitionId,
    part: PartInstanceId,
    operation: &ModelingOperationSpec,
    selected_operation: Option<OperationId>,
) -> Option<AssetCutOperation> {
    let operation_id = operation.operation_id();
    let (kind, controls) = match operation {
        ModelingOperationSpec::RecessedPanelCut {
            center,
            size,
            depth,
            corner_radius,
            rim_width,
            corner_segments,
            ..
        } => (
            AssetCutOperationKind::RecessedPanel,
            vec![
                cut_control(
                    "recessed_panel_cut.center.x",
                    "Position X",
                    center[0],
                    -2.0,
                    2.0,
                    0.01,
                    false,
                ),
                cut_control(
                    "recessed_panel_cut.center.y",
                    "Position Y",
                    center[1],
                    -2.0,
                    2.0,
                    0.01,
                    false,
                ),
                cut_control(
                    "recessed_panel_cut.size.x",
                    "Width",
                    size[0],
                    0.05,
                    4.0,
                    0.01,
                    false,
                ),
                cut_control(
                    "recessed_panel_cut.size.y",
                    "Height",
                    size[1],
                    0.05,
                    4.0,
                    0.01,
                    false,
                ),
                cut_control(
                    "recessed_panel_cut.depth",
                    "Depth",
                    *depth,
                    0.005,
                    1.0,
                    0.005,
                    false,
                ),
                cut_control(
                    "recessed_panel_cut.rim_width",
                    "Rim Width",
                    *rim_width,
                    0.0,
                    0.5,
                    0.005,
                    false,
                ),
                cut_control(
                    "recessed_panel_cut.corner_radius",
                    "Corner Radius",
                    *corner_radius,
                    0.0,
                    1.0,
                    0.005,
                    false,
                ),
                cut_control(
                    "recessed_panel_cut.corner_segments",
                    "Corner Resolution",
                    *corner_segments as f32,
                    1.0,
                    16.0,
                    1.0,
                    true,
                ),
            ],
        ),
        ModelingOperationSpec::RectangularThroughCut {
            center,
            size,
            corner_radius,
            rim_width,
            corner_segments,
            ..
        } => (
            AssetCutOperationKind::RectangularOpening,
            vec![
                cut_control(
                    "rectangular_through_cut.center.x",
                    "Position X",
                    center[0],
                    -2.0,
                    2.0,
                    0.01,
                    false,
                ),
                cut_control(
                    "rectangular_through_cut.center.y",
                    "Position Y",
                    center[1],
                    -2.0,
                    2.0,
                    0.01,
                    false,
                ),
                cut_control(
                    "rectangular_through_cut.size.x",
                    "Width",
                    size[0],
                    0.05,
                    4.0,
                    0.01,
                    false,
                ),
                cut_control(
                    "rectangular_through_cut.size.y",
                    "Height",
                    size[1],
                    0.05,
                    4.0,
                    0.01,
                    false,
                ),
                cut_control(
                    "rectangular_through_cut.rim_width",
                    "Rim Width",
                    *rim_width,
                    0.0,
                    0.5,
                    0.005,
                    false,
                ),
                cut_control(
                    "rectangular_through_cut.corner_radius",
                    "Corner Radius",
                    *corner_radius,
                    0.0,
                    1.0,
                    0.005,
                    false,
                ),
                cut_control(
                    "rectangular_through_cut.corner_segments",
                    "Corner Resolution",
                    *corner_segments as f32,
                    1.0,
                    16.0,
                    1.0,
                    true,
                ),
            ],
        ),
        ModelingOperationSpec::CircularThroughCut {
            center,
            radius,
            radial_segments,
            rim_width,
            ..
        } => (
            AssetCutOperationKind::CircularOpening,
            vec![
                cut_control(
                    "circular_through_cut.center.x",
                    "Position X",
                    center[0],
                    -2.0,
                    2.0,
                    0.01,
                    false,
                ),
                cut_control(
                    "circular_through_cut.center.y",
                    "Position Y",
                    center[1],
                    -2.0,
                    2.0,
                    0.01,
                    false,
                ),
                cut_control(
                    "circular_through_cut.radius",
                    "Radius",
                    *radius,
                    0.01,
                    2.0,
                    0.005,
                    false,
                ),
                cut_control(
                    "circular_through_cut.rim_width",
                    "Rim Width",
                    *rim_width,
                    0.0,
                    0.5,
                    0.005,
                    false,
                ),
                cut_control(
                    "circular_through_cut.radial_segments",
                    "Roundness",
                    *radial_segments as f32,
                    6.0,
                    48.0,
                    1.0,
                    true,
                ),
            ],
        ),
        _ => return None,
    };

    Some(AssetCutOperation {
        definition,
        part,
        operation: operation_id,
        label: format!("{} {}", kind.label(), operation_id.0),
        kind,
        controls,
        selected: selected_operation == Some(operation_id),
    })
}

fn cut_control(
    field: &str,
    label: &str,
    value: f32,
    minimum: f32,
    maximum: f32,
    step: f32,
    topology_changing: bool,
) -> AssetCutControl {
    AssetCutControl {
        field: field.to_owned(),
        label: label.to_owned(),
        value,
        minimum,
        maximum,
        step,
        topology_changing,
    }
}

fn parameter_for_recipe(
    recipe: &AssetRecipe,
    parameter: ParameterDescriptor,
) -> Option<AssetParameter> {
    let value = get_scalar(recipe, &parameter.path).ok()?;
    let definition = definition_id_from_scalar_path(&parameter.path);
    let part = instance_id_from_scalar_path(&parameter.path).or_else(|| {
        definition.and_then(|definition| first_instance_of_definition(recipe, definition))
    });
    Some(AssetParameter {
        id: parameter.id,
        part,
        definition,
        label: parameter.label.clone(),
        technical_name: parameter.path.clone(),
        group: parameter_group(&parameter),
        value,
        minimum: parameter.minimum,
        maximum: parameter.maximum,
        step: parameter.step,
        locked: recipe.locks.contains(&parameter.id),
        topology_changing: parameter.topology_changing,
        beginner_description: parameter.beginner_description.clone(),
    })
}

fn candidates_for_state(state: &AssetAppState) -> Vec<AssetCandidate> {
    state
        .candidate_slots
        .iter()
        .map(candidate_for_slot)
        .collect()
}

#[must_use]
pub(crate) fn candidate_for_slot(slot: &AssetCandidateSlot) -> AssetCandidate {
    let edits = slot
        .candidate
        .changes
        .iter()
        .map(candidate_edit_from_change)
        .collect::<Vec<_>>();
    let structural_changes = slot
        .candidate
        .changes
        .iter()
        .filter(|change| change.structural)
        .count();
    let numeric_changes = edits.len().saturating_sub(structural_changes);
    AssetCandidate {
        id: slot.candidate.id,
        title: slot.candidate.label.clone(),
        structural_changes,
        numeric_changes,
        edits,
        validation: slot
            .preview
            .as_ref()
            .map(|preview| {
                if preview.validation_summary.valid {
                    AssetValidationState::Valid
                } else {
                    AssetValidationState::Error(format!(
                        "{} validation issue(s)",
                        preview.validation_summary.issue_count
                    ))
                }
            })
            .or_else(|| {
                slot.preview_failure
                    .as_ref()
                    .map(|message| AssetValidationState::Error(message.clone()))
            })
            .unwrap_or(AssetValidationState::Pending),
    }
}

fn history_for_state(state: &AssetAppState) -> Vec<AssetHistoryRevision> {
    let mut child_counts = BTreeMap::<_, usize>::new();
    for revision in state.revision_history.revisions.values() {
        if let Some(parent) = revision.parent {
            *child_counts.entry(parent).or_insert(0) += 1;
        }
    }
    state
        .revision_history
        .revisions
        .values()
        .map(|revision| AssetHistoryRevision {
            id: revision.id,
            parent: revision.parent,
            label: revision.label.clone(),
            operation_summary: revision.label.clone(),
            child_count: child_counts.get(&revision.id).copied().unwrap_or(0),
            selected: revision.id == state.revision_history.current,
        })
        .collect()
}

fn active_job_for_state(state: &AssetAppState) -> Option<AssetJobProgress> {
    let (slot, active) = state.active_jobs.iter().next()?;
    let (kind, phase, completed, total) = match slot {
        AssetJobSlot::CompileCurrentAsset => (AssetUiJobKind::Compile, "Compiling asset", 0, 1),
        AssetJobSlot::RenderCurrentPreview => (AssetUiJobKind::Preview, "Rendering preview", 0, 1),
        AssetJobSlot::GenerateCandidates => (
            AssetUiJobKind::CandidateSearch,
            "Generating directions",
            state.candidate_slots.len(),
            6,
        ),
        AssetJobSlot::CompileCandidatePreviews => (
            AssetUiJobKind::CandidateSearch,
            "Rendering candidates",
            state
                .candidate_slots
                .iter()
                .filter(|slot| slot.preview.is_some())
                .count(),
            6,
        ),
        AssetJobSlot::ExportObj | AssetJobSlot::ExportPackage => {
            (AssetUiJobKind::Inspect, "Exporting asset", 0, 1)
        }
    };
    Some(AssetJobProgress {
        job_id: active.job_id,
        kind,
        phase: phase.to_owned(),
        completed,
        total,
    })
}

fn validation_for_state(issues: &[AssetAppIssue]) -> Vec<AssetValidationMessage> {
    issues
        .iter()
        .map(|issue| AssetValidationMessage {
            part: issue
                .subject
                .as_deref()
                .and_then(part_id_from_validation_subject),
            state: AssetValidationState::Warning(issue.code.clone()),
            message: issue.message.clone(),
        })
        .collect()
}

fn candidate_edit_from_change(change: &super::jobs::AssetCandidateChange) -> AssetCandidateEdit {
    AssetCandidateEdit {
        subject: change.subject.clone(),
        label: change_label(change),
        before: change.before.as_deref().and_then(parse_scalar_summary),
        after: change.after.as_deref().and_then(parse_scalar_summary),
        structural: change.structural,
    }
}

fn change_label(change: &super::jobs::AssetCandidateChange) -> String {
    if change
        .before
        .as_deref()
        .and_then(parse_scalar_summary)
        .is_some()
        || change
            .after
            .as_deref()
            .and_then(parse_scalar_summary)
            .is_some()
    {
        return change.label.clone();
    }

    match change.kind {
        AssetCandidateEditKind::Parameter => change.label.clone(),
        AssetCandidateEditKind::Transform => "placement".to_owned(),
        AssetCandidateEditKind::GeneratorDimension => "proportions".to_owned(),
        AssetCandidateEditKind::Bevel => "edge softness".to_owned(),
        AssetCandidateEditKind::Sweep => "profile curve".to_owned(),
        AssetCandidateEditKind::Lathe => "turned profile".to_owned(),
        AssetCandidateEditKind::ArrayCount => "repeat count".to_owned(),
        AssetCandidateEditKind::ArraySpacing => "spacing".to_owned(),
        AssetCandidateEditKind::OptionalPart => "presence".to_owned(),
        AssetCandidateEditKind::Replacement => "part choice".to_owned(),
        AssetCandidateEditKind::DetailDensity => "detail density".to_owned(),
        AssetCandidateEditKind::ModelingOperation => "operation".to_owned(),
    }
}

fn parse_scalar_summary(value: &str) -> Option<f32> {
    value.parse::<f32>().ok()
}

fn issues_for_part(issues: &[AssetAppIssue], part: PartInstanceId) -> usize {
    let needle = format!("part.{}", part.0);
    issues
        .iter()
        .filter(|issue| {
            issue
                .subject
                .as_deref()
                .is_some_and(|subject| subject.contains(&needle))
        })
        .count()
}

fn first_instance_of_definition(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
) -> Option<PartInstanceId> {
    recipe
        .instances
        .values()
        .find(|instance| instance.definition == definition)
        .map(|instance| instance.id)
}

fn parameter_group(parameter: &ParameterDescriptor) -> AssetParameterGroup {
    match parameter.group.as_str() {
        "Size" => AssetParameterGroup::Size,
        "Proportions" => AssetParameterGroup::Proportions,
        "Placement" => AssetParameterGroup::Placement,
        "Curvature" => AssetParameterGroup::Curvature,
        "Edge Softness" => AssetParameterGroup::EdgeSoftness,
        "Repetition" => AssetParameterGroup::Repetition,
        "Part Presence" => AssetParameterGroup::PartPresence,
        "Detail Density" => AssetParameterGroup::DetailDensity,
        _ => {
            let lower = parameter.path.to_ascii_lowercase();
            if lower.contains("linear_array") || lower.contains("radial_array") {
                AssetParameterGroup::Repetition
            } else if lower.contains("bevel") || lower.contains("radius") {
                AssetParameterGroup::EdgeSoftness
            } else if lower.contains("translation") || lower.contains("rotation") {
                AssetParameterGroup::Placement
            } else if lower.contains("sweep") || lower.contains("lathe") {
                AssetParameterGroup::Curvature
            } else {
                AssetParameterGroup::Size
            }
        }
    }
}

fn definition_id_from_scalar_path(path: &str) -> Option<PartDefinitionId> {
    let mut parts = path.split('.');
    if parts.next()? != "definition" {
        return None;
    }
    parts.next()?.parse().ok().map(PartDefinitionId)
}

fn instance_id_from_scalar_path(path: &str) -> Option<PartInstanceId> {
    let mut parts = path.split('.');
    if parts.next()? != "instance" {
        return None;
    }
    parts.next()?.parse().ok().map(PartInstanceId)
}

fn part_id_from_validation_subject(subject: &str) -> Option<PartInstanceId> {
    let parts = subject.split('.').collect::<Vec<_>>();
    parts.windows(2).find_map(|window| {
        (window[0] == "part").then(|| window[1].parse().ok().map(PartInstanceId))?
    })
}

#[allow(dead_code)]
fn _parameter_id_for_docs(_: ParameterId) {}
