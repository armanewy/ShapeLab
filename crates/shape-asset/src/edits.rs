//! Edit reports, lock checks, and compatibility helpers for asset recipes.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    AssetEdit, AssetEditProgram, AssetError, AssetRecipe, AssetValidationReport, GeometrySource,
    ModelingOperationSpec, PartDefinition, PartDefinitionId, PartInstanceId, validate_asset_recipe,
};

/// Result of a successfully applied edit program.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetEditOutcome {
    /// Edited recipe.
    pub recipe: AssetRecipe,
    /// Deterministic report for the program.
    pub report: AssetEditReport,
}

/// Deterministic report for an edit program attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetEditReport {
    /// Human-facing program label.
    pub label: String,
    /// Deterministic seed carried by the program.
    pub seed: u64,
    /// Number of edit operations attempted.
    pub attempted: usize,
    /// Number of edit operations applied before success or rejection.
    pub applied: usize,
    /// Per-edit report entries in program order.
    pub entries: Vec<AssetEditReportEntry>,
    /// Final validation report. Empty when rejection happened before validation.
    pub validation: AssetValidationReport,
}

/// One deterministic edit report entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetEditReportEntry {
    /// Zero-based operation index.
    pub index: usize,
    /// Stable edit kind.
    pub edit_type: String,
    /// Stable subject path when available.
    pub subject: Option<String>,
    /// Entry status.
    pub status: AssetEditReportStatus,
    /// Human-readable result.
    pub message: String,
    /// Whether the edit can change topology.
    pub topology_changing: bool,
}

/// Status for one edit report entry.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetEditReportStatus {
    /// The edit was applied to the cloned recipe.
    Applied,
    /// The program was rejected at this edit or validation step.
    Rejected,
}

/// Rejection that still carries the deterministic report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Error)]
#[error("asset edit program rejected")]
pub struct AssetEditRejection {
    /// Report accumulated before rejection.
    pub report: AssetEditReport,
}

/// Apply an edit program atomically and return the edited recipe plus report.
///
/// The input recipe is never mutated. Edits are applied to a clone, then the
/// complete clone is validated. Any edit error or validation issue rejects the
/// whole program and returns the report accumulated so far.
pub fn apply_edit_program_with_report(
    recipe: &AssetRecipe,
    program: &AssetEditProgram,
) -> Result<AssetEditOutcome, AssetEditRejection> {
    let mut clone = recipe.clone();
    let mut report = AssetEditReport {
        label: program.label.clone(),
        seed: program.seed,
        attempted: program.operations.len(),
        applied: 0,
        entries: Vec::with_capacity(program.operations.len()),
        validation: AssetValidationReport::default(),
    };

    for (index, edit) in program.operations.iter().enumerate() {
        let mut entry = report_entry(recipe, index, edit);
        match super::apply_edit(&mut clone, edit) {
            Ok(()) => {
                entry.status = AssetEditReportStatus::Applied;
                entry.message = "applied".to_owned();
                report.applied += 1;
                report.entries.push(entry);
            }
            Err(error) => {
                entry.status = AssetEditReportStatus::Rejected;
                entry.message = error.to_string();
                report.entries.push(entry);
                return Err(AssetEditRejection { report });
            }
        }
    }

    let validation = validate_asset_recipe(&clone);
    report.validation = validation.clone();
    if !validation.is_valid() {
        report.entries.push(AssetEditReportEntry {
            index: program.operations.len(),
            edit_type: "Validate".to_owned(),
            subject: None,
            status: AssetEditReportStatus::Rejected,
            message: "asset recipe validation failed".to_owned(),
            topology_changing: false,
        });
        return Err(AssetEditRejection { report });
    }

    Ok(AssetEditOutcome {
        recipe: clone,
        report,
    })
}

pub(crate) fn ensure_instance_editable(
    recipe: &AssetRecipe,
    instance: PartInstanceId,
) -> Result<(), AssetError> {
    if !recipe.instances.contains_key(&instance) {
        return Err(AssetError::UnknownInstance(instance));
    }
    if recipe.instance_locks.contains(&instance) {
        return Err(AssetError::LockedInstance(instance));
    }
    for root in &recipe.subtree_locks {
        if *root == instance || instance_is_descendant_of(recipe, instance, *root) {
            return Err(AssetError::LockedSubtree {
                root: *root,
                instance,
            });
        }
    }
    Ok(())
}

pub(crate) fn ensure_topology_editable(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
) -> Result<(), AssetError> {
    if !recipe.definitions.contains_key(&definition) {
        return Err(AssetError::UnknownDefinition(definition));
    }
    if recipe.topology_locks.contains(&definition) {
        return Err(AssetError::LockedTopology(definition));
    }
    Ok(())
}

pub(crate) fn ensure_compatible_replacement(
    recipe: &AssetRecipe,
    from: PartDefinitionId,
    to: PartDefinitionId,
) -> Result<(), AssetError> {
    if from == to {
        return Ok(());
    }
    let from_definition = recipe
        .definitions
        .get(&from)
        .ok_or(AssetError::UnknownDefinition(from))?;
    let to_definition = recipe
        .definitions
        .get(&to)
        .ok_or(AssetError::UnknownDefinition(to))?;
    let Some(from_group) = from_definition.variant_group.as_deref() else {
        return Err(AssetError::IncompatibleReplacement { from, to });
    };
    if from_group.is_empty() || to_definition.variant_group.as_deref() != Some(from_group) {
        return Err(AssetError::IncompatibleReplacement { from, to });
    }
    if let Some(group) = recipe.variation.replacement_groups.get(from_group)
        && (!group.definitions.contains(&from) || !group.definitions.contains(&to))
    {
        return Err(AssetError::IncompatibleReplacement { from, to });
    }
    Ok(())
}

pub(crate) fn definition_id_from_scalar_path(path: &str) -> Option<PartDefinitionId> {
    let mut parts = path.split('.');
    match (parts.next(), parts.next()) {
        (Some("definition"), Some(raw)) => raw.parse::<u64>().ok().map(PartDefinitionId),
        _ => None,
    }
}

pub(crate) fn instance_id_from_scalar_path(path: &str) -> Option<PartInstanceId> {
    let mut parts = path.split('.');
    match (parts.next(), parts.next()) {
        (Some("instance"), Some(raw)) => raw.parse::<u64>().ok().map(PartInstanceId),
        _ => None,
    }
}

pub(crate) fn definition_topology_signature(definition: &PartDefinition) -> String {
    let mut signature = String::new();
    push_source_topology(&mut signature, &definition.geometry.source);
    for operation in &definition.geometry.operations {
        signature.push('|');
        push_operation_topology(&mut signature, operation);
    }
    signature
}

fn report_entry(recipe: &AssetRecipe, index: usize, edit: &AssetEdit) -> AssetEditReportEntry {
    AssetEditReportEntry {
        index,
        edit_type: edit_type(edit).to_owned(),
        subject: edit_subject(edit),
        status: AssetEditReportStatus::Applied,
        message: String::new(),
        topology_changing: edit_may_change_topology(recipe, edit),
    }
}

fn edit_type(edit: &AssetEdit) -> &'static str {
    match edit {
        AssetEdit::SetScalar { .. } => "SetScalar",
        AssetEdit::SetTransform { .. } => "SetTransform",
        AssetEdit::SetEnabled { .. } => "SetEnabled",
        AssetEdit::SetOptionalPartEnabled { .. } => "SetOptionalPartEnabled",
        AssetEdit::SetGeneratorDimension { .. } => "SetGeneratorDimension",
        AssetEdit::ReplaceGeometrySource { .. } => "ReplaceGeometrySource",
        AssetEdit::SetBevelSettings { .. } => "SetBevelSettings",
        AssetEdit::SetSweepProfilePoint { .. } => "SetSweepProfilePoint",
        AssetEdit::SetSweepPathFrame { .. } => "SetSweepPathFrame",
        AssetEdit::SetLatheProfilePoint { .. } => "SetLatheProfilePoint",
        AssetEdit::AddInstance { .. } => "AddInstance",
        AssetEdit::RemoveInstance { .. } => "RemoveInstance",
        AssetEdit::ReplaceDefinition { .. } => "ReplaceDefinition",
        AssetEdit::ReplaceInstanceDefinition { .. } => "ReplaceInstanceDefinition",
        AssetEdit::SetArrayCount { .. } => "SetArrayCount",
        AssetEdit::SetArraySpacing { .. } => "SetArraySpacing",
        AssetEdit::DuplicateInstance { .. } => "DuplicateInstance",
        AssetEdit::MirrorInstance { .. } => "MirrorInstance",
        AssetEdit::Attach { .. } => "Attach",
        AssetEdit::Detach { .. } => "Detach",
        AssetEdit::SetLock { .. } => "SetLock",
        AssetEdit::SetInstanceLock { .. } => "SetInstanceLock",
        AssetEdit::SetSubtreeLock { .. } => "SetSubtreeLock",
        AssetEdit::SetTopologyLock { .. } => "SetTopologyLock",
        AssetEdit::ReorderChildInstances { .. } => "ReorderChildInstances",
    }
}

fn edit_subject(edit: &AssetEdit) -> Option<String> {
    match edit {
        AssetEdit::SetScalar { parameter, .. } | AssetEdit::SetLock { parameter, .. } => {
            Some(format!("parameter.{}", parameter.0))
        }
        AssetEdit::SetTransform { instance, .. }
        | AssetEdit::SetEnabled { instance, .. }
        | AssetEdit::SetOptionalPartEnabled { instance, .. }
        | AssetEdit::RemoveInstance { instance }
        | AssetEdit::ReplaceInstanceDefinition { instance, .. }
        | AssetEdit::Attach { instance, .. }
        | AssetEdit::Detach { instance }
        | AssetEdit::SetInstanceLock { instance, .. }
        | AssetEdit::SetSubtreeLock { instance, .. } => Some(format!("instance.{}", instance.0)),
        AssetEdit::DuplicateInstance { instance, .. }
        | AssetEdit::MirrorInstance { instance, .. }
        | AssetEdit::AddInstance {
            instance: crate::PartInstance { id: instance, .. },
        } => Some(format!("instance.{}", instance.0)),
        AssetEdit::SetGeneratorDimension { definition, .. }
        | AssetEdit::ReplaceGeometrySource { definition, .. }
        | AssetEdit::SetBevelSettings { definition, .. }
        | AssetEdit::SetSweepProfilePoint { definition, .. }
        | AssetEdit::SetSweepPathFrame { definition, .. }
        | AssetEdit::SetLatheProfilePoint { definition, .. }
        | AssetEdit::SetArrayCount { definition, .. }
        | AssetEdit::SetArraySpacing { definition, .. }
        | AssetEdit::SetTopologyLock { definition, .. } => {
            Some(format!("definition.{}", definition.0))
        }
        AssetEdit::ReplaceDefinition { definition } => {
            Some(format!("definition.{}", definition.id.0))
        }
        AssetEdit::ReorderChildInstances { parent, .. } => Some(
            parent
                .map(|parent| format!("instance.{}", parent.0))
                .unwrap_or_else(|| "root_instances".to_owned()),
        ),
    }
}

fn edit_may_change_topology(recipe: &AssetRecipe, edit: &AssetEdit) -> bool {
    match edit {
        AssetEdit::SetScalar { parameter, .. } => recipe
            .parameters
            .get(parameter)
            .is_some_and(|descriptor| descriptor.topology_changing),
        AssetEdit::SetGeneratorDimension { dimension, .. } => dimension.topology_changing(),
        AssetEdit::ReplaceGeometrySource { .. }
        | AssetEdit::AddInstance { .. }
        | AssetEdit::RemoveInstance { .. }
        | AssetEdit::SetOptionalPartEnabled { .. }
        | AssetEdit::ReplaceInstanceDefinition { .. }
        | AssetEdit::SetArrayCount { .. }
        | AssetEdit::DuplicateInstance { .. }
        | AssetEdit::MirrorInstance { .. }
        | AssetEdit::Attach { .. }
        | AssetEdit::Detach { .. } => true,
        AssetEdit::SetBevelSettings { segments, .. } => segments.is_some(),
        AssetEdit::ReplaceDefinition { definition } => recipe
            .definitions
            .get(&definition.id)
            .map(|existing| {
                definition_topology_signature(existing) != definition_topology_signature(definition)
            })
            .unwrap_or(true),
        AssetEdit::SetEnabled { .. }
        | AssetEdit::SetTransform { .. }
        | AssetEdit::SetSweepProfilePoint { .. }
        | AssetEdit::SetSweepPathFrame { .. }
        | AssetEdit::SetLatheProfilePoint { .. }
        | AssetEdit::SetArraySpacing { .. }
        | AssetEdit::SetLock { .. }
        | AssetEdit::SetInstanceLock { .. }
        | AssetEdit::SetSubtreeLock { .. }
        | AssetEdit::SetTopologyLock { .. }
        | AssetEdit::ReorderChildInstances { .. } => false,
    }
}

fn instance_is_descendant_of(
    recipe: &AssetRecipe,
    instance: PartInstanceId,
    root: PartInstanceId,
) -> bool {
    let mut cursor = recipe.instances.get(&instance).and_then(|item| item.parent);
    while let Some(parent) = cursor {
        if parent == root {
            return true;
        }
        cursor = recipe.instances.get(&parent).and_then(|item| item.parent);
    }
    false
}

fn push_source_topology(signature: &mut String, source: &GeometrySource) {
    match source {
        GeometrySource::RoundedBox { .. } => signature.push_str("rounded_box"),
        GeometrySource::Cylinder {
            radial_segments, ..
        } => signature.push_str(&format!("cylinder:{radial_segments}")),
        GeometrySource::Frustum {
            radial_segments, ..
        } => signature.push_str(&format!("frustum:{radial_segments}")),
        GeometrySource::Plate { .. } => signature.push_str("plate"),
        GeometrySource::Sweep { profile, path } => {
            signature.push_str(&format!("sweep:{}:{}", profile.len(), path.len()));
        }
        GeometrySource::Lathe { profile, segments } => {
            signature.push_str(&format!("lathe:{}:{segments}", profile.len()));
        }
        GeometrySource::LiteralMesh { positions, faces } => {
            signature.push_str(&format!("literal:{}:{}", positions.len(), faces.len()));
            for face in faces {
                signature.push_str(&format!(":{}", face.len()));
            }
        }
        GeometrySource::ReservedBooleanResult { .. } => signature.push_str("reserved_boolean"),
    }
}

fn push_operation_topology(signature: &mut String, operation: &ModelingOperationSpec) {
    match operation {
        ModelingOperationSpec::TransformGeometry { .. } => signature.push_str("transform"),
        ModelingOperationSpec::SetBevelProfile {
            operation,
            segments,
            ..
        } => signature.push_str(&format!("bevel:{}:{segments}", operation.0)),
        ModelingOperationSpec::AddPanel { operation, .. } => {
            signature.push_str(&format!("panel:{}", operation.0));
        }
        ModelingOperationSpec::AddTrim { operation, .. } => {
            signature.push_str(&format!("trim:{}", operation.0));
        }
        ModelingOperationSpec::RecessedPanelCut {
            operation,
            corner_radius,
            ..
        } => {
            signature.push_str(&format!(
                "recessed_panel_cut:{}:{corner_radius:.6}",
                operation.0
            ));
        }
        ModelingOperationSpec::RectangularThroughCut {
            operation,
            corner_radius,
            ..
        } => {
            signature.push_str(&format!(
                "rectangular_through_cut:{}:{corner_radius:.6}",
                operation.0
            ));
        }
        ModelingOperationSpec::CircularThroughCut {
            operation,
            radial_segments,
            ..
        } => {
            signature.push_str(&format!(
                "circular_through_cut:{}:{radial_segments}",
                operation.0
            ));
        }
        ModelingOperationSpec::MirrorInstances { operation, .. } => {
            signature.push_str(&format!("mirror:{}", operation.0));
        }
        ModelingOperationSpec::LinearArray {
            operation, count, ..
        } => signature.push_str(&format!("linear_array:{}:{count}", operation.0)),
        ModelingOperationSpec::RadialArray {
            operation, count, ..
        } => signature.push_str(&format!("radial_array:{}:{count}", operation.0)),
        ModelingOperationSpec::ReservedBoolean { operation, .. } => {
            signature.push_str(&format!("reserved_boolean:{}", operation.0));
        }
        ModelingOperationSpec::ReservedDeformationProgram { operation, .. } => {
            signature.push_str(&format!("reserved_deformation:{}", operation.0));
        }
    }
}
