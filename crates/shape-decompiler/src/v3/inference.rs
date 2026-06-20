//! Future schema-3 candidate generation and ordered program search contracts.
//!
//! This module intentionally does not infer bend operators yet. The stubs
//! validate settings and return deterministic empty/baseline results so later
//! waves can implement candidate generation behind stable contracts.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::diagnostics::{InferenceDiagnosticsV4, ProgramOperatorDiagnostics};
use super::program::{OperatorProgram, ProgramOperator};

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
            maximum_refinement_iterations: 0,
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

/// One fitted semantic operator candidate available to program search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FittedOperatorCandidate {
    /// Semantic operator represented by this candidate.
    pub operator: ProgramOperator,
    /// Diagnostics-ready semantic parameter record.
    pub diagnostics: ProgramOperatorDiagnostics,
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

/// Generates bend candidates.
///
/// This stub validates settings and returns no candidates until bend inference
/// is implemented. It does not use randomness; the deterministic seed is
/// reserved for future implementations.
pub fn generate_bend_candidates(
    _source_positions: &[[f32; 3]],
    _target_positions: &[[f32; 3]],
    fit_settings: &BendFitSettings,
    search_settings: &ProgramSearchSettings,
) -> Result<Vec<FittedOperatorCandidate>, InferenceError> {
    validate_bend_fit_settings(fit_settings)?;
    validate_program_search_settings(search_settings)?;
    Ok(Vec::new())
}

/// Searches ordered explanatory operator programs.
///
/// This stub validates settings and returns the deterministic lossless-only
/// baseline hypothesis represented by an empty explanatory program.
pub fn search_programs(
    _source_positions: &[[f32; 3]],
    _target_positions: &[[f32; 3]],
    settings: &ProgramSearchSettings,
) -> Result<ProgramSearchResult, InferenceError> {
    validate_program_search_settings(settings)?;
    Ok(ProgramSearchResult {
        hypotheses: vec![ProgramHypothesis {
            program: OperatorProgram {
                operators: Vec::new(),
            },
            candidates: Vec::new(),
            weighted_explained_fraction: 0.0,
            raw_explained_fraction: 0.0,
            total_score: 0.0,
        }],
        selected_hypothesis_index: Some(0),
        diagnostics: None,
    })
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
    fn bend_candidate_stub_is_deterministic_and_empty() {
        let candidates = generate_bend_candidates(
            &[[0.0, 0.0, 0.0]],
            &[[1.0, 0.0, 0.0]],
            &BendFitSettings::default(),
            &ProgramSearchSettings {
                deterministic_seed: 123,
                ..ProgramSearchSettings::default()
            },
        )
        .unwrap();

        assert!(candidates.is_empty());
    }

    #[test]
    fn program_search_stub_returns_lossless_only_baseline() {
        let result = search_programs(
            &[[0.0, 0.0, 0.0]],
            &[[1.0, 0.0, 0.0]],
            &ProgramSearchSettings::default(),
        )
        .unwrap();

        assert_eq!(result.selected_hypothesis_index, Some(0));
        assert!(result.hypotheses[0].program.operators.is_empty());
    }
}
