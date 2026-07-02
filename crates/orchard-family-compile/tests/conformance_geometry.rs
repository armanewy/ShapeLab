use std::collections::{BTreeMap, BTreeSet};

use orchard_asset::{
    AssetId, AssetRecipe, AttachmentMode, AttachmentSpec, Frame3, GeometryRecipe, GeometrySource,
    PartDefinition, PartDefinitionId, PartInstance, PartInstanceId, SocketId, SocketSpec,
    Transform3,
};
use orchard_compile::{AssetArtifact, compile_asset};
use orchard_family::{
    ConstraintKind, ExportRequirement, FamilyRuleExecutionPolicy, GeometricConstraint,
    RuntimeMetadataRequirement,
};
use orchard_family_compile::conformance::{
    ArtifactTriangleBudget, ConformanceStatus, ConstraintBindingMap, ExplicitConstraintBinding,
    ExportMetadataAvailability, FamilyConformanceReport, RoleBounds, RoleClearance,
    SocketConnection, SupportViaAttachment, evaluate_export_requirements,
    evaluate_geometric_constraints, validate_constraint_binding_coverage,
};

#[test]
fn floating_deck_fails_support_via_attachment() {
    let (recipe, artifact) = compile_fixture([
        PartSpec::new(1, "deck", [2.0, 0.1, 1.0], [0.0, 1.2, 0.0]),
        PartSpec::new(2, "support", [0.25, 0.5, 0.25], [0.0, 0.5, 0.0]),
    ]);
    let constraint = constraint(
        "deck_supported",
        ["deck", "support"],
        FamilyRuleExecutionPolicy::Required,
    );
    let rows = evaluate_geometric_constraints(
        &[constraint],
        &bindings([(
            "deck_supported",
            ExplicitConstraintBinding::SupportViaAttachment(SupportViaAttachment {
                max_clearance: 0.01,
                vertical_axis: 1,
            }),
        )]),
        &recipe,
        Some(&artifact),
    );

    assert_eq!(rows[0].status, ConformanceStatus::Failed);
    assert!(
        rows[0]
            .issue_codes
            .contains(&"support_attachment_missing".to_owned())
    );
}

#[test]
fn intersecting_supports_fail_clearance() {
    let (recipe, artifact) = compile_fixture([
        PartSpec::new(1, "support", [0.35, 0.5, 0.35], [0.0, 0.5, 0.0]),
        PartSpec::new(2, "support", [0.35, 0.5, 0.35], [0.2, 0.5, 0.0]),
    ]);
    let constraint = constraint(
        "support_clearance",
        ["support"],
        FamilyRuleExecutionPolicy::Required,
    );
    let rows = evaluate_geometric_constraints(
        &[constraint],
        &bindings([(
            "support_clearance",
            ExplicitConstraintBinding::RoleClearance(RoleClearance { minimum: 0.05 }),
        )]),
        &recipe,
        Some(&artifact),
    );

    assert_eq!(rows[0].status, ConformanceStatus::Failed);
    assert!(
        rows[0]
            .issue_codes
            .contains(&"role_clearance_below_minimum".to_owned())
    );
}

#[test]
fn separated_supports_pass_clearance() {
    let (recipe, artifact) = compile_fixture([
        PartSpec::new(1, "support", [0.2, 0.5, 0.2], [-0.8, 0.5, 0.0]),
        PartSpec::new(2, "support", [0.2, 0.5, 0.2], [0.8, 0.5, 0.0]),
    ]);
    let constraint = constraint(
        "support_clearance",
        ["support"],
        FamilyRuleExecutionPolicy::Required,
    );
    let rows = evaluate_geometric_constraints(
        &[constraint],
        &bindings([(
            "support_clearance",
            ExplicitConstraintBinding::RoleClearance(RoleClearance { minimum: 0.5 }),
        )]),
        &recipe,
        Some(&artifact),
    );

    assert_eq!(rows[0].status, ConformanceStatus::Passed);
    assert!(rows[0].measurements[0].value >= 0.5);
}

#[test]
fn role_bounds_reports_extent_violations() {
    let (recipe, artifact) =
        compile_fixture([PartSpec::new(1, "deck", [2.0, 0.1, 1.0], [0.0, 0.0, 0.0])]);
    let constraint = constraint("deck_bounds", ["deck"], FamilyRuleExecutionPolicy::Required);
    let rows = evaluate_geometric_constraints(
        &[constraint],
        &bindings([(
            "deck_bounds",
            ExplicitConstraintBinding::RoleBounds(RoleBounds {
                minimum_extent: None,
                maximum_extent: Some([3.0, 1.0, 3.0]),
            }),
        )]),
        &recipe,
        Some(&artifact),
    );

    assert_eq!(rows[0].status, ConformanceStatus::Failed);
    assert!(
        rows[0]
            .issue_codes
            .contains(&"role_bounds_exceeds_maximum".to_owned())
    );
}

#[test]
fn socket_connection_requires_artifact_for_required_checks() {
    let (recipe, _artifact) = socket_fixture(false);
    let constraint = constraint(
        "socket_connection",
        ["parent", "child"],
        FamilyRuleExecutionPolicy::Required,
    );
    let rows = evaluate_geometric_constraints(
        &[constraint],
        &bindings([(
            "socket_connection",
            ExplicitConstraintBinding::SocketConnection(SocketConnection {
                parent_socket: None,
                child_socket: None,
                max_origin_distance: 0.01,
                max_axis_angle_degrees: 1.0,
                max_clearance: None,
            }),
        )]),
        &recipe,
        None,
    );

    assert_eq!(rows[0].status, ConformanceStatus::Missing);
    assert!(
        rows[0]
            .issue_codes
            .contains(&"socket_connection_missing_artifact".to_owned())
    );
}

#[test]
fn socket_connection_checks_the_attached_socket_pair() {
    let (recipe, mut artifact) = socket_fixture(true);
    artifact
        .compiled_parts
        .iter_mut()
        .find(|part| part.instance_id == PartInstanceId(2))
        .expect("compiled child part must exist")
        .sockets_world
        .remove(&SocketId(20));
    let constraint = constraint(
        "socket_connection",
        ["parent", "child"],
        FamilyRuleExecutionPolicy::Required,
    );
    let rows = evaluate_geometric_constraints(
        &[constraint],
        &bindings([(
            "socket_connection",
            ExplicitConstraintBinding::SocketConnection(SocketConnection {
                parent_socket: None,
                child_socket: None,
                max_origin_distance: 0.01,
                max_axis_angle_degrees: 1.0,
                max_clearance: None,
            }),
        )]),
        &recipe,
        Some(&artifact),
    );

    assert_eq!(rows[0].status, ConformanceStatus::Failed);
    assert!(
        rows[0]
            .issue_codes
            .contains(&"socket_connection_missing".to_owned())
    );
}

#[test]
fn artifact_triangle_budget_reports_over_budget() {
    let (recipe, artifact) =
        compile_fixture([PartSpec::new(1, "deck", [1.0, 0.1, 1.0], [0.0, 0.0, 0.0])]);
    let constraint = GeometricConstraint {
        id: "triangle_budget".to_owned(),
        roles: Vec::new(),
        kind: ConstraintKind::Custom("artifact_budget".to_owned()),
        execution_policy: FamilyRuleExecutionPolicy::Required,
    };
    let rows = evaluate_geometric_constraints(
        &[constraint],
        &bindings([(
            "triangle_budget",
            ExplicitConstraintBinding::ArtifactTriangleBudget(ArtifactTriangleBudget {
                maximum_triangles: 1,
            }),
        )]),
        &recipe,
        Some(&artifact),
    );

    assert_eq!(rows[0].status, ConformanceStatus::Failed);
    assert!(
        rows[0]
            .issue_codes
            .contains(&"triangle_budget_exceeded".to_owned())
    );
}

#[test]
fn runtime_metadata_requirement_can_be_adapter_deferred() {
    let (recipe, artifact) =
        compile_fixture([PartSpec::new(1, "deck", [1.0, 0.1, 1.0], [0.0, 0.0, 0.0])]);
    let rows = evaluate_export_requirements(
        &[ExportRequirement {
            profile: "game-runtime".to_owned(),
            required_metadata: vec![RuntimeMetadataRequirement::CollisionProxies],
            triangle_budget_hint: None,
        }],
        &recipe,
        Some(&artifact),
    );

    assert_eq!(rows[0].status, ConformanceStatus::Passed);
    assert_eq!(
        rows[0].metadata[0].availability,
        ExportMetadataAvailability::AdapterDeferred
    );
}

#[test]
fn missing_required_metadata_rejects_export_requirement() {
    let (recipe, artifact) =
        compile_fixture([PartSpec::new(1, "deck", [1.0, 0.1, 1.0], [0.0, 0.0, 0.0])]);
    let rows = evaluate_export_requirements(
        &[ExportRequirement {
            profile: "game-runtime".to_owned(),
            required_metadata: vec![RuntimeMetadataRequirement::Custom(
                "gameplay_tags".to_owned(),
            )],
            triangle_budget_hint: None,
        }],
        &recipe,
        Some(&artifact),
    );

    assert_eq!(rows[0].status, ConformanceStatus::Missing);
    assert_eq!(
        rows[0].metadata[0].availability,
        ExportMetadataAvailability::Missing
    );
}

#[test]
fn advisory_constraint_reports_without_rejecting() {
    let (recipe, artifact) = compile_fixture([
        PartSpec::new(1, "support", [0.35, 0.5, 0.35], [0.0, 0.5, 0.0]),
        PartSpec::new(2, "support", [0.35, 0.5, 0.35], [0.2, 0.5, 0.0]),
    ]);
    let constraint = constraint(
        "support_clearance",
        ["support"],
        FamilyRuleExecutionPolicy::Advisory,
    );
    let rows = evaluate_geometric_constraints(
        &[constraint],
        &bindings([(
            "support_clearance",
            ExplicitConstraintBinding::RoleClearance(RoleClearance { minimum: 0.05 }),
        )]),
        &recipe,
        Some(&artifact),
    );
    let report = FamilyConformanceReport {
        constraints: rows,
        ..FamilyConformanceReport::default()
    };

    assert_eq!(report.constraints[0].status, ConformanceStatus::Failed);
    assert!(report.is_accepted());
}

#[test]
fn required_unbound_constraint_is_unsupported_for_implementation_coverage() {
    let constraint = constraint(
        "future_constraint",
        ["deck"],
        FamilyRuleExecutionPolicy::Required,
    );
    let rows = validate_constraint_binding_coverage(&[constraint], &ConstraintBindingMap::new());

    assert_eq!(rows[0].status, ConformanceStatus::Unsupported);
    assert!(
        rows[0]
            .issue_codes
            .contains(&"required_constraint_binding_missing".to_owned())
    );
}

fn constraint<const N: usize>(
    id: &str,
    roles: [&str; N],
    policy: FamilyRuleExecutionPolicy,
) -> GeometricConstraint {
    GeometricConstraint {
        id: id.to_owned(),
        roles: roles.into_iter().map(str::to_owned).collect(),
        kind: ConstraintKind::Custom("test_constraint_kind".to_owned()),
        execution_policy: policy,
    }
}

fn bindings<const N: usize>(
    bindings: [(&str, ExplicitConstraintBinding); N],
) -> ConstraintBindingMap {
    bindings
        .into_iter()
        .map(|(id, binding)| (id.to_owned(), binding))
        .collect()
}

fn compile_fixture<const N: usize>(parts: [PartSpec; N]) -> (AssetRecipe, AssetArtifact) {
    let mut recipe = AssetRecipe::new(AssetId(1), "Conformance fixture");
    for part in parts {
        let definition = PartDefinition {
            id: PartDefinitionId(part.id),
            name: format!("{} definition", part.role),
            tags: role_tags(part.role),
            geometry: GeometryRecipe {
                source: GeometrySource::RoundedBox {
                    half_extents: part.half_extents,
                    radius: 0.0,
                },
                operations: Vec::new(),
            },
            regions: BTreeMap::new(),
            sockets: BTreeMap::new(),
            local_pivot: Frame3::default(),
            variant_group: None,
            production_hints: None,
        };
        let instance = PartInstance {
            id: PartInstanceId(part.id),
            definition: definition.id,
            name: format!("{} instance", part.role),
            parent: None,
            local_transform: Transform3 {
                translation: part.translation,
                ..Transform3::default()
            },
            attachment: None,
            enabled: true,
            tags: role_tags(part.role),
            generated_by: None,
        };
        recipe.root_instances.push(instance.id);
        recipe.instances.insert(instance.id, instance);
        recipe.definitions.insert(definition.id, definition);
    }
    recipe.next_ids.part_definition = N as u64 + 1;
    recipe.next_ids.part_instance = N as u64 + 1;
    recipe.next_ids.operation = 1;
    recipe.next_ids.region = 1;
    recipe.next_ids.socket = 1;
    let artifact = compile_asset(&recipe).expect("fixture should compile");
    (recipe, artifact)
}

fn socket_fixture(with_unused_aligned_pair: bool) -> (AssetRecipe, AssetArtifact) {
    let mut recipe = AssetRecipe::new(AssetId(2), "Socket conformance fixture");
    let parent_definition = PartDefinition {
        id: PartDefinitionId(1),
        name: "parent definition".to_owned(),
        tags: role_tags("parent"),
        geometry: GeometryRecipe {
            source: GeometrySource::RoundedBox {
                half_extents: [0.5, 0.5, 0.5],
                radius: 0.0,
            },
            operations: Vec::new(),
        },
        regions: BTreeMap::new(),
        sockets: BTreeMap::from([
            (SocketId(10), socket(SocketId(10), [0.0, 0.0, 0.0])),
            (SocketId(11), socket(SocketId(11), [2.0, 0.0, 0.0])),
        ]),
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
    };
    let attached_child_socket_origin = if with_unused_aligned_pair {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 0.0, 0.0]
    };
    let child_definition = PartDefinition {
        id: PartDefinitionId(2),
        name: "child definition".to_owned(),
        tags: role_tags("child"),
        geometry: GeometryRecipe {
            source: GeometrySource::RoundedBox {
                half_extents: [0.25, 0.25, 0.25],
                radius: 0.0,
            },
            operations: Vec::new(),
        },
        regions: BTreeMap::new(),
        sockets: BTreeMap::from([
            (
                SocketId(20),
                socket(SocketId(20), attached_child_socket_origin),
            ),
            (SocketId(21), socket(SocketId(21), [1.0, 0.0, 0.0])),
        ]),
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
    };
    recipe
        .definitions
        .insert(parent_definition.id, parent_definition);
    recipe
        .definitions
        .insert(child_definition.id, child_definition);
    recipe.instances.insert(
        PartInstanceId(1),
        PartInstance {
            id: PartInstanceId(1),
            definition: PartDefinitionId(1),
            name: "parent instance".to_owned(),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: role_tags("parent"),
            generated_by: None,
        },
    );
    recipe.instances.insert(
        PartInstanceId(2),
        PartInstance {
            id: PartInstanceId(2),
            definition: PartDefinitionId(2),
            name: "child instance".to_owned(),
            parent: Some(PartInstanceId(1)),
            local_transform: Transform3::default(),
            attachment: Some(AttachmentSpec {
                parent_instance: PartInstanceId(1),
                parent_socket: SocketId(10),
                child_socket: SocketId(20),
                local_offset: Transform3::default(),
                mode: AttachmentMode::RigidSeparate,
            }),
            enabled: true,
            tags: role_tags("child"),
            generated_by: None,
        },
    );
    recipe.root_instances.push(PartInstanceId(1));
    recipe.next_ids.part_definition = 3;
    recipe.next_ids.part_instance = 3;
    recipe.next_ids.operation = 1;
    recipe.next_ids.region = 1;
    recipe.next_ids.socket = 22;
    let artifact = compile_asset(&recipe).expect("socket fixture should compile");
    (recipe, artifact)
}

fn socket(id: SocketId, origin: [f32; 3]) -> SocketSpec {
    SocketSpec {
        id,
        name: format!("socket {}", id.0),
        local_frame: Frame3 {
            origin,
            ..Frame3::default()
        },
        role: "attachment".to_owned(),
        tags: BTreeSet::from(["attachment".to_owned()]),
    }
}

fn role_tags(role: &str) -> BTreeSet<String> {
    BTreeSet::from([role.to_owned(), format!("role:{role}")])
}

#[derive(Debug, Copy, Clone)]
struct PartSpec {
    id: u64,
    role: &'static str,
    half_extents: [f32; 3],
    translation: [f32; 3],
}

impl PartSpec {
    fn new(id: u64, role: &'static str, half_extents: [f32; 3], translation: [f32; 3]) -> Self {
        Self {
            id,
            role,
            half_extents,
            translation,
        }
    }
}
