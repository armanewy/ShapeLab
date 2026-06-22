//! Camp and logistics module reservations for Project Caesar.

use shape_gamekit::{
    GameAssetDefinition, GameplayTag, SnapAnchorRole, SnapRelationship, SupportRole, TraversalRole,
};

use crate::{
    ModuleStubSpec, anchor, box_proxy, budget, footprint, module_stub, support_rect, walkable_rect,
};

/// Stable runtime key for the road module.
pub const ROAD_RUNTIME_KEY: &str = "road";
/// Stable runtime key for the marching camp module.
pub const MARCHING_CAMP_RUNTIME_KEY: &str = "marching_camp";
/// Stable runtime key for the decoy worksite module.
pub const DECOY_WORKSITE_RUNTIME_KEY: &str = "decoy_worksite";

/// Return all reserved camp and logistics templates.
#[must_use]
pub fn templates() -> Vec<GameAssetDefinition> {
    vec![decoy_worksite(), marching_camp(), road()]
}

fn road() -> GameAssetDefinition {
    module_stub(ModuleStubSpec {
        asset_id: 20_201,
        id: "project-caesar:support:road",
        display_name: "Road Segment",
        family: "Camp and Logistics",
        runtime_key: ROAD_RUNTIME_KEY,
        footprint: footprint([0, 0], [1, 0]),
        half_extents: [1.0, 0.025, 0.35],
        gameplay_tags: vec![GameplayTag::Walkable, GameplayTag::RoadSurface],
        snap_anchors: vec![
            anchor(
                "start",
                SnapAnchorRole::Continuation,
                [0.0, 0.0, 0.5],
                &["road_connection"],
                SnapRelationship::Optional,
            ),
            anchor(
                "end",
                SnapAnchorRole::Continuation,
                [2.0, 0.0, 0.5],
                &["road_connection"],
                SnapRelationship::Optional,
            ),
        ],
        support_surfaces: Vec::new(),
        walkable_surfaces: vec![walkable_rect(
            "road_surface",
            [1.0, 0.5],
            [2.0, 0.8],
            0.04,
            TraversalRole::Road,
            &["start", "end"],
        )],
        collision_proxies: vec![box_proxy([1.0, 0.025, 0.5], [1.0, 0.025, 0.35])],
        triangle_budget: budget(250),
        tags: vec!["logistics", "road"],
    })
}

fn marching_camp() -> GameAssetDefinition {
    module_stub(ModuleStubSpec {
        asset_id: 20_202,
        id: "project-caesar:support:marching_camp",
        display_name: "Marching Camp",
        family: "Camp and Logistics",
        runtime_key: MARCHING_CAMP_RUNTIME_KEY,
        footprint: footprint([0, 0], [2, 2]),
        half_extents: [1.35, 0.18, 1.35],
        gameplay_tags: vec![GameplayTag::ProvidesSupport, GameplayTag::CoverSource],
        snap_anchors: vec![
            anchor(
                "gate",
                SnapAnchorRole::Entry,
                [1.5, 0.0, 0.0],
                &["camp_gate", "road_connection"],
                SnapRelationship::Optional,
            ),
            anchor(
                "depot",
                SnapAnchorRole::Support,
                [1.5, 0.0, 1.5],
                &["supply_depot"],
                SnapRelationship::Supporting,
            ),
        ],
        support_surfaces: vec![support_rect(
            "camp_yard",
            [1.5, 1.5],
            [2.6, 2.6],
            SupportRole::Foundation,
        )],
        walkable_surfaces: vec![walkable_rect(
            "camp_ground",
            [1.5, 1.5],
            [2.8, 2.8],
            0.04,
            TraversalRole::Ground,
            &["gate"],
        )],
        collision_proxies: vec![box_proxy([1.5, 0.12, 1.5], [1.35, 0.12, 1.35])],
        triangle_budget: budget(6_000),
        tags: vec!["logistics", "camp", "depot", "rally"],
    })
}

fn decoy_worksite() -> GameAssetDefinition {
    module_stub(ModuleStubSpec {
        asset_id: 20_203,
        id: "project-caesar:support:decoy_worksite",
        display_name: "Decoy Worksite",
        family: "Camp and Logistics",
        runtime_key: DECOY_WORKSITE_RUNTIME_KEY,
        footprint: footprint([0, 0], [1, 1]),
        half_extents: [0.9, 0.18, 0.9],
        gameplay_tags: vec![
            GameplayTag::DecoySignature,
            GameplayTag::ConcealmentSignature,
        ],
        snap_anchors: vec![anchor(
            "signal_platform",
            SnapAnchorRole::Support,
            [1.0, 0.3, 1.0],
            &["decoy_signal"],
            SnapRelationship::Supporting,
        )],
        support_surfaces: vec![support_rect(
            "worksite_platform",
            [1.0, 1.0],
            [0.8, 0.8],
            SupportRole::Scaffold,
        )],
        walkable_surfaces: Vec::new(),
        collision_proxies: vec![box_proxy([1.0, 0.12, 1.0], [0.9, 0.12, 0.9])],
        triangle_budget: budget(1_500),
        tags: vec!["logistics", "decoy"],
    })
}
