#![forbid(unsafe_code)]

//! Benchmark explicit asset recipes for the modeling kernel.

use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetId, AssetRecipe, Frame3, GeometryRecipe, GeometrySource, ModelingOperationSpec,
    OperationId, PartDefinition, PartDefinitionId, PartInstance, PartInstanceId, SocketId,
    SocketSpec, Transform3,
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
        definition(
            1,
            "rounded crate body",
            GeometrySource::RoundedBox {
                half_extents: [2.0, 1.0, 1.2],
                radius: 0.14,
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(1),
                radius: 0.14,
                segments: 3,
            }],
            BTreeMap::new(),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(2),
        definition(
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
            BTreeMap::new(),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(3),
        definition(
            3,
            "raised crate panel",
            GeometrySource::Plate {
                size: [3.25, 0.78],
                thickness: 0.12,
            },
            vec![ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(3),
                radius: 0.025,
                segments: 1,
            }],
            BTreeMap::new(),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(4),
        definition(
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
            BTreeMap::new(),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(5),
        definition(
            5,
            "panel bolt",
            GeometrySource::Cylinder {
                radius: 0.075,
                height: 0.14,
                radial_segments: 16,
            },
            vec![ModelingOperationSpec::LinearArray {
                operation: OperationId(5),
                count: 6,
                offset: [0.56, 0.0, 0.0],
            }],
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
        transform([0.0, 0.06, 1.255], [90.0, 0.0, 0.0]),
    );
    add_instance(
        &mut recipe,
        7,
        3,
        "back raised panel",
        Some(PartInstanceId(1)),
        transform([0.0, 0.06, -1.255], [-90.0, 0.0, 0.0]),
    );
    add_instance(
        &mut recipe,
        8,
        4,
        "left swept side handle",
        Some(PartInstanceId(1)),
        transform([-2.08, 0.06, 0.0], [0.0, 0.0, 0.0]),
    );
    add_instance(
        &mut recipe,
        9,
        5,
        "front bolt row",
        Some(PartInstanceId(6)),
        transform([-1.4, 0.55, 1.36], [90.0, 0.0, 0.0]),
    );
    add_instance(
        &mut recipe,
        10,
        5,
        "back bolt row",
        Some(PartInstanceId(7)),
        transform([-1.4, 0.55, -1.36], [-90.0, 0.0, 0.0]),
    );
    recipe.root_instances.push(PartInstanceId(1));
    finish_ids(&mut recipe, 6, 11, 6, 1);
    recipe
}

/// Build the explicit desk lamp recipe.
#[must_use]
pub fn explicit_desk_lamp_recipe() -> AssetRecipe {
    let mut recipe = AssetRecipe::new(AssetId(1002), "Explicit Desk Lamp");
    recipe.definitions.insert(
        PartDefinitionId(1),
        definition(
            1,
            "lathed weighted base",
            GeometrySource::Lathe {
                profile: vec![
                    [0.0, -0.12],
                    [0.86, -0.12],
                    [0.76, 0.08],
                    [0.34, 0.18],
                    [0.0, 0.18],
                ],
                segments: 48,
            },
            Vec::new(),
            sockets([(SocketId(1), "base top pivot", [0.0, 0.18, 0.0])]),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(2),
        definition(
            2,
            "swept angled stem",
            GeometrySource::Sweep {
                profile: regular_profile(0.065, 8),
                path: vec![
                    frame([0.0, 0.0, 0.0]),
                    frame([0.12, 0.82, 0.0]),
                    frame([0.62, 1.55, 0.0]),
                ],
            },
            Vec::new(),
            sockets([
                (SocketId(1), "stem lower pivot", [0.0, 0.0, 0.0]),
                (SocketId(2), "stem upper pivot", [0.62, 1.55, 0.0]),
            ]),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(3),
        definition(
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
            sockets([(SocketId(1), "joint axis pivot", [0.0, 0.0, 0.0])]),
        ),
    );
    recipe.definitions.insert(
        PartDefinitionId(4),
        definition(
            4,
            "lathed shade",
            GeometrySource::Lathe {
                profile: vec![
                    [0.0, 0.0],
                    [0.34, 0.0],
                    [0.73, 0.24],
                    [0.62, 0.58],
                    [0.30, 0.50],
                    [0.0, 0.50],
                ],
                segments: 48,
            },
            Vec::new(),
            sockets([(SocketId(1), "shade neck socket", [0.0, 0.02, 0.0])]),
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
        transform([0.0, 0.28, 0.0], [0.0, 0.0, 90.0]),
    );
    add_instance(
        &mut recipe,
        3,
        2,
        "swept angled stem",
        Some(PartInstanceId(1)),
        transform([0.0, 0.30, 0.0], [0.0, 0.0, 0.0]),
    );
    add_instance(
        &mut recipe,
        4,
        3,
        "upper cylinder pivot",
        Some(PartInstanceId(3)),
        transform([0.62, 1.88, 0.0], [0.0, 0.0, 90.0]),
    );
    add_instance(
        &mut recipe,
        5,
        4,
        "lathed shade",
        Some(PartInstanceId(4)),
        transform([0.92, 1.66, 0.0], [0.0, 0.0, -23.0]),
    );
    recipe.root_instances.push(PartInstanceId(1));
    finish_ids(&mut recipe, 5, 6, 2, 3);
    recipe
}

fn definition(
    id: u64,
    name: &str,
    source: GeometrySource,
    operations: Vec<ModelingOperationSpec>,
    sockets: BTreeMap<SocketId, SocketSpec>,
) -> PartDefinition {
    PartDefinition {
        id: PartDefinitionId(id),
        name: name.to_owned(),
        tags: BTreeSet::new(),
        geometry: GeometryRecipe { source, operations },
        regions: BTreeMap::new(),
        sockets,
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
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
    recipe.next_ids.socket = socket;
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
