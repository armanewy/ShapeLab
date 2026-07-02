#![forbid(unsafe_code)]

use orchard_core_legacy::{Aabb, Transform3};
use orchard_search_internal::asset::scoring::{
    AssetCandidateInput, AssetHardRejectionReason, AssetSelectionPolicy,
    score_and_select_asset_candidates, score_and_select_asset_candidates_with_policy,
};

macro_rules! vec3_value {
    ($x:expr, $y:expr, $z:expr) => {{
        let mut value = Transform3::default().translation;
        value.x = $x;
        value.y = $y;
        value.z = $z;
        value
    }};
}

fn bounds(size: f32) -> Aabb {
    Aabb {
        min: vec3_value!(-size, -size, -size),
        max: vec3_value!(size, size, size),
    }
}

fn candidate(id: &str, silhouette: [f32; 4]) -> AssetCandidateInput {
    let mut input = AssetCandidateInput::new(id, id, bounds(1.0));
    input.silhouette_occupancy = silhouette;
    input.part_volumes = vec![0.7, 0.22, 0.08];
    input.region_count = 3;
    input.detail_count = 3;
    input.symmetry_score = silhouette[0].clamp(0.0, 1.0);
    input.repeated_element_count = (silhouette[1] * 10.0) as usize;
    input.bevel_radii = vec![0.04, 0.045, 0.05];
    input.topology_cost = 0.35 + silhouette[2];
    input
}

fn candidate_set() -> Vec<AssetCandidateInput> {
    vec![
        candidate("asset-a", [0.12, 0.18, 0.24, 0.20]),
        candidate("asset-b", [0.24, 0.32, 0.16, 0.28]),
        candidate("asset-c", [0.36, 0.22, 0.48, 0.34]),
        candidate("asset-d", [0.48, 0.56, 0.28, 0.50]),
        candidate("asset-e", [0.62, 0.36, 0.64, 0.58]),
        candidate("asset-f", [0.74, 0.68, 0.40, 0.72]),
        candidate("asset-g", [0.86, 0.46, 0.76, 0.80]),
        candidate("asset-h", [0.52, 0.78, 0.88, 0.92]),
    ]
}

#[test]
fn duplicates_collapse_before_selection() {
    let mut simple = candidate("simple", [0.42, 0.42, 0.42, 0.42]);
    simple.recipe_fingerprint = "same-recipe".to_owned();
    simple.triangle_count = 1_000;

    let mut retessellated = simple.clone();
    retessellated.id = "retessellated".to_owned();
    retessellated.triangle_count = 18_000;

    let report = score_and_select_asset_candidates(&[retessellated, simple]);

    assert_eq!(report.scored_candidates.len(), 2);
    assert_eq!(report.unique_candidates.len(), 1);
    assert_eq!(report.duplicate_groups.len(), 1);
    assert_eq!(report.duplicate_groups[0].kept_id, "simple");
    assert_eq!(report.representatives.len(), 1);
    assert_eq!(report.representatives[0].id, "simple");
}

#[test]
fn distinct_silhouettes_survive_diversity_selection() {
    let report = score_and_select_asset_candidates(&candidate_set());
    let selected_ids: Vec<_> = report
        .representatives
        .iter()
        .map(|candidate| candidate.id.as_str())
        .collect();

    assert_eq!(selected_ids.len(), 6);
    assert!(selected_ids.contains(&"asset-a"));
    assert!(selected_ids.contains(&"asset-h"));
    assert!(
        report
            .representatives
            .windows(2)
            .all(|pair| pair[0].id != pair[1].id)
    );
}

#[test]
fn binary_silhouette_masks_affect_duplicate_collapse() {
    let mut filled = candidate("filled", [0.5, 0.5, 0.5, 0.5]);
    filled.silhouette_masks = vec![vec![u64::MAX; 64]; 4];
    let mut empty = candidate("empty", [0.5, 0.5, 0.5, 0.5]);
    empty.silhouette_masks = vec![vec![0; 64]; 4];

    let report = score_and_select_asset_candidates(&[filled, empty]);

    assert_eq!(report.scored_candidates.len(), 2);
    assert_eq!(report.unique_candidates.len(), 2);
}

#[test]
fn invalid_candidate_is_hard_rejected() {
    let mut invalid = candidate("invalid", [0.3, 0.3, 0.3, 0.3]);
    invalid.recipe_valid = false;

    let report = score_and_select_asset_candidates(&[invalid]);

    assert!(report.scored_candidates.is_empty());
    assert!(report.representatives.is_empty());
    assert_eq!(report.rejected_candidates.len(), 1);
    assert_eq!(
        report.rejected_candidates[0].reason,
        AssetHardRejectionReason::InvalidRecipe
    );
    assert_eq!(
        report
            .rejection_counts()
            .get(&AssetHardRejectionReason::InvalidRecipe)
            .copied(),
        Some(1)
    );
}

#[test]
fn excessive_detail_is_penalized_without_hard_rejection() {
    let simple = candidate("simple", [0.4, 0.4, 0.4, 0.4]);
    let mut detailed = candidate("detailed", [0.4, 0.4, 0.4, 0.4]);
    detailed.recipe_fingerprint = "detailed".to_owned();
    detailed.region_count = 30;
    detailed.detail_count = 48;
    detailed.topology_cost = 4.0;

    let report = score_and_select_asset_candidates(&[simple, detailed]);
    let simple_score = report
        .scored_candidates
        .iter()
        .find(|candidate| candidate.id == "simple")
        .expect("simple candidate scored");
    let detailed_score = report
        .scored_candidates
        .iter()
        .find(|candidate| candidate.id == "detailed")
        .expect("detailed candidate scored");

    assert_eq!(report.rejected_candidates.len(), 0);
    assert!(
        detailed_score
            .quality_penalties
            .excessive_detail_relative_to_primary_forms
            > simple_score
                .quality_penalties
                .excessive_detail_relative_to_primary_forms
    );
    assert!(detailed_score.weighted_quality_penalty > simple_score.weighted_quality_penalty);
}

#[test]
fn representative_order_is_deterministic() {
    let inputs = candidate_set();
    let first = score_and_select_asset_candidates(&inputs);
    let second = score_and_select_asset_candidates(&inputs);
    let first_ids: Vec<_> = first
        .representatives
        .iter()
        .map(|candidate| candidate.id.clone())
        .collect();
    let second_ids: Vec<_> = second
        .representatives
        .iter()
        .map(|candidate| candidate.id.clone())
        .collect();

    assert_eq!(first_ids, second_ids);
}

#[test]
fn default_policy_selects_six_representative_candidates() {
    let report = score_and_select_asset_candidates(&candidate_set());

    assert_eq!(report.representatives.len(), 6);
    assert_eq!(report.unique_candidates.len(), 8);
}

#[test]
fn same_recipe_under_different_tessellation_does_not_dominate_scoring() {
    let mut inputs = Vec::new();
    for index in 0..7 {
        let mut variant = candidate(&format!("same-{index}"), [0.44, 0.44, 0.44, 0.44]);
        variant.recipe_fingerprint = "same-recipe".to_owned();
        variant.triangle_count = 1_000 + index * 2_000;
        variant.silhouette_masks = vec![vec![u64::MAX; 64]; 4];
        inputs.push(variant);
    }
    inputs.extend(candidate_set());

    let report = score_and_select_asset_candidates(&inputs);
    let selected_same_recipe_count = report
        .representatives
        .iter()
        .filter(|candidate| candidate.recipe_fingerprint == "same-recipe")
        .count();

    assert!(selected_same_recipe_count <= 1);
    assert_eq!(
        report
            .unique_candidates
            .iter()
            .filter(|candidate| candidate.recipe_fingerprint == "same-recipe")
            .count(),
        1
    );
    assert!(
        report
            .duplicate_groups
            .iter()
            .any(|group| group.member_ids.len() == 7)
    );
}

#[test]
fn policy_can_select_fewer_than_six_candidates() {
    let policy = AssetSelectionPolicy {
        representative_count: 3,
        ..AssetSelectionPolicy::default()
    };
    let report = score_and_select_asset_candidates_with_policy(&candidate_set(), &policy);

    assert_eq!(report.representatives.len(), 3);
}
