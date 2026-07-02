use std::collections::{BTreeMap, BTreeSet};
use std::panic;

use glam::Vec3;
use orchard_core_legacy::{
    Aabb, NodeId, NodeKind, PrimitiveKind, ShapeDocument, ShapeNode, Transform3,
};
use orchard_field::{FieldCompileError, GridSpec, ScalarField, compile_document, sample_grid};

const BOUNDS_EPSILON: f32 = 1.0e-3;

#[test]
fn random_valid_documents_compile_and_sample_without_nan() {
    for seed in 100..350 {
        let document = random_document(seed, 14);
        let field = compile_document(&document).expect("valid document compiles");
        let scan_bounds = scan_bounds_for(field.bounds());
        let mut rng = TestRng::new(seed ^ 0x0a11_ce55);

        for _ in 0..32 {
            let point = random_point(&mut rng, scan_bounds);
            let value = field.sample(point);
            assert!(
                !value.is_nan(),
                "seed {seed} returned NaN at {point:?} for bounds {:?}",
                field.bounds()
            );
        }
    }
}

#[test]
fn computed_bounds_contain_sampled_negative_regions() {
    for seed in 900..1_020 {
        let document = random_document(seed, 12);
        let field = compile_document(&document).expect("valid document compiles");
        let bounds = field.bounds();
        let scan_bounds = scan_bounds_for(bounds);

        for z in 0..5 {
            for y in 0..5 {
                for x in 0..5 {
                    let point = grid_point(scan_bounds, x, y, z, 5);
                    let value = field.sample(point);
                    assert!(!value.is_nan(), "seed {seed} returned NaN at {point:?}");
                    if value < -BOUNDS_EPSILON {
                        assert!(
                            contains_point(bounds, point),
                            "seed {seed} sampled negative value {value} outside bounds {bounds:?} at {point:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn unsafe_grid_specs_and_malformed_documents_fail_without_panicking() {
    let field = compile_document(&sphere_document()).expect("sphere compiles");

    let zero_resolution = sample_grid(
        &field,
        GridSpec {
            bounds: field.bounds(),
            resolution_x: 0,
            resolution_y: 4,
            resolution_z: 4,
        },
    );
    assert!(matches!(
        zero_resolution,
        Err(FieldCompileError::InvalidGrid(_))
    ));

    let too_large = sample_grid(
        &field,
        GridSpec {
            bounds: field.bounds(),
            resolution_x: 257,
            resolution_y: 257,
            resolution_z: 257,
        },
    );
    assert!(matches!(too_large, Err(FieldCompileError::InvalidGrid(_))));

    let empty_bounds = sample_grid(
        &field,
        GridSpec {
            bounds: Aabb::empty(),
            resolution_x: 4,
            resolution_y: 4,
            resolution_z: 4,
        },
    );
    assert!(matches!(
        empty_bounds,
        Err(FieldCompileError::InvalidGrid(_))
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
    for seed in 5_000..5_400 {
        let result = panic::catch_unwind(|| {
            let document = random_document(seed, 16);
            let field = compile_document(&document).expect("valid document compiles");
            let bounds = field.bounds();
            let scan_bounds = scan_bounds_for(bounds);

            for index in 0..8 {
                let point = grid_point(scan_bounds, index % 2, (index / 2) % 2, index / 4, 2);
                let value = field.sample(point);
                assert!(!value.is_nan(), "seed {seed} returned NaN at {point:?}");
            }

            if !bounds.is_empty() {
                let samples = sample_grid(
                    &field,
                    GridSpec {
                        bounds,
                        resolution_x: 4,
                        resolution_y: 4,
                        resolution_z: 4,
                    },
                )
                .expect("small grid samples");
                assert!(samples.values.iter().all(|value| !value.is_nan()));
            }
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
        let kind = if number == 1 || rng.chance(55) {
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
        title: format!("random field {seed}"),
        root: root_id,
        next_node_id: root_id.0 + 1,
        nodes,
        locks: BTreeSet::new(),
    }
}

fn sphere_document() -> ShapeDocument {
    ShapeDocument::new(
        "sphere",
        node(
            NodeId(1),
            NodeKind::Primitive(PrimitiveKind::Sphere { radius: 1.0 }),
            Transform3::default(),
        ),
    )
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

    let mut cycle_nodes = BTreeMap::new();
    cycle_nodes.insert(
        NodeId(1),
        node(
            NodeId(1),
            NodeKind::Union {
                children: vec![NodeId(2)],
            },
            Transform3::default(),
        ),
    );
    cycle_nodes.insert(
        NodeId(2),
        node(
            NodeId(2),
            NodeKind::Union {
                children: vec![NodeId(1)],
            },
            Transform3::default(),
        ),
    );
    let cycle = ShapeDocument {
        schema_version: 1,
        title: "cycle".to_owned(),
        root: NodeId(1),
        nodes: cycle_nodes,
        next_node_id: 3,
        locks: BTreeSet::new(),
    };

    let invalid_primitive = ShapeDocument::new(
        "invalid primitive",
        node(
            NodeId(1),
            NodeKind::Primitive(PrimitiveKind::Torus {
                major_radius: 0.25,
                minor_radius: 0.25,
            }),
            Transform3::default(),
        ),
    );

    vec![missing_root, dangling, cycle, invalid_primitive]
}

fn random_primitive(rng: &mut TestRng) -> PrimitiveKind {
    match rng.usize(5) {
        0 => PrimitiveKind::Sphere {
            radius: rng.f32(0.1, 1.8),
        },
        1 => {
            let half_extents = Vec3::new(rng.f32(0.2, 1.6), rng.f32(0.2, 1.6), rng.f32(0.2, 1.6));
            PrimitiveKind::RoundedBox {
                half_extents,
                roundness: rng.f32(0.0, half_extents.min_element() * 0.5),
            }
        }
        2 => PrimitiveKind::Capsule {
            half_length: rng.f32(0.1, 1.5),
            radius: rng.f32(0.08, 0.8),
        },
        3 => {
            let half_height = rng.f32(0.15, 1.6);
            let radius = rng.f32(0.1, 0.9);
            PrimitiveKind::Cylinder {
                half_height,
                radius,
                roundness: rng.f32(0.0, half_height.min(radius) * 0.5),
            }
        }
        _ => {
            let minor_radius = rng.f32(0.05, 0.45);
            PrimitiveKind::Torus {
                major_radius: rng.f32(minor_radius + 0.05, 1.5),
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
            smoothness: rng.f32(0.0, 0.55),
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
        translation: Vec3::new(
            rng.f32(-1.25, 1.25),
            rng.f32(-1.25, 1.25),
            rng.f32(-1.25, 1.25),
        ),
        rotation_degrees: Vec3::new(
            rng.f32(-180.0, 180.0),
            rng.f32(-180.0, 180.0),
            rng.f32(-180.0, 180.0),
        ),
        scale: Vec3::new(rng.f32(0.35, 1.8), rng.f32(0.35, 1.8), rng.f32(0.35, 1.8)),
    }
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

fn scan_bounds_for(bounds: Aabb) -> Aabb {
    if bounds.is_empty() {
        Aabb {
            min: Vec3::splat(-2.0),
            max: Vec3::splat(2.0),
        }
    } else {
        bounds.expanded(0.75)
    }
}

fn grid_point(bounds: Aabb, x: usize, y: usize, z: usize, resolution: usize) -> Vec3 {
    let denom = resolution.saturating_sub(1).max(1) as f32;
    let t = Vec3::new(x as f32, y as f32, z as f32) / denom;
    bounds.min + bounds.extent() * t
}

fn random_point(rng: &mut TestRng, bounds: Aabb) -> Vec3 {
    Vec3::new(
        rng.f32(bounds.min.x, bounds.max.x),
        rng.f32(bounds.min.y, bounds.max.y),
        rng.f32(bounds.min.z, bounds.max.z),
    )
}

fn contains_point(bounds: Aabb, point: Vec3) -> bool {
    !bounds.is_empty()
        && point.cmpge(bounds.min - Vec3::splat(BOUNDS_EPSILON)).all()
        && point.cmple(bounds.max + Vec3::splat(BOUNDS_EPSILON)).all()
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
