//! River-crossing module reservations for Project Caesar.

use shape_gamekit::{
    GameAssetDefinition, GameplayTag, SnapAnchorRole, SnapRelationship, SupportRole, TraversalRole,
};

use crate::{
    ModuleStubSpec, anchor, box_proxy, budget, cylinder_proxy, footprint, module_stub,
    support_rect, walkable_rect,
};

/// Stable runtime key for the timber pile module.
pub const PILE_RUNTIME_KEY: &str = "pile";
/// Stable runtime key for the bridge deck module.
pub const DECK_RUNTIME_KEY: &str = "deck";
/// Stable runtime key for the ramp module.
pub const RAMP_RUNTIME_KEY: &str = "ramp";

/// Return all reserved river-crossing templates.
#[must_use]
pub fn templates() -> Vec<GameAssetDefinition> {
    vec![pile(), deck(), ramp()]
}

fn pile() -> GameAssetDefinition {
    module_stub(ModuleStubSpec {
        asset_id: 20_001,
        id: "project-caesar:crossing:pile",
        display_name: "River Pile",
        family: "River Crossing",
        runtime_key: PILE_RUNTIME_KEY,
        footprint: footprint([0, 0], [0, 0]),
        half_extents: [0.18, 0.8, 0.18],
        gameplay_tags: vec![GameplayTag::ProvidesSupport],
        snap_anchors: vec![
            anchor(
                "top_support",
                SnapAnchorRole::Support,
                [0.0, 1.6, 0.0],
                &["bridge_support"],
                SnapRelationship::Supporting,
            ),
            anchor(
                "lateral_brace",
                SnapAnchorRole::Brace,
                [0.0, 0.8, 0.0],
                &["brace"],
                SnapRelationship::Optional,
            ),
        ],
        support_surfaces: vec![support_rect(
            "pile_cap",
            [0.5, 0.5],
            [0.5, 0.5],
            SupportRole::DeckSupport,
        )],
        walkable_surfaces: Vec::new(),
        collision_proxies: vec![cylinder_proxy([0.0, 0.8, 0.0], 0.18, 1.6)],
        triangle_budget: budget(400),
        tags: vec!["crossing", "support"],
    })
}

fn deck() -> GameAssetDefinition {
    module_stub(ModuleStubSpec {
        asset_id: 20_002,
        id: "project-caesar:crossing:deck",
        display_name: "Bridge Deck",
        family: "River Crossing",
        runtime_key: DECK_RUNTIME_KEY,
        footprint: footprint([0, 0], [1, 0]),
        half_extents: [1.0, 0.08, 0.45],
        gameplay_tags: vec![GameplayTag::Walkable, GameplayTag::ProvidesSupport],
        snap_anchors: vec![
            anchor(
                "start",
                SnapAnchorRole::Continuation,
                [0.0, 0.15, 0.5],
                &["bridge_connection"],
                SnapRelationship::Optional,
            ),
            anchor(
                "end",
                SnapAnchorRole::Continuation,
                [2.0, 0.15, 0.5],
                &["bridge_connection"],
                SnapRelationship::Optional,
            ),
            anchor(
                "underside_support",
                SnapAnchorRole::Support,
                [1.0, 0.0, 0.5],
                &["bridge_support"],
                SnapRelationship::Required,
            ),
        ],
        support_surfaces: vec![support_rect(
            "deck_top_support",
            [1.0, 0.5],
            [2.0, 1.0],
            SupportRole::ElevatedPlatform,
        )],
        walkable_surfaces: vec![walkable_rect(
            "deck_walkable",
            [1.0, 0.5],
            [2.0, 1.0],
            0.18,
            TraversalRole::BridgeDeck,
            &["start", "end"],
        )],
        collision_proxies: vec![box_proxy([1.0, 0.1, 0.5], [1.0, 0.1, 0.45])],
        triangle_budget: budget(1_200),
        tags: vec!["crossing", "walkable"],
    })
}

fn ramp() -> GameAssetDefinition {
    module_stub(ModuleStubSpec {
        asset_id: 20_003,
        id: "project-caesar:crossing:ramp",
        display_name: "Bridge Ramp",
        family: "River Crossing",
        runtime_key: RAMP_RUNTIME_KEY,
        footprint: footprint([0, 0], [1, 0]),
        half_extents: [1.0, 0.12, 0.45],
        gameplay_tags: vec![GameplayTag::Walkable],
        snap_anchors: vec![
            anchor(
                "lower_connection",
                SnapAnchorRole::Entry,
                [0.0, 0.05, 0.5],
                &["ground_connection"],
                SnapRelationship::Optional,
            ),
            anchor(
                "upper_connection",
                SnapAnchorRole::Exit,
                [2.0, 0.35, 0.5],
                &["bridge_connection"],
                SnapRelationship::Optional,
            ),
        ],
        support_surfaces: vec![support_rect(
            "high_end_support",
            [1.75, 0.5],
            [0.5, 1.0],
            SupportRole::DeckSupport,
        )],
        walkable_surfaces: vec![walkable_rect(
            "ramp_walkable",
            [1.0, 0.5],
            [2.0, 1.0],
            0.2,
            TraversalRole::Ramp,
            &["lower_connection", "upper_connection"],
        )],
        collision_proxies: vec![box_proxy([1.0, 0.16, 0.5], [1.0, 0.16, 0.45])],
        triangle_budget: budget(1_200),
        tags: vec!["crossing", "walkable"],
    })
}
