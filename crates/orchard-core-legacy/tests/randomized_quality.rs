use std::collections::{BTreeMap, BTreeSet};
use std::panic;

use glam::Vec3;
use orchard_core_legacy::{
    EditProgram, NodeId, NodeKind, ParamPath, PrimitiveKind, SetScalarEdit, ShapeDocument,
    ShapeNode, Transform3, apply_edit, descendants_of, enumerate_parameters, get_scalar,
    set_scalar, validate_document,
};

const PARAM_EPSILON: f32 = 1.0e-5;

#[test]
fn random_valid_primitive_ranges_validate() {
    for seed in 0..500 {
        let mut rng = TestRng::new(seed);
        let document = document_with_root(node(
            NodeId(1),
            NodeKind::Primitive(random_primitive(&mut rng)),
            random_transform(&mut rng),
        ));

        assert_valid(&document);

        for descriptor in enumerate_parameters(&document) {
            let value = get_scalar(&document, &descriptor.path).expect("descriptor is readable");
            assert!(
                value.is_finite(),
                "non-finite value at {:?}",
                descriptor.path
            );
            assert!(
                descriptor.minimum.is_finite()
                    && descriptor.maximum.is_finite()
                    && descriptor.step.is_finite()
                    && descriptor.mutation_sigma.is_finite(),
                "non-finite descriptor for {:?}: {descriptor:?}",
                descriptor.path
            );
            assert!(
                descriptor.minimum <= descriptor.maximum,
                "invalid descriptor range for {:?}: {descriptor:?}",
                descriptor.path
            );
            assert!(
                value >= descriptor.minimum - PARAM_EPSILON
                    && value <= descriptor.maximum + PARAM_EPSILON,
                "value {value} is outside descriptor range for {:?}: {descriptor:?}",
                descriptor.path
            );
        }
    }
}

#[test]
fn bounded_random_acyclic_graphs_serde_round_trip() {
    for seed in 10_000..10_300 {
        let document = random_document(seed, 14);

        assert_valid(&document);
        assert_references_are_acyclic_by_construction(&document);

        let descendants = descendants_of(&document, document.root).expect("valid descendants");
        assert!(!descendants.contains(&document.root));

        let json = serde_json::to_string(&document).expect("document serializes");
        let round_tripped: ShapeDocument =
            serde_json::from_str(&json).expect("document deserializes");
        assert_eq!(document, round_tripped);
    }
}

#[test]
fn random_parameter_get_set_round_trips_preserve_validity() {
    for seed in 20_000..20_080 {
        let document = random_document(seed, 10);
        assert_valid(&document);

        for descriptor in enumerate_parameters(&document) {
            let mut edited = document.clone();
            let mut rng =
                TestRng::new(seed ^ descriptor.path.node.0 ^ descriptor.path.key.len() as u64);
            let value = random_descriptor_value(&mut rng, descriptor.minimum, descriptor.maximum);

            set_scalar(&mut edited, &descriptor.path, value).expect("set scalar succeeds");
            let actual = get_scalar(&edited, &descriptor.path).expect("get scalar succeeds");

            assert!(
                (actual - value).abs() <= PARAM_EPSILON,
                "round trip failed for {:?}: {actual} != {value}",
                descriptor.path
            );
            assert_valid(&edited);
        }
    }
}

#[test]
fn malformed_documents_and_bad_edits_fail_without_panicking() {
    let dangling = document_with_root(node(
        NodeId(1),
        NodeKind::Union {
            children: vec![NodeId(99)],
        },
        Transform3::default(),
    ));
    assert!(issue_codes(&dangling).contains("dangling_reference"));

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
        root: NodeId(2),
        nodes: cycle_nodes,
        next_node_id: 3,
        locks: BTreeSet::new(),
    };
    assert!(issue_codes(&cycle).contains("cycle"));

    let mut invalid = document_with_root(node(
        NodeId(1),
        NodeKind::Primitive(PrimitiveKind::Sphere { radius: -1.0 }),
        Transform3 {
            translation: Vec3::new(f32::NAN, 0.0, 0.0),
            scale: Vec3::new(0.0, 1.0, 1.0),
            ..Transform3::default()
        },
    ));
    invalid.next_node_id = 1;
    let invalid_codes = issue_codes(&invalid);
    assert!(invalid_codes.contains("invalid_dimension"));
    assert!(invalid_codes.contains("near_zero_scale"));
    assert!(invalid_codes.contains("non_finite"));
    assert!(invalid_codes.contains("next_node_id_not_fresh"));

    let mut document = document_with_root(node(
        NodeId(1),
        NodeKind::Primitive(PrimitiveKind::Sphere { radius: 1.0 }),
        Transform3::default(),
    ));
    let original = document.clone();
    let missing_path = ParamPath {
        node: NodeId(999),
        key: "primitive.radius".to_owned(),
    };
    assert!(set_scalar(&mut document, &missing_path, 1.25).is_err());
    assert_eq!(document, original, "failed scalar edits must be atomic");

    let radius_path = ParamPath {
        node: NodeId(1),
        key: "primitive.radius".to_owned(),
    };
    assert!(set_scalar(&mut document, &radius_path, f32::NAN).is_err());
    assert_eq!(document, original, "non-finite scalar edits must be atomic");

    let bad_edit = EditProgram {
        label: "invalid radius".to_owned(),
        seed: 44,
        operations: vec![SetScalarEdit {
            path: radius_path,
            before: 1.0,
            after: -0.5,
        }],
    };
    assert!(apply_edit(&document, &bad_edit).is_err());
}

#[test]
fn several_hundred_seeded_documents_do_not_panic() {
    for seed in 30_000..30_400 {
        let result = panic::catch_unwind(|| {
            let document = random_document(seed, 16);
            let report = validate_document(&document);
            assert!(
                report.is_valid(),
                "seed {seed} produced {:?}",
                report.issues
            );
            let _ = enumerate_parameters(&document);
            let _ = descendants_of(&document, document.root).expect("valid descendants");
            let json = serde_json::to_string(&document).expect("document serializes");
            let _: ShapeDocument = serde_json::from_str(&json).expect("document deserializes");
        });

        assert!(result.is_ok(), "seed {seed} panicked");
    }
}

fn random_document(seed: u64, max_nodes: usize) -> ShapeDocument {
    let mut rng = TestRng::new(seed);
    let node_count = 1 + rng.usize(max_nodes);
    let mut nodes = BTreeMap::new();

    for number in 1..=node_count {
        let id = NodeId(number as u64);
        let force_csg_root = number == node_count && number > 1 && rng.chance(70);
        let kind = if number == 1 || (!force_csg_root && rng.chance(45)) {
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

    ShapeDocument {
        schema_version: 1,
        title: format!("random {seed}"),
        root: NodeId(node_count as u64),
        next_node_id: node_count as u64 + 1,
        nodes,
        locks: BTreeSet::new(),
    }
}

fn random_primitive(rng: &mut TestRng) -> PrimitiveKind {
    match rng.usize(5) {
        0 => PrimitiveKind::Sphere {
            radius: rng.f32(0.05, 2.5),
        },
        1 => {
            let half_extents = Vec3::new(rng.f32(0.2, 2.5), rng.f32(0.2, 2.5), rng.f32(0.2, 2.5));
            PrimitiveKind::RoundedBox {
                half_extents,
                roundness: rng.f32(0.0, half_extents.min_element() * 0.65),
            }
        }
        2 => PrimitiveKind::Capsule {
            half_length: rng.f32(0.05, 2.5),
            radius: rng.f32(0.05, 1.25),
        },
        3 => {
            let half_height = rng.f32(0.1, 2.5);
            let radius = rng.f32(0.05, 1.5);
            PrimitiveKind::Cylinder {
                half_height,
                radius,
                roundness: rng.f32(0.0, half_height.min(radius) * 0.65),
            }
        }
        _ => {
            let minor_radius = rng.f32(0.04, 0.7);
            PrimitiveKind::Torus {
                major_radius: rng.f32(minor_radius + 0.05, 2.5),
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
            smoothness: rng.f32(0.0, 0.8),
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
        translation: Vec3::new(rng.f32(-1.5, 1.5), rng.f32(-1.5, 1.5), rng.f32(-1.5, 1.5)),
        rotation_degrees: Vec3::new(
            rng.f32(-180.0, 180.0),
            rng.f32(-180.0, 180.0),
            rng.f32(-180.0, 180.0),
        ),
        scale: Vec3::new(rng.f32(0.25, 2.0), rng.f32(0.25, 2.0), rng.f32(0.25, 2.0)),
    }
}

fn random_descriptor_value(rng: &mut TestRng, minimum: f32, maximum: f32) -> f32 {
    if (maximum - minimum).abs() <= f32::EPSILON {
        minimum
    } else {
        rng.f32(minimum, maximum)
    }
}

fn document_with_root(root: ShapeNode) -> ShapeDocument {
    ShapeDocument::new("test", root)
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

fn assert_valid(document: &ShapeDocument) {
    let report = validate_document(document);
    assert!(
        report.is_valid(),
        "expected valid document, got {:?}",
        report.issues
    );
}

fn issue_codes(document: &ShapeDocument) -> BTreeSet<String> {
    validate_document(document)
        .issues
        .into_iter()
        .map(|issue| issue.code)
        .collect()
}

fn assert_references_are_acyclic_by_construction(document: &ShapeDocument) {
    for (id, node) in &document.nodes {
        for reference in referenced_nodes(&node.kind) {
            assert!(
                reference.0 < id.0,
                "node {id:?} references non-previous node {reference:?}"
            );
        }
    }
}

fn referenced_nodes(kind: &NodeKind) -> Vec<NodeId> {
    match kind {
        NodeKind::Primitive(_) => Vec::new(),
        NodeKind::Union { children }
        | NodeKind::SmoothUnion { children, .. }
        | NodeKind::Intersection { children } => children.clone(),
        NodeKind::Difference { base, subtractors } => {
            let mut result = vec![*base];
            result.extend(subtractors.iter().copied());
            result
        }
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
