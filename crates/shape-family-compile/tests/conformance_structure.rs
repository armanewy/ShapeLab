use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetId, AssetRecipe, AttachmentMode, AttachmentSpec, Frame3, GeometryRecipe, GeometrySource,
    ModelingOperationSpec, OperationId, PartDefinition, PartDefinitionId, PartInstance,
    PartInstanceId, SocketId, SocketSpec, Transform3,
};
use shape_family::{
    ASSET_FAMILY_SCHEMA_VERSION, AllowedOperationKind, AssetFamilySchema, AttachmentRule,
    FamilyRuleExecutionPolicy, PartRole, RoleMultiplicity, RoleProvision,
};
use shape_family_compile::conformance::{
    ConformanceStatus, FamilyConformanceReport, evaluate_attachment_conformance,
    evaluate_operation_conformance, evaluate_role_conformance,
};
use shape_family_compile::remap::{FragmentRemap, ports::SelectedFragmentPorts};
use shape_family_compile::{
    FAMILY_IMPLEMENTATION_SCHEMA_VERSION, FamilyImplementation, FragmentAttachmentBinding,
    FragmentAttachmentPairing, FragmentSocketPort, RECIPE_FRAGMENT_SCHEMA_VERSION, RecipeFragment,
    RecipeFragmentExports, RigidOffset,
};

#[test]
fn valid_box_structural_conformance_passes() {
    let fixture = box_fixture(1, 1, &["load"], &["load"], &["load"], &[(0, 0)]);
    let report = structural_report(&fixture);

    assert!(report.is_accepted());
    assert!(
        report
            .roles
            .iter()
            .all(|row| row.status == ConformanceStatus::Passed)
    );
    assert!(
        report
            .operations
            .iter()
            .all(|row| row.status == ConformanceStatus::Passed)
    );
    let attachment = &report.attachments[0];
    assert_eq!(attachment.status, ConformanceStatus::Passed);
    assert_eq!(attachment.pairs.len(), 1);
    assert!(attachment.pairs[0].socket_compatible);
    assert!(attachment.pairs[0].connected);
}

#[test]
fn missing_support_attachment_is_required_failure() {
    let fixture = box_fixture(1, 1, &["load"], &["load"], &["load"], &[]);
    let report = structural_report(&fixture);
    let attachment = &report.attachments[0];

    assert!(!report.is_accepted());
    assert_eq!(attachment.status, ConformanceStatus::Missing);
    assert!(has_code(
        &attachment.issue_codes,
        "missing_required_attachment"
    ));
    assert!(has_code(
        &attachment.issue_codes,
        "disconnected_required_role"
    ));
    assert_eq!(attachment.pairs.len(), 1);
    assert!(!attachment.pairs[0].connected);
}

#[test]
fn forbidden_operation_class_is_reported() {
    let mut fixture = box_fixture(1, 1, &["load"], &["load"], &["load"], &[(0, 0)]);
    fixture
        .recipe
        .definitions
        .get_mut(&SUPPORT_DEFINITION)
        .expect("support definition must exist")
        .geometry
        .operations
        .push(ModelingOperationSpec::LinearArray {
            operation: OperationId(9),
            count: 2,
            offset: [1.0, 0.0, 0.0],
        });

    let report = structural_report(&fixture);
    let array_row = report
        .operations
        .iter()
        .find(|row| row.operation == AllowedOperationKind::Array)
        .expect("array row must be present");

    assert!(!report.is_accepted());
    assert_eq!(array_row.actual_count, 1);
    assert!(!array_row.allowed);
    assert_eq!(array_row.status, ConformanceStatus::Failed);
    assert!(has_code(&array_row.issue_codes, "forbidden_operation"));
}

#[test]
fn disabled_required_role_is_missing() {
    let mut fixture = box_fixture(1, 1, &["load"], &["load"], &["load"], &[(0, 0)]);
    fixture
        .recipe
        .instances
        .get_mut(&SUPPORT_INSTANCE_START)
        .expect("support instance must exist")
        .enabled = false;

    let report = structural_report(&fixture);
    let support = report
        .roles
        .iter()
        .find(|row| row.role == "support")
        .expect("support row must exist");

    assert!(!report.is_accepted());
    assert_eq!(support.actual_occurrences, 1);
    assert!(!support.effective_enabled);
    assert_eq!(support.status, ConformanceStatus::Missing);
    assert!(has_code(&support.issue_codes, "required_role_disabled"));
}

#[test]
fn incomplete_repeated_pairing_reports_unmatched_coverage() {
    let fixture = box_fixture(2, 1, &["load"], &["load"], &["load"], &[(0, 0)]);
    let report = structural_report(&fixture);
    let attachment = &report.attachments[0];

    assert!(!report.is_accepted());
    assert_eq!(attachment.status, ConformanceStatus::Failed);
    assert!(has_code(
        &attachment.issue_codes,
        "incomplete_attachment_pairing"
    ));
    assert!(has_code(
        &attachment.issue_codes,
        "disconnected_required_role"
    ));
    assert!(attachment.coverage.produced_pairs);
    assert_eq!(
        attachment.coverage.unmatched_first,
        vec![endpoint(SUPPORT_INSTANCE_START.0 + 1, SUPPORT_SOCKET.0)]
    );
    assert!(attachment.coverage.unmatched_second.is_empty());
}

#[test]
fn compatible_and_incompatible_sockets_are_distinguished() {
    let compatible = box_fixture(1, 1, &["load"], &["load"], &["load"], &[(0, 0)]);
    let compatible_report = structural_report(&compatible);
    assert!(compatible_report.attachments[0].pairs[0].socket_compatible);
    assert!(compatible_report.is_accepted());

    let incompatible = box_fixture(1, 1, &["pin"], &["slot"], &[], &[(0, 0)]);
    let incompatible_report = structural_report(&incompatible);
    let attachment = &incompatible_report.attachments[0];

    assert!(!incompatible_report.is_accepted());
    assert_eq!(attachment.status, ConformanceStatus::Failed);
    assert!(!attachment.pairs[0].socket_compatible);
    assert!(attachment.pairs[0].connected);
    assert!(has_code(
        &attachment.issue_codes,
        "incompatible_attachment_socket"
    ));
}

#[test]
fn attachment_compatibility_requires_fragment_port_and_concrete_socket_tags() {
    let mut socket_mismatch = box_fixture(1, 1, &["load"], &["load"], &["load"], &[(0, 0)]);
    for definition in socket_mismatch.recipe.definitions.values_mut() {
        for socket in definition.sockets.values_mut() {
            socket.tags = BTreeSet::from(["private".to_owned()]);
        }
    }
    let socket_report = structural_report(&socket_mismatch);
    let socket_attachment = &socket_report.attachments[0];
    assert!(!socket_report.is_accepted());
    assert_eq!(socket_attachment.status, ConformanceStatus::Failed);
    assert!(!socket_attachment.pairs[0].socket_compatible);
    assert!(has_code(
        &socket_attachment.issue_codes,
        "incompatible_attachment_socket"
    ));

    let mut port_mismatch = box_fixture(1, 1, &["load"], &["load"], &["load"], &[(0, 0)]);
    for port in &mut port_mismatch.span.fragment.exports.socket_ports {
        port.compatibility_tags = vec!["private".to_owned()];
    }
    for port in &mut port_mismatch.support.fragment.exports.socket_ports {
        port.compatibility_tags = vec!["private".to_owned()];
    }
    let port_report = structural_report(&port_mismatch);
    let port_attachment = &port_report.attachments[0];
    assert!(!port_report.is_accepted());
    assert_eq!(port_attachment.status, ConformanceStatus::Failed);
    assert!(!port_attachment.pairs[0].socket_compatible);
    assert!(has_code(
        &port_attachment.issue_codes,
        "incompatible_attachment_socket"
    ));
}

#[test]
fn runtime_only_attachment_rules_are_deferred() {
    let mut fixture = box_fixture(1, 1, &["load"], &["load"], &["load"], &[]);
    fixture.family.attachment_rules[0].execution_policy = FamilyRuleExecutionPolicy::RuntimeOnly;
    fixture.family.attachment_rules[0].required = false;

    let report = structural_report(&fixture);
    let attachment = &report.attachments[0];

    assert!(report.is_accepted());
    assert_eq!(attachment.status, ConformanceStatus::Deferred);
    assert!(attachment.pairs.is_empty());
    assert!(has_code(
        &attachment.issue_codes,
        "runtime_only_attachment_rule_deferred"
    ));
}

#[test]
fn ranged_role_multiplicity_is_enforced() {
    let mut recipe = AssetRecipe::new(AssetId(70), "range");
    recipe.definitions.insert(
        PartDefinitionId(70),
        definition(PartDefinitionId(70), SocketId(70), &["range"]),
    );
    let instances = [
        PartInstanceId(701),
        PartInstanceId(702),
        PartInstanceId(703),
    ];
    for instance_id in instances {
        recipe.instances.insert(
            instance_id,
            instance(instance_id, PartDefinitionId(70), None, [0.0, 0.0, 0.0]),
        );
        recipe.root_instances.push(instance_id);
    }
    let fragment = fragment_fixture(
        "pier",
        "pier_fragment",
        &instances,
        PartDefinitionId(70),
        SocketId(70),
        "pier_port",
        &["range"],
    );
    let family = AssetFamilySchema {
        schema_version: ASSET_FAMILY_SCHEMA_VERSION,
        id: "range_family".to_owned(),
        display_name: "Range Family".to_owned(),
        summary: "Range multiplicity test.".to_owned(),
        part_roles: vec![role(
            "pier",
            RoleMultiplicity::Range { min: 1, max: 2 },
            true,
        )],
        attachment_rules: Vec::new(),
        allowed_operations: vec![AllowedOperationKind::Primitive],
        parameter_slots: Vec::new(),
        constraints: Vec::new(),
        variant_rules: Vec::new(),
        export_requirements: Vec::new(),
        compatible_style_kits: Vec::new(),
        tags: Vec::new(),
    };
    let selected = vec![SelectedFragmentPorts {
        role: &fragment.role,
        fragment: &fragment.fragment,
        remap: &fragment.remap,
    }];

    let rows = evaluate_role_conformance(&family, &recipe, &selected);

    assert_eq!(rows[0].expected.min, 1);
    assert_eq!(rows[0].expected.max, Some(2));
    assert_eq!(rows[0].actual_occurrences, 3);
    assert_eq!(rows[0].status, ConformanceStatus::Failed);
    assert!(has_code(&rows[0].issue_codes, "role_multiplicity_overflow"));
}

#[test]
fn structural_report_order_is_deterministic() {
    let mut fixture = box_fixture(1, 1, &["load"], &["load"], &["load"], &[(0, 0)]);
    fixture.family.part_roles = vec![
        role("support", RoleMultiplicity::Repeated, true),
        role("deck", RoleMultiplicity::Optional, false),
        role("span", RoleMultiplicity::Single, true),
    ];
    fixture.family.attachment_rules = vec![
        rule(
            "z_rule",
            "support",
            "span",
            &["load"],
            FamilyRuleExecutionPolicy::Required,
        ),
        rule(
            "a_rule",
            "support",
            "span",
            &["load"],
            FamilyRuleExecutionPolicy::Required,
        ),
    ];
    fixture.family.allowed_operations =
        vec![AllowedOperationKind::Bevel, AllowedOperationKind::Primitive];
    fixture.family_impl.attachment_bindings = vec![
        binding("z_rule", FragmentAttachmentPairing::ByOccurrenceIndex),
        binding("a_rule", FragmentAttachmentPairing::ByOccurrenceIndex),
    ];

    let report = structural_report(&fixture);

    assert_eq!(
        report
            .roles
            .iter()
            .map(|row| row.role.as_str())
            .collect::<Vec<_>>(),
        vec!["deck", "span", "support"]
    );
    assert_eq!(
        report
            .attachments
            .iter()
            .map(|row| row.rule_id.as_str())
            .collect::<Vec<_>>(),
        vec!["a_rule", "z_rule"]
    );
    assert_eq!(
        report
            .operations
            .iter()
            .map(|row| row.operation.clone())
            .collect::<Vec<_>>(),
        vec![AllowedOperationKind::Primitive, AllowedOperationKind::Bevel]
    );
}

const SPAN_DEFINITION: PartDefinitionId = PartDefinitionId(10);
const SUPPORT_DEFINITION: PartDefinitionId = PartDefinitionId(20);
const SPAN_SOCKET: SocketId = SocketId(11);
const SUPPORT_SOCKET: SocketId = SocketId(21);
const SPAN_INSTANCE_START: PartInstanceId = PartInstanceId(100);
const SUPPORT_INSTANCE_START: PartInstanceId = PartInstanceId(200);

struct BridgeFixture {
    family: AssetFamilySchema,
    family_impl: FamilyImplementation,
    recipe: AssetRecipe,
    span: FragmentFixture,
    support: FragmentFixture,
}

impl BridgeFixture {
    fn selected(&self) -> Vec<SelectedFragmentPorts<'_>> {
        vec![
            SelectedFragmentPorts {
                role: &self.span.role,
                fragment: &self.span.fragment,
                remap: &self.span.remap,
            },
            SelectedFragmentPorts {
                role: &self.support.role,
                fragment: &self.support.fragment,
                remap: &self.support.remap,
            },
        ]
    }
}

struct FragmentFixture {
    role: String,
    fragment: RecipeFragment,
    remap: FragmentRemap,
}

fn box_fixture(
    support_count: u64,
    span_count: u64,
    support_tags: &[&str],
    span_tags: &[&str],
    rule_tags: &[&str],
    connected_pairs: &[(usize, usize)],
) -> BridgeFixture {
    let mut recipe = AssetRecipe::new(AssetId(7), "box_primitive");
    recipe.definitions.insert(
        SPAN_DEFINITION,
        definition(SPAN_DEFINITION, SPAN_SOCKET, span_tags),
    );
    recipe.definitions.insert(
        SUPPORT_DEFINITION,
        definition(SUPPORT_DEFINITION, SUPPORT_SOCKET, support_tags),
    );

    let span_instances = (0..span_count)
        .map(|index| PartInstanceId(SPAN_INSTANCE_START.0 + index))
        .collect::<Vec<_>>();
    let support_instances = (0..support_count)
        .map(|index| PartInstanceId(SUPPORT_INSTANCE_START.0 + index))
        .collect::<Vec<_>>();
    for (index, instance_id) in span_instances.iter().copied().enumerate() {
        recipe.instances.insert(
            instance_id,
            instance(
                instance_id,
                SPAN_DEFINITION,
                None,
                [index as f32 * 2.0, 0.0, 0.0],
            ),
        );
        recipe.root_instances.push(instance_id);
    }
    for (index, instance_id) in support_instances.iter().copied().enumerate() {
        recipe.instances.insert(
            instance_id,
            instance(
                instance_id,
                SUPPORT_DEFINITION,
                None,
                [index as f32 * 2.0, -1.0, 0.0],
            ),
        );
        recipe.root_instances.push(instance_id);
    }
    for (support_ordinal, span_ordinal) in connected_pairs {
        attach(
            &mut recipe,
            support_instances[*support_ordinal],
            span_instances[*span_ordinal],
            SUPPORT_SOCKET,
            SPAN_SOCKET,
        );
    }

    let family = AssetFamilySchema {
        schema_version: ASSET_FAMILY_SCHEMA_VERSION,
        id: "box_primitive".to_owned(),
        display_name: "Box Primitive".to_owned(),
        summary: "Structural box_primitive conformance.".to_owned(),
        part_roles: vec![
            role("span", RoleMultiplicity::Single, true),
            role("support", RoleMultiplicity::Repeated, true),
        ],
        attachment_rules: vec![rule(
            "support_to_span",
            "support",
            "span",
            rule_tags,
            FamilyRuleExecutionPolicy::Required,
        )],
        allowed_operations: vec![AllowedOperationKind::Primitive],
        parameter_slots: Vec::new(),
        constraints: Vec::new(),
        variant_rules: Vec::new(),
        export_requirements: Vec::new(),
        compatible_style_kits: Vec::new(),
        tags: Vec::new(),
    };
    let family_impl = implementation(vec![binding(
        "support_to_span",
        FragmentAttachmentPairing::ByOccurrenceIndex,
    )]);
    let span = fragment_fixture(
        "span",
        "span_fragment",
        &span_instances,
        SPAN_DEFINITION,
        SPAN_SOCKET,
        "span_port",
        span_tags,
    );
    let support = fragment_fixture(
        "support",
        "support_fragment",
        &support_instances,
        SUPPORT_DEFINITION,
        SUPPORT_SOCKET,
        "support_port",
        support_tags,
    );

    BridgeFixture {
        family,
        family_impl,
        recipe,
        span,
        support,
    }
}

fn structural_report(fixture: &BridgeFixture) -> FamilyConformanceReport {
    let selected = fixture.selected();
    FamilyConformanceReport {
        family_id: fixture.family.id.clone(),
        style_kit_id: "test_style".to_owned(),
        roles: evaluate_role_conformance(&fixture.family, &fixture.recipe, &selected),
        attachments: evaluate_attachment_conformance(
            &fixture.family,
            &fixture.family_impl,
            &fixture.recipe,
            &selected,
        ),
        operations: evaluate_operation_conformance(&fixture.family, &fixture.recipe),
        ..FamilyConformanceReport::default()
    }
}

fn implementation(bindings: Vec<FragmentAttachmentBinding>) -> FamilyImplementation {
    FamilyImplementation {
        schema_version: FAMILY_IMPLEMENTATION_SCHEMA_VERSION,
        family_id: "box_primitive".to_owned(),
        base_recipe: AssetRecipe::new(AssetId(1), "base"),
        parameter_bindings: Vec::new(),
        default_role_providers: BTreeMap::new(),
        fragments: BTreeMap::new(),
        attachment_bindings: bindings,
    }
}

fn binding(rule_id: &str, pairing: FragmentAttachmentPairing) -> FragmentAttachmentBinding {
    FragmentAttachmentBinding {
        family_attachment_rule: rule_id.to_owned(),
        parent_role: "span".to_owned(),
        parent_port: "span_port".to_owned(),
        child_role: "support".to_owned(),
        child_port: "support_port".to_owned(),
        pairing,
        rigid_offset: RigidOffset::default(),
        attachment_mode: AttachmentMode::RigidSeparate,
    }
}

fn fragment_fixture(
    role: &str,
    fragment_id: &str,
    concrete_instances: &[PartInstanceId],
    concrete_definition: PartDefinitionId,
    concrete_socket: SocketId,
    port_id: &str,
    tags: &[&str],
) -> FragmentFixture {
    let local_definition = PartDefinitionId(1);
    let local_socket = SocketId(1);
    let mut local_recipe = AssetRecipe::new(AssetId(concrete_definition.0), fragment_id);
    local_recipe.definitions.insert(
        local_definition,
        definition(local_definition, local_socket, tags),
    );
    let mut local_roots = Vec::new();
    let mut remap = FragmentRemap::default();
    remap
        .definitions
        .insert(local_definition, concrete_definition);
    remap.sockets.insert(local_socket, concrete_socket);
    for (index, concrete_instance) in concrete_instances.iter().copied().enumerate() {
        let local_instance = PartInstanceId(index as u64 + 1);
        local_recipe.instances.insert(
            local_instance,
            instance(
                local_instance,
                local_definition,
                None,
                [index as f32, 0.0, 0.0],
            ),
        );
        local_recipe.root_instances.push(local_instance);
        local_roots.push(local_instance);
        remap.instances.insert(local_instance, concrete_instance);
    }

    FragmentFixture {
        role: role.to_owned(),
        fragment: RecipeFragment {
            schema_version: RECIPE_FRAGMENT_SCHEMA_VERSION,
            id: fragment_id.to_owned(),
            provided_role: role.to_owned(),
            exports: RecipeFragmentExports {
                role_occurrence_roots: local_roots.clone(),
                internal_roots: Vec::new(),
                socket_ports: vec![FragmentSocketPort {
                    id: port_id.to_owned(),
                    local_occurrence_root: local_roots.first().copied().unwrap_or_default(),
                    local_socket,
                    compatibility_tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
                }],
                surface_ports: Vec::new(),
            },
            recipe: local_recipe,
        },
        remap,
    }
}

fn role(id: &str, multiplicity: RoleMultiplicity, required: bool) -> PartRole {
    PartRole {
        id: id.to_owned(),
        display_name: id.to_owned(),
        required,
        multiplicity,
        provision: RoleProvision::FamilyDefault,
        semantic_tags: Vec::new(),
    }
}

fn rule(
    id: &str,
    from_role: &str,
    to_role: &str,
    tags: &[&str],
    execution_policy: FamilyRuleExecutionPolicy,
) -> AttachmentRule {
    AttachmentRule {
        id: id.to_owned(),
        from_role: from_role.to_owned(),
        to_role: to_role.to_owned(),
        anchor_role: None,
        compatibility_tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
        required: execution_policy == FamilyRuleExecutionPolicy::Required,
        execution_policy,
    }
}

fn definition(id: PartDefinitionId, socket: SocketId, tags: &[&str]) -> PartDefinition {
    PartDefinition {
        id,
        name: format!("definition_{}", id.0),
        tags: BTreeSet::new(),
        geometry: GeometryRecipe {
            source: GeometrySource::RoundedBox {
                half_extents: [0.5, 0.5, 0.5],
                radius: 0.02,
            },
            operations: Vec::new(),
        },
        regions: BTreeMap::new(),
        sockets: BTreeMap::from([(
            socket,
            SocketSpec {
                id: socket,
                name: format!("socket_{}", socket.0),
                local_frame: Frame3::default(),
                role: "attachment".to_owned(),
                tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            },
        )]),
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
    }
}

fn instance(
    id: PartInstanceId,
    definition: PartDefinitionId,
    parent: Option<PartInstanceId>,
    translation: [f32; 3],
) -> PartInstance {
    PartInstance {
        id,
        definition,
        name: format!("instance_{}", id.0),
        parent,
        local_transform: Transform3 {
            translation,
            ..Transform3::default()
        },
        attachment: None,
        enabled: true,
        tags: BTreeSet::new(),
        generated_by: None,
    }
}

fn attach(
    recipe: &mut AssetRecipe,
    child: PartInstanceId,
    parent: PartInstanceId,
    child_socket: SocketId,
    parent_socket: SocketId,
) {
    let child_instance = recipe
        .instances
        .get_mut(&child)
        .expect("child instance must exist");
    child_instance.parent = Some(parent);
    child_instance.attachment = Some(AttachmentSpec {
        parent_instance: parent,
        parent_socket,
        child_socket,
        local_offset: Transform3::default(),
        mode: AttachmentMode::RigidSeparate,
    });
}

fn endpoint(
    instance: u64,
    socket: u64,
) -> shape_family_compile::conformance::AttachmentEndpointConformance {
    shape_family_compile::conformance::AttachmentEndpointConformance {
        instance: PartInstanceId(instance),
        socket: SocketId(socket),
    }
}

fn has_code(codes: &[String], expected: &str) -> bool {
    codes.iter().any(|code| code == expected)
}
