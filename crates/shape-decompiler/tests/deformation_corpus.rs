#![forbid(unsafe_code)]

use std::io::Cursor;

use shape_decompiler::{AffineSemanticFamily, DecompileSettings, OperatorManifest, decompile_pair};
use shape_mesh::{TriangleMesh, read_obj};

fn cube_mesh() -> TriangleMesh {
    read_obj(Cursor::new(
        "\
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
v 0 0 1
v 1 0 1
v 1 1 1
v 0 1 1
f 1 2 3
f 1 3 4
f 5 7 6
f 5 8 7
f 1 5 6
f 1 6 2
f 2 6 7
f 2 7 3
f 3 7 8
f 3 8 4
f 4 8 5
f 4 5 1
",
    ))
    .expect("cube mesh")
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

fn transformed_mesh(
    source: &TriangleMesh,
    transform: impl Fn([f32; 3]) -> [f32; 3],
) -> TriangleMesh {
    let mut target = source.clone();
    target.positions = source.positions.iter().copied().map(transform).collect();
    target
}

fn scaled_mesh(source: &TriangleMesh, scale: f32) -> TriangleMesh {
    transformed_mesh(source, |position| {
        [
            position[0] * scale,
            position[1] * scale,
            position[2] * scale,
        ]
    })
}

#[test]
fn corpus_translation_prefers_translation_metadata() {
    let source = cube_mesh();
    let target = transformed_mesh(&source, |position| {
        [position[0] + 0.25, position[1] - 0.5, position[2] + 1.5]
    });

    let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

    let Some(OperatorManifest::GlobalAffine {
        semantic_family,
        translation,
        ..
    }) = result.manifest.operators.first()
    else {
        panic!("expected affine operator");
    };
    assert_eq!(*semantic_family, AffineSemanticFamily::Translation);
    assert_eq!(*translation, Some([0.25, -0.5, 1.5]));
    assert!(result.residual_indices.is_empty());
    assert_eq!(result.reconstructed_positions, target.positions);
}

#[test]
fn corpus_rigid_transform_prefers_rigid_metadata() {
    let source = cube_mesh();
    let target = transformed_mesh(&source, |position| {
        [-position[1] + 0.25, position[0] - 0.5, position[2] + 1.5]
    });

    let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

    let Some(OperatorManifest::GlobalAffine {
        semantic_family,
        translation,
        rotation_row_major_3x3,
        uniform_scale,
        ..
    }) = result.manifest.operators.first()
    else {
        panic!("expected affine operator");
    };
    assert_eq!(*semantic_family, AffineSemanticFamily::RigidTransform);
    assert_eq!(*translation, Some([0.25, -0.5, 1.5]));
    assert_eq!(
        *rotation_row_major_3x3,
        Some([0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0])
    );
    assert_eq!(*uniform_scale, None);
    assert_eq!(result.reconstructed_positions, target.positions);
}

#[test]
fn corpus_large_translation_does_not_hide_rigid_transform() {
    let source = cube_mesh();
    let target = transformed_mesh(&source, |position| {
        [
            -position[1] + 1000.0,
            position[0] - 1000.0,
            position[2] + 500.0,
        ]
    });

    let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

    let Some(OperatorManifest::GlobalAffine {
        semantic_family,
        translation,
        ..
    }) = result.manifest.operators.first()
    else {
        panic!("expected affine operator");
    };
    assert_eq!(*semantic_family, AffineSemanticFamily::RigidTransform);
    assert_eq!(*translation, Some([1000.0, -1000.0, 500.0]));
    assert!(result.residual_indices.is_empty());
    assert_eq!(result.reconstructed_positions, target.positions);
}

#[test]
fn corpus_scoring_keeps_coherent_rotation_despite_large_translation() {
    let source = cube_mesh();
    let target = transformed_mesh(&source, |position| {
        [
            0.6 * position[0] - 0.8 * position[1] + 1000.0,
            0.8 * position[0] + 0.6 * position[1] - 1000.0,
            position[2] + 500.0,
        ]
    });

    let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

    let Some(OperatorManifest::GlobalAffine {
        semantic_family, ..
    }) = result.manifest.operators.first()
    else {
        panic!("expected affine operator");
    };
    assert_eq!(*semantic_family, AffineSemanticFamily::RigidTransform);
    assert_eq!(result.reconstructed_positions, target.positions);
}

#[test]
fn corpus_planar_rigid_transform_prefers_rigid_metadata() {
    let source = square_mesh();
    let target = transformed_mesh(&source, |position| {
        [-position[1] + 0.25, position[0] - 0.5, position[2]]
    });

    let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

    let Some(OperatorManifest::GlobalAffine {
        semantic_family,
        translation,
        ..
    }) = result.manifest.operators.first()
    else {
        panic!("expected affine operator");
    };
    assert_eq!(*semantic_family, AffineSemanticFamily::RigidTransform);
    assert_eq!(*translation, Some([0.25, -0.5, 0.0]));
    assert!(result.residual_indices.is_empty());
    assert_eq!(result.reconstructed_positions, target.positions);
}

#[test]
fn corpus_generated_coordinate_scales_keep_family_stable() {
    let base = cube_mesh();
    for scale in [1.0e-3_f32, 1.0, 1.0e3] {
        let source = scaled_mesh(&base, scale);
        let target = transformed_mesh(&source, |position| {
            [
                -position[1] + 0.25 * scale,
                position[0] - 0.5 * scale,
                position[2] + 1.5 * scale,
            ]
        });

        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

        let Some(OperatorManifest::GlobalAffine {
            semantic_family, ..
        }) = result.manifest.operators.first()
        else {
            panic!("expected affine operator at scale {scale}");
        };
        assert_eq!(
            *semantic_family,
            AffineSemanticFamily::RigidTransform,
            "unexpected family at scale {scale}"
        );
        assert_eq!(result.reconstructed_positions, target.positions);
    }
}

#[test]
fn corpus_near_unit_scale_uses_similarity_when_it_saves_residuals() {
    let source = cube_mesh();
    let target = transformed_mesh(&source, |position| {
        [
            -1.01 * position[1] + 0.25,
            1.01 * position[0] - 0.5,
            1.01 * position[2] + 1.5,
        ]
    });

    let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

    let Some(OperatorManifest::GlobalAffine {
        semantic_family, ..
    }) = result.manifest.operators.first()
    else {
        panic!("expected affine operator");
    };
    assert_eq!(*semantic_family, AffineSemanticFamily::SimilarityTransform);
    assert_eq!(result.reconstructed_positions, target.positions);
}

#[test]
fn corpus_similarity_transform_prefers_similarity_metadata() {
    let source = cube_mesh();
    let target = transformed_mesh(&source, |position| {
        [
            -2.0 * position[1] + 0.25,
            2.0 * position[0] - 0.5,
            2.0 * position[2] + 1.5,
        ]
    });

    let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

    let Some(OperatorManifest::GlobalAffine {
        semantic_family,
        translation,
        rotation_row_major_3x3,
        uniform_scale,
        ..
    }) = result.manifest.operators.first()
    else {
        panic!("expected affine operator");
    };
    assert_eq!(*semantic_family, AffineSemanticFamily::SimilarityTransform);
    assert_eq!(*translation, Some([0.25, -0.5, 1.5]));
    assert_eq!(
        *rotation_row_major_3x3,
        Some([0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0])
    );
    assert_eq!(*uniform_scale, Some(2.0));
    assert_eq!(result.reconstructed_positions, target.positions);
}

#[test]
fn corpus_non_translation_affine_stays_general_affine() {
    let source = cube_mesh();
    let target = transformed_mesh(&source, |position| {
        [
            position[0] * 2.0 + position[1] * 0.125,
            position[1] * 0.5 - 0.25,
            position[2] + position[0] * 0.25,
        ]
    });

    let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

    let Some(OperatorManifest::GlobalAffine {
        semantic_family,
        translation,
        ..
    }) = result.manifest.operators.first()
    else {
        panic!("expected affine operator");
    };
    assert_eq!(*semantic_family, AffineSemanticFamily::GeneralAffine);
    assert_eq!(*translation, None);
    assert_eq!(result.reconstructed_positions, target.positions);
}

#[test]
fn corpus_residual_fallback_is_lossless_when_affine_is_rejected() {
    let source = cube_mesh();
    let mut target = source.clone();
    target.positions[6] = [1.25, 0.75, 1.5];
    let settings = DecompileSettings {
        affine_min_explained: 1.0,
        ..DecompileSettings::default()
    };

    let result = decompile_pair(&source, &target, settings).unwrap();

    assert!(matches!(
        result.manifest.operators.first(),
        Some(OperatorManifest::LosslessCorrection { .. })
    ));
    assert_eq!(result.residual_indices, vec![6]);
    assert_eq!(result.residual_positions, vec![target.positions[6]]);
    assert_eq!(result.reconstructed_positions, target.positions);
}

#[test]
fn corpus_local_edit_prefers_lossless_only_over_affine_overfit() {
    let source = cube_mesh();
    let mut target = source.clone();
    target.positions[6] = [1.25, 0.75, 1.5];

    let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

    assert!(matches!(
        result.manifest.operators.first(),
        Some(OperatorManifest::LosslessCorrection { .. })
    ));
    assert_eq!(result.residual_indices, vec![6]);
    assert_eq!(result.residual_positions, vec![target.positions[6]]);
    assert_eq!(result.reconstructed_positions, target.positions);
}
