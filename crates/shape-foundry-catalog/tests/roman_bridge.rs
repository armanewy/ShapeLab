use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use shape_asset::{GeometrySource, ModelingOperationSpec};
use shape_compile::export::{verify_model_package, write_model_package};
use shape_compile::validation::{
    ValidationLimits, validate_model, validation_config_from_recipe_with_limits,
};
use shape_family_compile::conformance::ConformanceStatus;
use shape_foundry::{CandidateLegibilityClass, ControlKind, ControlValue, VariationIntent};
use shape_foundry_catalog::{
    CatalogCurationState, catalog_curation_metadata_for_slug, roman_bridge,
};
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
    let strategy_controls = |id: &str| {
        catalog
            .customizer_profile
            .candidate_strategies
            .iter()
            .find(|strategy| strategy.id == id)
            .unwrap_or_else(|| panic!("{id} strategy"))
            .control_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        strategy_controls("light_crossing"),
        vec![
            "span_length",
            "structural_heft",
            "support_style",
            "bracing_style"
        ]
    );
    assert_eq!(
        strategy_controls("wide_deck"),
        vec!["deck_width", "railing_style", "detail_density"]
    );
    assert_eq!(
        strategy_controls("stone_pier_outpost"),
        vec!["support_style", "railing_style", "detail_density"]
    );
    assert_eq!(
        strategy_controls("detailed_timberwork"),
        vec![
            "structural_heft",
            "bracing_style",
            "railing_style",
            "detail_density"
        ]
    );
}

#[test]
fn roman_bridge_hq_dogfood_visible_ideas_are_primary_quick_controls() {
    let dogfood = roman_bridge::hq_dogfood_hardening_v3();
    let fixture = roman_bridge::hq_fixture_catalog();
    let catalog = shape_foundry::resolve_foundry_catalog(&fixture.document, &fixture)
        .expect("HQ roman bridge catalog should resolve");

    let primary_controls = catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary)
        .collect::<Vec<_>>();
    assert!(primary_controls.len() <= catalog.customizer_profile.maximum_primary_controls as usize);
    assert_eq!(catalog.customizer_profile.maximum_primary_controls, 7);
    assert!(primary_controls.iter().all(|control| control.visible));

    let primary_ids = primary_controls
        .iter()
        .map(|control| control.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        dogfood.visible_idea_controls,
        [
            "support_style",
            "deck_width",
            "bracing_style",
            "railing_style",
            "structural_heft"
        ]
    );
    for control_id in dogfood.visible_idea_controls {
        assert!(
            primary_ids.contains(control_id),
            "{control_id} must stay visible as a quick control for bridge dogfood"
        );
    }

    let visible_labels = dogfood
        .visible_idea_controls
        .iter()
        .map(|control_id| {
            primary_controls
                .iter()
                .find(|control| control.id == *control_id)
                .expect("dogfood control should be primary")
                .label
                .as_str()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        visible_labels,
        vec![
            "Support Style",
            "Deck Width",
            "Bracing Style",
            "Railing Style",
            "Structural Heft"
        ]
    );

    let curation =
        catalog_curation_metadata_for_slug("roman-bridge-hq").expect("roman bridge HQ curation");
    if curation.state == CatalogCurationState::Usable {
        assert!(
            primary_controls.len() <= 7,
            "Usable Roman Bridge must expose only quick, novice-safe primary controls"
        );
    } else {
        assert_eq!(curation.state, CatalogCurationState::PreviewOnly);
    }
}

#[test]
fn roman_bridge_hq_preparation_completes_or_reports_long_running_state() {
    let dogfood = roman_bridge::hq_dogfood_hardening_v3();
    let threshold = Duration::from_millis(dogfood.preparation_threshold_ms as u64);
    let fixture = roman_bridge::hq_fixture_catalog();

    let started = Instant::now();
    let output = shape_foundry::compile_foundry_document(&fixture.document, &fixture)
        .expect("HQ roman bridge preparation should compile");
    let elapsed = started.elapsed();

    assert!(output.final_conformance.is_accepted());
    assert!(output.artifact.validation_report.is_valid());
    let preview_mesh = &output.artifact.combined_preview.mesh;
    assert!(
        !preview_mesh.positions.is_empty(),
        "preparation must produce a whole-model preview mesh"
    );
    assert!(
        !preview_mesh.indices.is_empty() && preview_mesh.indices.len().is_multiple_of(3),
        "preparation must produce triangle preview geometry"
    );
    let preparation_state = if elapsed <= threshold {
        PreparationReliabilityState::CompletedUnderThreshold
    } else {
        PreparationReliabilityState::LongRunningReported { elapsed, threshold }
    };
    match preparation_state {
        PreparationReliabilityState::CompletedUnderThreshold => {}
        PreparationReliabilityState::LongRunningReported { elapsed, threshold } => {
            assert!(
                dogfood
                    .app_dependency_notes
                    .iter()
                    .any(|note| note.contains("blocked preparation")),
                "long-running preparation should be documented as an app-owned blocked-preparation dependency"
            );
            assert!(
                elapsed > threshold,
                "long-running state should only be reported after the local threshold"
            );
        }
    }
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

    assert!(
        role_linear_arrays(&output.recipe, "deck")
            .iter()
            .any(|(count, offset)| *count >= 6 && offset[2].abs() > 0.2),
        "HQ deck should compile as visibly separated plank courses"
    );
    assert!(
        role_linear_arrays(&output.recipe, "span")
            .iter()
            .any(|(count, offset)| *count >= 2 && offset[2].abs() >= 0.9),
        "HQ span should keep visible paired main beams"
    );
    assert!(
        role_linear_arrays(&output.recipe, "connector")
            .iter()
            .any(|(count, offset)| *count >= 7 && offset[0].abs() > 0.2),
        "HQ connector details should read as repeated secondary beams/fasteners"
    );
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
        let model_config = validation_config_from_recipe_with_limits(
            &output.recipe,
            &output.artifact,
            ValidationLimits::default(),
        );
        let model_report = validate_model(&output.artifact, &model_config);
        assert!(
            model_report.is_valid(),
            "{provider} should not introduce floating or intersecting supports: {:#?}",
            model_report.issues
        );
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
        let model_config = validation_config_from_recipe_with_limits(
            &output.recipe,
            &output.artifact,
            ValidationLimits::default(),
        );
        let model_report = validate_model(&output.artifact, &model_config);
        assert!(
            model_report.is_valid(),
            "{choice} should not introduce broken or intersecting bracing: {:#?}",
            model_report.issues
        );
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
fn roman_bridge_hq_span_heft_rail_and_detail_controls_are_readable() {
    let fixture = roman_bridge::hq_fixture_catalog();

    let short = compile_with_control(&fixture, "span_length", ControlValue::Scalar(2.8));
    let long = compile_with_control(&fixture, "span_length", ControlValue::Scalar(5.4));
    let short_span = role_bounds(&short.recipe, &short.artifact, "span");
    let long_span = role_bounds(&long.recipe, &long.artifact, "span");
    assert!(
        long_span.width() > short_span.width() + 2.0,
        "span length should visibly change bridge reach: short={short_span:?} long={long_span:?}"
    );

    let light = compile_with_control(&fixture, "structural_heft", ControlValue::Scalar(0.18));
    let heavy = compile_with_control(&fixture, "structural_heft", ControlValue::Scalar(0.82));
    let light_span = role_bounds(&light.recipe, &light.artifact, "span");
    let heavy_span = role_bounds(&heavy.recipe, &heavy.artifact, "span");
    let light_support = role_bounds(&light.recipe, &light.artifact, "support");
    let heavy_support = role_bounds(&heavy.recipe, &heavy.artifact, "support");
    assert!(
        heavy_span.height() > light_span.height() + 0.12,
        "structural heft should deepen the main beams: light={light_span:?} heavy={heavy_span:?}"
    );
    assert!(
        heavy_support.width() > light_support.width() + 0.15,
        "structural heft should thicken support posts/piers: light={light_support:?} heavy={heavy_support:?}"
    );

    let curb = compile_with_control(
        &fixture,
        "railing_style",
        ControlValue::Provider("low_curb_rail".to_owned()),
    );
    let lookout = compile_with_control(
        &fixture,
        "railing_style",
        ControlValue::Provider("lookout_rail_courses".to_owned()),
    );
    let curb_rail = role_bounds(&curb.recipe, &curb.artifact, "rail");
    let lookout_rail = role_bounds(&lookout.recipe, &lookout.artifact, "rail");
    assert!(
        lookout_rail.height() > curb_rail.height() + 0.5,
        "railing style should read as a taller multi-course system: curb={curb_rail:?} lookout={lookout_rail:?}"
    );
    assert!(
        lookout_rail.depth() > curb_rail.depth() + 0.25,
        "railing style should widen the guarded side courses: curb={curb_rail:?} lookout={lookout_rail:?}"
    );

    let clean = compile_with_control(
        &fixture,
        "detail_density",
        ControlValue::Provider("clean_joinery_detail".to_owned()),
    );
    let dense = compile_with_control(
        &fixture,
        "detail_density",
        ControlValue::Provider("dense_weathered_joinery".to_owned()),
    );
    let clean_detail = role_signature(&clean, "connector");
    let dense_detail = role_signature(&dense, "connector");
    assert!(
        dense_detail.triangle_count > clean_detail.triangle_count * 2,
        "detail density should add clearly more fastener geometry: clean={clean_detail:?} dense={dense_detail:?}"
    );
}

#[test]
fn roman_bridge_hq_explore_returns_clear_distinct_whole_asset_directions() {
    let dogfood = roman_bridge::hq_dogfood_hardening_v3();
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
        let model_config = validation_config_from_recipe_with_limits(
            &compiled.recipe,
            &compiled.artifact,
            ValidationLimits::default(),
        );
        let model_report = validate_model(&compiled.artifact, &model_config);
        assert!(
            model_report.is_valid(),
            "candidate {} should survive model validation; controls={:?}; instances={:#?}; compiled_parts={:#?}; issues={:#?}",
            candidate.id.0,
            candidate.changed_controls,
            instance_names(&compiled.recipe),
            compiled_part_names(&compiled.artifact),
            model_report.issues
        );
        visual_signatures.insert(camera_signature(&compiled.artifact));
    }

    assert!(
        visual_signatures.len() >= dogfood.preview_required_visible_ideas,
        "at least {} Explore directions should be visually distinct; got {}",
        dogfood.preview_required_visible_ideas,
        visual_signatures.len()
    );

    let curation =
        catalog_curation_metadata_for_slug("roman-bridge-hq").expect("roman bridge HQ curation");
    assert!(
        dogfood.observed_surviving_directions < dogfood.usable_required_surviving_directions,
        "only a recorded six-survivor benchmark or approved exception can promote Roman Bridge HQ"
    );
    assert_eq!(
        curation.state,
        CatalogCurationState::PreviewOnly,
        "Roman Bridge HQ must remain PreviewOnly while fewer than six directions survive"
    );
    assert_eq!(curation.state, dogfood.tier_decision);
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum PreparationReliabilityState {
    CompletedUnderThreshold,
    LongRunningReported {
        elapsed: Duration,
        threshold: Duration,
    },
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

fn role_linear_arrays(recipe: &shape_asset::AssetRecipe, role: &str) -> Vec<(u32, [f32; 3])> {
    let role_tag = format!("role:{role}");
    recipe
        .definitions
        .values()
        .filter(|definition| definition.tags.contains(&role_tag))
        .flat_map(|definition| {
            definition
                .geometry
                .operations
                .iter()
                .filter_map(|operation| match operation {
                    ModelingOperationSpec::LinearArray { count, offset, .. } => {
                        Some((*count, *offset))
                    }
                    _ => None,
                })
        })
        .collect()
}

fn instance_names(recipe: &shape_asset::AssetRecipe) -> Vec<(u64, String)> {
    recipe
        .instances
        .iter()
        .map(|(id, instance)| (id.0, instance.name.clone()))
        .collect()
}

fn compiled_part_names(artifact: &shape_compile::AssetArtifact) -> Vec<(u64, String)> {
    artifact
        .compiled_parts
        .iter()
        .map(|part| (part.instance_id.0, part.instance_name.clone()))
        .collect()
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
