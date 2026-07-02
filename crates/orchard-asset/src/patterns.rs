//! Pattern contract shells for deterministic repetition.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{PartInstanceId, PatternContract, PatternId, PatternType};

/// Maximum occurrence count supported by the first deterministic evaluator.
pub const PATTERN_EVALUATION_MAX_COUNT: u32 = 10_000;

/// Axis for V0 linear pattern evaluation.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternAxis {
    /// Positive X axis.
    #[default]
    X,
    /// Positive Y axis.
    Y,
    /// Positive Z axis.
    Z,
}

impl PatternAxis {
    /// Convert an axis and scalar offset into a 3D translation vector.
    #[must_use]
    pub const fn offset_vector(self, offset: f32) -> [f32; 3] {
        match self {
            Self::X => [offset, 0.0, 0.0],
            Self::Y => [0.0, offset, 0.0],
            Self::Z => [0.0, 0.0, offset],
        }
    }
}

/// Generated occurrence ID policy.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneratedIdPolicy {
    /// Deterministic IDs from pattern ID plus occurrence index.
    #[default]
    PatternOccurrenceIndex,
}

/// One evaluated pattern occurrence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternOccurrence {
    /// Deterministic occurrence ID.
    pub occurrence_id: String,
    /// Source pattern ID.
    pub pattern_id: PatternId,
    /// Source instance, when supplied by the pattern contract.
    pub source_instance: Option<PartInstanceId>,
    /// Zero-based occurrence index.
    pub index: u32,
    /// Offset from the source instance origin.
    pub offset: [f32; 3],
}

/// Pattern evaluation report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternEvaluationReport {
    /// Source pattern ID.
    pub pattern_id: PatternId,
    /// Number of generated occurrences.
    pub generated_occurrence_count: u32,
    /// Deterministic occurrence IDs.
    pub occurrence_ids: Vec<String>,
    /// Export instancing policy copied from the contract.
    pub export_instancing_policy: PatternExportInstancingPolicy,
    /// V0 never claims export instancing support.
    pub export_instancing_enabled: bool,
}

/// Result of deterministic pattern evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternEvaluation {
    /// Generated occurrences.
    pub occurrences: Vec<PatternOccurrence>,
    /// Evaluation report.
    pub report: PatternEvaluationReport,
}

/// Pattern evaluation rejection.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PatternEvaluationError {
    /// V0 only evaluates linear patterns.
    #[error("unsupported pattern type")]
    UnsupportedPatternType,
    /// Pattern count is missing or invalid.
    #[error("invalid pattern count")]
    InvalidCount,
    /// Linear spacing must be finite and non-negative.
    #[error("invalid pattern spacing")]
    InvalidSpacing,
}

/// Count policy for a pattern.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternCountPolicy {
    /// Count is not yet authored.
    #[default]
    Unspecified,
    /// Exact finite count.
    Exact(u32),
    /// Bounded finite count range.
    Range {
        /// Minimum count.
        minimum: u32,
        /// Maximum count.
        maximum: u32,
    },
}

/// Density policy for future pattern tools.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternDensityPolicy {
    /// Exact density.
    Exact(f32),
    /// Bounded density range.
    Range {
        /// Minimum density.
        minimum: f32,
        /// Maximum density.
        maximum: f32,
    },
}

/// Export instancing policy shell.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternExportInstancingPolicy {
    /// Export instancing has not been decided.
    #[default]
    Pending,
    /// Do not claim export instancing.
    Disabled,
    /// Requested for later proof, not active yet.
    PreserveInstancesWhenSupported,
}

/// Evaluate a V0 linear pattern contract deterministically.
pub fn evaluate_linear_pattern_contract(
    pattern: &PatternContract,
) -> Result<PatternEvaluation, PatternEvaluationError> {
    if pattern.pattern_type != PatternType::Linear {
        return Err(PatternEvaluationError::UnsupportedPatternType);
    }
    let count = pattern_count(pattern)?;
    let spacing = pattern.spacing.unwrap_or(0.0);
    if !spacing.is_finite() || spacing < 0.0 {
        return Err(PatternEvaluationError::InvalidSpacing);
    }
    let axis = pattern.linear_axis.unwrap_or_default();
    let occurrences = (0..count)
        .map(|index| PatternOccurrence {
            occurrence_id: generated_occurrence_id(pattern.id, index, pattern.generated_id_policy),
            pattern_id: pattern.id,
            source_instance: pattern.source_instance,
            index,
            offset: axis.offset_vector(spacing * index as f32),
        })
        .collect::<Vec<_>>();
    let report = PatternEvaluationReport {
        pattern_id: pattern.id,
        generated_occurrence_count: count,
        occurrence_ids: occurrences
            .iter()
            .map(|occurrence| occurrence.occurrence_id.clone())
            .collect(),
        export_instancing_policy: pattern.export_instancing,
        export_instancing_enabled: false,
    };
    Ok(PatternEvaluation {
        occurrences,
        report,
    })
}

fn pattern_count(pattern: &PatternContract) -> Result<u32, PatternEvaluationError> {
    let count = match (pattern.count, pattern.count_policy) {
        (Some(count), _) => count,
        (None, PatternCountPolicy::Exact(count)) => count,
        (None, PatternCountPolicy::Range { minimum, .. }) => minimum,
        (None, PatternCountPolicy::Unspecified) => {
            return Err(PatternEvaluationError::InvalidCount);
        }
    };
    if !(1..=PATTERN_EVALUATION_MAX_COUNT).contains(&count) {
        return Err(PatternEvaluationError::InvalidCount);
    }
    Ok(count)
}

fn generated_occurrence_id(pattern_id: PatternId, index: u32, policy: GeneratedIdPolicy) -> String {
    match policy {
        GeneratedIdPolicy::PatternOccurrenceIndex => {
            format!("pattern-{}-occurrence-{index:04}", pattern_id.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PartInstanceId, PatternContract, PatternCountPolicy};

    #[test]
    fn pattern_evaluation_valid_linear_pattern_is_deterministic() {
        let pattern = linear_pattern();

        let first = evaluate_linear_pattern_contract(&pattern).expect("evaluates");
        let second = evaluate_linear_pattern_contract(&pattern).expect("evaluates");

        assert_eq!(first, second);
        assert_eq!(first.occurrences.len(), 3);
        assert_eq!(
            first.report.occurrence_ids,
            vec![
                "pattern-7-occurrence-0000",
                "pattern-7-occurrence-0001",
                "pattern-7-occurrence-0002"
            ]
        );
        assert_eq!(first.occurrences[2].offset, [0.5, 0.0, 0.0]);
        assert!(!first.report.export_instancing_enabled);
    }

    #[test]
    fn pattern_evaluation_rejects_invalid_count() {
        let mut pattern = linear_pattern();
        pattern.count = Some(0);

        let error = evaluate_linear_pattern_contract(&pattern).expect_err("count rejected");

        assert_eq!(error, PatternEvaluationError::InvalidCount);
    }

    #[test]
    fn pattern_evaluation_rejects_invalid_spacing() {
        let mut pattern = linear_pattern();
        pattern.spacing = Some(f32::NAN);

        let error = evaluate_linear_pattern_contract(&pattern).expect_err("spacing rejected");

        assert_eq!(error, PatternEvaluationError::InvalidSpacing);
    }

    #[test]
    fn pattern_evaluation_fixtures_parse_and_report_expected_blockers() {
        let valid: PatternContract = serde_json::from_str(include_str!(
            "../../../fixtures/orchard-asset/valid_linear_pattern_contract_v0.json"
        ))
        .expect("valid fixture parses");
        let invalid_count: PatternContract = serde_json::from_str(include_str!(
            "../../../fixtures/orchard-asset/invalid_linear_pattern_count_v0.json"
        ))
        .expect("invalid count fixture parses");
        let invalid_spacing: PatternContract = serde_json::from_str(include_str!(
            "../../../fixtures/orchard-asset/invalid_linear_pattern_spacing_v0.json"
        ))
        .expect("invalid spacing fixture parses");

        assert_eq!(
            evaluate_linear_pattern_contract(&valid)
                .expect("valid fixture evaluates")
                .report
                .generated_occurrence_count,
            3
        );
        assert_eq!(
            evaluate_linear_pattern_contract(&invalid_count).expect_err("count rejected"),
            PatternEvaluationError::InvalidCount
        );
        assert_eq!(
            evaluate_linear_pattern_contract(&invalid_spacing).expect_err("spacing rejected"),
            PatternEvaluationError::InvalidSpacing
        );
    }

    fn linear_pattern() -> PatternContract {
        PatternContract {
            id: PatternId(7),
            pattern_type: PatternType::Linear,
            source_instance: Some(PartInstanceId(2)),
            count: Some(3),
            label: "Three repeat proof".to_owned(),
            count_policy: PatternCountPolicy::Exact(3),
            density_policy: None,
            export_instancing: PatternExportInstancingPolicy::Pending,
            linear_axis: Some(PatternAxis::X),
            spacing: Some(0.25),
            generated_id_policy: GeneratedIdPolicy::PatternOccurrenceIndex,
        }
    }
}
