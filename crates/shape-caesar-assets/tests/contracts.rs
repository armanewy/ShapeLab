use std::collections::BTreeSet;

use shape_caesar_assets::{all_project_caesar_templates, project_caesar_pack, river_bend_pack};
use shape_compile::compile_asset;
use shape_gamekit::{validate_game_asset_definition, validate_game_asset_pack};

#[test]
fn river_bend_pack_reserves_all_nine_module_keys() {
    let pack = river_bend_pack();
    let keys = pack
        .assets
        .iter()
        .map(|asset| asset.module_semantics.runtime_key.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        keys,
        BTreeSet::from([
            "deck",
            "decoy_worksite",
            "gate",
            "marching_camp",
            "palisade",
            "pile",
            "ramp",
            "road",
            "tower",
        ])
    );
    assert!(validate_game_asset_pack(&pack).is_valid());
}

#[test]
fn all_project_caesar_templates_are_valid_and_ordered() {
    let pack = project_caesar_pack();
    let keys = pack
        .assets
        .iter()
        .map(|asset| asset.module_semantics.runtime_key.as_str())
        .collect::<Vec<_>>();
    let mut sorted = keys.clone();
    sorted.sort_unstable();

    assert_eq!(keys, sorted);
    assert!(validate_game_asset_pack(&pack).is_valid());
    assert!(all_project_caesar_templates().len() > river_bend_pack().assets.len());
}

#[test]
fn stubs_serialize_and_compile() {
    for asset in all_project_caesar_templates() {
        let json = serde_json::to_string(&asset).expect("asset should serialize");
        let round_tripped: shape_gamekit::GameAssetDefinition =
            serde_json::from_str(&json).expect("asset should deserialize");
        assert_eq!(asset, round_tripped);
        assert!(
            validate_game_asset_definition(&asset).is_valid(),
            "{} should validate",
            asset.module_semantics.runtime_key
        );
        let artifact = compile_asset(&asset.source_recipe)
            .unwrap_or_else(|error| panic!("{} should compile: {error}", asset.id));
        assert!(
            artifact.validation_report.is_valid(),
            "{} compile validation should be clean",
            asset.id
        );
    }
}
