//! Compact deformation-stack recovery contracts.
//!
//! This module is descriptor-level only. It assumes topology has already been
//! explained by an earlier inverse stage, then describes how compact
//! deformation evidence can be assembled into deterministic ordered stacks.

use serde::{Deserialize, Serialize};
use shape_program::deformation::{DeformationInferenceHint, deformation_operator_contract};
use shape_program::{ModelingOperationKind, OperationPayloadKind, SemanticParameter};

/// Topology result that deformation recovery is allowed to build on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyExplanationDescriptor {
    /// Stable topology explanation fingerprint.
    pub topology_fingerprint: String,
    /// Number of topology operations already explained.
    pub topology_operation_count: usize,
    /// Number of unresolved topology regions left by the topology stage.
    pub unresolved_topology_regions: usize,
}

impl TopologyExplanationDescriptor {
    /// Return true when deformation recovery may be considered for strict
    /// semantic success. Non-zero unresolved topology makes the deformation
    /// stage diagnostic-only.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.unresolved_topology_regions == 0
    }
}

/// Semantic target for recovered deformation evidence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeformationSemanticTarget {
    /// Deformation targets a named semantic part.
    Part { part_id: String },
    /// Deformation targets a named semantic region.
    Region { region_id: String },
    /// Deformation targets a semantic joint chain.
    JointChain { chain_id: String },
    /// Deformation targets a compact semantic projection surface.
    ProjectionSurface { surface_id: String },
}

impl DeformationSemanticTarget {
    fn order_key(&self) -> String {
        match self {
            Self::Part { part_id } => format!("part:{part_id}"),
            Self::Region { region_id } => format!("region:{region_id}"),
            Self::JointChain { chain_id } => format!("joint_chain:{chain_id}"),
            Self::ProjectionSurface { surface_id } => {
                format!("projection_surface:{surface_id}")
            }
        }
    }
}

/// Descriptor for evidence produced by geometry analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeformationEvidenceDescriptor {
    /// Stable evidence ID.
    pub evidence_id: String,
    /// Semantic target inferred from the topology explanation.
    pub target: DeformationSemanticTarget,
    /// Evidence family.
    pub evidence_kind: DeformationEvidenceKind,
    /// Compact deformation operator this evidence is trying to explain.
    pub proposed_operator: ModelingOperationKind,
    /// Estimated compact semantic scalar-equivalent parameter count.
    pub estimated_parameter_count: u16,
    /// Number of source elements affected by the proposed deformation.
    pub affected_element_count: usize,
    /// Payloads required by the evidence descriptor.
    pub payload_kinds: Vec<OperationPayloadKind>,
    /// Optional recovered parameters. These must remain compact and semantic.
    pub recovered_parameters: Vec<SemanticParameter>,
    /// Local fit score for this evidence item.
    pub fit: DeformationFitScore,
    /// Optional residual diagnostic. Residuals are audit data and never strict
    /// deformation recovery.
    pub residual: Option<DeformationResidualDiagnostic>,
}

impl DeformationEvidenceDescriptor {
    /// Return true when the evidence contains a payload that cannot participate
    /// in strict semantic deformation recovery.
    #[must_use]
    pub fn contains_forbidden_strict_payload(&self) -> bool {
        self.payload_kinds.iter().any(|kind| {
            matches!(
                kind,
                OperationPayloadKind::DenseDisplacement
                    | OperationPayloadKind::OpaqueResidual
                    | OperationPayloadKind::PerVertexIndependentPositions
                    | OperationPayloadKind::PerVertexCageWeights
                    | OperationPayloadKind::LiteralTargetMesh
            )
        })
    }

    fn deterministic_key(&self) -> DeformationOrderingKey {
        DeformationOrderingKey {
            topology_phase_index: 0,
            semantic_target: self.target.order_key(),
            operator_precedence: deformation_operator_precedence(self.proposed_operator),
            evidence_id: self.evidence_id.clone(),
        }
    }
}

/// Evidence category emitted by deformation analysis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeformationEvidenceKind {
    /// Part or region frame transform.
    FrameDelta,
    /// Analytic bend/twist/taper/bulge axis and falloff evidence.
    AnalyticAxisFalloff,
    /// Bounded lattice or FFD control grid evidence.
    BoundedGridControls,
    /// Sparse cage handles with procedural weights.
    SparseCageHandles,
    /// Semantic joint-chain control evidence.
    JointChainControls,
    /// Smooth/relax field evidence.
    SmoothRelaxField,
    /// Tangent surface slide evidence.
    SurfaceSlideField,
    /// Shrinkwrap projection evidence.
    ProjectionSurface,
    /// Low-rank corrective coefficients.
    LowRankCorrectiveBasis,
    /// Per-vertex movement evidence. Diagnostic only.
    DensePerVertexDisplacement,
    /// Opaque correction evidence. Diagnostic only.
    OpaqueResidual,
}

/// Fit quality for one evidence item or one recovered stack.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DeformationFitScore {
    /// Best estimate of descriptor-level fit quality, in `[0, 1]`.
    pub quality: f64,
    /// Confidence in the evidence source, in `[0, 1]`.
    pub confidence: f64,
    /// Conservative lower bound on quality, in `[0, 1]`.
    pub lower_bound: f64,
}

impl DeformationFitScore {
    /// Clamp score channels to the stable public range.
    #[must_use]
    pub fn normalized(self) -> Self {
        Self {
            quality: clamp_unit(self.quality),
            confidence: clamp_unit(self.confidence),
            lower_bound: clamp_unit(self.lower_bound),
        }
    }
}

/// Residual or unexplained deformation diagnostic. This is never a strict
/// success carrier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeformationResidualDiagnostic {
    /// Stable diagnostic ID.
    pub diagnostic_id: String,
    /// Residual kind.
    pub kind: DeformationResidualKind,
    /// Residual byte count, if a byte carrier exists for audit purposes.
    pub residual_bytes: usize,
    /// Maximum measured error after the compact stack is applied.
    pub max_error: f64,
    /// Human-readable explanation.
    pub message: String,
}

impl DeformationResidualDiagnostic {
    /// Residual diagnostics are permitted as failure evidence only and always
    /// exclude strict deformation recovery.
    #[must_use]
    pub fn excludes_strict_success(&self) -> bool {
        true
    }
}

/// Residual diagnostic kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeformationResidualKind {
    /// Dense vertex delta buffer detected.
    VertexDeltaBuffer,
    /// Texture-like displacement carrier detected.
    TextureLikeDisplacement,
    /// Opaque correction blob detected.
    OpaqueCorrectionBlob,
    /// Audit-only error heatmap.
    AuditOnlyHeatmap,
    /// Compact stack fit left measurable unexplained error.
    UnexplainedFitError,
}

/// Candidate kind produced by deformation recovery.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeformationStackCandidateKind {
    /// One compact operator.
    SingleOperator,
    /// Ordered combination of compact operators.
    OrderedStack,
    /// Lower-bound explanation that may guide search but is not strict.
    LowerBoundOnly,
    /// Diagnostic evidence only.
    DiagnosticOnly,
}

/// One compact deformation operator recovered for a stack candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveredDeformationOperator {
    /// Position in deterministic deformation order after topology explanation.
    pub order_index: usize,
    /// Stable recovered operation ID.
    pub operation_id: String,
    /// Compact operation kind.
    pub kind: ModelingOperationKind,
    /// Semantic target.
    pub target: DeformationSemanticTarget,
    /// Evidence descriptors used by this operator.
    pub evidence_ids: Vec<String>,
    /// Compact semantic parameters.
    pub parameters: Vec<SemanticParameter>,
    /// Estimated scalar-equivalent parameter count.
    pub estimated_parameter_count: u16,
    /// Whether the operator satisfies the compact shape-program contract.
    pub contract_satisfied: bool,
}

/// Deterministic ordering key for stack operators and candidates.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeformationOrderingKey {
    /// Deformation phase index. Topology is always complete before this stage.
    pub topology_phase_index: usize,
    /// Stable semantic target key.
    pub semantic_target: String,
    /// Operator precedence derived from shape-program deformation contracts.
    pub operator_precedence: usize,
    /// Stable source evidence ID.
    pub evidence_id: String,
}

/// Ordered deformation-stack candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeformationStackCandidate {
    /// Stable candidate ID.
    pub candidate_id: String,
    /// Candidate kind.
    pub kind: DeformationStackCandidateKind,
    /// Topology explanation this stack depends on.
    pub topology: TopologyExplanationDescriptor,
    /// Operators in deterministic application order.
    pub operators: Vec<RecoveredDeformationOperator>,
    /// Candidate fit summary.
    pub fit: DeformationFitScore,
    /// Conservative lower bound for downstream ranking.
    pub lower_bound_score: f64,
    /// Residual diagnostics retained for failure reporting only.
    pub residual_diagnostics: Vec<DeformationResidualDiagnostic>,
    /// Rejected input evidence that prevents this candidate from claiming a
    /// complete strict deformation explanation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub strict_blocking_rejections: Vec<RejectedDeformationEvidence>,
    /// Deterministic ordering key for this candidate.
    pub ordering_key: DeformationOrderingKey,
}

impl DeformationStackCandidate {
    /// Return true only for a complete, compact, residual-free deformation
    /// explanation. This does not prove whole-program strict success; it only
    /// proves the deformation recovery stage did not rely on residuals.
    #[must_use]
    pub fn is_strict_deformation_recovery(&self) -> bool {
        self.topology.is_complete()
            && !self.operators.is_empty()
            && self.residual_diagnostics.is_empty()
            && self.strict_blocking_rejections.is_empty()
            && !matches!(self.kind, DeformationStackCandidateKind::DiagnosticOnly)
            && self
                .operators
                .iter()
                .all(|operator| operator.contract_satisfied)
    }
}

/// Full descriptor-level recovery report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeformationRecoveryReport {
    /// Topology explanation supplied to deformation recovery.
    pub topology: TopologyExplanationDescriptor,
    /// Candidate stacks in deterministic order.
    pub candidates: Vec<DeformationStackCandidate>,
    /// Evidence rejected from strict recovery and retained for diagnostics.
    pub rejected_evidence: Vec<RejectedDeformationEvidence>,
}

/// Rejected deformation evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedDeformationEvidence {
    /// Evidence ID.
    pub evidence_id: String,
    /// Rejection reason.
    pub reason: DeformationEvidenceRejection,
}

/// Stable rejection reason.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeformationEvidenceRejection {
    /// No shape-program compact deformation contract exists for the proposed
    /// operator.
    UnsupportedOperator,
    /// Semantic target does not match the operator contract hints.
    TargetDoesNotMatchContract,
    /// Parameter count is outside the compact contract.
    ParameterCountOutsideContract,
    /// Evidence requires a forbidden dense or opaque payload.
    ForbiddenStrictPayload,
    /// Evidence contains a residual diagnostic.
    ResidualDiagnosticOnly,
    /// Evidence kind itself is dense or opaque.
    DenseOrOpaqueEvidenceKind,
}

/// Recover deterministic ordered deformation-stack candidates from descriptors.
#[must_use]
pub fn recover_deformation_stack_candidates(
    topology: TopologyExplanationDescriptor,
    mut evidence: Vec<DeformationEvidenceDescriptor>,
) -> DeformationRecoveryReport {
    evidence.sort_by_key(DeformationEvidenceDescriptor::deterministic_key);

    let mut operators = Vec::new();
    let mut residuals = Vec::new();
    let mut rejected = Vec::new();

    for descriptor in evidence {
        let evidence_id = descriptor.evidence_id.clone();
        if let Some(residual) = descriptor.residual.clone() {
            residuals.push(residual);
            rejected.push(RejectedDeformationEvidence {
                evidence_id,
                reason: DeformationEvidenceRejection::ResidualDiagnosticOnly,
            });
            continue;
        }

        if matches!(
            descriptor.evidence_kind,
            DeformationEvidenceKind::DensePerVertexDisplacement
                | DeformationEvidenceKind::OpaqueResidual
        ) {
            rejected.push(RejectedDeformationEvidence {
                evidence_id,
                reason: DeformationEvidenceRejection::DenseOrOpaqueEvidenceKind,
            });
            continue;
        }

        if descriptor.contains_forbidden_strict_payload() {
            rejected.push(RejectedDeformationEvidence {
                evidence_id,
                reason: DeformationEvidenceRejection::ForbiddenStrictPayload,
            });
            continue;
        }

        let Some(contract) = deformation_operator_contract(descriptor.proposed_operator) else {
            rejected.push(RejectedDeformationEvidence {
                evidence_id,
                reason: DeformationEvidenceRejection::UnsupportedOperator,
            });
            continue;
        };

        if !target_matches_contract(&descriptor.target, &contract.inference_hints) {
            rejected.push(RejectedDeformationEvidence {
                evidence_id,
                reason: DeformationEvidenceRejection::TargetDoesNotMatchContract,
            });
            continue;
        }

        if !parameter_count_matches_contract(
            descriptor.estimated_parameter_count,
            contract.semantic_parameter_count.minimum,
            contract.semantic_parameter_count.maximum,
        ) {
            rejected.push(RejectedDeformationEvidence {
                evidence_id,
                reason: DeformationEvidenceRejection::ParameterCountOutsideContract,
            });
            continue;
        }

        let operation_index = operators.len();
        operators.push(RecoveredDeformationOperator {
            order_index: operation_index,
            operation_id: format!("recovered.deformation.{operation_index:04}"),
            kind: descriptor.proposed_operator,
            target: descriptor.target,
            evidence_ids: vec![descriptor.evidence_id],
            parameters: descriptor.recovered_parameters,
            estimated_parameter_count: descriptor.estimated_parameter_count,
            contract_satisfied: true,
        });
    }

    let candidates = build_candidates(topology.clone(), operators, residuals, rejected.clone());

    DeformationRecoveryReport {
        topology,
        candidates,
        rejected_evidence: rejected,
    }
}

fn build_candidates(
    topology: TopologyExplanationDescriptor,
    operators: Vec<RecoveredDeformationOperator>,
    residuals: Vec<DeformationResidualDiagnostic>,
    strict_blocking_rejections: Vec<RejectedDeformationEvidence>,
) -> Vec<DeformationStackCandidate> {
    if operators.is_empty() {
        return residuals
            .first()
            .map(|residual| {
                vec![DeformationStackCandidate {
                    candidate_id: "deformation.candidate.diagnostic_only".to_owned(),
                    kind: DeformationStackCandidateKind::DiagnosticOnly,
                    topology,
                    operators: Vec::new(),
                    fit: DeformationFitScore {
                        quality: 0.0,
                        confidence: 0.0,
                        lower_bound: 0.0,
                    },
                    lower_bound_score: 0.0,
                    residual_diagnostics: vec![residual.clone()],
                    strict_blocking_rejections,
                    ordering_key: DeformationOrderingKey {
                        topology_phase_index: 0,
                        semantic_target: "diagnostic".to_owned(),
                        operator_precedence: usize::MAX,
                        evidence_id: residual.diagnostic_id.clone(),
                    },
                }]
            })
            .unwrap_or_default();
    }

    let lower_bound_score = 1.0 / (operators.len() as f64 + 1.0);
    let kind = if residuals.is_empty() {
        match operators.len() {
            1 => DeformationStackCandidateKind::SingleOperator,
            _ => DeformationStackCandidateKind::OrderedStack,
        }
    } else {
        DeformationStackCandidateKind::LowerBoundOnly
    };

    let first_operator = operators
        .first()
        .expect("operators is non-empty after early return");
    let ordering_key = DeformationOrderingKey {
        topology_phase_index: 0,
        semantic_target: first_operator.target.order_key(),
        operator_precedence: deformation_operator_precedence(first_operator.kind),
        evidence_id: first_operator
            .evidence_ids
            .first()
            .cloned()
            .unwrap_or_default(),
    };

    vec![DeformationStackCandidate {
        candidate_id: "deformation.candidate.0000".to_owned(),
        kind,
        topology,
        operators,
        fit: DeformationFitScore {
            quality: 0.75,
            confidence: 0.70,
            lower_bound: lower_bound_score,
        }
        .normalized(),
        lower_bound_score,
        residual_diagnostics: residuals,
        strict_blocking_rejections,
        ordering_key,
    }]
}

fn target_matches_contract(
    target: &DeformationSemanticTarget,
    hints: &[DeformationInferenceHint],
) -> bool {
    match target {
        DeformationSemanticTarget::Part { .. } => hints
            .iter()
            .any(|hint| matches!(hint, DeformationInferenceHint::SemanticPartSelection)),
        DeformationSemanticTarget::Region { .. } => hints
            .iter()
            .any(|hint| matches!(hint, DeformationInferenceHint::SemanticRegionSelection)),
        DeformationSemanticTarget::JointChain { .. } => hints
            .iter()
            .any(|hint| matches!(hint, DeformationInferenceHint::JointChainControls)),
        DeformationSemanticTarget::ProjectionSurface { .. } => hints
            .iter()
            .any(|hint| matches!(hint, DeformationInferenceHint::ProjectionTarget)),
    }
}

fn parameter_count_matches_contract(count: u16, minimum: u16, maximum: Option<u16>) -> bool {
    count >= minimum && maximum.is_none_or(|maximum| count <= maximum)
}

fn deformation_operator_precedence(kind: ModelingOperationKind) -> usize {
    match kind {
        ModelingOperationKind::PartTransform => 0,
        ModelingOperationKind::RegionTransform => 1,
        ModelingOperationKind::Bend => 2,
        ModelingOperationKind::Twist => 3,
        ModelingOperationKind::Taper => 4,
        ModelingOperationKind::Bulge => 5,
        ModelingOperationKind::Lattice => 6,
        ModelingOperationKind::Ffd => 7,
        ModelingOperationKind::CageDeformation => 8,
        ModelingOperationKind::JointChainDeformation => 9,
        ModelingOperationKind::SmoothRelax => 10,
        ModelingOperationKind::SurfaceSlide => 11,
        ModelingOperationKind::ShrinkwrapProject => 12,
        ModelingOperationKind::BoundedCorrectiveBasis => 13,
        _ => usize::MAX,
    }
}

fn clamp_unit(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deformation_recovery_orders_compact_stack_deterministically() {
        let topology = complete_topology();
        let report = recover_deformation_stack_candidates(
            topology,
            vec![
                analytic_evidence(
                    "evidence.twist",
                    ModelingOperationKind::Twist,
                    8,
                    DeformationFitScore {
                        quality: 0.8,
                        confidence: 0.7,
                        lower_bound: 0.6,
                    },
                ),
                analytic_evidence(
                    "evidence.bend",
                    ModelingOperationKind::Bend,
                    8,
                    DeformationFitScore {
                        quality: 0.9,
                        confidence: 0.8,
                        lower_bound: 0.7,
                    },
                ),
            ],
        );

        assert!(report.rejected_evidence.is_empty());
        let candidate = report.candidates.first().expect("candidate should exist");
        assert_eq!(candidate.kind, DeformationStackCandidateKind::OrderedStack);
        assert!(candidate.is_strict_deformation_recovery());
        assert_eq!(
            candidate
                .operators
                .iter()
                .map(|operator| operator.kind)
                .collect::<Vec<_>>(),
            vec![ModelingOperationKind::Bend, ModelingOperationKind::Twist]
        );
        assert_eq!(candidate.operators[0].order_index, 0);
        assert!(candidate.lower_bound_score > 0.0);
    }

    #[test]
    fn deformation_recovery_rejects_dense_per_vertex_displacement_as_strict() {
        let topology = complete_topology();
        let mut evidence = analytic_evidence(
            "evidence.dense",
            ModelingOperationKind::Bend,
            8,
            DeformationFitScore {
                quality: 1.0,
                confidence: 1.0,
                lower_bound: 1.0,
            },
        );
        evidence.evidence_kind = DeformationEvidenceKind::DensePerVertexDisplacement;
        evidence
            .payload_kinds
            .push(OperationPayloadKind::DenseDisplacement);

        let report = recover_deformation_stack_candidates(topology, vec![evidence]);

        assert!(report.candidates.is_empty());
        assert_eq!(
            report.rejected_evidence[0].reason,
            DeformationEvidenceRejection::DenseOrOpaqueEvidenceKind
        );
    }

    #[test]
    fn compact_candidate_is_not_strict_when_same_input_has_dense_evidence() {
        let topology = complete_topology();
        let compact = analytic_evidence(
            "evidence.compact",
            ModelingOperationKind::Bend,
            8,
            DeformationFitScore {
                quality: 0.9,
                confidence: 0.8,
                lower_bound: 0.7,
            },
        );
        let mut dense = analytic_evidence(
            "evidence.dense",
            ModelingOperationKind::Bend,
            8,
            DeformationFitScore {
                quality: 1.0,
                confidence: 1.0,
                lower_bound: 1.0,
            },
        );
        dense.evidence_kind = DeformationEvidenceKind::DensePerVertexDisplacement;

        let report = recover_deformation_stack_candidates(topology, vec![compact, dense]);

        let candidate = report.candidates.first().expect("compact candidate");
        assert!(!candidate.is_strict_deformation_recovery());
        assert_eq!(candidate.strict_blocking_rejections.len(), 1);
        assert_eq!(
            candidate.strict_blocking_rejections[0].reason,
            DeformationEvidenceRejection::DenseOrOpaqueEvidenceKind
        );
    }

    #[test]
    fn deformation_recovery_rejects_opaque_residual_as_strict() {
        let topology = complete_topology();
        let mut evidence = analytic_evidence(
            "evidence.residual",
            ModelingOperationKind::Bend,
            8,
            DeformationFitScore {
                quality: 0.9,
                confidence: 0.8,
                lower_bound: 0.5,
            },
        );
        evidence.residual = Some(DeformationResidualDiagnostic {
            diagnostic_id: "residual.opaque".to_owned(),
            kind: DeformationResidualKind::OpaqueCorrectionBlob,
            residual_bytes: 1024,
            max_error: 0.01,
            message: "opaque correction cannot be a semantic deformation".to_owned(),
        });

        let report = recover_deformation_stack_candidates(topology, vec![evidence]);
        let candidate = report.candidates.first().expect("diagnostic candidate");

        assert_eq!(
            candidate.kind,
            DeformationStackCandidateKind::DiagnosticOnly
        );
        assert!(!candidate.is_strict_deformation_recovery());
        assert!(candidate.residual_diagnostics[0].excludes_strict_success());
        assert_eq!(
            report.rejected_evidence[0].reason,
            DeformationEvidenceRejection::ResidualDiagnosticOnly
        );
    }

    #[test]
    fn deformation_recovery_enforces_contract_targets_and_parameter_bounds() {
        let topology = complete_topology();
        let mut part_target_for_bend = analytic_evidence(
            "evidence.bad_target",
            ModelingOperationKind::Bend,
            8,
            DeformationFitScore {
                quality: 0.9,
                confidence: 0.8,
                lower_bound: 0.7,
            },
        );
        part_target_for_bend.target = DeformationSemanticTarget::Part {
            part_id: "part.arm".to_owned(),
        };

        let too_many_parameters = analytic_evidence(
            "evidence.too_many_parameters",
            ModelingOperationKind::Bend,
            80,
            DeformationFitScore {
                quality: 0.9,
                confidence: 0.8,
                lower_bound: 0.7,
            },
        );

        let report = recover_deformation_stack_candidates(
            topology,
            vec![part_target_for_bend, too_many_parameters],
        );

        assert!(report.candidates.is_empty());
        assert_eq!(report.rejected_evidence.len(), 2);
        assert!(report.rejected_evidence.iter().any(|rejection| {
            rejection.reason == DeformationEvidenceRejection::TargetDoesNotMatchContract
        }));
        assert!(report.rejected_evidence.iter().any(|rejection| {
            rejection.reason == DeformationEvidenceRejection::ParameterCountOutsideContract
        }));
    }

    #[test]
    fn deformation_recovery_is_lower_bound_when_topology_is_incomplete() {
        let topology = TopologyExplanationDescriptor {
            topology_fingerprint: "topology.partial".to_owned(),
            topology_operation_count: 4,
            unresolved_topology_regions: 1,
        };

        let report = recover_deformation_stack_candidates(
            topology,
            vec![analytic_evidence(
                "evidence.bend",
                ModelingOperationKind::Bend,
                8,
                DeformationFitScore {
                    quality: 0.9,
                    confidence: 0.8,
                    lower_bound: 0.7,
                },
            )],
        );

        let candidate = report.candidates.first().expect("candidate should exist");
        assert!(!candidate.is_strict_deformation_recovery());
        assert!(candidate.lower_bound_score > 0.0);
    }

    fn complete_topology() -> TopologyExplanationDescriptor {
        TopologyExplanationDescriptor {
            topology_fingerprint: "topology.complete".to_owned(),
            topology_operation_count: 8,
            unresolved_topology_regions: 0,
        }
    }

    fn analytic_evidence(
        evidence_id: &str,
        proposed_operator: ModelingOperationKind,
        estimated_parameter_count: u16,
        fit: DeformationFitScore,
    ) -> DeformationEvidenceDescriptor {
        DeformationEvidenceDescriptor {
            evidence_id: evidence_id.to_owned(),
            target: DeformationSemanticTarget::Region {
                region_id: "region.arm".to_owned(),
            },
            evidence_kind: DeformationEvidenceKind::AnalyticAxisFalloff,
            proposed_operator,
            estimated_parameter_count,
            affected_element_count: 128,
            payload_kinds: vec![OperationPayloadKind::SemanticParameters],
            recovered_parameters: vec![
                SemanticParameter::Vector3 {
                    name: "axis".to_owned(),
                    value: [0.0, 1.0, 0.0],
                },
                SemanticParameter::Scalar {
                    name: "amount".to_owned(),
                    value: 0.2,
                },
                SemanticParameter::Scalar {
                    name: "falloff".to_owned(),
                    value: 1.0,
                },
            ],
            fit,
            residual: None,
        }
    }
}
