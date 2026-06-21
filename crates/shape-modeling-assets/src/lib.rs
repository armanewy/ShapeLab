#![forbid(unsafe_code)]

//! Benchmark explicit asset recipes for the modeling kernel.

use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetId, AssetRecipe, Frame3, GeometryRecipe, GeometrySource, ModelingOperationSpec,
    OperationId, ParameterDescriptor, ParameterId, PartDefinition, PartDefinitionId, PartInstance,
    PartInstanceId, RegionId, SocketId, SocketSpec, SurfaceRegionSpec, SurfaceRole, Transform3,
};

/// Built-in explicit modeling benchmark asset.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BenchmarkAsset {
    /// Industrial crate benchmark.
    IndustrialCrate,
    /// Explicit desk lamp benchmark.
    ExplicitDeskLamp,
}

impl BenchmarkAsset {
    /// Parse the CLI/display slug.
    #[must_use]
    pub fn parse(slug: &str) -> Option<Self> {
        match slug {
            "industrial-crate" => Some(Self::IndustrialCrate),
            "explicit-desk-lamp" => Some(Self::ExplicitDeskLamp),
            _ => None,
        }
    }

    /// Stable slug.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::IndustrialCrate => "industrial-crate",
            Self::ExplicitDeskLamp => "explicit-desk-lamp",
        }
    }

    /// Build the asset recipe.
    #[must_use]
    pub fn recipe(self) -> AssetRecipe {
        match self {
            Self::IndustrialCrate => industrial_crate_recipe(),
            Self::ExplicitDeskLamp => explicit_desk_lamp_recipe(),
        }
    }
}

/// Return every bundled benchmark asset.
#[must_use]
pub const fn benchmark_assets() -> [BenchmarkAsset; 2] {
    [
        BenchmarkAsset::IndustrialCrate,
        BenchmarkAsset::ExplicitDeskLamp,
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
                ModelingOperationSpec::SetBevelProfile {
                    operation: OperationId(1),
                    radius: 0.14,
                    segments: 3,
                },
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
            "raised crate panel",
            GeometrySource::Plate {
                size: [3.25, 0.82],
                thickness: 0.10,
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(3),
                radius: 0.025,
                segments: 1,
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
                ModelingOperationSpec::SetBevelProfile {
                    operation: OperationId(10),
                    radius: 0.01,
                    segments: 1,
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
        "front raised panel",
        Some(PartInstanceId(1)),
        transform([0.0, 0.06, 1.27], [90.0, 0.0, 0.0]),
    );
    add_instance(
        &mut recipe,
        7,
        3,
        "back raised panel",
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
    recipe.parameters.insert(
        ParameterId(1),
        parameter(
            1,
            "Body bevel radius",
            "Edges",
            "definition.1.operation.1.bevel.radius",
            0.08,
            0.18,
            0.01,
        ),
    );
    recipe.root_instances.push(PartInstanceId(1));
    finish_ids(&mut recipe, 9, 18, 13, 3);
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
            "Stem sweep radius",
            "Form",
            "definition.2.geometry.sweep.profile.0.x",
            0.045,
            0.075,
            0.005,
        ),
    );
    recipe.root_instances.push(PartInstanceId(1));
    finish_ids(&mut recipe, 9, 11, 8, 3);
    recipe
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
        .flat_map(|definition| definition.regions.keys())
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
}
