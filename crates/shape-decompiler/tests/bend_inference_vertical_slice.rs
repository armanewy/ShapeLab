#![forbid(unsafe_code)]

use std::fs;
use std::io::Cursor;
use std::path::Path;

use shape_decompiler::AffineSemanticFamily;
use shape_decompiler::v3::bend::BendParameters;
use shape_decompiler::v3::diagnostics::InferenceDiagnosticsV4;
use shape_decompiler::v3::inference::{ProgramSearchSettings, search_programs_for_mesh_pair};
use shape_decompiler::v3::package::{
    OperatorManifestV3, build_v3_package_from_program_with_diagnostics, read_decompile_package_v3,
    verify_decompile_package_v3,
};
use shape_decompiler::v3::program::{
    AffineOperator, OperatorProgram, ProgramOperator, evaluate_operator,
};
use shape_mesh::{TriangleMesh, read_obj};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Family {
    AnyAffine,
    Bend,
    Translation,
    Rigid,
    Similarity,
}

#[derive(Debug)]
struct GeneratedCase {
    name: &'static str,
    source: TriangleMesh,
    target: TriangleMesh,
    expected_sequence: Option<Vec<Family>>,
    expected_bend: Option<BendParameters>,
}

#[test]
fn generated_bend_inference_corpus_roundtrips_schema_three_packages() {
    for case in generated_cases() {
        let (program, diagnostics, package_dir) = infer_and_package(&case);
        verify_package_contract(&case, &program, &diagnostics, &package_dir);
        if let Some(expected_sequence) = &case.expected_sequence {
            let actual_sequence = selected_sequence(&program);
            assert!(
                sequence_matches(&actual_sequence, expected_sequence),
                "{} selected an unexpected explanatory family sequence",
                case.name
            );
        } else {
            assert_deterministic_ambiguous_selection(&case, &program, &diagnostics);
        }
        if let Some(expected_bend) = case.expected_bend {
            assert_recovered_bend(&case, &program, expected_bend);
        }
    }
}

#[test]
fn bend_inference_rejects_schema_three_robustness_failures() {
    for invalid in [
        BendParameters {
            origin: [0.0, 0.0, 0.0],
            longitudinal_axis: [0.0, 0.0, 0.0],
            bend_direction: [1.0, 0.0, 0.0],
            angle_radians: 0.25,
            interval_start: 0.0,
            interval_end: 1.0,
        },
        BendParameters {
            origin: [0.0, 0.0, 0.0],
            longitudinal_axis: [1.0, 0.0, 0.0],
            bend_direction: [0.0, 1.0, 0.0],
            angle_radians: 0.25,
            interval_start: 0.0,
            interval_end: 0.0,
        },
        BendParameters {
            origin: [0.0, 0.0, 0.0],
            longitudinal_axis: [1.0, 0.0, 0.0],
            bend_direction: [0.0, 1.0, 0.0],
            angle_radians: std::f32::consts::PI + 0.1,
            interval_start: 0.0,
            interval_end: 1.0,
        },
    ] {
        let source = regular_beam();
        let program = OperatorProgram {
            operators: vec![ProgramOperator::Bend(invalid)],
        };
        let error = build_v3_package_from_program_with_diagnostics(
            &program,
            &source,
            &source,
            tempfile::tempdir().unwrap().path(),
            None,
        )
        .unwrap_err();
        assert!(
            matches!(
                error,
                shape_decompiler::DecompileError::InvalidPackage { .. }
            ),
            "unexpected error for invalid bend: {error:?}"
        );
    }

    let source = regular_beam();
    let bend = bend_x_to_y(35.0);
    let target = mesh_with_positions(
        &source,
        apply_sequence(&source.positions, &[bend_operator(bend)]),
    );
    let case = GeneratedCase {
        name: "robustness-bend-package",
        source,
        target,
        expected_sequence: Some(vec![Family::Bend]),
        expected_bend: Some(bend),
    };
    let (_program, _diagnostics, package_dir) = infer_and_package(&case);
    corrupt_first_bend_stage(&package_dir);
    assert!(verify_decompile_package_v3(&package_dir).is_err());

    let (_program, _diagnostics, package_dir) = infer_and_package(&case);
    remove_first_stage_payload(&package_dir);
    assert!(verify_decompile_package_v3(&package_dir).is_err());

    let (_program, _diagnostics, package_dir) = infer_and_package(&case);
    replace_first_operator_with_unknown(&package_dir);
    assert!(verify_decompile_package_v3(&package_dir).is_err());
}

fn generated_cases() -> Vec<GeneratedCase> {
    let mut cases = Vec::new();
    let regular = regular_beam();
    let uneven = uneven_beam();
    let bend = bend_x_to_y(45.0);
    let negative_bend = bend_x_to_y(-30.0);
    let partial_bend = BendParameters {
        interval_start: -0.25,
        interval_end: 0.25,
        ..bend
    };

    cases.push(case_from_ops(
        "bend only",
        regular.clone(),
        vec![bend_operator(bend)],
        Some(vec![Family::Bend]),
        Some(bend),
    ));
    cases.push(case_from_ops(
        "negative bend",
        regular.clone(),
        vec![bend_operator(negative_bend)],
        Some(vec![Family::Bend]),
        Some(negative_bend),
    ));
    cases.push(case_from_ops(
        "partial interval bend",
        regular.clone(),
        vec![bend_operator(partial_bend)],
        Some(vec![Family::Bend]),
        Some(partial_bend),
    ));

    let translation = [2.0, -1.0, 0.5];
    let translated_bend = translate_bend(bend, translation);
    cases.push(case_from_ops(
        "translation then bend",
        regular.clone(),
        vec![
            translation_operator(translation),
            bend_operator(translated_bend),
        ],
        None,
        None,
    ));

    let rigid_angle = 25.0_f32.to_radians();
    let rigid_translation = [0.2, -0.4, 0.15];
    let rigid_bend = transform_bend_similarity(bend, rigid_angle, 1.0, rigid_translation);
    cases.push(case_from_ops(
        "rigid transform then bend",
        regular.clone(),
        vec![
            rigid_z_operator(rigid_angle, rigid_translation),
            bend_operator(rigid_bend),
        ],
        None,
        None,
    ));

    let similarity_angle = -20.0_f32.to_radians();
    let similarity_scale = 1.4;
    let similarity_translation = [-0.25, 0.15, 0.3];
    let similarity_bend = transform_bend_similarity(
        bend,
        similarity_angle,
        similarity_scale,
        similarity_translation,
    );
    cases.push(case_from_ops(
        "similarity then bend",
        regular.clone(),
        vec![
            similarity_z_operator(similarity_angle, similarity_scale, similarity_translation),
            bend_operator(similarity_bend),
        ],
        None,
        None,
    ));

    cases.push(case_from_ops(
        "bend then translation",
        regular.clone(),
        vec![bend_operator(bend), translation_operator([0.2, 0.3, -0.4])],
        None,
        None,
    ));
    cases.push(case_from_ops(
        "bend then rigid transform",
        regular.clone(),
        vec![
            bend_operator(bend),
            rigid_z_operator(-15.0_f32.to_radians(), [0.15, -0.2, 0.1]),
        ],
        None,
        None,
    ));

    let mut bend_plus_edit = mesh_with_positions(
        &regular,
        apply_sequence(&regular.positions, &[bend_operator(bend)]),
    );
    bend_plus_edit.positions[7][2] += 0.025;
    cases.push(GeneratedCase {
        name: "bend plus localized edit",
        source: regular.clone(),
        target: bend_plus_edit,
        expected_sequence: Some(vec![Family::Bend]),
        expected_bend: Some(bend),
    });

    cases.push(case_from_ops(
        "affine-only case",
        regular.clone(),
        vec![similarity_z_operator(
            10.0_f32.to_radians(),
            1.2,
            [0.1, 0.2, -0.1],
        )],
        Some(vec![Family::Similarity]),
        None,
    ));

    let mut local_edit = regular.clone();
    local_edit.positions[5][1] += 0.08;
    cases.push(GeneratedCase {
        name: "local-edit-only case",
        source: regular.clone(),
        target: local_edit,
        expected_sequence: None,
        expected_bend: None,
    });

    cases.push(case_from_ops(
        "uneven tessellation",
        uneven.clone(),
        vec![bend_operator(bend)],
        Some(vec![Family::Bend]),
        Some(bend),
    ));

    let offset = [10_000.0, -20_000.0, 5_000.0];
    let offset_source = transform_mesh(&regular, |position| translate(position, offset));
    let offset_bend = translate_bend(bend, offset);
    cases.push(case_from_ops(
        "large coordinate offset",
        offset_source,
        vec![bend_operator(offset_bend)],
        Some(vec![Family::Bend]),
        Some(offset_bend),
    ));

    for scale in [1.0e-3, 1.0, 1.0e3] {
        let scaled = transform_mesh(&regular, |position| scale_position(position, scale));
        let scaled_bend = scale_bend(bend, scale);
        let expected_sequence = if scale == 1.0e-3 {
            vec![Family::Bend, Family::AnyAffine]
        } else {
            vec![Family::Bend]
        };
        cases.push(case_from_ops(
            match scale {
                0.001 => "scale 1e-3",
                1.0 => "scale 1",
                _ => "scale 1e3",
            },
            scaled,
            vec![bend_operator(scaled_bend)],
            Some(expected_sequence),
            Some(scaled_bend),
        ));
    }

    cases
}

fn infer_and_package(
    case: &GeneratedCase,
) -> (OperatorProgram, InferenceDiagnosticsV4, std::path::PathBuf) {
    let search = search_programs_for_mesh_pair(
        &case.source,
        &case.target,
        &ProgramSearchSettings::default(),
        true,
    )
    .unwrap_or_else(|error| panic!("{} search failed: {error}", case.name));
    let selected_index = search.selected_hypothesis_index.unwrap();
    let selected = &search.hypotheses[selected_index];
    let diagnostics = search.diagnostics.unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    let package_dir = temp_dir.keep().join(case.name.replace(' ', "-"));
    build_v3_package_from_program_with_diagnostics(
        &selected.program,
        &case.source,
        &case.target,
        &package_dir,
        Some(diagnostics.clone()),
    )
    .unwrap_or_else(|error| panic!("{} package failed: {error}", case.name));
    (selected.program.clone(), diagnostics, package_dir)
}

fn verify_package_contract(
    case: &GeneratedCase,
    program: &OperatorProgram,
    diagnostics: &InferenceDiagnosticsV4,
    package_dir: &Path,
) {
    let report = verify_decompile_package_v3(package_dir)
        .unwrap_or_else(|error| panic!("{} package verification failed: {error}", case.name));
    assert_eq!(report.schema_version, 3);
    assert!(report.topology_exact);
    assert!(report.topology_hash_matches_manifest);
    assert!(report.positions_bit_exact);
    assert!(report.semantic_stage_reports_passed);
    assert_eq!(report.vertex_count, case.source.positions.len());
    assert_eq!(report.triangle_count, case.source.indices.len() / 3);

    let package = read_decompile_package_v3(package_dir).unwrap();
    assert_eq!(package.semantic_program, *program);
    assert_eq!(package.manifest.schema_version, 3);
    assert_eq!(
        package.manifest.operators.len(),
        program.operators.len() + 1
    );
    assert_eq!(diagnostics.diagnostics_schema_version, 4);
    assert_eq!(diagnostics.package_schema_version, 3);
    assert!(diagnostics.program_hypotheses.len() >= 2);
    assert!(
        diagnostics
            .timing_by_phase_ms
            .contains_key("program_scoring_ms")
    );

    let final_stage = package.manifest.operators.last().unwrap();
    let final_stage_file = match final_stage {
        OperatorManifestV3::LosslessCorrection { stage, .. } => &stage.baked_positions_file,
        _ => panic!("{} final operator was not lossless", case.name),
    };
    let final_positions = read_positions_payload(&package_dir.join(final_stage_file));
    assert_positions_bit_equal(&final_positions, &case.target.positions, case.name);
}

fn assert_deterministic_ambiguous_selection(
    case: &GeneratedCase,
    program: &OperatorProgram,
    diagnostics: &InferenceDiagnosticsV4,
) {
    let repeat = search_programs_for_mesh_pair(
        &case.source,
        &case.target,
        &ProgramSearchSettings::default(),
        true,
    )
    .unwrap();
    let repeat_program = &repeat.hypotheses[repeat.selected_hypothesis_index.unwrap()].program;
    assert_eq!(repeat_program, program);
    let scores = diagnostics
        .program_hypotheses
        .iter()
        .map(|hypothesis| hypothesis.score.total_component_sum)
        .collect::<Vec<_>>();
    assert!(
        scores.len() >= 2,
        "{} did not record competing scores",
        case.name
    );
    assert!(scores.iter().all(|score| score.is_finite()));
}

fn assert_recovered_bend(
    case: &GeneratedCase,
    program: &OperatorProgram,
    expected: BendParameters,
) {
    let mut current = case.source.positions.clone();
    for operator in &program.operators {
        if let ProgramOperator::Bend(actual) = operator {
            let axis_error =
                angle_between_degrees(actual.longitudinal_axis, expected.longitudinal_axis);
            let direction_sign = dot(actual.bend_direction, expected.bend_direction).signum();
            let signed_angle = actual.angle_radians * direction_sign;
            let angle_error = (signed_angle - expected.angle_radians).abs().to_degrees();
            let extent = longitudinal_extent(&current, expected).max(1.0e-12);
            let start_error = (actual.interval_start - expected.interval_start).abs() / extent;
            let end_error = (actual.interval_end - expected.interval_end).abs() / extent;
            assert!(
                axis_error <= 2.0,
                "{} bend axis error {axis_error} exceeded tolerance; actual={actual:?}",
                case.name
            );
            assert!(
                angle_error <= 2.0,
                "{} bend angle error {angle_error} exceeded tolerance; actual={actual:?}",
                case.name
            );
            assert!(
                start_error <= 0.05 && end_error <= 0.05,
                "{} bend interval error exceeded tolerance; actual={actual:?}",
                case.name
            );
            return;
        }
        current = evaluate_operator(operator, &current).unwrap();
    }
    panic!("{} selected program did not contain a bend", case.name);
}

fn selected_sequence(program: &OperatorProgram) -> Vec<Family> {
    program
        .operators
        .iter()
        .map(|operator| match operator {
            ProgramOperator::Bend(_) => Family::Bend,
            ProgramOperator::Affine(affine) => match affine.semantic_family {
                AffineSemanticFamily::Translation => Family::Translation,
                AffineSemanticFamily::RigidTransform => Family::Rigid,
                AffineSemanticFamily::SimilarityTransform => Family::Similarity,
                AffineSemanticFamily::GeneralAffine => {
                    panic!("unexpected general affine in generated corpus")
                }
            },
        })
        .collect()
}

fn sequence_matches(actual: &[Family], expected: &[Family]) -> bool {
    actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(actual, expected)| {
            (matches!(expected, Family::AnyAffine)
                && matches!(
                    actual,
                    Family::Translation | Family::Rigid | Family::Similarity
                ))
                || actual == expected
        })
}

fn case_from_ops(
    name: &'static str,
    source: TriangleMesh,
    operators: Vec<ProgramOperator>,
    expected_sequence: Option<Vec<Family>>,
    expected_bend: Option<BendParameters>,
) -> GeneratedCase {
    let target_positions = apply_sequence(&source.positions, &operators);
    GeneratedCase {
        name,
        target: mesh_with_positions(&source, target_positions),
        source,
        expected_sequence,
        expected_bend,
    }
}

fn regular_beam() -> TriangleMesh {
    let stations = (0..=20)
        .map(|index| index as f32 / 20.0)
        .collect::<Vec<_>>();
    beam_with_stations(&stations)
}

fn uneven_beam() -> TriangleMesh {
    beam_with_stations(&[
        0.0, 0.02, 0.04, 0.06, 0.08, 0.10, 0.18, 0.28, 0.40, 0.55, 0.72, 0.86, 1.0,
    ])
}

fn beam_with_stations(stations: &[f32]) -> TriangleMesh {
    let mut obj = String::new();
    for x in stations {
        obj.push_str(&format!("v {x} -0.10 -0.05\n"));
        obj.push_str(&format!("v {x} 0.10 -0.05\n"));
        obj.push_str(&format!("v {x} 0.10 0.05\n"));
        obj.push_str(&format!("v {x} -0.10 0.05\n"));
    }
    for ring in 0..stations.len() - 1 {
        let current = ring * 4 + 1;
        let next = current + 4;
        for corner in 0..4 {
            let a = current + corner;
            let b = current + (corner + 1) % 4;
            let c = next + (corner + 1) % 4;
            let d = next + corner;
            obj.push_str(&format!("f {a} {b} {c}\n"));
            obj.push_str(&format!("f {a} {c} {d}\n"));
        }
    }
    let last = (stations.len() - 1) * 4 + 1;
    obj.push_str("f 1 2 3\nf 1 3 4\n");
    obj.push_str(&format!("f {last} {} {}\n", last + 2, last + 1));
    obj.push_str(&format!("f {last} {} {}\n", last + 3, last + 2));
    read_obj(Cursor::new(obj)).unwrap()
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

fn bend_operator(parameters: BendParameters) -> ProgramOperator {
    ProgramOperator::Bend(parameters)
}

fn translation_operator(offset: [f32; 3]) -> ProgramOperator {
    ProgramOperator::Affine(AffineOperator {
        semantic_family: AffineSemanticFamily::Translation,
        matrix_row_major_4x4: [
            1.0, 0.0, 0.0, offset[0], 0.0, 1.0, 0.0, offset[1], 0.0, 0.0, 1.0, offset[2], 0.0, 0.0,
            0.0, 1.0,
        ],
        translation: Some(offset),
        rotation_row_major_3x3: None,
        uniform_scale: None,
    })
}

fn rigid_z_operator(angle: f32, translation: [f32; 3]) -> ProgramOperator {
    ProgramOperator::Affine(AffineOperator {
        semantic_family: AffineSemanticFamily::RigidTransform,
        matrix_row_major_4x4: rigid_matrix(angle, translation),
        translation: Some(translation),
        rotation_row_major_3x3: Some(rotation_z_3x3(angle)),
        uniform_scale: None,
    })
}

fn similarity_z_operator(angle: f32, scale: f32, translation: [f32; 3]) -> ProgramOperator {
    let rotation = rotation_z_3x3(angle);
    ProgramOperator::Affine(AffineOperator {
        semantic_family: AffineSemanticFamily::SimilarityTransform,
        matrix_row_major_4x4: [
            rotation[0] * scale,
            rotation[1] * scale,
            rotation[2] * scale,
            translation[0],
            rotation[3] * scale,
            rotation[4] * scale,
            rotation[5] * scale,
            translation[1],
            rotation[6] * scale,
            rotation[7] * scale,
            rotation[8] * scale,
            translation[2],
            0.0,
            0.0,
            0.0,
            1.0,
        ],
        translation: Some(translation),
        rotation_row_major_3x3: Some(rotation),
        uniform_scale: Some(scale),
    })
}

fn rigid_matrix(angle: f32, translation: [f32; 3]) -> [f32; 16] {
    let rotation = rotation_z_3x3(angle);
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

fn rotation_z_3x3(angle: f32) -> [f32; 9] {
    let (sin, cos) = angle.sin_cos();
    [cos, -sin, 0.0, sin, cos, 0.0, 0.0, 0.0, 1.0]
}

fn apply_sequence(source: &[[f32; 3]], operators: &[ProgramOperator]) -> Vec<[f32; 3]> {
    operators
        .iter()
        .fold(source.to_vec(), |positions, operator| {
            evaluate_operator(operator, &positions).unwrap()
        })
}

fn mesh_with_positions(source: &TriangleMesh, positions: Vec<[f32; 3]>) -> TriangleMesh {
    let mut target = source.clone();
    target.positions = positions;
    target
}

fn transform_mesh(source: &TriangleMesh, transform: impl Fn([f32; 3]) -> [f32; 3]) -> TriangleMesh {
    mesh_with_positions(
        source,
        source.positions.iter().copied().map(transform).collect(),
    )
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

fn transform_bend_similarity(
    bend: BendParameters,
    angle: f32,
    scale: f32,
    translation: [f32; 3],
) -> BendParameters {
    BendParameters {
        origin: translate(
            scale_position(rotate_z(bend.origin, angle), scale),
            translation,
        ),
        longitudinal_axis: rotate_z(bend.longitudinal_axis, angle),
        bend_direction: rotate_z(bend.bend_direction, angle),
        interval_start: bend.interval_start * scale,
        interval_end: bend.interval_end * scale,
        ..bend
    }
}

fn translate_bend(bend: BendParameters, offset: [f32; 3]) -> BendParameters {
    BendParameters {
        origin: translate(bend.origin, offset),
        ..bend
    }
}

fn scale_bend(bend: BendParameters, scale: f32) -> BendParameters {
    BendParameters {
        origin: scale_position(bend.origin, scale),
        interval_start: bend.interval_start * scale,
        interval_end: bend.interval_end * scale,
        ..bend
    }
}

fn longitudinal_extent(positions: &[[f32; 3]], bend: BendParameters) -> f32 {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for position in positions {
        let relative = [
            position[0] - bend.origin[0],
            position[1] - bend.origin[1],
            position[2] - bend.origin[2],
        ];
        let projection = dot(relative, bend.longitudinal_axis);
        min = min.min(projection);
        max = max.max(projection);
    }
    max - min
}

fn angle_between_degrees(left: [f32; 3], right: [f32; 3]) -> f32 {
    dot(left, right).abs().clamp(-1.0, 1.0).acos().to_degrees()
}

fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn read_positions_payload(path: &Path) -> Vec<[f32; 3]> {
    let bytes = fs::read(path).unwrap();
    bytes
        .chunks_exact(12)
        .map(|chunk| {
            [
                f32::from_le_bytes(chunk[0..4].try_into().unwrap()),
                f32::from_le_bytes(chunk[4..8].try_into().unwrap()),
                f32::from_le_bytes(chunk[8..12].try_into().unwrap()),
            ]
        })
        .collect()
}

fn assert_positions_bit_equal(left: &[[f32; 3]], right: &[[f32; 3]], name: &str) {
    assert_eq!(left.len(), right.len(), "{name} position count differs");
    for (index, (left, right)) in left.iter().zip(right).enumerate() {
        for axis in 0..3 {
            assert_eq!(
                left[axis].to_bits(),
                right[axis].to_bits(),
                "{name} final position {index}.{axis} differed"
            );
        }
    }
}

fn corrupt_first_bend_stage(package_dir: &Path) {
    let package = read_decompile_package_v3(package_dir).unwrap();
    let path = package
        .manifest
        .operators
        .iter()
        .find_map(|operator| match operator {
            OperatorManifestV3::Bend { stage, .. } => Some(stage.baked_positions_file.clone()),
            _ => None,
        })
        .unwrap();
    let path = package_dir.join(path);
    let mut bytes = fs::read(&path).unwrap();
    bytes[0] ^= 1;
    fs::write(path, bytes).unwrap();
}

fn remove_first_stage_payload(package_dir: &Path) {
    let package = read_decompile_package_v3(package_dir).unwrap();
    let first_stage = match &package.manifest.operators[0] {
        OperatorManifestV3::Affine { stage, .. }
        | OperatorManifestV3::Bend { stage, .. }
        | OperatorManifestV3::LosslessCorrection { stage, .. } => {
            stage.baked_positions_file.clone()
        }
    };
    fs::remove_file(package_dir.join(first_stage)).unwrap();
}

fn replace_first_operator_with_unknown(package_dir: &Path) {
    let manifest_path = package_dir.join("manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["operators"][0]["kind"] = serde_json::Value::String("mystery_operator".to_owned());
    fs::write(
        manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
}
