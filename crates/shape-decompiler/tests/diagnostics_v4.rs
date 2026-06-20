use std::cmp::Ordering;

use shape_decompiler::v3::bend::BendParameters;
use shape_decompiler::v3::diagnostics::{
    DEFAULT_BEND_FAMILY_PRIOR_V4, DIAGNOSTICS_SCHEMA_VERSION_V4, DiagnosticsErrorV4,
    InferenceDiagnosticsV4, InferenceScoringPolicyV4, ProgramCorrectionDiagnostics,
    ProgramDiagnosticsInput, ProgramHypothesisDiagnosticsV4, ProgramOperatorDiagnostics,
    ProgramOperatorFamilyV4, StageDiagnostics, StageDiagnosticsInput, build_program_diagnostics,
    build_stage_diagnostics, compare_program_hypotheses, default_scoring_policy_v4,
    verify_score_recomputation,
};
use shape_decompiler::v3::program::{
    SemanticVerificationMode, SemanticVerificationPolicy, SemanticVerificationReport,
};

const IDENTITY_3X3: [f32; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
const IDENTITY_4X4: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 1.0,
];

fn affine_operator() -> ProgramOperatorDiagnostics {
    ProgramOperatorDiagnostics::GeneralAffine {
        matrix_row_major_4x4: IDENTITY_4X4,
    }
}

fn bend_operator() -> ProgramOperatorDiagnostics {
    ProgramOperatorDiagnostics::Bend {
        parameters: BendParameters {
            origin: [0.0, 0.0, 0.0],
            longitudinal_axis: [0.0, 1.0, 0.0],
            bend_direction: [1.0, 0.0, 0.0],
            angle_radians: 0.25,
            interval_start: -1.0,
            interval_end: 1.0,
        },
    }
}

fn translation_operator(value: f32) -> ProgramOperatorDiagnostics {
    ProgramOperatorDiagnostics::Translation {
        translation: [value, 0.0, 0.0],
    }
}

fn rigid_operator() -> ProgramOperatorDiagnostics {
    ProgramOperatorDiagnostics::RigidTransform {
        translation: [0.0, 0.0, 0.0],
        rotation_row_major_3x3: IDENTITY_3X3,
    }
}

fn similarity_operator() -> ProgramOperatorDiagnostics {
    ProgramOperatorDiagnostics::SimilarityTransform {
        translation: [0.0, 0.0, 0.0],
        rotation_row_major_3x3: IDENTITY_3X3,
        uniform_scale: 1.0,
    }
}

fn verification_policy() -> SemanticVerificationPolicy {
    SemanticVerificationPolicy {
        mode: SemanticVerificationMode::Tolerance,
        absolute_epsilon: 1.0e-6,
        relative_epsilon: 1.0e-5,
        ulp_multiplier: 2.0,
    }
}

fn verification_report(
    max_component_error: f64,
    max_euclidean_error: f64,
    rms_euclidean_error: f64,
    passed: bool,
) -> SemanticVerificationReport {
    SemanticVerificationReport {
        max_component_error,
        max_euclidean_error,
        mean_euclidean_error: rms_euclidean_error * 0.5,
        rms_euclidean_error,
        outside_tolerance: usize::from(!passed),
        passed,
    }
}

fn stage(
    stage_index: usize,
    operator: ProgramOperatorDiagnostics,
    weighted_before: f64,
    weighted_after: f64,
    raw_before: f64,
    raw_after: f64,
) -> StageDiagnostics {
    build_stage_diagnostics(StageDiagnosticsInput {
        stage_index,
        operator,
        weighted_error_before: weighted_before,
        weighted_error_after: weighted_after,
        raw_error_before: raw_before,
        raw_error_after: raw_after,
        semantic_verification_policy: verification_policy(),
        semantic_verification_report: verification_report(1.0e-8, 2.0e-8, 1.5e-8, true),
    })
    .unwrap()
}

fn program_with_sequences(
    policy: InferenceScoringPolicyV4,
    operators: Vec<ProgramOperatorDiagnostics>,
    weighted_sequence: &[f64],
    raw_sequence: &[f64],
    approximate_residual_coverage: f64,
    exact_residual_bytes: usize,
) -> ProgramHypothesisDiagnosticsV4 {
    assert_eq!(weighted_sequence.len(), operators.len() + 1);
    assert_eq!(raw_sequence.len(), operators.len() + 1);
    let stages = operators
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, operator)| {
            stage(
                index,
                operator,
                weighted_sequence[index],
                weighted_sequence[index + 1],
                raw_sequence[index],
                raw_sequence[index + 1],
            )
        })
        .collect();

    build_program_diagnostics(ProgramDiagnosticsInput {
        operators,
        stages,
        final_correction: ProgramCorrectionDiagnostics {
            corrected_vertex_count: exact_residual_bytes / 16,
            exact_residual_bytes,
            weighted_error_before: *weighted_sequence.last().unwrap(),
            weighted_error_after: 0.0,
            raw_error_before: *raw_sequence.last().unwrap(),
            raw_error_after: 0.0,
        },
        raw_identity_error: raw_sequence[0],
        weighted_identity_error: weighted_sequence[0],
        error_normalization_scale: 10.0,
        literal_size_bytes: 1200,
        approximate_residual_coverage,
        scoring_policy: policy,
        selected: false,
        rejection_reason: None,
    })
    .unwrap()
}

fn zero_weight_policy() -> InferenceScoringPolicyV4 {
    let mut policy = default_scoring_policy_v4();
    policy.geometric_error_weight = 0.0;
    policy.parameter_weight = 0.0;
    policy.semantic_metadata_weight = 0.0;
    policy.approximate_residual_weight = 0.0;
    policy.exact_residual_weight = 0.0;
    policy.per_operator_overhead = 0.0;
    for prior in policy.family_priors.values_mut() {
        *prior = 0.0;
    }
    policy
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1.0e-12,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn empty_program_scoring_uses_empty_operator_array() {
    let policy = default_scoring_policy_v4();
    let hypothesis = program_with_sequences(policy.clone(), Vec::new(), &[10.0], &[12.0], 1.0, 160);

    assert!(hypothesis.operators.is_empty());
    assert!(hypothesis.stages.is_empty());
    assert_eq!(hypothesis.semantic_parameter_count, 0);
    assert_eq!(hypothesis.semantic_metadata_bytes, 0);
    assert_eq!(hypothesis.score.family_prior_sum, 0.0);
    assert_eq!(hypothesis.score.per_operator_overhead, 0.0);
    assert_close(
        hypothesis.score.normalized_weighted_final_geometric_error,
        1.0,
    );
    assert_close(
        hypothesis.score.exact_residual_byte_cost,
        160.0 / 1200.0 * policy.exact_residual_weight,
    );

    let json = serde_json::to_string(&hypothesis).unwrap();
    assert!(!json.contains("no_op"));
    verify_score_recomputation(&policy, &hypothesis).unwrap();
}

#[test]
fn one_affine_stage_records_parameters_and_score_components() {
    let policy = default_scoring_policy_v4();
    let hypothesis = program_with_sequences(
        policy.clone(),
        vec![affine_operator()],
        &[10.0, 4.0],
        &[12.0, 5.0],
        0.25,
        96,
    );

    assert_eq!(hypothesis.stages[0].stage_index, 0);
    assert_eq!(hypothesis.stages[0].operator, hypothesis.operators[0]);
    assert_close(hypothesis.stages[0].weighted_explained_increment, 6.0);
    assert_close(hypothesis.stages[0].raw_explained_increment, 7.0);
    assert_eq!(hypothesis.semantic_parameter_count, 12);
    assert_eq!(hypothesis.semantic_metadata_bytes, 64);
    assert_close(
        hypothesis.score.family_prior_sum,
        *policy
            .family_priors
            .get(&ProgramOperatorFamilyV4::GeneralAffine)
            .unwrap(),
    );
    assert_close(hypothesis.weighted_explained_fraction, 0.6);
    verify_score_recomputation(&policy, &hypothesis).unwrap();
}

#[test]
fn one_bend_stage_uses_synthetic_diagnostics() {
    let policy = default_scoring_policy_v4();
    let operator = bend_operator();
    let bend_stage = build_stage_diagnostics(StageDiagnosticsInput {
        stage_index: 0,
        operator: operator.clone(),
        weighted_error_before: 10.0,
        weighted_error_after: 3.0,
        raw_error_before: 11.0,
        raw_error_after: 4.0,
        semantic_verification_policy: verification_policy(),
        semantic_verification_report: verification_report(0.001, 0.002, 0.0015, false),
    })
    .unwrap();
    let hypothesis = build_program_diagnostics(ProgramDiagnosticsInput {
        operators: vec![operator],
        stages: vec![bend_stage],
        final_correction: ProgramCorrectionDiagnostics {
            corrected_vertex_count: 4,
            exact_residual_bytes: 64,
            weighted_error_before: 3.0,
            weighted_error_after: 0.0,
            raw_error_before: 4.0,
            raw_error_after: 0.0,
        },
        raw_identity_error: 11.0,
        weighted_identity_error: 10.0,
        error_normalization_scale: 10.0,
        literal_size_bytes: 1200,
        approximate_residual_coverage: 0.1,
        scoring_policy: policy.clone(),
        selected: false,
        rejection_reason: None,
    })
    .unwrap();

    assert_eq!(hypothesis.semantic_parameter_count, 9);
    assert_eq!(hypothesis.semantic_metadata_bytes, 48);
    assert!(!hypothesis.stages[0].semantic_verification_passed);
    assert_close(
        hypothesis.stages[0].semantic_to_baked_max_component_error,
        0.001,
    );
    assert_close(
        hypothesis.score.family_prior_sum,
        DEFAULT_BEND_FAMILY_PRIOR_V4,
    );
    verify_score_recomputation(&policy, &hypothesis).unwrap();
}

#[test]
fn affine_then_bend_preserves_ordered_stage_accounting() {
    let policy = default_scoring_policy_v4();
    let hypothesis = program_with_sequences(
        policy.clone(),
        vec![affine_operator(), bend_operator()],
        &[20.0, 8.0, 4.0],
        &[30.0, 12.0, 6.0],
        0.2,
        96,
    );

    assert_eq!(
        hypothesis.operators[0].family(),
        ProgramOperatorFamilyV4::GeneralAffine
    );
    assert_eq!(
        hypothesis.operators[1].family(),
        ProgramOperatorFamilyV4::Bend
    );
    assert_close(hypothesis.stages[0].weighted_explained_increment, 12.0);
    assert_close(hypothesis.stages[1].weighted_explained_increment, 4.0);
    assert_eq!(hypothesis.semantic_parameter_count, 21);
    assert_close(
        hypothesis.score.family_prior_sum,
        policy.family_priors[&ProgramOperatorFamilyV4::GeneralAffine]
            + policy.family_priors[&ProgramOperatorFamilyV4::Bend],
    );
    verify_score_recomputation(&policy, &hypothesis).unwrap();
}

#[test]
fn bend_then_affine_preserves_ordered_stage_accounting() {
    let policy = default_scoring_policy_v4();
    let hypothesis = program_with_sequences(
        policy.clone(),
        vec![bend_operator(), affine_operator()],
        &[20.0, 9.0, 4.0],
        &[30.0, 15.0, 6.0],
        0.2,
        96,
    );

    assert_eq!(
        hypothesis.operators[0].family(),
        ProgramOperatorFamilyV4::Bend
    );
    assert_eq!(
        hypothesis.operators[1].family(),
        ProgramOperatorFamilyV4::GeneralAffine
    );
    assert_close(hypothesis.stages[0].weighted_explained_increment, 11.0);
    assert_close(hypothesis.stages[1].raw_explained_increment, 9.0);
    assert_eq!(hypothesis.semantic_parameter_count, 21);
    verify_score_recomputation(&policy, &hypothesis).unwrap();
}

#[test]
fn score_recomputes_from_serialized_policy_and_diagnostics_only() {
    let policy = default_scoring_policy_v4();
    let hypothesis = program_with_sequences(
        policy.clone(),
        vec![affine_operator(), bend_operator()],
        &[10.0, 7.0, 2.0],
        &[15.0, 9.0, 3.0],
        0.15,
        80,
    );

    let policy_json = serde_json::to_string_pretty(&policy).unwrap();
    let hypothesis_json = serde_json::to_string_pretty(&hypothesis).unwrap();
    let decoded_policy: InferenceScoringPolicyV4 = serde_json::from_str(&policy_json).unwrap();
    let decoded_hypothesis: ProgramHypothesisDiagnosticsV4 =
        serde_json::from_str(&hypothesis_json).unwrap();

    verify_score_recomputation(&decoded_policy, &decoded_hypothesis).unwrap();
}

#[test]
fn family_priors_sum_across_stages() {
    let policy = default_scoring_policy_v4();
    assert_eq!(
        policy.family_priors.get(&ProgramOperatorFamilyV4::Bend),
        Some(&DEFAULT_BEND_FAMILY_PRIOR_V4)
    );
    let hypothesis = program_with_sequences(
        policy.clone(),
        vec![
            translation_operator(1.0),
            rigid_operator(),
            similarity_operator(),
            bend_operator(),
        ],
        &[25.0, 20.0, 15.0, 10.0, 5.0],
        &[30.0, 24.0, 18.0, 12.0, 6.0],
        0.3,
        112,
    );
    let expected = hypothesis
        .operators
        .iter()
        .map(|operator| policy.family_priors[&operator.family()])
        .sum::<f64>();

    assert_close(hypothesis.score.family_prior_sum, expected);
    verify_score_recomputation(&policy, &hypothesis).unwrap();
}

#[test]
fn per_operator_overhead_penalizes_unnecessary_splitting() {
    let mut policy = zero_weight_policy();
    policy.per_operator_overhead = 0.25;
    let one_operator = program_with_sequences(
        policy.clone(),
        vec![translation_operator(1.0)],
        &[10.0, 2.0],
        &[10.0, 2.0],
        0.0,
        0,
    );
    let split_operator = program_with_sequences(
        policy.clone(),
        vec![translation_operator(0.5), translation_operator(0.5)],
        &[10.0, 6.0, 2.0],
        &[10.0, 6.0, 2.0],
        0.0,
        0,
    );

    assert_close(one_operator.score.per_operator_overhead, 0.25);
    assert_close(split_operator.score.per_operator_overhead, 0.5);
    assert_eq!(
        compare_program_hypotheses(&one_operator, &split_operator),
        Ordering::Less
    );
}

#[test]
fn deterministic_tie_breaking_uses_declared_order() {
    let policy = zero_weight_policy();
    let low_coverage = program_with_sequences(
        policy.clone(),
        vec![translation_operator(1.0)],
        &[10.0, 2.0],
        &[10.0, 2.0],
        0.1,
        64,
    );
    let high_coverage = program_with_sequences(
        policy.clone(),
        vec![translation_operator(1.0)],
        &[10.0, 2.0],
        &[10.0, 2.0],
        0.2,
        64,
    );
    assert_eq!(
        compare_program_hypotheses(&low_coverage, &high_coverage),
        Ordering::Less
    );

    let fewer_exact_bytes = program_with_sequences(
        policy.clone(),
        vec![translation_operator(1.0)],
        &[10.0, 2.0],
        &[10.0, 2.0],
        0.1,
        32,
    );
    assert_eq!(
        compare_program_hypotheses(&fewer_exact_bytes, &low_coverage),
        Ordering::Less
    );

    let empty = program_with_sequences(policy.clone(), Vec::new(), &[2.0], &[2.0], 0.1, 32);
    assert_eq!(
        compare_program_hypotheses(&empty, &fewer_exact_bytes),
        Ordering::Less
    );

    let fewer_parameters = program_with_sequences(
        policy.clone(),
        vec![translation_operator(1.0)],
        &[10.0, 2.0],
        &[10.0, 2.0],
        0.1,
        32,
    );
    let more_parameters = program_with_sequences(
        policy.clone(),
        vec![affine_operator()],
        &[10.0, 2.0],
        &[10.0, 2.0],
        0.1,
        32,
    );
    assert_eq!(
        compare_program_hypotheses(&fewer_parameters, &more_parameters),
        Ordering::Less
    );

    let rigid_then_translation = program_with_sequences(
        policy.clone(),
        vec![rigid_operator(), translation_operator(1.0)],
        &[10.0, 6.0, 2.0],
        &[10.0, 6.0, 2.0],
        0.1,
        32,
    );
    let translation_then_rigid = program_with_sequences(
        policy,
        vec![translation_operator(1.0), rigid_operator()],
        &[10.0, 6.0, 2.0],
        &[10.0, 6.0, 2.0],
        0.1,
        32,
    );
    assert_eq!(
        compare_program_hypotheses(&rigid_then_translation, &translation_then_rigid),
        Ordering::Less
    );
}

#[test]
fn diagnostics_schema_version_is_four() {
    let policy = default_scoring_policy_v4();
    let hypothesis = program_with_sequences(policy.clone(), Vec::new(), &[0.0], &[0.0], 0.0, 0);
    let diagnostics = InferenceDiagnosticsV4 {
        diagnostics_schema_version: DIAGNOSTICS_SCHEMA_VERSION_V4,
        package_schema_version: 3,
        surface_weighting: "triangle_area_derived_vertex_weights".to_owned(),
        raw_identity_error: 0.0,
        weighted_identity_error: 0.0,
        scoring_policy: policy,
        selected_program_hypothesis_index: 0,
        program_hypotheses: vec![hypothesis],
    };

    assert_eq!(DIAGNOSTICS_SCHEMA_VERSION_V4, 4);
    assert_eq!(diagnostics.diagnostics_schema_version, 4);
    assert_eq!(diagnostics.scoring_policy.scoring_version, 4);
}

#[test]
fn malformed_or_non_finite_score_data_is_rejected() {
    let policy = default_scoring_policy_v4();
    let hypothesis = program_with_sequences(
        policy.clone(),
        vec![affine_operator()],
        &[10.0, 4.0],
        &[10.0, 4.0],
        0.25,
        96,
    );
    let mut non_finite = hypothesis.clone();
    non_finite.score.total_component_sum = f64::NAN;

    assert!(verify_score_recomputation(&policy, &non_finite).is_err());

    let mut malformed = hypothesis;
    malformed.stages[0].stage_index = 3;
    assert!(matches!(
        verify_score_recomputation(&policy, &malformed),
        Err(DiagnosticsErrorV4::StageIndexMismatch { .. })
    ));
}
