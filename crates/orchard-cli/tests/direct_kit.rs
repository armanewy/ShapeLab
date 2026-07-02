#![forbid(unsafe_code)]

use std::fs;
use std::process::Command;

use orchard_foundry::{
    DirectKitCreatedFrom, DirectKitDraft, DirectKitEvidenceKind, DirectKitEvidenceRef,
    DirectKitEvidenceStatus, DirectKitSourceKind, DirectKitVisibility, ObjectPlanReviewTier,
    PrimitiveKind, direct_kit_property_exposures_for_primitive,
};

#[test]
fn direct_kit_cli_valid_box_test_runs() {
    let report = run_kit(
        valid_kit(PrimitiveKind::BoxPrimitive, "box_primitive"),
        "box",
    );

    assert_eq!(report["status"], "Passed");
    assert_eq!(report["approved"], false);
    assert_eq!(report["publish_allowed"], false);
    assert_eq!(report["human_review_required"], true);
}

#[test]
fn direct_kit_cli_valid_flat_panel_test_runs() {
    let report = run_kit(
        valid_kit(PrimitiveKind::FlatPanelPrimitive, "flat_panel_primitive"),
        "panel",
    );

    assert_eq!(report["status"], "Passed");
    assert_eq!(report["failed_capabilities"], 0);
}

#[test]
fn direct_kit_cli_missing_evidence_produces_warnings_not_passed() {
    let mut kit = valid_kit(PrimitiveKind::BoxPrimitive, "box_primitive");
    kit.evidence_refs.clear();
    let report = run_kit(kit, "missing-evidence");

    assert_eq!(report["status"], "Warnings");
    assert_ne!(report["status"], "Passed");
    assert_eq!(report["approved"], false);
    assert_eq!(report["publish_allowed"], false);
}

#[test]
fn direct_kit_cli_invalid_property_fails() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let kit_path = temp_dir.path().join("kit.json");
    let out_dir = temp_dir.path().join("out");
    let mut kit = valid_kit(PrimitiveKind::BoxPrimitive, "box_primitive");
    kit.changeable_properties[0].property_id = "unknown_width".to_owned();
    write_kit(&kit_path, &kit);

    let output = Command::new(exe)
        .args(["direct-kit", "test", "--kit"])
        .arg(&kit_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .output()
        .expect("run direct-kit test");

    assert!(!output.status.success());
    let report = read_json(out_dir.join("direct-kit-test-report.json"));
    assert_eq!(report["status"], "Failed");
    assert_eq!(report["approved"], false);
    assert_eq!(report["publish_allowed"], false);
}

#[test]
fn direct_kit_cli_user_summary_hides_technical_terms() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let kit_path = temp_dir.path().join("kit.json");
    let out_dir = temp_dir.path().join("out");
    write_kit(
        &kit_path,
        &valid_kit(PrimitiveKind::SpherePrimitive, "sphere_primitive"),
    );
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    assert!(
        Command::new(exe)
            .args(["direct-kit", "test", "--kit"])
            .arg(&kit_path)
            .args(["--out-dir"])
            .arg(&out_dir)
            .status()
            .expect("run direct-kit test")
            .success()
    );

    let summary = fs::read_to_string(out_dir.join("user-summary.md")).expect("read summary");
    let lower = summary.to_ascii_lowercase();
    for forbidden in [
        "kernel",
        "module",
        "provider",
        "slot",
        "topology",
        "fingerprint",
        "conformance",
        "artifact",
        "raw transform",
        "mesh payload",
        "generated variation",
        "candidate",
        "runtime llm",
        "public catalog",
        "game-ready",
    ] {
        assert!(
            !lower.contains(forbidden),
            "summary should not expose {forbidden}: {summary}"
        );
    }
}

#[test]
fn direct_kit_cli_output_is_deterministic() {
    let first = run_kit(
        valid_kit(PrimitiveKind::BoxPrimitive, "box_primitive"),
        "first",
    );
    let second = run_kit(
        valid_kit(PrimitiveKind::BoxPrimitive, "box_primitive"),
        "second",
    );

    assert_eq!(first, second);
}

fn run_kit(kit: DirectKitDraft, name: &str) -> serde_json::Value {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let kit_path = temp_dir.path().join(format!("{name}.json"));
    let out_dir = temp_dir.path().join("out");
    write_kit(&kit_path, &kit);

    let output = Command::new(exe)
        .args(["direct-kit", "test", "--kit"])
        .arg(&kit_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .output()
        .expect("run direct-kit test");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    read_json(out_dir.join("direct-kit-test-report.json"))
}

fn valid_kit(primitive_kind: PrimitiveKind, source_ref: &str) -> DirectKitDraft {
    let mut exposures = direct_kit_property_exposures_for_primitive(primitive_kind);
    let locked_properties = exposures.split_off(1);
    DirectKitDraft {
        kit_id: format!("{source_ref}_kit"),
        display_name: "Reusable Shape Kit".to_owned(),
        source_kind: DirectKitSourceKind::Primitive,
        source_ref: source_ref.to_owned(),
        identity_summary: identity_summary(primitive_kind).to_owned(),
        changeable_properties: exposures,
        locked_properties,
        included_presets: Vec::new(),
        evidence_refs: vec![DirectKitEvidenceRef {
            evidence_kind: DirectKitEvidenceKind::PropertyEndpointSheet,
            path: "evidence/property-endpoints.json".to_owned(),
            status: DirectKitEvidenceStatus::Passed,
            human_review_required: true,
        }],
        review_tier: ObjectPlanReviewTier::Draft,
        visibility: DirectKitVisibility::Draft,
        created_from: DirectKitCreatedFrom::CurrentPrimitive,
    }
}

fn identity_summary(primitive_kind: PrimitiveKind) -> &'static str {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => "This stays a box-like primitive.",
        PrimitiveKind::FlatPanelPrimitive => "This stays a flat panel.",
        PrimitiveKind::SpherePrimitive => "This stays a round primitive.",
        PrimitiveKind::CylinderPrimitive => "This primitive is not active.",
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
