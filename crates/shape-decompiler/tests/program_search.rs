use shape_decompiler::AffineSemanticFamily;
use shape_decompiler::v3::bend::{BendParameters, evaluate_bend};
use shape_decompiler::v3::diagnostics::{
    DIAGNOSTICS_SCHEMA_VERSION_V4, ProgramOperatorDiagnostics,
};
use shape_decompiler::v3::inference::program_search::search_programs_with_generators;
use shape_decompiler::v3::inference::{
    AffineCandidateGenerator, BendCandidateGenerator, BendFitSettings, FittedOperatorCandidate,
    FittingDiagnostics, InferenceError, ProgramSearchResult, ProgramSearchSettings,
};
use shape_decompiler::v3::program::{AffineOperator, ProgramOperator, evaluate_operator};

#[derive(Debug, Clone)]
struct CandidateRule {
    source_positions: Vec<[f32; 3]>,
    candidates: Vec<ProgramOperator>,
}

#[derive(Debug, Clone, Default)]
struct FakeAffineGenerator {
    rules: Vec<CandidateRule>,
}

impl FakeAffineGenerator {
    fn with_rule(
        mut self,
        source_positions: Vec<[f32; 3]>,
        candidates: Vec<ProgramOperator>,
    ) -> Self {
        self.rules.push(CandidateRule {
            source_positions,
            candidates,
        });
        self
    }
}

impl AffineCandidateGenerator for FakeAffineGenerator {
    fn generate_candidates(
        &self,
        source_positions: &[[f32; 3]],
        _target_positions: &[[f32; 3]],
        _search_settings: &ProgramSearchSettings,
    ) -> Result<Vec<FittedOperatorCandidate>, InferenceError> {
        Ok(self
            .rules
            .iter()
            .find(|rule| positions_bit_equal(&rule.source_positions, source_positions))
            .map(|rule| {
                rule.candidates
                    .iter()
                    .copied()
                    .filter(|operator| matches!(operator, ProgramOperator::Affine(_)))
                    .map(fitted_candidate)
                    .collect()
            })
            .unwrap_or_default())
    }
}

#[derive(Debug, Clone, Default)]
struct FakeBendGenerator {
    rules: Vec<CandidateRule>,
}

impl FakeBendGenerator {
    fn with_rule(
        mut self,
        source_positions: Vec<[f32; 3]>,
        candidates: Vec<ProgramOperator>,
    ) -> Self {
        self.rules.push(CandidateRule {
            source_positions,
            candidates,
        });
        self
    }
}

impl BendCandidateGenerator for FakeBendGenerator {
    fn generate_candidates(
        &self,
        source_positions: &[[f32; 3]],
        _target_positions: &[[f32; 3]],
        _fit_settings: &BendFitSettings,
        _search_settings: &ProgramSearchSettings,
    ) -> Result<Vec<FittedOperatorCandidate>, InferenceError> {
        Ok(self
            .rules
            .iter()
            .find(|rule| positions_bit_equal(&rule.source_positions, source_positions))
            .map(|rule| {
                rule.candidates
                    .iter()
                    .copied()
                    .filter(|operator| matches!(operator, ProgramOperator::Bend(_)))
                    .map(fitted_candidate)
                    .collect()
            })
            .unwrap_or_default())
    }
}

fn run_search(
    source: &[[f32; 3]],
    target: &[[f32; 3]],
    affine_generator: &FakeAffineGenerator,
    bend_generator: &FakeBendGenerator,
    settings: ProgramSearchSettings,
) -> ProgramSearchResult {
    search_programs_with_generators(
        source,
        target,
        &BendFitSettings::default(),
        &settings,
        affine_generator,
        bend_generator,
    )
    .unwrap()
}

fn selected_shape(result: &ProgramSearchResult) -> Vec<&'static str> {
    result.hypotheses[result.selected_hypothesis_index.unwrap()]
        .program
        .operators
        .iter()
        .map(|operator| match operator {
            ProgramOperator::Affine(_) => "affine",
            ProgramOperator::Bend(_) => "bend",
        })
        .collect()
}

fn fitted_candidate(operator: ProgramOperator) -> FittedOperatorCandidate {
    let diagnostics = diagnostics_for_operator(operator);
    let semantic_parameter_count = diagnostics.semantic_parameter_count();
    let semantic_metadata_bytes = diagnostics.semantic_metadata_bytes();
    FittedOperatorCandidate {
        operator,
        diagnostics,
        cumulative_positions: Vec::new(),
        weighted_error_before: 0.0,
        weighted_error_after: 0.0,
        raw_error_before: 0.0,
        raw_error_after: 0.0,
        weighted_explained_fraction: 0.0,
        raw_explained_fraction: 0.0,
        semantic_parameter_count,
        semantic_metadata_bytes,
        stable_candidate_id: String::new(),
        fitting_diagnostics: FittingDiagnostics::default(),
    }
}

fn diagnostics_for_operator(operator: ProgramOperator) -> ProgramOperatorDiagnostics {
    match operator {
        ProgramOperator::Affine(affine) => match affine.semantic_family {
            AffineSemanticFamily::Translation => ProgramOperatorDiagnostics::Translation {
                translation: affine.translation.unwrap(),
            },
            AffineSemanticFamily::RigidTransform => ProgramOperatorDiagnostics::RigidTransform {
                translation: affine.translation.unwrap(),
                rotation_row_major_3x3: affine.rotation_row_major_3x3.unwrap(),
            },
            AffineSemanticFamily::SimilarityTransform => {
                ProgramOperatorDiagnostics::SimilarityTransform {
                    translation: affine.translation.unwrap(),
                    rotation_row_major_3x3: affine.rotation_row_major_3x3.unwrap(),
                    uniform_scale: affine.uniform_scale.unwrap(),
                }
            }
            AffineSemanticFamily::GeneralAffine => ProgramOperatorDiagnostics::GeneralAffine {
                matrix_row_major_4x4: affine.matrix_row_major_4x4,
            },
        },
        ProgramOperator::Bend(parameters) => ProgramOperatorDiagnostics::Bend { parameters },
    }
}

fn source_points() -> Vec<[f32; 3]> {
    vec![
        [0.0, 0.0, 0.0],
        [0.5, 0.2, 0.0],
        [1.0, -0.1, 0.3],
        [1.5, 0.4, -0.2],
    ]
}

fn standard_bend() -> ProgramOperator {
    ProgramOperator::Bend(BendParameters {
        origin: [0.0, 0.0, 0.0],
        longitudinal_axis: [1.0, 0.0, 0.0],
        bend_direction: [0.0, 1.0, 0.0],
        angle_radians: 0.7,
        interval_start: 0.0,
        interval_end: 2.0,
    })
}

fn translation(delta: [f32; 3]) -> ProgramOperator {
    ProgramOperator::Affine(AffineOperator {
        semantic_family: AffineSemanticFamily::Translation,
        matrix_row_major_4x4: [
            1.0, 0.0, 0.0, delta[0], //
            0.0, 1.0, 0.0, delta[1], //
            0.0, 0.0, 1.0, delta[2], //
            0.0, 0.0, 0.0, 1.0,
        ],
        translation: Some(delta),
        rotation_row_major_3x3: None,
        uniform_scale: None,
    })
}

fn apply(operator: ProgramOperator, positions: &[[f32; 3]]) -> Vec<[f32; 3]> {
    evaluate_operator(&operator, positions).unwrap()
}

fn positions_bit_equal(left: &[[f32; 3]], right: &[[f32; 3]]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.iter()
                .zip(right)
                .all(|(left, right)| left.to_bits() == right.to_bits())
        })
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1.0e-12,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn empty_program_wins_isolated_local_edit() {
    let source = source_points();
    let mut target = source.clone();
    target[2][1] += 0.25;
    let affine_generator = FakeAffineGenerator::default()
        .with_rule(source.clone(), vec![translation([1.0, 0.0, 0.0])]);

    let result = run_search(
        &source,
        &target,
        &affine_generator,
        &FakeBendGenerator::default(),
        ProgramSearchSettings::default(),
    );

    assert!(selected_shape(&result).is_empty());
}

#[test]
fn affine_wins_exact_translation() {
    let source = source_points();
    let affine = translation([1.0, -2.0, 0.5]);
    let target = apply(affine, &source);
    let affine_generator = FakeAffineGenerator::default().with_rule(source.clone(), vec![affine]);

    let result = run_search(
        &source,
        &target,
        &affine_generator,
        &FakeBendGenerator::default(),
        ProgramSearchSettings::default(),
    );

    assert_eq!(selected_shape(&result), vec!["affine"]);
}

#[test]
fn bend_wins_exact_bend() {
    let source = source_points();
    let bend = standard_bend();
    let target = apply(bend, &source);
    let bend_generator = FakeBendGenerator::default().with_rule(source.clone(), vec![bend]);

    let result = run_search(
        &source,
        &target,
        &FakeAffineGenerator::default(),
        &bend_generator,
        ProgramSearchSettings::default(),
    );

    assert_eq!(selected_shape(&result), vec!["bend"]);
}

#[test]
fn affine_then_bend_wins_corresponding_synthetic_case() {
    let source = source_points();
    let affine = translation([0.25, 0.0, 0.0]);
    let bend = standard_bend();
    let affine_stage = apply(affine, &source);
    let target = apply(bend, &affine_stage);
    let affine_generator = FakeAffineGenerator::default().with_rule(source.clone(), vec![affine]);
    let bend_generator = FakeBendGenerator::default().with_rule(affine_stage.clone(), vec![bend]);

    let result = run_search(
        &source,
        &target,
        &affine_generator,
        &bend_generator,
        ProgramSearchSettings::default(),
    );

    assert_eq!(selected_shape(&result), vec!["affine", "bend"]);
}

#[test]
fn bend_then_affine_wins_corresponding_synthetic_case() {
    let source = source_points();
    let bend = standard_bend();
    let affine = translation([0.25, -0.5, 0.75]);
    let bend_stage = apply(bend, &source);
    let target = apply(affine, &bend_stage);
    let bend_generator = FakeBendGenerator::default().with_rule(source.clone(), vec![bend]);
    let affine_generator =
        FakeAffineGenerator::default().with_rule(bend_stage.clone(), vec![affine]);

    let result = run_search(
        &source,
        &target,
        &affine_generator,
        &bend_generator,
        ProgramSearchSettings::default(),
    );

    assert_eq!(selected_shape(&result), vec!["bend", "affine"]);
}

#[test]
fn unnecessary_second_operation_loses_due_to_overhead() {
    let source = vec![[0.5, 0.0, 0.0]];
    let direct_affine = translation([0.0, 1.0, 0.0]);
    let target = apply(direct_affine, &source);
    let bend = standard_bend();
    let bend_stage = apply(bend, &source);
    let post_bend_affine = translation([
        target[0][0] - bend_stage[0][0],
        target[0][1] - bend_stage[0][1],
        target[0][2] - bend_stage[0][2],
    ]);
    let affine_generator = FakeAffineGenerator::default()
        .with_rule(source.clone(), vec![direct_affine])
        .with_rule(bend_stage.clone(), vec![post_bend_affine]);
    let bend_generator = FakeBendGenerator::default().with_rule(source.clone(), vec![bend]);

    let result = run_search(
        &source,
        &target,
        &affine_generator,
        &bend_generator,
        ProgramSearchSettings::default(),
    );

    assert_eq!(selected_shape(&result), vec!["affine"]);
}

#[test]
fn equivalent_duplicate_programs_collapse() {
    let source = source_points();
    let affine = translation([0.5, 0.0, 0.0]);
    let target = apply(affine, &source);
    let affine_generator =
        FakeAffineGenerator::default().with_rule(source.clone(), vec![affine, affine]);

    let result = run_search(
        &source,
        &target,
        &affine_generator,
        &FakeBendGenerator::default(),
        ProgramSearchSettings::default(),
    );

    let affine_program_count = result
        .hypotheses
        .iter()
        .filter(|hypothesis| {
            matches!(
                hypothesis.program.operators.as_slice(),
                [ProgramOperator::Affine(_)]
            )
        })
        .count();
    assert_eq!(affine_program_count, 1);
}

#[test]
fn total_program_cap_is_enforced_deterministically() {
    let source = source_points();
    let exact_affine = translation([1.0, 0.0, 0.0]);
    let target = apply(exact_affine, &source);
    let affine_generator = FakeAffineGenerator::default().with_rule(
        source.clone(),
        vec![
            exact_affine,
            translation([0.5, 0.0, 0.0]),
            translation([1.5, 0.0, 0.0]),
            translation([0.25, 0.0, 0.0]),
            translation([1.75, 0.0, 0.0]),
        ],
    );
    let settings = ProgramSearchSettings {
        maximum_total_programs: 3,
        ..ProgramSearchSettings::default()
    };

    let first = run_search(
        &source,
        &target,
        &affine_generator,
        &FakeBendGenerator::default(),
        settings,
    );
    let second = run_search(
        &source,
        &target,
        &affine_generator,
        &FakeBendGenerator::default(),
        settings,
    );

    assert_eq!(first.hypotheses.len(), 3);
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
    assert_eq!(selected_shape(&first), vec!["affine"]);
}

#[test]
fn stage_diagnostics_record_before_and_after_error() {
    let source = source_points();
    let affine = translation([0.25, 0.0, 0.0]);
    let bend = standard_bend();
    let affine_stage = apply(affine, &source);
    let target = apply(bend, &affine_stage);
    let affine_generator = FakeAffineGenerator::default().with_rule(source.clone(), vec![affine]);
    let bend_generator = FakeBendGenerator::default().with_rule(affine_stage.clone(), vec![bend]);

    let result = run_search(
        &source,
        &target,
        &affine_generator,
        &bend_generator,
        ProgramSearchSettings::default(),
    );
    let selected = &result.hypotheses[result.selected_hypothesis_index.unwrap()];
    let diagnostics = result.diagnostics.as_ref().unwrap();
    let selected_diagnostics =
        &diagnostics.program_hypotheses[diagnostics.selected_program_hypothesis_index];

    assert_eq!(
        diagnostics.diagnostics_schema_version,
        DIAGNOSTICS_SCHEMA_VERSION_V4
    );
    assert_eq!(selected_diagnostics.stages.len(), 2);
    assert_close(
        selected_diagnostics.stages[0].weighted_error_before,
        selected.candidates[0].weighted_error_before,
    );
    assert_close(
        selected_diagnostics.stages[0].weighted_error_after,
        selected.candidates[0].weighted_error_after,
    );
    assert_close(
        selected_diagnostics.stages[1].weighted_error_before,
        selected_diagnostics.stages[0].weighted_error_after,
    );
    assert_close(
        selected_diagnostics.final_correction.weighted_error_before,
        0.0,
    );
}

#[test]
fn selected_program_is_stable_across_repeated_runs() {
    let source = source_points();
    let bend = standard_bend();
    let affine = translation([0.25, -0.5, 0.75]);
    let bend_stage = apply(bend, &source);
    let target = apply(affine, &bend_stage);
    let bend_generator = FakeBendGenerator::default().with_rule(source.clone(), vec![bend]);
    let affine_generator =
        FakeAffineGenerator::default().with_rule(bend_stage.clone(), vec![affine]);
    let expected = serde_json::to_string(&run_search(
        &source,
        &target,
        &affine_generator,
        &bend_generator,
        ProgramSearchSettings::default(),
    ))
    .unwrap();

    for _ in 0..10 {
        let actual = serde_json::to_string(&run_search(
            &source,
            &target,
            &affine_generator,
            &bend_generator,
            ProgramSearchSettings::default(),
        ))
        .unwrap();
        assert_eq!(actual, expected);
    }
}

#[test]
fn bend_target_helper_uses_real_bend_evaluation() {
    let source = source_points();
    let ProgramOperator::Bend(parameters) = standard_bend() else {
        unreachable!();
    };

    assert_eq!(
        evaluate_bend(&parameters, &source).unwrap(),
        apply(ProgramOperator::Bend(parameters), &source)
    );
}
