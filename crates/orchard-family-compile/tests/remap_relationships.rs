use std::collections::{BTreeMap, BTreeSet};

use orchard_asset::{
    AssetConstraint, AssetId, AssetPartSelector, AssetRecipe, AssetRelationshipPolicy,
    AttachmentMode, AttachmentSpec, Frame3, GeometryRecipe, GeometrySource, ModelingOperationSpec,
    OperationId, ParameterDescriptor, ParameterId, PartDefinition, PartDefinitionId, PartInstance,
    PartInstanceId, RelationshipPairing, SocketId, SocketSpec, Transform3,
};
use orchard_family_compile::remap::{
    FragmentRemap, FragmentRemapError,
    relationships::{
        append_remapped_fragment_relationships, remap_fragment_relationships, remap_part_selector,
    },
};

const FRAGMENT: &str = "fragment-a";
const BODY_DEF: PartDefinitionId = PartDefinitionId(10);
const CHILD_DEF: PartDefinitionId = PartDefinitionId(11);
const BODY: PartInstanceId = PartInstanceId(100);
const CHILD: PartInstanceId = PartInstanceId(101);
const GENERATED: PartInstanceId = PartInstanceId(102);
const ARRAY_OP: OperationId = OperationId(40);
const BODY_SOCKET: SocketId = SocketId(500);
const CHILD_SOCKET: SocketId = SocketId(501);
const WIDTH: ParameterId = ParameterId(7);

#[test]
fn remaps_every_selector_policy_pairing_constraint_and_lock() {
    let recipe = source_recipe();
    let remap = fragment_remap();

    let remapped =
        remap_fragment_relationships(FRAGMENT, &recipe, &remap).expect("relationships remap");

    assert_eq!(
        remapped.constraints,
        vec![
            AssetConstraint::RequireInstance {
                instance: PartInstanceId(200)
            },
            AssetConstraint::MutuallyExclusiveTags {
                first: "left".to_owned(),
                second: "right".to_owned()
            },
            AssetConstraint::Custom {
                code: "author-note".to_owned(),
                message: "preserve text".to_owned()
            },
        ]
    );
    assert_eq!(
        remapped.relationships,
        vec![
            AssetRelationshipPolicy::MayOverlap {
                first: AssetPartSelector::SpecificInstance {
                    instance: PartInstanceId(200)
                },
                second: AssetPartSelector::GeneratedByOperation {
                    operation: OperationId(140)
                },
                pairing: RelationshipPairing::AllPairs,
                reason: "intentional generated overlap".to_owned(),
            },
            AssetRelationshipPolicy::MustNotIntersect {
                first: AssetPartSelector::PrototypeAndGeneratedOccurrences {
                    prototype: PartInstanceId(201)
                },
                second: AssetPartSelector::PartTag {
                    tag: "detail".to_owned()
                },
                pairing: RelationshipPairing::ByOccurrenceIndex,
            },
            AssetRelationshipPolicy::MustTouch {
                first: AssetPartSelector::DefinitionRole {
                    role: "role:body".to_owned()
                },
                second: AssetPartSelector::SpecificInstance {
                    instance: PartInstanceId(201)
                },
                pairing: RelationshipPairing::ByPrototypeLineage,
                max_clearance: 0.25,
            },
            AssetRelationshipPolicy::MustContain {
                container: AssetPartSelector::PartTag {
                    tag: "shell".to_owned()
                },
                contained: AssetPartSelector::DefinitionRole {
                    role: "role:detail".to_owned()
                },
                pairing: RelationshipPairing::NearestOneToOne,
            },
            AssetRelationshipPolicy::MinimumClearance {
                first: AssetPartSelector::GeneratedByOperation {
                    operation: OperationId(140)
                },
                second: AssetPartSelector::SpecificInstance {
                    instance: PartInstanceId(202)
                },
                pairing: RelationshipPairing::Explicit(vec![
                    (PartInstanceId(200), PartInstanceId(201)),
                    (PartInstanceId(201), PartInstanceId(201)),
                    (PartInstanceId(201), PartInstanceId(202)),
                ]),
                clearance: 0.5,
            },
            AssetRelationshipPolicy::SocketAttached {
                parent: AssetPartSelector::SpecificInstance {
                    instance: PartInstanceId(200)
                },
                child: AssetPartSelector::SpecificInstance {
                    instance: PartInstanceId(201)
                },
                pairing: RelationshipPairing::Explicit(vec![(
                    PartInstanceId(200),
                    PartInstanceId(201)
                )]),
                parent_socket: SocketId(600),
                child_socket: SocketId(601),
                max_origin_distance: 0.01,
                max_axis_angle_degrees: 1.0,
                max_clearance: Some(0.02),
            },
        ]
    );
    assert_eq!(remapped.parameter_locks, BTreeSet::from([ParameterId(17)]));
    assert_eq!(
        remapped.instance_locks,
        BTreeSet::from([PartInstanceId(200), PartInstanceId(201)])
    );
    assert_eq!(
        remapped.subtree_locks,
        BTreeSet::from([PartInstanceId(200)])
    );
    assert_eq!(
        remapped.topology_locks,
        BTreeSet::from([PartDefinitionId(110), PartDefinitionId(111)])
    );
}

#[test]
fn append_remapped_relationships_preserves_existing_target_metadata() {
    let recipe = source_recipe();
    let remap = fragment_remap();
    let mut target = AssetRecipe::new(AssetId(9), "target");
    target.constraints.push(AssetConstraint::Custom {
        code: "base".to_owned(),
        message: "kept".to_owned(),
    });
    target
        .relationships
        .push(AssetRelationshipPolicy::MustNotIntersect {
            first: AssetPartSelector::specific(PartInstanceId(1)),
            second: AssetPartSelector::specific(PartInstanceId(2)),
            pairing: RelationshipPairing::AllPairs,
        });
    target.locks.insert(ParameterId(1));

    let remapped = append_remapped_fragment_relationships(&mut target, FRAGMENT, &recipe, &remap)
        .expect("append remaps");

    assert_eq!(target.constraints.len(), remapped.constraints.len() + 1);
    assert_eq!(target.relationships.len(), remapped.relationships.len() + 1);
    assert_eq!(
        target.locks,
        BTreeSet::from([ParameterId(1), ParameterId(17)])
    );
    assert!(target.instance_locks.contains(&PartInstanceId(200)));
    assert!(target.subtree_locks.contains(&PartInstanceId(200)));
    assert!(target.topology_locks.contains(&PartDefinitionId(110)));
}

#[test]
fn tag_selectors_remain_unchanged() {
    let recipe = source_recipe();
    let remap = fragment_remap();

    let tag = remap_part_selector(
        FRAGMENT,
        &recipe,
        &remap,
        &AssetPartSelector::PartTag {
            tag: "detail".to_owned(),
        },
    )
    .expect("part tag remaps");
    let role = remap_part_selector(
        FRAGMENT,
        &recipe,
        &remap,
        &AssetPartSelector::DefinitionRole {
            role: "role:detail".to_owned(),
        },
    )
    .expect("role tag remaps");

    assert_eq!(
        tag,
        AssetPartSelector::PartTag {
            tag: "detail".to_owned()
        }
    );
    assert_eq!(
        role,
        AssetPartSelector::DefinitionRole {
            role: "role:detail".to_owned()
        }
    );
}

#[test]
fn malformed_external_refs_are_rejected_before_missing_mappings() {
    let mut recipe = source_recipe();
    recipe.relationships.clear();
    recipe
        .relationships
        .push(AssetRelationshipPolicy::MustNotIntersect {
            first: AssetPartSelector::specific(PartInstanceId(999)),
            second: AssetPartSelector::specific(CHILD),
            pairing: RelationshipPairing::AllPairs,
        });
    let mut remap = fragment_remap();
    remap.instances.remove(&PartInstanceId(999));

    let error =
        remap_fragment_relationships(FRAGMENT, &recipe, &remap).expect_err("external ref fails");

    assert!(matches!(
        error,
        FragmentRemapError::ExternalReference {
            fragment,
            id_kind,
            id,
        } if fragment == FRAGMENT && id_kind == "part instance" && id == "999"
    ));
}

fn source_recipe() -> AssetRecipe {
    let mut recipe = AssetRecipe::new(AssetId(1), "source");
    recipe.definitions.insert(
        BODY_DEF,
        definition(
            BODY_DEF,
            "Body",
            ["shell", "role:body"],
            [socket(BODY_SOCKET, "parent")],
            [ModelingOperationSpec::LinearArray {
                operation: ARRAY_OP,
                count: 3,
                offset: [1.0, 0.0, 0.0],
            }],
        ),
    );
    recipe.definitions.insert(
        CHILD_DEF,
        definition(
            CHILD_DEF,
            "Child",
            ["detail", "role:detail"],
            [socket(CHILD_SOCKET, "child")],
            [],
        ),
    );
    recipe
        .instances
        .insert(BODY, instance(BODY, BODY_DEF, "body", None, None, None));
    recipe.instances.insert(
        CHILD,
        instance(
            CHILD,
            CHILD_DEF,
            "child",
            Some(BODY),
            Some(attachment(BODY, BODY_SOCKET, CHILD_SOCKET)),
            None,
        ),
    );
    recipe.instances.insert(
        GENERATED,
        instance(
            GENERATED,
            CHILD_DEF,
            "generated child",
            Some(BODY),
            None,
            Some(ARRAY_OP),
        ),
    );
    recipe.root_instances.push(BODY);
    recipe.parameters.insert(
        WIDTH,
        ParameterDescriptor {
            id: WIDTH,
            path: "definition.10.geometry.width".to_owned(),
            label: "Width".to_owned(),
            group: "Main".to_owned(),
            minimum: 0.0,
            maximum: 10.0,
            step: 0.1,
            mutation_sigma: 0.1,
            topology_changing: false,
            beginner_description: "Width".to_owned(),
        },
    );
    recipe.locks.insert(WIDTH);
    recipe.instance_locks.extend([CHILD, BODY]);
    recipe.subtree_locks.insert(BODY);
    recipe.topology_locks.extend([CHILD_DEF, BODY_DEF]);
    recipe.constraints.extend([
        AssetConstraint::RequireInstance { instance: BODY },
        AssetConstraint::MutuallyExclusiveTags {
            first: "left".to_owned(),
            second: "right".to_owned(),
        },
        AssetConstraint::Custom {
            code: "author-note".to_owned(),
            message: "preserve text".to_owned(),
        },
    ]);
    recipe.relationships.extend([
        AssetRelationshipPolicy::MayOverlap {
            first: AssetPartSelector::SpecificInstance { instance: BODY },
            second: AssetPartSelector::GeneratedByOperation {
                operation: ARRAY_OP,
            },
            pairing: RelationshipPairing::AllPairs,
            reason: "intentional generated overlap".to_owned(),
        },
        AssetRelationshipPolicy::MustNotIntersect {
            first: AssetPartSelector::PrototypeAndGeneratedOccurrences { prototype: CHILD },
            second: AssetPartSelector::PartTag {
                tag: "detail".to_owned(),
            },
            pairing: RelationshipPairing::ByOccurrenceIndex,
        },
        AssetRelationshipPolicy::MustTouch {
            first: AssetPartSelector::DefinitionRole {
                role: "role:body".to_owned(),
            },
            second: AssetPartSelector::SpecificInstance { instance: CHILD },
            pairing: RelationshipPairing::ByPrototypeLineage,
            max_clearance: 0.25,
        },
        AssetRelationshipPolicy::MustContain {
            container: AssetPartSelector::PartTag {
                tag: "shell".to_owned(),
            },
            contained: AssetPartSelector::DefinitionRole {
                role: "role:detail".to_owned(),
            },
            pairing: RelationshipPairing::NearestOneToOne,
        },
        AssetRelationshipPolicy::MinimumClearance {
            first: AssetPartSelector::GeneratedByOperation {
                operation: ARRAY_OP,
            },
            second: AssetPartSelector::SpecificInstance {
                instance: GENERATED,
            },
            pairing: RelationshipPairing::Explicit(vec![
                (BODY, CHILD),
                (CHILD, CHILD),
                (CHILD, GENERATED),
            ]),
            clearance: 0.5,
        },
        AssetRelationshipPolicy::SocketAttached {
            parent: AssetPartSelector::SpecificInstance { instance: BODY },
            child: AssetPartSelector::SpecificInstance { instance: CHILD },
            pairing: RelationshipPairing::Explicit(vec![(BODY, CHILD)]),
            parent_socket: BODY_SOCKET,
            child_socket: CHILD_SOCKET,
            max_origin_distance: 0.01,
            max_axis_angle_degrees: 1.0,
            max_clearance: Some(0.02),
        },
    ]);
    recipe
}

fn fragment_remap() -> FragmentRemap {
    FragmentRemap {
        definitions: BTreeMap::from([
            (BODY_DEF, PartDefinitionId(110)),
            (CHILD_DEF, PartDefinitionId(111)),
        ]),
        instances: BTreeMap::from([
            (BODY, PartInstanceId(200)),
            (CHILD, PartInstanceId(201)),
            (GENERATED, PartInstanceId(202)),
        ]),
        parameters: BTreeMap::from([(WIDTH, ParameterId(17))]),
        operations: BTreeMap::from([(ARRAY_OP, OperationId(140))]),
        regions: BTreeMap::new(),
        boundary_loops: BTreeMap::new(),
        sockets: BTreeMap::from([(BODY_SOCKET, SocketId(600)), (CHILD_SOCKET, SocketId(601))]),
    }
}

fn definition<const T: usize, const S: usize, const O: usize>(
    id: PartDefinitionId,
    name: &str,
    tags: [&str; T],
    sockets: [SocketSpec; S],
    operations: [ModelingOperationSpec; O],
) -> PartDefinition {
    PartDefinition {
        id,
        name: name.to_owned(),
        tags: tags.into_iter().map(str::to_owned).collect(),
        geometry: GeometryRecipe {
            source: GeometrySource::RoundedBox {
                half_extents: [0.5, 0.5, 0.5],
                radius: 0.05,
            },
            operations: operations.into(),
        },
        regions: BTreeMap::new(),
        sockets: sockets
            .into_iter()
            .map(|socket| (socket.id, socket))
            .collect(),
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
    }
}

fn instance(
    id: PartInstanceId,
    definition: PartDefinitionId,
    name: &str,
    parent: Option<PartInstanceId>,
    attachment: Option<AttachmentSpec>,
    generated_by: Option<OperationId>,
) -> PartInstance {
    PartInstance {
        id,
        definition,
        name: name.to_owned(),
        parent,
        local_transform: Transform3::default(),
        attachment,
        enabled: true,
        tags: BTreeSet::new(),
        generated_by,
    }
}

fn socket(id: SocketId, role: &str) -> SocketSpec {
    SocketSpec {
        id,
        name: role.to_owned(),
        local_frame: Frame3::default(),
        role: role.to_owned(),
        tags: BTreeSet::from([role.to_owned()]),
    }
}

fn attachment(
    parent_instance: PartInstanceId,
    parent_socket: SocketId,
    child_socket: SocketId,
) -> AttachmentSpec {
    AttachmentSpec {
        parent_instance,
        parent_socket,
        child_socket,
        local_offset: Transform3::default(),
        mode: AttachmentMode::RigidSeparate,
    }
}
