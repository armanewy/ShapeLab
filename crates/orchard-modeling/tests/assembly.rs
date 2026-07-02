use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use orchard_asset::{
    AssetId, AssetRecipe, AttachmentMode, AttachmentSpec, Frame3, GeneratedIdPolicy,
    GeometryRecipe, GeometrySource, OperationId, PartDefinition, PartDefinitionId, PartInstance,
    PartInstanceId, PatternAxis, PatternContract, PatternCountPolicy,
    PatternExportInstancingPolicy, PatternId, PatternType, RegionId, SocketId, SocketSpec,
    Transform3,
};
use orchard_modeling::assembly::{
    AssemblyError, AssemblyOperation, AssemblyPlan, LinearArrayOperation, MirrorOperation,
    MirrorPlane, RadialArrayOperation, evaluate_assembly_plan_with_generator,
    evaluate_assembly_with_generator, evaluate_pattern_contract,
};
use orchard_modeling::{GeneratedPart, GeneratorContext, GeometryGenerator, ModelingError};
use orchard_poly::{FaceMetadata, MeshBounds, PolygonMesh, polygon_mesh_from_faces};

const FACE_OPERATION: OperationId = OperationId(77);
const FACE_REGION: RegionId = RegionId(5);

#[test]
fn pattern_contract_evaluates_deterministically_in_assembly_layer() {
    let pattern = PatternContract {
        id: PatternId(4),
        pattern_type: PatternType::Linear,
        source_instance: Some(PartInstanceId(3)),
        count: Some(3),
        label: "Assembly repeat proof".to_owned(),
        count_policy: PatternCountPolicy::Exact(3),
        density_policy: None,
        export_instancing: PatternExportInstancingPolicy::Pending,
        linear_axis: Some(PatternAxis::Y),
        spacing: Some(0.4),
        generated_id_policy: GeneratedIdPolicy::PatternOccurrenceIndex,
    };

    let first = evaluate_pattern_contract(&pattern).expect("evaluates");
    let second = evaluate_pattern_contract(&pattern).expect("evaluates");

    assert_eq!(first, second);
    assert_eq!(first.report.generated_occurrence_count, 3);
    assert_eq!(first.occurrences[2].offset, [0.0, 0.8, 0.0]);
    assert!(!first.report.export_instancing_enabled);
}

#[derive(Default)]
struct FixtureGenerator {
    calls: RefCell<BTreeMap<PartDefinitionId, u32>>,
}

impl FixtureGenerator {
    fn call_count(&self, definition: PartDefinitionId) -> u32 {
        self.calls
            .borrow()
            .get(&definition)
            .copied()
            .unwrap_or_default()
    }
}

impl GeometryGenerator for FixtureGenerator {
    fn generate(
        &self,
        definition: &PartDefinition,
        context: &mut GeneratorContext,
    ) -> Result<GeneratedPart, ModelingError> {
        assert_eq!(context.part_definition, definition.id);
        *self.calls.borrow_mut().entry(definition.id).or_default() += 1;
        let mesh = triangle_mesh(definition.id);
        Ok(GeneratedPart {
            mesh,
            sockets: BTreeMap::new(),
            regions: BTreeMap::new(),
            local_bounds: MeshBounds::empty(),
            generator_signature: format!("fixture-definition-{}", definition.id.0),
        })
    }
}

#[test]
fn parent_child_socket_attachment_aligns_frames() {
    let parent_socket = SocketId(1);
    let child_socket = SocketId(2);
    let mut recipe = recipe_with([
        definition(PartDefinitionId(1), [(parent_socket, [1.0, 0.0, 0.0])]),
        definition(PartDefinitionId(2), [(child_socket, [0.0, 0.0, 0.0])]),
    ]);
    recipe.instances.insert(
        PartInstanceId(1),
        instance(
            PartInstanceId(1),
            PartDefinitionId(1),
            None,
            [2.0, 0.0, 0.0],
        ),
    );
    recipe.instances.insert(
        PartInstanceId(2),
        attached_instance(
            PartInstanceId(2),
            PartDefinitionId(2),
            PartInstanceId(1),
            parent_socket,
            child_socket,
            Transform3::default(),
            AttachmentMode::RigidSeparate,
        ),
    );
    recipe.root_instances.push(PartInstanceId(1));
    recipe.next_ids.part_instance = 3;

    let evaluation =
        evaluate_assembly_with_generator(&recipe, &FixtureGenerator::default()).expect("assemble");

    let parent_world_socket = evaluation.world_sockets[&PartInstanceId(1)][&parent_socket]
        .local_frame
        .origin;
    let child_world_socket = evaluation.world_sockets[&PartInstanceId(2)][&child_socket]
        .local_frame
        .origin;
    assert_close(parent_world_socket, [3.0, 0.0, 0.0]);
    assert_close(child_world_socket, parent_world_socket);
}

#[test]
fn attachment_offset_is_applied_after_socket_alignment() {
    let parent_socket = SocketId(1);
    let child_socket = SocketId(2);
    let mut recipe = recipe_with([
        definition(PartDefinitionId(1), [(parent_socket, [1.0, 0.0, 0.0])]),
        definition(PartDefinitionId(2), [(child_socket, [0.0, 0.0, 0.0])]),
    ]);
    recipe.instances.insert(
        PartInstanceId(1),
        instance(
            PartInstanceId(1),
            PartDefinitionId(1),
            None,
            [2.0, 0.0, 0.0],
        ),
    );
    recipe.instances.insert(
        PartInstanceId(2),
        attached_instance(
            PartInstanceId(2),
            PartDefinitionId(2),
            PartInstanceId(1),
            parent_socket,
            child_socket,
            Transform3 {
                translation: [0.0, 2.0, 0.0],
                ..Transform3::default()
            },
            AttachmentMode::RigidSeparate,
        ),
    );
    recipe.root_instances.push(PartInstanceId(1));
    recipe.next_ids.part_instance = 3;

    let evaluation =
        evaluate_assembly_with_generator(&recipe, &FixtureGenerator::default()).expect("assemble");

    assert_close(
        instance_origin(&evaluation, PartInstanceId(2)),
        [3.0, 2.0, 0.0],
    );
}

#[test]
fn nested_hierarchy_resolves_parent_transforms_before_attachments() {
    let parent_socket = SocketId(1);
    let child_socket = SocketId(2);
    let mut recipe = recipe_with([
        definition(PartDefinitionId(1), []),
        definition(PartDefinitionId(2), [(parent_socket, [0.0, 1.0, 0.0])]),
        definition(PartDefinitionId(3), [(child_socket, [0.0, 0.0, 0.0])]),
    ]);
    recipe.instances.insert(
        PartInstanceId(1),
        instance(
            PartInstanceId(1),
            PartDefinitionId(1),
            None,
            [1.0, 0.0, 0.0],
        ),
    );
    recipe.instances.insert(
        PartInstanceId(2),
        instance(
            PartInstanceId(2),
            PartDefinitionId(2),
            Some(PartInstanceId(1)),
            [0.0, 2.0, 0.0],
        ),
    );
    recipe.instances.insert(
        PartInstanceId(3),
        attached_instance(
            PartInstanceId(3),
            PartDefinitionId(3),
            PartInstanceId(2),
            parent_socket,
            child_socket,
            Transform3::default(),
            AttachmentMode::RigidSeparate,
        ),
    );
    recipe.root_instances.push(PartInstanceId(1));
    recipe.next_ids.part_instance = 4;

    let evaluation =
        evaluate_assembly_with_generator(&recipe, &FixtureGenerator::default()).expect("assemble");

    assert_close(
        instance_origin(&evaluation, PartInstanceId(3)),
        [1.0, 3.0, 0.0],
    );
}

#[test]
fn shared_definition_instances_reuse_one_local_part() {
    let generator = FixtureGenerator::default();
    let mut recipe = recipe_with([definition(PartDefinitionId(1), [])]);
    recipe.instances.insert(
        PartInstanceId(1),
        instance(
            PartInstanceId(1),
            PartDefinitionId(1),
            None,
            [0.0, 0.0, 0.0],
        ),
    );
    recipe.instances.insert(
        PartInstanceId(2),
        instance(
            PartInstanceId(2),
            PartDefinitionId(1),
            None,
            [5.0, 0.0, 0.0],
        ),
    );
    recipe
        .root_instances
        .extend([PartInstanceId(1), PartInstanceId(2)]);
    recipe.next_ids.part_instance = 3;

    let evaluation = evaluate_assembly_with_generator(&recipe, &generator).expect("assemble");

    assert_eq!(generator.call_count(PartDefinitionId(1)), 1);
    assert_eq!(evaluation.local_parts.len(), 1);
    assert_close(
        instance_origin(&evaluation, PartInstanceId(1)),
        [0.0, 0.0, 0.0],
    );
    assert_close(
        instance_origin(&evaluation, PartInstanceId(2)),
        [5.0, 0.0, 0.0],
    );
    assert_eq!(
        evaluation.world_meshes[&PartInstanceId(2)].face_metadata[0].part_instance,
        Some(PartInstanceId(2))
    );
}

#[test]
fn mirror_generates_copy_with_corrected_winding_and_provenance() {
    let mut recipe = recipe_with([definition(PartDefinitionId(1), [])]);
    recipe.instances.insert(
        PartInstanceId(1),
        instance(
            PartInstanceId(1),
            PartDefinitionId(1),
            None,
            [1.0, 0.0, 0.0],
        ),
    );
    recipe.root_instances.push(PartInstanceId(1));
    recipe.next_ids.part_instance = 2;
    let plan = AssemblyPlan {
        operations: vec![AssemblyOperation::Mirror(MirrorOperation {
            operation: OperationId(20),
            prototypes: vec![PartInstanceId(1)],
            plane: MirrorPlane {
                normal: [1.0, 0.0, 0.0],
                offset: 0.0,
            },
        })],
    };

    let evaluation =
        evaluate_assembly_plan_with_generator(&recipe, &plan, &FixtureGenerator::default())
            .expect("assemble");

    let mirrored = &evaluation.world_meshes[&PartInstanceId(2)];
    assert_eq!(
        mirrored.face_metadata[0].part_definition,
        Some(PartDefinitionId(1))
    );
    assert_eq!(
        mirrored.face_metadata[0].part_instance,
        Some(PartInstanceId(2))
    );
    assert_eq!(mirrored.face_metadata[0].operation, Some(FACE_OPERATION));
    assert_eq!(
        evaluation.instances[1].prototype_instance_id,
        Some(PartInstanceId(1))
    );
    assert_eq!(evaluation.instances[1].generated_by, Some(OperationId(20)));
    assert!(triangle_normal_z(mirrored) > 0.99);
    assert_close(
        instance_origin(&evaluation, PartInstanceId(2)),
        [-1.0, 0.0, 0.0],
    );
}

#[test]
fn linear_array_generates_deterministic_non_centered_copies() {
    let mut recipe = recipe_with([definition(PartDefinitionId(1), [])]);
    recipe.instances.insert(
        PartInstanceId(1),
        instance(
            PartInstanceId(1),
            PartDefinitionId(1),
            None,
            [1.0, 0.0, 0.0],
        ),
    );
    recipe.root_instances.push(PartInstanceId(1));
    recipe.next_ids.part_instance = 2;
    let plan = AssemblyPlan {
        operations: vec![AssemblyOperation::LinearArray(LinearArrayOperation {
            operation: OperationId(30),
            prototypes: vec![PartInstanceId(1)],
            count: 3,
            step: Transform3 {
                translation: [2.0, 0.0, 0.0],
                ..Transform3::default()
            },
            centered: false,
        })],
    };

    let evaluation =
        evaluate_assembly_plan_with_generator(&recipe, &plan, &FixtureGenerator::default())
            .expect("assemble");

    assert_eq!(
        evaluation.provenance.instance_order,
        vec![PartInstanceId(1), PartInstanceId(2), PartInstanceId(3)]
    );
    assert_close(
        instance_origin(&evaluation, PartInstanceId(2)),
        [3.0, 0.0, 0.0],
    );
    assert_close(
        instance_origin(&evaluation, PartInstanceId(3)),
        [5.0, 0.0, 0.0],
    );
    assert_eq!(
        evaluation.provenance.instances[1].generated_by,
        Some(OperationId(30))
    );
}

#[test]
fn centered_linear_array_places_generated_copies_around_the_prototype() {
    let mut recipe = recipe_with([definition(PartDefinitionId(1), [])]);
    recipe.instances.insert(
        PartInstanceId(1),
        instance(
            PartInstanceId(1),
            PartDefinitionId(1),
            None,
            [0.0, 0.0, 0.0],
        ),
    );
    recipe.root_instances.push(PartInstanceId(1));
    recipe.next_ids.part_instance = 2;
    let plan = AssemblyPlan {
        operations: vec![AssemblyOperation::LinearArray(LinearArrayOperation {
            operation: OperationId(31),
            prototypes: vec![PartInstanceId(1)],
            count: 3,
            step: Transform3 {
                translation: [2.0, 0.0, 0.0],
                ..Transform3::default()
            },
            centered: true,
        })],
    };

    let evaluation =
        evaluate_assembly_plan_with_generator(&recipe, &plan, &FixtureGenerator::default())
            .expect("assemble");

    assert_close(
        instance_origin(&evaluation, PartInstanceId(2)),
        [-2.0, 0.0, 0.0],
    );
    assert_close(
        instance_origin(&evaluation, PartInstanceId(3)),
        [2.0, 0.0, 0.0],
    );
}

#[test]
fn radial_array_generates_deterministic_oriented_copies() {
    let mut recipe = recipe_with([definition(PartDefinitionId(1), [])]);
    recipe.instances.insert(
        PartInstanceId(1),
        instance(
            PartInstanceId(1),
            PartDefinitionId(1),
            None,
            [1.0, 0.0, 0.0],
        ),
    );
    recipe.root_instances.push(PartInstanceId(1));
    recipe.next_ids.part_instance = 2;
    let plan = AssemblyPlan {
        operations: vec![AssemblyOperation::RadialArray(RadialArrayOperation {
            operation: OperationId(40),
            prototypes: vec![PartInstanceId(1)],
            count: 3,
            center: [0.0, 0.0, 0.0],
            axis: [0.0, 0.0, 1.0],
            angular_span_degrees: 180.0,
            rotate_instances: true,
        })],
    };

    let evaluation =
        evaluate_assembly_plan_with_generator(&recipe, &plan, &FixtureGenerator::default())
            .expect("assemble");

    assert_close(
        instance_origin(&evaluation, PartInstanceId(2)),
        [0.0, 1.0, 0.0],
    );
    assert_close(
        instance_origin(&evaluation, PartInstanceId(3)),
        [-1.0, 0.0, 0.0],
    );
    let rotated_x =
        evaluation.world_transforms[&PartInstanceId(2)].transform_vector([1.0, 0.0, 0.0]);
    assert_close(rotated_x, [0.0, 1.0, 0.0]);
}

#[test]
fn generated_ids_are_deterministic_across_evaluations() {
    let mut recipe = recipe_with([definition(PartDefinitionId(1), [])]);
    recipe.instances.insert(
        PartInstanceId(1),
        instance(
            PartInstanceId(1),
            PartDefinitionId(1),
            None,
            [0.0, 0.0, 0.0],
        ),
    );
    recipe.root_instances.push(PartInstanceId(1));
    recipe.next_ids.part_instance = 10;
    let plan = AssemblyPlan {
        operations: vec![
            AssemblyOperation::Mirror(MirrorOperation {
                operation: OperationId(50),
                prototypes: vec![PartInstanceId(1)],
                plane: MirrorPlane {
                    normal: [1.0, 0.0, 0.0],
                    offset: 0.0,
                },
            }),
            AssemblyOperation::LinearArray(LinearArrayOperation {
                operation: OperationId(51),
                prototypes: vec![PartInstanceId(1)],
                count: 2,
                step: Transform3 {
                    translation: [1.0, 0.0, 0.0],
                    ..Transform3::default()
                },
                centered: false,
            }),
        ],
    };

    let first = evaluate_assembly_plan_with_generator(&recipe, &plan, &FixtureGenerator::default())
        .expect("first assembly");
    let second =
        evaluate_assembly_plan_with_generator(&recipe, &plan, &FixtureGenerator::default())
            .expect("second assembly");

    assert_eq!(
        first.provenance.instance_order,
        second.provenance.instance_order
    );
    assert_eq!(
        first.provenance.instance_order,
        vec![PartInstanceId(1), PartInstanceId(10), PartInstanceId(11)]
    );
}

#[test]
fn disabled_parts_are_skipped() {
    let generator = FixtureGenerator::default();
    let mut recipe = recipe_with([
        definition(PartDefinitionId(1), []),
        definition(PartDefinitionId(2), []),
    ]);
    recipe.instances.insert(
        PartInstanceId(1),
        instance(
            PartInstanceId(1),
            PartDefinitionId(1),
            None,
            [0.0, 0.0, 0.0],
        ),
    );
    let mut disabled = instance(
        PartInstanceId(2),
        PartDefinitionId(2),
        None,
        [2.0, 0.0, 0.0],
    );
    disabled.enabled = false;
    recipe.instances.insert(PartInstanceId(2), disabled);
    recipe
        .root_instances
        .extend([PartInstanceId(1), PartInstanceId(2)]);
    recipe.next_ids.part_instance = 3;

    let evaluation = evaluate_assembly_with_generator(&recipe, &generator).expect("assemble");

    assert_eq!(generator.call_count(PartDefinitionId(1)), 1);
    assert_eq!(generator.call_count(PartDefinitionId(2)), 0);
    assert_eq!(evaluation.instances.len(), 1);
    assert!(!evaluation.world_meshes.contains_key(&PartInstanceId(2)));
}

#[test]
fn attachment_cycle_is_rejected() {
    let socket = SocketId(1);
    let mut recipe = recipe_with([
        definition(PartDefinitionId(1), [(socket, [0.0, 0.0, 0.0])]),
        definition(PartDefinitionId(2), [(socket, [0.0, 0.0, 0.0])]),
    ]);
    recipe.instances.insert(
        PartInstanceId(1),
        attached_instance(
            PartInstanceId(1),
            PartDefinitionId(1),
            PartInstanceId(2),
            socket,
            socket,
            Transform3::default(),
            AttachmentMode::RigidSeparate,
        ),
    );
    recipe.instances.insert(
        PartInstanceId(2),
        attached_instance(
            PartInstanceId(2),
            PartDefinitionId(2),
            PartInstanceId(1),
            socket,
            socket,
            Transform3::default(),
            AttachmentMode::RigidSeparate,
        ),
    );
    recipe.next_ids.part_instance = 3;

    let error = evaluate_assembly_with_generator(&recipe, &FixtureGenerator::default())
        .expect_err("cycle should fail");

    assert!(matches!(error, AssemblyError::AttachmentCycle(_)));
}

#[test]
fn missing_socket_is_rejected() {
    let mut recipe = recipe_with([
        definition(PartDefinitionId(1), [(SocketId(1), [0.0, 0.0, 0.0])]),
        definition(PartDefinitionId(2), []),
    ]);
    recipe.instances.insert(
        PartInstanceId(1),
        instance(
            PartInstanceId(1),
            PartDefinitionId(1),
            None,
            [0.0, 0.0, 0.0],
        ),
    );
    recipe.instances.insert(
        PartInstanceId(2),
        attached_instance(
            PartInstanceId(2),
            PartDefinitionId(2),
            PartInstanceId(1),
            SocketId(1),
            SocketId(99),
            Transform3::default(),
            AttachmentMode::RigidSeparate,
        ),
    );
    recipe.root_instances.push(PartInstanceId(1));
    recipe.next_ids.part_instance = 3;

    let error = evaluate_assembly_with_generator(&recipe, &FixtureGenerator::default())
        .expect_err("missing socket should fail");

    assert!(matches!(
        error,
        AssemblyError::MissingSocket {
            instance: PartInstanceId(2),
            socket: SocketId(99),
            ..
        }
    ));
}

#[test]
fn weld_boundary_reserved_is_reported_as_unsupported() {
    let socket = SocketId(1);
    let mut recipe = recipe_with([
        definition(PartDefinitionId(1), [(socket, [0.0, 0.0, 0.0])]),
        definition(PartDefinitionId(2), [(socket, [0.0, 0.0, 0.0])]),
    ]);
    recipe.instances.insert(
        PartInstanceId(1),
        instance(
            PartInstanceId(1),
            PartDefinitionId(1),
            None,
            [0.0, 0.0, 0.0],
        ),
    );
    recipe.instances.insert(
        PartInstanceId(2),
        attached_instance(
            PartInstanceId(2),
            PartDefinitionId(2),
            PartInstanceId(1),
            socket,
            socket,
            Transform3::default(),
            AttachmentMode::WeldBoundaryReserved,
        ),
    );
    recipe.root_instances.push(PartInstanceId(1));
    recipe.next_ids.part_instance = 3;

    let error = evaluate_assembly_with_generator(&recipe, &FixtureGenerator::default())
        .expect_err("reserved weld should fail");

    assert!(
        matches!(error, AssemblyError::Unsupported { feature } if feature == "WeldBoundaryReserved")
    );
}

#[test]
fn provenance_survives_assembly_and_generated_occurrences() {
    let mut recipe = recipe_with([definition(PartDefinitionId(1), [])]);
    recipe.instances.insert(
        PartInstanceId(1),
        instance(
            PartInstanceId(1),
            PartDefinitionId(1),
            None,
            [0.0, 0.0, 0.0],
        ),
    );
    recipe.root_instances.push(PartInstanceId(1));
    recipe.next_ids.part_instance = 2;
    let plan = AssemblyPlan {
        operations: vec![AssemblyOperation::LinearArray(LinearArrayOperation {
            operation: OperationId(90),
            prototypes: vec![PartInstanceId(1)],
            count: 2,
            step: Transform3 {
                translation: [1.0, 0.0, 0.0],
                ..Transform3::default()
            },
            centered: false,
        })],
    };

    let evaluation =
        evaluate_assembly_plan_with_generator(&recipe, &plan, &FixtureGenerator::default())
            .expect("assemble");

    let generated_metadata = &evaluation.world_meshes[&PartInstanceId(2)].face_metadata[0];
    assert_eq!(
        generated_metadata.part_definition,
        Some(PartDefinitionId(1))
    );
    assert_eq!(generated_metadata.part_instance, Some(PartInstanceId(2)));
    assert_eq!(generated_metadata.region, Some(FACE_REGION));
    assert_eq!(generated_metadata.operation, Some(FACE_OPERATION));
    assert_eq!(
        evaluation.provenance.instances[1].prototype_instance_id,
        Some(PartInstanceId(1))
    );
    assert_eq!(
        evaluation.provenance.instances[1].generated_by,
        Some(OperationId(90))
    );
}

fn recipe_with<const N: usize>(definitions: [PartDefinition; N]) -> AssetRecipe {
    let mut recipe = AssetRecipe::new(AssetId(1), "Assembly Test");
    for definition in definitions {
        recipe.definitions.insert(definition.id, definition);
    }
    recipe.next_ids.part_definition = recipe
        .definitions
        .keys()
        .map(|id| id.0)
        .max()
        .unwrap_or_default()
        + 1;
    recipe
}

fn definition<const N: usize>(
    id: PartDefinitionId,
    sockets: [(SocketId, [f32; 3]); N],
) -> PartDefinition {
    PartDefinition {
        id,
        name: format!("Definition {}", id.0),
        tags: BTreeSet::new(),
        geometry: GeometryRecipe {
            source: GeometrySource::LiteralMesh {
                positions: Vec::new(),
                faces: Vec::new(),
            },
            operations: Vec::new(),
        },
        regions: BTreeMap::new(),
        sockets: sockets
            .into_iter()
            .map(|(socket_id, origin)| (socket_id, socket(socket_id, origin)))
            .collect(),
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
        name: format!("Instance {}", id.0),
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

fn attached_instance(
    id: PartInstanceId,
    definition: PartDefinitionId,
    parent: PartInstanceId,
    parent_socket: SocketId,
    child_socket: SocketId,
    local_offset: Transform3,
    mode: AttachmentMode,
) -> PartInstance {
    PartInstance {
        id,
        definition,
        name: format!("Attached {}", id.0),
        parent: Some(parent),
        local_transform: Transform3::default(),
        attachment: Some(AttachmentSpec {
            parent_instance: parent,
            parent_socket,
            child_socket,
            local_offset,
            mode,
        }),
        enabled: true,
        tags: BTreeSet::new(),
        generated_by: None,
    }
}

fn socket(id: SocketId, origin: [f32; 3]) -> SocketSpec {
    SocketSpec {
        id,
        name: format!("Socket {}", id.0),
        local_frame: Frame3 {
            origin,
            ..Frame3::default()
        },
        role: "attachment".to_owned(),
        tags: BTreeSet::new(),
    }
}

fn triangle_mesh(definition: PartDefinitionId) -> PolygonMesh {
    polygon_mesh_from_faces(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![vec![0, 1, 2]],
        vec![FaceMetadata {
            part_definition: Some(definition),
            part_instance: None,
            region: Some(FACE_REGION),
            operation: Some(FACE_OPERATION),
            ..FaceMetadata::default()
        }],
    )
    .expect("fixture mesh should be valid")
}

fn instance_origin(
    evaluation: &orchard_modeling::assembly::AssemblyEvaluation,
    instance: PartInstanceId,
) -> [f32; 3] {
    evaluation.world_transforms[&instance].transform_point([0.0, 0.0, 0.0])
}

fn triangle_normal_z(mesh: &PolygonMesh) -> f32 {
    let face = &mesh.faces[0];
    let a = mesh.positions[face.vertices[0] as usize];
    let b = mesh.positions[face.vertices[1] as usize];
    let c = mesh.positions[face.vertices[2] as usize];
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let cross_z = ab[0] * ac[1] - ab[1] * ac[0];
    let length = ((ab[1] * ac[2] - ab[2] * ac[1]).powi(2)
        + (ab[2] * ac[0] - ab[0] * ac[2]).powi(2)
        + cross_z.powi(2))
    .sqrt();
    cross_z / length
}

fn assert_close(actual: [f32; 3], expected: [f32; 3]) {
    for index in 0..3 {
        assert!(
            (actual[index] - expected[index]).abs() <= 1.0e-4,
            "component {index}: expected {expected:?}, got {actual:?}",
        );
    }
}
