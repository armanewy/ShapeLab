#![forbid(unsafe_code)]

use std::fs;
use std::process::Command;

use orchard_foundry::{
    DirectKitCreatedFrom, DirectKitDraft, DirectKitEvidenceKind, DirectKitEvidenceRef,
    DirectKitEvidenceStatus, DirectKitSourceKind, DirectKitVisibility, ObjectPlanReviewTier,
    PrimitiveKind, direct_kit_property_exposures_for_primitive,
};

#[test]
fn personal_kit_cli_saves_lists_and_validates_draft() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let kit_path = temp_dir.path().join("direct-kit.json");
    let store = temp_dir.path().join("store");
    write_kit(&kit_path, &valid_kit(DirectKitVisibility::Draft));

    let save = Command::new(exe)
        .args(["personal-kit", "save", "--kit"])
        .arg(&kit_path)
        .args(["--out-dir"])
        .arg(&store)
        .output()
        .expect("run personal-kit save");

    assert!(save.status.success(), "stderr: {}", stderr(&save));
    assert!(store.join("personal-kits/manifest.json").exists());
    assert!(store.join("personal-kit-save-report.json").exists());
    let saved = read_json(store.join("personal-kits/kits/box_primitive_kit/kit.json"));
    assert_eq!(saved["novice_visible"], false);
    assert_eq!(saved["public_catalog_visible"], false);

    let list = Command::new(exe)
        .args(["personal-kit", "list", "--store"])
        .arg(&store)
        .output()
        .expect("run personal-kit list");
    assert!(list.status.success(), "stderr: {}", stderr(&list));
    let manifest: serde_json::Value =
        serde_json::from_slice(&list.stdout).expect("parse list stdout");
    assert_eq!(manifest["kits"][0]["kit_id"], "box_primitive_kit");
    assert_eq!(
        manifest["kits"][0]["kit_path"],
        "kits/box_primitive_kit/kit.json"
    );

    let validate = Command::new(exe)
        .args(["personal-kit", "validate", "--store"])
        .arg(&store)
        .output()
        .expect("run personal-kit validate");
    assert!(validate.status.success(), "stderr: {}", stderr(&validate));
    let report: serde_json::Value =
        serde_json::from_slice(&validate.stdout).expect("parse validate stdout");
    assert_eq!(report["errors"].as_array().expect("errors").len(), 0);
}

#[test]
fn personal_kit_cli_saves_personal_only_without_public_visibility() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let kit_path = temp_dir.path().join("direct-kit.json");
    let store = temp_dir.path().join("store");
    write_kit(&kit_path, &valid_kit(DirectKitVisibility::PersonalOnly));

    let output = Command::new(exe)
        .args(["personal-kit", "save", "--kit"])
        .arg(&kit_path)
        .args(["--out-dir"])
        .arg(&store)
        .output()
        .expect("run personal-kit save");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let saved = fs::read_to_string(store.join("personal-kits/kits/box_primitive_kit/kit.json"))
        .expect("read saved kit");
    assert!(saved.contains("\"visibility\": \"PersonalOnly\""));
    assert!(!saved.contains("\"public_catalog_visible\": true"));
    assert!(!saved.contains(temp_dir.path().to_string_lossy().as_ref()));
}

#[test]
fn personal_kit_cli_rejects_public_visibility() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let kit_path = temp_dir.path().join("direct-kit.json");
    let store = temp_dir.path().join("store");
    write_kit(&kit_path, &valid_kit(DirectKitVisibility::PublicCatalog));

    let output = Command::new(exe)
        .args(["personal-kit", "save", "--kit"])
        .arg(&kit_path)
        .args(["--out-dir"])
        .arg(&store)
        .output()
        .expect("run personal-kit save");

    assert!(!output.status.success());
    assert!(stderr(&output).contains("validation failed"));
}

#[test]
fn personal_kit_cli_manifest_is_deterministic() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let first_store = temp_dir.path().join("first");
    let second_store = temp_dir.path().join("second");
    let kit_path = temp_dir.path().join("direct-kit.json");
    write_kit(&kit_path, &valid_kit(DirectKitVisibility::Draft));

    for store in [&first_store, &second_store] {
        assert!(
            Command::new(exe)
                .args(["personal-kit", "save", "--kit"])
                .arg(&kit_path)
                .args(["--out-dir"])
                .arg(store)
                .status()
                .expect("run personal-kit save")
                .success()
        );
    }

    let first = fs::read(first_store.join("personal-kits/manifest.json")).expect("read first");
    let second = fs::read(second_store.join("personal-kits/manifest.json")).expect("read second");
    assert_eq!(first, second);
}

fn valid_kit(visibility: DirectKitVisibility) -> DirectKitDraft {
    let mut exposures = direct_kit_property_exposures_for_primitive(PrimitiveKind::BoxPrimitive);
    let locked_properties = exposures.split_off(1);
    DirectKitDraft {
        kit_id: "box_primitive_kit".to_owned(),
        display_name: "Box Primitive Kit".to_owned(),
        source_kind: DirectKitSourceKind::Primitive,
        source_ref: "box_primitive".to_owned(),
        identity_summary: "This stays a box-like primitive.".to_owned(),
        changeable_properties: exposures,
        locked_properties,
        included_presets: Vec::new(),
        evidence_refs: vec![DirectKitEvidenceRef {
            evidence_kind: DirectKitEvidenceKind::PropertyEndpointSheet,
            path: "evidence/property-endpoints.json".to_owned(),
            status: DirectKitEvidenceStatus::Passed,
            human_review_required: true,
        }],
        review_tier: if visibility == DirectKitVisibility::PersonalOnly {
            ObjectPlanReviewTier::Personal
        } else {
            ObjectPlanReviewTier::Draft
        },
        visibility,
        created_from: DirectKitCreatedFrom::CurrentPrimitive,
    }
}

fn write_kit(path: &std::path::Path, kit: &DirectKitDraft) {
    fs::write(
        path,
        serde_json::to_string_pretty(kit).expect("kit serializes"),
    )
    .expect("write kit");
}

fn read_json(path: impl AsRef<std::path::Path>) -> serde_json::Value {
    serde_json::from_slice(&fs::read(path).expect("read json")).expect("parse json")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
