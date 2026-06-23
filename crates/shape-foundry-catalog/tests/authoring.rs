#![forbid(unsafe_code)]

use std::fs;

use shape_foundry::CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE;
use shape_foundry_catalog::{
    FoundryAuthorPreviewCamera, FoundryCatalogManifest, author_profile_template,
    headless_fixture_catalogs, validate_author_profile_package,
};

#[test]
fn built_in_fixtures_round_trip_through_author_packages() {
    for fixture in headless_fixture_catalogs() {
        let profile = author_profile_template(&fixture.slug)
            .unwrap_or_else(|| panic!("{} should have author template", fixture.slug));
        let report = validate_author_profile_package(&profile);
        assert!(
            report.is_valid(),
            "{} author profile should validate: {:#?}",
            fixture.slug,
            report.issues
        );
        assert_eq!(report.primary_control_count, 7);
        assert!(report.candidate_strategy_count > 0);
        assert_eq!(report.preview_camera_count, 1);
        assert_eq!(report.pack_policy_count, 1);
        assert_eq!(report.catalog_entry_count, 5);
        assert!(report.build_fingerprint.is_some());
        assert!(report.compiled_part_count.unwrap() > 0);
        assert!(report.triangle_count.unwrap() > 0);

        let packaged = profile.to_fixture_catalog();
        assert_eq!(packaged.entries.len(), 5);
        assert!(
            packaged
                .document
                .catalog_lock
                .as_ref()
                .unwrap()
                .exact_refs
                .contains_key(CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE)
        );
        let output = shape_foundry::compile_foundry_document(&packaged.document, &packaged)
            .expect("author package catalog should compile");
        assert!(output.final_conformance.is_accepted());
        assert!(output.artifact.validation_report.is_valid());
    }
}

#[test]
fn author_package_writes_exact_local_catalog() {
    let mut profile = author_profile_template("roman-bridge").expect("template");
    profile.package_version = 7;
    let catalog = profile.to_fixture_catalog();
    let temp_dir = tempfile::tempdir().expect("temp dir");
    catalog
        .write_to_dir(temp_dir.path())
        .expect("write author package catalog");

    for name in [
        "foundry-document.json",
        "catalog-manifest.json",
        "roman-bridge-family.json",
        "roman-bridge-style.json",
        "roman-bridge-family-impl.json",
        "roman-bridge-style-impl.json",
        "roman-bridge-profile.json",
    ] {
        let path = temp_dir.path().join(name);
        assert!(path.exists(), "{name} should exist");
        assert!(
            path.metadata().unwrap().len() > 0,
            "{name} should not be empty"
        );
    }

    let document_json = fs::read_to_string(temp_dir.path().join("foundry-document.json")).unwrap();
    let document: shape_foundry::FoundryAssetDocument =
        serde_json::from_str(&document_json).unwrap();
    assert_eq!(document.document_id.0, "roman-bridge-doc");
    assert_eq!(
        document.catalog_lock.as_ref().unwrap().catalog_version,
        profile.package_version
    );

    let manifest_json = fs::read_to_string(temp_dir.path().join("catalog-manifest.json")).unwrap();
    let manifest: FoundryCatalogManifest = serde_json::from_str(&manifest_json).unwrap();
    assert_eq!(manifest.catalog_version, profile.package_version);
}

#[test]
fn author_validation_rejects_bad_metadata_before_compile() {
    let mut profile = author_profile_template("sci-fi-crate").expect("template");
    profile.preview_cameras.push(FoundryAuthorPreviewCamera {
        id: "default".to_owned(),
        label: "Duplicate".to_owned(),
        width: 0,
        height: 512,
        orbit_degrees: [0.0, 0.0, 0.0],
    });
    profile.pack_policies[0]
        .shared_control_ids
        .push("missing-control".to_owned());

    let report = validate_author_profile_package(&profile);
    let codes = report
        .issues
        .iter()
        .map(|issue| issue.code.as_str())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"duplicate_author_preview_camera"));
    assert!(codes.contains(&"invalid_author_preview_camera_size"));
    assert!(codes.contains(&"unknown_author_pack_shared_control"));
    assert!(
        report.build_fingerprint.is_none(),
        "invalid metadata should not be compile-proved"
    );
}

#[test]
fn author_validation_rejects_cross_reference_drift() {
    let mut profile = author_profile_template("stylized-lamp").expect("template");
    profile.customizer_profile.family_id = "crate".to_owned();

    let report = validate_author_profile_package(&profile);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "author_profile_family_mismatch")
    );
}

#[test]
fn author_validation_runs_full_customizer_contract() {
    let mut profile = author_profile_template("roman-bridge").expect("template");
    profile.customizer_profile.controls[0].section = Some("missing-section".to_owned());

    let report = validate_author_profile_package(&profile);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "unknown_control_section")
    );
    assert!(
        report.build_fingerprint.is_none(),
        "customizer contract failures should block compile proof"
    );
}
