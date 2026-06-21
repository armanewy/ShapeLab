use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    Frame3, GeometryRecipe, GeometrySource, ModelingOperationSpec, OperationId, PartDefinition,
    PartDefinitionId, PartInstanceId, RegionId, SurfaceRole,
};
use shape_modeling::{
    GeneratedPart, GeneratorContext, ModelingError,
    bevel::{
        BevelSource, SemanticEdgeClass, capability_for_source,
        compute_weighted_split_normal_groups, triangulate_with_weighted_normals,
    },
    generate_geometry,
    generators::basic::{PlateParams, build_plate},
};
use shape_poly::{
    EdgeClassification, ElementId, PolygonMesh, TriangulatedPolygonMesh, compute_face_normals,
    triangulate_polygon_mesh, validate_polygon_mesh,
};

const EPSILON: f32 = 1.0e-5;

#[test]
fn capability_dispatch_is_explicit_and_bounded() {
    let rounded = capability_for_source(&GeometrySource::RoundedBox {
        half_extents: [1.0, 0.75, 0.5],
        radius: 0.0,
    })
    .expect("rounded box capability should resolve")
    .expect("rounded boxes support bevels");
    assert_eq!(rounded.source, BevelSource::RoundedBox);
    assert_close(rounded.max_radius, 0.5);
    assert_eq!(
        rounded.semantic_edge_classes,
        vec![SemanticEdgeClass::RoundedBoxEdges]
    );

    assert!(
        capability_for_source(&GeometrySource::Plate {
            size: [2.0, 1.5],
            thickness: 0.4,
        })
        .expect("plate capability should resolve")
        .is_some()
    );
    assert!(
        capability_for_source(&GeometrySource::Cylinder {
            radius: 1.0,
            height: 1.0,
            radial_segments: 16,
        })
        .expect("cylinder capability should resolve")
        .is_some()
    );
    assert!(
        capability_for_source(&GeometrySource::Frustum {
            bottom_radius: 1.0,
            top_radius: 0.6,
            height: 1.0,
            radial_segments: 16,
        })
        .expect("frustum capability should resolve")
        .is_some()
    );
    assert!(
        capability_for_source(&GeometrySource::Sweep {
            profile: square_profile(),
            path: sweep_path(),
        })
        .expect("sweep capability should resolve")
        .is_some()
    );
    assert!(
        capability_for_source(&GeometrySource::LiteralMesh {
            positions: Vec::new(),
            faces: Vec::new(),
        })
        .expect("literal mesh capability should resolve")
        .is_none()
    );
}

#[test]
fn rounded_box_bevel_widths_preserve_regions_and_segment_topology() {
    let source = GeometrySource::RoundedBox {
        half_extents: [1.0, 0.75, 0.5],
        radius: 0.0,
    };
    let part = generate(source.clone(), vec![set_bevel(10, 0.2, 2)]);

    assert_valid_quality_mesh(&part.mesh);
    assert_region_role_faces(&part, SurfaceRole::PrimarySurface);
    assert_region_role_faces(&part, SurfaceRole::BevelBand);
    assert_region_name(&part, "corners");

    let scalar_change = generate(source.clone(), vec![set_bevel(10, 0.15, 2)]);
    assert_same_region_ids(&part, &scalar_change);
    assert_eq!(
        part.mesh.topology_signature,
        scalar_change.mesh.topology_signature
    );

    let topology_change = generate(source, vec![set_bevel(10, 0.2, 3)]);
    assert_same_region_ids(&part, &topology_change);
    assert_ne!(
        part.mesh.topology_signature,
        topology_change.mesh.topology_signature
    );
}

#[test]
fn plate_corner_and_face_bevels_have_clean_bands() {
    let params = PlateParams {
        width: 3.0,
        height: 2.0,
        thickness: 0.3,
        corner_radius: 0.3,
        corner_segments: 4,
        front_back_bevel: 0.06,
    };
    let part = build_plate(&params, &context()).expect("plate should generate");

    assert_valid_quality_mesh(&part.mesh);
    assert_region_role_faces(&part, SurfaceRole::PrimarySurface);
    assert_region_role_faces(&part, SurfaceRole::Side);
    assert_region_role_faces(&part, SurfaceRole::BevelBand);
    assert!(
        part.mesh.positions.len() > 16,
        "rounded plate corners should add deterministic perimeter samples"
    );
}

#[test]
fn cylinder_cap_bevel_preserves_hard_cap_boundaries() {
    let part = generate(
        GeometrySource::Cylinder {
            radius: 1.0,
            height: 1.2,
            radial_segments: 18,
        },
        vec![set_bevel(11, 0.12, 3)],
    );

    assert_valid_quality_mesh(&part.mesh);
    assert_region_faces(&part, RegionId(4));
    assert_region_faces(&part, RegionId(5));
    assert!(part.mesh.edge_metadata.values().any(|metadata| {
        metadata.region_transition.is_some() && metadata.classification == EdgeClassification::Hard
    }));
}

#[test]
fn sweep_profile_corner_bevel_changes_topology_without_losing_regions() {
    let source = GeometrySource::Sweep {
        profile: square_profile(),
        path: sweep_path(),
    };
    let unbeveled = generate(source.clone(), Vec::new());
    let beveled = generate(source.clone(), vec![set_bevel(12, 0.15, 2)]);
    let more_segments = generate(source, vec![set_bevel(12, 0.15, 4)]);

    assert_valid_quality_mesh(&beveled.mesh);
    assert_same_region_ids(&unbeveled, &beveled);
    assert_same_region_ids(&beveled, &more_segments);
    assert!(beveled.mesh.positions.len() > unbeveled.mesh.positions.len());
    assert_ne!(
        unbeveled.mesh.topology_signature,
        beveled.mesh.topology_signature
    );
    assert_ne!(
        beveled.mesh.topology_signature,
        more_segments.mesh.topology_signature
    );
}

#[test]
fn excessive_width_and_unsupported_topology_are_typed_errors() {
    let excessive = definition(
        GeometrySource::Cylinder {
            radius: 1.0,
            height: 0.5,
            radial_segments: 12,
        },
        vec![set_bevel(13, 0.3, 2)],
    );
    assert!(matches!(
        generate_geometry(&excessive, &mut context()),
        Err(ModelingError::InvalidInput(message)) if message.contains("exceeds safe")
    ));

    let literal = definition(
        GeometrySource::LiteralMesh {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            faces: vec![vec![0, 1, 2]],
        },
        vec![set_bevel(14, 0.05, 1)],
    );
    assert!(matches!(
        generate_geometry(&literal, &mut context()),
        Err(ModelingError::UnsupportedOperation {
            operation: OperationId(14),
            ..
        })
    ));
}

#[test]
fn zero_bevel_cleanly_disables_generated_bands() {
    let source = GeometrySource::Cylinder {
        radius: 1.0,
        height: 1.0,
        radial_segments: 12,
    };
    let unbeveled = generate(source.clone(), Vec::new());
    let zero = generate(source, vec![set_bevel(15, 0.0, 0)]);

    assert_valid_quality_mesh(&zero.mesh);
    assert_eq!(
        unbeveled.mesh.topology_signature,
        zero.mesh.topology_signature
    );
    assert_eq!(region_face_count(&zero.mesh, RegionId(4)), 0);
    assert_eq!(region_face_count(&zero.mesh, RegionId(5)), 0);
}

#[test]
fn weighted_normals_are_continuous_across_smooth_bevels() {
    let part = generate(
        GeometrySource::RoundedBox {
            half_extents: [1.0, 1.0, 1.0],
            radius: 0.0,
        },
        vec![set_bevel(16, 0.2, 3)],
    );
    let geometric_normals =
        compute_face_normals(&part.mesh).expect("geometric normals should remain available");
    let weighted =
        compute_weighted_split_normal_groups(&part.mesh).expect("weighted normals should compute");

    assert!(weighted.iter().all(|normal| unit(normal.normal)));
    let smooth_bevel_group = weighted
        .iter()
        .find(|group| {
            group.faces.len() > 1
                && group.faces.iter().any(|face| {
                    part.mesh.face_metadata[*face].surface_role == Some(SurfaceRole::BevelBand)
                })
        })
        .expect("expected a smooth bevel split-normal group");
    for face in &smooth_bevel_group.faces {
        assert!(
            dot(smooth_bevel_group.normal, geometric_normals[*face]) > 0.35,
            "weighted normal should remain compatible with contributing face normals"
        );
    }
}

#[test]
fn weighted_preview_normals_preserve_hard_boundaries() {
    let part = generate(
        GeometrySource::Cylinder {
            radius: 1.0,
            height: 1.0,
            radial_segments: 16,
        },
        vec![set_bevel(17, 0.15, 2)],
    );
    let triangulated =
        triangulate_with_weighted_normals(&part.mesh).expect("weighted triangulation should work");

    assert_no_degenerate_triangles(&triangulated);
    let mut normals_by_vertex: BTreeMap<ElementId, Vec<[f32; 3]>> = BTreeMap::new();
    for (vertex_id, normal) in triangulated
        .vertex_ids
        .iter()
        .zip(triangulated.mesh.normals.iter())
    {
        normals_by_vertex
            .entry(*vertex_id)
            .or_default()
            .push(*normal);
    }
    assert!(
        normals_by_vertex.values().any(|normals| {
            normals.iter().enumerate().any(|(index, left)| {
                normals[index + 1..]
                    .iter()
                    .any(|right| dot(*left, *right) < 0.95)
            })
        }),
        "hard cap and side boundaries should keep split preview normals"
    );
}

#[test]
fn weighted_triangulation_is_deterministic_and_non_degenerate() {
    let part = generate(
        GeometrySource::Frustum {
            bottom_radius: 1.0,
            top_radius: 0.55,
            height: 1.2,
            radial_segments: 14,
        },
        vec![set_bevel(18, 0.08, 2)],
    );
    let first =
        triangulate_with_weighted_normals(&part.mesh).expect("first triangulation should work");
    let second =
        triangulate_with_weighted_normals(&part.mesh).expect("second triangulation should work");

    assert_eq!(first, second);
    assert_no_degenerate_triangles(&first);
}

fn generate(source: GeometrySource, operations: Vec<ModelingOperationSpec>) -> GeneratedPart {
    let definition = definition(source, operations);
    generate_geometry(&definition, &mut context()).expect("geometry should generate")
}

fn definition(source: GeometrySource, operations: Vec<ModelingOperationSpec>) -> PartDefinition {
    PartDefinition {
        id: PartDefinitionId(1),
        name: "Part".to_owned(),
        tags: BTreeSet::new(),
        geometry: GeometryRecipe { source, operations },
        regions: BTreeMap::new(),
        sockets: BTreeMap::new(),
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
    }
}

fn set_bevel(operation: u64, radius: f32, segments: u32) -> ModelingOperationSpec {
    ModelingOperationSpec::SetBevelProfile {
        operation: OperationId(operation),
        radius,
        segments,
    }
}

fn context() -> GeneratorContext {
    GeneratorContext::new(PartDefinitionId(1), PartInstanceId(1), 100, 0)
}

fn square_profile() -> Vec<[f32; 2]> {
    vec![[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]]
}

fn sweep_path() -> Vec<Frame3> {
    vec![
        Frame3 {
            origin: [0.0, 0.0, 0.0],
            ..Frame3::default()
        },
        Frame3 {
            origin: [0.0, 2.0, 0.0],
            ..Frame3::default()
        },
    ]
}

fn assert_valid_quality_mesh(mesh: &PolygonMesh) {
    assert!(
        validate_polygon_mesh(mesh).is_valid(),
        "mesh should satisfy polygon validation"
    );
    let triangulated = triangulate_polygon_mesh(mesh).expect("mesh should triangulate");
    assert_no_degenerate_triangles(&triangulated);
    assert!(
        compute_face_normals(mesh)
            .expect("face normals should compute")
            .iter()
            .copied()
            .all(finite3)
    );
}

fn assert_region_role_faces(part: &GeneratedPart, role: SurfaceRole) {
    let regions = part
        .regions
        .iter()
        .filter_map(|(id, region)| (region.role == role).then_some(*id))
        .collect::<Vec<_>>();
    assert!(!regions.is_empty(), "expected region role {role:?}");
    assert!(
        regions
            .iter()
            .any(|region| region_face_count(&part.mesh, *region) > 0),
        "expected faces in region role {role:?}"
    );
}

fn assert_region_faces(part: &GeneratedPart, region: RegionId) {
    assert!(
        region_face_count(&part.mesh, region) > 0,
        "expected faces in region {region:?}"
    );
}

fn assert_region_name(part: &GeneratedPart, name: &str) {
    assert!(
        part.regions.values().any(|region| region.name == name),
        "missing region {name}"
    );
}

fn assert_same_region_ids(first: &GeneratedPart, second: &GeneratedPart) {
    assert_eq!(
        first.regions.keys().copied().collect::<Vec<_>>(),
        second.regions.keys().copied().collect::<Vec<_>>()
    );
}

fn region_face_count(mesh: &PolygonMesh, region: RegionId) -> usize {
    mesh.face_metadata
        .iter()
        .filter(|metadata| metadata.region == Some(region))
        .count()
}

fn assert_no_degenerate_triangles(triangulated: &TriangulatedPolygonMesh) {
    for triangle in triangulated.mesh.indices.chunks_exact(3) {
        let a = triangulated.mesh.positions[triangle[0] as usize];
        let b = triangulated.mesh.positions[triangle[1] as usize];
        let c = triangulated.mesh.positions[triangle[2] as usize];
        assert!(length(cross(sub(b, a), sub(c, a))) > EPSILON);
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "expected {expected}, got {actual}"
    );
}

fn finite3(vector: [f32; 3]) -> bool {
    vector.iter().copied().all(f32::is_finite)
}

fn unit(vector: [f32; 3]) -> bool {
    (length(vector) - 1.0).abs() < 1.0e-4
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(vector: [f32; 3]) -> f32 {
    dot(vector, vector).sqrt()
}
