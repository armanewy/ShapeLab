#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use shape_foundry::{
    ControlKind, ControlValue, FoundryKitQualityTier, compile_foundry_document,
    resolve_foundry_catalog, validate_foundry_kit_package,
};
use shape_foundry_catalog::{
    built_in_fixture_catalogs_with_labels, built_in_foundry_kit_package,
    showcase_gear::{SHOWCASE_GEAR_SLUGS, showcase_gear_pack_report},
};

#[test]
fn showcase_gear_kits_exist_compile_and_are_review_gated_usable() {
    let fixtures = gear_fixtures_by_slug();

    for slug in SHOWCASE_GEAR_SLUGS {
        let (_label, fixture) = fixtures.get(slug).expect("showcase gear fixture exists");
        let output = compile_foundry_document(&fixture.document, fixture)
            .unwrap_or_else(|error| panic!("{slug} should compile: {error:#?}"));
        assert!(
            output.final_conformance.is_accepted(),
            "{slug} final conformance should pass: {:#?}",
            output.final_conformance
        );
        assert!(
            output.artifact.validation_report.is_valid(),
            "{slug} generated artifact should validate"
        );
        assert!(
            output.artifact.statistics.part_count >= 4,
            "{slug} should be a whole-model multi-part asset"
        );

        let package = built_in_foundry_kit_package(slug).expect("showcase gear kit package");
        let report = validate_foundry_kit_package(&package);
        assert!(
            report.is_valid(),
            "{slug} kit package should validate: {:?}",
            report.issues
        );
        assert_eq!(package.kit.quality_tier, FoundryKitQualityTier::Usable);
        assert_ne!(package.kit.quality_tier, FoundryKitQualityTier::Showcase);
        assert!(!package.review_manifest.human_approval_marker);
        assert!(package.kit.preview_camera_policy.clay_preview_required);
        assert!(package.kit.preview_camera_policy.contact_sheet_required);
        assert_eq!(
            package.quality_gate_profile.required_tier,
            FoundryKitQualityTier::Usable
        );
        assert!(
            package
                .candidate_strategy_pack
                .diversity_goals
                .iter()
                .any(|goal| goal.contains("Six coherent whole-model options"))
        );
        assert!(
            package
                .review_manifest
                .benchmark_refs
                .iter()
                .any(|path| path == &format!("target/hq-benchmark/{slug}/quality-report.json"))
        );

        let primary_controls = package
            .control_profile
            .controls
            .iter()
            .filter(|control| control.primary && control.visible)
            .count();
        assert!(
            primary_controls <= 7,
            "{slug} should keep the novice deck at seven controls or fewer"
        );
    }
}

#[test]
fn showcase_gear_option_tiles_have_whole_model_preview_refs_and_compile() {
    let fixtures = gear_fixtures_by_slug();

    for slug in SHOWCASE_GEAR_SLUGS {
        let (_label, fixture) = fixtures.get(slug).expect("showcase gear fixture exists");
        let catalog = resolve_foundry_catalog(&fixture.document, fixture)
            .unwrap_or_else(|error| panic!("{slug} catalog should resolve: {error:#?}"));
        let mut option_controls = 0_usize;
        for control in &catalog.customizer_profile.controls {
            let ControlKind::ChoiceGallery { options } = &control.kind else {
                continue;
            };
            option_controls += 1;
            assert!(options.len() >= 3);
            for option in options {
                assert!(
                    option.preview.preview_id.starts_with(&control.id),
                    "{slug} {} option {} should use a stable whole-model preview id",
                    control.id,
                    option.value
                );
                let mut document = fixture.document.clone();
                document.catalog_lock = None;
                document.build_stamp = None;
                document.control_state.insert(
                    control.id.clone(),
                    ControlValue::Choice(option.value.clone()),
                );
                let output = compile_foundry_document(&document, fixture).unwrap_or_else(|error| {
                    panic!(
                        "{slug} option {}={} should compile: {error:#?}",
                        control.id, option.value
                    )
                });
                assert!(output.final_conformance.is_accepted());
                assert!(output.artifact.validation_report.is_valid());
                assert!(output.artifact.statistics.triangle_count > 0);
            }
        }
        assert!(
            option_controls >= 2,
            "{slug} should expose whole-model silhouette and ornament option tiles"
        );
    }
}

#[test]
fn showcase_gear_pack_report_passes_and_user_metadata_is_product_safe() {
    let report = showcase_gear_pack_report();
    assert!(report.passed);
    assert_eq!(report.kit_slugs, SHOWCASE_GEAR_SLUGS);
    assert_eq!(report.minimum_target_tier, "Usable");
    assert!(report.showcase_requires_human_approval);
    assert_eq!(report.selected_kit_count, SHOWCASE_GEAR_SLUGS.len());
    assert_eq!(report.valid_kit_count, SHOWCASE_GEAR_SLUGS.len());
    assert_eq!(report.usable_kit_count, SHOWCASE_GEAR_SLUGS.len());
    assert!(report.maximum_primary_control_count <= 7);
    assert_eq!(report.option_preview_kit_count, SHOWCASE_GEAR_SLUGS.len());
    assert_eq!(report.review_evidence_ref_count, SHOWCASE_GEAR_SLUGS.len());
    assert_eq!(report.showcase_without_human_approval_count, 0);
    assert_product_safe(&report.display_name);
    assert_product_safe(&report.style_language);
    assert_product_safe(&report.readiness);

    for slug in SHOWCASE_GEAR_SLUGS {
        let package = built_in_foundry_kit_package(slug).expect("showcase gear kit package");
        assert_product_safe(&package.kit.display_name);
        assert_product_safe(&package.style_pack.display_name);
        for chip in &package.kit.category_chips {
            assert_product_safe(chip);
        }
        if let Some(reason) = &package.kit.catalog_visibility_policy.hidden_reason {
            assert_product_safe(reason);
        }
        for control in &package.control_profile.controls {
            assert_product_safe(&control.label);
            assert_product_safe(&control.description);
        }
        for option in &package.provider_pack.provider_options {
            assert_product_safe(&option.label);
        }
    }
}

fn gear_fixtures_by_slug()
-> BTreeMap<String, (&'static str, shape_foundry_catalog::FoundryFixtureCatalog)> {
    built_in_fixture_catalogs_with_labels()
        .into_iter()
        .filter(|(_, fixture)| SHOWCASE_GEAR_SLUGS.contains(&fixture.slug.as_str()))
        .map(|(label, fixture)| (fixture.slug.clone(), (label, fixture)))
        .collect()
}

fn assert_product_safe(value: &str) {
    let lower = value.to_ascii_lowercase();
    for forbidden in [
        "legacy implicit mode",
        "asset modeling lab",
        "modeling workspace",
        "advanced recipe",
        "providerpack",
        "provider pack",
        "socket",
        "port id",
        "raw recipe",
        "recipe",
        "raw scalar path",
        "scalar",
        "provider id",
        "provider",
        "semantic id",
        "semantic",
        "operation id",
        "operation",
        "compiler",
        "decompiler",
        "sdf",
        "fragment remap",
        "fragment",
        "remap",
        "role binding",
        "role provider",
        "conformance binding",
        "conformance",
        "family facet",
    ] {
        assert!(
            !lower.contains(forbidden),
            "novice-visible metadata contains technical term {forbidden:?}: {value}"
        );
    }
}
