#![forbid(unsafe_code)]

//! Strict semantic modeling-program verification.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_program::{
    CANONICAL_EVALUATOR_VERSION, GrammarProfile, MODELING_PROGRAM_SCHEMA_VERSION,
    ModelingOperation, ModelingProgram, OperationPayloadKind, ProgramOperationId, RawGeometrySize,
    SemanticAdmissibilityPolicy, SemanticSelection, SemanticSelectionId, SemanticTopologyExact,
    SerializationOrderExact,
};
use thiserror::Error;

/// Evidence supplied by a canonical evaluator or audit adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrictVerificationEvidence {
    /// Exact canonical positions, bit-for-bit under evaluator rules.
    pub canonical_positions_exact: bool,
    /// Exact semantic topology channels.
    pub semantic_topology_exact: SemanticTopologyExact,
    /// Exact vertex and face serialization order.
    pub serialization_order_exact: SerializationOrderExact,
    /// Residual bytes used to close the result.
    pub residual_bytes: usize,
    /// Literal target mesh bytes embedded in the explanation.
    pub literal_target_mesh_bytes: usize,
    /// One independent position parameter per vertex count.
    pub per_vertex_independent_position_parameters: usize,
    /// Whether nearby semantic perturbations remain valid.
    pub perturbation_valid: bool,
    /// Audit/export adapter bytes for target index permutations.
    ///
    /// These bytes are reported but never counted as semantic explanation.
    pub target_index_permutation_adapter_bytes: usize,
}

/// Complete strict verification report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrictSemanticVerification {
    /// Whether every strict-success gate passed.
    pub accepted: bool,
    /// Exact canonical positions.
    pub canonical_positions_exact: bool,
    /// Exact semantic topology channels.
    pub semantic_topology_exact: SemanticTopologyExact,
    /// Exact serialization order channels.
    pub serialization_order_exact: SerializationOrderExact,
    /// Residual bytes used by the candidate explanation.
    pub residual_bytes: usize,
    /// Literal target mesh bytes used by the candidate explanation.
    pub literal_target_mesh_bytes: usize,
    /// Per-vertex independent position parameters.
    pub per_vertex_independent_position_parameters: usize,
    /// Whether every operation passed admissibility checks.
    pub every_operation_admissible: bool,
    /// Serialized modeling-program size.
    pub program_description_size: usize,
    /// Raw target geometry size.
    pub raw_geometry_size: RawGeometrySize,
    /// Raw geometry size divided by program description size.
    pub compression_ratio: f64,
    /// Audit/export-only adapter bytes, excluded from semantic explanation.
    pub target_index_permutation_adapter_bytes: usize,
    /// Verification issues.
    pub issues: Vec<StrictVerificationIssue>,
    /// Per-operation admissibility reports.
    pub operation_reports: Vec<OperationAdmissibilityReport>,
}

/// One strict verification issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrictVerificationIssue {
    /// Stable issue code.
    pub code: StrictVerificationIssueCode,
    /// Human-readable target path.
    pub path: String,
    /// Human-readable message.
    pub message: String,
}

/// Stable strict verification issue code.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrictVerificationIssueCode {
    UnsupportedSchemaVersion,
    UnsupportedEvaluatorVersion,
    InvalidBaseTopologyContract,
    DuplicateOperationId,
    DuplicateSelectionId,
    InvalidDependencyGraph,
    CanonicalPositionsNotExact,
    SemanticTopologyNotExact,
    SerializationOrderNotExact,
    ResidualBytesPresent,
    LiteralTargetMeshBytesPresent,
    PerVertexIndependentPositionsPresent,
    OperationNotAdmissible,
    CompressionRatioTooLow,
    PerturbationValidityMissing,
}

/// Per-operation admissibility report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationAdmissibilityReport {
    /// Operation ID.
    pub operation_id: ProgramOperationId,
    /// Whether this operation is admissible.
    pub admissible: bool,
    /// Operation-level issues.
    pub issues: Vec<OperationAdmissibilityIssue>,
}

/// One operation admissibility issue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationAdmissibilityIssue {
    /// Stable issue code.
    pub code: OperationAdmissibilityIssueCode,
    /// Human-readable message.
    pub message: String,
}

/// Stable operation admissibility issue code.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationAdmissibilityIssueCode {
    ForbiddenOperationKind,
    ForbiddenPayloadKind,
    ParameterGrowthTooHigh,
    MissingPerturbationValidity,
    ExplicitSelectionPayloadTooLarge,
    UnknownSelection,
}

/// Verification error.
#[derive(Debug, Error)]
pub enum StrictVerificationError {
    /// Program serialization failed.
    #[error("failed to serialize modeling program: {0}")]
    ProgramSerialization(#[from] shape_program::ModelingProgramError),
}

/// Verify a modeling program against strict semantic-success gates.
pub fn verify_strict_semantic_program(
    program: &ModelingProgram,
    policy: &SemanticAdmissibilityPolicy,
    raw_geometry_size: RawGeometrySize,
    evidence: &StrictVerificationEvidence,
) -> Result<StrictSemanticVerification, StrictVerificationError> {
    let mut issues = Vec::new();
    validate_program_header(program, &mut issues);
    validate_program_structure(program, &mut issues);

    if !evidence.canonical_positions_exact {
        push_issue(
            &mut issues,
            StrictVerificationIssueCode::CanonicalPositionsNotExact,
            "canonical_positions",
            "Canonical positions are not exact.",
        );
    }
    if !evidence.semantic_topology_exact.is_exact() {
        push_issue(
            &mut issues,
            StrictVerificationIssueCode::SemanticTopologyNotExact,
            "semantic_topology",
            "Semantic topology is not exact.",
        );
    }
    if !evidence.serialization_order_exact.is_exact() {
        push_issue(
            &mut issues,
            StrictVerificationIssueCode::SerializationOrderNotExact,
            "serialization_order",
            "Vertex or face serialization order is not exact.",
        );
    }
    if evidence.residual_bytes != 0 {
        push_issue(
            &mut issues,
            StrictVerificationIssueCode::ResidualBytesPresent,
            "residual_bytes",
            "Strict success cannot use residual bytes.",
        );
    }
    if evidence.literal_target_mesh_bytes != 0 {
        push_issue(
            &mut issues,
            StrictVerificationIssueCode::LiteralTargetMeshBytesPresent,
            "literal_target_mesh_bytes",
            "Strict success cannot embed literal target mesh bytes.",
        );
    }
    if evidence.per_vertex_independent_position_parameters != 0 {
        push_issue(
            &mut issues,
            StrictVerificationIssueCode::PerVertexIndependentPositionsPresent,
            "per_vertex_independent_position_parameters",
            "Strict success cannot use independent per-vertex positions.",
        );
    }
    if policy.perturbation_validity_required && !evidence.perturbation_valid {
        push_issue(
            &mut issues,
            StrictVerificationIssueCode::PerturbationValidityMissing,
            "perturbation_valid",
            "Strict success requires a valid nearby perturbation neighborhood.",
        );
    }

    let selection_map = program
        .selections
        .iter()
        .map(|selection| (selection.id.clone(), selection))
        .collect::<BTreeMap<_, _>>();
    let operation_reports = program
        .operations
        .iter()
        .map(|operation| assess_operation_admissibility(operation, &selection_map, policy))
        .collect::<Vec<_>>();
    let every_operation_admissible = operation_reports.iter().all(|report| report.admissible);
    if !every_operation_admissible {
        push_issue(
            &mut issues,
            StrictVerificationIssueCode::OperationNotAdmissible,
            "operations",
            "One or more operations fail semantic admissibility.",
        );
    }

    let program_description_size = program.description_size_bytes()?;
    let compression_ratio = if program_description_size == 0 {
        0.0
    } else {
        raw_geometry_size.total_bytes() as f64 / program_description_size as f64
    };
    if compression_ratio < policy.minimum_compression_ratio {
        push_issue(
            &mut issues,
            StrictVerificationIssueCode::CompressionRatioTooLow,
            "compression_ratio",
            "Program is not compact enough relative to raw geometry.",
        );
    }

    let accepted = issues.is_empty();
    Ok(StrictSemanticVerification {
        accepted,
        canonical_positions_exact: evidence.canonical_positions_exact,
        semantic_topology_exact: evidence.semantic_topology_exact,
        serialization_order_exact: evidence.serialization_order_exact,
        residual_bytes: evidence.residual_bytes,
        literal_target_mesh_bytes: evidence.literal_target_mesh_bytes,
        per_vertex_independent_position_parameters: evidence
            .per_vertex_independent_position_parameters,
        every_operation_admissible,
        program_description_size,
        raw_geometry_size,
        compression_ratio,
        target_index_permutation_adapter_bytes: evidence.target_index_permutation_adapter_bytes,
        issues,
        operation_reports,
    })
}

/// Assess one operation against semantic admissibility.
#[must_use]
pub fn assess_operation_admissibility(
    operation: &ModelingOperation,
    selections: &BTreeMap<SemanticSelectionId, &SemanticSelection>,
    policy: &SemanticAdmissibilityPolicy,
) -> OperationAdmissibilityReport {
    let mut issues = Vec::new();
    if policy.forbidden_operation_kinds.contains(&operation.kind) {
        issues.push(OperationAdmissibilityIssue {
            code: OperationAdmissibilityIssueCode::ForbiddenOperationKind,
            message: format!("Operation kind {:?} is forbidden.", operation.kind),
        });
    }

    check_parameter_growth(
        operation.parameters.len(),
        operation.affected_element_count,
        policy,
        "Direct operation parameters",
        &mut issues,
    );

    for payload in &operation.payloads {
        if policy
            .forbidden_opaque_payload_kinds
            .contains(&payload.kind)
        {
            issues.push(OperationAdmissibilityIssue {
                code: OperationAdmissibilityIssueCode::ForbiddenPayloadKind,
                message: format!("Payload kind {:?} is forbidden.", payload.kind),
            });
        }
        if payload.kind == OperationPayloadKind::ExplicitSelectionIndices
            && payload.affected_element_count > policy.maximum_explicit_selection_payload
        {
            issues.push(OperationAdmissibilityIssue {
                code: OperationAdmissibilityIssueCode::ExplicitSelectionPayloadTooLarge,
                message: format!(
                    "Explicit selection payload declares {} element IDs.",
                    payload.affected_element_count
                ),
            });
        }
        check_parameter_growth(
            payload.semantic_parameter_count,
            payload.affected_element_count,
            policy,
            "Payload",
            &mut issues,
        );
        if policy.perturbation_validity_required && !payload.perturbation_valid {
            issues.push(OperationAdmissibilityIssue {
                code: OperationAdmissibilityIssueCode::MissingPerturbationValidity,
                message: "Payload does not declare perturbation validity.".to_owned(),
            });
        }
    }

    for selection_id in &operation.selections {
        let Some(selection) = selections.get(selection_id) else {
            issues.push(OperationAdmissibilityIssue {
                code: OperationAdmissibilityIssueCode::UnknownSelection,
                message: format!("Selection {} is not declared.", selection_id.0),
            });
            continue;
        };
        let explicit_len = selection.explicit_payload_len();
        if explicit_len > policy.maximum_explicit_selection_payload {
            issues.push(OperationAdmissibilityIssue {
                code: OperationAdmissibilityIssueCode::ExplicitSelectionPayloadTooLarge,
                message: format!("Explicit selection contains {explicit_len} element IDs."),
            });
        }
    }

    OperationAdmissibilityReport {
        operation_id: operation.id.clone(),
        admissible: issues.is_empty(),
        issues,
    }
}

fn check_parameter_growth(
    semantic_parameter_count: usize,
    affected_element_count: usize,
    policy: &SemanticAdmissibilityPolicy,
    label: &str,
    issues: &mut Vec<OperationAdmissibilityIssue>,
) {
    if affected_element_count == 0 && semantic_parameter_count > 0 {
        issues.push(OperationAdmissibilityIssue {
            code: OperationAdmissibilityIssueCode::ParameterGrowthTooHigh,
            message: format!("{label} has parameters but no affected elements."),
        });
    } else if affected_element_count > 0 {
        let ratio = semantic_parameter_count as f64 / affected_element_count as f64;
        if ratio > policy.maximum_parameter_growth_relative_to_affected_elements {
            issues.push(OperationAdmissibilityIssue {
                code: OperationAdmissibilityIssueCode::ParameterGrowthTooHigh,
                message: format!("{label} parameter growth ratio {ratio:.3} exceeds policy."),
            });
        }
    }
}

fn validate_program_header(program: &ModelingProgram, issues: &mut Vec<StrictVerificationIssue>) {
    if program.schema_version != MODELING_PROGRAM_SCHEMA_VERSION {
        push_issue(
            issues,
            StrictVerificationIssueCode::UnsupportedSchemaVersion,
            "schema_version",
            "Modeling program schema version is not supported.",
        );
    }
    if program.canonical_evaluator_version != CANONICAL_EVALUATOR_VERSION {
        push_issue(
            issues,
            StrictVerificationIssueCode::UnsupportedEvaluatorVersion,
            "canonical_evaluator_version",
            "Canonical evaluator version is not supported.",
        );
    }
    match program.grammar_profile {
        GrammarProfile::StrictFromPrimitives if program.base_topology.is_some() => {
            push_issue(
                issues,
                StrictVerificationIssueCode::InvalidBaseTopologyContract,
                "base_topology",
                "Strict-from-primitives programs must not declare a base topology.",
            );
        }
        GrammarProfile::StrictFromVersionedLibrary => {
            let valid = program.base_topology.as_ref().is_some_and(|base| {
                !base.catalog_id.is_empty()
                    && !base.version.is_empty()
                    && !base.fingerprint.is_empty()
            });
            if !valid {
                push_issue(
                    issues,
                    StrictVerificationIssueCode::InvalidBaseTopologyContract,
                    "base_topology",
                    "Strict-from-versioned-library programs require a cataloged base topology.",
                );
            }
        }
        GrammarProfile::StrictFromPrimitives => {}
    }
}

fn validate_program_structure(
    program: &ModelingProgram,
    issues: &mut Vec<StrictVerificationIssue>,
) {
    let mut operation_order = BTreeMap::new();
    let mut duplicate_operations = BTreeSet::new();
    for (index, operation) in program.operations.iter().enumerate() {
        if operation_order
            .insert(operation.id.clone(), index)
            .is_some()
        {
            duplicate_operations.insert(operation.id.clone());
        }
    }
    for operation_id in duplicate_operations {
        push_issue(
            issues,
            StrictVerificationIssueCode::DuplicateOperationId,
            "operations",
            format!("Duplicate operation ID {}.", operation_id.0),
        );
    }

    let mut selection_ids = BTreeSet::new();
    let mut duplicate_selections = BTreeSet::new();
    for selection in &program.selections {
        if !selection_ids.insert(selection.id.clone()) {
            duplicate_selections.insert(selection.id.clone());
        }
    }
    for selection_id in duplicate_selections {
        push_issue(
            issues,
            StrictVerificationIssueCode::DuplicateSelectionId,
            "selections",
            format!("Duplicate selection ID {}.", selection_id.0),
        );
    }

    let mut adjacency = BTreeMap::<ProgramOperationId, Vec<ProgramOperationId>>::new();
    for (producer, consumer) in &program.dependency_graph.operation_edges {
        let Some(producer_index) = operation_order.get(producer) else {
            push_issue(
                issues,
                StrictVerificationIssueCode::InvalidDependencyGraph,
                "dependency_graph.operation_edges",
                format!("Unknown producer operation {}.", producer.0),
            );
            continue;
        };
        let Some(consumer_index) = operation_order.get(consumer) else {
            push_issue(
                issues,
                StrictVerificationIssueCode::InvalidDependencyGraph,
                "dependency_graph.operation_edges",
                format!("Unknown consumer operation {}.", consumer.0),
            );
            continue;
        };
        if producer_index >= consumer_index {
            push_issue(
                issues,
                StrictVerificationIssueCode::InvalidDependencyGraph,
                "dependency_graph.operation_edges",
                format!(
                    "Operation dependency {} -> {} violates operation order.",
                    producer.0, consumer.0
                ),
            );
        }
        adjacency
            .entry(producer.clone())
            .or_default()
            .push(consumer.clone());
    }

    if dependency_graph_has_cycle(&adjacency) {
        push_issue(
            issues,
            StrictVerificationIssueCode::InvalidDependencyGraph,
            "dependency_graph.operation_edges",
            "Operation dependency graph contains a cycle.",
        );
    }

    for (selection, operation) in &program.dependency_graph.selection_edges {
        if !selection_ids.contains(selection) {
            push_issue(
                issues,
                StrictVerificationIssueCode::InvalidDependencyGraph,
                "dependency_graph.selection_edges",
                format!("Unknown selection {}.", selection.0),
            );
        }
        if !operation_order.contains_key(operation) {
            push_issue(
                issues,
                StrictVerificationIssueCode::InvalidDependencyGraph,
                "dependency_graph.selection_edges",
                format!("Unknown operation {}.", operation.0),
            );
        }
    }
}

fn dependency_graph_has_cycle(
    adjacency: &BTreeMap<ProgramOperationId, Vec<ProgramOperationId>>,
) -> bool {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for node in adjacency.keys() {
        if dependency_node_has_cycle(node, adjacency, &mut visiting, &mut visited) {
            return true;
        }
    }
    false
}

fn dependency_node_has_cycle(
    node: &ProgramOperationId,
    adjacency: &BTreeMap<ProgramOperationId, Vec<ProgramOperationId>>,
    visiting: &mut BTreeSet<ProgramOperationId>,
    visited: &mut BTreeSet<ProgramOperationId>,
) -> bool {
    if visited.contains(node) {
        return false;
    }
    if !visiting.insert(node.clone()) {
        return true;
    }
    if let Some(next_nodes) = adjacency.get(node) {
        for next in next_nodes {
            if dependency_node_has_cycle(next, adjacency, visiting, visited) {
                return true;
            }
        }
    }
    visiting.remove(node);
    visited.insert(node.clone());
    false
}

fn push_issue(
    issues: &mut Vec<StrictVerificationIssue>,
    code: StrictVerificationIssueCode,
    path: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(StrictVerificationIssue {
        code,
        path: path.into(),
        message: message.into(),
    });
}

#[cfg(test)]
mod tests {
    use shape_program::{
        ExplicitSelectionTarget, ModelingOperationKind, OperationPayloadDescriptor,
        OperationPayloadKind, SemanticParameter, SemanticSelectionPayload,
    };

    use super::*;

    fn exact_evidence() -> StrictVerificationEvidence {
        StrictVerificationEvidence {
            canonical_positions_exact: true,
            semantic_topology_exact: SemanticTopologyExact {
                graph: true,
                polygon_boundaries: true,
                winding: true,
                part_object_membership: true,
                geometry: true,
            },
            serialization_order_exact: SerializationOrderExact {
                vertex_order: true,
                face_order: true,
            },
            residual_bytes: 0,
            literal_target_mesh_bytes: 0,
            per_vertex_independent_position_parameters: 0,
            perturbation_valid: true,
            target_index_permutation_adapter_bytes: 0,
        }
    }

    fn large_raw_geometry() -> RawGeometrySize {
        RawGeometrySize {
            vertex_count: 1_000,
            face_count: 2_000,
            position_bytes: 12_000,
            topology_bytes: 24_000,
        }
    }

    #[test]
    fn strict_success_requires_exact_zero_residual_program() {
        let mut program = ModelingProgram::strict_from_primitives();
        program.operations.push(ModelingOperation::compact(
            "op.primitive",
            ModelingOperationKind::PrimitiveCreate,
        ));

        let report = verify_strict_semantic_program(
            &program,
            &SemanticAdmissibilityPolicy::strict(),
            large_raw_geometry(),
            &exact_evidence(),
        )
        .expect("verification should run");

        assert!(report.accepted);
        assert_eq!(report.residual_bytes, 0);
        assert_eq!(report.literal_target_mesh_bytes, 0);
        assert_eq!(report.per_vertex_independent_position_parameters, 0);
    }

    #[test]
    fn residual_or_literal_target_mesh_blocks_strict_success() {
        let program = ModelingProgram::strict_from_primitives();
        let mut evidence = exact_evidence();
        evidence.residual_bytes = 1;
        evidence.literal_target_mesh_bytes = 12;

        let report = verify_strict_semantic_program(
            &program,
            &SemanticAdmissibilityPolicy::strict(),
            large_raw_geometry(),
            &evidence,
        )
        .expect("verification should run");

        assert!(!report.accepted);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.code == StrictVerificationIssueCode::ResidualBytesPresent })
        );
        assert!(report.issues.iter().any(|issue| {
            issue.code == StrictVerificationIssueCode::LiteralTargetMeshBytesPresent
        }));
    }

    #[test]
    fn forbidden_operation_and_payload_are_rejected() {
        let mut program = ModelingProgram::strict_from_primitives();
        let mut operation =
            ModelingOperation::compact("op.cheat", ModelingOperationKind::MoveVertex);
        operation.payloads.push(OperationPayloadDescriptor {
            kind: OperationPayloadKind::PerVertexIndependentPositions,
            encoded_bytes: 4096,
            semantic_parameter_count: 300,
            affected_element_count: 100,
            perturbation_valid: false,
        });
        program.operations.push(operation);

        let report = verify_strict_semantic_program(
            &program,
            &SemanticAdmissibilityPolicy::strict(),
            large_raw_geometry(),
            &exact_evidence(),
        )
        .expect("verification should run");

        assert!(!report.accepted);
        let operation_report = &report.operation_reports[0];
        assert!(operation_report.issues.iter().any(|issue| {
            issue.code == OperationAdmissibilityIssueCode::ForbiddenOperationKind
        }));
        assert!(
            operation_report.issues.iter().any(|issue| {
                issue.code == OperationAdmissibilityIssueCode::ForbiddenPayloadKind
            })
        );
    }

    #[test]
    fn explicit_selection_payload_limit_is_enforced() {
        let mut program = ModelingProgram::strict_from_primitives();
        program.selections.push(SemanticSelection {
            id: SemanticSelectionId("sel.faces".to_owned()),
            payload: SemanticSelectionPayload::ExplicitIndices {
                target: ExplicitSelectionTarget::Face,
                indices: (0..100).collect(),
            },
        });
        let mut operation =
            ModelingOperation::compact("op.inset", ModelingOperationKind::RegionInset);
        operation
            .selections
            .push(SemanticSelectionId("sel.faces".to_owned()));
        program.operations.push(operation);

        let report = verify_strict_semantic_program(
            &program,
            &SemanticAdmissibilityPolicy::strict(),
            large_raw_geometry(),
            &exact_evidence(),
        )
        .expect("verification should run");

        assert!(!report.accepted);
        assert!(report.operation_reports[0].issues.iter().any(|issue| {
            issue.code == OperationAdmissibilityIssueCode::ExplicitSelectionPayloadTooLarge
        }));
    }

    #[test]
    fn direct_operation_parameters_count_against_growth_policy() {
        let mut program = ModelingProgram::strict_from_primitives();
        let mut operation = ModelingOperation::compact(
            "op.region.transform",
            ModelingOperationKind::RegionTransform,
        );
        operation.affected_element_count = 4;
        operation.parameters.push(SemanticParameter::Scalar {
            name: "offset_u".to_owned(),
            value: 0.1,
        });
        operation.parameters.push(SemanticParameter::Scalar {
            name: "offset_v".to_owned(),
            value: 0.2,
        });
        program.operations.push(operation);

        let report = verify_strict_semantic_program(
            &program,
            &SemanticAdmissibilityPolicy::strict(),
            large_raw_geometry(),
            &exact_evidence(),
        )
        .expect("verification should run");

        assert!(!report.accepted);
        assert!(report.operation_reports[0].issues.iter().any(|issue| {
            issue.code == OperationAdmissibilityIssueCode::ParameterGrowthTooHigh
        }));
    }

    #[test]
    fn explicit_index_payload_descriptor_limit_is_enforced() {
        let mut program = ModelingProgram::strict_from_primitives();
        let mut operation =
            ModelingOperation::compact("op.explicit.select", ModelingOperationKind::RegionInset);
        operation.payloads.push(OperationPayloadDescriptor {
            kind: OperationPayloadKind::ExplicitSelectionIndices,
            encoded_bytes: 400,
            semantic_parameter_count: 0,
            affected_element_count: 100,
            perturbation_valid: true,
        });
        program.operations.push(operation);

        let report = verify_strict_semantic_program(
            &program,
            &SemanticAdmissibilityPolicy::strict(),
            large_raw_geometry(),
            &exact_evidence(),
        )
        .expect("verification should run");

        assert!(!report.accepted);
        assert!(report.operation_reports[0].issues.iter().any(|issue| {
            issue.code == OperationAdmissibilityIssueCode::ExplicitSelectionPayloadTooLarge
        }));
    }

    #[test]
    fn invalid_dependency_graph_blocks_strict_success() {
        let mut program = ModelingProgram::strict_from_primitives();
        program.operations.push(ModelingOperation::compact(
            "op.first",
            ModelingOperationKind::PrimitiveCreate,
        ));
        program.operations.push(ModelingOperation::compact(
            "op.second",
            ModelingOperationKind::RegionInset,
        ));
        program.dependency_graph.operation_edges.push((
            ProgramOperationId("op.second".to_owned()),
            ProgramOperationId("op.first".to_owned()),
        ));
        program.dependency_graph.selection_edges.push((
            SemanticSelectionId("missing.selection".to_owned()),
            ProgramOperationId("op.second".to_owned()),
        ));

        let report = verify_strict_semantic_program(
            &program,
            &SemanticAdmissibilityPolicy::strict(),
            large_raw_geometry(),
            &exact_evidence(),
        )
        .expect("verification should run");

        assert!(!report.accepted);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.code == StrictVerificationIssueCode::InvalidDependencyGraph })
        );
    }

    #[test]
    fn duplicate_program_ids_block_strict_success() {
        let mut program = ModelingProgram::strict_from_primitives();
        program.operations.push(ModelingOperation::compact(
            "op.same",
            ModelingOperationKind::PrimitiveCreate,
        ));
        program.operations.push(ModelingOperation::compact(
            "op.same",
            ModelingOperationKind::RegionInset,
        ));
        program.selections.push(SemanticSelection {
            id: SemanticSelectionId("sel.same".to_owned()),
            payload: SemanticSelectionPayload::SemanticLandmarkGroup {
                group_id: "landmarks".to_owned(),
            },
        });
        program.selections.push(SemanticSelection {
            id: SemanticSelectionId("sel.same".to_owned()),
            payload: SemanticSelectionPayload::SemanticLandmarkGroup {
                group_id: "other".to_owned(),
            },
        });

        let report = verify_strict_semantic_program(
            &program,
            &SemanticAdmissibilityPolicy::strict(),
            large_raw_geometry(),
            &exact_evidence(),
        )
        .expect("verification should run");

        assert!(!report.accepted);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.code == StrictVerificationIssueCode::DuplicateOperationId })
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.code == StrictVerificationIssueCode::DuplicateSelectionId })
        );
    }

    #[test]
    fn target_index_permutation_adapter_does_not_count_as_residual() {
        let program = ModelingProgram::strict_from_primitives();
        let mut evidence = exact_evidence();
        evidence.target_index_permutation_adapter_bytes = 512;

        let report = verify_strict_semantic_program(
            &program,
            &SemanticAdmissibilityPolicy::strict(),
            large_raw_geometry(),
            &evidence,
        )
        .expect("verification should run");

        assert!(report.accepted);
        assert_eq!(report.target_index_permutation_adapter_bytes, 512);
        assert_eq!(report.residual_bytes, 0);
    }
}
