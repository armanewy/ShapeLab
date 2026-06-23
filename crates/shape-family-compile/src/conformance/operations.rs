//! Operation inventory conformance report contracts.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_asset::{AssetRecipe, GeometrySource, ModelingOperationSpec, PartDefinitionId};
use shape_family::{AllowedOperationKind, AssetFamilySchema};

use super::ConformanceStatus;
use super::roles::is_effectively_enabled;

/// Conformance row for operation-class inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationConformance {
    /// Operation class.
    pub operation: AllowedOperationKind,
    /// Number of compiled operations in this class.
    pub actual_count: u32,
    /// Whether this operation class is allowed by the family/style contract.
    pub allowed: bool,
    /// Row status.
    pub status: ConformanceStatus,
    /// Deterministic issue codes attached to this operation class.
    pub issue_codes: Vec<String>,
}

/// Evaluate actual recipe operation classes against the family allow-list.
#[must_use]
pub fn evaluate_operation_conformance(
    family: &AssetFamilySchema,
    recipe: &AssetRecipe,
) -> Vec<OperationConformance> {
    let inventory = actual_operation_inventory(recipe);
    let allowed = family
        .allowed_operations
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut operations = allowed.clone();
    operations.extend(inventory.keys().cloned());

    operations
        .into_iter()
        .map(|operation| {
            let actual_count = inventory.get(&operation).copied().unwrap_or_default();
            let allowed = allowed.contains(&operation);
            let issue_codes = if !allowed && actual_count > 0 {
                vec!["forbidden_operation".to_owned()]
            } else {
                Vec::new()
            };
            let status = if issue_codes.is_empty() {
                ConformanceStatus::Passed
            } else {
                ConformanceStatus::Failed
            };
            OperationConformance {
                operation,
                actual_count,
                allowed,
                status,
                issue_codes,
            }
        })
        .collect()
}

fn actual_operation_inventory(recipe: &AssetRecipe) -> BTreeMap<AllowedOperationKind, u32> {
    let enabled_definitions = enabled_part_definitions(recipe);
    let mut inventory = BTreeMap::<AllowedOperationKind, u32>::new();
    for definition_id in enabled_definitions {
        let Some(definition) = recipe.definitions.get(&definition_id) else {
            continue;
        };
        *inventory
            .entry(source_operation_class(&definition.geometry.source))
            .or_default() += 1;
        for operation in &definition.geometry.operations {
            *inventory.entry(operation_class(operation)).or_default() += 1;
        }
    }
    inventory
}

fn enabled_part_definitions(recipe: &AssetRecipe) -> BTreeSet<PartDefinitionId> {
    recipe
        .instances
        .iter()
        .filter(|(instance_id, _)| is_effectively_enabled(recipe, **instance_id))
        .map(|(_, instance)| instance.definition)
        .collect()
}

fn source_operation_class(source: &GeometrySource) -> AllowedOperationKind {
    match source {
        GeometrySource::RoundedBox { .. }
        | GeometrySource::Cylinder { .. }
        | GeometrySource::Frustum { .. }
        | GeometrySource::Plate { .. }
        | GeometrySource::LiteralMesh { .. } => AllowedOperationKind::Primitive,
        GeometrySource::Sweep { .. } => AllowedOperationKind::Sweep,
        GeometrySource::Lathe { .. } => AllowedOperationKind::Lathe,
        GeometrySource::ReservedBooleanResult { .. } => AllowedOperationKind::BooleanReserved,
    }
}

fn operation_class(operation: &ModelingOperationSpec) -> AllowedOperationKind {
    match operation {
        ModelingOperationSpec::TransformGeometry { .. } => AllowedOperationKind::Transform,
        ModelingOperationSpec::SetBevelProfile { .. }
        | ModelingOperationSpec::BevelBoundaryLoop { .. } => AllowedOperationKind::Bevel,
        ModelingOperationSpec::RecessedPanelCut { .. }
        | ModelingOperationSpec::RectangularThroughCut { .. }
        | ModelingOperationSpec::CircularThroughCut { .. } => AllowedOperationKind::Cut,
        ModelingOperationSpec::MirrorInstances { .. }
        | ModelingOperationSpec::LinearArray { .. }
        | ModelingOperationSpec::RadialArray { .. } => AllowedOperationKind::Array,
        ModelingOperationSpec::ReservedBoolean { .. } => AllowedOperationKind::BooleanReserved,
        ModelingOperationSpec::AddPanel { .. } => AllowedOperationKind::Custom("panel".to_owned()),
        ModelingOperationSpec::AddTrim { .. } => AllowedOperationKind::Custom("trim".to_owned()),
        ModelingOperationSpec::ReservedDeformationProgram { .. } => {
            AllowedOperationKind::Custom("deformation".to_owned())
        }
    }
}
