use std::collections::BTreeSet;

use shape_asset::GeometrySource;
use shape_compile::export::{verify_model_package, write_model_package};
use shape_compile::validation::{
    ValidationLimits, validate_model, validation_config_from_recipe_with_limits,
};
use shape_family_compile::conformance::ConformanceStatus;
use shape_foundry::{CandidateLegibilityClass, ControlKind, ControlValue, VariationIntent};
use shape_foundry_catalog::roman_bridge;
use shape_search::foundry::{
    FoundryCandidateMode, FoundryCandidateRequest, generate_foundry_candidate_plans,
};

#[test]
fn roman_bridge_exposes_product_part_groups() {
    let groups = roman_bridge::part_group_descriptors();

    assert_eq!(
        groups
            .iter()
            .map(|group| group.display_name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Deck",
            "Supports",
            "Bracing",
            "Railing",
            "Ramps",
            "Fasteners"
        ]
    );
    let ramps = groups
        .iter()
        .find(|group| group.group_id == "ramps")
        .expect("ramps group");
    assert!(!ramps.focusable);
    assert_eq!(
        ramps.capability.unavailable_reasons,
        vec!["This part has no focused variations yet."]
    );
}

#[test]
fn roman_bridge_profile_declares_required_controls_and_strategies() {
    let fixture = roman_bridge::fixture_catalog();
    let catalog = shape_foundry::resolve_foundry_catalog(&fixture.document, &fixture)
        .expect("roman bridge catalog should resolve");

    assert_eq!(catalog.family.id, "bridge");
    assert_eq!(catalog.style_kit.id, "roman_timber_engineering");
    assert_eq!(catalog.style_kit.display_name, "Roman Timber Engineering");

    let role_ids = catalog
        .family
        .part_roles
        .iter()
        .map(|role| role.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        role_ids,
        vec![
            "support",
            "span",
            "deck",
            "brace",
            "ramp",
            "rail",
            "connector"
        ]
    );

    let primary_controls = catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary)
        .collect::<Vec<_>>();
    assert_eq!(primary_controls.len(), 7);
    assert_eq!(catalog.customizer_profile.maximum_primary_controls, 7);
    assert!(primary_controls.iter().all(|control| control.visible));
    assert_eq!(
        primary_controls
            .iter()
            .map(|control| control.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Span Length",
            "Deck Width",
            "Structural Heft",
            "Support Rhythm",
            "Bracing Style",
            "Railing",
            "Edge Finish"
        ]
    );

    assert!(matches!(
        primary_controls[3].kind,
        ControlKind::ProviderGallery { .. }
    ));
    assert!(matches!(
        primary_controls[4].kind,
        ControlKind::ChoiceGallery { .. }
    ));
    assert!(matches!(
        primary_controls[5].kind,
        ControlKind::ProviderGallery { .. }
    ));
    assert!(matches!(
        primary_controls[6].kind,
        ControlKind::ProviderGallery { .. }
    ));

    for control in &primary_controls[3..] {
        match &control.kind {
            ControlKind::ChoiceGallery { options } => {
                assert!(options.len() >= 3);
                assert!(options.iter().all(|option| {
                    option.preview.preview_id.starts_with(&control.id)
                        && option.preview.artifact_fingerprint.is_none()
                }));
            }
            ControlKind::ProviderGallery { options, .. } => {
                assert!(options.len() >= 3);
                assert!(options.iter().all(|option| {
                    option.preview.preview_id.starts_with(&control.id)
                        && option.preview.artifact_fingerprint.is_none()
                }));
            }
            _ => {}
        }
    }

    assert_eq!(
        catalog
            .customizer_profile
            .candidate_strategies
            .iter()
            .map(|strategy| strategy.label.as_str())
            .collect::<Vec<_>>(),
        vec!["Light", "Balanced", "Reinforced", "Wide Crossing"]
    );
}

#[test]
fn roman_bridge_hq_profile_declares_required_controls_and_direction_strategies() {
    let fixture = roman_bridge::hq_fixture_catalog();
    let catalog = shape_foundry::resolve_foundry_catalog(&fixture.document, &fixture)
        .expect("HQ roman bridge catalog should resolve");

    assert_eq!(fixture.slug, "roman-bridge-hq");
    let connector = catalog
        .family
        .part_roles
        .iter()
        .find(|role| role.id == "connector")
        .expect("connector role");
    assert!(connector.required);

    let primary_controls = catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary)
        .collect::<Vec<_>>();
    assert_eq!(primary_controls.len(), 7);
    assert_eq!(catalog.customizer_profile.maximum_primary_controls, 7);
    assert!(primary_controls.iter().all(|control| control.visible));
    assert_eq!(
        primary_controls
            .iter()
            .map(|control| control.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Span Length",
            "Deck Width",
            "Structural Heft",
            "Support Style",
            "Bracing Style",
            "Railing Style",
            "Detail Density"
        ]
    );

    assert!(matches!(
        primary_controls[3].kind,
        ControlKind::ProviderGallery { .. }
    ));
    assert!(matches!(
        primary_controls[4].kind,
        ControlKind::ChoiceGallery { .. }
    ));
    assert!(matches!(
        primary_controls[5].kind,
        ControlKind::ProviderGallery { .. }
    ));
    assert!(matches!(
        primary_controls[6].kind,
        ControlKind::ProviderGallery { .. }
    ));

    for control in &primary_controls[3..] {
        match &control.kind {
            ControlKind::ChoiceGallery { options } => {
                assert!(options.len() >= 3);
                assert!(options.iter().all(|option| {
                    option.preview.preview_id.starts_with(&control.id)
                        && option.preview.artifact_fingerprint.is_none()
                }));
            }
            ControlKind::ProviderGallery { options, .. } => {
                assert!(options.len() >= 3);
                assert!(options.iter().all(|option| {
                    option.preview.preview_id.starts_with(&control.id)
                        && option.preview.artifact_fingerprint.is_none()
                }));
            }
            _ => {}
        }
    }

    assert_eq!(
        catalog
            .customizer_profile
            .candidate_strategies
            .iter()
            .map(|strategy| strategy.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Reinforced",
            "Light Crossing",
            "Wide Crossing",
            "Compact Span",
            "Stone-Pier Outpost",
            "Detailed Timberwork",
            "Minimal Span"
        ]
    );
}

#[test]
fn roman_bridge_compiles_with_connected_deck_and_support_conformance() {
    let fixture = roman_bridge::fixture_catalog();
    let output = shape_foundry::compile_foundry_document(&fixture.document, &fixture)
        .expect("roman bridge should compile");

    assert!(output.final_conformance.is_accepted());
    assert!(output.artifact.validation_report.is_valid());
    let model_config = validation_config_from_recipe_with_limits(
        &output.recipe,
        &output.artifact,
        ValidationLimits::default(),
    );
    let model_report = validate_model(&output.artifact, &model_config);
    assert!(
        model_report.is_valid(),
        "roman bridge model validation should pass: {:#?}",
        model_report.issues
    );
    assert!(
        output
            .final_conformance
            .roles
            .iter()
            .all(|row| row.status == ConformanceStatus::Passed),
        "{:#?}",
        output.final_conformance.roles
    );

    let attachment_ids = output
        .final_conformance
        .attachments
        .iter()
        .map(|row| row.rule_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        attachment_ids,
        vec![
            "brace_to_span",
            "deck_to_span",
            "rail_to_deck",
            "ramp_to_deck",
            "support_to_span"
        ]
    );
    for row in &output.final_conformance.attachments {
        assert_eq!(row.status, ConformanceStatus::Passed, "{row:#?}");
        assert!(row.coverage.produced_pairs);
        assert!(row.pairs.iter().all(|pair| pair.connected));
        assert!(row.pairs.iter().all(|pair| pair.socket_compatible));
    }

    let support_row = output
        .final_conformance
        .roles
        .iter()
        .find(|row| row.role == "support")
        .expect("support role conformance");
    assert!(support_row.actual_occurrences >= 1);

    let deck_instances = role_instances(&output.recipe, "deck");
    assert_eq!(deck_instances.len(), 1);
    let deck = output
        .recipe
        .instances
        .get(&deck_instances[0])
        .expect("deck instance");
    assert!(deck.attachment.is_some(), "deck should be attached to span");

    let support_instances = role_instances(&output.recipe, "support");
    assert_eq!(support_instances.len(), 1);
    assert!(support_instances.iter().all(|instance| {
        output
            .recipe
            .instances
            .get(instance)
            .is_some_and(|part| part.attachment.is_some())
    }));
    assert!(compiled_role_count(&output.recipe, &output.artifact, "support") >= 6);

    assert!(output.recipe.definitions.values().all(|definition| {
        !matches!(
            definition.geometry.source,
            GeometrySource::ReservedBooleanResult { .. }
        )
    }));
    assert!(output.recipe.definitions.values().any(|definition| {
        definition
            .geometry
            .operations
            .iter()
            .any(|operation| matches!(operation, shape_asset::ModelingOperationSpec::LinearArray { count, .. } if *count > 1))
    }));
    assert!(
        output
            .final_conformance
            .operations
            .iter()
            .any(
                |row| row.operation == shape_family::AllowedOperationKind::Array
                    && row.actual_count > 0
                    && row.status == ConformanceStatus::Passed
            )
    );
}

#[test]
fn roman_bridge_hq_compiles_with_connector_details_and_valid_model() {
    let fixture = roman_bridge::hq_fixture_catalog();
    let output = shape_foundry::compile_foundry_document(&fixture.document, &fixture)
        .expect("HQ roman bridge should compile");

    assert!(output.final_conformance.is_accepted());
    assert!(output.artifact.validation_report.is_valid());
    let model_config = validation_config_from_recipe_with_limits(
        &output.recipe,
        &output.artifact,
        ValidationLimits::default(),
    );
    let model_report = validate_model(&output.artifact, &model_config);
    assert!(
        model_report.is_valid(),
        "HQ roman bridge model validation should pass: {:#?}",
        model_report.issues
    );

    let attachment_ids = output
        .final_conformance
        .attachments
        .iter()
        .map(|row| row.rule_id.as_str())
        .collect::<Vec<_>>();
    assert!(attachment_ids.contains(&"connector_to_deck"));
    for row in &output.final_conformance.attachments {
        assert_eq!(row.status, ConformanceStatus::Passed, "{row:#?}");
        assert!(row.coverage.produced_pairs);
        assert!(row.pairs.iter().all(|pair| pair.connected));
        assert!(row.pairs.iter().all(|pair| pair.socket_compatible));
    }

    assert!(compiled_role_count(&output.recipe, &output.artifact, "connector") >= 6);
    assert!(compiled_role_count(&output.recipe, &output.artifact, "support") >= 6);
    assert!(output.artifact.statistics.triangle_count > 2_000);
}

#[test]
fn roman_bridge_hq_support_options_are_connected_and_structurally_distinct() {
    let fixture = roman_bridge::hq_fixture_catalog();
    let support_options = provider_options(&fixture, "support_style");
    assert_eq!(
        support_options,
        vec![
            "round_pile_supports",
            "squared_post_supports",
            "stone_pier_blocks",
            "trestle_frame_supports"
        ]
    );

    let mut signatures = BTreeSet::new();
    for provider in support_options {
        let output = compile_with_control(
            &fixture,
            "support_style",
            ControlValue::Provider(provider.to_owned()),
        );
        assert_required_attachments_connected(&output);
        let support = role_bounds(&output.recipe, &output.artifact, "support");
        let span = role_bounds(&output.recipe, &output.artifact, "span");
        assert!(
            support.max_y >= span.min_y - 0.2,
            "{provider} should visibly reach the span: support={support:?} span={span:?}"
        );
        signatures.insert(role_signature(&output, "support"));
    }

    assert_eq!(
        signatures.len(),
        4,
        "round piles, squared posts, stone piers, and trestle frames should not collapse visually"
    );
}

#[test]
fn roman_bridge_hq_bracing_options_are_structurally_distinct() {
    let fixture = roman_bridge::hq_fixture_catalog();
    let brace_options = choice_options(&fixture, "bracing_style");
    assert_eq!(
        brace_options,
        vec![
            "minimal_under_ties",
            "x_brace_beam",
            "k_brace_beam",
            "heavy_reinforced_brace"
        ]
    );

    let mut signatures = BTreeSet::new();
    for choice in brace_options {
        let output = compile_with_control(
            &fixture,
            "bracing_style",
            ControlValue::Choice(choice.to_owned()),
        );
        assert_required_attachments_connected(&output);
        signatures.insert(role_signature(&output, "brace"));
    }

    assert_eq!(
        signatures.len(),
        4,
        "minimal, X, K, and heavy reinforced braces should produce different brace structures"
    );
}

#[test]
fn roman_bridge_hq_deck_width_lock_changes_walkable_width_without_breaking_export() {
    let fixture = roman_bridge::hq_fixture_catalog();
    let narrow = compile_with_control(&fixture, "deck_width", ControlValue::Scalar(0.86));
    let wide = compile_with_control(&fixture, "deck_width", ControlValue::Scalar(1.58));

    let narrow_deck = role_bounds(&narrow.recipe, &narrow.artifact, "deck");
    let wide_deck = role_bounds(&wide.recipe, &wide.artifact, "deck");
    assert!(
        wide_deck.depth() > narrow_deck.depth() + 0.5,
        "deck width should visibly widen the deck: narrow={narrow_deck:?} wide={wide_deck:?}"
    );
    assert_required_attachments_connected(&wide);

    let package_dir = tempfile::tempdir().expect("temporary package directory");
    write_model_package(&wide.recipe, &wide.artifact, package_dir.path())
        .expect("HQ bridge package should export");
    let verification =
        verify_model_package(package_dir.path()).expect("HQ bridge package should reopen");
    assert!(verification.checksums_match);
    assert!(verification.topology_matches_manifest);
    assert!(verification.finite_numeric_payloads);
}

#[test]
fn roman_bridge_hq_explore_returns_clear_distinct_whole_asset_directions() {
    let fixture = roman_bridge::hq_fixture_catalog();
    let output = generate_foundry_candidate_plans(
        &fixture.document,
        &fixture,
        &FoundryCandidateRequest {
            seed: 101,
            proposal_count: 72,
            result_count: 6,
            mode: FoundryCandidateMode::Explore,
            strategy_id: None,
            preference_profile: None,
            variation_intent: VariationIntent::default(),
        },
    )
    .expect("HQ roman bridge Explore candidates should generate");

    assert_eq!(output.candidates.len(), 6);
    let mut visual_signatures = BTreeSet::new();
    for candidate in &output.candidates {
        assert!(
            candidate.changed_controls.len() >= 2,
            "direction should combine controls: {candidate:#?}"
        );
        assert_ne!(
            candidate.variation_metadata.visible_delta.legibility_class,
            CandidateLegibilityClass::TooSubtle,
            "TooSubtle candidates must not be returned as normal whole-asset ideas"
        );
        assert!(
            candidate
                .variation_metadata
                .visible_delta
                .legibility_class
                .selectable(),
            "candidate should be selectable: {:#?}",
            candidate.variation_metadata.visible_delta
        );
        let compiled = shape_foundry::compile_foundry_document(&candidate.document, &fixture)
            .expect("candidate document should compile");
        visual_signatures.insert(camera_signature(&compiled.artifact));
    }

    assert!(
        visual_signatures.len() >= 4,
        "at least four Explore directions should be visually distinct; got {}",
        visual_signatures.len()
    );
}

#[test]
fn every_primary_control_has_visible_endpoint_difference() {
    let fixture = roman_bridge::fixture_catalog();
    let catalog = shape_foundry::resolve_foundry_catalog(&fixture.document, &fixture)
        .expect("roman bridge catalog should resolve");

    for control in catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary)
    {
        let (low, high) = endpoint_values(control);
        let low_output = compile_with_control(&fixture, &control.id, low);
        let high_output = compile_with_control(&fixture, &control.id, high);

        assert_ne!(
            low_output.build_stamp.geometry_input_fingerprint,
            high_output.build_stamp.geometry_input_fingerprint,
            "{} should change geometry input",
            control.id
        );
        assert_ne!(
            camera_signature(&low_output.artifact),
            camera_signature(&high_output.artifact),
            "{} should be visible from the fixed camera",
            control.id
        );
    }
}

#[test]
fn every_hq_primary_control_has_visible_endpoint_difference() {
    let fixture = roman_bridge::hq_fixture_catalog();
    let catalog = shape_foundry::resolve_foundry_catalog(&fixture.document, &fixture)
        .expect("HQ roman bridge catalog should resolve");

    for control in catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary)
    {
        let (low, high) = endpoint_values(control);
        let low_output = compile_with_control(&fixture, &control.id, low);
        let high_output = compile_with_control(&fixture, &control.id, high);

        assert_ne!(
            low_output.build_stamp.geometry_input_fingerprint,
            high_output.build_stamp.geometry_input_fingerprint,
            "{} should change geometry input",
            control.id
        );
        assert_ne!(
            camera_signature(&low_output.artifact),
            camera_signature(&high_output.artifact),
            "{} should be visible from the fixed camera",
            control.id
        );
    }
}

#[test]
fn roman_bridge_compile_is_deterministic() {
    let fixture = roman_bridge::fixture_catalog();
    let first = shape_foundry::compile_foundry_document(&fixture.document, &fixture)
        .expect("first compile should pass");
    let second = shape_foundry::compile_foundry_document(&fixture.document, &fixture)
        .expect("second compile should pass");

    assert_eq!(first.build_stamp, second.build_stamp);
    assert_eq!(
        first.recipe_snapshot.recipe_fingerprint,
        second.recipe_snapshot.recipe_fingerprint
    );
    assert_eq!(
        first.recipe_snapshot.canonical_json,
        second.recipe_snapshot.canonical_json
    );
    assert_eq!(first.artifact.statistics, second.artifact.statistics);
    assert_eq!(first.final_conformance, second.final_conformance);
}

fn compile_with_control(
    fixture: &shape_foundry_catalog::FoundryFixtureCatalog,
    control_id: &str,
    value: ControlValue,
) -> shape_foundry::FoundryCompilationOutput {
    let mut document = fixture.document.clone();
    document.control_state.insert(control_id.to_owned(), value);
    shape_foundry::compile_foundry_document(&document, fixture)
        .unwrap_or_else(|error| panic!("{control_id} endpoint should compile: {error:#?}"))
}

fn endpoint_values(control: &shape_foundry::CustomizerControl) -> (ControlValue, ControlValue) {
    match &control.kind {
        ControlKind::ContinuousAxis { .. } => {
            let interval = control
                .domain
                .continuous_intervals
                .first()
                .expect("continuous control should have interval");
            (
                ControlValue::Scalar(interval.minimum),
                ControlValue::Scalar(interval.maximum),
            )
        }
        ControlKind::ChoiceGallery { options } => {
            let first = options.first().expect("choice control options");
            let last = options.last().expect("choice control options");
            (
                ControlValue::Choice(first.value.clone()),
                ControlValue::Choice(last.value.clone()),
            )
        }
        ControlKind::ProviderGallery { options, .. } => {
            let first = options.first().expect("provider control options");
            let last = options.last().expect("provider control options");
            (
                ControlValue::Provider(first.provider_id.clone()),
                ControlValue::Provider(last.provider_id.clone()),
            )
        }
        ControlKind::IntegerStepper { .. } | ControlKind::Toggle { .. } => {
            panic!("roman bridge primary controls should not use this kind")
        }
    }
}

fn provider_options(
    fixture: &shape_foundry_catalog::FoundryFixtureCatalog,
    control_id: &str,
) -> Vec<String> {
    let catalog = shape_foundry::resolve_foundry_catalog(&fixture.document, fixture)
        .expect("catalog should resolve");
    let control = catalog
        .customizer_profile
        .controls
        .iter()
        .find(|control| control.id == control_id)
        .expect("provider control");
    match &control.kind {
        ControlKind::ProviderGallery { options, .. } => options
            .iter()
            .map(|option| option.provider_id.clone())
            .collect(),
        _ => panic!("{control_id} should be a provider gallery"),
    }
}

fn choice_options(
    fixture: &shape_foundry_catalog::FoundryFixtureCatalog,
    control_id: &str,
) -> Vec<String> {
    let catalog = shape_foundry::resolve_foundry_catalog(&fixture.document, fixture)
        .expect("catalog should resolve");
    let control = catalog
        .customizer_profile
        .controls
        .iter()
        .find(|control| control.id == control_id)
        .expect("choice control");
    match &control.kind {
        ControlKind::ChoiceGallery { options } => {
            options.iter().map(|option| option.value.clone()).collect()
        }
        _ => panic!("{control_id} should be a choice gallery"),
    }
}

fn assert_required_attachments_connected(output: &shape_foundry::FoundryCompilationOutput) {
    assert!(output.final_conformance.is_accepted());
    for row in &output.final_conformance.attachments {
        assert_eq!(row.status, ConformanceStatus::Passed, "{row:#?}");
        assert!(row.coverage.produced_pairs);
        assert!(row.pairs.iter().all(|pair| pair.connected));
        assert!(row.pairs.iter().all(|pair| pair.socket_compatible));
    }
}

fn role_instances(
    recipe: &shape_asset::AssetRecipe,
    role: &str,
) -> Vec<shape_asset::PartInstanceId> {
    let role_tag = format!("role:{role}");
    recipe
        .instances
        .iter()
        .filter(|(_, instance)| instance.tags.contains(&role_tag))
        .map(|(id, _)| *id)
        .collect()
}

fn compiled_role_count(
    recipe: &shape_asset::AssetRecipe,
    artifact: &shape_compile::AssetArtifact,
    role: &str,
) -> usize {
    let role_tag = format!("role:{role}");
    artifact
        .compiled_parts
        .iter()
        .filter(|part| {
            recipe
                .definitions
                .get(&part.definition_id)
                .is_some_and(|definition| definition.tags.contains(&role_tag))
        })
        .count()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RoleSignature {
    part_count: u64,
    triangle_count: u64,
    width: i32,
    height: i32,
    depth: i32,
    checksum: i64,
}

fn role_signature(output: &shape_foundry::FoundryCompilationOutput, role: &str) -> RoleSignature {
    let bounds = role_bounds(&output.recipe, &output.artifact, role);
    let role_tag = format!("role:{role}");
    let mut part_count = 0;
    let mut triangle_count = 0;
    let mut checksum = 0_i64;
    for part in &output.artifact.compiled_parts {
        if output
            .recipe
            .definitions
            .get(&part.definition_id)
            .is_some_and(|definition| definition.tags.contains(&role_tag))
        {
            part_count += 1;
            triangle_count += part.world_mesh.faces.len() as u64;
            for point in &part.world_mesh.positions {
                checksum += i64::from(quantize(point[0] + point[1] * 3.0 + point[2] * 7.0));
            }
        }
    }
    RoleSignature {
        part_count,
        triangle_count,
        width: quantize(bounds.width()),
        height: quantize(bounds.height()),
        depth: quantize(bounds.depth()),
        checksum,
    }
}

#[derive(Debug, Copy, Clone)]
struct RoleBounds {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    min_z: f32,
    max_z: f32,
}

impl RoleBounds {
    fn width(self) -> f32 {
        self.max_x - self.min_x
    }

    fn height(self) -> f32 {
        self.max_y - self.min_y
    }

    fn depth(self) -> f32 {
        self.max_z - self.min_z
    }
}

fn role_bounds(
    recipe: &shape_asset::AssetRecipe,
    artifact: &shape_compile::AssetArtifact,
    role: &str,
) -> RoleBounds {
    let role_tag = format!("role:{role}");
    let mut bounds = RoleBounds {
        min_x: f32::INFINITY,
        max_x: f32::NEG_INFINITY,
        min_y: f32::INFINITY,
        max_y: f32::NEG_INFINITY,
        min_z: f32::INFINITY,
        max_z: f32::NEG_INFINITY,
    };

    for part in &artifact.compiled_parts {
        if recipe
            .definitions
            .get(&part.definition_id)
            .is_some_and(|definition| definition.tags.contains(&role_tag))
        {
            for point in &part.world_mesh.positions {
                bounds.min_x = bounds.min_x.min(point[0]);
                bounds.max_x = bounds.max_x.max(point[0]);
                bounds.min_y = bounds.min_y.min(point[1]);
                bounds.max_y = bounds.max_y.max(point[1]);
                bounds.min_z = bounds.min_z.min(point[2]);
                bounds.max_z = bounds.max_z.max(point[2]);
            }
        }
    }

    assert!(
        bounds.min_x.is_finite(),
        "{role} should have compiled geometry"
    );
    bounds
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CameraSignature {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
    projection_checksum: i64,
    part_count: u64,
    triangle_count: u64,
}

fn camera_signature(artifact: &shape_compile::AssetArtifact) -> CameraSignature {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut projection_checksum = 0_i64;

    for part in &artifact.compiled_parts {
        for point in &part.world_mesh.positions {
            let screen_x = point[0] + point[2] * 0.35;
            let screen_y = point[1] + point[2] * 0.2;
            min_x = min_x.min(screen_x);
            max_x = max_x.max(screen_x);
            min_y = min_y.min(screen_y);
            max_y = max_y.max(screen_y);
            projection_checksum += i64::from(quantize(screen_x.abs() + screen_y.abs()));
            projection_checksum += i64::from(quantize(point[0] + point[1] * 2.0 + point[2] * 3.0));
            projection_checksum += i64::from(quantize(
                point[0] * point[0] + point[1] * point[1] + point[2] * point[2],
            ));
        }
    }

    CameraSignature {
        min_x: quantize(min_x),
        max_x: quantize(max_x),
        min_y: quantize(min_y),
        max_y: quantize(max_y),
        projection_checksum,
        part_count: artifact.statistics.part_count,
        triangle_count: artifact.statistics.triangle_count,
    }
}

fn quantize(value: f32) -> i32 {
    (value * 1000.0).round() as i32
}
