//! Diagnostics schema 4 contracts for ordered operator programs.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::bend::{BendParameters, validate_bend_parameters};
use super::program::{SemanticVerificationPolicy, SemanticVerificationReport};

/// Diagnostics schema version for ordered schema-3 program diagnostics.
pub const DIAGNOSTICS_SCHEMA_VERSION_V4: u32 = 4;
/// Score formula version for schema-4 ordered program diagnostics.
pub const INFERENCE_SCORING_VERSION_V4: u32 = 4;
/// Initial conservative bend prior serialized by the default policy.
pub const DEFAULT_BEND_FAMILY_PRIOR_V4: f64 = 1.5e-2;

const FLOAT32_SIZE_BYTES: usize = 4;
const DEFAULT_GEOMETRIC_ERROR_WEIGHT: f64 = 1.0;
const DEFAULT_PARAMETER_WEIGHT: f64 = 2.0e-3;
const DEFAULT_SEMANTIC_METADATA_WEIGHT: f64 = 5.0e-4;
const DEFAULT_APPROXIMATE_RESIDUAL_WEIGHT: f64 = 1.0;
const DEFAULT_EXACT_RESIDUAL_WEIGHT: f64 = 1.0e-3;
const DEFAULT_PER_OPERATOR_OVERHEAD: f64 = 1.0e-3;
const DEFAULT_ABSOLUTE_RESIDUAL_EPSILON: f64 = 1.0e-6;
const DEFAULT_RELATIVE_RESIDUAL_EPSILON: f64 = 1.0e-5;
const DEFAULT_RESIDUAL_ULP_MULTIPLIER: f64 = 2.0;
const RECOMPUTATION_EPSILON: f64 = 1.0e-12;

/// Operator families that can appear in schema-4 explanatory diagnostics.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgramOperatorFamilyV4 {
    /// Uniform-curvature bend.
    Bend,
    /// Arbitrary affine transform.
    GeneralAffine,
    /// Proper rotation plus translation.
    RigidTransform,
    /// Proper rotation, uniform scale, and translation.
    SimilarityTransform,
    /// Pure translation.
    Translation,
}

impl ProgramOperatorFamilyV4 {
    /// Stable snake-case family name used for deterministic lexical tie-breaking.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bend => "bend",
            Self::GeneralAffine => "general_affine",
            Self::RigidTransform => "rigid_transform",
            Self::SimilarityTransform => "similarity_transform",
            Self::Translation => "translation",
        }
    }
}

/// Real semantic parameters for one explanatory operator in diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case")]
pub enum ProgramOperatorDiagnostics {
    /// Pure translation candidate.
    Translation {
        /// Translation vector.
        translation: [f32; 3],
    },
    /// Proper rotation plus translation candidate.
    RigidTransform {
        /// Translation vector.
        translation: [f32; 3],
        /// Row-major 3x3 proper rotation basis.
        rotation_row_major_3x3: [f32; 9],
    },
    /// Proper rotation, uniform scale, and translation candidate.
    SimilarityTransform {
        /// Translation vector.
        translation: [f32; 3],
        /// Row-major 3x3 proper rotation basis.
        rotation_row_major_3x3: [f32; 9],
        /// Positive uniform scale.
        uniform_scale: f32,
    },
    /// Arbitrary affine candidate.
    GeneralAffine {
        /// Row-major 4x4 affine matrix.
        matrix_row_major_4x4: [f32; 16],
    },
    /// Uniform-curvature bend candidate.
    Bend {
        /// Bend parameters.
        parameters: BendParameters,
    },
}

impl ProgramOperatorDiagnostics {
    /// Operator family for scoring, priors, and tie-breaking.
    pub fn family(&self) -> ProgramOperatorFamilyV4 {
        match self {
            Self::Translation { .. } => ProgramOperatorFamilyV4::Translation,
            Self::RigidTransform { .. } => ProgramOperatorFamilyV4::RigidTransform,
            Self::SimilarityTransform { .. } => ProgramOperatorFamilyV4::SimilarityTransform,
            Self::GeneralAffine { .. } => ProgramOperatorFamilyV4::GeneralAffine,
            Self::Bend { .. } => ProgramOperatorFamilyV4::Bend,
        }
    }

    /// Semantic degrees of freedom used for the parameter-count score.
    pub fn semantic_parameter_count(&self) -> usize {
        match self {
            Self::Translation { .. } => 3,
            Self::RigidTransform { .. } => 6,
            Self::SimilarityTransform { .. } => 7,
            Self::GeneralAffine { .. } => 12,
            Self::Bend { .. } => 9,
        }
    }

    /// Serialized scalar payload bytes carried by the diagnostic operator.
    pub fn semantic_metadata_bytes(&self) -> usize {
        let scalar_count = match self {
            Self::Translation { .. } => 3,
            Self::RigidTransform { .. } => 3 + 9,
            Self::SimilarityTransform { .. } => 3 + 9 + 1,
            Self::GeneralAffine { .. } => 16,
            Self::Bend { .. } => 3 + 3 + 3 + 1 + 2,
        };
        scalar_count * FLOAT32_SIZE_BYTES
    }
}

/// Per-stage score and replay diagnostics for a program hypothesis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageDiagnostics {
    /// Zero-based stage index.
    pub stage_index: usize,
    /// Operator parameters evaluated at this stage.
    pub operator: ProgramOperatorDiagnostics,
    /// Triangle-area-weighted error before this stage.
    pub weighted_error_before: f64,
    /// Triangle-area-weighted error after this stage.
    pub weighted_error_after: f64,
    /// Raw unweighted error before this stage.
    pub raw_error_before: f64,
    /// Raw unweighted error after this stage.
    pub raw_error_after: f64,
    /// Weighted error reduction contributed by this stage.
    pub weighted_explained_increment: f64,
    /// Raw error reduction contributed by this stage.
    pub raw_explained_increment: f64,
    /// Maximum absolute semantic-to-baked component error for this stage.
    pub semantic_to_baked_max_component_error: f64,
    /// Maximum semantic-to-baked Euclidean vertex error for this stage.
    pub semantic_to_baked_max_euclidean_error: f64,
    /// RMS semantic-to-baked Euclidean vertex error for this stage.
    pub semantic_to_baked_rms_error: f64,
    /// Verification policy used for semantic-to-baked comparison.
    pub semantic_verification_policy: SemanticVerificationPolicy,
    /// Whether semantic-to-baked verification passed.
    pub semantic_verification_passed: bool,
}

/// Inputs for constructing one stage diagnostic record.
#[derive(Debug, Clone, PartialEq)]
pub struct StageDiagnosticsInput {
    /// Zero-based stage index.
    pub stage_index: usize,
    /// Operator parameters evaluated at this stage.
    pub operator: ProgramOperatorDiagnostics,
    /// Triangle-area-weighted error before this stage.
    pub weighted_error_before: f64,
    /// Triangle-area-weighted error after this stage.
    pub weighted_error_after: f64,
    /// Raw unweighted error before this stage.
    pub raw_error_before: f64,
    /// Raw unweighted error after this stage.
    pub raw_error_after: f64,
    /// Verification policy used for semantic-to-baked comparison.
    pub semantic_verification_policy: SemanticVerificationPolicy,
    /// Semantic-to-baked verification metrics.
    pub semantic_verification_report: SemanticVerificationReport,
}

/// Terminal correction diagnostics for a program hypothesis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramCorrectionDiagnostics {
    /// Number of vertices corrected by the exact residual.
    pub corrected_vertex_count: usize,
    /// Exact correction payload size in bytes.
    pub exact_residual_bytes: usize,
    /// Weighted geometric error before exact correction.
    pub weighted_error_before: f64,
    /// Weighted geometric error after exact correction.
    pub weighted_error_after: f64,
    /// Raw geometric error before exact correction.
    pub raw_error_before: f64,
    /// Raw geometric error after exact correction.
    pub raw_error_after: f64,
}

/// Score breakdown for an ordered explanatory program before final correction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramScoreComponentsV4 {
    /// Final weighted geometric error normalized by the diagnostic error scale and policy weight.
    pub normalized_weighted_final_geometric_error: f64,
    /// Semantic parameter count contribution.
    pub parameter_cost: f64,
    /// Serialized scalar metadata contribution.
    pub semantic_metadata_cost: f64,
    /// Approximate residual coverage contribution.
    pub approximate_residual_coverage_cost: f64,
    /// Exact residual byte contribution.
    pub exact_residual_byte_cost: f64,
    /// Sum of family prior penalties for explanatory operators.
    pub family_prior_sum: f64,
    /// Fixed per-operator overhead contribution.
    pub per_operator_overhead: f64,
    /// Sum of every serialized score component.
    pub total_component_sum: f64,
}

/// Auditable score inputs for a complete explanatory program.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramScoreInputs<'a> {
    /// Ordered explanatory operators before final correction.
    pub operators: &'a [ProgramOperatorDiagnostics],
    /// Final weighted geometric error before final correction.
    pub weighted_final_geometric_error: f64,
    /// Positive scale used to normalize weighted geometric error.
    pub error_normalization_scale: f64,
    /// Full literal target payload size used for byte-normalized costs.
    pub literal_size_bytes: usize,
    /// Tolerance-based residual coverage in `[0, 1]`.
    pub approximate_residual_coverage: f64,
    /// Exact residual correction payload size in bytes.
    pub exact_residual_bytes: usize,
}

/// Score breakdown for one ordered program hypothesis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramHypothesisDiagnosticsV4 {
    /// Ordered explanatory operators before the terminal correction.
    pub operators: Vec<ProgramOperatorDiagnostics>,
    /// Per-stage diagnostics for the explanatory operators.
    pub stages: Vec<StageDiagnostics>,
    /// Terminal exact correction summary.
    pub final_correction: ProgramCorrectionDiagnostics,
    /// Weighted geometric error before the final correction.
    pub weighted_geometric_error: f64,
    /// Raw geometric error before the final correction.
    pub raw_geometric_error: f64,
    /// Surface-weighted displacement fraction explained by the program.
    pub weighted_explained_fraction: f64,
    /// Raw unweighted displacement fraction explained by the program.
    pub raw_explained_fraction: f64,
    /// Weighted error scale used to normalize geometric error.
    pub error_normalization_scale: f64,
    /// Full literal target position payload size used for byte-normalized costs.
    pub literal_size_bytes: usize,
    /// Semantic degree count used for parameter scoring.
    pub semantic_parameter_count: usize,
    /// Serialized scalar payload bytes used for metadata scoring.
    pub semantic_metadata_bytes: usize,
    /// Tolerance-based approximate residual coverage before policy weighting.
    pub approximate_residual_coverage: f64,
    /// Exact residual byte count used for tie-breaking and score verification.
    pub exact_residual_bytes: usize,
    /// Independently serialized score components.
    pub score: ProgramScoreComponentsV4,
    /// Whether this hypothesis was selected.
    pub selected: bool,
    /// Why this hypothesis was rejected, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}

/// Inputs for constructing one complete program diagnostic record.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramDiagnosticsInput {
    /// Ordered explanatory operators before the terminal correction.
    pub operators: Vec<ProgramOperatorDiagnostics>,
    /// Per-stage diagnostics for the explanatory operators.
    pub stages: Vec<StageDiagnostics>,
    /// Terminal exact correction summary.
    pub final_correction: ProgramCorrectionDiagnostics,
    /// Raw source-to-target identity error.
    pub raw_identity_error: f64,
    /// Weighted source-to-target identity error.
    pub weighted_identity_error: f64,
    /// Weighted error scale used to normalize geometric error.
    pub error_normalization_scale: f64,
    /// Full literal target position payload size used for byte-normalized costs.
    pub literal_size_bytes: usize,
    /// Tolerance-based approximate residual coverage before policy weighting.
    pub approximate_residual_coverage: f64,
    /// Scoring constants used to score this program.
    pub scoring_policy: InferenceScoringPolicyV4,
    /// Whether this hypothesis was selected.
    pub selected: bool,
    /// Why this hypothesis was rejected, when applicable.
    pub rejection_reason: Option<String>,
}

/// Tolerance policy used to compute approximate residual coverage.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApproximateResidualTolerancePolicyV4 {
    /// Absolute residual tolerance floor.
    pub absolute_epsilon: f64,
    /// Residual tolerance multiplier for intrinsic shape scale.
    pub relative_epsilon: f64,
    /// Residual tolerance multiplier for local `f32` coordinate spacing.
    pub ulp_multiplier: f64,
}

/// Scoring constants for schema-4 inference diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InferenceScoringPolicyV4 {
    /// Score formula version. Must be [`INFERENCE_SCORING_VERSION_V4`].
    pub scoring_version: u32,
    /// Human-readable scoring model identifier.
    pub model: String,
    /// Weight applied to normalized weighted final geometric error.
    pub geometric_error_weight: f64,
    /// Weight applied to semantic parameter count.
    pub parameter_weight: f64,
    /// Weight applied to serialized scalar metadata bytes.
    pub semantic_metadata_weight: f64,
    /// Weight applied to tolerance-based approximate residual coverage.
    pub approximate_residual_weight: f64,
    /// Weight applied to exact residual byte cost.
    pub exact_residual_weight: f64,
    /// Fixed overhead applied per explanatory operator in a program.
    pub per_operator_overhead: f64,
    /// Fixed family prior penalties for explanatory operators.
    pub family_priors: BTreeMap<ProgramOperatorFamilyV4, f64>,
    /// Tolerance policy used to compute approximate residual coverage.
    pub approximate_residual_tolerance: ApproximateResidualTolerancePolicyV4,
}

impl Default for InferenceScoringPolicyV4 {
    fn default() -> Self {
        default_scoring_policy_v4()
    }
}

/// Top-level diagnostics schema-4 report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InferenceDiagnosticsV4 {
    /// Diagnostics format version. Must be [`DIAGNOSTICS_SCHEMA_VERSION_V4`].
    pub diagnostics_schema_version: u32,
    /// Replay package schema version associated with this report.
    pub package_schema_version: u32,
    /// Weighting model used for semantic fitting and scoring.
    pub surface_weighting: String,
    /// Raw source-to-target identity error.
    pub raw_identity_error: f64,
    /// Weighted source-to-target identity error.
    pub weighted_identity_error: f64,
    /// Scoring constants used for this report.
    pub scoring_policy: InferenceScoringPolicyV4,
    /// Index of the selected program hypothesis.
    pub selected_program_hypothesis_index: usize,
    /// Ordered program hypotheses in deterministic inference order.
    pub program_hypotheses: Vec<ProgramHypothesisDiagnosticsV4>,
}

/// Diagnostics and scoring validation failures.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum DiagnosticsErrorV4 {
    /// A score or diagnostic input was not finite.
    #[error("{field} must be finite")]
    NonFinite { field: &'static str },
    /// A score or diagnostic input was outside its accepted range.
    #[error("{field} is outside the accepted range")]
    InvalidValue { field: &'static str },
    /// A policy was missing a required family prior.
    #[error("missing family prior for {family:?}")]
    MissingFamilyPrior {
        /// Missing family.
        family: ProgramOperatorFamilyV4,
    },
    /// Stage diagnostics did not match the operator program.
    #[error("program has {operator_count} operators but {stage_count} stages")]
    OperatorStageCountMismatch {
        /// Operator count.
        operator_count: usize,
        /// Stage count.
        stage_count: usize,
    },
    /// A stage index was not the deterministic zero-based offset.
    #[error("stage at offset {expected} declared stage_index {actual}")]
    StageIndexMismatch {
        /// Expected zero-based stage index.
        expected: usize,
        /// Actual stage index.
        actual: usize,
    },
    /// A stage operator did not match the program operator at the same index.
    #[error("stage {stage_index} operator does not match operators[{operator_index}]")]
    StageOperatorMismatch {
        /// Stage index.
        stage_index: usize,
        /// Operator index.
        operator_index: usize,
    },
    /// A serialized diagnostic value did not match recomputation.
    #[error("{component} mismatch: expected {expected}, actual {actual}")]
    RecomputationMismatch {
        /// Component name.
        component: &'static str,
        /// Expected recomputed value.
        expected: f64,
        /// Actual serialized value.
        actual: f64,
    },
    /// A serialized diagnostic count did not match recomputation.
    #[error("{component} mismatch: expected {expected}, actual {actual}")]
    CountMismatch {
        /// Component name.
        component: &'static str,
        /// Expected recomputed count.
        expected: usize,
        /// Actual serialized count.
        actual: usize,
    },
    /// Operator parameters were malformed.
    #[error("{0}")]
    InvalidOperator(&'static str),
}

/// Returns the default schema-4 scoring policy.
pub fn default_scoring_policy_v4() -> InferenceScoringPolicyV4 {
    InferenceScoringPolicyV4 {
        scoring_version: INFERENCE_SCORING_VERSION_V4,
        model: "ordered_program_schema4_v1".to_owned(),
        geometric_error_weight: DEFAULT_GEOMETRIC_ERROR_WEIGHT,
        parameter_weight: DEFAULT_PARAMETER_WEIGHT,
        semantic_metadata_weight: DEFAULT_SEMANTIC_METADATA_WEIGHT,
        approximate_residual_weight: DEFAULT_APPROXIMATE_RESIDUAL_WEIGHT,
        exact_residual_weight: DEFAULT_EXACT_RESIDUAL_WEIGHT,
        per_operator_overhead: DEFAULT_PER_OPERATOR_OVERHEAD,
        family_priors: default_family_priors_v4(),
        approximate_residual_tolerance: ApproximateResidualTolerancePolicyV4 {
            absolute_epsilon: DEFAULT_ABSOLUTE_RESIDUAL_EPSILON,
            relative_epsilon: DEFAULT_RELATIVE_RESIDUAL_EPSILON,
            ulp_multiplier: DEFAULT_RESIDUAL_ULP_MULTIPLIER,
        },
    }
}

/// Returns the default fixed family priors used by schema-4 scoring.
pub fn default_family_priors_v4() -> BTreeMap<ProgramOperatorFamilyV4, f64> {
    [
        (ProgramOperatorFamilyV4::Bend, DEFAULT_BEND_FAMILY_PRIOR_V4),
        (ProgramOperatorFamilyV4::GeneralAffine, 1.0e-2),
        (ProgramOperatorFamilyV4::RigidTransform, 1.0e-3),
        (ProgramOperatorFamilyV4::SimilarityTransform, 2.0e-3),
        (ProgramOperatorFamilyV4::Translation, 0.0),
    ]
    .into_iter()
    .collect()
}

/// Builds a validated stage diagnostics record.
pub fn build_stage_diagnostics(
    input: StageDiagnosticsInput,
) -> Result<StageDiagnostics, DiagnosticsErrorV4> {
    validate_operator(&input.operator)?;
    validate_nonnegative_f64("weighted_error_before", input.weighted_error_before)?;
    validate_nonnegative_f64("weighted_error_after", input.weighted_error_after)?;
    validate_nonnegative_f64("raw_error_before", input.raw_error_before)?;
    validate_nonnegative_f64("raw_error_after", input.raw_error_after)?;
    validate_semantic_verification_policy(&input.semantic_verification_policy)?;
    validate_semantic_verification_report(&input.semantic_verification_report)?;

    Ok(StageDiagnostics {
        stage_index: input.stage_index,
        operator: input.operator,
        weighted_error_before: input.weighted_error_before,
        weighted_error_after: input.weighted_error_after,
        raw_error_before: input.raw_error_before,
        raw_error_after: input.raw_error_after,
        weighted_explained_increment: input.weighted_error_before - input.weighted_error_after,
        raw_explained_increment: input.raw_error_before - input.raw_error_after,
        semantic_to_baked_max_component_error: input
            .semantic_verification_report
            .max_component_error,
        semantic_to_baked_max_euclidean_error: input
            .semantic_verification_report
            .max_euclidean_error,
        semantic_to_baked_rms_error: input.semantic_verification_report.rms_euclidean_error,
        semantic_verification_policy: input.semantic_verification_policy,
        semantic_verification_passed: input.semantic_verification_report.passed,
    })
}

/// Scores a complete explanatory program before the final correction is applied.
pub fn score_program(
    policy: &InferenceScoringPolicyV4,
    inputs: ProgramScoreInputs<'_>,
) -> Result<ProgramScoreComponentsV4, DiagnosticsErrorV4> {
    validate_policy(policy)?;
    validate_score_inputs(&inputs)?;
    for operator in inputs.operators {
        validate_operator(operator)?;
    }

    let literal_size_bytes = inputs.literal_size_bytes as f64;
    let normalized_weighted_final_geometric_error = inputs.weighted_final_geometric_error
        / inputs.error_normalization_scale
        * policy.geometric_error_weight;
    let parameter_cost =
        semantic_parameter_count(inputs.operators) as f64 * policy.parameter_weight;
    let semantic_metadata_cost = semantic_metadata_bytes(inputs.operators) as f64
        / literal_size_bytes
        * policy.semantic_metadata_weight;
    let approximate_residual_coverage_cost =
        inputs.approximate_residual_coverage * policy.approximate_residual_weight;
    let exact_residual_byte_cost =
        inputs.exact_residual_bytes as f64 / literal_size_bytes * policy.exact_residual_weight;
    let family_prior_sum = inputs
        .operators
        .iter()
        .map(|operator| {
            policy.family_priors.get(&operator.family()).copied().ok_or(
                DiagnosticsErrorV4::MissingFamilyPrior {
                    family: operator.family(),
                },
            )
        })
        .sum::<Result<f64, _>>()?;
    let per_operator_overhead = inputs.operators.len() as f64 * policy.per_operator_overhead;
    let total_component_sum = normalized_weighted_final_geometric_error
        + parameter_cost
        + semantic_metadata_cost
        + approximate_residual_coverage_cost
        + exact_residual_byte_cost
        + family_prior_sum
        + per_operator_overhead;

    validate_finite(
        "normalized_weighted_final_geometric_error",
        normalized_weighted_final_geometric_error,
    )?;
    validate_finite("parameter_cost", parameter_cost)?;
    validate_finite("semantic_metadata_cost", semantic_metadata_cost)?;
    validate_finite(
        "approximate_residual_coverage_cost",
        approximate_residual_coverage_cost,
    )?;
    validate_finite("exact_residual_byte_cost", exact_residual_byte_cost)?;
    validate_finite("family_prior_sum", family_prior_sum)?;
    validate_finite("per_operator_overhead", per_operator_overhead)?;
    validate_finite("total_component_sum", total_component_sum)?;

    Ok(ProgramScoreComponentsV4 {
        normalized_weighted_final_geometric_error,
        parameter_cost,
        semantic_metadata_cost,
        approximate_residual_coverage_cost,
        exact_residual_byte_cost,
        family_prior_sum,
        per_operator_overhead,
        total_component_sum,
    })
}

/// Builds a validated program diagnostics record and computes its score.
pub fn build_program_diagnostics(
    input: ProgramDiagnosticsInput,
) -> Result<ProgramHypothesisDiagnosticsV4, DiagnosticsErrorV4> {
    validate_stage_sequence(&input.operators, &input.stages)?;
    validate_final_correction(&input.final_correction)?;
    validate_nonnegative_f64("raw_identity_error", input.raw_identity_error)?;
    validate_nonnegative_f64("weighted_identity_error", input.weighted_identity_error)?;
    validate_score_inputs(&ProgramScoreInputs {
        operators: &input.operators,
        weighted_final_geometric_error: input.final_correction.weighted_error_before,
        error_normalization_scale: input.error_normalization_scale,
        literal_size_bytes: input.literal_size_bytes,
        approximate_residual_coverage: input.approximate_residual_coverage,
        exact_residual_bytes: input.final_correction.exact_residual_bytes,
    })?;

    let semantic_parameter_count = semantic_parameter_count(&input.operators);
    let semantic_metadata_bytes = semantic_metadata_bytes(&input.operators);
    let exact_residual_bytes = input.final_correction.exact_residual_bytes;
    let weighted_geometric_error = input.final_correction.weighted_error_before;
    let raw_geometric_error = input.final_correction.raw_error_before;
    let score = score_program(
        &input.scoring_policy,
        ProgramScoreInputs {
            operators: &input.operators,
            weighted_final_geometric_error: weighted_geometric_error,
            error_normalization_scale: input.error_normalization_scale,
            literal_size_bytes: input.literal_size_bytes,
            approximate_residual_coverage: input.approximate_residual_coverage,
            exact_residual_bytes,
        },
    )?;

    Ok(ProgramHypothesisDiagnosticsV4 {
        operators: input.operators,
        stages: input.stages,
        final_correction: input.final_correction,
        weighted_geometric_error,
        raw_geometric_error,
        weighted_explained_fraction: explained_fraction(
            input.weighted_identity_error,
            weighted_geometric_error,
        ),
        raw_explained_fraction: explained_fraction(input.raw_identity_error, raw_geometric_error),
        error_normalization_scale: input.error_normalization_scale,
        literal_size_bytes: input.literal_size_bytes,
        semantic_parameter_count,
        semantic_metadata_bytes,
        approximate_residual_coverage: input.approximate_residual_coverage,
        exact_residual_bytes,
        score,
        selected: input.selected,
        rejection_reason: input.rejection_reason,
    })
}

/// Verifies that a serialized score can be recomputed from policy and diagnostics.
pub fn verify_score_recomputation(
    policy: &InferenceScoringPolicyV4,
    hypothesis: &ProgramHypothesisDiagnosticsV4,
) -> Result<(), DiagnosticsErrorV4> {
    validate_policy(policy)?;
    validate_stage_sequence(&hypothesis.operators, &hypothesis.stages)?;
    validate_final_correction(&hypothesis.final_correction)?;
    validate_nonnegative_f64(
        "weighted_geometric_error",
        hypothesis.weighted_geometric_error,
    )?;
    validate_nonnegative_f64("raw_geometric_error", hypothesis.raw_geometric_error)?;
    validate_nonnegative_f64(
        "weighted_explained_fraction",
        hypothesis.weighted_explained_fraction,
    )?;
    validate_nonnegative_f64("raw_explained_fraction", hypothesis.raw_explained_fraction)?;
    validate_score_inputs(&ProgramScoreInputs {
        operators: &hypothesis.operators,
        weighted_final_geometric_error: hypothesis.weighted_geometric_error,
        error_normalization_scale: hypothesis.error_normalization_scale,
        literal_size_bytes: hypothesis.literal_size_bytes,
        approximate_residual_coverage: hypothesis.approximate_residual_coverage,
        exact_residual_bytes: hypothesis.exact_residual_bytes,
    })?;

    compare_count(
        "semantic_parameter_count",
        semantic_parameter_count(&hypothesis.operators),
        hypothesis.semantic_parameter_count,
    )?;
    compare_count(
        "semantic_metadata_bytes",
        semantic_metadata_bytes(&hypothesis.operators),
        hypothesis.semantic_metadata_bytes,
    )?;
    compare_count(
        "exact_residual_bytes",
        hypothesis.final_correction.exact_residual_bytes,
        hypothesis.exact_residual_bytes,
    )?;
    compare_component(
        "weighted_geometric_error",
        hypothesis.final_correction.weighted_error_before,
        hypothesis.weighted_geometric_error,
    )?;
    compare_component(
        "raw_geometric_error",
        hypothesis.final_correction.raw_error_before,
        hypothesis.raw_geometric_error,
    )?;

    validate_score_components(&hypothesis.score)?;
    let recomputed = score_program(
        policy,
        ProgramScoreInputs {
            operators: &hypothesis.operators,
            weighted_final_geometric_error: hypothesis.weighted_geometric_error,
            error_normalization_scale: hypothesis.error_normalization_scale,
            literal_size_bytes: hypothesis.literal_size_bytes,
            approximate_residual_coverage: hypothesis.approximate_residual_coverage,
            exact_residual_bytes: hypothesis.exact_residual_bytes,
        },
    )?;

    compare_component(
        "normalized_weighted_final_geometric_error",
        recomputed.normalized_weighted_final_geometric_error,
        hypothesis.score.normalized_weighted_final_geometric_error,
    )?;
    compare_component(
        "parameter_cost",
        recomputed.parameter_cost,
        hypothesis.score.parameter_cost,
    )?;
    compare_component(
        "semantic_metadata_cost",
        recomputed.semantic_metadata_cost,
        hypothesis.score.semantic_metadata_cost,
    )?;
    compare_component(
        "approximate_residual_coverage_cost",
        recomputed.approximate_residual_coverage_cost,
        hypothesis.score.approximate_residual_coverage_cost,
    )?;
    compare_component(
        "exact_residual_byte_cost",
        recomputed.exact_residual_byte_cost,
        hypothesis.score.exact_residual_byte_cost,
    )?;
    compare_component(
        "family_prior_sum",
        recomputed.family_prior_sum,
        hypothesis.score.family_prior_sum,
    )?;
    compare_component(
        "per_operator_overhead",
        recomputed.per_operator_overhead,
        hypothesis.score.per_operator_overhead,
    )?;
    compare_component(
        "total_component_sum",
        recomputed.total_component_sum,
        hypothesis.score.total_component_sum,
    )?;

    Ok(())
}

/// Compares two program hypotheses using the schema-4 deterministic tie-breaks.
pub fn compare_program_hypotheses(
    left: &ProgramHypothesisDiagnosticsV4,
    right: &ProgramHypothesisDiagnosticsV4,
) -> Ordering {
    left.score
        .total_component_sum
        .total_cmp(&right.score.total_component_sum)
        .then_with(|| {
            left.approximate_residual_coverage
                .total_cmp(&right.approximate_residual_coverage)
        })
        .then_with(|| left.exact_residual_bytes.cmp(&right.exact_residual_bytes))
        .then_with(|| left.operators.len().cmp(&right.operators.len()))
        .then_with(|| {
            left.semantic_parameter_count
                .cmp(&right.semantic_parameter_count)
        })
        .then_with(|| {
            operator_family_names(&left.operators).cmp(&operator_family_names(&right.operators))
        })
}

fn semantic_parameter_count(operators: &[ProgramOperatorDiagnostics]) -> usize {
    operators
        .iter()
        .map(ProgramOperatorDiagnostics::semantic_parameter_count)
        .sum()
}

fn semantic_metadata_bytes(operators: &[ProgramOperatorDiagnostics]) -> usize {
    operators
        .iter()
        .map(ProgramOperatorDiagnostics::semantic_metadata_bytes)
        .sum()
}

fn operator_family_names(operators: &[ProgramOperatorDiagnostics]) -> Vec<&'static str> {
    operators
        .iter()
        .map(|operator| operator.family().as_str())
        .collect()
}

fn validate_policy(policy: &InferenceScoringPolicyV4) -> Result<(), DiagnosticsErrorV4> {
    if policy.scoring_version != INFERENCE_SCORING_VERSION_V4 {
        return Err(DiagnosticsErrorV4::InvalidValue {
            field: "scoring_version",
        });
    }
    if policy.model.trim().is_empty() {
        return Err(DiagnosticsErrorV4::InvalidValue { field: "model" });
    }
    validate_nonnegative_f64("geometric_error_weight", policy.geometric_error_weight)?;
    validate_nonnegative_f64("parameter_weight", policy.parameter_weight)?;
    validate_nonnegative_f64("semantic_metadata_weight", policy.semantic_metadata_weight)?;
    validate_nonnegative_f64(
        "approximate_residual_weight",
        policy.approximate_residual_weight,
    )?;
    validate_nonnegative_f64("exact_residual_weight", policy.exact_residual_weight)?;
    validate_nonnegative_f64("per_operator_overhead", policy.per_operator_overhead)?;
    validate_approximate_residual_tolerance_policy(&policy.approximate_residual_tolerance)?;
    for family in [
        ProgramOperatorFamilyV4::Bend,
        ProgramOperatorFamilyV4::GeneralAffine,
        ProgramOperatorFamilyV4::RigidTransform,
        ProgramOperatorFamilyV4::SimilarityTransform,
        ProgramOperatorFamilyV4::Translation,
    ] {
        let prior = policy
            .family_priors
            .get(&family)
            .copied()
            .ok_or(DiagnosticsErrorV4::MissingFamilyPrior { family })?;
        validate_nonnegative_f64("family_prior", prior)?;
    }
    Ok(())
}

fn validate_approximate_residual_tolerance_policy(
    policy: &ApproximateResidualTolerancePolicyV4,
) -> Result<(), DiagnosticsErrorV4> {
    validate_nonnegative_f64(
        "approximate_residual_tolerance.absolute_epsilon",
        policy.absolute_epsilon,
    )?;
    validate_nonnegative_f64(
        "approximate_residual_tolerance.relative_epsilon",
        policy.relative_epsilon,
    )?;
    validate_nonnegative_f64(
        "approximate_residual_tolerance.ulp_multiplier",
        policy.ulp_multiplier,
    )
}

fn validate_score_inputs(inputs: &ProgramScoreInputs<'_>) -> Result<(), DiagnosticsErrorV4> {
    validate_nonnegative_f64(
        "weighted_final_geometric_error",
        inputs.weighted_final_geometric_error,
    )?;
    validate_positive_f64(
        "error_normalization_scale",
        inputs.error_normalization_scale,
    )?;
    if inputs.literal_size_bytes == 0 {
        return Err(DiagnosticsErrorV4::InvalidValue {
            field: "literal_size_bytes",
        });
    }
    validate_unit_interval_f64(
        "approximate_residual_coverage",
        inputs.approximate_residual_coverage,
    )
}

fn validate_stage_sequence(
    operators: &[ProgramOperatorDiagnostics],
    stages: &[StageDiagnostics],
) -> Result<(), DiagnosticsErrorV4> {
    if operators.len() != stages.len() {
        return Err(DiagnosticsErrorV4::OperatorStageCountMismatch {
            operator_count: operators.len(),
            stage_count: stages.len(),
        });
    }
    for (index, (operator, stage)) in operators.iter().zip(stages).enumerate() {
        if stage.stage_index != index {
            return Err(DiagnosticsErrorV4::StageIndexMismatch {
                expected: index,
                actual: stage.stage_index,
            });
        }
        if &stage.operator != operator {
            return Err(DiagnosticsErrorV4::StageOperatorMismatch {
                stage_index: stage.stage_index,
                operator_index: index,
            });
        }
        validate_stage(stage)?;
    }
    Ok(())
}

fn validate_stage(stage: &StageDiagnostics) -> Result<(), DiagnosticsErrorV4> {
    validate_operator(&stage.operator)?;
    validate_nonnegative_f64("stage.weighted_error_before", stage.weighted_error_before)?;
    validate_nonnegative_f64("stage.weighted_error_after", stage.weighted_error_after)?;
    validate_nonnegative_f64("stage.raw_error_before", stage.raw_error_before)?;
    validate_nonnegative_f64("stage.raw_error_after", stage.raw_error_after)?;
    validate_finite(
        "stage.weighted_explained_increment",
        stage.weighted_explained_increment,
    )?;
    validate_finite(
        "stage.raw_explained_increment",
        stage.raw_explained_increment,
    )?;
    compare_component(
        "stage.weighted_explained_increment",
        stage.weighted_error_before - stage.weighted_error_after,
        stage.weighted_explained_increment,
    )?;
    compare_component(
        "stage.raw_explained_increment",
        stage.raw_error_before - stage.raw_error_after,
        stage.raw_explained_increment,
    )?;
    validate_nonnegative_f64(
        "stage.semantic_to_baked_max_component_error",
        stage.semantic_to_baked_max_component_error,
    )?;
    validate_nonnegative_f64(
        "stage.semantic_to_baked_max_euclidean_error",
        stage.semantic_to_baked_max_euclidean_error,
    )?;
    validate_nonnegative_f64(
        "stage.semantic_to_baked_rms_error",
        stage.semantic_to_baked_rms_error,
    )?;
    validate_semantic_verification_policy(&stage.semantic_verification_policy)
}

fn validate_final_correction(
    correction: &ProgramCorrectionDiagnostics,
) -> Result<(), DiagnosticsErrorV4> {
    validate_nonnegative_f64(
        "final_correction.weighted_error_before",
        correction.weighted_error_before,
    )?;
    validate_nonnegative_f64(
        "final_correction.weighted_error_after",
        correction.weighted_error_after,
    )?;
    validate_nonnegative_f64(
        "final_correction.raw_error_before",
        correction.raw_error_before,
    )?;
    validate_nonnegative_f64(
        "final_correction.raw_error_after",
        correction.raw_error_after,
    )
}

fn validate_semantic_verification_policy(
    policy: &SemanticVerificationPolicy,
) -> Result<(), DiagnosticsErrorV4> {
    validate_nonnegative_f64(
        "semantic_verification_policy.absolute_epsilon",
        policy.absolute_epsilon,
    )?;
    validate_nonnegative_f64(
        "semantic_verification_policy.relative_epsilon",
        policy.relative_epsilon,
    )?;
    validate_nonnegative_f64(
        "semantic_verification_policy.ulp_multiplier",
        policy.ulp_multiplier,
    )
}

fn validate_semantic_verification_report(
    report: &SemanticVerificationReport,
) -> Result<(), DiagnosticsErrorV4> {
    validate_nonnegative_f64(
        "semantic_verification_report.max_component_error",
        report.max_component_error,
    )?;
    validate_nonnegative_f64(
        "semantic_verification_report.max_euclidean_error",
        report.max_euclidean_error,
    )?;
    validate_nonnegative_f64(
        "semantic_verification_report.mean_euclidean_error",
        report.mean_euclidean_error,
    )?;
    validate_nonnegative_f64(
        "semantic_verification_report.rms_euclidean_error",
        report.rms_euclidean_error,
    )
}

fn validate_score_components(score: &ProgramScoreComponentsV4) -> Result<(), DiagnosticsErrorV4> {
    validate_finite(
        "score.normalized_weighted_final_geometric_error",
        score.normalized_weighted_final_geometric_error,
    )?;
    validate_finite("score.parameter_cost", score.parameter_cost)?;
    validate_finite("score.semantic_metadata_cost", score.semantic_metadata_cost)?;
    validate_finite(
        "score.approximate_residual_coverage_cost",
        score.approximate_residual_coverage_cost,
    )?;
    validate_finite(
        "score.exact_residual_byte_cost",
        score.exact_residual_byte_cost,
    )?;
    validate_finite("score.family_prior_sum", score.family_prior_sum)?;
    validate_finite("score.per_operator_overhead", score.per_operator_overhead)?;
    validate_finite("score.total_component_sum", score.total_component_sum)?;
    compare_component(
        "score.total_component_sum",
        score.normalized_weighted_final_geometric_error
            + score.parameter_cost
            + score.semantic_metadata_cost
            + score.approximate_residual_coverage_cost
            + score.exact_residual_byte_cost
            + score.family_prior_sum
            + score.per_operator_overhead,
        score.total_component_sum,
    )
}

fn validate_operator(operator: &ProgramOperatorDiagnostics) -> Result<(), DiagnosticsErrorV4> {
    match operator {
        ProgramOperatorDiagnostics::Translation { translation } => {
            validate_f32_slice("translation", translation)
        }
        ProgramOperatorDiagnostics::RigidTransform {
            translation,
            rotation_row_major_3x3,
        } => {
            validate_f32_slice("translation", translation)?;
            validate_f32_slice("rotation_row_major_3x3", rotation_row_major_3x3)
        }
        ProgramOperatorDiagnostics::SimilarityTransform {
            translation,
            rotation_row_major_3x3,
            uniform_scale,
        } => {
            validate_f32_slice("translation", translation)?;
            validate_f32_slice("rotation_row_major_3x3", rotation_row_major_3x3)?;
            if !uniform_scale.is_finite() || *uniform_scale <= 0.0 {
                return Err(DiagnosticsErrorV4::InvalidOperator(
                    "similarity uniform_scale must be finite and positive",
                ));
            }
            Ok(())
        }
        ProgramOperatorDiagnostics::GeneralAffine {
            matrix_row_major_4x4,
        } => validate_f32_slice("matrix_row_major_4x4", matrix_row_major_4x4),
        ProgramOperatorDiagnostics::Bend { parameters } => validate_bend_parameters(parameters)
            .map(|_| ())
            .map_err(|_| DiagnosticsErrorV4::InvalidOperator("bend parameters are invalid")),
    }
}

fn validate_f32_slice(field: &'static str, values: &[f32]) -> Result<(), DiagnosticsErrorV4> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(DiagnosticsErrorV4::InvalidOperator(field))
    }
}

fn validate_finite(field: &'static str, value: f64) -> Result<(), DiagnosticsErrorV4> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(DiagnosticsErrorV4::NonFinite { field })
    }
}

fn validate_nonnegative_f64(field: &'static str, value: f64) -> Result<(), DiagnosticsErrorV4> {
    validate_finite(field, value)?;
    if value >= 0.0 {
        Ok(())
    } else {
        Err(DiagnosticsErrorV4::InvalidValue { field })
    }
}

fn validate_positive_f64(field: &'static str, value: f64) -> Result<(), DiagnosticsErrorV4> {
    validate_finite(field, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(DiagnosticsErrorV4::InvalidValue { field })
    }
}

fn validate_unit_interval_f64(field: &'static str, value: f64) -> Result<(), DiagnosticsErrorV4> {
    validate_finite(field, value)?;
    if (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(DiagnosticsErrorV4::InvalidValue { field })
    }
}

fn compare_component(
    component: &'static str,
    expected: f64,
    actual: f64,
) -> Result<(), DiagnosticsErrorV4> {
    validate_finite(component, expected)?;
    validate_finite(component, actual)?;
    if (expected - actual).abs() <= RECOMPUTATION_EPSILON {
        Ok(())
    } else {
        Err(DiagnosticsErrorV4::RecomputationMismatch {
            component,
            expected,
            actual,
        })
    }
}

fn compare_count(
    component: &'static str,
    expected: usize,
    actual: usize,
) -> Result<(), DiagnosticsErrorV4> {
    if expected == actual {
        Ok(())
    } else {
        Err(DiagnosticsErrorV4::CountMismatch {
            component,
            expected,
            actual,
        })
    }
}

fn explained_fraction(identity_error: f64, candidate_error: f64) -> f64 {
    if identity_error <= f64::EPSILON {
        1.0
    } else {
        (1.0 - candidate_error / identity_error).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_explanatory_operator_hypothesis_roundtrips() {
        let policy = default_scoring_policy_v4();
        let hypothesis = build_program_diagnostics(ProgramDiagnosticsInput {
            operators: Vec::new(),
            stages: Vec::new(),
            final_correction: ProgramCorrectionDiagnostics {
                corrected_vertex_count: 4,
                exact_residual_bytes: 64,
                weighted_error_before: 1.0,
                weighted_error_after: 0.0,
                raw_error_before: 1.0,
                raw_error_after: 0.0,
            },
            raw_identity_error: 1.0,
            weighted_identity_error: 1.0,
            error_normalization_scale: 1.0,
            literal_size_bytes: 120,
            approximate_residual_coverage: 1.0,
            scoring_policy: policy.clone(),
            selected: true,
            rejection_reason: None,
        })
        .unwrap();
        let diagnostics = InferenceDiagnosticsV4 {
            diagnostics_schema_version: DIAGNOSTICS_SCHEMA_VERSION_V4,
            package_schema_version: 3,
            surface_weighting: "triangle_area_derived_vertex_weights".to_owned(),
            raw_identity_error: 1.0,
            weighted_identity_error: 1.0,
            scoring_policy: policy,
            selected_program_hypothesis_index: 0,
            program_hypotheses: vec![hypothesis],
        };

        let json = serde_json::to_string_pretty(&diagnostics).unwrap();
        let decoded: InferenceDiagnosticsV4 = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, diagnostics);
        assert!(decoded.program_hypotheses[0].operators.is_empty());
    }
}
