use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetId, AssetRecipe, Frame3, GeneratedIdPolicy, GeometryRecipe, GeometrySource,
    PartDefinition, PartDefinitionId, PartInstance, PartInstanceId, PatternAxis, PatternContract,
    PatternCountPolicy, PatternExportInstancingPolicy, PatternId, PatternType, Transform3,
};
use shape_compile::evaluate_recipe_patterns;

#[test]
fn pattern_contract_compile_report_evaluates_linear_pattern() {
    let mut recipe = source_recipe();
    recipe.semantic.patterns.insert(
        PatternId(1),
        PatternContract {
            id: PatternId(1),
            pattern_type: PatternType::Linear,
            source_instance: Some(PartInstanceId(1)),
            count: Some(3),
            label: "Compile repeat proof".to_owned(),
            count_policy: PatternCountPolicy::Exact(3),
            density_policy: None,
            export_instancing: PatternExportInstancingPolicy::Pending,
            linear_axis: Some(PatternAxis::Z),
            spacing: Some(0.5),
            generated_id_policy: GeneratedIdPolicy::PatternOccurrenceIndex,
        },
    );
    recipe.next_ids.pattern = 2;

    let report = evaluate_recipe_patterns(&recipe);

    assert!(report.blockers.is_empty());
    assert_eq!(report.pattern_reports.len(), 1);
    assert_eq!(report.pattern_reports[0].generated_occurrence_count, 3);
    assert_eq!(
        report.pattern_reports[0].occurrence_ids,
        vec![
            "pattern-1-occurrence-0000",
            "pattern-1-occurrence-0001",
            "pattern-1-occurrence-0002"
        ]
    );
    assert!(!report.pattern_reports[0].export_instancing_enabled);
}

#[test]
fn pattern_contract_compile_report_blocks_invalid_count() {
    let mut recipe = source_recipe();
    recipe.semantic.patterns.insert(
        PatternId(2),
        PatternContract {
            id: PatternId(2),
            pattern_type: PatternType::Linear,
            source_instance: Some(PartInstanceId(1)),
            count: Some(0),
            label: "Invalid repeat proof".to_owned(),
            count_policy: PatternCountPolicy::Exact(0),
            density_policy: None,
            export_instancing: PatternExportInstancingPolicy::Pending,
            linear_axis: Some(PatternAxis::X),
            spacing: Some(0.5),
            generated_id_policy: GeneratedIdPolicy::PatternOccurrenceIndex,
        },
    );
    recipe.next_ids.pattern = 3;

    let report = evaluate_recipe_patterns(&recipe);

    assert!(report.pattern_reports.is_empty());
    assert_eq!(report.blockers.len(), 1);
    assert_eq!(report.blockers[0].pattern_id, PatternId(2));
    assert_eq!(report.blockers[0].reason, "invalid pattern count");
}

fn source_recipe() -> AssetRecipe {
    let definition_id = PartDefinitionId(1);
    let instance_id = PartInstanceId(1);
    let mut recipe = AssetRecipe::new(AssetId(1), "Pattern proof");
    recipe.definitions.insert(
        definition_id,
        PartDefinition {
            id: definition_id,
            name: "Source".to_owned(),
            tags: BTreeSet::new(),
            geometry: GeometryRecipe {
                source: GeometrySource::RoundedBox {
                    half_extents: [0.5, 0.5, 0.5],
                    radius: 0.05,
                },
                operations: Vec::new(),
            },
            regions: BTreeMap::new(),
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
            name: "Source".to_owned(),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::new(),
            generated_by: None,
        },
    );
    recipe.root_instances.push(instance_id);
    recipe.next_ids.part_definition = 2;
    recipe.next_ids.part_instance = 2;
    recipe
}
