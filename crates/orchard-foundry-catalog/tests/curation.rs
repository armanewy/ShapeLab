#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use orchard_foundry_catalog::{
    CatalogCurationState, StarterTemplateQualityEvidence, box_primitive,
    built_in_catalog_curation_metadata, built_in_fixture_catalogs_with_labels,
    catalog_curation_metadata_for_slug, curated_fixture_catalogs_with_labels, flat_panel,
    panel_knob, sphere_primitive, starter_template_curation_state_from_quality,
};

#[test]
fn curation_metadata_covers_visible_starter_profiles_once() {
    let fixture_slugs = built_in_fixture_catalogs_with_labels()
        .into_iter()
        .map(|(_, fixture)| fixture.slug)
        .collect::<BTreeSet<_>>();
    let curation = built_in_catalog_curation_metadata();
    let curation_slugs = curation
        .iter()
        .map(|metadata| metadata.profile_slug.to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        fixture_slugs,
        BTreeSet::from([
            box_primitive::BOX_PRIMITIVE_SLUG.to_owned(),
            box_primitive::LIDDED_BOX_SLUG.to_owned(),
            flat_panel::FLAT_PANEL_PRIMITIVE_SLUG.to_owned(),
            sphere_primitive::SPHERE_PRIMITIVE_SLUG.to_owned(),
            flat_panel::HINGED_PANEL_SLUG.to_owned(),
            flat_panel::HANDLED_PANEL_SLUG.to_owned(),
            panel_knob::PANEL_KNOB_SLUG.to_owned(),
        ])
    );
    assert_eq!(curation.len(), 7);
    assert_eq!(curation_slugs, fixture_slugs);
    assert!(
        curation
            .iter()
            .all(|metadata| metadata.policy_invariants_pass()),
        "curation metadata must enforce Usable evidence and Showcase human review"
    );
}

#[test]
fn default_and_preview_catalogs_show_supported_starter_profiles() {
    for preview_enabled in [false, true] {
        let slugs = curated_fixture_catalogs_with_labels(preview_enabled)
            .into_iter()
            .map(|(_, fixture)| fixture.slug)
            .collect::<Vec<_>>();
        assert_eq!(
            slugs,
            vec![
                box_primitive::BOX_PRIMITIVE_SLUG,
                box_primitive::LIDDED_BOX_SLUG,
                flat_panel::FLAT_PANEL_PRIMITIVE_SLUG,
                sphere_primitive::SPHERE_PRIMITIVE_SLUG,
                flat_panel::HINGED_PANEL_SLUG,
                flat_panel::HANDLED_PANEL_SLUG,
                panel_knob::PANEL_KNOB_SLUG,
            ]
        );
    }
}

#[test]
fn box_primitive_is_usable_but_not_showcase() {
    let metadata = catalog_curation_metadata_for_slug(box_primitive::BOX_PRIMITIVE_SLUG)
        .expect("box primitive curation metadata");

    assert_eq!(metadata.state, CatalogCurationState::Usable);
    assert!(metadata.default_novice_visible());
    assert!(metadata.has_visual_direction_evidence);
    assert!(metadata.has_readable_control_evidence);
    assert!(!metadata.has_human_showcase_review);
}

#[test]
fn lidded_box_is_usable_after_lid_seam_gate_but_not_showcase() {
    let metadata = catalog_curation_metadata_for_slug(box_primitive::LIDDED_BOX_SLUG)
        .expect("lidded box curation metadata");

    assert_eq!(metadata.state, CatalogCurationState::Usable);
    assert!(metadata.default_novice_visible());
    assert!(metadata.has_visual_direction_evidence);
    assert!(metadata.has_readable_control_evidence);
    assert!(!metadata.has_human_showcase_review);
    assert!(metadata.note.contains("no crate claim"));
}

#[test]
fn hinged_panel_is_usable_after_hinge_edge_gate_but_not_door() {
    let metadata = catalog_curation_metadata_for_slug(flat_panel::HINGED_PANEL_SLUG)
        .expect("hinged panel curation metadata");

    assert_eq!(metadata.state, CatalogCurationState::Usable);
    assert!(metadata.default_novice_visible());
    assert!(metadata.has_visual_direction_evidence);
    assert!(metadata.has_readable_control_evidence);
    assert!(!metadata.has_human_showcase_review);
    assert!(metadata.note.contains("Hinge Edge"));
    assert!(metadata.note.contains("does not claim Door"));
}

#[test]
fn flat_panel_is_usable_second_kernel_but_not_showcase() {
    let metadata = catalog_curation_metadata_for_slug(flat_panel::FLAT_PANEL_PRIMITIVE_SLUG)
        .expect("flat panel curation metadata");

    assert_eq!(metadata.state, CatalogCurationState::Usable);
    assert!(metadata.default_novice_visible());
    assert!(metadata.has_visual_direction_evidence);
    assert!(metadata.has_readable_control_evidence);
    assert!(!metadata.has_human_showcase_review);
    assert!(metadata.note.contains("second kernel proof"));
    assert!(metadata.note.contains("no Door"));
}

#[test]
fn sphere_primitive_is_usable_direct_primitive() {
    let metadata = catalog_curation_metadata_for_slug(sphere_primitive::SPHERE_PRIMITIVE_SLUG)
        .expect("sphere curation metadata");

    assert_eq!(metadata.state, CatalogCurationState::Usable);
    assert!(metadata.default_novice_visible());
    assert!(metadata.has_visual_direction_evidence);
    assert!(metadata.has_readable_control_evidence);
    assert!(!metadata.has_human_showcase_review);
    assert!(metadata.note.contains("Sphere Primitive"));
}

#[test]
fn handled_panel_is_usable_but_not_motion_or_door() {
    let metadata = catalog_curation_metadata_for_slug(flat_panel::HANDLED_PANEL_SLUG)
        .expect("handled panel curation metadata");

    assert_eq!(metadata.state, CatalogCurationState::Usable);
    assert!(metadata.default_novice_visible());
    assert!(metadata.note.contains("Handle / Knob"));
    assert!(metadata.note.contains("does not claim Door"));
}

#[test]
fn panel_knob_is_usable_safe_anchor_composition() {
    let metadata = catalog_curation_metadata_for_slug(panel_knob::PANEL_KNOB_SLUG)
        .expect("panel knob curation metadata");

    assert_eq!(metadata.state, CatalogCurationState::Usable);
    assert!(metadata.default_novice_visible());
    assert!(metadata.note.contains("safe-anchor composition"));
    assert!(metadata.note.contains("no Door"));
}

fn passing_starter_template_quality_evidence() -> StarterTemplateQualityEvidence {
    StarterTemplateQualityEvidence {
        profile_slug: box_primitive::BOX_PRIMITIVE_SLUG,
        visible_idea_count: 4,
        distinct_visible_idea_count: 4,
        primary_control_count: 2,
        endpoint_reported_primary_control_count: 2,
        endpoint_readable_primary_control_count: 2,
        returned_too_subtle_candidate_count: 0,
        broken_or_floating_part_count: 0,
        export_conformance_clean: true,
        advanced_recipe_required: false,
        raw_technical_summary_count: 0,
    }
}

#[test]
fn starter_template_quality_passing_template_can_be_usable() {
    let evidence = passing_starter_template_quality_evidence();

    assert!(evidence.passes_benchmark());
    assert_eq!(
        starter_template_curation_state_from_quality(evidence),
        CatalogCurationState::Usable
    );
}

#[test]
fn starter_template_quality_failing_template_cannot_be_usable() {
    let mut evidence = passing_starter_template_quality_evidence();
    evidence.visible_idea_count = 3;
    evidence.distinct_visible_idea_count = 3;

    assert!(!evidence.passes_benchmark());
    assert_eq!(
        starter_template_curation_state_from_quality(evidence),
        CatalogCurationState::PreviewOnly
    );
    assert!(
        !starter_template_curation_state_from_quality(evidence).default_novice_visible(),
        "failed starters must stay out of the default novice catalog"
    );
}

#[test]
fn starter_template_quality_missing_endpoint_reports_stay_preview_only() {
    let mut evidence = passing_starter_template_quality_evidence();
    evidence.endpoint_reported_primary_control_count = 1;
    evidence.endpoint_readable_primary_control_count = 1;

    assert!(!evidence.passes_benchmark());
    assert_eq!(
        starter_template_curation_state_from_quality(evidence),
        CatalogCurationState::PreviewOnly
    );
}
