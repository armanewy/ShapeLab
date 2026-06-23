//! Strict inverse verification and failure taxonomy contracts.
//!
//! This module is the inverse-search side of the strict semantic contract.  It
//! does not run reconstruction search; it classifies why a candidate did not
//! prove exact semantic reconstruction and delegates final semantic admissibility
//! checks to `shape_program_verify`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shape_program::{
    ModelingOperationKind, ModelingProgram, ProgramOperationId, RawGeometrySize,
    SemanticAdmissibilityPolicy,
};
use shape_program_verify::{
    OperationAdmissibilityIssueCode, OperationAdmissibilityReport, StrictSemanticVerification,
    StrictVerificationEvidence, StrictVerificationIssue, StrictVerificationIssueCode,
    verify_strict_semantic_program,
};

use crate::{
    MissingOperatorCapability, ResidualCarrier, ResidualDiagnostic, SearchLimitReached,
    StrictReconstructionFailureReport, UnexplainedGeometryRegion, UnexplainedTopologyRegion,
};

/// Strict inverse failure classes that are actionable by the next search pass.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrictInverseFailureClass {
    /// The target contains a primitive family the inverse compiler cannot seed.
    MissingPrimitive,
    /// The target requires a topology operation missing from the forward grammar.
    MissingTopologyOperator,
    /// The target requires a deformation operation missing from the forward grammar.
    MissingDeformationOperator,
    /// A required target set cannot be represented as a semantic selection.
    SelectionNotExpressible,
    /// Search reached a deterministic limit before proof.
    SearchExhaustion,
    /// Geometry replay is numerically non-exact or contains a residual.
    NumericalNonExactness,
    /// Canonical vertex, face, operation, or serialization order cannot be proven.
    UnsupportedSerializationOrder,
}

impl StrictInverseFailureClass {
    /// Stable one-line class label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::MissingPrimitive => "missing primitive",
            Self::MissingTopologyOperator => "missing topology operator",
            Self::MissingDeformationOperator => "missing deformation operator",
            Self::SelectionNotExpressible => "selection not expressible",
            Self::SearchExhaustion => "search exhaustion",
            Self::NumericalNonExactness => "numerical non-exactness",
            Self::UnsupportedSerializationOrder => "unsupported serialization order",
        }
    }

    /// Default remediation hint for this class.
    #[must_use]
    pub fn default_next_action(self) -> &'static str {
        match self {
            Self::MissingPrimitive => {
                "add a semantic primitive recognizer or map the target to a versioned library base"
            }
            Self::MissingTopologyOperator => {
                "add or enable a forward topology operator with provenance and stable IDs"
            }
            Self::MissingDeformationOperator => {
                "recover the deformation as a compact semantic operator, not dense deltas"
            }
            Self::SelectionNotExpressible => {
                "replace explicit target indices with a semantic selector or introduce a selector type"
            }
            Self::SearchExhaustion => {
                "increase the bounded search frontier or add a more specific hypothesis generator"
            }
            Self::NumericalNonExactness => {
                "make canonical replay exact; residual correction cannot count as strict success"
            }
            Self::UnsupportedSerializationOrder => {
                "define deterministic operation, vertex, face, and adapter ordering before accepting"
            }
        }
    }
}

/// One classified strict inverse failure.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StrictInverseFailure {
    /// Failure class.
    pub class: StrictInverseFailureClass,
    /// Stable diagnostic path, for example `search.beam` or `evidence.residual_bytes`.
    pub path: String,
    /// Concrete evidence observed by inverse search or verification.
    pub detail: String,
    /// Suggested next implementation or search action.
    pub next_action: String,
    /// Forward operation kind related to the failure, when known.
    pub operation_kind: Option<ModelingOperationKind>,
}

impl StrictInverseFailure {
    /// Build a failure with the class default remediation hint.
    #[must_use]
    pub fn new(
        class: StrictInverseFailureClass,
        path: impl Into<String>,
        detail: impl Into<String>,
        operation_kind: Option<ModelingOperationKind>,
    ) -> Self {
        Self {
            class,
            path: path.into(),
            detail: detail.into(),
            next_action: class.default_next_action().to_owned(),
            operation_kind,
        }
    }

    /// Missing primitive failure.
    #[must_use]
    pub fn missing_primitive(path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(
            StrictInverseFailureClass::MissingPrimitive,
            path,
            detail,
            Some(ModelingOperationKind::PrimitiveCreate),
        )
    }

    /// Missing topology operator failure.
    #[must_use]
    pub fn missing_topology_operator(
        path: impl Into<String>,
        detail: impl Into<String>,
        operation_kind: ModelingOperationKind,
    ) -> Self {
        Self::new(
            StrictInverseFailureClass::MissingTopologyOperator,
            path,
            detail,
            Some(operation_kind),
        )
    }

    /// Missing deformation operator failure.
    #[must_use]
    pub fn missing_deformation_operator(
        path: impl Into<String>,
        detail: impl Into<String>,
        operation_kind: ModelingOperationKind,
    ) -> Self {
        Self::new(
            StrictInverseFailureClass::MissingDeformationOperator,
            path,
            detail,
            Some(operation_kind),
        )
    }

    /// Inexpressible semantic selection failure.
    #[must_use]
    pub fn selection_not_expressible(path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(
            StrictInverseFailureClass::SelectionNotExpressible,
            path,
            detail,
            None,
        )
    }

    /// Search exhaustion failure.
    #[must_use]
    pub fn search_exhaustion(stage: impl Into<String>, limit: usize, explored: usize) -> Self {
        let stage = stage.into();
        Self::new(
            StrictInverseFailureClass::SearchExhaustion,
            stage.clone(),
            format!("stage `{stage}` explored {explored} candidate(s) at limit {limit}"),
            None,
        )
    }

    /// Numerical non-exactness failure.
    #[must_use]
    pub fn numerical_non_exactness(path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(
            StrictInverseFailureClass::NumericalNonExactness,
            path,
            detail,
            None,
        )
    }

    /// Unsupported canonical serialization-order failure.
    #[must_use]
    pub fn unsupported_serialization_order(
        path: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self::new(
            StrictInverseFailureClass::UnsupportedSerializationOrder,
            path,
            detail,
            None,
        )
    }

    /// Return true when the failure contains enough information to route work.
    #[must_use]
    pub fn is_actionable(&self) -> bool {
        !self.path.trim().is_empty()
            && !self.detail.trim().is_empty()
            && !self.next_action.trim().is_empty()
    }
}

/// Strict inverse verification output for one candidate reconstruction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrictInverseVerificationReport {
    /// True only when the semantic verifier accepted and no inverse failures remain.
    pub strict_success: bool,
    /// Raw semantic verification, when a program was supplied and could be checked.
    pub verification: Option<StrictSemanticVerification>,
    /// Classified inverse failures.
    pub failures: Vec<StrictInverseFailure>,
    /// Compatibility report shape already used by the inverse crate.
    pub reconstruction_failure: Option<StrictReconstructionFailureReport>,
}

impl StrictInverseVerificationReport {
    /// Return true only for proven strict inverse success.
    #[must_use]
    pub fn is_strict_success(&self) -> bool {
        self.strict_success
            && self.verification.as_ref().is_some_and(|verification| {
                verification.accepted && verification.residual_bytes == 0
            })
            && self.failures.is_empty()
            && self.reconstruction_failure.is_none()
    }
}

/// Verify one inverse candidate with the default strict semantic policy.
#[must_use]
pub fn verify_strict_inverse_candidate(
    program: Option<&ModelingProgram>,
    raw_geometry_size: RawGeometrySize,
    evidence: &StrictVerificationEvidence,
    failure_hints: Vec<StrictInverseFailure>,
) -> StrictInverseVerificationReport {
    verify_strict_inverse_candidate_with_policy(
        program,
        raw_geometry_size,
        evidence,
        &SemanticAdmissibilityPolicy::strict(),
        failure_hints,
    )
}

/// Verify one inverse candidate with an explicit semantic admissibility policy.
#[must_use]
pub fn verify_strict_inverse_candidate_with_policy(
    program: Option<&ModelingProgram>,
    raw_geometry_size: RawGeometrySize,
    evidence: &StrictVerificationEvidence,
    policy: &SemanticAdmissibilityPolicy,
    failure_hints: Vec<StrictInverseFailure>,
) -> StrictInverseVerificationReport {
    let mut failures = failure_hints;
    classify_evidence(evidence, &mut failures);

    let verification = if let Some(program) = program {
        match verify_strict_semantic_program(program, policy, raw_geometry_size, evidence) {
            Ok(verification) => {
                classify_verification_report(program, &verification, &mut failures);
                Some(verification)
            }
            Err(error) => {
                failures.push(StrictInverseFailure::unsupported_serialization_order(
                    "program.serialization",
                    format!("program could not be serialized for canonical verification: {error}"),
                ));
                None
            }
        }
    } else {
        failures.push(StrictInverseFailure::search_exhaustion(
            "search.program",
            0,
            0,
        ));
        None
    };

    deduplicate_failures(&mut failures);

    let strict_success = failures.is_empty()
        && verification
            .as_ref()
            .is_some_and(|verification| verification.accepted && verification.residual_bytes == 0);

    let reconstruction_failure = if strict_success {
        None
    } else {
        Some(build_reconstruction_failure_report(
            program.cloned(),
            evidence,
            verification.clone(),
            &failures,
        ))
    };

    StrictInverseVerificationReport {
        strict_success,
        verification,
        failures,
        reconstruction_failure,
    }
}

fn classify_evidence(
    evidence: &StrictVerificationEvidence,
    failures: &mut Vec<StrictInverseFailure>,
) {
    if evidence.residual_bytes != 0 {
        failures.push(StrictInverseFailure::numerical_non_exactness(
            "evidence.residual_bytes",
            format!(
                "strict reconstruction carried {} residual byte(s)",
                evidence.residual_bytes
            ),
        ));
    }
    if evidence.literal_target_mesh_bytes != 0 {
        failures.push(StrictInverseFailure::selection_not_expressible(
            "evidence.literal_target_mesh_bytes",
            format!(
                "candidate embedded {} literal target mesh byte(s)",
                evidence.literal_target_mesh_bytes
            ),
        ));
    }
    if evidence.per_vertex_independent_position_parameters != 0 {
        failures.push(StrictInverseFailure::selection_not_expressible(
            "evidence.per_vertex_independent_position_parameters",
            format!(
                "candidate used {} independent per-vertex position parameter(s)",
                evidence.per_vertex_independent_position_parameters
            ),
        ));
    }
    if !evidence.canonical_positions_exact {
        failures.push(StrictInverseFailure::numerical_non_exactness(
            "evidence.canonical_positions_exact",
            "canonical replay positions are not exact",
        ));
    }
    if !evidence.semantic_topology_exact.is_exact() {
        failures.push(StrictInverseFailure::missing_topology_operator(
            "evidence.semantic_topology_exact",
            "semantic topology channels are not exact",
            ModelingOperationKind::ConstrainedBoolean,
        ));
    }
    if !evidence.serialization_order_exact.is_exact() {
        failures.push(StrictInverseFailure::unsupported_serialization_order(
            "evidence.serialization_order_exact",
            "canonical vertex or face order is not exact",
        ));
    }
}

/// Convert strict verification issues into inverse failure classes.
#[must_use]
pub fn classify_verification_issue(
    issue: &StrictVerificationIssue,
) -> Option<StrictInverseFailure> {
    match issue.code {
        StrictVerificationIssueCode::UnsupportedSchemaVersion
        | StrictVerificationIssueCode::UnsupportedEvaluatorVersion
        | StrictVerificationIssueCode::InvalidDependencyGraph
        | StrictVerificationIssueCode::DuplicateOperationId
        | StrictVerificationIssueCode::DuplicateSelectionId => {
            Some(StrictInverseFailure::unsupported_serialization_order(
                issue.path.clone(),
                issue.message.clone(),
            ))
        }
        StrictVerificationIssueCode::CanonicalPositionsNotExact
        | StrictVerificationIssueCode::ResidualBytesPresent
        | StrictVerificationIssueCode::CompressionRatioTooLow
        | StrictVerificationIssueCode::PerturbationValidityMissing => {
            Some(StrictInverseFailure::numerical_non_exactness(
                issue.path.clone(),
                issue.message.clone(),
            ))
        }
        StrictVerificationIssueCode::SemanticTopologyNotExact
        | StrictVerificationIssueCode::InvalidBaseTopologyContract => {
            Some(StrictInverseFailure::missing_topology_operator(
                issue.path.clone(),
                issue.message.clone(),
                ModelingOperationKind::ConstrainedBoolean,
            ))
        }
        StrictVerificationIssueCode::SerializationOrderNotExact => {
            Some(StrictInverseFailure::unsupported_serialization_order(
                issue.path.clone(),
                issue.message.clone(),
            ))
        }
        StrictVerificationIssueCode::LiteralTargetMeshBytesPresent
        | StrictVerificationIssueCode::PerVertexIndependentPositionsPresent => {
            Some(StrictInverseFailure::selection_not_expressible(
                issue.path.clone(),
                issue.message.clone(),
            ))
        }
        StrictVerificationIssueCode::OperationNotAdmissible => None,
    }
}

fn classify_verification_issues(
    issues: &[StrictVerificationIssue],
    failures: &mut Vec<StrictInverseFailure>,
) {
    failures.extend(issues.iter().filter_map(classify_verification_issue));
}

fn classify_verification_report(
    program: &ModelingProgram,
    verification: &StrictSemanticVerification,
    failures: &mut Vec<StrictInverseFailure>,
) {
    classify_verification_issues(&verification.issues, failures);
    classify_operation_admissibility(program, &verification.operation_reports, failures);
}

fn classify_operation_admissibility(
    program: &ModelingProgram,
    operation_reports: &[OperationAdmissibilityReport],
    failures: &mut Vec<StrictInverseFailure>,
) {
    let operation_kinds = program
        .operations
        .iter()
        .map(|operation| (operation.id.clone(), operation.kind))
        .collect::<BTreeMap<_, _>>();

    for report in operation_reports.iter().filter(|report| !report.admissible) {
        let kind = operation_kinds.get(&report.operation_id).copied();
        for issue in &report.issues {
            failures.push(classify_operation_admissibility_issue(
                &report.operation_id,
                kind,
                issue.code,
                &issue.message,
            ));
        }
    }
}

fn classify_operation_admissibility_issue(
    operation_id: &ProgramOperationId,
    operation_kind: Option<ModelingOperationKind>,
    issue_code: OperationAdmissibilityIssueCode,
    message: &str,
) -> StrictInverseFailure {
    let path = format!("operations.{}", operation_id.0);
    match issue_code {
        OperationAdmissibilityIssueCode::UnknownSelection
        | OperationAdmissibilityIssueCode::ExplicitSelectionPayloadTooLarge => {
            StrictInverseFailure::selection_not_expressible(path, message.to_owned())
        }
        OperationAdmissibilityIssueCode::ForbiddenPayloadKind => {
            classify_forbidden_payload(path, operation_kind, message)
        }
        OperationAdmissibilityIssueCode::ForbiddenOperationKind
        | OperationAdmissibilityIssueCode::ParameterGrowthTooHigh
        | OperationAdmissibilityIssueCode::MissingPerturbationValidity => {
            classify_operation_family_failure(path, operation_kind, message)
        }
    }
}

fn classify_forbidden_payload(
    path: String,
    operation_kind: Option<ModelingOperationKind>,
    message: &str,
) -> StrictInverseFailure {
    if message.contains("DenseDisplacement")
        || message.contains("PerVertexIndependentPositions")
        || message.contains("PerVertexCageWeights")
    {
        let missing_kind = if message.contains("PerVertexCageWeights") {
            ModelingOperationKind::PerVertexCageWeights
        } else {
            ModelingOperationKind::DenseDisplacement
        };
        StrictInverseFailure::missing_deformation_operator(path, message.to_owned(), missing_kind)
    } else if message.contains("LiteralTargetMesh") || message.contains("OpaqueResidual") {
        StrictInverseFailure::numerical_non_exactness(path, message.to_owned())
    } else {
        classify_operation_family_failure(path, operation_kind, message)
    }
}

fn classify_operation_family_failure(
    path: String,
    operation_kind: Option<ModelingOperationKind>,
    message: &str,
) -> StrictInverseFailure {
    let Some(operation_kind) = operation_kind else {
        return StrictInverseFailure::unsupported_serialization_order(
            path,
            format!("admissibility failure for unknown operation kind: {message}"),
        );
    };

    if operation_kind == ModelingOperationKind::PrimitiveCreate {
        StrictInverseFailure::missing_primitive(path, message.to_owned())
    } else if is_deformation_operation(operation_kind) {
        StrictInverseFailure::missing_deformation_operator(path, message.to_owned(), operation_kind)
    } else if is_topology_operation(operation_kind) {
        StrictInverseFailure::missing_topology_operator(path, message.to_owned(), operation_kind)
    } else {
        StrictInverseFailure::selection_not_expressible(path, message.to_owned())
    }
}

fn is_topology_operation(kind: ModelingOperationKind) -> bool {
    matches!(
        kind,
        ModelingOperationKind::RegionExtrude
            | ModelingOperationKind::RegionInset
            | ModelingOperationKind::LoopCut
            | ModelingOperationKind::BridgeLoops
            | ModelingOperationKind::Merge
            | ModelingOperationKind::Split
            | ModelingOperationKind::Dissolve
            | ModelingOperationKind::Separate
            | ModelingOperationKind::Join
            | ModelingOperationKind::Mirror
            | ModelingOperationKind::Array
            | ModelingOperationKind::Subdivide
            | ModelingOperationKind::Bevel
            | ModelingOperationKind::ShellSolidify
            | ModelingOperationKind::ConstrainedBoolean
    )
}

fn is_deformation_operation(kind: ModelingOperationKind) -> bool {
    matches!(
        kind,
        ModelingOperationKind::PartTransform
            | ModelingOperationKind::RegionTransform
            | ModelingOperationKind::Bend
            | ModelingOperationKind::Twist
            | ModelingOperationKind::Taper
            | ModelingOperationKind::Bulge
            | ModelingOperationKind::Lattice
            | ModelingOperationKind::Ffd
            | ModelingOperationKind::CageDeformation
            | ModelingOperationKind::JointChainDeformation
            | ModelingOperationKind::SmoothRelax
            | ModelingOperationKind::SurfaceSlide
            | ModelingOperationKind::ShrinkwrapProject
            | ModelingOperationKind::BoundedCorrectiveBasis
            | ModelingOperationKind::SetAllPositions
            | ModelingOperationKind::MoveVertex
            | ModelingOperationKind::DenseDisplacement
            | ModelingOperationKind::PerVertexCageWeights
    )
}

fn deduplicate_failures(failures: &mut Vec<StrictInverseFailure>) {
    failures.sort();
    failures.dedup();
}

fn build_reconstruction_failure_report(
    best_semantic_program: Option<ModelingProgram>,
    evidence: &StrictVerificationEvidence,
    verification: Option<StrictSemanticVerification>,
    failures: &[StrictInverseFailure],
) -> StrictReconstructionFailureReport {
    let mut unexplained_topology_regions = Vec::new();
    let mut unexplained_geometry = Vec::new();
    let mut missing_operator_capabilities = Vec::new();
    let mut search_limits_reached = Vec::new();

    for failure in failures {
        match failure.class {
            StrictInverseFailureClass::MissingPrimitive
            | StrictInverseFailureClass::MissingTopologyOperator
            | StrictInverseFailureClass::MissingDeformationOperator => {
                missing_operator_capabilities.push(MissingOperatorCapability {
                    operation_kind: failure.operation_kind,
                    capability: failure.class.label().to_owned(),
                    evidence: failure.detail.clone(),
                });
            }
            StrictInverseFailureClass::SelectionNotExpressible => {
                unexplained_topology_regions.push(UnexplainedTopologyRegion {
                    id: failure.path.clone(),
                    part: None,
                    boundary_loop: None,
                    reason: failure.detail.clone(),
                });
            }
            StrictInverseFailureClass::SearchExhaustion => {
                search_limits_reached.push(SearchLimitReached {
                    stage: failure.path.clone(),
                    limit: 0,
                    explored: 0,
                });
            }
            StrictInverseFailureClass::NumericalNonExactness => {
                unexplained_geometry.push(UnexplainedGeometryRegion {
                    id: failure.path.clone(),
                    affected_vertices: 0,
                    max_error: 0.0,
                    reason: failure.detail.clone(),
                });
            }
            StrictInverseFailureClass::UnsupportedSerializationOrder => {
                missing_operator_capabilities.push(MissingOperatorCapability {
                    operation_kind: None,
                    capability: "canonical serialization order".to_owned(),
                    evidence: failure.detail.clone(),
                });
            }
        }
    }

    StrictReconstructionFailureReport {
        best_semantic_program,
        unexplained_topology_regions,
        unexplained_geometry,
        missing_operator_capabilities,
        search_limits_reached,
        residual_diagnostic: residual_diagnostic(evidence),
        verification,
    }
}

fn residual_diagnostic(evidence: &StrictVerificationEvidence) -> Option<ResidualDiagnostic> {
    (evidence.residual_bytes != 0).then(|| ResidualDiagnostic {
        residual_bytes: evidence.residual_bytes,
        carrier: ResidualCarrier::OpaqueCorrectionBlob,
        message: "strict inverse verification requires exactly zero residual bytes".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use shape_program::{
        ModelingOperation, OperationPayloadDescriptor, OperationPayloadKind, SemanticTopologyExact,
        SerializationOrderExact,
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

    fn primitive_program() -> ModelingProgram {
        let mut program = ModelingProgram::strict_from_primitives();
        program.operations.push(ModelingOperation::compact(
            "op.create.box",
            ModelingOperationKind::PrimitiveCreate,
        ));
        program
    }

    #[test]
    fn strict_success_requires_zero_residual_bytes() {
        let program = primitive_program();
        let mut evidence = exact_evidence();
        evidence.residual_bytes = 12;

        let report = verify_strict_inverse_candidate(
            Some(&program),
            large_raw_geometry(),
            &evidence,
            Vec::new(),
        );

        assert!(!report.strict_success);
        assert!(!report.is_strict_success());
        assert!(report.failures.iter().any(|failure| {
            failure.class == StrictInverseFailureClass::NumericalNonExactness
                && failure.path == "evidence.residual_bytes"
        }));
        let failure_report = report
            .reconstruction_failure
            .expect("residual must produce a failure report");
        assert!(failure_report.strict_success_excluded());
        assert_eq!(
            failure_report
                .residual_diagnostic
                .expect("residual diagnostic should be preserved")
                .residual_bytes,
            12
        );
    }

    #[test]
    fn strict_acceptance_requires_verifier_acceptance_and_no_failures() {
        let program = primitive_program();
        let evidence = exact_evidence();

        let report = verify_strict_inverse_candidate(
            Some(&program),
            large_raw_geometry(),
            &evidence,
            Vec::new(),
        );

        assert!(report.strict_success);
        assert!(report.is_strict_success());
        assert!(report.reconstruction_failure.is_none());
        assert!(report.verification.expect("verification").accepted);
    }

    #[test]
    fn operation_admissibility_failures_keep_specific_inverse_classes() {
        let mut forbidden_operation = primitive_program();
        forbidden_operation.operations[0].kind = ModelingOperationKind::SetAllPositions;
        let forbidden_report = verify_strict_inverse_candidate(
            Some(&forbidden_operation),
            large_raw_geometry(),
            &exact_evidence(),
            Vec::new(),
        );
        assert!(!forbidden_report.strict_success);
        assert!(forbidden_report.failures.iter().any(|failure| {
            failure.class == StrictInverseFailureClass::MissingDeformationOperator
                && failure.operation_kind == Some(ModelingOperationKind::SetAllPositions)
        }));

        let mut dense_payload = primitive_program();
        dense_payload.operations[0]
            .payloads
            .push(OperationPayloadDescriptor {
                kind: OperationPayloadKind::DenseDisplacement,
                encoded_bytes: 128,
                semantic_parameter_count: 128,
                affected_element_count: 16,
                perturbation_valid: true,
            });
        let dense_report = verify_strict_inverse_candidate(
            Some(&dense_payload),
            large_raw_geometry(),
            &exact_evidence(),
            Vec::new(),
        );
        assert!(!dense_report.strict_success);
        assert!(dense_report.failures.iter().any(|failure| {
            failure.class == StrictInverseFailureClass::MissingDeformationOperator
                && failure.operation_kind == Some(ModelingOperationKind::DenseDisplacement)
        }));
    }

    #[test]
    fn strict_failure_classes_are_actionable() {
        let failures = vec![
            StrictInverseFailure::missing_primitive(
                "analysis.primitive",
                "target has no recognized base primitive",
            ),
            StrictInverseFailure::missing_topology_operator(
                "analysis.topology",
                "target has a branching through-cut",
                ModelingOperationKind::ConstrainedBoolean,
            ),
            StrictInverseFailure::missing_deformation_operator(
                "analysis.deformation",
                "target includes compact bend not recoverable by current grammar",
                ModelingOperationKind::Bend,
            ),
            StrictInverseFailure::selection_not_expressible(
                "selection.panel",
                "panel boundary required 300 explicit face IDs",
            ),
            StrictInverseFailure::search_exhaustion("search.beam", 128, 128),
            StrictInverseFailure::numerical_non_exactness(
                "evaluator.positions",
                "maximum canonical position error was non-zero",
            ),
            StrictInverseFailure::unsupported_serialization_order(
                "evaluator.face_order",
                "face order depended on hash iteration",
            ),
        ];

        for failure in failures {
            assert!(failure.is_actionable(), "{failure:?}");
            assert!(!failure.class.label().is_empty());
        }
    }

    #[test]
    fn strict_serialization_order_failure_is_classified() {
        let program = primitive_program();
        let mut evidence = exact_evidence();
        evidence.serialization_order_exact.face_order = false;

        let report = verify_strict_inverse_candidate(
            Some(&program),
            large_raw_geometry(),
            &evidence,
            Vec::new(),
        );

        assert!(!report.strict_success);
        assert!(report.failures.iter().any(|failure| {
            failure.class == StrictInverseFailureClass::UnsupportedSerializationOrder
        }));
    }

    #[test]
    fn strict_missing_candidate_reports_search_exhaustion() {
        let evidence = exact_evidence();

        let report =
            verify_strict_inverse_candidate(None, large_raw_geometry(), &evidence, Vec::new());

        assert!(!report.strict_success);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| { failure.class == StrictInverseFailureClass::SearchExhaustion })
        );
        assert!(
            report
                .reconstruction_failure
                .expect("missing candidate should report failure")
                .strict_success_excluded()
        );
    }
}
