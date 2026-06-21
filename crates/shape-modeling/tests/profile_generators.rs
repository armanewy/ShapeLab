use shape_asset::{PartDefinitionId, PartInstanceId, SurfaceRole};
use shape_modeling::{
    GeneratorContext, ModelingError,
    generators::profile::{
        CapMode, LATHE_BOTTOM_SOCKET, LATHE_END_CAP_REGION, LATHE_END_SEAM_REGION,
        LATHE_SIDE_REGION, LATHE_START_CAP_REGION, LATHE_START_SEAM_REGION, LATHE_TOP_SOCKET,
        LatheSpec, SWEEP_END_CAP_REGION, SWEEP_END_SOCKET, SWEEP_SIDE_REGION,
        SWEEP_START_CAP_REGION, SWEEP_START_SOCKET, generate_lathe, generate_sweep,
    },
};
use shape_poly::{
    BoundaryRole, EdgeClassification, compute_face_normals, triangulate_polygon_mesh,
};

fn context() -> GeneratorContext {
    GeneratorContext::new(PartDefinitionId(7), PartInstanceId(11), 100, 1)
}

fn square_profile() -> Vec<[f32; 2]> {
    vec![[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]]
}

fn straight_sweep_spec() -> shape_modeling::generators::profile::SweepSpec {
    shape_modeling::generators::profile::SweepSpec::new(
        square_profile(),
        vec![[0.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
        [0.0, 0.0, 1.0],
    )
}

#[test]
fn straight_sweep_builds_ring_quads_caps_and_sockets() {
    let part = generate_sweep(&straight_sweep_spec(), &context()).expect("sweep should generate");

    assert_eq!(part.mesh.positions.len(), 8);
    assert_eq!(part.mesh.faces.len(), 6);
    assert_eq!(
        part.mesh
            .faces
            .iter()
            .filter(|face| face.vertices.len() == 4)
            .count(),
        6
    );
    assert!(part.regions.contains_key(&SWEEP_SIDE_REGION));
    assert!(part.regions.contains_key(&SWEEP_START_CAP_REGION));
    assert!(part.regions.contains_key(&SWEEP_END_CAP_REGION));
    assert!(part.sockets.contains_key(&SWEEP_START_SOCKET));
    assert!(part.sockets.contains_key(&SWEEP_END_SOCKET));
    assert!(
        part.mesh
            .face_metadata
            .iter()
            .all(
                |metadata| metadata.part_definition == Some(PartDefinitionId(7))
                    && metadata.part_instance == Some(PartInstanceId(11))
            )
    );
}

#[test]
fn bent_sweep_marks_corner_regions_and_hard_transitions() {
    let spec = shape_modeling::generators::profile::SweepSpec::new(
        square_profile(),
        vec![[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]],
        [0.0, 0.0, 1.0],
    );

    let part = generate_sweep(&spec, &context()).expect("bent sweep should generate");

    assert!(
        part.regions
            .values()
            .any(|region| region.role == SurfaceRole::Custom("corner".to_owned()))
    );
    assert!(part.mesh.face_metadata.iter().any(|metadata| {
        metadata
            .region
            .map(|region| region.0 >= 100)
            .unwrap_or(false)
    }));
    assert!(part.mesh.edge_metadata.values().any(|metadata| {
        metadata.region_transition.is_some() && metadata.classification == EdgeClassification::Hard
    }));
}

#[test]
fn nonplanar_sweep_uses_stable_transport_without_frame_flips() {
    let spec = shape_modeling::generators::profile::SweepSpec::new(
        square_profile(),
        vec![
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.75, 1.5, 0.75],
            [1.25, 2.25, 1.5],
        ],
        [0.0, 0.0, 1.0],
    );

    let part = generate_sweep(&spec, &context()).expect("nonplanar sweep should generate");

    let profile_count = square_profile().len();
    let ring_offsets = (0..spec.path.len())
        .map(|ring| {
            let center = ring_center(&part.mesh.positions, ring, profile_count);
            subtract(part.mesh.positions[ring * profile_count], center)
        })
        .collect::<Vec<_>>();
    for offsets in ring_offsets.windows(2) {
        assert!(
            dot(offsets[0], offsets[1]) > -0.15,
            "transported frame flipped between adjacent rings"
        );
    }
}

#[test]
fn sweep_scale_changes_geometry_without_changing_topology() {
    let mut scaled = shape_modeling::generators::profile::SweepSpec::new(
        square_profile(),
        vec![[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 2.0, 0.0]],
        [0.0, 0.0, 1.0],
    );
    scaled.scales = vec![1.0, 2.0, 0.5];
    let unit = shape_modeling::generators::profile::SweepSpec::new(
        square_profile(),
        scaled.path.clone(),
        [0.0, 0.0, 1.0],
    );

    let scaled_part = generate_sweep(&scaled, &context()).expect("scaled sweep should generate");
    let unit_part = generate_sweep(&unit, &context()).expect("unit sweep should generate");

    assert_eq!(
        scaled_part.mesh.topology_signature,
        unit_part.mesh.topology_signature
    );
    assert!(scaled_part.mesh.bounds.max[0] > unit_part.mesh.bounds.max[0]);
}

#[test]
fn sweep_canonicalizes_closed_profile_winding_for_outward_caps() {
    let mut spec = straight_sweep_spec();
    spec.profile = vec![[-0.5, -0.5], [-0.5, 0.5], [0.5, 0.5], [0.5, -0.5]];

    let part = generate_sweep(&spec, &context()).expect("rewound sweep should generate");
    let normals = compute_face_normals(&part.mesh).expect("normals should compute");

    assert!(
        normals[4][1] < -0.9,
        "start cap should face backward along path"
    );
    assert!(
        normals[5][1] > 0.9,
        "end cap should face forward along path"
    );
}

#[test]
fn sweep_supports_capped_uncapped_and_closed_paths() {
    let capped = generate_sweep(&straight_sweep_spec(), &context()).expect("capped sweep");

    let mut uncapped_spec = straight_sweep_spec();
    uncapped_spec.cap_mode = CapMode::None;
    let uncapped = generate_sweep(&uncapped_spec, &context()).expect("uncapped sweep");

    assert_eq!(capped.mesh.faces.len(), uncapped.mesh.faces.len() + 2);
    assert!(!uncapped.regions.contains_key(&SWEEP_START_CAP_REGION));
    assert!(
        uncapped
            .mesh
            .edge_metadata
            .values()
            .any(|metadata| metadata.boundary_role == BoundaryRole::OpenBoundary)
    );

    let mut closed_spec = shape_modeling::generators::profile::SweepSpec::new(
        square_profile(),
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        [0.0, 0.0, 1.0],
    );
    closed_spec.path_closed = true;
    let closed = generate_sweep(&closed_spec, &context()).expect("closed sweep");

    assert_eq!(closed.mesh.faces.len(), 16);
    assert!(!closed.regions.contains_key(&SWEEP_START_CAP_REGION));
    assert!(!closed.regions.contains_key(&SWEEP_END_CAP_REGION));
}

#[test]
fn full_lathe_builds_quad_strips_caps_sockets_and_seam_metadata() {
    let spec = LatheSpec::new(vec![[1.0, -1.0], [1.0, 1.0]], 8);

    let part = generate_lathe(&spec, &context()).expect("lathe should generate");

    assert_eq!(part.mesh.positions.len(), 16);
    assert_eq!(part.mesh.faces.len(), 10);
    assert_eq!(
        part.mesh
            .faces
            .iter()
            .filter(|face| face.vertices.len() == 4)
            .count(),
        8
    );
    assert_eq!(
        part.mesh
            .faces
            .iter()
            .filter(|face| face.vertices.len() == 8)
            .count(),
        2
    );
    assert!(part.regions.contains_key(&LATHE_SIDE_REGION));
    assert!(part.regions.contains_key(&LATHE_START_CAP_REGION));
    assert!(part.regions.contains_key(&LATHE_END_CAP_REGION));
    assert!(part.sockets.contains_key(&LATHE_BOTTOM_SOCKET));
    assert!(part.sockets.contains_key(&LATHE_TOP_SOCKET));
    assert!(part.mesh.edge_metadata.values().any(|metadata| {
        metadata.seam_candidate && metadata.boundary_role == BoundaryRole::SeamCandidate
    }));
}

#[test]
fn partial_lathe_leaves_stable_open_seams() {
    let mut spec = LatheSpec::new(vec![[1.0, -1.0], [1.0, 1.0]], 4);
    spec.angular_span_degrees = 180.0;
    spec.cap_mode = CapMode::None;

    let part = generate_lathe(&spec, &context()).expect("partial lathe should generate");

    assert_eq!(part.mesh.positions.len(), 10);
    assert_eq!(part.mesh.faces.len(), 4);
    assert!(part.regions.contains_key(&LATHE_START_SEAM_REGION));
    assert!(part.regions.contains_key(&LATHE_END_SEAM_REGION));
    assert!(part.mesh.edge_metadata.values().any(|metadata| {
        metadata.seam_candidate && metadata.boundary_role == BoundaryRole::SeamCandidate
    }));
}

#[test]
fn axis_touching_lathe_profile_avoids_collapsed_rings() {
    let spec = LatheSpec::new(vec![[0.0, -1.0], [1.0, 0.0], [0.0, 1.0]], 8);

    let part = generate_lathe(&spec, &context()).expect("axis-touching lathe should generate");

    assert_eq!(part.mesh.positions.len(), 10);
    assert_eq!(part.mesh.faces.len(), 16);
    assert!(part.mesh.faces.iter().all(|face| face.vertices.len() == 3));
    for face in &part.mesh.faces {
        let mut vertices = face.vertices.clone();
        vertices.sort_unstable();
        vertices.dedup();
        assert_eq!(vertices.len(), face.vertices.len());
    }
}

#[test]
fn topology_and_element_ids_are_deterministic() {
    let first = generate_sweep(&straight_sweep_spec(), &context()).expect("first sweep");
    let second = generate_sweep(&straight_sweep_spec(), &context()).expect("second sweep");

    assert_eq!(
        first.mesh.topology_signature,
        second.mesh.topology_signature
    );
    assert_eq!(first.mesh.vertex_ids, second.mesh.vertex_ids);
    assert_eq!(first.mesh.faces, second.mesh.faces);
    assert_eq!(first.generator_signature, second.generator_signature);
}

#[test]
fn generated_faces_and_edges_carry_region_and_provenance_metadata() {
    let spec = shape_modeling::generators::profile::SweepSpec::new(
        square_profile(),
        vec![[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 2.0, 0.0]],
        [0.0, 0.0, 1.0],
    );
    let part = generate_sweep(&spec, &context()).expect("sweep should generate");

    assert!(part.mesh.face_metadata.iter().all(|metadata| {
        metadata.part_definition == Some(PartDefinitionId(7))
            && metadata.part_instance == Some(PartInstanceId(11))
            && metadata.region.is_some()
            && metadata.surface_role.is_some()
    }));
    assert!(
        part.mesh
            .edge_metadata
            .values()
            .any(|metadata| metadata.classification == EdgeClassification::Hard)
    );
    assert!(
        part.mesh
            .edge_metadata
            .values()
            .any(|metadata| metadata.classification == EdgeClassification::Smooth)
    );
}

#[test]
fn generated_meshes_triangulate_with_nonzero_normals_and_region_maps() {
    let part = generate_lathe(
        &LatheSpec::new(vec![[1.0, -1.0], [1.0, 1.0]], 12),
        &context(),
    )
    .expect("lathe should generate");

    let triangulated = triangulate_polygon_mesh(&part.mesh).expect("lathe should triangulate");

    assert_eq!(
        triangulated.triangle_to_region.len(),
        triangulated.mesh.indices.len() / 3
    );
    assert!(
        triangulated
            .mesh
            .normals
            .iter()
            .all(|normal| dot(*normal, *normal) > 0.5)
    );
}

#[test]
fn invalid_path_profile_and_axis_inputs_are_rejected() {
    let invalid_sweep = shape_modeling::generators::profile::SweepSpec::new(
        vec![[0.0, 0.0], [1.0, 0.0]],
        vec![[0.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        [0.0, 0.0, 1.0],
    );
    assert!(matches!(
        generate_sweep(&invalid_sweep, &context()),
        Err(ModelingError::InvalidInput(_))
    ));

    let parallel_up = shape_modeling::generators::profile::SweepSpec::new(
        square_profile(),
        vec![[0.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        [0.0, 2.0, 0.0],
    );
    assert!(matches!(
        generate_sweep(&parallel_up, &context()),
        Err(ModelingError::InvalidInput(_))
    ));

    let invalid_lathe = LatheSpec::new(vec![[-1.0, 0.0], [1.0, 1.0]], 8);
    assert!(matches!(
        generate_lathe(&invalid_lathe, &context()),
        Err(ModelingError::InvalidInput(_))
    ));

    let invalid_segments = LatheSpec::new(vec![[1.0, 0.0], [1.0, 1.0]], 2);
    assert!(matches!(
        generate_lathe(&invalid_segments, &context()),
        Err(ModelingError::InvalidInput(_))
    ));
}

fn ring_center(positions: &[[f32; 3]], ring: usize, profile_count: usize) -> [f32; 3] {
    let mut center = [0.0, 0.0, 0.0];
    for position in &positions[ring * profile_count..(ring + 1) * profile_count] {
        center[0] += position[0];
        center[1] += position[1];
        center[2] += position[2];
    }
    let divisor = profile_count as f32;
    [
        center[0] / divisor,
        center[1] / divisor,
        center[2] / divisor,
    ]
}

fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
