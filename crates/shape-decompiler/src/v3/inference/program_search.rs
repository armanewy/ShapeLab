//! Ordered explanatory program search.
//!
//! Search is intentionally shallow for the initial schema-3 program model:
//! empty, one affine, one bend, affine then bend, or bend then affine. The
//! terminal lossless correction is scored but is not counted as explanatory
//! depth.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::{
    AffineSemanticFamily, approximate_residual_cost, exact_residual_storage_size,
    explained_fraction, sum_squared_distance, weighted_centered_sum_squared_distance,
    weighted_sum_squared_distance,
};

use super::super::diagnostics::{
    DIAGNOSTICS_SCHEMA_VERSION_V4, InferenceDiagnosticsV4, ProgramCorrectionDiagnostics,
    ProgramDiagnosticsInput, ProgramHypothesisDiagnosticsV4, ProgramOperatorDiagnostics,
    StageDiagnosticsInput, build_program_diagnostics, build_stage_diagnostics,
    compare_program_hypotheses, default_scoring_policy_v4,
};
use super::super::program::{
    OperatorProgram, ProgramOperator, SemanticVerificationPolicy, SemanticVerificationReport,
    evaluate_operator,
};
use super::{
    AffineCandidateGenerator, BendCandidateGenerator, BendFitSettings, FittedOperatorCandidate,
    InferenceError, ProgramHypothesis, ProgramSearchResult, ProgramSearchSettings,
    generate_affine_candidates, generate_bend_candidates, uniform_weights,
    validate_bend_fit_settings, validate_position_pair, validate_program_search_settings,
};

const PACKAGE_SCHEMA_VERSION_V3: u32 = 3;
const SURFACE_WEIGHTING_MODEL: &str = "triangle_area_derived_vertex_weights";
const MEANINGFUL_STAGE_WEIGHTED_IMPROVEMENT: f64 = 1.0e-9;
const SUBSTANTIAL_SCORE_IMPROVEMENT: f64 = 1.0e-6;
const DIAGNOSTICS_BUILD_ERROR: &str = "schema-4 program diagnostics could not be built";

/// Default affine provider backed by the finalized Wave 2 affine interface.
#[derive(Debug, Copy, Clone, Default)]
pub struct DefaultAffineCandidateGenerator;

impl AffineCandidateGenerator for DefaultAffineCandidateGenerator {
    fn generate_candidates(
        &self,
        source_positions: &[[f32; 3]],
        target_positions: &[[f32; 3]],
        search_settings: &ProgramSearchSettings,
    ) -> Result<Vec<FittedOperatorCandidate>, InferenceError> {
        generate_affine_candidates(source_positions, target_positions, search_settings)
    }
}

/// Default bend provider backed by the finalized Wave 2 bend interface.
#[derive(Debug, Copy, Clone, Default)]
pub struct DefaultBendCandidateGenerator;

impl BendCandidateGenerator for DefaultBendCandidateGenerator {
    fn generate_candidates(
        &self,
        source_positions: &[[f32; 3]],
        target_positions: &[[f32; 3]],
        fit_settings: &BendFitSettings,
        search_settings: &ProgramSearchSettings,
    ) -> Result<Vec<FittedOperatorCandidate>, InferenceError> {
        generate_bend_candidates(
            source_positions,
            target_positions,
            fit_settings,
            search_settings,
        )
    }
}

/// Searches ordered explanatory operator programs with the default providers.
pub fn search_programs(
    source_positions: &[[f32; 3]],
    target_positions: &[[f32; 3]],
    settings: &ProgramSearchSettings,
) -> Result<ProgramSearchResult, InferenceError> {
    let affine_generator = DefaultAffineCandidateGenerator;
    let bend_generator = DefaultBendCandidateGenerator;
    search_programs_with_generators(
        source_positions,
        target_positions,
        &BendFitSettings::default(),
        settings,
        &affine_generator,
        &bend_generator,
    )
}

/// Searches ordered explanatory operator programs with injected providers.
///
/// This entry point is used by focused tests so program enumeration and scoring
/// do not depend on bend-fitting quality.
pub fn search_programs_with_generators(
    source_positions: &[[f32; 3]],
    target_positions: &[[f32; 3]],
    bend_fit_settings: &BendFitSettings,
    settings: &ProgramSearchSettings,
    affine_generator: &impl AffineCandidateGenerator,
    bend_generator: &impl BendCandidateGenerator,
) -> Result<ProgramSearchResult, InferenceError> {
    validate_position_pair(source_positions, target_positions)?;
    validate_program_search_settings(settings)?;
    validate_bend_fit_settings(bend_fit_settings)?;

    let weights = uniform_weights(source_positions.len());
    let context = SearchContext::new(source_positions, target_positions, weights);
    let mut builder = SearchBuilder::new(&context, settings);

    builder.consider_program(Vec::new())?;

    if settings.maximum_explanatory_depth >= 1 {
        let affine_candidates = collect_affine_candidates(
            affine_generator,
            source_positions,
            target_positions,
            settings,
            &context.weights,
            "root-affine",
        )?;
        let bend_candidates = collect_bend_candidates(
            bend_generator,
            source_positions,
            target_positions,
            bend_fit_settings,
            settings,
            &context.weights,
            "root-bend",
        )?;

        for candidate in &affine_candidates {
            builder.consider_program(vec![candidate.clone()])?;
        }
        for candidate in &bend_candidates {
            builder.consider_program(vec![candidate.clone()])?;
        }

        if settings.maximum_explanatory_depth >= 2 {
            for affine_candidate in &affine_candidates {
                let composed_bends = collect_bend_candidates(
                    bend_generator,
                    &affine_candidate.cumulative_positions,
                    target_positions,
                    bend_fit_settings,
                    settings,
                    &context.weights,
                    &format!("{}-then-bend", affine_candidate.candidate_id),
                )?;
                for bend_candidate in composed_bends {
                    builder.consider_program(vec![affine_candidate.clone(), bend_candidate])?;
                }
            }

            for bend_candidate in &bend_candidates {
                let composed_affines = collect_affine_candidates(
                    affine_generator,
                    &bend_candidate.cumulative_positions,
                    target_positions,
                    settings,
                    &context.weights,
                    &format!("{}-then-affine", bend_candidate.candidate_id),
                )?;
                for affine_candidate in composed_affines {
                    builder.consider_program(vec![bend_candidate.clone(), affine_candidate])?;
                }
            }
        }
    }

    Ok(builder.finish())
}

#[derive(Debug)]
struct SearchContext<'a> {
    source_positions: &'a [[f32; 3]],
    target_positions: &'a [[f32; 3]],
    weights: Vec<f64>,
    raw_identity_error: f64,
    weighted_identity_error: f64,
    error_normalization_scale: f64,
    literal_size_bytes: usize,
    scoring_policy: super::super::diagnostics::InferenceScoringPolicyV4,
}

impl<'a> SearchContext<'a> {
    fn new(
        source_positions: &'a [[f32; 3]],
        target_positions: &'a [[f32; 3]],
        weights: Vec<f64>,
    ) -> Self {
        let raw_identity_error = sum_squared_distance(source_positions, target_positions);
        let weighted_identity_error =
            weighted_sum_squared_distance(source_positions, target_positions, &weights);
        let error_normalization_scale =
            weighted_centered_sum_squared_distance(source_positions, &weights)
                .max(weighted_centered_sum_squared_distance(
                    target_positions,
                    &weights,
                ))
                .max(f64::EPSILON);
        Self {
            source_positions,
            target_positions,
            weights,
            raw_identity_error,
            weighted_identity_error,
            error_normalization_scale,
            literal_size_bytes: source_positions.len() * 3 * std::mem::size_of::<f32>(),
            scoring_policy: default_scoring_policy_v4(),
        }
    }
}

#[derive(Debug, Clone)]
struct StageCandidate {
    candidate_id: String,
    operator_key: String,
    source_order: usize,
    candidate: FittedOperatorCandidate,
    cumulative_positions: Vec<[f32; 3]>,
}

#[derive(Debug)]
struct ProgramEntry {
    program_key: String,
    candidate_ids: Vec<String>,
    ordinal: usize,
    program: OperatorProgram,
    candidates: Vec<FittedOperatorCandidate>,
    diagnostics: ProgramHypothesisDiagnosticsV4,
    all_stages_meaningful: bool,
}

struct SearchBuilder<'a, 'b> {
    context: &'a SearchContext<'b>,
    settings: &'a ProgramSearchSettings,
    seen_programs: BTreeSet<String>,
    entries: Vec<ProgramEntry>,
    next_ordinal: usize,
    best_all_meaningful_score: f64,
}

impl<'a, 'b> SearchBuilder<'a, 'b> {
    fn new(context: &'a SearchContext<'b>, settings: &'a ProgramSearchSettings) -> Self {
        Self {
            context,
            settings,
            seen_programs: BTreeSet::new(),
            entries: Vec::new(),
            next_ordinal: 0,
            best_all_meaningful_score: f64::INFINITY,
        }
    }

    fn consider_program(&mut self, stages: Vec<StageCandidate>) -> Result<(), InferenceError> {
        let program_key = program_key_for_stages(&stages);
        if self.seen_programs.contains(&program_key) {
            return Ok(());
        }

        let Some(entry) = build_program_entry(
            self.context,
            self.settings,
            stages,
            program_key.clone(),
            self.next_ordinal,
            self.best_all_meaningful_score,
        )?
        else {
            return Ok(());
        };

        self.seen_programs.insert(program_key);
        self.next_ordinal += 1;
        if entry.all_stages_meaningful {
            self.best_all_meaningful_score = self
                .best_all_meaningful_score
                .min(entry.diagnostics.score.total_component_sum);
        }
        self.entries.push(entry);
        Ok(())
    }

    fn finish(mut self) -> ProgramSearchResult {
        self.entries.sort_by(compare_entries);
        self.entries.truncate(self.settings.maximum_total_programs);

        for (index, entry) in self.entries.iter_mut().enumerate() {
            entry.diagnostics.selected = index == 0;
            entry.diagnostics.rejection_reason = (index != 0)
                .then(|| "higher score than selected ordered program hypothesis".to_owned());
        }

        let selected_hypothesis_index = (!self.entries.is_empty()).then_some(0);
        let hypotheses = self
            .entries
            .iter()
            .map(|entry| ProgramHypothesis {
                program: entry.program.clone(),
                candidates: entry.candidates.clone(),
                weighted_explained_fraction: entry.diagnostics.weighted_explained_fraction,
                raw_explained_fraction: entry.diagnostics.raw_explained_fraction,
                total_score: entry.diagnostics.score.total_component_sum,
            })
            .collect::<Vec<_>>();
        let diagnostics = InferenceDiagnosticsV4 {
            diagnostics_schema_version: DIAGNOSTICS_SCHEMA_VERSION_V4,
            package_schema_version: PACKAGE_SCHEMA_VERSION_V3,
            surface_weighting: SURFACE_WEIGHTING_MODEL.to_owned(),
            raw_identity_error: self.context.raw_identity_error,
            weighted_identity_error: self.context.weighted_identity_error,
            scoring_policy: self.context.scoring_policy.clone(),
            selected_program_hypothesis_index: selected_hypothesis_index.unwrap_or(0),
            program_hypotheses: self
                .entries
                .iter()
                .map(|entry| entry.diagnostics.clone())
                .collect(),
        };

        ProgramSearchResult {
            hypotheses,
            selected_hypothesis_index,
            diagnostics: Some(diagnostics),
        }
    }
}

fn collect_affine_candidates(
    generator: &impl AffineCandidateGenerator,
    source_positions: &[[f32; 3]],
    target_positions: &[[f32; 3]],
    settings: &ProgramSearchSettings,
    weights: &[f64],
    id_prefix: &str,
) -> Result<Vec<StageCandidate>, InferenceError> {
    let candidates = generator.generate_candidates(source_positions, target_positions, settings)?;
    Ok(retain_stage_candidates(
        candidates,
        CandidateKind::Affine,
        source_positions,
        target_positions,
        weights,
        id_prefix,
        None,
    ))
}

fn collect_bend_candidates(
    generator: &impl BendCandidateGenerator,
    source_positions: &[[f32; 3]],
    target_positions: &[[f32; 3]],
    bend_fit_settings: &BendFitSettings,
    settings: &ProgramSearchSettings,
    weights: &[f64],
    id_prefix: &str,
) -> Result<Vec<StageCandidate>, InferenceError> {
    let candidates = generator.generate_candidates(
        source_positions,
        target_positions,
        bend_fit_settings,
        settings,
    )?;
    Ok(retain_stage_candidates(
        candidates,
        CandidateKind::Bend,
        source_positions,
        target_positions,
        weights,
        id_prefix,
        Some(settings.maximum_bend_candidates),
    ))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum CandidateKind {
    Affine,
    Bend,
}

fn retain_stage_candidates(
    candidates: Vec<FittedOperatorCandidate>,
    kind: CandidateKind,
    source_positions: &[[f32; 3]],
    target_positions: &[[f32; 3]],
    weights: &[f64],
    id_prefix: &str,
    limit: Option<usize>,
) -> Vec<StageCandidate> {
    let mut seen = BTreeSet::new();
    let mut retained = candidates
        .into_iter()
        .enumerate()
        .filter_map(|(source_order, candidate)| {
            if candidate_kind(&candidate.operator) != kind {
                return None;
            }
            if !operator_matches_diagnostics(&candidate.operator, &candidate.diagnostics) {
                return None;
            }
            let cumulative_positions =
                evaluate_operator(&candidate.operator, source_positions).ok()?;
            if cumulative_positions.len() != source_positions.len()
                || !positions_are_finite(&cumulative_positions)
                || positions_bit_equal(source_positions, &cumulative_positions)
            {
                return None;
            }
            let operator_key = operator_key(&candidate.diagnostics);
            if !seen.insert(operator_key.clone()) {
                return None;
            }
            let weighted_error_before =
                weighted_sum_squared_distance(source_positions, target_positions, weights);
            let weighted_error_after =
                weighted_sum_squared_distance(&cumulative_positions, target_positions, weights);
            let raw_error_before = sum_squared_distance(source_positions, target_positions);
            let raw_error_after = sum_squared_distance(&cumulative_positions, target_positions);
            if ![
                weighted_error_before,
                weighted_error_after,
                raw_error_before,
                raw_error_after,
            ]
            .iter()
            .all(|value| value.is_finite())
            {
                return None;
            }

            let candidate_id = format!("{id_prefix}:{source_order:04}:{operator_key}");
            let stable_candidate_id = if candidate.stable_candidate_id.is_empty() {
                candidate_id.clone()
            } else {
                candidate.stable_candidate_id
            };
            Some(StageCandidate {
                candidate_id,
                operator_key,
                source_order,
                candidate: FittedOperatorCandidate {
                    operator: candidate.operator,
                    diagnostics: candidate.diagnostics,
                    cumulative_positions: cumulative_positions.clone(),
                    weighted_error_before,
                    weighted_error_after,
                    raw_error_before,
                    raw_error_after,
                    weighted_explained_fraction: explained_fraction(
                        weighted_error_before,
                        weighted_error_after,
                    ),
                    raw_explained_fraction: explained_fraction(raw_error_before, raw_error_after),
                    semantic_parameter_count: candidate.semantic_parameter_count,
                    semantic_metadata_bytes: candidate.semantic_metadata_bytes,
                    stable_candidate_id,
                    fitting_diagnostics: candidate.fitting_diagnostics,
                },
                cumulative_positions,
            })
        })
        .collect::<Vec<_>>();

    retained.sort_by(compare_stage_candidates);
    if let Some(limit) = limit {
        retained.truncate(limit);
    }
    retained
}

fn build_program_entry(
    context: &SearchContext<'_>,
    settings: &ProgramSearchSettings,
    stages: Vec<StageCandidate>,
    program_key: String,
    ordinal: usize,
    best_all_meaningful_score: f64,
) -> Result<Option<ProgramEntry>, InferenceError> {
    let final_positions = stages
        .last()
        .map(|stage| stage.cumulative_positions.as_slice())
        .unwrap_or(context.source_positions);
    let weighted_final_error =
        weighted_sum_squared_distance(final_positions, context.target_positions, &context.weights);
    let raw_final_error = sum_squared_distance(final_positions, context.target_positions);
    if !weighted_final_error.is_finite() || !raw_final_error.is_finite() {
        return Ok(None);
    }

    if !stages.is_empty() {
        if weighted_final_error >= context.weighted_identity_error {
            return Ok(None);
        }
        let weighted_explained_fraction =
            explained_fraction(context.weighted_identity_error, weighted_final_error);
        if weighted_explained_fraction + f64::EPSILON < settings.minimum_weighted_explained_fraction
        {
            return Ok(None);
        }
    }

    let stage_diagnostics = stages
        .iter()
        .enumerate()
        .map(|(stage_index, stage)| {
            build_stage_diagnostics(StageDiagnosticsInput {
                stage_index,
                operator: stage.candidate.diagnostics.clone(),
                weighted_error_before: stage.candidate.weighted_error_before,
                weighted_error_after: stage.candidate.weighted_error_after,
                raw_error_before: stage.candidate.raw_error_before,
                raw_error_after: stage.candidate.raw_error_after,
                semantic_verification_policy: SemanticVerificationPolicy::default(),
                semantic_verification_report: SemanticVerificationReport::default(),
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| InferenceError::InvalidSettings(DIAGNOSTICS_BUILD_ERROR))?;

    let exact_residual_bytes =
        exact_residual_storage_size(final_positions, context.target_positions);
    let diagnostics = build_program_diagnostics(ProgramDiagnosticsInput {
        operators: stages
            .iter()
            .map(|stage| stage.candidate.diagnostics.clone())
            .collect(),
        stages: stage_diagnostics,
        final_correction: ProgramCorrectionDiagnostics {
            corrected_vertex_count: corrected_vertex_count(
                final_positions,
                context.target_positions,
            ),
            exact_residual_bytes,
            weighted_error_before: weighted_final_error,
            weighted_error_after: 0.0,
            raw_error_before: raw_final_error,
            raw_error_after: 0.0,
        },
        raw_identity_error: context.raw_identity_error,
        weighted_identity_error: context.weighted_identity_error,
        error_normalization_scale: context.error_normalization_scale,
        literal_size_bytes: context.literal_size_bytes,
        approximate_residual_coverage: approximate_residual_cost(
            final_positions,
            context.target_positions,
            &context.weights,
        ),
        scoring_policy: context.scoring_policy.clone(),
        selected: false,
        rejection_reason: None,
    })
    .map_err(|_| InferenceError::InvalidSettings(DIAGNOSTICS_BUILD_ERROR))?;

    let all_stages_meaningful = stages.iter().all(stage_has_meaningful_improvement);
    if !all_stages_meaningful
        && diagnostics.score.total_component_sum + SUBSTANTIAL_SCORE_IMPROVEMENT
            >= best_all_meaningful_score
    {
        return Ok(None);
    }

    Ok(Some(ProgramEntry {
        program_key,
        candidate_ids: stages
            .iter()
            .map(|stage| stage.candidate_id.clone())
            .collect(),
        ordinal,
        program: OperatorProgram {
            operators: stages
                .iter()
                .map(|stage| stage.candidate.operator)
                .collect(),
        },
        candidates: stages.iter().map(|stage| stage.candidate.clone()).collect(),
        diagnostics,
        all_stages_meaningful,
    }))
}

fn compare_entries(left: &ProgramEntry, right: &ProgramEntry) -> Ordering {
    compare_program_hypotheses(&left.diagnostics, &right.diagnostics)
        .then_with(|| left.program_key.cmp(&right.program_key))
        .then_with(|| left.candidate_ids.cmp(&right.candidate_ids))
        .then_with(|| left.ordinal.cmp(&right.ordinal))
}

fn compare_stage_candidates(left: &StageCandidate, right: &StageCandidate) -> Ordering {
    left.candidate
        .weighted_error_after
        .total_cmp(&right.candidate.weighted_error_after)
        .then_with(|| {
            left.candidate
                .raw_error_after
                .total_cmp(&right.candidate.raw_error_after)
        })
        .then_with(|| left.operator_key.cmp(&right.operator_key))
        .then_with(|| left.source_order.cmp(&right.source_order))
}

fn stage_has_meaningful_improvement(stage: &StageCandidate) -> bool {
    stage.candidate.weighted_error_before - stage.candidate.weighted_error_after
        > MEANINGFUL_STAGE_WEIGHTED_IMPROVEMENT
}

fn program_key_for_stages(stages: &[StageCandidate]) -> String {
    if stages.is_empty() {
        return "empty".to_owned();
    }
    stages
        .iter()
        .map(|stage| stage.operator_key.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn operator_matches_diagnostics(
    operator: &ProgramOperator,
    diagnostics: &ProgramOperatorDiagnostics,
) -> bool {
    matches!(
        (operator, diagnostics),
        (
            ProgramOperator::Affine(super::super::program::AffineOperator {
                semantic_family: AffineSemanticFamily::Translation,
                ..
            }),
            ProgramOperatorDiagnostics::Translation { .. },
        ) | (
            ProgramOperator::Affine(super::super::program::AffineOperator {
                semantic_family: AffineSemanticFamily::RigidTransform,
                ..
            }),
            ProgramOperatorDiagnostics::RigidTransform { .. },
        ) | (
            ProgramOperator::Affine(super::super::program::AffineOperator {
                semantic_family: AffineSemanticFamily::SimilarityTransform,
                ..
            }),
            ProgramOperatorDiagnostics::SimilarityTransform { .. },
        ) | (
            ProgramOperator::Affine(super::super::program::AffineOperator {
                semantic_family: AffineSemanticFamily::GeneralAffine,
                ..
            }),
            ProgramOperatorDiagnostics::GeneralAffine { .. },
        ) | (
            ProgramOperator::Bend(_),
            ProgramOperatorDiagnostics::Bend { .. },
        )
    )
}

fn candidate_kind(operator: &ProgramOperator) -> CandidateKind {
    match operator {
        ProgramOperator::Affine(_) => CandidateKind::Affine,
        ProgramOperator::Bend(_) => CandidateKind::Bend,
    }
}

fn operator_key(diagnostics: &ProgramOperatorDiagnostics) -> String {
    match diagnostics {
        ProgramOperatorDiagnostics::Translation { translation } => {
            format!("affine:translation:{}", f32_array_key(translation))
        }
        ProgramOperatorDiagnostics::RigidTransform {
            translation,
            rotation_row_major_3x3,
        } => format!(
            "affine:rigid:{}:{}",
            f32_array_key(translation),
            f32_array_key(rotation_row_major_3x3)
        ),
        ProgramOperatorDiagnostics::SimilarityTransform {
            translation,
            rotation_row_major_3x3,
            uniform_scale,
        } => format!(
            "affine:similarity:{}:{}:{:08x}",
            f32_array_key(translation),
            f32_array_key(rotation_row_major_3x3),
            uniform_scale.to_bits()
        ),
        ProgramOperatorDiagnostics::GeneralAffine {
            matrix_row_major_4x4,
        } => format!("affine:general:{}", f32_array_key(matrix_row_major_4x4)),
        ProgramOperatorDiagnostics::Bend { parameters } => format!(
            "bend:{}:{}:{}:{:08x}:{:08x}:{:08x}",
            f32_array_key(&parameters.origin),
            f32_array_key(&parameters.longitudinal_axis),
            f32_array_key(&parameters.bend_direction),
            parameters.angle_radians.to_bits(),
            parameters.interval_start.to_bits(),
            parameters.interval_end.to_bits()
        ),
    }
}

fn f32_array_key(values: &[f32]) -> String {
    values
        .iter()
        .map(|value| format!("{:08x}", value.to_bits()))
        .collect::<Vec<_>>()
        .join(",")
}

fn positions_are_finite(positions: &[[f32; 3]]) -> bool {
    positions.iter().flatten().all(|value| value.is_finite())
}

fn positions_bit_equal(left: &[[f32; 3]], right: &[[f32; 3]]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.iter()
                .zip(right)
                .all(|(left, right)| left.to_bits() == right.to_bits())
        })
}

fn corrected_vertex_count(reconstructed: &[[f32; 3]], target: &[[f32; 3]]) -> usize {
    reconstructed
        .iter()
        .zip(target)
        .filter(|(left, right)| {
            left.iter()
                .zip(*right)
                .any(|(left, right)| left.to_bits() != right.to_bits())
        })
        .count()
}
