use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AuthoredVariationMetadata, CountRangeHint, CutGroupRole, OperationId, ParameterDescriptor,
    ParameterId, ParameterRangeOverride, PartDefinitionId, PartInstanceId, ReplacementGroupHint,
    SemanticCutGroupHint,
};
use shape_family_compile::remap::{
    FragmentRemap, FragmentRemapError,
    variation::{
        VariationMetadataSource, remap_fragment_variation_metadata, remap_parameter_descriptor,
        remap_parameter_locks, remap_scalar_path, remap_variation_metadata,
    },
};

#[test]
fn remaps_generator_dimension_path() {
    let remap = sample_remap();

    let path = remap_scalar_path(
        "fragment",
        "definition.10.geometry.rounded_box.half_extents.x",
        &remap,
    )
    .expect("generator path should remap");

    assert_eq!(path, "definition.110.geometry.rounded_box.half_extents.x");
}

#[test]
fn remaps_operation_scalar_path() {
    let remap = sample_remap();

    let path = remap_scalar_path(
        "fragment",
        "definition.10.operation.40.linear_array.count",
        &remap,
    )
    .expect("operation scalar path should remap");

    assert_eq!(path, "definition.110.operation.440.linear_array.count");
}

#[test]
fn remaps_instance_transform_path() {
    let remap = sample_remap();

    let path = remap_scalar_path("fragment", "instance.20.transform.translation.z", &remap)
        .expect("instance transform path should remap");

    assert_eq!(path, "instance.220.transform.translation.z");
}

#[test]
fn remaps_cut_operation_path() {
    let remap = sample_remap();

    let path = remap_scalar_path(
        "fragment",
        "definition.10.operation.41.rectangular_through_cut.center.y",
        &remap,
    )
    .expect("cut operation path should remap");

    assert_eq!(
        path,
        "definition.110.operation.441.rectangular_through_cut.center.y"
    );
}

#[test]
fn remaps_bevel_operation_path() {
    let remap = sample_remap();

    let path = remap_scalar_path(
        "fragment",
        "definition.10.operation.42.bevel_boundary_loop.profile",
        &remap,
    )
    .expect("bevel operation path should remap");

    assert_eq!(
        path,
        "definition.110.operation.442.bevel_boundary_loop.profile"
    );
}

#[test]
fn remaps_parameter_descriptor_and_topology_metadata() {
    let remap = sample_remap();

    let descriptor = remap_parameter_descriptor(
        "fragment",
        &parameter(
            ParameterId(30),
            "definition.10.operation.40.linear_array.count",
            true,
        ),
        &remap,
    )
    .expect("parameter descriptor should remap");

    assert_eq!(descriptor.id, ParameterId(330));
    assert_eq!(
        descriptor.path,
        "definition.110.operation.440.linear_array.count"
    );
    assert!(descriptor.topology_changing);
}

#[test]
fn remaps_locked_parameter() {
    let remap = sample_remap();

    let locks = remap_parameter_locks(
        "fragment",
        &BTreeSet::from([ParameterId(30), ParameterId(31)]),
        &remap,
    )
    .expect("locked parameters should remap");

    assert_eq!(locks, BTreeSet::from([ParameterId(330), ParameterId(331)]));
}

#[test]
fn remaps_replacement_group() {
    let remap = sample_remap();
    let mut metadata = AuthoredVariationMetadata::default();
    metadata.replacement_groups.insert(
        "body_style".to_owned(),
        ReplacementGroupHint {
            definitions: BTreeSet::from([PartDefinitionId(10), PartDefinitionId(11)]),
        },
    );

    let remapped = remap_variation_metadata("fragment", &metadata, &remap)
        .expect("replacement group should remap");

    assert_eq!(
        remapped.replacement_groups["body_style"].definitions,
        BTreeSet::from([PartDefinitionId(110), PartDefinitionId(111)])
    );
}

#[test]
fn remaps_count_range() {
    let remap = sample_remap();
    let mut metadata = AuthoredVariationMetadata::default();
    metadata.count_ranges.insert(
        OperationId(40),
        CountRangeHint {
            minimum: 2,
            maximum: 7,
        },
    );

    let remapped =
        remap_variation_metadata("fragment", &metadata, &remap).expect("count range should remap");

    assert_eq!(
        remapped.count_ranges.get(&OperationId(440)),
        Some(&CountRangeHint {
            minimum: 2,
            maximum: 7
        })
    );
    assert!(!remapped.count_ranges.contains_key(&OperationId(40)));
}

#[test]
fn remaps_cut_group() {
    let remap = sample_remap();
    let mut metadata = AuthoredVariationMetadata::default();
    metadata.semantic_cut_groups.insert(
        "vents".to_owned(),
        SemanticCutGroupHint {
            label: "Vents".to_owned(),
            definition: PartDefinitionId(10),
            operations: vec![OperationId(41), OperationId(42)],
            role: CutGroupRole::Vents,
            count_range: Some(CountRangeHint {
                minimum: 2,
                maximum: 4,
            }),
        },
    );

    let remapped =
        remap_variation_metadata("fragment", &metadata, &remap).expect("cut group should remap");
    let group = &remapped.semantic_cut_groups["vents"];

    assert_eq!(group.definition, PartDefinitionId(110));
    assert_eq!(group.operations, vec![OperationId(441), OperationId(442)]);
    assert_eq!(
        group.count_range,
        Some(CountRangeHint {
            minimum: 2,
            maximum: 4
        })
    );
}

#[test]
fn rejects_malformed_numeric_id() {
    let remap = sample_remap();

    let error = remap_scalar_path(
        "fragment",
        "definition.bad.geometry.rounded_box.radius",
        &remap,
    )
    .expect_err("malformed numeric ID should fail");

    assert_unsupported_reason_contains(error, "malformed scalar path");
}

#[test]
fn rejects_unknown_path() {
    let remap = sample_remap();

    let error = remap_scalar_path("fragment", "definition.10.color.tint", &remap)
        .expect_err("unknown path should fail");

    assert_unsupported_reason_contains(error, "unknown scalar path grammar");
}

#[test]
fn remapping_is_deterministic() {
    let remap = sample_remap();
    let parameters = BTreeMap::from([
        (
            ParameterId(31),
            parameter(
                ParameterId(31),
                "definition.11.geometry.plate.size.x",
                false,
            ),
        ),
        (
            ParameterId(30),
            parameter(ParameterId(30), "instance.20.transform.scale.z", true),
        ),
    ]);
    let mut metadata = AuthoredVariationMetadata::default();
    metadata.optional_instances.insert(PartInstanceId(21));
    metadata.replacement_groups.insert(
        "body_style".to_owned(),
        ReplacementGroupHint {
            definitions: BTreeSet::from([PartDefinitionId(11)]),
        },
    );
    metadata.count_ranges.insert(
        OperationId(41),
        CountRangeHint {
            minimum: 1,
            maximum: 3,
        },
    );
    metadata.parameter_range_overrides.insert(
        ParameterId(30),
        ParameterRangeOverride {
            minimum: 0.1,
            maximum: 0.9,
            step: Some(0.1),
            mutation_sigma: Some(0.05),
        },
    );
    metadata.semantic_cut_groups.insert(
        "vents".to_owned(),
        SemanticCutGroupHint {
            label: "Vents".to_owned(),
            definition: PartDefinitionId(11),
            operations: vec![OperationId(41)],
            role: CutGroupRole::Vents,
            count_range: None,
        },
    );

    let first = remap_fragment_variation_metadata(
        "fragment",
        VariationMetadataSource {
            parameters: &parameters,
            parameter_locks: &BTreeSet::from([ParameterId(30)]),
            instance_locks: &BTreeSet::from([PartInstanceId(20)]),
            subtree_locks: &BTreeSet::from([PartInstanceId(21)]),
            topology_locks: &BTreeSet::from([PartDefinitionId(11)]),
            variation: &metadata,
        },
        &remap,
    )
    .expect("metadata should remap");
    let second = remap_fragment_variation_metadata(
        "fragment",
        VariationMetadataSource {
            parameters: &parameters,
            parameter_locks: &BTreeSet::from([ParameterId(30)]),
            instance_locks: &BTreeSet::from([PartInstanceId(20)]),
            subtree_locks: &BTreeSet::from([PartInstanceId(21)]),
            topology_locks: &BTreeSet::from([PartDefinitionId(11)]),
            variation: &metadata,
        },
        &remap,
    )
    .expect("metadata should remap again");

    assert_eq!(first, second);
    assert_eq!(
        first.parameters.keys().copied().collect::<Vec<_>>(),
        vec![ParameterId(330), ParameterId(331)]
    );
    assert_eq!(first.parameter_locks, BTreeSet::from([ParameterId(330)]));
    assert_eq!(first.instance_locks, BTreeSet::from([PartInstanceId(220)]));
    assert_eq!(first.subtree_locks, BTreeSet::from([PartInstanceId(221)]));
    assert_eq!(
        first.topology_locks,
        BTreeSet::from([PartDefinitionId(111)])
    );
    assert_eq!(
        first.variation.optional_instances,
        BTreeSet::from([PartInstanceId(221)])
    );
    assert!(
        first
            .variation
            .parameter_range_overrides
            .contains_key(&ParameterId(330))
    );
}

fn sample_remap() -> FragmentRemap {
    let mut remap = FragmentRemap::default();
    remap.definitions.extend([
        (PartDefinitionId(10), PartDefinitionId(110)),
        (PartDefinitionId(11), PartDefinitionId(111)),
    ]);
    remap.instances.extend([
        (PartInstanceId(20), PartInstanceId(220)),
        (PartInstanceId(21), PartInstanceId(221)),
    ]);
    remap.parameters.extend([
        (ParameterId(30), ParameterId(330)),
        (ParameterId(31), ParameterId(331)),
    ]);
    remap.operations.extend([
        (OperationId(40), OperationId(440)),
        (OperationId(41), OperationId(441)),
        (OperationId(42), OperationId(442)),
    ]);
    remap
}

fn parameter(id: ParameterId, path: &str, topology_changing: bool) -> ParameterDescriptor {
    ParameterDescriptor {
        id,
        path: path.to_owned(),
        label: "Parameter".to_owned(),
        group: "Group".to_owned(),
        minimum: 0.0,
        maximum: 1.0,
        step: 0.1,
        mutation_sigma: 0.05,
        topology_changing,
        beginner_description: "Parameter for remap tests.".to_owned(),
    }
}

fn assert_unsupported_reason_contains(error: FragmentRemapError, needle: &str) {
    let FragmentRemapError::Unsupported { stage, reason, .. } = error else {
        panic!("expected unsupported variation error");
    };
    assert_eq!(stage, "variation");
    assert!(
        reason.contains(needle),
        "reason `{reason}` should contain `{needle}`"
    );
}
