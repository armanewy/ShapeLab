#![forbid(unsafe_code)]

use std::fs;
use std::process::Command;

use shape_foundry::{
    ControlProfileControlKind, ControlProfileTopologyBehavior, DraftControl, DraftRepairSuggestion,
    FoundryFoundationDraft, FoundryKitPackage, WAVE37_WEAPON_ARMOR_FAMILY_IDS,
    foundation_adversarial_report, foundation_draft_template, validate_foundation_draft,
    weapon_armor_foundation_batch_summary, weapon_armor_foundation_draft_batch,
};

#[test]
fn foundry_foundation_cli_creates_validates_materializes_and_reports() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let draft_path = temp_dir.path().join("sword-draft.json");
    let kit_dir = temp_dir.path().join("kit-draft");
    let report_path = temp_dir.path().join("adversarial-report.json");

    assert!(
        Command::new(exe)
            .args([
                "foundry-foundation",
                "new",
                "--category",
                "weapons",
                "--family",
                "sword",
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
        shape_foundry::FoundryKitQualityTier::Draft
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
    let report: shape_foundry::DraftAdversarialReport =
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
fn foundry_foundation_cli_suggests_repairs_from_validation_report() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let draft_path = temp_dir.path().join("invalid-draft.json");
    let validation_path = temp_dir.path().join("validation.json");
    let repair_path = temp_dir.path().join("repair.json");

    let mut draft = foundation_draft_template("weapons", "sword");
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
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let draft_path = temp_dir.path().join("raw-field-draft.json");

    let draft: FoundryFoundationDraft = foundation_draft_template("weapons", "sword");
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

#[test]
fn foundry_foundation_cli_exports_wave37_weapon_armor_batch() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("wave37-foundations");

    assert!(
        Command::new(exe)
            .args(["foundry-foundation", "batch", "--out-dir"])
            .arg(&out_dir)
            .status()
            .expect("run foundation batch")
            .success()
    );

    assert!(out_dir.join("foundation-batch-summary.json").is_file());
    let summary: Vec<shape_foundry::FoundationBatchSummaryRow> =
        serde_json::from_slice(&fs::read(out_dir.join("foundation-batch-summary.json")).unwrap())
            .expect("parse batch summary");
    assert_eq!(summary, weapon_armor_foundation_batch_summary());

    for family_id in WAVE37_WEAPON_ARMOR_FAMILY_IDS {
        let draft_path = out_dir
            .join("drafts")
            .join(format!("{family_id}.foundation-draft.json"));
        let validation_path = out_dir
            .join("validation")
            .join(format!("{family_id}.validation.json"));
        let report_path = out_dir
            .join("adversarial")
            .join(format!("{family_id}.adversarial-report.json"));
        assert!(draft_path.is_file(), "{family_id} draft file should exist");
        assert!(
            validation_path.is_file(),
            "{family_id} validation file should exist"
        );
        assert!(
            report_path.is_file(),
            "{family_id} adversarial report should exist"
        );

        assert!(
            Command::new(exe)
                .args(["foundry-foundation", "validate"])
                .arg(&draft_path)
                .status()
                .expect("validate exported batch draft")
                .success()
        );

        let exported_draft: FoundryFoundationDraft =
            serde_json::from_slice(&fs::read(&draft_path).expect("read draft"))
                .expect("parse exported draft");
        let expected_draft = weapon_armor_foundation_draft_batch()
            .into_iter()
            .find(|draft| draft.family_blueprint.family_id == *family_id)
            .expect("expected draft");
        assert_eq!(exported_draft, expected_draft);

        let exported_report: shape_foundry::DraftAdversarialReport =
            serde_json::from_slice(&fs::read(&report_path).expect("read adversarial report"))
                .expect("parse adversarial report");
        assert_eq!(
            exported_report,
            foundation_adversarial_report(&exported_draft)
        );
    }
}
