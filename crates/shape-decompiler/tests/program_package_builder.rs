#![forbid(unsafe_code)]

use std::io::Cursor;

use shape_decompiler::v3::decompile::{
    DecompilePackageV3, build_v3_package_from_program, validate_decompile_package_v3,
};
use shape_decompiler::v3::package::{OperatorManifestV3, StageManifestV3};
use shape_decompiler::v3::program::{
    AffineOperator, OperatorProgram, ProgramOperator, SemanticVerificationMode, evaluate_operator,
};
use shape_decompiler::{AffineSemanticFamily, DecompileError, DecompileSettings};
use shape_mesh::{TriangleMesh, read_obj};

#[test]
fn empty_program_builds_lossless_only_package() {
    let source = square_mesh();
    let target = sparse_residual_target(&source);
    let program = OperatorProgram {
        operators: Vec::new(),
    };

    let package = build_package(&source, &target, &program);

    assert!(package.semantic_program.operators.is_empty());
    assert_eq!(package.manifest.operators.len(), 1);
    assert_eq!(package.stage_payloads.len(), 1);
    assert_eq!(
        package.manifest.operators[0].stage().baked_positions_file,
        "operators/0000-lossless-correction-positions.f32"
    );
    assert_eq!(
        package.manifest.operators[0].stage().operator_id.0,
        "op-0000-lossless-correction"
    );
    assert_positions_bit_equal(&package.final_positions, &target.positions);
    assert_topology(&package, &source);
}

#[test]
fn affine_program_builds_bit_exact_stage() {
    let source = square_mesh();
    let program = OperatorProgram {
        operators: vec![translation_operator([0.25, -0.5, 1.5])],
    };
    let target = target_from_program(&source, &program);

    let package = build_package(&source, &target, &program);
    let stage = package.manifest.operators[0].stage();

    assert_eq!(stage.operator_id.0, "op-0000-translation");
    assert_eq!(
        stage.baked_positions_file,
        "operators/0000-translation-positions.f32"
    );
    assert_eq!(
        stage.semantic_verification_policy.mode,
        SemanticVerificationMode::BitExact
    );
    assert!(stage.semantic_verification_report.passed);
    assert_eq!(stage.semantic_verification_report.max_component_error, 0.0);
    assert_eq!(package.residual_indices, Vec::<u32>::new());
    assert_positions_bit_equal(&package.final_positions, &target.positions);
}

#[test]
fn bend_program_uses_explicit_parameters_with_tolerance_report() {
    let source = square_mesh();
    let bend = bend_operator();
    let program = OperatorProgram {
        operators: vec![bend],
    };
    let target = target_from_program(&source, &program);

    let package = build_package(&source, &target, &program);
    let OperatorManifestV3::Bend { parameters, stage } = &package.manifest.operators[0] else {
        panic!("expected bend operator");
    };

    assert_eq!(*parameters, bend_parameters());
    assert_eq!(stage.operator_id.0, "op-0000-bend");
    assert_eq!(
        stage.baked_positions_file,
        "operators/0000-bend-positions.f32"
    );
    assert_eq!(
        stage.semantic_verification_policy.mode,
        SemanticVerificationMode::Tolerance
    );
    assert!(stage.semantic_verification_report.passed);
    assert_eq!(stage.semantic_verification_report.max_component_error, 0.0);
    assert_eq!(stage.semantic_verification_report.rms_euclidean_error, 0.0);
    assert_positions_bit_equal(&package.final_positions, &target.positions);
}

#[test]
fn affine_then_bend_preserves_program_order_and_paths() {
    let source = square_mesh();
    let program = OperatorProgram {
        operators: vec![translation_operator([0.5, 0.25, 0.0]), bend_operator()],
    };
    let target = target_from_program(&source, &program);

    let package = build_package(&source, &target, &program);

    assert_stage(
        &package,
        0,
        "op-0000-translation",
        "operators/0000-translation-positions.f32",
    );
    assert_stage(
        &package,
        1,
        "op-0001-bend",
        "operators/0001-bend-positions.f32",
    );
    assert_stage(
        &package,
        2,
        "op-0002-lossless-correction",
        "operators/0002-lossless-correction-positions.f32",
    );
    assert_positions_bit_equal(&package.final_positions, &target.positions);
}

#[test]
fn bend_then_affine_preserves_program_order_and_paths() {
    let source = square_mesh();
    let program = OperatorProgram {
        operators: vec![bend_operator(), translation_operator([0.5, 0.25, 0.0])],
    };
    let target = target_from_program(&source, &program);

    let package = build_package(&source, &target, &program);

    assert_stage(
        &package,
        0,
        "op-0000-bend",
        "operators/0000-bend-positions.f32",
    );
    assert_stage(
        &package,
        1,
        "op-0001-translation",
        "operators/0001-translation-positions.f32",
    );
    assert_stage(
        &package,
        2,
        "op-0002-lossless-correction",
        "operators/0002-lossless-correction-positions.f32",
    );
    assert_positions_bit_equal(&package.final_positions, &target.positions);
}

#[test]
fn final_correction_can_be_empty() {
    let source = square_mesh();
    let program = OperatorProgram {
        operators: vec![translation_operator([0.25, -0.5, 1.5])],
    };
    let target = target_from_program(&source, &program);

    let package = build_package(&source, &target, &program);

    assert!(package.residual_indices.is_empty());
    assert!(package.residual_positions.is_empty());
    assert_eq!(
        package
            .manifest
            .operators
            .last()
            .unwrap()
            .stage()
            .baked_positions_file,
        "operators/0001-lossless-correction-positions.f32"
    );
    assert_positions_bit_equal(&package.final_positions, &target.positions);
}

#[test]
fn sparse_correction_stores_strict_indices_and_absolute_targets() {
    let source = square_mesh();
    let program = OperatorProgram {
        operators: vec![translation_operator([0.25, -0.5, 1.5])],
    };
    let mut target = target_from_program(&source, &program);
    target.positions[2][2] += 0.5;

    let package = build_package(&source, &target, &program);

    assert_eq!(package.residual_indices, vec![2]);
    assert_eq!(package.residual_positions, vec![target.positions[2]]);
    assert_positions_bit_equal(&package.final_positions, &target.positions);
}

#[test]
fn dense_correction_stores_every_target_position() {
    let source = square_mesh();
    let target = transformed_mesh(&source, |position| {
        [position[0] + 1.0, position[1] - 2.0, position[2] + 3.0]
    });
    let program = OperatorProgram {
        operators: Vec::new(),
    };

    let package = build_package(&source, &target, &program);

    assert_eq!(package.residual_indices, vec![0, 1, 2, 3]);
    assert_eq!(package.residual_positions, target.positions);
    assert_positions_bit_equal(&package.final_positions, &target.positions);
}

#[test]
fn bad_program_is_rejected() {
    let source = square_mesh();
    let target = source.clone();
    let program = OperatorProgram {
        operators: vec![ProgramOperator::Affine(AffineOperator {
            semantic_family: AffineSemanticFamily::Translation,
            matrix_row_major_4x4: [
                1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 2.0, 0.0, 0.0, 1.0, 3.0, 0.0, 0.0, 0.0, 2.0,
            ],
            translation: Some([1.0, 2.0, 3.0]),
            rotation_row_major_3x3: None,
            uniform_scale: None,
        })],
    };

    let error =
        build_v3_package_from_program(&source, &target, &program, DecompileSettings::default())
            .unwrap_err();

    assert!(matches!(error, DecompileError::InvalidPackage { .. }));
}

#[test]
fn bad_stage_path_is_rejected() {
    let source = square_mesh();
    let program = OperatorProgram {
        operators: vec![translation_operator([0.25, -0.5, 1.5])],
    };
    let target = target_from_program(&source, &program);
    let mut package = build_package(&source, &target, &program);
    package.manifest.operators[0]
        .stage_mut()
        .baked_positions_file = "operators/0000-affine-positions.f32".to_owned();

    let error = validate_decompile_package_v3(&package, &source, &target).unwrap_err();

    assert!(matches!(error, DecompileError::InvalidPackage { .. }));
}

#[test]
fn exact_final_positions_and_topology_are_reported() {
    let source = square_mesh();
    let target = sparse_residual_target(&source);
    let program = OperatorProgram {
        operators: Vec::new(),
    };

    let package = build_package(&source, &target, &program);
    let report = package.package_verification.as_ref().unwrap();

    assert_topology(&package, &source);
    assert!(report.topology_exact);
    assert!(report.topology_hash_matches_manifest);
    assert!(report.positions_bit_exact);
    assert_eq!(report.max_component_error, 0.0);
    assert_eq!(report.max_euclidean_error, 0.0);
    assert_eq!(report.outside_tolerance, 0);
    assert_positions_bit_equal(&package.final_positions, &target.positions);
}

#[test]
fn stage_payload_count_equals_manifest_operator_count() {
    let source = square_mesh();
    let program = OperatorProgram {
        operators: vec![translation_operator([0.25, -0.5, 1.5]), bend_operator()],
    };
    let target = target_from_program(&source, &program);

    let package = build_package(&source, &target, &program);

    assert_eq!(
        package.stage_payloads.len(),
        package.manifest.operators.len()
    );
}

#[test]
fn stable_ids_and_paths_use_affine_family_slugs() {
    let source = square_mesh();
    let cases = [
        (
            translation_operator([0.25, -0.5, 1.5]),
            "op-0000-translation",
            "operators/0000-translation-positions.f32",
        ),
        (
            rigid_operator(
                [0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
                [0.25, -0.5, 1.5],
            ),
            "op-0000-rigid-transform",
            "operators/0000-rigid-transform-positions.f32",
        ),
        (
            similarity_operator(
                [0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
                2.0,
                [0.25, -0.5, 1.5],
            ),
            "op-0000-similarity-transform",
            "operators/0000-similarity-transform-positions.f32",
        ),
        (
            general_affine_operator([
                1.2, 0.25, 0.0, -0.5, 0.1, 0.8, 0.0, 0.75, 0.0, 0.0, 1.0, 0.25, 0.0, 0.0, 0.0, 1.0,
            ]),
            "op-0000-general-affine",
            "operators/0000-general-affine-positions.f32",
        ),
    ];

    for (operator, expected_id, expected_path) in cases {
        let program = OperatorProgram {
            operators: vec![operator],
        };
        let target = target_from_program(&source, &program);
        let package = build_package(&source, &target, &program);

        assert_stage(&package, 0, expected_id, expected_path);
    }
}

fn build_package(
    source: &TriangleMesh,
    target: &TriangleMesh,
    program: &OperatorProgram,
) -> DecompilePackageV3 {
    build_v3_package_from_program(source, target, program, DecompileSettings::default()).unwrap()
}

fn assert_stage(
    package: &DecompilePackageV3,
    index: usize,
    expected_id: &str,
    expected_path: &str,
) {
    let stage = package.manifest.operators[index].stage();
    assert_eq!(stage.operator_id.0, expected_id);
    assert_eq!(stage.baked_positions_file, expected_path);
    assert_eq!(package.stage_payloads[index].operator_id.0, expected_id);
    assert_eq!(package.stage_payloads[index].positions_file, expected_path);
}

fn assert_topology(package: &DecompilePackageV3, source: &TriangleMesh) {
    assert_eq!(
        package.manifest.topology.vertex_count,
        source.positions.len()
    );
    assert_eq!(package.manifest.topology.index_count, source.indices.len());
    assert_eq!(
        package.manifest.topology.triangle_count,
        source.indices.len() / 3
    );
}

fn assert_positions_bit_equal(left: &[[f32; 3]], right: &[[f32; 3]]) {
    assert_eq!(left.len(), right.len());
    for (left_position, right_position) in left.iter().zip(right) {
        for component in 0..3 {
            assert_eq!(
                left_position[component].to_bits(),
                right_position[component].to_bits()
            );
        }
    }
}

fn square_mesh() -> TriangleMesh {
    read_obj(Cursor::new(
        "\
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
f 1 2 3
f 1 3 4
",
    ))
    .expect("square mesh")
}

fn target_from_program(source: &TriangleMesh, program: &OperatorProgram) -> TriangleMesh {
    let mut positions = source.positions.clone();
    for operator in &program.operators {
        positions = evaluate_operator(operator, &positions).unwrap();
    }
    let mut target = source.clone();
    target.positions = positions;
    target
}

fn sparse_residual_target(source: &TriangleMesh) -> TriangleMesh {
    let mut target = source.clone();
    target.positions[2] = [1.25, 0.75, 0.5];
    target
}

fn transformed_mesh(
    source: &TriangleMesh,
    transform: impl Fn([f32; 3]) -> [f32; 3],
) -> TriangleMesh {
    let mut target = source.clone();
    target.positions = source.positions.iter().copied().map(transform).collect();
    target
}

fn translation_operator(offset: [f32; 3]) -> ProgramOperator {
    ProgramOperator::Affine(AffineOperator {
        semantic_family: AffineSemanticFamily::Translation,
        matrix_row_major_4x4: translation_matrix(offset),
        translation: Some(offset),
        rotation_row_major_3x3: None,
        uniform_scale: None,
    })
}

fn rigid_operator(rotation: [f32; 9], translation: [f32; 3]) -> ProgramOperator {
    ProgramOperator::Affine(AffineOperator {
        semantic_family: AffineSemanticFamily::RigidTransform,
        matrix_row_major_4x4: rigid_matrix(rotation, translation),
        translation: Some(translation),
        rotation_row_major_3x3: Some(rotation),
        uniform_scale: None,
    })
}

fn similarity_operator(
    rotation: [f32; 9],
    uniform_scale: f32,
    translation: [f32; 3],
) -> ProgramOperator {
    ProgramOperator::Affine(AffineOperator {
        semantic_family: AffineSemanticFamily::SimilarityTransform,
        matrix_row_major_4x4: similarity_matrix(rotation, uniform_scale, translation),
        translation: Some(translation),
        rotation_row_major_3x3: Some(rotation),
        uniform_scale: Some(uniform_scale),
    })
}

fn general_affine_operator(matrix: [f32; 16]) -> ProgramOperator {
    ProgramOperator::Affine(AffineOperator {
        semantic_family: AffineSemanticFamily::GeneralAffine,
        matrix_row_major_4x4: matrix,
        translation: None,
        rotation_row_major_3x3: None,
        uniform_scale: None,
    })
}

fn bend_operator() -> ProgramOperator {
    ProgramOperator::Bend(bend_parameters())
}

fn bend_parameters() -> shape_decompiler::v3::bend::BendParameters {
    shape_decompiler::v3::bend::BendParameters {
        origin: [0.0, 0.0, 0.0],
        longitudinal_axis: [0.0, 1.0, 0.0],
        bend_direction: [1.0, 0.0, 0.0],
        angle_radians: 0.5,
        interval_start: 0.0,
        interval_end: 1.0,
    }
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

fn similarity_matrix(rotation: [f32; 9], scale: f32, translation: [f32; 3]) -> [f32; 16] {
    [
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
    ]
}

trait OperatorStage {
    fn stage(&self) -> &StageManifestV3;
    fn stage_mut(&mut self) -> &mut StageManifestV3;
}

impl OperatorStage for OperatorManifestV3 {
    fn stage(&self) -> &StageManifestV3 {
        match self {
            OperatorManifestV3::Affine { stage, .. }
            | OperatorManifestV3::Bend { stage, .. }
            | OperatorManifestV3::LosslessCorrection { stage, .. } => stage,
        }
    }

    fn stage_mut(&mut self) -> &mut StageManifestV3 {
        match self {
            OperatorManifestV3::Affine { stage, .. }
            | OperatorManifestV3::Bend { stage, .. }
            | OperatorManifestV3::LosslessCorrection { stage, .. } => stage,
        }
    }
}
