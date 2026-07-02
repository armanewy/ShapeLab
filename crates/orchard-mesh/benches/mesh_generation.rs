use std::collections::{BTreeMap, BTreeSet};

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use glam::Vec3;
use orchard_core_legacy::{NodeId, NodeKind, PrimitiveKind, ShapeDocument, ShapeNode, Transform3};
use orchard_field::compile_document;
use orchard_mesh::{MeshSettings, mesh_field};

fn candidate_meshing_36_cubed(c: &mut Criterion) {
    let document = representative_document();
    let field = compile_document(&document).expect("representative document compiles");
    let settings = MeshSettings {
        resolution: 36,
        padding_fraction: 0.12,
        iso_value: 0.0,
    };

    let mut group = c.benchmark_group("candidate_meshing");
    group.sample_size(10);
    group.bench_function("36_cubed_candidate_meshing", |bench| {
        bench.iter(|| {
            let mesh = mesh_field(black_box(&field), black_box(settings)).expect("mesh succeeds");
            black_box(mesh.indices.len());
        });
    });
    group.finish();
}

fn current_meshing_56_cubed(c: &mut Criterion) {
    let document = representative_document();
    let field = compile_document(&document).expect("representative document compiles");
    let settings = MeshSettings {
        resolution: 56,
        padding_fraction: 0.12,
        iso_value: 0.0,
    };

    let mut group = c.benchmark_group("current_meshing");
    group.sample_size(10);
    group.bench_function("56_cubed_current_meshing", |bench| {
        bench.iter(|| {
            let mesh = mesh_field(black_box(&field), black_box(settings)).expect("mesh succeeds");
            black_box(mesh.indices.len());
        });
    });
    group.finish();
}

fn representative_document() -> ShapeDocument {
    let mut nodes = BTreeMap::new();
    nodes.insert(
        NodeId(1),
        node(
            NodeId(1),
            NodeKind::Primitive(PrimitiveKind::Sphere { radius: 0.9 }),
            Transform3 {
                translation: Vec3::new(-0.45, 0.0, 0.0),
                ..Transform3::default()
            },
        ),
    );
    nodes.insert(
        NodeId(2),
        node(
            NodeId(2),
            NodeKind::Primitive(PrimitiveKind::RoundedBox {
                half_extents: Vec3::new(0.62, 0.52, 0.46),
                roundness: 0.1,
            }),
            Transform3 {
                translation: Vec3::new(0.45, 0.0, 0.05),
                rotation_degrees: Vec3::new(0.0, 28.0, 0.0),
                ..Transform3::default()
            },
        ),
    );
    nodes.insert(
        NodeId(3),
        node(
            NodeId(3),
            NodeKind::Primitive(PrimitiveKind::Cylinder {
                half_height: 0.68,
                radius: 0.3,
                roundness: 0.04,
            }),
            Transform3 {
                translation: Vec3::new(0.05, 0.18, -0.42),
                rotation_degrees: Vec3::new(18.0, 0.0, 12.0),
                ..Transform3::default()
            },
        ),
    );
    nodes.insert(
        NodeId(4),
        node(
            NodeId(4),
            NodeKind::SmoothUnion {
                children: vec![NodeId(1), NodeId(2), NodeId(3)],
                smoothness: 0.22,
            },
            Transform3::default(),
        ),
    );

    ShapeDocument {
        schema_version: 1,
        title: "representative mesh bench".to_owned(),
        root: NodeId(4),
        next_node_id: 5,
        nodes,
        locks: BTreeSet::new(),
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

criterion_group!(
    benches,
    candidate_meshing_36_cubed,
    current_meshing_56_cubed
);
criterion_main!(benches);
