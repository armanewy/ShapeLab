//! Field-fortification module reservations for Project Caesar.

use shape_gamekit::{
    GameAssetDefinition, GameplayTag, SnapAnchorRole, SnapRelationship, SupportRole, TraversalRole,
};

use crate::{
    ModuleStubSpec, anchor, box_proxy, budget, footprint, module_stub, support_rect, walkable_rect,
};

/// Stable runtime key for the palisade module.
pub const PALISADE_RUNTIME_KEY: &str = "palisade";
/// Stable runtime key for the gate module.
pub const GATE_RUNTIME_KEY: &str = "gate";
/// Stable runtime key for the tower module.
pub const TOWER_RUNTIME_KEY: &str = "tower";

/// Return all reserved field-fortification templates.
#[must_use]
pub fn templates() -> Vec<GameAssetDefinition> {
    vec![gate(), palisade(), tower()]
}

fn palisade() -> GameAssetDefinition {
    module_stub(ModuleStubSpec {
        asset_id: 20_101,
        id: "project-caesar:fortification:palisade",
        display_name: "Palisade Segment",
        family: "Field Fortification",
        runtime_key: PALISADE_RUNTIME_KEY,
        footprint: footprint([0, 0], [1, 0]),
        half_extents: [1.0, 0.6, 0.12],
        gameplay_tags: vec![GameplayTag::BlocksMovement, GameplayTag::CoverSource],
        snap_anchors: vec![
            anchor(
                "start",
                SnapAnchorRole::Continuation,
                [0.0, 0.0, 0.5],
                &["palisade_line"],
                SnapRelationship::Optional,
            ),
            anchor(
                "end",
                SnapAnchorRole::Continuation,
                [2.0, 0.0, 0.5],
                &["palisade_line"],
                SnapRelationship::Optional,
            ),
        ],
        support_surfaces: Vec::new(),
        walkable_surfaces: Vec::new(),
        collision_proxies: vec![box_proxy([1.0, 0.45, 0.5], [1.0, 0.45, 0.15])],
        triangle_budget: budget(800),
        tags: vec!["fortification", "blocking"],
    })
}

fn gate() -> GameAssetDefinition {
    module_stub(ModuleStubSpec {
        asset_id: 20_102,
        id: "project-caesar:fortification:gate",
        display_name: "Fortification Gate",
        family: "Field Fortification",
        runtime_key: GATE_RUNTIME_KEY,
        footprint: footprint([0, 0], [1, 0]),
        half_extents: [1.0, 0.55, 0.16],
        gameplay_tags: vec![GameplayTag::CoverSource],
        snap_anchors: vec![
            anchor(
                "left_line",
                SnapAnchorRole::Continuation,
                [0.0, 0.0, 0.5],
                &["palisade_line"],
                SnapRelationship::Optional,
            ),
            anchor(
                "right_line",
                SnapAnchorRole::Continuation,
                [2.0, 0.0, 0.5],
                &["palisade_line"],
                SnapRelationship::Optional,
            ),
            anchor(
                "passage",
                SnapAnchorRole::Entry,
                [1.0, 0.0, 0.5],
                &["ground_connection"],
                SnapRelationship::Optional,
            ),
        ],
        support_surfaces: Vec::new(),
        walkable_surfaces: Vec::new(),
        collision_proxies: vec![box_proxy([1.0, 0.4, 0.5], [1.0, 0.4, 0.12])],
        triangle_budget: budget(1_800),
        tags: vec!["fortification", "passage"],
    })
}

fn tower() -> GameAssetDefinition {
    module_stub(ModuleStubSpec {
        asset_id: 20_103,
        id: "project-caesar:fortification:tower",
        display_name: "Watchtower",
        family: "Field Fortification",
        runtime_key: TOWER_RUNTIME_KEY,
        footprint: footprint([0, 0], [0, 0]),
        half_extents: [0.48, 1.2, 0.48],
        gameplay_tags: vec![
            GameplayTag::ProvidesSupport,
            GameplayTag::ElevatedPlatform,
            GameplayTag::CoverSource,
        ],
        snap_anchors: vec![
            anchor(
                "ladder_base",
                SnapAnchorRole::Entry,
                [0.5, 0.0, 0.0],
                &["vertical_access"],
                SnapRelationship::Optional,
            ),
            anchor(
                "platform",
                SnapAnchorRole::Support,
                [0.5, 1.6, 0.5],
                &["elevated_platform"],
                SnapRelationship::Supporting,
            ),
        ],
        support_surfaces: vec![support_rect(
            "tower_platform_support",
            [0.5, 0.5],
            [0.9, 0.9],
            SupportRole::ElevatedPlatform,
        )],
        walkable_surfaces: vec![walkable_rect(
            "tower_platform",
            [0.5, 0.5],
            [0.9, 0.9],
            1.6,
            TraversalRole::Platform,
            &["ladder_base", "platform"],
        )],
        collision_proxies: vec![box_proxy([0.5, 0.8, 0.5], [0.48, 0.8, 0.48])],
        triangle_budget: budget(4_000),
        tags: vec!["fortification", "elevated"],
    })
}
