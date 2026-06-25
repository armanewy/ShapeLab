#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use shape_decompiler::v3::bend::{BendParameters, evaluate_bend};
use shape_foundry_catalog::{
    FoundryAuthorPreviewCamera, FoundryAuthorProfilePackage, FoundryFixtureCatalog,
    built_in_foundry_kit_package, headless_fixture_catalogs,
};
use shape_inverse::{
    external_character::external_input_from_character_mesh_artifact,
    import_triage::{ImportTriageOutcome, ImportTriageReport},
};
use shape_render::foundry::FOUNDRY_DEFAULT_PREVIEW_CACHE_CAPACITY;
use shape_search::foundry::{
    FOUNDRY_MAX_PROPOSAL_COUNT, FOUNDRY_MAX_RESULT_COUNT, FOUNDRY_MIN_PROPOSAL_COUNT,
};

#[test]
fn demo_generates_headless_artifacts() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("demo");

    let status = Command::new(exe)
        .args([
            "demo",
            "--preset",
            "desk-lamp",
            "--seed",
            "7",
            "--mode",
            "explore",
            "--proposal-count",
            "12",
            "--result-count",
            "3",
            "--descriptor-resolution",
            "6",
            "--mesh-resolution",
            "12",
            "--out-dir",
        ])
        .arg(&out_dir)
        .status()
        .expect("run shape-cli demo");

    assert!(status.success());
    for name in [
        "project-before.json",
        "current.obj",
        "current.png",
        "candidate-00.obj",
        "candidate-00.png",
        "contact-sheet.png",
        "project-after.json",
        "accepted.obj",
        "accepted.png",
        "summary.json",
    ] {
        let path = out_dir.join(name);
        assert!(path.exists(), "{name} should exist");
        assert!(
            path.metadata().expect("metadata").len() > 0,
            "{name} is empty"
        );
    }

    let validate_status = Command::new(exe)
        .arg("validate")
        .arg(out_dir.join("project-after.json"))
        .status()
        .expect("run shape-cli validate");
    assert!(validate_status.success());
}

#[test]
fn release_readiness_reports_wave30_bounds_and_deferred_release_claims() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_path = temp_dir.path().join("release-readiness.json");

    let output = Command::new(exe)
        .args(["release-readiness", "--out"])
        .arg(&out_path)
        .output()
        .expect("run shape-cli release-readiness");

    assert!(output.status.success());
    assert!(out_path.exists());
    let stdout_report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be readiness JSON");
    let file_report: serde_json::Value =
        serde_json::from_slice(&fs::read(out_path).expect("read output report"))
            .expect("file should be readiness JSON");
    assert_eq!(stdout_report, file_report);

    assert_eq!(stdout_report["schema_version"].as_u64(), Some(5));
    assert_eq!(
        stdout_report["visual_product_gate"]["verification_status"],
        "not-run"
    );
    assert_eq!(
        stdout_report["visual_product_gate"]["verification_command"],
        "shape-cli release-readiness --verify-visual-gate"
    );
    assert_eq!(
        stdout_report["visual_product_gate"]["native_state_verification_status"],
        "requires-explicit-app-test"
    );
    assert_eq!(
        stdout_report["visual_product_gate"]["native_state_verification_command"],
        "cargo test -p shape-app release_gate_all_builtin_profiles_render_real_option_thumbnails -- --ignored"
    );
    assert_eq!(
        stdout_report["visual_product_gate"]["expected_built_in_profile_count"].as_u64(),
        Some(11)
    );
    assert_eq!(
        stdout_report["visual_product_gate"]["expected_primary_controls_per_profile"].as_u64(),
        Some(7)
    );
    assert_eq!(
        stdout_report["visual_product_gate"]["option_thumbnail_contract"],
        "computed-cli-64px-whole-model-thumbnails-plus-native-state-test"
    );
    assert_eq!(
        stdout_report["visual_product_gate"]["default_path_advanced_recipe_gate"],
        "verified-by-native-state-release-test"
    );
    assert!(stdout_report["visual_product_gate"]["evidence"].is_null());
    assert_eq!(
        stdout_report["product_ui_gate"]["verification_status"],
        "not-run"
    );
    assert_eq!(
        stdout_report["product_ui_gate"]["verification_command"],
        "shape-cli release-readiness --verify-product-ui-gate"
    );
    assert_eq!(
        stdout_report["product_ui_gate"]["app_shell"],
        "direct_visual_foundry"
    );
    assert_eq!(
        stdout_report["product_ui_gate"]["legacy_surfaces_present"],
        false
    );
    assert_eq!(
        stdout_report["product_ui_gate"]["product_home_profiles"].as_u64(),
        Some(0)
    );
    assert_eq!(
        stdout_report["product_ui_gate"]["installed_kit_count"].as_u64(),
        Some(11)
    );
    assert_eq!(
        stdout_report["product_ui_gate"]["developer_preview_kit_count"].as_u64(),
        Some(11)
    );
    assert_eq!(stdout_report["product_ui_gate"]["startup_blank"], false);
    assert_eq!(
        stdout_report["product_ui_gate"]["default_advanced_recipe_visible"],
        false
    );
    assert_eq!(
        stdout_report["product_ui_gate"]["manual_gate_required"],
        true
    );
    assert!(stdout_report["product_ui_gate"]["evidence"].is_null());
    assert_eq!(
        stdout_report["performance"]["preview_cache"]["bounded_lru_capacity"].as_u64(),
        Some(FOUNDRY_DEFAULT_PREVIEW_CACHE_CAPACITY as u64)
    );
    assert_eq!(
        stdout_report["performance"]["preview_cache"]["duplicate_miss_coalescing"],
        true
    );
    assert_eq!(
        stdout_report["performance"]["candidate_generation"]["minimum_proposal_count"].as_u64(),
        Some(FOUNDRY_MIN_PROPOSAL_COUNT as u64)
    );
    assert_eq!(
        stdout_report["performance"]["candidate_generation"]["maximum_proposal_count"].as_u64(),
        Some(FOUNDRY_MAX_PROPOSAL_COUNT as u64)
    );
    assert_eq!(
        stdout_report["performance"]["candidate_generation"]["maximum_returned_candidates"]
            .as_u64(),
        Some(FOUNDRY_MAX_RESULT_COUNT as u64)
    );
    assert_eq!(
        stdout_report["rendering"]["deterministic_cpu_reference"],
        "required-and-tested"
    );
    assert_eq!(
        stdout_report["rendering"]["gpu_required_for_release_checks"],
        false
    );
    assert_eq!(stdout_report["packaging"]["code_signing"], "not-configured");
    assert_eq!(
        stdout_report["window_regression"]["desktop_window_pixel_tests"],
        "not-configured"
    );
}

#[test]
#[ignore = "explicit Wave 30 release gate; computes all built-in profile option thumbnails"]
fn release_readiness_verifies_visual_product_gate_when_requested() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let output = Command::new(exe)
        .args(["release-readiness", "--verify-visual-gate"])
        .output()
        .expect("run shape-cli release-readiness --verify-visual-gate");

    assert!(
        output.status.success(),
        "release-readiness visual gate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be readiness JSON");
    assert_eq!(report["schema_version"].as_u64(), Some(5));
    assert_eq!(
        report["visual_product_gate"]["verification_status"],
        "verified"
    );
    assert_eq!(
        report["visual_product_gate"]["native_state_verification_status"],
        "requires-explicit-app-test"
    );
    let evidence = &report["visual_product_gate"]["evidence"];
    assert_eq!(evidence["built_in_profile_count"].as_u64(), Some(11));
    assert_eq!(evidence["profiles_checked"].as_u64(), Some(11));
    assert_eq!(evidence["all_profiles_verified"], true);
    assert_eq!(evidence["option_thumbnail_size_px"].as_u64(), Some(64));
    assert!(
        evidence["option_thumbnail_count"]
            .as_u64()
            .unwrap_or_default()
            > 0,
        "visual gate should render option thumbnails"
    );
    assert_eq!(evidence["profiles"].as_array().unwrap().len(), 11);
    assert!(
        evidence["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .all(|profile| {
                profile["primary_control_count"] == 7
                    && profile["option_control_count"].as_u64().unwrap_or_default() > 0
                    && profile["option_thumbnail_count"]
                        .as_u64()
                        .unwrap_or_default()
                        > 0
                    && profile["per_option_rgba_complete"] == true
                    && profile["per_option_camera_recorded"] == true
                    && profile["every_option_control_has_visual_delta"] == true
            })
    );
}

#[test]
fn release_readiness_verifies_product_ui_gate_when_requested() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let output = Command::new(exe)
        .args(["release-readiness", "--verify-product-ui-gate"])
        .output()
        .expect("run shape-cli release-readiness --verify-product-ui-gate");

    assert!(
        output.status.success(),
        "release-readiness product UI gate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be readiness JSON");
    assert_eq!(report["schema_version"].as_u64(), Some(5));
    let gate = &report["product_ui_gate"];
    assert_eq!(gate["verification_status"], "verified");
    assert_eq!(gate["app_shell"], "direct_visual_foundry");
    assert_eq!(gate["legacy_surfaces_present"], false);
    assert_eq!(gate["product_home_profiles"].as_u64(), Some(0));
    assert_eq!(gate["installed_kit_count"].as_u64(), Some(11));
    assert_eq!(gate["developer_preview_kit_count"].as_u64(), Some(11));
    assert_eq!(gate["startup_blank"], false);
    assert_eq!(gate["default_advanced_recipe_visible"], false);
    assert_eq!(gate["default_raw_technical_terms_visible"], false);
    assert_eq!(gate["directions_board_gate"], "pass");
    assert_eq!(gate["customize_deck_gate"], "pass");
    assert_eq!(gate["pack_gate"], "pass");
    assert_eq!(gate["export_gate"], "pass");
    assert_eq!(gate["disabled_states_have_reasons"], true);
    assert_eq!(gate["manual_gate_required"], true);
    let evidence = &gate["evidence"];
    assert_eq!(evidence["automated_gate_passed"], true);
    assert!(
        evidence["rendered_action_label_count"]
            .as_u64()
            .unwrap_or_default()
            > 0
    );
    assert_eq!(evidence["rendered_action_labels_audited"], true);
    assert_eq!(
        evidence["forbidden_terms_found"].as_array().unwrap().len(),
        0
    );
    assert_eq!(evidence["direction_candidate_slots"].as_u64(), Some(6));
    assert_eq!(
        evidence["direction_modes"],
        serde_json::json!(["Refine", "Explore", "Silhouette", "Structure", "Detail"])
    );
    assert_eq!(evidence["core_profiles"].as_array().unwrap().len(), 3);
    assert!(
        evidence["core_profiles"]
            .as_array()
            .unwrap()
            .iter()
            .all(|profile| {
                profile["compiled"] == true
                    && profile["reaches_main_shell"] == true
                    && profile["primary_control_count"]
                        .as_u64()
                        .unwrap_or_default()
                        > 0
                    && profile["primary_control_count"]
                        .as_u64()
                        .unwrap_or_default()
                        <= 7
                    && profile["triangle_count"].as_u64().unwrap_or_default() > 0
            })
    );
}

#[test]
fn model_demo_generates_explicit_asset_artifacts() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("crate");

    let status = Command::new(exe)
        .args(["model-demo", "--asset", "industrial-crate", "--out-dir"])
        .arg(&out_dir)
        .status()
        .expect("run shape-cli model-demo");

    assert!(status.success());
    for name in [
        "recipe.json",
        "asset.obj",
        "provenance.json",
        "validation.json",
        "statistics.json",
        "preview.png",
        "blender_reconstruct.py",
    ] {
        let path = out_dir.join(name);
        assert!(path.exists(), "{name} should exist");
        assert!(
            path.metadata().expect("metadata").len() > 0,
            "{name} is empty"
        );
    }

    let validation: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("validation.json")).unwrap())
            .unwrap();
    assert_eq!(validation["issues"].as_array().unwrap().len(), 0);
    let statistics: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("statistics.json")).unwrap())
            .unwrap();
    assert!(statistics["triangle_count"].as_u64().unwrap() < 25_000);
    assert_eq!(statistics["used_sdf_or_remeshing"], false);
}

#[test]
fn asset_visual_benchmark_writes_search_contact_sheets() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("visual");

    let status = Command::new(exe)
        .args([
            "asset-visual-benchmark",
            "--asset",
            "explicit-desk-lamp",
            "--seed",
            "9",
            "--proposal-count",
            "24",
            "--result-count",
            "2",
            "--out-dir",
        ])
        .arg(&out_dir)
        .status()
        .expect("run shape-cli asset-visual-benchmark");

    assert!(status.success());
    let asset_dir = out_dir.join("explicit-desk-lamp");
    for name in [
        "original.png",
        "original-wireframe.png",
        "accepted.png",
        "accepted-wireframe.png",
        "accepted.obj",
        "final-exported.png",
        "final-exported-wireframe.png",
        "visual-benchmark-summary.json",
        "refine/contact-sheet.png",
        "refine/contact-sheet-wireframe.png",
        "explore/contact-sheet.png",
        "explore/contact-sheet-wireframe.png",
        "final-package/asset-manifest.json",
        "final-package/blender_reconstruct.py",
    ] {
        let path = asset_dir.join(name);
        assert!(path.exists(), "{name} should exist");
        assert!(
            path.metadata().expect("metadata").len() > 0,
            "{name} is empty"
        );
    }

    let summary: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(asset_dir.join("visual-benchmark-summary.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(summary["refine_candidates"].as_array().unwrap().len(), 2);
    assert_eq!(summary["explore_candidates"].as_array().unwrap().len(), 2);
}

#[test]
fn inspect_and_compile_asset_use_canonical_model_package() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("lamp");

    let inspect = Command::new(exe)
        .args(["inspect-asset", "explicit-desk-lamp"])
        .output()
        .expect("run shape-cli inspect-asset");
    assert!(
        inspect.status.success(),
        "inspect failed: {}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(stdout.contains("Part tree:"));
    assert!(stdout.contains("Construction timeline:"));
    assert!(stdout.contains("model validation: valid"));
    assert!(stdout.contains("accidental intersections: 0"));

    let status = Command::new(exe)
        .args(["compile-asset", "explicit-desk-lamp", "--out-dir"])
        .arg(&out_dir)
        .status()
        .expect("run shape-cli compile-asset");
    assert!(status.success());
    for name in [
        "asset-manifest.json",
        "recipe.json",
        "provenance.json",
        "validation.json",
        "blender_reconstruct.py",
        "asset.obj",
        "grouped-obj-report.json",
        "statistics.json",
        "model-validation.json",
        "construction-timeline.json",
        "package-verification.json",
        "preview.png",
    ] {
        let path = out_dir.join(name);
        assert!(path.exists(), "{name} should exist");
        assert!(
            path.metadata().expect("metadata").len() > 0,
            "{name} is empty"
        );
    }
    assert!(out_dir.join("parts").join("part-001.meshbin").exists());

    let timeline: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(out_dir.join("construction-timeline.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(timeline["stages"].as_array().unwrap().len(), 8);
    let model_validation: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("model-validation.json")).unwrap())
            .unwrap();
    assert_eq!(
        model_validation["metrics"]["accidental_intersection_count"],
        0
    );
}

#[test]
fn foundry_build_generates_headless_outputs_for_fixtures() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    for fixture in headless_fixture_catalogs() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let catalog_dir = temp_dir.path().join("catalog");
        let out_dir = temp_dir.path().join("build");
        let document = write_foundry_fixture(&fixture, &catalog_dir);

        let output = Command::new(exe)
            .args(["foundry-build", "--catalog"])
            .arg(&catalog_dir)
            .args(["--document"])
            .arg(&document)
            .args(["--out-dir"])
            .arg(&out_dir)
            .output()
            .unwrap_or_else(|error| panic!("run shape-cli foundry-build: {error}"));

        assert!(
            output.status.success(),
            "{} failed: {}",
            fixture.slug,
            String::from_utf8_lossy(&output.stderr)
        );
        for name in [
            "foundry-document.json",
            "catalog-lock.json",
            "effective-request.json",
            "family-conformance.json",
            "recipe.json",
            "build-stamp.json",
            "local-overrides.json",
            "local-override-divergence.json",
            "control-divergence.json",
            "provider-overrides.json",
            "model-validation.json",
            "package-verification.json",
            "asset.obj",
            "preview.png",
            "model-package/asset-manifest.json",
            "model-package/blender_reconstruct.py",
        ] {
            let path = out_dir.join(name);
            assert!(path.exists(), "{} should write {name}", fixture.slug);
            assert!(
                path.metadata().expect("metadata").len() > 0,
                "{} wrote empty {name}",
                fixture.slug
            );
        }

        let conformance: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(out_dir.join("family-conformance.json")).unwrap(),
        )
        .unwrap();
        assert!(
            !conformance["roles"].as_array().unwrap().is_empty(),
            "{} should report role conformance",
            fixture.slug
        );
        assert!(
            conformance["issues"].as_array().unwrap().is_empty(),
            "{} should pass required conformance",
            fixture.slug
        );
        assert!(
            conformance["exports"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["profile"] == "canonical-model-package"
                    && row["status"] == "Passed"),
            "{} should pass export conformance",
            fixture.slug
        );
        let model_validation: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(out_dir.join("model-validation.json")).unwrap(),
        )
        .unwrap();
        assert!(
            model_validation["issues"]
                .as_array()
                .unwrap()
                .iter()
                .all(|issue| issue["severity"] != "Error"),
            "{} should pass model validation",
            fixture.slug
        );
        let verification: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(out_dir.join("package-verification.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(verification["checksums_match"], true);
        assert_eq!(verification["topology_matches_manifest"], true);
    }
}

#[test]
fn foundry_author_profile_cli_creates_validates_previews_and_packages() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let profile = temp_dir.path().join("roman-author-profile.json");
    let preview_dir = temp_dir.path().join("preview");
    let package_dir = temp_dir.path().join("package");

    let new_profile = Command::new(exe)
        .args(["foundry-new-profile", "--template", "roman-bridge", "--out"])
        .arg(&profile)
        .output()
        .expect("run shape-cli foundry-new-profile");
    assert!(
        new_profile.status.success(),
        "new profile failed: {}",
        String::from_utf8_lossy(&new_profile.stderr)
    );
    assert!(profile.exists());
    let mut profile_document: FoundryAuthorProfilePackage =
        serde_json::from_str(&fs::read_to_string(&profile).unwrap()).unwrap();
    profile_document.preview_cameras[0].width = 320;
    profile_document.preview_cameras[0].height = 180;
    profile_document
        .preview_cameras
        .push(FoundryAuthorPreviewCamera {
            id: "side".to_owned(),
            label: "Side".to_owned(),
            width: 128,
            height: 96,
            orbit_degrees: [90.0, 15.0, 0.0],
        });
    fs::write(
        &profile,
        serde_json::to_string_pretty(&profile_document).unwrap(),
    )
    .unwrap();

    let validate = Command::new(exe)
        .arg("foundry-validate-profile")
        .arg(&profile)
        .output()
        .expect("run shape-cli foundry-validate-profile");
    assert!(
        validate.status.success(),
        "validate failed: {}",
        String::from_utf8_lossy(&validate.stderr)
    );
    assert!(String::from_utf8_lossy(&validate.stdout).contains("status: valid"));

    let preview = Command::new(exe)
        .arg("foundry-preview-profile")
        .arg(&profile)
        .arg("--out-dir")
        .arg(&preview_dir)
        .output()
        .expect("run shape-cli foundry-preview-profile");
    assert!(
        preview.status.success(),
        "preview failed: {}",
        String::from_utf8_lossy(&preview.stderr)
    );
    for name in [
        "foundry-author-validation.json",
        "foundry-document.json",
        "family-conformance.json",
        "model-validation.json",
        "asset.obj",
        "preview.png",
        "preview-cameras.json",
        "previews/default.png",
        "previews/side.png",
    ] {
        let path = preview_dir.join(name);
        assert!(path.exists(), "preview should write {name}");
        assert!(
            path.metadata().unwrap().len() > 0,
            "{name} should not be empty"
        );
    }
    assert_eq!(
        image::image_dimensions(preview_dir.join("previews/default.png")).unwrap(),
        (320, 180)
    );
    assert_eq!(
        image::image_dimensions(preview_dir.join("previews/side.png")).unwrap(),
        (128, 96)
    );

    let package = Command::new(exe)
        .arg("foundry-package-profile")
        .arg(&profile)
        .arg("--out-dir")
        .arg(&package_dir)
        .output()
        .expect("run shape-cli foundry-package-profile");
    assert!(
        package.status.success(),
        "package failed: {}",
        String::from_utf8_lossy(&package.stderr)
    );
    for name in [
        "foundry-author-profile.json",
        "foundry-author-validation.json",
        "catalog/foundry-document.json",
        "catalog/catalog-manifest.json",
        "catalog/roman-bridge-family.json",
        "catalog/roman-bridge-style.json",
        "catalog/roman-bridge-family-impl.json",
        "catalog/roman-bridge-style-impl.json",
        "catalog/roman-bridge-profile.json",
        "build-proof/preview.png",
        "build-proof/previews/default.png",
        "build-proof/previews/side.png",
        "build-proof/model-validation.json",
    ] {
        let path = package_dir.join(name);
        assert!(path.exists(), "package should write {name}");
        assert!(
            path.metadata().unwrap().len() > 0,
            "{name} should not be empty"
        );
    }

    let second_profile = temp_dir.path().join("lamp-author-profile.json");
    let second_new_profile = Command::new(exe)
        .args([
            "foundry-new-profile",
            "--template",
            "stylized-lamp",
            "--out",
        ])
        .arg(&second_profile)
        .output()
        .expect("run shape-cli foundry-new-profile for lamp");
    assert!(
        second_new_profile.status.success(),
        "second new profile failed: {}",
        String::from_utf8_lossy(&second_new_profile.stderr)
    );
    let repackaged = Command::new(exe)
        .arg("foundry-package-profile")
        .arg(&second_profile)
        .arg("--out-dir")
        .arg(&package_dir)
        .output()
        .expect("rerun shape-cli foundry-package-profile");
    assert!(
        repackaged.status.success(),
        "repackage failed: {}",
        String::from_utf8_lossy(&repackaged.stderr)
    );
    assert!(
        !package_dir
            .join("catalog/roman-bridge-family.json")
            .exists(),
        "repackaging should remove stale catalog entries"
    );
    assert!(
        package_dir
            .join("catalog/stylized-lamp-family.json")
            .exists()
    );
}

#[test]
fn foundry_kit_cli_inspects_packages_and_rejects_incoherent_metadata() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let package_dir = temp_dir.path().join("kit-package");

    let inspect = Command::new(exe)
        .args(["foundry-kit", "inspect", "roman-bridge"])
        .output()
        .expect("run shape-cli foundry-kit inspect");
    assert!(
        inspect.status.success(),
        "inspect failed: {}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(stdout.contains("Foundry kit: Roman Timber Bridge"));
    assert!(stdout.contains("quality tier: Prototype"));
    assert!(stdout.contains("default catalog: hidden"));

    let package_status = Command::new(exe)
        .args(["foundry-kit", "package", "roman-bridge", "--out-dir"])
        .arg(&package_dir)
        .status()
        .expect("run shape-cli foundry-kit package");
    assert!(package_status.success());
    for name in [
        "foundry-kit-package.json",
        "kit-manifest.json",
        "family-blueprint.json",
        "provider-pack.json",
        "style-pack.json",
        "control-profile.json",
        "candidate-strategy-pack.json",
        "quality-gate-profile.json",
        "compatibility-matrix.json",
        "review-manifest.json",
        "kit-catalog-manifest.json",
        "build-proof/preview.png",
        "build-proof/model-package/asset-manifest.json",
    ] {
        let path = package_dir.join(name);
        assert!(path.exists(), "{name} should exist");
        assert!(
            path.metadata().expect("metadata").len() > 0,
            "{name} is empty"
        );
    }

    let validate_status = Command::new(exe)
        .args(["foundry-kit", "validate"])
        .arg(package_dir.join("foundry-kit-package.json"))
        .status()
        .expect("run shape-cli foundry-kit validate");
    assert!(validate_status.success());

    let mut invalid_package = built_in_foundry_kit_package("roman-bridge").expect("kit package");
    invalid_package.kit.provider_pack_id = "missing-provider-pack".to_owned();
    let invalid_path = temp_dir.path().join("invalid-kit.json");
    fs::write(
        &invalid_path,
        serde_json::to_string_pretty(&invalid_package).unwrap(),
    )
    .unwrap();
    let invalid = Command::new(exe)
        .args(["foundry-kit", "validate"])
        .arg(&invalid_path)
        .output()
        .expect("run shape-cli foundry-kit validate invalid");
    assert!(!invalid.status.success());
    assert!(
        String::from_utf8_lossy(&invalid.stderr).contains("missing_provider_pack_ref"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&invalid.stderr)
    );
}

#[test]
fn foundry_kit_cli_renders_contact_sheet_for_builtin() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("contact-sheet");

    let status = Command::new(exe)
        .args(["foundry-kit", "contact-sheet", "sci-fi-crate", "--out-dir"])
        .arg(&out_dir)
        .status()
        .expect("run shape-cli foundry-kit contact-sheet");
    assert!(status.success());
    for name in [
        "front.png",
        "three-quarter.png",
        "side.png",
        "back.png",
        "wireframe.png",
        "contact-sheet.png",
        "foundry-kit-package.json",
    ] {
        let path = out_dir.join(name);
        assert!(path.exists(), "{name} should exist");
        assert!(
            path.metadata().expect("metadata").len() > 0,
            "{name} is empty"
        );
    }
}

#[test]
fn foundry_kit_build_proof_rejects_forged_builtin_source_package() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let mut forged_package = built_in_foundry_kit_package("roman-bridge").expect("bridge kit");
    forged_package.kit.display_name = "Forged Bridge Kit".to_owned();
    let kit_path = temp_dir.path().join("forged-foundry-kit-package.json");
    fs::write(
        &kit_path,
        serde_json::to_string_pretty(&forged_package).unwrap(),
    )
    .unwrap();

    let validate = Command::new(exe)
        .args(["foundry-kit", "validate"])
        .arg(&kit_path)
        .status()
        .expect("run shape-cli foundry-kit validate forged package");
    assert!(
        validate.success(),
        "forged metadata remains internally valid"
    );

    let preview = Command::new(exe)
        .args(["foundry-kit", "preview"])
        .arg(&kit_path)
        .args(["--out-dir"])
        .arg(temp_dir.path().join("preview"))
        .output()
        .expect("run shape-cli foundry-kit preview forged package");
    assert!(!preview.status.success());
    assert!(
        String::from_utf8_lossy(&preview.stderr).contains("canonical built-in kit package"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&preview.stderr)
    );

    let mut forged_review_package =
        built_in_foundry_kit_package("roman-bridge").expect("bridge kit");
    forged_review_package.review_manifest.visual_review_notes =
        vec!["Forged review evidence.".to_owned()];
    let forged_review_path = temp_dir
        .path()
        .join("forged-review-foundry-kit-package.json");
    fs::write(
        &forged_review_path,
        serde_json::to_string_pretty(&forged_review_package).unwrap(),
    )
    .unwrap();
    let package = Command::new(exe)
        .args(["foundry-kit", "package"])
        .arg(&forged_review_path)
        .args(["--out-dir"])
        .arg(temp_dir.path().join("forged-review-package"))
        .output()
        .expect("run shape-cli foundry-kit package forged review package");
    assert!(!package.status.success());
    assert!(
        String::from_utf8_lossy(&package.stderr).contains("canonical built-in kit package"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&package.stderr)
    );
}

#[test]
fn foundry_kit_review_rejects_mismatched_quality_report_and_clears_stale_contact_sheet() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let mismatched_report = temp_dir.path().join("mismatched-quality-report.json");
    fs::write(
        &mismatched_report,
        serde_json::to_string_pretty(&serde_json::json!({
            "profile_id": "sci-fi-crate",
            "quality_tier_requested": "usable",
            "quality_tier_achieved": "usable",
            "human_approval_status": "approved",
            "adversarial_review_status": "pending",
            "contact_sheet_available": true,
            "quality_tier_blockers": []
        }))
        .unwrap(),
    )
    .unwrap();
    let mismatched_out = temp_dir.path().join("mismatched-review.json");
    let mismatched = Command::new(exe)
        .args(["foundry-kit", "review", "roman-bridge", "--quality-report"])
        .arg(&mismatched_report)
        .args(["--out"])
        .arg(&mismatched_out)
        .output()
        .expect("run mismatched foundry-kit review");
    assert!(!mismatched.status.success());
    assert!(
        String::from_utf8_lossy(&mismatched.stderr).contains("does not match kit source profile"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&mismatched.stderr)
    );

    let missing_contact_report = temp_dir.path().join("missing-contact-quality-report.json");
    fs::write(
        &missing_contact_report,
        serde_json::to_string_pretty(&serde_json::json!({
            "profile_id": "sci-fi-crate",
            "quality_tier_requested": "usable",
            "quality_tier_achieved": "usable",
            "human_approval_status": "approved",
            "adversarial_review_status": "pending",
            "contact_sheet_available": true,
            "quality_tier_blockers": []
        }))
        .unwrap(),
    )
    .unwrap();
    let missing_contact_out = temp_dir.path().join("missing-contact-review.json");
    let missing_contact = Command::new(exe)
        .args(["foundry-kit", "review", "sci-fi-crate", "--quality-report"])
        .arg(&missing_contact_report)
        .args(["--out"])
        .arg(&missing_contact_out)
        .output()
        .expect("run foundry-kit review without claimed contact sheet");
    assert!(!missing_contact.status.success());
    assert!(
        String::from_utf8_lossy(&missing_contact.stderr).contains("contact_sheet_available"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&missing_contact.stderr)
    );

    let crate_package = built_in_foundry_kit_package("sci-fi-crate").expect("crate kit");
    assert!(
        !crate_package.review_manifest.contact_sheet_paths.is_empty(),
        "fixture should start with automated contact sheet metadata"
    );
    let no_contact_report = temp_dir.path().join("no-contact-quality-report.json");
    fs::write(
        &no_contact_report,
        serde_json::to_string_pretty(&serde_json::json!({
            "profile_id": "sci-fi-crate",
            "kit_id": crate_package.style_pack.style_id,
            "style_id": crate_package.style_pack.style_id,
            "quality_tier_requested": "usable",
            "quality_tier_achieved": "prototype",
            "human_approval_status": "pending",
            "adversarial_review_status": "pending",
            "contact_sheet_available": false,
            "quality_tier_blockers": ["Contact sheet missing"]
        }))
        .unwrap(),
    )
    .unwrap();
    let review_out = temp_dir.path().join("review-no-contact.json");
    let review = Command::new(exe)
        .args(["foundry-kit", "review", "sci-fi-crate", "--quality-report"])
        .arg(&no_contact_report)
        .args(["--out"])
        .arg(&review_out)
        .output()
        .expect("run foundry-kit review without contact sheet");
    assert!(
        review.status.success(),
        "review failed: {}",
        String::from_utf8_lossy(&review.stderr)
    );
    let review_manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&review_out).unwrap()).unwrap();
    assert!(
        review_manifest["contact_sheet_paths"]
            .as_array()
            .unwrap()
            .is_empty(),
        "stale contact-sheet paths should be cleared"
    );
}

#[test]
fn foundry_build_rejects_model_validation_errors() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let mut fixture = shape_foundry_catalog::scifi_crate::fixture_catalog();
    let base = shape_foundry::compile_foundry_document(&fixture.document, &fixture)
        .expect("base crate should compile");
    let panel = base
        .recipe
        .instances
        .values()
        .find(|instance| {
            base.recipe
                .definitions
                .get(&instance.definition)
                .is_some_and(|definition| definition.tags.contains("panel"))
        })
        .expect("panel instance should survive remapping")
        .clone();
    let duplicate_panel = shape_asset::PartInstance {
        id: shape_asset::PartInstanceId(base.recipe.next_ids.part_instance),
        name: "overlapping duplicate panel".to_owned(),
        generated_by: None,
        ..panel.clone()
    };
    fixture
        .document
        .local_recipe_overrides
        .push(shape_foundry::LocalRecipeOverride {
            id: shape_foundry::LocalRecipeOverrideId("duplicate-panel-overlap".to_owned()),
            base_geometry_fingerprint: base.base_geometry_fingerprint,
            edit_program: shape_asset::AssetEditProgram {
                label: "duplicate panel overlap".to_owned(),
                seed: 23,
                operations: vec![shape_asset::AssetEdit::AddInstance {
                    instance: duplicate_panel.clone(),
                }],
            },
            touched_targets: vec![shape_foundry::TouchedSemanticTarget::PartInstance(
                duplicate_panel.id,
            )],
            survival_policy: shape_foundry::OverrideSurvivalPolicy::Revalidate,
        });
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let catalog_dir = temp_dir.path().join("catalog");
    let out_dir = temp_dir.path().join("build");
    let document = write_foundry_fixture(&fixture, &catalog_dir);

    let output = Command::new(exe)
        .args(["foundry-build", "--catalog"])
        .arg(&catalog_dir)
        .args(["--document"])
        .arg(&document)
        .args(["--out-dir"])
        .arg(&out_dir)
        .output()
        .expect("run shape-cli foundry-build");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("model validation failed"),
        "model validation failure should be explicit, stderr was: {stderr}"
    );
    let model_validation: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("model-validation.json")).unwrap())
            .unwrap();
    assert!(
        model_validation["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["severity"] == "Error"),
        "model-validation.json should contain error severity diagnostics"
    );
}

#[test]
fn foundry_build_reports_local_override_divergence() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let mut fixture = shape_foundry_catalog::scifi_crate::fixture_catalog();
    let base = shape_foundry::compile_foundry_document(&fixture.document, &fixture)
        .expect("base crate should compile");
    let parameter = base
        .recipe
        .parameters
        .values()
        .find(|parameter| parameter.path.contains("rounded_box.half_extents.x"))
        .expect("body width parameter should survive remapping")
        .id;
    fixture
        .document
        .local_recipe_overrides
        .push(shape_foundry::LocalRecipeOverride {
            id: shape_foundry::LocalRecipeOverrideId("widen-body".to_owned()),
            base_geometry_fingerprint: base.base_geometry_fingerprint,
            edit_program: shape_asset::AssetEditProgram {
                label: "widen body".to_owned(),
                seed: 11,
                operations: vec![shape_asset::AssetEdit::SetScalar {
                    parameter,
                    value: 1.45,
                }],
            },
            touched_targets: vec![shape_foundry::TouchedSemanticTarget::Parameter(parameter)],
            survival_policy: shape_foundry::OverrideSurvivalPolicy::Revalidate,
        });
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let catalog_dir = temp_dir.path().join("catalog");
    let out_dir = temp_dir.path().join("build");
    let document = write_foundry_fixture(&fixture, &catalog_dir);

    let output = Command::new(exe)
        .args(["foundry-build", "--catalog"])
        .arg(&catalog_dir)
        .args(["--document"])
        .arg(&document)
        .args(["--out-dir"])
        .arg(&out_dir)
        .output()
        .expect("run shape-cli foundry-build");
    assert!(
        output.status.success(),
        "foundry build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let control_divergence: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("control-divergence.json")).unwrap())
            .unwrap();
    assert_eq!(control_divergence["body_proportions"], "DivergedByOverride");
    let effective_request: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("effective-request.json")).unwrap())
            .unwrap();
    assert_eq!(
        effective_request["parameters"]["body_proportions"]["Scalar"],
        0.45
    );
    let override_divergence: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(out_dir.join("local-override-divergence.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        override_divergence[0]["diverged_controls"][0]["control_id"],
        "body_proportions"
    );
    assert_eq!(
        override_divergence[0]["diverged_controls"][0]["slots"],
        serde_json::json!(["body_proportions"])
    );
}

#[test]
fn foundry_build_replays_deterministically_and_reports_catalog_mismatch() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let fixture = shape_foundry_catalog::scifi_crate::fixture_catalog();
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let catalog_dir = temp_dir.path().join("catalog");
    let document = write_foundry_fixture(&fixture, &catalog_dir);
    let first_out = temp_dir.path().join("first");
    let second_out = temp_dir.path().join("second");

    for out_dir in [&first_out, &second_out] {
        let output = Command::new(exe)
            .args(["foundry-build", "--catalog"])
            .arg(&catalog_dir)
            .args(["--document"])
            .arg(&document)
            .args(["--out-dir"])
            .arg(out_dir)
            .output()
            .expect("run shape-cli foundry-build");
        assert!(
            output.status.success(),
            "foundry build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let first_files = relative_files(&first_out);
    let second_files = relative_files(&second_out);
    assert_eq!(first_files, second_files);
    for name in first_files {
        assert_eq!(
            fs::read(first_out.join(&name)).unwrap(),
            fs::read(second_out.join(&name)).unwrap(),
            "{} should replay deterministically",
            name.display()
        );
    }

    let bad_family_path = catalog_dir.join(format!(
        "{}.json",
        fixture.document.family_content_ref.stable_id
    ));
    fs::write(&bad_family_path, "{}").expect("corrupt family catalog entry");
    let mismatch = Command::new(exe)
        .args(["foundry-build", "--catalog"])
        .arg(&catalog_dir)
        .args(["--document"])
        .arg(&document)
        .args(["--out-dir"])
        .arg(temp_dir.path().join("mismatch"))
        .output()
        .expect("run shape-cli foundry-build with mismatch");
    assert!(!mismatch.status.success());
    let stderr = String::from_utf8_lossy(&mismatch.stderr);
    assert!(
        stderr.contains("FingerprintMismatch"),
        "catalog mismatch should be explicit, stderr was: {stderr}"
    );
}

#[test]
fn hq_quality_benchmark_emits_honest_quality_report() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("hq-quality");

    let output = Command::new(exe)
        .args([
            "hq-quality-benchmark",
            "--profile",
            "roman-bridge",
            "--out-dir",
        ])
        .arg(&out_dir)
        .arg("--json")
        .output()
        .expect("run shape-cli hq-quality-benchmark");
    assert!(
        output.status.success(),
        "hq quality benchmark failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for name in [
        "contact-sheet.png",
        "front.png",
        "three-quarter.png",
        "side.png",
        "back.png",
        "wireframe.png",
        "silhouette.png",
        "mesh-stats.json",
        "semantic-parts.json",
        "candidate-report.json",
        "controls-visibility-report.json",
        "export-reopen-report.json",
        "quality-report.json",
    ] {
        let path = out_dir.join(name);
        assert!(path.exists(), "HQ benchmark should write {name}");
        assert!(
            path.metadata().expect("metadata").len() > 0,
            "HQ benchmark {name} is empty"
        );
    }

    let stdout_report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout quality report");
    let file_report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("quality-report.json")).unwrap())
            .unwrap();
    assert_eq!(stdout_report, file_report);
    assert_eq!(file_report["schema_version"], 1);
    assert_eq!(file_report["profile_id"], "roman-bridge");
    assert_eq!(file_report["quality_tier_requested"], "usable");
    assert_eq!(file_report["quality_tier_achieved"], "prototype");
    assert_eq!(file_report["export_status"], "not_run");
    assert_eq!(file_report["reopen_status"], "not_run");
    assert_eq!(file_report["six_direction_availability"], false);
    assert!(
        file_report["quality_tier_blockers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|blocker| blocker
                .as_str()
                .unwrap()
                .contains("six surviving direction candidates"))
    );
    assert_eq!(file_report["placeholder_thumbnail_detected"], false);
    assert_eq!(
        file_report["novice_catalog_exposure_allowed_by_default"],
        false
    );
    assert_eq!(
        file_report["visible_control_difference_evidence"]["changed_control_count"],
        file_report["primary_control_count"]
    );
    assert!(
        file_report["visible_control_difference_evidence"]["controls"]
            .as_array()
            .unwrap()
            .iter()
            .all(|control| control["visual_delta_from_parent"].as_u64().unwrap() > 0),
        "every changed control should include rendered image delta evidence"
    );
    assert!(
        file_report["unsupported_outputs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["output"] == "photoreal_render")
    );

    let export_report: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(out_dir.join("export-reopen-report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(export_report["export_status"], "not_run");
    assert!(
        export_report["not_run_reason"]
            .as_str()
            .unwrap()
            .contains("--verify-export")
    );
}

#[test]
fn foundry_visual_benchmark_generates_usability_gate_artifacts() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("visual-benchmark");

    let metrics = run_foundry_visual_benchmark(exe, "roman-bridge", &out_dir);
    assert_eq!(metrics["profile"], "roman-bridge");
    assert!(
        metrics["primary_controls"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|control| control["control_kind"] == "continuous_axis")
            .all(|control| control["sample_count"] == 5),
        "continuous controls should render five samples"
    );

    let refine: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("refine/candidates.json")).unwrap())
            .unwrap();
    let candidates = refine["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 6);
    assert!(
        candidates
            .iter()
            .all(|candidate| !candidate["explanations"].as_array().unwrap().is_empty()),
        "every candidate should carry a structured explanation"
    );
}

fn run_foundry_visual_benchmark(exe: &str, profile: &str, out_dir: &Path) -> serde_json::Value {
    let output = Command::new(exe)
        .args([
            "foundry-visual-benchmark",
            "--profile",
            profile,
            "--proposal-count",
            "24",
            "--skip-blender",
            "--out-dir",
        ])
        .arg(out_dir)
        .output()
        .expect("run shape-cli foundry-visual-benchmark");
    assert!(
        output.status.success(),
        "{profile} foundry visual benchmark failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for name in [
        "source-document.json",
        "catalog-lock.json",
        "customizer-profile.json",
        "conformance.json",
        "validation.json",
        "metrics.json",
        "parent/preview.png",
        "parent/preview-wireframe.png",
        "parent/blender-verification.json",
        "refine/contact-sheet.png",
        "refine/candidates.json",
        "explore/contact-sheet.png",
        "explore/candidates.json",
        "silhouette/candidates.json",
        "structure/candidates.json",
        "detail/candidates.json",
        "control-strips/summary.json",
        "option-galleries/summary.json",
        "coherent-pack/pack-document.json",
        "coherent-pack/pack-report.json",
    ] {
        let path = out_dir.join(name);
        assert!(path.exists(), "{profile} benchmark should write {name}");
        assert!(
            path.metadata().expect("metadata").len() > 0,
            "{profile} {name} is empty"
        );
    }

    let metrics: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("metrics.json")).unwrap()).unwrap();
    assert_eq!(metrics["advanced_recipe_required"], false);
    assert_eq!(metrics["all_primary_controls_measurable"], true);
    assert_eq!(metrics["invalid_state_became_current"], false);
    assert_eq!(
        metrics["provider_options_rendered"],
        metrics["provider_options_total"]
    );
    assert_eq!(metrics["coherent_pack"]["member_count"], 3);

    let modes = metrics["candidate_modes"].as_array().unwrap();
    for mode in ["refine", "explore"] {
        let summary = modes
            .iter()
            .find(|summary| summary["mode"] == mode)
            .expect("candidate mode summary");
        assert_eq!(
            summary["returned_count"], 6,
            "{profile} {mode} should return six candidates"
        );
    }
    metrics
}

fn write_foundry_fixture(fixture: &FoundryFixtureCatalog, catalog_dir: &Path) -> PathBuf {
    fixture
        .write_to_dir(catalog_dir)
        .unwrap_or_else(|error| panic!("write foundry fixture {}: {error}", fixture.slug));
    catalog_dir.join("foundry-document.json")
}

fn relative_files(root: &Path) -> Vec<PathBuf> {
    fn visit(root: &Path, current: &Path, files: &mut Vec<PathBuf>) {
        let mut entries = fs::read_dir(current)
            .unwrap_or_else(|error| panic!("read {}: {error}", current.display()))
            .map(|entry| entry.expect("directory entry").path())
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.push(
                    path.strip_prefix(root)
                        .expect("relative path")
                        .to_path_buf(),
                );
            }
        }
    }

    let mut files = Vec::new();
    visit(root, root, &mut files);
    files
}

#[test]
fn decompile_generates_lossless_package() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let source = temp_dir.path().join("source.obj");
    let target = temp_dir.path().join("target.obj");
    let package = temp_dir.path().join("package");
    fs::write(
        &source,
        "\
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
v 0.5 0.5 1
f 1 2 3
f 1 3 4
f 1 2 5
f 2 3 5
f 3 4 5
f 4 1 5
",
    )
    .expect("write source obj");
    fs::write(
        &target,
        "\
v 0.25 0 0
v 1.25 0 0
v 1.25 1 0
v 0.25 1 0
v 0.5 0.5 1.25
f 1 2 3
f 1 3 4
f 1 2 5
f 2 3 5
f 3 4 5
f 4 1 5
",
    )
    .expect("write target obj");

    let status = Command::new(exe)
        .arg("decompile")
        .arg(&source)
        .arg(&target)
        .arg("--out-dir")
        .arg(&package)
        .status()
        .expect("run shape-cli decompile");

    assert!(status.success());
    for name in [
        "manifest.json",
        "verification.json",
        "package-verification.json",
        "inference-diagnostics.json",
        "source.meshbin",
        "target.meshbin",
        "blender_reconstruct.py",
    ] {
        let path = package.join(name);
        assert!(path.exists(), "{name} should exist");
        assert!(
            path.metadata().expect("metadata").len() > 0,
            "{name} is empty"
        );
    }
    assert!(package.join("residual").join("indices.u32").exists());
    assert!(package.join("residual").join("positions.f32").exists());

    let diagnostics: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(package.join("inference-diagnostics.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(diagnostics["diagnostics_schema_version"], 3);
    assert!(diagnostics["program_hypotheses"].is_array());
    assert!(diagnostics.get("hypotheses").is_none());
    assert!(diagnostics.get("selected_hypothesis_index").is_none());

    let verification: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(package.join("verification.json")).unwrap())
            .unwrap();
    assert_eq!(verification["topology_exact"], true);
    assert_eq!(verification["max_euclidean_error"], 0.0);

    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(package.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["schema_version"], 2);
    assert_eq!(
        manifest["numeric_format"]["affine_evaluation"],
        "float32_stepwise_no_fma"
    );
    let operators = manifest["operators"].as_array().unwrap();
    assert_eq!(operators.len(), 2);
    for operator in operators {
        if let Some(baked_positions_file) = operator["baked_positions_file"].as_str() {
            let path = package.join(baked_positions_file);
            assert!(path.exists(), "{baked_positions_file} should exist");
            assert!(
                path.metadata().expect("metadata").len() > 0,
                "{baked_positions_file} is empty"
            );
        }
    }
    let lossless_stage = package
        .join("operators")
        .join("0001-lossless-correction-positions.f32");
    assert!(lossless_stage.exists());
    assert!(lossless_stage.metadata().expect("metadata").len() > 0);

    let verify_status = Command::new(exe)
        .arg("verify-decompile")
        .arg(&package)
        .status()
        .expect("run shape-cli verify-decompile");
    assert!(verify_status.success());
}

#[test]
fn decompile_generates_schema_three_residual_only_package() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let source = temp_dir.path().join("source.obj");
    let target = temp_dir.path().join("target.obj");
    let package = temp_dir.path().join("package-v3");
    let source_obj = "\
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
f 1 2 3
f 1 3 4
";
    fs::write(&source, source_obj).expect("write source obj");
    fs::write(&target, source_obj).expect("write target obj");

    let status = Command::new(exe)
        .arg("decompile")
        .arg(&source)
        .arg(&target)
        .arg("--package-schema")
        .arg("3")
        .arg("--out-dir")
        .arg(&package)
        .status()
        .expect("run shape-cli schema-3 decompile");

    assert!(status.success());
    for name in [
        "manifest.json",
        "package-verification.json",
        "inference-diagnostics.json",
        "source.meshbin",
        "target.meshbin",
        "blender_reconstruct.py",
    ] {
        let path = package.join(name);
        assert!(path.exists(), "{name} should exist");
        assert!(
            path.metadata().expect("metadata").len() > 0,
            "{name} is empty"
        );
    }

    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(package.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["schema_version"], 3);
    let operators = manifest["operators"].as_array().unwrap();
    assert_eq!(operators.len(), 1);
    assert_eq!(operators[0]["kind"], "lossless_correction");
    assert_eq!(
        operators[0]["stage"]["baked_positions_file"],
        "operators/0000-lossless-correction-positions.f32"
    );

    let diagnostics: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(package.join("inference-diagnostics.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(diagnostics["diagnostics_schema_version"], 4);
    assert_eq!(diagnostics["package_schema_version"], 3);
    assert!(
        diagnostics["program_hypotheses"][0]["operators"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let verify_status = Command::new(exe)
        .arg("verify-decompile")
        .arg(&package)
        .status()
        .expect("run shape-cli verify-decompile for schema 3");
    assert!(verify_status.success());
}

#[test]
fn enable_bend_requires_schema_three() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let source = temp_dir.path().join("source.obj");
    let target = temp_dir.path().join("target.obj");
    let obj = "\
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
";
    fs::write(&source, obj).expect("write source obj");
    fs::write(&target, obj).expect("write target obj");

    let status = Command::new(exe)
        .arg("decompile")
        .arg(&source)
        .arg(&target)
        .arg("--enable-bend")
        .arg("--out-dir")
        .arg(temp_dir.path().join("package"))
        .status()
        .expect("run shape-cli invalid bend decompile");

    assert!(!status.success());
}

#[test]
fn import_triage_character_cli_writes_truthful_reports() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let descriptor = temp_dir.path().join("external-character.json");
    let out_dir = temp_dir.path().join("triage");
    let corpus = shape_character::corpus::generated_character_corpus(2401);
    let exact_input =
        external_input_from_character_mesh_artifact("cli.triage.exact", &corpus.cases[4].mesh);
    fs::write(
        &descriptor,
        serde_json::to_string_pretty(&exact_input).unwrap(),
    )
    .unwrap();

    let exact = Command::new(exe)
        .arg("import-triage-character")
        .arg(&descriptor)
        .arg("--out-dir")
        .arg(&out_dir)
        .output()
        .expect("run shape-cli import-triage-character");
    assert!(
        exact.status.success(),
        "exact triage failed: {}",
        String::from_utf8_lossy(&exact.stderr)
    );
    for name in [
        "import-triage-report.json",
        "external-character-analysis.json",
        "strict-known-base-recovery.json",
    ] {
        let path = out_dir.join(name);
        assert!(path.exists(), "triage should write {name}");
        assert!(
            path.metadata().unwrap().len() > 0,
            "{name} should not be empty"
        );
    }
    let exact_report: ImportTriageReport = serde_json::from_str(
        &fs::read_to_string(out_dir.join("import-triage-report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        exact_report.outcome,
        ImportTriageOutcome::ExactEditableRecovery
    );
    assert!(exact_report.strict_recovery_proven);
    assert_eq!(
        exact_report.user_facing_label,
        "Recover exact editable program"
    );

    let mut partial_input =
        external_input_from_character_mesh_artifact("cli.triage.partial", &corpus.cases[2].mesh);
    partial_input.topology_fingerprint = None;
    partial_input.canonical_position_fingerprint = None;
    partial_input.semantic_descriptor_fingerprint = None;
    fs::write(
        &descriptor,
        serde_json::to_string_pretty(&partial_input).unwrap(),
    )
    .unwrap();

    let partial = Command::new(exe)
        .arg("import-triage-character")
        .arg(&descriptor)
        .arg("--out-dir")
        .arg(&out_dir)
        .output()
        .expect("rerun shape-cli import-triage-character");
    assert!(
        partial.status.success(),
        "partial triage failed: {}",
        String::from_utf8_lossy(&partial.stderr)
    );
    let partial_report: ImportTriageReport = serde_json::from_str(
        &fs::read_to_string(out_dir.join("import-triage-report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        partial_report.outcome,
        ImportTriageOutcome::KnownBasePartialDiagnostic
    );
    assert!(!partial_report.strict_recovery_proven);
    assert_eq!(
        partial_report.user_facing_label,
        "Known-base partial diagnostic"
    );
    let strict_report: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(out_dir.join("strict-known-base-recovery.json")).unwrap(),
    )
    .unwrap();
    assert!(strict_report.is_null());

    let mut hidden_field = serde_json::to_value(&partial_input).unwrap();
    hidden_field.as_object_mut().unwrap().insert(
        "source_program".to_owned(),
        serde_json::json!({"program_id": "private.answer.key"}),
    );
    fs::write(
        &descriptor,
        serde_json::to_string_pretty(&hidden_field).unwrap(),
    )
    .unwrap();
    let hidden = Command::new(exe)
        .arg("import-triage-character")
        .arg(&descriptor)
        .arg("--out-dir")
        .arg(&out_dir)
        .output()
        .expect("rerun shape-cli import-triage-character with hidden field");
    assert!(
        hidden.status.success(),
        "hidden-field triage should write invalid report: {}",
        String::from_utf8_lossy(&hidden.stderr)
    );
    assert!(
        String::from_utf8_lossy(&hidden.stdout).contains("outcome: invalid_input"),
        "stdout should use snake_case outcome"
    );
    assert!(
        String::from_utf8_lossy(&hidden.stderr).is_empty(),
        "diagnostic triage should not write normal reasons to stderr"
    );
    let hidden_report: ImportTriageReport = serde_json::from_str(
        &fs::read_to_string(out_dir.join("import-triage-report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(hidden_report.outcome, ImportTriageOutcome::InvalidInput);
    assert!(!hidden_report.strict_recovery_proven);
    assert!(hidden_report.strict_known_base_recovery.is_none());
    let hidden_strict_report: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(out_dir.join("strict-known-base-recovery.json")).unwrap(),
    )
    .unwrap();
    assert!(hidden_strict_report.is_null());
}

#[test]
fn decompile_schema_three_enable_bend_writes_bend_program() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let source = temp_dir.path().join("source.obj");
    let target = temp_dir.path().join("target.obj");
    let package = temp_dir.path().join("bend-package");
    let (source_obj, target_obj) = bend_pair_objs();
    fs::write(&source, source_obj).expect("write bend source obj");
    fs::write(&target, target_obj).expect("write bend target obj");

    let status = Command::new(exe)
        .arg("decompile")
        .arg(&source)
        .arg(&target)
        .arg("--package-schema")
        .arg("3")
        .arg("--enable-bend")
        .arg("--verbose")
        .arg("--out-dir")
        .arg(&package)
        .status()
        .expect("run shape-cli bend decompile");

    assert!(status.success());
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(package.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["schema_version"], 3);
    assert!(
        manifest["operators"]
            .as_array()
            .unwrap()
            .iter()
            .any(|operator| operator["kind"] == "bend")
    );

    let diagnostics: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(package.join("inference-diagnostics.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(diagnostics["diagnostics_schema_version"], 4);
    assert!(diagnostics["program_hypotheses"].as_array().unwrap().len() > 1);
    assert!(diagnostics["timing_by_phase_ms"]["program_scoring_ms"].is_number());
    let selected = diagnostics["selected_program_hypothesis_index"]
        .as_u64()
        .unwrap() as usize;
    let selected_hypothesis = &diagnostics["program_hypotheses"].as_array().unwrap()[selected];
    assert!(
        selected_hypothesis["operators"]
            .as_array()
            .unwrap()
            .iter()
            .any(|operator| operator["family"] == "bend")
    );

    let verify_status = Command::new(exe)
        .arg("verify-decompile")
        .arg(&package)
        .status()
        .expect("run shape-cli verify-decompile for bend package");
    assert!(verify_status.success());
}

fn bend_pair_objs() -> (String, String) {
    let mut source_positions = Vec::new();
    for index in 0..=10 {
        let x = index as f32 / 10.0;
        source_positions.extend([
            [x, -0.10, -0.05],
            [x, 0.10, -0.05],
            [x, 0.10, 0.05],
            [x, -0.10, 0.05],
        ]);
    }
    let target_positions = evaluate_bend(
        &BendParameters {
            origin: [0.5, 0.0, 0.0],
            longitudinal_axis: [1.0, 0.0, 0.0],
            bend_direction: [0.0, 1.0, 0.0],
            angle_radians: 45.0_f32.to_radians(),
            interval_start: -0.5,
            interval_end: 0.5,
        },
        &source_positions,
    )
    .unwrap();
    let faces = beam_faces(11);
    (
        obj_from_positions(&source_positions, &faces),
        obj_from_positions(&target_positions, &faces),
    )
}

fn beam_faces(station_count: usize) -> Vec<[usize; 3]> {
    let mut faces = Vec::new();
    for ring in 0..station_count - 1 {
        let current = ring * 4 + 1;
        let next = current + 4;
        for corner in 0..4 {
            let a = current + corner;
            let b = current + (corner + 1) % 4;
            let c = next + (corner + 1) % 4;
            let d = next + corner;
            faces.push([a, b, c]);
            faces.push([a, c, d]);
        }
    }
    let last = (station_count - 1) * 4 + 1;
    faces.push([1, 2, 3]);
    faces.push([1, 3, 4]);
    faces.push([last, last + 2, last + 1]);
    faces.push([last, last + 3, last + 2]);
    faces
}

fn obj_from_positions(positions: &[[f32; 3]], faces: &[[usize; 3]]) -> String {
    let mut obj = String::new();
    for position in positions {
        obj.push_str(&format!(
            "v {:.9} {:.9} {:.9}\n",
            position[0], position[1], position[2]
        ));
    }
    for [a, b, c] in faces {
        obj.push_str(&format!("f {a} {b} {c}\n"));
    }
    obj
}
