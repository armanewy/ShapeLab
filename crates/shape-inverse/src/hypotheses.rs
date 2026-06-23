//! Forward-operation hypothesis generation contracts.
//!
//! This module works at descriptor level. It does not inspect mesh buffers or
//! claim inverse success; it turns coarse hard-surface evidence into candidate
//! semantic operations that downstream search can score, reject, or replay.

use serde::{Deserialize, Serialize};
use shape_program::topology::{
    GeneratedTopologyRule, InferenceHint, ProvenanceRelationship, RetainedInputRule,
    SelectionCount, SelectionSubject, TOPOLOGY_OPERATOR_KINDS, TopologyEffect,
    topology_contract_for,
};
use shape_program::{
    ModelingOperationKind, SemanticBoundaryLoopId, SemanticParameter, SemanticPartId,
    SemanticRegionId, SemanticSelection, SemanticSelectionId, SemanticSelectionPayload,
    SpatialPrimitiveSelection,
};

/// Descriptor-level topology state used by hypothesis generation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyStateDescriptor {
    /// Semantic part count.
    pub part_count: usize,
    /// Semantic object or root count.
    pub object_count: usize,
    /// Semantic region count.
    pub region_count: usize,
    /// Boundary-loop count.
    pub boundary_loop_count: usize,
    /// Edge-loop count.
    pub edge_loop_count: usize,
    /// Edge-set or edge-class count.
    pub edge_set_count: usize,
    /// Vertex-set count.
    pub vertex_set_count: usize,
    /// Sharp edge count.
    pub sharp_edge_count: usize,
    /// Detected shell-like offset layer count.
    pub shell_layer_count: usize,
    /// Closed-volume overlap count.
    pub closed_volume_overlap_count: usize,
}

/// Descriptor evidence that suggests topology operators.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyEvidenceDescriptor {
    /// New primitive-like roots.
    pub primitive_roots: usize,
    /// Regions with extrusion side-wall evidence.
    pub extruded_regions: usize,
    /// Regions with inset boundary-loop evidence.
    pub inset_regions: usize,
    /// Continuous loop-cut tracks.
    pub loop_cuts: usize,
    /// Compatible loop pairs that can be bridged.
    pub bridgeable_loop_pairs: usize,
    /// Coincident or near-coincident element groups.
    pub coincident_element_groups: usize,
    /// Split planes, seams, or branch separations.
    pub split_features: usize,
    /// Redundant elements surrounded by smooth continuation.
    pub dissolved_element_sets: usize,
    /// Connected components that can become separate parts.
    pub separable_components: usize,
    /// Part sets with joinable contact or shared roots.
    pub joinable_part_sets: usize,
    /// Reflection planes with counterpart evidence.
    pub symmetry_planes: usize,
    /// Repeated topology runs.
    pub array_runs: usize,
    /// Subdivision-pattern evidence.
    pub subdivision_patterns: usize,
    /// Bevel bands or support-loop pairs.
    pub bevel_bands: usize,
    /// Offset shell layers.
    pub shell_offsets: usize,
    /// Constrained boolean overlap cells.
    pub closed_volume_overlaps: usize,
}

impl TopologyEvidenceDescriptor {
    /// Evidence fixture that supports every topology operator once.
    #[must_use]
    pub fn all_supported() -> Self {
        Self {
            primitive_roots: 1,
            extruded_regions: 1,
            inset_regions: 1,
            loop_cuts: 1,
            bridgeable_loop_pairs: 1,
            coincident_element_groups: 1,
            split_features: 1,
            dissolved_element_sets: 1,
            separable_components: 1,
            joinable_part_sets: 1,
            symmetry_planes: 1,
            array_runs: 1,
            subdivision_patterns: 1,
            bevel_bands: 1,
            shell_offsets: 1,
            closed_volume_overlaps: 1,
        }
    }
}

/// Input bundle for topology hypothesis generation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HypothesisGenerationInput {
    /// Baseline/source descriptor.
    pub source: TopologyStateDescriptor,
    /// Observed/target descriptor.
    pub target: TopologyStateDescriptor,
    /// Operator-specific evidence.
    pub evidence: TopologyEvidenceDescriptor,
}

/// Candidate topology operation inferred from descriptors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationHypothesis {
    /// Forward topology operator this candidate would instantiate.
    pub operation_kind: ModelingOperationKind,
    /// Estimated compact semantic parameters.
    pub estimated_parameters: Vec<SemanticParameter>,
    /// Semantic selections consumed by this operation.
    pub semantic_selection: Vec<SemanticSelection>,
    /// Expected topology effect if the candidate is replayed.
    pub expected_topology_change: ExpectedTopologyChange,
    /// Descriptor-level confidence in `[0, 1]`.
    pub confidence: f64,
    /// Conservative score floor in `[0, 1]`.
    pub lower_bound_score: f64,
    /// Reason this hypothesis should be rejected or deferred, when known.
    pub failure_reason: Option<HypothesisFailureReason>,
}

/// Expected topology change for a descriptor-level candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedTopologyChange {
    /// Contract topology effect.
    pub effect: TopologyEffect,
    /// Retained-input provenance rule.
    pub retained_input: RetainedInputRule,
    /// Generated-topology provenance rule.
    pub generated_topology: GeneratedTopologyRule,
    /// Relationship between source and generated topology.
    pub relationship: ProvenanceRelationship,
    /// Expected part-count delta.
    pub delta_parts: i64,
    /// Expected region-count delta.
    pub delta_regions: i64,
    /// Expected boundary-loop-count delta.
    pub delta_boundary_loops: i64,
    /// Expected edge-loop-count delta.
    pub delta_edge_loops: i64,
    /// Expected vertex-set-count delta.
    pub delta_vertex_sets: i64,
}

/// Deterministic reason a descriptor-level hypothesis is not yet admissible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HypothesisFailureReason {
    /// The topology registry does not define a contract for this operation kind.
    UnsupportedTopologyContract {
        /// Operation kind.
        operation_kind: ModelingOperationKind,
    },
    /// The descriptor does not contain evidence for the operator.
    InsufficientEvidence {
        /// Operation kind.
        operation_kind: ModelingOperationKind,
        /// Contract hints that would make this candidate stronger.
        expected_hints: Vec<InferenceHint>,
    },
    /// A semantic selection subject could not be represented in the shared IR.
    MissingSemanticSelection {
        /// Required subject.
        subject: SelectionSubject,
    },
    /// Source/target descriptors are internally inconsistent.
    InvalidDescriptor {
        /// Human-readable reason.
        reason: String,
    },
}

/// Generate one candidate for every forward topology operator in registry order.
#[must_use]
pub fn generate_topology_operation_hypotheses(
    input: &HypothesisGenerationInput,
) -> Vec<OperationHypothesis> {
    TOPOLOGY_OPERATOR_KINDS
        .into_iter()
        .map(|kind| generate_topology_operation_hypothesis(kind, input))
        .collect()
}

/// Generate a descriptor-level hypothesis for a single topology operation kind.
#[must_use]
pub fn generate_topology_operation_hypothesis(
    kind: ModelingOperationKind,
    input: &HypothesisGenerationInput,
) -> OperationHypothesis {
    let Some(contract) = topology_contract_for(kind) else {
        return OperationHypothesis {
            operation_kind: kind,
            estimated_parameters: Vec::new(),
            semantic_selection: Vec::new(),
            expected_topology_change: empty_change(),
            confidence: 0.0,
            lower_bound_score: 0.0,
            failure_reason: Some(HypothesisFailureReason::UnsupportedTopologyContract {
                operation_kind: kind,
            }),
        };
    };

    let evidence_count = evidence_count_for(kind, &input.evidence);
    let confidence = confidence_for(kind, evidence_count, input);
    let failure_reason = if evidence_count == 0 {
        Some(HypothesisFailureReason::InsufficientEvidence {
            operation_kind: kind,
            expected_hints: contract.inference_hints.clone(),
        })
    } else {
        None
    };

    let lower_bound_score = if failure_reason.is_some() {
        0.0
    } else {
        lower_bound_score(confidence, contract.semantic_parameter_count)
    };

    OperationHypothesis {
        operation_kind: kind,
        estimated_parameters: estimated_parameters_for(kind),
        semantic_selection: semantic_selection_for(kind, &contract.selection_requirements.count),
        expected_topology_change: expected_change_for(kind, input),
        confidence,
        lower_bound_score,
        failure_reason,
    }
}

fn empty_change() -> ExpectedTopologyChange {
    ExpectedTopologyChange {
        effect: TopologyEffect::CreatePrimitive,
        retained_input: RetainedInputRule::None,
        generated_topology: GeneratedTopologyRule::PrimitiveRoot,
        relationship: ProvenanceRelationship::NewRoot,
        delta_parts: 0,
        delta_regions: 0,
        delta_boundary_loops: 0,
        delta_edge_loops: 0,
        delta_vertex_sets: 0,
    }
}

fn expected_change_for(
    kind: ModelingOperationKind,
    input: &HypothesisGenerationInput,
) -> ExpectedTopologyChange {
    let contract = topology_contract_for(kind).expect("topology kind must have a contract");
    let raw_part_delta = count_delta(input.target.part_count, input.source.part_count);
    let raw_region_delta = count_delta(input.target.region_count, input.source.region_count);
    let raw_boundary_delta = count_delta(
        input.target.boundary_loop_count,
        input.source.boundary_loop_count,
    );
    let raw_edge_loop_delta =
        count_delta(input.target.edge_loop_count, input.source.edge_loop_count);
    let raw_vertex_set_delta =
        count_delta(input.target.vertex_set_count, input.source.vertex_set_count);

    let (delta_parts, delta_regions, delta_boundary_loops, delta_edge_loops, delta_vertex_sets) =
        match kind {
            ModelingOperationKind::PrimitiveCreate => (
                raw_part_delta.max(1),
                raw_region_delta.max(6),
                raw_boundary_delta.max(6),
                raw_edge_loop_delta.max(0),
                raw_vertex_set_delta.max(0),
            ),
            ModelingOperationKind::RegionExtrude => (
                raw_part_delta.max(0),
                raw_region_delta.max(1),
                raw_boundary_delta.max(1),
                raw_edge_loop_delta.max(0),
                raw_vertex_set_delta.max(0),
            ),
            ModelingOperationKind::RegionInset => (
                0,
                raw_region_delta.max(1),
                raw_boundary_delta.max(1),
                raw_edge_loop_delta.max(0),
                raw_vertex_set_delta.max(0),
            ),
            ModelingOperationKind::LoopCut => (
                0,
                raw_region_delta.max(1),
                raw_boundary_delta.max(0),
                raw_edge_loop_delta.max(1),
                raw_vertex_set_delta.max(0),
            ),
            ModelingOperationKind::BridgeLoops => (
                raw_part_delta.max(0),
                raw_region_delta.max(1),
                raw_boundary_delta.min(0),
                raw_edge_loop_delta.max(0),
                raw_vertex_set_delta.max(0),
            ),
            ModelingOperationKind::Merge | ModelingOperationKind::Dissolve => (
                raw_part_delta.min(0),
                raw_region_delta.min(0),
                raw_boundary_delta.min(0),
                raw_edge_loop_delta.min(0),
                raw_vertex_set_delta.min(0),
            ),
            ModelingOperationKind::Split | ModelingOperationKind::Separate => (
                raw_part_delta.max(0),
                raw_region_delta.max(1),
                raw_boundary_delta.max(0),
                raw_edge_loop_delta.max(0),
                raw_vertex_set_delta.max(0),
            ),
            ModelingOperationKind::Join => (
                raw_part_delta.min(-1),
                raw_region_delta,
                raw_boundary_delta,
                raw_edge_loop_delta,
                raw_vertex_set_delta,
            ),
            ModelingOperationKind::Mirror | ModelingOperationKind::Array => (
                raw_part_delta.max(1),
                raw_region_delta.max(1),
                raw_boundary_delta.max(1),
                raw_edge_loop_delta,
                raw_vertex_set_delta,
            ),
            ModelingOperationKind::Subdivide => (
                0,
                raw_region_delta.max(2),
                raw_boundary_delta.max(0),
                raw_edge_loop_delta.max(1),
                raw_vertex_set_delta.max(0),
            ),
            ModelingOperationKind::Bevel => (
                0,
                raw_region_delta.max(1),
                raw_boundary_delta.max(0),
                raw_edge_loop_delta.max(1),
                raw_vertex_set_delta,
            ),
            ModelingOperationKind::ShellSolidify => (
                0,
                raw_region_delta.max(1),
                raw_boundary_delta.max(1),
                raw_edge_loop_delta,
                raw_vertex_set_delta,
            ),
            ModelingOperationKind::ConstrainedBoolean => (
                raw_part_delta.min(0),
                raw_region_delta.max(1),
                raw_boundary_delta.max(1),
                raw_edge_loop_delta.max(0),
                raw_vertex_set_delta,
            ),
            _ => (0, 0, 0, 0, 0),
        };

    ExpectedTopologyChange {
        effect: contract.topology_effect,
        retained_input: contract.stable_provenance.retained_input,
        generated_topology: contract.stable_provenance.generated,
        relationship: contract.stable_provenance.relationship,
        delta_parts,
        delta_regions,
        delta_boundary_loops,
        delta_edge_loops,
        delta_vertex_sets,
    }
}

fn count_delta(target: usize, source: usize) -> i64 {
    target as i64 - source as i64
}

fn evidence_count_for(kind: ModelingOperationKind, evidence: &TopologyEvidenceDescriptor) -> usize {
    match kind {
        ModelingOperationKind::PrimitiveCreate => evidence.primitive_roots,
        ModelingOperationKind::RegionExtrude => evidence.extruded_regions,
        ModelingOperationKind::RegionInset => evidence.inset_regions,
        ModelingOperationKind::LoopCut => evidence.loop_cuts,
        ModelingOperationKind::BridgeLoops => evidence.bridgeable_loop_pairs,
        ModelingOperationKind::Merge => evidence.coincident_element_groups,
        ModelingOperationKind::Split => evidence.split_features,
        ModelingOperationKind::Dissolve => evidence.dissolved_element_sets,
        ModelingOperationKind::Separate => evidence.separable_components,
        ModelingOperationKind::Join => evidence.joinable_part_sets,
        ModelingOperationKind::Mirror => evidence.symmetry_planes,
        ModelingOperationKind::Array => evidence.array_runs,
        ModelingOperationKind::Subdivide => evidence.subdivision_patterns,
        ModelingOperationKind::Bevel => evidence.bevel_bands,
        ModelingOperationKind::ShellSolidify => evidence.shell_offsets,
        ModelingOperationKind::ConstrainedBoolean => evidence.closed_volume_overlaps,
        _ => 0,
    }
}

fn confidence_for(
    kind: ModelingOperationKind,
    evidence_count: usize,
    input: &HypothesisGenerationInput,
) -> f64 {
    if evidence_count == 0 {
        return 0.0;
    }

    let evidence_term = (evidence_count.min(4) as f64) * 0.09;
    let delta_term = if descriptor_delta_supports(kind, input) {
        0.14
    } else {
        0.0
    };

    (0.42 + evidence_term + delta_term).clamp(0.0, 0.95)
}

fn descriptor_delta_supports(
    kind: ModelingOperationKind,
    input: &HypothesisGenerationInput,
) -> bool {
    let part_delta = count_delta(input.target.part_count, input.source.part_count);
    let region_delta = count_delta(input.target.region_count, input.source.region_count);
    let boundary_delta = count_delta(
        input.target.boundary_loop_count,
        input.source.boundary_loop_count,
    );
    let edge_loop_delta = count_delta(input.target.edge_loop_count, input.source.edge_loop_count);

    match kind {
        ModelingOperationKind::PrimitiveCreate
        | ModelingOperationKind::Mirror
        | ModelingOperationKind::Array => part_delta > 0 || region_delta > 0,
        ModelingOperationKind::RegionExtrude
        | ModelingOperationKind::RegionInset
        | ModelingOperationKind::Split
        | ModelingOperationKind::Subdivide
        | ModelingOperationKind::Bevel
        | ModelingOperationKind::ShellSolidify
        | ModelingOperationKind::ConstrainedBoolean => region_delta > 0 || boundary_delta > 0,
        ModelingOperationKind::LoopCut => edge_loop_delta > 0 || region_delta > 0,
        ModelingOperationKind::BridgeLoops => region_delta > 0 || boundary_delta < 0,
        ModelingOperationKind::Merge
        | ModelingOperationKind::Dissolve
        | ModelingOperationKind::Join => part_delta < 0 || region_delta < 0 || boundary_delta < 0,
        ModelingOperationKind::Separate => part_delta > 0 || region_delta > 0,
        _ => false,
    }
}

fn lower_bound_score(confidence: f64, parameter_count: usize) -> f64 {
    (confidence - (parameter_count as f64 * 0.015)).clamp(0.0, 1.0)
}

fn semantic_selection_for(
    kind: ModelingOperationKind,
    count: &SelectionCount,
) -> Vec<SemanticSelection> {
    match count {
        SelectionCount::ExactlyZero => Vec::new(),
        SelectionCount::ExactlyOne => vec![primary_selection_for(kind, 0)],
        SelectionCount::ExactlyTwo => ordered_pair_selection_for(kind),
        SelectionCount::OneOrMore => vec![primary_selection_for(kind, 0)],
        SelectionCount::TwoOrMore => vec![
            primary_selection_for(kind, 0),
            primary_selection_for(kind, 1),
        ],
    }
}

fn ordered_pair_selection_for(kind: ModelingOperationKind) -> Vec<SemanticSelection> {
    match kind {
        ModelingOperationKind::BridgeLoops => vec![
            boundary_loop_selection("sel.bridge.loop.a", "loop.bridge.a"),
            boundary_loop_selection("sel.bridge.loop.b", "loop.bridge.b"),
        ],
        ModelingOperationKind::ConstrainedBoolean => vec![
            part_selection("sel.boolean.host", "part.boolean.host"),
            boolean_operand_selection("sel.boolean.operand", "operand.boolean.cutter"),
        ],
        _ => vec![
            primary_selection_for(kind, 0),
            primary_selection_for(kind, 1),
        ],
    }
}

fn primary_selection_for(kind: ModelingOperationKind, ordinal: usize) -> SemanticSelection {
    match kind {
        ModelingOperationKind::RegionExtrude
        | ModelingOperationKind::RegionInset
        | ModelingOperationKind::Split
        | ModelingOperationKind::Separate
        | ModelingOperationKind::Subdivide
        | ModelingOperationKind::ShellSolidify => region_selection(
            &format!("sel.{kind:?}.{ordinal}.region").to_ascii_lowercase(),
            &format!("region.{kind:?}.{ordinal}.observed").to_ascii_lowercase(),
        ),
        ModelingOperationKind::LoopCut => edge_class_selection(
            &format!("sel.{kind:?}.{ordinal}.edge_loop").to_ascii_lowercase(),
            &format!("edge_loop.{kind:?}.{ordinal}.observed").to_ascii_lowercase(),
        ),
        ModelingOperationKind::Merge
        | ModelingOperationKind::Dissolve
        | ModelingOperationKind::Bevel => edge_class_selection(
            &format!("sel.{kind:?}.{ordinal}.edge_set").to_ascii_lowercase(),
            &format!("edge_set.{kind:?}.{ordinal}.observed").to_ascii_lowercase(),
        ),
        ModelingOperationKind::Join
        | ModelingOperationKind::Mirror
        | ModelingOperationKind::Array => part_selection(
            &format!("sel.{kind:?}.{ordinal}.part").to_ascii_lowercase(),
            &format!("part.{kind:?}.{ordinal}.observed").to_ascii_lowercase(),
        ),
        ModelingOperationKind::ConstrainedBoolean => {
            if ordinal == 0 {
                part_selection("sel.boolean.host", "part.boolean.host")
            } else {
                boolean_operand_selection("sel.boolean.operand", "operand.boolean.cutter")
            }
        }
        ModelingOperationKind::PrimitiveCreate => {
            spatial_box_selection("sel.primitive.bounds", [-0.5, -0.5, -0.5], [0.5, 0.5, 0.5])
        }
        _ => region_selection("sel.generic.region", "region.generic.observed"),
    }
}

fn part_selection(id: &str, part: &str) -> SemanticSelection {
    SemanticSelection {
        id: SemanticSelectionId(id.to_owned()),
        payload: SemanticSelectionPayload::Part {
            part: SemanticPartId(part.to_owned()),
        },
    }
}

fn region_selection(id: &str, region: &str) -> SemanticSelection {
    SemanticSelection {
        id: SemanticSelectionId(id.to_owned()),
        payload: SemanticSelectionPayload::Region {
            region: SemanticRegionId(region.to_owned()),
        },
    }
}

fn boundary_loop_selection(id: &str, boundary_loop: &str) -> SemanticSelection {
    SemanticSelection {
        id: SemanticSelectionId(id.to_owned()),
        payload: SemanticSelectionPayload::BoundaryLoop {
            boundary_loop: SemanticBoundaryLoopId(boundary_loop.to_owned()),
        },
    }
}

fn edge_class_selection(id: &str, class: &str) -> SemanticSelection {
    SemanticSelection {
        id: SemanticSelectionId(id.to_owned()),
        payload: SemanticSelectionPayload::EdgeClass {
            class: class.to_owned(),
        },
    }
}

fn boolean_operand_selection(id: &str, operand_id: &str) -> SemanticSelection {
    SemanticSelection {
        id: SemanticSelectionId(id.to_owned()),
        payload: SemanticSelectionPayload::BooleanOperand {
            operand_id: operand_id.to_owned(),
        },
    }
}

fn spatial_box_selection(id: &str, min: [f64; 3], max: [f64; 3]) -> SemanticSelection {
    SemanticSelection {
        id: SemanticSelectionId(id.to_owned()),
        payload: SemanticSelectionPayload::SpatialPrimitive {
            shape: SpatialPrimitiveSelection::Box { min, max },
        },
    }
}

fn estimated_parameters_for(kind: ModelingOperationKind) -> Vec<SemanticParameter> {
    match kind {
        ModelingOperationKind::PrimitiveCreate => vec![
            choice("primitive_class", "box"),
            scalar("width", 1.0),
            scalar("height", 1.0),
            scalar("depth", 1.0),
            vector3("center", [0.0, 0.0, 0.0]),
            quaternion("orientation", [0.0, 0.0, 0.0, 1.0]),
            integer("segments_u", 1),
            integer("segments_v", 1),
        ],
        ModelingOperationKind::RegionExtrude => vec![
            scalar("distance", 0.25),
            vector3("direction", [0.0, 0.0, 1.0]),
            boolean("keep_source_region", true),
            choice("profile", "linear"),
        ],
        ModelingOperationKind::RegionInset => vec![
            scalar("offset", 0.05),
            boolean("even_offset", true),
            choice("corner_policy", "miter"),
        ],
        ModelingOperationKind::LoopCut => vec![
            integer("cuts", 1),
            scalar("factor", 0.5),
            boolean("clamp_to_region", true),
        ],
        ModelingOperationKind::BridgeLoops => vec![
            integer("segments", 1),
            scalar("twist", 0.0),
            boolean("smooth", false),
            choice("pairing", "ordered"),
        ],
        ModelingOperationKind::Merge => vec![
            scalar("tolerance", 0.001),
            choice("survivor_policy", "stable_id"),
        ],
        ModelingOperationKind::Split => vec![
            choice("mode", "planar"),
            scalar("plane_offset", 0.0),
            vector3("plane_normal", [0.0, 0.0, 1.0]),
        ],
        ModelingOperationKind::Dissolve => {
            vec![
                choice("mode", "preserve_surface"),
                boolean("preserve_boundary", true),
            ]
        }
        ModelingOperationKind::Separate => {
            vec![
                choice("partition", "connected_component"),
                boolean("retain_transform", true),
            ]
        }
        ModelingOperationKind::Join => {
            vec![
                choice("merge_policy", "keep_parts"),
                boolean("weld_coincident", false),
            ]
        }
        ModelingOperationKind::Mirror => vec![
            vector3("plane_normal", [1.0, 0.0, 0.0]),
            scalar("plane_offset", 0.0),
            boolean("weld", true),
            scalar("weld_tolerance", 0.001),
            choice("side", "positive_to_negative"),
            boolean("preserve_source", true),
        ],
        ModelingOperationKind::Array => vec![
            vector3("offset", [1.0, 0.0, 0.0]),
            integer("count", 3),
            scalar("scale_step", 1.0),
            vector3("axis", [1.0, 0.0, 0.0]),
            scalar("rotation_step", 0.0),
            boolean("merge_adjacent", false),
            choice("ordering", "index"),
        ],
        ModelingOperationKind::Subdivide => vec![
            integer("levels", 1),
            choice("scheme", "catmull_clark"),
            scalar("sharpness_weight", 0.0),
            boolean("preserve_boundary", true),
        ],
        ModelingOperationKind::Bevel => vec![
            scalar("width", 0.03),
            integer("segments", 2),
            scalar("profile", 0.5),
            boolean("affect_vertices", false),
            choice("miter", "arc"),
        ],
        ModelingOperationKind::ShellSolidify => vec![
            scalar("thickness", 0.05),
            boolean("even_thickness", true),
            choice("direction", "outward"),
            boolean("fill_rim", true),
            scalar("rim_width", 0.0),
        ],
        ModelingOperationKind::ConstrainedBoolean => vec![
            choice("operation", "difference"),
            scalar("tolerance", 0.0005),
            boolean("keep_operand", false),
            choice("boundary_policy", "classified_cells"),
            scalar("minimum_clearance", 0.0),
            boolean("exact_cells", true),
        ],
        _ => Vec::new(),
    }
}

fn scalar(name: &str, value: f64) -> SemanticParameter {
    SemanticParameter::Scalar {
        name: name.to_owned(),
        value,
    }
}

fn integer(name: &str, value: i64) -> SemanticParameter {
    SemanticParameter::Integer {
        name: name.to_owned(),
        value,
    }
}

fn boolean(name: &str, value: bool) -> SemanticParameter {
    SemanticParameter::Boolean {
        name: name.to_owned(),
        value,
    }
}

fn choice(name: &str, value: &str) -> SemanticParameter {
    SemanticParameter::Choice {
        name: name.to_owned(),
        value: value.to_owned(),
    }
}

fn vector3(name: &str, value: [f64; 3]) -> SemanticParameter {
    SemanticParameter::Vector3 {
        name: name.to_owned(),
        value,
    }
}

fn quaternion(name: &str, value: [f64; 4]) -> SemanticParameter {
    SemanticParameter::Quaternion {
        name: name.to_owned(),
        value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shape_program::topology::{SelectionCount, topology_contract_for};

    #[test]
    fn generators_cover_every_topology_operator_kind() {
        let input = supported_input();
        let hypotheses = generate_topology_operation_hypotheses(&input);
        let kinds = hypotheses
            .iter()
            .map(|hypothesis| hypothesis.operation_kind)
            .collect::<Vec<_>>();

        assert_eq!(kinds, TOPOLOGY_OPERATOR_KINDS);
        assert_eq!(hypotheses.len(), TOPOLOGY_OPERATOR_KINDS.len());

        for hypothesis in hypotheses {
            let contract = topology_contract_for(hypothesis.operation_kind).unwrap();
            assert_eq!(
                hypothesis.estimated_parameters.len(),
                contract.semantic_parameter_count,
                "{:?} parameter count should match forward contract",
                hypothesis.operation_kind
            );
            assert_eq!(
                hypothesis.expected_topology_change.effect,
                contract.topology_effect
            );
            assert!(hypothesis.failure_reason.is_none());
            assert!(hypothesis.confidence > 0.0);
            assert!(hypothesis.lower_bound_score > 0.0);
        }
    }

    #[test]
    fn generation_is_deterministic() {
        let input = supported_input();

        let first = generate_topology_operation_hypotheses(&input);
        let second = generate_topology_operation_hypotheses(&input);

        assert_eq!(first, second);
    }

    #[test]
    fn sparse_evidence_still_returns_rejectable_candidates() {
        let input = HypothesisGenerationInput::default();
        let hypotheses = generate_topology_operation_hypotheses(&input);

        assert_eq!(hypotheses.len(), TOPOLOGY_OPERATOR_KINDS.len());
        for hypothesis in hypotheses {
            assert_eq!(hypothesis.confidence, 0.0);
            assert_eq!(hypothesis.lower_bound_score, 0.0);
            assert!(matches!(
                hypothesis.failure_reason,
                Some(HypothesisFailureReason::InsufficientEvidence { .. })
            ));
            assert_eq!(
                hypothesis.estimated_parameters.len(),
                topology_contract_for(hypothesis.operation_kind)
                    .unwrap()
                    .semantic_parameter_count
            );
        }
    }

    #[test]
    fn boolean_candidate_preserves_ordered_host_and_operand_slots() {
        let input = supported_input();
        let hypothesis = generate_topology_operation_hypothesis(
            ModelingOperationKind::ConstrainedBoolean,
            &input,
        );

        assert_eq!(hypothesis.semantic_selection.len(), 2);
        assert!(matches!(
            hypothesis.semantic_selection[0].payload,
            SemanticSelectionPayload::Part { .. }
        ));
        assert!(matches!(
            hypothesis.semantic_selection[1].payload,
            SemanticSelectionPayload::BooleanOperand { .. }
        ));
    }

    #[test]
    fn selection_counts_follow_forward_contracts() {
        let input = supported_input();
        for hypothesis in generate_topology_operation_hypotheses(&input) {
            let contract = topology_contract_for(hypothesis.operation_kind).unwrap();
            let actual = hypothesis.semantic_selection.len();
            match contract.selection_requirements.count {
                SelectionCount::ExactlyZero => assert_eq!(actual, 0),
                SelectionCount::ExactlyOne => assert_eq!(actual, 1),
                SelectionCount::ExactlyTwo => assert_eq!(actual, 2),
                SelectionCount::OneOrMore => assert!(actual >= 1),
                SelectionCount::TwoOrMore => assert!(actual >= 2),
            }
        }
    }

    fn supported_input() -> HypothesisGenerationInput {
        HypothesisGenerationInput {
            source: TopologyStateDescriptor {
                part_count: 2,
                object_count: 1,
                region_count: 8,
                boundary_loop_count: 8,
                edge_loop_count: 4,
                edge_set_count: 4,
                vertex_set_count: 2,
                sharp_edge_count: 8,
                shell_layer_count: 0,
                closed_volume_overlap_count: 0,
            },
            target: TopologyStateDescriptor {
                part_count: 4,
                object_count: 1,
                region_count: 16,
                boundary_loop_count: 12,
                edge_loop_count: 7,
                edge_set_count: 5,
                vertex_set_count: 3,
                sharp_edge_count: 12,
                shell_layer_count: 1,
                closed_volume_overlap_count: 1,
            },
            evidence: TopologyEvidenceDescriptor::all_supported(),
        }
    }
}
