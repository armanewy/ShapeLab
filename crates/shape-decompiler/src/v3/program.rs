//! Ordered schema-3 operator programs.
//!
//! A program contains only explanatory semantic operators. The terminal
//! lossless correction used by packages is intentionally outside
//! [`OperatorProgram`] and is appended by package construction after the
//! explanatory operators.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::AffineSemanticFamily;
use crate::apply_affine_to_positions;

use super::bend::{
    BendEvaluationError, BendParameters, BendValidationError, evaluate_bend,
    validate_bend_parameters,
};

/// Stable identifier for an explanatory operator.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OperatorId(pub String);

/// Zero-based stage index in an evaluated ordered program.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StageIndex(pub usize);

/// Verification mode for comparing semantic evaluation to baked positions.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticVerificationMode {
    /// Every serialized `f32` component must match exactly.
    BitExact,
    /// Components may differ within the declared tolerance policy.
    Tolerance,
}

/// Tolerance policy for semantic-to-baked stage verification.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticVerificationPolicy {
    /// Verification mode for this stage.
    pub mode: SemanticVerificationMode,
    /// Absolute component tolerance.
    pub absolute_epsilon: f64,
    /// Relative tolerance scaled by local magnitude.
    pub relative_epsilon: f64,
    /// Multiplier for local `f32` unit-in-the-last-place spacing.
    pub ulp_multiplier: f64,
}

impl Default for SemanticVerificationPolicy {
    fn default() -> Self {
        Self {
            mode: SemanticVerificationMode::BitExact,
            absolute_epsilon: 0.0,
            relative_epsilon: 0.0,
            ulp_multiplier: 0.0,
        }
    }
}

/// Metrics from comparing a semantic operator stage to its baked stage.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticVerificationReport {
    /// Maximum absolute per-component error.
    pub max_component_error: f64,
    /// Maximum Euclidean vertex error.
    pub max_euclidean_error: f64,
    /// Mean Euclidean vertex error.
    pub mean_euclidean_error: f64,
    /// Root-mean-square Euclidean vertex error.
    pub rms_euclidean_error: f64,
    /// Number of vertices outside the declared tolerance.
    pub outside_tolerance: usize,
    /// Whether the semantic stage satisfied the policy.
    pub passed: bool,
}

impl Default for SemanticVerificationReport {
    fn default() -> Self {
        Self {
            max_component_error: 0.0,
            max_euclidean_error: 0.0,
            mean_euclidean_error: 0.0,
            rms_euclidean_error: 0.0,
            outside_tolerance: 0,
            passed: true,
        }
    }
}

/// Metadata shared by schema-3 package stages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageCommon {
    /// Stable stage/operator identifier.
    pub id: String,
    /// Human-facing stage label.
    pub label: String,
    /// Package-relative cumulative baked positions file for exact replay.
    pub baked_positions_file: String,
    /// Policy used to compare semantic evaluation to the baked stage.
    pub semantic_verification_policy: SemanticVerificationPolicy,
    /// Report from comparing semantic evaluation to the baked stage.
    pub semantic_verification_report: SemanticVerificationReport,
}

/// Schema-3 affine explanatory operator.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct AffineOperator {
    /// More editable affine semantic family represented by the matrix.
    pub semantic_family: AffineSemanticFamily,
    /// Row-major 4x4 matrix using the schema-2 affine arithmetic contract.
    pub matrix_row_major_4x4: [f32; 16],
    /// Translation parameters for translation, rigid, and similarity families.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<[f32; 3]>,
    /// Row-major 3x3 proper rotation for rigid and similarity families.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_row_major_3x3: Option<[f32; 9]>,
    /// Positive uniform scale for similarity transforms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uniform_scale: Option<f32>,
}

/// One explanatory operator in an ordered program.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProgramOperator {
    /// Affine-family explanatory operator.
    Affine(AffineOperator),
    /// Uniform-curvature bend explanatory operator.
    Bend(BendParameters),
}

/// Ordered explanatory operator program.
///
/// An empty program is valid and means no explanatory operations were inferred.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorProgram {
    /// Ordered explanatory operators.
    pub operators: Vec<ProgramOperator>,
}

/// Cumulative result after evaluating one program operator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluatedProgramStage {
    /// Zero-based operator index.
    pub operator_index: StageIndex,
    /// Operator evaluated for this stage.
    pub operator: ProgramOperator,
    /// Cumulative positions after this operator.
    pub cumulative_positions: Vec<[f32; 3]>,
    /// Semantic verification report for this stage.
    pub semantic_verification_report: SemanticVerificationReport,
}

/// Complete semantic evaluation result for an operator program.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramEvaluation {
    /// Ordered cumulative stage evaluations.
    pub stages: Vec<EvaluatedProgramStage>,
    /// Final cumulative positions after every explanatory operator.
    pub final_positions: Vec<[f32; 3]>,
}

/// Validation failures for schema-3 operator programs.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ProgramValidationError {
    /// An affine matrix contained non-finite values.
    #[error("affine matrix must contain only finite values")]
    NonFiniteAffineMatrix,
    /// The affine matrix bottom row was not exactly `[0, 0, 0, 1]`.
    #[error("affine matrix bottom row must be exactly [0, 0, 0, 1]")]
    InvalidAffineBottomRow,
    /// Affine semantic parameters were missing, invalid, or inconsistent.
    #[error("{0}")]
    InvalidAffineSemantics(&'static str),
    /// A bend operator failed parameter validation.
    #[error(transparent)]
    Bend(#[from] BendValidationError),
}

/// Evaluation failures for schema-3 operator programs.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ProgramEvaluationError {
    /// Program validation failed before evaluation.
    #[error(transparent)]
    Validation(#[from] ProgramValidationError),
    /// Bend evaluation is not yet implemented for non-identity bends.
    #[error(transparent)]
    Bend(#[from] BendEvaluationError),
}

/// Validates an ordered explanatory operator program.
///
/// Empty programs are accepted.
pub fn validate_program(program: &OperatorProgram) -> Result<(), ProgramValidationError> {
    for operator in &program.operators {
        validate_operator(operator)?;
    }
    Ok(())
}

/// Evaluates one explanatory operator against current positions.
///
/// Affine evaluation delegates to the existing schema-2 canonical evaluator so
/// stepwise binary32 arithmetic stays in one implementation. Non-identity bend
/// evaluation returns a typed not-implemented error.
pub fn evaluate_operator(
    operator: &ProgramOperator,
    positions: &[[f32; 3]],
) -> Result<Vec<[f32; 3]>, ProgramEvaluationError> {
    validate_operator(operator)?;
    match operator {
        ProgramOperator::Affine(affine) => Ok(apply_affine_to_positions(
            positions,
            affine.matrix_row_major_4x4,
        )),
        ProgramOperator::Bend(parameters) => Ok(evaluate_bend(parameters, positions)?),
    }
}

/// Evaluates all explanatory operators in order.
pub fn evaluate_program(
    program: &OperatorProgram,
    source_positions: &[[f32; 3]],
) -> Result<ProgramEvaluation, ProgramEvaluationError> {
    validate_program(program)?;
    let mut stages = Vec::with_capacity(program.operators.len());
    let mut current = source_positions.to_vec();
    for (index, operator) in program.operators.iter().copied().enumerate() {
        current = evaluate_operator(&operator, &current)?;
        stages.push(EvaluatedProgramStage {
            operator_index: StageIndex(index),
            operator,
            cumulative_positions: current.clone(),
            semantic_verification_report: SemanticVerificationReport::default(),
        });
    }
    Ok(ProgramEvaluation {
        stages,
        final_positions: current,
    })
}

fn validate_operator(operator: &ProgramOperator) -> Result<(), ProgramValidationError> {
    match operator {
        ProgramOperator::Affine(affine) => validate_affine_operator(*affine),
        ProgramOperator::Bend(parameters) => {
            validate_bend_parameters(parameters)?;
            Ok(())
        }
    }
}

fn validate_affine_operator(affine: AffineOperator) -> Result<(), ProgramValidationError> {
    if !affine
        .matrix_row_major_4x4
        .iter()
        .all(|value| value.is_finite())
    {
        return Err(ProgramValidationError::NonFiniteAffineMatrix);
    }
    let expected_bottom_row = [0.0_f32, 0.0, 0.0, 1.0];
    if !affine.matrix_row_major_4x4[12..16]
        .iter()
        .zip(expected_bottom_row)
        .all(|(actual, expected)| actual.to_bits() == expected.to_bits())
    {
        return Err(ProgramValidationError::InvalidAffineBottomRow);
    }

    match affine.semantic_family {
        AffineSemanticFamily::GeneralAffine => {
            if affine.translation.is_some()
                || affine.rotation_row_major_3x3.is_some()
                || affine.uniform_scale.is_some()
            {
                return Err(ProgramValidationError::InvalidAffineSemantics(
                    "general affine must not declare semantic parameters",
                ));
            }
        }
        AffineSemanticFamily::Translation => {
            let translation = require_translation(affine.translation, "translation")?;
            reject_rotation_or_scale(
                affine.rotation_row_major_3x3,
                affine.uniform_scale,
                "translation",
            )?;
            if affine.matrix_row_major_4x4 != translation_matrix(translation) {
                return Err(ProgramValidationError::InvalidAffineSemantics(
                    "translation matrix must match translation parameters",
                ));
            }
        }
        AffineSemanticFamily::RigidTransform => {
            let translation = require_translation(affine.translation, "rigid transform")?;
            let rotation = require_rotation(affine.rotation_row_major_3x3, "rigid transform")?;
            if affine.uniform_scale.is_some() {
                return Err(ProgramValidationError::InvalidAffineSemantics(
                    "rigid transform must not declare uniform scale",
                ));
            }
            if affine.matrix_row_major_4x4 != rigid_matrix(rotation, translation) {
                return Err(ProgramValidationError::InvalidAffineSemantics(
                    "rigid transform matrix must match semantic parameters",
                ));
            }
        }
        AffineSemanticFamily::SimilarityTransform => {
            let translation = require_translation(affine.translation, "similarity transform")?;
            let rotation = require_rotation(affine.rotation_row_major_3x3, "similarity transform")?;
            let uniform_scale =
                affine
                    .uniform_scale
                    .ok_or(ProgramValidationError::InvalidAffineSemantics(
                        "similarity transform is missing uniform scale",
                    ))?;
            if !uniform_scale.is_finite() || uniform_scale <= 0.0 {
                return Err(ProgramValidationError::InvalidAffineSemantics(
                    "similarity transform uniform scale must be finite and positive",
                ));
            }
            if affine.matrix_row_major_4x4
                != similarity_matrix(rotation, uniform_scale, translation)
            {
                return Err(ProgramValidationError::InvalidAffineSemantics(
                    "similarity transform matrix must match semantic parameters",
                ));
            }
        }
    }
    Ok(())
}

fn require_translation(
    translation: Option<[f32; 3]>,
    label: &'static str,
) -> Result<[f32; 3], ProgramValidationError> {
    let translation = translation.ok_or(ProgramValidationError::InvalidAffineSemantics(
        match label {
            "translation" => "translation is missing translation parameters",
            "rigid transform" => "rigid transform is missing translation parameters",
            "similarity transform" => "similarity transform is missing translation parameters",
            _ => "affine operator is missing translation parameters",
        },
    ))?;
    if !translation.iter().all(|value| value.is_finite()) {
        return Err(ProgramValidationError::InvalidAffineSemantics(
            "translation parameters must be finite",
        ));
    }
    Ok(translation)
}

fn require_rotation(
    rotation: Option<[f32; 9]>,
    label: &'static str,
) -> Result<[f32; 9], ProgramValidationError> {
    let rotation = rotation.ok_or(ProgramValidationError::InvalidAffineSemantics(
        match label {
            "rigid transform" => "rigid transform is missing rotation parameters",
            "similarity transform" => "similarity transform is missing rotation parameters",
            _ => "affine operator is missing rotation parameters",
        },
    ))?;
    if !rotation.iter().all(|value| value.is_finite()) {
        return Err(ProgramValidationError::InvalidAffineSemantics(
            "rotation parameters must be finite",
        ));
    }
    Ok(rotation)
}

fn reject_rotation_or_scale(
    rotation: Option<[f32; 9]>,
    uniform_scale: Option<f32>,
    label: &'static str,
) -> Result<(), ProgramValidationError> {
    if rotation.is_some() || uniform_scale.is_some() {
        return Err(ProgramValidationError::InvalidAffineSemantics(
            match label {
                "translation" => "translation must not declare rotation or uniform scale",
                _ => "affine operator declares unsupported semantic parameters",
            },
        ));
    }
    Ok(())
}

fn translation_matrix(translation: [f32; 3]) -> [f32; 16] {
    [
        1.0,
        0.0,
        0.0,
        translation[0],
        0.0,
        1.0,
        0.0,
        translation[1],
        0.0,
        0.0,
        1.0,
        translation[2],
        0.0,
        0.0,
        0.0,
        1.0,
    ]
}

fn rigid_matrix(rotation: [f32; 9], translation: [f32; 3]) -> [f32; 16] {
    [
        rotation[0],
        rotation[1],
        rotation[2],
        translation[0],
        rotation[3],
        rotation[4],
        rotation[5],
        translation[1],
        rotation[6],
        rotation[7],
        rotation[8],
        translation[2],
        0.0,
        0.0,
        0.0,
        1.0,
    ]
}

fn similarity_matrix(rotation: [f32; 9], uniform_scale: f32, translation: [f32; 3]) -> [f32; 16] {
    [
        rotation[0] * uniform_scale,
        rotation[1] * uniform_scale,
        rotation[2] * uniform_scale,
        translation[0],
        rotation[3] * uniform_scale,
        rotation[4] * uniform_scale,
        rotation[5] * uniform_scale,
        translation[1],
        rotation[6] * uniform_scale,
        rotation[7] * uniform_scale,
        rotation[8] * uniform_scale,
        translation[2],
        0.0,
        0.0,
        0.0,
        1.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_program_is_valid_and_evaluates_to_source() {
        let source = [[1.0, 2.0, 3.0]];
        let program = OperatorProgram {
            operators: Vec::new(),
        };

        let evaluation = evaluate_program(&program, &source).unwrap();

        assert!(evaluation.stages.is_empty());
        assert_eq!(evaluation.final_positions, source);
    }

    #[test]
    fn affine_evaluation_delegates_to_canonical_schema_two_arithmetic() {
        let position = [[
            f32::from_bits(0x3f7e_7e92),
            f32::from_bits(0xbf80_2a10),
            f32::from_bits(0xbf7f_a514),
        ]];
        let matrix = [
            f32::from_bits(0xbda0_7359),
            f32::from_bits(0x3f73_68b4),
            f32::from_bits(0x3fad_290e),
            f32::from_bits(0xbf8c_b836),
            f32::from_bits(0x3fb9_8184),
            f32::from_bits(0xbf78_b131),
            f32::from_bits(0x3f18_9150),
            f32::from_bits(0xc039_a62e),
            f32::from_bits(0xbf96_330c),
            f32::from_bits(0xbf82_756f),
            f32::from_bits(0xbf9c_5776),
            f32::from_bits(0x4007_f472),
            0.0,
            0.0,
            0.0,
            1.0,
        ];
        let operator = ProgramOperator::Affine(AffineOperator {
            semantic_family: AffineSemanticFamily::GeneralAffine,
            matrix_row_major_4x4: matrix,
            translation: None,
            rotation_row_major_3x3: None,
            uniform_scale: None,
        });

        let evaluated = evaluate_operator(&operator, &position).unwrap()[0];

        assert_eq!(evaluated[0].to_bits(), 0xc05e_bc1d);
        assert_eq!(evaluated[1].to_bits(), 0xbf8a_8e3e);
        assert_eq!(evaluated[2].to_bits(), 0x404c_ac1c);
    }

    #[test]
    fn affine_semantic_parameters_are_validated() {
        let program = OperatorProgram {
            operators: vec![ProgramOperator::Affine(AffineOperator {
                semantic_family: AffineSemanticFamily::Translation,
                matrix_row_major_4x4: translation_matrix([1.0, 2.0, 3.0]),
                translation: Some([1.0, 2.0, 3.0]),
                rotation_row_major_3x3: None,
                uniform_scale: None,
            })],
        };

        validate_program(&program).unwrap();
    }

    #[test]
    fn bend_not_implemented_is_typed() {
        let program = OperatorProgram {
            operators: vec![ProgramOperator::Bend(BendParameters {
                origin: [0.0, 0.0, 0.0],
                longitudinal_axis: [0.0, 1.0, 0.0],
                bend_direction: [1.0, 0.0, 0.0],
                angle_radians: 0.5,
                interval_start: 0.0,
                interval_end: 1.0,
            })],
        };

        let error = evaluate_program(&program, &[[0.0, 0.0, 0.0]]).unwrap_err();

        assert_eq!(
            error,
            ProgramEvaluationError::Bend(BendEvaluationError::NotImplemented)
        );
    }
}
