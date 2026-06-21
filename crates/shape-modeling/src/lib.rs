#![forbid(unsafe_code)]

//! Deterministic modeling-operation contracts for the explicit polygon lane.
//!
//! This crate defines the public generator surface for asset recipes but does
//! not implement substantive geometry production yet. Heavy generators return
//! typed unsupported errors so downstream compile code can distinguish contract
//! gaps from invalid input.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shape_asset::{
    GeometrySource, OperationId, PartDefinition, PartDefinitionId, PartInstanceId, RegionId,
    SocketId, SocketSpec, SurfaceRegionSpec,
};
use shape_poly::{MeshBounds, PolyError, PolygonMesh};
use thiserror::Error;

/// Explicit profile-based generators.
pub mod generators {
    /// Sweep and lathe profile generators.
    pub mod profile;
}

/// Generated local part payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedPart {
    /// Generated polygon mesh.
    pub mesh: PolygonMesh,
    /// Sockets declared or generated for this part.
    pub sockets: BTreeMap<SocketId, SocketSpec>,
    /// Regions declared or generated for this part.
    pub regions: BTreeMap<RegionId, SurfaceRegionSpec>,
    /// Local mesh bounds.
    pub local_bounds: MeshBounds,
    /// Deterministic signature for the generator configuration.
    pub generator_signature: String,
}

/// Context passed to deterministic generators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratorContext {
    /// Part definition being generated.
    pub part_definition: PartDefinitionId,
    /// Part instance being generated.
    pub part_instance: PartInstanceId,
    /// Next operation ID available to generator-created operations.
    pub next_operation_id: u64,
    /// Topology epoch used by callers to identify topology-changing revisions.
    pub topology_epoch: u64,
}

impl GeneratorContext {
    /// Create a generator context.
    #[must_use]
    pub fn new(
        part_definition: PartDefinitionId,
        part_instance: PartInstanceId,
        next_operation_id: u64,
        topology_epoch: u64,
    ) -> Self {
        Self {
            part_definition,
            part_instance,
            next_operation_id,
            topology_epoch,
        }
    }

    /// Allocate an operation ID from the context.
    pub fn allocate_operation_id(&mut self) -> OperationId {
        let operation = OperationId(self.next_operation_id);
        self.next_operation_id = self.next_operation_id.saturating_add(1);
        operation
    }
}

/// Trait implemented by explicit geometry generators.
pub trait GeometryGenerator {
    /// Generate a part from a definition and generator context.
    fn generate(
        &self,
        definition: &PartDefinition,
        context: &mut GeneratorContext,
    ) -> Result<GeneratedPart, ModelingError>;
}

/// Error type for deterministic modeling contracts.
#[derive(Debug, Error)]
pub enum ModelingError {
    /// Geometry source is known but not implemented in this wave.
    #[error("unsupported geometry source {geometry_source}")]
    UnsupportedGeometry {
        /// Source family name.
        geometry_source: &'static str,
    },
    /// Modeling operation is known but not supported by compilation yet.
    #[error("unsupported modeling operation {operation:?}: {reason}")]
    UnsupportedOperation {
        /// Operation ID.
        operation: OperationId,
        /// Explanation.
        reason: String,
    },
    /// Definition or context is inconsistent.
    #[error("invalid modeling input: {0}")]
    InvalidInput(String),
    /// Polygon topology helper failed.
    #[error("polygon error: {0}")]
    Polygon(#[from] PolyError),
}

/// Dispatch geometry generation by source family.
pub fn generate_geometry(
    definition: &PartDefinition,
    context: &mut GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    ensure_context_matches(definition, context)?;
    ensure_operations_supported(definition)?;
    match &definition.geometry.source {
        GeometrySource::RoundedBox { .. } => generate_rounded_box(definition, context),
        GeometrySource::Cylinder { .. } => generate_cylinder(definition, context),
        GeometrySource::Frustum { .. } => generate_frustum(definition, context),
        GeometrySource::Plate { .. } => generate_plate(definition, context),
        GeometrySource::Sweep { .. } => generate_sweep(definition, context),
        GeometrySource::Lathe { .. } => generate_lathe(definition, context),
        GeometrySource::LiteralMesh { .. } => unsupported_geometry("LiteralMesh"),
        GeometrySource::ReservedBooleanResult { .. } => {
            unsupported_geometry("ReservedBooleanResult")
        }
    }
}

/// Stub rounded-box generator.
pub fn generate_rounded_box(
    _definition: &PartDefinition,
    _context: &mut GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    unsupported_geometry("RoundedBox")
}

/// Stub cylinder generator.
pub fn generate_cylinder(
    _definition: &PartDefinition,
    _context: &mut GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    unsupported_geometry("Cylinder")
}

/// Stub frustum generator.
pub fn generate_frustum(
    _definition: &PartDefinition,
    _context: &mut GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    unsupported_geometry("Frustum")
}

/// Stub plate generator.
pub fn generate_plate(
    _definition: &PartDefinition,
    _context: &mut GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    unsupported_geometry("Plate")
}

/// Stub sweep generator.
pub fn generate_sweep(
    _definition: &PartDefinition,
    _context: &mut GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    unsupported_geometry("Sweep")
}

/// Stub lathe generator.
pub fn generate_lathe(
    _definition: &PartDefinition,
    _context: &mut GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    unsupported_geometry("Lathe")
}

fn ensure_context_matches(
    definition: &PartDefinition,
    context: &GeneratorContext,
) -> Result<(), ModelingError> {
    if context.part_definition != definition.id {
        return Err(ModelingError::InvalidInput(format!(
            "context definition {:?} does not match {:?}",
            context.part_definition, definition.id
        )));
    }
    Ok(())
}

fn ensure_operations_supported(definition: &PartDefinition) -> Result<(), ModelingError> {
    for operation in &definition.geometry.operations {
        match operation {
            shape_asset::ModelingOperationSpec::ReservedBoolean { operation, .. } => {
                return Err(ModelingError::UnsupportedOperation {
                    operation: *operation,
                    reason: "reserved boolean operations serialize but do not compile yet"
                        .to_owned(),
                });
            }
            shape_asset::ModelingOperationSpec::ReservedDeformationProgram {
                operation, ..
            } => {
                return Err(ModelingError::UnsupportedOperation {
                    operation: *operation,
                    reason: "reserved deformation programs serialize but do not compile yet"
                        .to_owned(),
                });
            }
            shape_asset::ModelingOperationSpec::TransformGeometry { .. }
            | shape_asset::ModelingOperationSpec::SetBevelProfile { .. }
            | shape_asset::ModelingOperationSpec::AddPanel { .. }
            | shape_asset::ModelingOperationSpec::AddTrim { .. }
            | shape_asset::ModelingOperationSpec::MirrorInstances { .. }
            | shape_asset::ModelingOperationSpec::LinearArray { .. }
            | shape_asset::ModelingOperationSpec::RadialArray { .. } => {}
        }
    }
    Ok(())
}

fn unsupported_geometry(source: &'static str) -> Result<GeneratedPart, ModelingError> {
    Err(ModelingError::UnsupportedGeometry {
        geometry_source: source,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use shape_asset::{Frame3, GeometryRecipe, ModelingOperationSpec, Transform3};

    use super::*;

    fn definition(source: GeometrySource) -> PartDefinition {
        PartDefinition {
            id: PartDefinitionId(1),
            name: "Part".to_owned(),
            tags: BTreeSet::new(),
            geometry: GeometryRecipe {
                source,
                operations: Vec::new(),
            },
            regions: BTreeMap::new(),
            sockets: BTreeMap::new(),
            local_pivot: Frame3::default(),
            variant_group: None,
            production_hints: None,
        }
    }

    #[test]
    fn context_allocates_operation_ids_deterministically() {
        let mut context = GeneratorContext::new(PartDefinitionId(1), PartInstanceId(2), 10, 0);

        assert_eq!(context.allocate_operation_id(), OperationId(10));
        assert_eq!(context.allocate_operation_id(), OperationId(11));
    }

    #[test]
    fn rounded_box_generation_is_explicitly_unsupported() {
        let definition = definition(GeometrySource::RoundedBox {
            half_extents: [1.0, 1.0, 1.0],
            radius: 0.1,
        });
        let mut context = GeneratorContext::new(PartDefinitionId(1), PartInstanceId(1), 1, 0);

        assert!(matches!(
            generate_geometry(&definition, &mut context),
            Err(ModelingError::UnsupportedGeometry {
                geometry_source: "RoundedBox"
            })
        ));
    }

    #[test]
    fn reserved_operations_are_rejected_before_generation() {
        let mut definition = definition(GeometrySource::Plate {
            size: [1.0, 1.0],
            thickness: 0.1,
        });
        definition
            .geometry
            .operations
            .push(ModelingOperationSpec::ReservedBoolean {
                operation: OperationId(3),
                label: "future".to_owned(),
            });
        let mut context = GeneratorContext::new(PartDefinitionId(1), PartInstanceId(1), 1, 0);

        assert!(matches!(
            generate_geometry(&definition, &mut context),
            Err(ModelingError::UnsupportedOperation {
                operation: OperationId(3),
                ..
            })
        ));
    }

    #[test]
    fn context_definition_must_match() {
        let definition = definition(GeometrySource::Cylinder {
            radius: 1.0,
            height: 2.0,
            radial_segments: 12,
        });
        let mut context = GeneratorContext::new(PartDefinitionId(2), PartInstanceId(1), 1, 0);

        assert!(matches!(
            generate_geometry(&definition, &mut context),
            Err(ModelingError::InvalidInput(_))
        ));
    }

    #[test]
    fn generated_part_contract_round_trips() {
        let part = GeneratedPart {
            mesh: PolygonMesh::empty(),
            sockets: BTreeMap::new(),
            regions: BTreeMap::new(),
            local_bounds: MeshBounds::empty(),
            generator_signature: "empty-contract".to_owned(),
        };

        let json = serde_json::to_string(&part).expect("part should serialize");
        let round_tripped: GeneratedPart =
            serde_json::from_str(&json).expect("part should deserialize");

        assert_eq!(part, round_tripped);
    }

    #[test]
    fn transform_type_remains_available_for_operation_specs() {
        let operation = ModelingOperationSpec::TransformGeometry {
            operation: OperationId(1),
            transform: Transform3::default(),
        };

        assert_eq!(operation.operation_id(), OperationId(1));
    }
}
