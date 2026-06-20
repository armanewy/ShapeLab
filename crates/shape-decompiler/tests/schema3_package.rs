#![forbid(unsafe_code)]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use shape_decompiler::v3::package::{
    DecompileManifestV3, OperatorManifestV3, StageManifestV3, read_decompile_package_v3,
    verify_decompile_package_v3, write_decompile_package_v3,
};
use shape_decompiler::v3::program::{
    AffineOperator, OperatorProgram, ProgramOperator, SemanticVerificationMode,
};
use shape_decompiler::{AffineSemanticFamily, DecompileError, verify_decompile_package};
use shape_mesh::{TriangleMesh, read_obj};

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

fn transformed_mesh(
    source: &TriangleMesh,
    transform: impl Fn([f32; 3]) -> [f32; 3],
) -> TriangleMesh {
    let mut target = source.clone();
    target.positions = source.positions.iter().copied().map(transform).collect();
    target
}

fn translation_program(offset: [f32; 3]) -> OperatorProgram {
    OperatorProgram {
        operators: vec![translation_operator(offset)],
    }
}

fn rigid_program(rotation: [f32; 9], translation: [f32; 3]) -> OperatorProgram {
    OperatorProgram {
        operators: vec![ProgramOperator::Affine(AffineOperator {
            semantic_family: AffineSemanticFamily::RigidTransform,
            matrix_row_major_4x4: rigid_matrix(rotation, translation),
            translation: Some(translation),
            rotation_row_major_3x3: Some(rotation),
            uniform_scale: None,
        })],
    }
}

fn similarity_program(
    rotation: [f32; 9],
    uniform_scale: f32,
    translation: [f32; 3],
) -> OperatorProgram {
    OperatorProgram {
        operators: vec![ProgramOperator::Affine(AffineOperator {
            semantic_family: AffineSemanticFamily::SimilarityTransform,
            matrix_row_major_4x4: similarity_matrix(rotation, uniform_scale, translation),
            translation: Some(translation),
            rotation_row_major_3x3: Some(rotation),
            uniform_scale: Some(uniform_scale),
        })],
    }
}

fn general_affine_program(matrix: [f32; 16]) -> OperatorProgram {
    OperatorProgram {
        operators: vec![ProgramOperator::Affine(AffineOperator {
            semantic_family: AffineSemanticFamily::GeneralAffine,
            matrix_row_major_4x4: matrix,
            translation: None,
            rotation_row_major_3x3: None,
            uniform_scale: None,
        })],
    }
}

fn two_affine_program(offset: [f32; 3], scale: f32) -> OperatorProgram {
    OperatorProgram {
        operators: vec![translation_operator(offset), scale_operator(scale)],
    }
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

fn apply_matrix(position: [f32; 3], matrix: [f32; 16]) -> [f32; 3] {
    [
        matrix[0] * position[0] + matrix[1] * position[1] + matrix[2] * position[2] + matrix[3],
        matrix[4] * position[0] + matrix[5] * position[1] + matrix[6] * position[2] + matrix[7],
        matrix[8] * position[0] + matrix[9] * position[1] + matrix[10] * position[2] + matrix[11],
    ]
}

fn scale_operator(scale: f32) -> ProgramOperator {
    ProgramOperator::Affine(AffineOperator {
        semantic_family: AffineSemanticFamily::GeneralAffine,
        matrix_row_major_4x4: [
            scale, 0.0, 0.0, 0.0, 0.0, scale, 0.0, 0.0, 0.0, 0.0, scale, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
        translation: None,
        rotation_row_major_3x3: None,
        uniform_scale: None,
    })
}

fn residual_target(source: &TriangleMesh) -> TriangleMesh {
    let mut target = source.clone();
    target.positions[2] = [1.25, 0.75, 0.5];
    target
}

fn translated_residual_target(source: &TriangleMesh, offset: [f32; 3]) -> TriangleMesh {
    let mut target = transformed_mesh(source, |position| {
        [
            position[0] + offset[0],
            position[1] + offset[1],
            position[2] + offset[2],
        ]
    });
    target.positions[2][2] += 0.5;
    target
}

fn two_affine_target(source: &TriangleMesh, offset: [f32; 3], scale: f32) -> TriangleMesh {
    transformed_mesh(source, |position| {
        [
            (position[0] + offset[0]) * scale,
            (position[1] + offset[1]) * scale,
            (position[2] + offset[2]) * scale,
        ]
    })
}

fn package_path(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
    }
    path
}

fn read_manifest(package: &Path) -> DecompileManifestV3 {
    serde_json::from_str(&fs::read_to_string(package.join("manifest.json")).unwrap()).unwrap()
}

fn write_manifest(package: &Path, manifest: &DecompileManifestV3) {
    fs::write(
        package.join("manifest.json"),
        serde_json::to_string_pretty(manifest).unwrap(),
    )
    .unwrap();
}

fn stage_mut(operator: &mut OperatorManifestV3) -> &mut StageManifestV3 {
    match operator {
        OperatorManifestV3::Affine { stage, .. }
        | OperatorManifestV3::Bend { stage, .. }
        | OperatorManifestV3::LosslessCorrection { stage, .. } => stage,
    }
}

fn assert_invalid_package(error: DecompileError) {
    assert!(
        matches!(error, DecompileError::InvalidPackage { .. }),
        "unexpected error: {error:?}"
    );
}

fn write_u32_payload(path: &Path, values: &[u32]) {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    fs::write(path, bytes).unwrap();
}

fn write_positions_payload(path: &Path, positions: &[[f32; 3]]) {
    let mut bytes = Vec::with_capacity(positions.len() * 12);
    for position in positions {
        for component in position {
            bytes.extend_from_slice(&component.to_le_bytes());
        }
    }
    fs::write(path, bytes).unwrap();
}

fn read_positions_payload(path: &Path) -> Vec<[f32; 3]> {
    let bytes = fs::read(path).unwrap();
    assert_eq!(bytes.len() % 12, 0);
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

fn collect_package_bytes(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(&path).unwrap() {
            let entry = entry.unwrap();
            let entry_path = entry.path();
            if entry_path.is_dir() {
                stack.push(entry_path);
            } else {
                let relative = entry_path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push((relative, fs::read(entry_path).unwrap()));
            }
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}

#[test]
fn residual_only_v3_package_roundtrips() {
    let source = square_mesh();
    let target = residual_target(&source);
    let program = OperatorProgram {
        operators: Vec::new(),
    };
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");

    write_decompile_package_v3(&program, &source, &target, &package).unwrap();
    let report = verify_decompile_package_v3(&package).unwrap();
    let read_package = read_decompile_package_v3(&package).unwrap();

    assert_eq!(report.schema_version, 3);
    assert!(report.topology_exact);
    assert!(report.positions_bit_exact);
    assert_eq!(report.operator_count, 1);
    assert_eq!(report.stage_count, 1);
    assert_eq!(report.residual_vertex_count, 1);
    assert!(read_package.semantic_program.operators.is_empty());
    assert!(read_package.package_verification.is_some());
    let root_report = verify_decompile_package(&package).unwrap();
    assert_eq!(root_report.schema_version, 3);
    assert!(root_report.positions_bit_exact);
}

#[test]
fn affine_plus_residual_v3_package_roundtrips() {
    let source = square_mesh();
    let offset = [0.25, -0.5, 1.5];
    let target = translated_residual_target(&source, offset);
    let program = translation_program(offset);
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");

    write_decompile_package_v3(&program, &source, &target, &package).unwrap();
    let report = verify_decompile_package_v3(&package).unwrap();

    assert_eq!(report.operator_count, 2);
    assert_eq!(report.stage_count, 2);
    assert_eq!(report.residual_vertex_count, 1);
    assert!(report.semantic_stage_reports_passed);
    let diagnostics: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(package.join("inference-diagnostics.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(diagnostics["diagnostics_schema_version"], 4);
    assert_eq!(diagnostics["package_schema_version"], 3);
    assert_eq!(
        diagnostics["program_hypotheses"][0]["operators"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(package.join("blender_reconstruct.py").exists());
}

#[test]
fn two_affine_stages_roundtrip_with_final_lossless_stage() {
    let source = square_mesh();
    let offset = [0.25, -0.5, 1.5];
    let scale = 2.0;
    let target = two_affine_target(&source, offset, scale);
    let program = two_affine_program(offset, scale);
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");

    write_decompile_package_v3(&program, &source, &target, &package).unwrap();
    let report = verify_decompile_package_v3(&package).unwrap();

    assert_eq!(report.operator_count, 3);
    assert_eq!(report.stage_count, 3);
    assert_eq!(report.residual_vertex_count, 0);
    assert!(report.positions_bit_exact);
}

#[test]
fn rigid_transform_v3_package_roundtrips() {
    let source = square_mesh();
    let rotation = [0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    let translation = [0.25, -0.5, 1.5];
    let matrix = rigid_matrix(rotation, translation);
    let target = transformed_mesh(&source, |position| apply_matrix(position, matrix));
    let program = rigid_program(rotation, translation);
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");

    write_decompile_package_v3(&program, &source, &target, &package).unwrap();
    let report = verify_decompile_package_v3(&package).unwrap();
    let manifest = read_manifest(&package);

    assert!(report.positions_bit_exact);
    let OperatorManifestV3::Affine { operator, .. } = &manifest.operators[0] else {
        panic!("expected affine operator");
    };
    assert_eq!(
        operator.semantic_family,
        AffineSemanticFamily::RigidTransform
    );
}

#[test]
fn similarity_transform_v3_package_roundtrips() {
    let source = square_mesh();
    let rotation = [0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    let translation = [0.25, -0.5, 1.5];
    let scale = 2.0;
    let matrix = similarity_matrix(rotation, scale, translation);
    let target = transformed_mesh(&source, |position| apply_matrix(position, matrix));
    let program = similarity_program(rotation, scale, translation);
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");

    write_decompile_package_v3(&program, &source, &target, &package).unwrap();
    let report = verify_decompile_package_v3(&package).unwrap();
    let manifest = read_manifest(&package);

    assert!(report.positions_bit_exact);
    let OperatorManifestV3::Affine { operator, .. } = &manifest.operators[0] else {
        panic!("expected affine operator");
    };
    assert_eq!(
        operator.semantic_family,
        AffineSemanticFamily::SimilarityTransform
    );
}

#[test]
fn general_affine_v3_package_roundtrips() {
    let source = square_mesh();
    let matrix = [
        1.2, 0.25, 0.0, -0.5, 0.1, 0.8, 0.0, 0.75, 0.0, 0.0, 1.0, 0.25, 0.0, 0.0, 0.0, 1.0,
    ];
    let target = transformed_mesh(&source, |position| apply_matrix(position, matrix));
    let program = general_affine_program(matrix);
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");

    write_decompile_package_v3(&program, &source, &target, &package).unwrap();
    let report = verify_decompile_package_v3(&package).unwrap();
    let manifest = read_manifest(&package);

    assert!(report.positions_bit_exact);
    let OperatorManifestV3::Affine { operator, .. } = &manifest.operators[0] else {
        panic!("expected affine operator");
    };
    assert_eq!(
        operator.semantic_family,
        AffineSemanticFamily::GeneralAffine
    );
}

#[test]
fn exact_baked_stages_and_final_positions_are_serialized() {
    let source = square_mesh();
    let offset = [0.25, -0.5, 1.5];
    let target = translated_residual_target(&source, offset);
    let program = translation_program(offset);
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");

    write_decompile_package_v3(&program, &source, &target, &package).unwrap();

    let affine_stage = read_positions_payload(&package_path(
        &package,
        "operators/0000-affine-positions.f32",
    ));
    let final_stage = read_positions_payload(&package_path(
        &package,
        "operators/0001-lossless-correction-positions.f32",
    ));
    let expected_affine = transformed_mesh(&source, |position| {
        [
            position[0] + offset[0],
            position[1] + offset[1],
            position[2] + offset[2],
        ]
    });

    assert_eq!(affine_stage, expected_affine.positions);
    assert_eq!(final_stage, target.positions);
}

#[test]
fn verifier_rejects_corrupted_intermediate_baked_stage() {
    let source = square_mesh();
    let offset = [0.25, -0.5, 1.5];
    let scale = 2.0;
    let target = two_affine_target(&source, offset, scale);
    let program = two_affine_program(offset, scale);
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");
    write_decompile_package_v3(&program, &source, &target, &package).unwrap();

    let stage_path = package_path(&package, "operators/0000-affine-positions.f32");
    let mut bytes = fs::read(&stage_path).unwrap();
    bytes[0..4].copy_from_slice(&99.0_f32.to_le_bytes());
    fs::write(stage_path, bytes).unwrap();

    assert_invalid_package(verify_decompile_package_v3(&package).unwrap_err());
}

#[test]
fn verifier_rejects_missing_baked_stage() {
    let source = square_mesh();
    let target = residual_target(&source);
    let program = OperatorProgram {
        operators: Vec::new(),
    };
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");
    write_decompile_package_v3(&program, &source, &target, &package).unwrap();

    fs::remove_file(package_path(
        &package,
        "operators/0000-lossless-correction-positions.f32",
    ))
    .unwrap();

    assert_invalid_package(verify_decompile_package_v3(&package).unwrap_err());
}

#[test]
fn verifier_rejects_duplicated_operator_id() {
    let source = square_mesh();
    let offset = [0.25, -0.5, 1.5];
    let target = translated_residual_target(&source, offset);
    let program = translation_program(offset);
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");
    write_decompile_package_v3(&program, &source, &target, &package).unwrap();

    let mut manifest = read_manifest(&package);
    let first_id = manifest.operators[0].stage().operator_id.clone();
    stage_mut(&mut manifest.operators[1]).operator_id = first_id;
    write_manifest(&package, &manifest);

    assert_invalid_package(verify_decompile_package_v3(&package).unwrap_err());
}

#[test]
fn verifier_rejects_lossless_operation_not_final() {
    let source = square_mesh();
    let offset = [0.25, -0.5, 1.5];
    let target = translated_residual_target(&source, offset);
    let program = translation_program(offset);
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");
    write_decompile_package_v3(&program, &source, &target, &package).unwrap();

    let mut manifest = read_manifest(&package);
    manifest.operators.swap(0, 1);
    stage_mut(&mut manifest.operators[0]).stage_index.0 = 0;
    stage_mut(&mut manifest.operators[1]).stage_index.0 = 1;
    write_manifest(&package, &manifest);

    assert_invalid_package(verify_decompile_package_v3(&package).unwrap_err());
}

#[test]
fn verifier_rejects_repeated_residual_index() {
    let source = square_mesh();
    let offset = [0.25, -0.5, 1.5];
    let target = translated_residual_target(&source, offset);
    let program = translation_program(offset);
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");
    write_decompile_package_v3(&program, &source, &target, &package).unwrap();

    let mut manifest = read_manifest(&package);
    let OperatorManifestV3::LosslessCorrection { correction, .. } =
        manifest.operators.last_mut().unwrap()
    else {
        panic!("expected lossless correction");
    };
    correction.corrected_vertex_count = 2;
    write_manifest(&package, &manifest);
    write_u32_payload(&package_path(&package, "residual/indices.u32"), &[2, 2]);
    write_positions_payload(
        &package_path(&package, "residual/positions.f32"),
        &[target.positions[2], target.positions[2]],
    );

    assert_invalid_package(verify_decompile_package_v3(&package).unwrap_err());
}

#[test]
fn verifier_rejects_malformed_tolerance_policy() {
    let source = square_mesh();
    let offset = [0.25, -0.5, 1.5];
    let target = translated_residual_target(&source, offset);
    let program = translation_program(offset);
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");
    write_decompile_package_v3(&program, &source, &target, &package).unwrap();

    let mut manifest = read_manifest(&package);
    let policy = &mut stage_mut(&mut manifest.operators[0]).semantic_verification_policy;
    policy.mode = SemanticVerificationMode::Tolerance;
    policy.absolute_epsilon = -1.0;
    write_manifest(&package, &manifest);

    assert_invalid_package(verify_decompile_package_v3(&package).unwrap_err());
}

#[test]
fn verifier_rejects_unsafe_path() {
    let source = square_mesh();
    let target = residual_target(&source);
    let program = OperatorProgram {
        operators: Vec::new(),
    };
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");
    write_decompile_package_v3(&program, &source, &target, &package).unwrap();

    let mut manifest = read_manifest(&package);
    manifest.source.path = "../source.meshbin".to_owned();
    write_manifest(&package, &manifest);

    assert_invalid_package(verify_decompile_package_v3(&package).unwrap_err());
}

#[test]
fn verifier_reports_exact_final_topology_and_positions() {
    let source = square_mesh();
    let target = residual_target(&source);
    let program = OperatorProgram {
        operators: Vec::new(),
    };
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");

    write_decompile_package_v3(&program, &source, &target, &package).unwrap();
    let report = verify_decompile_package_v3(&package).unwrap();

    assert!(report.topology_exact);
    assert!(report.topology_hash_matches_manifest);
    assert!(report.positions_bit_exact);
    assert_eq!(report.vertex_count, source.positions.len());
    assert_eq!(report.triangle_count, source.indices.len() / 3);
    assert_eq!(report.max_component_error, 0.0);
    assert_eq!(report.max_euclidean_error, 0.0);
    assert_eq!(report.outside_tolerance, 0);
}

#[test]
fn package_output_is_deterministic() {
    let source = square_mesh();
    let offset = [0.25, -0.5, 1.5];
    let target = translated_residual_target(&source, offset);
    let program = translation_program(offset);
    let dir = tempfile::tempdir().unwrap();
    let first_package = dir.path().join("first");
    let second_package = dir.path().join("second");

    write_decompile_package_v3(&program, &source, &target, &first_package).unwrap();
    write_decompile_package_v3(&program, &source, &target, &second_package).unwrap();

    assert_eq!(
        collect_package_bytes(&first_package),
        collect_package_bytes(&second_package)
    );
}

trait OperatorStage {
    fn stage(&self) -> &StageManifestV3;
}

impl OperatorStage for OperatorManifestV3 {
    fn stage(&self) -> &StageManifestV3 {
        match self {
            OperatorManifestV3::Affine { stage, .. }
            | OperatorManifestV3::Bend { stage, .. }
            | OperatorManifestV3::LosslessCorrection { stage, .. } => stage,
        }
    }
}
