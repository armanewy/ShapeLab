use std::collections::{BTreeMap, BTreeSet};

use shape_asset::*;

const BODY: PartDefinitionId = PartDefinitionId(1);
const WHEEL: PartDefinitionId = PartDefinitionId(2);
const WHEEL_ALT: PartDefinitionId = PartDefinitionId(3);
const PLATE: PartDefinitionId = PartDefinitionId(4);
const SWEEP: PartDefinitionId = PartDefinitionId(5);
const LATHE: PartDefinitionId = PartDefinitionId(6);
const ARM: PartDefinitionId = PartDefinitionId(7);

const BODY_INSTANCE: PartInstanceId = PartInstanceId(1);
const WHEEL_INSTANCE: PartInstanceId = PartInstanceId(2);

const BEVEL: OperationId = OperationId(1);
const LINEAR_ARRAY: OperationId = OperationId(2);
const RADIAL_ARRAY: OperationId = OperationId(3);

const BODY_SOCKET: SocketId = SocketId(1);
const WHEEL_SOCKET: SocketId = SocketId(2);
const PLATE_SOCKET: SocketId = SocketId(3);

const BODY_RADIUS: ParameterId = ParameterId(1);
const WHEEL_SEGMENTS: ParameterId = ParameterId(2);
const BODY_TRANSLATE_X: ParameterId = ParameterId(3);
const ARRAY_COUNT: ParameterId = ParameterId(4);
const WHEEL_RADIUS: ParameterId = ParameterId(5);

fn socket(id: SocketId, name: &str) -> SocketSpec {
    SocketSpec {
        id,
        name: name.to_owned(),
        local_frame: Frame3::default(),
        role: "mount".to_owned(),
        tags: BTreeSet::new(),
    }
}

fn frame_at(origin: [f32; 3]) -> Frame3 {
    Frame3 {
        origin,
        ..Frame3::default()
    }
}

fn base_definition(
    id: PartDefinitionId,
    name: &str,
    source: GeometrySource,
    sockets: BTreeMap<SocketId, SocketSpec>,
    variant_group: Option<&str>,
) -> PartDefinition {
    PartDefinition {
        id,
        name: name.to_owned(),
        tags: BTreeSet::new(),
        geometry: GeometryRecipe {
            source,
            operations: Vec::new(),
        },
        regions: BTreeMap::new(),
        sockets,
        local_pivot: Frame3::default(),
        variant_group: variant_group.map(str::to_owned),
        production_hints: None,
    }
}

struct ParameterSeed {
    id: ParameterId,
    path: String,
    label: &'static str,
    group: &'static str,
    minimum: f32,
    maximum: f32,
    topology_changing: bool,
}

fn add_parameter(recipe: &mut AssetRecipe, seed: ParameterSeed) {
    recipe.parameters.insert(
        seed.id,
        ParameterDescriptor {
            id: seed.id,
            path: seed.path,
            label: seed.label.to_owned(),
            group: seed.group.to_owned(),
            minimum: seed.minimum,
            maximum: seed.maximum,
            step: 1.0,
            mutation_sigma: 0.0,
            topology_changing: seed.topology_changing,
            beginner_description: seed.label.to_owned(),
        },
    );
}

fn edit_recipe() -> AssetRecipe {
    let mut recipe = AssetRecipe::new(AssetId(1), "Asset edits");

    let mut body_sockets = BTreeMap::new();
    body_sockets.insert(BODY_SOCKET, socket(BODY_SOCKET, "body_mount"));
    let mut body = base_definition(
        BODY,
        "Body",
        GeometrySource::RoundedBox {
            half_extents: [1.0, 0.5, 0.25],
            radius: 0.1,
        },
        body_sockets,
        None,
    );
    body.geometry.operations = vec![
        ModelingOperationSpec::SetBevelProfile {
            operation: BEVEL,
            radius: 0.05,
            segments: 2,
        },
        ModelingOperationSpec::LinearArray {
            operation: LINEAR_ARRAY,
            count: 2,
            offset: [1.0, 0.0, 0.0],
        },
        ModelingOperationSpec::RadialArray {
            operation: RADIAL_ARRAY,
            count: 3,
            axis: [0.0, 1.0, 0.0],
            angle_degrees: 180.0,
        },
    ];

    let mut wheel_sockets = BTreeMap::new();
    wheel_sockets.insert(WHEEL_SOCKET, socket(WHEEL_SOCKET, "wheel_mount"));
    let wheel = base_definition(
        WHEEL,
        "Wheel",
        GeometrySource::Cylinder {
            radius: 0.25,
            height: 0.2,
            radial_segments: 16,
        },
        wheel_sockets.clone(),
        Some("wheel"),
    );
    let wheel_alt = base_definition(
        WHEEL_ALT,
        "Wheel Alt",
        GeometrySource::Cylinder {
            radius: 0.3,
            height: 0.2,
            radial_segments: 16,
        },
        wheel_sockets,
        Some("wheel"),
    );

    let mut plate_sockets = BTreeMap::new();
    plate_sockets.insert(PLATE_SOCKET, socket(PLATE_SOCKET, "plate_mount"));
    let plate = base_definition(
        PLATE,
        "Plate",
        GeometrySource::Plate {
            size: [0.4, 0.2],
            thickness: 0.05,
        },
        plate_sockets,
        None,
    );
    let sweep = base_definition(
        SWEEP,
        "Sweep",
        GeometrySource::Sweep {
            profile: vec![[0.0, 0.0], [0.5, 0.0]],
            path: vec![Frame3::default(), frame_at([0.0, 1.0, 0.0])],
        },
        BTreeMap::new(),
        None,
    );
    let lathe = base_definition(
        LATHE,
        "Lathe",
        GeometrySource::Lathe {
            profile: vec![[0.2, 0.0], [0.4, 1.0]],
            segments: 16,
        },
        BTreeMap::new(),
        None,
    );
    let arm = base_definition(
        ARM,
        "Arm",
        GeometrySource::Cylinder {
            radius: 0.1,
            height: 0.8,
            radial_segments: 12,
        },
        BTreeMap::new(),
        Some("arm"),
    );

    for definition in [body, wheel, wheel_alt, plate, sweep, lathe, arm] {
        recipe.definitions.insert(definition.id, definition);
    }

    recipe.instances.insert(
        BODY_INSTANCE,
        PartInstance {
            id: BODY_INSTANCE,
            definition: BODY,
            name: "Body".to_owned(),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::new(),
            generated_by: None,
        },
    );
    recipe.instances.insert(
        WHEEL_INSTANCE,
        PartInstance {
            id: WHEEL_INSTANCE,
            definition: WHEEL,
            name: "Wheel L".to_owned(),
            parent: Some(BODY_INSTANCE),
            local_transform: Transform3 {
                translation: [-0.5, 0.0, 0.0],
                ..Transform3::default()
            },
            attachment: Some(AttachmentSpec {
                parent_instance: BODY_INSTANCE,
                parent_socket: BODY_SOCKET,
                child_socket: WHEEL_SOCKET,
                local_offset: Transform3::default(),
                mode: AttachmentMode::RigidSeparate,
            }),
            enabled: true,
            tags: BTreeSet::new(),
            generated_by: None,
        },
    );
    recipe.root_instances.push(BODY_INSTANCE);

    add_parameter(
        &mut recipe,
        ParameterSeed {
            id: BODY_RADIUS,
            path: definition_scalar_path(BODY, "geometry.rounded_box.radius"),
            label: "Corner Radius",
            group: "Edge Softness",
            minimum: 0.0,
            maximum: 1.0,
            topology_changing: false,
        },
    );
    add_parameter(
        &mut recipe,
        ParameterSeed {
            id: WHEEL_SEGMENTS,
            path: definition_scalar_path(WHEEL, "geometry.cylinder.radial_segments"),
            label: "Wheel Segments",
            group: "Detail Density",
            minimum: 3.0,
            maximum: 64.0,
            topology_changing: true,
        },
    );
    add_parameter(
        &mut recipe,
        ParameterSeed {
            id: BODY_TRANSLATE_X,
            path: instance_scalar_path(BODY_INSTANCE, "transform.translation.x"),
            label: "Body X",
            group: "Placement",
            minimum: -10.0,
            maximum: 10.0,
            topology_changing: false,
        },
    );
    add_parameter(
        &mut recipe,
        ParameterSeed {
            id: ARRAY_COUNT,
            path: definition_scalar_path(BODY, "operation.2.linear_array.count"),
            label: "Copies",
            group: "Repetition",
            minimum: 1.0,
            maximum: 6.0,
            topology_changing: true,
        },
    );
    add_parameter(
        &mut recipe,
        ParameterSeed {
            id: WHEEL_RADIUS,
            path: definition_scalar_path(WHEEL, "geometry.cylinder.radius"),
            label: "Wheel Radius",
            group: "Size",
            minimum: 0.1,
            maximum: 1.0,
            topology_changing: false,
        },
    );

    recipe.variation.optional_instances.insert(WHEEL_INSTANCE);
    recipe.variation.replacement_groups.insert(
        "wheel".to_owned(),
        ReplacementGroupHint {
            definitions: BTreeSet::from([WHEEL, WHEEL_ALT]),
        },
    );
    recipe.variation.count_ranges.insert(
        LINEAR_ARRAY,
        CountRangeHint {
            minimum: 1,
            maximum: 6,
        },
    );

    recipe.next_ids.part_definition = 8;
    recipe.next_ids.part_instance = 3;
    recipe.next_ids.operation = 4;
    recipe.next_ids.socket = 4;
    recipe.next_ids.parameter = 6;
    recipe
}

fn new_plate_instance(id: PartInstanceId) -> PartInstance {
    PartInstance {
        id,
        definition: PLATE,
        name: "Accessory".to_owned(),
        parent: None,
        local_transform: Transform3::default(),
        attachment: None,
        enabled: true,
        tags: BTreeSet::new(),
        generated_by: None,
    }
}

fn issue_codes(report: &AssetValidationReport) -> BTreeSet<&str> {
    report
        .issues
        .iter()
        .map(|issue| issue.code.as_str())
        .collect()
}

fn minimal_cut_recipe_value(
    schema_version: u32,
    operation: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": schema_version,
        "id": 42,
        "title": "cut recipe",
        "definitions": {
            "1": {
                "id": 1,
                "name": "plate",
                "tags": [],
                "geometry": {
                    "source": { "Plate": { "size": [1.0, 1.0], "thickness": 0.1 } },
                    "operations": [operation]
                },
                "regions": {
                    "1": { "id": 1, "name": "front", "role": "PrimarySurface", "tags": [] },
                    "2": { "id": 2, "name": "back", "role": "PrimarySurface", "tags": [] },
                    "3": { "id": 3, "name": "side", "role": "Side", "tags": [] }
                },
                "sockets": {},
                "local_pivot": {
                    "origin": [0.0, 0.0, 0.0],
                    "x_axis": [1.0, 0.0, 0.0],
                    "y_axis": [0.0, 1.0, 0.0],
                    "z_axis": [0.0, 0.0, 1.0]
                },
                "variant_group": null,
                "production_hints": null
            }
        },
        "instances": {
            "1": {
                "id": 1,
                "definition": 1,
                "name": "plate",
                "parent": null,
                "local_transform": {
                    "translation": [0.0, 0.0, 0.0],
                    "rotation_degrees": [0.0, 0.0, 0.0],
                    "scale": [1.0, 1.0, 1.0]
                },
                "attachment": null,
                "enabled": true,
                "tags": [],
                "generated_by": null
            }
        },
        "root_instances": [1],
        "parameters": {},
        "locks": [],
        "instance_locks": [],
        "subtree_locks": [],
        "topology_locks": [],
        "constraints": [],
        "relationships": [],
        "variation": {
            "optional_instances": [],
            "replacement_groups": {},
            "count_ranges": {},
            "parameter_range_overrides": {}
        },
        "next_ids": {
            "part_definition": 2,
            "part_instance": 2,
            "operation": 6,
            "region": 23,
            "boundary_loop": 9,
            "socket": 1,
            "parameter": 1,
            "revision": 1
        }
    })
}

#[test]
fn every_edit_type_applies_and_reports() {
    let recipe = edit_recipe();
    let mut replacement_arm = recipe.definitions[&ARM].clone();
    replacement_arm.name = "Renamed Arm".to_owned();

    let program = AssetEditProgram {
        label: "everything".to_owned(),
        seed: 42,
        operations: vec![
            AssetEdit::SetScalar {
                parameter: BODY_RADIUS,
                value: 0.2,
            },
            AssetEdit::SetTransform {
                instance: BODY_INSTANCE,
                transform: Transform3 {
                    translation: [0.25, 0.0, 0.0],
                    ..Transform3::default()
                },
            },
            AssetEdit::SetEnabled {
                instance: BODY_INSTANCE,
                enabled: false,
            },
            AssetEdit::SetOptionalPartEnabled {
                instance: WHEEL_INSTANCE,
                enabled: false,
            },
            AssetEdit::SetGeneratorDimension {
                definition: BODY,
                dimension: GeneratorDimensionEdit::RoundedBoxHalfExtents([1.2, 0.6, 0.3]),
            },
            AssetEdit::SetBevelSettings {
                definition: BODY,
                operation: BEVEL,
                radius: Some(0.08),
                segments: Some(3),
            },
            AssetEdit::SetSweepProfilePoint {
                definition: SWEEP,
                index: 1,
                point: [0.6, 0.1],
            },
            AssetEdit::SetSweepPathFrame {
                definition: SWEEP,
                index: 1,
                frame: frame_at([0.0, 2.0, 0.0]),
            },
            AssetEdit::SetLatheProfilePoint {
                definition: LATHE,
                index: 1,
                point: [0.45, 1.0],
            },
            AssetEdit::SetArraySpacing {
                definition: BODY,
                operation: LINEAR_ARRAY,
                spacing: ArraySpacingEdit::LinearOffset([1.5, 0.0, 0.0]),
            },
            AssetEdit::SetArraySpacing {
                definition: BODY,
                operation: RADIAL_ARRAY,
                spacing: ArraySpacingEdit::RadialAngleDegrees(270.0),
            },
            AssetEdit::SetArrayCount {
                definition: BODY,
                operation: LINEAR_ARRAY,
                count: 4,
            },
            AssetEdit::ReplaceGeometrySource {
                definition: PLATE,
                source: GeometrySource::Cylinder {
                    radius: 0.15,
                    height: 0.1,
                    radial_segments: 12,
                },
            },
            AssetEdit::AddInstance {
                instance: new_plate_instance(PartInstanceId(3)),
            },
            AssetEdit::Attach {
                instance: PartInstanceId(3),
                attachment: AttachmentSpec {
                    parent_instance: BODY_INSTANCE,
                    parent_socket: BODY_SOCKET,
                    child_socket: PLATE_SOCKET,
                    local_offset: Transform3::default(),
                    mode: AttachmentMode::RigidSeparate,
                },
            },
            AssetEdit::Detach {
                instance: PartInstanceId(3),
            },
            AssetEdit::DuplicateInstance {
                source: WHEEL_INSTANCE,
                instance: PartInstanceId(4),
                name: Some("Wheel copy".to_owned()),
                transform: None,
            },
            AssetEdit::MirrorInstance {
                source: WHEEL_INSTANCE,
                instance: PartInstanceId(5),
                plane: MirrorInstanceSpec {
                    plane_normal: [1.0, 0.0, 0.0],
                    plane_offset: 0.0,
                },
                name: Some("Wheel R".to_owned()),
            },
            AssetEdit::ReplaceInstanceDefinition {
                instance: WHEEL_INSTANCE,
                definition: WHEEL_ALT,
            },
            AssetEdit::ReorderChildInstances {
                parent: Some(BODY_INSTANCE),
                ordered_children: vec![PartInstanceId(5), PartInstanceId(4), WHEEL_INSTANCE],
            },
            AssetEdit::ReorderChildInstances {
                parent: None,
                ordered_children: vec![PartInstanceId(3), BODY_INSTANCE],
            },
            AssetEdit::RemoveInstance {
                instance: PartInstanceId(5),
            },
            AssetEdit::ReplaceDefinition {
                definition: replacement_arm,
            },
            AssetEdit::SetLock {
                parameter: WHEEL_RADIUS,
                locked: true,
            },
            AssetEdit::SetInstanceLock {
                instance: PartInstanceId(4),
                locked: true,
            },
            AssetEdit::SetSubtreeLock {
                instance: BODY_INSTANCE,
                locked: true,
            },
            AssetEdit::SetTopologyLock {
                definition: SWEEP,
                locked: true,
            },
        ],
    };

    let outcome =
        apply_edit_program_with_report(&recipe, &program).expect("complete edit program applies");

    assert_eq!(outcome.report.applied, program.operations.len());
    assert!(outcome.report.validation.is_valid());
    assert_eq!(
        get_scalar(
            &outcome.recipe,
            definition_scalar_path(BODY, "geometry.rounded_box.radius")
        )
        .expect("radius is readable"),
        0.2
    );
    assert_eq!(
        outcome.recipe.instances[&WHEEL_INSTANCE].definition,
        WHEEL_ALT
    );
    assert!(outcome.recipe.instance_locks.contains(&PartInstanceId(4)));
    assert!(outcome.recipe.subtree_locks.contains(&BODY_INSTANCE));
    assert!(outcome.recipe.topology_locks.contains(&SWEEP));
    assert!(!outcome.recipe.instances.contains_key(&PartInstanceId(5)));
}

#[test]
fn structural_edit_rollback_rejects_without_mutation() {
    let recipe = edit_recipe();
    let program = AssetEditProgram {
        label: "rollback".to_owned(),
        seed: 1,
        operations: vec![
            AssetEdit::AddInstance {
                instance: new_plate_instance(PartInstanceId(3)),
            },
            AssetEdit::SetTransform {
                instance: PartInstanceId(3),
                transform: Transform3 {
                    scale: [0.0, 1.0, 1.0],
                    ..Transform3::default()
                },
            },
        ],
    };

    assert!(matches!(
        apply_edit_program(&recipe, &program),
        Err(AssetError::ValidationFailed(report)) if issue_codes(&report).contains("zero_scale")
    ));
    assert!(!recipe.instances.contains_key(&PartInstanceId(3)));
}

#[test]
fn locked_parameter_rejects_atomically() {
    let mut recipe = edit_recipe();
    recipe.locks.insert(BODY_RADIUS);

    assert!(matches!(
        apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "locked parameter".to_owned(),
                seed: 2,
                operations: vec![AssetEdit::SetScalar {
                    parameter: BODY_RADIUS,
                    value: 0.3,
                }],
            },
        ),
        Err(AssetError::LockedParameter(BODY_RADIUS))
    ));
    assert_eq!(
        get_scalar(
            &recipe,
            definition_scalar_path(BODY, "geometry.rounded_box.radius")
        )
        .expect("radius is readable"),
        0.1
    );
}

#[test]
fn locked_part_rejects_instance_edits() {
    let mut recipe = edit_recipe();
    recipe.instance_locks.insert(WHEEL_INSTANCE);

    assert!(matches!(
        apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "locked part".to_owned(),
                seed: 3,
                operations: vec![AssetEdit::SetTransform {
                    instance: WHEEL_INSTANCE,
                    transform: Transform3 {
                        translation: [1.0, 0.0, 0.0],
                        ..Transform3::default()
                    },
                }],
            },
        ),
        Err(AssetError::LockedInstance(WHEEL_INSTANCE))
    ));
    assert_eq!(
        recipe.instances[&WHEEL_INSTANCE]
            .local_transform
            .translation,
        [-0.5, 0.0, 0.0]
    );
}

#[test]
fn topology_lock_allows_shape_preserving_parameters_only() {
    let mut recipe = edit_recipe();
    recipe.topology_locks.insert(WHEEL);

    let edited = apply_edit_program(
        &recipe,
        &AssetEditProgram {
            label: "non topology".to_owned(),
            seed: 4,
            operations: vec![AssetEdit::SetScalar {
                parameter: WHEEL_RADIUS,
                value: 0.3,
            }],
        },
    )
    .expect("non-topology parameter should apply");
    assert_eq!(
        get_scalar(
            &edited,
            definition_scalar_path(WHEEL, "geometry.cylinder.radius")
        )
        .expect("radius is readable"),
        0.3
    );

    assert!(matches!(
        apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "segments".to_owned(),
                seed: 5,
                operations: vec![AssetEdit::SetGeneratorDimension {
                    definition: WHEEL,
                    dimension: GeneratorDimensionEdit::CylinderRadialSegments(24),
                }],
            },
        ),
        Err(AssetError::LockedTopology(WHEEL))
    ));
    assert!(matches!(
        apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "replace generator".to_owned(),
                seed: 6,
                operations: vec![AssetEdit::ReplaceGeometrySource {
                    definition: WHEEL,
                    source: GeometrySource::Plate {
                        size: [0.2, 0.2],
                        thickness: 0.1,
                    },
                }],
            },
        ),
        Err(AssetError::LockedTopology(WHEEL))
    ));
}

#[test]
fn cut_corner_radius_positive_changes_preserve_topology_lock() {
    let mut recipe = edit_recipe();
    let cut_operation = OperationId(50);
    let mut plate = recipe.definitions[&PLATE].clone();
    plate.regions.insert(
        RegionId(1),
        SurfaceRegionSpec {
            id: RegionId(1),
            name: "front".to_owned(),
            role: SurfaceRole::PrimarySurface,
            tags: BTreeSet::new(),
        },
    );
    plate.geometry.operations = vec![ModelingOperationSpec::RecessedPanelCut {
        operation: cut_operation,
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.0, 0.0],
        size: [0.4, 0.3],
        depth: 0.04,
        corner_radius: 0.02,
        rim_width: 0.03,
        corner_segments: 4,
        entry_loop: BoundaryLoopId(1),
        floor_loop: BoundaryLoopId(2),
        outer_region: RegionId(1),
        rim_region: RegionId(20),
        wall_region: RegionId(21),
        floor_region: RegionId(22),
        edge_treatment: CutEdgeTreatment::Hard,
    }];
    recipe.definitions.insert(PLATE, plate.clone());
    recipe.topology_locks.insert(PLATE);

    let mut radius_changed = plate.clone();
    let ModelingOperationSpec::RecessedPanelCut { corner_radius, .. } =
        &mut radius_changed.geometry.operations[0]
    else {
        unreachable!("test plate should use a recessed cut");
    };
    *corner_radius = 0.05;
    apply_edit_program(
        &recipe,
        &AssetEditProgram {
            label: "radius".to_owned(),
            seed: 71,
            operations: vec![AssetEdit::ReplaceDefinition {
                definition: radius_changed,
            }],
        },
    )
    .expect("positive radius changes should preserve topology");

    let mut segment_changed = plate.clone();
    let ModelingOperationSpec::RecessedPanelCut {
        corner_segments, ..
    } = &mut segment_changed.geometry.operations[0]
    else {
        unreachable!("test plate should use a recessed cut");
    };
    *corner_segments = 6;
    assert!(matches!(
        apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "segments".to_owned(),
                seed: 72,
                operations: vec![AssetEdit::ReplaceDefinition {
                    definition: segment_changed,
                }],
            },
        ),
        Err(AssetError::LockedTopology(PLATE))
    ));

    let mut sharp_changed = plate.clone();
    let ModelingOperationSpec::RecessedPanelCut { corner_radius, .. } =
        &mut sharp_changed.geometry.operations[0]
    else {
        unreachable!("test plate should use a recessed cut");
    };
    *corner_radius = 0.0;
    assert!(matches!(
        apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "sharp".to_owned(),
                seed: 73,
                operations: vec![AssetEdit::ReplaceDefinition {
                    definition: sharp_changed,
                }],
            },
        ),
        Err(AssetError::LockedTopology(PLATE))
    ));
}

#[test]
fn compatible_and_incompatible_replacements_are_distinct() {
    let recipe = edit_recipe();

    let compatible = apply_edit_program(
        &recipe,
        &AssetEditProgram {
            label: "compatible".to_owned(),
            seed: 7,
            operations: vec![AssetEdit::ReplaceInstanceDefinition {
                instance: WHEEL_INSTANCE,
                definition: WHEEL_ALT,
            }],
        },
    )
    .expect("compatible variant should apply");

    assert_eq!(compatible.instances[&WHEEL_INSTANCE].definition, WHEEL_ALT);
    assert!(matches!(
        apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "incompatible".to_owned(),
                seed: 8,
                operations: vec![AssetEdit::ReplaceInstanceDefinition {
                    instance: WHEEL_INSTANCE,
                    definition: ARM,
                }],
            },
        ),
        Err(AssetError::IncompatibleReplacement {
            from: WHEEL,
            to: ARM
        })
    ));
}

#[test]
fn array_count_updates_operation_and_enforces_authored_range() {
    let recipe = edit_recipe();

    let edited = apply_edit_program(
        &recipe,
        &AssetEditProgram {
            label: "array count".to_owned(),
            seed: 9,
            operations: vec![AssetEdit::SetArrayCount {
                definition: BODY,
                operation: LINEAR_ARRAY,
                count: 5,
            }],
        },
    )
    .expect("array count should apply");

    assert!(matches!(
        edited.definitions[&BODY].geometry.operations.as_slice(),
        [_, ModelingOperationSpec::LinearArray { count: 5, .. }, _]
    ));
    assert!(matches!(
        apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "array count high".to_owned(),
                seed: 10,
                operations: vec![AssetEdit::SetArrayCount {
                    definition: BODY,
                    operation: LINEAR_ARRAY,
                    count: 7,
                }],
            },
        ),
        Err(AssetError::InvalidScalarValue { .. })
    ));
}

#[test]
fn structural_modeling_operation_edits_update_history_and_ids() {
    let mut recipe = edit_recipe();
    recipe
        .definitions
        .get_mut(&PLATE)
        .expect("plate exists")
        .regions
        .insert(
            RegionId(1),
            SurfaceRegionSpec {
                id: RegionId(1),
                name: "front".to_owned(),
                role: SurfaceRole::PrimarySurface,
                tags: BTreeSet::new(),
            },
        );
    let cut = ModelingOperationSpec::RectangularThroughCut {
        operation: OperationId(10),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.0, 0.0],
        size: [0.16, 0.08],
        corner_radius: 0.0,
        rim_width: 0.02,
        corner_segments: 1,
        entry_loop: BoundaryLoopId(11),
        exit_loop: BoundaryLoopId(12),
        outer_region: RegionId(1),
        rim_region: RegionId(40),
        wall_region: RegionId(41),
        edge_treatment: CutEdgeTreatment::Hard,
    };

    let outcome = apply_edit_program(
        &recipe,
        &AssetEditProgram {
            label: "operations".to_owned(),
            seed: 80,
            operations: vec![
                AssetEdit::InsertModelingOperation {
                    definition: PLATE,
                    index: 0,
                    operation: cut,
                },
                AssetEdit::DuplicateCutOperation {
                    definition: PLATE,
                    source: OperationId(10),
                    operation: OperationId(13),
                    entry_loop: BoundaryLoopId(14),
                    secondary_loop: BoundaryLoopId(15),
                    rim_region: RegionId(42),
                    wall_region: RegionId(43),
                    floor_region: None,
                    center_offset: [0.24, 0.0],
                },
                AssetEdit::MoveModelingOperation {
                    definition: PLATE,
                    operation: OperationId(13),
                    new_index: 0,
                },
                AssetEdit::RemoveModelingOperation {
                    definition: PLATE,
                    operation: OperationId(10),
                },
            ],
        },
    )
    .expect("structural operation edits should apply");

    let operations = &outcome.definitions[&PLATE].geometry.operations;
    assert_eq!(operations.len(), 1);
    let ModelingOperationSpec::RectangularThroughCut {
        operation,
        center,
        entry_loop,
        exit_loop,
        rim_region,
        wall_region,
        ..
    } = &operations[0]
    else {
        panic!("duplicate should preserve cut kind");
    };
    assert_eq!(*operation, OperationId(13));
    assert_eq!(*center, [0.24, 0.0]);
    assert_eq!(*entry_loop, BoundaryLoopId(14));
    assert_eq!(*exit_loop, BoundaryLoopId(15));
    assert_eq!(*rim_region, RegionId(42));
    assert_eq!(*wall_region, RegionId(43));
    assert_eq!(outcome.next_ids.operation, 14);
    assert_eq!(outcome.next_ids.boundary_loop, 16);
    assert_eq!(outcome.next_ids.region, 44);
}

#[test]
fn structural_modeling_operation_edits_respect_topology_locks() {
    let mut recipe = edit_recipe();
    recipe.topology_locks.insert(PLATE);
    let cut = ModelingOperationSpec::RectangularThroughCut {
        operation: OperationId(10),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.0, 0.0],
        size: [0.16, 0.08],
        corner_radius: 0.0,
        rim_width: 0.02,
        corner_segments: 1,
        entry_loop: BoundaryLoopId(11),
        exit_loop: BoundaryLoopId(12),
        outer_region: RegionId(1),
        rim_region: RegionId(40),
        wall_region: RegionId(41),
        edge_treatment: CutEdgeTreatment::Hard,
    };

    assert!(matches!(
        apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "locked".to_owned(),
                seed: 81,
                operations: vec![AssetEdit::InsertModelingOperation {
                    definition: PLATE,
                    index: 0,
                    operation: cut,
                }],
            },
        ),
        Err(AssetError::LockedTopology(PLATE))
    ));
}

#[test]
fn unrelated_parameter_edit_preserves_semantic_ids() {
    let recipe = edit_recipe();
    let definition_ids = recipe.definitions.keys().copied().collect::<Vec<_>>();
    let instance_ids = recipe.instances.keys().copied().collect::<Vec<_>>();
    let operation_ids = recipe.definitions[&BODY]
        .geometry
        .operations
        .iter()
        .map(ModelingOperationSpec::operation_id)
        .collect::<Vec<_>>();
    let next_ids = recipe.next_ids.clone();

    let edited = apply_edit_program(
        &recipe,
        &AssetEditProgram {
            label: "radius".to_owned(),
            seed: 11,
            operations: vec![AssetEdit::SetScalar {
                parameter: BODY_RADIUS,
                value: 0.25,
            }],
        },
    )
    .expect("scalar edit should apply");

    assert_eq!(
        edited.definitions.keys().copied().collect::<Vec<_>>(),
        definition_ids
    );
    assert_eq!(
        edited.instances.keys().copied().collect::<Vec<_>>(),
        instance_ids
    );
    assert_eq!(
        edited.definitions[&BODY]
            .geometry
            .operations
            .iter()
            .map(ModelingOperationSpec::operation_id)
            .collect::<Vec<_>>(),
        operation_ids
    );
    assert_eq!(edited.next_ids, next_ids);
}

#[test]
fn deterministic_edit_report_is_stable() {
    let recipe = edit_recipe();
    let program = AssetEditProgram {
        label: "report".to_owned(),
        seed: 12,
        operations: vec![
            AssetEdit::SetScalar {
                parameter: BODY_RADIUS,
                value: 0.2,
            },
            AssetEdit::SetArraySpacing {
                definition: BODY,
                operation: RADIAL_ARRAY,
                spacing: ArraySpacingEdit::RadialAxis([0.0, 0.0, 1.0]),
            },
        ],
    };

    let first = apply_edit_program_with_report(&recipe, &program)
        .expect("first report should apply")
        .report;
    let second = apply_edit_program_with_report(&recipe, &program)
        .expect("second report should apply")
        .report;

    assert_eq!(first, second);
    assert_eq!(first.entries[0].edit_type, "SetScalar");
    assert_eq!(first.entries[1].subject, Some("definition.1".to_owned()));
}

#[test]
fn serde_round_trip_preserves_edit_program_and_report() {
    let recipe = edit_recipe();
    let program = AssetEditProgram {
        label: "serde".to_owned(),
        seed: 13,
        operations: vec![
            AssetEdit::SetGeneratorDimension {
                definition: LATHE,
                dimension: GeneratorDimensionEdit::LatheSegments(24),
            },
            AssetEdit::MirrorInstance {
                source: WHEEL_INSTANCE,
                instance: PartInstanceId(3),
                plane: MirrorInstanceSpec {
                    plane_normal: [1.0, 0.0, 0.0],
                    plane_offset: 0.0,
                },
                name: None,
            },
        ],
    };

    let json = serde_json::to_string(&program).expect("program serializes");
    let round_tripped: AssetEditProgram = serde_json::from_str(&json).expect("program parses");
    assert_eq!(program, round_tripped);

    let report = apply_edit_program_with_report(&recipe, &program)
        .expect("program applies")
        .report;
    let report_json = serde_json::to_string(&report).expect("report serializes");
    let report_round_trip: AssetEditReport =
        serde_json::from_str(&report_json).expect("report parses");
    assert_eq!(report, report_round_trip);
}

#[test]
fn legacy_cut_boundary_loop_migrates_to_distinct_physical_loops() {
    let json = r#"{
        "schema_version": 3,
        "id": 42,
        "title": "legacy cut",
        "definitions": {
            "1": {
                "id": 1,
                "name": "plate",
                "tags": [],
                "geometry": {
                    "source": { "Plate": { "size": [1.0, 1.0], "thickness": 0.1 } },
                    "operations": [
                        { "RecessedPanelCut": {
                            "operation": 5,
                            "region": 1,
                            "face": "PositiveY",
                            "center": [0.0, 0.0],
                            "size": [0.4, 0.3],
                            "depth": 0.03,
                            "corner_radius": 0.02,
                            "boundary_loop": 7,
                            "outer_region": 1,
                            "rim_region": 20,
                            "wall_region": 21,
                            "floor_region": 22,
                            "edge_treatment": "BevelEligible"
                        } }
                    ]
                },
                "regions": {
                    "1": { "id": 1, "name": "front", "role": "PrimarySurface", "tags": [] },
                    "2": { "id": 2, "name": "back", "role": "PrimarySurface", "tags": [] },
                    "3": { "id": 3, "name": "side", "role": "Side", "tags": [] }
                },
                "sockets": {},
                "local_pivot": {
                    "origin": [0.0, 0.0, 0.0],
                    "x_axis": [1.0, 0.0, 0.0],
                    "y_axis": [0.0, 1.0, 0.0],
                    "z_axis": [0.0, 0.0, 1.0]
                },
                "variant_group": null,
                "production_hints": null
            }
        },
        "instances": {
            "1": {
                "id": 1,
                "definition": 1,
                "name": "plate",
                "parent": null,
                "local_transform": {
                    "translation": [0.0, 0.0, 0.0],
                    "rotation_degrees": [0.0, 0.0, 0.0],
                    "scale": [1.0, 1.0, 1.0]
                },
                "attachment": null,
                "enabled": true,
                "tags": [],
                "generated_by": null
            }
        },
        "root_instances": [1],
        "parameters": {},
        "locks": [],
        "instance_locks": [],
        "subtree_locks": [],
        "topology_locks": [],
        "constraints": [],
        "relationships": [],
        "variation": {
            "optional_instances": [],
            "replacement_groups": {},
            "count_ranges": {},
            "parameter_range_overrides": {}
        },
        "next_ids": {
            "part_definition": 2,
            "part_instance": 2,
            "operation": 6,
            "region": 23,
            "boundary_loop": 8,
            "socket": 1,
            "parameter": 1,
            "revision": 1
        }
    }"#;

    let recipe: AssetRecipe = serde_json::from_str(json).expect("legacy recipe parses");

    assert_eq!(recipe.schema_version, ASSET_RECIPE_SCHEMA_VERSION);
    assert_eq!(
        recipe.definitions[&PartDefinitionId(1)].geometry.operations[0].boundary_loop_ids(),
        vec![BoundaryLoopId(7), BoundaryLoopId(8)]
    );
    assert_eq!(recipe.next_ids.boundary_loop, 9);
    assert!(validate_asset_recipe(&recipe).is_valid());
}

#[test]
fn schema_four_cut_fields_migrate_to_schema_five() {
    let recipe: AssetRecipe = serde_json::from_value(minimal_cut_recipe_value(
        4,
        serde_json::json!({
            "RecessedPanelCut": {
                "operation": 5,
                "region": 1,
                "face": "PositiveY",
                "center": [0.0, 0.0],
                "size": [0.4, 0.3],
                "depth": 0.03,
                "corner_radius": 0.02,
                "entry_loop": 7,
                "floor_loop": 8,
                "outer_region": 1,
                "rim_region": 20,
                "wall_region": 21,
                "floor_region": 22,
                "edge_treatment": "BevelEligible"
            }
        }),
    ))
    .expect("schema 4 recipe should migrate missing explicit cut fields");

    assert_eq!(recipe.schema_version, ASSET_RECIPE_SCHEMA_VERSION);
    let ModelingOperationSpec::RecessedPanelCut {
        rim_width,
        corner_segments,
        ..
    } = &recipe.definitions[&PartDefinitionId(1)].geometry.operations[0]
    else {
        panic!("schema 4 fixture should contain a recessed cut");
    };
    assert_eq!(*rim_width, 0.048);
    assert_eq!(*corner_segments, 4);
    assert!(validate_asset_recipe(&recipe).is_valid());
}

#[test]
fn schema_five_cut_fields_are_required() {
    let error = serde_json::from_value::<AssetRecipe>(minimal_cut_recipe_value(
        5,
        serde_json::json!({
            "RecessedPanelCut": {
                "operation": 5,
                "region": 1,
                "face": "PositiveY",
                "center": [0.0, 0.0],
                "size": [0.4, 0.3],
                "depth": 0.03,
                "corner_radius": 0.02,
                "entry_loop": 7,
                "floor_loop": 8,
                "outer_region": 1,
                "rim_region": 20,
                "wall_region": 21,
                "floor_region": 22,
                "edge_treatment": "BevelEligible"
            }
        }),
    ))
    .expect_err("schema 5 recipe should require explicit cut fields");

    assert!(
        error.to_string().contains("RecessedPanelCut.rim_width"),
        "unexpected error: {error}"
    );
}

#[test]
fn schema_zero_is_not_migrated_to_current_schema() {
    let mut recipe = edit_recipe();
    recipe.schema_version = 0;
    let json = serde_json::to_string(&recipe).expect("recipe serializes");
    let parsed: AssetRecipe = serde_json::from_str(&json).expect("recipe parses");

    assert_eq!(parsed.schema_version, 0);
    assert!(
        validate_asset_recipe(&parsed)
            .issues
            .iter()
            .any(|issue| issue.code == "unsupported_schema_version")
    );
}

#[test]
fn beginner_parameter_reflection_uses_expected_groups_and_hides_raw_controls() {
    let mut recipe = edit_recipe();
    recipe.parameters.insert(
        ParameterId(99),
        ParameterDescriptor {
            id: ParameterId(99),
            path: "definition.99.geometry.literal_mesh.positions.0.x".to_owned(),
            label: "Raw Vertex X".to_owned(),
            group: "Size".to_owned(),
            minimum: -1.0,
            maximum: 1.0,
            step: 0.1,
            mutation_sigma: 0.0,
            topology_changing: true,
            beginner_description: "Raw".to_owned(),
        },
    );

    let reflected = reflect_beginner_parameters(&recipe);
    let labels = reflected
        .iter()
        .map(|group| group.label.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        labels,
        vec![
            "Size",
            "Proportions",
            "Placement",
            "Curvature",
            "Edge Softness",
            "Repetition",
            "Part Presence",
            "Detail Density",
        ]
    );
    assert!(reflected.iter().all(|group| {
        group
            .parameters
            .iter()
            .all(|parameter| !parameter.label.contains("Raw Vertex"))
    }));
    let presence = reflected
        .iter()
        .find(|group| group.group == BeginnerParameterGroup::PartPresence)
        .expect("presence group exists");
    assert_eq!(presence.part_presence[0].instance, WHEEL_INSTANCE);
}
