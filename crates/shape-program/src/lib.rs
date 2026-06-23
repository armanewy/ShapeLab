#![forbid(unsafe_code)]

//! Shared semantic modeling-program IR.
//!
//! This crate contains data contracts only. It does not evaluate geometry or run
//! inverse search. Forward modeling and inverse reconstruction both use this IR
//! so strict semantic success cannot be achieved by smuggling target geometry
//! through an opaque residual.

pub mod corpus;
pub mod deformation;
pub mod evaluator;
pub mod runtime;
pub mod selection;

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod topology;

/// Current schema version for semantic modeling programs.
pub const MODELING_PROGRAM_SCHEMA_VERSION: u32 = 1;

/// Canonical evaluator version understood by this contract crate.
pub const CANONICAL_EVALUATOR_VERSION: &str = "shape-program-canonical-evaluator-v1";

/// Stable operation ID inside a modeling program.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProgramOperationId(pub String);

/// Stable selection ID inside a modeling program.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SemanticSelectionId(pub String);

/// Stable semantic part ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SemanticPartId(pub String);

/// Stable semantic region ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SemanticRegionId(pub String);

/// Stable semantic boundary-loop ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SemanticBoundaryLoopId(pub String);

/// Declared grammar profile for a modeling program.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrammarProfile {
    /// Program starts from semantic primitives only.
    StrictFromPrimitives,
    /// Program starts from a cataloged, versioned, independently fingerprinted base.
    StrictFromVersionedLibrary,
}

/// Versioned base topology reference for strict-from-library programs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseTopologyReference {
    /// Stable catalog key, for example `humanoid-body`.
    pub catalog_id: String,
    /// Version inside that catalog.
    pub version: String,
    /// Independent fingerprint of the cataloged base topology.
    pub fingerprint: String,
}

/// Serializable semantic modeling program.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelingProgram {
    /// Modeling program schema version.
    pub schema_version: u32,
    /// Declared grammar profile.
    pub grammar_profile: GrammarProfile,
    /// Optional versioned base topology reference.
    pub base_topology: Option<BaseTopologyReference>,
    /// Ordered semantic operations.
    pub operations: Vec<ModelingOperation>,
    /// Reusable semantic selections referenced by operations.
    pub selections: Vec<SemanticSelection>,
    /// Explicit operation dependency graph.
    pub dependency_graph: ProgramDependencyGraph,
    /// Canonical evaluator version required for exact replay.
    pub canonical_evaluator_version: String,
}

impl ModelingProgram {
    /// Create an empty strict-from-primitives program.
    #[must_use]
    pub fn strict_from_primitives() -> Self {
        Self {
            schema_version: MODELING_PROGRAM_SCHEMA_VERSION,
            grammar_profile: GrammarProfile::StrictFromPrimitives,
            base_topology: None,
            operations: Vec::new(),
            selections: Vec::new(),
            dependency_graph: ProgramDependencyGraph::default(),
            canonical_evaluator_version: CANONICAL_EVALUATOR_VERSION.to_owned(),
        }
    }

    /// Return the canonical JSON description size.
    pub fn description_size_bytes(&self) -> Result<usize, ModelingProgramError> {
        serde_json::to_vec(self)
            .map(|bytes| bytes.len())
            .map_err(ModelingProgramError::Serialize)
    }
}

/// One semantic modeling operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelingOperation {
    /// Stable operation ID.
    pub id: ProgramOperationId,
    /// Operation kind.
    pub kind: ModelingOperationKind,
    /// Selection IDs consumed by this operation.
    pub selections: Vec<SemanticSelectionId>,
    /// Compact semantic parameters.
    pub parameters: Vec<SemanticParameter>,
    /// Number of mesh or semantic elements affected by direct parameters.
    pub affected_element_count: usize,
    /// Declared payload descriptors.
    pub payloads: Vec<OperationPayloadDescriptor>,
}

impl ModelingOperation {
    /// Construct a compact semantic operation with no opaque payload.
    #[must_use]
    pub fn compact(id: impl Into<String>, kind: ModelingOperationKind) -> Self {
        Self {
            id: ProgramOperationId(id.into()),
            kind,
            selections: Vec::new(),
            parameters: Vec::new(),
            affected_element_count: 1,
            payloads: Vec::new(),
        }
    }
}

/// Semantic operation vocabulary.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelingOperationKind {
    PrimitiveCreate,
    RegionExtrude,
    RegionInset,
    LoopCut,
    BridgeLoops,
    Merge,
    Split,
    Dissolve,
    Separate,
    Join,
    Mirror,
    Array,
    Subdivide,
    Bevel,
    ShellSolidify,
    ConstrainedBoolean,
    PartTransform,
    RegionTransform,
    Bend,
    Twist,
    Taper,
    Bulge,
    Lattice,
    Ffd,
    CageDeformation,
    JointChainDeformation,
    SmoothRelax,
    SurfaceSlide,
    ShrinkwrapProject,
    BoundedCorrectiveBasis,
    /// Forbidden in strict success: one command sets every target position.
    SetAllPositions,
    /// Forbidden in strict success: one independent movement per vertex.
    MoveVertex,
    /// Forbidden in strict success: dense arbitrary displacement data.
    DenseDisplacement,
    /// Forbidden in strict success: literal target mesh payload.
    LiteralTargetMesh,
    /// Forbidden in strict success: opaque residual correction.
    OpaqueResidual,
    /// Forbidden in strict success: one arbitrary weight per vertex cage.
    PerVertexCageWeights,
}

/// Compact semantic parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SemanticParameter {
    Scalar { name: String, value: f64 },
    Integer { name: String, value: i64 },
    Boolean { name: String, value: bool },
    Choice { name: String, value: String },
    Vector3 { name: String, value: [f64; 3] },
    Quaternion { name: String, value: [f64; 4] },
}

/// Descriptor for any non-trivial payload attached to an operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationPayloadDescriptor {
    /// Payload kind.
    pub kind: OperationPayloadKind,
    /// Encoded bytes in the serialized program.
    pub encoded_bytes: usize,
    /// Semantic parameter count represented by this payload.
    pub semantic_parameter_count: usize,
    /// Number of mesh or semantic elements affected by this payload.
    ///
    /// For `ExplicitSelectionIndices`, this is the explicit element-ID count.
    pub affected_element_count: usize,
    /// Whether small nearby semantic perturbations are valid.
    pub perturbation_valid: bool,
}

/// Payload categories used by strict admissibility.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationPayloadKind {
    SemanticParameters,
    CompactSelection,
    ExplicitSelectionIndices,
    ProceduralSeed,
    LiteralTargetMesh,
    DenseDisplacement,
    OpaqueResidual,
    PerVertexIndependentPositions,
    PerVertexCageWeights,
}

/// Reusable semantic selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticSelection {
    /// Stable selection ID.
    pub id: SemanticSelectionId,
    /// Selection payload.
    pub payload: SemanticSelectionPayload,
}

impl SemanticSelection {
    /// Number of explicitly encoded element IDs.
    #[must_use]
    pub fn explicit_payload_len(&self) -> usize {
        match &self.payload {
            SemanticSelectionPayload::ExplicitIndices { indices, .. } => indices.len(),
            _ => 0,
        }
    }
}

/// Compact selection vocabulary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SemanticSelectionPayload {
    Part {
        part: SemanticPartId,
    },
    Region {
        region: SemanticRegionId,
    },
    BoundaryLoop {
        boundary_loop: SemanticBoundaryLoopId,
    },
    EdgeClass {
        class: String,
    },
    FacePatch {
        patch: String,
    },
    SymmetryPartner {
        selection: SemanticSelectionId,
    },
    GeodesicNeighborhood {
        seed: SemanticSelectionId,
        radius: f64,
    },
    SpatialPrimitive {
        shape: SpatialPrimitiveSelection,
    },
    BooleanOperand {
        operand_id: String,
    },
    CompactFalloffField {
        field_id: String,
        parameter_count: usize,
    },
    SemanticLandmarkGroup {
        group_id: String,
    },
    ExplicitIndices {
        target: ExplicitSelectionTarget,
        indices: Vec<u32>,
    },
}

/// Spatial primitive selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum SpatialPrimitiveSelection {
    Sphere {
        center: [f64; 3],
        radius: f64,
    },
    Box {
        min: [f64; 3],
        max: [f64; 3],
    },
    PlaneSlab {
        normal: [f64; 3],
        offset: f64,
        half_width: f64,
    },
}

/// Target kind for explicit index selections.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplicitSelectionTarget {
    Vertex,
    Edge,
    Face,
    Loop,
}

/// Directed operation dependency graph.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramDependencyGraph {
    /// Edges are `(producer, consumer)`.
    pub operation_edges: Vec<(ProgramOperationId, ProgramOperationId)>,
    /// Selection dependencies are `(selection, operation)`.
    pub selection_edges: Vec<(SemanticSelectionId, ProgramOperationId)>,
}

/// Strict semantic admissibility policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticAdmissibilityPolicy {
    /// Maximum semantic parameter count divided by affected element count.
    pub maximum_parameter_growth_relative_to_affected_elements: f64,
    /// Maximum number of explicit element IDs in one selection payload.
    pub maximum_explicit_selection_payload: usize,
    /// Payload kinds forbidden in strict success.
    pub forbidden_opaque_payload_kinds: BTreeSet<OperationPayloadKind>,
    /// Operation kinds forbidden in strict success.
    pub forbidden_operation_kinds: BTreeSet<ModelingOperationKind>,
    /// Required raw-geometry-size / program-description-size ratio.
    pub minimum_compression_ratio: f64,
    /// Whether operations must declare local perturbation validity.
    pub perturbation_validity_required: bool,
}

impl SemanticAdmissibilityPolicy {
    /// Default strict policy for Shape Lab semantic reconstruction.
    #[must_use]
    pub fn strict() -> Self {
        Self {
            maximum_parameter_growth_relative_to_affected_elements: 0.25,
            maximum_explicit_selection_payload: 64,
            forbidden_opaque_payload_kinds: BTreeSet::from([
                OperationPayloadKind::LiteralTargetMesh,
                OperationPayloadKind::DenseDisplacement,
                OperationPayloadKind::OpaqueResidual,
                OperationPayloadKind::PerVertexIndependentPositions,
                OperationPayloadKind::PerVertexCageWeights,
            ]),
            forbidden_operation_kinds: BTreeSet::from([
                ModelingOperationKind::SetAllPositions,
                ModelingOperationKind::MoveVertex,
                ModelingOperationKind::DenseDisplacement,
                ModelingOperationKind::LiteralTargetMesh,
                ModelingOperationKind::OpaqueResidual,
                ModelingOperationKind::PerVertexCageWeights,
            ]),
            minimum_compression_ratio: 2.0,
            perturbation_validity_required: true,
        }
    }
}

impl Default for SemanticAdmissibilityPolicy {
    fn default() -> Self {
        Self::strict()
    }
}

/// Raw target geometry size used only for compression accounting.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawGeometrySize {
    /// Vertex count in the target mesh.
    pub vertex_count: usize,
    /// Polygon or face count in the target mesh.
    pub face_count: usize,
    /// Bytes required by canonical raw positions.
    pub position_bytes: usize,
    /// Bytes required by canonical topology indices.
    pub topology_bytes: usize,
}

impl RawGeometrySize {
    /// Total raw bytes represented by this target.
    #[must_use]
    pub fn total_bytes(self) -> usize {
        self.position_bytes.saturating_add(self.topology_bytes)
    }
}

/// Exact semantic topology components.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticTopologyExact {
    /// Graph connectivity is exact.
    pub graph: bool,
    /// Polygon boundary loops are exact.
    pub polygon_boundaries: bool,
    /// Face winding is exact.
    pub winding: bool,
    /// Part and object membership are exact.
    pub part_object_membership: bool,
    /// Geometry-carrying topology is exact.
    pub geometry: bool,
}

impl SemanticTopologyExact {
    /// Return true only when all semantic topology channels are exact.
    #[must_use]
    pub fn is_exact(self) -> bool {
        self.graph
            && self.polygon_boundaries
            && self.winding
            && self.part_object_membership
            && self.geometry
    }
}

/// Serialization order exactness is separate from semantic topology exactness.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializationOrderExact {
    /// Vertex index order is exact.
    pub vertex_order: bool,
    /// Face index order is exact.
    pub face_order: bool,
}

impl SerializationOrderExact {
    /// Return true only when all serialization-order channels are exact.
    #[must_use]
    pub fn is_exact(self) -> bool {
        self.vertex_order && self.face_order
    }
}

/// Contract errors.
#[derive(Debug, Error)]
pub enum ModelingProgramError {
    /// Program serialization failed.
    #[error("failed to serialize modeling program: {0}")]
    Serialize(serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_policy_forbids_residual_and_dense_payloads() {
        let policy = SemanticAdmissibilityPolicy::strict();

        assert!(
            policy
                .forbidden_opaque_payload_kinds
                .contains(&OperationPayloadKind::OpaqueResidual)
        );
        assert!(
            policy
                .forbidden_opaque_payload_kinds
                .contains(&OperationPayloadKind::LiteralTargetMesh)
        );
        assert!(
            policy
                .forbidden_operation_kinds
                .contains(&ModelingOperationKind::MoveVertex)
        );
        assert!(policy.perturbation_validity_required);
    }

    #[test]
    fn semantic_topology_and_serialization_order_are_distinct() {
        let semantic = SemanticTopologyExact {
            graph: true,
            polygon_boundaries: true,
            winding: true,
            part_object_membership: true,
            geometry: true,
        };
        let serialization = SerializationOrderExact {
            vertex_order: true,
            face_order: false,
        };

        assert!(semantic.is_exact());
        assert!(!serialization.is_exact());
    }

    #[test]
    fn program_description_size_uses_serialized_ir_not_target_mesh() {
        let mut program = ModelingProgram::strict_from_primitives();
        program.operations.push(ModelingOperation::compact(
            "op.create.box",
            ModelingOperationKind::PrimitiveCreate,
        ));

        let size = program
            .description_size_bytes()
            .expect("program should serialize");

        assert!(size > 0);
        assert!(
            !serde_json::to_string(&program)
                .unwrap()
                .contains("positions")
        );
    }
}
