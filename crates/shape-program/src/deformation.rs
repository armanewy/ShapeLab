//! Compact deformation-operator contracts.
//!
//! These contracts describe what a strict semantic program may encode for
//! deformation operations. They intentionally do not evaluate geometry.

use crate::{
    ModelingOperationKind, OperationPayloadKind, SemanticAdmissibilityPolicy, SemanticParameter,
};

/// Number of deformation operator kinds covered by this module.
pub const DEFORMATION_OPERATOR_COUNT: usize = 14;

/// Contract metadata for one compact deformation operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeformationOperatorContract {
    /// Operation kind covered by this contract.
    pub kind: ModelingOperationKind,
    /// Semantic parameter-count expectation.
    pub semantic_parameter_count: SemanticParameterCount,
    /// Compact control structure used instead of arbitrary per-vertex edits.
    pub compact_control_structure: CompactControlStructure,
    /// Perturbation validity expected by strict inverse search.
    pub perturbation_validity: PerturbationValidityExpectation,
    /// Hints an inference system may use when proposing this operation.
    pub inference_hints: Vec<DeformationInferenceHint>,
    /// Payload categories this operator is allowed to carry in strict mode.
    pub allowed_payload_kinds: Vec<OperationPayloadKind>,
}

impl DeformationOperatorContract {
    /// Return true when the contract is admissible under the supplied strict policy.
    #[must_use]
    pub fn is_admissible_under(&self, policy: &SemanticAdmissibilityPolicy) -> bool {
        !policy.forbidden_operation_kinds.contains(&self.kind)
            && self.uses_compact_parameters()
            && !self.declares_dense_per_vertex_payload()
            && (!policy.perturbation_validity_required
                || self
                    .perturbation_validity
                    .valid_under_small_parameter_perturbation)
    }

    /// Return true when semantic parameters are bounded by compact controls.
    #[must_use]
    pub fn uses_compact_parameters(&self) -> bool {
        !matches!(
            self.semantic_parameter_count.growth,
            SemanticParameterGrowth::PerAffectedElement
        )
    }

    /// Return true if any allowed payload kind would smuggle dense target geometry.
    #[must_use]
    pub fn declares_dense_per_vertex_payload(&self) -> bool {
        self.allowed_payload_kinds.iter().any(|kind| {
            matches!(
                kind,
                OperationPayloadKind::DenseDisplacement
                    | OperationPayloadKind::PerVertexIndependentPositions
                    | OperationPayloadKind::PerVertexCageWeights
                    | OperationPayloadKind::LiteralTargetMesh
                    | OperationPayloadKind::OpaqueResidual
            )
        }) || self
            .compact_control_structure
            .declares_per_affected_vertex_displacement()
    }

    /// Build a compact modeling operation stub for this deformation contract.
    #[must_use]
    pub fn compact_operation(&self, id: impl Into<String>) -> crate::ModelingOperation {
        let mut operation = crate::ModelingOperation::compact(id, self.kind);
        operation.parameters = self.example_semantic_parameters();
        operation
    }

    fn example_semantic_parameters(&self) -> Vec<SemanticParameter> {
        match self.kind {
            ModelingOperationKind::PartTransform | ModelingOperationKind::RegionTransform => {
                vec![
                    SemanticParameter::Vector3 {
                        name: "translation".to_owned(),
                        value: [0.0, 0.0, 0.0],
                    },
                    SemanticParameter::Quaternion {
                        name: "rotation".to_owned(),
                        value: [0.0, 0.0, 0.0, 1.0],
                    },
                    SemanticParameter::Vector3 {
                        name: "scale".to_owned(),
                        value: [1.0, 1.0, 1.0],
                    },
                ]
            }
            ModelingOperationKind::Bend
            | ModelingOperationKind::Twist
            | ModelingOperationKind::Taper
            | ModelingOperationKind::Bulge => vec![
                SemanticParameter::Vector3 {
                    name: "axis".to_owned(),
                    value: [0.0, 1.0, 0.0],
                },
                SemanticParameter::Scalar {
                    name: "amount".to_owned(),
                    value: 0.0,
                },
                SemanticParameter::Scalar {
                    name: "falloff".to_owned(),
                    value: 1.0,
                },
            ],
            ModelingOperationKind::SmoothRelax => vec![
                SemanticParameter::Integer {
                    name: "iterations".to_owned(),
                    value: 1,
                },
                SemanticParameter::Scalar {
                    name: "strength".to_owned(),
                    value: 0.25,
                },
            ],
            ModelingOperationKind::SurfaceSlide => vec![
                SemanticParameter::Scalar {
                    name: "distance".to_owned(),
                    value: 0.0,
                },
                SemanticParameter::Choice {
                    name: "direction".to_owned(),
                    value: "geodesic".to_owned(),
                },
            ],
            ModelingOperationKind::ShrinkwrapProject => vec![
                SemanticParameter::Scalar {
                    name: "offset".to_owned(),
                    value: 0.0,
                },
                SemanticParameter::Choice {
                    name: "projection".to_owned(),
                    value: "normal".to_owned(),
                },
            ],
            ModelingOperationKind::Lattice
            | ModelingOperationKind::Ffd
            | ModelingOperationKind::CageDeformation
            | ModelingOperationKind::JointChainDeformation
            | ModelingOperationKind::BoundedCorrectiveBasis => Vec::new(),
            _ => Vec::new(),
        }
    }
}

/// Semantic parameter-count contract for a deformation operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticParameterCount {
    /// Minimum number of semantic scalar-equivalent parameters.
    pub minimum: u16,
    /// Typical number of semantic scalar-equivalent parameters.
    pub typical: u16,
    /// Maximum number when the compact structure is bounded by this contract.
    pub maximum: Option<u16>,
    /// How parameter count may grow.
    pub growth: SemanticParameterGrowth,
}

/// Growth source for semantic parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticParameterGrowth {
    /// Fixed semantic parameter count.
    Constant,
    /// Parameter count grows only with bounded compact handles/control points.
    BoundedByCompactControlStructure,
    /// Parameter count grows only with semantic chain or segment count.
    BoundedBySemanticTopology,
    /// Parameter count grows only with declared low-rank basis size.
    BoundedByBasisRank,
    /// Non-admissible sentinel: one parameter tuple per affected element.
    PerAffectedElement,
}

/// Compact control structure used by a deformation operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactControlStructure {
    /// Transform of a semantic part frame.
    PartFrameTransform,
    /// Transform of a semantic region frame.
    RegionFrameTransform,
    /// Analytic axis plus a compact falloff profile.
    AnalyticAxisFalloff {
        /// Deformation family.
        family: AnalyticDeformationFamily,
    },
    /// Regular lattice with bounded control points.
    LatticeGrid {
        /// Maximum number of lattice control points in strict mode.
        max_control_points: u16,
    },
    /// Free-form deformation volume with bounded control points.
    FreeFormDeformationVolume {
        /// Maximum number of FFD control points in strict mode.
        max_control_points: u16,
    },
    /// Sparse cage handles, with procedural or semantic weights.
    CageHandles {
        /// Maximum number of semantic cage handles in strict mode.
        max_handles: u16,
        /// Weight representation contract.
        weight_contract: CageWeightContract,
    },
    /// Semantic joint chain controls.
    JointChain {
        /// Maximum number of joints encoded directly in strict mode.
        max_joints: u16,
    },
    /// Local smoothing or relaxation field.
    SmoothRelaxField,
    /// Compact tangent-direction surface slide.
    SurfaceSlideField,
    /// Projection to semantic surface target.
    ShrinkwrapProjection,
    /// Low-rank corrective basis referenced by semantic source and coefficients.
    BoundedCorrectiveBasis {
        /// Maximum number of active basis terms in strict mode.
        max_basis_terms: u16,
    },
}

impl CompactControlStructure {
    /// Return true if this structure directly declares one displacement per affected vertex.
    #[must_use]
    pub fn declares_per_affected_vertex_displacement(self) -> bool {
        false
    }
}

/// Analytic deformation families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyticDeformationFamily {
    /// Bend around an axis.
    Bend,
    /// Twist around an axis.
    Twist,
    /// Taper along an axis.
    Taper,
    /// Bulge from an axis or centerline.
    Bulge,
}

/// Cage weight representation allowed in strict mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CageWeightContract {
    /// Weights are derived procedurally from the cage and target surface.
    Procedural,
    /// Weights are referenced from a versioned semantic basis, not encoded per vertex.
    VersionedSemanticBasis,
}

/// Expected validity of nearby semantic perturbations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerturbationValidityExpectation {
    /// Small changes to semantic parameters should remain valid.
    pub valid_under_small_parameter_perturbation: bool,
    /// Operation is expected not to change topology.
    pub topology_preserving: bool,
    /// Evaluation may need reprojection against a compact semantic target.
    pub may_require_reprojection: bool,
}

/// Inference hints for proposing compact deformation operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeformationInferenceHint {
    /// Prefer semantic part selections.
    SemanticPartSelection,
    /// Prefer semantic region selections.
    SemanticRegionSelection,
    /// Estimate a stable local frame.
    LocalFrame,
    /// Estimate a principal or landmark axis.
    PrincipalAxis,
    /// Fit a compact falloff profile.
    FalloffProfile,
    /// Infer bounded regular-grid controls.
    BoundedGridControls,
    /// Infer sparse cage handles.
    SparseCageHandles,
    /// Infer semantic joint-chain controls.
    JointChainControls,
    /// Infer tangent flow on the source surface.
    TangentSurfaceFlow,
    /// Infer projection target and offset.
    ProjectionTarget,
    /// Infer low-rank corrective coefficients.
    LowRankCorrectiveCoefficients,
}

/// Return all compact deformation operator contracts.
#[must_use]
pub fn all_deformation_operator_contracts() -> Vec<DeformationOperatorContract> {
    vec![
        contract(
            ModelingOperationKind::PartTransform,
            SemanticParameterCount {
                minimum: 10,
                typical: 10,
                maximum: Some(10),
                growth: SemanticParameterGrowth::Constant,
            },
            CompactControlStructure::PartFrameTransform,
            perturbation(false),
            vec![
                DeformationInferenceHint::SemanticPartSelection,
                DeformationInferenceHint::LocalFrame,
            ],
            vec![OperationPayloadKind::SemanticParameters],
        ),
        contract(
            ModelingOperationKind::RegionTransform,
            SemanticParameterCount {
                minimum: 10,
                typical: 10,
                maximum: Some(10),
                growth: SemanticParameterGrowth::Constant,
            },
            CompactControlStructure::RegionFrameTransform,
            perturbation(false),
            vec![
                DeformationInferenceHint::SemanticRegionSelection,
                DeformationInferenceHint::LocalFrame,
                DeformationInferenceHint::FalloffProfile,
            ],
            vec![
                OperationPayloadKind::SemanticParameters,
                OperationPayloadKind::CompactSelection,
            ],
        ),
        analytic_contract(
            ModelingOperationKind::Bend,
            AnalyticDeformationFamily::Bend,
            DeformationInferenceHint::PrincipalAxis,
        ),
        analytic_contract(
            ModelingOperationKind::Twist,
            AnalyticDeformationFamily::Twist,
            DeformationInferenceHint::PrincipalAxis,
        ),
        analytic_contract(
            ModelingOperationKind::Taper,
            AnalyticDeformationFamily::Taper,
            DeformationInferenceHint::PrincipalAxis,
        ),
        analytic_contract(
            ModelingOperationKind::Bulge,
            AnalyticDeformationFamily::Bulge,
            DeformationInferenceHint::FalloffProfile,
        ),
        contract(
            ModelingOperationKind::Lattice,
            SemanticParameterCount {
                minimum: 12,
                typical: 36,
                maximum: Some(81),
                growth: SemanticParameterGrowth::BoundedByCompactControlStructure,
            },
            CompactControlStructure::LatticeGrid {
                max_control_points: 27,
            },
            perturbation(false),
            vec![
                DeformationInferenceHint::SemanticRegionSelection,
                DeformationInferenceHint::BoundedGridControls,
                DeformationInferenceHint::FalloffProfile,
            ],
            vec![
                OperationPayloadKind::SemanticParameters,
                OperationPayloadKind::CompactSelection,
            ],
        ),
        contract(
            ModelingOperationKind::Ffd,
            SemanticParameterCount {
                minimum: 12,
                typical: 48,
                maximum: Some(192),
                growth: SemanticParameterGrowth::BoundedByCompactControlStructure,
            },
            CompactControlStructure::FreeFormDeformationVolume {
                max_control_points: 64,
            },
            perturbation(false),
            vec![
                DeformationInferenceHint::SemanticRegionSelection,
                DeformationInferenceHint::BoundedGridControls,
            ],
            vec![
                OperationPayloadKind::SemanticParameters,
                OperationPayloadKind::CompactSelection,
            ],
        ),
        contract(
            ModelingOperationKind::CageDeformation,
            SemanticParameterCount {
                minimum: 12,
                typical: 48,
                maximum: Some(96),
                growth: SemanticParameterGrowth::BoundedByCompactControlStructure,
            },
            CompactControlStructure::CageHandles {
                max_handles: 32,
                weight_contract: CageWeightContract::Procedural,
            },
            perturbation(false),
            vec![
                DeformationInferenceHint::SemanticRegionSelection,
                DeformationInferenceHint::SparseCageHandles,
            ],
            vec![
                OperationPayloadKind::SemanticParameters,
                OperationPayloadKind::CompactSelection,
            ],
        ),
        contract(
            ModelingOperationKind::JointChainDeformation,
            SemanticParameterCount {
                minimum: 6,
                typical: 24,
                maximum: Some(96),
                growth: SemanticParameterGrowth::BoundedBySemanticTopology,
            },
            CompactControlStructure::JointChain { max_joints: 16 },
            perturbation(false),
            vec![
                DeformationInferenceHint::SemanticPartSelection,
                DeformationInferenceHint::JointChainControls,
                DeformationInferenceHint::LocalFrame,
            ],
            vec![
                OperationPayloadKind::SemanticParameters,
                OperationPayloadKind::CompactSelection,
            ],
        ),
        contract(
            ModelingOperationKind::SmoothRelax,
            SemanticParameterCount {
                minimum: 2,
                typical: 4,
                maximum: Some(8),
                growth: SemanticParameterGrowth::Constant,
            },
            CompactControlStructure::SmoothRelaxField,
            perturbation(false),
            vec![
                DeformationInferenceHint::SemanticRegionSelection,
                DeformationInferenceHint::FalloffProfile,
            ],
            vec![
                OperationPayloadKind::SemanticParameters,
                OperationPayloadKind::CompactSelection,
            ],
        ),
        contract(
            ModelingOperationKind::SurfaceSlide,
            SemanticParameterCount {
                minimum: 2,
                typical: 5,
                maximum: Some(12),
                growth: SemanticParameterGrowth::Constant,
            },
            CompactControlStructure::SurfaceSlideField,
            perturbation(true),
            vec![
                DeformationInferenceHint::SemanticRegionSelection,
                DeformationInferenceHint::TangentSurfaceFlow,
                DeformationInferenceHint::FalloffProfile,
            ],
            vec![
                OperationPayloadKind::SemanticParameters,
                OperationPayloadKind::CompactSelection,
            ],
        ),
        contract(
            ModelingOperationKind::ShrinkwrapProject,
            SemanticParameterCount {
                minimum: 3,
                typical: 6,
                maximum: Some(12),
                growth: SemanticParameterGrowth::Constant,
            },
            CompactControlStructure::ShrinkwrapProjection,
            perturbation(true),
            vec![
                DeformationInferenceHint::SemanticRegionSelection,
                DeformationInferenceHint::ProjectionTarget,
            ],
            vec![
                OperationPayloadKind::SemanticParameters,
                OperationPayloadKind::CompactSelection,
            ],
        ),
        contract(
            ModelingOperationKind::BoundedCorrectiveBasis,
            SemanticParameterCount {
                minimum: 1,
                typical: 8,
                maximum: Some(16),
                growth: SemanticParameterGrowth::BoundedByBasisRank,
            },
            CompactControlStructure::BoundedCorrectiveBasis {
                max_basis_terms: 16,
            },
            perturbation(false),
            vec![
                DeformationInferenceHint::SemanticRegionSelection,
                DeformationInferenceHint::LowRankCorrectiveCoefficients,
            ],
            vec![
                OperationPayloadKind::SemanticParameters,
                OperationPayloadKind::CompactSelection,
                OperationPayloadKind::ProceduralSeed,
            ],
        ),
    ]
}

/// Return the contract for a deformation operator kind.
#[must_use]
pub fn deformation_operator_contract(
    kind: ModelingOperationKind,
) -> Option<DeformationOperatorContract> {
    all_deformation_operator_contracts()
        .into_iter()
        .find(|contract| contract.kind == kind)
}

/// Return true when the kind is a compact deformation operator.
#[must_use]
pub fn is_deformation_operator_kind(kind: ModelingOperationKind) -> bool {
    deformation_operator_contract(kind).is_some()
}

fn analytic_contract(
    kind: ModelingOperationKind,
    family: AnalyticDeformationFamily,
    primary_hint: DeformationInferenceHint,
) -> DeformationOperatorContract {
    contract(
        kind,
        SemanticParameterCount {
            minimum: 5,
            typical: 8,
            maximum: Some(12),
            growth: SemanticParameterGrowth::Constant,
        },
        CompactControlStructure::AnalyticAxisFalloff { family },
        perturbation(false),
        vec![
            DeformationInferenceHint::SemanticRegionSelection,
            primary_hint,
            DeformationInferenceHint::FalloffProfile,
        ],
        vec![
            OperationPayloadKind::SemanticParameters,
            OperationPayloadKind::CompactSelection,
        ],
    )
}

fn contract(
    kind: ModelingOperationKind,
    semantic_parameter_count: SemanticParameterCount,
    compact_control_structure: CompactControlStructure,
    perturbation_validity: PerturbationValidityExpectation,
    inference_hints: Vec<DeformationInferenceHint>,
    allowed_payload_kinds: Vec<OperationPayloadKind>,
) -> DeformationOperatorContract {
    DeformationOperatorContract {
        kind,
        semantic_parameter_count,
        compact_control_structure,
        perturbation_validity,
        inference_hints,
        allowed_payload_kinds,
    }
}

fn perturbation(may_require_reprojection: bool) -> PerturbationValidityExpectation {
    PerturbationValidityExpectation {
        valid_under_small_parameter_perturbation: true,
        topology_preserving: true,
        may_require_reprojection,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn every_compact_deformation_operator_has_a_contract() {
        let actual = all_deformation_operator_contracts()
            .into_iter()
            .map(|contract| contract.kind)
            .collect::<BTreeSet<_>>();
        let expected = BTreeSet::from([
            ModelingOperationKind::PartTransform,
            ModelingOperationKind::RegionTransform,
            ModelingOperationKind::Bend,
            ModelingOperationKind::Twist,
            ModelingOperationKind::Taper,
            ModelingOperationKind::Bulge,
            ModelingOperationKind::Lattice,
            ModelingOperationKind::Ffd,
            ModelingOperationKind::CageDeformation,
            ModelingOperationKind::JointChainDeformation,
            ModelingOperationKind::SmoothRelax,
            ModelingOperationKind::SurfaceSlide,
            ModelingOperationKind::ShrinkwrapProject,
            ModelingOperationKind::BoundedCorrectiveBasis,
        ]);

        assert_eq!(actual, expected);
        assert_eq!(actual.len(), DEFORMATION_OPERATOR_COUNT);
    }

    #[test]
    fn deformation_contracts_are_strictly_admissible_by_design() {
        let policy = SemanticAdmissibilityPolicy::strict();

        for contract in all_deformation_operator_contracts() {
            assert!(
                contract.is_admissible_under(&policy),
                "{:?} should be admissible under strict semantic policy",
                contract.kind
            );
            assert!(contract.semantic_parameter_count.maximum.is_some());
            assert!(
                contract.semantic_parameter_count.minimum
                    <= contract.semantic_parameter_count.typical
            );
        }
    }

    #[test]
    fn deformation_contracts_do_not_declare_dense_per_vertex_payloads() {
        for contract in all_deformation_operator_contracts() {
            assert!(
                !contract.declares_dense_per_vertex_payload(),
                "{:?} must not carry dense per-vertex deformation payloads",
                contract.kind
            );
            assert_ne!(
                contract.semantic_parameter_count.growth,
                SemanticParameterGrowth::PerAffectedElement,
                "{:?} must not grow one arbitrary tuple per affected element",
                contract.kind
            );
        }
    }

    #[test]
    fn bounded_corrective_basis_uses_low_rank_coefficients_not_target_mesh() {
        let contract = deformation_operator_contract(ModelingOperationKind::BoundedCorrectiveBasis)
            .expect("bounded corrective basis contract should exist");

        assert_eq!(
            contract.semantic_parameter_count.growth,
            SemanticParameterGrowth::BoundedByBasisRank
        );
        assert_eq!(
            contract.compact_control_structure,
            CompactControlStructure::BoundedCorrectiveBasis {
                max_basis_terms: 16,
            }
        );
        assert!(
            !contract
                .allowed_payload_kinds
                .contains(&OperationPayloadKind::DenseDisplacement)
        );
        assert!(
            !contract
                .allowed_payload_kinds
                .contains(&OperationPayloadKind::LiteralTargetMesh)
        );
    }

    #[test]
    fn compact_operation_stub_contains_no_payloads() {
        let contract = deformation_operator_contract(ModelingOperationKind::Bend)
            .expect("bend contract should exist");
        let operation = contract.compact_operation("op.bend");

        assert_eq!(operation.kind, ModelingOperationKind::Bend);
        assert!(operation.payloads.is_empty());
        assert!(!operation
            .parameters
            .iter()
            .any(|parameter| matches!(parameter, SemanticParameter::Choice { name, .. } if name == "vertex")));
    }
}
