#![forbid(unsafe_code)]

use shape_decompiler::v3::bend::{BendParameters, evaluate_bend, validate_bend_parameters};
use shape_decompiler::v3::bend_fit::generate_bend_candidates;
use shape_decompiler::v3::inference::{BendFitSettings, FittedOperatorCandidate};
use shape_decompiler::v3::program::ProgramOperator;

#[derive(Debug, Clone)]
struct TestMesh {
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

fn fitting_settings() -> BendFitSettings {
    BendFitSettings {
        maximum_absolute_angle_radians: std::f32::consts::PI,
        minimum_interval_length: 1.0e-9,
        maximum_refinement_iterations: 4,
    }
}

fn regular_beam() -> TestMesh {
    let stations = (0..=20)
        .map(|index| index as f32 / 20.0)
        .collect::<Vec<_>>();
    beam_with_stations(&stations)
}

fn uneven_beam() -> TestMesh {
    beam_with_stations(&[
        0.0, 0.02, 0.04, 0.06, 0.08, 0.10, 0.18, 0.28, 0.40, 0.55, 0.72, 0.86, 1.0,
    ])
}

fn beam_with_stations(stations: &[f32]) -> TestMesh {
    let mut positions = Vec::new();
    for x in stations {
        positions.extend([
            [*x, -0.10, -0.05],
            [*x, 0.10, -0.05],
            [*x, 0.10, 0.05],
            [*x, -0.10, 0.05],
        ]);
    }

    let mut indices = Vec::new();
    for ring in 0..stations.len() - 1 {
        let current = ring as u32 * 4;
        let next = current + 4;
        for corner in 0..4 {
            let a = current + corner;
            let b = current + (corner + 1) % 4;
            let c = next + (corner + 1) % 4;
            let d = next + corner;
            indices.extend([a, b, c, a, c, d]);
        }
    }
    let last = (stations.len() as u32 - 1) * 4;
    indices.extend([0, 1, 2, 0, 2, 3]);
    indices.extend([last, last + 2, last + 1, last, last + 3, last + 2]);

    TestMesh { positions, indices }
}

fn bend_x_to_y(angle_degrees: f32) -> BendParameters {
    BendParameters {
        origin: [0.5, 0.0, 0.0],
        longitudinal_axis: [1.0, 0.0, 0.0],
        bend_direction: [0.0, 1.0, 0.0],
        angle_radians: angle_degrees.to_radians(),
        interval_start: -0.5,
        interval_end: 0.5,
    }
}

fn candidates_for(mesh: &TestMesh, target: &[[f32; 3]]) -> Vec<FittedOperatorCandidate> {
    let candidates = generate_bend_candidates(
        &mesh.positions,
        target,
        &mesh.indices,
        &[],
        fitting_settings(),
    )
    .expect("bend candidates");
    assert!(!candidates.is_empty(), "expected at least one candidate");
    candidates
}

fn top_bend_parameters(candidate: &FittedOperatorCandidate) -> BendParameters {
    match candidate.operator {
        ProgramOperator::Bend(parameters) => parameters,
        ProgramOperator::Affine(_) => panic!("expected bend candidate"),
    }
}

fn assert_recovered(candidate: &FittedOperatorCandidate, expected: BendParameters) {
    let candidate = validate_bend_parameters(&top_bend_parameters(candidate)).unwrap();
    let expected = validate_bend_parameters(&expected).unwrap();
    let axis_error = angle_between_degrees(candidate.longitudinal_axis, expected.longitudinal_axis);
    let mut candidate_angle = candidate.angle_radians;
    if dot(candidate.bend_direction, expected.bend_direction) < 0.0 {
        candidate_angle = -candidate_angle;
    }
    let angle_error = (candidate_angle - expected.angle_radians)
        .abs()
        .to_degrees();
    assert!(
        axis_error <= 2.0,
        "axis error {axis_error} exceeded tolerance; candidate={candidate:?}"
    );
    assert!(
        angle_error <= 2.0,
        "angle error {angle_error} exceeded tolerance; candidate={candidate:?}"
    );
    assert!(
        candidate.interval_end > candidate.interval_start,
        "candidate interval must be non-degenerate"
    );
    assert!(
        candidate_angle.signum() == expected.angle_radians.signum(),
        "signed bend angle was not retained"
    );
}

fn assert_strong(candidate: &FittedOperatorCandidate) {
    assert!(
        candidate.weighted_explained_fraction >= 0.95,
        "weighted explained fraction was {}",
        candidate.weighted_explained_fraction
    );
}

fn angle_between_degrees(left: [f32; 3], right: [f32; 3]) -> f32 {
    dot(left, right).abs().clamp(-1.0, 1.0).acos().to_degrees()
}

fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn apply_bend(source: &[[f32; 3]], parameters: BendParameters) -> Vec<[f32; 3]> {
    evaluate_bend(&parameters, source).expect("ground-truth bend")
}

fn transform_mesh(mesh: &TestMesh, transform: impl Fn([f32; 3]) -> [f32; 3]) -> TestMesh {
    TestMesh {
        positions: mesh.positions.iter().copied().map(transform).collect(),
        indices: mesh.indices.clone(),
    }
}

fn translate(position: [f32; 3], offset: [f32; 3]) -> [f32; 3] {
    [
        position[0] + offset[0],
        position[1] + offset[1],
        position[2] + offset[2],
    ]
}

fn scale_position(position: [f32; 3], scale: f32) -> [f32; 3] {
    [
        position[0] * scale,
        position[1] * scale,
        position[2] * scale,
    ]
}

fn rotate_z(position: [f32; 3], angle: f32) -> [f32; 3] {
    let (sin, cos) = angle.sin_cos();
    [
        cos * position[0] - sin * position[1],
        sin * position[0] + cos * position[1],
        position[2],
    ]
}

#[test]
fn full_interval_positive_bend_is_recovered() {
    let source = regular_beam();
    let expected = bend_x_to_y(45.0);
    let target = apply_bend(&source.positions, expected);

    let candidates = candidates_for(&source, &target);
    let candidate = &candidates[0];

    assert_recovered(candidate, expected);
    assert_strong(candidate);
    assert_eq!(candidate.cumulative_positions.len(), source.positions.len());
    assert_eq!(candidate.semantic_parameter_count, 9);
    assert_eq!(candidate.semantic_metadata_bytes, 48);
    assert!(candidate.stable_candidate_id.starts_with("bend-"));
    assert_eq!(
        candidate.fitting_diagnostics.generator,
        "deterministic_bend_fit"
    );
}

#[test]
fn negative_bend_is_recovered() {
    let source = regular_beam();
    let expected = bend_x_to_y(-30.0);
    let target = apply_bend(&source.positions, expected);

    let candidates = candidates_for(&source, &target);

    assert_recovered(&candidates[0], expected);
    assert_strong(&candidates[0]);
}

#[test]
fn partial_interval_bend_is_refined() {
    let source = regular_beam();
    let mut expected = bend_x_to_y(45.0);
    expected.interval_start = -0.25;
    expected.interval_end = 0.25;
    let target = apply_bend(&source.positions, expected);

    let candidates = candidates_for(&source, &target);

    assert_recovered(&candidates[0], expected);
    assert_strong(&candidates[0]);
    assert!(candidates[0].fitting_diagnostics.refinement_rounds >= 3);
}

#[test]
fn translated_object_bend_is_recovered() {
    let offset = [10.0, -3.0, 2.0];
    let source = transform_mesh(&regular_beam(), |position| translate(position, offset));
    let mut expected = bend_x_to_y(30.0);
    expected.origin = translate(expected.origin, offset);
    let target = apply_bend(&source.positions, expected);

    let candidates = candidates_for(&source, &target);

    assert_recovered(&candidates[0], expected);
    assert_strong(&candidates[0]);
}

#[test]
fn rotated_source_bend_is_recovered() {
    let angle = 35.0_f32.to_radians();
    let source = transform_mesh(&regular_beam(), |position| rotate_z(position, angle));
    let mut expected = bend_x_to_y(30.0);
    expected.origin = rotate_z(expected.origin, angle);
    expected.longitudinal_axis = rotate_z(expected.longitudinal_axis, angle);
    expected.bend_direction = rotate_z(expected.bend_direction, angle);
    let target = apply_bend(&source.positions, expected);

    let candidates = candidates_for(&source, &target);

    assert_recovered(&candidates[0], expected);
    assert_strong(&candidates[0]);
}

#[test]
fn coordinate_scales_are_recovered() {
    for scale in [1.0e-3_f32, 1.0, 1.0e3] {
        let source = transform_mesh(&regular_beam(), |position| scale_position(position, scale));
        let mut expected = bend_x_to_y(45.0);
        expected.origin = scale_position(expected.origin, scale);
        expected.interval_start *= scale;
        expected.interval_end *= scale;
        let target = apply_bend(&source.positions, expected);

        let candidates = candidates_for(&source, &target);

        assert_recovered(&candidates[0], expected);
        assert_strong(&candidates[0]);
    }
}

#[test]
fn uneven_tessellation_bend_is_recovered_with_area_weights() {
    let source = uneven_beam();
    let expected = bend_x_to_y(45.0);
    let target = apply_bend(&source.positions, expected);

    let candidates = candidates_for(&source, &target);

    assert_recovered(&candidates[0], expected);
    assert_strong(&candidates[0]);
}

#[test]
fn bend_with_one_local_vertex_edit_remains_strong() {
    let source = regular_beam();
    let expected = bend_x_to_y(45.0);
    let mut target = apply_bend(&source.positions, expected);
    target[13] = translate(target[13], [0.0, 0.01, -0.005]);

    let candidates = candidates_for(&source, &target);

    assert_recovered(&candidates[0], expected);
    assert!(
        candidates[0].weighted_explained_fraction >= 0.90,
        "local edit should not erase the bend signal"
    );
}

#[test]
fn affine_only_deformation_does_not_make_strong_bend_candidate() {
    let source = regular_beam();
    let target = source
        .positions
        .iter()
        .copied()
        .map(|position| translate(position, [0.0, 0.25, 0.0]))
        .collect::<Vec<_>>();

    let candidates = generate_bend_candidates(
        &source.positions,
        &target,
        &source.indices,
        &[],
        fitting_settings(),
    )
    .expect("bend candidates");

    if let Some(candidate) = candidates.first() {
        assert!(
            candidate.weighted_explained_fraction < 0.80,
            "affine-only deformation produced a strong bend candidate: {}",
            candidate.weighted_explained_fraction
        );
    }
}

#[test]
fn candidate_ordering_is_deterministic() {
    let source = regular_beam();
    let expected = bend_x_to_y(45.0);
    let target = apply_bend(&source.positions, expected);

    let first = candidates_for(&source, &target);
    let second = candidates_for(&source, &target);

    let first_ids = first
        .iter()
        .map(|candidate| {
            (
                candidate.stable_candidate_id.clone(),
                candidate.weighted_error_after.to_bits(),
                top_bend_parameters(candidate),
            )
        })
        .collect::<Vec<_>>();
    let second_ids = second
        .iter()
        .map(|candidate| {
            (
                candidate.stable_candidate_id.clone(),
                candidate.weighted_error_after.to_bits(),
                top_bend_parameters(candidate),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(first_ids, second_ids);
}
