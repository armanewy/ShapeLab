//! Sci-fi crate headless foundry fixture.

use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetId, AssetRecipe, BoundaryLoopId, CountRangeHint, CutEdgeTreatment, CutGroupRole, Frame3,
    GeometryRecipe, GeometrySource, ModelingOperationSpec, OperationId, ParameterId,
    PartDefinition, PartInstance, PlanarCutFace, RegionId, SemanticCutGroupHint, SocketId,
    SocketSpec, SurfaceRegionSpec, SurfaceRole, Transform3, validate_asset_recipe,
};
use shape_family::{AllowedOperationKind, RoleMultiplicity};
use shape_family_compile::{
    FragmentSocketPort, ParameterBinding, RECIPE_FRAGMENT_SCHEMA_VERSION, RecipeFragment,
    RecipeFragmentExports, ScalarTransform, scalar_parameter,
};
use shape_foundry::{CandidateStrategy, ControlValue};

use crate::{
    FamilySchemaSpec, FixtureCatalogSpec, FoundryFixtureCatalog, advisory_control,
    advisory_ratio_slot, build_fixture_catalog, choice_control, choice_slot, continuous_control,
    count_slot, cylinder_fragment, family_implementation, family_schema, linear_array,
    plate_fragment, ratio_slot, role, rounded_box_fragment, runtime_control, runtime_ratio_slot,
    style_implementation, style_kit, toggle_control, toggle_slot,
};

const FRONT_REGION: RegionId = RegionId(0);
const BODY_HALF_EXTENTS: [f32; 3] = [1.2, 0.45, 0.72];
const BODY_RADIUS: f32 = 0.075;

/// Build the sci-fi crate fixture catalog.
#[must_use]
pub fn fixture_catalog() -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: "crate",
        display_name: "Crate",
        summary: "Theme-neutral hard-surface container with authored crate controls.",
        roles: vec![
            role("body", RoleMultiplicity::Single, true),
            role("panel", RoleMultiplicity::Repeated, true),
            role("fastener", RoleMultiplicity::Repeated, true),
            role("handle", RoleMultiplicity::Optional, false),
            role("trim", RoleMultiplicity::Optional, false),
        ],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Cut,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: vec![
            ratio_slot(
                "body_proportions",
                "Body Proportions",
                "body",
                0.0,
                1.0,
                0.05,
                0.45,
            ),
            ratio_slot(
                "structural_heft",
                "Structural Heft",
                "body",
                0.0,
                1.0,
                0.05,
                0.45,
            ),
            ratio_slot("panel_depth", "Panel Depth", "body", 0.0, 1.0, 0.05, 0.55),
            choice_slot(
                "vent_density",
                "Vent Density",
                "body",
                vec![
                    "sparse".to_owned(),
                    "standard".to_owned(),
                    "dense".to_owned(),
                ],
            ),
            choice_slot(
                "handle_style",
                "Handle Style",
                "handle",
                vec![
                    "flush".to_owned(),
                    "side_rail".to_owned(),
                    "cargo_bar".to_owned(),
                ],
            ),
            ratio_slot(
                "edge_softness",
                "Edge Softness",
                "body",
                0.0,
                1.0,
                0.05,
                0.4,
            ),
            count_slot(
                "detail_density",
                "Detail Density",
                "fastener",
                4.0,
                12.0,
                1.0,
                8,
            ),
            toggle_slot("has_trim", "Has Trim", "trim", true),
            runtime_ratio_slot("runtime_wear", "Runtime Wear", "body", 0.2),
            advisory_ratio_slot("advisory_weathering", "Advisory Weathering", "body", 0.25),
        ],
        compatible_style_kits: vec!["sci_fi_industrial".to_owned()],
        tags: vec![
            "crate".to_owned(),
            "sci-fi".to_owned(),
            "hard_surface".to_owned(),
        ],
    });
    let mut style = style_kit(
        "sci_fi_industrial",
        "Sci-Fi Industrial",
        "crate",
        &[
            ("sparse_vent_body", "Sparse vent body", "body"),
            ("standard_vent_body", "Standard vent body", "body"),
            ("dense_vent_body", "Dense vent body", "body"),
            ("front_access_plate", "Front access plate", "panel"),
            ("corner_fastener", "Corner fastener", "fastener"),
            ("flush_handle", "Flush handle", "handle"),
            ("side_rail_handle", "Side rail handle", "handle"),
            ("cargo_bar_handle", "Cargo bar handle", "handle"),
            ("edge_trim", "Edge trim", "trim"),
        ],
        vec![
            "sci-fi".to_owned(),
            "industrial".to_owned(),
            "crate".to_owned(),
        ],
    );
    if let Some(facet) = style.family_facets.get_mut("crate") {
        for prototype in &mut facet.part_prototypes {
            if prototype.role == "body"
                && !prototype
                    .operation_tags
                    .contains(&AllowedOperationKind::Cut)
            {
                prototype.operation_tags.push(AllowedOperationKind::Cut);
            }
        }
    }
    let family_impl = family_implementation(
        "crate",
        "Sci-fi crate base",
        vec![
            ParameterBinding::Scalar {
                slot: "body_proportions".to_owned(),
                role: "body".to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
                    "geometry.rounded_box.half_extents.x",
                ),
                transform: ScalarTransform::Ratio {
                    minimum: 1.0,
                    maximum: 1.55,
                },
            },
            ParameterBinding::Scalar {
                slot: "structural_heft".to_owned(),
                role: "body".to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
                    "geometry.rounded_box.half_extents.y",
                ),
                transform: ScalarTransform::Ratio {
                    minimum: 0.36,
                    maximum: 0.56,
                },
            },
            ParameterBinding::Scalar {
                slot: "panel_depth".to_owned(),
                role: "body".to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
                    "operation.1.recessed_panel_cut.depth",
                ),
                transform: ScalarTransform::Ratio {
                    minimum: 0.025,
                    maximum: 0.085,
                },
            },
            ParameterBinding::ChoiceToPrototype {
                slot: "vent_density".to_owned(),
                role: "body".to_owned(),
                choices: BTreeMap::from([
                    ("sparse".to_owned(), "sparse_vent_body".to_owned()),
                    ("standard".to_owned(), "standard_vent_body".to_owned()),
                    ("dense".to_owned(), "dense_vent_body".to_owned()),
                ]),
            },
            ParameterBinding::ChoiceToPrototype {
                slot: "handle_style".to_owned(),
                role: "handle".to_owned(),
                choices: BTreeMap::from([
                    ("flush".to_owned(), "flush_handle".to_owned()),
                    ("side_rail".to_owned(), "side_rail_handle".to_owned()),
                    ("cargo_bar".to_owned(), "cargo_bar_handle".to_owned()),
                ]),
            },
            ParameterBinding::Scalar {
                slot: "edge_softness".to_owned(),
                role: "body".to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
                    "geometry.rounded_box.radius",
                ),
                transform: ScalarTransform::Ratio {
                    minimum: 0.035,
                    maximum: 0.13,
                },
            },
            ParameterBinding::Scalar {
                slot: "detail_density".to_owned(),
                role: "fastener".to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
                    "operation.1.linear_array.count",
                ),
                transform: ScalarTransform::IntegerCount,
            },
            ParameterBinding::TogglePartPresence {
                slot: "has_trim".to_owned(),
                role: "trim".to_owned(),
            },
        ],
    );
    let style_impl = style_implementation(
        "sci_fi_industrial",
        "crate",
        BTreeMap::from([
            ("body".to_owned(), "standard_vent_body".to_owned()),
            ("panel".to_owned(), "front_access_plate".to_owned()),
            ("fastener".to_owned(), "corner_fastener".to_owned()),
            ("trim".to_owned(), "edge_trim".to_owned()),
        ]),
        vec![
            body_fragment("sparse_vent_body", &[[-0.22, -0.16], [0.22, -0.16]]),
            body_fragment(
                "standard_vent_body",
                &[[-0.3, -0.16], [0.0, -0.16], [0.3, -0.16]],
            ),
            body_fragment(
                "dense_vent_body",
                &[[-0.39, -0.16], [-0.13, -0.16], [0.13, -0.16], [0.39, -0.16]],
            ),
            plate_fragment(
                "front_access_plate",
                "panel",
                [1.45, 0.11],
                0.035,
                [0.0, 0.61, 0.62],
                Vec::new(),
            ),
            fastener_fragment(),
            rounded_box_fragment(
                "flush_handle",
                "handle",
                [0.42, 0.055, 0.055],
                0.018,
                [0.0, 0.64, -0.47],
                Vec::new(),
            ),
            rounded_box_fragment(
                "side_rail_handle",
                "handle",
                [0.055, 0.08, 0.42],
                0.025,
                [1.31, 0.0, 0.0],
                Vec::new(),
            ),
            rounded_box_fragment(
                "cargo_bar_handle",
                "handle",
                [0.62, 0.055, 0.055],
                0.02,
                [0.0, 0.64, -0.77],
                Vec::new(),
            ),
            plate_fragment(
                "edge_trim",
                "trim",
                [2.05, 0.06],
                0.03,
                [0.0, 0.625, 0.705],
                vec![linear_array(1, 2, [0.0, 0.0, -1.41])],
            ),
        ],
    );
    let mut trim_control = toggle_control("has_trim", "Has Trim", "has_trim", true);
    trim_control.primary = false;
    trim_control.visible = false;
    let mut profile = crate::customizer_profile(
        "crate",
        "sci_fi_industrial",
        vec![
            continuous_control(
                "body_proportions",
                "Body Proportions",
                "body_proportions",
                0.45,
                0.0,
                1.0,
            ),
            continuous_control(
                "structural_heft",
                "Structural Heft",
                "structural_heft",
                0.45,
                0.0,
                1.0,
            ),
            continuous_control("panel_depth", "Panel Depth", "panel_depth", 0.55, 0.0, 1.0),
            choice_control(
                "vent_density",
                "Vent Density",
                "vent_density",
                &["sparse", "standard", "dense"],
            ),
            choice_control(
                "handle_style",
                "Handle Style",
                "handle_style",
                &["flush", "side_rail", "cargo_bar"],
            ),
            continuous_control(
                "edge_softness",
                "Edge Softness",
                "edge_softness",
                0.4,
                0.0,
                1.0,
            ),
            crate::integer_control(
                "detail_density",
                "Detail Density",
                "detail_density",
                8,
                4,
                12,
            ),
            trim_control,
            runtime_control("runtime_wear", "Runtime Wear", "runtime_wear", 0.2),
            advisory_control(
                "advisory_weathering",
                "Advisory Weathering",
                "advisory_weathering",
                0.25,
            ),
        ],
    );
    profile.candidate_strategies = vec![
        CandidateStrategy {
            id: "compact-storage".to_owned(),
            label: "Compact Storage".to_owned(),
            control_ids: vec![
                "body_proportions".to_owned(),
                "structural_heft".to_owned(),
                "handle_style".to_owned(),
                "has_trim".to_owned(),
            ],
        },
        CandidateStrategy {
            id: "reinforced-cargo".to_owned(),
            label: "Reinforced Cargo".to_owned(),
            control_ids: vec![
                "structural_heft".to_owned(),
                "edge_softness".to_owned(),
                "detail_density".to_owned(),
                "handle_style".to_owned(),
            ],
        },
        CandidateStrategy {
            id: "vented-equipment".to_owned(),
            label: "Vented Equipment".to_owned(),
            control_ids: vec![
                "vent_density".to_owned(),
                "panel_depth".to_owned(),
                "detail_density".to_owned(),
            ],
        },
        CandidateStrategy {
            id: "minimal-industrial".to_owned(),
            label: "Minimal Industrial".to_owned(),
            control_ids: vec![
                "panel_depth".to_owned(),
                "handle_style".to_owned(),
                "edge_softness".to_owned(),
                "has_trim".to_owned(),
            ],
        },
        CandidateStrategy {
            id: "hero-prop".to_owned(),
            label: "Hero Prop".to_owned(),
            control_ids: vec![
                "body_proportions".to_owned(),
                "structural_heft".to_owned(),
                "panel_depth".to_owned(),
                "vent_density".to_owned(),
                "handle_style".to_owned(),
                "edge_softness".to_owned(),
                "detail_density".to_owned(),
            ],
        },
    ];

    build_fixture_catalog(FixtureCatalogSpec {
        slug: "sci-fi-crate",
        document_id: "sci-fi-crate-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state: BTreeMap::from([
            ("body_proportions".to_owned(), ControlValue::Scalar(0.45)),
            ("structural_heft".to_owned(), ControlValue::Scalar(0.5)),
            ("panel_depth".to_owned(), ControlValue::Scalar(0.55)),
            (
                "vent_density".to_owned(),
                ControlValue::Choice("standard".to_owned()),
            ),
            (
                "handle_style".to_owned(),
                ControlValue::Choice("side_rail".to_owned()),
            ),
            ("edge_softness".to_owned(), ControlValue::Scalar(0.45)),
            ("detail_density".to_owned(), ControlValue::Integer(8)),
            ("has_trim".to_owned(), ControlValue::Toggle(true)),
            ("runtime_wear".to_owned(), ControlValue::Scalar(0.2)),
            ("advisory_weathering".to_owned(), ControlValue::Scalar(0.25)),
        ]),
    })
}

fn body_fragment(id: &str, vent_centers: &[[f32; 2]]) -> RecipeFragment {
    let mut recipe = AssetRecipe::new(AssetId(1), format!("{id} fragment"));
    recipe.definitions.insert(
        crate::LOCAL_DEFINITION,
        PartDefinition {
            id: crate::LOCAL_DEFINITION,
            name: format!("{id} definition"),
            tags: BTreeSet::from(["body".to_owned(), "role:body".to_owned()]),
            geometry: GeometryRecipe {
                source: GeometrySource::RoundedBox {
                    half_extents: BODY_HALF_EXTENTS,
                    radius: BODY_RADIUS,
                },
                operations: body_operations(vent_centers),
            },
            regions: body_regions(vent_centers.len()),
            sockets: BTreeMap::from([(
                SocketId(7),
                SocketSpec {
                    id: SocketId(7),
                    name: "body origin".to_owned(),
                    local_frame: Frame3::default(),
                    role: "body".to_owned(),
                    tags: BTreeSet::from(["body".to_owned()]),
                },
            )]),
            local_pivot: Frame3::default(),
            variant_group: None,
            production_hints: None,
        },
    );
    recipe.instances.insert(
        crate::LOCAL_INSTANCE,
        PartInstance {
            id: crate::LOCAL_INSTANCE,
            definition: crate::LOCAL_DEFINITION,
            name: format!("{id} body"),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::from(["body".to_owned(), "role:body".to_owned()]),
            generated_by: None,
        },
    );
    recipe.root_instances.push(crate::LOCAL_INSTANCE);
    recipe.variation.semantic_cut_groups.insert(
        "front_recesses".to_owned(),
        SemanticCutGroupHint {
            label: "Front recessed panels".to_owned(),
            definition: crate::LOCAL_DEFINITION,
            operations: vec![OperationId(1), OperationId(2)],
            role: CutGroupRole::Recesses,
            count_range: Some(CountRangeHint {
                minimum: 1,
                maximum: 4,
            }),
        },
    );
    recipe.variation.semantic_cut_groups.insert(
        "vent_slots".to_owned(),
        SemanticCutGroupHint {
            label: "Vent slots".to_owned(),
            definition: crate::LOCAL_DEFINITION,
            operations: (0..vent_centers.len())
                .map(|index| OperationId(3 + index as u64))
                .collect(),
            role: CutGroupRole::Vents,
            count_range: Some(CountRangeHint {
                minimum: 2,
                maximum: 6,
            }),
        },
    );
    let hole_start = 3 + vent_centers.len() as u64;
    recipe.variation.semantic_cut_groups.insert(
        "mount_holes".to_owned(),
        SemanticCutGroupHint {
            label: "Mounting holes".to_owned(),
            definition: crate::LOCAL_DEFINITION,
            operations: vec![OperationId(hole_start), OperationId(hole_start + 1)],
            role: CutGroupRole::MountHoles,
            count_range: Some(CountRangeHint {
                minimum: 2,
                maximum: 4,
            }),
        },
    );
    recipe.parameters.insert(
        ParameterId(1),
        scalar_parameter(
            1,
            shape_asset::definition_scalar_path(
                crate::LOCAL_DEFINITION,
                "geometry.rounded_box.half_extents.x",
            ),
            "Body half width",
            0.8,
            2.0,
            0.05,
            false,
        ),
    );
    recipe.parameters.insert(
        ParameterId(2),
        scalar_parameter(
            2,
            shape_asset::definition_scalar_path(
                crate::LOCAL_DEFINITION,
                "geometry.rounded_box.half_extents.y",
            ),
            "Body structural heft",
            0.3,
            0.65,
            0.025,
            false,
        ),
    );
    recipe.parameters.insert(
        ParameterId(3),
        scalar_parameter(
            3,
            shape_asset::definition_scalar_path(
                crate::LOCAL_DEFINITION,
                "geometry.rounded_box.half_extents.z",
            ),
            "Body half depth",
            0.45,
            1.2,
            0.05,
            false,
        ),
    );
    recipe.parameters.insert(
        ParameterId(4),
        scalar_parameter(
            4,
            shape_asset::definition_scalar_path(
                crate::LOCAL_DEFINITION,
                "geometry.rounded_box.radius",
            ),
            "Body edge radius",
            0.02,
            0.16,
            0.01,
            false,
        ),
    );
    recipe.parameters.insert(
        ParameterId(5),
        scalar_parameter(
            5,
            shape_asset::definition_scalar_path(
                crate::LOCAL_DEFINITION,
                "operation.1.recessed_panel_cut.depth",
            ),
            "Main panel depth",
            0.02,
            0.09,
            0.005,
            false,
        ),
    );
    recipe.next_ids.parameter = 6;
    recipe.next_ids.region = 120;
    recipe.next_ids.boundary_loop = 120;
    recipe.next_ids.operation = recipe
        .definitions
        .get(&crate::LOCAL_DEFINITION)
        .expect("body definition exists")
        .geometry
        .operations
        .iter()
        .map(ModelingOperationSpec::operation_id)
        .map(|operation| operation.0)
        .max()
        .unwrap_or_default()
        + 1;
    recipe.next_ids.part_definition = crate::LOCAL_DEFINITION.0 + 1;
    recipe.next_ids.part_instance = crate::LOCAL_INSTANCE.0 + 1;
    recipe.next_ids.socket = 8;
    assert!(validate_asset_recipe(&recipe).is_valid());
    RecipeFragment {
        schema_version: RECIPE_FRAGMENT_SCHEMA_VERSION,
        id: id.to_owned(),
        provided_role: "body".to_owned(),
        exports: RecipeFragmentExports {
            role_occurrence_roots: vec![crate::LOCAL_INSTANCE],
            internal_roots: Vec::new(),
            socket_ports: vec![FragmentSocketPort {
                id: "body-origin".to_owned(),
                local_occurrence_root: crate::LOCAL_INSTANCE,
                local_socket: SocketId(7),
                compatibility_tags: vec!["body".to_owned()],
            }],
            surface_ports: Vec::new(),
        },
        recipe,
    }
}

fn body_operations(vent_centers: &[[f32; 2]]) -> Vec<ModelingOperationSpec> {
    let mut operations = vec![
        recessed_panel(1, [-0.38, 0.22], 1, 2, 10, 11, 12),
        recessed_panel(2, [0.38, 0.22], 3, 4, 13, 14, 15),
    ];
    for (index, center) in vent_centers.iter().enumerate() {
        let operation = 3 + index as u64;
        let loop_base = 5 + index as u64 * 2;
        let region_base = 30 + index as u64 * 2;
        operations.push(rectangular_vent(
            operation,
            *center,
            loop_base,
            loop_base + 1,
            region_base,
            region_base + 1,
        ));
    }
    let hole_start = 3 + vent_centers.len() as u64;
    let loop_start = 5 + vent_centers.len() as u64 * 2;
    let region_start = 30 + vent_centers.len() as u64 * 2;
    operations.push(mount_hole(
        hole_start,
        [-0.72, -0.45],
        loop_start,
        loop_start + 1,
        region_start,
        region_start + 1,
    ));
    operations.push(mount_hole(
        hole_start + 1,
        [0.72, -0.45],
        loop_start + 2,
        loop_start + 3,
        region_start + 2,
        region_start + 3,
    ));
    let bevel_start = hole_start + 2;
    operations.extend([
        boundary_bevel(bevel_start, 1, 80, 100, 101, 0.012),
        boundary_bevel(bevel_start + 1, 3, 81, 102, 103, 0.012),
        boundary_bevel(bevel_start + 2, loop_start, 82, 104, 105, 0.008),
        boundary_bevel(bevel_start + 3, loop_start + 2, 83, 106, 107, 0.008),
    ]);
    operations
}

fn recessed_panel(
    operation: u64,
    center: [f32; 2],
    entry_loop: u64,
    floor_loop: u64,
    rim_region: u64,
    wall_region: u64,
    floor_region: u64,
) -> ModelingOperationSpec {
    ModelingOperationSpec::RecessedPanelCut {
        operation: OperationId(operation),
        region: FRONT_REGION,
        face: PlanarCutFace::PositiveY,
        center,
        size: [0.36, 0.22],
        depth: 0.058,
        corner_radius: 0.025,
        rim_width: 0.022,
        corner_segments: 3,
        entry_loop: BoundaryLoopId(entry_loop),
        floor_loop: BoundaryLoopId(floor_loop),
        outer_region: FRONT_REGION,
        rim_region: RegionId(rim_region),
        wall_region: RegionId(wall_region),
        floor_region: RegionId(floor_region),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    }
}

fn rectangular_vent(
    operation: u64,
    center: [f32; 2],
    entry_loop: u64,
    exit_loop: u64,
    rim_region: u64,
    wall_region: u64,
) -> ModelingOperationSpec {
    ModelingOperationSpec::RectangularThroughCut {
        operation: OperationId(operation),
        region: FRONT_REGION,
        face: PlanarCutFace::PositiveY,
        center,
        size: [0.18, 0.045],
        corner_radius: 0.012,
        rim_width: 0.016,
        corner_segments: 2,
        entry_loop: BoundaryLoopId(entry_loop),
        exit_loop: BoundaryLoopId(exit_loop),
        outer_region: FRONT_REGION,
        rim_region: RegionId(rim_region),
        wall_region: RegionId(wall_region),
        edge_treatment: CutEdgeTreatment::Hard,
    }
}

fn mount_hole(
    operation: u64,
    center: [f32; 2],
    entry_loop: u64,
    exit_loop: u64,
    rim_region: u64,
    wall_region: u64,
) -> ModelingOperationSpec {
    ModelingOperationSpec::CircularThroughCut {
        operation: OperationId(operation),
        region: FRONT_REGION,
        face: PlanarCutFace::PositiveY,
        center,
        radius: 0.04,
        radial_segments: 14,
        rim_width: 0.016,
        entry_loop: BoundaryLoopId(entry_loop),
        exit_loop: BoundaryLoopId(exit_loop),
        outer_region: FRONT_REGION,
        rim_region: RegionId(rim_region),
        wall_region: RegionId(wall_region),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    }
}

fn boundary_bevel(
    operation: u64,
    target_loop: u64,
    bevel_region: u64,
    outer_loop: u64,
    inner_loop: u64,
    width: f32,
) -> ModelingOperationSpec {
    ModelingOperationSpec::BevelBoundaryLoop {
        operation: OperationId(operation),
        target_loop: BoundaryLoopId(target_loop),
        width,
        segments: 2,
        profile: 1.0,
        bevel_region: RegionId(bevel_region),
        outer_replacement_loop: BoundaryLoopId(outer_loop),
        inner_replacement_loop: BoundaryLoopId(inner_loop),
    }
}

fn body_regions(vent_count: usize) -> BTreeMap<RegionId, SurfaceRegionSpec> {
    let _ = vent_count;
    BTreeMap::from([(
        FRONT_REGION,
        surface_region(0, "front armored shell", SurfaceRole::PrimarySurface),
    )])
}

fn surface_region(id: u64, name: &str, role: SurfaceRole) -> SurfaceRegionSpec {
    SurfaceRegionSpec {
        id: RegionId(id),
        name: name.to_owned(),
        role,
        tags: BTreeSet::from([name.replace(' ', "_")]),
    }
}

fn fastener_fragment() -> RecipeFragment {
    let mut fragment = cylinder_fragment(
        "corner_fastener",
        "fastener",
        0.035,
        0.038,
        18,
        [-0.77, 0.625, -0.62],
        vec![linear_array(1, 8, [0.14, 0.0, 0.0])],
    );
    let recipe = &mut fragment.recipe;
    recipe.parameters.insert(
        ParameterId(4),
        scalar_parameter(
            4,
            shape_asset::definition_scalar_path(
                crate::LOCAL_DEFINITION,
                "operation.1.linear_array.count",
            ),
            "Fastener count",
            4.0,
            12.0,
            1.0,
            true,
        ),
    );
    recipe.variation.count_ranges.insert(
        OperationId(1),
        CountRangeHint {
            minimum: 4,
            maximum: 12,
        },
    );
    recipe.next_ids.parameter = 5;
    assert!(validate_asset_recipe(recipe).is_valid());
    fragment
}
