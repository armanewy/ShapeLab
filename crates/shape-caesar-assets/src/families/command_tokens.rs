//! Formation-level command-piece reservations for Project Caesar.

use shape_gamekit::{
    GameAssetDefinition, GameplayTag, SnapAnchorRole, SnapRelationship, TraversalRole,
};

use crate::{ModuleStubSpec, anchor, box_proxy, budget, footprint, module_stub, walkable_rect};

const COMMAND_FAMILY: &str = "Command Pieces";

/// Return all reserved command-piece templates.
#[must_use]
pub fn templates() -> Vec<GameAssetDefinition> {
    [
        (
            "project-caesar:command:roman_legion",
            "Roman Legion",
            "roman_legion",
            "roman",
            "legion",
        ),
        (
            "project-caesar:command:roman_cavalry",
            "Roman Cavalry",
            "roman_cavalry",
            "roman",
            "cavalry",
        ),
        (
            "project-caesar:command:roman_engineer",
            "Roman Engineer",
            "roman_engineer",
            "roman",
            "engineer",
        ),
        (
            "project-caesar:command:roman_commander",
            "Roman Commander",
            "roman_commander",
            "roman",
            "commander",
        ),
        (
            "project-caesar:command:gallic_ford_guard",
            "Gallic Ford Guard",
            "gallic_ford_guard",
            "gallic",
            "guard",
        ),
        (
            "project-caesar:command:gallic_depot_garrison",
            "Gallic Depot Garrison",
            "gallic_depot_garrison",
            "gallic",
            "garrison",
        ),
        (
            "project-caesar:command:gallic_mobile_reserve",
            "Gallic Mobile Reserve",
            "gallic_mobile_reserve",
            "gallic",
            "reserve",
        ),
        (
            "project-caesar:command:gallic_scout",
            "Gallic Scout",
            "gallic_scout",
            "gallic",
            "scout",
        ),
        (
            "project-caesar:command:roman_supply_base",
            "Roman Supply Base",
            "roman_supply_base",
            "roman",
            "supply",
        ),
        (
            "project-caesar:command:enemy_depot",
            "Enemy Depot",
            "enemy_depot",
            "gallic",
            "depot",
        ),
        (
            "project-caesar:command:uncertain_enemy",
            "Uncertain Enemy",
            "uncertain_enemy",
            "unknown",
            "uncertain",
        ),
        (
            "project-caesar:command:baggage_supply_marker",
            "Baggage Supply Marker",
            "baggage_supply_marker",
            "neutral",
            "supply",
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (id, display_name, runtime_key, team, role))| {
        command_token(
            20_300 + index as u64,
            id,
            display_name,
            runtime_key,
            team,
            role,
        )
    })
    .collect()
}

fn command_token(
    asset_id: u64,
    id: &'static str,
    display_name: &'static str,
    runtime_key: &'static str,
    team: &'static str,
    role: &'static str,
) -> GameAssetDefinition {
    module_stub(ModuleStubSpec {
        asset_id,
        id,
        display_name,
        family: COMMAND_FAMILY,
        runtime_key,
        footprint: footprint([0, 0], [0, 0]),
        half_extents: [0.38, 0.12, 0.38],
        gameplay_tags: vec![GameplayTag::Custom("command_piece".to_owned())],
        snap_anchors: vec![anchor(
            "origin",
            SnapAnchorRole::Center,
            [0.5, 0.0, 0.5],
            &["command_piece"],
            SnapRelationship::Optional,
        )],
        support_surfaces: Vec::new(),
        walkable_surfaces: vec![walkable_rect(
            "base",
            [0.5, 0.5],
            [0.75, 0.75],
            0.08,
            TraversalRole::Ground,
            &["origin"],
        )],
        collision_proxies: vec![box_proxy([0.5, 0.08, 0.5], [0.38, 0.08, 0.38])],
        triangle_budget: budget(1_200),
        tags: vec!["command", team, role],
    })
}
