use std::collections::{BTreeMap, BTreeSet};

use orchard_asset::{
    AssetId, AssetRecipe, BoundaryLoopDependencyMode, BoundaryLoopId, CountRangeHint,
    CutEdgeTreatment, CutGroupRole, Frame3, GeometryRecipe, GeometrySource, ModelingOperationSpec,
    OperationId, ParameterId, PartDefinition, PartDefinitionId, PartInstance, PartInstanceId,
    PlanarCutFace, RegionId, SemanticCutGroupHint, SocketId, SocketSpec, SurfaceRegionSpec,
    SurfaceRole, Transform3, definition_scalar_path,
};
use orchard_family_compile::remap::FragmentRemapError;
use orchard_family_compile::remap::ids::prepare_fragment_id_remap;
use orchard_family_compile::remap::operations::{
    remap_boundary_loop_reference, remap_definition_reference, remap_generated_by,
    remap_instance_reference, remap_modeling_operation, remap_modeling_operations,
    remap_operation_count_ranges, remap_region_reference, remap_semantic_cut_group_hint,
    remap_socket_reference, unsupported_operation_remap,
};
use orchard_family_compile::{
    RECIPE_FRAGMENT_SCHEMA_VERSION, RecipeFragment, RecipeFragmentExports, scalar_parameter,
};

const FRAGMENT_ID: &str = "modeling_ops";
const SOURCE_DEFINITION: PartDefinitionId = PartDefinitionId(90);
const SOURCE_INSTANCE: PartInstanceId = PartInstanceId(91);
const GENERATED_INSTANCE: PartInstanceId = PartInstanceId(92);
const SOURCE_PARAMETER: ParameterId = ParameterId(3);
const HOST_REGION: RegionId = RegionId(10);
const SOURCE_SOCKET: SocketId = SocketId(7);

const ADD_PANEL_OP: OperationId = OperationId(10);
const ADD_TRIM_OP: OperationId = OperationId(11);
const RECESSED_CUT_OP: OperationId = OperationId(12);
const RECTANGULAR_CUT_OP: OperationId = OperationId(13);
const CIRCULAR_CUT_OP: OperationId = OperationId(14);
const RESERVED_BOOLEAN_OP: OperationId = OperationId(15);
const BEVEL_PROFILE_OP: OperationId = OperationId(16);
const BEVEL_LOOP_OP: OperationId = OperationId(17);
const TRANSFORM_OP: OperationId = OperationId(18);
const RESERVED_DEFORM_OP: OperationId = OperationId(19);
const MIRROR_OP: OperationId = OperationId(20);
const LINEAR_ARRAY_OP: OperationId = OperationId(21);
const RADIAL_ARRAY_OP: OperationId = OperationId(22);

const RECESSED_ENTRY_LOOP: BoundaryLoopId = BoundaryLoopId(30);
const RECESSED_FLOOR_LOOP: BoundaryLoopId = BoundaryLoopId(31);
const RECT_ENTRY_LOOP: BoundaryLoopId = BoundaryLoopId(32);
const RECT_EXIT_LOOP: BoundaryLoopId = BoundaryLoopId(33);
const CIRCLE_ENTRY_LOOP: BoundaryLoopId = BoundaryLoopId(34);
const CIRCLE_EXIT_LOOP: BoundaryLoopId = BoundaryLoopId(35);
const BEVEL_OUTER_LOOP: BoundaryLoopId = BoundaryLoopId(36);
const BEVEL_INNER_LOOP: BoundaryLoopId = BoundaryLoopId(37);

const RECESSED_RIM_REGION: RegionId = RegionId(20);
const RECESSED_WALL_REGION: RegionId = RegionId(21);
const RECESSED_FLOOR_REGION: RegionId = RegionId(22);
const RECT_RIM_REGION: RegionId = RegionId(23);
const RECT_WALL_REGION: RegionId = RegionId(24);
const CIRCLE_RIM_REGION: RegionId = RegionId(25);
const CIRCLE_WALL_REGION: RegionId = RegionId(26);
const BEVEL_REGION: RegionId = RegionId(27);

#[test]
fn allocation_is_deterministic_and_avoids_populated_target_collisions() {
    let fragment = source_fragment();
    let mut first_target = populated_target_with_stale_counters();
    let first = prepare_fragment_id_remap(&mut first_target, &fragment);
    let mut second_target = populated_target_with_stale_counters();
    let second = prepare_fragment_id_remap(&mut second_target, &fragment);

    assert_eq!(first, second);
    assert_eq!(
        first.remap.definitions[&SOURCE_DEFINITION],
        PartDefinitionId(2)
    );
    assert_eq!(first.remap.instances[&SOURCE_INSTANCE], PartInstanceId(2));
    assert_eq!(
        first.remap.instances[&GENERATED_INSTANCE],
        PartInstanceId(3)
    );
    assert_eq!(first.remap.parameters[&SOURCE_PARAMETER], ParameterId(2));
    assert_eq!(first.remap.operations[&ADD_PANEL_OP], OperationId(2));
    assert_eq!(first.remap.regions[&HOST_REGION], RegionId(4));
    assert_eq!(
        first.remap.boundary_loops[&RECESSED_ENTRY_LOOP],
        BoundaryLoopId(3)
    );
    assert_eq!(first.remap.sockets[&SOURCE_SOCKET], SocketId(8));
    assert_eq!(first.allocated.operations.len(), source_operations().len());
    assert_eq!(first.allocated.regions.len(), 9);
    assert_eq!(first.allocated.boundary_loops.len(), 8);
    assert!(
        first
            .allocated
            .operations
            .iter()
            .all(|operation| operation.0 > 1)
    );
    assert!(first.allocated.regions.iter().all(|region| region.0 > 3));
    assert!(
        first
            .allocated
            .boundary_loops
            .iter()
            .all(|boundary_loop| boundary_loop.0 > 2)
    );
    assert!(first.allocated.sockets.iter().all(|socket| socket.0 > 7));
}

#[test]
fn remaps_all_operation_variants_preserving_order_phase_and_dependencies() {
    let fragment = source_fragment();
    let mut target = populated_target_with_stale_counters();
    let prepared = prepare_fragment_id_remap(&mut target, &fragment);
    let source_operations = &fragment.recipe.definitions[&SOURCE_DEFINITION]
        .geometry
        .operations;
    let remapped =
        remap_modeling_operations(FRAGMENT_ID, source_operations, &prepared.remap).unwrap();

    assert_eq!(remapped.len(), source_operations.len());
    assert_eq!(
        remapped
            .iter()
            .map(ModelingOperationSpec::operation_id)
            .collect::<Vec<_>>(),
        prepared.allocated.operations
    );
    assert_eq!(
        remapped
            .iter()
            .map(ModelingOperationSpec::phase)
            .collect::<Vec<_>>(),
        source_operations
            .iter()
            .map(ModelingOperationSpec::phase)
            .collect::<Vec<_>>()
    );

    match &remapped[0] {
        ModelingOperationSpec::AddPanel {
            operation,
            region,
            inset,
            depth,
        } => {
            assert_eq!(*operation, prepared.remap.operations[&ADD_PANEL_OP]);
            assert_eq!(*region, prepared.remap.regions[&HOST_REGION]);
            assert_eq!((*inset, *depth), (0.08, 0.03));
        }
        other => panic!("expected AddPanel, got {other:?}"),
    }
    match &remapped[1] {
        ModelingOperationSpec::AddTrim {
            operation,
            region,
            width,
            height,
        } => {
            assert_eq!(*operation, prepared.remap.operations[&ADD_TRIM_OP]);
            assert_eq!(*region, prepared.remap.regions[&HOST_REGION]);
            assert_eq!((*width, *height), (0.02, 0.04));
        }
        other => panic!("expected AddTrim, got {other:?}"),
    }

    let recessed_entry = match &remapped[2] {
        ModelingOperationSpec::RecessedPanelCut {
            operation,
            region,
            entry_loop,
            floor_loop,
            outer_region,
            rim_region,
            wall_region,
            floor_region,
            edge_treatment,
            ..
        } => {
            assert_eq!(*operation, prepared.remap.operations[&RECESSED_CUT_OP]);
            assert_eq!(*region, prepared.remap.regions[&HOST_REGION]);
            assert_eq!(
                (*entry_loop, *floor_loop),
                (
                    prepared.remap.boundary_loops[&RECESSED_ENTRY_LOOP],
                    prepared.remap.boundary_loops[&RECESSED_FLOOR_LOOP]
                )
            );
            assert_eq!(*outer_region, prepared.remap.regions[&HOST_REGION]);
            assert_eq!(*rim_region, prepared.remap.regions[&RECESSED_RIM_REGION]);
            assert_eq!(*wall_region, prepared.remap.regions[&RECESSED_WALL_REGION]);
            assert_eq!(
                *floor_region,
                prepared.remap.regions[&RECESSED_FLOOR_REGION]
            );
            assert_eq!(*edge_treatment, CutEdgeTreatment::BevelEligible);
            *entry_loop
        }
        other => panic!("expected RecessedPanelCut, got {other:?}"),
    };
    match &remapped[3] {
        ModelingOperationSpec::RectangularThroughCut {
            operation,
            entry_loop,
            exit_loop,
            rim_region,
            wall_region,
            ..
        } => {
            assert_eq!(*operation, prepared.remap.operations[&RECTANGULAR_CUT_OP]);
            assert_eq!(
                (*entry_loop, *exit_loop),
                (
                    prepared.remap.boundary_loops[&RECT_ENTRY_LOOP],
                    prepared.remap.boundary_loops[&RECT_EXIT_LOOP]
                )
            );
            assert_eq!(*rim_region, prepared.remap.regions[&RECT_RIM_REGION]);
            assert_eq!(*wall_region, prepared.remap.regions[&RECT_WALL_REGION]);
        }
        other => panic!("expected RectangularThroughCut, got {other:?}"),
    }
    match &remapped[4] {
        ModelingOperationSpec::CircularThroughCut {
            operation,
            entry_loop,
            exit_loop,
            rim_region,
            wall_region,
            ..
        } => {
            assert_eq!(*operation, prepared.remap.operations[&CIRCULAR_CUT_OP]);
            assert_eq!(
                (*entry_loop, *exit_loop),
                (
                    prepared.remap.boundary_loops[&CIRCLE_ENTRY_LOOP],
                    prepared.remap.boundary_loops[&CIRCLE_EXIT_LOOP]
                )
            );
            assert_eq!(*rim_region, prepared.remap.regions[&CIRCLE_RIM_REGION]);
            assert_eq!(*wall_region, prepared.remap.regions[&CIRCLE_WALL_REGION]);
        }
        other => panic!("expected CircularThroughCut, got {other:?}"),
    }
    match &remapped[5] {
        ModelingOperationSpec::ReservedBoolean { operation, label } => {
            assert_eq!(*operation, prepared.remap.operations[&RESERVED_BOOLEAN_OP]);
            assert_eq!(label, "future boolean");
        }
        other => panic!("expected ReservedBoolean, got {other:?}"),
    }
    match &remapped[6] {
        ModelingOperationSpec::SetBevelProfile {
            operation,
            radius,
            segments,
        } => {
            assert_eq!(*operation, prepared.remap.operations[&BEVEL_PROFILE_OP]);
            assert_eq!((*radius, *segments), (0.025, 3));
        }
        other => panic!("expected SetBevelProfile, got {other:?}"),
    }
    match &remapped[7] {
        ModelingOperationSpec::BevelBoundaryLoop {
            operation,
            target_loop,
            width,
            segments,
            profile,
            bevel_region,
            outer_replacement_loop,
            inner_replacement_loop,
        } => {
            assert_eq!(*operation, prepared.remap.operations[&BEVEL_LOOP_OP]);
            assert_eq!(*target_loop, recessed_entry);
            assert_eq!((*width, *segments, *profile), (0.018, 4, 1.35));
            assert_eq!(*bevel_region, prepared.remap.regions[&BEVEL_REGION]);
            assert_eq!(
                (*outer_replacement_loop, *inner_replacement_loop),
                (
                    prepared.remap.boundary_loops[&BEVEL_OUTER_LOOP],
                    prepared.remap.boundary_loops[&BEVEL_INNER_LOOP]
                )
            );
            let dependencies = remapped[7].boundary_loop_dependencies();
            assert_eq!(dependencies.len(), 1);
            assert_eq!(dependencies[0].input, recessed_entry);
            assert_eq!(dependencies[0].mode, BoundaryLoopDependencyMode::Consume);
            assert_eq!(
                dependencies[0].outputs,
                vec![
                    prepared.remap.boundary_loops[&BEVEL_OUTER_LOOP],
                    prepared.remap.boundary_loops[&BEVEL_INNER_LOOP]
                ]
            );
        }
        other => panic!("expected BevelBoundaryLoop, got {other:?}"),
    }
    match &remapped[8] {
        ModelingOperationSpec::TransformGeometry {
            operation,
            transform,
        } => {
            assert_eq!(*operation, prepared.remap.operations[&TRANSFORM_OP]);
            assert_eq!(transform.translation, [0.1, 0.2, 0.3]);
            assert_eq!(transform.scale, [1.0, 1.1, 0.9]);
        }
        other => panic!("expected TransformGeometry, got {other:?}"),
    }
    match &remapped[9] {
        ModelingOperationSpec::ReservedDeformationProgram { operation, label } => {
            assert_eq!(*operation, prepared.remap.operations[&RESERVED_DEFORM_OP]);
            assert_eq!(label, "future bend");
        }
        other => panic!("expected ReservedDeformationProgram, got {other:?}"),
    }
    match &remapped[10] {
        ModelingOperationSpec::MirrorInstances {
            operation,
            plane_normal,
            plane_offset,
        } => {
            assert_eq!(*operation, prepared.remap.operations[&MIRROR_OP]);
            assert_eq!((*plane_normal, *plane_offset), ([1.0, 0.0, 0.0], 0.25));
        }
        other => panic!("expected MirrorInstances, got {other:?}"),
    }
    match &remapped[11] {
        ModelingOperationSpec::LinearArray {
            operation,
            count,
            offset,
        } => {
            assert_eq!(*operation, prepared.remap.operations[&LINEAR_ARRAY_OP]);
            assert_eq!((*count, *offset), (5, [0.2, 0.0, 0.0]));
        }
        other => panic!("expected LinearArray, got {other:?}"),
    }
    match &remapped[12] {
        ModelingOperationSpec::RadialArray {
            operation,
            count,
            axis,
            angle_degrees,
        } => {
            assert_eq!(*operation, prepared.remap.operations[&RADIAL_ARRAY_OP]);
            assert_eq!((*count, *axis, *angle_degrees), (6, [0.0, 1.0, 0.0], 180.0));
        }
        other => panic!("expected RadialArray, got {other:?}"),
    }
}

#[test]
fn reference_helpers_remap_cut_groups_ranges_sockets_and_generated_provenance() {
    let fragment = source_fragment();
    let mut target = populated_target_with_stale_counters();
    let prepared = prepare_fragment_id_remap(&mut target, &fragment);

    assert_eq!(
        remap_definition_reference(FRAGMENT_ID, SOURCE_DEFINITION, &prepared.remap).unwrap(),
        prepared.remap.definitions[&SOURCE_DEFINITION]
    );
    assert_eq!(
        remap_instance_reference(FRAGMENT_ID, GENERATED_INSTANCE, &prepared.remap).unwrap(),
        prepared.remap.instances[&GENERATED_INSTANCE]
    );
    assert_eq!(
        remap_socket_reference(FRAGMENT_ID, SOURCE_SOCKET, &prepared.remap).unwrap(),
        prepared.remap.sockets[&SOURCE_SOCKET]
    );
    assert_eq!(
        remap_region_reference(FRAGMENT_ID, ADD_PANEL_OP, HOST_REGION, &prepared.remap).unwrap(),
        prepared.remap.regions[&HOST_REGION]
    );
    assert_eq!(
        remap_boundary_loop_reference(
            FRAGMENT_ID,
            BEVEL_LOOP_OP,
            RECESSED_ENTRY_LOOP,
            &prepared.remap,
        )
        .unwrap(),
        prepared.remap.boundary_loops[&RECESSED_ENTRY_LOOP]
    );
    assert_eq!(
        remap_generated_by(FRAGMENT_ID, Some(MIRROR_OP), &prepared.remap).unwrap(),
        Some(prepared.remap.operations[&MIRROR_OP])
    );
    assert_eq!(
        remap_generated_by(FRAGMENT_ID, None, &prepared.remap).unwrap(),
        None
    );

    let count_ranges = BTreeMap::from([
        (
            LINEAR_ARRAY_OP,
            CountRangeHint {
                minimum: 2,
                maximum: 8,
            },
        ),
        (
            RADIAL_ARRAY_OP,
            CountRangeHint {
                minimum: 3,
                maximum: 12,
            },
        ),
    ]);
    let remapped_ranges =
        remap_operation_count_ranges(FRAGMENT_ID, &count_ranges, &prepared.remap).unwrap();
    assert_eq!(
        remapped_ranges[&prepared.remap.operations[&LINEAR_ARRAY_OP]],
        count_ranges[&LINEAR_ARRAY_OP]
    );
    assert_eq!(
        remapped_ranges[&prepared.remap.operations[&RADIAL_ARRAY_OP]],
        count_ranges[&RADIAL_ARRAY_OP]
    );

    let cut_group = SemanticCutGroupHint {
        label: "edge_mark cuts".to_owned(),
        definition: SOURCE_DEFINITION,
        operations: vec![RECESSED_CUT_OP, RECTANGULAR_CUT_OP, CIRCULAR_CUT_OP],
        role: CutGroupRole::Vents,
        count_range: Some(CountRangeHint {
            minimum: 1,
            maximum: 4,
        }),
    };
    let remapped_group =
        remap_semantic_cut_group_hint(FRAGMENT_ID, &cut_group, &prepared.remap).unwrap();
    assert_eq!(
        remapped_group.definition,
        prepared.remap.definitions[&SOURCE_DEFINITION]
    );
    assert_eq!(
        remapped_group.operations,
        vec![
            prepared.remap.operations[&RECESSED_CUT_OP],
            prepared.remap.operations[&RECTANGULAR_CUT_OP],
            prepared.remap.operations[&CIRCULAR_CUT_OP]
        ]
    );
    assert_eq!(remapped_group.role, CutGroupRole::Vents);
    assert_eq!(remapped_group.count_range, cut_group.count_range);
}

#[test]
fn missing_reference_errors_include_fragment_and_operation_context() {
    let mut remap = orchard_family_compile::remap::FragmentRemap::default();
    remap.operations.insert(ADD_PANEL_OP, OperationId(100));
    let error = remap_modeling_operation(
        FRAGMENT_ID,
        &ModelingOperationSpec::AddPanel {
            operation: ADD_PANEL_OP,
            region: HOST_REGION,
            inset: 0.1,
            depth: 0.2,
        },
        &remap,
    )
    .expect_err("missing region remap should fail");

    match error {
        FragmentRemapError::MissingMapping {
            fragment,
            id_kind,
            id,
        } => {
            assert_eq!(fragment, FRAGMENT_ID);
            assert!(id_kind.contains("region"));
            assert!(id_kind.contains("operation 10"));
            assert_eq!(id, HOST_REGION.0.to_string());
        }
        other => panic!("expected MissingMapping, got {other:?}"),
    }

    let error = remap_modeling_operation(
        FRAGMENT_ID,
        &ModelingOperationSpec::ReservedBoolean {
            operation: RESERVED_BOOLEAN_OP,
            label: "future boolean".to_owned(),
        },
        &orchard_family_compile::remap::FragmentRemap::default(),
    )
    .expect_err("missing operation remap should fail");
    match error {
        FragmentRemapError::MissingMapping {
            fragment,
            id_kind,
            id,
        } => {
            assert_eq!(fragment, FRAGMENT_ID);
            assert!(id_kind.contains("operation"));
            assert!(id_kind.contains("operation 15"));
            assert_eq!(id, RESERVED_BOOLEAN_OP.0.to_string());
        }
        other => panic!("expected MissingMapping, got {other:?}"),
    }

    match unsupported_operation_remap(FRAGMENT_ID, "future ModelingOperationSpec variant") {
        FragmentRemapError::Unsupported {
            fragment,
            stage,
            reason,
        } => {
            assert_eq!(fragment, FRAGMENT_ID);
            assert_eq!(stage, "operations");
            assert!(reason.contains("future"));
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

fn source_fragment() -> RecipeFragment {
    let mut recipe = AssetRecipe::new(AssetId(1), "operation fragment");
    recipe.definitions.insert(
        SOURCE_DEFINITION,
        PartDefinition {
            id: SOURCE_DEFINITION,
            name: "operation source".to_owned(),
            tags: BTreeSet::from(["source".to_owned()]),
            geometry: GeometryRecipe {
                source: GeometrySource::RoundedBox {
                    half_extents: [0.5, 0.4, 0.3],
                    radius: 0.03,
                },
                operations: source_operations(),
            },
            regions: BTreeMap::from([(HOST_REGION, region(HOST_REGION, "host"))]),
            sockets: BTreeMap::from([(SOURCE_SOCKET, socket(SOURCE_SOCKET, "mount"))]),
            local_pivot: Frame3::default(),
            variant_group: None,
            production_hints: None,
        },
    );
    recipe.instances.insert(
        SOURCE_INSTANCE,
        PartInstance {
            id: SOURCE_INSTANCE,
            definition: SOURCE_DEFINITION,
            name: "source root".to_owned(),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::from(["source".to_owned()]),
            generated_by: None,
        },
    );
    recipe.instances.insert(
        GENERATED_INSTANCE,
        PartInstance {
            id: GENERATED_INSTANCE,
            definition: SOURCE_DEFINITION,
            name: "linear occurrence".to_owned(),
            parent: Some(SOURCE_INSTANCE),
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::from(["generated".to_owned()]),
            generated_by: Some(LINEAR_ARRAY_OP),
        },
    );
    recipe.root_instances.push(SOURCE_INSTANCE);
    recipe.parameters.insert(
        SOURCE_PARAMETER,
        scalar_parameter(
            SOURCE_PARAMETER.0,
            definition_scalar_path(SOURCE_DEFINITION, "geometry.rounded_box.radius"),
            "radius",
            0.0,
            0.2,
            0.01,
            false,
        ),
    );
    recipe.next_ids.part_definition = SOURCE_DEFINITION.0 + 1;
    recipe.next_ids.part_instance = GENERATED_INSTANCE.0 + 1;
    recipe.next_ids.parameter = SOURCE_PARAMETER.0 + 1;
    recipe.next_ids.operation = RADIAL_ARRAY_OP.0 + 1;
    recipe.next_ids.region = BEVEL_REGION.0 + 1;
    recipe.next_ids.boundary_loop = BEVEL_INNER_LOOP.0 + 1;
    recipe.next_ids.socket = SOURCE_SOCKET.0 + 1;

    RecipeFragment {
        schema_version: RECIPE_FRAGMENT_SCHEMA_VERSION,
        id: FRAGMENT_ID.to_owned(),
        provided_role: "source".to_owned(),
        exports: RecipeFragmentExports {
            role_occurrence_roots: vec![SOURCE_INSTANCE],
            internal_roots: Vec::new(),
            socket_ports: Vec::new(),
            surface_ports: Vec::new(),
        },
        recipe,
    }
}

fn populated_target_with_stale_counters() -> AssetRecipe {
    let mut target = AssetRecipe::new(AssetId(2), "populated target");
    target.definitions.insert(
        PartDefinitionId(1),
        PartDefinition {
            id: PartDefinitionId(1),
            name: "occupied definition".to_owned(),
            tags: BTreeSet::from(["occupied".to_owned()]),
            geometry: GeometryRecipe {
                source: GeometrySource::RoundedBox {
                    half_extents: [0.2, 0.2, 0.2],
                    radius: 0.01,
                },
                operations: vec![ModelingOperationSpec::RectangularThroughCut {
                    operation: OperationId(1),
                    region: RegionId(1),
                    face: PlanarCutFace::PositiveX,
                    center: [0.0, 0.0],
                    size: [0.1, 0.1],
                    corner_radius: 0.0,
                    rim_width: 0.01,
                    corner_segments: 1,
                    entry_loop: BoundaryLoopId(1),
                    exit_loop: BoundaryLoopId(2),
                    outer_region: RegionId(1),
                    rim_region: RegionId(2),
                    wall_region: RegionId(3),
                    edge_treatment: CutEdgeTreatment::Hard,
                }],
            },
            regions: BTreeMap::from([(RegionId(1), region(RegionId(1), "occupied"))]),
            sockets: BTreeMap::from([(SocketId(1), socket(SocketId(1), "occupied"))]),
            local_pivot: Frame3::default(),
            variant_group: None,
            production_hints: None,
        },
    );
    target.instances.insert(
        PartInstanceId(1),
        PartInstance {
            id: PartInstanceId(1),
            definition: PartDefinitionId(1),
            name: "occupied instance".to_owned(),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::from(["occupied".to_owned()]),
            generated_by: None,
        },
    );
    target.root_instances.push(PartInstanceId(1));
    target.parameters.insert(
        ParameterId(1),
        scalar_parameter(
            1,
            definition_scalar_path(PartDefinitionId(1), "geometry.rounded_box.radius"),
            "occupied radius",
            0.0,
            0.2,
            0.01,
            false,
        ),
    );
    target
}

fn source_operations() -> Vec<ModelingOperationSpec> {
    vec![
        ModelingOperationSpec::AddPanel {
            operation: ADD_PANEL_OP,
            region: HOST_REGION,
            inset: 0.08,
            depth: 0.03,
        },
        ModelingOperationSpec::AddTrim {
            operation: ADD_TRIM_OP,
            region: HOST_REGION,
            width: 0.02,
            height: 0.04,
        },
        ModelingOperationSpec::RecessedPanelCut {
            operation: RECESSED_CUT_OP,
            region: HOST_REGION,
            face: PlanarCutFace::PositiveX,
            center: [0.0, 0.1],
            size: [0.35, 0.2],
            depth: 0.04,
            corner_radius: 0.01,
            rim_width: 0.02,
            corner_segments: 3,
            entry_loop: RECESSED_ENTRY_LOOP,
            floor_loop: RECESSED_FLOOR_LOOP,
            outer_region: HOST_REGION,
            rim_region: RECESSED_RIM_REGION,
            wall_region: RECESSED_WALL_REGION,
            floor_region: RECESSED_FLOOR_REGION,
            edge_treatment: CutEdgeTreatment::BevelEligible,
        },
        ModelingOperationSpec::RectangularThroughCut {
            operation: RECTANGULAR_CUT_OP,
            region: HOST_REGION,
            face: PlanarCutFace::NegativeZ,
            center: [0.12, -0.08],
            size: [0.18, 0.12],
            corner_radius: 0.005,
            rim_width: 0.015,
            corner_segments: 2,
            entry_loop: RECT_ENTRY_LOOP,
            exit_loop: RECT_EXIT_LOOP,
            outer_region: HOST_REGION,
            rim_region: RECT_RIM_REGION,
            wall_region: RECT_WALL_REGION,
            edge_treatment: CutEdgeTreatment::Hard,
        },
        ModelingOperationSpec::CircularThroughCut {
            operation: CIRCULAR_CUT_OP,
            region: HOST_REGION,
            face: PlanarCutFace::PositiveY,
            center: [-0.1, 0.2],
            radius: 0.055,
            radial_segments: 16,
            rim_width: 0.012,
            entry_loop: CIRCLE_ENTRY_LOOP,
            exit_loop: CIRCLE_EXIT_LOOP,
            outer_region: HOST_REGION,
            rim_region: CIRCLE_RIM_REGION,
            wall_region: CIRCLE_WALL_REGION,
            edge_treatment: CutEdgeTreatment::BevelEligible,
        },
        ModelingOperationSpec::ReservedBoolean {
            operation: RESERVED_BOOLEAN_OP,
            label: "future boolean".to_owned(),
        },
        ModelingOperationSpec::SetBevelProfile {
            operation: BEVEL_PROFILE_OP,
            radius: 0.025,
            segments: 3,
        },
        ModelingOperationSpec::BevelBoundaryLoop {
            operation: BEVEL_LOOP_OP,
            target_loop: RECESSED_ENTRY_LOOP,
            width: 0.018,
            segments: 4,
            profile: 1.35,
            bevel_region: BEVEL_REGION,
            outer_replacement_loop: BEVEL_OUTER_LOOP,
            inner_replacement_loop: BEVEL_INNER_LOOP,
        },
        ModelingOperationSpec::TransformGeometry {
            operation: TRANSFORM_OP,
            transform: Transform3 {
                translation: [0.1, 0.2, 0.3],
                rotation_degrees: [5.0, 10.0, 15.0],
                scale: [1.0, 1.1, 0.9],
            },
        },
        ModelingOperationSpec::ReservedDeformationProgram {
            operation: RESERVED_DEFORM_OP,
            label: "future bend".to_owned(),
        },
        ModelingOperationSpec::MirrorInstances {
            operation: MIRROR_OP,
            plane_normal: [1.0, 0.0, 0.0],
            plane_offset: 0.25,
        },
        ModelingOperationSpec::LinearArray {
            operation: LINEAR_ARRAY_OP,
            count: 5,
            offset: [0.2, 0.0, 0.0],
        },
        ModelingOperationSpec::RadialArray {
            operation: RADIAL_ARRAY_OP,
            count: 6,
            axis: [0.0, 1.0, 0.0],
            angle_degrees: 180.0,
        },
    ]
}

fn region(id: RegionId, name: &str) -> SurfaceRegionSpec {
    SurfaceRegionSpec {
        id,
        name: name.to_owned(),
        role: SurfaceRole::PrimarySurface,
        tags: BTreeSet::from([name.to_owned()]),
    }
}

fn socket(id: SocketId, name: &str) -> SocketSpec {
    SocketSpec {
        id,
        name: name.to_owned(),
        local_frame: Frame3::default(),
        role: "mount".to_owned(),
        tags: BTreeSet::from([name.to_owned()]),
    }
}
