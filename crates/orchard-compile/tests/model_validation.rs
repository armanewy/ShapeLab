use std::collections::{BTreeMap, BTreeSet};

use orchard_asset::{
    AssetId, AssetPartSelector, AssetRecipe, AssetRelationshipPolicy, Frame3, GeometryRecipe,
    GeometrySource, ModelingOperationSpec, OperationId, PartDefinition, PartDefinitionId,
    PartInstance, PartInstanceId, RegionId, RelationshipPairing, SocketId, SocketSpec, Transform3,
};
use orchard_compile::validation::{
    ExpectedAttachment, ModelValidationConfig, ModelValidationReport, PartRelationship,
    ValidationLimits, validate_model, validation_config_from_recipe,
};
use orchard_compile::{
    AssetArtifact, CompileStatistics, CompileValidationReport, CompiledPart, ProvenanceReport,
};
use orchard_poly::{
    ElementId, FaceMetadata, MeshBounds, PolygonFace, PolygonMesh, TriangleMesh,
    TriangulatedPolygonMesh, bounds_from_positions, compute_topology_signature,
    polygon_mesh_from_faces, triangulate_polygon_mesh,
};

fn part_id(id: u64) -> PartInstanceId {
    PartInstanceId(id)
}

fn definition_id(id: u64) -> PartDefinitionId {
    PartDefinitionId(id)
}

fn socket_id(id: u64) -> SocketId {
    SocketId(id)
}

fn region_id(id: u64) -> RegionId {
    RegionId(id)
}

fn metadata(part: PartInstanceId, count: usize) -> Vec<FaceMetadata> {
    vec![
        FaceMetadata {
            part_definition: Some(definition_id(part.0)),
            part_instance: Some(part),
            region: Some(region_id(1)),
            operation: None,
            smoothing_group: None,
            surface_role: None,
        };
        count
    ]
}

fn frame(origin: [f32; 3]) -> Frame3 {
    Frame3 {
        origin,
        x_axis: [1.0, 0.0, 0.0],
        y_axis: [0.0, 1.0, 0.0],
        z_axis: [0.0, 0.0, 1.0],
    }
}

fn socket(id: SocketId, origin: [f32; 3]) -> SocketSpec {
    SocketSpec {
        id,
        name: format!("socket_{}", id.0),
        local_frame: frame(origin),
        role: "attachment".to_owned(),
        tags: BTreeSet::new(),
    }
}

fn cube_mesh(part: PartInstanceId, center: [f32; 3], size: [f32; 3]) -> PolygonMesh {
    let half = [size[0] * 0.5, size[1] * 0.5, size[2] * 0.5];
    let positions = vec![
        [
            center[0] - half[0],
            center[1] - half[1],
            center[2] - half[2],
        ],
        [
            center[0] + half[0],
            center[1] - half[1],
            center[2] - half[2],
        ],
        [
            center[0] + half[0],
            center[1] + half[1],
            center[2] - half[2],
        ],
        [
            center[0] - half[0],
            center[1] + half[1],
            center[2] - half[2],
        ],
        [
            center[0] - half[0],
            center[1] - half[1],
            center[2] + half[2],
        ],
        [
            center[0] + half[0],
            center[1] - half[1],
            center[2] + half[2],
        ],
        [
            center[0] + half[0],
            center[1] + half[1],
            center[2] + half[2],
        ],
        [
            center[0] - half[0],
            center[1] + half[1],
            center[2] + half[2],
        ],
    ];
    let faces = vec![
        vec![0, 3, 2, 1],
        vec![4, 5, 6, 7],
        vec![0, 1, 5, 4],
        vec![3, 7, 6, 2],
        vec![0, 4, 7, 3],
        vec![1, 2, 6, 5],
    ];
    polygon_mesh_from_faces(positions, faces, metadata(part, 6)).expect("cube mesh")
}

fn compiled_part(
    id: PartInstanceId,
    name: &str,
    mesh: PolygonMesh,
    sockets_world: BTreeMap<SocketId, SocketSpec>,
) -> CompiledPart {
    let triangulated_world =
        triangulate_polygon_mesh(&mesh).unwrap_or_else(|_| lossy_triangles(&mesh));
    CompiledPart {
        definition_id: definition_id(id.0),
        instance_id: id,
        instance_name: name.to_owned(),
        prototype_instance_id: None,
        generated_by: None,
        source_recipe_instance: true,
        local_mesh: mesh.clone(),
        world_mesh: mesh,
        triangulated_world,
        sockets_world,
        validation_report: CompileValidationReport::default(),
    }
}

fn cube_part(id: u64, name: &str, center: [f32; 3], size: [f32; 3]) -> CompiledPart {
    compiled_part(
        part_id(id),
        name,
        cube_mesh(part_id(id), center, size),
        BTreeMap::new(),
    )
}

fn lossy_triangles(mesh: &PolygonMesh) -> TriangulatedPolygonMesh {
    let mut indices = Vec::new();
    let mut triangle_to_polygon_face = Vec::new();
    let mut triangle_to_region = Vec::new();
    let mut triangle_to_part = Vec::new();
    let mut triangle_to_operation = Vec::new();
    for (face_index, face) in mesh.faces.iter().enumerate() {
        if face.vertices.len() < 3 {
            continue;
        }
        for local_index in 1..face.vertices.len() - 1 {
            indices.push(face.vertices[0]);
            indices.push(face.vertices[local_index]);
            indices.push(face.vertices[local_index + 1]);
            triangle_to_polygon_face.push(face.id);
            let metadata = mesh
                .face_metadata
                .get(face_index)
                .cloned()
                .unwrap_or_default();
            triangle_to_region.push(metadata.region);
            triangle_to_part.push(metadata.part_instance);
            triangle_to_operation.push(metadata.operation);
        }
    }
    TriangulatedPolygonMesh {
        mesh: TriangleMesh {
            positions: mesh.positions.clone(),
            normals: vec![[0.0, 0.0, 1.0]; mesh.positions.len()],
            indices,
            bounds: mesh.bounds,
        },
        triangle_to_polygon_face,
        triangle_to_region,
        triangle_to_part,
        triangle_to_operation,
        vertex_ids: mesh.vertex_ids.clone(),
    }
}

fn empty_triangulated() -> TriangulatedPolygonMesh {
    TriangulatedPolygonMesh {
        mesh: TriangleMesh {
            positions: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
            bounds: MeshBounds::empty(),
        },
        triangle_to_polygon_face: Vec::new(),
        triangle_to_region: Vec::new(),
        triangle_to_part: Vec::new(),
        triangle_to_operation: Vec::new(),
        vertex_ids: Vec::new(),
    }
}

fn artifact(parts: Vec<CompiledPart>) -> AssetArtifact {
    let part_count = parts.len() as u64;
    let combined_polygon = PolygonMesh::empty();
    let combined_preview = empty_triangulated();
    let polygon_vertex_count = parts
        .iter()
        .map(|part| part.world_mesh.positions.len() as u64)
        .sum();
    let polygon_face_count = parts
        .iter()
        .map(|part| part.world_mesh.faces.len() as u64)
        .sum();
    let triangle_count = parts
        .iter()
        .map(|part| (part.triangulated_world.mesh.indices.len() / 3) as u64)
        .sum();
    AssetArtifact {
        source_recipe_hash: 1,
        provenance_report: ProvenanceReport {
            definition_generation_order: parts
                .iter()
                .map(|part| part.definition_id)
                .collect::<Vec<_>>(),
            instance_order: parts
                .iter()
                .map(|part| part.instance_id)
                .collect::<Vec<_>>(),
            part_region_operation_mappings: Vec::new(),
            element_counts: BTreeMap::new(),
            topology_signatures: parts
                .iter()
                .map(|part| (part.instance_id, part.world_mesh.topology_signature))
                .collect(),
        },
        compiled_parts: parts,
        combined_polygon,
        combined_preview,
        validation_report: CompileValidationReport::default(),
        statistics: CompileStatistics {
            part_count,
            polygon_vertex_count,
            polygon_face_count,
            triangle_count,
            used_sdf_or_remeshing: false,
        },
    }
}

fn issue_codes(report: &ModelValidationReport) -> Vec<&str> {
    report
        .issues
        .iter()
        .map(|issue| issue.code.as_str())
        .collect()
}

fn has_code(report: &ModelValidationReport, code: &str) -> bool {
    report.issues.iter().any(|issue| issue.code == code)
}

fn attachment_config() -> ModelValidationConfig {
    ModelValidationConfig {
        required_parts: BTreeSet::from([part_id(1), part_id(2)]),
        expected_attachments: vec![ExpectedAttachment {
            parent: part_id(1),
            child: part_id(2),
            parent_socket: socket_id(1),
            child_socket: socket_id(1),
            max_origin_distance: 0.01,
            max_axis_angle_degrees: 1.0,
            max_clearance: Some(0.05),
        }],
        ..ModelValidationConfig::default()
    }
}

fn selector_recipe() -> AssetRecipe {
    let operation = OperationId(7);
    let definition = PartDefinition {
        id: definition_id(1),
        name: "Panel".to_owned(),
        tags: BTreeSet::from(["panel".to_owned()]),
        geometry: GeometryRecipe {
            source: GeometrySource::RoundedBox {
                half_extents: [0.5, 0.5, 0.5],
                radius: 0.02,
            },
            operations: vec![ModelingOperationSpec::LinearArray {
                operation,
                count: 3,
                offset: [1.0, 0.0, 0.0],
            }],
        },
        regions: BTreeMap::new(),
        sockets: BTreeMap::new(),
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
    };
    let instance = PartInstance {
        id: part_id(1),
        definition: definition_id(1),
        name: "Panel prototype".to_owned(),
        parent: None,
        local_transform: Transform3::default(),
        attachment: None,
        enabled: true,
        tags: BTreeSet::new(),
        generated_by: None,
    };
    let mut recipe = AssetRecipe::new(AssetId(1), "Selector test");
    recipe.definitions.insert(definition.id, definition);
    recipe.instances.insert(instance.id, instance);
    recipe.root_instances.push(part_id(1));
    recipe
        .relationships
        .push(AssetRelationshipPolicy::MayOverlap {
            first: AssetPartSelector::specific(part_id(1)),
            second: AssetPartSelector::GeneratedByOperation { operation },
            pairing: RelationshipPairing::AllPairs,
            reason: "generated panels intentionally interlock".to_owned(),
        });
    recipe
}

fn generated_part(id: u64, prototype: PartInstanceId, operation: OperationId) -> CompiledPart {
    let mut part = cube_part(id, "generated", [id as f32, 0.0, 0.0], [0.5, 0.5, 0.5]);
    part.definition_id = definition_id(1);
    part.prototype_instance_id = Some(prototype);
    part.generated_by = Some(operation);
    part.source_recipe_instance = false;
    part
}

fn instance(id: u64, definition: u64) -> PartInstance {
    PartInstance {
        id: part_id(id),
        definition: definition_id(definition),
        name: format!("part_{id}"),
        parent: None,
        local_transform: Transform3::default(),
        attachment: None,
        enabled: true,
        tags: BTreeSet::new(),
        generated_by: None,
    }
}

fn tagged_definition(id: u64, tag: &str) -> PartDefinition {
    PartDefinition {
        id: definition_id(id),
        name: format!("definition_{id}"),
        tags: BTreeSet::from([tag.to_owned()]),
        geometry: GeometryRecipe {
            source: GeometrySource::RoundedBox {
                half_extents: [0.5, 0.5, 0.5],
                radius: 0.02,
            },
            operations: Vec::new(),
        },
        regions: BTreeMap::new(),
        sockets: BTreeMap::new(),
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
    }
}

#[test]
fn valid_crate_like_assembly_reports_clean_metrics() {
    let mut body = cube_part(1, "crate_body", [0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
    body.sockets_world
        .insert(socket_id(1), socket(socket_id(1), [1.0, 0.0, 0.0]));
    let mut handle = cube_part(2, "handle", [1.25, 0.0, 0.0], [0.5, 0.5, 0.5]);
    handle
        .sockets_world
        .insert(socket_id(1), socket(socket_id(1), [1.0, 0.0, 0.0]));
    let artifact = artifact(vec![body, handle]);

    let report = validate_model(&artifact, &attachment_config());

    assert!(report.issues.is_empty(), "{:?}", issue_codes(&report));
    assert!(report.is_valid());
    assert_eq!(report.metrics.part_count, 2);
    assert_eq!(report.metrics.triangle_count, 24);
    assert_eq!(report.metrics.quad_fraction, 1.0);
    assert_eq!(report.metrics.manifold_closed_part_fraction, 1.0);
    assert_eq!(report.metrics.provenance_coverage, 1.0);
}

#[test]
fn recipe_validation_config_expands_generated_operation_selectors() {
    let recipe = selector_recipe();
    let artifact = artifact(vec![
        cube_part(1, "prototype", [0.0, 0.0, 0.0], [0.5, 0.5, 0.5]),
        generated_part(11, part_id(1), OperationId(7)),
        generated_part(12, part_id(1), OperationId(7)),
    ]);

    let config = validation_config_from_recipe(&recipe, &artifact);

    assert_eq!(config.required_parts, BTreeSet::from([part_id(1)]));
    assert_eq!(config.relationships.len(), 2);
    assert!(config.relationships.iter().any(|relationship| matches!(
        relationship,
        PartRelationship::IntentionalOverlap { first, second, .. }
            if *first == part_id(1) && *second == part_id(11)
    )));
    assert!(config.relationships.iter().any(|relationship| matches!(
        relationship,
        PartRelationship::IntentionalOverlap { first, second, .. }
            if *first == part_id(1) && *second == part_id(12)
    )));
}

#[test]
fn relationship_pairing_by_occurrence_index_avoids_cartesian_expansion() {
    let mut recipe = AssetRecipe::new(AssetId(2), "Pairing test");
    recipe
        .definitions
        .insert(definition_id(1), tagged_definition(1, "parent"));
    recipe
        .definitions
        .insert(definition_id(2), tagged_definition(2, "child"));
    for instance in [
        instance(11, 1),
        instance(12, 1),
        instance(21, 2),
        instance(22, 2),
    ] {
        recipe.root_instances.push(instance.id);
        recipe.instances.insert(instance.id, instance);
    }
    recipe
        .relationships
        .push(AssetRelationshipPolicy::MustTouch {
            first: AssetPartSelector::PartTag {
                tag: "parent".to_owned(),
            },
            second: AssetPartSelector::PartTag {
                tag: "child".to_owned(),
            },
            pairing: RelationshipPairing::ByOccurrenceIndex,
            max_clearance: 0.05,
        });
    let mut parent_a = cube_part(11, "parent_a", [0.0, 0.0, 0.0], [0.5, 0.5, 0.5]);
    let mut parent_b = cube_part(12, "parent_b", [2.0, 0.0, 0.0], [0.5, 0.5, 0.5]);
    let mut child_a = cube_part(21, "child_a", [0.5, 0.0, 0.0], [0.5, 0.5, 0.5]);
    let mut child_b = cube_part(22, "child_b", [2.5, 0.0, 0.0], [0.5, 0.5, 0.5]);
    parent_a.definition_id = definition_id(1);
    parent_b.definition_id = definition_id(1);
    child_a.definition_id = definition_id(2);
    child_b.definition_id = definition_id(2);
    let artifact = artifact(vec![parent_a, parent_b, child_a, child_b]);

    let config = validation_config_from_recipe(&recipe, &artifact);

    assert_eq!(config.relationships.len(), 2);
    assert!(config.relationships.iter().any(|relationship| matches!(
        relationship,
        PartRelationship::MustTouch { first, second, .. }
            if *first == part_id(11) && *second == part_id(21)
    )));
    assert!(config.relationships.iter().any(|relationship| matches!(
        relationship,
        PartRelationship::MustTouch { first, second, .. }
            if *first == part_id(12) && *second == part_id(22)
    )));
}

#[test]
fn conflicting_overlap_and_must_not_intersect_policies_report_error() {
    let mut recipe = AssetRecipe::new(AssetId(3), "Conflict test");
    recipe
        .relationships
        .push(AssetRelationshipPolicy::MayOverlap {
            first: AssetPartSelector::specific(part_id(1)),
            second: AssetPartSelector::specific(part_id(2)),
            pairing: RelationshipPairing::AllPairs,
            reason: "legacy contact".to_owned(),
        });
    recipe
        .relationships
        .push(AssetRelationshipPolicy::MustNotIntersect {
            first: AssetPartSelector::specific(part_id(1)),
            second: AssetPartSelector::specific(part_id(2)),
            pairing: RelationshipPairing::AllPairs,
        });
    let artifact = artifact(vec![
        cube_part(1, "body", [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        cube_part(2, "bolt", [0.25, 0.0, 0.0], [1.0, 1.0, 1.0]),
    ]);

    let config = validation_config_from_recipe(&recipe, &artifact);
    let report = validate_model(&artifact, &config);

    assert!(has_code(&report, "conflicting_relationship_policy"));
    assert!(has_code(&report, "triangle_intersection"));
}

#[test]
fn detached_handle_reports_required_part_detachment() {
    let mut body = cube_part(1, "crate_body", [0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
    body.sockets_world
        .insert(socket_id(1), socket(socket_id(1), [1.0, 0.0, 0.0]));
    let mut handle = cube_part(2, "handle", [3.0, 0.0, 0.0], [0.5, 0.5, 0.5]);
    handle
        .sockets_world
        .insert(socket_id(1), socket(socket_id(1), [1.0, 0.0, 0.0]));
    let artifact = artifact(vec![body, handle]);

    let report = validate_model(&artifact, &attachment_config());

    assert!(has_code(&report, "detached_required_part"));
}

#[test]
fn intersecting_bolt_reports_aabb_and_narrow_phase_intersection() {
    let body = cube_part(1, "crate_body", [0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
    let bolt = cube_part(2, "bolt", [1.1, 0.0, 0.0], [0.4, 0.4, 0.4]);
    let artifact = artifact(vec![body, bolt]);

    let report = validate_model(&artifact, &ModelValidationConfig::default());

    assert!(has_code(&report, "accidental_aabb_overlap"));
    assert!(has_code(&report, "triangle_intersection"));
    assert_eq!(report.metrics.accidental_intersection_count, 1);
}

#[test]
fn intentional_overlap_metadata_suppresses_accidental_overlap() {
    let body = cube_part(1, "crate_body", [0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
    let bolt = cube_part(2, "bolt", [1.1, 0.0, 0.0], [0.4, 0.4, 0.4]);
    let artifact = artifact(vec![body, bolt]);
    let config = ModelValidationConfig {
        relationships: vec![PartRelationship::intentional_overlap(
            part_id(1),
            part_id(2),
            "press-fit insert",
        )],
        ..ModelValidationConfig::default()
    };

    let report = validate_model(&artifact, &config);

    assert!(!has_code(&report, "accidental_aabb_overlap"));
    assert!(!has_code(&report, "triangle_intersection"));
    assert_eq!(report.metrics.accidental_intersection_count, 0);
}

#[test]
fn nonmanifold_part_reports_nonmanifold_edge() {
    let positions = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0],
    ];
    let faces = vec![
        PolygonFace {
            id: ElementId(0),
            vertices: vec![0, 1, 2],
        },
        PolygonFace {
            id: ElementId(1),
            vertices: vec![1, 0, 3],
        },
        PolygonFace {
            id: ElementId(2),
            vertices: vec![0, 1, 4],
        },
    ];
    let mesh = PolygonMesh {
        vertex_ids: (0..positions.len())
            .map(|index| ElementId(index as u64))
            .collect(),
        bounds: bounds_from_positions(&positions).expect("bounds"),
        topology_signature: compute_topology_signature(&positions, &faces),
        positions,
        faces,
        face_metadata: metadata(part_id(1), 3),
        edge_metadata: BTreeMap::new(),
    };
    let artifact = artifact(vec![compiled_part(
        part_id(1),
        "bad",
        mesh,
        BTreeMap::new(),
    )]);

    let report = validate_model(&artifact, &ModelValidationConfig::default());

    assert!(has_code(&report, "nonmanifold_edge"));
    assert_eq!(report.metrics.manifold_closed_part_fraction, 0.0);
}

#[test]
fn tiny_face_reports_minimum_face_area() {
    let mesh = polygon_mesh_from_faces(
        vec![[0.0, 0.0, 0.0], [0.001, 0.0, 0.0], [0.0, 0.001, 0.0]],
        vec![vec![0, 1, 2]],
        metadata(part_id(1), 1),
    )
    .expect("tiny triangle is still valid topology");
    let artifact = artifact(vec![compiled_part(
        part_id(1),
        "tiny",
        mesh,
        BTreeMap::new(),
    )]);
    let config = ModelValidationConfig {
        limits: ValidationLimits {
            minimum_face_area: 0.0001,
            ..ValidationLimits::default()
        },
        ..ModelValidationConfig::default()
    };

    let report = validate_model(&artifact, &config);

    assert!(has_code(&report, "minimum_face_area"));
}

#[test]
fn inverted_winding_reports_inverted_normals() {
    let mut mesh = cube_mesh(part_id(1), [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    for face in &mut mesh.faces {
        face.vertices.reverse();
    }
    mesh.topology_signature = compute_topology_signature(&mesh.positions, &mesh.faces);
    let artifact = artifact(vec![compiled_part(
        part_id(1),
        "inside_out",
        mesh,
        BTreeMap::new(),
    )]);

    let report = validate_model(&artifact, &ModelValidationConfig::default());

    assert!(has_code(&report, "inverted_normal"));
}

#[test]
fn polygon_budget_violation_reports_excessive_triangle_count() {
    let artifact = artifact(vec![cube_part(
        1,
        "crate_body",
        [0.0, 0.0, 0.0],
        [2.0, 2.0, 2.0],
    )]);
    let config = ModelValidationConfig {
        limits: ValidationLimits {
            maximum_triangle_count: 6,
            ..ValidationLimits::default()
        },
        ..ModelValidationConfig::default()
    };

    let report = validate_model(&artifact, &config);

    assert!(has_code(&report, "excessive_total_triangle_count"));
}

#[test]
fn complete_provenance_reports_full_coverage() {
    let artifact = artifact(vec![cube_part(
        1,
        "crate_body",
        [0.0, 0.0, 0.0],
        [2.0, 2.0, 2.0],
    )]);

    let report = validate_model(&artifact, &ModelValidationConfig::default());

    assert_eq!(report.metrics.provenance_coverage, 1.0);
    assert_eq!(report.metrics.region_count, 1);
}

#[test]
fn issue_ordering_is_deterministic() {
    let mut detached = cube_part(2, "handle", [3.0, 0.0, 0.0], [0.5, 0.5, 0.5]);
    detached
        .sockets_world
        .insert(socket_id(1), socket(socket_id(1), [1.0, 0.0, 0.0]));
    let artifact = artifact(vec![
        cube_part(1, "crate_body", [0.0, 0.0, 0.0], [2.0, 2.0, 2.0]),
        detached,
    ]);
    let config = ModelValidationConfig {
        limits: ValidationLimits {
            maximum_triangle_count: 12,
            ..ValidationLimits::default()
        },
        ..attachment_config()
    };

    let first = validate_model(&artifact, &config);
    let second = validate_model(&artifact, &config);

    assert_eq!(first.issues, second.issues);
    assert!(has_code(&first, "excessive_total_triangle_count"));
    assert!(has_code(&first, "missing_attachment"));
}
