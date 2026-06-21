use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetEdit, AssetId, AssetRecipe, BoundaryLoopId, CountRangeHint, CutEdgeTreatment,
    CutGroupRole, Frame3, GeometryRecipe, GeometrySource, ModelingOperationSpec, OperationId,
    ParameterDescriptor, ParameterId, PartDefinition, PartDefinitionId, PartInstance,
    PartInstanceId, PlanarCutFace, RegionId, ReplacementGroupHint, SemanticCutGroupHint,
    SurfaceRegionSpec, SurfaceRole, Transform3, validate_asset_recipe,
};
use shape_search::asset::{
    AssetCandidateEditKind, AssetCandidateMode, AssetCandidateRequest, generate_asset_candidates,
};

fn rich_recipe() -> AssetRecipe {
    let mut recipe = AssetRecipe::new(AssetId(42), "Candidate Fixture");
    recipe.definitions.insert(
        PartDefinitionId(1),
        definition(
            1,
            "body",
            GeometrySource::RoundedBox {
                half_extents: [1.0, 0.45, 0.75],
                radius: 0.08,
            },
            vec![
                ModelingOperationSpec::SetBevelProfile {
                    operation: OperationId(1),
                    radius: 0.08,
                    segments: 2,
                },
                ModelingOperationSpec::LinearArray {
                    operation: OperationId(2),
                    count: 3,
                    offset: [0.35, 0.0, 0.0],
                },
            ],
            None,
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(2),
        definition(
            2,
            "round detail",
            GeometrySource::Cylinder {
                radius: 0.16,
                height: 0.18,
                radial_segments: 16,
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(3),
                radius: 0.02,
                segments: 1,
            }],
            Some("detail"),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(3),
        definition(
            3,
            "square detail",
            GeometrySource::Plate {
                size: [0.28, 0.20],
                thickness: 0.06,
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(4),
                radius: 0.01,
                segments: 1,
            }],
            Some("detail"),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(4),
        definition(
            4,
            "swept handle",
            GeometrySource::Sweep {
                profile: vec![[0.04, -0.03], [0.04, 0.03]],
                path: vec![
                    frame([0.0, -0.2, 0.0]),
                    frame([0.0, 0.0, 0.18]),
                    frame([0.0, 0.2, 0.0]),
                ],
            },
            Vec::new(),
            None,
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(5),
        definition(
            5,
            "lathed knob",
            GeometrySource::Lathe {
                profile: vec![[0.0, -0.08], [0.12, -0.06], [0.16, 0.04], [0.0, 0.08]],
                segments: 24,
            },
            Vec::new(),
            None,
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(6),
        definition(
            6,
            "literal reference",
            GeometrySource::LiteralMesh {
                positions: vec![[0.0, 0.0, 0.0], [0.1, 0.0, 0.0], [0.0, 0.1, 0.0]],
                faces: vec![vec![0, 1, 2]],
            },
            Vec::new(),
            None,
        ),
    );
    let mut cut_panel = definition(
        7,
        "cut panel",
        GeometrySource::Plate {
            size: [1.6, 0.9],
            thickness: 0.08,
        },
        vec![ModelingOperationSpec::RectangularThroughCut {
            operation: OperationId(5),
            region: RegionId(1),
            face: PlanarCutFace::PositiveY,
            center: [0.0, 0.0],
            size: [0.24, 0.12],
            corner_radius: 0.02,
            rim_width: 0.03,
            corner_segments: 3,
            entry_loop: BoundaryLoopId(1),
            exit_loop: BoundaryLoopId(2),
            outer_region: RegionId(1),
            rim_region: RegionId(4),
            wall_region: RegionId(5),
            edge_treatment: CutEdgeTreatment::Hard,
        }],
        None,
    );
    cut_panel
        .regions
        .insert(RegionId(1), region(1, "front", SurfaceRole::PrimarySurface));
    cut_panel
        .regions
        .insert(RegionId(2), region(2, "back", SurfaceRole::PrimarySurface));
    cut_panel
        .regions
        .insert(RegionId(3), region(3, "side", SurfaceRole::Side));
    cut_panel
        .regions
        .insert(RegionId(4), region(4, "cut rim", SurfaceRole::Detail));
    cut_panel
        .regions
        .insert(RegionId(5), region(5, "cut wall", SurfaceRole::Interior));
    recipe.definitions.insert(PartDefinitionId(7), cut_panel);

    recipe
        .instances
        .insert(PartInstanceId(1), instance(1, 1, "body", None));
    recipe.instances.insert(
        PartInstanceId(2),
        instance(2, 2, "optional detail", Some(1)),
    );
    recipe
        .instances
        .insert(PartInstanceId(3), instance(3, 4, "handle", Some(1)));
    recipe
        .instances
        .insert(PartInstanceId(4), instance(4, 5, "knob", Some(1)));
    recipe
        .instances
        .insert(PartInstanceId(5), instance(5, 6, "literal", Some(1)));
    recipe
        .instances
        .insert(PartInstanceId(6), instance(6, 7, "cut panel", Some(1)));
    recipe.root_instances.push(PartInstanceId(1));

    recipe.parameters.insert(
        ParameterId(1),
        parameter(
            1,
            "Body width",
            "definition.1.geometry.rounded_box.half_extents.x",
            0.75,
            1.35,
            0.01,
            false,
        ),
    );
    recipe.parameters.insert(
        ParameterId(2),
        parameter(
            2,
            "Body bevel",
            "definition.1.operation.1.bevel.radius",
            0.02,
            0.16,
            0.01,
            false,
        ),
    );
    recipe.parameters.insert(
        ParameterId(3),
        parameter(
            3,
            "Detail X",
            "instance.2.transform.translation.x",
            -0.6,
            0.6,
            0.01,
            false,
        ),
    );
    recipe.parameters.insert(
        ParameterId(4),
        parameter(
            4,
            "Detail segments",
            "definition.2.geometry.cylinder.radial_segments",
            8.0,
            32.0,
            1.0,
            true,
        ),
    );
    recipe.variation.parameter_range_overrides.insert(
        ParameterId(1),
        shape_asset::ParameterRangeOverride {
            minimum: 0.85,
            maximum: 1.15,
            step: Some(0.01),
            mutation_sigma: Some(0.04),
        },
    );
    recipe
        .variation
        .optional_instances
        .insert(PartInstanceId(2));
    recipe.variation.replacement_groups.insert(
        "detail".to_owned(),
        ReplacementGroupHint {
            definitions: BTreeSet::from([PartDefinitionId(2), PartDefinitionId(3)]),
        },
    );
    recipe.variation.count_ranges.insert(
        OperationId(2),
        CountRangeHint {
            minimum: 2,
            maximum: 6,
        },
    );
    recipe.next_ids.part_definition = 8;
    recipe.next_ids.part_instance = 7;
    recipe.next_ids.operation = 6;
    recipe.next_ids.region = 6;
    recipe.next_ids.boundary_loop = 3;
    recipe.next_ids.parameter = 5;

    assert!(validate_asset_recipe(&recipe).is_valid());
    recipe
}

fn grouped_cut_recipe() -> AssetRecipe {
    let mut recipe = rich_recipe();
    let definition = recipe
        .definitions
        .get_mut(&PartDefinitionId(7))
        .expect("cut panel definition should exist");
    definition.geometry.operations = vec![
        rectangular_cut(
            OperationId(5),
            [-0.36, 0.0],
            BoundaryLoopId(1),
            BoundaryLoopId(2),
            RegionId(4),
            RegionId(5),
        ),
        rectangular_cut(
            OperationId(6),
            [0.0, 0.0],
            BoundaryLoopId(3),
            BoundaryLoopId(4),
            RegionId(6),
            RegionId(7),
        ),
        rectangular_cut(
            OperationId(7),
            [0.36, 0.0],
            BoundaryLoopId(5),
            BoundaryLoopId(6),
            RegionId(8),
            RegionId(9),
        ),
    ];
    for (region_id, name, role) in [
        (RegionId(6), "cut rim 2", SurfaceRole::Detail),
        (RegionId(7), "cut wall 2", SurfaceRole::Interior),
        (RegionId(8), "cut rim 3", SurfaceRole::Detail),
        (RegionId(9), "cut wall 3", SurfaceRole::Interior),
    ] {
        definition
            .regions
            .insert(region_id, region(region_id.0, name, role));
    }
    recipe.variation.semantic_cut_groups.insert(
        "vents".to_owned(),
        SemanticCutGroupHint {
            label: "Vent slots".to_owned(),
            definition: PartDefinitionId(7),
            operations: vec![OperationId(5), OperationId(6), OperationId(7)],
            role: CutGroupRole::Vents,
            count_range: Some(CountRangeHint {
                minimum: 1,
                maximum: 5,
            }),
        },
    );
    recipe.next_ids.operation = 8;
    recipe.next_ids.region = 10;
    recipe.next_ids.boundary_loop = 7;
    assert!(validate_asset_recipe(&recipe).is_valid());
    recipe
}

fn rectangular_cut(
    operation: OperationId,
    center: [f32; 2],
    entry_loop: BoundaryLoopId,
    exit_loop: BoundaryLoopId,
    rim_region: RegionId,
    wall_region: RegionId,
) -> ModelingOperationSpec {
    ModelingOperationSpec::RectangularThroughCut {
        operation,
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center,
        size: [0.16, 0.08],
        corner_radius: 0.02,
        rim_width: 0.03,
        corner_segments: 3,
        entry_loop,
        exit_loop,
        outer_region: RegionId(1),
        rim_region,
        wall_region,
        edge_treatment: CutEdgeTreatment::Hard,
    }
}

fn request(seed: u64, mode: AssetCandidateMode) -> AssetCandidateRequest {
    AssetCandidateRequest {
        seed,
        proposal_count: 96,
        result_count: 24,
        mode,
    }
}

#[test]
fn same_seed_is_deterministic() {
    let recipe = rich_recipe();
    let first = generate_asset_candidates(&recipe, &request(7, AssetCandidateMode::Explore))
        .expect("candidates should generate");
    let second = generate_asset_candidates(&recipe, &request(7, AssetCandidateMode::Explore))
        .expect("candidates should generate");

    assert_eq!(first, second);
}

#[test]
fn refine_remains_local_and_topology_preserving() {
    let recipe = rich_recipe();
    let output = generate_asset_candidates(&recipe, &request(11, AssetCandidateMode::Refine))
        .expect("candidates should generate");

    assert!(!output.candidates.is_empty());
    for candidate in &output.candidates {
        assert!(candidate.program.operations.len() <= 2);
        assert!(
            candidate
                .diagnostics
                .changes
                .iter()
                .all(|change| !change.topology_changing)
        );
        assert!(candidate.diagnostics.changes.iter().all(|change| {
            !matches!(
                change.kind,
                AssetCandidateEditKind::OptionalPart
                    | AssetCandidateEditKind::Replacement
                    | AssetCandidateEditKind::ArrayCount
                    | AssetCandidateEditKind::DetailDensity
                    | AssetCandidateEditKind::ModelingOperation
            )
        }));
    }
}

#[test]
fn explore_is_broader_than_refine() {
    let recipe = rich_recipe();
    let refine = generate_asset_candidates(&recipe, &request(13, AssetCandidateMode::Refine))
        .expect("refine candidates should generate");
    let explore = generate_asset_candidates(&recipe, &request(13, AssetCandidateMode::Explore))
        .expect("explore candidates should generate");

    assert!(mean_operation_count(&explore) > mean_operation_count(&refine));
    assert!(has_kind(&explore, AssetCandidateEditKind::OptionalPart));
    assert!(has_kind(&explore, AssetCandidateEditKind::Replacement));
    assert!(has_kind(&explore, AssetCandidateEditKind::ArrayCount));
}

#[test]
fn locks_are_honored() {
    let mut recipe = rich_recipe();
    recipe.locks.insert(ParameterId(1));
    recipe.instance_locks.insert(PartInstanceId(2));
    recipe.subtree_locks.insert(PartInstanceId(3));
    let output = generate_asset_candidates(&recipe, &request(17, AssetCandidateMode::Explore))
        .expect("candidates should generate");

    for candidate in &output.candidates {
        for operation in &candidate.program.operations {
            match operation {
                shape_asset::AssetEdit::SetScalar { parameter, .. } => {
                    assert_ne!(*parameter, ParameterId(1));
                    assert_ne!(*parameter, ParameterId(3));
                }
                shape_asset::AssetEdit::SetTransform { instance, .. }
                | shape_asset::AssetEdit::SetOptionalPartEnabled { instance, .. }
                | shape_asset::AssetEdit::ReplaceInstanceDefinition { instance, .. } => {
                    assert_ne!(*instance, PartInstanceId(2));
                    assert_ne!(*instance, PartInstanceId(3));
                }
                _ => {}
            }
        }
    }
    assert!(output.diagnostics.locked_targets_skipped > 0);
}

#[test]
fn topology_locks_block_topology_changes_for_definition() {
    let mut recipe = rich_recipe();
    recipe.topology_locks.insert(PartDefinitionId(1));
    recipe.topology_locks.insert(PartDefinitionId(2));
    let output = generate_asset_candidates(&recipe, &request(19, AssetCandidateMode::Explore))
        .expect("candidates should generate");

    for candidate in &output.candidates {
        for operation in &candidate.program.operations {
            match operation {
                shape_asset::AssetEdit::SetArrayCount { definition, .. }
                | shape_asset::AssetEdit::DuplicateCutOperation { definition, .. }
                | shape_asset::AssetEdit::RemoveModelingOperation { definition, .. }
                | shape_asset::AssetEdit::InsertModelingOperation { definition, .. }
                | shape_asset::AssetEdit::MoveModelingOperation { definition, .. }
                | shape_asset::AssetEdit::SetBevelSettings {
                    definition,
                    segments: Some(_),
                    ..
                } => {
                    assert_ne!(*definition, PartDefinitionId(1));
                    assert_ne!(*definition, PartDefinitionId(2));
                }
                shape_asset::AssetEdit::SetGeneratorDimension {
                    definition,
                    dimension,
                } if dimension.topology_changing() => {
                    assert_ne!(*definition, PartDefinitionId(1));
                    assert_ne!(*definition, PartDefinitionId(2));
                }
                shape_asset::AssetEdit::ReplaceInstanceDefinition { instance, .. } => {
                    assert_ne!(*instance, PartInstanceId(2));
                }
                _ => {}
            }
        }
    }
}

#[test]
fn optional_parts_replacements_and_array_counts_are_generated() {
    let recipe = rich_recipe();
    let output = generate_asset_candidates(&recipe, &request(23, AssetCandidateMode::Explore))
        .expect("candidates should generate");

    assert!(output.candidates.iter().any(|candidate| {
        candidate.program.operations.iter().any(|operation| {
            matches!(
                operation,
                shape_asset::AssetEdit::SetOptionalPartEnabled {
                    instance: PartInstanceId(2),
                    ..
                }
            )
        })
    }));
    assert!(output.candidates.iter().any(|candidate| {
        candidate.program.operations.iter().any(|operation| {
            matches!(
                operation,
                shape_asset::AssetEdit::ReplaceInstanceDefinition {
                    instance: PartInstanceId(2),
                    definition: PartDefinitionId(3),
                }
            )
        })
    }));
    assert!(output.candidates.iter().any(|candidate| {
        candidate.program.operations.iter().any(|operation| {
            matches!(
                operation,
                shape_asset::AssetEdit::SetArrayCount {
                    operation: OperationId(2),
                    count: 2..=6,
                    ..
                }
            )
        })
    }));
}

#[test]
fn explore_generates_structural_cut_duplication() {
    let recipe = rich_recipe();
    let output = generate_asset_candidates(&recipe, &request(37, AssetCandidateMode::Explore))
        .expect("candidates should generate");

    assert!(has_kind(&output, AssetCandidateEditKind::ModelingOperation));
    let duplicate_cut = output
        .candidates
        .iter()
        .flat_map(|candidate| &candidate.program.operations)
        .find_map(|operation| match operation {
            AssetEdit::DuplicateCutOperation {
                definition,
                source,
                operation,
                entry_loop,
                secondary_loop,
                rim_region,
                wall_region,
                floor_region,
                center_offset,
            } => Some((
                *definition,
                *source,
                *operation,
                *entry_loop,
                *secondary_loop,
                *rim_region,
                *wall_region,
                *floor_region,
                *center_offset,
            )),
            _ => None,
        })
        .expect("Explore should propose at least one cut duplication");

    assert_eq!(duplicate_cut.0, PartDefinitionId(7));
    assert_eq!(duplicate_cut.1, OperationId(5));
    assert!(duplicate_cut.2.0 >= 6);
    assert!(duplicate_cut.3.0 >= 3);
    assert!(duplicate_cut.4.0 > duplicate_cut.3.0);
    assert!(duplicate_cut.5.0 >= 6);
    assert!(duplicate_cut.6.0 > duplicate_cut.5.0);
    assert_eq!(duplicate_cut.7, None);
    assert_ne!(duplicate_cut.8, [0.0, 0.0]);
}

#[test]
fn explore_generates_semantic_cut_group_edits() {
    let recipe = grouped_cut_recipe();
    let output = generate_asset_candidates(
        &recipe,
        &AssetCandidateRequest {
            seed: 41,
            proposal_count: 192,
            result_count: 96,
            mode: AssetCandidateMode::Explore,
        },
    )
    .expect("candidates should generate");

    let grouped_fields = output
        .candidates
        .iter()
        .flat_map(|candidate| &candidate.program.operations)
        .filter_map(|operation| match operation {
            AssetEdit::SetOperationScalar {
                definition,
                operation,
                field,
                ..
            } if *definition == PartDefinitionId(7)
                && matches!(*operation, OperationId(5) | OperationId(6) | OperationId(7)) =>
            {
                Some(field.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        grouped_fields
            .windows(3)
            .any(|fields| fields.iter().all(|field| *field == fields[0])),
        "expected at least one semantic cut group proposal to edit all vents together"
    );
}

#[test]
fn literal_mesh_vertices_are_not_mutated() {
    let recipe = rich_recipe();
    let before_positions = literal_positions(&recipe);
    let output = generate_asset_candidates(&recipe, &request(29, AssetCandidateMode::Explore))
        .expect("candidates should generate");

    for candidate in &output.candidates {
        assert_eq!(literal_positions(&candidate.recipe), before_positions);
        assert!(candidate.program.operations.iter().all(|operation| {
            !matches!(
                operation,
                shape_asset::AssetEdit::ReplaceGeometrySource {
                    source: GeometrySource::LiteralMesh { .. },
                    ..
                }
            )
        }));
    }
}

#[test]
fn all_survivors_validate_and_explain_changes() {
    let recipe = rich_recipe();
    let output = generate_asset_candidates(&recipe, &request(31, AssetCandidateMode::Explore))
        .expect("candidates should generate");

    assert!(!output.candidates.is_empty());
    for candidate in &output.candidates {
        assert!(validate_asset_recipe(&candidate.recipe).is_valid());
        assert_eq!(
            candidate.program.operations.len(),
            candidate.diagnostics.changes.len()
        );
        assert!(
            candidate
                .diagnostics
                .changes
                .iter()
                .all(|change| !change.subject.is_empty() && !change.message.is_empty())
        );
    }
}

fn has_kind(
    output: &shape_search::asset::AssetCandidateOutput,
    kind: AssetCandidateEditKind,
) -> bool {
    output
        .candidates
        .iter()
        .flat_map(|candidate| &candidate.diagnostics.changes)
        .any(|change| change.kind == kind)
}

fn mean_operation_count(output: &shape_search::asset::AssetCandidateOutput) -> f32 {
    let total = output
        .candidates
        .iter()
        .map(|candidate| candidate.program.operations.len())
        .sum::<usize>();
    total as f32 / output.candidates.len().max(1) as f32
}

fn literal_positions(recipe: &AssetRecipe) -> Vec<[f32; 3]> {
    let definition = recipe
        .definitions
        .get(&PartDefinitionId(6))
        .expect("literal definition should exist");
    match &definition.geometry.source {
        GeometrySource::LiteralMesh { positions, .. } => positions.clone(),
        _ => panic!("expected literal mesh"),
    }
}

fn definition(
    id: u64,
    name: &str,
    source: GeometrySource,
    operations: Vec<ModelingOperationSpec>,
    variant_group: Option<&str>,
) -> PartDefinition {
    PartDefinition {
        id: PartDefinitionId(id),
        name: name.to_owned(),
        tags: BTreeSet::new(),
        geometry: GeometryRecipe { source, operations },
        regions: BTreeMap::new(),
        sockets: BTreeMap::new(),
        local_pivot: Frame3::default(),
        variant_group: variant_group.map(str::to_owned),
        production_hints: None,
    }
}

fn region(id: u64, name: &str, role: SurfaceRole) -> SurfaceRegionSpec {
    SurfaceRegionSpec {
        id: RegionId(id),
        name: name.to_owned(),
        role,
        tags: BTreeSet::new(),
    }
}

fn instance(id: u64, definition: u64, name: &str, parent: Option<u64>) -> PartInstance {
    PartInstance {
        id: PartInstanceId(id),
        definition: PartDefinitionId(definition),
        name: name.to_owned(),
        parent: parent.map(PartInstanceId),
        local_transform: Transform3::default(),
        attachment: None,
        enabled: true,
        tags: BTreeSet::new(),
        generated_by: None,
    }
}

fn parameter(
    id: u64,
    label: &str,
    path: &str,
    minimum: f32,
    maximum: f32,
    step: f32,
    topology_changing: bool,
) -> ParameterDescriptor {
    ParameterDescriptor {
        id: ParameterId(id),
        path: path.to_owned(),
        label: label.to_owned(),
        group: "Fixture".to_owned(),
        minimum,
        maximum,
        step,
        mutation_sigma: step.max(0.02),
        topology_changing,
        beginner_description: format!("Adjust {label}."),
    }
}

fn frame(origin: [f32; 3]) -> Frame3 {
    Frame3 {
        origin,
        ..Frame3::default()
    }
}
