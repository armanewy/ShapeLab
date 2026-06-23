#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use shape_foundry::{
    ControlEvaluationContext, ControlKind, ControlTopologyBehavior, ControlValue,
    FoundryAssetDocument, FoundryDocumentId, FoundryPackDocument, FoundryPackExportProfile,
    compile_foundry_document, compile_foundry_pack, default_control_value,
    whole_model_preview_sample_requests_with_count,
};
use shape_foundry_catalog::{built_in_fixture_catalogs_with_labels, headless_fixture_catalogs};

#[test]
fn built_in_catalog_exposes_ten_labeled_profiles() {
    let labeled = built_in_fixture_catalogs_with_labels();
    assert_eq!(labeled.len(), 10);
    assert_eq!(headless_fixture_catalogs().len(), 10);

    let labels = labeled
        .iter()
        .map(|(label, _)| *label)
        .collect::<BTreeSet<_>>();
    assert_eq!(labels.len(), 10);
    for expected in [
        "Roman Timber Bridge",
        "Sci-Fi Industrial Crate",
        "Stylized Furniture Lamp",
        "Market Stall Kit",
        "Sci-Fi Door Panel",
        "Coopered Storage Barrel",
        "Wayfinding Signpost",
        "Workshop Chair",
        "Market Handcart",
        "Storybook Tree",
    ] {
        assert!(labels.contains(expected), "missing {expected}");
    }
}

#[test]
fn every_built_in_profile_resolves_compiles_and_stays_novice_sized() {
    let fixtures = headless_fixture_catalogs();
    let slugs = fixtures
        .iter()
        .map(|fixture| fixture.slug.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(slugs.len(), fixtures.len());

    for fixture in fixtures {
        let catalog = shape_foundry::resolve_foundry_catalog(&fixture.document, &fixture)
            .unwrap_or_else(|error| panic!("{} catalog should resolve: {error:#?}", fixture.slug));
        let primary_controls = catalog
            .customizer_profile
            .controls
            .iter()
            .filter(|control| control.primary && control.visible)
            .count();
        assert_eq!(
            primary_controls, 7,
            "{} should expose exactly seven primary visible controls",
            fixture.slug
        );
        assert_eq!(catalog.customizer_profile.maximum_primary_controls, 7);
        assert!(
            !catalog.customizer_profile.candidate_strategies.is_empty(),
            "{} should expose whole-model candidate strategies",
            fixture.slug
        );

        let output = shape_foundry::compile_foundry_document(&fixture.document, &fixture)
            .unwrap_or_else(|error| panic!("{} should compile: {error:#?}", fixture.slug));
        assert!(
            output.final_conformance.is_accepted(),
            "{} final conformance should pass: {:#?}",
            fixture.slug,
            output.final_conformance
        );
        assert!(
            output.artifact.validation_report.is_valid(),
            "{} compiled artifact should validate",
            fixture.slug
        );
        assert!(
            output.artifact.statistics.part_count >= 3,
            "{} should compile to a multi-part whole-model asset",
            fixture.slug
        );
        if let Some(control) = catalog
            .customizer_profile
            .controls
            .iter()
            .find(|control| control.id == "detail_density")
        {
            let ControlKind::IntegerStepper { default } = &control.kind else {
                panic!(
                    "{} detail_density should be an integer control",
                    fixture.slug
                );
            };
            assert_eq!(
                fixture.document.control_state.get("detail_density"),
                Some(&ControlValue::Integer(*default)),
                "{} detail_density reset default should match the document default",
                fixture.slug
            );
        }
    }
}

#[test]
fn author_templates_cover_every_built_in_profile() {
    for fixture in headless_fixture_catalogs() {
        let profile = shape_foundry_catalog::author_profile_template(&fixture.slug)
            .unwrap_or_else(|| panic!("{} should have an author template", fixture.slug));
        assert_eq!(profile.package_id, fixture.slug);
        assert_eq!(profile.customizer_profile.maximum_primary_controls, 7);
    }

    let compact_crate = shape_foundry_catalog::author_profile_template("scifi-crate")
        .expect("compact sci-fi crate alias should resolve");
    assert_eq!(compact_crate.package_id, "sci-fi-crate");
}

#[test]
fn expansion_profiles_compile_coherent_three_member_packs() {
    for fixture in headless_fixture_catalogs()
        .into_iter()
        .filter(|fixture| is_expansion_profile(&fixture.slug))
    {
        let mut pack = FoundryPackDocument::new(
            format!("{}-wave26-pack", fixture.slug),
            fixture.document.family_content_ref.clone(),
            fixture.document.style_content_ref.clone(),
            FoundryPackExportProfile {
                profile: "canonical-model-package".to_owned(),
                require_all_members: true,
            },
        );
        pack.members.insert(
            "base".to_owned(),
            member_document(&fixture.document, "base", [0.5, 0.5, 0.5]),
        );
        pack.members.insert(
            "wide".to_owned(),
            member_document(&fixture.document, "wide", [0.85, 0.45, 0.7]),
        );
        pack.members.insert(
            "tall".to_owned(),
            member_document(&fixture.document, "tall", [0.35, 0.85, 0.45]),
        );

        let output = compile_foundry_pack(&pack, &fixture)
            .unwrap_or_else(|error| panic!("{} pack should compile: {error:#?}", fixture.slug));
        assert_eq!(output.report.members.len(), 3);
        assert!(output.report.conformance_status.accepted);
        assert!(output.report.triangle_totals.total > 0);
        assert_eq!(output.member_outputs.len(), 3);
    }
}

#[test]
fn expansion_profiles_have_six_valid_refine_supplement_samples() {
    for fixture in headless_fixture_catalogs()
        .into_iter()
        .filter(|fixture| is_expansion_profile(&fixture.slug))
    {
        let parent = compile_foundry_document(&fixture.document, &fixture)
            .unwrap_or_else(|error| panic!("{} parent should compile: {error:#?}", fixture.slug));
        let context = ControlEvaluationContext::new(&parent.catalog.family.parameter_slots);
        let mut recipe_fingerprints =
            BTreeSet::from([parent.build_stamp.recipe_fingerprint.0.to_hex()]);
        let mut valid_samples = 0_usize;
        let mut controls = parent
            .catalog
            .customizer_profile
            .controls
            .iter()
            .filter(|control| {
                control.primary
                    && control.visible
                    && control.topology_behavior == ControlTopologyBehavior::TopologyPreserving
            })
            .collect::<Vec<_>>();
        controls.sort_by(|left, right| left.id.cmp(&right.id));

        'controls: for control in controls {
            let current = fixture
                .document
                .control_state
                .get(&control.id)
                .cloned()
                .unwrap_or_else(|| {
                    default_control_value(control, context)
                        .unwrap_or_else(|error| panic!("{} default failed: {error:#?}", control.id))
                });
            let samples = whole_model_preview_sample_requests_with_count(control, context, 5)
                .unwrap_or_else(|error| panic!("{} samples failed: {error:#?}", control.id));
            for sample in samples {
                if sample.value == current {
                    continue;
                }
                let mut document = fixture.document.clone();
                document.catalog_lock = None;
                document.build_stamp = None;
                document
                    .control_state
                    .insert(control.id.clone(), sample.value);
                let output =
                    compile_foundry_document(&document, &fixture).unwrap_or_else(|error| {
                        panic!(
                            "{} refine supplement {} should compile: {error:#?}",
                            fixture.slug, control.id
                        )
                    });
                assert!(output.final_conformance.is_accepted());
                assert!(output.artifact.validation_report.is_valid());
                if recipe_fingerprints.insert(output.build_stamp.recipe_fingerprint.0.to_hex()) {
                    valid_samples += 1;
                }
                if valid_samples == 6 {
                    break 'controls;
                }
            }
        }

        assert_eq!(
            valid_samples, 6,
            "{} should expose six valid topology-preserving refine samples",
            fixture.slug
        );
    }
}

fn is_expansion_profile(slug: &str) -> bool {
    matches!(
        slug,
        "market-stall"
            | "sci-fi-door"
            | "storage-barrel"
            | "signpost"
            | "workshop-chair"
            | "handcart"
            | "stylized-tree"
    )
}

fn member_document(
    document: &FoundryAssetDocument,
    suffix: &str,
    dimensions: [f32; 3],
) -> FoundryAssetDocument {
    let mut document = document.clone();
    document.document_id = FoundryDocumentId(format!("{}-{suffix}", document.document_id.0));
    document
        .control_state
        .insert("width".to_owned(), ControlValue::Scalar(dimensions[0]));
    document
        .control_state
        .insert("height".to_owned(), ControlValue::Scalar(dimensions[1]));
    document
        .control_state
        .insert("depth".to_owned(), ControlValue::Scalar(dimensions[2]));
    document.catalog_lock = None;
    document.build_stamp = None;
    document
}
