use std::collections::{BTreeMap, BTreeSet};

use glam::{EulerRot, Quat};
use shape_asset::{
    AssetId, AssetRecipe, AttachmentMode, Frame3, GeometryRecipe, GeometrySource, OperationId,
    PartDefinition, PartDefinitionId, PartInstance, PartInstanceId, SocketId, SocketSpec,
    Transform3, validate_asset_recipe,
};
use shape_family::{
    ASSET_FAMILY_SCHEMA_VERSION, AllowedOperationKind, AssetFamilySchema, AttachmentRule,
    FamilyRuleExecutionPolicy, RoleMultiplicity, RoleProvision,
};
use shape_family_compile::remap::{
    FragmentRemap,
    ports::{SelectedFragmentPorts, apply_family_attachment_bindings},
};
use shape_family_compile::{
    FAMILY_IMPLEMENTATION_SCHEMA_VERSION, FamilyImplementation, FragmentAttachmentBinding,
    FragmentAttachmentPairing, FragmentSocketPort, RECIPE_FRAGMENT_SCHEMA_VERSION, RecipeFragment,
    RecipeFragmentExports, RigidOffset,
};

#[test]
fn bridge_support_attachments() {
    let mut recipe = AssetRecipe::new(AssetId(1), "bridge");
    let support = add_fragment(
        &mut recipe,
        FragmentSpec::new("support_fragment", "support", "top", &["load_path"]).with_instances(
            10,
            100,
            1000,
            &[[0.0, 0.0, 0.0]],
        ),
    );
    let span = add_fragment(
        &mut recipe,
        FragmentSpec::new("span_fragment", "span", "underside", &["load_path"]).with_instances(
            20,
            200,
            2000,
            &[[0.0, 1.0, 0.0]],
        ),
    );
    let mut implementation = implementation(vec![binding(
        "support_span",
        "support",
        "top",
        "span",
        "underside",
        FragmentAttachmentPairing::ByOccurrenceIndex,
    )]);
    implementation.attachment_bindings[0]
        .rigid_offset
        .translation = [0.0, 0.25, 0.0];

    let report = apply_family_attachment_bindings(
        &mut recipe,
        &family(vec![rule(
            "support_span",
            "support",
            "span",
            &["load_path"],
        )]),
        &implementation,
        &selected(&support, &span),
    )
    .expect("support attachment should bind");

    assert_eq!(report.attachments.len(), 1);
    assert_eq!(report.attachments[0].child_instance, PartInstanceId(100));
    assert_eq!(report.attachments[0].parent_instance, PartInstanceId(200));
    let support_instance = recipe.instances.get(&PartInstanceId(100)).unwrap();
    let attachment = support_instance.attachment.as_ref().unwrap();
    assert_eq!(support_instance.parent, Some(PartInstanceId(200)));
    assert_eq!(attachment.parent_socket, SocketId(2000));
    assert_eq!(attachment.child_socket, SocketId(1000));
    assert_eq!(attachment.local_offset.translation, [0.0, 0.25, 0.0]);
    assert_eq!(recipe.root_instances, vec![PartInstanceId(200)]);
    assert!(validate_asset_recipe(&recipe).is_valid());
}

#[test]
fn rigid_offset_rotation_replays_with_transform3_xyz_semantics() {
    let mut recipe = AssetRecipe::new(AssetId(1), "rotated");
    let support = add_fragment(
        &mut recipe,
        FragmentSpec::new("support_fragment", "support", "top", &["load_path"]).with_instances(
            10,
            100,
            1000,
            &[[0.0, 0.0, 0.0]],
        ),
    );
    let span = add_fragment(
        &mut recipe,
        FragmentSpec::new("span_fragment", "span", "underside", &["load_path"]).with_instances(
            20,
            200,
            2000,
            &[[0.0, 1.0, 0.0]],
        ),
    );
    let mut implementation = implementation(vec![binding(
        "support_span",
        "support",
        "top",
        "span",
        "underside",
        FragmentAttachmentPairing::ByOccurrenceIndex,
    )]);
    let expected_rotation: [f32; 3] = [30.0, 20.0, 10.0];
    let quaternion = Quat::from_euler(
        EulerRot::XYZ,
        expected_rotation[0].to_radians(),
        expected_rotation[1].to_radians(),
        expected_rotation[2].to_radians(),
    );
    implementation.attachment_bindings[0].rigid_offset.rotation =
        [quaternion.x, quaternion.y, quaternion.z, quaternion.w];

    apply_family_attachment_bindings(
        &mut recipe,
        &family(vec![rule(
            "support_span",
            "support",
            "span",
            &["load_path"],
        )]),
        &implementation,
        &selected(&support, &span),
    )
    .expect("rotated rigid offset should bind");

    let attachment = recipe.instances[&PartInstanceId(100)]
        .attachment
        .as_ref()
        .expect("attachment");
    let expected = Transform3 {
        rotation_degrees: expected_rotation,
        ..Transform3::default()
    }
    .matrix();
    let actual = attachment.local_offset.matrix();
    for (actual, expected) in actual.to_cols_array().iter().zip(expected.to_cols_array()) {
        assert!((actual - expected).abs() < 1.0e-5);
    }
}

#[test]
fn repeated_supports_bind_by_occurrence_index() {
    let mut recipe = AssetRecipe::new(AssetId(1), "indexed");
    let support = add_fragment(
        &mut recipe,
        FragmentSpec::new("support_fragment", "support", "top", &["load_path"]).with_instances(
            10,
            100,
            1000,
            &[[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        ),
    );
    let span = add_fragment(
        &mut recipe,
        FragmentSpec::new("span_fragment", "span", "underside", &["load_path"]).with_instances(
            20,
            200,
            2000,
            &[[0.0, 1.0, 0.0], [2.0, 1.0, 0.0]],
        ),
    );

    let report = apply_family_attachment_bindings(
        &mut recipe,
        &family(vec![rule(
            "support_span",
            "support",
            "span",
            &["load_path"],
        )]),
        &implementation(vec![binding(
            "support_span",
            "support",
            "top",
            "span",
            "underside",
            FragmentAttachmentPairing::ByOccurrenceIndex,
        )]),
        &selected(&support, &span),
    )
    .expect("indexed supports should bind");

    let pairs = attachment_pairs(&report);
    assert_eq!(
        pairs,
        vec![
            (PartInstanceId(100), PartInstanceId(200)),
            (PartInstanceId(101), PartInstanceId(201)),
        ]
    );
}

#[test]
fn all_pairs_bind_multiple_fasteners_to_one_parent() {
    let mut recipe = AssetRecipe::new(AssetId(1), "all pairs");
    let fastener = add_fragment(
        &mut recipe,
        FragmentSpec::new("fastener_fragment", "fastener", "pin", &["mount"]).with_instances(
            10,
            100,
            1000,
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
        ),
    );
    let plate = add_fragment(
        &mut recipe,
        FragmentSpec::new("plate_fragment", "plate", "mount", &["mount"]).with_instances(
            20,
            200,
            2000,
            &[[0.5, 0.0, 0.0]],
        ),
    );

    let report = apply_family_attachment_bindings(
        &mut recipe,
        &family(vec![rule(
            "fastener_plate",
            "fastener",
            "plate",
            &["mount"],
        )]),
        &implementation(vec![binding(
            "fastener_plate",
            "fastener",
            "pin",
            "plate",
            "mount",
            FragmentAttachmentPairing::AllPairs,
        )]),
        &selected(&fastener, &plate),
    )
    .expect("all pairs should bind when each child has one parent");

    assert_eq!(
        attachment_pairs(&report),
        vec![
            (PartInstanceId(100), PartInstanceId(200)),
            (PartInstanceId(101), PartInstanceId(200)),
        ]
    );
}

#[test]
fn nearest_one_to_one_uses_positions_with_stable_tiebreaks() {
    let mut recipe = AssetRecipe::new(AssetId(1), "nearest");
    let support = add_fragment(
        &mut recipe,
        FragmentSpec::new("support_fragment", "support", "top", &["load_path"]).with_instances(
            10,
            100,
            1000,
            &[[10.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        ),
    );
    let span = add_fragment(
        &mut recipe,
        FragmentSpec::new("span_fragment", "span", "underside", &["load_path"]).with_instances(
            20,
            200,
            2000,
            &[[0.2, 0.0, 0.0], [9.8, 0.0, 0.0]],
        ),
    );

    let report = apply_family_attachment_bindings(
        &mut recipe,
        &family(vec![rule(
            "support_span",
            "support",
            "span",
            &["load_path"],
        )]),
        &implementation(vec![binding(
            "support_span",
            "support",
            "top",
            "span",
            "underside",
            FragmentAttachmentPairing::NearestOneToOne,
        )]),
        &selected(&support, &span),
    )
    .expect("nearest supports should bind");

    assert_eq!(
        attachment_pairs(&report),
        vec![
            (PartInstanceId(100), PartInstanceId(201)),
            (PartInstanceId(101), PartInstanceId(200)),
        ]
    );
}

#[test]
fn explicit_ordinal_pairs_bind_requested_occurrences() {
    let mut recipe = AssetRecipe::new(AssetId(1), "explicit");
    let support = add_fragment(
        &mut recipe,
        FragmentSpec::new("support_fragment", "support", "top", &["load_path"]).with_instances(
            10,
            100,
            1000,
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
        ),
    );
    let span = add_fragment(
        &mut recipe,
        FragmentSpec::new("span_fragment", "span", "underside", &["load_path"]).with_instances(
            20,
            200,
            2000,
            &[[1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
        ),
    );

    let report = apply_family_attachment_bindings(
        &mut recipe,
        &family(vec![rule(
            "support_span",
            "support",
            "span",
            &["load_path"],
        )]),
        &implementation(vec![binding(
            "support_span",
            "support",
            "top",
            "span",
            "underside",
            FragmentAttachmentPairing::ExplicitOrdinalPairs(vec![(0, 1), (1, 0)]),
        )]),
        &selected(&support, &span),
    )
    .expect("explicit ordinal pairs should bind");

    assert_eq!(
        attachment_pairs(&report),
        vec![
            (PartInstanceId(100), PartInstanceId(201)),
            (PartInstanceId(101), PartInstanceId(200)),
        ]
    );
}

#[test]
fn missing_port_is_rejected_without_mutating_recipe() {
    let mut recipe = AssetRecipe::new(AssetId(1), "missing port");
    let support = add_fragment(
        &mut recipe,
        FragmentSpec::new("support_fragment", "support", "top", &["load_path"]).with_instances(
            10,
            100,
            1000,
            &[[0.0, 0.0, 0.0]],
        ),
    );
    let span = add_fragment(
        &mut recipe,
        FragmentSpec::new("span_fragment", "span", "underside", &["load_path"]).with_instances(
            20,
            200,
            2000,
            &[[0.0, 1.0, 0.0]],
        ),
    );

    let error = apply_family_attachment_bindings(
        &mut recipe,
        &family(vec![rule(
            "support_span",
            "support",
            "span",
            &["load_path"],
        )]),
        &implementation(vec![binding(
            "support_span",
            "support",
            "missing",
            "span",
            "underside",
            FragmentAttachmentPairing::ByOccurrenceIndex,
        )]),
        &selected(&support, &span),
    )
    .expect_err("missing parent port should fail");

    assert!(issue_codes(&error).contains("unknown_fragment_attachment_port"));
    assert!(
        recipe
            .instances
            .values()
            .all(|instance| instance.attachment.is_none())
    );
}

#[test]
fn tag_mismatch_is_rejected() {
    let mut recipe = AssetRecipe::new(AssetId(1), "tag mismatch");
    let support = add_fragment(
        &mut recipe,
        FragmentSpec::new("support_fragment", "support", "top", &["decor"]).with_instances(
            10,
            100,
            1000,
            &[[0.0, 0.0, 0.0]],
        ),
    );
    let span = add_fragment(
        &mut recipe,
        FragmentSpec::new("span_fragment", "span", "underside", &["load_path"]).with_instances(
            20,
            200,
            2000,
            &[[0.0, 1.0, 0.0]],
        ),
    );

    let error = apply_family_attachment_bindings(
        &mut recipe,
        &family(vec![rule(
            "support_span",
            "support",
            "span",
            &["load_path"],
        )]),
        &implementation(vec![binding(
            "support_span",
            "support",
            "top",
            "span",
            "underside",
            FragmentAttachmentPairing::ByOccurrenceIndex,
        )]),
        &selected(&support, &span),
    )
    .expect_err("tag mismatch should fail");

    assert!(issue_codes(&error).contains("fragment_attachment_tag_mismatch"));
}

#[test]
fn incomplete_pairing_is_rejected() {
    let mut recipe = AssetRecipe::new(AssetId(1), "incomplete");
    let support = add_fragment(
        &mut recipe,
        FragmentSpec::new("support_fragment", "support", "top", &["load_path"]).with_instances(
            10,
            100,
            1000,
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
        ),
    );
    let span = add_fragment(
        &mut recipe,
        FragmentSpec::new("span_fragment", "span", "underside", &["load_path"]).with_instances(
            20,
            200,
            2000,
            &[[0.0, 1.0, 0.0]],
        ),
    );

    let error = apply_family_attachment_bindings(
        &mut recipe,
        &family(vec![rule(
            "support_span",
            "support",
            "span",
            &["load_path"],
        )]),
        &implementation(vec![binding(
            "support_span",
            "support",
            "top",
            "span",
            "underside",
            FragmentAttachmentPairing::ByOccurrenceIndex,
        )]),
        &selected(&support, &span),
    )
    .expect_err("partial index coverage should fail");

    assert!(issue_codes(&error).contains("incomplete_fragment_attachment_pairing"));
}

#[test]
fn duplicate_parent_attachment_is_rejected() {
    let mut recipe = AssetRecipe::new(AssetId(1), "duplicate");
    let support = add_fragment(
        &mut recipe,
        FragmentSpec::new("support_fragment", "support", "top", &["load_path"]).with_instances(
            10,
            100,
            1000,
            &[[0.0, 0.0, 0.0]],
        ),
    );
    let span = add_fragment(
        &mut recipe,
        FragmentSpec::new("span_fragment", "span", "underside", &["load_path"]).with_instances(
            20,
            200,
            2000,
            &[[0.0, 1.0, 0.0], [1.0, 1.0, 0.0]],
        ),
    );

    let error = apply_family_attachment_bindings(
        &mut recipe,
        &family(vec![rule(
            "support_span",
            "support",
            "span",
            &["load_path"],
        )]),
        &implementation(vec![binding(
            "support_span",
            "support",
            "top",
            "span",
            "underside",
            FragmentAttachmentPairing::AllPairs,
        )]),
        &selected(&support, &span),
    )
    .expect_err("one child cannot bind to two parents");

    assert!(issue_codes(&error).contains("fragment_attachment_all_pairs_multiple_parents"));
}

#[test]
fn generated_occurrence_attachment_ports_are_rejected() {
    let mut recipe = AssetRecipe::new(AssetId(1), "generated occurrence");
    let mut support = add_fragment(
        &mut recipe,
        FragmentSpec::new("support_fragment", "support", "top", &["load_path"]).with_instances(
            10,
            100,
            1000,
            &[[0.0, 0.0, 0.0]],
        ),
    );
    support
        .fragment
        .recipe
        .instances
        .get_mut(&PartInstanceId(1))
        .expect("local occurrence")
        .generated_by = Some(OperationId(7));
    let span = add_fragment(
        &mut recipe,
        FragmentSpec::new("span_fragment", "span", "underside", &["load_path"]).with_instances(
            20,
            200,
            2000,
            &[[0.0, 1.0, 0.0]],
        ),
    );

    let error = apply_family_attachment_bindings(
        &mut recipe,
        &family(vec![rule(
            "support_span",
            "support",
            "span",
            &["load_path"],
        )]),
        &implementation(vec![binding(
            "support_span",
            "support",
            "top",
            "span",
            "underside",
            FragmentAttachmentPairing::ByOccurrenceIndex,
        )]),
        &selected(&support, &span),
    )
    .expect_err("generated occurrence ports are unsupported");

    assert!(issue_codes(&error).contains("unsupported_fragment_attachment_generated_occurrence"));
}

#[test]
fn generated_parent_occurrence_attachment_ports_are_rejected() {
    let mut recipe = AssetRecipe::new(AssetId(1), "generated parent occurrence");
    let support = add_fragment(
        &mut recipe,
        FragmentSpec::new("support_fragment", "support", "top", &["load_path"]).with_instances(
            10,
            100,
            1000,
            &[[0.0, 0.0, 0.0]],
        ),
    );
    let mut span = add_fragment(
        &mut recipe,
        FragmentSpec::new("span_fragment", "span", "underside", &["load_path"]).with_instances(
            20,
            200,
            2000,
            &[[0.0, 1.0, 0.0]],
        ),
    );
    span.fragment
        .recipe
        .instances
        .get_mut(&PartInstanceId(1))
        .expect("local occurrence")
        .generated_by = Some(OperationId(7));

    let error = apply_family_attachment_bindings(
        &mut recipe,
        &family(vec![rule(
            "support_span",
            "support",
            "span",
            &["load_path"],
        )]),
        &implementation(vec![binding(
            "support_span",
            "support",
            "top",
            "span",
            "underside",
            FragmentAttachmentPairing::ByOccurrenceIndex,
        )]),
        &selected(&support, &span),
    )
    .expect_err("generated parent occurrence ports are unsupported");

    assert!(issue_codes(&error).contains("unsupported_fragment_attachment_generated_occurrence"));
}

#[test]
fn invalid_rigid_offsets_are_rejected() {
    let mut recipe = AssetRecipe::new(AssetId(1), "invalid rigid offset");
    let support = add_fragment(
        &mut recipe,
        FragmentSpec::new("support_fragment", "support", "top", &["load_path"]).with_instances(
            10,
            100,
            1000,
            &[[0.0, 0.0, 0.0]],
        ),
    );
    let span = add_fragment(
        &mut recipe,
        FragmentSpec::new("span_fragment", "span", "underside", &["load_path"]).with_instances(
            20,
            200,
            2000,
            &[[0.0, 1.0, 0.0]],
        ),
    );
    let mut implementation = implementation(vec![binding(
        "support_span",
        "support",
        "top",
        "span",
        "underside",
        FragmentAttachmentPairing::ByOccurrenceIndex,
    )]);
    implementation.attachment_bindings[0]
        .rigid_offset
        .translation = [0.0, f32::NAN, 0.0];
    implementation.attachment_bindings[0].rigid_offset.rotation = [0.0, 0.0, 0.0, -2.0];

    let error = apply_family_attachment_bindings(
        &mut recipe,
        &family(vec![rule(
            "support_span",
            "support",
            "span",
            &["load_path"],
        )]),
        &implementation,
        &selected(&support, &span),
    )
    .expect_err("invalid rigid offset should fail validation");
    let issue_codes = issue_codes(&error);

    assert!(issue_codes.contains("non_finite_fragment_attachment_translation"));
    assert!(issue_codes.contains("non_unit_fragment_attachment_rotation"));
    assert!(issue_codes.contains("non_canonical_fragment_attachment_rotation"));
    assert!(
        recipe
            .instances
            .values()
            .all(|instance| instance.attachment.is_none())
    );
}

#[test]
fn attachment_cycle_is_rejected() {
    let mut recipe = AssetRecipe::new(AssetId(1), "cycle");
    let support = add_fragment(
        &mut recipe,
        FragmentSpec::new("support_fragment", "support", "top", &["load_path"]).with_instances(
            10,
            100,
            1000,
            &[[0.0, 0.0, 0.0]],
        ),
    );
    let span = add_fragment(
        &mut recipe,
        FragmentSpec::new("span_fragment", "span", "underside", &["load_path"]).with_instances(
            20,
            200,
            2000,
            &[[0.0, 1.0, 0.0]],
        ),
    );

    let error = apply_family_attachment_bindings(
        &mut recipe,
        &family(vec![
            rule("support_span", "support", "span", &["load_path"]),
            rule("span_support", "span", "support", &["load_path"]),
        ]),
        &implementation(vec![
            binding(
                "support_span",
                "support",
                "top",
                "span",
                "underside",
                FragmentAttachmentPairing::ByOccurrenceIndex,
            ),
            binding(
                "span_support",
                "span",
                "underside",
                "support",
                "top",
                FragmentAttachmentPairing::ByOccurrenceIndex,
            ),
        ]),
        &selected(&support, &span),
    )
    .expect_err("cyclic attachment should fail");

    assert!(issue_codes(&error).contains("fragment_attachment_cycle"));
}

#[test]
fn attachment_ordering_is_deterministic() {
    let first = deterministic_ordering_run(false);
    let second = deterministic_ordering_run(true);

    assert_eq!(first, second);
}

fn deterministic_ordering_run(reverse_selected: bool) -> Vec<(PartInstanceId, PartInstanceId)> {
    let mut recipe = AssetRecipe::new(AssetId(1), "deterministic");
    let fastener = add_fragment(
        &mut recipe,
        FragmentSpec::new("fastener_fragment", "fastener", "pin", &["mount"]).with_instances(
            10,
            100,
            1000,
            &[[2.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        ),
    );
    let plate = add_fragment(
        &mut recipe,
        FragmentSpec::new("plate_fragment", "plate", "mount", &["mount"]).with_instances(
            20,
            200,
            2000,
            &[[1.0, 0.0, 0.0]],
        ),
    );
    let selected_fragments = if reverse_selected {
        selected(&plate, &fastener)
    } else {
        selected(&fastener, &plate)
    };
    let report = apply_family_attachment_bindings(
        &mut recipe,
        &family(vec![rule(
            "fastener_plate",
            "fastener",
            "plate",
            &["mount"],
        )]),
        &implementation(vec![binding(
            "fastener_plate",
            "fastener",
            "pin",
            "plate",
            "mount",
            FragmentAttachmentPairing::AllPairs,
        )]),
        &selected_fragments,
    )
    .expect("deterministic binding should succeed");

    attachment_pairs(&report)
}

struct RemappedFragment {
    role: String,
    fragment: RecipeFragment,
    remap: FragmentRemap,
}

struct FragmentSpec<'a> {
    fragment_id: &'a str,
    role: &'a str,
    port_id: &'a str,
    tags: &'a [&'a str],
    definition: u64,
    instance_start: u64,
    socket: u64,
    positions: &'a [[f32; 3]],
}

impl<'a> FragmentSpec<'a> {
    fn new(fragment_id: &'a str, role: &'a str, port_id: &'a str, tags: &'a [&'a str]) -> Self {
        Self {
            fragment_id,
            role,
            port_id,
            tags,
            definition: 1,
            instance_start: 1,
            socket: 1,
            positions: &[],
        }
    }

    fn with_instances(
        mut self,
        definition: u64,
        instance_start: u64,
        socket: u64,
        positions: &'a [[f32; 3]],
    ) -> Self {
        self.definition = definition;
        self.instance_start = instance_start;
        self.socket = socket;
        self.positions = positions;
        self
    }
}

fn add_fragment(recipe: &mut AssetRecipe, spec: FragmentSpec<'_>) -> RemappedFragment {
    let local_definition = PartDefinitionId(1);
    let local_socket = SocketId(1);
    let concrete_definition = PartDefinitionId(spec.definition);
    let concrete_socket = SocketId(spec.socket);
    let local_roots = (0..spec.positions.len())
        .map(|index| PartInstanceId(index as u64 + 1))
        .collect::<Vec<_>>();
    let concrete_roots = (0..spec.positions.len())
        .map(|index| PartInstanceId(spec.instance_start + index as u64))
        .collect::<Vec<_>>();

    let mut local_recipe = AssetRecipe::new(AssetId(spec.definition), spec.fragment_id);
    local_recipe.definitions.insert(
        local_definition,
        definition(local_definition, local_socket, spec.tags),
    );
    recipe.definitions.insert(
        concrete_definition,
        definition(concrete_definition, concrete_socket, spec.tags),
    );
    for ((local, concrete), position) in local_roots
        .iter()
        .copied()
        .zip(concrete_roots.iter().copied())
        .zip(spec.positions.iter().copied())
    {
        local_recipe
            .instances
            .insert(local, instance(local, local_definition, None, position));
        local_recipe.root_instances.push(local);
        recipe.instances.insert(
            concrete,
            instance(concrete, concrete_definition, None, position),
        );
        recipe.root_instances.push(concrete);
    }
    local_recipe.next_ids.part_definition = 2;
    local_recipe.next_ids.part_instance = local_roots.len() as u64 + 1;
    local_recipe.next_ids.socket = 2;
    recipe.next_ids.part_definition = recipe
        .next_ids
        .part_definition
        .max(concrete_definition.0.saturating_add(1));
    recipe.next_ids.part_instance = recipe
        .next_ids
        .part_instance
        .max(spec.instance_start + spec.positions.len() as u64);
    recipe.next_ids.socket = recipe
        .next_ids
        .socket
        .max(concrete_socket.0.saturating_add(1));

    let mut remap = FragmentRemap::default();
    remap
        .definitions
        .insert(local_definition, concrete_definition);
    remap.sockets.insert(local_socket, concrete_socket);
    for (local, concrete) in local_roots
        .iter()
        .copied()
        .zip(concrete_roots.iter().copied())
    {
        remap.instances.insert(local, concrete);
    }

    RemappedFragment {
        role: spec.role.to_owned(),
        fragment: RecipeFragment {
            schema_version: RECIPE_FRAGMENT_SCHEMA_VERSION,
            id: spec.fragment_id.to_owned(),
            provided_role: spec.role.to_owned(),
            exports: RecipeFragmentExports {
                role_occurrence_roots: local_roots,
                internal_roots: Vec::new(),
                socket_ports: vec![FragmentSocketPort {
                    id: spec.port_id.to_owned(),
                    local_occurrence_root: PartInstanceId(1),
                    local_socket,
                    compatibility_tags: spec.tags.iter().map(|tag| (*tag).to_owned()).collect(),
                }],
                surface_ports: Vec::new(),
            },
            recipe: local_recipe,
        },
        remap,
    }
}

fn selected<'a>(
    first: &'a RemappedFragment,
    second: &'a RemappedFragment,
) -> Vec<SelectedFragmentPorts<'a>> {
    vec![
        SelectedFragmentPorts {
            role: &first.role,
            fragment: &first.fragment,
            remap: &first.remap,
        },
        SelectedFragmentPorts {
            role: &second.role,
            fragment: &second.fragment,
            remap: &second.remap,
        },
    ]
}

fn definition(id: PartDefinitionId, socket: SocketId, tags: &[&str]) -> PartDefinition {
    PartDefinition {
        id,
        name: format!("definition_{}", id.0),
        tags: BTreeSet::new(),
        geometry: GeometryRecipe {
            source: GeometrySource::RoundedBox {
                half_extents: [0.25, 0.25, 0.25],
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

fn family(rules: Vec<AttachmentRule>) -> AssetFamilySchema {
    let role_ids = rules
        .iter()
        .flat_map(|rule| [rule.from_role.clone(), rule.to_role.clone()])
        .collect::<BTreeSet<_>>();
    AssetFamilySchema {
        schema_version: ASSET_FAMILY_SCHEMA_VERSION,
        id: "test_family".to_owned(),
        display_name: "Test Family".to_owned(),
        summary: "Fragment attachment tests.".to_owned(),
        part_roles: role_ids
            .into_iter()
            .map(|id| shape_family::PartRole {
                id: id.clone(),
                display_name: id,
                required: true,
                multiplicity: RoleMultiplicity::Repeated,
                provision: RoleProvision::FamilyDefault,
                semantic_tags: Vec::new(),
            })
            .collect(),
        attachment_rules: rules,
        allowed_operations: vec![AllowedOperationKind::Primitive],
        parameter_slots: Vec::new(),
        constraints: Vec::new(),
        variant_rules: Vec::new(),
        export_requirements: Vec::new(),
        compatible_style_kits: Vec::new(),
        tags: Vec::new(),
    }
}

fn rule(id: &str, from_role: &str, to_role: &str, tags: &[&str]) -> AttachmentRule {
    AttachmentRule {
        id: id.to_owned(),
        from_role: from_role.to_owned(),
        to_role: to_role.to_owned(),
        anchor_role: None,
        compatibility_tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
        required: true,
        execution_policy: FamilyRuleExecutionPolicy::Required,
    }
}

fn implementation(bindings: Vec<FragmentAttachmentBinding>) -> FamilyImplementation {
    FamilyImplementation {
        schema_version: FAMILY_IMPLEMENTATION_SCHEMA_VERSION,
        family_id: "test_family".to_owned(),
        base_recipe: AssetRecipe::new(AssetId(1), "base"),
        parameter_bindings: Vec::new(),
        default_role_providers: BTreeMap::new(),
        fragments: BTreeMap::new(),
        attachment_bindings: bindings,
    }
}

fn binding(
    family_attachment_rule: &str,
    child_role: &str,
    child_port: &str,
    parent_role: &str,
    parent_port: &str,
    pairing: FragmentAttachmentPairing,
) -> FragmentAttachmentBinding {
    FragmentAttachmentBinding {
        family_attachment_rule: family_attachment_rule.to_owned(),
        parent_role: parent_role.to_owned(),
        parent_port: parent_port.to_owned(),
        child_role: child_role.to_owned(),
        child_port: child_port.to_owned(),
        pairing,
        rigid_offset: RigidOffset::default(),
        attachment_mode: AttachmentMode::RigidSeparate,
    }
}

fn attachment_pairs(
    report: &shape_family_compile::remap::ports::FragmentAttachmentBindingReport,
) -> Vec<(PartInstanceId, PartInstanceId)> {
    report
        .attachments
        .iter()
        .map(|attachment| (attachment.child_instance, attachment.parent_instance))
        .collect()
}

fn issue_codes(report: &shape_family::FamilyValidationReport) -> BTreeSet<&str> {
    report
        .issues
        .iter()
        .map(|issue| issue.code.as_str())
        .collect()
}
