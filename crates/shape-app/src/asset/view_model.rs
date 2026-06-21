//! Adapter from reducer state to asset panel DTOs.

use std::collections::BTreeMap;

use shape_asset::{AssetRecipe, ParameterDescriptor, enumerate_parameters, get_scalar};

use crate::asset::{
    AssetAppState, AssetCandidate, AssetCandidateEdit, AssetHistoryRevision, AssetJobProgress,
    AssetParameter, AssetParameterGroup, AssetPart, AssetUiJobKind, AssetUiState,
    AssetValidationMessage, AssetValidationState, GeneratedPartKind, ParameterId, PartDefinitionId,
    PartInstanceId,
};

use super::jobs::AssetJobSlot;
use super::state::AssetAppIssue;

/// Build the complete UI DTO consumed by asset panels.
pub(crate) fn build_asset_ui_state(state: &AssetAppState, wireframe: bool) -> AssetUiState {
    let mut ui_state = AssetUiState::empty(state.recipe.title.clone());
    ui_state.selected_part = state.selected_part_instance;
    ui_state.parts = parts_for_recipe(&state.recipe, &state.validation_issues);
    ui_state.parameters = parameters_for_recipe(&state.recipe);
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
        .map(|slot| {
            let descriptors = slot
                .candidate
                .changed_parameters
                .iter()
                .filter_map(|parameter| state.recipe.parameters.get(parameter))
                .collect::<Vec<_>>();
            let edits = descriptors
                .iter()
                .map(|descriptor| candidate_edit(&state.recipe, &slot.candidate.recipe, descriptor))
                .collect::<Vec<_>>();
            let structural_changes = descriptors
                .iter()
                .filter(|descriptor| descriptor.topology_changing)
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
                    .unwrap_or(AssetValidationState::Pending),
            }
        })
        .collect()
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

fn candidate_edit(
    before_recipe: &AssetRecipe,
    after_recipe: &AssetRecipe,
    descriptor: &ParameterDescriptor,
) -> AssetCandidateEdit {
    AssetCandidateEdit {
        subject: parameter_subject(before_recipe, descriptor),
        label: descriptor.label.clone(),
        before: get_scalar(before_recipe, &descriptor.path).ok(),
        after: get_scalar(after_recipe, &descriptor.path).ok(),
        structural: descriptor.topology_changing,
    }
}

fn parameter_subject(recipe: &AssetRecipe, descriptor: &ParameterDescriptor) -> String {
    if let Some(instance) = instance_id_from_scalar_path(&descriptor.path)
        && let Some(part) = recipe.instances.get(&instance)
    {
        return part.name.clone();
    }
    if let Some(definition) = definition_id_from_scalar_path(&descriptor.path)
        && let Some(definition) = recipe.definitions.get(&definition)
    {
        return definition.name.clone();
    }
    "Asset".to_owned()
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
