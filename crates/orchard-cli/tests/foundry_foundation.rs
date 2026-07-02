#![forbid(unsafe_code)]

use std::fs;
use std::process::Command;

use orchard_foundry::{
    ArchetypeDraftMaterializationReport, ControlProfileControlKind, ControlProfileTopologyBehavior,
    DraftControl, DraftRepairSuggestion, FoundryFoundationDraft, FoundryKitPackage,
    foundation_adversarial_report, foundation_draft_template, validate_foundation_draft,
};

#[test]
fn foundry_foundation_cli_creates_validates_materializes_and_reports() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let draft_path = temp_dir.path().join("box-primitive-draft.json");
    let kit_dir = temp_dir.path().join("kit-draft");
    let report_path = temp_dir.path().join("adversarial-report.json");

    assert!(
        Command::new(exe)
            .args([
                "foundry-foundation",
                "new",
                "--category",
                "boxes",
                "--family",
                "box-primitive",
                "--out",
            ])
            .arg(&draft_path)
            .status()
            .expect("run foundation new")
            .success()
    );
    assert!(draft_path.is_file());

    assert!(
        Command::new(exe)
            .args(["foundry-foundation", "validate"])
            .arg(&draft_path)
            .status()
            .expect("run foundation validate")
            .success()
    );

    assert!(
        Command::new(exe)
            .args(["foundry-foundation", "materialize"])
            .arg(&draft_path)
            .args(["--out-dir"])
            .arg(&kit_dir)
            .status()
            .expect("run foundation materialize")
            .success()
    );
    for name in [
        "foundation-draft.json",
        "foundation-validation.json",
        "foundry-kit-package.json",
        "kit-manifest.json",
        "provider-pack.json",
        "style-pack.json",
        "control-profile.json",
        "candidate-strategy-pack.json",
        "quality-gate-profile.json",
        "review-manifest.json",
    ] {
        assert!(kit_dir.join(name).is_file(), "{name} should exist");
    }
    let package: FoundryKitPackage = serde_json::from_slice(
        &fs::read(kit_dir.join("foundry-kit-package.json")).expect("read package"),
    )
    .expect("parse package");
    assert!(matches!(
        package.kit.quality_tier,
        orchard_foundry::FoundryKitQualityTier::Draft
    ));
    assert!(!package.kit.catalog_visibility_policy.default_novice_catalog);
    assert!(package.catalog_manifest.default_visible_kit_ids.is_empty());

    assert!(
        Command::new(exe)
            .args(["foundry-foundation", "adversarial-report"])
            .arg(&draft_path)
            .args(["--out"])
            .arg(&report_path)
            .status()
            .expect("run foundation adversarial report")
            .success()
    );
    let report: orchard_foundry::DraftAdversarialReport =
        serde_json::from_slice(&fs::read(&report_path).expect("read report"))
            .expect("parse report");
    assert_eq!(report.questions.len(), 11);
    assert_eq!(
        report,
        foundation_adversarial_report(
            &serde_json::from_slice(&fs::read(&draft_path).expect("read draft"))
                .expect("parse draft")
        )
    );
}

#[test]
fn foundry_archetype_materializer_cli_writes_internal_box_primitive_drafts() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("box-primitive");

    assert!(
        Command::new(exe)
            .args([
                "foundry",
                "materialize-archetype",
                "--archetype",
                "box-primitive",
                "--family-id",
                "box-primitive",
                "--style-id",
                "plain-clay",
                "--out-dir",
            ])
            .arg(&out_dir)
            .status()
            .expect("run archetype materializer")
            .success()
    );

    for name in [
        "family-blueprint-draft.json",
        "provider-taxonomy-draft.json",
        "style-pack-draft.json",
        "control-profile-draft.json",
        "candidate-strategy-draft.json",
        "quality-gate-draft.json",
        "test-plan-draft.json",
        "review-checklist.md",
        "materialization-report.json",
    ] {
        assert!(out_dir.join(name).is_file(), "{name} should exist");
    }

    let report: ArchetypeDraftMaterializationReport = serde_json::from_slice(
        &fs::read(out_dir.join("materialization-report.json")).expect("read report"),
    )
    .expect("parse report");
    assert_eq!(report.archetype_id, "box-primitive");
    assert_eq!(report.family_id, "box_primitive");
    assert_eq!(report.style_id, "plain_clay");
    assert!(!report.publish_allowed);
    assert!(!report.novice_visible);
    assert!(report.human_review_required);
    assert!(!report.showcase_allowed);
    assert!(!report.geometry_payload_present);
    assert!(!report.raw_vertex_payload_present);
    assert!(!report.missing_taste_bearing_providers.is_empty());
}

#[test]
fn foundry_foundation_cli_suggests_repairs_from_validation_report() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let draft_path = temp_dir.path().join("invalid-draft.json");
    let validation_path = temp_dir.path().join("validation.json");
    let repair_path = temp_dir.path().join("repair.json");

    let mut draft = foundation_draft_template("boxes", "box-primitive");
    draft.quality_gate_profile = None;
    draft.control_profile.controls = (0..8)
        .map(|index| DraftControl {
            control_id: format!("control_{index}"),
            label: format!("Control {index}"),
            description: "Visible change.".to_owned(),
            kind: ControlProfileControlKind::Choice,
            primary: true,
            visible: true,
            owned_family_slots: Vec::new(),
            owned_provider_slots: vec![format!("slot_{index}")],
            topology_behavior: ControlProfileTopologyBehavior::TopologyChanging,
            visible_effect_expectation: "Visible change.".to_owned(),
        })
        .collect();
    fs::write(
        &draft_path,
        serde_json::to_vec_pretty(&draft).expect("serialize draft"),
    )
    .expect("write draft");
    let validation = validate_foundation_draft(&draft);
    assert!(!validation.is_valid());
    fs::write(
        &validation_path,
        serde_json::to_vec_pretty(&validation).expect("serialize validation"),
    )
    .expect("write validation");

    assert!(
        !Command::new(exe)
            .args(["foundry-foundation", "validate"])
            .arg(&draft_path)
            .status()
            .expect("run invalid foundation validate")
            .success()
    );

    assert!(
        Command::new(exe)
            .args(["foundry-foundation", "suggest-repair"])
            .arg(&draft_path)
            .args(["--validation-report"])
            .arg(&validation_path)
            .args(["--out"])
            .arg(&repair_path)
            .status()
            .expect("run foundation suggest repair")
            .success()
    );
    let repair: DraftRepairSuggestion =
        serde_json::from_slice(&fs::read(&repair_path).expect("read repair"))
            .expect("parse repair");
    assert!(
        repair
            .suggestions
            .iter()
            .any(|suggestion| suggestion.contains("quality gate"))
    );
    assert!(
        repair
            .suggestions
            .iter()
            .any(|suggestion| suggestion.contains("seven or fewer"))
    );
}

#[test]
fn foundry_foundation_cli_rejects_unknown_raw_geometry_fields() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let draft_path = temp_dir.path().join("raw-field-draft.json");

    let draft: FoundryFoundationDraft = foundation_draft_template("boxes", "box-primitive");
    let mut value = serde_json::to_value(draft).expect("draft value");
    let object = value.as_object_mut().expect("draft object");
    object.insert(
        "mesh_payload".to_owned(),
        serde_json::json!({"vertices": [[0, 0, 0]]}),
    );
    object.insert(
        "raw_vertex_positions".to_owned(),
        serde_json::json!([[0, 0, 0]]),
    );
    fs::write(
        &draft_path,
        serde_json::to_vec_pretty(&value).expect("serialize raw field draft"),
    )
    .expect("write raw field draft");

    assert!(
        !Command::new(exe)
            .args(["foundry-foundation", "validate"])
            .arg(&draft_path)
            .status()
            .expect("run foundation validate")
            .success()
    );
}
