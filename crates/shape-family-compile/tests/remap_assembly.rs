use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetId, AssetRecipe, AttachmentMode, AttachmentSpec, Frame3, GeometryRecipe, GeometrySource,
    ModelingOperationSpec, OperationId, PartDefinition, PartDefinitionId, PartInstance,
    PartInstanceId, RegionId, SocketId, SocketSpec, SurfaceRegionSpec, SurfaceRole, Transform3,
    validate_asset_recipe,
};
use shape_family_compile::{
    FragmentSocketPort, FragmentSurfacePort, FragmentSurfaceTarget, RECIPE_FRAGMENT_SCHEMA_VERSION,
    RecipeFragment, RecipeFragmentExports,
    remap::{FragmentRemapError, assembly::remap_fragment_assembly},
};

#[test]
fn multipart_hierarchy_remaps_local_structure_and_ports() {
    let mut target = target_recipe();
    let fragment = fragment(
        "multipart",
        "case",
        recipe_with_parts(
            vec![
                definition(
                    PartDefinitionId(1),
                    "body",
                    &[RegionId(1)],
                    &[SocketId(10)],
                    vec![],
                ),
                definition(
                    PartDefinitionId(2),
                    "panel",
                    &[RegionId(2)],
                    &[SocketId(11)],
                    vec![],
                ),
                definition(
                    PartDefinitionId(3),
                    "helper",
                    &[RegionId(3)],
                    &[SocketId(12)],
                    vec![],
                ),
            ],
            vec![
                instance(PartInstanceId(1), PartDefinitionId(1), None),
                transformed_instance(
                    PartInstanceId(2),
                    PartDefinitionId(2),
                    Some(PartInstanceId(1)),
                    [1.0, 2.0, 3.0],
                ),
                instance(
                    PartInstanceId(3),
                    PartDefinitionId(3),
                    Some(PartInstanceId(2)),
                ),
            ],
            vec![PartInstanceId(1)],
        ),
        RecipeFragmentExports {
            role_occurrence_roots: vec![PartInstanceId(2)],
            internal_roots: vec![PartInstanceId(3)],
            socket_ports: vec![FragmentSocketPort {
                id: "panel-mount".to_owned(),
                local_occurrence_root: PartInstanceId(2),
                local_socket: SocketId(11),
                compatibility_tags: vec!["panel".to_owned()],
            }],
            surface_ports: vec![
                FragmentSurfacePort {
                    id: "body-face".to_owned(),
                    target: FragmentSurfaceTarget::Definition(PartDefinitionId(1)),
                    local_region: RegionId(1),
                    semantic_tags: vec!["outer".to_owned()],
                },
                FragmentSurfacePort {
                    id: "panel-face".to_owned(),
                    target: FragmentSurfaceTarget::Occurrence(PartInstanceId(2)),
                    local_region: RegionId(2),
                    semantic_tags: vec!["visible".to_owned()],
                },
            ],
        },
    );

    let remapped = remap_fragment_assembly(&mut target, &fragment).expect("fragment remaps");
    let body = remapped.remap.definitions[&PartDefinitionId(1)];
    let panel = remapped.remap.definitions[&PartDefinitionId(2)];
    let root = remapped.remap.instances[&PartInstanceId(1)];
    let occurrence = remapped.remap.instances[&PartInstanceId(2)];
    let internal = remapped.remap.instances[&PartInstanceId(3)];

    assert_eq!(target.root_instances, vec![root]);
    assert_eq!(target.instances[&occurrence].parent, Some(root));
    assert_eq!(target.instances[&internal].parent, Some(occurrence));
    assert_eq!(
        target.instances[&occurrence].local_transform.translation,
        [1.0, 2.0, 3.0]
    );
    assert_eq!(
        target.definitions[&body].local_pivot.origin,
        [1.0, 0.0, 0.0]
    );
    assert!(target.definitions[&body].tags.contains("body"));
    assert!(
        target.instances[&occurrence]
            .tags
            .contains("panel-instance")
    );

    assert_eq!(remapped.exports.role_occurrence_roots, vec![occurrence]);
    assert_eq!(remapped.exports.internal_roots, vec![internal]);
    assert_eq!(
        remapped.exports.socket_ports[0].local_socket,
        remapped.remap.sockets[&SocketId(11)]
    );
    assert_eq!(
        remapped.exports.surface_ports[0].target,
        FragmentSurfaceTarget::Definition(body)
    );
    assert_eq!(
        remapped.exports.surface_ports[1].target,
        FragmentSurfaceTarget::Occurrence(occurrence)
    );
    assert_eq!(
        remapped.exports.surface_ports[1].local_region,
        remapped.remap.regions[&RegionId(2)]
    );
    assert!(
        target.definitions[&panel]
            .sockets
            .contains_key(&remapped.remap.sockets[&SocketId(11)])
    );
    assert_valid(&target);
}

#[test]
fn shared_definitions_reuse_one_remapped_definition() {
    let mut target = target_recipe();
    let fragment = fragment(
        "shared-definition",
        "leg",
        recipe_with_parts(
            vec![definition(
                PartDefinitionId(8),
                "shared leg",
                &[RegionId(8)],
                &[SocketId(8)],
                vec![],
            )],
            vec![
                instance(PartInstanceId(4), PartDefinitionId(8), None),
                instance(PartInstanceId(5), PartDefinitionId(8), None),
            ],
            vec![PartInstanceId(5), PartInstanceId(4)],
        ),
        RecipeFragmentExports {
            role_occurrence_roots: vec![PartInstanceId(4), PartInstanceId(5)],
            ..RecipeFragmentExports::default()
        },
    );

    let remapped = remap_fragment_assembly(&mut target, &fragment).expect("fragment remaps");
    let shared_definition = remapped.remap.definitions[&PartDefinitionId(8)];
    let first = remapped.remap.instances[&PartInstanceId(4)];
    let second = remapped.remap.instances[&PartInstanceId(5)];

    assert_eq!(target.definitions.len(), 1);
    assert_eq!(target.instances[&first].definition, shared_definition);
    assert_eq!(target.instances[&second].definition, shared_definition);
    assert_eq!(target.root_instances, vec![first, second]);
    assert_eq!(remapped.exports.role_occurrence_roots, vec![first, second]);
    assert_valid(&target);
}

#[test]
fn disabled_parent_keeps_nested_occurrence_effectively_disabled() {
    let mut target = target_recipe();
    let mut parent = instance(PartInstanceId(1), PartDefinitionId(1), None);
    parent.enabled = false;
    let fragment = fragment(
        "disabled-parent",
        "drawer",
        recipe_with_parts(
            vec![definition(
                PartDefinitionId(1),
                "disabled parent",
                &[RegionId(1)],
                &[SocketId(1)],
                vec![],
            )],
            vec![
                parent,
                instance(
                    PartInstanceId(2),
                    PartDefinitionId(1),
                    Some(PartInstanceId(1)),
                ),
            ],
            vec![PartInstanceId(1)],
        ),
        RecipeFragmentExports {
            role_occurrence_roots: vec![PartInstanceId(2)],
            ..RecipeFragmentExports::default()
        },
    );

    let remapped = remap_fragment_assembly(&mut target, &fragment).expect("fragment remaps");
    let parent = remapped.remap.instances[&PartInstanceId(1)];
    let child = remapped.remap.instances[&PartInstanceId(2)];

    assert!(!target.instances[&parent].enabled);
    assert!(target.instances[&child].enabled);
    assert!(!is_effectively_enabled(&target, child));
    assert_valid(&target);
}

#[test]
fn socket_attachment_remaps_parent_child_and_sockets() {
    let mut target = target_recipe();
    let mut child = instance(
        PartInstanceId(2),
        PartDefinitionId(2),
        Some(PartInstanceId(1)),
    );
    child.attachment = Some(AttachmentSpec {
        parent_instance: PartInstanceId(1),
        parent_socket: SocketId(10),
        child_socket: SocketId(20),
        local_offset: Transform3 {
            translation: [0.25, 0.0, 0.0],
            ..Transform3::default()
        },
        mode: AttachmentMode::RigidSeparate,
    });
    let fragment = fragment(
        "socket-attachment",
        "wheel",
        recipe_with_parts(
            vec![
                definition(
                    PartDefinitionId(1),
                    "parent",
                    &[RegionId(1)],
                    &[SocketId(10)],
                    vec![],
                ),
                definition(
                    PartDefinitionId(2),
                    "child",
                    &[RegionId(2)],
                    &[SocketId(20)],
                    vec![],
                ),
            ],
            vec![
                instance(PartInstanceId(1), PartDefinitionId(1), None),
                child,
            ],
            vec![PartInstanceId(1)],
        ),
        RecipeFragmentExports::default(),
    );

    let remapped = remap_fragment_assembly(&mut target, &fragment).expect("fragment remaps");
    let child = remapped.remap.instances[&PartInstanceId(2)];
    let attachment = target.instances[&child]
        .attachment
        .as_ref()
        .expect("attachment remapped");

    assert_eq!(
        attachment.parent_instance,
        remapped.remap.instances[&PartInstanceId(1)]
    );
    assert_eq!(
        attachment.parent_socket,
        remapped.remap.sockets[&SocketId(10)]
    );
    assert_eq!(
        attachment.child_socket,
        remapped.remap.sockets[&SocketId(20)]
    );
    assert_eq!(attachment.local_offset.translation, [0.25, 0.0, 0.0]);
    assert_valid(&target);
}

#[test]
fn generated_occurrence_and_mirrored_provenance_remap_operations() {
    let mut target = target_recipe();
    let generator = OperationId(50);
    let mut generated = instance(
        PartInstanceId(2),
        PartDefinitionId(1),
        Some(PartInstanceId(1)),
    );
    generated.generated_by = Some(generator);
    let fragment = fragment(
        "mirrored-generated",
        "bolt",
        recipe_with_parts(
            vec![definition(
                PartDefinitionId(1),
                "mirrored part",
                &[RegionId(1)],
                &[SocketId(1)],
                vec![ModelingOperationSpec::MirrorInstances {
                    operation: generator,
                    plane_normal: [1.0, 0.0, 0.0],
                    plane_offset: 0.0,
                }],
            )],
            vec![
                instance(PartInstanceId(1), PartDefinitionId(1), None),
                generated,
            ],
            vec![PartInstanceId(1)],
        ),
        RecipeFragmentExports::default(),
    );

    let remapped = remap_fragment_assembly(&mut target, &fragment).expect("fragment remaps");
    let new_operation = remapped.remap.operations[&generator];
    let new_definition = remapped.remap.definitions[&PartDefinitionId(1)];
    let generated = remapped.remap.instances[&PartInstanceId(2)];

    assert_eq!(
        target.instances[&generated].generated_by,
        Some(new_operation)
    );
    assert_eq!(
        target.definitions[&new_definition].geometry.operations[0].operation_id(),
        new_operation
    );
    assert_valid(&target);
}

#[test]
fn multiple_exported_socket_ports_remap_through_socket_map() {
    let mut target = target_recipe();
    let fragment = fragment(
        "socket-ports",
        "rail",
        recipe_with_parts(
            vec![definition(
                PartDefinitionId(1),
                "rail",
                &[RegionId(1)],
                &[SocketId(10), SocketId(11)],
                vec![],
            )],
            vec![instance(PartInstanceId(1), PartDefinitionId(1), None)],
            vec![PartInstanceId(1)],
        ),
        RecipeFragmentExports {
            role_occurrence_roots: vec![PartInstanceId(1)],
            socket_ports: vec![
                FragmentSocketPort {
                    id: "left".to_owned(),
                    local_occurrence_root: PartInstanceId(1),
                    local_socket: SocketId(10),
                    compatibility_tags: vec!["left".to_owned()],
                },
                FragmentSocketPort {
                    id: "right".to_owned(),
                    local_occurrence_root: PartInstanceId(1),
                    local_socket: SocketId(11),
                    compatibility_tags: vec!["right".to_owned()],
                },
            ],
            ..RecipeFragmentExports::default()
        },
    );

    let remapped = remap_fragment_assembly(&mut target, &fragment).expect("fragment remaps");

    assert_eq!(
        remapped.exports.socket_ports[0].local_socket,
        remapped.remap.sockets[&SocketId(10)]
    );
    assert_eq!(
        remapped.exports.socket_ports[1].local_socket,
        remapped.remap.sockets[&SocketId(11)]
    );
    assert_ne!(
        remapped.exports.socket_ports[0].local_socket,
        remapped.exports.socket_ports[1].local_socket
    );
    assert_eq!(
        remapped.exports.socket_ports[1].compatibility_tags,
        vec!["right".to_owned()]
    );
    assert_valid(&target);
}

#[test]
fn invalid_external_parent_reference_is_rejected() {
    let mut target = target_recipe();
    let fragment = fragment(
        "external-parent",
        "bad",
        recipe_with_parts(
            vec![definition(
                PartDefinitionId(1),
                "part",
                &[RegionId(1)],
                &[SocketId(1)],
                vec![],
            )],
            vec![instance(
                PartInstanceId(1),
                PartDefinitionId(1),
                Some(PartInstanceId(999)),
            )],
            vec![PartInstanceId(1)],
        ),
        RecipeFragmentExports::default(),
    );

    let err = remap_fragment_assembly(&mut target, &fragment).expect_err("external parent rejects");

    assert!(matches!(
        err,
        FragmentRemapError::ExternalReference {
            id_kind,
            id,
            ..
        } if id_kind == "part_instance" && id == "999"
    ));
    assert!(target.instances.is_empty());
}

#[test]
fn parent_cycles_are_rejected() {
    let mut target = target_recipe();
    let fragment = fragment(
        "cycle",
        "bad",
        recipe_with_parts(
            vec![definition(
                PartDefinitionId(1),
                "part",
                &[RegionId(1)],
                &[SocketId(1)],
                vec![],
            )],
            vec![
                instance(
                    PartInstanceId(1),
                    PartDefinitionId(1),
                    Some(PartInstanceId(2)),
                ),
                instance(
                    PartInstanceId(2),
                    PartDefinitionId(1),
                    Some(PartInstanceId(1)),
                ),
            ],
            vec![],
        ),
        RecipeFragmentExports::default(),
    );

    let err = remap_fragment_assembly(&mut target, &fragment).expect_err("cycle rejects");

    assert!(matches!(
        err,
        FragmentRemapError::Unsupported { stage, reason, .. }
            if stage == "assembly" && reason.contains("cycle")
    ));
    assert!(target.instances.is_empty());
}

#[test]
fn overlapping_occurrence_and_internal_roots_are_rejected() {
    let mut target = target_recipe();
    let fragment = fragment(
        "overlapping-roots",
        "bad",
        recipe_with_parts(
            vec![definition(
                PartDefinitionId(1),
                "part",
                &[RegionId(1)],
                &[SocketId(1)],
                vec![],
            )],
            vec![instance(PartInstanceId(1), PartDefinitionId(1), None)],
            vec![PartInstanceId(1)],
        ),
        RecipeFragmentExports {
            role_occurrence_roots: vec![PartInstanceId(1)],
            internal_roots: vec![PartInstanceId(1)],
            ..RecipeFragmentExports::default()
        },
    );

    let err = remap_fragment_assembly(&mut target, &fragment).expect_err("overlap rejects");

    assert!(matches!(
        err,
        FragmentRemapError::Unsupported { stage, reason, .. }
            if stage == "assembly" && reason.contains("disjoint")
    ));
    assert!(target.instances.is_empty());
}

fn target_recipe() -> AssetRecipe {
    let mut recipe = AssetRecipe::new(AssetId(9000), "target");
    recipe.next_ids.part_definition = 100;
    recipe.next_ids.part_instance = 200;
    recipe.next_ids.operation = 300;
    recipe.next_ids.region = 400;
    recipe.next_ids.boundary_loop = 500;
    recipe.next_ids.socket = 600;
    recipe
}

fn recipe_with_parts(
    definitions: Vec<PartDefinition>,
    instances: Vec<PartInstance>,
    roots: Vec<PartInstanceId>,
) -> AssetRecipe {
    let mut recipe = AssetRecipe::new(AssetId(1000), "fragment");
    recipe.definitions = definitions
        .into_iter()
        .map(|definition| (definition.id, definition))
        .collect();
    recipe.instances = instances
        .into_iter()
        .map(|instance| (instance.id, instance))
        .collect();
    recipe.root_instances = roots;
    recipe
}

fn fragment(
    id: &str,
    role: &str,
    recipe: AssetRecipe,
    exports: RecipeFragmentExports,
) -> RecipeFragment {
    RecipeFragment {
        schema_version: RECIPE_FRAGMENT_SCHEMA_VERSION,
        id: id.to_owned(),
        provided_role: role.to_owned(),
        exports,
        recipe,
    }
}

fn definition(
    id: PartDefinitionId,
    name: &str,
    region_ids: &[RegionId],
    socket_ids: &[SocketId],
    operations: Vec<ModelingOperationSpec>,
) -> PartDefinition {
    let mut tags = BTreeSet::new();
    tags.insert(name.replace(' ', "-"));
    let mut regions = BTreeMap::new();
    for region_id in region_ids {
        regions.insert(
            *region_id,
            SurfaceRegionSpec {
                id: *region_id,
                name: format!("region-{}", region_id.0),
                role: SurfaceRole::PrimarySurface,
                tags: BTreeSet::new(),
            },
        );
    }
    let mut sockets = BTreeMap::new();
    for socket_id in socket_ids {
        sockets.insert(
            *socket_id,
            SocketSpec {
                id: *socket_id,
                name: format!("socket-{}", socket_id.0),
                local_frame: Frame3::default(),
                role: "mount".to_owned(),
                tags: BTreeSet::new(),
            },
        );
    }
    PartDefinition {
        id,
        name: name.to_owned(),
        tags,
        geometry: GeometryRecipe {
            source: GeometrySource::RoundedBox {
                half_extents: [1.0, 1.0, 1.0],
                radius: 0.05,
            },
            operations,
        },
        regions,
        sockets,
        local_pivot: Frame3 {
            origin: [1.0, 0.0, 0.0],
            ..Frame3::default()
        },
        variant_group: None,
        production_hints: None,
    }
}

fn instance(
    id: PartInstanceId,
    definition: PartDefinitionId,
    parent: Option<PartInstanceId>,
) -> PartInstance {
    let mut tags = BTreeSet::new();
    tags.insert(format!("instance-{}", id.0));
    PartInstance {
        id,
        definition,
        name: format!("instance-{}", id.0),
        parent,
        local_transform: Transform3::default(),
        attachment: None,
        enabled: true,
        tags,
        generated_by: None,
    }
}

fn transformed_instance(
    id: PartInstanceId,
    definition: PartDefinitionId,
    parent: Option<PartInstanceId>,
    translation: [f32; 3],
) -> PartInstance {
    let mut instance = instance(id, definition, parent);
    instance.tags.insert("panel-instance".to_owned());
    instance.local_transform.translation = translation;
    instance
}

fn is_effectively_enabled(recipe: &AssetRecipe, instance: PartInstanceId) -> bool {
    let mut current = Some(instance);
    let mut seen = BTreeSet::new();
    while let Some(instance_id) = current {
        if !seen.insert(instance_id) {
            return false;
        }
        let Some(part) = recipe.instances.get(&instance_id) else {
            return false;
        };
        if !part.enabled {
            return false;
        }
        current = part.parent;
    }
    true
}

fn assert_valid(recipe: &AssetRecipe) {
    let report = validate_asset_recipe(recipe);
    assert!(report.is_valid(), "{:#?}", report.issues);
}
