use std::collections::{BTreeMap, BTreeSet};
use std::panic;

use glam::Vec3;
use shape_core::{Aabb, NodeId, NodeKind, PrimitiveKind, ShapeDocument, ShapeNode, Transform3};
use shape_field::{ScalarField, compile_document};
use shape_mesh::{MeshError, MeshSettings, TriangleMesh, mesh_field, write_obj_to_string};

#[test]
fn random_valid_document_meshes_have_valid_topology() {
    for seed in 700..820 {
        let document = random_document(seed, 12);
        let field = compile_document(&document).expect("valid document compiles");
        let mesh = mesh_field(
            &field,
            MeshSettings {
                resolution: 8,
                padding_fraction: 0.12,
                iso_value: 0.0,
            },
        )
        .expect("valid field meshes");

        assert_mesh_valid(&mesh);
    }
}

#[test]
fn representative_fields_have_outward_winding() {
    for document in representative_documents() {
        let field = compile_document(&document).expect("representative document compiles");
        let mesh = mesh_field(
            &field,
            MeshSettings {
                resolution: 18,
                padding_fraction: 0.16,
                iso_value: 0.0,
            },
        )
        .expect("representative field meshes");

        assert_outward_winding(&field, &mesh);
    }
}

#[test]
fn deterministic_obj_output_for_seeded_random_documents() {
    for seed in 1_400..1_425 {
        let document = random_document(seed, 10);
        let first = obj_for_document(&document, 9);
        let second = obj_for_document(&document, 9);

        assert_eq!(first, second, "OBJ output changed for seed {seed}");
    }
}

#[test]
fn unsafe_resolution_and_malformed_documents_fail_without_panicking() {
    let field = compile_document(&representative_documents().remove(0)).expect("sphere compiles");
    let too_large = mesh_field(
        &field,
        MeshSettings {
            resolution: 256,
            ..MeshSettings::default()
        },
    );
    assert!(matches!(too_large, Err(MeshError::TooLarge(_))));

    assert!(matches!(
        mesh_field(
            &NonFiniteField,
            MeshSettings {
                resolution: 4,
                ..MeshSettings::default()
            },
        ),
        Err(MeshError::NonFiniteSample { .. })
    ));

    assert!(matches!(
        mesh_field(
            &EmptyBoundsField,
            MeshSettings {
                resolution: 4,
                ..MeshSettings::default()
            },
        ),
        Err(MeshError::InvalidBounds(_))
    ));

    for malformed in malformed_documents() {
        assert!(
            compile_document(&malformed).is_err(),
            "malformed document unexpectedly compiled: {malformed:?}"
        );
    }
}

#[test]
fn several_hundred_seeded_documents_do_not_panic() {
    for seed in 9_000..9_300 {
        let result = panic::catch_unwind(|| {
            let document = random_document(seed, 14);
            let field = compile_document(&document).expect("valid document compiles");
            let mesh = mesh_field(
                &field,
                MeshSettings {
                    resolution: 5,
                    padding_fraction: 0.12,
                    iso_value: 0.0,
                },
            )
            .expect("low-resolution mesh succeeds");
            assert_mesh_valid(&mesh);
        });

        assert!(result.is_ok(), "seed {seed} panicked");
    }
}

fn random_document(seed: u64, max_nodes: usize) -> ShapeDocument {
    let mut rng = TestRng::new(seed);
    let node_count = 2 + rng.usize(max_nodes.saturating_sub(1).max(1));
    let mut nodes = BTreeMap::new();
    let mut primitive_ids = Vec::new();

    for number in 1..node_count {
        let id = NodeId(number as u64);
        let kind = if number == 1 || rng.chance(60) {
            primitive_ids.push(id);
            NodeKind::Primitive(random_primitive(&mut rng))
        } else {
            random_csg_kind(&mut rng, number)
        };
        let transform = if matches!(kind, NodeKind::Primitive(_)) {
            random_transform(&mut rng)
        } else {
            Transform3::default()
        };
        nodes.insert(id, node(id, kind, transform));
    }

    let root_id = NodeId(node_count as u64);
    let mut root_children = random_children(&mut rng, node_count, 4);
    if let Some(primitive_id) = primitive_ids.get(rng.usize(primitive_ids.len())).copied() {
        root_children.push(primitive_id);
        root_children.sort();
        root_children.dedup();
    }
    nodes.insert(
        root_id,
        node(
            root_id,
            NodeKind::Union {
                children: root_children,
            },
            Transform3::default(),
        ),
    );

    ShapeDocument {
        schema_version: 1,
        title: format!("random mesh {seed}"),
        root: root_id,
        next_node_id: root_id.0 + 1,
        nodes,
        locks: BTreeSet::new(),
    }
}

fn representative_documents() -> Vec<ShapeDocument> {
    let sphere = ShapeDocument::new(
        "sphere",
        node(
            NodeId(1),
            NodeKind::Primitive(PrimitiveKind::Sphere { radius: 1.0 }),
            Transform3::default(),
        ),
    );

    let scaled_sphere = ShapeDocument::new(
        "scaled sphere",
        node(
            NodeId(1),
            NodeKind::Primitive(PrimitiveKind::Sphere { radius: 1.0 }),
            Transform3 {
                scale: Vec3::new(1.2, 0.8, 1.6),
                rotation_degrees: Vec3::new(0.0, 25.0, 0.0),
                ..Transform3::default()
            },
        ),
    );

    let mut union_nodes = BTreeMap::new();
    union_nodes.insert(
        NodeId(1),
        node(
            NodeId(1),
            NodeKind::Primitive(PrimitiveKind::Sphere { radius: 0.85 }),
            Transform3 {
                translation: Vec3::new(-0.45, 0.0, 0.0),
                ..Transform3::default()
            },
        ),
    );
    union_nodes.insert(
        NodeId(2),
        node(
            NodeId(2),
            NodeKind::Primitive(PrimitiveKind::Sphere { radius: 0.85 }),
            Transform3 {
                translation: Vec3::new(0.45, 0.0, 0.0),
                ..Transform3::default()
            },
        ),
    );
    union_nodes.insert(
        NodeId(3),
        node(
            NodeId(3),
            NodeKind::SmoothUnion {
                children: vec![NodeId(1), NodeId(2)],
                smoothness: 0.25,
            },
            Transform3::default(),
        ),
    );
    let smooth_union = ShapeDocument {
        schema_version: 1,
        title: "smooth union".to_owned(),
        root: NodeId(3),
        nodes: union_nodes,
        next_node_id: 4,
        locks: BTreeSet::new(),
    };

    vec![sphere, scaled_sphere, smooth_union]
}

fn malformed_documents() -> Vec<ShapeDocument> {
    let missing_root = ShapeDocument {
        schema_version: 1,
        title: "missing root".to_owned(),
        root: NodeId(100),
        nodes: BTreeMap::new(),
        next_node_id: 101,
        locks: BTreeSet::new(),
    };

    let dangling = ShapeDocument::new(
        "dangling",
        node(
            NodeId(1),
            NodeKind::Union {
                children: vec![NodeId(2)],
            },
            Transform3::default(),
        ),
    );

    let invalid_primitive = ShapeDocument::new(
        "invalid primitive",
        node(
            NodeId(1),
            NodeKind::Primitive(PrimitiveKind::RoundedBox {
                half_extents: Vec3::splat(0.25),
                roundness: 0.5,
            }),
            Transform3::default(),
        ),
    );

    vec![missing_root, dangling, invalid_primitive]
}

fn random_primitive(rng: &mut TestRng) -> PrimitiveKind {
    match rng.usize(5) {
        0 => PrimitiveKind::Sphere {
            radius: rng.f32(0.15, 1.35),
        },
        1 => {
            let half_extents = Vec3::new(
                rng.f32(0.25, 1.25),
                rng.f32(0.25, 1.25),
                rng.f32(0.25, 1.25),
            );
            PrimitiveKind::RoundedBox {
                half_extents,
                roundness: rng.f32(0.0, half_extents.min_element() * 0.45),
            }
        }
        2 => PrimitiveKind::Capsule {
            half_length: rng.f32(0.15, 1.25),
            radius: rng.f32(0.12, 0.65),
        },
        3 => {
            let half_height = rng.f32(0.2, 1.25);
            let radius = rng.f32(0.12, 0.75);
            PrimitiveKind::Cylinder {
                half_height,
                radius,
                roundness: rng.f32(0.0, half_height.min(radius) * 0.45),
            }
        }
        _ => {
            let minor_radius = rng.f32(0.06, 0.35);
            PrimitiveKind::Torus {
                major_radius: rng.f32(minor_radius + 0.08, 1.15),
                minor_radius,
            }
        }
    }
}

fn random_csg_kind(rng: &mut TestRng, current_number: usize) -> NodeKind {
    match rng.usize(4) {
        0 => NodeKind::Union {
            children: random_children(rng, current_number, 3),
        },
        1 => NodeKind::SmoothUnion {
            children: random_children(rng, current_number, 3),
            smoothness: rng.f32(0.0, 0.45),
        },
        2 => NodeKind::Difference {
            base: random_child(rng, current_number),
            subtractors: random_children(rng, current_number, 2),
        },
        _ => NodeKind::Intersection {
            children: random_children(rng, current_number, 3),
        },
    }
}

fn random_children(rng: &mut TestRng, current_number: usize, max_children: usize) -> Vec<NodeId> {
    let available = current_number - 1;
    let wanted = 1 + rng.usize(max_children.min(available));
    let mut children = BTreeSet::new();
    while children.len() < wanted {
        children.insert(random_child(rng, current_number));
    }
    children.into_iter().collect()
}

fn random_child(rng: &mut TestRng, current_number: usize) -> NodeId {
    NodeId(1 + rng.usize(current_number - 1) as u64)
}

fn random_transform(rng: &mut TestRng) -> Transform3 {
    Transform3 {
        translation: Vec3::new(rng.f32(-1.0, 1.0), rng.f32(-1.0, 1.0), rng.f32(-1.0, 1.0)),
        rotation_degrees: Vec3::new(
            rng.f32(-180.0, 180.0),
            rng.f32(-180.0, 180.0),
            rng.f32(-180.0, 180.0),
        ),
        scale: Vec3::new(rng.f32(0.45, 1.6), rng.f32(0.45, 1.6), rng.f32(0.45, 1.6)),
    }
}

fn obj_for_document(document: &ShapeDocument, resolution: usize) -> String {
    let field = compile_document(document).expect("document compiles");
    let mesh = mesh_field(
        &field,
        MeshSettings {
            resolution,
            padding_fraction: 0.12,
            iso_value: 0.0,
        },
    )
    .expect("mesh succeeds");
    write_obj_to_string(&mesh).expect("OBJ writes")
}

fn assert_mesh_valid(mesh: &TriangleMesh) {
    assert_eq!(mesh.positions.len(), mesh.normals.len());
    assert_eq!(mesh.indices.len() % 3, 0);

    for position in &mesh.positions {
        assert!(
            array_is_finite(*position),
            "non-finite position {position:?}"
        );
    }
    for normal in &mesh.normals {
        assert!(array_is_finite(*normal), "non-finite normal {normal:?}");
    }
    for index in &mesh.indices {
        assert!(
            (*index as usize) < mesh.positions.len(),
            "position index {index} out of range {}",
            mesh.positions.len()
        );
        assert!(
            (*index as usize) < mesh.normals.len(),
            "normal index {index} out of range {}",
            mesh.normals.len()
        );
    }

    for position in &mesh.positions {
        let point = Vec3::from_array(*position);
        assert!(
            mesh.bounds.is_empty() || contains_point(mesh.bounds, point),
            "mesh bounds {:?} do not contain {point:?}",
            mesh.bounds
        );
    }
}

fn assert_outward_winding(field: &impl ScalarField, mesh: &TriangleMesh) {
    assert_mesh_valid(mesh);

    let mut checked = 0_usize;
    let mut outward = 0_usize;
    for triangle in mesh.indices.chunks_exact(3) {
        let a = Vec3::from_array(mesh.positions[triangle[0] as usize]);
        let b = Vec3::from_array(mesh.positions[triangle[1] as usize]);
        let c = Vec3::from_array(mesh.positions[triangle[2] as usize]);
        let face = (b - a).cross(c - a);
        let length = face.length();
        if !length.is_finite() || length <= 1.0e-6 {
            continue;
        }
        let normal = face / length;
        let centroid = (a + b + c) / 3.0;
        let outside = field.sample(centroid + normal * 0.04);
        let inside = field.sample(centroid - normal * 0.04);
        if outside.is_finite() && inside.is_finite() {
            checked += 1;
            if outside > inside {
                outward += 1;
            }
        }
    }

    assert!(checked > 20, "not enough non-degenerate triangles to check");
    assert!(
        outward * 20 >= checked * 19,
        "only {outward}/{checked} triangles had outward winding"
    );
}

fn node(id: NodeId, kind: NodeKind, transform: Transform3) -> ShapeNode {
    ShapeNode {
        id,
        name: format!("node {}", id.0),
        tags: BTreeSet::new(),
        enabled: true,
        transform,
        kind,
    }
}

fn contains_point(bounds: Aabb, point: Vec3) -> bool {
    point.cmpge(bounds.min - Vec3::splat(1.0e-4)).all()
        && point.cmple(bounds.max + Vec3::splat(1.0e-4)).all()
}

fn array_is_finite(array: [f32; 3]) -> bool {
    array[0].is_finite() && array[1].is_finite() && array[2].is_finite()
}

#[derive(Debug, Copy, Clone)]
struct NonFiniteField;

impl ScalarField for NonFiniteField {
    fn sample(&self, _point: Vec3) -> f32 {
        f32::NAN
    }

    fn bounds(&self) -> Aabb {
        Aabb {
            min: Vec3::splat(-1.0),
            max: Vec3::splat(1.0),
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct EmptyBoundsField;

impl ScalarField for EmptyBoundsField {
    fn sample(&self, _point: Vec3) -> f32 {
        1.0
    }

    fn bounds(&self) -> Aabb {
        Aabb::empty()
    }
}

#[derive(Debug, Clone)]
struct TestRng {
    state: u64,
}

impl TestRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9e37_79b9_7f4a_7c15,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn usize(&mut self, upper_exclusive: usize) -> usize {
        assert!(upper_exclusive > 0);
        (self.next_u64() % upper_exclusive as u64) as usize
    }

    fn chance(&mut self, percent: u32) -> bool {
        self.next_u64() % 100 < u64::from(percent)
    }

    fn f32(&mut self, minimum: f32, maximum: f32) -> f32 {
        let unit = ((self.next_u64() >> 40) as f32) / ((1_u64 << 24) as f32);
        minimum + (maximum - minimum) * unit
    }
}
