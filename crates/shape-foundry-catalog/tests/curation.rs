#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use shape_foundry_catalog::{
    CatalogCurationState, built_in_catalog_curation_metadata,
    built_in_fixture_catalogs_with_labels, catalog_curation_metadata_for_slug,
    curated_fixture_catalogs_with_labels, moba_hero::MOBA_HERO_CLAY_SLUG,
};

#[test]
fn curation_metadata_covers_every_built_in_profile_once() {
    let fixture_slugs = built_in_fixture_catalogs_with_labels()
        .into_iter()
        .map(|(_, fixture)| fixture.slug)
        .collect::<BTreeSet<_>>();
    let curation = built_in_catalog_curation_metadata();
    let curation_slugs = curation
        .iter()
        .map(|metadata| metadata.profile_slug.to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(curation.len(), curation_slugs.len());
    assert_eq!(curation_slugs, fixture_slugs);
    assert!(
        curation
            .iter()
            .all(|metadata| metadata.policy_invariants_pass()),
        "curation metadata must enforce Usable evidence and Showcase human review"
    );
}

#[test]
fn default_catalog_hides_preview_only_and_hidden_draft_profiles() {
    let default_slugs = curated_fixture_catalogs_with_labels(false)
        .into_iter()
        .map(|(_, fixture)| fixture.slug)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        default_slugs,
        BTreeSet::from([
            "roman-bridge-hq".to_owned(),
            "sci-fi-crate".to_owned(),
            "stylized-lamp".to_owned(),
        ])
    );
    assert!(!default_slugs.contains("roman-bridge"));
    assert!(!default_slugs.contains("market-stall"));
    assert!(!default_slugs.contains(MOBA_HERO_CLAY_SLUG));
}

#[test]
fn preview_catalog_shows_preview_only_but_not_hidden_drafts() {
    let preview_slugs = curated_fixture_catalogs_with_labels(true)
        .into_iter()
        .map(|(_, fixture)| fixture.slug)
        .collect::<BTreeSet<_>>();

    assert!(preview_slugs.contains("roman-bridge"));
    assert!(preview_slugs.contains("market-stall"));
    assert!(preview_slugs.contains("fantasy-sword"));
    assert!(preview_slugs.contains("roman-bridge-hq"));
    assert!(!preview_slugs.contains(MOBA_HERO_CLAY_SLUG));
    assert_eq!(preview_slugs.len(), 16);
}

#[test]
fn hq_crate_bridge_and_lamp_are_usable_but_not_showcase() {
    for slug in ["sci-fi-crate", "roman-bridge-hq", "stylized-lamp"] {
        let metadata = catalog_curation_metadata_for_slug(slug).expect("curation metadata");
        assert_eq!(metadata.state, CatalogCurationState::Usable);
        assert!(metadata.has_visual_direction_evidence);
        assert!(metadata.has_readable_control_evidence);
        assert!(!metadata.has_human_showcase_review);
    }

    assert_eq!(
        catalog_curation_metadata_for_slug("roman-bridge")
            .expect("legacy bridge metadata")
            .state,
        CatalogCurationState::PreviewOnly
    );
}

#[test]
fn weak_profiles_cannot_claim_usable_without_legibility_evidence() {
    for slug in [
        "market-stall",
        "storage-barrel",
        "workshop-chair",
        "handcart",
        "stylized-tree",
        MOBA_HERO_CLAY_SLUG,
    ] {
        let metadata = catalog_curation_metadata_for_slug(slug).expect("curation metadata");
        assert_ne!(metadata.state, CatalogCurationState::Usable);
        assert_ne!(metadata.state, CatalogCurationState::Showcase);
    }

    assert!(
        built_in_catalog_curation_metadata()
            .iter()
            .filter(|metadata| {
                matches!(
                    metadata.state,
                    CatalogCurationState::Usable | CatalogCurationState::Showcase
                )
            })
            .all(|metadata| {
                metadata.has_visual_direction_evidence && metadata.has_readable_control_evidence
            })
    );
}
