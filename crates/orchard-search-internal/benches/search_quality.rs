#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::time::Instant;

use orchard_core_legacy::{
    NodeId, NodeKind, ParamGroup, PrimitiveKind, ShapeDocument, ShapeNode, Transform3,
};
use orchard_search_internal::{
    ExplorationMode, SearchRequest, TargetScope, generate_candidates_with_diagnostics,
};

macro_rules! vec3_value {
    ($x:expr, $y:expr, $z:expr) => {{
        let mut value = Transform3::default().translation;
        value.x = $x;
        value.y = $y;
        value.z = $z;
        value
    }};
}

fn all_groups() -> BTreeSet<ParamGroup> {
    [
        ParamGroup::Form,
        ParamGroup::Placement,
        ParamGroup::Rotation,
        ParamGroup::Scale,
        ParamGroup::Blend,
    ]
    .into_iter()
    .collect()
}

fn request(seed: u64, mode: ExplorationMode) -> SearchRequest {
    SearchRequest {
        seed,
        proposal_count: 96,
        result_count: 6,
        descriptor_resolution: 7,
        selected_node: None,
        target_scope: TargetScope::WholeModel,
        enabled_groups: all_groups(),
        mode,
    }
}

fn primitive_node(id: u64, name: &str, kind: PrimitiveKind) -> ShapeNode {
    ShapeNode {
        id: NodeId(id),
        name: name.to_owned(),
        tags: BTreeSet::new(),
        enabled: true,
        transform: Transform3::default(),
        kind: NodeKind::Primitive(kind),
    }
}

fn benchmark_document() -> ShapeDocument {
    let root = ShapeNode {
        id: NodeId(1),
        name: "Benchmark model".to_owned(),
        tags: BTreeSet::new(),
        enabled: true,
        transform: Transform3::default(),
        kind: NodeKind::SmoothUnion {
            children: vec![NodeId(2), NodeId(3), NodeId(4)],
            smoothness: 0.12,
        },
    };
    let mut document = ShapeDocument::new("benchmark", root);
    let mut sphere = primitive_node(2, "Sphere", PrimitiveKind::Sphere { radius: 0.72 });
    sphere.transform.translation = vec3_value!(-0.52, 0.0, 0.0);
    let mut block = primitive_node(
        3,
        "Block",
        PrimitiveKind::RoundedBox {
            half_extents: vec3_value!(0.46, 0.82, 0.36),
            roundness: 0.08,
        },
    );
    block.transform.translation = vec3_value!(0.35, 0.04, 0.0);
    let mut capsule = primitive_node(
        4,
        "Capsule",
        PrimitiveKind::Capsule {
            half_length: 0.54,
            radius: 0.21,
        },
    );
    capsule.transform.translation = vec3_value!(0.0, 1.02, 0.0);
    document.nodes.insert(NodeId(2), sphere);
    document.nodes.insert(NodeId(3), block);
    document.nodes.insert(NodeId(4), capsule);
    document.next_node_id = 5;
    document
}

fn main() {
    let document = benchmark_document();
    let runs = 12;
    let start = Instant::now();
    let mut total_candidates = 0_usize;
    let mut total_attempts = 0_usize;
    for run in 0..runs {
        let result = generate_candidates_with_diagnostics(
            &document,
            &request(900 + run as u64, ExplorationMode::Explore),
        )
        .unwrap_or_else(|error| panic!("search benchmark failed: {error}"));
        total_candidates += result.candidates.len();
        total_attempts += result.diagnostics.attempted_proposals;
    }
    let elapsed = start.elapsed();
    println!(
        "orchard_search_internal_explore_diagnostics: runs={runs} candidates={total_candidates} attempts={total_attempts} elapsed_ms={:.3}",
        elapsed.as_secs_f64() * 1000.0
    );
}
