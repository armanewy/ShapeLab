use std::collections::{BTreeMap, BTreeSet};

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use glam::Vec3;
use orchard_core_legacy::{NodeId, NodeKind, PrimitiveKind, ShapeDocument, ShapeNode, Transform3};
use orchard_field::{GridSpec, ScalarField, compile_document, sample_grid};

fn field_sampling_representative_graph_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("field_sampling_representative_graph_sizes");
    let points = sample_points();

    for graph_size in [1_usize, 8, 24] {
        let document = representative_document(graph_size);
        let field = compile_document(&document).expect("representative document compiles");
        group.throughput(Throughput::Elements(points.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(graph_size),
            &field,
            |bench, field| {
                bench.iter(|| {
                    let mut accumulator = 0.0_f32;
                    for point in &points {
                        accumulator += black_box(field.sample(*point));
                    }
                    black_box(accumulator);
                });
            },
        );
    }

    group.finish();
}

fn descriptor_sampling_16_cubed(c: &mut Criterion) {
    let document = representative_document(12);
    let field = compile_document(&document).expect("representative document compiles");
    let spec = GridSpec {
        bounds: field.bounds(),
        resolution_x: 16,
        resolution_y: 16,
        resolution_z: 16,
    };

    c.bench_function("descriptor_sampling_16_cubed", |bench| {
        bench.iter(|| {
            let samples = sample_grid(black_box(&field), black_box(spec)).expect("grid samples");
            black_box(samples.values.len());
        });
    });
}

fn representative_document(graph_size: usize) -> ShapeDocument {
    let clamped_size = graph_size.max(1);
    if clamped_size == 1 {
        return ShapeDocument::new(
            "single sphere",
            node(
                NodeId(1),
                NodeKind::Primitive(PrimitiveKind::Sphere { radius: 1.0 }),
                Transform3::default(),
            ),
        );
    }

    let mut nodes = BTreeMap::new();
    for number in 1..clamped_size {
        let id = NodeId(number as u64);
        let offset = number as f32 * 0.173;
        nodes.insert(
            id,
            node(
                id,
                primitive_for(number),
                Transform3 {
                    translation: Vec3::new(offset.sin() * 1.4, offset.cos() * 0.45, offset * 0.15),
                    rotation_degrees: Vec3::new(offset * 31.0, offset * 17.0, offset * 11.0),
                    scale: Vec3::splat(0.8 + (offset.sin().abs() * 0.45)),
                },
            ),
        );
    }

    let root_id = NodeId(clamped_size as u64);
    nodes.insert(
        root_id,
        node(
            root_id,
            NodeKind::SmoothUnion {
                children: (1..clamped_size)
                    .map(|number| NodeId(number as u64))
                    .collect(),
                smoothness: 0.18,
            },
            Transform3::default(),
        ),
    );

    ShapeDocument {
        schema_version: 1,
        title: format!("representative {graph_size}"),
        root: root_id,
        next_node_id: root_id.0 + 1,
        nodes,
        locks: BTreeSet::new(),
    }
}

fn primitive_for(number: usize) -> NodeKind {
    let kind = match number % 5 {
        0 => PrimitiveKind::Sphere { radius: 0.72 },
        1 => PrimitiveKind::RoundedBox {
            half_extents: Vec3::new(0.62, 0.45, 0.54),
            roundness: 0.08,
        },
        2 => PrimitiveKind::Capsule {
            half_length: 0.58,
            radius: 0.24,
        },
        3 => PrimitiveKind::Cylinder {
            half_height: 0.52,
            radius: 0.32,
            roundness: 0.05,
        },
        _ => PrimitiveKind::Torus {
            major_radius: 0.52,
            minor_radius: 0.14,
        },
    };
    NodeKind::Primitive(kind)
}

fn sample_points() -> Vec<Vec3> {
    (0..192)
        .map(|index| {
            let t = index as f32;
            Vec3::new(
                (t * 0.37).sin() * 2.2,
                (t * 0.19).cos() * 1.8,
                (t * 0.23).sin() * 2.0,
            )
        })
        .collect()
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
    field_sampling_representative_graph_sizes,
    descriptor_sampling_16_cubed
);
criterion_main!(benches);
