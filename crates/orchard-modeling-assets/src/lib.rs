#![forbid(unsafe_code)]

//! Box Primitive explicit asset recipe for the modeling kernel.

use std::collections::{BTreeMap, BTreeSet};

use orchard_asset::{
    AssetId, AssetRecipe, Frame3, GeometryRecipe, GeometrySource, PartDefinition, PartDefinitionId,
    PartInstance, PartInstanceId, RegionId, SurfaceRegionSpec, SurfaceRole, Transform3,
};

/// Built-in explicit modeling benchmark asset.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BenchmarkAsset {
    /// Closed rounded box benchmark.
    BoxPrimitive,
}

impl BenchmarkAsset {
    /// Parse the CLI/display slug.
    #[must_use]
    pub fn parse(slug: &str) -> Option<Self> {
        match slug {
            "box-primitive" => Some(Self::BoxPrimitive),
            _ => None,
        }
    }

    /// Stable slug.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::BoxPrimitive => "box-primitive",
        }
    }

    /// Build the asset recipe.
    #[must_use]
    pub fn recipe(self) -> AssetRecipe {
        match self {
            Self::BoxPrimitive => box_primitive_recipe(),
        }
    }
}

/// Return every bundled benchmark asset.
#[must_use]
pub const fn benchmark_assets() -> [BenchmarkAsset; 1] {
    [BenchmarkAsset::BoxPrimitive]
}

/// Build the Box Primitive recipe.
#[must_use]
pub fn box_primitive_recipe() -> AssetRecipe {
    let definition_id = PartDefinitionId(1);
    let instance_id = PartInstanceId(1);
    let mut recipe = AssetRecipe::new(AssetId(1), "Box Primitive");
    recipe.definitions.insert(
        definition_id,
        PartDefinition {
            id: definition_id,
            name: "Closed box body".to_owned(),
            tags: tags(["box", "body", "primitive"]),
            geometry: GeometryRecipe {
                source: GeometrySource::RoundedBox {
                    half_extents: [0.78, 0.5, 0.58],
                    radius: 0.08,
                },
                operations: Vec::new(),
            },
            regions: rounded_box_regions(),
            sockets: BTreeMap::new(),
            local_pivot: Frame3::default(),
            variant_group: None,
            production_hints: None,
        },
    );
    recipe.instances.insert(
        instance_id,
        PartInstance {
            id: instance_id,
            definition: definition_id,
            name: "Box Primitive".to_owned(),
            parent: None,
            local_transform: Transform3 {
                translation: [0.0, 0.5, 0.0],
                rotation_degrees: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            },
            attachment: None,
            enabled: true,
            tags: tags(["box", "primitive"]),
            generated_by: None,
        },
    );
    recipe.root_instances.push(instance_id);
    recipe.next_ids.part_definition = 2;
    recipe.next_ids.part_instance = 2;
    recipe.next_ids.region = 4;
    recipe
}

fn rounded_box_regions() -> BTreeMap<RegionId, SurfaceRegionSpec> {
    [
        (RegionId(1), "primary faces", SurfaceRole::PrimarySurface),
        (RegionId(2), "edge bands", SurfaceRole::BevelBand),
        (RegionId(3), "corner blends", SurfaceRole::Detail),
    ]
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

fn tags<const N: usize>(values: [&str; N]) -> BTreeSet<String> {
    values.into_iter().map(str::to_owned).collect()
}

#[cfg(test)]
mod tests {
    use orchard_asset::validate_asset_recipe;
    use orchard_compile::compile_asset;

    use super::*;

    #[test]
    fn benchmark_assets_only_contains_box_primitive() {
        assert_eq!(benchmark_assets(), [BenchmarkAsset::BoxPrimitive]);
    }

    #[test]
    fn box_primitive_recipe_validates_and_compiles() {
        let recipe = box_primitive_recipe();
        assert!(validate_asset_recipe(&recipe).is_valid());
        let artifact = compile_asset(&recipe).expect("box primitive should compile");
        assert!(artifact.validation_report.is_valid());
        assert_eq!(artifact.statistics.part_count, 1);
        assert!(!artifact.statistics.used_sdf_or_remeshing);
    }
}
