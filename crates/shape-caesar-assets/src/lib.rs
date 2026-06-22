#![forbid(unsafe_code)]

//! Project Caesar asset pack stubs built on Shape Lab recipes.
//!
//! This crate is intentionally Project Caesar-specific. Runtime-neutral
//! contracts live in `shape-gamekit`; this crate reserves stable module keys,
//! families, and authored template IDs for the River Bend dogfooding pack.

use std::collections::{BTreeMap, BTreeSet};

pub mod families;

use shape_asset::{
    AssetId, AssetRecipe, Frame3, GeometryRecipe, GeometrySource, PartDefinition, PartDefinitionId,
    PartInstance, PartInstanceId, RegionId, SurfaceRegionSpec, SurfaceRole, Transform3,
};
use shape_gamekit::{
    CellBounds, CollisionProxy, ConstructionPhase, ConstructionProfile, ExportProfile,
    FixedCameraProfile, GAME_ASSET_PACK_SCHEMA_VERSION, GameAssetDefinition, GameAssetPack,
    GameplayTag, GridRotation, LayerBounds, LogicalFootprint, ModuleSemantics,
    MonotonicVisibilityPolicy, ReadabilityProfile, RotationSymmetry, SnapAnchor, SnapAnchorRole,
    SnapRelationship, SupportRole, SupportSurface, SurfaceShape, TraversalRole, TriangleBudget,
    WalkableSurface,
};

/// Source revision marker used by generated stub packs.
pub const PROJECT_CAESAR_SOURCE_REVISION: &str = "shape-lab:d921140-project-caesar-contracts";

/// Build the complete Project Caesar template pack currently known to Shape Lab.
#[must_use]
pub fn project_caesar_pack() -> GameAssetPack {
    pack(
        "project-caesar",
        "Project Caesar Asset Foundry",
        all_project_caesar_templates(),
    )
}

/// Build the River Bend engineering-module pack.
#[must_use]
pub fn river_bend_pack() -> GameAssetPack {
    pack(
        "project-caesar-river-bend",
        "Project Caesar River Bend Engineering Modules",
        river_bend_templates(),
    )
}

/// Return all currently declared Project Caesar templates.
#[must_use]
pub fn all_project_caesar_templates() -> Vec<GameAssetDefinition> {
    let mut assets = Vec::new();
    assets.extend(families::crossing::templates());
    assets.extend(families::fortification::templates());
    assets.extend(families::support::templates());
    assets.extend(families::command_tokens::templates());
    sort_assets(assets)
}

/// Return the nine River Bend engineering modules.
#[must_use]
pub fn river_bend_templates() -> Vec<GameAssetDefinition> {
    let mut assets = Vec::new();
    assets.extend(families::crossing::templates());
    assets.extend(families::fortification::templates());
    assets.extend(families::support::templates());
    sort_assets(assets)
}

fn pack(id: &str, title: &str, assets: Vec<GameAssetDefinition>) -> GameAssetPack {
    GameAssetPack {
        schema_version: GAME_ASSET_PACK_SCHEMA_VERSION,
        id: id.to_owned(),
        title: title.to_owned(),
        assets,
        export_profile: ExportProfile::internal_dogfood(),
        source_revision: PROJECT_CAESAR_SOURCE_REVISION.to_owned(),
    }
}

fn sort_assets(mut assets: Vec<GameAssetDefinition>) -> Vec<GameAssetDefinition> {
    assets.sort_by(|left, right| {
        left.module_semantics
            .runtime_key
            .cmp(&right.module_semantics.runtime_key)
    });
    assets
}

pub(crate) struct ModuleStubSpec {
    pub asset_id: u64,
    pub id: &'static str,
    pub display_name: &'static str,
    pub family: &'static str,
    pub runtime_key: &'static str,
    pub footprint: LogicalFootprint,
    pub half_extents: [f32; 3],
    pub gameplay_tags: Vec<GameplayTag>,
    pub snap_anchors: Vec<SnapAnchor>,
    pub support_surfaces: Vec<SupportSurface>,
    pub walkable_surfaces: Vec<WalkableSurface>,
    pub collision_proxies: Vec<CollisionProxy>,
    pub triangle_budget: TriangleBudget,
    pub tags: Vec<&'static str>,
}

pub(crate) fn module_stub(spec: ModuleStubSpec) -> GameAssetDefinition {
    GameAssetDefinition {
        id: spec.id.to_owned(),
        display_name: spec.display_name.to_owned(),
        family: spec.family.to_owned(),
        source_recipe: stub_recipe(spec.asset_id, spec.display_name, spec.half_extents),
        module_semantics: ModuleSemantics {
            runtime_key: spec.runtime_key.to_owned(),
            logical_footprint: spec.footprint,
            rotation_symmetry: RotationSymmetry::None,
            instanceable: true,
            snap_anchors: spec.snap_anchors,
            support_surfaces: spec.support_surfaces,
            walkable_surfaces: spec.walkable_surfaces,
            traversal_links: Vec::new(),
            collision_proxies: spec.collision_proxies,
            gameplay_tags: spec.gameplay_tags,
        },
        construction_profile: default_construction_profile(),
        readability_profile: caesar_readability_profile(),
        budgets: spec.triangle_budget,
        tags: spec.tags.into_iter().map(str::to_owned).collect(),
    }
}

pub(crate) fn footprint(min: [i32; 2], max: [i32; 2]) -> LogicalFootprint {
    LogicalFootprint {
        cell_bounds: CellBounds { min, max },
        vertical_layers: LayerBounds { min: 0, max: 0 },
        origin_cell: min,
        permitted_rotations: vec![
            GridRotation::R0,
            GridRotation::R90,
            GridRotation::R180,
            GridRotation::R270,
        ],
    }
}

pub(crate) fn anchor(
    id: &'static str,
    role: SnapAnchorRole,
    origin: [f32; 3],
    tags: &[&str],
    relationship: SnapRelationship,
) -> SnapAnchor {
    SnapAnchor {
        id: id.to_owned(),
        role,
        local_frame: frame(origin),
        compatibility_tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
        relationship,
    }
}

pub(crate) fn walkable_rect(
    id: &'static str,
    center: [f32; 2],
    size: [f32; 2],
    elevation: f32,
    role: TraversalRole,
    anchors: &[&str],
) -> WalkableSurface {
    let half = [size[0] * 0.5, size[1] * 0.5];
    WalkableSurface {
        id: id.to_owned(),
        polygon: vec![
            [center[0] - half[0], center[1] - half[1]],
            [center[0] + half[0], center[1] - half[1]],
            [center[0] + half[0], center[1] + half[1]],
            [center[0] - half[0], center[1] + half[1]],
        ],
        elevation,
        traversal_role: role,
        entry_exit_anchors: anchors.iter().map(|anchor| (*anchor).to_owned()).collect(),
    }
}

pub(crate) fn support_rect(
    id: &'static str,
    center: [f32; 2],
    size: [f32; 2],
    role: SupportRole,
) -> SupportSurface {
    SupportSurface {
        id: id.to_owned(),
        shape: SurfaceShape::Rectangle { center, size },
        support_role: role,
        maximum_supported_layer_hint: Some(1),
    }
}

pub(crate) fn box_proxy(center: [f32; 3], half_extents: [f32; 3]) -> CollisionProxy {
    CollisionProxy::Box {
        center,
        half_extents,
    }
}

pub(crate) fn cylinder_proxy(center: [f32; 3], radius: f32, height: f32) -> CollisionProxy {
    CollisionProxy::Cylinder {
        center,
        radius,
        height,
    }
}

pub(crate) const fn budget(maximum: u32) -> TriangleBudget {
    TriangleBudget {
        preview_maximum: maximum,
        game_maximum: maximum,
        repeated_instance_maximum: maximum,
    }
}

fn frame(origin: [f32; 3]) -> Frame3 {
    Frame3 {
        origin,
        ..Frame3::default()
    }
}

fn default_construction_profile() -> ConstructionProfile {
    ConstructionProfile {
        phases: vec![
            ConstructionPhase {
                id: "placed".to_owned(),
                label: "Placed".to_owned(),
                progress_threshold: 0.0,
                visible_part_tags: vec!["foundation".to_owned()],
                required_predecessor: None,
            },
            ConstructionPhase {
                id: "complete".to_owned(),
                label: "Complete".to_owned(),
                progress_threshold: 1.0,
                visible_part_tags: vec!["foundation".to_owned(), "complete".to_owned()],
                required_predecessor: Some("placed".to_owned()),
            },
        ],
        optional_damaged_state: None,
        final_phase: "complete".to_owned(),
        monotonic_visibility_policy: MonotonicVisibilityPolicy::Strict,
    }
}

fn caesar_readability_profile() -> ReadabilityProfile {
    ReadabilityProfile {
        fixed_camera_profiles: vec![
            FixedCameraProfile::Custom("project_caesar_oblique".to_owned()),
            FixedCameraProfile::Custom("project_caesar_top".to_owned()),
        ],
        minimum_recognizable_pixel_size: 32,
        silhouette_importance: 0.75,
        maximum_hidden_area_fraction: 0.35,
        orientation_coverage: vec![
            GridRotation::R0,
            GridRotation::R90,
            GridRotation::R180,
            GridRotation::R270,
        ],
    }
}

fn stub_recipe(asset_id: u64, title: &str, half_extents: [f32; 3]) -> AssetRecipe {
    let definition = PartDefinition {
        id: PartDefinitionId(1),
        name: format!("{title} stub"),
        tags: BTreeSet::from(["foundation".to_owned(), "complete".to_owned()]),
        geometry: GeometryRecipe {
            source: GeometrySource::RoundedBox {
                half_extents,
                radius: 0.025,
            },
            operations: Vec::new(),
        },
        regions: rounded_box_regions(),
        sockets: BTreeMap::new(),
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
    };
    let instance = PartInstance {
        id: PartInstanceId(1),
        definition: PartDefinitionId(1),
        name: title.to_owned(),
        parent: None,
        local_transform: Transform3::default(),
        attachment: None,
        enabled: true,
        tags: BTreeSet::from(["foundation".to_owned(), "complete".to_owned()]),
        generated_by: None,
    };
    let mut recipe = AssetRecipe::new(AssetId(asset_id), title);
    recipe.definitions.insert(definition.id, definition);
    let instance_id = instance.id;
    recipe.instances.insert(instance_id, instance);
    recipe.root_instances.push(instance_id);
    recipe.next_ids.part_definition = 2;
    recipe.next_ids.part_instance = 2;
    recipe.next_ids.operation = 1;
    recipe.next_ids.region = 4;
    recipe
}

fn rounded_box_regions() -> BTreeMap<RegionId, SurfaceRegionSpec> {
    [
        (1, "primary faces", SurfaceRole::PrimarySurface),
        (2, "bevel bands", SurfaceRole::BevelBand),
        (3, "corner patches", SurfaceRole::Detail),
    ]
    .into_iter()
    .map(|(id, name, role)| {
        (
            RegionId(id),
            SurfaceRegionSpec {
                id: RegionId(id),
                name: name.to_owned(),
                role,
                tags: BTreeSet::new(),
            },
        )
    })
    .collect()
}
