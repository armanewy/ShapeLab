//! Diagnostics schema 4 contracts for ordered operator programs.

use serde::{Deserialize, Serialize};

use super::bend::BendParameters;

/// Diagnostics schema version for ordered schema-3 program diagnostics.
pub const DIAGNOSTICS_SCHEMA_VERSION_V4: u32 = 4;

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

/// Per-stage score and replay diagnostics for a program hypothesis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageDiagnostics {
    /// Zero-based stage index.
    pub stage_index: usize,
    /// Triangle-area-weighted error before this stage.
    pub weighted_error_before: f64,
    /// Triangle-area-weighted error after this stage.
    pub weighted_error_after: f64,
    /// Raw unweighted error before this stage.
    pub raw_error_before: f64,
    /// Raw unweighted error after this stage.
    pub raw_error_after: f64,
    /// Additional weighted explained fraction contributed by this stage.
    pub weighted_explained_increment: f64,
    /// Maximum semantic-to-baked Euclidean error for this stage.
    pub semantic_to_baked_max_error: f64,
    /// RMS semantic-to-baked Euclidean error for this stage.
    pub semantic_to_baked_rms_error: f64,
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

/// Score breakdown for one ordered program hypothesis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramHypothesisDiagnosticsV4 {
    /// Ordered explanatory operators before the terminal correction.
    pub operators: Vec<ProgramOperatorDiagnostics>,
    /// Per-stage diagnostics for the explanatory operators.
    pub stages: Vec<StageDiagnostics>,
    /// Terminal exact correction summary.
    pub final_correction: ProgramCorrectionDiagnostics,
    /// Surface-weighted displacement fraction explained by the program.
    pub weighted_explained_fraction: f64,
    /// Raw unweighted displacement fraction explained by the program.
    pub raw_explained_fraction: f64,
    /// Normalized geometric error contribution.
    pub normalized_geometric_error_cost: f64,
    /// Semantic parameter contribution.
    pub parameter_cost: f64,
    /// Per-operator program overhead contribution.
    pub program_overhead_cost: f64,
    /// Exact residual byte contribution.
    pub exact_residual_cost: f64,
    /// Total model-selection score.
    pub total_score: f64,
    /// Whether this hypothesis was selected.
    pub selected: bool,
    /// Why this hypothesis was rejected, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}

/// Scoring constants for schema-4 inference diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InferenceScoringPolicyV4 {
    /// Human-readable scoring model identifier.
    pub model: String,
    /// Weight applied to normalized geometric error.
    pub geometric_error_weight: f64,
    /// Weight applied to semantic parameter count.
    pub parameter_weight: f64,
    /// Fixed overhead applied per explanatory operator in a program.
    pub per_operator_program_overhead: f64,
    /// Weight applied to exact residual byte cost.
    pub exact_residual_weight: f64,
    /// Minimum weighted explained fraction required for explanatory programs.
    pub minimum_weighted_explained_fraction: f64,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_explanatory_operator_hypothesis_roundtrips() {
        let diagnostics = InferenceDiagnosticsV4 {
            diagnostics_schema_version: DIAGNOSTICS_SCHEMA_VERSION_V4,
            package_schema_version: 3,
            surface_weighting: "triangle_area_derived_vertex_weights".to_owned(),
            raw_identity_error: 1.0,
            weighted_identity_error: 1.0,
            scoring_policy: InferenceScoringPolicyV4 {
                model: "schema4-contract-test".to_owned(),
                geometric_error_weight: 1.0,
                parameter_weight: 0.1,
                per_operator_program_overhead: 0.01,
                exact_residual_weight: 0.001,
                minimum_weighted_explained_fraction: 0.0,
            },
            selected_program_hypothesis_index: 0,
            program_hypotheses: vec![ProgramHypothesisDiagnosticsV4 {
                operators: Vec::new(),
                stages: Vec::new(),
                final_correction: ProgramCorrectionDiagnostics {
                    corrected_vertex_count: 4,
                    exact_residual_bytes: 48,
                    weighted_error_before: 1.0,
                    weighted_error_after: 0.0,
                    raw_error_before: 1.0,
                    raw_error_after: 0.0,
                },
                weighted_explained_fraction: 0.0,
                raw_explained_fraction: 0.0,
                normalized_geometric_error_cost: 1.0,
                parameter_cost: 0.0,
                program_overhead_cost: 0.0,
                exact_residual_cost: 0.048,
                total_score: 1.048,
                selected: true,
                rejection_reason: None,
            }],
        };

        let json = serde_json::to_string_pretty(&diagnostics).unwrap();
        let decoded: InferenceDiagnosticsV4 = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, diagnostics);
        assert!(decoded.program_hypotheses[0].operators.is_empty());
    }
}
