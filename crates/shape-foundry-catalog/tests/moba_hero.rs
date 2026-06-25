#![forbid(unsafe_code)]

use shape_foundry::{
    ControlKind, ControlValue, FoundryDocumentId, FoundryKitQualityTier, FoundryLock,
    FoundryLockMode, FoundryLockTarget, FoundryPackDocument, FoundryPackExportProfile,
    compile_foundry_document, compile_foundry_pack, foundry_kit_visibility_decision,
    resolve_foundry_catalog, validate_foundry_kit_package,
};
use shape_foundry_catalog::{
    built_in_fixture_catalogs_with_labels, built_in_foundry_kit_package,
    moba_hero::{
        MOBA_HERO_CLAY_SLUG, fixture_catalog, profile_report,
        validate_prepared_template_compatibility,
    },
};
use shape_search::foundry::{
    FOUNDRY_MAX_RESULT_COUNT, FoundryCandidateMode, FoundryCandidateRequest,
    generate_foundry_candidate_plans,
};

#[test]
fn moba_hero_profile_exists_compiles_and_is_preview_gated() {
    let fixture = fixture_catalog();
    assert_eq!(fixture.slug, MOBA_HERO_CLAY_SLUG);
    assert!(
        built_in_fixture_catalogs_with_labels()
            .iter()
            .any(|(label, built_in)| {
                *label == "Hero Foundry, Clay MVP" && built_in.slug == MOBA_HERO_CLAY_SLUG
            })
    );

    let output = compile_foundry_document(&fixture.document, &fixture)
        .unwrap_or_else(|error| panic!("moba hero should compile: {error:#?}"));
    assert!(output.final_conformance.is_accepted());
    assert!(output.artifact.validation_report.is_valid());
    assert!(output.artifact.statistics.triangle_count > 0);
    assert!(output.artifact.statistics.part_count >= 10);

    let package = built_in_foundry_kit_package(MOBA_HERO_CLAY_SLUG).expect("moba hero kit");
    let report = validate_foundry_kit_package(&package);
    assert!(
        report.is_valid(),
        "moba hero kit package should validate: {:?}",
        report.issues
    );
    assert_eq!(package.kit.quality_tier, FoundryKitQualityTier::Prototype);
    assert_ne!(package.kit.quality_tier, FoundryKitQualityTier::Showcase);
    assert!(!package.review_manifest.human_approval_marker);
    assert!(!package.kit.catalog_visibility_policy.default_novice_catalog);
    assert!(
        package
            .kit
            .catalog_visibility_policy
            .developer_preview_catalog
    );
    assert!(
        !foundry_kit_visibility_decision(&package.kit, &package.review_manifest, false).visible
    );
    assert!(foundry_kit_visibility_decision(&package.kit, &package.review_manifest, true).visible);
}

#[test]
fn moba_hero_controls_and_option_groups_are_product_safe() {
    let fixture = fixture_catalog();
    let catalog = resolve_foundry_catalog(&fixture.document, &fixture)
        .expect("moba hero catalog should resolve");
    let primary_controls = catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .collect::<Vec<_>>();
    assert_eq!(primary_controls.len(), 7);
    assert_eq!(catalog.customizer_profile.maximum_primary_controls, 7);

    let expected = [
        "Hero Archetype",
        "Body Proportions",
        "Silhouette",
        "Armor Mass",
        "Head & Face",
        "Hair / Headgear",
        "Weapon / Accessory",
    ];
    for label in expected {
        assert!(
            primary_controls
                .iter()
                .any(|control| control.label == label),
            "missing control {label}"
        );
    }

    let mut option_groups = 0_usize;
    for control in primary_controls {
        assert_product_safe(&control.label);
        let ControlKind::ChoiceGallery { options } = &control.kind else {
            continue;
        };
        option_groups += 1;
        assert!(options.len() >= 5);
        for option in options {
            assert_product_safe(&option.label);
            assert!(option.preview.preview_id.starts_with(&control.id));
            let mut document = fixture.document.clone();
            document.catalog_lock = None;
            document.build_stamp = None;
            document.control_state.insert(
                control.id.clone(),
                ControlValue::Choice(option.value.clone()),
            );
            let output = compile_foundry_document(&document, &fixture).unwrap_or_else(|error| {
                panic!(
                    "moba hero option {}={} should compile: {error:#?}",
                    control.id, option.value
                )
            });
            assert!(output.final_conformance.is_accepted());
            assert!(output.artifact.validation_report.is_valid());
            assert!(output.artifact.statistics.triangle_count > 0);
        }
    }
    assert!(option_groups >= 5);

    let profile = profile_report();
    assert_eq!(profile.profile_id, MOBA_HERO_CLAY_SLUG);
    assert_eq!(profile.source_template_id, "prepared-hero-template-v1");
    assert!(!profile.source_base_fingerprint.is_empty());
    assert!(profile.prepared_template_compatible);
    validate_prepared_template_compatibility().expect("prepared hero compatibility should pass");
    assert!(profile.default_catalog_hidden);
    assert_eq!(profile.primary_controls, expected);
    assert!(
        profile
            .provider_sample_counts
            .values()
            .all(|count| *count >= 5)
    );
    assert_eq!(profile.candidate_directions.len(), 6);
    assert!(
        profile
            .unsupported_claims
            .iter()
            .any(|claim| claim.contains("no Dota"))
    );
}

#[test]
fn moba_hero_candidate_modes_return_six_survivors_with_human_explanations() {
    let fixture = fixture_catalog();
    for (strategy_id, mode) in [
        ("explore", FoundryCandidateMode::Explore),
        ("silhouette", FoundryCandidateMode::Silhouette),
        ("armor_gear", FoundryCandidateMode::Structure),
    ] {
        let output = generate_foundry_candidate_plans(
            &fixture.document,
            &fixture,
            &candidate_request(strategy_id, mode),
        )
        .unwrap_or_else(|error| panic!("{strategy_id} candidates should generate: {error:#?}"));
        assert_eq!(
            output.candidates.len(),
            FOUNDRY_MAX_RESULT_COUNT,
            "{strategy_id} should return six candidates"
        );
        for candidate in &output.candidates {
            let compiled = compile_foundry_document(&candidate.document, &fixture)
                .unwrap_or_else(|error| panic!("candidate should compile: {error:#?}"));
            assert!(compiled.final_conformance.is_accepted());
            assert!(compiled.artifact.validation_report.is_valid());
            assert!(!candidate.diagnostics.changes.is_empty());
            for change in &candidate.diagnostics.changes {
                assert_product_safe(&change.control_label);
                assert_product_safe(&change.message);
                assert!(!change.message.contains(&change.control_id));
            }
        }
    }
}

#[test]
fn moba_hero_locks_are_respected_by_candidate_generation() {
    let fixture = fixture_catalog();
    let mut document = fixture.document.clone();
    document.foundry_locks.push(FoundryLock {
        target: FoundryLockTarget::Control("armor_mass".to_owned()),
        mode: FoundryLockMode::SearchProtected,
        reason: Some("Keep armor consistent across the pack.".to_owned()),
    });

    let output = generate_foundry_candidate_plans(
        &document,
        &fixture,
        &candidate_request("armor_gear", FoundryCandidateMode::Structure),
    )
    .expect("armor gear candidates should generate with one locked control");

    assert_eq!(output.candidates.len(), FOUNDRY_MAX_RESULT_COUNT);
    assert!(output.candidates.iter().all(|candidate| {
        !candidate
            .changed_controls
            .contains(&"armor_mass".to_owned())
    }));
}

#[test]
fn moba_hero_three_member_pack_variants_compile() {
    let fixture = fixture_catalog();
    let mut pack = FoundryPackDocument::new(
        "moba-hero-clay-test-pack",
        fixture.document.family_content_ref.clone(),
        fixture.document.style_content_ref.clone(),
        FoundryPackExportProfile {
            profile: "canonical-model-package".to_owned(),
            require_all_members: true,
        },
    );
    for (name, controls) in pack_members() {
        let mut document = pack_member_document(&fixture, name, &controls);
        let output = compile_foundry_document(&document, &fixture)
            .unwrap_or_else(|error| panic!("{name} should compile: {error:#?}"));
        assert!(output.final_conformance.is_accepted(), "{name}");
        assert!(output.artifact.validation_report.is_valid(), "{name}");
        assert!(output.artifact.statistics.triangle_count > 0, "{name}");
        document.build_stamp = None;
        pack.members.insert(member_id_from_name(name), document);
    }

    let pack_output = compile_foundry_pack(&pack, &fixture)
        .unwrap_or_else(|error| panic!("moba hero pack should compile: {error:#?}"));
    assert_eq!(pack_output.report.members.len(), 3);
    assert!(pack_output.report.conformance_status.accepted);
    assert_eq!(pack_output.member_outputs.len(), 3);
    assert!(pack_output.report.triangle_totals.total > 0);
}

fn member_id_from_name(name: &str) -> String {
    name.to_ascii_lowercase().replace(' ', "-")
}

fn pack_member_document(
    fixture: &shape_foundry_catalog::FoundryFixtureCatalog,
    name: &str,
    controls: &[(&'static str, ControlValue)],
) -> shape_foundry::FoundryAssetDocument {
    let mut document = fixture.document.clone();
    document.document_id = FoundryDocumentId(format!(
        "{}-{}",
        MOBA_HERO_CLAY_SLUG,
        name.to_ascii_lowercase().replace(' ', "-")
    ));
    document.catalog_lock = None;
    document.build_stamp = None;
    for (control, value) in controls {
        document
            .control_state
            .insert((*control).to_owned(), value.clone());
    }
    document
}

fn candidate_request(strategy_id: &str, mode: FoundryCandidateMode) -> FoundryCandidateRequest {
    FoundryCandidateRequest {
        seed: 42,
        proposal_count: 72,
        result_count: FOUNDRY_MAX_RESULT_COUNT,
        mode,
        strategy_id: Some(strategy_id.to_owned()),
        preference_profile: None,
    }
}

fn pack_members() -> [(&'static str, Vec<(&'static str, ControlValue)>); 3] {
    [
        (
            "Duelist Vanguard",
            vec![
                (
                    "hero_archetype",
                    ControlValue::Choice("armored_duelist".to_owned()),
                ),
                (
                    "armor_mass",
                    ControlValue::Choice("duelist_mail".to_owned()),
                ),
                (
                    "weapon_accessory",
                    ControlValue::Choice("blade_and_scabbard".to_owned()),
                ),
            ],
        ),
        (
            "Arcane Ranger",
            vec![
                (
                    "hero_archetype",
                    ControlValue::Choice("arcane_ranger".to_owned()),
                ),
                ("head_face", ControlValue::Choice("arcane_mask".to_owned())),
                (
                    "weapon_accessory",
                    ControlValue::Choice("staff_and_cloak".to_owned()),
                ),
            ],
        ),
        (
            "Monster Hunter",
            vec![
                (
                    "hero_archetype",
                    ControlValue::Choice("monster_hunter".to_owned()),
                ),
                (
                    "hair_headgear",
                    ControlValue::Choice("horned_hood".to_owned()),
                ),
                (
                    "weapon_accessory",
                    ControlValue::Choice("axe_and_trophy".to_owned()),
                ),
            ],
        ),
    ]
}

fn assert_product_safe(value: &str) {
    let lower = value.to_ascii_lowercase();
    for forbidden in [
        "dota",
        "ip comparison",
        "texture",
        "material",
        "uv",
        "rig",
        "animation",
        "advanced recipe",
        "scalar",
        "provider id",
        "semantic id",
        "cage",
        "landmark",
        "compiler",
        "decompiler",
        "operation id",
    ] {
        assert!(
            !lower.contains(forbidden),
            "product-facing metadata contains {forbidden:?}: {value}"
        );
    }
}
