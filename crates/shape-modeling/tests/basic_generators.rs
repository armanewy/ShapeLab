use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{PartDefinitionId, PartInstanceId, RegionId, SocketId};
use shape_modeling::generators::basic::{
    CapMode, CylinderParams, FaceMask, FrustumParams, PlateParams, RoundedBoxParams,
    build_cylinder, build_frustum, build_plate, build_rounded_box,
};
use shape_modeling::{GeneratedPart, GeneratorContext};
use shape_poly::{
    BoundaryRole, PolygonMesh, build_adjacency, compute_face_normals, compute_split_vertex_normals,
    triangulate_polygon_mesh, validate_polygon_mesh,
};

const EPSILON: f32 = 1.0e-5;

#[test]
fn rounded_box_closed_topology_is_stable_and_semantic() {
    let params = RoundedBoxParams {
        half_extents: [1.0, 0.75, 0.5],
        bevel_radius: 0.2,
        bevel_segments: 2,
        face_subdivisions: 2,
        face_mask: FaceMask::all(),
    };
    let part = build_rounded_box(&params, &context()).expect("rounded box should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_region_names(&part, &["primary_faces", "bevel_bands", "corners"]);
    assert_faces_use_regions(&part, &[RegionId(1), RegionId(2), RegionId(3)]);
    assert_eq!(part.sockets.len(), 6);
    assert_socket_origin(&part, SocketId(1), [1.0, 0.0, 0.0]);
    assert_socket_origin(&part, SocketId(3), [0.0, 0.75, 0.0]);
    assert_bounds(&part.mesh, [-1.0, -0.75, -0.5], [1.0, 0.75, 0.5]);

    let repeated = build_rounded_box(&params, &context()).expect("repeat should generate");
    assert_deterministic_ids(&part.mesh, &repeated.mesh);

    let mut scalar_change = params.clone();
    scalar_change.bevel_radius = 0.15;
    let scalar_part =
        build_rounded_box(&scalar_change, &context()).expect("scalar change should generate");
    assert_same_region_ids(&part, &scalar_part);
    assert_eq!(
        part.mesh.topology_signature,
        scalar_part.mesh.topology_signature
    );

    let mut topology_change = params;
    topology_change.bevel_segments = 3;
    let topology_part =
        build_rounded_box(&topology_change, &context()).expect("topology change should generate");
    assert_ne!(
        part.mesh.topology_signature,
        topology_part.mesh.topology_signature
    );
}

#[test]
fn rounded_box_open_mask_reports_open_boundaries() {
    let params = RoundedBoxParams {
        half_extents: [1.0, 1.0, 1.0],
        bevel_radius: 0.15,
        bevel_segments: 2,
        face_subdivisions: 1,
        face_mask: FaceMask {
            positive_y: false,
            ..FaceMask::all()
        },
    };
    let part = build_rounded_box(&params, &context()).expect("open rounded box should generate");

    assert_valid_with_open_boundaries(&part.mesh, 20);
    assert_common_mesh_quality(&part.mesh);
}

#[test]
fn cylinder_closed_and_open_modes_are_indexed_and_semantic() {
    let params = CylinderParams {
        radius: 1.0,
        half_height: 1.25,
        radial_segments: 12,
        height_segments: 2,
        cap_mode: CapMode::Both,
        top_bevel_radius: 0.12,
        bottom_bevel_radius: 0.12,
        bevel_segments: 2,
    };
    let part = build_cylinder(&params, &context()).expect("cylinder should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_region_names(
        &part,
        &["side", "top_cap", "bottom_cap", "top_bevel", "bottom_bevel"],
    );
    assert_faces_use_regions(
        &part,
        &[
            RegionId(1),
            RegionId(2),
            RegionId(3),
            RegionId(4),
            RegionId(5),
        ],
    );
    assert_socket_origin(&part, SocketId(1), [0.0, 1.25, 0.0]);
    assert_socket_origin(&part, SocketId(2), [0.0, -1.25, 0.0]);
    assert_socket_origin(&part, SocketId(3), [0.0, 0.0, 0.0]);
    assert_bounds(&part.mesh, [-1.0, -1.25, -1.0], [1.0, 1.25, 1.0]);

    let repeated = build_cylinder(&params, &context()).expect("repeat should generate");
    assert_deterministic_ids(&part.mesh, &repeated.mesh);

    let mut scalar_change = params.clone();
    scalar_change.radius = 1.1;
    let scalar_part =
        build_cylinder(&scalar_change, &context()).expect("scalar change should generate");
    assert_same_region_ids(&part, &scalar_part);
    assert_eq!(
        part.mesh.topology_signature,
        scalar_part.mesh.topology_signature
    );

    let mut topology_change = params;
    topology_change.radial_segments = 16;
    let topology_part =
        build_cylinder(&topology_change, &context()).expect("topology change should generate");
    assert_ne!(
        part.mesh.topology_signature,
        topology_part.mesh.topology_signature
    );

    let open = CylinderParams {
        cap_mode: CapMode::None,
        top_bevel_radius: 0.0,
        bottom_bevel_radius: 0.0,
        bevel_segments: 0,
        radial_segments: 8,
        height_segments: 1,
        ..scalar_change
    };
    let open_part = build_cylinder(&open, &context()).expect("open cylinder should generate");
    assert_valid_with_open_boundaries(&open_part.mesh, 16);
}

#[test]
fn frustum_closed_and_open_modes_preserve_regions() {
    let params = FrustumParams {
        bottom_radius: 1.0,
        top_radius: 0.45,
        half_height: 1.0,
        radial_segments: 12,
        height_segments: 3,
        cap_mode: CapMode::Both,
        top_bevel_radius: 0.08,
        bottom_bevel_radius: 0.1,
        bevel_segments: 2,
    };
    let part = build_frustum(&params, &context()).expect("frustum should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_faces_use_regions(
        &part,
        &[
            RegionId(1),
            RegionId(2),
            RegionId(3),
            RegionId(4),
            RegionId(5),
        ],
    );
    assert_socket_origin(&part, SocketId(1), [0.0, 1.0, 0.0]);
    assert_socket_origin(&part, SocketId(2), [0.0, -1.0, 0.0]);
    assert_bounds(&part.mesh, [-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]);

    let repeated = build_frustum(&params, &context()).expect("repeat should generate");
    assert_deterministic_ids(&part.mesh, &repeated.mesh);

    let mut scalar_change = params.clone();
    scalar_change.top_radius = 0.6;
    let scalar_part =
        build_frustum(&scalar_change, &context()).expect("scalar change should generate");
    assert_same_region_ids(&part, &scalar_part);
    assert_eq!(
        part.mesh.topology_signature,
        scalar_part.mesh.topology_signature
    );

    let mut topology_change = params;
    topology_change.height_segments = 4;
    let topology_part =
        build_frustum(&topology_change, &context()).expect("topology change should generate");
    assert_ne!(
        part.mesh.topology_signature,
        topology_part.mesh.topology_signature
    );

    let open = FrustumParams {
        cap_mode: CapMode::Bottom,
        top_bevel_radius: 0.0,
        bottom_bevel_radius: 0.0,
        bevel_segments: 0,
        radial_segments: 12,
        height_segments: 1,
        ..scalar_change
    };
    let open_part = build_frustum(&open, &context()).expect("open frustum should generate");
    assert_valid_with_open_boundaries(&open_part.mesh, 12);
}

#[test]
fn plate_is_closed_rounded_and_semantic() {
    let params = PlateParams {
        width: 3.0,
        height: 2.0,
        thickness: 0.25,
        corner_radius: 0.25,
        corner_segments: 3,
        front_back_bevel: 0.05,
    };
    let part = build_plate(&params, &context()).expect("plate should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_region_names(&part, &["front", "back", "side", "bevel"]);
    assert_faces_use_regions(&part, &[RegionId(1), RegionId(2), RegionId(3), RegionId(4)]);
    assert_socket_origin(&part, SocketId(1), [0.0, 0.125, 0.0]);
    assert_socket_origin(&part, SocketId(2), [0.0, -0.125, 0.0]);
    assert_bounds(&part.mesh, [-1.5, -0.125, -1.0], [1.5, 0.125, 1.0]);

    let repeated = build_plate(&params, &context()).expect("repeat should generate");
    assert_deterministic_ids(&part.mesh, &repeated.mesh);

    let mut scalar_change = params.clone();
    scalar_change.thickness = 0.35;
    let scalar_part =
        build_plate(&scalar_change, &context()).expect("scalar change should generate");
    assert_same_region_ids(&part, &scalar_part);
    assert_eq!(
        part.mesh.topology_signature,
        scalar_part.mesh.topology_signature
    );

    let mut topology_change = params;
    topology_change.corner_segments = 4;
    let topology_part =
        build_plate(&topology_change, &context()).expect("topology change should generate");
    assert_ne!(
        part.mesh.topology_signature,
        topology_part.mesh.topology_signature
    );
}

fn context() -> GeneratorContext {
    GeneratorContext::new(PartDefinitionId(7), PartInstanceId(11), 100, 0)
}

fn assert_closed_mesh(mesh: &PolygonMesh) {
    let adjacency = build_adjacency(mesh).expect("adjacency should build");
    assert!(
        adjacency.edge_faces.values().all(|faces| faces.len() == 2),
        "closed mesh should have exactly two incident faces per edge"
    );
    assert_eq!(open_boundary_count(mesh), 0);
    assert!(
        signed_volume(mesh) > EPSILON,
        "closed mesh should have consistent outward winding"
    );
}

fn assert_valid_with_open_boundaries(mesh: &PolygonMesh, expected_open_edges: usize) {
    let adjacency = build_adjacency(mesh).expect("adjacency should build");
    assert!(
        adjacency
            .edge_faces
            .values()
            .all(|faces| (1..=2).contains(&faces.len())),
        "open mesh should remain manifold"
    );
    assert_eq!(open_boundary_count(mesh), expected_open_edges);
}

fn assert_common_mesh_quality(mesh: &PolygonMesh) {
    assert!(
        validate_polygon_mesh(mesh).is_valid(),
        "mesh contract validation should pass"
    );
    assert_no_duplicate_positions(mesh);
    assert_no_degenerate_faces(mesh);
    let face_normals = compute_face_normals(mesh).expect("face normals should compute");
    assert!(
        face_normals.iter().copied().all(finite_vector),
        "face normals should be finite"
    );
    let split_normals = compute_split_vertex_normals(mesh).expect("split normals should compute");
    assert!(
        split_normals.iter().copied().all(finite_vector),
        "split vertex normals should be finite"
    );
    let triangulated = triangulate_polygon_mesh(mesh).expect("triangulation should succeed");
    assert_eq!(
        triangulated.mesh.indices.len() % 3,
        0,
        "triangulation should produce whole triangles"
    );
}

fn assert_faces_use_regions(part: &GeneratedPart, expected_regions: &[RegionId]) {
    let counts = region_face_counts(&part.mesh);
    for region in expected_regions {
        assert!(
            counts.get(region).copied().unwrap_or_default() > 0,
            "expected region {region:?} to have faces"
        );
    }
}

fn assert_region_names(part: &GeneratedPart, expected_names: &[&str]) {
    let names = part
        .regions
        .values()
        .map(|region| region.name.as_str())
        .collect::<BTreeSet<_>>();
    for expected in expected_names {
        assert!(names.contains(expected), "missing region name {expected}");
    }
}

fn assert_socket_origin(part: &GeneratedPart, socket: SocketId, expected: [f32; 3]) {
    let actual = part
        .sockets
        .get(&socket)
        .unwrap_or_else(|| panic!("missing socket {socket:?}"))
        .local_frame
        .origin;
    assert_vec3_close(actual, expected);
}

fn assert_bounds(mesh: &PolygonMesh, expected_min: [f32; 3], expected_max: [f32; 3]) {
    assert_vec3_close(mesh.bounds.min, expected_min);
    assert_vec3_close(mesh.bounds.max, expected_max);
}

fn assert_deterministic_ids(first: &PolygonMesh, second: &PolygonMesh) {
    assert_eq!(first.topology_signature, second.topology_signature);
    assert_eq!(first.vertex_ids, second.vertex_ids);
    let first_face_ids = first.faces.iter().map(|face| face.id).collect::<Vec<_>>();
    let second_face_ids = second.faces.iter().map(|face| face.id).collect::<Vec<_>>();
    assert_eq!(first_face_ids, second_face_ids);
}

fn assert_same_region_ids(first: &GeneratedPart, second: &GeneratedPart) {
    assert_eq!(
        first.regions.keys().copied().collect::<Vec<_>>(),
        second.regions.keys().copied().collect::<Vec<_>>()
    );
}

fn assert_no_duplicate_positions(mesh: &PolygonMesh) {
    let mut seen = BTreeSet::new();
    for position in &mesh.positions {
        assert!(
            seen.insert(VertexKey::from_position(*position)),
            "duplicate vertex position {position:?}"
        );
    }
}

fn assert_no_degenerate_faces(mesh: &PolygonMesh) {
    for face in &mesh.faces {
        let area = polygon_area(mesh, &face.vertices);
        assert!(area > EPSILON, "degenerate face {:?}", face.id);
    }
}

fn open_boundary_count(mesh: &PolygonMesh) -> usize {
    mesh.edge_metadata
        .values()
        .filter(|metadata| metadata.boundary_role == BoundaryRole::OpenBoundary)
        .count()
}

fn region_face_counts(mesh: &PolygonMesh) -> BTreeMap<RegionId, usize> {
    let mut counts = BTreeMap::new();
    for metadata in &mesh.face_metadata {
        if let Some(region) = metadata.region {
            *counts.entry(region).or_insert(0) += 1;
        }
    }
    counts
}

fn signed_volume(mesh: &PolygonMesh) -> f32 {
    let triangles = triangulate_polygon_mesh(mesh).expect("closed mesh should triangulate");
    let mut volume = 0.0;
    for triangle in triangles.mesh.indices.chunks_exact(3) {
        let a = triangles.mesh.positions[triangle[0] as usize];
        let b = triangles.mesh.positions[triangle[1] as usize];
        let c = triangles.mesh.positions[triangle[2] as usize];
        volume += dot(a, cross(b, c)) / 6.0;
    }
    volume
}

fn polygon_area(mesh: &PolygonMesh, vertices: &[u32]) -> f32 {
    let origin = mesh.positions[vertices[0] as usize];
    let mut area = 0.0;
    for index in 1..vertices.len() - 1 {
        let a = mesh.positions[vertices[index] as usize];
        let b = mesh.positions[vertices[index + 1] as usize];
        area += length(cross(sub(a, origin), sub(b, origin))) * 0.5;
    }
    area
}

fn finite_vector(vector: [f32; 3]) -> bool {
    vector.iter().copied().all(f32::is_finite)
}

fn assert_vec3_close(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, got {actual}"
        );
    }
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct VertexKey(i64, i64, i64);

impl VertexKey {
    fn from_position(position: [f32; 3]) -> Self {
        Self(
            quantize(position[0]),
            quantize(position[1]),
            quantize(position[2]),
        )
    }
}

fn quantize(value: f32) -> i64 {
    (value * 1_000_000.0).round() as i64
}
