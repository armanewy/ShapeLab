#![forbid(unsafe_code)]

//! Benchmark explicit asset recipes for the modeling kernel.

use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetId, AssetRecipe, AssetRelationshipPolicy, BoundaryLoopId, CountRangeHint,
    CutEdgeTreatment, CutGroupRole, Frame3, GeometryRecipe, GeometrySource, ModelingOperationSpec,
    OperationId, ParameterDescriptor, ParameterId, PartDefinition, PartDefinitionId, PartInstance,
    PartInstanceId, PlanarCutFace, RegionId, SemanticCutGroupHint, SocketId, SocketSpec,
    SurfaceRegionSpec, SurfaceRole, Transform3,
};

/// Built-in explicit modeling benchmark asset.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BenchmarkAsset {
    /// Industrial crate benchmark.
    IndustrialCrate,
    /// Semantic multi-cut panel benchmark.
    MultiCutPanel,
    /// Explicit desk lamp benchmark.
    ExplicitDeskLamp,
    /// Stylized stool benchmark.
    StylizedStool,
}

impl BenchmarkAsset {
    /// Parse the CLI/display slug.
    #[must_use]
    pub fn parse(slug: &str) -> Option<Self> {
        match slug {
            "industrial-crate" => Some(Self::IndustrialCrate),
            "multi-cut-panel" => Some(Self::MultiCutPanel),
            "explicit-desk-lamp" => Some(Self::ExplicitDeskLamp),
            "stylized-stool" => Some(Self::StylizedStool),
            _ => None,
        }
    }

    /// Stable slug.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::IndustrialCrate => "industrial-crate",
            Self::MultiCutPanel => "multi-cut-panel",
            Self::ExplicitDeskLamp => "explicit-desk-lamp",
            Self::StylizedStool => "stylized-stool",
        }
    }

    /// Build the asset recipe.
    #[must_use]
    pub fn recipe(self) -> AssetRecipe {
        match self {
            Self::IndustrialCrate => industrial_crate_recipe(),
            Self::MultiCutPanel => multi_cut_panel_recipe(),
            Self::ExplicitDeskLamp => explicit_desk_lamp_recipe(),
            Self::StylizedStool => stylized_stool_recipe(),
        }
    }
}

/// Return every bundled benchmark asset.
#[must_use]
pub const fn benchmark_assets() -> [BenchmarkAsset; 4] {
    [
        BenchmarkAsset::IndustrialCrate,
        BenchmarkAsset::MultiCutPanel,
        BenchmarkAsset::ExplicitDeskLamp,
        BenchmarkAsset::StylizedStool,
    ]
}

/// Build the industrial crate recipe.
#[must_use]
pub fn industrial_crate_recipe() -> AssetRecipe {
    let mut recipe = AssetRecipe::new(AssetId(1001), "Industrial Crate");
    recipe.definitions.insert(
        PartDefinitionId(1),
        definition_with_regions(
            1,
            "rounded crate body",
            GeometrySource::RoundedBox {
                half_extents: [2.0, 1.0, 1.2],
                radius: 0.14,
            },
            vec![
                ModelingOperationSpec::AddPanel {
                    operation: OperationId(7),
                    region: RegionId(1),
                    inset: 0.14,
                    depth: 0.035,
                },
                ModelingOperationSpec::AddTrim {
                    operation: OperationId(8),
                    region: RegionId(2),
                    width: 0.08,
                    height: 0.045,
                },
                ModelingOperationSpec::RecessedPanelCut {
                    operation: OperationId(15),
                    region: RegionId(1),
                    face: PlanarCutFace::PositiveZ,
                    center: [0.0, 0.0],
                    size: [2.38, 0.48],
                    depth: 0.045,
                    corner_radius: 0.075,
                    rim_width: 0.0768,
                    corner_segments: 4,
                    entry_loop: BoundaryLoopId(30),
                    floor_loop: BoundaryLoopId(31),
                    outer_region: RegionId(1),
                    rim_region: RegionId(60),
                    wall_region: RegionId(61),
                    floor_region: RegionId(62),
                    edge_treatment: CutEdgeTreatment::BevelEligible,
                },
                ModelingOperationSpec::RectangularThroughCut {
                    operation: OperationId(18),
                    region: RegionId(1),
                    face: PlanarCutFace::PositiveZ,
                    center: [-0.62, 0.62],
                    size: [0.34, 0.05],
                    corner_radius: 0.0,
                    rim_width: 0.035,
                    corner_segments: 1,
                    entry_loop: BoundaryLoopId(36),
                    exit_loop: BoundaryLoopId(37),
                    outer_region: RegionId(1),
                    rim_region: RegionId(65),
                    wall_region: RegionId(66),
                    edge_treatment: CutEdgeTreatment::Hard,
                },
                ModelingOperationSpec::RectangularThroughCut {
                    operation: OperationId(19),
                    region: RegionId(1),
                    face: PlanarCutFace::PositiveZ,
                    center: [0.0, 0.62],
                    size: [0.34, 0.05],
                    corner_radius: 0.0,
                    rim_width: 0.035,
                    corner_segments: 1,
                    entry_loop: BoundaryLoopId(38),
                    exit_loop: BoundaryLoopId(39),
                    outer_region: RegionId(1),
                    rim_region: RegionId(67),
                    wall_region: RegionId(68),
                    edge_treatment: CutEdgeTreatment::Hard,
                },
                ModelingOperationSpec::RectangularThroughCut {
                    operation: OperationId(20),
                    region: RegionId(1),
                    face: PlanarCutFace::PositiveZ,
                    center: [0.62, 0.62],
                    size: [0.34, 0.05],
                    corner_radius: 0.0,
                    rim_width: 0.035,
                    corner_segments: 1,
                    entry_loop: BoundaryLoopId(40),
                    exit_loop: BoundaryLoopId(41),
                    outer_region: RegionId(1),
                    rim_region: RegionId(69),
                    wall_region: RegionId(70),
                    edge_treatment: CutEdgeTreatment::Hard,
                },
                ModelingOperationSpec::CircularThroughCut {
                    operation: OperationId(21),
                    region: RegionId(1),
                    face: PlanarCutFace::PositiveZ,
                    center: [-0.52, -0.62],
                    radius: 0.065,
                    radial_segments: 12,
                    rim_width: 0.035,
                    entry_loop: BoundaryLoopId(42),
                    exit_loop: BoundaryLoopId(43),
                    outer_region: RegionId(1),
                    rim_region: RegionId(71),
                    wall_region: RegionId(72),
                    edge_treatment: CutEdgeTreatment::BevelEligible,
                },
                ModelingOperationSpec::CircularThroughCut {
                    operation: OperationId(22),
                    region: RegionId(1),
                    face: PlanarCutFace::PositiveZ,
                    center: [0.52, -0.62],
                    radius: 0.065,
                    radial_segments: 12,
                    rim_width: 0.035,
                    entry_loop: BoundaryLoopId(44),
                    exit_loop: BoundaryLoopId(45),
                    outer_region: RegionId(1),
                    rim_region: RegionId(73),
                    wall_region: RegionId(74),
                    edge_treatment: CutEdgeTreatment::BevelEligible,
                },
                ModelingOperationSpec::SetBevelProfile {
                    operation: OperationId(1),
                    radius: 0.14,
                    segments: 3,
                },
                ModelingOperationSpec::BevelBoundaryLoop {
                    operation: OperationId(16),
                    target_loop: BoundaryLoopId(30),
                    width: 0.022,
                    segments: 2,
                    profile: 1.15,
                    bevel_region: RegionId(63),
                    outer_replacement_loop: BoundaryLoopId(32),
                    inner_replacement_loop: BoundaryLoopId(33),
                },
                ModelingOperationSpec::BevelBoundaryLoop {
                    operation: OperationId(17),
                    target_loop: BoundaryLoopId(31),
                    width: 0.018,
                    segments: 2,
                    profile: 1.1,
                    bevel_region: RegionId(64),
                    outer_replacement_loop: BoundaryLoopId(34),
                    inner_replacement_loop: BoundaryLoopId(35),
                },
                ModelingOperationSpec::BevelBoundaryLoop {
                    operation: OperationId(23),
                    target_loop: BoundaryLoopId(42),
                    width: 0.018,
                    segments: 2,
                    profile: 1.0,
                    bevel_region: RegionId(75),
                    outer_replacement_loop: BoundaryLoopId(46),
                    inner_replacement_loop: BoundaryLoopId(47),
                },
                ModelingOperationSpec::BevelBoundaryLoop {
                    operation: OperationId(24),
                    target_loop: BoundaryLoopId(44),
                    width: 0.018,
                    segments: 2,
                    profile: 1.0,
                    bevel_region: RegionId(76),
                    outer_replacement_loop: BoundaryLoopId(48),
                    inner_replacement_loop: BoundaryLoopId(49),
                },
            ],
            rounded_box_regions(),
            BTreeMap::new(),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(2),
        definition_with_regions(
            2,
            "rubber foot",
            GeometrySource::RoundedBox {
                half_extents: [0.28, 0.18, 0.28],
                radius: 0.05,
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(2),
                radius: 0.05,
                segments: 2,
            }],
            rounded_box_regions(),
            BTreeMap::new(),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(3),
        definition_with_regions(
            3,
            "recessed crate panel",
            GeometrySource::Plate {
                size: [3.25, 0.82],
                thickness: 0.10,
            },
            vec![ModelingOperationSpec::RecessedPanelCut {
                operation: OperationId(13),
                region: RegionId(1),
                face: PlanarCutFace::PositiveY,
                center: [0.0, 0.0],
                size: [2.38, 0.48],
                depth: 0.045,
                corner_radius: 0.075,
                rim_width: 0.0768,
                corner_segments: 4,
                entry_loop: BoundaryLoopId(1),
                floor_loop: BoundaryLoopId(2),
                outer_region: RegionId(1),
                rim_region: RegionId(20),
                wall_region: RegionId(21),
                floor_region: RegionId(22),
                edge_treatment: CutEdgeTreatment::BevelEligible,
            }],
            plate_regions(),
            BTreeMap::new(),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(4),
        definition_with_regions(
            4,
            "side handle",
            GeometrySource::Sweep {
                profile: regular_profile(0.085, 8),
                path: vec![
                    frame([0.0, -0.42, 0.0]),
                    frame([0.0, -0.16, 0.22]),
                    frame([0.0, 0.16, 0.22]),
                    frame([0.0, 0.42, 0.0]),
                ],
            },
            vec![ModelingOperationSpec::MirrorInstances {
                operation: OperationId(4),
                plane_normal: [1.0, 0.0, 0.0],
                plane_offset: 0.0,
            }],
            sweep_regions(),
            sockets([
                (SocketId(1), "handle lower mount", [0.0, -0.42, 0.0]),
                (SocketId(2), "handle upper mount", [0.0, 0.42, 0.0]),
            ]),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(5),
        definition_with_regions(
            5,
            "panel fastener",
            GeometrySource::Cylinder {
                radius: 0.075,
                height: 0.10,
                radial_segments: 16,
            },
            vec![
                ModelingOperationSpec::SetBevelProfile {
                    operation: OperationId(5),
                    radius: 0.012,
                    segments: 1,
                },
                ModelingOperationSpec::LinearArray {
                    operation: OperationId(6),
                    count: 6,
                    offset: [0.56, 0.0, 0.0],
                },
            ],
            cylinder_regions(),
            BTreeMap::new(),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(6),
        definition_with_regions(
            6,
            "corner reinforcement trim",
            GeometrySource::RoundedBox {
                half_extents: [0.08, 0.86, 0.08],
                radius: 0.025,
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(9),
                radius: 0.025,
                segments: 1,
            }],
            rounded_box_regions(),
            BTreeMap::new(),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(7),
        definition_with_regions(
            7,
            "top ventilation slat",
            GeometrySource::Plate {
                size: [0.84, 0.08],
                thickness: 0.045,
            },
            vec![
                ModelingOperationSpec::RectangularThroughCut {
                    operation: OperationId(14),
                    region: RegionId(1),
                    face: PlanarCutFace::PositiveY,
                    center: [0.0, 0.0],
                    size: [0.42, 0.032],
                    corner_radius: 0.006,
                    rim_width: 0.00512,
                    corner_segments: 4,
                    entry_loop: BoundaryLoopId(3),
                    exit_loop: BoundaryLoopId(4),
                    outer_region: RegionId(1),
                    rim_region: RegionId(23),
                    wall_region: RegionId(24),
                    edge_treatment: CutEdgeTreatment::Hard,
                },
                ModelingOperationSpec::LinearArray {
                    operation: OperationId(11),
                    count: 4,
                    offset: [0.0, 0.0, 0.18],
                },
            ],
            plate_regions(),
            BTreeMap::new(),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(8),
        definition_with_regions(
            8,
            "lower skid rail trim",
            GeometrySource::RoundedBox {
                half_extents: [1.50, 0.055, 0.055],
                radius: 0.018,
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(12),
                radius: 0.018,
                segments: 1,
            }],
            rounded_box_regions(),
            BTreeMap::new(),
        ),
    );

    add_instance(&mut recipe, 1, 1, "crate body", None, Transform3::default());
    for (id, x, z) in [
        (2, -1.55, -0.82),
        (3, 1.55, -0.82),
        (4, -1.55, 0.82),
        (5, 1.55, 0.82),
    ] {
        add_instance(
            &mut recipe,
            id,
            2,
            &format!("foot {id}"),
            Some(PartInstanceId(1)),
            transform([x, -1.18, z], [0.0, 0.0, 0.0]),
        );
    }
    add_instance(
        &mut recipe,
        6,
        3,
        "front recessed panel",
        Some(PartInstanceId(1)),
        transform([0.0, 0.06, 1.27], [90.0, 0.0, 0.0]),
    );
    add_instance(
        &mut recipe,
        7,
        3,
        "back recessed panel",
        Some(PartInstanceId(1)),
        transform([0.0, 0.06, -1.27], [-90.0, 0.0, 0.0]),
    );
    add_instance(
        &mut recipe,
        8,
        4,
        "left swept side handle",
        Some(PartInstanceId(1)),
        transform([-2.13, 0.06, 0.0], [0.0, 0.0, 0.0]),
    );
    add_instance(
        &mut recipe,
        9,
        5,
        "front fastener row",
        Some(PartInstanceId(6)),
        transform([-1.4, 0.55, 1.385], [90.0, 0.0, 0.0]),
    );
    add_instance(
        &mut recipe,
        10,
        5,
        "back fastener row",
        Some(PartInstanceId(7)),
        transform([-1.4, 0.55, -1.385], [-90.0, 0.0, 0.0]),
    );
    for (id, x, z, name) in [
        (11, -2.11, -1.31, "back left corner reinforcement"),
        (12, 2.11, -1.31, "back right corner reinforcement"),
        (13, -2.11, 1.31, "front left corner reinforcement"),
        (14, 2.11, 1.31, "front right corner reinforcement"),
    ] {
        add_instance(
            &mut recipe,
            id,
            6,
            name,
            Some(PartInstanceId(1)),
            transform([x, 0.02, z], [0.0, 0.0, 0.0]),
        );
    }
    add_instance(
        &mut recipe,
        15,
        7,
        "optional top ventilation slat array",
        Some(PartInstanceId(1)),
        transform([0.0, 1.055, -0.27], [0.0, 0.0, 0.0]),
    );
    add_instance(
        &mut recipe,
        16,
        8,
        "front lower skid rail trim",
        Some(PartInstanceId(1)),
        transform([0.0, -1.075, 1.32], [0.0, 0.0, 0.0]),
    );
    add_instance(
        &mut recipe,
        17,
        8,
        "back lower skid rail trim",
        Some(PartInstanceId(1)),
        transform([0.0, -1.075, -1.32], [0.0, 0.0, 0.0]),
    );
    recipe
        .variation
        .optional_instances
        .insert(PartInstanceId(15));
    recipe.variation.count_ranges.insert(
        OperationId(6),
        shape_asset::CountRangeHint {
            minimum: 4,
            maximum: 8,
        },
    );
    recipe.variation.count_ranges.insert(
        OperationId(11),
        shape_asset::CountRangeHint {
            minimum: 2,
            maximum: 6,
        },
    );
    recipe
        .variation
        .optional_instances
        .insert(PartInstanceId(16));
    recipe
        .variation
        .optional_instances
        .insert(PartInstanceId(17));
    recipe.parameters.insert(
        ParameterId(1),
        parameter(
            1,
            "Body width",
            "Size",
            "definition.1.geometry.rounded_box.half_extents.x",
            1.65,
            2.45,
            0.05,
        ),
    );
    recipe.parameters.insert(
        ParameterId(2),
        parameter(
            2,
            "Body height",
            "Size",
            "definition.1.geometry.rounded_box.half_extents.y",
            0.78,
            1.25,
            0.04,
        ),
    );
    recipe.parameters.insert(
        ParameterId(3),
        parameter(
            3,
            "Body depth",
            "Proportions",
            "definition.1.geometry.rounded_box.half_extents.z",
            0.95,
            1.45,
            0.04,
        ),
    );
    recipe.parameters.insert(
        ParameterId(4),
        parameter(
            4,
            "Body bevel radius",
            "Edge Softness",
            "definition.1.operation.1.bevel.radius",
            0.08,
            0.18,
            0.01,
        ),
    );
    recipe.parameters.insert(
        ParameterId(5),
        parameter(
            5,
            "Handle thickness",
            "Size",
            "definition.4.geometry.sweep.profile.0.x",
            0.06,
            0.13,
            0.005,
        ),
    );
    recipe.parameters.insert(
        ParameterId(6),
        topology_parameter(
            6,
            "Bolt count",
            "Repetition",
            "definition.5.operation.6.linear_array.count",
            4.0,
            8.0,
            1.0,
        ),
    );
    recipe.parameters.insert(
        ParameterId(7),
        parameter(
            7,
            "Panel edge rounding",
            "Detail Density",
            "definition.1.operation.16.bevel_boundary_loop.width",
            0.012,
            0.036,
            0.002,
        ),
    );
    recipe.parameters.insert(
        ParameterId(8),
        parameter(
            8,
            "Skid rail height",
            "Size",
            "definition.8.geometry.rounded_box.half_extents.y",
            0.035,
            0.085,
            0.005,
        ),
    );
    recipe.parameters.insert(
        ParameterId(9),
        topology_parameter(
            9,
            "Vent slat count",
            "Repetition",
            "definition.7.operation.11.linear_array.count",
            2.0,
            6.0,
            1.0,
        ),
    );
    recipe.parameters.insert(
        ParameterId(10),
        parameter(
            10,
            "Foot width",
            "Size",
            "definition.2.geometry.rounded_box.half_extents.x",
            0.20,
            0.36,
            0.02,
        ),
    );
    recipe.parameters.insert(
        ParameterId(11),
        parameter(
            11,
            "Panel recess width",
            "Panel Cuts",
            "definition.1.operation.15.recessed_panel_cut.size.x",
            1.85,
            2.85,
            0.05,
        ),
    );
    recipe.parameters.insert(
        ParameterId(12),
        parameter(
            12,
            "Panel recess height",
            "Panel Cuts",
            "definition.1.operation.15.recessed_panel_cut.size.y",
            0.34,
            0.64,
            0.03,
        ),
    );
    recipe.parameters.insert(
        ParameterId(13),
        parameter(
            13,
            "Panel recess depth",
            "Panel Cuts",
            "definition.1.operation.15.recessed_panel_cut.depth",
            0.025,
            0.075,
            0.005,
        ),
    );
    recipe.parameters.insert(
        ParameterId(14),
        parameter(
            14,
            "Panel corner radius",
            "Panel Cuts",
            "definition.1.operation.15.recessed_panel_cut.corner_radius",
            0.04,
            0.14,
            0.01,
        ),
    );
    recipe.parameters.insert(
        ParameterId(15),
        parameter(
            15,
            "Vent opening width",
            "Vent Cuts",
            "definition.1.operation.18.rectangular_through_cut.size.x",
            0.26,
            0.62,
            0.03,
        ),
    );
    recipe.parameters.insert(
        ParameterId(16),
        parameter(
            16,
            "Vent opening height",
            "Vent Cuts",
            "definition.1.operation.18.rectangular_through_cut.size.y",
            0.035,
            0.085,
            0.005,
        ),
    );
    recipe.parameters.insert(
        ParameterId(17),
        parameter(
            17,
            "Panel rim width",
            "Panel Cuts",
            "definition.1.operation.15.recessed_panel_cut.rim_width",
            0.045,
            0.12,
            0.005,
        ),
    );
    recipe.parameters.insert(
        ParameterId(18),
        topology_parameter(
            18,
            "Panel corner segments",
            "Detail Density",
            "definition.1.operation.15.recessed_panel_cut.corner_segments",
            1.0,
            6.0,
            1.0,
        ),
    );
    recipe.parameters.insert(
        ParameterId(19),
        parameter(
            19,
            "Vent rim width",
            "Vent Cuts",
            "definition.1.operation.18.rectangular_through_cut.rim_width",
            0.018,
            0.060,
            0.003,
        ),
    );
    recipe.parameters.insert(
        ParameterId(20),
        topology_parameter(
            20,
            "Vent corner segments",
            "Detail Density",
            "definition.1.operation.18.rectangular_through_cut.corner_segments",
            1.0,
            6.0,
            1.0,
        ),
    );
    recipe.parameters.insert(
        ParameterId(21),
        parameter(
            21,
            "Left mounting hole radius",
            "Panel Cuts",
            "definition.1.operation.21.circular_through_cut.radius",
            0.045,
            0.11,
            0.005,
        ),
    );
    recipe.parameters.insert(
        ParameterId(22),
        parameter(
            22,
            "Left hole edge rounding",
            "Detail Density",
            "definition.1.operation.23.bevel_boundary_loop.width",
            0.010,
            0.028,
            0.002,
        ),
    );
    recipe.parameters.insert(
        ParameterId(23),
        parameter(
            23,
            "Right mounting hole radius",
            "Panel Cuts",
            "definition.1.operation.22.circular_through_cut.radius",
            0.045,
            0.11,
            0.005,
        ),
    );
    recipe.parameters.insert(
        ParameterId(24),
        parameter(
            24,
            "Right hole edge rounding",
            "Detail Density",
            "definition.1.operation.24.bevel_boundary_loop.width",
            0.010,
            0.028,
            0.002,
        ),
    );
    recipe.variation.semantic_cut_groups.insert(
        "body_vents".to_owned(),
        SemanticCutGroupHint {
            label: "Body vent slots".to_owned(),
            definition: PartDefinitionId(1),
            operations: vec![OperationId(18), OperationId(19), OperationId(20)],
            role: CutGroupRole::Vents,
            count_range: Some(CountRangeHint {
                minimum: 1,
                maximum: 6,
            }),
        },
    );
    recipe.variation.semantic_cut_groups.insert(
        "body_mount_holes".to_owned(),
        SemanticCutGroupHint {
            label: "Body mounting holes".to_owned(),
            definition: PartDefinitionId(1),
            operations: vec![OperationId(21), OperationId(22)],
            role: CutGroupRole::MountHoles,
            count_range: Some(CountRangeHint {
                minimum: 2,
                maximum: 6,
            }),
        },
    );
    recipe.root_instances.push(PartInstanceId(1));
    finish_ids(&mut recipe, 9, 18, 25, 3);
    recipe
}

/// Build a visible semantic multi-cut panel benchmark.
#[must_use]
pub fn multi_cut_panel_recipe() -> AssetRecipe {
    let mut recipe = AssetRecipe::new(AssetId(1004), "Multi-Cut Panel");
    recipe.definitions.insert(
        PartDefinitionId(1),
        definition_with_regions(
            1,
            "semantic multi-cut plate",
            GeometrySource::Plate {
                size: [4.0, 3.4],
                thickness: 0.30,
            },
            vec![
                ModelingOperationSpec::RecessedPanelCut {
                    operation: OperationId(1),
                    region: RegionId(1),
                    face: PlanarCutFace::PositiveY,
                    center: [-1.40, 0.0],
                    size: [0.55, 0.44],
                    depth: 0.08,
                    corner_radius: 0.08,
                    rim_width: 0.07,
                    corner_segments: 4,
                    entry_loop: BoundaryLoopId(1),
                    floor_loop: BoundaryLoopId(2),
                    outer_region: RegionId(1),
                    rim_region: RegionId(10),
                    wall_region: RegionId(11),
                    floor_region: RegionId(12),
                    edge_treatment: CutEdgeTreatment::BevelEligible,
                },
                ModelingOperationSpec::CircularThroughCut {
                    operation: OperationId(2),
                    region: RegionId(1),
                    face: PlanarCutFace::PositiveY,
                    center: [-0.45, -0.72],
                    radius: 0.08,
                    radial_segments: 12,
                    rim_width: 0.04,
                    entry_loop: BoundaryLoopId(3),
                    exit_loop: BoundaryLoopId(4),
                    outer_region: RegionId(1),
                    rim_region: RegionId(20),
                    wall_region: RegionId(21),
                    edge_treatment: CutEdgeTreatment::BevelEligible,
                },
                ModelingOperationSpec::CircularThroughCut {
                    operation: OperationId(3),
                    region: RegionId(1),
                    face: PlanarCutFace::PositiveY,
                    center: [0.15, -0.72],
                    radius: 0.08,
                    radial_segments: 12,
                    rim_width: 0.04,
                    entry_loop: BoundaryLoopId(5),
                    exit_loop: BoundaryLoopId(6),
                    outer_region: RegionId(1),
                    rim_region: RegionId(22),
                    wall_region: RegionId(23),
                    edge_treatment: CutEdgeTreatment::BevelEligible,
                },
                ModelingOperationSpec::CircularThroughCut {
                    operation: OperationId(4),
                    region: RegionId(1),
                    face: PlanarCutFace::PositiveY,
                    center: [0.75, -0.72],
                    radius: 0.08,
                    radial_segments: 12,
                    rim_width: 0.04,
                    entry_loop: BoundaryLoopId(7),
                    exit_loop: BoundaryLoopId(8),
                    outer_region: RegionId(1),
                    rim_region: RegionId(24),
                    wall_region: RegionId(25),
                    edge_treatment: CutEdgeTreatment::BevelEligible,
                },
                ModelingOperationSpec::CircularThroughCut {
                    operation: OperationId(5),
                    region: RegionId(1),
                    face: PlanarCutFace::PositiveY,
                    center: [1.35, -0.72],
                    radius: 0.08,
                    radial_segments: 12,
                    rim_width: 0.04,
                    entry_loop: BoundaryLoopId(9),
                    exit_loop: BoundaryLoopId(10),
                    outer_region: RegionId(1),
                    rim_region: RegionId(26),
                    wall_region: RegionId(27),
                    edge_treatment: CutEdgeTreatment::BevelEligible,
                },
                ModelingOperationSpec::RectangularThroughCut {
                    operation: OperationId(6),
                    region: RegionId(1),
                    face: PlanarCutFace::PositiveY,
                    center: [-0.45, 0.70],
                    size: [0.24, 0.05],
                    corner_radius: 0.01,
                    rim_width: 0.04,
                    corner_segments: 3,
                    entry_loop: BoundaryLoopId(11),
                    exit_loop: BoundaryLoopId(12),
                    outer_region: RegionId(1),
                    rim_region: RegionId(30),
                    wall_region: RegionId(31),
                    edge_treatment: CutEdgeTreatment::Hard,
                },
                ModelingOperationSpec::RectangularThroughCut {
                    operation: OperationId(7),
                    region: RegionId(1),
                    face: PlanarCutFace::PositiveY,
                    center: [0.15, 0.70],
                    size: [0.24, 0.05],
                    corner_radius: 0.01,
                    rim_width: 0.04,
                    corner_segments: 3,
                    entry_loop: BoundaryLoopId(13),
                    exit_loop: BoundaryLoopId(14),
                    outer_region: RegionId(1),
                    rim_region: RegionId(32),
                    wall_region: RegionId(33),
                    edge_treatment: CutEdgeTreatment::Hard,
                },
                ModelingOperationSpec::RectangularThroughCut {
                    operation: OperationId(8),
                    region: RegionId(1),
                    face: PlanarCutFace::PositiveY,
                    center: [0.75, 0.70],
                    size: [0.24, 0.05],
                    corner_radius: 0.01,
                    rim_width: 0.04,
                    corner_segments: 3,
                    entry_loop: BoundaryLoopId(15),
                    exit_loop: BoundaryLoopId(16),
                    outer_region: RegionId(1),
                    rim_region: RegionId(34),
                    wall_region: RegionId(35),
                    edge_treatment: CutEdgeTreatment::Hard,
                },
                ModelingOperationSpec::BevelBoundaryLoop {
                    operation: OperationId(9),
                    target_loop: BoundaryLoopId(1),
                    width: 0.022,
                    segments: 2,
                    profile: 1.0,
                    bevel_region: RegionId(40),
                    outer_replacement_loop: BoundaryLoopId(17),
                    inner_replacement_loop: BoundaryLoopId(18),
                },
                ModelingOperationSpec::BevelBoundaryLoop {
                    operation: OperationId(10),
                    target_loop: BoundaryLoopId(2),
                    width: 0.018,
                    segments: 2,
                    profile: 1.0,
                    bevel_region: RegionId(41),
                    outer_replacement_loop: BoundaryLoopId(19),
                    inner_replacement_loop: BoundaryLoopId(20),
                },
                ModelingOperationSpec::BevelBoundaryLoop {
                    operation: OperationId(11),
                    target_loop: BoundaryLoopId(3),
                    width: 0.015,
                    segments: 2,
                    profile: 1.0,
                    bevel_region: RegionId(42),
                    outer_replacement_loop: BoundaryLoopId(21),
                    inner_replacement_loop: BoundaryLoopId(22),
                },
                ModelingOperationSpec::BevelBoundaryLoop {
                    operation: OperationId(12),
                    target_loop: BoundaryLoopId(5),
                    width: 0.015,
                    segments: 2,
                    profile: 1.0,
                    bevel_region: RegionId(43),
                    outer_replacement_loop: BoundaryLoopId(23),
                    inner_replacement_loop: BoundaryLoopId(24),
                },
                ModelingOperationSpec::BevelBoundaryLoop {
                    operation: OperationId(13),
                    target_loop: BoundaryLoopId(7),
                    width: 0.015,
                    segments: 2,
                    profile: 1.0,
                    bevel_region: RegionId(44),
                    outer_replacement_loop: BoundaryLoopId(25),
                    inner_replacement_loop: BoundaryLoopId(26),
                },
                ModelingOperationSpec::BevelBoundaryLoop {
                    operation: OperationId(14),
                    target_loop: BoundaryLoopId(9),
                    width: 0.015,
                    segments: 2,
                    profile: 1.0,
                    bevel_region: RegionId(45),
                    outer_replacement_loop: BoundaryLoopId(27),
                    inner_replacement_loop: BoundaryLoopId(28),
                },
            ],
            plate_regions(),
            BTreeMap::new(),
        ),
    );
    add_instance(
        &mut recipe,
        1,
        1,
        "multi-cut panel",
        None,
        Transform3::default(),
    );
    recipe.parameters.insert(
        ParameterId(1),
        parameter(
            1,
            "Recess width",
            "Panel Cuts",
            "definition.1.operation.1.recessed_panel_cut.size.x",
            0.38,
            0.78,
            0.02,
        ),
    );
    recipe.parameters.insert(
        ParameterId(2),
        parameter(
            2,
            "Recess height",
            "Panel Cuts",
            "definition.1.operation.1.recessed_panel_cut.size.y",
            0.30,
            0.58,
            0.02,
        ),
    );
    recipe.parameters.insert(
        ParameterId(3),
        parameter(
            3,
            "Recess depth",
            "Panel Cuts",
            "definition.1.operation.1.recessed_panel_cut.depth",
            0.045,
            0.12,
            0.005,
        ),
    );
    recipe.parameters.insert(
        ParameterId(4),
        parameter(
            4,
            "Recess corner radius",
            "Panel Cuts",
            "definition.1.operation.1.recessed_panel_cut.corner_radius",
            0.025,
            0.12,
            0.005,
        ),
    );
    recipe.parameters.insert(
        ParameterId(5),
        parameter(
            5,
            "Mount hole radius",
            "Hole Cuts",
            "definition.1.operation.2.circular_through_cut.radius",
            0.045,
            0.12,
            0.005,
        ),
    );
    recipe.parameters.insert(
        ParameterId(6),
        topology_parameter(
            6,
            "Mount hole segments",
            "Detail Density",
            "definition.1.operation.2.circular_through_cut.radial_segments",
            8.0,
            20.0,
            2.0,
        ),
    );
    recipe.parameters.insert(
        ParameterId(7),
        parameter(
            7,
            "Vent width",
            "Vent Cuts",
            "definition.1.operation.6.rectangular_through_cut.size.x",
            0.16,
            0.36,
            0.01,
        ),
    );
    recipe.parameters.insert(
        ParameterId(8),
        parameter(
            8,
            "Vent height",
            "Vent Cuts",
            "definition.1.operation.6.rectangular_through_cut.size.y",
            0.032,
            0.08,
            0.004,
        ),
    );
    recipe.parameters.insert(
        ParameterId(9),
        parameter(
            9,
            "Vent spacing",
            "Vent Cuts",
            "definition.1.operation.7.rectangular_through_cut.center.x",
            -0.05,
            0.35,
            0.02,
        ),
    );
    recipe.variation.semantic_cut_groups.insert(
        "mount_holes".to_owned(),
        SemanticCutGroupHint {
            label: "Mounting holes".to_owned(),
            definition: PartDefinitionId(1),
            operations: vec![
                OperationId(2),
                OperationId(3),
                OperationId(4),
                OperationId(5),
            ],
            role: CutGroupRole::MountHoles,
            count_range: Some(CountRangeHint {
                minimum: 2,
                maximum: 8,
            }),
        },
    );
    recipe.variation.semantic_cut_groups.insert(
        "vents".to_owned(),
        SemanticCutGroupHint {
            label: "Vent slots".to_owned(),
            definition: PartDefinitionId(1),
            operations: vec![OperationId(6), OperationId(7), OperationId(8)],
            role: CutGroupRole::Vents,
            count_range: Some(CountRangeHint {
                minimum: 1,
                maximum: 6,
            }),
        },
    );
    recipe.root_instances.push(PartInstanceId(1));
    finish_ids(&mut recipe, 2, 2, 15, 1);
    recipe
}

/// Build the explicit desk lamp recipe.
#[must_use]
pub fn explicit_desk_lamp_recipe() -> AssetRecipe {
    let mut recipe = AssetRecipe::new(AssetId(1002), "Explicit Desk Lamp");
    recipe.definitions.insert(
        PartDefinitionId(1),
        definition_with_regions(
            1,
            "lathed weighted base",
            GeometrySource::Lathe {
                profile: vec![
                    [0.0, -0.14],
                    [0.88, -0.14],
                    [0.88, -0.08],
                    [0.76, 0.02],
                    [0.58, 0.10],
                    [0.28, 0.18],
                    [0.0, 0.18],
                ],
                segments: 48,
            },
            Vec::new(),
            lathe_regions(),
            sockets([(SocketId(1), "base top pivot", [0.0, 0.18, 0.0])]),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(2),
        definition_with_regions(
            2,
            "swept angled stem",
            GeometrySource::Sweep {
                profile: regular_profile(0.055, 8),
                path: vec![
                    frame([0.0, 0.0, 0.0]),
                    frame([0.12, 0.62, 0.0]),
                    frame([0.56, 1.30, 0.0]),
                ],
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(2),
                radius: 0.006,
                segments: 1,
            }],
            sweep_regions(),
            sockets([
                (SocketId(1), "stem lower pivot", [0.0, 0.0, 0.0]),
                (SocketId(2), "stem upper pivot", [0.56, 1.30, 0.0]),
            ]),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(3),
        definition_with_regions(
            3,
            "pivot cylinder joint",
            GeometrySource::Cylinder {
                radius: 0.16,
                height: 0.28,
                radial_segments: 24,
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(1),
                radius: 0.025,
                segments: 1,
            }],
            cylinder_regions(),
            sockets([(SocketId(1), "joint axis pivot", [0.0, 0.0, 0.0])]),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(4),
        definition_with_regions(
            4,
            "lathed shade",
            GeometrySource::Lathe {
                profile: vec![
                    [0.0, 0.0],
                    [0.30, 0.0],
                    [0.52, 0.08],
                    [0.76, 0.30],
                    [0.69, 0.54],
                    [0.35, 0.50],
                    [0.0, 0.50],
                ],
                segments: 48,
            },
            Vec::new(),
            lathe_regions(),
            sockets([(SocketId(1), "shade neck socket", [0.0, 0.02, 0.0])]),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(5),
        definition_with_regions(
            5,
            "support bracket",
            GeometrySource::Sweep {
                profile: regular_profile(0.035, 8),
                path: vec![
                    frame([0.0, 0.0, 0.0]),
                    frame([0.14, -0.05, 0.0]),
                    frame([0.28, -0.19, 0.0]),
                ],
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(3),
                radius: 0.004,
                segments: 1,
            }],
            sweep_regions(),
            sockets([
                (SocketId(1), "bracket pivot socket", [0.0, 0.0, 0.0]),
                (SocketId(2), "bracket shade socket", [0.28, -0.19, 0.0]),
            ]),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(6),
        definition_with_regions(
            6,
            "pivot collar trim",
            GeometrySource::Cylinder {
                radius: 0.205,
                height: 0.055,
                radial_segments: 32,
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(4),
                radius: 0.012,
                segments: 1,
            }],
            cylinder_regions(),
            BTreeMap::new(),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(7),
        definition_with_regions(
            7,
            "shade rim trim",
            GeometrySource::Frustum {
                bottom_radius: 0.79,
                top_radius: 0.73,
                height: 0.055,
                radial_segments: 48,
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(5),
                radius: 0.01,
                segments: 1,
            }],
            cylinder_regions(),
            BTreeMap::new(),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(8),
        definition_with_regions(
            8,
            "base switch detail",
            GeometrySource::Cylinder {
                radius: 0.055,
                height: 0.035,
                radial_segments: 16,
            },
            vec![
                ModelingOperationSpec::SetBevelProfile {
                    operation: OperationId(6),
                    radius: 0.006,
                    segments: 1,
                },
                ModelingOperationSpec::RadialArray {
                    operation: OperationId(7),
                    count: 3,
                    axis: [0.0, 1.0, 0.0],
                    angle_degrees: 40.0,
                },
            ],
            cylinder_regions(),
            BTreeMap::new(),
        ),
    );

    add_instance(
        &mut recipe,
        1,
        1,
        "lathed base",
        None,
        Transform3::default(),
    );
    add_instance(
        &mut recipe,
        2,
        3,
        "lower cylinder pivot",
        Some(PartInstanceId(1)),
        transform([0.0, 0.34, 0.0], [0.0, 0.0, 90.0]),
    );
    add_instance(
        &mut recipe,
        3,
        2,
        "swept angled stem",
        Some(PartInstanceId(1)),
        transform([0.0, 0.52, 0.0], [0.0, 0.0, 0.0]),
    );
    add_instance(
        &mut recipe,
        4,
        3,
        "upper cylinder pivot",
        Some(PartInstanceId(1)),
        transform([0.62, 1.90, 0.0], [0.0, 0.0, 90.0]),
    );
    add_instance(
        &mut recipe,
        5,
        4,
        "lathed shade",
        Some(PartInstanceId(1)),
        transform([1.04, 1.56, 0.0], [0.0, 0.0, -24.0]),
    );
    add_instance(
        &mut recipe,
        6,
        5,
        "support bracket from pivot to shade",
        Some(PartInstanceId(1)),
        transform([0.66, 1.88, 0.0], [0.0, 0.0, -10.0]),
    );
    add_instance(
        &mut recipe,
        7,
        6,
        "lower pivot collar trim",
        Some(PartInstanceId(1)),
        transform([0.0, 0.34, -0.18], [0.0, 0.0, 90.0]),
    );
    add_instance(
        &mut recipe,
        8,
        6,
        "upper pivot collar trim",
        Some(PartInstanceId(1)),
        transform([0.62, 1.90, -0.18], [0.0, 0.0, 90.0]),
    );
    add_instance(
        &mut recipe,
        9,
        7,
        "shade rim trim",
        Some(PartInstanceId(1)),
        transform([1.16, 1.50, 0.0], [0.0, 0.0, -24.0]),
    );
    add_instance(
        &mut recipe,
        10,
        8,
        "optional base switch detail group",
        Some(PartInstanceId(1)),
        transform([0.46, 0.215, 0.20], [0.0, 0.0, 90.0]),
    );
    recipe
        .variation
        .optional_instances
        .insert(PartInstanceId(10));
    recipe.variation.count_ranges.insert(
        OperationId(7),
        shape_asset::CountRangeHint {
            minimum: 1,
            maximum: 5,
        },
    );
    recipe.parameters.insert(
        ParameterId(1),
        parameter(
            1,
            "Base radius",
            "Size",
            "definition.1.geometry.lathe.profile.1.x",
            0.72,
            1.02,
            0.03,
        ),
    );
    recipe.parameters.insert(
        ParameterId(2),
        parameter(
            2,
            "Stem sweep radius",
            "Size",
            "definition.2.geometry.sweep.profile.0.x",
            0.045,
            0.075,
            0.005,
        ),
    );
    recipe.parameters.insert(
        ParameterId(3),
        parameter(
            3,
            "Stem reach",
            "Placement",
            "definition.2.geometry.sweep.path.2.origin.x",
            0.42,
            0.74,
            0.03,
        ),
    );
    recipe.parameters.insert(
        ParameterId(4),
        parameter(
            4,
            "Shade flare",
            "Curvature",
            "definition.4.geometry.lathe.profile.3.x",
            0.60,
            0.88,
            0.03,
        ),
    );
    recipe.parameters.insert(
        ParameterId(5),
        parameter(
            5,
            "Shade rim height",
            "Proportions",
            "definition.4.geometry.lathe.profile.4.y",
            0.46,
            0.64,
            0.02,
        ),
    );
    recipe.parameters.insert(
        ParameterId(6),
        parameter(
            6,
            "Pivot collar radius",
            "Size",
            "definition.6.geometry.cylinder.radius",
            0.17,
            0.25,
            0.01,
        ),
    );
    recipe.parameters.insert(
        ParameterId(7),
        topology_parameter(
            7,
            "Switch detail count",
            "Repetition",
            "definition.8.operation.7.radial_array.count",
            1.0,
            5.0,
            1.0,
        ),
    );
    recipe.parameters.insert(
        ParameterId(8),
        parameter(
            8,
            "Shade rim bevel",
            "Edge Softness",
            "definition.7.operation.5.bevel.radius",
            0.006,
            0.018,
            0.002,
        ),
    );
    recipe.root_instances.push(PartInstanceId(1));
    recipe.relationships = lamp_relationships();
    finish_ids(&mut recipe, 9, 11, 8, 3);
    recipe
}

fn lamp_relationships() -> Vec<AssetRelationshipPolicy> {
    [
        (1, 7, "lower collar nests into base pivot boss"),
        (1, 10, "base switch detail sits in the base surface"),
        (2, 7, "lower pivot and collar share a socket"),
        (3, 4, "stem terminates into upper pivot socket"),
        (3, 5, "stem passes near shade neck under the shade"),
        (3, 7, "stem lower end aligns with lower collar"),
        (3, 8, "stem upper end aligns with upper collar"),
        (3, 9, "stem routes under shade rim trim"),
        (4, 5, "upper pivot seats shade socket"),
        (4, 6, "support bracket attaches to upper pivot"),
        (4, 8, "upper collar surrounds upper pivot"),
        (4, 9, "shade rim trim surrounds upper pivot clearance"),
        (5, 6, "support bracket attaches to shade neck"),
        (5, 8, "shade clears through upper collar"),
        (
            5,
            9,
            "shade and rim trim are authored as nested shade parts",
        ),
        (6, 8, "bracket passes through upper collar"),
        (6, 9, "bracket meets shade rim trim"),
        (8, 9, "upper collar and shade rim trim meet at shade socket"),
    ]
    .into_iter()
    .map(|(first, second, reason)| {
        AssetRelationshipPolicy::may_overlap(PartInstanceId(first), PartInstanceId(second), reason)
    })
    .collect()
}

/// Build the stylized stool recipe.
#[must_use]
pub fn stylized_stool_recipe() -> AssetRecipe {
    let mut recipe = AssetRecipe::new(AssetId(1003), "Stylized Stool");
    recipe.definitions.insert(
        PartDefinitionId(1),
        definition_with_regions(
            1,
            "rounded stool seat",
            GeometrySource::RoundedBox {
                half_extents: [1.25, 0.14, 1.0],
                radius: 0.12,
            },
            vec![
                ModelingOperationSpec::AddPanel {
                    operation: OperationId(2),
                    region: RegionId(1),
                    inset: 0.10,
                    depth: 0.025,
                },
                ModelingOperationSpec::SetBevelProfile {
                    operation: OperationId(1),
                    radius: 0.12,
                    segments: 3,
                },
            ],
            rounded_box_regions(),
            sockets([(SocketId(1), "seat underside center", [0.0, -0.14, 0.0])]),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(2),
        definition_with_regions(
            2,
            "tapered stool leg",
            GeometrySource::Frustum {
                bottom_radius: 0.10,
                top_radius: 0.14,
                height: 1.28,
                radial_segments: 16,
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(3),
                radius: 0.012,
                segments: 1,
            }],
            cylinder_regions(),
            sockets([
                (SocketId(1), "leg top", [0.0, 0.64, 0.0]),
                (SocketId(2), "leg foot", [0.0, -0.64, 0.0]),
            ]),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(3),
        definition_with_regions(
            3,
            "stool foot pad",
            GeometrySource::Cylinder {
                radius: 0.18,
                height: 0.08,
                radial_segments: 20,
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(4),
                radius: 0.018,
                segments: 1,
            }],
            cylinder_regions(),
            BTreeMap::new(),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(4),
        definition_with_regions(
            4,
            "under-seat support rail",
            GeometrySource::RoundedBox {
                half_extents: [0.82, 0.045, 0.055],
                radius: 0.018,
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(5),
                radius: 0.018,
                segments: 1,
            }],
            rounded_box_regions(),
            BTreeMap::new(),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(5),
        definition_with_regions(
            5,
            "optional seat edge trim",
            GeometrySource::RoundedBox {
                half_extents: [1.05, 0.045, 0.035],
                radius: 0.016,
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(6),
                radius: 0.016,
                segments: 1,
            }],
            rounded_box_regions(),
            BTreeMap::new(),
        ),
    );

    add_instance(
        &mut recipe,
        1,
        1,
        "rounded stool seat",
        None,
        Transform3::default(),
    );
    for (id, x, z, name) in [
        (2, -0.95, -0.72, "back left tapered leg"),
        (3, 0.95, -0.72, "back right tapered leg"),
        (4, -0.95, 0.72, "front left tapered leg"),
        (5, 0.95, 0.72, "front right tapered leg"),
    ] {
        add_instance(
            &mut recipe,
            id,
            2,
            name,
            Some(PartInstanceId(1)),
            transform([x, -0.82, z], [0.0, 0.0, 0.0]),
        );
    }
    for (id, x, z, name) in [
        (6, -0.95, -0.72, "back left foot pad"),
        (7, 0.95, -0.72, "back right foot pad"),
        (8, -0.95, 0.72, "front left foot pad"),
        (9, 0.95, 0.72, "front right foot pad"),
    ] {
        add_instance(
            &mut recipe,
            id,
            3,
            name,
            Some(PartInstanceId(1)),
            transform([x, -1.51, z], [0.0, 0.0, 0.0]),
        );
    }
    add_instance(
        &mut recipe,
        10,
        4,
        "front support rail",
        Some(PartInstanceId(1)),
        transform([0.0, -0.82, 0.72], [0.0, 0.0, 0.0]),
    );
    add_instance(
        &mut recipe,
        11,
        4,
        "back support rail",
        Some(PartInstanceId(1)),
        transform([0.0, -0.82, -0.72], [0.0, 0.0, 0.0]),
    );
    add_instance(
        &mut recipe,
        12,
        5,
        "optional front seat trim",
        Some(PartInstanceId(1)),
        transform([0.0, 0.18, 1.05], [0.0, 0.0, 0.0]),
    );
    add_instance(
        &mut recipe,
        13,
        5,
        "optional back seat trim",
        Some(PartInstanceId(1)),
        transform([0.0, 0.18, -1.05], [0.0, 0.0, 0.0]),
    );
    recipe
        .variation
        .optional_instances
        .insert(PartInstanceId(12));
    recipe
        .variation
        .optional_instances
        .insert(PartInstanceId(13));
    recipe.relationships = stool_relationships();

    recipe.parameters.insert(
        ParameterId(1),
        parameter(
            1,
            "Seat width",
            "Size",
            "definition.1.geometry.rounded_box.half_extents.x",
            1.0,
            1.55,
            0.05,
        ),
    );
    recipe.parameters.insert(
        ParameterId(2),
        parameter(
            2,
            "Seat depth",
            "Proportions",
            "definition.1.geometry.rounded_box.half_extents.z",
            0.82,
            1.20,
            0.04,
        ),
    );
    recipe.parameters.insert(
        ParameterId(3),
        parameter(
            3,
            "Seat bevel radius",
            "Edge Softness",
            "definition.1.operation.1.bevel.radius",
            0.08,
            0.16,
            0.01,
        ),
    );
    recipe.parameters.insert(
        ParameterId(4),
        parameter(
            4,
            "Leg top radius",
            "Size",
            "definition.2.geometry.frustum.top_radius",
            0.10,
            0.18,
            0.01,
        ),
    );
    recipe.parameters.insert(
        ParameterId(5),
        parameter(
            5,
            "Leg bottom radius",
            "Proportions",
            "definition.2.geometry.frustum.bottom_radius",
            0.08,
            0.14,
            0.01,
        ),
    );
    recipe.parameters.insert(
        ParameterId(6),
        parameter(
            6,
            "Support rail height",
            "Placement",
            "instance.10.transform.translation.y",
            -0.96,
            -0.68,
            0.03,
        ),
    );
    recipe.parameters.insert(
        ParameterId(7),
        parameter(
            7,
            "Trim thickness",
            "Size",
            "definition.5.geometry.rounded_box.half_extents.y",
            0.025,
            0.065,
            0.005,
        ),
    );
    recipe.parameters.insert(
        ParameterId(8),
        topology_parameter(
            8,
            "Leg radial segments",
            "Detail Density",
            "definition.2.geometry.frustum.radial_segments",
            12.0,
            24.0,
            2.0,
        ),
    );

    recipe.root_instances.push(PartInstanceId(1));
    finish_ids(&mut recipe, 6, 14, 7, 3);
    recipe
}

fn stool_relationships() -> Vec<AssetRelationshipPolicy> {
    [
        (
            1,
            12,
            "front trim is seated into the rounded stool seat edge",
        ),
        (
            1,
            13,
            "back trim is seated into the rounded stool seat edge",
        ),
        (2, 11, "back support rail overlaps the back left leg tenon"),
        (3, 11, "back support rail overlaps the back right leg tenon"),
        (
            4,
            10,
            "front support rail overlaps the front left leg tenon",
        ),
        (
            5,
            10,
            "front support rail overlaps the front right leg tenon",
        ),
    ]
    .into_iter()
    .map(|(first, second, reason)| {
        AssetRelationshipPolicy::may_overlap(PartInstanceId(first), PartInstanceId(second), reason)
    })
    .collect()
}

fn definition_with_regions(
    id: u64,
    name: &str,
    source: GeometrySource,
    operations: Vec<ModelingOperationSpec>,
    regions: BTreeMap<RegionId, SurfaceRegionSpec>,
    sockets: BTreeMap<SocketId, SocketSpec>,
) -> PartDefinition {
    PartDefinition {
        id: PartDefinitionId(id),
        name: name.to_owned(),
        tags: BTreeSet::new(),
        geometry: GeometryRecipe { source, operations },
        regions,
        sockets,
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
    }
}

fn parameter(
    id: u64,
    label: &str,
    group: &str,
    path: &str,
    minimum: f32,
    maximum: f32,
    step: f32,
) -> ParameterDescriptor {
    ParameterDescriptor {
        id: ParameterId(id),
        path: path.to_owned(),
        label: label.to_owned(),
        group: group.to_owned(),
        minimum,
        maximum,
        step,
        mutation_sigma: step,
        topology_changing: false,
        beginner_description: format!("Adjust {label}."),
    }
}

fn topology_parameter(
    id: u64,
    label: &str,
    group: &str,
    path: &str,
    minimum: f32,
    maximum: f32,
    step: f32,
) -> ParameterDescriptor {
    let mut descriptor = parameter(id, label, group, path, minimum, maximum, step);
    descriptor.topology_changing = true;
    descriptor.mutation_sigma = step;
    descriptor
}

fn add_instance(
    recipe: &mut AssetRecipe,
    id: u64,
    definition: u64,
    name: &str,
    parent: Option<PartInstanceId>,
    local_transform: Transform3,
) {
    let instance = PartInstance {
        id: PartInstanceId(id),
        definition: PartDefinitionId(definition),
        name: name.to_owned(),
        parent,
        local_transform,
        attachment: None,
        enabled: true,
        tags: BTreeSet::new(),
        generated_by: None,
    };
    recipe.instances.insert(instance.id, instance);
}

fn finish_ids(
    recipe: &mut AssetRecipe,
    part_definition: u64,
    part_instance: u64,
    operation: u64,
    socket: u64,
) {
    recipe.next_ids.part_definition = part_definition;
    recipe.next_ids.part_instance = part_instance;
    recipe.next_ids.operation = operation;
    recipe.next_ids.region = recipe
        .definitions
        .values()
        .flat_map(|definition| {
            definition.regions.keys().copied().chain(
                definition
                    .geometry
                    .operations
                    .iter()
                    .flat_map(ModelingOperationSpec::generated_region_ids),
            )
        })
        .map(|id| id.0)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    recipe.next_ids.boundary_loop = recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .flat_map(ModelingOperationSpec::boundary_loop_ids)
        .map(|id| id.0)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    recipe.next_ids.socket = socket;
    recipe.next_ids.parameter = recipe
        .parameters
        .keys()
        .map(|id| id.0)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
}

fn transform(translation: [f32; 3], rotation_degrees: [f32; 3]) -> Transform3 {
    Transform3 {
        translation,
        rotation_degrees,
        scale: [1.0, 1.0, 1.0],
    }
}

fn frame(origin: [f32; 3]) -> Frame3 {
    Frame3 {
        origin,
        ..Frame3::default()
    }
}

fn sockets<const N: usize>(
    entries: [(SocketId, &str, [f32; 3]); N],
) -> BTreeMap<SocketId, SocketSpec> {
    entries
        .into_iter()
        .map(|(id, name, origin)| {
            (
                id,
                SocketSpec {
                    id,
                    name: name.to_owned(),
                    local_frame: Frame3 {
                        origin,
                        ..Frame3::default()
                    },
                    role: "pivot".to_owned(),
                    tags: BTreeSet::from(["socket".to_owned(), "pivot".to_owned()]),
                },
            )
        })
        .collect()
}

fn rounded_box_regions() -> BTreeMap<RegionId, SurfaceRegionSpec> {
    regions([
        (RegionId(1), "primary faces", SurfaceRole::PrimarySurface),
        (RegionId(2), "bevel bands", SurfaceRole::BevelBand),
        (RegionId(3), "corner blend patches", SurfaceRole::Detail),
    ])
}

fn cylinder_regions() -> BTreeMap<RegionId, SurfaceRegionSpec> {
    regions([
        (RegionId(1), "side wall", SurfaceRole::Side),
        (RegionId(2), "top cap", SurfaceRole::Cap),
        (RegionId(3), "bottom cap", SurfaceRole::Cap),
        (RegionId(4), "top bevel band", SurfaceRole::BevelBand),
        (RegionId(5), "bottom bevel band", SurfaceRole::BevelBand),
    ])
}

fn plate_regions() -> BTreeMap<RegionId, SurfaceRegionSpec> {
    regions([
        (RegionId(1), "front panel face", SurfaceRole::PrimarySurface),
        (RegionId(2), "back panel face", SurfaceRole::PrimarySurface),
        (RegionId(3), "plate side wall", SurfaceRole::Side),
        (RegionId(4), "plate bevel band", SurfaceRole::BevelBand),
    ])
}

fn sweep_regions() -> BTreeMap<RegionId, SurfaceRegionSpec> {
    regions([
        (RegionId(1), "swept side", SurfaceRole::Side),
        (RegionId(2), "sweep start cap", SurfaceRole::Cap),
        (RegionId(3), "sweep end cap", SurfaceRole::Cap),
    ])
}

fn lathe_regions() -> BTreeMap<RegionId, SurfaceRegionSpec> {
    regions([
        (RegionId(1), "lathed side", SurfaceRole::Side),
        (RegionId(2), "lower cap", SurfaceRole::Cap),
        (RegionId(3), "upper cap", SurfaceRole::Cap),
    ])
}

fn regions<const N: usize>(
    entries: [(RegionId, &str, SurfaceRole); N],
) -> BTreeMap<RegionId, SurfaceRegionSpec> {
    entries
        .into_iter()
        .map(|(id, name, role)| {
            (
                id,
                SurfaceRegionSpec {
                    id,
                    name: name.to_owned(),
                    role,
                    tags: BTreeSet::new(),
                },
            )
        })
        .collect()
}

fn regular_profile(radius: f32, segments: u32) -> Vec<[f32; 2]> {
    (0..segments)
        .map(|index| {
            let angle = std::f32::consts::TAU * index as f32 / segments as f32;
            [radius * angle.cos(), radius * angle.sin()]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use shape_asset::validate_asset_recipe;
    use shape_compile::compile_asset;

    use super::*;

    #[test]
    fn benchmark_recipes_validate_and_compile() {
        for asset in benchmark_assets() {
            let recipe = asset.recipe();
            assert!(
                validate_asset_recipe(&recipe).is_valid(),
                "{}",
                asset.slug()
            );
            let artifact = compile_asset(&recipe).unwrap_or_else(|_| panic!("{}", asset.slug()));
            assert!(artifact.validation_report.is_valid(), "{}", asset.slug());
            assert!(!artifact.statistics.used_sdf_or_remeshing);
        }
    }

    #[test]
    fn industrial_crate_body_cut_mesh_is_closed() {
        let recipe = industrial_crate_recipe();
        let artifact = compile_asset(&recipe).expect("industrial crate should compile");
        let body = artifact
            .compiled_parts
            .iter()
            .find(|part| part.instance_id == PartInstanceId(1))
            .expect("crate body should compile");
        let mut edge_faces = BTreeMap::<(u32, u32), usize>::new();
        for face in &body.local_mesh.faces {
            for index in 0..face.vertices.len() {
                let a = face.vertices[index];
                let b = face.vertices[(index + 1) % face.vertices.len()];
                let key = if a <= b { (a, b) } else { (b, a) };
                *edge_faces.entry(key).or_default() += 1;
            }
        }
        let bad_edges = edge_faces
            .iter()
            .filter(|(_, faces)| **faces != 2)
            .take(8)
            .map(|((a, b), faces)| {
                format!(
                    "{}.{} {:?}->{:?} faces={}",
                    a,
                    b,
                    body.local_mesh.positions[*a as usize],
                    body.local_mesh.positions[*b as usize],
                    faces
                )
            })
            .collect::<Vec<_>>();
        assert!(
            edge_faces.values().all(|faces| *faces == 2),
            "crate body should be closed; bad edges: {bad_edges:?}"
        );
    }

    #[test]
    fn multi_cut_panel_authors_semantic_cut_groups() {
        let recipe = multi_cut_panel_recipe();
        let mount_holes = recipe
            .variation
            .semantic_cut_groups
            .get("mount_holes")
            .expect("mount hole group should be authored");
        let vents = recipe
            .variation
            .semantic_cut_groups
            .get("vents")
            .expect("vent group should be authored");

        assert_eq!(mount_holes.operations.len(), 4);
        assert_eq!(mount_holes.role, CutGroupRole::MountHoles);
        assert_eq!(vents.operations.len(), 3);
        assert_eq!(vents.role, CutGroupRole::Vents);
    }

    #[test]
    fn multi_cut_panel_authors_boundary_loop_bevels() {
        let recipe = multi_cut_panel_recipe();
        let definition = recipe
            .definitions
            .get(&PartDefinitionId(1))
            .expect("panel definition should exist");
        let bevels = definition
            .geometry
            .operations
            .iter()
            .filter_map(|operation| match operation {
                ModelingOperationSpec::BevelBoundaryLoop {
                    target_loop,
                    outer_replacement_loop,
                    inner_replacement_loop,
                    ..
                } => Some((
                    *target_loop,
                    *outer_replacement_loop,
                    *inner_replacement_loop,
                )),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(bevels.len(), 6);
        assert!(bevels.contains(&(BoundaryLoopId(1), BoundaryLoopId(17), BoundaryLoopId(18))));
        assert!(bevels.contains(&(BoundaryLoopId(2), BoundaryLoopId(19), BoundaryLoopId(20))));
        assert!(bevels.contains(&(BoundaryLoopId(3), BoundaryLoopId(21), BoundaryLoopId(22))));
    }
}
