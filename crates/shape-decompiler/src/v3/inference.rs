//! Future schema-3 candidate generation and ordered program search contracts.
//!
//! This module exposes stable contracts for deterministic candidate generation
//! and ordered search.

use serde::{Deserialize, Serialize};
use shape_mesh::TriangleMesh;
use thiserror::Error;

use crate::{
    AffineSemanticFamily, apply_affine_to_positions, explained_fraction, fit_affine,
    fit_rigid_matrix, fit_similarity_matrix, fit_translation_matrix, sum_squared_distance,
    weighted_sum_squared_distance,
};

use super::diagnostics::{InferenceDiagnosticsV4, ProgramOperatorDiagnostics};
use super::program::{AffineOperator, OperatorProgram, ProgramOperator};

pub mod program_search;

/// Settings for fitting uniform-curvature bend candidates.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct BendFitSettings {
    /// Maximum absolute bend angle to consider, in radians.
    pub maximum_absolute_angle_radians: f32,
    /// Minimum longitudinal interval length to consider.
    pub minimum_interval_length: f32,
    /// Maximum deterministic refinement iterations for a candidate.
    pub maximum_refinement_iterations: usize,
}

impl Default for BendFitSettings {
    fn default() -> Self {
        Self {
            maximum_absolute_angle_radians: std::f32::consts::PI,
            minimum_interval_length: 1.0e-4,
            maximum_refinement_iterations: 4,
        }
    }
}

/// Settings for ordered explanatory program search.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramSearchSettings {
    /// Maximum explanatory operator depth. Schema-3 contract stubs allow at most 2.
    pub maximum_explanatory_depth: usize,
    /// Maximum bend candidates admitted to program enumeration.
    pub maximum_bend_candidates: usize,
    /// Maximum total ordered programs to score.
    pub maximum_total_programs: usize,
    /// Minimum weighted explained fraction required for explanatory programs.
    pub minimum_weighted_explained_fraction: f64,
    /// Deterministic seed reserved for future tie-breaking or sampling.
    pub deterministic_seed: u64,
}

impl Default for ProgramSearchSettings {
    fn default() -> Self {
        Self {
            maximum_explanatory_depth: 2,
            maximum_bend_candidates: 64,
            maximum_total_programs: 256,
            minimum_weighted_explained_fraction: 0.0,
            deterministic_seed: 0,
        }
    }
}

/// Deterministic fitting trace attached to one operator candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FittingDiagnostics {
    /// Generator implementation that produced this candidate.
    #[serde(default)]
    pub generator: String,
    /// Rank in the coarse search before refinement, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coarse_rank: Option<usize>,
    /// Number of deterministic refinement rounds applied to this candidate.
    #[serde(default)]
    pub refinement_rounds: usize,
    /// Number of candidate evaluations spent during coordinate descent.
    #[serde(default)]
    pub coordinate_descent_evaluations: usize,
    /// Parameter duplicates rejected while building this candidate set.
    #[serde(default)]
    pub duplicate_parameter_rejections: usize,
    /// Output-geometry duplicates rejected while building this candidate set.
    #[serde(default)]
    pub duplicate_geometry_rejections: usize,
}

/// One fitted semantic operator candidate available to program search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FittedOperatorCandidate {
    /// Semantic operator represented by this candidate.
    pub operator: ProgramOperator,
    /// Diagnostics-ready semantic parameter record.
    pub diagnostics: ProgramOperatorDiagnostics,
    /// Cumulative positions after applying this candidate to its source.
    #[serde(default)]
    pub cumulative_positions: Vec<[f32; 3]>,
    /// Weighted error before applying this candidate.
    pub weighted_error_before: f64,
    /// Weighted error after applying this candidate.
    pub weighted_error_after: f64,
    /// Raw unweighted error before applying this candidate.
    pub raw_error_before: f64,
    /// Raw unweighted error after applying this candidate.
    pub raw_error_after: f64,
    /// Weighted explained fraction contributed by this candidate.
    pub weighted_explained_fraction: f64,
    /// Raw unweighted explained fraction contributed by this candidate.
    pub raw_explained_fraction: f64,
    /// Semantic degrees of freedom represented by this candidate.
    #[serde(default)]
    pub semantic_parameter_count: usize,
    /// Serialized semantic scalar payload size in bytes.
    #[serde(default)]
    pub semantic_metadata_bytes: usize,
    /// Deterministic candidate identifier derived from semantic parameters.
    #[serde(default)]
    pub stable_candidate_id: String,
    /// Deterministic fitting trace.
    #[serde(default)]
    pub fitting_diagnostics: FittingDiagnostics,
}

/// One ordered explanatory program hypothesis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramHypothesis {
    /// Ordered explanatory program.
    pub program: OperatorProgram,
    /// Fitted candidates used by the program in order.
    pub candidates: Vec<FittedOperatorCandidate>,
    /// Weighted explained fraction for the full program.
    pub weighted_explained_fraction: f64,
    /// Raw unweighted explained fraction for the full program.
    pub raw_explained_fraction: f64,
    /// Total deterministic selection score.
    pub total_score: f64,
}

/// Result of deterministic ordered program search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramSearchResult {
    /// Program hypotheses in deterministic scoring order.
    pub hypotheses: Vec<ProgramHypothesis>,
    /// Selected hypothesis index, when any hypothesis is selectable.
    pub selected_hypothesis_index: Option<usize>,
    /// Optional schema-4 diagnostics report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<InferenceDiagnosticsV4>,
}

/// Trait for future bend candidate generators.
pub trait BendCandidateGenerator {
    /// Generates deterministic bend candidates for an ordered source/target pair.
    fn generate_candidates(
        &self,
        source_positions: &[[f32; 3]],
        target_positions: &[[f32; 3]],
        fit_settings: &BendFitSettings,
        search_settings: &ProgramSearchSettings,
    ) -> Result<Vec<FittedOperatorCandidate>, InferenceError>;
}

/// Trait for affine candidate generators used by ordered program search.
pub trait AffineCandidateGenerator {
    /// Generates deterministic affine-family candidates for an ordered source/target pair.
    fn generate_candidates(
        &self,
        source_positions: &[[f32; 3]],
        target_positions: &[[f32; 3]],
        search_settings: &ProgramSearchSettings,
    ) -> Result<Vec<FittedOperatorCandidate>, InferenceError>;
}

/// Inference contract errors.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum InferenceError {
    /// Program search depth exceeded the current schema-3 limit.
    #[error("maximum explanatory depth {actual} exceeds current limit {limit}")]
    ExplanatoryDepthLimitExceeded {
        /// Current implementation limit.
        limit: usize,
        /// Requested depth.
        actual: usize,
    },
    /// A numeric setting was non-finite or outside its accepted range.
    #[error("{0}")]
    InvalidSettings(&'static str),
}

/// Generates deterministic affine-family candidates for an ordered
/// source/target position pair.
///
/// The current contract emits at most one candidate for each affine semantic
/// family: translation, rigid transform, similarity transform, and general
/// affine. Candidate evaluation delegates to the schema-2 canonical affine
/// replay function so schema 3 does not fork numeric behavior.
pub fn generate_affine_candidates(
    source_positions: &[[f32; 3]],
    target_positions: &[[f32; 3]],
    search_settings: &ProgramSearchSettings,
) -> Result<Vec<FittedOperatorCandidate>, InferenceError> {
    validate_position_pair(source_positions, target_positions)?;
    validate_program_search_settings(search_settings)?;

    let weights = uniform_weights(source_positions.len());
    let mut candidates = Vec::new();

    if let Some(matrix) = fit_translation_matrix(source_positions, target_positions, &weights) {
        let translation = [matrix[3], matrix[7], matrix[11]];
        candidates.push(affine_candidate(
            source_positions,
            target_positions,
            &weights,
            AffineOperator {
                semantic_family: AffineSemanticFamily::Translation,
                matrix_row_major_4x4: matrix,
                translation: Some(translation),
                rotation_row_major_3x3: None,
                uniform_scale: None,
            },
            ProgramOperatorDiagnostics::Translation { translation },
        ));
    }

    if let Some(rigid) = fit_rigid_matrix(source_positions, target_positions, &weights)
        && let (Some(translation), Some(rotation)) =
            (rigid.parameters.translation, rigid.parameters.rotation)
    {
        candidates.push(affine_candidate(
            source_positions,
            target_positions,
            &weights,
            AffineOperator {
                semantic_family: AffineSemanticFamily::RigidTransform,
                matrix_row_major_4x4: rigid.matrix,
                translation: Some(translation),
                rotation_row_major_3x3: Some(rotation),
                uniform_scale: None,
            },
            ProgramOperatorDiagnostics::RigidTransform {
                translation,
                rotation_row_major_3x3: rotation,
            },
        ));
    }

    if let Some(similarity) = fit_similarity_matrix(source_positions, target_positions, &weights)
        && let (Some(translation), Some(rotation), Some(uniform_scale)) = (
            similarity.parameters.translation,
            similarity.parameters.rotation,
            similarity.parameters.uniform_scale,
        )
    {
        candidates.push(affine_candidate(
            source_positions,
            target_positions,
            &weights,
            AffineOperator {
                semantic_family: AffineSemanticFamily::SimilarityTransform,
                matrix_row_major_4x4: similarity.matrix,
                translation: Some(translation),
                rotation_row_major_3x3: Some(rotation),
                uniform_scale: Some(uniform_scale),
            },
            ProgramOperatorDiagnostics::SimilarityTransform {
                translation,
                rotation_row_major_3x3: rotation,
                uniform_scale,
            },
        ));
    }

    if let Some(matrix) = fit_affine(source_positions, target_positions, &weights) {
        candidates.push(affine_candidate(
            source_positions,
            target_positions,
            &weights,
            AffineOperator {
                semantic_family: AffineSemanticFamily::GeneralAffine,
                matrix_row_major_4x4: matrix,
                translation: None,
                rotation_row_major_3x3: None,
                uniform_scale: None,
            },
            ProgramOperatorDiagnostics::GeneralAffine {
                matrix_row_major_4x4: matrix,
            },
        ));
    }

    Ok(candidates)
}

/// Generates deterministic bend candidates using uniform vertex weights.
pub fn generate_bend_candidates(
    source_positions: &[[f32; 3]],
    target_positions: &[[f32; 3]],
    fit_settings: &BendFitSettings,
    search_settings: &ProgramSearchSettings,
) -> Result<Vec<FittedOperatorCandidate>, InferenceError> {
    validate_bend_fit_settings(fit_settings)?;
    validate_program_search_settings(search_settings)?;
    let mut candidates = super::bend_fit::generate_bend_candidates(
        source_positions,
        target_positions,
        &[],
        &[],
        *fit_settings,
    )?;
    candidates.truncate(search_settings.maximum_bend_candidates);
    Ok(candidates)
}

/// Searches ordered explanatory operator programs.
///
/// The current search enumerates the deterministic lossless-only baseline and
/// one-step affine-family programs. Bend candidates are intentionally not
/// admitted until bend inference is enabled.
pub fn search_programs(
    source_positions: &[[f32; 3]],
    target_positions: &[[f32; 3]],
    settings: &ProgramSearchSettings,
) -> Result<ProgramSearchResult, InferenceError> {
    program_search::search_programs(source_positions, target_positions, settings)
}

/// Searches ordered explanatory operator programs using mesh topology for
/// area-derived surface weights.
pub fn search_programs_for_mesh_pair(
    source: &TriangleMesh,
    target: &TriangleMesh,
    settings: &ProgramSearchSettings,
    enable_bend: bool,
) -> Result<ProgramSearchResult, InferenceError> {
    program_search::search_programs_for_mesh_pair(source, target, settings, enable_bend)
}

/// Validates bend fit settings.
pub fn validate_bend_fit_settings(settings: &BendFitSettings) -> Result<(), InferenceError> {
    if !settings.maximum_absolute_angle_radians.is_finite()
        || settings.maximum_absolute_angle_radians < 0.0
    {
        return Err(InferenceError::InvalidSettings(
            "maximum bend angle must be finite and non-negative",
        ));
    }
    if !settings.minimum_interval_length.is_finite() || settings.minimum_interval_length < 0.0 {
        return Err(InferenceError::InvalidSettings(
            "minimum interval length must be finite and non-negative",
        ));
    }
    Ok(())
}

fn validate_position_pair(
    source_positions: &[[f32; 3]],
    target_positions: &[[f32; 3]],
) -> Result<(), InferenceError> {
    if source_positions.is_empty() || source_positions.len() != target_positions.len() {
        return Err(InferenceError::InvalidSettings(
            "source and target positions must be non-empty and have equal counts",
        ));
    }
    if !source_positions
        .iter()
        .chain(target_positions)
        .flatten()
        .all(|value| value.is_finite())
    {
        return Err(InferenceError::InvalidSettings(
            "source and target positions must be finite",
        ));
    }
    Ok(())
}

fn uniform_weights(count: usize) -> Vec<f64> {
    vec![1.0; count]
}

fn affine_candidate(
    source_positions: &[[f32; 3]],
    target_positions: &[[f32; 3]],
    weights: &[f64],
    affine: AffineOperator,
    diagnostics: ProgramOperatorDiagnostics,
) -> FittedOperatorCandidate {
    let weighted_error_before =
        weighted_sum_squared_distance(source_positions, target_positions, weights);
    let raw_error_before = sum_squared_distance(source_positions, target_positions);
    let reconstructed = apply_affine_to_positions(source_positions, affine.matrix_row_major_4x4);
    let weighted_error_after =
        weighted_sum_squared_distance(&reconstructed, target_positions, weights);
    let raw_error_after = sum_squared_distance(&reconstructed, target_positions);
    let semantic_parameter_count = diagnostics.semantic_parameter_count();
    let semantic_metadata_bytes = diagnostics.semantic_metadata_bytes();
    let operator = ProgramOperator::Affine(affine);
    let stable_candidate_id = stable_candidate_id(&operator);
    FittedOperatorCandidate {
        operator,
        diagnostics,
        cumulative_positions: reconstructed,
        weighted_error_before,
        weighted_error_after,
        raw_error_before,
        raw_error_after,
        weighted_explained_fraction: explained_fraction(
            weighted_error_before,
            weighted_error_after,
        ),
        raw_explained_fraction: explained_fraction(raw_error_before, raw_error_after),
        semantic_parameter_count,
        semantic_metadata_bytes,
        stable_candidate_id,
        fitting_diagnostics: FittingDiagnostics {
            generator: "deterministic_affine_fit".to_owned(),
            ..FittingDiagnostics::default()
        },
    }
}

fn stable_candidate_id(operator: &ProgramOperator) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    fn update(hash: &mut u64, bytes: &[u8]) {
        for byte in bytes {
            *hash ^= u64::from(*byte);
            *hash = hash.wrapping_mul(FNV_PRIME);
        }
    }

    fn update_f32(hash: &mut u64, value: f32) {
        update(hash, &value.to_bits().to_le_bytes());
    }

    let mut hash = FNV_OFFSET;
    match operator {
        ProgramOperator::Affine(affine) => {
            update(&mut hash, b"affine");
            update(
                &mut hash,
                match affine.semantic_family {
                    AffineSemanticFamily::GeneralAffine => b"general_affine",
                    AffineSemanticFamily::Translation => b"translation",
                    AffineSemanticFamily::RigidTransform => b"rigid_transform",
                    AffineSemanticFamily::SimilarityTransform => b"similarity_transform",
                },
            );
            for value in affine.matrix_row_major_4x4 {
                update_f32(&mut hash, value);
            }
        }
        ProgramOperator::Bend(parameters) => {
            update(&mut hash, b"bend");
            for value in parameters.origin {
                update_f32(&mut hash, value);
            }
            for value in parameters.longitudinal_axis {
                update_f32(&mut hash, value);
            }
            for value in parameters.bend_direction {
                update_f32(&mut hash, value);
            }
            update_f32(&mut hash, parameters.angle_radians);
            update_f32(&mut hash, parameters.interval_start);
            update_f32(&mut hash, parameters.interval_end);
        }
    }
    format!("candidate-{hash:016x}")
}

/// Validates ordered program search settings.
pub fn validate_program_search_settings(
    settings: &ProgramSearchSettings,
) -> Result<(), InferenceError> {
    if settings.maximum_explanatory_depth > 2 {
        return Err(InferenceError::ExplanatoryDepthLimitExceeded {
            limit: 2,
            actual: settings.maximum_explanatory_depth,
        });
    }
    if settings.maximum_total_programs == 0 {
        return Err(InferenceError::InvalidSettings(
            "maximum total programs must be greater than zero",
        ));
    }
    if !settings.minimum_weighted_explained_fraction.is_finite()
        || !(0.0..=1.0).contains(&settings.minimum_weighted_explained_fraction)
    {
        return Err(InferenceError::InvalidSettings(
            "minimum weighted explained fraction must be finite and between zero and one",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_search_depth_is_limited_to_two() {
        let settings = ProgramSearchSettings {
            maximum_explanatory_depth: 3,
            ..ProgramSearchSettings::default()
        };

        let error = validate_program_search_settings(&settings).unwrap_err();

        assert_eq!(
            error,
            InferenceError::ExplanatoryDepthLimitExceeded {
                limit: 2,
                actual: 3
            }
        );
    }

    #[test]
    fn bend_candidate_generation_is_deterministic() {
        let first = generate_bend_candidates(
            &[[0.0, 0.0, 0.0]],
            &[[1.0, 0.0, 0.0]],
            &BendFitSettings::default(),
            &ProgramSearchSettings {
                deterministic_seed: 123,
                ..ProgramSearchSettings::default()
            },
        )
        .unwrap();
        let second = generate_bend_candidates(
            &[[0.0, 0.0, 0.0]],
            &[[1.0, 0.0, 0.0]],
            &BendFitSettings::default(),
            &ProgramSearchSettings {
                deterministic_seed: 999,
                ..ProgramSearchSettings::default()
            },
        )
        .unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn affine_candidate_generation_returns_translation() {
        let candidates = generate_affine_candidates(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            &[[1.0, 2.0, 3.0], [2.0, 2.0, 3.0]],
            &ProgramSearchSettings::default(),
        )
        .unwrap();

        assert!(candidates.iter().any(|candidate| matches!(
            &candidate.diagnostics,
            ProgramOperatorDiagnostics::Translation { .. }
        )));
    }

    #[test]
    fn program_search_returns_affine_hypotheses() {
        let result = search_programs(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            &[[1.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
            &ProgramSearchSettings::default(),
        )
        .unwrap();

        assert_eq!(result.selected_hypothesis_index, Some(0));
        assert!(
            result
                .hypotheses
                .iter()
                .any(|hypothesis| !hypothesis.program.operators.is_empty())
        );
    }
}
