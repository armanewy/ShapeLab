use shape_foundry::{
    ControlKind, FoundryKitQualityTier, compile_foundry_document, foundry_kit_visibility_decision,
    validate_foundry_kit_package,
};
use shape_foundry_catalog::{
    built_in_foundry_kit_package,
    moba_hero::{
        MOBA_HERO_CLAY_SLUG, fixture_catalog, profile_report,
        validate_prepared_template_compatibility,
    },
};

#[test]
fn moba_hero_profile_is_a_hidden_prototype_with_seven_controls() {
    let fixture = fixture_catalog();
    let output = compile_foundry_document(&fixture.document, &fixture)
        .unwrap_or_else(|error| panic!("moba hero should compile: {error:#?}"));
    let primary_controls = output
        .catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .collect::<Vec<_>>();

    assert_eq!(fixture.slug, MOBA_HERO_CLAY_SLUG);
    assert_eq!(primary_controls.len(), 7);
    assert!(
        primary_controls
            .iter()
            .any(|control| control.label == "Hero Archetype")
    );
    assert!(
        primary_controls
            .iter()
            .any(|control| control.label == "Armor Mass")
    );
    assert!(
        primary_controls
            .iter()
            .any(|control| control.label == "Weapon / Accessory")
    );
    assert!(
        primary_controls
            .iter()
            .filter(|control| matches!(control.kind, ControlKind::ChoiceGallery { .. }))
            .count()
            >= 5
    );
    assert!(output.final_conformance.is_accepted());
    assert!(output.artifact.validation_report.is_valid());

    let package = built_in_foundry_kit_package(MOBA_HERO_CLAY_SLUG).expect("moba hero kit");
    let package_report = validate_foundry_kit_package(&package);
    assert!(
        package_report.is_valid(),
        "moba hero kit metadata should validate: {:?}",
        package_report.issues
    );
    assert_eq!(package.kit.quality_tier, FoundryKitQualityTier::Prototype);
    assert_ne!(package.kit.quality_tier, FoundryKitQualityTier::Showcase);
    assert!(!package.review_manifest.human_approval_marker);
    assert!(!package.review_manifest.adversarial_review_marker);
    assert!(
        !foundry_kit_visibility_decision(&package.kit, &package.review_manifest, false).visible
    );
    assert!(foundry_kit_visibility_decision(&package.kit, &package.review_manifest, true).visible);
}

#[test]
fn moba_hero_profile_report_preserves_product_boundary() {
    let report = profile_report();

    assert_eq!(report.profile_id, MOBA_HERO_CLAY_SLUG);
    assert_eq!(report.source_template_id, "prepared-hero-template-v1");
    assert!(!report.source_base_fingerprint.is_empty());
    assert!(report.prepared_template_compatible);
    validate_prepared_template_compatibility().expect("prepared hero compatibility should pass");
    assert!(report.default_catalog_hidden);
    assert_eq!(report.primary_controls.len(), 7);
    assert_eq!(report.candidate_directions.len(), 6);
    assert!(
        report
            .unsupported_claims
            .iter()
            .any(|claim| claim.contains("no Dota"))
    );
    assert!(
        report
            .unsupported_claims
            .iter()
            .any(|claim| claim.contains("no textures"))
    );
}
